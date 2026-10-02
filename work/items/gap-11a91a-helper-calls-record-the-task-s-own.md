+++
id = "gap-11a91a"
kind = "gap"
title = "Helper calls record the task's own agent id, not a helper-specific one"
status = "open"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-cli/graph-task-dispatch"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "backlog wave reports 2026-10-02 (PK01 gap-625195, PK07 gap-f548c1)"
discovered_from = "gap-f548c1, gap-625195 (found during their work, not a listed backlog task)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/helper_calls.rs::write_side_call_rows"]
lane = "rust-hot"
parent = "spec-99d417"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn helper_call_rows_get_a_distinct_agent_id' crates/roko-cli/ && cargo test -p roko-cli helper_call_rows_get_a_distinct_agent_id"
+++

## Problem

A helper call's cost and efficiency rows are recorded with `agent_id: format!("{}/{task_id}", spec.plan_id)`
(`crates/roko-cli/src/graph_task_dispatch/helper_calls.rs::write_side_call_rows`, line 354), the same plan/task
identity used for the attempt's own dispatch rows. A verify-judge call, an error-diagnosis call, and a
gate-reflection call (the three helper calls `settle_helper_calls` accounts, lines 262-306) all get the identical
`agent_id` as the task's main attempt, even though `role = "helper"` (`HELPER_ROLE`, line 21) already distinguishes
them in the same row. Any consumer that groups or routes by `agent_id` cannot tell a helper call's cost from the
task's own attempt cost, or tell the three kinds of helper call apart from each other.

## Why it matters

Goal: truth (the attempt ledger should count tokens and money honestly, attributed to who actually made the call).
Found while working PK07's attempt-ledger package (gap-f548c1) and PK01's immune/attempt work (gap-625195), but not
one of either package's listed backlog tasks. Mis-attributed agent ids corrupt any per-agent cost rollup or
model-routing signal that keys off `agent_id` (e.g. `roko show costs`, cascade router inputs).

## Where

- `crates/roko-cli/src/graph_task_dispatch/helper_calls.rs::write_side_call_rows` (~line 309-360): builds the cost
  row and efficiency row for one helper/side call; sets `agent_id: format!("{}/{task_id}", spec.plan_id)`.
- Called from `settle_helper_calls` (line 262), once per completed helper call, with a label
  `format!("{attempt_key}/helper-{}", index + 1)` already computed for a different field right next to the
  `agent_id` assignment, but not reused for `agent_id` itself.

## Current state

`agent_id` is literally `"{plan_id}/{task_id}"`, identical to the task's own attempt rows. `role = "helper"` on the
same row is the only thing distinguishing it today; nothing distinguishes the three helper kinds or which attempt's
helper call (on retry) a row belongs to, beyond fields threaded separately.

## Plan

1. Give each helper call's row an `agent_id` that includes the helper's own identity, e.g.
   `format!("{}/{task_id}/helper-{}", spec.plan_id, index + 1)` (reusing the label already built at the
   `settle_helper_calls` call site), or the helper's model slug/kind (judge/diagnosis/reflection) if available here.
2. Add a unit test asserting a helper call's row has an `agent_id` distinguishable from its task's own attempt row
   and from other helper calls in the same batch.
3. Check any downstream reader of `agent_id` (cascade router inputs, `roko show costs`, dashboards) that assumes
   `agent_id == "{plan_id}/{task_id}"` for every row still groups sensibly once helper rows get a different format.

## Done when

- A helper call's cost/efficiency row has an `agent_id` that is not identical to its task's own attempt row's
  `agent_id`.
- The `[[verify]]` command passes.

## Notes

Don't change `role = "helper"` or the cost/efficiency row schema — only the `agent_id` value.
`crates/roko-cli/src/graph_task_dispatch/helper_calls.rs` is in the `rust-hot` lane; the hot dispatch files it
depends on (`graph_task_dispatch.rs`) are excluded from some packages' anchors so two hot packages can run at once —
check for a live claim before working this if it's picked up later.
