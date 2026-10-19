+++
id = "find-5bcba7"
kind = "finding"
title = "A whole-plan check failure ends a golden-path run: in the 2026-10-18 rerun T7 left a clippy lint no task check runs (VERSION after the test module), and roko sent the failure back to no task, so run 1 of 3 failed with every task passed"
status = "open"
triage = "verified"
severity = "p1"
goal = "golden-path"
size = "M"
subsystem = ["self-hosting"]
created = 2026-10-18
updated = 2026-10-19
last_verified = 2026-10-18
source = "gp2 golden-path rerun, live run 1 (find-4003b9, gap-f30b8e; 2026-10-18)"
discovered_from = "gap-f30b8e"
anchors = ["crates/roko-cli/tests/fixtures/golden_path/plan/tasks.toml", "crates/roko-cli/src/graph_execution/delivery.rs"]
links = { depends_on = [], blocks = [], related = ["gap-f30b8e", "find-4003b9", "spec-f09094", "bug-2110e8"], supersedes = [], duplicate_of = "" }
+++

## Finding

Run 1 of the golden-path rerun on the tightened fixture (`15837120f`, 2026-10-18; gap-f30b8e, find-4003b9) failed
after every task had passed. All nine tasks passed their own checks on their first attempt. But T7 (mechanical,
gpt-oss-120b, "Expose alpha's version") appended `pub const VERSION` after alpha's `#[cfg(test)] mod tests`. roko's
whole-plan check then failed `cargo clippy --workspace --all-targets -- -D warnings` on
`clippy::items_after_test_module`. The delivery settled `regression_failed`, which is terminal. Nothing was
delivered, and roko exited 1 (Graph run `graph-09f0bd03-b7ec-463f-b311-5d6409516cac`).

No check at the task level could see it:

- T7's only check is its pinned accept test (`cargo test -p alpha --test accept_version`). Its description, unlike
  T3's and T5's, does not ask for a clippy-clean crate. The workspace config has `[gates] rungs = []`, so no rung
  adds a lint.
- Once the whole-plan check fails, roko gives no task the failure back. The run ends there, though the clippy output
  names the file (`crates/alpha/src/lib.rs`) and the fix is one move.

In the five other live runs (2026-10-16 runs 1-3, 2026-10-18 runs 2-3), T7 put `VERSION` above the tests. So this is
a per-run hazard of the cheap rung, about 1 in 6 so far, not a fixed failure.

Run 3 of the same rerun came close to failing the same way. T4's accepted commit (`27a3265`) has a doc example that
cannot compile: it calls `farewell` with no `use beta::farewell;`. T4's only check,
`cargo test -p beta --test accept_farewell`, builds the integration test and runs no doctests. T6 later edited the
same doc block and added the line (`015840a`). Had it not, the whole-plan check's `cargo test --workspace` would have
failed run 3 the way clippy failed run 1. Both are defects that only the whole-plan check runs (clippy, doctests).

## Why it matters

The golden path's claim is that a frontier plan runs on cheap models, escalates only when a check fails, and merges
green with nobody stepping in (gap-f30b8e, W8's GP). Here roko's own whole-plan check found the defect, which is the
check doing its job. But the run could only fail: no attempt saw the failure, so the ladder had nothing to escalate
on. It is the one failure of this rerun's pass^3 (2 of 3).

## Options (a decision for Will)

1. **Count it as a fail and rerun pass^3 unchanged.** The hazard stays, about 1 in 6 a run.
2. **Operator config: the plan's Rust checks as a rung for every task.** For example, `[[gates.rungs]]` command
   rungs in the live config that run the plan's clippy step and `cargo test --workspace --doc`. The task that leaves
   a lint or a broken doc example then fails its own check and retries with the output. This changes no task and no
   frozen plan text. It adds a few seconds of the seed workspace's clippy and doctests to every attempt, and the
   Python and TypeScript tasks would run them too.
3. **Fixture: give T7 the lint.** Add a clippy verify step to T7, or the "free of clippy warnings" sentence T3 and
   T5 have. This changes the frozen plan again, as find-4003b9 did, and plan.md must say so.
4. **roko: send a failed whole-plan check back to a task.** Attribute the failure to the task that last changed a
   file the output names (here T7, `crates/alpha/src/lib.rs`), retry that task with the output as feedback, and run
   the whole-plan check again before the delivery gives up. This is the general fix, and also the largest. It
   touches the delivery state machine (`regression_failed` is terminal today).

## Evidence

- In the main checkout (gitignored): `tmp/status-quo/2026-10-16-live/evidence/gp2/golden-path/run1/`. It has
  `stdout.json` (`batch.deliveries[0]`: `state = regression_failed`, the clippy error), `stderr.txt` ("delivery
  regression failed (terminal)"), `plan.diff`, `plan.log`, and the run record. The T7 code is in the plan branch
  tip `983207cc`, `crates/alpha/src/lib.rs` lines 56-71.
- Run 3's doc example: `golden-path/run3/review.md` (T4 line) and `merged.log`. The blob is
  `27a3265:crates/beta/src/lib.rs` in the run's scratch repo (gone once the scratchpad is cleaned).
- The record: `work/history/golden-path-live-2026-10-18.md`.

## Decision (2026-10-19, Will)

Hand it back to the task: a whole-plan check failure goes back as a retry to the task(s) whose files it names, then the plan is checked again. Then rerun the golden path's pass^3 (about $0.70).
