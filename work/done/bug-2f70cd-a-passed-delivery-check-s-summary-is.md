+++
id = "bug-2f70cd"
kind = "bug"
title = "A passed delivery check's summary is dropped: its receipt and the run log can't say it ran cold, so a cold ship check is recorded only in the plan's check log, which its next delivery overwrites"
status = "done"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/graph_execution"]
created = 2026-10-27
updated = 2026-10-27
last_verified = 2026-10-27
last_verified_rev = "2a36515ed"
source = "round-v7 roko-speed, the $0 CLI proof of gap-dc24dd"
discovered_from = "agent:roko-speed (round v7)"
anchors = ["crates/roko-cli/src/graph_execution/delivery.rs::deliver", "crates/roko-cli/src/graph_execution/delivery.rs::DeliveryRegressionOutcome"]
links = { depends_on = [], blocks = [], related = ["gap-dc24dd"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_passed_cold_check_s_receipt_says_it_ran_cold' crates/roko-cli/src && cargo test -p roko-cli --lib a_passed_cold_check_s_receipt_says_it_ran_cold"

[closed]
at = 2026-10-27
at_ts = "2026-10-27T05:22:07Z"
commit = "2a36515ed"
executor = "claude-agent"
via = "manual"
size = "S"
claimed_at = "2026-10-27T01:30:52Z"
forced = false
evidence = "chain50 on main a871b2664 with the change (2a36515ed): fmt and clippy (roko-cli lib and the roko binary, -D warnings) green; 52 targeted tests pass, among them a_passed_cold_check_s_receipt_says_it_ran_cold (a passing cold delivery through the service: its receipt holds roko.delivery.build_trees@1 = {cold: true, started_from: 'a cold check: the regression checkout starts from no build trees and an empty cargo target dir'}; a failing warm one: {cold: false, started_from: 'a warm check: neither the regression checkout nor your checkout has build trees'}), recorded_batch_delivery_reads_only_a_delivered_plan (plan status reads the record) and every delivery and batch test; roko-cli lib suite 4070 passed, 0 failed (8 ignored, share_run_scrubs_output_text_and_prompt skipped). The $0 CLI proof ran on the pre-fix binary f9328a636 (runs/dc24dd-proof): both receipts lacked any summary, which this fixes"
+++

## Problem

`CliCompletionDeliveryService::deliver` (`crates/roko-cli/src/graph_execution/delivery.rs`) keeps a regression
outcome's `summary` only when the check fails: it becomes the receipt's `error`, and the warn line names it. On a
pass, the summary is dropped.
- The receipt gets `regression_evidence_ref` and the checks extension, and nothing more.
- The log line is `delivery regression passed delivery_id=…`, with no summary.

gap-dc24dd (f9328a636) put which build trees a check started from into two places. The check log's first line says
it, for example "[a cold check: the regression checkout starts from no build trees and an empty cargo target dir]".
A cold check's summary also ends ", from no build trees (a cold check)". So a passed cold check is recorded only in
the plan's check log. That log is `.roko/state/graph/<plan>/delivery-check.log`, which the plan's next delivery
overwrites.

Seen in roko-speed's $0 proof run `runs/dc24dd-before` (roko fe204e330, the delivered receipt in
`.roko/state/graph/cold-bench/checkpoint.json`): the receipt has no summary, and `roko.log` reads
`delivery regression passed delivery_id=20261027T012214Z-237e269a:cold-bench`. The code is unchanged at f9328a636.

## Why it matters

Truth. The claim that a build works from a clean clone rests on a cold ship check having run and passed. The run's
durable record, the checkpoint receipt that serve's run summary reads, cannot say so. gap-dc24dd's docs, item text
and commit message say a cold check's summary says so, which holds only for a failed check.

## Where

- `crates/roko-cli/src/graph_execution/delivery.rs`:
  - `CliCompletionDeliveryService::deliver`, the `Merged` arm, where a passed outcome advances to `RegressionPassed`.
  - `DeliveryRegressionOutcome`.
  - `GitDeliveryBackend::regression_output` and `prepare_build_trees`, which make the build-trees line.

## Plan

1. `DeliveryRegressionOutcome` carries what the check started from: whether it was cold, and the build-trees line.
2. `deliver` records it in every receipt whose check ran, passed or failed, as an extension, for example
   `roko.delivery.build_trees@1 = {"cold": true, "started_from": "…"}`.
3. The `delivery regression passed` log line carries the summary.
4. Correct gap-dc24dd's docs sentence if the wording changes.

## Done when

- A passed cold delivery's receipt in the checkpoint says it ran cold. A test drives a cold check that passes
  through the service and reads the receipt.
- The `[[verify]]` passes.

## What was built

- `DeliveryRegressionOutcome::build_trees` (`DeliveryBuildTrees`: `cold`, `started_from`) comes from the check log's
  first line. `CliCompletionDeliveryService::deliver` records it under `roko.delivery.build_trees@1` on every
  receipt whose check ran, whether it passed, failed or was stopped. The `delivery regression passed` line now
  carries the summary.
- Beyond the plan: `roko plan status` reads the record. Its text gains a `plan check from:` line, and its JSON
  gains `delivery.build_trees` (`RecordedBatchDelivery::build_trees`).
- The Warm Build Trees section of `docs/v3/depth/04-execution/worktree-isolation.md` now says where the record
  lives. gap-dc24dd's record has a dated correction.
- Tests: `a_passed_cold_check_s_receipt_says_it_ran_cold` (a passing cold delivery and a failing warm one through
  the service, each receipt read back) and `recorded_batch_delivery_reads_only_a_delivered_plan` (the record read
  from a checkpoint).

## Notes

- Found by roko-speed (round v7) while running the lead's $0 CLI proof of gap-dc24dd's cold check, 2026-10-27. It
  corrects what roko-speed told the lead: the receipt carries the summary only for a failed check.
- Related: gap-dc24dd.
