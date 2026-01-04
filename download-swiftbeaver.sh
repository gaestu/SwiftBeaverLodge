#!/bin/bash
# Download swiftbeaver binary from SwiftBeaver releases
# Supports multiple GPU variants: cpu-only, opencl, cuda

set -e

VERSION="v0.3.0"
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

# Default variant (can be overridden via argument)
VARIANT="${1:-cpu-only}"

# Validate variant
case "$VARIANT" in
    cpu-only|opencl|cuda)
        ;;
    *)
        echo "Invalid variant: $VARIANT"
        echo "Usage: $0 [cpu-only|opencl|cuda]"
        echo ""
        echo "Available variants:"
        echo "  cpu-only  - CPU-only build, no GPU support (default)"
        echo "  opencl    - OpenCL GPU support (NVIDIA/AMD/Intel)"
        echo "  cuda      - CUDA GPU support (NVIDIA only, best performance)"
        exit 1
        ;;
esac

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

# Map OS to filename format
case "$OS" in
    linux)
        OS_NAME="linux"
        ;;
    darwin)
        OS_NAME="macos"
        ;;
    *)
        echo "Unsupported OS: $OS"
        exit 1
        ;;
esac

# GitHub release format: swiftbeaver-linux-x86_64-<variant>.tar.gz
FILENAME="swiftbeaver-${OS_NAME}-${ARCH}-${VARIANT}.tar.gz"
URL="https://github.com/gaestu/SwiftBeaver/releases/download/${VERSION}/${FILENAME}"

echo "╔════════════════════════════════════════════════════════╗"
echo "║           SwiftBeaver Binary Downloader                ║"
echo "╠════════════════════════════════════════════════════════╣"
echo "║  Version:  ${VERSION}                                       ║"
echo "║  Variant:  ${VARIANT}$(printf '%*s' $((15 - ${#VARIANT})) '')                            ║"
echo "║  Platform: ${OS_NAME}-${ARCH}$(printf '%*s' $((10 - ${#OS_NAME})) '')                        ║"
echo "╚════════════════════════════════════════════════════════╝"
echo ""
echo "URL: ${URL}"
echo ""

# Create bin directory
mkdir -p bin

# Download
echo "📥 Downloading..."
if command -v curl &> /dev/null; then
    curl -L -o /tmp/${FILENAME} "${URL}"
elif command -v wget &> /dev/null; then
    wget -O /tmp/${FILENAME} "${URL}"
else
    echo "Neither curl nor wget found. Please install one of them."
    exit 1
fi

# Extract
echo "📦 Extracting..."
tar -xzf /tmp/${FILENAME} -C bin/

# Make executable
chmod +x bin/swiftbeaver

# Store variant info
echo "${VARIANT}" > bin/.variant

# Verify
echo "✅ Verifying..."
./bin/swiftbeaver --version

echo ""
echo "╔════════════════════════════════════════════════════════╗"
echo "║  ✅ swiftbeaver ${VERSION} (${VARIANT}) installed!              ║"
echo "║                                                        ║"
echo "║  Binary location: bin/swiftbeaver                      ║"
echo "║                                                        ║"
echo "║  You can now run: cargo run                            ║"
echo "╚════════════════════════════════════════════════════════╝"

# Cleanup
rm -f /tmp/${FILENAME}

# GPU-specific hints
if [ "$VARIANT" == "opencl" ]; then
    echo ""
    echo "💡 OpenCL variant requires:"
    echo "   - OpenCL runtime installed (ocl-icd-opencl-dev)"
    echo "   - Use --gpu flag to enable GPU acceleration"
elif [ "$VARIANT" == "cuda" ]; then
    echo ""
    echo "💡 CUDA variant requires:"
    echo "   - NVIDIA GPU with CUDA 12.x"
    echo "   - Use --gpu flag to enable GPU acceleration"
fi
