#!/usr/bin/env bash
# Axon v5.0 Standalone Toolchain Installer (Linux & macOS)
set -euo pipefail

AXON_VERSION="5.0.0"
INSTALL_DIR="${HOME}/.axon/bin"
AXC_BIN="${INSTALL_DIR}/axc"

echo "[*] Installing Axon v${AXON_VERSION} Native Compiler..."
mkdir -p "${INSTALL_DIR}"

if [ -f "./bin/axc" ]; then
    cp "./bin/axc" "${AXC_BIN}"
elif command -v cargo >/dev/null 2>&1; then
    echo "[*] Building Stage-0 native compiler from bootstrap_archive/axc_native..."
    cargo build --release --manifest-path bootstrap_archive/axc_native/Cargo.toml
    cp "bootstrap_archive/axc_native/target/release/axc" "${AXC_BIN}"
else
    echo "[-] ./bin/axc not found and cargo is unavailable to bootstrap Stage-0." >&2
    exit 1
fi

chmod +x "${AXC_BIN}"

if ! echo "${PATH}" | grep -q "${INSTALL_DIR}"; then
    SHELL_PROFILE="${HOME}/.bashrc"
    if [ -f "${HOME}/.zshrc" ]; then
        SHELL_PROFILE="${HOME}/.zshrc"
    fi
    echo "export PATH=\"${INSTALL_DIR}:\$PATH\"" >> "${SHELL_PROFILE}"
    echo "[+] Added ${INSTALL_DIR} to PATH in ${SHELL_PROFILE}"
fi

echo "[+] Installed Axon compiler to ${AXC_BIN}"
"${AXC_BIN}" version
