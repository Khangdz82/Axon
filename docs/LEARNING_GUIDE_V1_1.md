# Axon v1.1 — Standard Learning Path (Humans & AI Agents)

A concise, practical curriculum to master **Axon v1.1** in 4 stages.

---

## Stage 1: 15-Minute Quickstart (Core Syntax)
1. **Run & Check:**
   ```bash
   bin/axc.exe check examples/hello.ax
   bin/axc.exe run examples/hello.ax
   ```
2. **Core Rules:**
   - Logic: `and`, `or`, `not` (never `&&`, `||`, `!`).
   - Error unions (`T ! i64`): return `ok(v)` / `err(-1)`; unwrap with `x := f() or return` or `x := f() or fallback`.
   - Structs & named args use `=`: `Point(x = 1, y = 2)`.
   - Read [`LEARN_AXON_EN.md`](../LEARN_AXON_EN.md) (or [`LEARN_AXON_VI.md`](../LEARN_AXON_VI.md)) Chapters 1–4.

## Stage 2: Zero-GC Memory & Data-Oriented Design
1. Master Perceus `move` semantics and `@no_alloc` hot paths in [`docs/MEMORY_ARCHITECTURE.md`](MEMORY_ARCHITECTURE.md).
2. Replace ad-hoc `malloc`/`free` with `FrameArena`, `ObjectPool`, and `LeakGuard` (`std::mem_guard`).
3. Use `Table[T]` (SoA) and `std::linq` for zero-allocation in-place pipelines.

## Stage 3: Production Domains (UI, AI, Game2D, Ring 0/3)
- **Desktop UI:** `std::ui`, `std::ui_render` (`buf_fill_u32` ERMS), `std::ui_tailwind`, `std::ui_signal`.
- **AI / Vector / Math:** `std::ai_engine` (`ai_rmsnorm_f32`, `ai_swiglu_f32`, `PagedKvCache`), `std::numpy`.
- **2D/3D Games:** `std::game2d` (SoA `ParticlePool2D`, Binary-Heap A*, DDA raycaster), `std::game_ecs`.
- **Kernel / Driver Bridge:** `std::ring_bridge` (`IoRingBridge`, `ring0_probe_user_range`, `ring_ctl_code_win32`).

## Stage 4: Hardening, Telemetry & Benchmarking
1. **Intelligent Compiler Telemetry:** Run `bin/axc.exe advise app.ax` to detect missing `@no_alloc`, `unsafe` leaks, and optimization opportunities.
2. **Anti-Crack Protection:** Compile with `--shield` (Shield v5) and use `std::shield` (`shield_mba_seal`, `shield_split_key3`, `ShieldMesh`).
3. **Verify Performance:** Run the 30-benchmark suite (`python examples/bench_30_suite/run_30_benchmarks.py`) and full `@test` suites (`tests/pass/*.ax`).

---

## AI Agent Fast Loop
```bash
bin/axc.exe check --json path/to/app.ax
bin/axc.exe test path/to/app.ax
bin/axc.exe advise path/to/app.ax --json
```
Always write **100% pure Axon (`.ax`)** using the 77 native `std/*.ax` modules listed in [`AGENTS.md`](../AGENTS.md).
