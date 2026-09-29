Status: reviewed · budget 450 words · owner gap-424bf8

# 6 Measured trust

Cheap executors are worth using only if a team can tell which results to trust. Roko is designed to report three
measures of that trust from the team's own runs; none is produced today. §5 describes the mechanisms, and §8 how
they will be evaluated.

## 6.1 Three measures

| Measure | Counts | Denominator | Reported as | Mechanism and status |
|---|---|---|---|---|
| Routing learned from verified outcomes | Verified passes on any vendor's model, credited to the model that ran | Settled attempts per model and kind of task; an unverified attempt counts as failed | Pass rate and cost per verified task, with forecast calibration | M3, a calibrated self-model: MISSING@a17d4dadd (row RG5; spec-6ac537). Its base, the router, is PARTIAL@a17d4dadd (row RC2): its learned stage sees successes only, and unverified work earns full reward (bug-8da8ba, bug-f68404, bug-c34782) |
| False-green rate | Accepted tasks that fail an independent audit: a tamper check, a clean re-run, and hidden tests written from the spec by another model family | Tasks accepted in a window. A keyed lottery audits each with a known chance, and weights it by the inverse of that chance | A Hájek estimate with a confidence interval, per model and kind of task | M4, random deep audits: MISSING@a17d4dadd (row QA5; spec-6ac537) |
| Per-loop evidence that learning helps | Per loop: exposure (learned state reached the decision), influence (the decision left the default, net of A/A noise) and benefit (the change in verified success) | Decisions the loop could change; a random holdout keeps some on the default as the control | The three figures, with anytime-valid intervals; a harmful loop is demoted to its default | M2, the loop-liveness audit: MISSING@a17d4dadd (row RG4; spec-6ac537). It needs frozen learning (gap-644040) |

## 6.2 What exists today

Today Roko has only the base these measures need: verdicts honest about its own gates. Since the 09-28 fix, 0 of
151 recorded passes had a failing gate, against 101 of 373 (27%) before.[^6-verdicts] But those gates are each
task's visible verify commands, which the agent can read and could game. So the figure shows that Roko no longer
records a failed check as a pass; it is not a false-green rate, since nothing yet re-checks a pass against tests
the agent never saw.

## 6.3 The field

As documented on 2026-09-29, we found no product that documents any of the three measures. Several pair a stronger
model with a cheaper one: Claude Code's `opusplan` plans on Opus and executes on Sonnet, and Devin's Fusion pairs
"a frontier lead model with a cost-efficient sidekick" [@anthropic2026advisor; @cognition2026models]. Model choice
is tied to one vendor or made by the vendor's router: Claude Code's workers are always Claude sessions, Cursor's
router is "managed by Cursor", and Factory's sends routine steps to cheaper models and escalates when one struggles
[@anthropic2026agents; @cursor2026router; @factory2026router]. None of these pages says that routing learns from
the user's own verdicts, reports how often a pass is wrong, or gives evidence per learning mechanism.

[^6-verdicts]: Research note B7, frozen as `evidence/2026-09-29-b7-real-run-evidence.md` (sha256 `799b6a2b6184`),
    "TL;DR"; fix commit `725f21e05` (bug-82d47b, bug-521f08, bug-06e2d1). Before: 430 attempts in 31 plans,
    2026-09-05 to the fix; after: 168 attempts to 2026-09-29 07:41Z. §7 gives the full case (CASE-001).
