+++
id = "bug-3a037b"
kind = "bug"
title = "roko bench demo presents simulated savings as benchmark results"
status = "open"
triage = "verified"
severity = "p2"
goal = "release"
subsystem = ["roko-cli/bench"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-cli/src/bench_demo.rs::run_bench_demo", "crates/roko-cli/src/bench_demo.rs::simulate_task"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn real_mode_does_not_fabricate_passes' crates/roko-cli/ && cargo test -p roko-cli --lib bench_demo::tests::real_mode_does_not_fabricate_passes"
+++

Without `--real`, `run_bench_demo` (`bench_demo.rs:109`) computes every figure with `simulate_task` (`:492`: fixed per-difficulty costs and pass rates) and prints "{x}x cost reduction" (`:436`) plus a cost waterfall from constant percentages; nothing says the numbers are simulated.
Per a local audit, `--real` marks each task passed without running a gate and falls back to simulated numbers when dispatch fails.
Fix: label simulated output, never fabricate `--real` results (run the gates or report "not run"), and drop the constant waterfall.

## Notes

- **wk-honestbench (2026-09-30):** Implemented on `work/bug-28becc` at `7a5b388ca`; cargo verification deferred to the
  batch check.
- `--real` cost stays "n/a": `dispatch_bench_prompt` returns tokens but no cost, so nothing measures it. The
  `bench demo` help text in `main.rs` still says "comparative benchmark" (out of scope here).
