+++
id = "gap-ed1a08"
kind = "gap"
title = "PK67 ViabilityBench proof: Replay R-H6: S06's controllers A0-A5 plus A3-gated and A3-mis (+2 more)"
status = "done"
triage = "verified"
severity = "p1"
goal = "proof"
rank = 67
size = "L"
subsystem = ["benchmarks/viabilitybench/analysis"]
created = 2026-10-02
updated = 2026-10-04
last_verified = 2026-10-04
last_verified_rev = "0174ea3e9"
source = "tmp/backlog/2026-10-02-complete-and-wire PK67"
anchors = ["benchmarks/viabilitybench/driver/planemit.py"]
lane = "bench"
parent = "spec-635697"
links = { depends_on = ["gap-cb5133", "gap-de0b87", "gap-1149aa", "gap-c06ff3", "gap-2ca903", "gap-c1d920", "gap-85d176", "gap-7ec3ef", "gap-63fd4c", "gap-0c429f", "gap-147c4d", "gap-f7bab8", "gap-414e56"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_r_h6_replay_runs_every_controller_and_hook' benchmarks/viabilitybench/analysis/test_replay_h6.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/test_replay_h6.py -k test_r_h6_replay_runs_every_controller_and_hook -q"

[[verify]]
command = "grep -qw 'def test_closure_replays_emit_their_preregistered_estimands' benchmarks/viabilitybench/analysis/test_replay_closure.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/test_replay_closure.py -k test_closure_replays_emit_their_preregistered_estimands -q"

[[verify]]
command = "grep -qw 'def test_roko_full_overlay_turns_on_gate_routing_audits_and_holdout' benchmarks/viabilitybench/driver/test_planemit.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_planemit.py -k test_roko_full_overlay_turns_on_gate_routing_audits_and_holdout -q"

[closed]
at = 2026-10-04
at_ts = "2026-10-04T00:04:35Z"
commit = "0174ea3e9"
executor = "claude-agent"
via = "work-batch"
size = "L"
claimed_at = "2026-10-03T23:12:34Z"
forced = false
evidence = "Gate 12a (work/backlog-batch-12a, merged into main as 0174ea3e9; Python only, no Rust changed): the ViabilityBench suite 702 passed, 2 skipped; every [[verify]] passes. PK67 3/3: replay_h6.py (R-H6, evaluated:false until 8117's evaluator has an entry point), replay_closure.py (X1 computed with a bootstrap CI; X2-X4 report why not), arms/roko_full.toml with planemit's overlay, now passed through run_roko._plan_spec (e9738240f)."
+++

## Problem

This package delivers 3 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK67, slice 33xx, phase 8), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 3357 | S | p2 | Replay R-H6: S06's controllers A0-A5 plus A3-gated and A3-mis | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3357-replay-r-h6-controllers.md` |
| 2 | 3358 | M | p2 | Closure replays X1-X4, pre-registered as exploratory | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3358-closure-replays-x1-to-x4.md` |
| 3 | 3360 | M | p1 | The roko_full arm overlay: the ladder plus S07's gate, S04 routing, S05 audits and the holdout | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3360-roko-full-arm-overlay.md` |

## Why it matters

Phase 8: M1 controller and guarded commit (S06). The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3300-the-proof-viabilitybench-pilots-log1-and-head-to-head.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `benchmarks/viabilitybench/analysis/replay_closure.py`, `benchmarks/viabilitybench/analysis/replay_h6.py`, `benchmarks/viabilitybench/analysis/test_replay_closure.py`, `benchmarks/viabilitybench/analysis/test_replay_h6.py`, `benchmarks/viabilitybench/arms/roko_full.toml`, `benchmarks/viabilitybench/driver/planemit.py`, `benchmarks/viabilitybench/driver/test_planemit.py`.

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

- Waits on: PK17 (gap-cb5133), PK19 (gap-de0b87), PK21 (gap-1149aa), PK28 (gap-c06ff3), PK30 (gap-2ca903), PK43 (gap-c1d920), PK44 (gap-85d176), PK49 (gap-7ec3ef), PK50 (gap-63fd4c), PK52 (gap-0c429f), PK59 (gap-147c4d), PK62 (gap-f7bab8), PK66 (gap-414e56).
- Suggested model: sonnet.

## Progress

- 3357: implemented at d828e8486. New replay_h6.py (8 arms A0-A5/A3-gated/A3-mis x 6 disturbance kinds from
  driver/disturb.py's KINDS), reading an optional `--table` of `roko_learn::homeostasis::replay::ArmReport`-shaped
  JSON lines. 8117 (S06.T7) built that evaluator as a roko-learn library function with no CLI or subprocess entry
  point (grepped roko-cli and roko-learn/examples/: nothing calls it), so every cell is `evaluated: false` with
  that reason until one exists, the same pattern replay_h4.py/replay_h5.py already use for a mechanism that has
  not run on the matrix. Verify passes.
- 3358: implemented at af098523a. New replay_closure.py: X1 (delta_brier(gate_labels, ipw_audit_labels | y_vs))
  is real, with a bootstrap 95% CI, reusing replay_h5.units() and replay_h4.brier_score over a previously-written
  R-H5 document (`--h5`). X2 (delta_iae(A4, A3), guarded by A3-mis) reads its cells from a previously-written
  R-H6 document (`--h6`); since every H6 cell is unevaluated (3357), X2 stays not evaluated per kind too, and its
  own ci95_excludes_0 rule is flagged separately not evaluated even once real (no paired bootstrap draws in
  ArmReport's summary). X3 (per-position lottery breach timing) and X4 (no_gate/always_refine/always_escalate
  policy cells) have no data behind them in this codebase, so both report evaluated: false with why. Closure 4
  (H7's E0) reuses campaign.census_report/is_loop_live (3359) rather than reimplementing the loop census. Verify
  passes.
- 3360: implemented at 5deb293c4. New arms/roko_full.toml (BL6): roko_ladder's 3-rung pool plus an `[overlay]`
  table (gate_mode=enforce D14, audit_floor=0.05 D13, routing_mode=active + routing_policy=lcb_aci S04,
  holdout=0.10 D10, homeostasis=if_live S06). planemit.py gained `PlanSpec.overlay`, `resolve_overlay()` (refuses
  an unknown key; resolves if_live via campaign.census_report/is_loop_live, faked in the test) and
  `_overlay_text()`, appended after the existing templates so an empty overlay (roko_fixed, roko_ladder) stays
  byte for byte unchanged (TEMPLATE_SHA256 and the golden fixtures untouched); `_check`/`_check_ladder` verify the
  overlay's tables round-trip. Verify passes. Known gap, outside this task's files: run_roko.py's `_plan_spec()`
  does not yet pass the arm's `[overlay]` table into PlanSpec (confirmed by reading it: no `overlay=` kwarg in
  its `PlanSpec(...)` call), so a real roko_full dispatch still builds an empty overlay until that one-line change
  lands; flagged for the coordinator, not fixed here since driver/run_roko.py is not in this task's `files`.
