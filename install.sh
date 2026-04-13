#!/usr/bin/env bash
set -euo pipefail

# Harness CLI installer
# Usage: curl -fsSL https://raw.githubusercontent.com/david1996yong-design/harness-cli-releases/master/install.sh | bash

REPO="david1996yong-design/harness-cli-releases"
BINARY="harness-cli"
INSTALL_DIR="${HARNESS_INSTALL_DIR:-$HOME/.local/bin}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
NC='\033[0m'

info()  { printf "${CYAN}info${NC}  %s\n" "$1"; }
ok()    { printf "${GREEN}ok${NC}    %s\n" "$1"; }
warn()  { printf "${YELLOW}warn${NC}  %s\n" "$1"; }
error() { printf "${RED}error${NC} %s\n" "$1" >&2; exit 1; }

# Detect OS
detect_os() {
    case "$(uname -s)" in
        Linux*)  echo "linux" ;;
        Darwin*) echo "macos" ;;
        MINGW*|MSYS*|CYGWIN*) echo "windows" ;;
        *) error "Unsupported OS: $(uname -s)" ;;
    esac
}

# Detect architecture
detect_arch() {
    case "$(uname -m)" in
        x86_64|amd64)  echo "x86_64" ;;
        aarch64|arm64) echo "aarch64" ;;
        *) error "Unsupported architecture: $(uname -m)" ;;
    esac
}

# Map OS/arch to Rust target triple
get_target() {
    local os="$1" arch="$2"
    case "${os}-${arch}" in
        linux-x86_64)   echo "x86_64-unknown-linux-gnu" ;;
        linux-aarch64)  echo "aarch64-unknown-linux-gnu" ;;
        macos-x86_64)   echo "x86_64-apple-darwin" ;;
        macos-aarch64)  echo "aarch64-apple-darwin" ;;
        windows-x86_64) echo "x86_64-pc-windows-msvc" ;;
        *) error "No prebuilt binary for ${os}-${arch}" ;;
    esac
}

# Get latest release tag from GitHub API
get_latest_version() {
    local url="https://api.github.com/repos/${REPO}/releases/latest"
    if command -v curl &>/dev/null; then
        curl -fsSL "$url" | grep '"tag_name"' | head -1 | sed 's/.*"tag_name": *"//;s/".*//'
    elif command -v wget &>/dev/null; then
        wget -qO- "$url" | grep '"tag_name"' | head -1 | sed 's/.*"tag_name": *"//;s/".*//'
    else
        error "curl or wget is required"
    fi
}

# Download file
download() {
    local url="$1" dest="$2"
    if command -v curl &>/dev/null; then
        curl -fsSL "$url" -o "$dest"
    elif command -v wget &>/dev/null; then
        wget -qO "$dest" "$url"
    fi
}

main() {
    local os arch target version ext

    os=$(detect_os)
    arch=$(detect_arch)
    target=$(get_target "$os" "$arch")

    info "Detected platform: ${os}/${arch} -> ${target}"

    # Get version (from argument or latest release)
    version="${1:-$(get_latest_version)}"
    if [ -z "$version" ]; then
        error "Could not determine latest version. Check https://github.com/${REPO}/releases"
    fi
    info "Installing harness-cli ${version}"

    # Determine archive extension
    if [ "$os" = "windows" ]; then
        ext="zip"
    else
        ext="tar.gz"
    fi

    local archive="${BINARY}-${target}.${ext}"
    local url="https://github.com/${REPO}/releases/download/${version}/${archive}"
    local tmpdir
    tmpdir=$(mktemp -d)
    trap 'rm -rf "$tmpdir"' EXIT

    # Download
    info "Downloading ${url}"
    download "$url" "${tmpdir}/${archive}"

    # Verify checksum if available
    local sha_url="${url}.sha256"
    if download "$sha_url" "${tmpdir}/${archive}.sha256" 2>/dev/null; then
        info "Verifying checksum..."
        (cd "$tmpdir" && sha256sum -c "${archive}.sha256")
        ok "Checksum verified"
    else
        warn "Checksum file not available, skipping verification"
    fi

    # Extract
    info "Extracting..."
    if [ "$ext" = "tar.gz" ]; then
        tar xzf "${tmpdir}/${archive}" -C "$tmpdir"
    else
        unzip -q "${tmpdir}/${archive}" -d "$tmpdir"
    fi

    # Install
    mkdir -p "$INSTALL_DIR"
    mv "${tmpdir}/${BINARY}" "${INSTALL_DIR}/${BINARY}"
    chmod +x "${INSTALL_DIR}/${BINARY}"
    ok "Installed to ${INSTALL_DIR}/${BINARY}"

    # Check if INSTALL_DIR is in PATH
    if ! echo "$PATH" | tr ':' '\n' | grep -qx "$INSTALL_DIR"; then
        warn "${INSTALL_DIR} is not in your PATH"
        echo ""
        echo "  Add it to your shell profile:"
        echo ""
        echo "    export PATH=\"${INSTALL_DIR}:\$PATH\""
        echo ""
    fi

    # Verify installation
    if command -v "$BINARY" &>/dev/null; then
        ok "$(${BINARY} --version)"
    else
        ok "Installation complete. Restart your shell or update PATH to use '${BINARY}'"
    fi
}

main "$@"
