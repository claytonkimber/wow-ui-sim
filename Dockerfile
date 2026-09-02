# wow-ui-sim Dockerfile
#
# Build: docker build -t ghcr.io/osso/wow-ui-sim .
# Usage: docker run --rm -v ./MyAddon:/app/Interface/AddOns/MyAddon ghcr.io/osso/wow-ui-sim run-tests MyAddon

# =============================================================================
# Build Stage
# =============================================================================
FROM rust:1.92-bookworm AS builder

# Install system build dependencies:
# - clang + mold: fast linker configured in .cargo/config.toml
# - git: needed by some build scripts
# - pkg-config + cmake: required by C-backed Rust crates (mlua vendored Lua, wgpu)
RUN apt-get update && apt-get install -y --no-install-recommends \
    clang \
    mold \
    git \
    pkg-config \
    cmake \
    fonts-dejavu-core \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy dependency manifests first so Docker can cache the dep-fetch layer.
# iced-wgpu-patched/ is a [patch.crates-io] override and must be present
# before `cargo fetch` or any build step.
COPY Cargo.toml Cargo.lock ./
COPY .cargo/config.toml .cargo/config.toml
COPY iced-wgpu-patched/ iced-wgpu-patched/
COPY iced-dynamic/ iced-dynamic/
COPY xtask/ xtask/

# Copy test targets referenced by Cargo.toml's [[test]] sections.
COPY tests/ tests/

# Fetch all dependencies (cached layer — only invalidated by Cargo.toml/lock changes).
RUN cargo fetch --locked

# Copy the full source tree and build the simulator binary.
# Drop the `sound` feature (rodio/ALSA) — not needed for headless test runs and
# it would pull extra system libraries into the distroless runtime stage.
# `gui` is still required: src/iced_app/frame_collect.rs (always compiled)
# references hit_grid::HitOrderKey, and `mod hit_grid` is #[cfg(feature="gui")].
# `client-retail` selects the retail BlizzardUI profile and defines
# client_profile::ACTIVE. This matches the repo's CI build (test.yml /
# release.yml use `sound,gui,client-retail`); we omit only `sound`.
COPY build.rs ./
COPY data/ data/
COPY src/ src/
RUN cargo build --release --bin wow-sim --no-default-features \
        --features gui,client-retail --locked \
    && strip /build/target/release/wow-sim

# =============================================================================
# BlizzardUI Stage — sparse-checkout from Gethe/wow-ui-source
# =============================================================================
FROM alpine/git AS blizzard-ui

ARG BLIZZARD_UI_TAG=12.1.0
RUN git clone --filter=blob:none --no-checkout --depth=1 --branch ${BLIZZARD_UI_TAG} \
        https://github.com/Gethe/wow-ui-source.git /wow-ui-source \
    && cd /wow-ui-source \
    && git sparse-checkout init --cone \
    && git sparse-checkout set Interface/AddOns \
    && git checkout ${BLIZZARD_UI_TAG} \
    && rm -rf /wow-ui-source/.git \
    && touch /wow-ui-source/Interface/AddOns/.wow-ui-sim-blizzard-ui-complete \
    && printf 'profile=retail\nsource=gethe-image-build\nfallback=none\n' > /wow-ui-source/Interface/AddOns/.wow-ui-sim-blizzard-ui-provenance

# =============================================================================
# Runtime Stage
# =============================================================================
# debian:bookworm-slim (not distroless) so we can install the Mesa software
# Vulkan stack. The `screenshot` command renders via wgpu, which needs a Vulkan
# adapter; the distroless image had none, so screenshots panicked with
# "Failed to find GPU adapter". mesa-vulkan-drivers provides lavapipe (the
# llvmpipe CPU Vulkan ICD), and headless.rs selects it via
# force_fallback_adapter when WOW_SIM_SOFTWARE_RENDER=1 (set below). This is the
# same software-render path upstream CI uses (.github/workflows/test.yml), so it
# needs no GPU passthrough and renders deterministically anywhere. Same Debian 12
# glibc base as the rust:1.92-bookworm builder, so the binary is ABI-compatible.
FROM debian:bookworm-slim

# Vulkan loader + Mesa lavapipe (software Vulkan) for headless screenshot
# rendering. ca-certificates kept for any runtime TLS (CASC fallback fetches).
RUN apt-get update && apt-get install -y --no-install-recommends \
    libvulkan1 \
    mesa-vulkan-drivers \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy stripped binary from build stage.
COPY --from=builder /build/target/release/wow-sim /app/wow-sim

# Copy data directories from the build context.
# These are read at runtime and are NOT compiled into the binary.
#
# BlizzardUI: Blizzard's base UI Lua/XML, placed in the cache path the
# simulator checks. The .wow-ui-sim-blizzard-ui-complete marker is
# created in the blizzard-ui stage and tells the runtime the cache
# is ready, skipping the CASC sync attempt.
COPY --from=blizzard-ui /wow-ui-source/Interface/AddOns/ /root/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/

# TestFramework: assertion library loaded automatically by `run-tests`
COPY Interface/AddOns/TestFramework/ /app/Interface/AddOns/TestFramework/

# DejaVu fonts for text shaping fallback when CASC is unavailable.
# fontdb::load_system_fonts() picks these up from the standard path.
COPY --from=builder /usr/share/fonts/truetype/dejavu/ /usr/share/fonts/truetype/dejavu/

# Skip SavedVariables loading — no WTF directory is available in the image.
ENV WOW_SIM_NO_SAVED_VARS=1

# Force the wgpu software (fallback) adapter so `screenshot` renders on lavapipe
# without a hardware GPU. headless.rs reads this; run-tests/dump-tree ignore it.
ENV WOW_SIM_SOFTWARE_RENDER=1

ENTRYPOINT ["/app/wow-sim"]
