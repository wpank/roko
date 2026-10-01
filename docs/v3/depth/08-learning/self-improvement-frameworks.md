# 08-learning/14 -- Self-Improvement Frameworks

> Survey of the academic and industrial frameworks that inform Roko's
> learning architecture, with explicit mappings to concrete subsystems
> and the external verifier requirement that gates all self-improvement.

**Parent:** [08-LEARNING](../../08-LEARNING.md) section 12

**Source:** `crates/roko-learn/src/playbook_rules.rs` (Reflexion/ExpeL),
`crates/roko-learn/src/prompt_experiment.rs` (DSPy),
`crates/roko-learn/src/cascade_router.rs` (RouteLLM/FrugalGPT),
`crates/roko-learn/src/hindsight.rs` (SiriuS)

---

## 1. Purpose

Each framework in the agent self-improvement literature contributes a
specific insight to Roko's learning architecture. This document maps
frameworks to concrete subsystems, identifies the external verifier
requirement that gates all self-improvement, and specifies the safety
invariants that prevent harmful self-modification.

The key contribution is the distinction between **what the literature
proposes** and **what Roko actually ships**. Every mapping includes explicit
differences -- where Roko deviates from the academic design and why.

---

## 2. Agent Self-Improvement Frameworks

### 2.1 Reflexion (Shinn et al. 2023)

**Paper insight:** Agents improve by reflecting on failures in natural
language, then using those reflections as additional context in subsequent
attempts.

**Roko implementation:** The playbook rule system extracts if-then rules
from failure patterns and injects them into subsequent agent prompts. This
is a structured form of Reflexion: instead of free-form natural language
reflection, Roko extracts typed rules with confidence tracking and trigger
matching.

**Key difference:** Reflexion operates within a single task's retry loop.
Roko's playbook rules persist across tasks and plans -- a failure in plan A
prevents the same mistake in plan B. The bounded confidence dynamics
(validate +0.05, contradict -0.10, ceiling 0.95) provide automatic pruning
of stale reflections that Reflexion's free-form approach lacks.

### 2.2 ExpeL (Zhao et al. 2024)

**Paper insight:** Agents should extract generalizable "experiences" from
successful and failed trials, accumulating them into a growing library.

**Roko implementation:** Successful episodes produce playbook entries
(positive experiences); failure patterns produce playbook rules (negative
experiences). Both persist across sessions and grow monotonically (subject
to confidence-driven pruning).

**Key difference:** ExpeL uses natural language experiences without
confidence tracking. Roko's playbook rules have bounded confidence dynamics
that automatically prune stale experiences. The asymmetric update rate
(contradictions penalize 2x more than validations reward) ensures rules
that stop being accurate are demoted before they can cause widespread harm.

### 2.3 Voyager (Wang et al. 2023)

**Paper insight:** Embodied agents accumulate reusable skills from
exploration, building a growing library of capabilities.

**Roko implementation:** The skill library accumulates proven approaches
from successful episodes. Skills are tagged with trigger patterns (file
paths, task categories, crate names) for retrieval at dispatch time.

**Key difference:** Voyager operates in Minecraft with immediate environment
feedback. Roko operates in a software engineering domain where feedback is
delayed (gate pipeline evaluation after full agent turn). Skills are
injected into the system prompt at dispatch time rather than executed as
code functions.

### 2.4 DSPy (Khattab et al. 2024)

**Paper insight:** Prompt optimization should be treated as a compiler
problem: define a program signature, generate prompt variations, evaluate
against a metric, and select the best-performing variant.

**Roko implementation:** The prompt experiment system (`ExperimentStore`)
implements DSPy-style prompt optimization. Each experiment defines a prompt
section, registers variants, assigns variants using bandit selection (with
reservation tracking for concurrent assignments), and evaluates against
gate pass rate.

**Key difference:** DSPy optimizes statically (generate many variants,
evaluate on a test set, select the winner). Roko optimizes online -- bandit-
driven variant selection during live execution, with continuous evaluation
and durable settlement. Chi-square and Wilson score intervals provide
statistically rigorous conclusion criteria. The Variance Inequality check
enables early stopping when no variant can statistically surpass the leader.

### 2.5 GRASP (arXiv:2605.29668, May 2026)

**Paper insight:** Unrestricted grounding of agent actions on retrieved
knowledge causes silent degradation -- rules that help new cases can break
established trajectories. On MedAgentBench, GRASP improved task success
from 40.6% to 88.8% (+48 points) by gating admission through a regression
test.

**Roko implementation (target):** Each candidate playbook entry is tested
against a balanced held-out probe set under a hard regression budget before
admission. Only admitted if net improvement is positive. See section 2.5 of
the parent chapter for the full admission protocol.

**Key difference:** GRASP operates on a fixed task corpus. Roko's probe set
is drawn from recent successful episodes, requiring dynamic sampling rather
than a static evaluation set.

### 2.6 SiriuS (arXiv:2502.04780, Feb 2025)

**Paper insight:** Rather than discarding failed episodes outright, augment
them with corrective annotations and reuse them as negative examples.

**Roko implementation:** The hindsight relabeling system (`hindsight.rs`)
decomposes failed trajectories into sub-goals and marks achieved sub-goals
as positive episodes. This recovers useful learning signal from at least
45% of otherwise-discarded episodes.

**Key difference:** SiriuS augments full failed episodes with corrections.
Roko's hindsight system extracts the portion that succeeded (sub-goal
achieved) and relabels it as a positive episode for a different (smaller)
goal. The original episode is never modified -- adjustments are append-only.

### 2.7 SkillZip (arXiv:2608.11079, Aug 2026)

**Paper insight:** Minimum Description Length (MDL) compression prevents
skill libraries from growing without bound while preserving predictive
accuracy.

**Roko implementation (target):** When the playbook store exceeds configured
capacity (default: 500 rules), rules with overlapping triggers and high HDC
similarity are merged into a single generalized rule. The MDL criterion
ensures that generalization only happens when the merged rule is shorter
(in description length) than the two originals while preserving accuracy.

### 2.8 ReSkill (arXiv:2606.01619, Jun 2026)

**Paper insight:** Skills can be refined through iterative self-correction
rather than only accumulated.

**Roko implementation (target):** Post-admission refinement cycles could
augment the GRASP admission gate. A rule admitted at confidence 0.50 could
be iteratively refined based on its performance on subsequent matching tasks.

---

## 3. Model Routing Research

### 3.1 RouteLLM (Ong et al., ICLR 2025)

**Result:** 85% cost reduction while maintaining quality by routing queries
to strong or weak models based on predicted difficulty.

**Roko adaptation:** The cascade router's confidence stage implements a
simpler version: empirical pass rates per model with confidence intervals,
rather than a neural classifier. The LinUCB stage provides context-dependent
routing using linear contextual bandits instead of neural networks.

### 3.2 FrugalGPT (Chen et al., arXiv:2305.05176)

**Result:** 98% cost reduction by cascading through models from cheapest to
most expensive, stopping when confidence is high enough.

**Roko adaptation:** The `CascadeModel` includes both a primary and a
fallback model. If the primary fails (gate failure, timeout), the
orchestrator retries with the fallback. The three-stage cascade (Static ->
Confidence -> UCB) is a different dimension of cascading: strategy
complexity rather than model cost.

### 3.3 AutoMix (NeurIPS 2024)

**Insight:** Self-verification enables cascading without a separate scoring
model.

**Roko adaptation:** Gate verification serves as Roko's self-verification.
The compile, test, and lint gates provide ground-truth feedback that is more
reliable than LLM self-verification because gates are deterministic.

### 3.4 Cascade Routing (ETH Zurich; arXiv:2410.10347)

**Insight:** Routing picks one model per query; cascading runs increasingly
larger models until an answer is good enough. Dekoninck et al. derive optimal
strategies for both and unify them into cascade routing, which outperforms
either alone; good quality estimators are the critical factor.

**Roko implementation:** `ProviderHealthRegistry` + `CascadeRouter` +
`LatencyRegistry` cover routing: provider health filters degraded providers and
the cascade router selects models. Cascading happens twice: the fallback chain
moves to another model when a call fails, and the model ladder
(`[routing.ladder]`) moves a plan task one rung up after two failed gate
verdicts. Cascade routing's finding applies to the second: the gate verdict is
Roko's quality estimate, and the payoff depends on how good it is.

### 3.5 Router-R1

A router that is itself an LLM, trained with reinforcement learning to
interleave reasoning ("think") with calls to other models ("route") over
several rounds and to aggregate their answers. Unlike RouteLLM's classifier
approach, Router-R1 reasons explicitly before and between routing decisions.

**Roko relevance:** The cascade router's stage transitions (Static ->
Confidence -> UCB) can be seen as a hardcoded reasoning chain. Router-R1
suggests that this chain itself could be learned -- an ADAS-level
optimization (see
[autocatalytic-compounding.md](autocatalytic-compounding.md)).

---

## 4. External Verifier Requirement

The self-improvement literature consistently identifies an external verifier
as the prerequisite for genuine self-improvement:

| Paper | Finding |
|-------|---------|
| Huang et al. (ICLR 2024) | LLMs cannot reliably improve without external feedback |
| Song et al. (ICLR 2025) | Self-generated feedback is unreliable for self-improvement |
| Pan et al. (ICML 2024) | External verification required for training signal quality |

**Roko's external verifier:** The 19-gate pipeline (compile, test, clippy,
diff, format, doc, etc.) provides deterministic external verification. This
is stronger than weak verifiers (LLM-as-judge) because:

1. Gate outcomes are not subject to model bias or hallucination.
2. Gates are deterministic -- the same input always produces the same verdict.
3. Gates are independent of the model being evaluated.
4. Gate results are machine-readable (pass/fail with optional signature).

The gate pipeline's role as external verifier is what distinguishes Roko's
self-improvement from prompt-only self-improvement systems that lack
ground-truth feedback.

---

## 5. Four Key Metrics

From production analysis of the predecessor system (mori), four metrics
capture the axes along which self-improvement occurs:

| Metric | Definition | Self-Improvement Lever |
|--------|-----------|----------------------|
| First-attempt pass rate | % tasks passing gates on first try | Playbook rules prevent known failures |
| Iterations per plan | Avg iterations to complete a plan | Better routing + better prompts |
| Cost per plan | Total USD per plan execution | Model routing + cache optimization |
| Prompt tokens per spawn | Input tokens for initial prompt | Context assembly optimization |

Every learning subsystem ultimately aims to improve one or more of these
metrics. The compound effect of all four improving simultaneously is
multiplicative: see the autocatalytic compounding analysis in
[autocatalytic-compounding.md](autocatalytic-compounding.md).

---

## 6. Improvement Safety

### 6.1 Constitutional Constraints

Self-improvement must be bounded. Constitutional constraints (inspired by
Bai et al. 2022) prevent learning subsystems from disabling gates, modifying
safety modules, or reducing quality below configurable floors:

```toml
[safety.constitution]
gates_immutable = true
self_modification_forbidden_crates = ["roko-gate", "roko-agent/safety"]
min_quality_model_tier = "standard"
quality_floor = 0.50
self_mod_requires_review = true
```

### 6.2 Gate Gaming Detection

The most insidious failure mode is gate gaming: the system learns to produce
outputs that pass gates without actually solving the task. Detection relies
on divergence signals:

```
Gate gaming indicators:
    1. Pass rate increases while downstream quality decreases
    2. Output complexity decreases (shorter, simpler code)
    3. Test coverage decreases while test pass rate increases
    4. Diff size shrinks toward zero (minimal changes)
```

The `GateGamingDetector` monitors for concurrent increase in pass rate and
decrease in quality score, alerting when the divergence exceeds a
configurable threshold (default: 5% quality decrease paired with >10% pass
rate increase over a 50-episode window).

### 6.3 Improvement Velocity Limits

Even beneficial improvements are rate-limited to prevent cascade failures:

| Limit | Default | Purpose |
|-------|---------|---------|
| Max playbook rule changes/day | 10 | Prevent prompt instability |
| Max routing table changes/day | 20 | Prevent model oscillation |
| Max experiment conclusions/day | 5 | Prevent premature conclusions |
| Safety violation cooldown | 60 min | Recovery time after violation |
| Max C-Factor delta/episode | 0.02 | Damping against rapid change |

These limits prevent a scenario where a false positive in the improvement
pipeline triggers a cascade of changes that collectively degrade the system.

---

## 7. Improvement Measurement

### 7.1 Scorecard

```rust
pub struct ImprovementScoreCard {
    pub window: TimeWindow,
    pub baseline: PeriodMetrics,
    pub current: PeriodMetrics,
    pub significance: SignificanceTests,
    pub confounds: Vec<Confound>,
}
```

The scorecard compares a current period against a baseline period using
three statistical tests:

- **Two-proportion z-test** for pass rate difference
- **Welch's t-test** for cost difference
- **Mann-Whitney U test** for iteration count (non-parametric)

### 7.2 Attribution

When improvement is detected, attribution identifies which learning
subsystem caused it by checking, in order: model routing changes, new
playbook rules promoted, skill library growth, and prompt experiment
conclusions. Residual unexplained improvement is logged for investigation.

### 7.3 Monotonicity Tracking

Self-improvement should be monotonic. The C-Factor trend is tracked:

```
Monotonicity score = fraction of steps where C(t) > C(t-1)

If monotonicity < 0.60 over 20+ episodes:
    -> Learning system is not converging
    -> Investigate: oscillation? regression? environmental shift?
```

---

## 8. Framework Comparison Matrix

| Framework | Input | Output | Learning Signal | Persistence | Roko Equivalent |
|-----------|-------|--------|----------------|-------------|-----------------|
| Reflexion | Failed attempt | NL reflection | Task retry success | Per-task context | Playbook rules |
| ExpeL | Episode batch | Generalized insights | Insight validation | Cross-task library | Skill library |
| DSPy | Program signature | Optimized prompt | Test set accuracy | Static compilation | Prompt experiments |
| Voyager | Exploration | JS function | Environment feedback | Skill library | Skill library |
| RouteLLM | Query | Strong/weak routing | Human preference | Router weights | Cascade router |
| FrugalGPT | Query | Model cascade | Scoring model | Cascade config | Cascade router |
| AutoMix | Query | Self-verified cascade | Self-verification | None | Gate pipeline |
| GRASP | Candidate rule | Admission decision | Regression budget | Probe set | Target design |
| SiriuS | Failed episode | Augmented episode | Corrective feedback | Training set | Hindsight system |
| SkillZip | Skill library | Compressed library | MDL criterion | Compressed store | Target design |

---

## 9. Connection to AI Safety Research

The improvement safety framework draws on four lines of research:

1. **Constitutional AI** (Bai et al. 2022) -- inviolable rules that
   constrain self-improvement. Roko's constitutional constraints implement
   the safety analogue.

2. **Scalable oversight** (Amodei et al. 2016) -- as systems become more
   capable, human oversight must scale. The `self_mod_requires_review` flag
   ensures human-in-the-loop for self-referential changes.

3. **Reward hacking** (Skalse et al. 2022) -- optimizing for a proxy metric
   (gate pass rate) can diverge from the true objective (correct code). Gate
   gaming detection monitors for this divergence.

4. **Self-play safety** (Silver et al. 2017) -- self-play can discover
   exploits in the reward function. The holdout experiment design provides
   a control group that detects if the "improved" system is gaming rather
   than improving.

---

## 10. Open Research Questions

1. **Can a system improve its own improvement mechanisms?** The autocatalytic
   thesis says yes in principle, but empirical evidence is limited to
   Karpathy's autoresearch experiment (11% speedup) and small-scale ADAS
   results (+14% on ARC). Transfer to large-scale software engineering is
   untested.

2. **Does the external verifier create a ceiling?** The gate pipeline
   provides verification but does not itself improve. A system that improves
   its own verifiers (adding test cases, discovering lint rules) would have a
   higher improvement ceiling.

3. **What is the optimal exploration budget?** The optimal tradeoff between
   exploration and exploitation depends on the rate of environmental change,
   which is itself changing. Adaptive exploration budgets (Thompson Sampling
   with drift) are theoretically sound but empirically untested in agent
   systems.

4. **Can cross-project transfer overcome cold start?** Skills and patterns
   from project A may accelerate project B, but transfer quality depends on
   structural similarity. HDC fingerprints enable fast similarity matching,
   but transferred knowledge quality is untested at scale.

---

## References

- Bai, Y. et al. (2022). Constitutional AI: Harmlessness from AI Feedback.
  arXiv:2212.08073.
- Chen, L. et al. (2023). FrugalGPT: How to Use Large Language Models While
  Reducing Cost and Improving Performance. arXiv:2305.05176.
- Huang, J. et al. (2024). Large Language Models Cannot Self-Correct
  Reasoning Yet. *ICLR 2024*.
- Khattab, O. et al. (2024). DSPy: Compiling Declarative Language Model
  Calls into Self-Improving Pipelines. *ICLR 2024*.
- Ong, I. et al. (2025). RouteLLM: Learning to Route LLMs with Preference
  Data. *ICLR 2025*.
- Shinn, N. et al. (2023). Reflexion: Language Agents with Verbal
  Reinforcement Learning. *NeurIPS 2023*.
- Wang, G. et al. (2023). Voyager: An Open-Ended Embodied Agent with Large
  Language Models. *NeurIPS 2023 (Oral)*.
- Zhao, A. et al. (2024). ExpeL: LLM Agents Are Experiential Learners.
  *AAAI 2024*.
