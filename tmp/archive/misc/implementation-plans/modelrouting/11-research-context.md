# 11 — Research & Academic Context

> **Type**: Reference document (no tasks — background knowledge for implementation decisions)

## Purpose

This document captures the research underpinning the design decisions in docs 01–12. It provides citations, benchmark data, and architectural patterns from the broader LLM routing literature, agent runtime architectures, self-improvement theory, and formal verification. Agents implementing tasks should consult this when they need to understand *why* a design choice was made.

---

## 1. Model Benchmarks (April 2026)

### SWE-Bench Pro (hardest software engineering benchmark)

| Model | Score | Open-weight? |
|---|---|---|
| **GLM-5.1** | **58.4** | Yes (MIT) |
| GPT-5.4 | 57.7 | No |
| Claude Opus 4.6 | 57.3 | No |
| Gemini 3.1 Pro | 54.2 | No |

### SWE-Bench Verified

| Model | Score |
|---|---|
| Claude Opus 4.6 | **80.8%** |
| GPT-5.2 | 80.0% |
| MiniMax M2.5 | 80.2% |
| GLM-5.1 | 77.8% |
| Kimi-K2.5 | 76.8% |

### LiveCodeBench

| Model | Score |
|---|---|
| **Kimi-K2.5** | **85.0** |
| GLM-4.7 | 84.9 |
| Qwen 3.5-397B | 83.6 |

### Key Insight

All frontier models are within 1.3% on SWE-Bench Pro. The agent harness drives the remaining variance more than the model. Claude Code's 80.9% SWE-bench score exceeds raw Opus 4.6 because of Anthropic's tool use patterns, retry logic, and context management.

**Source**: [philschmid.de/agent-harness-2026](https://www.philschmid.de/agent-harness-2026)

---

## 2. LLM Routing Academic Literature

### RouteLLM (ICLR 2025, UC Berkeley)

Routes between strong (expensive) and weak (cheap) model using preference data. Matrix Factorization router is the best performer. 85% cost reduction for 95% of GPT-4 quality on MT-Bench. Key finding: 1,500 golden-label samples for data augmentation halved the required GPT-4 calls.

**Relevance**: RouteLLM's offline preference learning is complementary to roko's online LinUCB. Roko could augment observations with LLM-judge evaluations (doc 08, task 2G.19).

**Paper**: [arXiv:2406.18665](https://arxiv.org/pdf/2406.18665)

### MixLLM (2025)

Four-component system: tag-enhanced query embedding, per-LLM quality/cost prediction, meta decision maker with uncertainty, continual learning via Policy Gradient. Result: 97.25% of GPT-4 quality at 24.18% cost. Uses inverse covariance matrix per LLM (conceptually similar to roko's LinUCB `A^{-1}` matrix).

**Paper**: [arXiv:2502.18482](https://arxiv.org/html/2502.18482v1)

### FrugalGPT (Stanford, TMLR 2024)

LLM cascade with DistilBERT stop judge. Queries LLMs cheapest-first, stops when judge is confident. Up to 98% cost reduction matching GPT-4 quality.

**Relevance**: Roko's `CascadeRouter` with `CascadeStage::Static` → `Confidence` → `UCB` is a 3-stage cascade. FrugalGPT's "cheapest first with confidence stopping" could be a 4th stage.

**Paper**: [arXiv:2305.05176](https://arxiv.org/abs/2305.05176)

### AutoMix (NeurIPS 2024)

No-training-required router using few-shot self-verification + POMDP. Model checks its own answer, routes to stronger model if confidence is low. 50%+ cost reduction at comparable performance.

**Relevance**: AutoMix's self-verification could replace the LLM-judge for quality estimation (cheaper, no separate judge model needed).

**Paper**: [arXiv:2310.12963](https://arxiv.org/abs/2310.12963)

### Unified Routing & Cascading (ETH Zurich, ICLR 2025)

Derives theoretically optimal cascade strategy. Key finding: "quality estimators are the critical factor for success — the routing/cascading algorithm matters less than how well you can estimate output quality."

**Relevance**: Confirms roko's approach of investing in better signals (docs 08, 09) rather than more complex routing algorithms.

**Paper**: [arXiv:2410.10347](https://arxiv.org/abs/2410.10347)

### Router-R1 (NeurIPS 2025)

LLM-based router (Qwen2.5-3B, PPO-trained) that interleaves thinking and routing. Zero-shot generalization to new models.

**Paper**: [arXiv:2506.09033](https://arxiv.org/abs/2506.09033)

### Speculative Cascades (Google Research)

Combines speculative decoding with cascading. Small model drafts, large model verifies. Multiple deferral rules: confidence-based, comparative, cost-benefit, token-ranking.

**Source**: [Google Research Blog](https://research.google/blog/speculative-cascades-a-hybrid-approach-for-smarter-faster-llm-inference/)

---

## 3. Production Routing Systems

### LiteLLM (100+ providers)

- `BaseConfig` transform pattern: all providers subclass one base with `transform_request()` / `transform_response()`
- Cooldown logic: 3 failures → 5s cooldown, error-type-specific retry policies
- 6 routing strategies: simple-shuffle, rate-limit-aware, latency-based, cost-based, least-busy, usage-based
- OpenAI as lingua franca: canonical format is OpenAI, adapters translate

**Source**: [github.com/BerriAI/litellm](https://github.com/BerriAI/litellm)

### genai (Rust crate)

- `AdapterKind` enum + static dispatch (no vtables)
- Static `Adapter` trait methods (no `&self`)
- Pattern-based model→provider resolution
- OpenAI-compatible providers delegate to `OpenAIAdapter` utilities, only overriding endpoint/auth

**Source**: [github.com/jeremychone/rust-genai](https://github.com/jeremychone/rust-genai)

### OpenRouter

- Inverse-price-squared weighting for provider selection
- Rolling 5-minute window with p50/p75/p90/p99 percentiles
- 300+ models, 60+ providers
- `provider` routing params: sort, order, max_price, require_parameters

**Source**: [openrouter.ai/docs](https://openrouter.ai/docs/guides/routing/provider-selection)

### Portkey AI Gateway

- Lightweight routing layer (~122KB), 10B+ tokens/day
- Fallback chains, weight-based load balancing
- Both exact-match and semantic caching
- 50+ pre-built guardrails

**Source**: [github.com/Portkey-AI/gateway](https://github.com/Portkey-AI/gateway)

---

## 4. Observability Standards

### OpenTelemetry GenAI Semantic Conventions

Standardized span attributes for LLM calls (Development status, v1.38.0+):

| Attribute | Description |
|---|---|
| `gen_ai.operation.name` | chat, embeddings, text_completion |
| `gen_ai.request.model` | Requested model |
| `gen_ai.response.model` | Actual model |
| `gen_ai.usage.input_tokens` | Input tokens |
| `gen_ai.usage.output_tokens` | Output tokens |
| `gen_ai.usage.cache_read.input_tokens` | Cached tokens |
| `gen_ai.tool.name` | Tool name |
| `gen_ai.tool.call.id` | Tool call ID |

**Source**: [opentelemetry.io/docs/specs/semconv/gen-ai](https://opentelemetry.io/docs/specs/semconv/gen-ai/gen-ai-spans/)

### Langfuse Generation Schema

Key fields per LLM call: `model`, `modelParameters`, `input`, `output`, `inputTokens`, `outputTokens`, `totalTokens`, `cache_read_input_tokens`, `inputCost`, `outputCost`, `totalCost`, `completionStartTime` (TTFT).

**Source**: [langfuse.com/docs/tracing-data-model](https://langfuse.com/docs/tracing-data-model)

### Cost Normalization (Artificial Analysis methodology)

- **OpenAI tokens** (tiktoken o200k_base) as universal unit
- **3:1 input/output ratio** for blended cost
- TTFT, output speed, total response time measured per provider
- Intelligence Index v4.0: composite of 10 evaluations

**Source**: [artificialanalysis.ai/methodology](https://artificialanalysis.ai/methodology)

---

## 5. Prompt Caching Economics

| Provider | Cache Read Discount | Min Cacheable | TTL |
|---|---|---|---|
| Anthropic | 90% (10% of base) | 1,024 tokens | 5min → 1hr |
| OpenAI | 50% | 1,024 tokens | Automatic |
| DeepSeek | 90% | 64 tokens | Automatic |
| GLM-5.1 | ~80% ($0.26 vs $1.40/M) | Unknown | Automatic |
| Kimi-K2.5 | ~83% ($0.10 vs $0.60/M) | Unknown | Automatic |

**Key insight**: With 87% cache hit rate in production, effective cost reduction is ~78%. Roko's `SystemPromptBuilder` generates 6-layer prompts with substantial shared prefixes across tasks in the same plan. Cache affinity routing (2G.08–2G.09) captures this optimization.

---

## 6. Circuit Breaker Patterns

### State Machine

```
    CLOSED  ──[3+ failures]──→  OPEN
       ↑                           │
       │                      [cooldown]
       │                           │
       └──[probe success]←──  HALF-OPEN
```

### Error-Specific Cooldowns (from LiteLLM)

| Error Type | Cooldown | Retry? |
|---|---|---|
| Rate Limit (429) | 5 seconds | Yes, after cooldown |
| Auth Failure (401/403) | 5 minutes | No |
| Timeout (408) | 10 seconds | Try fallback |
| Server Error (500+) | 30 seconds | Try fallback |
| Content Policy | 0 | No retry (prompt issue) |
| Context Overflow | 0 | Try with smaller context |

---

## 7. Cost-Quality Pareto Frontiers

A model is **Pareto-optimal** if no other model has both higher quality and lower cost. Non-Pareto models are "dominated" — there's always a better choice available.

**Computation**: O(n²) dominance check. For n models with (quality, cost) observations:
```
for each model A:
    dominated = exists model B where B.quality >= A.quality AND B.cost <= A.cost
                AND (B.quality > A.quality OR B.cost < A.cost)
    if not dominated: A is on the frontier
```

**Application**: Roko recomputes the frontier every 50 observations. Non-frontier models get 90% less exploration in the LinUCB bandit, accelerating convergence to cost-optimal choices.

---

## 8. A/B Testing for LLMs

### TensorZero Track-and-Stop

- Generalized Likelihood Ratio Test (GLRT) for faster convergence
- Anytime-valid confidence sequences via martingale theory
- 37% faster identification of best variant vs uniform sampling
- Second-order cone program (SOCP) for optimal traffic allocation

**Source**: [tensorzero.com/blog/bandits-in-your-llm-gateway](https://www.tensorzero.com/blog/bandits-in-your-llm-gateway/)

### Roko's Approach

Uses UCB1 (Upper Confidence Bound) for both prompt experiments and model experiments. UCB1 is simpler than Track-and-Stop but well-suited for roko's sample sizes (tens to hundreds of observations, not thousands).

UCB1 score: `mean_reward + sqrt(2 * ln(total_trials) / variant_trials)`

Unsampled variants get score = infinity (forced exploration).

Conclusion check: all variants ≥ min_trials AND best variant leads by ≥ min_effect_size.

---

## 9. Quality Estimation Hierarchy

From most to least reliable (per academic survey):

1. **Probe-based**: Hidden-state classifiers on model internals (requires weights)
2. **Perplexity-based**: Token logits / softmax (requires logit access)
3. **LLM-as-judge**: Strong model evaluates output (extra API call)
4. **Self-consistency**: Multiple samples, majority vote (multiplies calls)
5. **Few-shot self-verification**: Model checks its own answer (AutoMix)
6. **Tool call / gate outcomes**: Binary success signal (roko's current approach)
7. **Self-reported confidence**: Ask model how confident it is (unreliable)

Roko uses #6 for gateable tasks (compile, test, clippy) and adds #3 (LLM-as-judge via 2G.19) for non-gateable tasks.

---

## 10. Drift Detection Methods

| Method | Threshold | What It Catches |
|---|---|---|
| Cosine distance (embeddings) | > 0.15 | Semantic drift |
| Token length shift | > 2 stddev | Output distribution change |
| Accuracy drop on benchmarks | 5-10% drop | Quality degradation |
| PSI (Population Stability Index) | > 0.25 | Input distribution shift |
| Code compilation success rate | Any sustained drop | Model code quality regression |
| Tool error rate | > 5% sustained | Tool use regression |

**Monitoring recommendation**: Daily automated batch evaluations against golden datasets. Immediate evaluation on model version updates.

---

## 11. Chinese Model Providers — International Access

### Z.AI (GLM family)

- International API: `api.z.ai`
- Registration: Standard email signup
- Payment: Standard credit card
- Rate limits: Concurrency-based, dynamically adjusted
- **Peak-hour pricing**: 3x during 14:00-18:00 UTC+8

### Moonshot AI (Kimi family)

- International API: `platform.moonshot.ai`
- Registration: May require Chinese mobile number
- Payment: AliPay/WeChat Pay (international cards difficult)
- **Easier access**: Via OpenRouter ($0.38/$1.72 per M tokens)
- Rate limit tiers: $1→1 concurrent, $10→50, $100→200, $1000→400

### Practical Recommendation

For initial integration, use **OpenRouter** for both GLM-5.1 and Kimi-K2.5. One API key, one endpoint, competitive pricing, automatic failover. Switch to direct APIs only when volume justifies dedicated keys.

---

## 12. Self-Improvement Theory (GVU Framework)

**The Generator-Verifier-Updater (GVU) Framework** (arXiv:2512.02731, Dec 2025) formalizes all self-improvement as: `theta → U(theta, V(G(theta)))`.

The **Variance Inequality** establishes when self-improvement is possible:
```
rho * ||g*||^2 > (eta*L/2) * (rho^2 * ||g*||^2 + sigma^2_G + sigma^2_V)
```

Key implications for Roko:
- **Oracle verifiers (sigma_V ≈ 0) always work**: Compile/test gates are oracles. This is why Roko's gate pipeline is more valuable than prompt optimization.
- **Self-critique amplifies errors**: When G=V=U, noise compounds. This formally explains why naive LLM self-correction fails.
- **"Strengthen the verifier, not the generator"**: Invest in richer gates (process rewards, property tests, ensemble checks) not better prompts.

**Paper**: [arXiv:2512.02731](https://arxiv.org/html/2512.02731v1)

---

## 13. Agent Runtime Architectures (SOTA 2025-2026)

### OpenHands Software Agent SDK

Four-package design: sdk (abstractions), tools (implementations), workspace (environments), agent_server (API). Key innovation: **event-sourced state** — all interactions are immutable events enabling deterministic replay and fault recovery.

**Paper**: [arXiv:2511.03690](https://arxiv.org/html/2511.03690v1)

### Coding Agent Taxonomy ("Inside the Scaffold")

First systematic source-code analysis of 14 coding agents. Key finding: **five agents independently converged on string-replacement edit semantics** (old_str/new_str). Three architectural layers: control architecture, tool/environment interface, resource management.

**Paper**: [arXiv:2604.03515](https://arxiv.org/html/2604.03515v1)

### Moatless Tools MCTS

Implements full Monte Carlo Tree Search (same algorithm as AlphaGo) for code modification. `AgenticLoop` (ReAct) and `SearchTree` (MCTS) are interchangeable via configuration. Avoids replay via shadow mode (file modifications tracked without writing to disk).

### SWE-agent ACI Design

Agent-Computer Interface: LM-centric commands and feedback formats. Extended to cybersecurity (EnIGMA), solving 13.5% of NYU CTF challenges (3x prior SOTA).

---

## 14. Prompt Evolution (GEPA)

**GEPA** (ICLR 2026 Oral, arXiv:2507.19457) — Genetic-Pareto prompt optimizer:
1. **Genetic prompt evolution**: Mutate and recombine prompt candidates
2. **Reflective diagnosis**: Read full execution traces to diagnose failures and propose targeted fixes
3. **Pareto-based selection**: Maintain frontier of diverse candidates

Results: **+13% over MIPROv2, +20% over GRPO, with 35x fewer rollouts**. Unlike RL that collapses traces into scalar rewards, GEPA uses LLMs to read full execution traces.

**Relevance**: Roko's `ExperimentStore` does simple A/B testing. GEPA's reflective evolution could replace it using the execution traces from `efficiency.jsonl`.

Also: **metaTextGrad** (arXiv:2505.18524) introduces meta-optimizers that improve other optimizers. Up to 11% absolute improvement over TextGrad.

---

## 15. Skill Libraries and Procedural Memory

### SAGE (arXiv:2512.17102, Dec 2025)

Skill-Augmented GRPO for Self-Evolution:
- Agents generate programmatic functions during execution
- Successfully executed functions persist to library
- Sequential rollout: skills from early tasks available to later ones
- **Results**: 26% fewer interaction steps, 59% fewer tokens

### Voyager (Wang et al., 2023)

Skill library accumulation for Minecraft agents. 3.3x improvement over baselines. Skills are (precondition, procedure, postcondition) tuples indexed by task type.

### Experiential Reflective Learning (ERL, arXiv:2603.24639)

Distills experiences into structured heuristics rather than raw trajectories. Single-attempt learning with cross-task transfer. +7.8% on GAIA2 over ReAct baseline.

---

## 16. Agent Behavioral Contracts

### ABC Framework (arXiv:2602.22302, Feb 2026)

Contracts as tuples C = (P, I, G, R): Preconditions, Invariants, Governance, Recovery.

**Stochastic Drift Bounds Theorem**: Contracts with recovery rate γ > α (drift rate) bound behavioral drift to D* = α/γ.

Drift score combines compliance (lagging) + JSD from reference action distribution (leading indicator).

Results across 1,980 sessions: Hard constraint compliance 88-100%, drift bounded to D* < 0.27, reliability index > 0.90.

**Runtime overhead**: <10ms per action check.

### AgentSpec (ICSE 2026, arXiv:2503.18666)

Lightweight DSL for runtime constraints. Rules as three-tuples: (event, predicate, enforcement). Prevents unsafe executions in 90%+ of cases.

---

## 17. Context Compression

### ACON (arXiv:2510.00615, Oct 2025)

Compresses both interaction histories and environment observations:
1. Compression guideline optimization via failure analysis
2. Distillation into smaller models

**26-54% peak token reduction**. Gradient-free, works with any API model.

### Anchored Iterative Summarization

Highest accuracy of all compression strategies (4.04 vs 3.43-3.74). Extends existing summaries rather than regenerating from scratch.

**Production recommendation**: Trigger compaction at 70% context utilization.

---

## 18. Process Reward Models

### AgentPRM (arXiv:2502.10325, Feb 2025)

Three-stage iterative process for step-level rewards:
1. Monte Carlo rollout → Q-value targets
2. PRM training via soft binary cross-entropy on intermediate steps
3. Policy optimization with KL-regularized distance

Key: agent environments have stochastic external effects, making beam search impractical. Uses asynchronous parallel rollouts.

### RLVR (Reinforcement Learning from Verifiable Rewards)

Exemplified by DeepSeek-R1: training against verifiable rewards produces emergent reasoning. For coding agents, test suite execution provides the verifiable reward signal — exactly what Roko's gate pipeline does.

---

## 19. Curriculum Learning

### SEC (arXiv:2505.14970, May 2025)

Self-Evolving Curriculum formulates curriculum selection as non-stationary MAB. Learning outcomes measured via gradient norm analog. **+13-33% on reasoning benchmarks**.

### Application to Roko

Task execution ordering in a plan could use SEC's approach: route easier tasks first (build model confidence), escalate to harder tasks. The CascadeRouter's RoutingContext.complexity field is the natural input.

---

## 20. Mori Reference Architecture (5-Layer Model)

From `/Users/will/dev/nunchi/roko/bardo-backup/tmp/mori-refactor/`:

| Layer | Name | Responsibility |
|---|---|---|
| L0 | Runtime | Process lifecycle, event streaming, supervision |
| L1 | Framework | Backend connections, roles, tools, model routing |
| L2 | Scaffold | Context engineering, prompt assembly, memory injection |
| L3 | Harness | Gates, scoring, pattern detection, interventions |
| L4 | Orchestration | Multi-agent coordination, scheduling, state machines |

Cross-cutting: Inference, Memory, Safety, Observability, Learning — injected via traits.

**Key principle**: Dependencies flow downward only. L4 may depend on L0-L3. L0 depends on nothing.

**Key anti-patterns to avoid** (from mori's current state):
- God objects (RunState: 289+ fields mixing all layers)
- Single-file orchestration (parallel.rs: 17,902 lines)
- Domain knowledge in framework code (prompts.rs: 5,784 lines with hardcoded crate names)
- Harness reaching into orchestration state (string matching on phase names)

---

## 21. Predictive Foraging (from Agent-Chain)

From `/Users/will/dev/nunchi/roko/bardo-backup/tmp/agent-chain/`:

Agents register predictions before execution → external system verifies outcome → residuals computed → arithmetic bias correction applied.

**Collective calibration**: Chain aggregates residuals from all agents for each (category, context) pair. New agents learn instantly from predecessors.

**Three-tier attention**: ACTIVE (full predictions), WATCHED (occasional), SCANNED (rare). Promotion triggered by prediction violations.

---

## 22. Knowledge Distillation Cascades

From agent-chain:

- **Layer 0**: Raw agent transcripts (thousands/week)
- **Layer 1**: Synthesized findings (distilled by synthesis agents)
- **Layer 2**: Distilled principles (extracted by distillation agents)
- **Layer 3**: Axiomatic truths (discovered by axiom agents)

Each layer **10x more applicable** than previous (1,000x by Layer 3). Maps to mori's Episode → Pattern → Playbook hierarchy.

---

## 23. Reproducibility and Determinism

### DFAH (arXiv:2601.15322, Jan 2026)

Three determinism levels:
1. **Action Determinism**: Identical tool sequences across runs
2. **Signature Determinism**: Identical sequences AND arguments
3. **Decision Determinism**: Identical final decisions

**Critical finding**: Determinism does NOT correlate with accuracy (r = -0.11).

### Practical Implication

Roko's episode logger already captures trajectories. Adding trajectory replay capability (event-sourced state) enables debugging without non-determinism.

---

## Complete Citation Index

| Paper/System | Year | Key Contribution | Section |
|---|---|---|---|
| GVU Framework | 2025 | Self-improvement theory, variance inequality | §12 |
| RouteLLM | 2025 | MF router, 85% cost reduction | §2 |
| MixLLM | 2025 | 4-component router, 97% quality at 24% cost | §2 |
| FrugalGPT | 2024 | LLM cascade, 98% cost reduction | §2 |
| AutoMix | 2024 | Self-verification + POMDP, 50% cost reduction | §2 |
| GEPA | 2026 | Genetic-Pareto prompt evolution, +13% over MIPROv2 | §14 |
| metaTextGrad | 2025 | Meta-optimizer for prompt optimizers | §14 |
| SAGE | 2025 | Skill library, 26% fewer steps | §15 |
| Voyager | 2023 | Skill accumulation, 3.3x improvement | §15 |
| ERL | 2026 | Experiential reflective learning | §15 |
| ABC | 2026 | Agent behavioral contracts, drift bounds | §16 |
| AgentSpec | 2026 | Runtime constraint DSL | §16 |
| ACON | 2025 | Context compression, 26-54% reduction | §17 |
| AgentPRM | 2025 | Process reward models for agents | §18 |
| DeepSWE | 2025 | RL-trained coding agent, 59% SWE-bench | §18 |
| SEC | 2025 | Self-evolving curriculum via MAB | §19 |
| OpenHands SDK | 2025 | Event-sourced agent runtime | §13 |
| Inside the Scaffold | 2026 | Coding agent taxonomy | §13 |
| DFAH | 2026 | Agent determinism measurement | §23 |
| SSR | 2025 | Self-play for SWE agents | §12 |
| DSPy MIPROv2 | 2024 | Bayesian prompt optimization | §14 |
| Thompson Sampling | various | Bandit algorithm superior to UCB | §5 |
| Multi-obj TS | 2025 | Pareto regret for joint optimization | §5 |

---

## 24. Existing Roko Infrastructure (Critical Discovery)

### ToolLoop Already Exists

**Location**: `crates/roko-agent/src/tool_loop/mod.rs`

The multi-turn tool loop is already implemented with:
- `ToolLoop` struct: translator + dispatcher + backend + max_iterations + context_token_limit
- `LlmBackend` trait: `async fn send_turn(messages, tools) -> BackendResponse`
- `run()` and `resume()` methods with full iteration logic
- Integration with `ToolDispatcher.dispatch_batch()` at line 250
- `Checkpoint` struct for crash recovery (defined but not persisted)
- Context pruning when approaching token limit

**What's missing**: Zero implementations of `LlmBackend` for any HTTP provider. The trait exists, the loop works, but nobody implements the trait. This is the ONE piece that connects the existing ToolLoop to HTTP providers.

### ToolDef is a Struct, Not an Enum

**Location**: `crates/roko-core/src/tool/def.rs` lines 260–278

`ToolDef` has: name, description, parameters (JSON Schema), category, permission, timeout_ms, concurrency, idempotent. It's a flat struct — the extended tool types (GLM web_search, retrieval, mcp) proposed in doc 04 task 2C.04 would need to either extend this struct or add a parallel enum.

### SystemPromptBuilder Has Cache Layers

**Location**: `crates/roko-compose/src/system_prompt_builder.rs`

Already has a 6-layer system with `cache_markers: bool` flag and `<!-- cache:system -->` / `<!-- cache:session -->` markers. Emission order already puts stable content first (Role → Conventions → Tools → Domain → Anti-patterns → Task). The cache layer alignment proposed in doc 13 task 2K.11–2K.15 needs to verify current behavior before adding changes.

### ProviderHealthTracker Already Exists

**Location**: `crates/roko-learn/src/provider_health.rs`

Already has a 3-state circuit breaker: `Healthy` → `Unhealthy { recovery_at }` → `Probing`. Methods: `record_success()`, `record_failure()`, `is_healthy()`, `filter_arms()`, `snapshot()`. Tracks consecutive failures, last success/failure, lifetime attempts/successes per provider. **Used internally but not exposed via HTTP API.** Docs 08 (2G.01–2G.03) should extend, not rebuild.

### MCP Bridge Functions Exist

**Location**: `crates/roko-agent/src/mcp/`

Already has:
- `mcp_to_tool_def()` — converts MCP ToolDef to roko ToolDef
- `DynamicToolRegistry` — composes static + MCP tools
- `dedup_tools()` — prevents duplicate names
- `McpClient`, `Transport`, `StdioTransport`

The bridge for HTTP backends (doc 14 task 2L.10) mainly needs to wire these existing functions into the HTTP path.

### Dual Config Architecture

**CLI** (`roko-cli/src/config.rs`): `Config` with `ConfigLayer` + `merge()` — layered loading with per-field provenance tracking (Global/Project/Default/Env). Fields: agent, tools, prompt, repos, gates, executor, budget, serve.

**Core** (`roko-core/src/config/schema.rs`): `RokoConfig` — loaded by daemon and core crate. Fields: project, prd, agent, gates, routing, budget, conductor, watcher, learning, tui, serve, scheduler, webhooks, subscriptions, server, deploy.

Both have `agent`, `gates`, `budget`, `serve` with **different field names/types**. New `[providers.*]` and `[models.*]` must go in BOTH (doc 18 task 2P.03). The daemon loads both independently with no sync.

### ToolDef Extension: Use ToolSource Field (doc 18)

Doc 18 task 2P.01 supersedes doc 04 task 2C.04. Rather than converting ToolDef to an enum (breaking), add a `ToolSource` enum field with `#[serde(default)]` (non-breaking). GLM's web_search/retrieval/mcp tool types use the new source variants. The `metadata: Option<Value>` bag handles future extensibility.

### LearningRuntime is THE Integration Hub

**Location**: `crates/roko-learn/src/runtime_feedback.rs`

`LearningRuntime::record_completed_run()` already coordinates ALL 10+ learning subsystems: episode logger, cost logs, provider health, playbook outcomes, skill library usage, pattern mining, cascade router, experiment store, task metrics, regression detection. Doc 17 identifies 6 missing feedback wires between these subsystems.

### Conductor Has 10 Watchers

**Location**: `crates/roko-conductor/src/watchers/*.rs`

10 pure-function anomaly detectors: compile_fail_repeat, context_window_pressure, cost_overrun, ghost_turn, iteration_loop, review_loop, spec_drift, stuck_pattern, test_failure_budget, time_overrun. Feed into `ConductorDecision`. Doc 17 task 2O.02 wires stuck detection into negative routing signal.

### roko-neuro (Knowledge Distillation) In Progress

Episode → Insights → Heuristics → Playbook tier progression actively being built. Doc 17 tasks 2O.11–2O.12 connect the distiller output to prompt assembly.

### BuildSystem Enum Has 6 Variants

**Location**: `crates/roko-gate/src/payload.rs`

Already has: Cargo, Npm, Go, Python, Forge, Make. Each provides `check_args()`, `test_args()`, `lint_args()`. Just needs auto-detection (doc 14 task 2L.16).

---

## 25. Rust Crates for Integration

### Token Counting

| Provider | Crate | Approach |
|---|---|---|
| OpenAI/GPT | `tiktoken-rs` v0.11.0 | Native Rust, o200k_base encoding |
| Claude | HTTP `/v1/messages/count_tokens` | Free API (no local tokenizer) |
| GLM/Kimi/others | `tokenizers` (HuggingFace) | Load tokenizer.json from model repo |
| Fallback | `tiktoken-rs` cl100k_base | ~15% accuracy for non-OpenAI |

### Rate Limiting

| Crate | Algorithm | Use Case |
|---|---|---|
| `governor` | GCRA (leaky bucket) | Per-provider RPM/TPM limiting |
| `tower-governor` | Tower middleware | HTTP service rate limiting |
| `reqwest-retry` | Exponential backoff | 429 retry handling |
| `backoff` | Exponential/constant backoff | Generic retry |

### HTTP

| Crate | Use |
|---|---|
| `reqwest` | HTTP client with connection pooling per-host |
| `reqwest-middleware` | Pluggable middleware (retry, logging, tracing) |

### MCP

| Crate | Use |
|---|---|
| `rust-mcp-schema` | Type-safe MCP protocol types |

---

## 26. Self-Hosted Deployment (vLLM/SGLang)

### vLLM Tool Call Parsers

| Parser | Models |
|---|---|
| `glm45` | GLM-4.5, GLM-4.6 |
| `glm47` | GLM-4.7, GLM-4.7-Flash |
| `kimi_k2` | Kimi-K2-Instruct |
| `hermes` | Hermes, Qwen2.5 |
| `deepseek_v3` | DeepSeek-V3, R1 |
| `llama3_json` | Llama 3.1, 3.2 |
| `openai` | GPT-OSS |

### Example Config

```toml
[deployment.glm47]
engine = "vllm"
model = "zai-org/GLM-4.7"
tensor_parallel = 4
tool_call_parser = "glm47"
reasoning_parser = "glm45"
enable_auto_tool_choice = true
```
