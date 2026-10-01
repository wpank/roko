+++
id = "bug-3a037b"
kind = "bug"
title = "roko bench demo presents simulated savings as benchmark results"
status = "done"
triage = "verified"
severity = "p2"
goal = "release"
subsystem = ["roko-cli/bench"]
created = 2026-09-28
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "39cd18049"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-cli/src/bench_demo.rs::run_bench_demo", "crates/roko-cli/src/bench_demo.rs::simulate_task"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn real_mode_does_not_fabricate_passes' crates/roko-cli/ && cargo test -p roko-cli --lib bench_demo::tests::real_mode_does_not_fabricate_passes"

[closed]
at = 2026-09-30
by = "coordinator (session 7622b882)"
evidence = "Merged in d25f9f2da. Every simulated figure is labelled simulated; --real reports not verified, cost n/a; no simulation fallback. Batch 13 gate (MAIN 39cd18049 has the same crates and Cargo.lock as gated 172f3683a/d76f9faf8): cargo check --workspace --tests clean; nightly fmt clean after the coordinator's rustfmt commits on 7 branches; clippy -p (10 crates) --keep-going -D warnings clean after two doc-paragraph fixes (8b8ec4f25, e3deb0c37); lib tests pass: roko-cli 3160, roko-agent 2262, roko-core 1945, roko-learn 1199, roko-serve 977, roko-gate 689, roko-graph 472, roko-execution 252, roko-std 226, roko-acp 199. Three load flakes (turn_policy escalated-timeout, roko-gate tautology, verification efficiency-record wait) pass alone and are noted on bug-779ae7. Verify: its test passes in that run and its static checks pass on MAIN."
+++

Without `--real`, `run_bench_demo` (`bench_demo.rs:109`) computes every figure with `simulate_task` (`:492`: fixed per-difficulty costs and pass rates) and prints "{x}x cost reduction" (`:436`) plus a cost waterfall from constant percentages; nothing says the numbers are simulated.
Per a local audit, `--real` marks each task passed without running a gate and falls back to simulated numbers when dispatch fails.
Fix: label simulated output, never fabricate `--real` results (run the gates or report "not run"), and drop the constant waterfall.

## Notes

- **wk-honestbench (2026-09-30):** Implemented on `work/bug-28becc` at `7a5b388ca`; cargo verification deferred to the
  batch check.
- `--real` cost stays "n/a": `dispatch_bench_prompt` returns tokens but no cost, so nothing measures it. The
  `bench demo` help text in `main.rs` still says "comparative benchmark" (out of scope here).
