# Contributing to Axon

Thanks for contributing to Axon! All changes to `master` must go through a Pull Request and pass automated checks.

## Engineering Rules

1. **Pure Axon:** Compiler passes (`compiler_ax/`) and standard library modules (`std/`) must be written in 100% pure Axon (`.ax`). Do not introduce external C/Python/Node/LLVM build dependencies.
2. **Code Ownership:** All PRs require review from `@Khangdz82` (`.github/CODEOWNERS`) and passing `verify-axon` CI checks.
3. **Zero Temp Garbage:** `axc check`, `axc test`, and `axc run` must leave no temporary artifacts behind.

## Required Verification Before Opening a PR

1. **Static verification & smoke tests:**
   ```bash
   ./bin/axc.exe check compiler_ax/main.ax        # Windows
   ./bin/axc-linux-x64 check compiler_ax/main.ax  # Linux
   ```
2. **Runtime & compile-rejection test suites (`tests/pass/` and `tests/fail_compile/`):**
   Ensure all relevant `@test` suites in `tests/pass/` pass and rejection tests in `tests/fail_compile/` still emit expected error codes.
3. **3-Stage Deterministic Bootstrap (mandatory when modifying `compiler_ax/*.ax`):**
   ```powershell
   .\bin\axc.exe build compiler_ax/main.ax -o axc_s1.exe
   .\axc_s1.exe build compiler_ax/main.ax -o axc_s2.exe
   .\axc_s2.exe build compiler_ax/main.ax -o axc_s3.exe
   ```
   Verify `SHA256(axc_s2.exe) == SHA256(axc_s3.exe)` before submitting.
