+++
id = "bug-11e54b"
kind = "bug"
title = "The run summary's cost_usd leaves out the failover_refused calls the ledger bills to the plan (phase 2.2 run 20c780ae: summary $0.6915, ledger $0.7185; the $0.027 is 4 glm-4.7 calls cut off by 429s)"
status = "open"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-serve/routes"]
created = 2026-10-21
updated = 2026-10-21
last_verified = 2026-10-21
source = "round-v7-hermes-pilot#L-p2-2-run-20c780ae"
discovered_from = "agent:hermes-pilot (round v7 phase 2.2 run 20c780ae)"
anchors = ["crates/roko-serve/src/routes/runs.rs::get_run_summary", "crates/roko-cli/src/graph_task_dispatch/failover.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_runs_summary_cost_includes_its_failover_refused_calls' crates/roko-serve/src && cargo test -p roko-serve --lib -- a_runs_summary_cost_includes_its_failover_refused_calls"
+++

## Problem

The run summary's `cost_usd` leaves out the `failover_refused` calls that roko's cost ledger bills to the plan, so
the two disagree.

Round v7, Undertow phase 2.2, run `20c780ae` (roko e4643dafd):

- `GET /api/runs/20c780ae…/summary` says `cost_usd` = $0.6915, the sum of the 8 tries.
- `.roko/learn/costs.jsonl` bills the plan $0.7185 over the run:

  | role | model | rows | cost |
  |---|---|---|---|
  | implementer | glm-4.7 | 3 | $0.4412 |
  | implementer | gpt-5.4-mini | 5 | $0.2503 |
  | `failover_refused` | glm-4.7 | 4 | $0.0270 |
  | helper | gpt-oss-120b | 5 | $0.0000 (402s) |

The $0.027 difference is 4 glm-4.7 calls that Z.ai cut off with a 429 before roko failed the try over. For example,
T2#1 ran about 2 minutes on glm-4.7 first. The provider billed them, and the ledger charges them to the plan, but the
summary, and so the plugin's run spend and receipt, never show them.

## Where

- `crates/roko-cli/src/graph_task_dispatch/failover.rs` (~line 507, `FAILOVER_REFUSED_ROLE`): a refused call's cost
  and efficiency rows are written to the ledger.
- `crates/roko-serve/src/routes/runs.rs`, the summary loop (~lines 2196–2230): `cost_usd` adds `inference_completed` or
  `agent_completed` costs, or takes a final `total_cost_usd`. No event carries the refused calls' cost.

## Plan

Add the refused calls to the summary's cost. Either publish an event with each refused call's cost and add it, or show
it on its own line (`refused_usd`) beside `cost_usd`, so the summary agrees with the ledger.

## Done when

A run with failover-refused calls has a summary cost that agrees with its plan's ledger rows, or one that names the
refused part. Checked by the test in `[[verify]]`.
