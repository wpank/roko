# Depth: Exponential Flywheel

> Parent: [16-COORDINATION](../../16-COORDINATION.md) -- Section 11

---

## Overview

Roko should improve **superlinearly** with accumulated usage, deployment
count, and connected data. The result is an exponential flywheel only when the
feedback loops are wired deliberately; otherwise the system falls back to
linear growth, local optimization, or outright stagnation.

Seven loops are the compounding mechanisms most relevant to coordination. They
are not independent; they reinforce one another. A better commons improves
c-factor. Better c-factor improves heuristics. Better heuristics improve
retrieval, distillation, and plugin adoption. That is the actual flywheel.

---

## The Seven Loops

| # | Loop | What Compounds | Coordination Effect |
|---|------|----------------|---------------------|
| 1 | Demurrage-weighted retrieval | Usage calibrates attention cost and reward | Keeps what is unique and useful; prevents memory bloat from dominating retrieval |
| 2 | Heuristic calibration | More trials tighten uncertainty | Each episode becomes a better test of the worldview and a better input to policy |
| 3 | HDC codebook cleanup | More exemplars improve similarity | Retrieval, clustering, and analogy get cleaner as the codebook grows |
| 4 | c-factor feedback | Better cohort process improves output quality | Cohort dynamics become a measured input to policy |
| 5 | Playbook distillation | Episodes compress into reusable playbooks | The system learns how to learn, lowering the cost of each improvement |
| 6 | Cross-deployment heuristic commons | Imported heuristics create shared calibration | Every deployment contributes to a rule base benefiting others at near-zero marginal cost |
| 7 | Plugin ecosystem | Each plugin increases value for users and builders | Interface becomes a platform when it stays narrow, stable, and composable |

---

## Loop 1: Demurrage-Weighted Retrieval

Naive memory grows without bound and retrieval quality degrades. Demurrage
changes the slope: usage earns reinforcement, while idle content pays a
holding cost. The working set stays small enough to remain relevant yet large
enough to preserve rare, load-bearing pieces.

The compounding effect is operational: better usage traces improve calibration
of holding costs, which improves retrieval quality, which improves the next
round of usage.

```
usage(t) --> calibrate holding costs --> retrieval quality(t+1)
     ^                                           |
     |___________________________________________|
```

**Failure signal**: Warm-tier content grows without reaching a steady state,
or retrieval quality stops improving even as trials accumulate.

---

## Loop 2: Heuristic Calibration

Every episode is a trial for multiple heuristics. As trial counts rise,
confidence intervals tighten and the system learns which heuristics are
reliable, narrow, or wrong. A well-calibrated heuristic changes downstream
decisions in proportion to its confidence.

This is one of the most important compounding loops because it turns
experience into better priors, not just more logs.

**Mechanism**: The Bayesian confidence infrastructure in `roko-learn`
maintains per-heuristic trial counts and beta-distribution posteriors.
Each gate outcome and each episode completion produces an observation
that updates the relevant heuristic confidence.

```rust
// From roko-learn: BayesianConfidence tracks per-heuristic posteriors.
// More trials --> tighter CI --> better routing decisions.
pub struct HeuristicTrial {
    pub heuristic_id: String,
    pub outcome: f64,           // [0, 1]
    pub context_hash: [u8; 32], // HDC fingerprint of the episode
}
```

**Failure signal**: Premature convergence. A heuristic reaching high
confidence early and then avoiding refutation will look stable while
becoming less useful.

---

## Loop 3: HDC Codebook Cleanup

HDC similarity makes retrieval and cleanup cheaper as the codebook gets
richer. More episodes, gate results, and heuristics create more anchors
for similarity search, raising the chance that a noisy query lands on the
right cluster.

The practical result is cleaner reuse: prompts, retrievals, and decisions
become less token-heavy because the right exemplar is easier to find.

```
episodes --> codebook anchors --> similarity precision --> cleaner reuse
                                       |                        |
                                       |________________________|
```

**Failure signal**: Codebook pollution. If the space fills with noisy,
redundant, or stale entries, similarity becomes less discriminative and the
flywheel slows.

---

## Loop 4: c-factor Feedback

The c-factor is the cohort-process signal that tells whether the collective
is functioning well (see `12-collective-intelligence-metrics.md`). High
c-factor should correlate with better turn-taking, better peer prediction,
better citation reciprocity, better delivery rate, and better HDC diversity.

The important distinction is causal discipline: **c-factor is a covariate,
not the objective**. The objective is task quality on work sampled by
difficulty. c-factor is a measured property that should move with better
coordination, not replace it.

**How it compounds**: When the `CohortWeightsLearner` fits weights to
observed cohort outcomes, the system learns which process axes predict success
for each task family. That knowledge improves future cohort formation,
routing, and intervention timing.

**Failure signal**: The system learns to optimize for easy tasks, flattering
cohorts, or metric gaming instead of hard-work quality.

---

## Loop 5: Playbook Distillation

Episodes should not stay raw forever. They compress into playbooks, then
meta-playbooks, so later work starts from a better compressed prior. This is
where the system learns to reuse its own past.

The compounding effect comes from transferability. A well-distilled playbook
is cheaper to apply than re-deriving the same lesson from scratch, and a
meta-playbook is cheaper still.

```
raw episodes --> playbooks --> meta-playbooks --> system prompt injection
                    |                                    |
                    |____________________________________|
                      cheaper downstream decisions
```

**Source**: `roko-learn` playbook store. Top when/then matches are queried at
live dispatch and injected into the system prompt by the runner.

**Failure signal**: Overcompression. If a playbook loses the context that made
it valid, reuse will look efficient while silently degrading decision quality.

---

## Loop 6: Cross-Deployment Heuristic Commons

When heuristics can be shared across deployments, each deployment contributes
to a commons that other deployments can reuse. The local cost of importing a
useful heuristic is small; the system value is large because one good
heuristic can help many deployments.

This is the coordination version of a network effect. It only works if the
commons stays curated, versioned, and revalidated in context.

**Mechanism**: Heuristics exported at `Mesh` or `Global` pheromone scope
propagate through the relay/gossip transport. Receiving deployments apply a
confidence discount (trust multiplier 0.60 for cross-collective, 0.50 for
anonymous/public) and require local revalidation before adoption.

**Failure signal**: Stale imports. A shared heuristic that no longer matches
the local deployment becomes a drag instead of an asset.

---

## Loop 7: Plugin Ecosystem

Plugins create the classic two-sided flywheel. Each plugin increases user
value for a capability that was previously missing, and each new user
increases the incentive to build more plugins.

The flywheel depends on the interface. If the plugin surface is stable, typed,
and narrow, the ecosystem compounds. If the interface leaks complexity, the
network effect weakens and support costs rise faster than adoption.

**Current state**: `roko-plugin` (E32 8/8) provides signed dependency graphs,
bounded typed WASM hooks, strict admission, verified relay/install, and
current CLI/MCP targets. The WIT/Component model hostcalls and OpenClaw/legacy
adapter parity remain open.

**Failure signal**: Integration friction. When every plugin needs bespoke
scaffolding, the platform effect collapses into one-off integrations.

---

## Phase 2 Amplifiers

The seven loops already compound. Phase 2 adds amplifiers:

- **Dream-consolidation compression**: Offline consolidation re-reads prior
  episodes during idle time and re-distills them with current priors. Old
  episodes can produce new heuristics when viewed through a better model.
- **Agent-chain specialization**: Roles specialize through chains of
  interaction, turning a generalist fleet into a structured functional
  ecosystem.
- **Witness-signed heuristic commons**: When shared heuristics carry stronger
  provenance, trust compounds across deployments instead of resetting at each
  boundary.

---

## Measurement

The flywheel is only real if it is measured on persistent workloads. Stateless
benchmarks hide compounding because they reset the substrate between trials.

### North-Star Metric

**Mean time to first successful PR on a new codebase.**

That metric depends on all seven loops: better priors, better retrieval,
better coordination, better compression, better tooling, and better
self-awareness. It should drop steeply early on and keep dropping at a
decreasing-but-positive rate.

### Secondary KPIs

| KPI | Loop | Expected Curve |
|-----|------|----------------|
| Median tokens per task by difficulty bucket | Demurrage + HDC + playbook | Monotonic decrease |
| % of Composer prompts hitting HDC-clean cache | HDC codebook cleanup | Asymptote near 1 |
| Mean calibration CI width per heuristic | Heuristic calibration | Decrease with trial count |
| % of heuristics sourced from commons | Cross-deployment commons | Increase, then stabilize |
| c-factor on randomly sampled cohorts | c-factor feedback | Stable or rising |
| Dream-cycle retroactive improvements/week | Dream compression | Grows with corpus |
| Plugin count and unique users | Plugin ecosystem | Count linear, users superlinear |
| First-task-after-install success minutes | Heuristic commons bootstrap | Decrease as commons grows |

### Anti-Metrics

These should **not** grow without corresponding quality gains:

- Warm-tier episode count should reach a steady state (not grow unbounded)
- Heuristics with fewer than three confirmations should shrink over time
- Mean lineage depth per response should not increase unless quality improves

If any of those rise without corresponding quality gains, the flywheel is
being simulated instead of earned.

---

## Failure Modes

| Failure Mode | Mechanism | Counter |
|-------------|-----------|---------|
| Echo chambers | Positive feedback reinforces wrong beliefs | Inject outsiders, explicit falsifiers, sampled disagreement |
| Reward hacking | Optimizing c-factor directly favors easy work | Keep objective on task quality sampled by difficulty |
| Premature convergence | High-confidence heuristics stop facing challenge | Importance sampling, deliberate boundary tests |
| Substrate bloat | Retention without demurrage overwhelms working set | Tune holding costs so cold tier gets used |
| Commons drift | Imported heuristics lose fit across deployments | Require local revalidation before adoption |
| Benchmark illusion | Synthetic tests reset state, missing compounding | Measure across sessions with preserved substrate |
| Plugin drag | Large plugin surface with unstable contracts | Keep the SPI narrow and predictable |

---

## Operating Rule

The flywheel is not a promise that growth will be exponential by default. It
is a design goal that becomes true only when each loop is instrumented, curbed
against its failure modes, and run on real persistent workloads. If a line
flatlines, the feedback loop is broken somewhere.

---

## Mathematical Foundation

The superlinear growth claim rests on Reed's Law [Reed, D.P. "The Law of the
Pack." *Harvard Business Review*, 2001]: network value grows as O(2^N) for
group-forming networks, while coordination cost grows linearly (O(N x M))
thanks to stigmergy (see `10-stigmergy-scaling.md`).

The value-to-cost ratio therefore grows exponentially:

```
V/C ratio ~ 2^N / (N x M)
```

For practical Collective sizes (N = 5 to 50), the ratio increases from
~1.6 to ~2.2 x 10^13 -- orders of magnitude of coordination value per unit
cost. This is the mathematical basis for claiming that the flywheel can
produce superlinear returns.

---

## Implementation Status

All seven loops have partial or complete implementation:

| Loop | Implementation | Source |
|------|---------------|--------|
| Demurrage-weighted retrieval | Wired (E24) | `roko-neuro` balance/demurrage |
| Heuristic calibration | Wired (E25) | `roko-learn` BayesianConfidence |
| HDC codebook cleanup | Wired | `roko-primitives` HDC vectors |
| c-factor feedback | Wired | `roko-learn` CFactorSummary |
| Playbook distillation | Wired (E25) | `roko-learn` playbook store |
| Cross-deployment commons | Partial (E29) | Relay transport, mesh sync |
| Plugin ecosystem | Wired (E32 8/8) | `roko-plugin` |

The primary gap is end-to-end measurement: the KPIs above are defined but
not yet wired into a single dashboard view that tracks compounding over time.

---

## Cross-References

- `10-stigmergy-scaling.md` -- Why coordination cost grows linearly
- `12-collective-intelligence-metrics.md` -- c-factor instrumentation
- [08-LEARNING](../../08-LEARNING.md) -- Playbook distillation, heuristic
  calibration
- [09-MEMORY](../../09-MEMORY.md) -- Demurrage-weighted retrieval, tier
  progression
- [10-DREAMS](../../10-DREAMS.md) -- Dream consolidation compression
- [19-TOOLS-PLUGINS](../../19-TOOLS-PLUGINS.md) -- Plugin ecosystem

---

## References

- [Reed 2001] The Law of the Pack, *Harvard Business Review*
- [Woolley et al. 2010] Evidence for a Collective Intelligence Factor,
  *Science*, 330(6004):686-688
- [Kauffman 1993] *The Origins of Order*, Oxford University Press
