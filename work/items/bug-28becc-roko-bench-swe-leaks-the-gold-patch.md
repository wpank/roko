+++
id = "bug-28becc"
kind = "bug"
title = "roko bench swe leaks the gold patch, passes empty tests, and records 100% 'resolved' by default"
status = "done"
triage = "verified"
severity = "p2"
goal = "release"
subsystem = ["roko-cli/bench"]
created = 2026-09-28
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "39cd18049"
source = "tmp/cybernetic-harness/assessment-2026-09-28/s07-s09-bench.md"
discovered_from = "audit:tmp/cybernetic-harness/assessment-2026-09-28/s07-s09-bench.md"
anchors = ["crates/roko-cli/src/bench.rs::SweBenchInstance", "crates/roko-cli/src/bench.rs:534", "crates/roko-cli/src/bench.rs:297", "crates/roko-cli/src/main.rs:1694"]
links = { depends_on = [], blocks = [], related = ["bug-3a037b"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqE 'fn swe_[A-Za-z0-9_]*' crates/roko-cli/ && cargo test -p roko-cli --lib bench::tests::swe_"

[closed]
at = 2026-09-30
by = "coordinator (session 7622b882)"
evidence = "Merged in d25f9f2da. The swe agent gets a redacted payload (no gold patch or test_cmd); an empty test_cmd is an error before scoring; gold/empty runs are labelled controls that write no learning state. Batch 13 gate (MAIN 39cd18049 has the same crates and Cargo.lock as gated 172f3683a/d76f9faf8): cargo check --workspace --tests clean; nightly fmt clean after the coordinator's rustfmt commits on 7 branches; clippy -p (10 crates) --keep-going -D warnings clean after two doc-paragraph fixes (8b8ec4f25, e3deb0c37); lib tests pass: roko-cli 3160, roko-agent 2262, roko-core 1945, roko-learn 1199, roko-serve 977, roko-gate 689, roko-graph 472, roko-execution 252, roko-std 226, roko-acp 199. Three load flakes (turn_policy escalated-timeout, roko-gate tautology, verification efficiency-record wait) pass alone and are noted on bug-779ae7. Verify: its test passes in that run and its static checks pass on MAIN."
+++
Three defects in `roko bench swe`:
- **Gold-patch leak.** `SweBenchInstance` carries the reference `patch` (`bench.rs:186-199`), and command mode sends the whole instance to the agent (`serde_json::to_string(instance)`, `:534`).
- **Vacuous pass.** An empty `test_cmd` yields `(true, None)`, which counts as resolved (`:297-304`).
- **Polluting default.** `--agent-mode` defaults to `gold` (`main.rs:1695`) with learning recording on. A bare run writes 100% "resolved", zero-cost episodes, efficiency events and a knowledge insight into `.roko/`.

Fix: strip `patch` (and hidden test commands) from what the agent sees, treat an empty test command as an error, and make gold mode opt-in and never recorded as learning.

Re-verified 2026-09-29: unchanged. The gold default is declared at main.rs:1694.

## Notes

- **wk-honestbench (2026-09-30):** Implemented on `work/bug-28becc` at `10f8a0fd3`; cargo verification deferred to the
  batch check (`cargo test -p roko-cli --lib bench::tests::swe_`, 4 tests; two of them need `python3`, as the old
  gold test did).
- **Still open, `main.rs` only:** `--agent-mode` still defaults to `gold` (`crates/roko-cli/src/main.rs:1695`); that
  file was out of scope for this branch. A bare run is now a labeled control that writes no learning state and
  appends to `.roko/bench/controls.jsonl`. Dropping `default_value_t` so the flag is required finishes "opt-in".
