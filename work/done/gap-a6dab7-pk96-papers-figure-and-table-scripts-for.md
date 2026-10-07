+++
id = "gap-a6dab7"
kind = "gap"
title = "PK96 Papers: Figure and table scripts for the P1 results (F2–F5, F9; T3, T7, T9, T11), dry-run on… (+5 more)"
status = "done"
triage = "verified"
severity = "p2"
goal = "whitepaper"
rank = 96
size = "L"
subsystem = ["companion"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "e74148090"
source = "tmp/backlog/2026-10-02-complete-and-wire PK96"
anchors = ["tmp/cybernetic-harness/companion-audit/E10-DRAFT.md"]
lane = "paper"
parent = "spec-6afe5c"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_p1_figures_dry_run' benchmarks/viabilitybench/analysis/test_figures_p1.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/test_figures_p1.py -q -k p1_figures_dry_run"

[[verify]]
command = "grep -qw 'def test_p2_figures_dry_run' benchmarks/viabilitybench/analysis/test_figures_p2.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/test_figures_p2.py -q -k p2_figures_dry_run"

[[verify]]
command = "python3 -m unittest discover -s tmp/cybernetic-harness/companion-audit/telemetry/scripts -p 'test_rater_stats.py' && grep -qw 'def test_rogan_gladen_fixed_values' tmp/cybernetic-harness/companion-audit/telemetry/scripts/test_rater_stats.py && grep -qw 'def test_gwet_ac1_fixed_values' tmp/cybernetic-harness/companion-audit/telemetry/scripts/test_rater_stats.py"

[[verify]]
command = "! grep -v '^>' tmp/cybernetic-harness/companion-audit/E10-DRAFT.md | grep -q '\\[\\[E4:' && grep -qE '^Result: (injected|not injected)' tmp/cybernetic-harness/companion-audit/research/E4-CANARY.md"

[[verify]]
command = "! grep -v '^>' tmp/cybernetic-harness/companion-audit/E10-DRAFT.md | grep -q '\\[\\[E11:' && test -f tmp/cybernetic-harness/companion-audit/figures/fig1-provenance.svg && test -f tmp/cybernetic-harness/companion-audit/figures/fig2-claims-funnel.svg && test -f tmp/cybernetic-harness/companion-audit/figures/fig3-citation-errors.svg"

[[verify]]
command = "python3 -c \"import csv,sys;r=list(csv.DictReader(open('tmp/cybernetic-harness/companion-audit/HUMAN-RATING-E6.csv')));sys.exit(0 if len(r)==150 else 1)\" && grep -qE '^Joined sessions: [0-9]+' tmp/cybernetic-harness/companion-audit/research/E6-TRANSCRIPT-JOIN.md"

[closed]
at = 2026-10-02
at_ts = "2026-10-02T16:52:36Z"
commit = "e74148090"
executor = "claude-agent"
via = "work-batch"
size = "L"
claimed_at = "2026-10-02T13:40:01Z"
forced = false
evidence = "Gate 2 (merged as e74148090): test_figures_p1 (7 passed) and test_figures_p2 (4 passed) in the bench venv; the four companion-audit verifies pass in the main checkout (untracked tmp/)."
+++

## Problem

This package delivers 6 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK96, slice 95xx, phase W), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 9513 | M | p2 | Figure and table scripts for the P1 results (F2–F5, F9; T3, T7, T9, T11), dry-run on synthetic MetricRecords | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9513-paper-p1-figure-and-table-scripts.md` |
| 2 | 9514 | M | p2 | Figure and table scripts for the P2 results (F6–F8, F10, F11; T4, T5, T8, T10), dry-run on synthetic records | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9514-paper-p2-figure-and-table-scripts.md` |
| 3 | 9517 | M | p2 | Companion: rater statistics for E2 and E3 (agreement, Rogan–Gladen, κ and AC1), tested on fixed data | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9517-companion-rater-statistics-e2-e3.md` |
| 4 | 9518 | M | p2 | Companion E4: a knowledge-injection canary at the audit tag, with opportunity counts and ε per loop | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9518-companion-e4-knowledge-injection-canary.md` |
| 5 | 9519 | M | p2 | Companion E11: draw Figures 1–3 (provenance, the claims funnel, citation errors) from the re-derived data | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9519-companion-e11-figures.md` |
| 6 | 9520 | M | p2 | Companion E6: the 150-commit rating sheet and the session-transcript join (derived counts only) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9520-companion-e6-rating-sheet-and-transcript-join.md` |

## Why it matters

Track W: the papers (alongside, after their inputs). The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9500-writing-papers-and-whitepaper-corrections.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `benchmarks/viabilitybench/analysis/fig_f10_loop_detection.py`, `benchmarks/viabilitybench/analysis/fig_f11_closures.py`, `benchmarks/viabilitybench/analysis/fig_f2_pareto.py`, `benchmarks/viabilitybench/analysis/fig_f3_envelope.py`, `benchmarks/viabilitybench/analysis/fig_f4_passk.py`, `benchmarks/viabilitybench/analysis/fig_f5_spec_interaction.py`, `benchmarks/viabilitybench/analysis/fig_f6_reliability.py`, `benchmarks/viabilitybench/analysis/fig_f7_saso.py`, `benchmarks/viabilitybench/analysis/fig_f8_audit.py`, `benchmarks/viabilitybench/analysis/fig_f9_routing.py`, `benchmarks/viabilitybench/analysis/figlib.py`, `benchmarks/viabilitybench/analysis/tab_t10_battery.py`, `benchmarks/viabilitybench/analysis/tab_t11_plan_slice.py`, `benchmarks/viabilitybench/analysis/tab_t3_headline.py`, `benchmarks/viabilitybench/analysis/tab_t4_loop_ledger.py`, `benchmarks/viabilitybench/analysis/tab_t5_contributions.py`, `benchmarks/viabilitybench/analysis/tab_t7_envelope.py`, `benchmarks/viabilitybench/analysis/tab_t8_saso.py`, `benchmarks/viabilitybench/analysis/tab_t9_costs.py`, `benchmarks/viabilitybench/analysis/test_figures_p1.py`, `benchmarks/viabilitybench/analysis/test_figures_p2.py`, `tmp/cybernetic-harness/companion-audit/E10-DRAFT.md`, `tmp/cybernetic-harness/companion-audit/HUMAN-RATING-E6.csv`, `tmp/cybernetic-harness/companion-audit/figures/fig1-provenance.svg`, `tmp/cybernetic-harness/companion-audit/figures/fig2-claims-funnel.svg`, `tmp/cybernetic-harness/companion-audit/figures/fig3-citation-errors.svg`, `tmp/cybernetic-harness/companion-audit/research/E4-CANARY.md`, `tmp/cybernetic-harness/companion-audit/research/E6-TRANSCRIPT-JOIN.md`, `tmp/cybernetic-harness/companion-audit/telemetry/scripts/companion_figures.py`, `tmp/cybernetic-harness/companion-audit/telemetry/scripts/e4_opportunities.py`, `tmp/cybernetic-harness/companion-audit/telemetry/scripts/e6_transcript_join.py`, `tmp/cybernetic-harness/companion-audit/telemetry/scripts/rater_stats.py`, `tmp/cybernetic-harness/companion-audit/telemetry/scripts/test_rater_stats.py`.

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

- Waits on: nothing.
- Suggested model: opus.

## Progress

- 9513: implemented at 50fb07a8c (figlib.py and the nine P1 scripts; `test_figures_p1.py`, 7 tests, verify passes)
- 9514: implemented at 14ab371e9 (the nine P2 scripts, a small figlib extension; `test_figures_p2.py`, 4 tests, verify passes)
- 9517: implemented at 9d0581ad4 (untracked files in MAIN's `tmp/`, recorded by an empty commit; 8 unittest tests, verify passes)
- 9518: implemented at d0c46a686 by plan option (b), a static trace (`Result: injected`); option (a), the planted-marker run, needs a cargo build at the tag and is still open; verify passes
- 9519: implemented at 1f3580661 (three SVGs, every plotted total checked against E1; verify passes)
- 9520: implemented at e64a6d615 (150-row sheet; join: 1,669 / 2,098 sessions joined at 1 h / 24 h; verify passes)
