# 05 -- Dual-Process Tier Routing

> **Implementation status (2026-09):** The CascadeRouter, LinUCB
> bandit, Pareto frontier computation, Thompson sampling, and anomaly
> detection are all wired. Routing state persists to
> `.roko/learn/cascade-router.json`. Decisions improve across sessions.

---

## The Cognitive Model

Roko's model routing is inspired by dual-process theory from cognitive
science (Kahneman, 2011, *Thinking, Fast and Slow*):

- **System 1** (fast, automatic) -- quick, pattern-matching responses.
  Low cost, low latency. Maps to the Fast model tier (Haiku-class).
- **System 2** (slow, deliberate) -- careful reasoning, multi-step
  analysis. Higher cost, higher quality. Maps to the Premium model tier
  (Opus-class).

Kahneman's central insight is that most cognitive work is handled by
System 1, with System 2 intervening only when System 1 detects
anomalies or lacks confidence. The same holds for agent tasks: most
agent work does not need premium models. Classification, validation,
orchestration overhead, and simple code changes can be handled by fast
models. Only hard debugging, architectural decisions, and complex
reasoning need premium models. Routing everything through premium
models wastes money without improving outcomes.

---

## Three Model Tiers

```rust
pub enum ModelTier {
    Fast,      // Haiku-class: classification, watchers, orchestration
    Standard,  // Sonnet-class: implementation, review (the workhorse)
    Premium,   // Opus/GPT-5-class: architecture, hard debugging
}
```

| Tier | Examples | Typical cost | Use cases |
|---|---|---|---|
| Fast | Claude Haiku, GPT-4o-mini, Cerebras | ~$0.25/M input | Watchers, validators, conductors |
| Standard | Claude Sonnet, GPT-4o | ~$3/M input | Implementation, review, testing |
| Premium | Claude Opus, GPT-5 | ~$15/M input | Architecture, hard debug, audit |

Each `AgentRole` has a default tier, but the CascadeRouter can override
it dynamically based on learned performance data.

---

## Three Cognitive Speeds

The dual-process model maps to three cognitive speeds in Roko's
execution, corresponding to Gamma/Theta/Delta heartbeat timescales
(Kahneman, 2011, Ch. 1--3):

| Speed | Tier | Latency | Examples |
|---|---|---|---|
| **T0 (Gamma)** | Fast | ~5-15s | File read, classification, watchers, validators |
| **T1 (Theta)** | Standard | ~75s | Implementation, review, testing |
| **T2 (Delta)** | Premium | Hours | Architecture, research, complex debug |

The CascadeRouter uses these speeds as a prior: Gamma tasks start at
Fast tier, Theta tasks start at Standard, Delta tasks start at Premium.
The bandit can override these starting points based on learned
performance data.

---

## CascadeRouter

The CascadeRouter implements the dual-process model as a multi-stage
confidence cascade:

```
Task arrives
    |
    v
Stage 1: Try Fast model (System 1)
    |-- Confidence >= threshold --> Accept result, done
    v
Stage 2: Try Standard model
    |-- Confidence >= threshold --> Accept result, done
    v
Stage 3: Try Premium model (System 2)
    |-- Accept result regardless
```

### Confidence computation

The confidence score is a weighted combination of signals:

```
confidence = w1 * gate_pass_rate
           + w2 * (1 - uncertainty_markers)
           + w3 * historical_success_rate
           + w4 * task_complexity_estimate
```

The weights are learned via the LinUCB bandit. The threshold for
accepting a fast-model result depends on the temperament setting:
Conservative requires 0.9 confidence, Balanced requires 0.7.

### Persistence

The CascadeRouter persists its state to
`.roko/learn/cascade-router.json`. This means routing decisions improve
across sessions -- a model that consistently fails for a task type will
be avoided in future runs.

---

## LinUCB Bandit

The CascadeRouter uses a **LinUCB contextual bandit** (Li et al., 2010,
"A contextual-bandit approach to personalized news article
recommendation," WWW 2010) to select models within each tier:

### How it works

1. **Context vector** -- for each task, compute features: task type,
   estimated complexity, historical performance, role, current budget.
2. **Arm selection** -- each model is an "arm". LinUCB computes an
   upper confidence bound for each arm given the context.
3. **Reward** -- after the task completes, the reward signal combines:
   gate pass/fail, token efficiency, wall-clock time, cost.
4. **Update** -- the bandit updates its weight matrix for the selected
   arm.

```
For each task:
  1. Observe context x (task type, complexity, role, budget)
  2. Select model a = argmax(theta_a . x + alpha * sqrt(x' A_a^(-1) x))
  3. Observe reward r (gate pass/fail, tokens, latency, cost)
  4. Update: A_a += x * x', b_a += r * x
```

LinUCB balances exploration (trying new models to learn their
performance) with exploitation (using the model historically best for
this context). The exploration parameter is controlled by temperament:
Exploratory temperament sets a high exploration parameter, causing the
bandit to try more models.

---

## Pareto Frontier Pruning

Before the bandit selects a model, a Pareto frontier computation prunes
the candidate set. Models are evaluated on two dimensions:

1. **Quality** -- historical gate pass rate for the task type.
2. **Cost** -- price per million tokens.

Models that are dominated (worse on both dimensions than another model)
are removed from consideration. This prevents the bandit from exploring
obviously bad options.

---

## Thompson Sampling

For the confidence-threshold decision (escalate or accept?), the
CascadeRouter uses Thompson sampling over the weighted confidence
signals:

```
For each tier t:
    sample theta_t ~ Beta(successes_t, failures_t)
    adjusted_confidence = theta_t * raw_confidence
```

This introduces beneficial randomness: even when the fast model's
average confidence is below threshold, it occasionally gets a chance
(when the sampled theta is high), allowing the system to discover that
the fast model has improved for certain task types.

---

## Anomaly Detection

The `AnomalyDetector` monitors model performance for unusual patterns:

- **Sudden quality drops** -- a model's gate pass rate drops
  significantly below its historical baseline.
- **Latency spikes** -- response times exceed 2x the rolling average.
- **Cost anomalies** -- token usage significantly higher than expected.

When an anomaly is detected, the router temporarily de-prioritizes the
affected model and records an alert.

---

## EMA-Based Adaptation

The CascadeRouter uses exponential moving averages for rapid adaptation
to shifts in model pricing and quality:

```rust
pub struct EmaStats {
    pub alpha: f64,  // Smoothing factor (default: 0.05)
    pub model_stats: HashMap<String, ModelRunningStats>,
}

pub struct ModelRunningStats {
    pub pass_rate: f64,        // EMA of gate pass rate
    pub latency_ms: f64,       // EMA of average latency
    pub cost_per_task: f64,    // EMA of cost per task (USD)
    pub token_efficiency: f64, // EMA of useful/total tokens
    pub observation_count: u64,
}
```

---

## Dual-Process Theory 2.0: Competing Intuitions

The classical System 1/System 2 dichotomy from Kahneman (2011) has been
refined by subsequent cognitive science research (De Neys & Pennycook,
2019; De Neys, 2018):

### Competing Intuitions Model

People can process logical principles *intuitively*, without
deliberation. The revised model proposes **multiple types of
intuitions**: some logical and reliable, others heuristic and less
reliable. These competing intuitions differ in **activation strength**.

**Mapping to Roko:** The CascadeRouter's confidence signal is the
activation strength. When the Fast tier's confidence is high (strong
intuition), accept immediately. When confidence is uncertain (competing
intuitions), escalate to Standard or Premium tier (deliberation).

### Hybrid Two-Stage Model

The **Hybrid Two-Stage model** best maps to Roko's CascadeRouter:

1. A "shallow analytic monitoring process" (confidence estimation) is
   always active.
2. An "optional deeper processing stage" (model escalation) activates
   only when conflict is detected (low confidence, gate failure).

### Triple-Process Theory: Type 3 Metacognition

Evans (2019) proposes a **Type 3 metacognitive process** that sits
above both System 1 and System 2 as a regulatory mechanism.

**Mapping to Roko:** Type 3 = meta-routing (routing the router). The
metacognitive layer decides *whether* to engage the CascadeRouter's
learned model selection or to use a simple heuristic.

---

## Avoiding Expert Collapse

A key concern is **model monoculture** -- always routing to one
familiar model. The LinUCB exploration parameter and Thompson sampling
address this, but additional mechanisms help:

- **Minimum exploration rate** -- fraction of tasks routed to
  non-default models even when the default appears optimal (5%).
- **Geometric forgetting** -- forgetting factor for sufficient
  statistics, allowing adaptation to model changes.
- **Maximum consecutive uses** -- forced exploration after 20
  consecutive uses of the same model.
- **Diversity bonus** -- reward for models that have not been used
  recently.

---

## Active Inference Connection

The routing system is theoretically grounded in the Free Energy
Principle (Friston, 2006). The CascadeRouter's behavior can be
interpreted as minimizing expected free energy:

- **Epistemic value** -- exploration (trying new models) reduces
  uncertainty about model performance, lowering expected free energy.
- **Pragmatic value** -- exploitation (using known-good models)
  directly achieves task objectives.
- **Confidence threshold** -- acts as a precision parameter: high
  precision (Conservative) demands more evidence before accepting, low
  precision (Exploratory) accepts with less evidence.

---

## Mixture of Experts Connection

MoE routing within a single model (choosing experts per token) is
architecturally analogous to model-level routing (choosing between
LLMs per query):

| MoE concept | Model routing equivalent |
|---|---|
| Gating network | CascadeRouter |
| Expert | Individual model (Haiku, Sonnet, Opus) |
| Top-K selection | Cascade stages |
| Load balancing | Rate limit + cost budget distribution |
| Expert collapse | Model monoculture |
| Sparse activation | Only invoking the cheapest sufficient model |

---

## Research Context

| System | Approach | Key result |
|--------|----------|------------|
| RouteLLM (2024) | Binary classifier for cheap/expensive routing | 85% cost reduction on MT Bench |
| FrugalGPT (2023) | Cascade + caching | Cost-efficient LLM serving |
| Router-R1 (2025) | RL-trained multi-round router | Open-sourced model weights |
| xRouter (2025) | Cost-aware RL orchestration | 80--90% GPT-5 accuracy at <1/5 cost |
| IRT-Router (2025) | Psychometric routing via Item Response Theory | Superior cold-start across 20 LLMs |
| BEST-Route (2025) | Test-time compute allocation | 60% cost reduction |
| PILOT (2025) | Offline preference priors + online bandits | Shared embedding space |
| kNN routing (2025) | Simple non-parametric methods | Matches or outperforms learned routers |

Roko's contribution is combining these approaches into a unified system:
Pareto pruning (multi-objective optimization) then LinUCB selection
(contextual bandits) then Thompson sampling for confidence (Bayesian
decision theory) then anomaly detection for robustness.

---

## Implementation Sources

| File | Purpose |
|------|---------|
| `crates/roko-learn/src/` | CascadeRouter, LinUCB, Pareto frontier, anomaly detection |
| `.roko/learn/cascade-router.json` | Persisted routing state |
| `crates/roko-runtime/src/heartbeat.rs` | Gamma/Theta/Delta timescales |
| `crates/roko-core/src/agent.rs` | ModelTier enum |

---

## Citations

1. Kahneman, D. (2011). *Thinking, Fast and Slow.* Farrar, Straus and
   Giroux. -- Dual-process theory grounding T0/T1/T2 routing; System 1
   automaticity; System 2 intervention on anomaly detection.
2. Li, L. et al. (2010). "A contextual-bandit approach to personalized
   news article recommendation." WWW 2010. -- LinUCB algorithm.
3. Friston, K. (2006). "A free energy principle for the brain." Journal
   of Physiology - Paris. -- Free Energy Principle.
4. De Neys, W. & Pennycook, G. (2019). "Logic, Fast and Slow." Current
   Directions in Psychological Science. -- Competing intuitions.
5. Chen, Z. et al. (2025). "Router-R1." arXiv:2506.09033. -- RL router.
6. Ong, I. et al. (2025). "RouteLLM." arXiv:2406.18665. -- Binary
   routing.
7. Chen, L. et al. (2023). "FrugalGPT." -- Cascade routing.
8. Qian, C. et al. (2025). "xRouter." arXiv:2510.08439. -- Cost-aware.
9. Song, J. et al. (2025). "IRT-Router." arXiv:2506.01048. --
   Psychometric routing.
10. Ding, Y. et al. (2025). "BEST-Route." arXiv:2506.22716. --
    Test-time compute allocation.
