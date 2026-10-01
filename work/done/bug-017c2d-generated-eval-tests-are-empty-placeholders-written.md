+++
id = "bug-017c2d"
kind = "bug"
title = "Generated eval tests are empty placeholders written into the repository root"
status = "done"
triage = "verified"
severity = "p2"
goal = "learning"
subsystem = ["roko-gate/eval-generator", "roko-cli/graph-dispatch"]
created = 2026-09-28
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-gate/src/eval_generator.rs::builtin_templates", "crates/roko-cli/src/graph_task_dispatch.rs::GraphTaskDispatcher::dispatch", "crates/roko-core/src/config/gates.rs:143"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[repro]]
command = 'test -z "$(ls generated-tests 2>/dev/null)"'

[[verify]]
command = "test -z \"$(ls generated-tests 2>/dev/null)\" && ! grep -qE 'verified by (ClippyGate|TestGate)|this is a placeholder' crates/roko-gate/src/eval_generator.rs && grep -q 'generate_checked' crates/roko-cli/src/graph_task_dispatch.rs"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:07Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T16:12:59Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

With `feedback.eval_generation_enabled`, Graph dispatch runs `EvalGenerator::generate_all` and writes `<workdir>/generated-tests/gen_*.rs` into the repository the agent edits.
The built-in `compile-check`, `clippy-clean` and `test-pass` templates (`eval_generator.rs:417`) render empty-bodied `#[test]` functions (e.g. `fn gen_clippy_clean() { // Clippy cleanliness verified by ClippyGate. }`), so these "generated test" gates prove nothing.
87 such untracked files currently sit in the repo root.
Fix: generate only tests with real bodies into a gitignored staging location, make a zero-assertion test fail its gate, and remove the placeholders.

Checked 2026-09-28: 42 of the 87 files under `generated-tests/` are tracked in git, so "untracked" is wrong for those. Removing them needs a `git rm`.

Checked 2026-09-29 at d9e79e9d8: 725f21e05 fixed the write location. Graph dispatch generates evals only when gates.write_eval_artifacts is set (default false) and writes them to .roko/generated-tests/, which is gitignored; no new files have appeared in the repo root since 2026-09-26. What remains: the compile-check, clippy-clean and test-pass templates in crates/roko-gate/src/eval_generator.rs still render empty-bodied tests, no gate rejects a zero-assertion generated test (the code comment says nothing in plan run executes them), and the 87 old placeholders under generated-tests/ still need removing (git rm for the 42 tracked files).

Checked 2026-09-29 at f99e45dba: unchanged. EvalGenerator::generate_checked (crates/roko-gate/src/eval_generator.rs:220) already rejects empty, comments-only, assert!(true) and todo!() bodies, but Graph dispatch calls generate_all (graph_task_dispatch.rs:3310), which renders the placeholder templates; switching to generate_checked (or dropping the three placeholder templates) is the obvious fix. The 87 files under generated-tests/ are still present (42 tracked, 45 untracked).

## Notes

- 2026-10-01 (wk-learn2): implemented on work/gap-14f08e; cargo verification deferred to the batch check.
  - `eval_generator.rs`: the compile-check, clippy-clean and test-pass templates are gone; the property template is
    the only built-in one. `generate_checked` now also rejects a rendered evaluation with no `#[test]` function, or
    one whose body is vacuous (new `EvalGenerationError::VacuousTest`), so a zero-assertion test fails generation.
    New `generate_checked_all` runs it for every gate type and returns the rejections too.
  - Graph dispatch calls `generate_checked_all` and logs each rejection at debug. Graph tasks author no property
    body, so `gates.write_eval_artifacts` now writes nothing; the config doc and the code comment say so. The test
    `eval_artifacts_never_hold_placeholder_tests_or_reach_the_repo_root` (renamed) asserts it.
  - `git rm` of the 42 tracked `generated-tests/` files. The 45 untracked ones exist only in MAIN and still need
    deleting there; until then this item's verify fails in MAIN.
- Follow-ups for the coordinator: `skip_enrichment_plan_meta_is_read_and_suppresses_eval_artifacts` no longer proves
  the suppression, since nothing is written either way; and `write_eval_artifacts` is now inert on Graph (a candidate
  for `graph_engine_inert_settings`, or for removal).
