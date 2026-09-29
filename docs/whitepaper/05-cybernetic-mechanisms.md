Status: reviewed · budget 850 words · owner gap-e8cb4d

# 5 Cybernetic mechanisms

Roko is designed to improve from verified outcomes and to show that it does. Three layers carry the design: essential
variables that define "better", first-order loops that tune prompts, retries and routing, and second-order mechanisms
that audit the gates and the loops and guard every change. Each tag names its appendix row.

## 5.1 Essential variables

The essential variables of §2 are the verified pass rate, the cost per verified task, the false-green rate (passes that
a stronger check would fail) and latency, each measured per task over all its attempts. People set their bounds, and §8
reports them. Nothing regulates them as a set yet; that is M1's job.

## 5.2 First-order learning loops

Each loop senses outcomes and changes a later decision.

| Loop | Senses | Changes | Status |
|---|---|---|---|
| Model routing | Each model choice's outcome | Which model runs | PARTIAL@ed0c33bd5 (RC2): it learns from the provider's success flag, before gates run; bug-c34782 |
| Retry feedback | The failed attempt's gate output | The retry prompt, across a resume | WIRED@ed0c33bd5 (EX6) |
| Adaptive thresholds | Each gate rung's pass rate | Retry budgets of tasks that set none | WIRED@ed0c33bd5 (QA7) |
| Failure memory | Error patterns, post-gate lessons | Retry prompts | ORPHANED@ed0c33bd5 (LM4): neither reaches a prompt; spec-6ac537 |
| Playbooks | Outcomes of the playbooks a prompt used | Which playbooks enter prompts | WIRED@ed0c33bd5 (LM2): a floor still puts three in every prompt |
| Knowledge store | Gate-verified attempts | Notes in prompts | BROKEN@ed0c33bd5 (LM3): written back, but plan prompts never retrieve it; bug-86117a |
| Prompt experiments | Each prompt variant's outcome | The variant a task gets | PARTIAL@ed0c33bd5 (LM5): its winner test is invalid under adaptive assignment; spec-6ac537 |
| Hindsight relabelling | A verify failure that blames an earlier task | That task's recorded success | PARTIAL@ed0c33bd5 (LM12): nothing reads the corrections; gap-5be28d |

Runner-v2 had 16 learning loops in its event loop, and most lost their caller or their data when `6b5da8616` deleted
it. Merges on 2026-09-29 (`ce3bdcbb8`, `33e107da1`) re-attached several (reg-3f5969, reg-06ae9f, gap-fdd27f,
gap-5fb9a7): of the 16, two are now wired, five partial, seven orphaned, one broken and one built but
unwired.[^5-lost] Two limits remain. Most loops still learn from a
pass/fail flag that counts unverified outcomes as passes (QA2, PARTIAL@ed0c33bd5; spec-e9d7ec). And no loop has a
measured benefit, under Runner-v2 or since: that learning improves outcomes is UNPROVEN@ed0c33bd5 until M2, a wiring
census (gap-1f2661) and frozen-learning runs (gap-644040) exist.

## 5.3 Second-order mechanisms and guarded commit

The loops change Roko's behaviour, but nothing checks that the changes help, or that the gates they learn from are
right. Four mechanisms and one commit rule are designed for that. All five are MISSING@ed0c33bd5; epic spec-6ac537 (E17)
carries their specifications.

| Mechanism | Regulates | Sensor → actuator | Bounds[^5-design] | Status |
|---|---|---|---|---|
| M1 bounded controller | The essential variables | Change detectors → one step on one harness setting, kept or rolled back | Acts only when a variable leaves its bounds; never widens permissions, removes authored checks or raises budget ceilings; 10% static holdout | MISSING@ed0c33bd5 (RG3) |
| M2 loop-liveness audit | Each loop's exposure, influence and benefit | Randomized holdouts with an A/A floor → keep or demote the loop | Holdout at least 2%; one state change a day at most; safety checks exempt | MISSING@ed0c33bd5 (RG4) |
| M3 calibrated self-model | The chance a task passes on a given model | Verified outcomes → model choice, verification depth | Shadow until calibrated; only adds verification; authored pins win | MISSING@ed0c33bd5 (RG5) |
| M4 random deep audits | The false-green rate | Audit lottery over passes, hidden tests → stricter checks, routing trust | Audit probability at least 5% per pass; at most 12% of spend | MISSING@ed0c33bd5 (QA5) |
| Guarded commit | Every self-modification: router, memory, controller | Held-out and anchor checks → commit, or roll back | The last known good version is kept | MISSING@ed0c33bd5 (RG6) |

**M1** starts in shadow mode, logging the moves it would make, and is judged by disturbance tests (a provider fault, a
model swap, a halved budget) against its holdout. It replaces the conductor without reviving the conductor's supervisor
tick, which caused restart storms.

**M2** exists because a loop can run without reaching any decision (§5.2). A loop that shows no benefit against a
withheld control reverts to its default policy.

**M3** forecasts from recorded outcomes, not from a model's stated confidence.

**M4** is the only sensor on the gates themselves. Another model family writes hidden tests from the spec, out of the
agent's reach, and a Hájek estimator, weighting each audited pass by the inverse of its audit probability, gives the
false-green rate with an interval. Today's honest verdicts (§6) rest on visible checks alone.

**Guarded commit** wraps every learner and every M1 change: a change stays only if it helps on held-out work and loses
no more than a set margin on fixed anchor tasks.

## 5.4 The claim

The claim is measured, guarded improvement, not compounding; the literature supports no more. In a two-phase test on
hard Terminal-Bench 2.0 tasks, two of three harness optimizers fell below baseline or stalled once new tasks arrived,
and the one that kept improving, the authors' own, built regression control into its loop [@wang2026compound]. On
Terminal-Bench 2.1, harness evolution did not consistently beat simple test-time scaling at matched budgets
[@wang2026rethinking]. In a re-evaluation on multi-step agent tasks, memory-based self-improvement amplified evaluation
noise and depended on task order [@ye2026fragility], and on a trading benchmark, gains on similar unseen tasks often
weakened under distribution shift [@lin2026evopath]. None of these measured repository-scale plans.

Subsystems built on analogies are proposals to park (§9), not features: affect (PARTIAL@ed0c33bd5, LM8) and offline
batch consolidation (ORPHANED@ed0c33bd5, LM7), both pending q-6b7cca; HDC similarity (BUILT-UNWIRED@ed0c33bd5, LM10);
and the conductor (ORPHANED@ed0c33bd5, RG2; gap-ebd656).

[^5-lost]: Research note B5, frozen as `evidence/learning-loops-B5.md` (sha256 `044ad5c96542`), "Re-check at
    `98ee1418f`": the 16 loops Runner-v2 had, tagged at `98ee1418f`; at `d9e79e9d8` none was wired, eight were
    partial, seven orphaned and one built but unwired. find-34a4b5 lists the closures in the deleted event loop. The
    table above groups the loops by mechanism, with the appendix's tags at `ed0c33bd5`.
[^5-design]: Design values from the specifications that epic spec-6ac537 carries; none is built at `ed0c33bd5`.
