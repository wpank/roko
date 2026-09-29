Status: draft · budget 700 words · owner gap-29a64e

# 7 Field evidence

Roko has built most of its own web portal. This section reports that build from Roko's records and from the notes
of the frontier sessions that supervised it. The evidence is observational: nothing was randomized, one model ran
every portal task, and a frontier session supervised the build, so no number here supports a causal claim or a
comparison with the benchmark of §8. The quoted inputs are frozen, with their sha256, in
[`evidence/`](evidence/README.md).

## 7.1 The portal build

Frontier Claude Code sessions wrote 16 plans for the portal's server and web app, and Roko ran them.

| Measure | Portal plans 01 to 08f[^7-portal] |
|---|---|
| Plans and tasks | 16 plans, 173 tasks, 168 of them passed by their authored gates |
| Output | About 14.4k lines of code and 645 tests |
| Attempts | 210, all on one pinned model, `claude-sonnet-4-6` |
| First-try passes after the 09-28 verdict fix | 138 of the 148 tasks run in the four lanes |
| Agent spend, as recorded | $174.87 over 25.6 agent-hours; a median of $0.83 and a p90 of $1.83 per task |
| Concurrency | A mean of 1.2–2.2 tasks at once within a plan (caps 2–4); across plans, a mean of 1.59 agents, peak 5 |

Table: The portal build, as Roko recorded it.

The parallelism across plans came from four worktrees the operator made by hand, not from Roko's scheduler.

## 7.2 Four cases

Four of the eight frozen cases show who closed each loop.

- **CASE-001: a false green, then honest verdicts.** A task that failed its verify step three times was
  force-accepted at the review-cycle cap and logged as passed. Before the operator's engine fix of 09-28, 101 of 373
  recorded successes (27%) had a failing gate; after it, 0 of 151.[^7-verdicts] That is honesty about the visible
  checks, not an audited false-green rate (§6).
- **CASE-005: merges made by hand.** The operator built four worktrees with pinned binaries, pruned the disk twice
  to fit them, and merged five times by hand. Three merges had semantic breaks that the text merge could not see,
  and 59 files needed formatting because no lane gate ran the formatter.[^7-merges]
- **CASE-006: gates green, product unusable.** Plans 05 to 08 passed every typecheck, unit-test and build gate. A
  browser pass against a real server then found about 20 defects, among them a blocker: nothing could run twice.
  Later passes found about 12, 5 and 4, and a smoke test 3 more. Roko built fixes from plans that the operator's
  subagents wrote from the findings.[^7-browser]
- **CASE-007: plan defects that only frontier audits caught.** Two audit rounds made about 65 edits to the first 87
  tasks. They found a plan that crossed a crate boundary the wrong way, two gates that correct code would fail, two
  vacuous gates, and a regression already shipped past a single-file gate. Roko's own validator caught only the
  references to a gitignored file, and only when run in a worktree.[^7-audits]

## 7.3 Who closed the loop, and at what cost

Each finished plan run is snapshotted, most of them after the fact, and the supervisors log interventions and
escapes (defects found after every gate passed) as notes. The rollup of 2026-09-29T14:37:51 covers 42 runs and 124
notes from 08-22 to 09-29. Its 158 tasks with checkpoint verdicts all passed, at $192.92 as recorded, or $1.22 per
verified task. It logs 39 interventions by people against 2 recoveries Roko made on its own: an autonomy index of
2/41 (5%). Its 11 escapes linked to a run are 7% of verified passes; 22 more could not be tied to a captured
run.[^7-rollup]

The supervising sessions did the rest: about 20 subagents wrote and audited the plans, fixed about 25 engine
defects, built the worktrees, merged and supervised. Roko's records hold none of their cost. Priced from token
counts, the Claude sessions of 09-25 to 09-29 come to about $2.7–3.4k API-equivalent, about 16–20× the $172.80 Roko
recorded over the same days.[^7-operator] The estimate includes the research programme and its papers, so it
overstates what supervising the portal cost; measured figures await the transcript harvest (gap-263de5,
gap-ccb87e).

## 7.4 What this does and does not show

The portal shows Roko carrying a real multi-plan build with honest per-task verdicts since the 09-28 fix, and it
shows where regulation still sits with people: auditing plans, isolating and merging lanes, checking the product and
repairing the engine. It does not show that Roko is cheaper than working in a frontier session directly, since
nobody has run that comparison; that cheap models suffice, since none ran a portal task; or that learning helped,
since the router saw only one model. §8 describes the controlled test, and §9 the limitations.

[^7-portal]: Research note B7 of 2026-09-29, frozen as `evidence/2026-09-29-b7-real-run-evidence.md` (sha256
    `799b6a2b6184`): "TL;DR", the section on parallel plan execution and "Corrections". Roko's attempt, gate and
    cost records for portal plans 01 to 08f, 09-05 to 2026-09-29 07:41Z; first-try passes after the fix in
    `725f21e05`; cross-plan concurrency from 09-28 11:42Z. Spend is Roko's ledger, which has no cost source, and
    excludes the supervising sessions. The four lanes merged in `3d0ee4d02`, `f7c542b5a`, `98e0af4d6`, `4ca38b5c8`
    and `188c43c8d`.

[^7-verdicts]: B7 (`evidence/2026-09-29-b7-real-run-evidence.md`), "Method" and "TL;DR", and CASE-001 in
    `evidence/2026-09-29-field-cases.md` (sha256 `d03e50476fb0`). Before: 430 attempts in 31 plans, from 09-05 to
    the fix in `725f21e05`, which deleted force-accept, checkpoints each verdict and settles episodes after the gate.
    After: 168 attempts, to 2026-09-29 07:41Z. The fix closed bug-82d47b, bug-521f08 and bug-06e2d1.

[^7-merges]: CASE-005 in `evidence/2026-09-29-field-cases.md`: the merges `3d0ee4d02`, `f7c542b5a`, `98e0af4d6`,
    `4ca38b5c8` and `188c43c8d`, on 09-28 and 09-29. The items for Roko's own worktree acceptance and merge step are
    gap-3b5361 and bug-a3760a.

[^7-browser]: CASE-006 in `evidence/2026-09-29-field-cases.md`; plans 05 to 08g, 09-28 and 09-29. Roko's fixes
    landed in `54704f451` (08b), `dabd1975f` (08d), `2bf9b05bb` (08e) and `a17d9d766` (08g).

[^7-audits]: CASE-007 in `evidence/2026-09-29-field-cases.md`; audits of the plans before they ran, 09-25 to 09-28.
    The validator gap is find-70edcb.

[^7-rollup]: Rollup 2026-09-29T14:37:51, frozen as `evidence/2026-09-29-field-rollup.md` (sha256 `7bade1532a6d`)
    and `evidence/2026-09-29-field-rollup.json` (sha256 `0fe93ad9d9ac`), "Totals". Every captured run, portal and
    other, 2026-08-22 to 09-29; the method is in `evidence/2026-09-29-field-readme.md` (sha256 `a17c2c3109b3`).
    A task counts only with a checkpoint verdict, which runs record since the 09-28 fix: the 26 earlier runs add
    $33.56 and no tasks, and the 16 runs of 09-28 and 09-29 cost $159.36 for 158 tasks. The autonomy index is
    automatic recoveries over automatic recoveries plus interventions; a retried pass is automatic only when no
    intervention note on that task joins the run (bug-7b37c4). Backfilled snapshots keep only each plan's latest
    run. The escape rate is linked escapes over verified passes.

[^7-operator]: Assessment note W12, table F2, frozen as `evidence/2026-09-29-w12-operator-loop-cost.md` (sha256
    `82676de5eee4`): token counts of the Claude sessions of 09-25 to 09-29, priced at list rates from the price
    table `prices-2026-09-28`; the range spans the 5-minute and 1-hour cache-write rates. It is an API-equivalent
    price, not cash paid. The $172.80 is the sum of the 09-26, 09-28 and 09-29 rows of rollup 2026-09-29T14:37:51.
    The subagents and engine defects: B7, "TL;DR".
