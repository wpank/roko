+++
id = "gap-c8e1f1"
kind = "gap"
title = "Split graph_task_dispatch.rs into modules without changing behaviour"
status = "open"
triage = "unverified"
severity = "p1"
goal = "tooling"
size = "M"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e15"
discovered_from = "tmp/cybernetic-harness/workstreams/assessment/W5-contention-parallelism.md (F4, rec 2a and 2d)"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs", "crates/roko-cli/src/graph_task_dispatch/"]
lane = "rust-hot"
parent = "spec-9a3131"
links = { depends_on = [], blocks = [], related = ["gap-96f7ed", "spec-e9d7ec"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f crates/roko-cli/src/graph_task_dispatch/verification.rs && test -f crates/roko-cli/src/graph_task_dispatch/feedback.rs && test -f crates/roko-cli/src/graph_task_dispatch/budget.rs && test -f crates/roko-cli/src/graph_task_dispatch/streaming.rs && test -f crates/roko-cli/src/graph_task_dispatch/failover.rs && test \"$(wc -l < crates/roko-cli/src/graph_task_dispatch.rs)\" -lt 3000 && cargo test -p roko-cli --lib graph_task_dispatch"
+++

## Problem

`crates/roko-cli/src/graph_task_dispatch.rs` is 7,899 lines, with its tests starting at line 5267. W5 assigns about 70
open pieces of work to it by anchor. Every Graph-dispatch change therefore queues on one file, and parallel branches
collide in it: on 09-28, four branches were editing it at once.

## Why it matters

After the split the file takes about four writers instead of one (W5 rec 2a). The split must land before E4.2
(gap-96f7ed) and the E2 dispatch fixes; otherwise they land in the monolith and conflict again. It is part of epic
spec-9a3131.

## Where

The spine stays in `graph_task_dispatch.rs`: the `GraphTaskDispatcher` struct and its builders (lines 1151–1552), and
`dispatch()` (3304–4109). The clusters move into `graph_task_dispatch/`, which already holds `retry_budget`,
`retry_feedback`, `sibling_settle` and `gate_output_accept`. Line numbers are at `41c7ffbd6`.

| New module | What moves | Lines |
|---|---|---|
| `verification` | `settle_task_verification`, the verify helpers | 1902–2945, 4110–4286 |
| `feedback` | `GraphFeedbackContext`, `emit_feedback` | 1051–1150, 1553–1901 |
| `budget` | the plan budget ledger and reservation, `GraphTaskSpendLedger` | 332–698 |
| `routing_context` | cheap-agent selection, routing bias, `build_routing_context` | 92–331, 3153–3303 |
| `turn_policy` | express tasks, turn caps, attempt timeouts, failure reasons | 699–828 |
| `inert_settings` | `InertGraphSetting`, `graph_engine_inert_settings` | 829–1050 |
| `tui_forward` | the TUI forwarding functions, the JSONL helpers | 2946–3152 |
| `streaming` | `dispatch_streaming`, `reconcile_attempt` | 4287–4766 |
| `failover` | `DispatchCandidate`, `run_bridge_with_failover`, `failover_*` | 4767–5266 |

## Current state

Unsplit at `41c7ffbd6`. Two in-flight worktrees hold uncommitted edits to this file:
- `../roko-wt-env` (bug-7d7200);
- `../roko-wt-learn-a` (`feat/learning-completion-loops`), which also adds `graph_task_dispatch/prompt_experiment.rs`.

## Plan

1. **Prepare.** Wait for both branches to merge, then claim a freeze of about a day with a single writer.
2. **Move the clusters.** Move each cluster as it is, keeping symbol names, the root file name and the public API;
   use `pub(super)` where needed. Tests move with the code they test.
3. **Keep behaviour unchanged.** The diff is moves plus `use` and visibility lines.
4. **Remap anchors in the same commit.** Point symbol anchors on moved symbols at their new files, and turn line
   anchors into symbol anchors.

## Done when

- [ ] The nine modules exist, and the root file is under 3,000 lines.
- [ ] The number of tests under `graph_task_dispatch` is the same before and after. Record both counts in the
      closing evidence.
- [ ] No open item keeps a line anchor into this file.
- [ ] The `[[verify]]` command passes.

## Notes

- This is a hot file: one writer only, and nothing else in flight on it.
- Keeping the root file name means spine anchors such as `graph_task_dispatch.rs::GraphTaskDispatcher::dispatch`
  stay valid.
