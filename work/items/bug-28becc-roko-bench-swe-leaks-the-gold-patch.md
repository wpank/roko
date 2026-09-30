+++
id = "bug-28becc"
kind = "bug"
title = "roko bench swe leaks the gold patch, passes empty tests, and records 100% 'resolved' by default"
status = "open"
triage = "verified"
severity = "p2"
goal = "release"
subsystem = ["roko-cli/bench"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "tmp/cybernetic-harness/assessment-2026-09-28/s07-s09-bench.md"
discovered_from = "audit:tmp/cybernetic-harness/assessment-2026-09-28/s07-s09-bench.md"
anchors = ["crates/roko-cli/src/bench.rs::SweBenchInstance", "crates/roko-cli/src/bench.rs:534", "crates/roko-cli/src/bench.rs:297", "crates/roko-cli/src/main.rs:1694"]
links = { depends_on = [], blocks = [], related = ["bug-3a037b"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqE 'fn swe_[A-Za-z0-9_]*' crates/roko-cli/ && cargo test -p roko-cli --lib bench::tests::swe_"
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
