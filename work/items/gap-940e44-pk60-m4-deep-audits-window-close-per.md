+++
id = "gap-940e44"
kind = "gap"
title = "PK60 M4 deep audits: Window close: per-stratum estimates of false greens, gaming and weak oracles, and the… (+4 more)"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
rank = 60
size = "L"
subsystem = ["roko-cli/audit"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK60"
anchors = ["CLAUDE.md", "crates/roko-cli/src/commands/mod.rs", "crates/roko-cli/src/graph_task_dispatch/routing_context.rs", "crates/roko-cli/src/main.rs", "crates/roko-learn/src/cascade_router.rs"]
lane = "rust-cold"
parent = "spec-c3abc8"
links = { depends_on = ["gap-7ec3ef", "gap-f0a7ee", "gap-147c4d"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn ladder_steps_up_on_ucb_and_down_after_two_quiet_windows' crates/roko-gate/src/audit/ && cargo test -p roko-gate --lib ladder_steps_up_on_ucb_and_down_after_two_quiet_windows"

[[verify]]
command = "grep -qw 'fn audit_trust_excludes_a_gaming_model_but_keeps_one_per_class' crates/roko-learn/src/cascade_router.rs && cargo test -p roko-learn --lib audit_trust_excludes_a_gaming_model_but_keeps_one_per_class"

[[verify]]
command = "grep -rqw 'fn audit_labels_correct_the_self_model_with_ipw_weights' crates/roko-cli/src/ && cargo test -p roko-cli --lib audit_labels_correct_the_self_model_with_ipw_weights"

[[verify]]
command = "grep -rqw 'fn a_confirmed_false_green_opens_an_incident_with_a_fix_proposal' crates/roko-cli/src/ && cargo test -p roko-cli --lib a_confirmed_false_green_opens_an_incident_with_a_fix_proposal"

[[verify]]
command = "grep -qw 'fn audit_reveal_recomputes_every_draw' crates/roko-cli/src/commands/audit.rs && cargo test -p roko-cli --bin roko audit_reveal_recomputes_every_draw"
+++

## Problem

This package delivers 5 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK60, slice 71xx, phase 7), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 7131 | M | p2 | Window close: per-stratum estimates of false greens, gaming and weak oracles, and the DP3 ladder state (S05 task 12, part 1) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/7131-window-estimates-and-dp3-ladder-state.md` |
| 2 | 7133 | M | p2 | DP4: routing trust from audit estimates in CascadeRouter::filter_unhealthy (S05 task 12, part 3) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/7133-dp4-routing-trust-from-audit-estimates.md` |
| 3 | 7134 | S | p2 | DP5: audit labels train the self-model with inverse-propensity weights (S05 task 12, part 4) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/7134-dp5-audit-labels-train-the-self-model.md` |
| 4 | 7135 | M | p2 | DP6: incident records, a fix-task proposal and operator-confirmed isolation (S05 task 12, part 5) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/7135-dp6-incidents-and-fix-proposals.md` |
| 5 | 7136 | S | p2 | roko audit status / replay / reveal / incidents, all read-only (S05 task 14) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/7136-roko-audit-read-only-cli.md` |

## Why it matters

Phase 7: M4 random deep audits (S05). The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/7100-m4-random-deep-audits.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `CLAUDE.md`, `crates/roko-cli/src/audit/labels.rs`, `crates/roko-cli/src/audit/mod.rs`, `crates/roko-cli/src/audit/worker.rs`, `crates/roko-cli/src/commands/audit.rs`, `crates/roko-cli/src/commands/mod.rs`, `crates/roko-cli/src/graph_task_dispatch/routing_context.rs`, `crates/roko-cli/src/main.rs`, `crates/roko-gate/src/audit/feedback.rs`, `crates/roko-gate/src/audit/incident.rs`, `crates/roko-gate/src/audit/mod.rs`, `crates/roko-gate/src/audit/window.rs`, `crates/roko-learn/src/cascade_router.rs`.

## Current state

The tasks were checked against `2c3ea9f73` on 2026-10-02. Re-check each task's anchors and premise at your base commit before implementing it, and report a task that is already done instead of redoing it.

## Plan

1. Work through the tasks in the order above. For each: read its file, implement its Plan, write the test it names, and make one commit per task whose message ends with `Backlog-Task: <task id>`, `Work-Item: <this item's id>` and `Executor: claude-agent`.
2. Follow `BUILD-RULES.md` in the backlog folder. Workers run no cargo: Rust is checked by the coordinator's batched gate. Python and doc checks you may run.
3. If a task cannot be done (a premise is false, a decision is missing, or its verify cannot pass), stop at that task, keep the earlier commits, and report it; do not skip ahead to tasks that depend on it.

## Done when

- Every task's verify command passes (this item's `[[verify]]` list, one entry per task), after the coordinator's batched gate.
- Each task's own "Done when" holds (see its file).

## Notes

- Waits on: PK49 (gap-7ec3ef), PK58 (gap-f0a7ee), PK59 (gap-147c4d).
- Suggested model: opus.

## Progress

Implemented on `work/gap-940e44`; cargo verification deferred to the batch gate. `773e82eef` applies lint fixes to
7131, 7134 and 7135.

- 7131: implemented at `c3c8c2715`. `roko_gate::audit::window` (window close, per-stratum Hájek/Wilson estimates and
  null bounds) and `audit::feedback` (the DP3 ladder in the vault's `ladder.json`, the trust book in `trust.json`,
  `close_due_windows`). The worker closes due windows after each result and at drain. One reading to check: a
  window with no known label of a task type doesn't step the ladder.
- 7133: implemented at `4c40a2b28`. DP4 is in `CascadeRouter`: `set_audit_trust`, `filter_untrusted`, used by
  `filter_unhealthy` and the health-scored route. Trust is loaded at plan start from `routing_context.rs`, through a
  one-line call in `attempt.rs`. An exclusion is a route decision candidate with `ineligible_reason = "audit_trust"`,
  which needed a small additive edit in `dispatch/model_routing.rs`.
- 7134: implemented at `429f9276b`. `audit/labels.rs` writes `vs.label` rows (a new `vs.label` ledger event, plus
  `.roko/runs/<run>/labels.jsonl`). A known VS teaches the run's self-model through 6129's `observe_label` with
  weight 1/π. The plan's audit selector passes the self-model to the worker (`audit_select.rs`, `attempt.rs`).
- 7135: implemented at `46ddf7882`. `roko_gate::audit::incident`: incident records with their status history, fix
  proposals (`<id>.task.toml`, option (b)) and isolation proposals that an operator confirms. The worker opens
  incidents for confirmed Y/G findings and for gate-gaming alerts. Each new incident downgrades the pair's routing
  trust at once.
- 7136: implemented at `5a51212dc`. `commands/audit.rs` (`status [--strata]`, `replay --runs N`, `reveal <run_id>`,
  `incidents`) is registered as `Command::Audit`. I left CLAUDE.md alone, since its row edits are held in gap-3698cd
  while main carries an uncommitted CLAUDE.md edit. The row for the "Learning & feedback" table is:
  `` | `roko audit status/replay/reveal/incidents` | M4 audits, read-only: window estimates, ladder and trust, lottery replay, a run's draws recomputed from its revealed key, incidents | ``
