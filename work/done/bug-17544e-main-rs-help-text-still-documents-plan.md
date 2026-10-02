+++
id = "bug-17544e"
kind = "bug"
title = "main.rs help text still documents plan run flags that plan run now rejects"
status = "done"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/main"]
created = 2026-10-01
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "6a08f9e2c"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-d60281"
anchors = ["crates/roko-cli/src/commands/plan.rs::PlanCmd", "crates/roko-cli/src/main_tests.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-d60281"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'fn plan_run_help_hides_the_flags_plan_run_rejects' crates/roko-cli/src/main_tests.rs && cargo test -p roko-cli --bin roko plan_run_help"

[closed]
at = 2026-10-02
at_ts = "2026-10-02T07:43:00Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-02T01:20:18Z"
forced = false
evidence = "plan run --help hides the flags plan run rejects (wk-climain d979f0a35); plan_run_help tests pass in the bin tests; gate 6h2 passed at 285282248 (cargo check, clippy -D warnings, 11,366 lib tests in roko-agent/cli/core/fs/gate/graph/learn/serve, canaries C1-C8 plus integration tests, 446 roko-cli bin tests, run_evidence py, portal tsc and 809 vitest); merged in 6a08f9e2c"
+++

## Problem

gap-d60281 made `roko plan run` fail on the seven flags the Graph engine ignored. Their `#[arg]` help text in `main.rs` still describes them as working options.

## Plan

Hide the rejected flags from `--help` (`hide = true`), or reword them as rejected, and add a test named `plan_run_help_*` that checks `--help` no longer advertises them.

## Done when

- `cargo test -p roko-cli --bin roko plan_run_help` passes.

## Notes

- Reported on 2026-10-01 by the worker on gap-d60281, during the evening close-out round.
- 2026-10-02 (wk-climain): implemented on work/bug-28c193; cargo verification deferred to the batch check.
  `PlanCmd::Run` (now in `commands/plan.rs` after gap-0d0e81) marks `--skip-preflight`, `--screenshots`,
  `--screenshot-interval`, `--screenshot-dir` and `--batch-size` `hide = true`, and their doc comments say why they
  still parse: `plan run` names what to use instead. The global `--resume` and `--effort`, the other two flags
  gap-d60281 rejects, stay visible: other commands use them, and their help text describes them generically. Test:
  `plan_run_help_hides_the_flags_plan_run_rejects` (main_tests.rs) checks that `plan run --help` lists none of the
  five and that they still parse. The verify now guards its cargo filter with a grep for the test, and the anchors
  follow PlanCmd to commands/plan.rs.
