#!/bin/bash
# Download SwiftBeaver/fastcarve binary for development
# Usage: ./download-fastcarve.sh [version]

set -e

VERSION="${1:-v0.2.1}"
RELEASE_URL="https://github.com/gaestu/SwiftBeaver/releases/download/${VERSION}"
BIN_DIR="$(dirname "$0")/bin"

mkdir -p "$BIN_DIR"

# Detect platform
case "$(uname -s)" in
    Linux)
        ARCHIVE="fastcarve-linux-x86_64.tar.gz"
        ;;
    Darwin)
        echo "macOS builds not yet available. Please build from source."
        exit 1
        ;;
    MINGW*|MSYS*|CYGWIN*)
        echo "Windows builds not yet available. Please build from source."
        exit 1
        ;;
    *)
        echo "Unsupported platform: $(uname -s)"
        exit 1
        ;;
esac

echo "Downloading fastcarve ${VERSION} for $(uname -s)..."

DOWNLOAD_URL="${RELEASE_URL}/${ARCHIVE}"
TEMP_FILE="/tmp/${ARCHIVE}"

# Download
curl -L -o "$TEMP_FILE" "$DOWNLOAD_URL"

# Extract
echo "Extracting to ${BIN_DIR}..."
tar -xzf "$TEMP_FILE" -C "$BIN_DIR"

# Make executable
chmod +x "${BIN_DIR}/fastcarve"

# Clean up
rm "$TEMP_FILE"

# Verify
echo ""
echo "Installed fastcarve:"
"${BIN_DIR}/fastcarve" --version

echo ""
echo "Done! fastcarve is ready at: ${BIN_DIR}/fastcarve"
