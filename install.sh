#!/usr/bin/env bash
# Axon v6.0 Ultra-Lightweight Self-Hosted Toolchain Installer (Linux & macOS)
set -euo pipefail

AXON_VERSION="6.0.0"
AXON_ROOT="${HOME}/.axon"
INSTALL_BIN="${AXON_ROOT}/bin"
INSTALL_STD="${AXON_ROOT}/std"
AXC_BIN="${INSTALL_BIN}/axc"

echo "[*] Installing Axon v${AXON_VERSION} Self-Hosted Native Toolchain..."
mkdir -p "${INSTALL_BIN}" "${INSTALL_STD}"

if [ -f "./bin/axc" ]; then
    cp "./bin/axc" "${AXC_BIN}"
elif command -v cargo >/dev/null 2>&1; then
    echo "[*] Building Stage-0 native compiler from bootstrap_archive/axc_native..."
    cargo build --release --manifest-path bootstrap_archive/axc_native/Cargo.toml
    cp "bootstrap_archive/axc_native/target/release/axc" "${AXC_BIN}"
    cargo clean --manifest-path bootstrap_archive/axc_native/Cargo.toml
else
    echo "[-] ./bin/axc not found and cargo is unavailable to bootstrap Stage-0." >&2
    exit 1
fi

cp ./std/*.ax "${INSTALL_STD}/"
chmod +x "${AXC_BIN}"

if ! echo "${PATH}" | grep -q "${INSTALL_BIN}"; then
    SHELL_PROFILE="${HOME}/.bashrc"
    if [ -f "${HOME}/.zshrc" ]; then
        SHELL_PROFILE="${HOME}/.zshrc"
    fi
    echo "export PATH=\"${INSTALL_BIN}:\$PATH\"" >> "${SHELL_PROFILE}"
    echo "[+] Added ${INSTALL_BIN} to PATH in ${SHELL_PROFILE}"
fi

echo "[+] Installed Axon v${AXON_VERSION} compiler + stdlib to ${AXON_ROOT}"
"${AXC_BIN}" version
