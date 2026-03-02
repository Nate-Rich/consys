#!/bin/bash
# consys installer
# Usage: bash <(curl -s https://raw.githubusercontent.com/Nate-Rich/consys/main/install.sh)

set -e

INSTALL_DIR="/usr/local/bin"
BINARY="consys"
REPO="Nate-Rich/consys"

ARCH=$(uname -m)
case "$ARCH" in
    x86_64)  TARGET="x86_64-unknown-linux-gnu" ;;
    aarch64) TARGET="aarch64-unknown-linux-gnu" ;;
    armv7l)  TARGET="armv7-unknown-linux-gnueabihf" ;;
    *)
        echo "unsupported architecture: $ARCH"
        exit 1
        ;;
esac

VERSION=$(curl -s "https://api.github.com/repos/$REPO/releases/latest" \
    | grep '"tag_name"' \
    | cut -d '"' -f 4)

if [ -z "$VERSION" ]; then
    echo "could not determine latest version"
    exit 1
fi

BASE_URL="https://github.com/$REPO/releases/download/$VERSION"
BINARY_NAME="consys-$TARGET"
CHECKSUM_NAME="consys-$TARGET.sha256"

echo "consys $VERSION ($TARGET)"
echo "installing to $INSTALL_DIR/$BINARY"

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

curl -sL "$BASE_URL/$BINARY_NAME"   -o "$TMP/$BINARY_NAME"
curl -sL "$BASE_URL/$CHECKSUM_NAME" -o "$TMP/$CHECKSUM_NAME"

cd "$TMP"
if ! sha256sum --check "$CHECKSUM_NAME" --status; then
    echo "checksum verification failed — aborting"
    exit 1
fi
echo "checksum verified"

sudo mv "$TMP/$BINARY_NAME" "$INSTALL_DIR/$BINARY"
sudo chmod +x "$INSTALL_DIR/$BINARY"

echo "done. run: consys"
