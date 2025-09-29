#!/bin/bash

# CacheKill Install Script
# This script downloads and installs the latest CacheKill release

set -e

# Configuration
REPO="kagehq/cache-kill"
BINARY_NAME="cachekill"
INSTALL_DIR="${HOME}/.local/bin"
BIN_DIR="${HOME}/.local/bin"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Detect OS and architecture
detect_platform() {
    local os arch
    
    case "$(uname -s)" in
        Linux*)     os="linux" ;;
        Darwin*)    os="darwin" ;;
        CYGWIN*|MINGW*|MSYS*) os="windows" ;;
        *)          os="unknown" ;;
    esac
    
    case "$(uname -m)" in
        x86_64|amd64) arch="x86_64" ;;
        arm64|aarch64) arch="aarch64" ;;
        armv7l) arch="armv7" ;;
        *) arch="unknown" ;;
    esac
    
    echo "${arch}-${os}"
}

# Get the latest release version
get_latest_version() {
    local api_url="https://api.github.com/repos/${REPO}/releases/latest"
    
    if command -v curl >/dev/null 2>&1; then
        curl -s "${api_url}" | grep '"tag_name"' | sed 's/.*"tag_name": "\(.*\)".*/\1/'
    elif command -v wget >/dev/null 2>&1; then
        wget -qO- "${api_url}" | grep '"tag_name"' | sed 's/.*"tag_name": "\(.*\)".*/\1/'
    else
        log_error "Neither curl nor wget is available. Please install one of them."
        exit 1
    fi
}

# Download and install binary
install_binary() {
    local version="$1"
    local platform="$2"
    local download_url
    local temp_dir
    local binary_path
    
    # Determine the correct asset name
    local asset_name
    case "${platform}" in
        *linux*) asset_name="cachekill-${platform}.tar.gz" ;;
        *darwin*) asset_name="cachekill-${platform}.tar.gz" ;;
        *windows*) asset_name="cachekill-${platform}.zip" ;;
        *) log_error "Unsupported platform: ${platform}"; exit 1 ;;
    esac
    
    download_url="https://github.com/${REPO}/releases/download/${version}/${asset_name}"
    temp_dir=$(mktemp -d)
    
    log_info "Downloading CacheKill ${version} for ${platform}..."
    log_info "URL: ${download_url}"
    
    # Download the release
    if command -v curl >/dev/null 2>&1; then
        if ! curl -L -o "${temp_dir}/${asset_name}" "${download_url}"; then
            log_error "Failed to download ${asset_name}"
            exit 1
        fi
    elif command -v wget >/dev/null 2>&1; then
        if ! wget -O "${temp_dir}/${asset_name}" "${download_url}"; then
            log_error "Failed to download ${asset_name}"
            exit 1
        fi
    fi
    
    # Extract the archive
    log_info "Extracting archive..."
    cd "${temp_dir}"
    
    if [[ "${asset_name}" == *.tar.gz ]]; then
        tar -xzf "${asset_name}"
    elif [[ "${asset_name}" == *.zip ]]; then
        unzip -q "${asset_name}"
    fi
    
    # Find the binary
    if [[ "${platform}" == *windows* ]]; then
        binary_path=$(find . -name "cachekill.exe" | head -1)
    else
        binary_path=$(find . -name "cachekill" | head -1)
    fi
    
    if [[ -z "${binary_path}" ]]; then
        log_error "Binary not found in downloaded archive"
        exit 1
    fi
    
    # Create install directory
    mkdir -p "${INSTALL_DIR}"
    
    # Install the binary
    log_info "Installing to ${INSTALL_DIR}..."
    cp "${binary_path}" "${INSTALL_DIR}/${BINARY_NAME}"
    chmod +x "${INSTALL_DIR}/${BINARY_NAME}"
    
    # Clean up
    rm -rf "${temp_dir}"
    
    log_success "CacheKill ${version} installed successfully!"
}

# Check if binary is already installed
check_existing_installation() {
    if command -v "${BINARY_NAME}" >/dev/null 2>&1; then
        local current_version
        current_version=$("${BINARY_NAME}" --version 2>/dev/null | head -1 | grep -o '[0-9]\+\.[0-9]\+\.[0-9]\+' || echo "unknown")
        log_warning "CacheKill is already installed (version: ${current_version})"
        
        if [[ "${1:-}" != "--force" ]]; then
            read -p "Do you want to update to the latest version? [y/N]: " -n 1 -r
            echo
            if [[ ! $REPLY =~ ^[Yy]$ ]]; then
                log_info "Installation cancelled."
                exit 0
            fi
        fi
    fi
}

# Add to PATH if needed
setup_path() {
    local shell_rc
    
    # Detect shell
    case "${SHELL}" in
        */bash) shell_rc="${HOME}/.bashrc" ;;
        */zsh) shell_rc="${HOME}/.zshrc" ;;
        */fish) shell_rc="${HOME}/.config/fish/config.fish" ;;
        *) shell_rc="" ;;
    esac
    
    # Check if INSTALL_DIR is in PATH
    if [[ ":${PATH}:" != *":${INSTALL_DIR}:"* ]]; then
        log_warning "${INSTALL_DIR} is not in your PATH"
        
        if [[ -n "${shell_rc}" ]]; then
            log_info "Adding ${INSTALL_DIR} to PATH in ${shell_rc}"
            echo "" >> "${shell_rc}"
            echo "# CacheKill" >> "${shell_rc}"
            echo "export PATH=\"${INSTALL_DIR}:\$PATH\"" >> "${shell_rc}"
            log_success "PATH updated. Please restart your shell or run: source ${shell_rc}"
        else
            log_warning "Please add ${INSTALL_DIR} to your PATH manually"
        fi
    else
        log_success "PATH is already configured correctly"
    fi
}

# Main installation function
main() {
    local platform version
    
    log_info "CacheKill Installer"
    log_info "=================="
    
    # Detect platform
    platform=$(detect_platform)
    if [[ "${platform}" == *"unknown"* ]]; then
        log_error "Unsupported platform: $(uname -s) $(uname -m)"
        exit 1
    fi
    
    log_info "Detected platform: ${platform}"
    
    # Check existing installation
    check_existing_installation "$@"
    
    # Get latest version
    log_info "Fetching latest version..."
    version=$(get_latest_version)
    if [[ -z "${version}" ]]; then
        log_error "Failed to get latest version"
        exit 1
    fi
    
    log_info "Latest version: ${version}"
    
    # Install binary
    install_binary "${version}" "${platform}"
    
    # Setup PATH
    setup_path
    
    # Verify installation
    if command -v "${BINARY_NAME}" >/dev/null 2>&1; then
        log_success "Installation completed successfully!"
        log_info "Run '${BINARY_NAME} --help' to get started"
    else
        log_warning "Installation completed, but ${BINARY_NAME} is not in PATH"
        log_info "Please restart your shell or run: source ~/.bashrc (or ~/.zshrc)"
    fi
}

# Run main function with all arguments
main "$@"
