+++
id = "bug-2504b3"
kind = "bug"
title = "An audit's clean re-run (A2) runs cargo test over the attempt's whole result tree, so a defect another task left in its base fails it: T5's audits failed on T4's broken doc example and opened false_green incidents against T5 and gpt-oss-120b in two of three golden-path live runs"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
size = "M"
subsystem = ["roko-cli/audit"]
created = 2026-10-19
updated = 2026-10-19
last_verified = 2026-10-19
source = "gap-f30b8e golden-path rerun, live runs 1-2 (2026-10-19)"
discovered_from = "gap-f30b8e"
anchors = ["crates/roko-cli/src/audit/rerun.rs::rerun", "crates/roko-gate/src/production_service.rs::run_canonical_pipeline", "crates/roko-cli/src/audit/worker.rs"]
links = { depends_on = [], blocks = [], related = ["gap-f30b8e", "find-fc5cdd", "gap-618548"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn an_audit_re_run_does_not_blame_another_tasks_defect' crates/roko-cli/src && cargo test -p roko-cli --lib an_audit_re_run_does_not_blame_another_tasks_defect"
+++

## Problem

An audit's clean re-run (A2) of a green attempt runs the production gate's canonical rungs first, and the
canonical Test rung runs `cargo test` over the whole workspace of the attempt's result tree, not over the crates
the attempt touched. So a defect another task left in the tree the attempt started from fails the re-run, and
the audit opens a `false_green` incident against the audited attempt's task and model.

Seen live twice on 2026-10-19 (gap-f30b8e's rerun, `4535b3914`):

- Run 1 (`graph-golden-path-31d6a6fb-…`): the audit drew T5's green attempt (`…:T5:1`, gpt-oss-120b). Its
  result tree `f47db9c1` holds T4's earlier work, whose doc example for `beta::farewell` does not compile
  (`farewell` with no `use`, E0425). All three A2 runs failed on that doctest (`checks.a2.passes =
  [false, false, false]`, `y = true`), the production gate log says `executed_rungs=3 failed_rungs=1` (compile,
  clippy, test), and the audit opened `inc-0bb304a538c446a5` (`false_green`, model gpt-oss-120b) and moved
  the knob `trust:gpt-oss-120b/roko` from 0.05 to 0.095 (`trust_downgrade`).
- Run 2 (`graph-golden-path-7f133a7f-…`): the same for T5's second attempt (tree `6a021e0f`, the same broken
  `farewell` doc example from T4), incident `inc-c58899c9ee9125b8`.

T5's own work was right in both runs (the reviews found nothing wrong with it, and the whole-plan check passed
once T6 fixed T4's example). A reproduction outside roko: `cargo test` in run 1's `f47db9c1` fails only on
`crates/beta/src/lib.rs - farewell (line 15)`.

## Why it matters

The audit is M4's and M1's only sensor of false greens (gap-618548): a false incident teaches the self-model a
wrong VS label and moves trust for the wrong model. In a plan whose tasks build on each other, any defect an
earlier task leaves (a lint, a doc example, a flaky test) is charged to whichever later attempt the lottery
draws. It also opens a fix proposal (`inc-….task.toml`) for a task that has nothing to fix.

## Where

- `crates/roko-cli/src/audit/rerun.rs::rerun` / `run_checks`: A2 runs through `ProductionGateService`
  (decision 7102), canonical rungs first.
- `crates/roko-gate/src/production_service.rs::run_canonical_pipeline` and `crates/roko-gate/src/test_gate.rs`:
  the Test rung scopes `cargo test` by the payload's `target_crates`, which the audit's request leaves empty.
- `crates/roko-cli/src/audit/worker.rs` builds the `ServiceContext`, which already holds `changed_files`.

## Plan

1. Scope A2's canonical rungs to the crates the attempt changed (the `changed_files` the ServiceContext holds),
   as its `test:<crate>` checks already are; or judge A2 against the attempt's base: a check that fails the
   same way on the base tree is not the attempt's failure.
2. A test: an audited attempt whose base holds another crate's failing doc test, and whose own crate is
   clean, gets `y = false` and no incident.

## Done when

- [ ] The `[[verify]]` test passes, and fails before the fix.

## Notes

- Found by the worker of gap-f30b8e's 2026-10-19 rerun (W-handback); the audit records are in each run's
  scratch HOME vault, and the copied run evidence is in the main checkout's
  `tmp/status-quo/2026-10-16-live/evidence/gp3/` (gitignored).
- The same runs show the audit's B1 author and B3 judge on `claude-haiku-4-5` through the Claude CLI, which the
  golden-path harness fences; both ended in errors (`b1.error`, `b3.reason = judge error`). That is the harness's
  fence, not this bug.
