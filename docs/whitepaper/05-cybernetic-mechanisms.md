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
| Model routing | Each model choice's outcome | Which model runs | PARTIAL@a43288b5f (RC2): it learns from settled verdicts, but the ladder places most tasks and its pick is only logged; spec-6ac537 |
| Retry feedback | The failed attempt's gate output | The retry prompt, across a resume | WIRED@a43288b5f (EX6) |
| Adaptive thresholds | Each gate rung's pass rate | Retry budgets of tasks that set none | PARTIAL@a43288b5f (QA7): the ladder raises a task's budget to five, so in the live run it changed nothing; spec-6ac537 |
| Failure memory | Error patterns, post-gate lessons | Retry prompts | PARTIAL@a43288b5f (LM4): error patterns reach later prompts, the lessons none, and live the only patterns were turn caps; gap-e483e7 |
| Playbooks | Outcomes of the playbooks a prompt used | Which playbooks enter prompts | WIRED@a43288b5f (LM2): a floor still puts three in every prompt |
| Knowledge store | Gate-verified attempts | Notes in prompts | PARTIAL@a43288b5f (LM3): since `133c02093` plan prompts get up to three entries, but live they were generic success notes;[^5-live] spec-6ac537 |
| Prompt experiments | Each prompt variant's outcome | The variant a task gets | PARTIAL@a43288b5f (LM5): its winner test is invalid under adaptive assignment; spec-6ac537 |
| Hindsight relabelling | A verify failure that blames an earlier task | That task's recorded success | WIRED@a43288b5f (LM12): since `26c592955` the prompt caches apply the corrections; no run has triggered one yet |

Runner-v2 had 16 learning loops in its event loop, and most lost their caller or their data when `6b5da8616` deleted
it. Merges on 2026-09-29 (`ce3bdcbb8`, `33e107da1`) re-attached several (reg-3f5969, reg-06ae9f, gap-fdd27f,
gap-5fb9a7): at `942d2a6c3`, of the 16, two are wired, five partial, seven orphaned, one broken and one built but
unwired.[^5-lost] Since `04c1da262` (gap-8f6206) every learner reads the settled verdict's learning label, so an
unverified outcome no longer counts as a pass, and since `4c0e5646e` neither does any surface (WIRED@a43288b5f, QA2).
The larger limit remains: no loop has a measured benefit, under Runner-v2 or since. That learning improves outcomes is
UNPROVEN@a43288b5f until M2 and frozen-learning runs (gap-644040) exist.

## 5.3 Second-order mechanisms and guarded commit

The loops change Roko's behaviour, but nothing checks that the changes help, or that the gates they learn from are
right. Four mechanisms and one commit rule are designed for that. All five are MISSING@a43288b5f; epic spec-6ac537 (E17)
carries their specifications.

| Mechanism | Regulates | Sensor → actuator | Bounds[^5-design] | Status |
|---|---|---|---|---|
| M1 bounded controller | The essential variables | Change detectors → one step on one harness setting, kept or rolled back | Acts only when a variable leaves its bounds; never widens permissions, removes authored checks or raises budget ceilings; 10% static holdout | MISSING@a43288b5f (RG3) |
| M2 loop-liveness audit | Each loop's exposure, influence and benefit | Randomized holdouts with an A/A floor → keep or demote the loop | Holdout at least 2%; one state change a day at most; safety checks exempt | MISSING@a43288b5f (RG4) |
| M3 calibrated self-model | The chance a task passes on a given model | Verified outcomes → model choice, verification depth | Shadow until calibrated; only adds verification; authored pins win | MISSING@a43288b5f (RG5) |
| M4 random deep audits | The false-green rate | Audit lottery over passes, hidden tests → stricter checks, routing trust | Audit probability at least 5% per pass; at most 12% of spend | MISSING@a43288b5f (QA5) |
| Guarded commit | Every self-modification: router, memory, controller | Held-out and anchor checks → commit, or roll back | The last known good version is kept | MISSING@a43288b5f (RG6) |

**M1** starts in shadow mode, logging the moves it would make, and is judged by disturbance tests (a provider fault, a
model swap, a halved budget) against its holdout. Since `dd192cb82` the conductor supervises running attempts again, each on
its own (PARTIAL@a43288b5f, RG2); M1 is meant to regulate the harness above it.

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

Subsystems built on analogies are proposals to park (§9), not features: affect (PARTIAL@a43288b5f, LM8) and offline
batch consolidation (ORPHANED@a43288b5f, LM7), both pending q-6b7cca; HDC similarity (BUILT-UNWIRED@a43288b5f, LM10);
and the conductor (PARTIAL@a43288b5f, RG2; gap-ebd656), which since `dd192cb82` supervises attempts.

[^5-lost]: The re-check of the 16 loops Runner-v2 had, at `942d2a6c3`, frozen as
    `evidence/2026-09-29-learning-loops-b5-942d2a6c3.md` (sha256 `22f79f28a844`): the tags of the earlier re-check in
    research note B5, with the evidence behind seven rows updated. At `d9e79e9d8` none was wired, eight were partial,
    seven orphaned and one built but unwired (`evidence/learning-loops-B5.md`, sha256 `044ad5c96542`). find-34a4b5
    lists the closures in the deleted event loop. The table above groups the loops by mechanism, with the appendix's
    tags at `a43288b5f`.
[^5-design]: Design values from the specifications that epic spec-6ac537 carries; none is built at `a43288b5f`.
[^5-live]: The live run of 2026-10-02, frozen as `evidence/2026-10-02-live-cheap-model-run.md` (sha256
    `813172c96b88`), "What worked" (knowledge write-back and injection) and "What broke" 11: two five-task plans, the
    second fed by the first's verified passes.