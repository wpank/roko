+++
id = "bug-a22228"
kind = "bug"
title = "Serve bench counts a task as passed when its output merely contains the expected text"
status = "open"
triage = "verified"
severity = "p2"
goal = "release"
subsystem = ["roko-serve/bench"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/cybernetic-harness/assessment-2026-09-28/s07-s09-bench.md"
discovered_from = "audit:tmp/cybernetic-harness/assessment-2026-09-28/s07-s09-bench.md"
anchors = ["crates/roko-serve/src/routes/bench.rs:312", "crates/roko-serve/src/routes/bench.rs:1388", "crates/roko-serve/src/bench.rs::builtin_learnable_rust_suite"]
links = { depends_on = [], blocks = [], related = ["bug-3a037b"], supersedes = [], duplicate_of = "" }
+++
- A serve bench task passes when the agent succeeded and its output contains `expected_output` as a substring (`roko-serve/src/routes/bench.rs:307-312`).
- The built-in `learnable-rust` suite ships `todo!()` stubs with no tests (`routes/bench.rs:1388`). Its expected outputs are strings like "Finished" and "test result: ok", which a build or an empty test run prints anyway, so it can pass without any edit. This is inferred from the scaffold; it was not run.
- All tasks share one temp dir.
- `estimate_cost_usd` prices models by substring and is out of date.

Fix: grade with executed checks (tests that fail before the change), not output substrings, and give each task its own workspace.
