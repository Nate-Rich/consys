#!/bin/bash
# consys installer
# Usage: bash install.sh

set -e

INSTALL_DIR="/usr/local/bin"
BINARY="consys"
REPO="Nate-Rich/consys"

# detect architecture
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

# get latest version tag from github
VERSION=$(curl -s "https://api.github.com/repos/$REPO/releases/latest" \
    | grep '"tag_name"' \
    | cut -d '"' -f 4)

if [ -z "$VERSION" ]; then
    echo "could not determine latest version"
    exit 1
fi

URL="https://github.com/$REPO/releases/download/$VERSION/consys-$TARGET"

echo "consys $VERSION ($TARGET)"
echo "installing to $INSTALL_DIR/$BINARY"

sudo curl -L "$URL" -o "$INSTALL_DIR/$BINARY"
sudo chmod +x "$INSTALL_DIR/$BINARY"

echo "done. run: consys"