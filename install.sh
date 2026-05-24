#!/bin/bash
# consys installer
# Usage: curl -fsSL https://raw.githubusercontent.com/Nate-Rich/consys/main/install.sh | bash

set -euo pipefail

INSTALL_DIR="/usr/local/bin"
BINARY="consys"
REPO="Nate-Rich/consys"

need() {
    if ! command -v "$1" >/dev/null 2>&1; then
        echo "missing required command: $1" >&2
        exit 1
    fi
}

need curl
need sha256sum
need tar
need uname

ARCH=$(uname -m)
case "$ARCH" in
    x86_64)  TARGET="x86_64-unknown-linux-gnu" ;;
    aarch64) TARGET="aarch64-unknown-linux-gnu" ;;
    armv7l)  TARGET="armv7-unknown-linux-gnueabihf" ;;
    *)
        echo "unsupported architecture: $ARCH" >&2
        exit 1
        ;;
esac

VERSION=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
    | grep '"tag_name"' \
    | cut -d '"' -f 4)

if [ -z "$VERSION" ]; then
    echo "could not determine latest version" >&2
    exit 1
fi

BASE_URL="https://github.com/$REPO/releases/download/$VERSION"
ARCHIVE="consys-$VERSION-$TARGET.tar.gz"
CHECKSUMS="sha256.txt"

echo "consys $VERSION ($TARGET)"
echo "installing to $INSTALL_DIR/$BINARY"

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

cd "$TMP"

if curl -fsSL "$BASE_URL/$ARCHIVE" -o "$ARCHIVE"; then
    curl -fsSL "$BASE_URL/$CHECKSUMS" -o "$CHECKSUMS"
    if ! grep " $ARCHIVE$" "$CHECKSUMS" | sha256sum --check --status; then
        echo "checksum verification failed — aborting" >&2
        exit 1
    fi
    echo "checksum verified"

    tar -xzf "$ARCHIVE"
    if [ ! -f "$BINARY" ]; then
        echo "archive did not contain $BINARY" >&2
        exit 1
    fi
else
    # Compatibility path for v1.0.0, which shipped raw binaries with
    # per-target .sha256 files instead of versioned tar archives.
    LEGACY_BINARY="consys-$TARGET"
    LEGACY_SUM="$LEGACY_BINARY.sha256"
    curl -fsSL "$BASE_URL/$LEGACY_BINARY" -o "$LEGACY_BINARY"
    curl -fsSL "$BASE_URL/$LEGACY_SUM" -o "$LEGACY_SUM"
    if ! sha256sum --check "$LEGACY_SUM" --status; then
        echo "checksum verification failed — aborting" >&2
        exit 1
    fi
    echo "checksum verified"
    mv "$LEGACY_BINARY" "$BINARY"
fi

sudo install -m 755 "$BINARY" "$INSTALL_DIR/$BINARY"

echo "done. run: consys"
