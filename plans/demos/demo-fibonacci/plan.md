---
plan: demo-fibonacci
estimated_tasks: 2
estimated_parallel_width: 1
estimated_minutes: 3
parallel_safe: true
tags: [demo, rust, implementer, scribe]
---

# Fibonacci demo

Two roles in sequence on a small, fully checkable Rust program:

1. `FIB-1` (implementer) creates the standalone crate `demo/fibonacci/` with a
   `fibonacci(n)` function, unit tests, and a `main` that prints the first ten
   Fibonacci numbers.
2. `FIB-2` (scribe) reads the accepted code and writes `demo/fibonacci/README.md`.

`FIB-1`'s verify steps compile `src/main.rs` with `rustc`, check the printed
sequence, and run the unit tests, so no Cargo build directory or lockfile is
written into the tree. `FIB-2` depends on `FIB-1`, so the README is written
from code that has already passed its checks.

## Origin

Converted from `plans/test-fibonacci`, left by a manual portal test on
2026-09-25. Its `plan.md` held the prompt `create a rust function that
calculates fibonacci numbers`, and its `tasks.toml` held these same two tasks
(implementer, then scribe) as `[[tasks]]` tables without titles or verify
steps, which the current parser rejects.
