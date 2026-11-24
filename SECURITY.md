# Security Policy

## Supported Versions

| Version | Supported |
| :--- | :--- |
| `v1.2.x` (`master`) | Yes |
| `< v1.2.0` | No |

## Reporting a Vulnerability

**Please do not report security vulnerabilities through public GitHub Issues.**

If you discover a security issue in the Axon compiler (`compiler_ax/`), `--shield` runtime protections, capability/taint analysis (`E0501`–`E0503`), or standard library modules (`std/`):

1. Open a **private Security Advisory** directly on GitHub:
   `https://github.com/Khangdz82/Axon/security/advisories/new`
2. Include:
   - Affected target (`win-x64`, `linux-x64`, `macos-arm64`, `wasm32`, etc.) and `axc --version`.
   - Minimal reproducible `.ax` snippet or command.
   - Expected vs. actual behavior and security impact.

We will acknowledge reports within 72 hours and coordinate a fix before public disclosure.
