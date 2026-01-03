#!/bin/bash
# Download fastcarve binary from SwiftBeaver releases

set -e

VERSION="v0.2.1"
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

# Map architecture
case "$ARCH" in
    x86_64)
        ARCH="x86_64"
        ;;
    aarch64|arm64)
        ARCH="aarch64"
        ;;
    *)
        echo "Unsupported architecture: $ARCH"
        exit 1
        ;;
esac

# Map OS
case "$OS" in
    linux)
        PLATFORM="unknown-linux-gnu"
        ;;
    darwin)
        PLATFORM="apple-darwin"
        ;;
    *)
        echo "Unsupported OS: $OS"
        exit 1
        ;;
esac

TARGET="${ARCH}-${PLATFORM}"
FILENAME="fastcarve-${VERSION}-${TARGET}.tar.gz"
URL="https://github.com/gaestu/SwiftBeaver/releases/download/${VERSION}/${FILENAME}"

echo "Downloading fastcarve ${VERSION} for ${TARGET}..."
echo "URL: ${URL}"

# Create bin directory
mkdir -p bin

# Download and extract
if command -v curl &> /dev/null; then
    curl -L -o /tmp/${FILENAME} "${URL}"
elif command -v wget &> /dev/null; then
    wget -O /tmp/${FILENAME} "${URL}"
else
    echo "Neither curl nor wget found. Please install one of them."
    exit 1
fi

# Extract
echo "Extracting..."
tar -xzf /tmp/${FILENAME} -C bin/

# Make executable
chmod +x bin/fastcarve

# Verify
echo "Verifying..."
./bin/fastcarve --version

echo ""
echo "✅ fastcarve ${VERSION} installed successfully to bin/fastcarve"
echo ""
echo "You can now run: cargo run"
