#!/usr/bin/env bash
# Run the Mists panel parity matrix once per installed third-party addon.
#
# Reads tools/classic-addon-manifest.tsv, selects the Mists rows, symlinks one
# addon at a time into Interface/AddOns, and runs scripts/mists-panel-parity.sh
# with third-party addons enabled. Any panel lua-error, missing frame,
# low-signal render, or visual-baseline regression fails that addon row.
# Use --start-at <addon> to resume from a known manifest row without rerunning
# earlier addons. Completed full-addon rows also write .passed under their
# artifact directory so interrupted runs against the same --out-dir skip them.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEFAULT_MISTS_CARGO_TARGET_DIR="${XDG_CACHE_HOME:-$HOME/.cache}/wow-ui-sim/cargo-targets/mists-panel-parity"
CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-${MISTS_CARGO_TARGET_DIR:-$DEFAULT_MISTS_CARGO_TARGET_DIR}}"
MANIFEST="$REPO_ROOT/tools/classic-addon-manifest.tsv"
COMPAT_ROOT="$REPO_ROOT/tools/classic-addon-compat"
ADDONS_DIR="$REPO_ROOT/Interface/AddOns"
OUT_DIR="$REPO_ROOT/target/mists-addon-panel-parity"
WOW_SIM_BIN="${WOW_SIM_BIN:-$CARGO_TARGET_DIR/debug/wow-sim}"
PANEL_VISUAL_METRICS_BIN="${PANEL_VISUAL_METRICS_BIN:-$CARGO_TARGET_DIR/debug/panel-visual-metrics}"

export CARGO_TARGET_DIR

source "$REPO_ROOT/scripts/classic-addon-sources.sh"

NAME_FILTER=""
START_AT=""
PANEL_FILTER=""
SKIP_BUILD=0
WITH_SAVED_VARS=0
KEEP_SYMLINKS=0
VALIDATE_ONLY=0
ACTIVE_ADDON=""

usage() {
    sed -n '2,/^set -euo/p' "$0" | sed 's/^# \?//;$d'
}

while [ $# -gt 0 ]; do
    case "$1" in
        --addon) NAME_FILTER="$2"; shift 2 ;;
        --start-at) START_AT="$2"; shift 2 ;;
        --panel) PANEL_FILTER="$2"; shift 2 ;;
        --out-dir) OUT_DIR="$2"; shift 2 ;;
        --skip-build) SKIP_BUILD=1; shift ;;
        --with-saved-vars) WITH_SAVED_VARS=1; shift ;;
        --keep-symlinks) KEEP_SYMLINKS=1; shift ;;
        --validate-only) VALIDATE_ONLY=1; shift ;;
        --help|-h) usage; exit 0 ;;
        *) echo "ERROR: unknown argument '$1'" >&2; exit 2 ;;
    esac
done

validate_manifest() {
    [ -f "$MANIFEST" ] || { echo "ERROR: manifest not found: $MANIFEST" >&2; return 2; }
    local count=0
    local reached_start=0
    while IFS=$'\t' read -r name profile url ref subpath; do
        should_skip_manifest_row "$name" "$profile" && continue
        should_skip_before_start "$name" reached_start && continue
        validate_local_mists_source "$name" "$url" "$subpath"
        count=$((count + 1))
    done < "$MANIFEST"
    if [ -n "$START_AT" ] && [ -z "$NAME_FILTER" ] && [ "$reached_start" -eq 0 ]; then
        echo "ERROR: --start-at addon not found in installed Mists manifest rows: $START_AT" >&2
        return 2
    fi
    if [ "$count" -eq 0 ]; then
        echo "ERROR: no installed Mists addons matched filter '$NAME_FILTER'" >&2
        return 2
    fi
    echo "$count installed Mists addon row(s) validated from $MANIFEST"
}

validate_runner_binaries() {
    [ -x "$WOW_SIM_BIN" ] || {
        echo "ERROR: wow-sim binary not found or not executable: $WOW_SIM_BIN" >&2
        echo "       Rebuild it or rerun without --skip-build." >&2
        return 2
    }
    [ -x "$PANEL_VISUAL_METRICS_BIN" ] || {
        echo "ERROR: panel-visual-metrics binary not found or not executable: $PANEL_VISUAL_METRICS_BIN" >&2
        echo "       Rebuild it or rerun without --skip-build." >&2
        return 2
    }
}

should_skip_manifest_row() {
    local name="${1:-}" profile="${2:-}"
    [[ "$name" =~ ^# ]] && return 0
    [ -z "$name" ] && return 0
    [ "$name" = "name" ] && return 0
    [ "$profile" != "mists" ] && return 0
    [ -n "$NAME_FILTER" ] && [ "$NAME_FILTER" != "$name" ] && return 0
    return 1
}

should_skip_before_start() {
    local name="$1"
    local -n reached_start_ref="$2"
    [ -z "$START_AT" ] && return 1
    [ -n "$NAME_FILTER" ] && return 1
    [ "$reached_start_ref" -eq 1 ] && return 1
    if [ "$name" = "$START_AT" ]; then
        reached_start_ref=1
        return 1
    fi
    return 0
}

addon_pass_marker() {
    local name="$1"
    [ -n "$PANEL_FILTER" ] && return 1
    echo "$OUT_DIR/$name/.passed"
}

should_skip_completed_addon() {
    local name="$1"
    local marker
    marker="$(addon_pass_marker "$name")" || return 1
    [ -f "$marker" ] || return 1
    echo ""
    echo "=== $name (mists panels) ==="
    echo "Skipping $name; existing pass marker: $marker"
    return 0
}

validate_local_mists_source() {
    local name="$1" url="$2" subpath="$3"
    if ! is_local_source "$url" && ! is_manifest_managed_source "$url"; then
        echo "ERROR: Mists addon $name must use local: or mists-addon: source, got $url" >&2
        return 2
    fi
    local src
    src="$(resolve_addon_source_root "$name" mists "$url" "$REPO_ROOT/vendor/addons")/$subpath"
    [ -d "$src" ] || { echo "ERROR: Mists addon source missing: $src" >&2; return 2; }
}

install_symlink() {
    local name="$1" url="$2" subpath="$3"
    local src
    src="$(resolve_addon_source_root "$name" mists "$url" "$REPO_ROOT/vendor/addons")/$subpath"
    local dst="$ADDONS_DIR/$name"
    [ -d "$src" ] || { echo "ERROR: $src not found" >&2; return 1; }
    [ -L "$dst" ] && rm "$dst"
    [ -e "$dst" ] && { echo "ERROR: $dst exists and is not a symlink" >&2; return 1; }
    ln -s "$src" "$dst"
}

install_compat_shims() {
    local name="$1"
    local compat_dir="$COMPAT_ROOT/$name"
    [ -d "$compat_dir" ] || return 0
    local shim
    for shim in "$compat_dir"/*/; do
        [ -d "$shim" ] || continue
        local shim_name
        shim_name="$(basename "$shim")"
        local dst="$ADDONS_DIR/$shim_name"
        [ -L "$dst" ] && rm "$dst"
        [ -e "$dst" ] && { echo "ERROR: $dst exists and is not a symlink" >&2; return 1; }
        ln -s "$shim" "$dst"
        echo "  -> compat shim: $shim_name"
    done
}

remove_symlink() {
    local name="$1"
    local dst="$ADDONS_DIR/$name"
    [ -L "$dst" ] && rm "$dst"
    return 0
}

remove_compat_shims() {
    local name="$1"
    local compat_dir="$COMPAT_ROOT/$name"
    [ -d "$compat_dir" ] || return 0
    local shim
    for shim in "$compat_dir"/*/; do
        [ -d "$shim" ] || continue
        remove_symlink "$(basename "$shim")"
    done
}

teardown_addon() {
    local name="$1"
    if [ "$KEEP_SYMLINKS" -eq 0 ]; then
        remove_symlink "$name"
        remove_compat_shims "$name"
    fi
    return 0
}

finish_active_addon() {
    local name="$ACTIVE_ADDON"
    ACTIVE_ADDON=""
    if [ -n "$name" ]; then
        teardown_addon "$name" || true
    fi
}

run_addon_panels() {
    local name="$1" url="$2" subpath="$3"
    local addon_out_dir="$OUT_DIR/$name"
    local args=(--skip-build --with-addons --out-dir "$addon_out_dir")
    if [ "$WITH_SAVED_VARS" -eq 1 ]; then
        args+=(--with-saved-vars)
    fi
    if [ -n "$PANEL_FILTER" ]; then
        args+=(--panel "$PANEL_FILTER")
    fi

    echo ""
    echo "=== $name (mists panels) ==="
    ACTIVE_ADDON="$name"
    install_symlink "$name" "$url" "$subpath"
    install_compat_shims "$name"
    if WOW_SIM_BIN="$WOW_SIM_BIN" PANEL_VISUAL_METRICS_BIN="$PANEL_VISUAL_METRICS_BIN" \
            "$REPO_ROOT/scripts/mists-panel-parity.sh" "${args[@]}"; then
        local marker
        if marker="$(addon_pass_marker "$name")"; then
            touch "$marker"
        fi
        finish_active_addon
        return 0
    fi
    finish_active_addon
    return 1
}

cleanup_active_addon_on_exit() {
    if [ -n "$ACTIVE_ADDON" ]; then
        finish_active_addon
    fi
}

cleanup_active_addon_on_interrupt() {
    if [ -n "$ACTIVE_ADDON" ]; then
        echo "ERROR: interrupted; removing addon symlinks for $ACTIVE_ADDON" >&2
        finish_active_addon
    fi
}

validate_manifest
if [ "$VALIDATE_ONLY" -eq 1 ]; then
    exit 0
fi

mkdir -p "$OUT_DIR"
if [ "$SKIP_BUILD" -eq 0 ]; then
    echo "Building Mists panel binaries in $CARGO_TARGET_DIR"
    cargo build --bin wow-sim --bin panel-visual-metrics --no-default-features --features "sound,gui,casc,client-mists"
fi
validate_runner_binaries
trap cleanup_active_addon_on_exit EXIT
trap 'cleanup_active_addon_on_interrupt; exit 130' INT TERM

declare -i pass=0 fail=0
declare -i reached_start=0
while IFS=$'\t' read -r name profile url ref subpath; do
    should_skip_manifest_row "$name" "$profile" && continue
    should_skip_before_start "$name" reached_start && continue
    if should_skip_completed_addon "$name"; then
        pass+=1
        continue
    fi
    if run_addon_panels "$name" "$url" "$subpath"; then
        pass+=1
    else
        fail+=1
    fi
done < "$MANIFEST"

echo ""
echo "================================================"
echo "  panel parity passed: $pass    failed: $fail"
echo "================================================"

[ "$fail" -gt 0 ] && exit 1
exit 0
