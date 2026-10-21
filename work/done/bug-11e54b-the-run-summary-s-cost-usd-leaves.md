+++
id = "bug-11e54b"
kind = "bug"
title = "The run summary's cost_usd leaves out the failover_refused calls the ledger bills to the plan (phase 2.2 run 20c780ae: summary $0.6915, ledger $0.7185; the $0.027 is 4 glm-4.7 calls cut off by 429s)"
status = "done"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-serve/routes"]
created = 2026-10-21
updated = 2026-10-21
last_verified = 2026-10-21
last_verified_rev = "80be517d1"
source = "round-v7-hermes-pilot#L-p2-2-run-20c780ae"
discovered_from = "agent:hermes-pilot (round v7 phase 2.2 run 20c780ae)"
anchors = ["crates/roko-serve/src/routes/runs.rs::get_run_summary", "crates/roko-cli/src/graph_task_dispatch/helper_calls.rs::write_side_call_rows", "crates/roko-core/src/dashboard_snapshot.rs::side_call_role", "crates/roko-cli/src/graph_task_dispatch/failover.rs::FAILOVER_REFUSED_ROLE"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_runs_summary_cost_includes_its_failover_refused_calls' crates/roko-serve/src && cargo test -p roko-serve --lib -- a_runs_summary_cost_includes_its_failover_refused_calls"

[closed]
at = 2026-10-21
at_ts = "2026-10-21T01:08:43Z"
commit = "80be517d1"
by = "roko-speed"
executor = "claude-agent"
via = "manual"
size = "S"
claimed_at = "2026-10-21T00:07:49Z"
forced = false
evidence = "80be517d1. Each side call (a call its provider refused before the try failed over, a helper call, an inline audit check, an audit worker's call) now reaches the hub from SideCallRows::write (helper_calls.rs) as an efficiency_event with metric side_call_usd.<role>, beside its cost and efficiency rows. roko-core has the prefix, side_call_usd_metric, side_call_role and FAILOVER_REFUSED_SIDE_CALL, the string failover.rs's FAILOVER_REFUSED_ROLE writes (failover.rs is unchanged). The run summary (routes/runs.rs) adds side calls to a Graph run's cost_usd and lists them by role as side_call_usd, in its metrics and in GET /api/runs/{id}/summary; a running try's refused calls, which its call_cost figure holds too, count once. The dashboard snapshot counts side calls in the agent's, the plan's and the total spend. The literal verify passes: a_runs_summary_cost_includes_its_failover_refused_calls (run 20c780ae's rows, tries $0.4412 + $0.2503, four refused glm-4.7 calls $0.027 and five $0 helper calls: cost_usd $0.7185 with side_call_usd {failover_refused 0.027, helper 0}; a running try whose refused call cost $0.0068 and whose substitute has spent $0.01 so far reads $0.0168, not $0.0236, and $0.0168 once it settles). Also a_side_calls_cost_reaches_the_hub (roko-cli: a helper call after a failed gate reaches the hub as side_call_usd.helper, and the hub's spend equals the plan ledger's $0.02014) and side_call_costs_count_in_the_plans_spend (roko-core). Full lib suites: roko-cli 3951, roko-core 2046, roko-learn 1391, roko-serve 1076 of 1077 (bug-904a1c's known JWKS flake); fmt and clippy clean."
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
