+++
id = "gap-1ee8c4"
kind = "gap"
title = "loop_health carries β's estimate without its interval or arm counts, and roko learn loops prints no ε, ι or β, so nobody sees a loop's benefit being measured"
status = "open"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-cli/commands"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
source = "round-v6-learning-research#4b.9"
discovered_from = "agent:learning-research (round v6, 2026-10-16)"
anchors = ["crates/roko-core/src/dashboard_snapshot.rs::LoopHealth", "crates/roko-cli/src/graph_execution/loop_audit.rs::audit_tick", "crates/roko-cli/src/commands/learn_loops.rs"]
lane = "rust-hot"
parent = "spec-7a71d6"
links = { depends_on = [], blocks = [], related = ["gap-ca63ee", "bug-550909"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn loop_health_event_carries_beta_interval' crates/roko-cli/ && cargo test -p roko-cli loop_health_event_carries_beta_interval"
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
