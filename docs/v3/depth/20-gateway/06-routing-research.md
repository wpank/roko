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

Router-R1 trains a small LLM to serve as the router itself, using reinforcement
learning to produce chain-of-thought reasoning traces before making routing decisions.
The router observes the query, reasons about its difficulty and structure, and then
selects a target model.

### Results

Outperforms hand-crafted routing heuristics and supervised classification routers on
Arena-Hard benchmarks. The reasoning trace provides interpretability -- operators can
inspect why a particular model was chosen.

### Relevance to Roko

Demonstrates that routing decisions benefit from explicit reasoning, not just feature
vectors. Roko's current CascadeRouter uses statistical features (task category,
complexity band, iteration count) rather than reasoning traces. The RoutingContext
metadata could be enriched with a lightweight reasoning step in future work.

---

## 3. xRouter (Qian et al. 2025)

**"xRouter: Training Cost-Aware LLMs Orchestration System via Reinforcement Learning"**
Cheng Qian, Zuxin Liu, Shirley Kokane, et al. arXiv:2510.08439, 2025.

### Key Insight

Most routing research evaluates single-turn queries. xRouter extends routing to
multi-turn conversations by using holistic trajectory evaluation rather than per-turn
classification. A conversation that starts simple may become complex mid-stream;
per-turn routing misses this.

### Results

Shows that naive turn-by-turn routing underperforms trajectory-aware routing by
5-15% on multi-turn benchmarks. The router considers the full conversation history,
not just the latest message.

### Relevance to Roko

Roko's gateway already receives per-request metadata including `iteration` count
and `progress_marker`, which provide weak trajectory signals. The `InferenceMeta`
struct carries task category and complexity hints that could evolve toward full
trajectory awareness. The loop detector and convergence detector provide additional
cross-turn signals that a trajectory-aware router could consume.

---

## 4. IRT-Router (Song et al. 2025)

**"IRT-Router: Effective and Interpretable Multi-LLM Routing via Item Response Theory"**
Wei Song, Zhenya Huang, Cheng Cheng, et al. arXiv:2506.01048, 2025.

### Key Insight

IRT-Router applies Item Response Theory (IRT) -- a psychometric framework for
standardized testing -- to jointly estimate model capability ("ability") and query
difficulty on a shared latent scale. This avoids the calibration problem where a
router trained on one task distribution fails on another.

### Results

Outperforms similarity-based routers (MoA, RouteLLM) by 4-12% on MMLU, BBH, and
heterogeneous task distributions. The shared latent scale enables zero-shot transfer
to unseen task types.

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

BEST-Route characterizes model strengths through probabilistic subspace profiles
rather than single quality scores. Each model is described by a distribution over
capability subspaces (e.g., "strong at math reasoning, weak at creative writing").
Routing becomes a Bayesian matching problem between query subspace and model profiles.

### Results

Achieves better calibration than point-estimate routers on heterogeneous task
distributions. The posterior uncertainty naturally handles exploration -- uncertain
subspaces are explored before being exploited.

### Relevance to Roko

The CascadeRouter's three-stage architecture already separates static routing
(operator preferences), confidence-based routing (learned reliability), and UCB1
exploration (bandit). BEST-Route's subspace profiles could replace or augment
the single-dimensional confidence scores with multi-dimensional capability
distributions. The `TaskCategory` enum (Scaffolding, Integration, Verification,
Research, Refactor, Infra, Docs, Implementation) provides a natural subspace
decomposition.

---

## 6. Unified Routing/Cascading Framework (Dekoninck et al. 2025)

**"A Unified Approach to Routing and Cascading for LLMs"**
Jasper Dekoninck, Maximilian Baader, Martin Vechev. arXiv:2410.10347, 2025.

### Key Insight

Proves that routing (choose one model) and cascading (try cheap model first, escalate
on failure) are special cases of a single decision framework. The optimal strategy is
a hybrid: route when the router is confident, cascade when uncertain. Neither pure
routing nor pure cascading dominates on cost-quality Pareto frontiers.

### Results

Shows that hybrid strategies achieve 10-25% better cost-quality tradeoffs than either
pure routing or pure cascading across multiple benchmarks. The framework provides a
principled way to combine routing confidence with cascading fallback.

### Relevance to Roko

Roko's gateway already implements this hybrid naturally:
- **Routing:** The CascadeRouter selects a primary model based on task context.
- **Cascading:** The `call_with_fallbacks` method tries fallback models on retryable
  failures (429/503/timeout).

The unified framework's insight is that the confidence threshold for routing vs.
cascading should be adaptive. Currently, Roko always routes first and only cascades
on failure. A more sophisticated strategy would cascade preemptively when the router's
confidence is below a learned threshold.

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
| **UCB1** | Upper Confidence Bound exploration-exploitation | BEST-Route uncertainty-driven exploration |

The routing context carries signals that map to the research:

| RoutingContext Field | Research Parallel |
|---------------------|-------------------|
| `task_category` | IRT-Router item type; BEST-Route subspace |
| `complexity` | IRT-Router difficulty parameter |
| `iteration` | xRouter trajectory position |
| `role` | BEST-Route model-task affinity |

### Where the Research Points Next

1. **Reasoning-augmented routing** (Router-R1): Use a small LLM to reason about
   routing decisions rather than relying on feature vectors.
2. **Trajectory-aware routing** (xRouter): Extend `InferenceMeta` to carry full
   conversation trajectory summaries.
3. **Continuous difficulty estimation** (IRT-Router): Replace discrete
   Fast/Standard/Complex with a continuous latent variable.
4. **Subspace profiles** (BEST-Route): Multi-dimensional capability distributions
   per model, not single reliability scores.
5. **Adaptive cascade thresholds** (Unified Framework): Learn when to route vs.
   cascade rather than always routing first.
