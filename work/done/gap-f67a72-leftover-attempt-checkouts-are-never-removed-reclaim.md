+++
id = "gap-f67a72"
kind = "gap"
title = "Leftover attempt checkouts are never removed: reclaim_idle has no production caller"
status = "done"
triage = "verified"
severity = "p3"
goal = "core"
size = "S"
subsystem = ["roko-cli/orchestrator"]
created = 2026-10-01
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "f88210c84"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-bdfb1d"
anchors = ["crates/roko-cli/src/orchestrator/worktree/cleanup.rs", "crates/roko-cli/src/doctor.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["bug-bdfb1d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn leftover_checkouts_go_only_when_their_plan_has_ended' crates/roko-cli/src/ && cargo test -p roko-cli --lib leftover_checkouts_go_only_when_their_plan_has_ended"

[[verify]]
command = "grep -rqw 'fn cli_parses_doctor_disk_fix' crates/roko-cli/src/ && cargo test -p roko-cli --bin roko cli_parses_doctor_disk_fix"

[closed]
at = 2026-10-02
at_ts = "2026-10-02T08:27:42Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-02T01:20:12Z"
forced = false
evidence = "roko doctor disk --fix removes leftover attempt checkouts by the recorded retention rule and keeps branches, dirty and locked checkouts; leftover_checkouts_go_only_when_their_plan_has_ended and cli_parses_doctor_disk_fix pass (wk-planrun 848ff330c; removal uses one --force, 09aa7e009); gate 6i passed at f4347b8eb (cargo check, clippy -D warnings, lib tests of roko-acp/cli/core/gate/learn/runtime/serve, canaries C1-C8 and integration tests, roko-acp integration tests, roko-cli doctor, learning_wiring_census and graph_plan_callers tests, 447 bin tests, run_evidence py); two fixes in 09aa7e009 re-tested; merged in f88210c84"
+++

## Problem

Attempt checkouts left by crashed or interrupted runs are never removed: `reclaim_idle` has no production caller, and `roko doctor disk` only reports orphans. Resumed runs re-attach kept checkouts (bug-056b40), so cleanup needs a retention rule.

## Plan

Decide the retention rule (for example: no live checkpoint references the checkout, and it is older than N days), then call the cleanup from `roko doctor disk --fix` or at run start.

## Done when

- A retention rule is recorded, and a test shows an orphaned checkout removed while a resumable one is kept.

## Notes

- Reported on 2026-10-01 by the worker on bug-bdfb1d, during the evening close-out round.

2026-10-02 (wk-planrun): implemented on work/gap-dd4826, based on f8906b3c0; cargo verification deferred to the batch check.
The retention rule, as the coordinator decided it on 2026-10-02. Leftover attempt checkouts are removed only by an explicit `roko doctor disk --fix`, never automatically, and only when all of these hold:
- the checkout's `roko-run` record names the run that made it, and a plan's Graph checkpoint, current or archived (`checkpoint.json.bak.*`), names that run;
- the current checkpoint of each such plan is terminal and not resumable: succeeded, failed or cancelled. A running, interrupted or unverified checkpoint keeps its plan's checkouts, and so does a run no checkpoint names;
- nothing touched the checkout for 7 days (`LEFTOVER_CHECKOUT_MIN_AGE`: the newest mtime of the checkout directory and of its git admin directory, `index` and `HEAD`);
- it has no uncommitted changes, and no `git worktree lock` holds it: `git worktree remove` runs without `--force`.

`--fix` holds the exclusive workspace lock and the runner lock, so no `roko plan run`, `roko run` or `roko serve` is live in the workspace while it removes checkouts. It prints each checkout it removed and each it kept, with the reason (`leftover_checkouts` in `--json`), and keeps the branches. With any subject other than `disk` it is refused.
Code: `WorktreeManager::remove_leftover_checkouts` (cleanup.rs), `recorded_checkpoint_runs` (graph_checkpoint.rs), `fix_leftover_checkouts` and `render_leftover_checkouts` (doctor.rs), and the `--fix` flag (main.rs, `cmd_doctor` in commands/util.rs).
Tests: `leftover_checkouts_go_only_when_their_plan_has_ended` (worktree/tests.rs) removes the checkouts of a succeeded plan's current run and of the run it replaced, keeps their branches, and keeps a resumable plan's checkout, one with no run record and one with changes; `cli_parses_doctor_disk_fix` (main_tests.rs); `leftover_checkouts_render_what_went_and_why_the_rest_stayed` (doctor.rs); `doctor_disk_fix_reports_leftover_checkouts` and `doctor_fix_is_refused_outside_disk` (tests/doctor.rs).
Left open by the rule: checkouts with uncommitted changes stay, and the failed attempts kept for post-mortem usually have some. Reclaiming those would discard that work, or first commit it onto the attempt branch; that needs a decision. `reclaim_idle` still has no production caller, and this rule does not need one.
