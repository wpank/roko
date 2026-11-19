+++
id = "gap-1ee8c4"
kind = "gap"
title = "loop_health carries β's estimate without its interval or arm counts, and roko learn loops prints no ε, ι or β, so nobody sees a loop's benefit being measured"
status = "done"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-cli/commands"]
created = 2026-10-16
updated = 2026-10-21
last_verified = 2026-10-21
last_verified_rev = "d75efe323"
source = "round-v6-learning-research#4b.9"
discovered_from = "agent:learning-research (round v6, 2026-10-16)"
anchors = ["crates/roko-core/src/dashboard_snapshot.rs::LoopHealth", "crates/roko-cli/src/graph_execution/loop_audit.rs::audit_tick", "crates/roko-cli/src/commands/learn_loops.rs"]
lane = "rust-hot"
parent = "spec-7a71d6"
links = { depends_on = [], blocks = [], related = ["gap-ca63ee", "bug-550909"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn loop_health_event_carries_beta_interval' crates/roko-cli/ && cargo test -p roko-cli loop_health_event_carries_beta_interval"

[closed]
at = 2026-10-21
at_ts = "2026-10-21T03:53:01Z"
commit = "d75efe323"
by = "commit trailer"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-21T01:31:02Z"
forced = false
evidence = "d75efe323. roko-core dashboard_snapshot.rs: DashboardEvent::LoopHealth carries beta_lcb, beta_ucb, beta_reason, n_learned and n_default as its own JSON fields, through a boxed, flattened LoopHealthDetail (as RouteDecided carries its ladder), all optional so an event written before them parses; boxed because as plain fields they grew the event enum to 264 bytes, which clippy's large_enum_variant caught on state_hub_ipc.rs HubIpcMessage. roko-cli graph_execution/loop_audit.rs loop_event fills them from the ledger health row the tick appends. roko-learn census.rs render_text, the roko learn loops table, adds n_L/n_D, ε, ι_net and β [LCB, UCB] columns (β's says not judged (<reason>) until it is), each column as wide as its widest cell; --json already carried the same numbers in rows[].measured. Checks: the verify command exits 0 (loop_health_event_carries_beta_interval ok, 77 test binaries, 0 failures); roko-cli lib learn_loops + loop_audit 8/8 incl. learn_loops_table_prints_beta_interval; roko-core loop_events_serialize_with_kind (JSON names, older event parses); roko-learn census 17/17; clippy -p roko-core -p roko-learn -p roko-cli --no-deps -D warnings clean; nightly fmt. CLI: target/debug/roko learn loops on a copy of the round-v6 audit demo's 24 runs (tmp/hermes-roko-v6/learning-research/evidence/audit-demo/workspace-runs, [learning.audit] n_beta = 12) prints L-know 13/30, ε 1.00, ι_net 1.00, β 0.976 [-2.301, 4.254] and L-play 26/17, β -0.502 [-3.844, 2.839]; --json gives L-know n_L 13, n_D 30, beta est 0.976, lcb -2.301, ucb 4.254. Display only: how β is judged is unchanged. Filed gap-2cc5be (GET /api/learn/loops rows still carry β̂ alone)."
+++

## Problem

Since gap-03fcd0 the loop audit estimates each loop's benefit β, and the ledger keeps the whole estimate
(`.roko/learn/loop-audit.jsonl` `beta {est, lcb, ucb, reason}`, plus `n_L`, `n_D`). Neither surface people look at
shows it:
- the `loop_health` hub event carries `beta` (the point estimate) only, with no interval and no arm counts;
- `roko learn loops` prints only state, reason, qualifiers and evidence, with no ε, ι or β.

Seen in round v6's audit demo (24 one-task plans, $0 scripted model, L-know holdout 0.5, `n_beta = 12`;
`tmp/hermes-roko-v6/learning-research/evidence/audit-demo/`):
- the ledger's L-know rows went β̂ 0.84 → 0.976 with LCB(β) −8.58 → −2.30 over 12 → 43 opportunities (13 learned, 30
  default chains);
- chains that saw the lesson cost $0.0038 and took 1 attempt, against $0.0159 and 3 attempts when the arm withheld it.

That is exactly the evidence that shows a loop being measured. Yet the plugin's L4 ring shows "lessons in prompts: on
probation · holdout 20% · reach 100%" and the CLI shows "probation".

## Why it matters

Goals `visibility` and `cybernetic`:
- M2 needs about 200 opportunities per loop before it can move one, so in any demo or young workspace "probation" is
  all a person sees;
- the narrowing interval is the only visible sign that the audit level works.

Related:
- gap-ca63ee: the census judges with default parameters, so the CLI and the tick disagree;
- the plugin's L4 text, a plugin-side request filed locally.

## Where

- `crates/roko-core/src/dashboard_snapshot.rs::LoopHealth`: the event's fields.
- `crates/roko-cli/src/graph_execution/loop_audit.rs::audit_tick`: publishes `loop_health` from the ledger rows it
  writes.
- `crates/roko-cli/src/commands/learn_loops.rs`: the census table.

## Current state

The numbers exist in the tick's ledger row. Only the event and the table drop them.

## Plan

1. Add `beta_lcb`, `beta_ucb`, `beta_reason`, `n_learned` and `n_default` to `LoopHealth` (all optional, so older
   readers keep working). Fill them from the ledger row.
2. Add ε̂, ι_net and β̂ [LCB, UCB] columns, with n per arm, to `roko learn loops` (`--json` too).
3. Test `loop_health_event_carries_beta_interval`: a tick over a fixture ledger with a judged β publishes the interval
   and the arm counts.

## Done when

The `[[verify]]` command passes, and `roko learn loops` prints β with its interval for a loop past `n_beta`.

## Notes

Don't change how β is judged. This is display only.
