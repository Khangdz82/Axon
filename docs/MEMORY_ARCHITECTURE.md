# Axon v1.1 — Memory Architecture & Zero-Leak Guide

Axon achieves **zero garbage-collector pauses** and **0 B memory leaks** through compile-time **Perceus linear ownership** paired with deterministic runtime memory primitives in [`std/mem_guard.ax`](../std/mem_guard.ax).

---

## 1. Compile-Time Ownership (Perceus 0-GC)

| Rule | Syntax | Compile-Time Guarantee |
|---|---|---|
| Immutable binding | `x := 10` | Cannot be reassigned |
| Mutable binding | `mut x := 10` | Required for in-place mutation (`x += 1`) or `mut` params |
| Linear move | `buf_free(move b)` | Transfers ownership; use-after-move rejected (`E0201`/`E0203`) |
| Borrowed reference | `&T` | Cannot be stored in structs or returned (`E0101`/`E0102`) |
| Zero-alloc hot path | `@no_alloc` | Rejects heap allocation inside annotated functions |
| Unsafe boundary | `unsafe { ... }` | Required for raw `ptr_read_*` / `ptr_write_*` / `mem_alloc_aligned` (`E0503`) |

---

## 2. Deterministic Runtime Allocators (`std::mem_guard`)

1. **`FrameArena` (O(1) Per-Frame Bump Allocator):**
   - Ideal for per-frame UI layouts, temporary AI scratchpads, and request arenas.
   - `frame_arena_new(cap)` (capped at 1 GB, overflow-checked), `frame_arena_alloc(a, bytes)` (64-byte aligned), `frame_arena_reset(a)` (O(1) pointer reset, 0 syscalls), `frame_arena_free(move a)`.
2. **`ObjectPool` (O(1) Fixed-Size Slab Pool):**
   - Ideal for ECS entities, network connections, and UI nodes.
   - `obj_pool_new(slot_bytes, cap)` -> `obj_pool_acquire(p)` -> `obj_pool_release(p, id)` (blocks double-release) -> `obj_pool_free(move p)`.
3. **`LeakGuard` (Zero-Leak Tracker + Small-Block Magazine):**
   - Tracks every allocation in an open-addressed Fibonacci hash table + 512×1024 B LIFO magazine for small blocks.
   - `leak_guard_alloc(g, sz)` -> `leak_guard_free(g, p)` -> `leak_guard_is_clean(g)` -> `leak_guard_destroy(move g)` (auto-sweeps any remaining leaks).

---

## 3. Ring 0 <-> Ring 3 & Security Boundaries

- **Overflow-Proof Bounds Trapping:** Intrinsics like `buf_fill_u32` and `buf_get`/`ibuf_get` validate `(buf.len - byte_off) >> 2 >= count_u32` without integer wrap-around, trapping to `h_sec_trap` with a PIE-safe stack backtrace.
- **Ring 0 <-> Ring 3 Bridge ([`std/ring_bridge.ax`](../std/ring_bridge.ax)):** `IoRingBridge` isolates SQ/CQ indices on separate 64-byte cache lines and validates every user-space pointer via `ring0_probe_user_range(addr, len)` (`0x10000..0x00007FFFFFFFFFFF`) with bounded batch sizes.
