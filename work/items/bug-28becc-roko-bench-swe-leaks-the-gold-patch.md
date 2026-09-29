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
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/cybernetic-harness/assessment-2026-09-28/s07-s09-bench.md"
discovered_from = "audit:tmp/cybernetic-harness/assessment-2026-09-28/s07-s09-bench.md"
anchors = ["crates/roko-cli/src/bench.rs:186", "crates/roko-cli/src/bench.rs:534", "crates/roko-cli/src/bench.rs:297", "crates/roko-cli/src/main.rs:1695"]
links = { depends_on = [], blocks = [], related = ["bug-3a037b"], supersedes = [], duplicate_of = "" }
+++
Three defects in `roko bench swe`:
- **Gold-patch leak.** `SweBenchInstance` carries the reference `patch` (`bench.rs:186-199`), and command mode sends the whole instance to the agent (`serde_json::to_string(instance)`, `:534`).
- **Vacuous pass.** An empty `test_cmd` yields `(true, None)`, which counts as resolved (`:297-304`).
- **Polluting default.** `--agent-mode` defaults to `gold` (`main.rs:1695`) with learning recording on. A bare run writes 100% "resolved", zero-cost episodes, efficiency events and a knowledge insight into `.roko/`.

Fix: strip `patch` (and hidden test commands) from what the agent sees, treat an empty test command as an error, and make gold mode opt-in and never recorded as learning.
