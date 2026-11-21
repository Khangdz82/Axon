#!/usr/bin/env bash
# Axon v1.0.0 Self-Hosted Multi-OS Native Toolchain Installer (Linux & macOS)
set -euo pipefail

AXON_VERSION="1.0.0"
AXON_ROOT="${HOME}/.axon"
INSTALL_BIN="${AXON_ROOT}/bin"
INSTALL_STD="${AXON_ROOT}/std"
AXC_BIN="${INSTALL_BIN}/axc"

if [ "${1:-}" = "--uninstall" ]; then
    rm -rf "${AXON_ROOT}"
    echo "[+] Uninstalled Axon v${AXON_VERSION} from ${AXON_ROOT}"
    exit 0
fi

echo "[*] Installing Axon v${AXON_VERSION} Self-Hosted Native Toolchain..."
mkdir -p "${INSTALL_BIN}" "${INSTALL_STD}"

OS_NAME="$(uname -s)"
ARCH_NAME="$(uname -m)"

if [ -f "./bin/axc" ]; then
    cp "./bin/axc" "${AXC_BIN}"
elif [ "${OS_NAME}" = "Linux" ] && [ "${ARCH_NAME}" = "x86_64" ] && [ -f "./bin/axc-linux-x64" ]; then
    cp "./bin/axc-linux-x64" "${AXC_BIN}"
elif [ "${OS_NAME}" = "Darwin" ] && [ "${ARCH_NAME}" = "arm64" ] && [ -f "./bin/axc-macos-arm64" ]; then
    cp "./bin/axc-macos-arm64" "${AXC_BIN}"
else
    echo "[-] No prebuilt ./bin/axc binary for ${OS_NAME}-${ARCH_NAME}. Cross-compile with: axc build compiler_ax/main.ax --target <target> -o bin/axc" >&2
    exit 1
fi

rm -f "${INSTALL_STD}"/*.ax
cp ./std/*.ax "${INSTALL_STD}/"
for f in AXON_LLM_SPEC.md axon_mcp.json; do
    [ -f "./${f}" ] && cp "./${f}" "${AXON_ROOT}/"
done
chmod +x "${AXC_BIN}"

SHELL_PROFILE="${HOME}/.bashrc"
[ -f "${HOME}/.zshrc" ] && SHELL_PROFILE="${HOME}/.zshrc"
if ! echo "${PATH}" | grep -Fq "${INSTALL_BIN}" && ! grep -Fq "${INSTALL_BIN}" "${SHELL_PROFILE}" 2>/dev/null; then
    echo "export PATH=\"${INSTALL_BIN}:\$PATH\"" >> "${SHELL_PROFILE}"
    echo "[+] Added ${INSTALL_BIN} to PATH in ${SHELL_PROFILE}"
fi

echo "[+] Installed Axon v${AXON_VERSION} compiler + stdlib to ${AXON_ROOT}"
"${AXC_BIN}" version
