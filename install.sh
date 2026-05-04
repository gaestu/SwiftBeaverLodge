#!/usr/bin/env bash
# Install SwiftBeaverLodge from GitHub release artifacts.

set -euo pipefail

LODGE_REPO="gaestu/SwiftBeaverLodge"
SWIFTBEAVER_REPO="gaestu/SwiftBeaver"
LODGE_VERSION="${LODGE_VERSION:-latest}"
SWIFTBEAVER_VERSION="${SWIFTBEAVER_VERSION:-bundled}"
SWIFTBEAVER_FLAVOR="${SWIFTBEAVER_FLAVOR:-cpu-only}"

usage() {
    printf '%s\n' \
        "Usage: $0" \
        "" \
        "Downloads a SwiftBeaverLodge release bundle and installs:" \
        "  swiftbeaverlodge" \
        "  bin/swiftbeaver" \
        "" \
        "Environment:" \
        "  LODGE_VERSION          SwiftBeaverLodge release tag, or latest (default: latest)" \
        "  INSTALL_DIR            Install directory (default: /usr/local/bin when writable," \
        "                         otherwise \$HOME/.local/bin)" \
        "  SWIFTBEAVER_VERSION    Optional SwiftBeaver release tag to replace the bundled" \
        "                         engine (default: bundled)" \
        "  SWIFTBEAVER_FLAVOR     SwiftBeaver flavor for overrides: cpu-only, opencl, cuda" \
        "                         (default: cpu-only)" \
        "  EXPOSE_SWIFTBEAVER=1   Also create INSTALL_DIR/swiftbeaver when no file" \
        "                         already exists there" \
        "" \
        "Examples:" \
        "  ./install.sh" \
        "  LODGE_VERSION=v0.1.0 ./install.sh" \
        "  INSTALL_DIR=\"\$HOME/.local/bin\" ./install.sh" \
        "  SWIFTBEAVER_VERSION=v0.5.1 SWIFTBEAVER_FLAVOR=opencl ./install.sh"
}

case "${1:-}" in
    -h|--help)
        usage
        exit 0
        ;;
    "")
        ;;
    *)
        echo "Unsupported argument: $1" >&2
        usage >&2
        exit 1
        ;;
esac

case "${SWIFTBEAVER_FLAVOR}" in
    cpu-only|opencl|cuda)
        ;;
    *)
        echo "Unsupported SwiftBeaver flavor: ${SWIFTBEAVER_FLAVOR}" >&2
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
        echo "Unsupported OS for SwiftBeaverLodge release installer: ${OS}" >&2
        echo "No prebuilt SwiftBeaverLodge release asset is currently documented for this OS." >&2
        exit 1
        ;;
esac

case "${ARCH}" in
    x86_64|amd64)
        ARCH_NAME="x86_64"
        ;;
    *)
        echo "Unsupported architecture for SwiftBeaverLodge release installer: ${ARCH}" >&2
        echo "No prebuilt SwiftBeaverLodge release asset is currently documented for this architecture." >&2
        exit 1
        ;;
esac

choose_install_dir() {
    if [[ -n "${INSTALL_DIR:-}" ]]; then
        printf '%s\n' "${INSTALL_DIR}"
        return
    fi

    if [[ -d /usr/local/bin && -w /usr/local/bin ]]; then
        printf '%s\n' "/usr/local/bin"
    elif [[ ! -e /usr/local/bin && -w /usr/local ]]; then
        printf '%s\n' "/usr/local/bin"
    else
        printf '%s\n' "${HOME}/.local/bin"
    fi
}

INSTALL_DIR="$(choose_install_dir)"
TMP_DIR="$(mktemp -d)"

cleanup() {
    rm -rf "${TMP_DIR}"
}
trap cleanup EXIT

download_file() {
    local url="$1"
    local output="$2"

    if command -v curl >/dev/null 2>&1; then
        curl -fL --retry 3 -o "${output}" "${url}"
    elif command -v wget >/dev/null 2>&1; then
        wget -O "${output}" "${url}"
    else
        echo "Neither curl nor wget found. Please install one of them." >&2
        return 127
    fi
}

download_required() {
    local url="$1"
    local output="$2"

    if ! download_file "${url}" "${output}"; then
        echo "Failed to download: ${url}" >&2
        exit 1
    fi
}

release_base_url() {
    local repo="$1"
    local version="$2"

    if [[ "${version}" == "latest" ]]; then
        printf 'https://github.com/%s/releases/latest/download\n' "${repo}"
    else
        printf 'https://github.com/%s/releases/download/%s\n' "${repo}" "${version}"
    fi
}

verify_checksum() {
    local asset="$1"
    local checksums_url="$2"
    local checksums_file="$3"

    if ! command -v sha256sum >/dev/null 2>&1; then
        echo "sha256sum not found; cannot verify ${asset}." >&2
        exit 1
    fi

    if ! download_file "${checksums_url}" "${checksums_file}"; then
        echo "Could not download checksums for ${asset}: ${checksums_url}" >&2
        exit 1
    fi

    if grep -E "([[:space:]]|\\*)${asset}$" "${checksums_file}" > "${checksums_file}.selected"; then
        (cd "$(dirname "${checksums_file}")" && sha256sum -c "$(basename "${checksums_file}.selected")")
    else
        echo "${asset} not found in downloaded checksums." >&2
        exit 1
    fi
}

find_executable() {
    local root="$1"
    local name="$2"
    local found

    if [[ ! -d "${root}" ]]; then
        return 0
    fi

    found="$(find "${root}" -type f -name "${name}" -perm /111 | head -n 1 || true)"
    if [[ -z "${found}" ]]; then
        found="$(find "${root}" -type f -name "${name}" | head -n 1 || true)"
    fi

    printf '%s\n' "${found}"
}

run_version_check() {
    local label="$1"
    local binary="$2"
    local output

    if ! command -v timeout >/dev/null 2>&1; then
        echo "timeout not found; cannot safely verify ${label} --version." >&2
        exit 1
    fi

    if ! output="$(timeout 10s "${binary}" --version 2>&1)"; then
        echo "Installed ${label} failed --version verification." >&2
        echo "${output}" >&2
        exit 1
    fi

    printf '%s\n' "${output}"
}

LODGE_ASSET="swiftbeaverlodge-${OS_NAME}-${ARCH_NAME}.tar.gz"
LODGE_BASE_URL="$(release_base_url "${LODGE_REPO}" "${LODGE_VERSION}")"
LODGE_ARCHIVE="${TMP_DIR}/${LODGE_ASSET}"
LODGE_EXTRACT_DIR="${TMP_DIR}/lodge"

echo "SwiftBeaverLodge installer"
echo "  Lodge release:      ${LODGE_VERSION}"
echo "  Lodge package:      ${LODGE_ASSET}"
echo "  SwiftBeaver engine: ${SWIFTBEAVER_VERSION}"
echo "  Install directory:  ${INSTALL_DIR}"
echo

download_required "${LODGE_BASE_URL}/${LODGE_ASSET}" "${LODGE_ARCHIVE}"
verify_checksum "${LODGE_ASSET}" "${LODGE_BASE_URL}/checksums.txt" "${TMP_DIR}/lodge-checksums.txt"

mkdir -p "${LODGE_EXTRACT_DIR}"
tar -xzf "${LODGE_ARCHIVE}" -C "${LODGE_EXTRACT_DIR}"

LODGE_BINARY="$(find_executable "${LODGE_EXTRACT_DIR}" swiftbeaverlodge)"
if [[ -z "${LODGE_BINARY}" ]]; then
    echo "Archive did not contain a swiftbeaverlodge executable." >&2
    exit 1
fi

LODGE_ROOT="$(dirname "${LODGE_BINARY}")"
ENGINE_BINARY="$(find_executable "${LODGE_ROOT}/bin" swiftbeaver 2>/dev/null || true)"
if [[ -z "${ENGINE_BINARY}" ]]; then
    ENGINE_BINARY="$(find_executable "${LODGE_EXTRACT_DIR}" swiftbeaver)"
fi

if [[ -z "${ENGINE_BINARY}" ]]; then
    echo "Archive did not contain a bundled swiftbeaver executable." >&2
    exit 1
fi

if [[ "${SWIFTBEAVER_VERSION}" != "bundled" ]]; then
    SWIFTBEAVER_ASSET="swiftbeaver-${OS_NAME}-${ARCH_NAME}-${SWIFTBEAVER_FLAVOR}.tar.gz"
    SWIFTBEAVER_BASE_URL="$(release_base_url "${SWIFTBEAVER_REPO}" "${SWIFTBEAVER_VERSION}")"
    SWIFTBEAVER_ARCHIVE="${TMP_DIR}/${SWIFTBEAVER_ASSET}"
    SWIFTBEAVER_EXTRACT_DIR="${TMP_DIR}/swiftbeaver"

    echo
    echo "Replacing bundled engine with SwiftBeaver ${SWIFTBEAVER_VERSION} (${SWIFTBEAVER_FLAVOR})."
    download_required "${SWIFTBEAVER_BASE_URL}/${SWIFTBEAVER_ASSET}" "${SWIFTBEAVER_ARCHIVE}"
    verify_checksum "${SWIFTBEAVER_ASSET}" "${SWIFTBEAVER_BASE_URL}/SHA256SUMS" "${TMP_DIR}/swiftbeaver-checksums.txt"

    mkdir -p "${SWIFTBEAVER_EXTRACT_DIR}"
    tar -xzf "${SWIFTBEAVER_ARCHIVE}" -C "${SWIFTBEAVER_EXTRACT_DIR}"
    ENGINE_BINARY="$(find_executable "${SWIFTBEAVER_EXTRACT_DIR}" swiftbeaver)"

    if [[ -z "${ENGINE_BINARY}" ]]; then
        echo "SwiftBeaver archive did not contain a swiftbeaver executable." >&2
        exit 1
    fi
fi

if ! mkdir -p "${INSTALL_DIR}/bin"; then
    echo "Could not create install directory: ${INSTALL_DIR}" >&2
    exit 1
fi

if [[ ! -w "${INSTALL_DIR}" || ! -w "${INSTALL_DIR}/bin" ]]; then
    echo "Install directory is not writable: ${INSTALL_DIR}" >&2
    echo "Set INSTALL_DIR to a writable directory or rerun with appropriate permissions." >&2
    exit 1
fi

install -m 0755 "${LODGE_BINARY}" "${INSTALL_DIR}/swiftbeaverlodge"
install -m 0755 "${ENGINE_BINARY}" "${INSTALL_DIR}/bin/swiftbeaver"

if [[ "${EXPOSE_SWIFTBEAVER:-0}" == "1" ]]; then
    if [[ -e "${INSTALL_DIR}/swiftbeaver" || -L "${INSTALL_DIR}/swiftbeaver" ]]; then
        echo "Refusing to overwrite existing ${INSTALL_DIR}/swiftbeaver." >&2
        echo "The GUI will still use ${INSTALL_DIR}/bin/swiftbeaver." >&2
    elif ! ln -s "bin/swiftbeaver" "${INSTALL_DIR}/swiftbeaver"; then
        echo "Could not create ${INSTALL_DIR}/swiftbeaver symlink." >&2
        exit 1
    fi
fi

echo
echo "Installed SwiftBeaverLodge: ${INSTALL_DIR}/swiftbeaverlodge"
echo "Installed SwiftBeaver:      ${INSTALL_DIR}/bin/swiftbeaver"

LODGE_VERSION_OUTPUT="$(run_version_check swiftbeaverlodge "${INSTALL_DIR}/swiftbeaverlodge")"
echo "Verified SwiftBeaverLodge: ${LODGE_VERSION_OUTPUT}"

SWIFTBEAVER_VERSION_OUTPUT="$(run_version_check swiftbeaver "${INSTALL_DIR}/bin/swiftbeaver")"
echo "Verified SwiftBeaver:      ${SWIFTBEAVER_VERSION_OUTPUT}"

case ":${PATH}:" in
    *:"${INSTALL_DIR}":*)
        ;;
    *)
        echo
        echo "Note: ${INSTALL_DIR} is not currently on PATH. Add it to run swiftbeaverlodge from any directory."
        ;;
esac

echo
echo "SwiftBeaverLodge will discover the installed engine via ${INSTALL_DIR}/bin/swiftbeaver."
