#!/bin/bash
# Download swiftbeaver binaries from SwiftBeaver releases
# Downloads all GPU variants: cpu-only, opencl, cuda

set -e

VERSION="v0.3.0"
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

# All variants to download
VARIANTS=("cpu-only" "opencl" "cuda")

# Check if user wants only specific variant
if [ -n "$1" ]; then
    case "$1" in
        cpu-only|opencl|cuda)
            VARIANTS=("$1")
            ;;
        all)
            # Keep all variants
            ;;
        *)
            echo "Invalid variant: $1"
            echo "Usage: $0 [cpu-only|opencl|cuda|all]"
            echo ""
            echo "Available variants:"
            echo "  cpu-only  - CPU-only build, no GPU support"
            echo "  opencl    - OpenCL GPU support (NVIDIA/AMD/Intel)"
            echo "  cuda      - CUDA GPU support (NVIDIA only, best performance)"
            echo "  all       - Download all variants (default)"
            exit 1
            ;;
    esac
fi

echo "╔════════════════════════════════════════════════════════╗"
echo "║           SwiftBeaver Binary Downloader                ║"
echo "╠════════════════════════════════════════════════════════╣"
echo "║  Version:  ${VERSION}                                       ║"
echo "║  Platform: ${OS_NAME}-${ARCH}$(printf '%*s' $((10 - ${#OS_NAME})) '')                        ║"
echo "║  Variants: ${VARIANTS[*]}$(printf '%*s' $((24 - ${#VARIANTS[*]})) '')              ║"
echo "╚════════════════════════════════════════════════════════╝"
echo ""

# Create bin directory
mkdir -p bin

# Download each variant
for VARIANT in "${VARIANTS[@]}"; do
    FILENAME="swiftbeaver-${OS_NAME}-${ARCH}-${VARIANT}.tar.gz"
    URL="https://github.com/gaestu/SwiftBeaver/releases/download/${VERSION}/${FILENAME}"
    BINARY_NAME="swiftbeaver-${VARIANT}"
    
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "📥 Downloading ${VARIANT} variant..."
    echo "   URL: ${URL}"
    
    # Download
    if command -v curl &> /dev/null; then
        curl -L -o /tmp/${FILENAME} "${URL}" 2>&1
    elif command -v wget &> /dev/null; then
        wget -O /tmp/${FILENAME} "${URL}" 2>&1
    else
        echo "Neither curl nor wget found. Please install one of them."
        exit 1
    fi
    
    # Extract to temp, then rename
    echo "📦 Extracting..."
    tar -xzf /tmp/${FILENAME} -C /tmp/
    mv /tmp/swiftbeaver bin/${BINARY_NAME}
    chmod +x bin/${BINARY_NAME}
    
    # Verify
    echo "✅ Verifying ${BINARY_NAME}..."
    ./bin/${BINARY_NAME} --version
    
    # Cleanup
    rm -f /tmp/${FILENAME}
    echo ""
done

# Create default symlink to cpu-only (safest default)
if [ -f "bin/swiftbeaver-cpu-only" ]; then
    ln -sf swiftbeaver-cpu-only bin/swiftbeaver
    echo "🔗 Created symlink: swiftbeaver → swiftbeaver-cpu-only"
fi

# List installed variants
echo ""
echo "╔════════════════════════════════════════════════════════╗"
echo "║  ✅ Installation Complete!                             ║"
echo "╠════════════════════════════════════════════════════════╣"
echo "║  Installed binaries:                                   ║"
for VARIANT in "${VARIANTS[@]}"; do
    printf "║    • bin/swiftbeaver-%-10s                       ║\n" "${VARIANT}"
done
echo "║                                                        ║"
echo "║  Select variant in SwiftBeaverLodge UI or run:         ║"
echo "║    ./bin/swiftbeaver-<variant> --version               ║"
echo "║                                                        ║"
echo "║  You can now run: cargo run                            ║"
echo "╚════════════════════════════════════════════════════════╝"

# GPU-specific hints
echo ""
echo "💡 GPU Requirements:"
echo "   • opencl: Install OpenCL runtime (ocl-icd-opencl-dev)"
echo "   • cuda:   NVIDIA GPU with CUDA 12.x runtime"
echo "   Use --gpu flag to enable GPU acceleration"
