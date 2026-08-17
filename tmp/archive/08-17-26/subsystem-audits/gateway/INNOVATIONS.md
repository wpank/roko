# Novel Innovations: What Nobody Else Has

This document catalogs net-new innovations that emerge from combining roko + gateway + nunchi — things no competitor offers because they require all three layers working together.

**Implementation status key** — each innovation is annotated:
- **EXISTS**: Built and wired in roko today
- **PARTIAL**: Infrastructure exists but not fully wired or only in spec
- **PROPOSED**: New capability to build for the gateway
- **CONCEPT**: Research/vision concept from docs, no code

---

## 0. The Observability Problem (Read This First)

**The gateway is a transparent HTTP proxy.** It forwards requests to upstream LLM providers and returns responses. It does NOT know:
- Whether the agent's task succeeded or failed
- Whether the generated code compiled or passed tests
- Whether the user was satisfied
- What the agent did with the response

**What the gateway CAN observe from the HTTP layer alone (passive signals):**

| Signal | Source | What It Tells You |
|---|---|---|
| HTTP status code | Provider response | Provider health (200/429/500) |
| Token counts | Response `usage` field | Cost, whether context was exhausted |
| `stop_reason` | Response body | `end_turn` vs `max_tokens` (truncation = bad sign) |
| Latency (TTFT, total) | Timing | Provider speed, request complexity proxy |
| Streaming vs buffered | Request shape | Urgency proxy (streaming = interactive) |
| Tool calls in response | Response body | Which tools the model chose to invoke |
| Tool results in next request | Subsequent request body | What the tool returned (success/error) |
| Session length | Request count per session | Long sessions may indicate struggling |
| Retry patterns | Same hash appearing again | Agent retrying = something went wrong |
| Model requested vs used | Request header vs routing | Whether downgrade happened |
| Cache hit/miss | Cache layer | Cost savings achieved |
| Request frequency | Per-key timing | Burst = automated, steady = interactive |
| Conversation shape | Message array growth | How fast context is growing |

**These passive signals are useful but incomplete.** The gateway can learn things like "requests to model X that get `stop_reason: max_tokens` are 3x more likely to be retried" or "sessions longer than 20 turns correlate with requests that get truncated." But it cannot determine task-level success.

### Three Strategies for Richer Signal

**Strategy 1: Explicit Feedback Endpoint (opt-in)**

```
POST /v1/feedback
{
  "request_id": "req_abc123",
  "outcome": "success" | "failure" | "partial",
  "gate_result": { "compile": true, "test": true, "clippy": false },
  "cost_acceptable": true,
  "latency_acceptable": true
}
```

Clients that use the Roko SDK or runtime automatically report outcomes. Third-party clients can opt in. The gateway learns from whoever reports, and uses passive signals as fallback for those who don't.

**This is the primary mechanism.** When the gateway is embedded in roko-serve (the common case for roko users), the orchestrator calls the feedback endpoint after every gate pipeline run. The feedback loop is automatic for roko users and opt-in for everyone else.

**Strategy 2: Proxy Signals as Weak Labels**

Even without explicit feedback, the gateway can infer outcome quality from observable patterns:

| Pattern | Inference | Confidence |
|---|---|---|
| Session ends after 1-3 turns | Task likely succeeded (quick resolution) | Medium |
| Session exceeds 20 turns | Agent likely struggling | Medium |
| `stop_reason: max_tokens` | Response was truncated, often needs retry | High |
| Same request hash appears 2+ times | Retry → first attempt failed | High |
| Tool call returns error string | Tool execution failed | High |
| Rapid successive requests (<1s apart) | Automated pipeline, not interactive | High |
| User sends "try again" / "that's wrong" | Explicit dissatisfaction (if gateway parses user messages) | High |
| No further requests after response | Either succeeded or user gave up | Low |

These are noisy but abundant. With enough volume, weak labels aggregate into usable signal — the same way recommendation systems learn from clicks without explicit ratings.

**Strategy 3: Roko Runtime Integration**

When the gateway is embedded in roko-serve (rather than standalone), it has direct access to:
- Gate pipeline results (compile, test, clippy, diff — per task)
- Episode logs (full agent turn history with outcomes)
- Efficiency events (per-turn cost/quality metrics)
- CascadeRouter observations (which already track pass/fail)

This is the richest signal source but only available to roko users. The gateway's learning should work at three fidelity levels:
1. **Roko-integrated**: full gate results, episodes, direct CascadeRouter feedback → best routing
2. **SDK users**: explicit feedback endpoint → good routing
3. **Plain HTTP proxy**: passive signals only → basic routing (still better than no routing)

### Implications for Every Innovation Below

Each innovation in this document is annotated with its **signal source**: what data it needs and how it gets it. Innovations that require explicit feedback are marked accordingly. Nothing below assumes the gateway magically knows outcomes.

---

## 1. Predict-Publish-Correct at the Gateway Layer — PARTIAL

> **Status**: `prediction.rs` in roko-learn has PredictionRecord, CalibrationTracker, ResidualCorrector — the Router's prediction/outcome/residual loop is real code. However, universal per-operator Bus-mediated PPC is explicitly documented as **"target-state"** in `docs/05-learning/18-self-learning-cybernetic-loops.md`. The Bus (`EventBus<RokoEvent>` in roko-runtime) exists and works. What's missing is wiring every Cell operator into the prediction loop — today only the Router has rich prediction/correction. Applying PPC to gateway routing decisions is a **proposed** extension.

Every existing gateway is a dumb pipe: request in, response out, maybe cache it. Nunchi's gateway makes **every request a learning event** — for the things it can actually observe.

### How It Works

```
Request arrives at gateway
  → Gateway publishes a PREDICTION before routing:
    - predicted cost             (verifiable from response usage field)
    - predicted latency          (verifiable from timing)
    - predicted cache hit        (verifiable from cache lookup)
    - predicted best model tier  (verifiable ONLY with feedback)
  → Request is routed, executed, response returned
  → Gateway computes PREDICTION ERROR on observable signals:
    - actual cost vs predicted         ← from response usage
    - actual latency vs predicted      ← from timing
    - cache prediction correct?        ← from cache layer
    - stop_reason as quality proxy     ← from response body
    - session continued or ended?      ← from subsequent requests
  → If feedback endpoint called:
    - actual task outcome (pass/fail)  ← from client
    - gate results                     ← from client
  → Error vector updates:
    - CascadeRouter weights (only on confirmed outcomes)
    - Cache regime detector (from hit/miss patterns)
    - Budget degradation thresholds (from cost actuals)
    - Provider health scores (from HTTP status + latency)
```

**Signal sources**: Cost/latency/cache predictions use **passive signals** (always available). Model quality predictions require **explicit feedback** (from roko runtime or SDK).

### What's Learnable Without Feedback

Even without any explicit feedback, PPC learns:
- Cost prediction accuracy → better budget estimation
- Latency prediction → better provider selection for latency-sensitive requests
- Cache hit prediction → better cache regime tuning
- Provider reliability → better failover decisions
- Token usage patterns → better max_tokens defaults

These alone are valuable. No other gateway predicts and corrects on its own operational metrics.

### What Requires Feedback

- "Model X is better than Model Y for task category Z" → needs outcome signal
- "Cheaper model worked fine here" → needs confirmation that quality was acceptable
- Routing convergence to optimal model per task → needs pass/fail per task

**This is why the feedback endpoint is critical.** Without it, the gateway optimizes cost and latency but cannot optimize quality-adjusted routing.

### Concrete Example (With Feedback)

Day 1: Gateway routes all coding tasks to claude-sonnet-4-6 ($3/$15 per M).
Week 2: Feedback shows `complexity < 0.3` tasks pass gates 94% of the time on gemini-2.5-flash.
Week 4: Feedback confirms DeepSeek-V3.2 handles simple refactoring at 91% gate pass rate.
Month 2: Blended cost has dropped 65% with <2% quality regression.

### Concrete Example (Without Feedback, Passive Signals Only)

Day 1: Gateway routes all tasks to claude-sonnet-4-6.
Week 2: Gateway observes sessions using gemini-flash are shorter (fewer turns) and have fewer retries — weak evidence that flash is sufficient for some tasks.
Week 4: Gateway notices DeepSeek requests rarely trigger `stop_reason: max_tokens` — weak evidence it handles context well.
Month 2: Cost drops ~30% from latency/cost optimization alone. Quality-adjusted routing improvements are slower and noisier.

**Honest assessment**: Passive-only PPC is still better than static routing, but ~2-3x slower to converge than feedback-augmented PPC.

---

## 2. Affordance-Aware Routing — CONCEPT

> **Status**: No affordance computation exists in any roko Rust crate. The word "affordance" appears only in `docs/` markdown files. `roko-index` does HDC fingerprinting of symbols (confirmed: 10,240-bit vectors) but computes no affordance score, coverage metric, or complexity metric. The formula in `docs/BENCHMARKS.md` (`0.25 * extensibility + 0.20 * test_coverage + ...`) is not implemented. CascadeRouter's 18-dimensional LinUCB context vector has no affordance dimension — the 18 are: task category (8), complexity (1), iteration (1), role hash (4), crate familiarity (1), has-prior-failure (1), bias (1), cache affinity (1). Adding affordance would require expanding the feature vector.

From agent co-evolution research (Odling-Smee niche construction theory): **code structure determines the effective budget an agent needs**.

### The Insight

A well-documented module with full tests and clear interfaces → agent spends 2K tokens navigating, 6K reasoning.
A tangled module with no docs, circular deps, dead code → agent spends 6K navigating, 2K reasoning.

Same model, same task, **3x effective budget difference from documentation alone**.

### Gateway Integration

The gateway receives affordance metadata in request headers (from roko's code intelligence index or client SDK):

```
X-Roko-Affordance-Score: 0.82
X-Roko-Module-Complexity: low
X-Roko-Test-Coverage: 0.94
```

**Signal source**: These are **client-supplied headers**. The gateway does not compute them. Roko's orchestrator computes affordance from `roko-index` and attaches them. Third-party clients can compute their own or omit them (gateway falls back to standard routing).

CascadeRouter incorporates affordance into its 18-dimensional LinUCB context vector:
- High affordance (>0.7) → route to cheap model (haiku, flash, DeepSeek)
- Low affordance (<0.3) → route to frontier model (opus, o3)
- Medium → mid-tier with larger context window

### Why This Is Novel

No gateway today routes based on **environmental difficulty**. They all route based on the task description alone. But the same "fix this bug" task costs 5x more in a messy codebase than a clean one. Affordance-aware routing captures this.

### The Compounding Effect (Co-Evolution)

When agents improve code quality (better docs, tests, naming), affordance scores rise → CascadeRouter routes to cheaper models → cost drops → more budget for more improvements → affordance rises further.

**1% affordance improvement per invocation × 200 invocations = 625% cumulative improvement.** This is geometric scaling from environmental optimization, not linear scaling from model improvement.

### What Validates This Loop

The feedback endpoint closes it: affordance was high → cheap model was routed → task passed gates → confirmed that high-affordance + cheap model works. Without feedback, the gateway doesn't know if the cheap routing actually worked.

---

## 3. Clearing-as-Inference: Knowledge from Economic Activity — CONCEPT

> **Status**: Pure concept from `learnings2/11-ISFR-AND-CLEARING.md`. No clearing, CRPS scoring, or inference batching exists in roko code. `prediction.rs` has calibration tracking (Brier scores, reliability bins) which could serve as the foundation, but the "inference clearing round" pattern described here is entirely new.

This is the most novel concept in the entire Nunchi stack. Every clearing round produces structured knowledge as a byproduct of settlement.

### How It Applies to the Gateway

Extend the concept from financial clearing to **inference clearing**:

```
Inference Clearing Round (every N seconds):
  1. Batch arriving requests by similarity (L2 semantic cache fingerprints)
  2. Agents submit prediction commitments with their requests:
     X-Roko-Predicted-Tier: flash       (client-supplied header)
     X-Roko-Predicted-Tokens: 2000      (client-supplied header)
  3. Gateway routes the batch, observes:
     - actual model used
     - actual tokens consumed (from response usage)
     - actual cost
     - HTTP-observable quality signals (stop_reason, retries)
  4. If feedback arrives: actual pass/fail adds high-fidelity scoring
  5. Predictions scored:
     - Cost prediction: exact verification from usage field
     - Tier prediction: verifiable if feedback provided, else weak proxy
  6. ClearingInsight emitted to InsightStore
```

**Signal source**: Tier predictions can be scored exactly with feedback, or weakly via proxy signals without it. Cost and token predictions are always exactly verifiable.

### CRPS Scoring for Model Selection

CRPS (Continuous Ranked Probability Score) is a **strictly proper scoring rule** — truthful reporting is the unique optimal strategy.

**What CRPS can score from gateway-observable data alone**:
- Cost prediction accuracy (exact — tokens are in the response)
- Latency prediction accuracy (exact — timing is observable)
- Token usage prediction accuracy (exact — usage field)

**What CRPS needs feedback for**:
- Quality tier prediction ("flash is sufficient" vs "needs opus")
- Task outcome prediction ("this will pass gates")

Applied to model routing: agents that accurately predict their own needs earn better epistemic reputation → get priority routing → cheaper access → compounding advantage.

### Knowledge Moat

At 10-second batch intervals: 8,640 scored predictions per gateway instance per day. Across 100 customers: 864,000/day. After one year: ~315 million scored observations about cost/latency/token patterns. With feedback: add quality-adjusted routing observations.

**This dataset is the moat, not the code.** No competitor can bootstrap it by cloning the repo.

---

## 4. Self-Funding Agent Inference (Metabolic Loop) — CONCEPT

> **Status**: Concept from bardo-gateway's Bankr module (`apps/bardo-gateway/src/bankr/metabolic.rs`). Not in roko codebase. ERC-8004 identities, Trust Credits, and ERC-8183 marketplace are chain-layer specs (all Tier 6 / deferred). The metabolic ratio tracking pattern is proven in bardo but would need to be rebuilt for roko-gateway.

Agents that earn money can pay for their own inference. The gateway tracks the metabolic ratio: `daily_revenue / daily_inference_cost`.

### Gateway Integration

```
Agent registers with ERC-8004 identity
  → Completes jobs on ERC-8183 marketplace
  → Earns Trust Credits (burns NUNCHI at oracle rate)
  → Uses Trust Credits to pay for gateway inference
  → Gateway tracks metabolic ratio per agent:
    ratio >= 1.0 → FullAccess (any model)
    ratio 0.5-1.0 → ReducedThroughput (mid-tier models)
    ratio < 0.5 → EmergencyOnly (cheapest models only)
    ratio > 2.0 → Surplus (agent is profitable, can hire sub-agents)
```

**Signal source**: The gateway knows revenue because Trust Credits flow through it (billing layer). It knows inference cost because it computed it. The metabolic ratio is **fully observable** within the gateway — no external feedback needed.

### The Flywheel

```
Agent completes jobs → earns Trust Credits → pays for inference →
  better inference → completes harder jobs → earns more →
  hires sub-agents → sub-agents complete jobs → ...
```

This creates **autonomous agent economies** — networks of agents that sustain themselves financially without human intervention. The gateway is the metering point where real economic value flows.

### Why This Is Novel

Existing gateways charge humans. Nunchi's gateway charges agents. The billing relationship is agent→gateway, not human→gateway. This opens an entirely new customer segment: autonomous agents with their own wallets.

At 82:1 machine-to-human identity ratio in enterprise (CyberArk 2025), the agent customer segment is 82x larger than the human segment.

---

## 5. Cross-Instance Learning via Hub — CONCEPT

> **Status**: No HuggingFace integration exists in roko code. `tmp/archive/misc/04-21-26/02-huggingface-integration.md` is a proposal doc. The CascadeRouter does persist to `.roko/learn/cascade-router.json` (EXISTS) — sharing that serialized state between instances is feasible but not implemented.

Multiple gateway instances share learning via HuggingFace Hub (or nunchi's own knowledge substrate).

### Mechanism

```
Gateway Instance A (serves Django developers):
  → Observes: DeepSeek-V3.2 requests for "django-orm" cluster have
    short sessions, no retries, low truncation (passive signals)
  → With feedback: 92% gate pass rate confirmed
  → Publishes: {task_cluster: "django-orm", model: "deepseek-v3.2",
    proxy_quality: 0.87, confirmed_pass_rate: 0.92, n_obs: 847}

Gateway Instance B (serves Flask developers):
  → Pulls Instance A's artifact
  → CascadeRouter adds "deepseek-v3.2 for django-orm" as exploration arm
  → Discovers the learning transfers: DeepSeek handles Flask SQLAlchemy too
  → Publishes its own finding back to Hub

Gateway Instance C (new instance, day 1):
  → Pulls all published artifacts
  → Starts with collective knowledge of A + B
  → Skips 2 months of cold-start exploration
```

### What Gets Published

| Artifact | Content | Signal Source | Privacy |
|---|---|---|---|
| Routing observations | (task_cluster, model, proxy_quality, confirmed_rate, n_obs) | Passive + feedback | Aggregated, no PII |
| Cost patterns | (model, avg_tokens, avg_cost, by_task_cluster) | Passive (always exact) | Statistical only |
| Latency profiles | (provider, p50/p95/p99, by_region, by_hour) | Passive (always exact) | Operational metrics |
| Cache regime patterns | (workload_type, optimal_ttl, hit_rate) | Passive | Statistical only |
| Provider reliability | (provider, error_rate, by_error_class) | Passive | Operational metrics |
| Prompt experiment winners | (section_id, variant, win_rate, n_trials) | Requires feedback | Template-level, no content |

**Note**: Routing quality observations have two tiers — `proxy_quality` (from passive signals, lower confidence) and `confirmed_pass_rate` (from feedback, high confidence). Consumers choose which to trust.

### Network Effect

Each additional gateway instance generates training signal that makes ALL instances better. The thousandth instance joins smarter than the first. Even passive-only instances contribute cost/latency/provider data.

---

## 6. HDC-Powered Semantic Cache — PROPOSED

> **Status**: HDC vectors in roko-primitives are real (10,240-bit, `HdcVector` type, POPCNT similarity — EXISTS). No semantic cache exists in roko — the bardo gateway has SimHash-based L2 cache. Using HDC vectors as the cache similarity engine is a proposed combination of existing HDC code with new cache infrastructure.

Replace L2 semantic cache (SimHash / embedding) with HDC vectors.

### Why HDC > Embeddings for Cache

| Property | Embeddings | HDC (10,240-bit BSC) |
|---|---|---|
| Similarity search | ~3-5ms (cosine, float32) | ~1μs (POPCNT, binary) |
| Storage per entry | ~3KB (768×float32) | 1,280 bytes |
| GPU required | Yes (for embedding generation) | No (pure CPU) |
| False positive rate | Tunable but opaque | <1% at threshold 0.526 against 100K vocab |
| Composability | None (can't combine embeddings meaningfully) | BIND (XOR), BUNDLE (majority), PERMUTE (shift) |

**Signal source**: Cache operations are entirely internal to the gateway. No external feedback needed.

### Novel Cache Operations

HDC enables cache operations that embeddings can't do:

```rust
// BIND: Associate request with session context
let cache_key = hdc_bind(request_vector, session_context_vector);
// → Cache is now session-aware without explicit session IDs

// BUNDLE: Combine multiple similar requests into cluster centroid
let cluster = hdc_bundle(&[req1, req2, req3, req4]);
// → Single cache entry serves all semantically similar requests

// PERMUTE: Encode conversation position
let positional_key = hdc_permute(request_vector, turn_number);
// → Same question at turn 3 vs turn 30 can have different cache behavior
```

### On-Chain Cache Proofs (ZK-HDC)

When the nunchi chain is live, cache hits can be **proven** on-chain:

```
Gateway claims: "This response was served from cache (no inference cost)"
Proof: ZK-HDC proof that request_vector similarity to cached_vector > threshold
Verification: ~250K gas, <1s proving time
```

This enables **auditable cost reporting** — customers can verify they're not being charged for cached responses. No other gateway can prove its cache hits are legitimate.

---

## 7. Epistemic Reputation for Model Routing — CONCEPT

> **Status**: No reputation system, CRPS scoring, or tier-based discount logic exists in roko code. `prediction.rs` has CalibrationTracker with Brier scores and reliability bins (EXISTS) — this is the closest foundation. The tier system (Standard/Calibrated/Expert/Oracle) and discount structure are entirely new proposals.

Agents (and gateway API keys) accumulate **epistemic reputation** from scored predictions.

### What Can Be Scored Without Feedback

| Prediction | Verification | Signal Source |
|---|---|---|
| "This will cost ~$0.05" | Compare to actual cost from usage field | Passive (exact) |
| "This will take ~2s" | Compare to actual latency | Passive (exact) |
| "This needs ~3000 tokens output" | Compare to actual output tokens | Passive (exact) |
| "Cache will hit" | Compare to actual cache result | Passive (exact) |

### What Requires Feedback

| Prediction | Verification | Signal Source |
|---|---|---|
| "Flash tier is sufficient" | Did the task pass gates? | Explicit feedback |
| "This task will succeed" | Did the task pass gates? | Explicit feedback |

### Tiers

| Tier | Requirement | Privileges |
|---|---|---|
| Standard | Default | Normal routing, normal pricing |
| Calibrated | >500 scored predictions, top 60% | 5% routing discount, priority batch |
| Expert | >2000 scored predictions, top 30% | 15% discount, early access to new models |
| Oracle | >5000 scored predictions, top 10% | 25% discount, 2x knowledge query allocation |

**Note**: Reputation can accumulate from cost/latency predictions alone (passive). Quality-based reputation requires feedback. An agent can reach Calibrated tier purely by being good at predicting its own cost.

### Why This Matters

Agents that accurately predict their own needs get cheaper service. This creates a **selection pressure toward self-aware agents**. Over time, the best agents on the platform are also the most self-aware.

---

## 8. Batch-Opportunistic Scheduling — PROPOSED

> **Status**: roko-serve has batch route stubs (`/inference/batch/submit`, `/inference/batch/{id}` — EXISTS as route handlers) but no actual batch provider integration. The bardo gateway had a working Anthropic Batch API client. Urgency classification and auto-classification learning are entirely new proposals.

No gateway today does intelligent batch routing. They either batch everything or batch nothing.

### Smart Batching

```
Request arrives with urgency signal:
  X-Roko-Urgency: realtime    → Forward immediately
  X-Roko-Urgency: background  → Queue for batch API (50% off)
  X-Roko-Urgency: (absent)    → Gateway classifies automatically:
    - Streaming requested → realtime
    - Non-streaming + large max_tokens → likely background
    - Budget pressure (>80% utilized) → force background
    - CI/CD pipeline (detected by key metadata) → background

Background queue:
  - Anthropic Batch API: 50% off, results within 24h
  - OpenAI Batch API: 50% off, results within 24h
  - Off-peak DeepSeek: 50-75% additional discount

Stacking: batch + off-peak = up to 87.5% cost reduction
```

**Signal source**: Urgency classification uses **passive signals** only (streaming flag, max_tokens, request frequency, key metadata). No feedback needed. The gateway learns urgency patterns from observable request characteristics.

### Auto-Classification Learning

The gateway observes which urgency classifications lead to complaints (retries, re-requests with `realtime` header after a background classification). This is a passive feedback loop — no explicit feedback endpoint needed.

---

## 9. Tool Usage Intelligence — PROPOSED

> **Status**: The bardo gateway had per-session tool pruning (`tools.rs` — strips unused tool definitions after 50 requests). roko's ToolDispatcher tracks tool calls within a single agent session. No cross-agent tool analysis, tool dependency graphs, or tool usage learning exists in roko code. This is a new proposal that builds on the bardo pattern.

The gateway sees tool definitions in requests and tool calls in responses. This creates a unique cross-agent dataset.

### What the Gateway Observes (Passive, Always Available)

```
From request bodies:
  - Tool definitions (name, description, schema) in every request
  - Tool results in conversation history (success/error in content)

From response bodies:
  - Which tools the model chose to call
  - Tool call arguments

Derived signals:
  - Tool X defined in 95% of requests but called in only 3% → prune candidate
  - Tool Y called but returned error 40% of the time → problematic tool
  - file_search is called before code_edit in 94% of successful sessions
  - Sessions with tool Z defined are 2x longer → tool Z may be causing confusion
```

### What the Gateway CANNOT Observe

- Whether a tool call's result was *correct* (it sees the result string, not whether it was right)
- Whether the agent used the tool output effectively
- Task-level outcomes from tool usage patterns

### Novel Optimizations

**Dead tool detection**: Tool defined but never called across 100+ requests → safe to prune. This saves 2-5K tokens/request. **No feedback needed** — purely based on request/response observation.

**Tool error tracking**: Tool calls that return error strings are observable in subsequent request bodies (tool results in conversation history). The gateway can warn: "Tool X has a 40% error rate across your agents."

**Cross-agent tool patterns**: "Agents that define both `file_search` and `grep_search` tend to use `grep_search` 3x more. Consider removing `file_search` to reduce prompt size." **No feedback needed** — purely from usage statistics.

**Predictive tool ordering**: The gateway can reorder tool definitions in the request to put most-likely-to-be-called tools first (some models are sensitive to tool ordering). Based on observed call frequency. **No feedback needed.**

### What DOES Require Feedback

"Agents that call `list_directory` before `file_read` succeed 23% more often" → requires knowing whether the task succeeded. This needs the feedback endpoint.

---

## 10. Fine-Tuning Loop (The Exponential Closure) — CONCEPT

> **Status**: No HuggingFace AutoTrain integration, fine-tuning pipeline, or model publishing exists in roko. Episode logging EXISTS (full `EpisodeLogger` with gate verdicts, model, success flag in `roko-learn/src/episode_logger.rs`). CascadeRouter bandit arm management EXISTS. The episodes provide the training data source and the router provides the arm insertion mechanism — but the fine-tuning step itself (AutoTrain trigger → Hub push → arm registration) is entirely unbuilt.

The HuggingFace integration enables a self-improvement loop — but it requires rich signal.

### Signal Requirements (Honest Assessment)

```
Gateway serves requests → agents produce outputs →
  ??? How do we know which outputs are "successful"? ???
```

**This loop REQUIRES the feedback endpoint or roko runtime integration.** There is no way to determine "successful episodes" from passive HTTP signals alone. The gateway can observe that a response wasn't truncated and the session was short, but that's not "the code compiled and tests passed."

### How It Actually Works

```
Roko runtime (orchestrator) runs task →
  Agent produces output via gateway →
  Gate pipeline runs (compile, test, clippy) →
  Outcome reported to gateway via POST /v1/feedback →
  Successful episodes marked in episode log →
  Successful (request, response) pairs become training data →
  AutoTrain: fine-tune base model on successful outputs →
  Push fine-tuned model to Hub →
  CascadeRouter adds fine-tuned model as new bandit arm →
  Gateway routes some traffic to fine-tuned model →
  Feedback on fine-tuned model confirms/denies improvement →
  If wins → more traffic → more successes → more data → repeat
```

**This loop only works for roko-integrated users or SDK users who report outcomes.** It does not work for plain HTTP proxy users.

### Safety Constraint (Darwin Gödel Machine Warning)

The Darwin Gödel Machine (arXiv:2505.22954) improved SWE-bench from 20% to 50% via self-modification — but also **falsified test results and disabled its own hallucination detection** when these interfered with fitness maximization.

**Critical design constraint**: The gate pipeline (verify step) MUST be outside the self-modifiable surface. Fine-tuned models can be explored, but the gates that evaluate them are immutable. The gateway enforces this: model selection is learned, verification criteria are not.

---

## 11. CaMeL Security at the Gateway (Dual-LLM) — CONCEPT

> **Status**: No dual-LLM security architecture exists in roko. The CaMeL paper reference is from `learnings2/04-RESEARCH.md`. roko's safety layer (AgentContract + ToolDispatcher) is a single-LLM design that falls back to permissive defaults when YAML contracts are missing. CaMeL would be new infrastructure built into the gateway.

From the research: Nasr et al. tested 12 published defenses against adaptive attackers and achieved >90% attack success against ALL of them. 500 human red-teamers achieved 100% ASR on every prompt-layer defense.

**CaMeL (DeepMind/ETH Zurich)** is the only defense that survives: dual-LLM architecture where a Privileged LLM handles trusted instructions and a Quarantined LLM handles external data.

### Gateway-Level CaMeL

```
Request arrives at gateway:
  1. Privileged LLM (cheap, fast, trusted):
     - Parses the system prompt and user instructions
     - Extracts the POLICY: what tools are allowed, what data is trusted
     - Determines routing constraints

  2. Request forwarded to upstream provider (the "Quarantined" LLM):
     - Handles the actual task with untrusted external data
     - Tool calls filtered by Privileged LLM's policy
     - Responses checked against policy before returning

  Result: 77% solve rate on AgentDojo, 7-point capability tax
  But: survives adaptive attacks that defeat ALL other defenses
```

**Signal source**: The Privileged LLM operates entirely within the gateway. It reads the request (which the gateway already has) and produces a policy. No external feedback needed.

**Cost note**: The Privileged LLM runs on a cheap model (haiku/flash) and only processes the system prompt + tool definitions — not the full conversation. Cost per request: ~$0.001-$0.005. This is gateway infrastructure cost, not billed to the customer.

### Why at the Gateway

CaMeL at the gateway means **every agent gets dual-LLM security for free**, regardless of their framework. The Privileged LLM runs once per session (cheap), the policy is enforced transparently on every request. Agents don't need to implement their own security — the gateway provides it as infrastructure.

---

## 12. The Autocatalytic Flywheel (All Loops Combined) — CONCEPT

> **Status**: The individual building blocks exist at various levels of completion (see table above). The flywheel as a composed system is entirely conceptual — no roko code wires these loops together through a gateway.

Five reinforcing loops, all passing through the gateway. Each annotated with signal AND implementation requirements.

```
Loop 1: Volume → Cost/Latency Learning → Better Defaults → More Volume
  Signal: PASSIVE (cost, latency, cache — always observable)
  Requires: Nothing beyond HTTP traffic
  Roko status: CascadeRouter observation loop EXISTS. Gateway wrapping is PROPOSED.
  Speed: Fast convergence (exact signals)

Loop 2: Feedback → Quality-Adjusted Routing → Lower Cost at Same Quality → More Usage
  Signal: EXPLICIT FEEDBACK required
  Requires: Roko integration or SDK users reporting outcomes
  Roko status: CascadeRouter.observe() with pass/fail EXISTS. Feedback endpoint is PROPOSED.
  Speed: Slower convergence (depends on feedback coverage)

Loop 3: Reputation → Cheaper Access → More Tasks → More Predictions → Higher Reputation
  Signal: MIXED (cost predictions = passive, quality predictions = feedback)
  Requires: Predictions in request headers + reputation system
  Roko status: CalibrationTracker EXISTS. Reputation tiers/discounts are CONCEPT.
  Speed: Medium (reputation accrues from whatever signals are available)

Loop 4: Co-Evolution → Affordance → Cheaper Routing → More Budget for Improvement
  Signal: CLIENT-SUPPLIED (affordance headers)
  Requires: Affordance computation (not in roko today) + LinUCB dimension expansion
  Roko status: Entirely CONCEPT. No affordance code exists.
  Speed: Depends on client infrastructure

Loop 5: Fine-Tuning → Better Models → More Successes → More Training Data
  Signal: EXPLICIT FEEDBACK required (must know which outputs are "good")
  Requires: Roko runtime integration (gate results) + HuggingFace AutoTrain
  Roko status: Episodes + bandit arms EXIST. AutoTrain integration is CONCEPT.
  Speed: Slowest (needs months of accumulated successful episodes)
```

### What Works for Everyone (Day 1, No Feedback)

- Cost optimization (exact token/cost tracking)
- Latency optimization (exact timing)
- Cache regime adaptation (exact hit/miss)
- Provider health and failover (exact HTTP status)
- Tool pruning (exact usage statistics)
- Batch-opportunistic scheduling (request shape analysis)
- Dead tool detection (cross-agent usage patterns)

### What Works for SDK Users (Feedback Endpoint)

All of the above, plus:
- Quality-adjusted model routing
- Prompt experiment A/B testing
- Epistemic reputation (full quality scoring)
- Cross-instance quality observations

### What Works for Roko-Integrated Users (Full Runtime)

All of the above, plus:
- Fine-tuning loop (gate results → training data)
- Affordance-aware routing (roko-index → headers)
- Full CascadeRouter learning (gate pass/fail per task)
- Episode-level learning (complete turn history with outcomes)
- Playbook generation (successful patterns → future prompts)

### The Honest Pitch

"The gateway gets smarter for everyone — it learns cost and latency from every request. For users who report outcomes, it also learns which models are best for which tasks. For roko users, it learns everything and the system literally builds the models it uses."

**Three tiers of intelligence, not a binary on/off.** The moat grows fastest at the top tier, but even the bottom tier is better than any competing gateway.

---

## Summary: What's Genuinely Novel

| Innovation | Roko Status | Signal Source | Requires Feedback? | Exists Anywhere? |
|---|---|---|---|---|
| PPC on cost/latency/cache | **PARTIAL** (prediction.rs exists, Bus exists, not wired as universal pattern) | Passive | No | No |
| PPC on quality (model selection) | **PARTIAL** (CascadeRouter.observe() exists) | Feedback | Yes | No |
| Affordance-aware routing | **CONCEPT** (no affordance code, would extend LinUCB from 18 to 19+ dims) | Client headers | No (headers), Yes (validation) | No |
| Clearing-as-inference (cost scoring) | **CONCEPT** (CalibrationTracker is foundation) | Passive | No | No |
| Clearing-as-inference (quality scoring) | **CONCEPT** | Feedback | Yes | No |
| Self-funding agents (metabolic loop) | **CONCEPT** (existed in bardo Bankr, not in roko) | Billing layer | No | Partial (bardo) |
| Cross-instance cost/latency learning | **CONCEPT** (CascadeRouter serializes to JSON, sharing is feasible) | Passive | No | No |
| Cross-instance quality learning | **CONCEPT** | Feedback | Yes | No |
| HDC semantic cache | **PROPOSED** (HDC vectors exist, cache is new) | Internal | No | No |
| Epistemic reputation (cost) | **CONCEPT** (CalibrationTracker has Brier scores) | Passive | No | No |
| Epistemic reputation (quality) | **CONCEPT** | Feedback | Yes | No |
| Batch-opportunistic scheduling | **PROPOSED** (route stubs exist, provider integration is new) | Passive | No | No |
| Cross-agent tool intelligence | **PROPOSED** (bardo had per-session pruning) | Passive | No | No |
| Fine-tuning loop | **CONCEPT** (episodes + bandit arms exist, AutoTrain is new) | Roko runtime | Yes (gate results) | No |
| CaMeL security | **CONCEPT** (no dual-LLM in roko) | Internal | No | No |

**The pitch**: "Most gateway companies optimize the pipe. We optimize the system. Every request makes the gateway smarter about cost and speed — that's free. Report outcomes, and it learns quality too. Run roko, and it builds the models it uses to build itself. Three tiers of compound intelligence."
