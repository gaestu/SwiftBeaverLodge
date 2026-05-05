#!/usr/bin/env bash
# Convenience installer for the SwiftBeaver CLI used by SwiftBeaverLodge.
#
# SwiftBeaverLodge discovers a single executable named `swiftbeaver`. Upstream
# SwiftBeaver release archives are packaged by build flavor, but
# this helper installs the chosen archive as ./bin/swiftbeaver.

set -euo pipefail

VERSION="${SWIFTBEAVER_VERSION:-v0.6.7}"
FLAVOR="${1:-cpu-only}"
REPO="gaestu/SwiftBeaver"

usage() {
    cat <<USAGE
Usage: $0 [cpu-only|opencl|cuda]

Downloads SwiftBeaver ${VERSION} for Linux x86_64 and installs it as:
  ./bin/swiftbeaver

Environment:
  SWIFTBEAVER_VERSION   Release tag to download (default: v0.6.7)

Notes:
  SwiftBeaverLodge requires swiftbeaver v0.6.7+ and only discovers a binary
  named "swiftbeaver". GPU acceleration is enabled in Lodge with SwiftBeaver's
  --gpu flag; Lodge no longer selects swiftbeaver-<flavor> binaries.
USAGE
}

case "${FLAVOR}" in
    cpu-only|opencl|cuda)
        ;;
    -h|--help)
        usage
        exit 0
        ;;
    *)
        echo "Unsupported SwiftBeaver release flavor: ${FLAVOR}" >&2
        usage >&2
        exit 1
        ;;
esac

OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"

case "${OS}" in
    linux)
        OS_NAME="linux"
        ;;
    *)
        echo "No ${VERSION} prebuilt SwiftBeaver artifact is documented for OS: ${OS}" >&2
        echo "Install swiftbeaver v0.6.7+ on PATH manually, or place it at ./bin/swiftbeaver." >&2
        exit 1
        ;;
esac

case "${ARCH}" in
    x86_64|amd64)
        ARCH_NAME="x86_64"
        ;;
    *)
        echo "No ${VERSION} prebuilt SwiftBeaver artifact is documented for architecture: ${ARCH}" >&2
        echo "Install swiftbeaver v0.6.7+ on PATH manually, or place it at ./bin/swiftbeaver." >&2
        exit 1
        ;;
esac

FILENAME="swiftbeaver-${OS_NAME}-${ARCH_NAME}-${FLAVOR}.tar.gz"
BASE_URL="https://github.com/${REPO}/releases/download/${VERSION}"
URL="${BASE_URL}/${FILENAME}"
SUMS_URL="${BASE_URL}/SHA256SUMS"
TMP_DIR="$(mktemp -d)"

cleanup() {
    rm -rf "${TMP_DIR}"
}
trap cleanup EXIT

download() {
    local url="$1"
    local output="$2"

    if command -v curl >/dev/null 2>&1; then
        curl -fL --retry 3 -o "${output}" "${url}"
    elif command -v wget >/dev/null 2>&1; then
        wget -O "${output}" "${url}"
    else
        echo "Neither curl nor wget found. Please install one of them." >&2
        exit 1
    fi
}

echo "SwiftBeaver CLI installer"
echo "  Release: ${VERSION}"
echo "  Package: ${FILENAME}"
echo "  Target:  ./bin/swiftbeaver"
echo

download "${URL}" "${TMP_DIR}/${FILENAME}"

if command -v sha256sum >/dev/null 2>&1; then
    if download "${SUMS_URL}" "${TMP_DIR}/SHA256SUMS"; then
        if grep -E "[[:space:]]${FILENAME}$" "${TMP_DIR}/SHA256SUMS" >"${TMP_DIR}/SHA256SUMS.selected"; then
            (cd "${TMP_DIR}" && sha256sum -c SHA256SUMS.selected)
        else
            echo "Warning: ${FILENAME} not found in SHA256SUMS; skipping checksum verification." >&2
        fi
    else
        echo "Warning: could not download SHA256SUMS; skipping checksum verification." >&2
    fi
else
    echo "Warning: sha256sum not found; skipping checksum verification." >&2
fi

mkdir -p "${TMP_DIR}/extract" bin
tar -xzf "${TMP_DIR}/${FILENAME}" -C "${TMP_DIR}/extract"

BINARY_PATH="$(find "${TMP_DIR}/extract" -type f -name swiftbeaver -perm /111 | head -n 1)"
if [[ -z "${BINARY_PATH}" ]]; then
    BINARY_PATH="$(find "${TMP_DIR}/extract" -type f -name swiftbeaver | head -n 1)"
fi

if [[ -z "${BINARY_PATH}" ]]; then
    echo "Archive did not contain a swiftbeaver executable." >&2
    exit 1
fi

cp "${BINARY_PATH}" bin/swiftbeaver
chmod 0755 bin/swiftbeaver

echo
echo "Installed ./bin/swiftbeaver"
./bin/swiftbeaver --version
echo
echo "SwiftBeaverLodge will discover this binary when run from the repository root."
