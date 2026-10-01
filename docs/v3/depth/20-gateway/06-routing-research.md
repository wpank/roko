# 20-gateway/06 -- Routing Research Context

> Academic context for the gateway's model routing architecture. Surveys recent
> work on learned LLM routers, cascading, and cost-quality optimization.

**Parent:** [20-GATEWAY](../../20-GATEWAY.md), section 19

---

## 1. The Routing Problem

Given a query and a portfolio of LLM providers with different cost/quality/latency
profiles, which model should serve the request? This is a real-time decision under
uncertainty: query difficulty is unknown before the call, model quality varies by task,
and the cost of a wrong choice is wasted tokens or degraded output.

The naive approaches are:
- **Always use the best model:** Correct but expensive. Most queries do not need the
  strongest model.
- **Always use the cheapest model:** Cheap but unreliable. Complex queries get poor
  answers.
- **Human-engineered rules:** Fragile, hard to maintain, and blind to distribution
  shifts.

The research community has converged on **learned routers** -- classifiers that predict
which model will succeed on a given query and route accordingly. Roko's `CascadeRouter`
implements a three-stage architecture (Static -> Confidence -> UCB1) that draws on this
work.

---

## 2. Router-R1 (Zhang et al. 2025)

**"Router-R1: Teaching LLMs Multi-Round Routing and Aggregation via Reinforcement Learning"**
Haozhen Zhang, Tao Feng, Jiaxuan You. arXiv:2506.09033, 2025.

### Key Insight

Router-R1 makes the router itself an LLM and trains it with reinforcement learning to
treat routing as a sequential decision process. The router interleaves "think" steps
(internal deliberation) with "route" steps that call other models, folds each answer
back into its context, and can consult several models before it responds: it both
routes and aggregates. Its reward combines format, final-outcome and cost terms, and it
sees each candidate model only through simple descriptors (price, latency, example
performance).

### Results

On seven general and multi-hop question-answering benchmarks it outperforms several
strong baselines while managing cost, and because it conditions only on model
descriptors it generalizes to models it has not seen. Code, models and datasets are
public.

### Relevance to Roko

Demonstrates that routing decisions can benefit from explicit reasoning and from
consulting more than one model, not just from feature vectors. Roko's current CascadeRouter uses statistical features (task category,
complexity band, iteration count) rather than reasoning traces. The RoutingContext
metadata could be enriched with a lightweight reasoning step in future work.

---

## 3. xRouter (Qian et al. 2025)

**"xRouter: Training Cost-Aware LLMs Orchestration System via Reinforcement Learning"**
Cheng Qian, Zuxin Liu, Shirley Kokane, et al. arXiv:2510.08439, 2025.

### Key Insight

xRouter is a tool-calling router: a learned router model either answers a query
itself or invokes one or more external models. It is trained end to end with
reinforcement learning against an explicit, cost-aware reward that encodes the
cost-performance trade-off, in place of hand-written escalation rules and keyword
heuristics.

### Results

Across diverse benchmarks it reaches strong cost-performance trade-offs: substantial
cost reductions at comparable task completion rates. The authors also report what did
and did not help learned routing, including how hard it is to elicit sophisticated
orchestration from small open models. The implementation is open.

### Relevance to Roko

In Roko, cost enters routing through configuration: the model ladder's rungs run
cheapest first (`[routing.ladder]`) and `[budget]` sets ceilings, while the
CascadeRouter's bandit learns from task outcomes.
xRouter shows the alternative: put cost into the router's reward, so that the
cost-quality trade-off is learned. Its finding about small open models is a caution
for any plan to make a cheap model the router.

---

## 4. IRT-Router (Song et al. 2025)

**"IRT-Router: Effective and Interpretable Multi-LLM Routing via Item Response Theory"**
Wei Song, Zhenya Huang, Cheng Cheng, et al. arXiv:2506.01048, 2025.

### Key Insight

IRT-Router borrows Item Response Theory (IRT), a psychometric method, to model the
relationship between each LLM's capabilities and the attributes of a query. The model
predicts how well each LLM will answer and yields interpretable estimates of LLM
ability and query difficulty. An online warm-up step based on semantic similarity
helps it generalize to new queries.

### Results

Across 20 LLMs and 12 datasets it outperforms most baseline methods in effectiveness
and interpretability, and it does especially well in cold-start scenarios.

### Relevance to Roko

The `CascadeRouter`'s UCB1 bandit stage learns model-specific reward distributions
conditioned on task category and complexity band. IRT's insight is that these
distributions should be projected onto a shared difficulty scale. The
`RoutingContext.complexity` field already provides a discrete difficulty estimate
(Fast/Standard/Complex). IRT suggests this should be a continuous latent variable
jointly estimated with model capability.

---

## 5. BEST-Route (Ding et al. 2025)

**"BEST-Route: Adaptive LLM Routing with Test-Time Optimal Compute"**
Dujian Ding, Ankur Mallick, Shaokun Zhang, et al. arXiv:2506.22716, 2025.

### Key Insight

Earlier routers draw one response from the chosen model, and one response from a small
model is often not good enough to beat one from a large model, so they overuse the
large model. Sampling several responses from a small model and keeping the best can
raise quality while still costing less than one large-model response. BEST-Route
builds on this: for each query it chooses a model and how many responses to sample
from it, based on the query's difficulty and a quality threshold.

### Results

On real-world datasets it cuts cost by up to 60% with less than a 1% drop in
performance (ICML 2025).

### Relevance to Roko

Roko's model ladder retries a failed task and moves it up a rung after two failures.
BEST-Route suggests spending part of that budget up front: sample several responses on
the cheap rung and keep the best, for example the first that passes its gates, before
escalating. The
CascadeRouter's per-category statistics could inform how many samples a query needs.

---

## 6. Unified Routing/Cascading Framework (Dekoninck et al. 2025)

**"A Unified Approach to Routing and Cascading for LLMs"**
Jasper Dekoninck, Maximilian Baader, Martin Vechev. arXiv:2410.10347, 2025.

### Key Insight

Routing picks one model per query; cascading runs increasingly larger models until an
answer is good enough. The paper derives an optimal cascading strategy, proves that an
existing routing strategy is optimal, and combines the two into cascade routing, a
unified framework that is optimal in its analysis. It identifies good quality
estimators as the critical factor in whether either paradigm pays off.

### Results

In its experiments cascade routing consistently outperforms routing and cascading
alone by a large margin, and an analysis of quality estimators shows when routing,
cascading or both are useful.

### Relevance to Roko

Roko's gateway already combines the two in a simple form:
- **Routing:** The CascadeRouter selects a primary model based on task context.
- **Cascading:** The `call_with_fallbacks` method tries fallback models on retryable
  failures (429/503/timeout).

Cascade routing makes the move to the next model a decision driven by estimated
answer quality, not by provider errors. Roko's plan runs have one such cascade: on the
model ladder (`[routing.ladder]`, on by default), a task whose attempts fail their
gates twice moves one rung up. The gate verdict is its quality estimate, and cascade
routing's analysis says the payoff depends on how good that estimate is.

---

## 7. Earlier Foundational Work

| Citation | Contribution |
|---|---|
| **FrugalGPT** (Chen et al. 2023, arXiv:2305.05176) | Demonstrated that cascade routing can match GPT-4 quality at 2% cost. Established the empirical case for learned routing. |
| **Friston 2006** | Free Energy Principle. Provides the theoretical basis for EFE-based routing: high-certainty queries route to cheap models (low epistemic value), high-uncertainty queries route to strong models (high epistemic value). |
| **Kanerva 2009** | Hyperdimensional computing. SimHash as used in the gateway's semantic cache and convergence detector is a derivative of Kanerva's binary HD vectors. |

---

## 8. Mapping to CascadeRouter Architecture

Roko's `CascadeRouter` (in `roko-learn`) implements a three-stage cascade:

| Stage | Mechanism | Academic Parallel |
|-------|-----------|-------------------|
| **Static** | Operator-configured model preferences | Baseline routing rules |
| **Confidence** | Bayesian reliability tracking per model | IRT-Router ability estimation |
| **UCB1** | Upper Confidence Bound exploration-exploitation | Multi-armed bandits (none of the papers above) |

The routing context carries signals that map to the research:

| RoutingContext Field | Research Parallel |
|---------------------|-------------------|
| `task_category` | IRT-Router query attributes |
| `complexity` | IRT-Router query difficulty; BEST-Route's difficulty-based sample count |

### Where the Research Points Next

1. **Reasoning-augmented routing** (Router-R1): An LLM router that reasons, and may
   consult several models, rather than relying on feature vectors.
2. **Cost in the reward** (xRouter): Learn the cost-quality trade-off by putting cost
   into the router's reward.
3. **Continuous difficulty estimation** (IRT-Router): Replace discrete
   Fast/Standard/Complex with a continuous latent variable.
4. **Best-of-n on the cheap tier** (BEST-Route): Choose how many responses to sample
   from a cheap model, by query difficulty, before escalating.
5. **Better quality estimates** (cascade routing): The model ladder already escalates
   on failed gate verdicts; since the payoff depends on the quality estimator, a
   calibrated estimate of answer quality could decide escalation sooner than two
   failures.
