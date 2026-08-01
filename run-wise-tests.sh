#!/usr/bin/env bash
# Run Wise's headless wow-ui-sim tests (tests/*.lua in the Wise addon).
#
# Usage:  ./run-wise-tests.sh [extra wow-sim args...]
#
# Builds nothing — assumes `cargo build --release --bin wow-sim --no-default-features`
# has already produced target/release/wow-sim. Pass --no-saved-vars by default
# for a fast startup; add --no-addons to skip third-party addons.
set -euo pipefail

# cargo/rust live in ~/.cargo/bin, which isn't always on PATH in fresh shells.
export PATH="${USERPROFILE:-$HOME}/.cargo/bin:$PATH"

SIM_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN="$SIM_DIR/target/release/wow-sim"

if [[ ! -x "$BIN" && ! -f "$BIN.exe" ]]; then
    echo "wow-sim not built. Run:" >&2
    echo "  cargo build --release --bin wow-sim --no-default-features" >&2
    exit 1
fi
[[ -f "$BIN.exe" ]] && BIN="$BIN.exe"

# The simulator mounts addons from its own Interface/AddOns. Wise lives in
# ../Wise (a sibling in Interface/_dev_), so symlink it in if not already there.
ADDONS_DIR="$SIM_DIR/Interface/AddOns"
WISE_SRC="$(cd "$SIM_DIR/../Wise" && pwd)"
WISE_LINK="$ADDONS_DIR/Wise"
if [[ ! -e "$WISE_LINK" ]]; then
    echo "Linking Wise into $ADDONS_DIR ..." >&2
    # Windows junction (no admin/symlink privilege needed). cmd.exe isn't on the
    # Git-Bash PATH, so call it by full path; mklink needs Windows-style paths.
    "$SYSTEMROOT/System32/cmd.exe" //c mklink //J \
        "$(cygpath -w "$WISE_LINK")" "$(cygpath -w "$WISE_SRC")" >/dev/null
fi

exec "$BIN" --no-saved-vars run-tests Wise "$@"
