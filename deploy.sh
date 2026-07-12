#!/usr/bin/env bash
# Deploys the site to Alwaysdata.
#
# The release binary is cross-compiled locally for x86_64 musl — a fully
# static binary with no glibc dependency, so nothing is ever compiled on
# the constrained server — then a targeted rsync ships it with the
# runtime assets (templates, static, migrations). The server's .env is
# never touched.
#
# One-time prerequisites:
#   sudo apt install musl-tools      # C compiler targeting musl (for ring)
#   (the rustup target installs itself below)
#
# Point the alwaysdata site at the binary once: working directory ~/avon,
# command ~/avon/avon.
set -euo pipefail

SERVER="avon@ssh-avon.alwaysdata.net"
DEST="avon"                       # relative to the remote home
TARGET="x86_64-unknown-linux-musl"
BIN="target/$TARGET/release/avon"

cd "$(dirname "$0")"

# --- prerequisites --------------------------------------------------------
if ! rustup target list --installed | grep -q "^$TARGET$"; then
    echo "==> installing rust target $TARGET"
    rustup target add "$TARGET"
fi

if ! command -v musl-gcc >/dev/null; then
    echo "error: musl-gcc not found."
    echo "The C parts of the dependency tree (ring, rustls' crypto) need a"
    echo "C compiler that targets musl. Install it once with:"
    echo
    echo "    sudo apt install musl-tools"
    exit 1
fi

# --- build -----------------------------------------------------------------
echo "==> building release ($TARGET)"
export CC_x86_64_unknown_linux_musl=musl-gcc
export CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=musl-gcc
cargo build --release --target "$TARGET"

# --- ship ------------------------------------------------------------------
echo "==> rsyncing binary + assets to $SERVER:~/$DEST"
rsync -avz "$BIN" "$SERVER:$DEST/avon"
rsync -avz --delete templates/ "$SERVER:$DEST/templates/"
rsync -avz --delete static/ "$SERVER:$DEST/static/"
rsync -avz --delete migrations/ "$SERVER:$DEST/migrations/"

# --- restart ----------------------------------------------------------------
if [[ -n "${ALWAYSDATA_API_KEY:-}" && -n "${ALWAYSDATA_SITE_ID:-}" ]]; then
    echo "==> restarting the site via the alwaysdata API"
    curl -fsS -X POST -u "$ALWAYSDATA_API_KEY account=avon:" \
        "https://api.alwaysdata.com/v1/site/$ALWAYSDATA_SITE_ID/restart/"
    echo "==> deployed and restarted"
else
    echo "==> deployed — restart the site from the alwaysdata panel"
    echo "    (or export ALWAYSDATA_API_KEY and ALWAYSDATA_SITE_ID so this"
    echo "    script restarts it via the API: profile > API keys, and the"
    echo "    site id from its admin URL)"
fi

echo
echo "reminder: new migrations aren't applied automatically — run any new"
echo "file in migrations/ against the database once."
