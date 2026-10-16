+++
id = "gap-c013c5"
kind = "gap"
title = "PK16 Defaults that apply: Three live golden-path runs pass (pass^3), with an independent review of each merged diff (+1 more)"
status = "done"
triage = "verified"
severity = "p1"
goal = "golden-path"
rank = 16
size = "M"
subsystem = ["self-hosting"]
created = 2026-10-02
updated = 2026-10-16
last_verified = 2026-10-16
last_verified_rev = "db6e21b00"
source = "tmp/backlog/2026-10-02-complete-and-wire PK16"
anchors = ["work/history/"]
lane = "rust-cold"
parent = "spec-fef7c5"
links = { depends_on = ["gap-625195", "gap-e00238", "gap-f548c1", "gap-997366", "gap-c1f4ac"], blocks = [], related = ["gap-f30b8e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "set -- work/history/golden-path-live-*.md; test -f \"$1\" && test \"$(grep -Eo 'graph-(run-)?[0-9a-f]+' \"$1\" | sort -u | wc -l)\" -ge 3 && grep -q '3 of 3 live runs passed' \"$1\""

[[verify]]
command = "set -- work/history/plan-set-live-*.md; test -f \"$1\" && grep -q -- '--max-parallel-plans 3' \"$1\" && grep -Eq 'graph-(run-)?[0-9a-f]' \"$1\" && grep -q 'all three plans started before the first finished' \"$1\""

[closed]
at = 2026-10-16
at_ts = "2026-10-16T15:05:03Z"
commit = "db6e21b00"
by = "L1-golden (c013 live operator)"
executor = "claude-agent"
via = "work-batch"
size = "M"
claimed_at = "2026-10-16T08:07:13Z"
forced = false
evidence = "Backlog 3118: the steps of golden_path_live ran 3 times at 5e32264ce on the D11 ladder (gpt-oss-120b, glm-4.7, gpt-5.4-mini, gpt-5.4). roko loaded the keys itself from ~/.roko/.env through a symlink in a scratch HOME; nobody read them. 3 of 3 live runs passed every check, with 0 interventions: graph-795d816a (230 s, $0.283094, $0.0315 per verified task, 0 escalations), graph-c0012506 (125 s, $0.282566, $0.0314, 0) and graph-6bf8aeb4 (316 s, $0.610246, $0.0678, 1 escalation: T5 cheap to mid). pass^3 on the live test: 3/3. In each run the tamper screen caught one edit of a pinned acceptance test; none landed. The reviews (Claude Opus, read-only) found low-severity escapes in all three runs: T5 ASCII-only upper-casing, T1 docstring rewrite, T4 doc example switched off. So W8's GP bar (the review finds no escape) is unmet: filed find-4003b9 (p1), gap-f9fa2c and gap-2dea57. Backlog 3119: plan set graph-3cc12de7 with --max-parallel-plans 3. All three plans started before the first finished (within 24 ms of each other, 9.2 s before the first finished), and 3 of 3 were delivered into roko/batch/20261016T144458Z-651be4f6 for $0.141548. Spend: $1.33 by the wire count ($1.32 by roko) of the $14 approved. ~/.roko unchanged (sha256 and file tree); no roko serve; no fence tripped. Records: work/history/golden-path-live-2026-10-16.md and work/history/plan-set-live-2026-10-16.md; write-up tmp/status-quo/2026-10-16-live/golden-path.md."
+++

## Problem

This package delivers 2 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK16, slice 31xx, phase 3), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 3118 | M | p1 | Three live golden-path runs pass (pass^3), with an independent review of each merged diff | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3118-golden-path-three-live-runs-pass.md` |
| 2 | 3119 | S | p2 | One live plan set: three independent plans run side by side under per-task worktrees | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3119-live-plan-set-three-plans-side-by-side.md` |

## Why it matters

Phase 3: golden-path proof. The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3100-defaults-that-apply-parallelism-ladder-integration.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `work/history/`.

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

- Waits on: PK01 (gap-625195), PK02 (gap-e00238), PK07 (gap-f548c1), PK14 (gap-997366), PK15 (gap-c1f4ac).
- On hold until Will takes the deferred decision(s) 3114 (spend or a public release); see `DECISIONS.md`.
- Existing work items this package covers or touches: gap-f30b8e. When its tasks are done, close those whose verify then passes.
- Suggested model: sonnet.

2026-10-15 (Will, 2026-10-15; tmp/status-quo/2026-10-15/DECISIONS-2026-10-15.md), decision 5: 3114 approved: $12 for the three golden-path runs ($4 each), $2 for the live plan set, $15 for the dogfood rerun, $29 in all. Hold lifted.
