#!/usr/bin/env bash
set -euo pipefail

REPO="tk-aria/netcap"
BINARY_NAME="netcap"
INSTALL_DIR="${NETCAP_INSTALL_PATH:-/usr/local/bin}"
VERSION="${NETCAP_VERSION:-latest}"

# --- Colors ---
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info() { echo -e "${GREEN}[INFO]${NC} $*"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $*"; }
error() { echo -e "${RED}[ERROR]${NC} $*" >&2; }

# --- OS/Arch 検出 ---
detect_os() {
    case "$(uname -s)" in
        Linux*)   echo "linux" ;;
        Darwin*)  echo "darwin" ;;
        MINGW*|MSYS*|CYGWIN*) echo "windows" ;;
        *)        echo "unknown" ;;
    esac
}

detect_arch() {
    case "$(uname -m)" in
        x86_64|amd64) echo "x86_64" ;;
        aarch64|arm64) echo "aarch64" ;;
        *)             echo "unknown" ;;
    esac
}

# --- バージョン取得 ---
get_latest_version() {
    local version
    version=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" 2>/dev/null \
        | grep '"tag_name"' | sed -E 's/.*"v?([^"]+)".*/\1/')
    if [ -z "$version" ]; then
        error "Failed to fetch latest version from GitHub"
        exit 1
    fi
    echo "$version"
}

# --- インストール ---
cmd_install() {
    local os arch version
    os=$(detect_os)
    arch=$(detect_arch)
    version="${VERSION}"

    if [ "$os" = "unknown" ]; then
        error "Unsupported OS: $(uname -s)"
        exit 1
    fi

    if [ "$arch" = "unknown" ]; then
        error "Unsupported architecture: $(uname -m)"
        exit 1
    fi

    if [ "${version}" = "latest" ]; then
        info "Fetching latest version..."
        version=$(get_latest_version)
    fi

    info "Installing ${BINARY_NAME} v${version} (${os}/${arch})..."

    local filename="${BINARY_NAME}-${arch}-${os}"
    [ "${os}" = "windows" ] && filename="${filename}.exe"
    local url="https://github.com/${REPO}/releases/download/v${version}/${filename}.tar.gz"

    local tmp
    tmp=$(mktemp -d)
    trap "rm -rf ${tmp}" EXIT

    info "Downloading from ${url}..."
    if ! curl -fsSL "${url}" -o "${tmp}/archive.tar.gz"; then
        error "Download failed. Check if version v${version} exists."
        error "URL: ${url}"
        exit 1
    fi

    info "Extracting..."
    tar xzf "${tmp}/archive.tar.gz" -C "${tmp}"

    if install -m 755 "${tmp}/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}" 2>/dev/null; then
        info "Installed to ${INSTALL_DIR}/${BINARY_NAME}"
    else
        warn "Permission denied for ${INSTALL_DIR}, falling back to ~/.local/bin"
        local fallback="${HOME}/.local/bin"
        mkdir -p "${fallback}"
        install -m 755 "${tmp}/${BINARY_NAME}" "${fallback}/${BINARY_NAME}"
        info "Installed to ${fallback}/${BINARY_NAME}"

        # Check if fallback is in PATH
        if ! echo "$PATH" | tr ':' '\n' | grep -q "^${fallback}$"; then
            warn "Add ${fallback} to your PATH:"
            warn "  export PATH=\"${fallback}:\$PATH\""
        fi
    fi

    info "${BINARY_NAME} v${version} installed successfully!"
}

# --- アンインストール ---
cmd_uninstall() {
    local removed=false
    local targets=(
        "${INSTALL_DIR}/${BINARY_NAME}"
        "${HOME}/.local/bin/${BINARY_NAME}"
    )
    for path in "${targets[@]}"; do
        if [ -f "${path}" ]; then
            rm -f "${path}"
            info "Removed: ${path}"
            removed=true
        fi
    done

    if [ "$removed" = false ]; then
        warn "${BINARY_NAME} not found in expected locations"
    else
        info "${BINARY_NAME} uninstalled successfully."
    fi
}

# --- バージョン表示 ---
cmd_version() {
    if command -v "${BINARY_NAME}" &>/dev/null; then
        ${BINARY_NAME} --version
    else
        error "${BINARY_NAME} is not installed."
        exit 1
    fi
}

# --- ヘルプ ---
cmd_help() {
    cat <<HELP
${BINARY_NAME} installer

Usage: $0 {install|uninstall|version|help}

Commands:
  install     Download and install ${BINARY_NAME}
  uninstall   Remove ${BINARY_NAME}
  version     Show installed version
  help        Show this help

Environment variables:
  NETCAP_VERSION        Version to install (default: latest)
  NETCAP_INSTALL_PATH   Installation directory (default: /usr/local/bin)

Examples:
  # Install latest version
  $0 install

  # Install specific version
  NETCAP_VERSION=0.1.0 $0 install

  # Install to custom directory
  NETCAP_INSTALL_PATH=/opt/bin $0 install

  # Pipe install
  curl -fsSL https://raw.githubusercontent.com/${REPO}/main/scripts/setup.sh | sh -s -- install
HELP
}

# --- メイン ---
case "${1:-install}" in
    install)   cmd_install ;;
    uninstall) cmd_uninstall ;;
    version)   cmd_version ;;
    help|-h|--help) cmd_help ;;
    *)
        error "Unknown command: $1"
        cmd_help
        exit 1
        ;;
esac
