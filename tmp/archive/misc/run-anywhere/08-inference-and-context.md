# Inference Architecture: Routing, Context Engineering, and Cost Optimization

> **Audience**: Developers integrating with roko, teams optimizing LLM costs
> **Scope**: How roko selects models, assembles context, and minimizes cost

---

## Three-Tier Cognitive Gating

The single most impactful mechanism: **don't call the LLM when you don't need to**.

| Tier | Handler | Model | Cost per Call | When Used |
|---|---|---|---|---|
| **T0** | Rust FSM + deterministic rules | None | $0.00 | ~80% of ticks (no state change, prediction accurate) |
| **T1** | Haiku-class LLM | claude-haiku / glm-4.5 | $0.001-0.003 | ~15% (surprising but routine) |
| **T2** | Frontier LLM | claude-opus / glm-5.1 | $0.01-0.25 | ~5% (high-stakes, novel, large prediction error) |

**Cost reduction**: 18x from cognitive gating alone. Combined with context engineering (6x from caching + pruning), total reduction is **~230x** vs. naive "always call Opus."

**Gating signal**: The prediction accuracy score on the CorticalState. When the agent's predictions about the environment are accurate (low prediction error), there's nothing to do → T0 suppresses. When prediction error spikes → T1 or T2 activates.

**Research**: FrugalGPT (Chen et al., 2024) — 98% cost reduction via cascade routing. DPT-Agent (Zhang et al., 2025) — separating thinking from acting.

---

## The Eight-Layer Context Pipeline

Every LLM request passes through eight layers of optimization before reaching the model:

### L1: Prompt Cache Alignment (90% Cost Reduction on Cached Prefix)

Stable content (system prompt, role instructions, tool definitions) is placed **first** in every request. The provider's KV cache recognizes the shared prefix and serves it at 90% discount (Anthropic) or 50% discount (OpenAI/GLM).

**Implementation**: The `SystemPromptBuilder` sorts sections by `CacheLayer` (Role → Workspace → Plan → Volatile). Section ordering is deterministic for maximum prefix match.

### L2: Semantic Cache (Cosine Similarity > 0.92)

If a semantically similar request was made recently, return the cached response. Uses embedding cosine similarity with 0.92 threshold.

**Latency**: 5-20ms (embedding computation + cache lookup).

### L3: Hash Cache (Exact Match, <1ms)

SHA-256 hash of the full request. If exact match exists in LRU cache, return immediately.

**Cost**: Zero — no LLM call, no embedding computation.

### L4: Tool Pruning (97.5% Token Reduction)

A roko agent may have 50+ tools available (16 built-in + MCP). Most are irrelevant for any given task. Tool pruning:
1. Classify the task (code edit? test? search? deploy?)
2. Remove tools not relevant to the classification
3. Reduce tool description verbosity for remaining tools

**Impact**: Tool definitions that would cost ~4,000 tokens → ~100 tokens after pruning.

### L5: History Compression (Lossy, 200-2000ms)

For multi-turn conversations exceeding 80% of context window:
1. Identify compactable region (old turns, not recent ones)
2. Summarize via cheap model (Haiku-class)
3. Replace compactable region with summary
4. Preserve: system prompt, recent turns, tool results from current task

**Research**: ACON (2025) — 26-54% token reduction from context compression. Semantic Kernel compaction framework — layered strategy (tool compaction → summarization → sliding window).

### L6: KV-Cache Routing (Session Affinity)

When the same agent makes multiple requests, route to the same provider endpoint to maximize server-side KV cache hits. The `prompt_cache_key` parameter hints the provider to maintain session affinity.

### L7: PII Masking (Round-Trip De-Identification)

ONNX-based NER model detects personally identifiable information. PII is replaced with deterministic tokens before sending to the LLM. Tokens are restored in the response. The LLM never sees real names, emails, or identifiers.

### L8: Injection Detection (DeBERTa Classifier)

A fine-tuned DeBERTa-v3 model classifies inputs for prompt injection attempts before they reach the LLM. Malicious inputs are blocked at the context pipeline, not at the LLM level.

**Research**: CaMeL (2025) — capability-based authorization separating control flow from data flow. Anthropic System Card — ~88% prompt injection block rate (12% failure → catastrophic for autonomous agents → motivates L8).

---

## The Context Governor (Adaptive Category Allocation)

The context pipeline doesn't just compress — it **learns which categories of context improve decisions**.

Based on Baddeley's working memory model (2000), the Context Governor maintains per-category token budgets that adapt via three feedback loops:

| Loop | Timescale | What It Learns |
|---|---|---|
| **Per-tick** | Each theta tick | Which context categories correlated with successful outcomes |
| **Per-curator** | Delta cycles (~50 ticks) | How category importance evolves over time |
| **Per-regime** | Regime changes | Complete restructuring of category weights when market regime shifts |

**Implementation**: Each context category (market data, position info, historical patterns, risk constraints, etc.) gets a token budget. The governor adjusts budgets based on outcome correlation — categories that help produce successful decisions get more tokens; irrelevant categories get fewer.

**Research**: Good Regulator Theorem (Conant & Ashby, 1970) — the context governor must model which context matters. Information Gain (Wang et al., 2025, arXiv:2510.14967) — IGPO measures information gain per turn.

---

## Provider Architecture

### Five Backend Types

| Provider | Auth | Logging | Payment | Use Case |
|---|---|---|---|---|
| **BlockRun** | x402 (USDC on Base) | Minimal | Per-request micropayment | Default for autonomous agents |
| **OpenRouter** | API key | Provider-dependent | Credit-based | 400+ models, BYOK option |
| **Venice** | DIEM stake | Zero-log guaranteed | Stake-based | Privacy-sensitive operations |
| **Direct** | API key (provider-native) | Provider terms | Standard billing | Maximum feature access |
| **Local** | None | None | Compute cost | Offline / air-gapped |

### Resource-Aware Model Selection

As the agent's resource budget decreases:
1. **Thriving phase** (>70% budget): Full T2 access, frontier models
2. **Stable phase** (50-70%): T2 only for high-stakes decisions
3. **Conservation phase** (30-50%): T1 for most tasks, T2 only for critical
4. **Declining phase** (10-30%): T0 + T1 only; maximize remaining budget
5. **Terminal phase** (<10%): Pure T0; no LLM spending

This creates **evolutionary selection pressure for efficient cognition**: agents that route well succeed; agents that waste inference fail.

**The economic math** (at 720-2,880 ticks/day):
- 80% T0 × $0.00 = $0.00
- 15% T1 × $0.003 = $0.32-1.30/day
- 4% T2-Sonnet × $0.01 = $0.29-1.15/day
- 1% T2-Opus × $0.25 = $1.80-7.20/day
- **Total: ~$2.41-9.65/day** inference cost

vs. naive (all Opus): $180-720/day. **Cost reduction: 18-75x from cognitive gating alone.**

With context engineering (cache alignment + tool pruning + compression): **~230x total reduction**.

**Budget impact**: One unnecessary Opus call ($0.25) depletes resources faster than dozens of cheaper tier calls. The routing decision is literally a make-or-break economic choice.

---

## The Cascade Router (Detailed)

### Stage 1: Static Table (0-49 observations)

Hardcoded role → model mapping:
```
mechanical tasks → haiku/kimi ($0.08)
focused tasks   → sonnet/glm ($0.19)
integrative     → sonnet/glm ($0.42)
architectural   → opus ($2.10)
```

### Stage 2: Wilson Confidence Intervals (50-199 observations)

For each model, compute:
- Pass rate = successes / trials
- Confidence width = 1.96 × sqrt(p×(1-p)/n)
- Upper bound = pass_rate + confidence_width

Select model with highest upper confidence bound. Unsampled models get UCB = infinity (forced exploration).

### Stage 3: LinUCB Contextual Bandit (200+ observations)

**17-dimensional feature vector** encoding:
```
[0..7]   = task category (one-hot: Implementation, Integration, Verification, Research, Refactor, Infra, Docs, Other)
[8]      = complexity (0.0 = mechanical, 0.5 = focused, 1.0 = architectural)
[9]      = iteration count / 10 (capped at 1.0)
[10..13] = role hash (FNV-based, distributed across 4 buckets)
[14]     = crate familiarity (success_count / total, clamped [0,1])
[15]     = has_prior_failure (0.0 or 1.0)
[16]     = bias term (always 1.0)
```

**LinUCB score**: `theta^T × x + alpha × sqrt(x^T × A_inv × x)`
- theta: learned parameter vector per arm (model)
- x: feature vector for current context
- alpha: exploration parameter (decays from 1.0 to 0.05 as observations increase)
- A_inv: inverse covariance matrix per arm (Cholesky decomposition for numerical stability)

**Reward**: `0.5 × pass_rate + 0.3 × (1 - normalized_cost) + 0.2 × (1 - normalized_latency)`

**Research**: LinUCB (Li et al., 2010) — contextual bandits with linear payoffs.

---

## What's Novel About This Stack

| Feature | Standard Approach | Roko's Approach |
|---|---|---|
| **When to call LLM** | Every request | Prediction-gated — 80% suppressed at T0 |
| **Which model** | Fixed per config | 3-stage cascade with contextual bandit |
| **Cache optimization** | None (provider handles it) | 6-layer cache alignment + canonical tool ordering |
| **Context size** | Send everything | 8-layer pipeline: cache + semantic + hash + prune + compress + route + mask + detect |
| **Context categories** | Fixed budgets | Adaptive governor learning which categories help |
| **Cost awareness** | After-the-fact reporting | Resource phase modulates model access in real-time |
| **Privacy** | Trust provider | L7 PII masking + L8 injection detection + Venice zero-log |
| **Tool surface** | All tools always | 97.5% pruning — only relevant tools in context |

## Orchestration as a Service (OaaS): Permissionless Compute Economy

The monolithic orchestration pipeline breaks into **5 independently operated MCP services**, each paid per-call via x402 micropayments:

| Service | Input → Output | Cost |
|---|---|---|
| **PRD Generator** | Task description → PRD document | $0.50-2.00 |
| **Plan Decomposer** | PRD → Plans YAML + DAG | $0.30-1.00 |
| **Agent Pool** | Plan + context → Code, tests, docs | $1.00-10.00 |
| **Review Service** | Work product → Review verdict + feedback | $0.50-3.00 |
| **Gate Runner** | Code + test specs → Pass/fail + diagnostics | $0.10-0.50 |

**Total example**: An ERC-4626 vault build = **$7.20** (zero human involvement, ~15 min wall time).

**Fractal decomposition**: Services can call other services. A complex task decomposes recursively until subtasks are atomic. Results aggregate upward.

**The flywheel**: More OaaS operators → more capacity → lower prices → more demand → exponential network effects.

**Research**: x402 protocol (EIP-3009) — wallet-native micropayments on Base. No API keys; payment IS authorization. Revenue split: 90% to service operator, 10% to protocol treasury, 0% for intra-clade calls.

---

## Decision Cache: Skip the LLM When You Already Know

When the agent's confidence on a routing/action decision exceeds 0.7, the decision is **cached** and reused without invoking the LLM. Cache invalidates on:
- Regime change (detected by probes)
- Prediction accuracy drops below threshold
- New high-priority signal arrives
- Cache TTL expires

**Hit rate**: ~60% in stable markets. At $0.10/T2 call, this saves ~$2.59/day (extending agent budgets significantly at current burn rate).

**Research**: Active Inference (Friston, 2010) — expected free energy minimization. When the expected information gain from a new LLM call is low (decision already cached with high confidence), the rational agent skips inference.

---

**The compound effect**: Cache alignment saves 50-90%. Tool pruning saves 97.5% of tool tokens. Cognitive gating suppresses 80% of requests. History compression saves 26-54% on remaining. Combined: ~230x cost reduction vs. naive prompting.

---

## The 13 Intent Types (Subsystem-Specific Routing)

Each subsystem declares its inference requirements as an **Intent** — not "which model" but "what features/quality":

| Intent | Typical Tier | Quality | Features Required |
|---|---|---|---|
| `heartbeat_t0` | T0 (no LLM) | N/A | Deterministic FSM only |
| `heartbeat_t1` | T1 (Haiku) | Low | Standard chat |
| `heartbeat_t2` | T2 (Sonnet/Opus) | High | Interleaved thinking, citations |
| `risk` | T2 (Opus) | Maximum | Interleaved thinking, never reduced |
| `dream` | T2 (Sonnet) | High | Visible thinking (for replay analysis) |
| `daimon` | T1 (Haiku) | Low | Privacy-preferred (affect state is sensitive) |
| `context` | T1 (Haiku) | Low | Inline think toggle |
| `curator` | T1 (Haiku) | Low | Standard chat |
| `playbook` | T1 (Sonnet) | Medium | Standard chat |
| `operator` | T2 (Sonnet) | Medium | Standard chat + citations |
| `death` | T2 (Opus) | Maximum | Visible thinking required |
| `safety` | T2 (Opus) | Maximum | PolicyCage constraint enforcement |
| `coordination` | T1 (Haiku) | Low | Clade communication |

The **Provider Resolver** walks an ordered provider list for each intent. First provider that satisfies the intent's feature requirements wins.

### PAD-Modulated Routing

The agent's emotional state dynamically adjusts inference:

| PAD Signal | Routing Effect |
|---|---|
| High arousal (surprised) | Lower T0→T1 threshold → spend more on inference ("this is surprising, think harder") |
| Low pleasure (frustrated) | Tier escalation more likely → use more capable model ("something is wrong") |
| High dominance (confident) | Higher T0 threshold → coast on cached decisions ("I know what to do") |
| Low dominance (uncertain) | Broader context retrieval → include more knowledge entries |

**Cost sensitivity scales with resource phase:**
- Thriving (>70%): Full Opus access accepted
- Conservation (30-50%): Downgrade T2→T1 for non-critical intents
- Terminal (<10%): T0 only; no inference spending

### The Self-Funding Economic Loop (DeFi Agents)

```
Agent earns revenue (LP fees, lending interest, arbitrage profit)
  → Revenue covers inference cost (x402 micropayments on Base)
  → Better inference → better strategies → more revenue → longer life

  OR:

  Revenue < inference cost → economic vitality drops
  → Resource phase degrades → cheaper models used
  → Performance may drop → revenue drops further
  → Terminal phase → death testament → legacy to successors
```

**The key constraint**: One unnecessary Opus call ($0.25) depletes 1.25 days of lifespan at typical burn rates. The routing decision is literally an economic survival choice.

---

## Prompt Normalization for Cache Optimization

Three techniques ensure identical logical prompts produce identical bytes (maximizing provider cache hits):

### 1. Canonical Tool Ordering
Tool definitions sorted alphabetically by name. Without this, `HashMap` iteration produces random orderings.

### 2. Whitespace Normalization
```rust
fn normalize(content: &str) -> String {
    content.lines()
        .map(|line| line.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
        .replace("\r\n", "\n")
        .replace("\t", "    ")
}
```

### 3. BTreeMap Serialization
All structured data (tool definitions, config objects, metadata) serialized via `BTreeMap` (sorted key order) instead of `HashMap` (random key order). This guarantees byte-identical JSON for identical data.

**Measured impact**: Cache hit rate 30% (HashMap) → 90%+ (BTreeMap) for identical role invocations.

---

## The Cognitive Workspace (Baddeley Working Memory Model)

The prompt is structured as a working memory system with three sections:

### Invariants (Always Included, Never Compressed)
- PolicyCage constraints (on-chain, safety-critical)
- Current positions (what the agent is responsible for)
- Active warnings (time-sensitive alerts)
- Strategy parameters (operator-defined rules)
- PAD affect state (emotional context)

### Rehearsed Knowledge (Budget-Allocated)
- Top-N playbook heuristics ranked by relevance
- 13 Munger-inspired mental models (reasoning scaffolds always loaded)

### Retrieved Knowledge (Budget-Allocated Per Learned ContextPolicy)
- Episodes (4-factor scoring: recency × relevance × emotional valence × causal weight)
- Insights and heuristics from Grimoire
- Causal graph edges
- **Contrarian entries** (minimum 15% mood-opposite per 200-tick window — anti-echo-chamber)
- Dream hypotheses (staged, partially validated)
- Somatic landscape reading (k-d tree valence map)

### Current Situation (Refreshed Each Tick)
- Observation data
- Resource context (vitality, budget remaining)
- Active interventions from operator
- Pheromone summary (threat/opportunity/wisdom levels)
- Conversation tail (if user is chatting)

**Research**: Baddeley (2000) — working memory model with episodic buffer. The context window IS working memory; the challenge is fitting the right information into limited capacity.

---

## Context Strategy Modes

Three strategies for how context is assembled, selectable per agent and per task:

| Strategy | How Context Is Built | Best For |
|---|---|---|
| **mcp_first** (default) | MCP code intelligence server provides structured context | Large codebases with good MCP support |
| **hybrid** | MCP for structure + inline file contents for implementation detail | Mixed codebases |
| **inline_heavy** | Full file contents with LLMLingua compression | Small codebases, unfamiliar repos |

Configurable in `roko.toml`:
```toml
context_strategy = "mcp_first"
context_limit_k = 200       # thousands of tokens (max input)
context_pressure_pct = 80   # trigger compaction at 80% of limit
```

---

## The Dynamic Prompt Generation Pipeline

### End-to-End: From Task to Minimal Prompt

Every prompt roko assembles passes through a six-stage pipeline that transforms a raw task into the minimal, maximally-effective prompt for that specific task, role, and complexity level.

### Stage 1: Task Classification

Each incoming task is classified along two dimensions:

**Category (8 types)**:
| Category | Signal Words | Example |
|---|---|---|
| `implementation` | build, create, add, implement, wire | "Add rate limiting to the API endpoint" |
| `integration` | connect, wire, hook, integrate, compose | "Wire the SystemPromptBuilder into orchestrate.rs" |
| `verification` | test, verify, validate, check, assert | "Add unit tests for the cascade router" |
| `research` | investigate, explore, analyze, understand | "Research how the MCP protocol handles timeouts" |
| `refactor` | rename, extract, simplify, reorganize | "Extract the gate logic into a separate module" |
| `infra` | deploy, configure, setup, CI, pipeline | "Configure the GitHub Actions workflow" |
| `docs` | document, describe, explain, update README | "Document the 8-layer context pipeline" |
| `other` | (fallback) | Anything not matching the above |

**Complexity band (4 levels)**:
| Band | Criteria | Typical Model |
|---|---|---|
| `mechanical` | Single file, <20 lines changed, no cross-module | Haiku / GLM-4.5 |
| `focused` | 1-3 files, clear scope, single crate | Sonnet / GLM-5 |
| `integrative` | 3-10 files, cross-crate, dependency graph | Sonnet / GLM-5.1 |
| `architectural` | 10+ files, new abstractions, system-wide impact | Opus |

Classification uses a cheap model (Haiku-class, <$0.001 per classification) or a rule-based heuristic when the task description is sufficiently clear (file count, crate references, keyword matching).

### Stage 2: Section Selection

Given the (category, role, complexity) triple, the pipeline selects which prompt sections to include. Each section has a measured **lift score** -- the difference in gate pass rate when the section is included vs. excluded.

**The selection rule**:
```
For each section S in the section library:
  lift = lookup(S, task_category, role)
  if lift > -0.02:     # not actively hurting
    include S with priority = lift
  else:
    exclude S
```

Sections with negative lift (hurting the agent for this task type) are excluded. Sections with positive lift are included, prioritized by lift magnitude.

**The cold-start problem**: New sections have no lift data. They are included at 50% probability for the first 20 observations (exploration), then their empirical lift determines inclusion.

### Stage 3: Compression

Two compression strategies depending on content type:

**Code context**: LLMLingua-2 (Jiang et al., 2023) achieves 26-54% token reduction on code while preserving semantic content. The compressor identifies low-information tokens (boilerplate, redundant type annotations, repeated patterns) and removes them while keeping the structural and semantic skeleton.

**History context**: Summarization via Haiku-class model. Multi-turn conversation history is summarized into a compact representation that preserves:
- Key decisions made and their rationale
- Errors encountered and fixes applied
- Current state of the task (what's done, what's remaining)
- Unresolved questions or blockers

**Selective compression**: Not all content is compressed equally. Recent turns (last 3-5) are kept verbatim. Older turns are progressively summarized. The compression ratio increases with age.

### Stage 4: Cache Alignment

Provider-side prefix caching requires that the shared prefix between requests be byte-identical. The pipeline ensures this:

1. **Stable sections first**: Role instructions, workspace rules, and tool definitions are placed at the beginning of the prompt. These are identical across invocations of the same role and maximize prefix cache hits.

2. **BTreeMap serialization**: All structured data (tool definitions, configuration objects, metadata) is serialized via `BTreeMap` (sorted keys), never `HashMap` (random keys). This guarantees byte-identical JSON for identical logical content.

3. **Deterministic ordering**: Sections within each cache layer are ordered by a fixed priority key, not by insertion order. The `CacheLayer` enum (Role > Workspace > Plan > Volatile) defines the ordering.

4. **Cache layer markers**: `<!-- roko:layer:N -->` tags in the generated prompt allow the provider (or a caching proxy) to identify prefix boundaries. Layer 0-1 content (Role + Workspace) is the most stable and most cacheable. Layer 3 (Volatile) changes every request.

### Stage 5: Tool Pruning

A full roko agent may have 50+ tool definitions available (16 built-in + MCP server tools). Including all definitions costs ~4,000 tokens. Most tools are irrelevant for any given task.

**Pruning by task category**:
| Task Category | Tools Included | Tools Excluded |
|---|---|---|
| `implementation` | file_edit, file_write, bash, cargo_check | deploy, monitor, bridge |
| `verification` | cargo_test, cargo_clippy, file_read, bash | file_write, deploy |
| `research` | file_read, grep, glob, web_fetch | file_write, file_edit, deploy |
| `docs` | file_read, file_write, file_edit | cargo_test, deploy, bridge |

**Result**: Tool definitions drop from ~4,000 tokens to ~100 tokens (97.5% reduction). The remaining tools are the ones the agent is likely to actually use.

**Safety**: Pruning never removes safety-critical tools (PolicyCage enforcement, resource checks). These are always included regardless of task category.

### Stage 6: Final Assembly

The surviving sections are assembled into the final prompt using the per-role budget allocation:

```
Total budget = context_limit_k × 1000 tokens

Per-role allocation (Implementer example):
  Code context:    45% of budget
  Plan context:    10% of budget
  Diff context:    10% of budget
  Documentation:    5% of budget
  Playbook rules:  15% of budget
  Tool definitions: 15% of budget
```

Within each allocation, sections are placed using U-shaped ordering (Liu et al., 2023): highest-lift sections at the beginning and end (high-attention zones), moderate-lift sections in the middle.

**Budget overflow**: If selected sections exceed the budget, lowest-lift sections are dropped first. The pipeline never truncates high-lift sections to make room for low-lift ones.

**Budget underflow**: If selected sections underfit the budget, additional context is pulled from the next-best excluded sections (those with lift close to the -0.02 threshold).

---

## Prompt Normalization Deep Dive

### The Cache Invalidation Problem

Provider-side prefix caching is the single largest cost optimization available (90% discount on cached prefix from Anthropic, 50% from OpenAI). But caching requires **byte-identical prefixes**. Any variation -- a reordered key, a trailing space, a different line ending -- invalidates the cache and forces full re-computation.

In practice, naive prompt construction produces wildly inconsistent bytes even for logically identical prompts. Three techniques solve this.

### Technique 1: Canonical Tool Ordering

Tool definitions are the most common source of cache invalidation. Without explicit ordering, `HashMap` iteration produces random orderings across invocations. Two identical requests with the same 15 tools but different tool orderings are cache misses.

**Solution**: Sort tool definitions alphabetically by name before serialization.

```rust
let mut tools: Vec<ToolDefinition> = tool_registry.all_tools();
tools.sort_by(|a, b| a.name.cmp(&b.name));
// Now serialize — identical tools always produce identical bytes
```

**Impact**: Eliminates the most frequent source of unnecessary cache misses. Tool definitions typically account for 15-20% of the prompt prefix.

### Technique 2: Whitespace Normalization

Invisible characters cause invisible cache misses:
- Trailing whitespace on any line
- Mixed line endings (`\r\n` vs `\n`)
- Tabs vs spaces inconsistency
- Multiple consecutive blank lines vs single blank lines

**Solution**: Normalize all content before inclusion in the prompt.

```rust
fn normalize(content: &str) -> String {
    content.lines()
        .map(|line| line.trim_end())      // Remove trailing whitespace
        .collect::<Vec<_>>()
        .join("\n")                         // Consistent line endings
        .replace("\r\n", "\n")              // Remove carriage returns
        .replace("\t", "    ")              // Tabs to 4 spaces
}
```

Applied to: all prompt sections, tool descriptions, context entries, configuration values. Everything that enters the prompt passes through normalization.

### Technique 3: BTreeMap Serialization

All structured data -- tool definitions, configuration objects, metadata, playbook entries -- is serialized via `BTreeMap` (sorted key order) instead of `HashMap` (random key order). This guarantees byte-identical JSON for logically identical data.

**Before (HashMap)**:
```json
// Run 1: {"model":"opus","temperature":0.7,"max_tokens":4096}
// Run 2: {"temperature":0.7,"max_tokens":4096,"model":"opus"}
// Run 3: {"max_tokens":4096,"model":"opus","temperature":0.7}
// Three different byte sequences → three cache misses
```

**After (BTreeMap)**:
```json
// Run 1: {"max_tokens":4096,"model":"opus","temperature":0.7}
// Run 2: {"max_tokens":4096,"model":"opus","temperature":0.7}
// Run 3: {"max_tokens":4096,"model":"opus","temperature":0.7}
// Identical bytes → cache hit every time
```

**Measured impact**: Cache hit rate improved from 30% (HashMap) to 90%+ (BTreeMap) for identical role invocations. On a typical plan run with 20 tasks, this saves ~$8-15 in inference costs.

### Cache Layer Markers

The prompt includes `<!-- roko:layer:N -->` markers that delineate cache boundaries:

```
<!-- roko:layer:0 -->
[Role instructions — identical across all invocations of this role]

<!-- roko:layer:1 -->
[Workspace rules — identical within a codebase]

<!-- roko:layer:2 -->
[Plan context — identical within a plan run]

<!-- roko:layer:3 -->
[Volatile — task-specific, changes every request]
```

Layers 0-1 are the most stable and produce the longest cache-hit prefix. Layer 2 is stable within a plan run. Layer 3 changes every request and is never cached.

A caching proxy (or provider with prefix-aware caching) can use these markers to identify the maximum cacheable prefix per request, even when Layer 3 content varies.

---

## The Inference Gateway Architecture

### Between Agents and LLM Providers

The inference gateway sits between roko agents and LLM providers, handling caching, routing, rate limiting, and cost optimization transparently. Agents make inference requests as if talking directly to a provider; the gateway intercepts and optimizes.

### Three Caching Layers

The gateway implements three caching layers, checked in order from fastest to slowest:

#### L3: Hash Cache (Exact Match)

- **Mechanism**: SHA-256 hash of the full request (system prompt + messages + tools + parameters)
- **Latency**: <1ms (hash computation + LRU lookup)
- **Hit rate**: ~10-15% (requires byte-identical requests)
- **Cost**: Zero -- no LLM call, no embedding computation
- **When it hits**: Retries of identical requests, deterministic tool calls, repeated status checks

#### L2: Semantic Cache (Fuzzy Match)

- **Mechanism**: Embed the request via a lightweight model (384-dim MiniLM). Cosine similarity > 0.92 against cached embeddings triggers a hit.
- **Latency**: 5-20ms (embedding computation + ANN search)
- **Hit rate**: ~20-30% (catches semantically equivalent but not byte-identical requests)
- **Cost**: Embedding computation only (~$0.0001 per request)
- **When it hits**: Rephrased requests, slightly different context orderings, minor whitespace variations that escape L3

#### L1: Prefix Cache (Provider-Side)

- **Mechanism**: Provider's own KV cache recognizes shared prefixes between requests
- **Latency**: Provider-dependent (transparent to gateway)
- **Hit rate**: 81-91% of requests that pass L3 and L2 (high because prompt normalization ensures stable prefixes)
- **Cost**: 90% discount on cached prefix (Anthropic), 50% (OpenAI)
- **When it hits**: Same role/workspace/plan prefix across different tasks

### Combined Effectiveness

```
100 requests arrive at the gateway:
  → 12 served by L3 Hash Cache (exact match, $0.00 each)
  → 22 served by L2 Semantic Cache (fuzzy match, ~$0.0001 each)
  → 66 proceed to provider
    → 56 get L1 prefix cache hit (90% discount on prefix)
    → 10 are full-price inference

Effective: only ~10% of original requests require full-price inference.
```

**Effective cost per task**: $0.30-0.50 with gateway vs. $2.00-8.00 without. The gateway pays for itself within the first plan run.

### Batch API Integration

Non-urgent work is routed to Batch APIs for additional cost savings:

| Work Type | Urgency | API | Discount |
|---|---|---|---|
| Task execution (agent loop) | High | Standard (streaming) | None |
| Context enrichment | Medium | Batch (Anthropic) | 50% |
| Episode analysis | Low | Batch (Anthropic/OpenAI) | 50% |
| Playbook distillation | Low | Batch (Anthropic) | 50% |
| Dream replay | Low | Batch (Anthropic) | 50% |

**Anthropic Batch API**: Up to 50% discount for requests that can tolerate up to 24 hours of latency. Episode analysis, playbook distillation, and dream replay are all offline workloads that benefit from batch pricing.

**OpenAI Batch API**: Similar 50% discount model. Used for embedding computation (MiniLM) and secondary model evaluations.

**Routing logic**: The gateway classifies each request by urgency. Only task execution (the inner agent loop) requires streaming. Everything else can be batched.

### Rate Limiting

Per-provider token buckets with exponential backoff ensure agents never hit provider rate limits:

```
Per provider:
  - Requests per minute: configurable (default: 50 RPM for Anthropic, 60 for OpenAI)
  - Tokens per minute: configurable (default: 100K TPM)
  - Concurrent requests: configurable (default: 5)

On limit hit:
  - Exponential backoff: 1s → 2s → 4s → 8s → 16s (max)
  - Jitter: ±20% randomization to prevent thundering herd
  - Spillover: if primary provider is rate-limited, route to secondary
```

**Agent transparency**: Agents do not know about rate limits. They make requests; the gateway handles queuing, backoff, and provider switching transparently. From the agent's perspective, the request simply takes longer.

**Provider failover**: If a provider returns 5xx errors or rate limit responses, the gateway automatically fails over to the next provider in the priority list. The agent sees a slightly longer latency, not an error.

### Cost Tracking

Every request through the gateway is metered:

```json
{
  "request_id": "req_abc123",
  "provider": "anthropic",
  "model": "claude-opus-4-6",
  "input_tokens": 12450,
  "output_tokens": 3200,
  "cached_tokens": 11200,
  "cache_layer": "L1",
  "cost_full": 0.42,
  "cost_actual": 0.08,
  "savings": 0.34,
  "latency_ms": 2340,
  "batch": false
}
```

These events feed into the efficiency tracking system (`.roko/learn/efficiency.jsonl`), enabling the cascade router to factor actual cost into model selection decisions.

**Research**: FrugalGPT (Chen et al., 2024) — cascading LLM calls for cost reduction. RouteLLM (Ong et al., 2024) — learned routing between models. ACON (2025) — context compression at the inference layer. Anthropic prompt caching documentation — prefix cache mechanics and pricing.
