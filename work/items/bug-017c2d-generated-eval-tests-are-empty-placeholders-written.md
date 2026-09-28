+++
id = "bug-017c2d"
kind = "bug"
title = "Generated eval tests are empty placeholders written into the repository root"
status = "open"
triage = "verified"
severity = "p2"
subsystem = ["roko-gate/eval-generator", "roko-cli/graph-dispatch"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-gate/src/eval_generator.rs::builtin_templates", "crates/roko-cli/src/graph_task_dispatch.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[repro]]
command = 'test -z "$(ls generated-tests 2>/dev/null)"'

[[verify]]
command = 'cargo test -p roko-gate eval_generator'
+++

With `feedback.eval_generation_enabled`, Graph dispatch runs `EvalGenerator::generate_all` and writes `<workdir>/generated-tests/gen_*.rs` into the repository the agent edits.
The built-in `compile-check`, `clippy-clean` and `test-pass` templates (`eval_generator.rs:417`) render empty-bodied `#[test]` functions (e.g. `fn gen_clippy_clean() { // Clippy cleanliness verified by ClippyGate. }`), so these "generated test" gates prove nothing.
87 such untracked files currently sit in the repo root.
Fix: generate only tests with real bodies into a gitignored staging location, make a zero-assertion test fail its gate, and remove the placeholders.
