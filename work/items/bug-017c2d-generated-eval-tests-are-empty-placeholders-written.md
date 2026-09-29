+++
id = "bug-017c2d"
kind = "bug"
title = "Generated eval tests are empty placeholders written into the repository root"
status = "open"
triage = "verified"
severity = "p2"
goal = "learning"
subsystem = ["roko-gate/eval-generator", "roko-cli/graph-dispatch"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-gate/src/eval_generator.rs::builtin_templates", "crates/roko-cli/src/graph_task_dispatch.rs:3290", "crates/roko-core/src/config/gates.rs:143"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[repro]]
command = 'test -z "$(ls generated-tests 2>/dev/null)"'

[[verify]]
command = "test -z \"$(ls generated-tests 2>/dev/null)\" && ! grep -qE 'verified by (ClippyGate|TestGate)|this is a placeholder' crates/roko-gate/src/eval_generator.rs  (cargo test -p roko-gate eval_generator runs the existing tests, which pass with the placeholder templates)"
+++

With `feedback.eval_generation_enabled`, Graph dispatch runs `EvalGenerator::generate_all` and writes `<workdir>/generated-tests/gen_*.rs` into the repository the agent edits.
The built-in `compile-check`, `clippy-clean` and `test-pass` templates (`eval_generator.rs:417`) render empty-bodied `#[test]` functions (e.g. `fn gen_clippy_clean() { // Clippy cleanliness verified by ClippyGate. }`), so these "generated test" gates prove nothing.
87 such untracked files currently sit in the repo root.
Fix: generate only tests with real bodies into a gitignored staging location, make a zero-assertion test fail its gate, and remove the placeholders.

Checked 2026-09-28: 42 of the 87 files under `generated-tests/` are tracked in git, so "untracked" is wrong for those. Removing them needs a `git rm`.

Checked 2026-09-29 at d9e79e9d8: 725f21e05 fixed the write location. Graph dispatch generates evals only when gates.write_eval_artifacts is set (default false) and writes them to .roko/generated-tests/, which is gitignored; no new files have appeared in the repo root since 2026-09-26. What remains: the compile-check, clippy-clean and test-pass templates in crates/roko-gate/src/eval_generator.rs still render empty-bodied tests, no gate rejects a zero-assertion generated test (the code comment says nothing in plan run executes them), and the 87 old placeholders under generated-tests/ still need removing (git rm for the 42 tracked files).
