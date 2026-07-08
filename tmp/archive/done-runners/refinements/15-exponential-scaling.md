# Exponential Scaling Patterns

> **TL;DR**: Roko should be designed so that performance, capability,
> and quality improve *superlinearly* with accumulated usage,
> deployment count, and connected data. This doc identifies seven
> specific mechanisms already or nearly in the codebase that exhibit
> compounding returns and shows how to tune each for maximum
> positive feedback. The goal: every unit of usage should make the
> next unit cheaper, faster, and better.

> **For first-time readers**: This doc is the "why does all the previous
> work compound?" story. Each of the seven loops builds on a specific
> earlier doc — demurrage (12), heuristics (14), HDC (11), c-factor
> (13), playbooks, commons (14 §10), plugins (17). Read this as a map of
> which features depend on which others to produce *superlinear* rather
> than *linear* returns. Read 31-synergy-integration-map.md alongside
> for the full cross-weave.

## 1. Why linear returns are the default failure

Most agent frameworks exhibit **diminishing** returns:

- Each new agent adds marginal value until the coordination cost
  exceeds the added throughput.
- Each new memory item dilutes retrieval quality.
- Each new tool adds context-window pressure.
- Each new user adds support burden without proportionate benefit
  to other users.

Roko's architecture has the ingredients to flip several of these
into *increasing* returns, but only if we deliberately connect the
feedback loops. This doc enumerates them.

## 2. The seven compounding loops

### 2.1 Demurrage-weighted retrieval (sub-linear → super-linear)

Naive memory grows O(n) and retrieval quality degrades. With
demurrage (`12`), memory grows but *attention-weighted* memory is
capped — so retrieval quality grows in *effectively-indexed* memory
which is bounded. Compounding mechanism: more usage → better
reinforcement signal → better calibration of demurrage rates →
sharper effective index.

Scaling law: retrieval quality ∝ log(trials) × calibration_quality(trials),
which superlinearly improves as trials increase.

### 2.2 Heuristic calibration (Bayesian compounding)

Every episode is a trial for dozens of heuristics. Confidence
intervals tighten as O(1/√n). The value of a well-calibrated
heuristic in a downstream decision goes as log-odds, which *depends
linearly on calibration quality*. Combined, the value extracted per
episode grows with √n *per heuristic* and with the count of
applicable heuristics *across* episodes — a multiplicative effect.

### 2.3 HDC codebook cleanup (quantization returns)

Every new episode, gate result, and heuristic adds to the HDC
codebook. HDC's *cleanup* operation snaps a noisy fingerprint to the
nearest codebook entry. Cleanup quality improves with codebook size
*up to* the capacity of the space (which is enormous at 10,240
bits). So for any foreseeable scale, every new episode makes every
subsequent retrieval more likely to hit a clean match. Payback
accelerates.

### 2.4 c-factor feedback

From `13`: teams with measured high c deliver higher-quality output.
High-quality output produces higher-quality reinforcement signals.
Higher-quality reinforcement improves heuristic calibration (2.2)
and demurrage rate tuning (2.1). Better priors and memory produce
higher c. **This is a three-loop reinforcement**: c → quality →
learning → c.

### 2.5 Playbook distillation (meta-learning)

Episodes are distilled into playbooks. Playbooks themselves can be
distilled into meta-playbooks ("when distilling a cluster of
episodes, the presence of gate failures suggests pre-condition
emphasis"). This is learning about learning. Each level of
distillation is cheaper per episode (because the level below is
already compressed) and more transferable (because it's more
abstract). Value per distillation-unit grows with depth.

### 2.6 Cross-deployment heuristic commons

If heuristics can be imported across deployments (`14` §10), then
every deployment contributes to a commons. The value of the commons
goes roughly as O(n) in deployment count but the cost to each
deployment is O(1). So the marginal value of the Nth deployment to
itself is O(1) but the marginal value to *everyone else* is also
O(1), giving a total system value of O(n²). This is a metcalfe's-law
effect at the heuristic level.

### 2.7 Plugin ecosystem (two-sided)

Once plugins exist (`17`), each new plugin increases the value of
Roko to users who need that capability, and each new user increases
the value of building a plugin. Classic two-sided network effect.
The magnitude depends on the quality of the plugin interface — bad
interfaces produce weak network effects because plugins leak
complexity to users.

## 3. Measuring compounding

We need instruments. Three scaling dashboards:

```
┌─ Learning Curves ─────────────────────────────────────┐
│ Retrieval quality vs episode count: ╱╱╱ (log-slope)   │
│ Heuristic calibration CI width:     ╲╲╲              │
│ c-factor trend:                     ╱╱                │
│ Codebook cleanup hit rate:          ╱╱╱               │
└───────────────────────────────────────────────────────┘
```

If any line flatlines, we have a blocker somewhere. Flatlining
retrieval quality means the feedback to demurrage isn't working. A
flatlining heuristic CI width means the Calibrator isn't getting
fresh trials.

## 4. Failure modes of compounding

### 4.1 Echo chambers

A tight positive loop can reinforce wrong beliefs. Countermeasures
are already in `13` (WisdomGate, outsider injection) and `14`
(challenger worldviews).

### 4.2 Reward hacking

If the Policy optimizes for c directly, it can achieve high c by
routing only to easy tasks. Countermeasure: **c is a covariate, not
the objective**. The objective is gate pass-rate on a task sampled
by difficulty. c is a measured property that correlates with — but
does not replace — outcome quality.

### 4.3 Premature convergence

A heuristic that hits 95% confidence early and then prevents its
own refutation by influencing which situations it's tested against.
Countermeasure: **importance sampling** — the Calibrator
occasionally runs heuristics on situations that *shouldn't* match,
to probe the boundary. Classic bandit exploration bonus applied to
a prior rather than an arm.

### 4.4 Substrate bloat

Infinite retention without demurrage blows out disk. Demurrage
(`12`) is the structural answer, but rates need tuning so cold tier
actually gets used. Observability: balance histogram.

## 5. Net-new superlinear primitives

These don't exist in other agent frameworks and are architecturally
possible only in Roko:

### 5.1 Prediction markets on heuristics

Agents can *stake* confidence on a heuristic's next outcome. Stakes
are balance-credits (see `12`). Correct predictions earn balance;
incorrect lose it. The aggregate stake becomes a secondary
confidence signal alongside the Bayesian calibration. This is a
Robin Hanson prediction-market design mapped onto the Bus: an
internal market for *truth* about the system's own priors.

### 5.2 Compositional tool-curricula

Every successful tool sequence is an Engram. HDC binding of tool
sequences gives us a compositional space of "plans of tools." New
plans are generated by HDC arithmetic: plan_for_new_task ≈
bundle(similar_plan_1, similar_plan_2) cleaned to the codebook. This
is Plotkin/MML/analogical reasoning, but fast and content-addressed.

### 5.3 Self-modeling

Roko observes its own latencies, its own failure rates, its own c.
These are Engrams too. A *meta-agent* can read those Engrams and
propose changes to policy parameters. The Bus carries a
`system.self` topic. This is John Holland's "internal models"
applied to the runtime itself.

## 6. Compounding as a product claim

"Every week your Roko gets faster, more accurate, and more
collaborative *on your codebase*."

This is a much stronger claim than "we have learning." It's
*provable* from the measurements above, and the failure modes are
known and instrumented. No other agent framework makes this claim
because none of them have the substrate to back it up.

## 7. The Phase-2 super-loops

Three additional compounding loops unlock when Phase 2 lands:

- **Witness-signed heuristic commons**: cross-deployment heuristics
  that carry chain witness signatures gain exponential trust with
  more signatures.
- **Dream-consolidation compression**: offline consolidation
  (`roko-dreams`) re-reads episodes during idle time, re-distilling
  them with current knowledge. Retroactive learning — old episodes
  produce *new* heuristics when viewed through current priors.
- **Agent-chain specialization**: roles specialize through chains
  of interactions; specialization stratifies the agent population
  into a functional ecosystem.

Phase 2 is where Roko gets *weirdly good* — not just incrementally
better but qualitatively different. These loops require the
two-fabric substrate from `02` and `03` to exist.

## 8. What to ship to enable compounding

Priority order for superlinear returns:

1. **Demurrage + reinforcement** (`12`). Enables 2.1 and supports 2.2.
2. **Heuristic type + Calibrator** (`14`). Enables 2.2 and 2.6.
3. **HDC-on-every-Engram** (`11`). Enables 2.3 and 5.2.
4. **c-factor metrics** (`13`). Enables 2.4 and 2.7 measurement.
5. **Plugin SPI** (`17`). Enables 2.7 network effect.
6. **Dream-cycle** (Phase 2). Enables retroactive compounding.

Steps 1–4 are a few weeks of focused work and unlock most of the
superlinear returns. Steps 5–6 require more build but produce the
"competitive moat" defensibility.

## 9. The single most important metric

If we had to pick one scaling KPI it would be:

**Mean time to first successful PR on a new codebase.**

This metric depends on all seven loops: good priors (2.2, 2.6), good
retrieval (2.1, 2.3), good collaboration (2.4), good compression
(2.5), good tooling (2.7), good self-awareness (5.3). It should drop
*steeply* over the first few weeks of a deployment and keep dropping
at a decreasing-but-positive rate indefinitely.

Plot this, optimize this, and the product tells its own story.

## 10. Secondary scaling KPIs

Beyond the single headline metric, track these to detect which loop
is doing the work (or failing):

| KPI | Loop it measures | Expected curve |
|---|---|---|
| Median tokens per task (by task difficulty bucket) | 2.1 + 2.3 + 2.5 | Monotonic decrease |
| % of Composer prompts hitting HDC-clean cache | 2.3 | Asymptote near 1 |
| Mean calibration CI width per heuristic | 2.2 | Decrease with n |
| % of heuristics sourced from commons | 2.6 | Increase then stabilize |
| c-factor on randomly-sampled cohorts | 2.4 | Stable or rising |
| Dream-cycle retroactive improvements / week | 7 (Phase 2) | Grows with corpus |
| Plugin count (unique) | 2.7 | Linear in t |
| Plugin unique users | 2.7 | Superlinear once flywheel turns |
| Mean time per task by task class | Composite | Monotonic decrease |
| First-task-after-install → success minutes | 2.6 commons bootstrap | Decreases over commons growth |

The first KPI is user-facing; the rest are operator KPIs. Expose
them as part of `33-observability-telemetry.md`.

## 11. Anti-metrics — what should *not* grow

Three numbers should stay flat or shrink even as usage grows.
These are the "we're not cheating" checks:

- **Episode count in warm tier** — if demurrage is working, warm-tier
  episode count reaches a steady state, it doesn't grow unbounded.
- **Heuristic count with confirmations < 3** — new hypotheses enter,
  but unconfirmed ones fade. If this grows unbounded, the Calibrator
  isn't probing them fast enough.
- **Mean lineage depth per response** — deep lineage is fine when
  it's load-bearing; growing lineage without growing answer quality
  means the Composer is hoarding context for no reason.

If any of these blow out, pause feature work and tune rates (per
12 §14).

## 12. Load-bearing assumption: real workloads, not synthetic benchmarks

The "superlinear returns" claim depends on real workloads with
persistence across sessions. A benchmarking suite that resets state
between runs will see none of the compounding. Two implications:

1. **Evals must span sessions.** Each benchmark task should be
   attempted N times over M days with the agent's substrate
   preserved. Measure the slope of `time_to_solve` over N trials.
2. **Heuristic commons contribution is a variable.** Evaluate the
   same agent with and without commons access. The gap is the
   commons' contribution to the compounding.

Without these guardrails, the product claim in §6 is an assertion,
not an observation. With them, the claim becomes falsifiable — which
is the replication-ledger ethos from `16-research-to-runtime.md`
applied to our own product promises.

## 13. The kill switches

If compounding goes wrong, the operator needs an emergency stop:

- `roko attention reset` — zero all balances back to starting value.
- `roko heuristic retire --confidence-below 0.3` — sweep out weakly
  calibrated heuristics.
- `roko substrate freeze --older-than 90d` — manually graduate old
  Engrams to cold tier.
- `roko commons opt-out` — stop importing from or exporting to the
  shared commons.
- `roko experiments pause` — freeze prompt-experiment variant rotation.

Each is reversible. Each is surfaced in the TUI Settings tab. Having
them documented and tested *before* they're needed is cheap insurance.
