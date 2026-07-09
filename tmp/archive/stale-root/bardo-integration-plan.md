# Bardo → Roko Integration Plan

> **Generated**: 2026-04-24 (v2 — fully scoped)
> **Source**: `/Users/will/dev/uniswap/bardo/` (34 crates, 8 apps, ~324K LOC)
> **Target**: `/Users/will/dev/nunchi/roko/roko/` (18 crates, ~177K LOC)
> **Total tasks**: 78 across 12 phases
> **Each task**: Agent-executable with source refs, prompt, and acceptance criteria

---

## Naming Map

| Bardo | Roko | Notes |
|-------|------|-------|
| golem-core | roko-core | Already migrated |
| golem-runtime | roko-runtime | Already migrated |
| golem-grimoire | roko-neuro | Renamed; partial port |
| golem-daimon | roko-daimon | Already migrated |
| golem-dreams | roko-dreams | Already migrated |
| golem-chain | roko-chain | Partial port |
| golem-tools | roko-std | Partial (19 builtin tools, no DeFi) |
| golem-heartbeat | roko-conductor + roko-runtime | Partial |
| golem-safety | roko-agent/safety | Already migrated |
| golem-eval | roko-gate | Already migrated |
| golem-inference | (new: roko-gateway) | Not ported |
| golem-triage | (merge into roko-orchestrator) | Not ported |
| golem-context | roko-compose (partial) | VCG exists |
| golem-identity | (merge into roko-chain) | Not ported |
| bardo-gateway | (new: crates/roko-gateway) | Key missing piece |
| dashboard | (new: apps/dashboard) | Port + extend |
| mori | roko-cli/src/orchestrate.rs | Reference only |
| mpp | (new: crates/roko-mpp) | Port for payments |

---

## Phase 1: Inference Gateway (port bardo-gateway)

> **Canonical spec**: Detailed gateway architecture (pipeline, subsystems, types) is in
> `tmp/roko-architecture-redesign-v2.md` section "Inference gateway". The tasks below are
> implementation work items that reference that spec.

### Task 1.1: Port inference protocol types

**Description**: Create `crates/roko-gateway/` with core types for the inference proxy.

**Source files**:
- `bardo/crates/bardo-inference/src/lib.rs` (413 LOC) — `InferenceRequest`, `InferenceResponse`, `TokenUsage`, `StopReason`
- `bardo/crates/golem-inference/src/client.rs` (723 LOC) — `InferenceClient` trait, `GatewayClient`, `InferenceMeta`

**Target**: `crates/roko-gateway/src/types.rs`, `crates/roko-gateway/Cargo.toml`

**Prompt**:
> Create a new crate `roko-gateway` in the workspace. Port the inference protocol types from bardo-inference and golem-inference. Key types: `InferenceRequest` (model, messages, max_tokens, temperature, tools, stream), `InferenceResponse` (text, stop_reason, usage), `TokenUsage` (input_tokens, output_tokens, cache_read_input_tokens, cache_creation_input_tokens, thinking_tokens, reasoning_tokens), `StopReason` (EndTurn, MaxTokens, ToolUse, ContentFilter). Also port the `InferenceClient` trait with `complete()` and `stream()` async methods and `InferenceMeta` (session_id, agent_id, tier, budget_remaining). Reference bardo's implementations but adapt to roko naming conventions (no golem prefix). Add the crate to workspace Cargo.toml.

**Acceptance criteria**:
- [ ] `crates/roko-gateway/` exists with `Cargo.toml` and `src/lib.rs`
- [ ] `InferenceRequest`, `InferenceResponse`, `TokenUsage`, `StopReason` structs compile
- [ ] `InferenceClient` trait with `complete()` and `stream()` methods
- [ ] Crate added to workspace and `cargo check -p roko-gateway` passes
- [ ] Serde Serialize/Deserialize derived on all public types

**Size**: S (half day)

---

### Task 1.2: Port hash cache (Layer 1)

**Description**: Port the exact-match hash cache from bardo-gateway.

**Source files**:
- `bardo/apps/bardo-gateway/src/cache.rs` — blake3 hashing, moka LRU, regime-aware TTL, normalization
- Key logic: `blake3(normalized_body)` → `CachedResponse { body, cost_usd, model, cached_at, effective_ttl }`
- Normalization: strip UUIDs, timestamps, git status blocks, sort JSON keys + tool definitions

**Target**: `crates/roko-gateway/src/cache/hash_cache.rs`

**Prompt**:
> Port bardo-gateway's hash cache (L1) into roko-gateway. The hash cache stores exact-match responses keyed by blake3 hash of normalized request bodies. Implementation requirements:
> 1. Use `moka` async LRU cache with TTL eviction (default 3600s)
> 2. Normalize requests before hashing: strip UUIDs (`[0-9a-f]{8}-...`), ISO timestamps, `cch=` hashes, `CWD:` lines, `Date:` lines, git status blocks → replace with `[VAR]`/`[GIT_STATUS]` placeholders. Sort JSON keys alphabetically. Sort tool definitions by name.
> 3. Exclude from cache: tool_use responses (stale tool IDs), responses with <3 output tokens, error payloads
> 4. `CachedResponse` struct: `body: Bytes`, `cost_usd: f64`, `model: String`, `cached_at: Instant`, `effective_ttl: Duration`
> 5. Regime-aware TTL: Normal=3600s, Calm=7200s, Volatile=900s, Crisis=300s (via `CachePolicy` enum)
> Reference: bardo/apps/bardo-gateway/src/cache.rs

**Acceptance criteria**:
- [ ] `HashCache::get(request_body) -> Option<CachedResponse>` works
- [ ] `HashCache::put(request_body, response, cost, model)` stores with TTL
- [ ] Normalization strips variable content (test with UUID/timestamp-laden input)
- [ ] Exclusion rules enforce no caching of tool_use / tiny / error responses
- [ ] Unit tests with synthetic requests verify hit/miss/expiry behavior
- [ ] `cargo test -p roko-gateway` passes

**Size**: M (2 days)

---

### Task 1.3: Port semantic cache (Layer 2 — SimHash)

**Description**: Port the similarity-based semantic cache using SimHash fingerprinting.

**Source files**:
- `bardo/apps/bardo-gateway/src/semantic_cache.rs` — SimHash backend, 64-bit fingerprints, Hamming distance matching

**Target**: `crates/roko-gateway/src/cache/semantic_cache.rs`

**Prompt**:
> Port bardo-gateway's semantic cache (L2) SimHash backend. This catches near-miss requests that differ slightly from cached ones.
> 1. `SimHash` fingerprinting: Tokenize request text → hash each token → XOR into 64-bit fingerprint
> 2. Matching: Hamming distance ≤ threshold (default 3 bits) = cache hit
> 3. Storage: `DashMap<u64, SimHashEntry>` where `SimHashEntry` = `{ response: Bytes, cost_usd, model, created_at }`
> 4. TTL: 7200s fixed (not regime-aware for L2)
> 5. Max entries: 5000 (LRU eviction by age)
> 6. Same exclusions as hash cache (no tool_use, no <3 tokens, no errors)
> 7. Namespace isolation: prepend namespace prefix to cache text for multi-tenant
> Do NOT port the embedding backend (fastembed) — SimHash only for now.
> Reference: bardo/apps/bardo-gateway/src/semantic_cache.rs

**Acceptance criteria**:
- [ ] `SemanticCache::lookup(text) -> Option<CachedResponse>` finds similar entries
- [ ] Hamming distance threshold configurable
- [ ] Max 5000 entries with LRU eviction
- [ ] Namespace isolation prevents cross-tenant cache hits
- [ ] Unit test: two similar prompts (differing by 1 word) hit cache; dissimilar prompts miss

**Size**: M (2 days)

---

### Task 1.4: Port provider abstraction + key rotation

**Description**: Unified provider backend trait with key rotation on 429.

**Source files**:
- `bardo/apps/bardo-gateway/src/providers/anthropic.rs` — Anthropic with key rotation, streaming, tool use, extended thinking, prefix caching
- `bardo/apps/bardo-gateway/src/providers/openai.rs` — OpenAI with format translation, reasoning tokens
- `bardo/apps/bardo-gateway/src/providers/mod.rs` — Provider resolution (priority order)

**Target**: `crates/roko-gateway/src/providers/`

**Prompt**:
> Create a `ProviderBackend` trait in roko-gateway with `complete()` and `stream()` methods. Implement for Anthropic and OpenAI initially.
> Key requirements:
> 1. **Anthropic**: POST `https://api.anthropic.com/v1/messages`. Support streaming, tool use, extended thinking, prefix caching (`cache_control: {"type": "ephemeral", "ttl": "1h"}`). Extract `cache_read_input_tokens`, `cache_creation_input_tokens`, `thinking_tokens`.
> 2. **OpenAI**: POST `https://api.openai.com/v1/chat/completions`. Translate Anthropic format ↔ OpenAI format. Extract `cached_tokens` (from `prompt_tokens_details`), `reasoning_tokens`.
> 3. **Key rotation**: Accept `Vec<String>` of API keys. On 429 (rate limit), rotate to next key. Track which key is active.
> 4. **Provider resolution**: Priority order: Anthropic for `claude-*`, OpenAI for `gpt-*/o1/o3/o4-*`.
> 5. Do NOT duplicate roko-agent backends — the gateway wraps provider API calls, while roko-agent handles agent-level dispatch.
> Reference: bardo/apps/bardo-gateway/src/providers/

**Acceptance criteria**:
- [ ] `AnthropicProvider::complete()` calls Anthropic API with proper headers
- [ ] `OpenAiProvider::complete()` translates format and calls OpenAI API
- [ ] Key rotation on 429: test with mock server returning 429, verify key switch
- [ ] Prefix caching: system block gets `cache_control` annotation
- [ ] Token usage extraction matches bardo's field mapping

**Size**: L (3-4 days)

---

### Task 1.5: Port cost computation + tracking

**Description**: Per-request cost calculation with actual vs naive pricing.

**Source files**:
- `bardo/apps/bardo-gateway/src/pricing.rs` — Per-model pricing table (USD/1M tokens)
- `bardo/apps/bardo-gateway/src/handler.rs` — `compute_cost()` function with cache/batch discounts
- `bardo/apps/bardo-gateway/src/cost_db.rs` — SQLite persistence

**Target**: Wire into existing `roko-learn/src/costs_db.rs` + new `crates/roko-gateway/src/pricing.rs`

**Prompt**:
> Port bardo-gateway's cost computation into roko-gateway.
> 1. **Pricing table**: HashMap<model_slug, ModelPricing> where `ModelPricing = { input_per_m, output_per_m, cached_input_per_m, reasoning_per_m }`. Default fallback: $3/M input, $15/M output. Support substring matching for model families.
> 2. **Cost computation** (per request):
>    - Fresh input tokens × input_per_m / 1e6
>    - Cached tokens × cached_input_per_m / 1e6 (Anthropic: 10%, OpenAI: 50%)
>    - Cache write tokens × input_per_m × 1.25 / 1e6 (25% surcharge)
>    - Regular output tokens × output_per_m / 1e6
>    - Reasoning tokens × reasoning_per_m / 1e6
>    - Thinking tokens × output_per_m / 1e6
>    - Batch requests: 50% discount on total
> 3. **Naive cost**: What the provider would charge without any caching (total_input × input + total_output × output). Savings = naive - actual.
> 4. **Integration**: Insert a `CostRecord` into existing `roko-learn/src/costs_db.rs` per request. Add `agent_id` and `session_id` fields to `CostRecord` if not present.
> Reference: bardo/apps/bardo-gateway/src/pricing.rs, handler.rs (compute_cost function)

**Acceptance criteria**:
- [ ] `compute_cost(usage, pricing, is_batch) -> (actual, naive)` matches bardo's calculations
- [ ] Pricing table loaded from config with model family substring matching
- [ ] CostRecord inserted per gateway request with agent_id attribution
- [ ] Savings tracking: `naive - actual` stored per request
- [ ] Unit tests verify pricing for Opus, Sonnet, Haiku, GPT-4o with cache/batch scenarios

**Size**: M (2 days)

---

### Task 1.6: Port loop detection

**Description**: Detect infinite agent loops (retry, oscillation, drift) and inject corrective guidance.

**Source files**:
- `bardo/apps/bardo-gateway/src/loop_guard.rs` — SessionLoopState, ring buffer of recent calls, threshold detection

**Target**: `crates/roko-gateway/src/loop_guard.rs`

**Prompt**:
> Port bardo-gateway's loop detection system. Per-session state tracks recent tool calls in a ring buffer:
> 1. **SessionLoopState**: `recent_calls: VecDeque<(tool_name, blake3_args_hash)>` (capacity 16), `consecutive_identical: u32`, `tokens_since_progress: u64`
> 2. **Retry detection**: Same tool + args called 5+ times consecutively → inject "try a different approach"
> 3. **Oscillation detection**: A→B→A→B pattern after 3+ full cycles → inject "break the loop"
> 4. **Drift detection**: 15,000+ output tokens accumulated without new tool_result content → inject "make progress or stop"
> 5. **Injection**: Return guidance string to prepend to next system prompt
> 6. **Counters**: Track `loops_detected`, `loop_injections`, `loop_retry_detected`, `loop_oscillation_detected`, `loop_drift_detected` for dashboard stats
> Reference: bardo/apps/bardo-gateway/src/loop_guard.rs

**Acceptance criteria**:
- [ ] Retry detection fires after 5 identical calls (test with mock call sequence)
- [ ] Oscillation detection fires after 3 A→B→A→B cycles
- [ ] Drift detection fires after 15K tokens without progress
- [ ] Guidance strings are non-empty and actionable
- [ ] Counters increment correctly per detection type
- [ ] Ring buffer doesn't grow unbounded (cap at 16)

**Size**: M (1-2 days)

---

### Task 1.7: Port output budgeting

**Description**: EMA-based automatic `max_tokens` capping per model.

**Source files**:
- `bardo/apps/bardo-gateway/src/output_budget.rs` — ModelOutputStats, EMA with variance, p95 estimation

**Target**: `crates/roko-gateway/src/output_budget.rs`

**Prompt**:
> Port bardo-gateway's output budgeting system.
> 1. **Per-model EMA tracking**: `ModelOutputStats { ema: f64, ema_sq: f64, max_seen: u64, count: u64 }`
> 2. **ALPHA**: 0.05 (5% weight to new observations)
> 3. **Minimum samples**: 20 before p95 estimation is trusted
> 4. **p95 calculation**: `ema + 2 * sqrt(ema_sq - ema^2)` (EMA + 2σ)
> 5. **Cap**: `p95 × 1.5`, floored at 1024 tokens minimum
> 6. **Behavior**: When `max_tokens` is absent or unreasonably high in a request, auto-set to computed cap. Never override explicit user-set values that are below the cap.
> 7. **Counter**: `output_budgets_applied`, `output_tokens_bounded`
> Reference: bardo/apps/bardo-gateway/src/output_budget.rs

**Acceptance criteria**:
- [ ] After 20+ observations, cap is computed and applied to requests missing max_tokens
- [ ] Cap never goes below 1024 tokens
- [ ] Explicit user max_tokens below cap is respected (not overridden)
- [ ] EMA updates correctly with alpha=0.05
- [ ] Unit test: feed 50 observations of ~1000 tokens, verify cap is ~2000-3000

**Size**: S (1 day)

---

### Task 1.8: Port tool pruning

**Description**: Session-aware adaptive tool schema compression.

**Source files**:
- `bardo/apps/bardo-gateway/src/tools.rs` — Per-session tool usage tracking, two-tier pruning, never-prune list

**Target**: `crates/roko-gateway/src/tool_pruning.rs`

**Prompt**:
> Port bardo-gateway's tool pruning system.
> 1. **Usage tracking**: Per-session `HashMap<tool_name, invocation_count>` + global `HashMap<tool_name, total_count>`
> 2. **Never-prune list** (core tools): Bash, Read, Write, Edit, Glob, Grep, WebSearch, WebFetch, TaskCreate, TaskUpdate, TaskList, Agent, SendMessage
> 3. **Session pruning (Tier 1)**: After 50+ requests in a session, remove tools never used in THIS session (keep protected + used)
> 4. **Global pruning (Tier 2)**: Before 50 session requests but with 50+ global requests, remove tools never used by ANY session
> 5. **Metrics**: `tools_pruned` count, `tool_tokens_saved` estimate (count removed tool schemas × avg schema size)
> Reference: bardo/apps/bardo-gateway/src/tools.rs

**Acceptance criteria**:
- [ ] Protected tools never pruned regardless of usage
- [ ] Session pruning activates after 50 requests
- [ ] Tools used at least once in session survive pruning
- [ ] Global fallback works for new sessions
- [ ] Metrics accurately count pruned tools and estimated token savings

**Size**: S (1 day)

---

### Task 1.9: Port convergence detection

**Description**: Detect when responses become repetitive and inject guidance.

**Source files**:
- `bardo/apps/bardo-gateway/src/convergence.rs` — SimHash of responses, consecutive similarity tracking

**Target**: `crates/roko-gateway/src/convergence.rs`

**Prompt**:
> Port bardo-gateway's convergence detection.
> 1. **Per-session state**: `recent_hashes: VecDeque<u64>` (last ~8 response SimHashes), `consecutive_similar: u32`
> 2. **Detection**: Compare latest response SimHash to previous. Hamming distance ≤ 2 = "similar". 3+ consecutive similar → convergence flagged.
> 3. **Injection**: On next request, inject guidance: "Your recent responses are converging. Try a different angle or move to the next step."
> 4. **Counters**: `convergence_detected`, `convergence_injections`
> Reference: bardo/apps/bardo-gateway/src/convergence.rs

**Acceptance criteria**:
- [ ] 3 consecutive similar responses triggers convergence
- [ ] Guidance injected on next request after detection
- [ ] Dissimilar response resets counter
- [ ] Counters increment correctly

**Size**: S (half day)

---

### Task 1.10: Port thinking cap

**Description**: Per-model extended thinking budget caps.

**Source files**:
- `bardo/apps/bardo-gateway/src/thinking_cap.rs` — Per-model defaults, injection logic

**Target**: `crates/roko-gateway/src/thinking_cap.rs`

**Prompt**:
> Port bardo-gateway's thinking cap system.
> 1. **Per-model defaults**: Opus=32768, Sonnet=16384, Haiku=4096 thinking tokens
> 2. **Logic**: Only activates if thinking already enabled (`{"type": "enabled"}`). Never forces thinking on. If no explicit `budget_tokens`, inject model-appropriate default. Respect explicit user budgets (don't override lower values).
> 3. **Counter**: `thinking_budgets_applied`, `thinking_tokens_capped_estimate`
> Reference: bardo/apps/bardo-gateway/src/thinking_cap.rs

**Acceptance criteria**:
- [ ] Thinking budget injected only when thinking is enabled but no budget set
- [ ] Per-model caps correct (opus=32K, sonnet=16K, haiku=4K)
- [ ] Explicit user budgets preserved
- [ ] Thinking NOT forced on when disabled

**Size**: S (half day)

---

### Task 1.11: Wire gateway into roko-serve

**Description**: Create HTTP routes in roko-serve that proxy inference through the gateway.

**Target**: `crates/roko-serve/src/routes/gateway.rs`

**Prompt**:
> Add gateway routes to roko-serve:
> 1. `POST /api/gateway/inference` — Main inference proxy endpoint
>    - Auth middleware validates agent token (existing middleware)
>    - Resolve API key from secret store (existing secrets module)
>    - Run through gateway pipeline: loop check → cache lookup → tool prune → output budget → thinking cap → convergence check → provider call → cache store → cost track
>    - Return `InferenceResponse`
> 2. `GET /api/gateway/stats` — Current gateway statistics (cache hits, costs, sessions)
> 3. `GET /api/gateway/ws` — WebSocket endpoint streaming per-request StatsEvents
>    - Broadcast channel (1024 slot capacity)
>    - Message format: `StatsEvent { seq, timestamp_ms, model, provider, input_tokens, output_tokens, cache_read_tokens, cost_usd, naive_cost_usd, savings_usd, cache_hit, elapsed_ms, session_id, gateway_actions: Vec<String> }`
> 4. Add `GatewayState` to `AppState` (shared across routes)
> Wire the cache, providers, and cost tracking from Tasks 1.2-1.10.

**Acceptance criteria**:
- [ ] `POST /api/gateway/inference` returns valid LLM response
- [ ] Second identical request returns cached response (hash cache hit)
- [ ] Cost recorded in costs_db per request
- [ ] WebSocket endpoint streams events to connected clients
- [ ] Auth required (401 without valid key)
- [ ] `cargo test -p roko-serve` passes

**Size**: L (3-4 days)

---

### Task 1.12: Port batch API

**Description**: Anthropic batch API integration for 50% cost reduction on async work.

**Source files**:
- `bardo/apps/bardo-gateway/src/batch.rs` — Queue, auto-flush, polling, result retrieval

**Target**: `crates/roko-gateway/src/batch.rs` + `crates/roko-serve/src/routes/gateway.rs`

**Prompt**:
> Port bardo-gateway's batch API.
> 1. `POST /api/gateway/batch/submit` — Queue request, return 202 + custom_id (`roko-{uuid}`)
> 2. Auto-flush triggers: 50 items accumulated OR 30 seconds elapsed OR manual `POST /api/gateway/batch/flush`
> 3. Submit to `POST https://api.anthropic.com/v1/messages/batches`
> 4. Poll every 60s: `GET /v1/messages/batches/{batch_id}`
> 5. Results in `DashMap<custom_id, BatchResult>`
> 6. `GET /api/gateway/batch/result/{custom_id}` retrieves completed result
> 7. Same preprocessing as real-time (prefix cache, cost tracking) but 50% cost discount
> Reference: bardo/apps/bardo-gateway/src/batch.rs

**Acceptance criteria**:
- [ ] Submit returns 202 with custom_id
- [ ] Auto-flush at 50 items (test with mock Anthropic endpoint)
- [ ] Timer flush at 30s
- [ ] Result retrievable by custom_id after batch completes
- [ ] Cost calculation applies 50% batch discount

**Size**: M (2-3 days)

---

## Phase 2: Orchestrator Gaps (from mori)

### Task 2.1: Port structured review verdict system

**Description**: Parse agent review output into structured verdicts with issue classification.

**Source files**:
- `bardo/apps/mori/src/orchestrator/review.rs` — `StructuredReview`, `ReviewVerdict`, `ReviewIssue`, `IssueCategory`, `IssueSeverity`

**Target**: `crates/roko-gate/src/review_verdict.rs` + wire into `crates/roko-cli/src/orchestrate.rs`

**Prompt**:
> Port mori's structured review verdict system.
> 1. **Types**:
>    - `ReviewVerdict` enum: `Approve | Revise | Skip`
>    - `ReviewIssue { severity: IssueSeverity, category: IssueCategory, file: Option<String>, line: Option<u32>, description: String }`
>    - `IssueSeverity` enum: `Blocking | Major | Minor`
>    - `IssueCategory` enum: `Compilation | Test | TypeMismatch | MissingImpl | Docs | Style | SpecDeviation`
>    - `IssueCategory::is_quick_fixable()` → true for Compilation, Docs, Style
>    - `StructuredReview { verdict, issues: Vec<ReviewIssue>, summary: String }`
>    - `StructuredReview::all_issues_quick_fixable()` → true when all issues are quick-fixable
> 2. **Parsing**: Try JSON first, then JSON code block, then TOML block. Provide JSON schema for reviewer agents.
> 3. **Integration**: In orchestrate.rs, after review phase, parse agent output as StructuredReview. If `all_issues_quick_fixable()`, skip strategist and go directly to implementer (express mode).
> Reference: bardo/apps/mori/src/orchestrator/review.rs

**Acceptance criteria**:
- [ ] `StructuredReview` parses from JSON agent output
- [ ] `IssueCategory::is_quick_fixable()` returns correct values
- [ ] `all_issues_quick_fixable()` correctly identifies trivial-fix scenarios
- [ ] Fallback parsing handles malformed JSON gracefully (returns Revise with raw text)
- [ ] Integration test: mock review JSON → parsed verdict → correct phase transition

**Size**: M (2-3 days)

---

### Task 2.2: Port compile error classification + auto-fix

**Description**: Parse cargo JSON output into classified error types for targeted auto-fix.

**Source files**:
- `bardo/apps/mori/src/orchestrator/autofix.rs` — `CompileErrorClass`, `parse_cargo_json_errors()`, `collect_rustc_suggestions()`, `apply_rustc_fixes()`

**Target**: `crates/roko-gate/src/compile_errors.rs` + wire into orchestrate.rs

**Prompt**:
> Port mori's compile error classification system.
> 1. **CompileErrorClass** enum:
>    - `ImportNotFound { module, item, file, line }`
>    - `TypeMismatch { expected, found, file, line }`
>    - `MissingField { struct_name, field, file, line }`
>    - `TraitNotImplemented { type_name, trait_name, file, line }`
>    - `Other { code: String, message, file, line }`
> 2. **`parse_cargo_json_errors(json_output: &str) -> Vec<CompileErrorClass>`**: Parse `cargo check --message-format=json` output. Extract `rendered`, `code`, `spans[0].file_name`, `spans[0].line_start`.
> 3. **`collect_rustc_suggestions(json_output: &str) -> Vec<RustcSuggestion>`**: Extract `children[].suggested_replacement` from diagnostic JSON.
> 4. **`apply_rustc_fixes(worktree: &Path)`**: Run `cargo fix --allow-dirty` + `cargo fmt` to apply compiler-suggested fixes directly (no agent needed).
> 5. **Integration**: In orchestrate.rs autofix path, first try `apply_rustc_fixes()`. If that resolves all errors, skip agent retry. Otherwise, pass classified errors to agent instead of raw cargo output.
> Reference: bardo/apps/mori/src/orchestrator/autofix.rs

**Acceptance criteria**:
- [ ] `parse_cargo_json_errors()` extracts structured errors from real cargo JSON
- [ ] `CompileErrorClass` variants populated with correct file/line/details
- [ ] `collect_rustc_suggestions()` finds and extracts suggested replacements
- [ ] `apply_rustc_fixes()` runs cargo fix + fmt successfully (test with intentional error)
- [ ] Agent receives classified errors instead of raw output (verified in prompt)

**Size**: M (2-3 days)

---

### Task 2.3: Port error pattern discovery + sharing

**Description**: Share discovered error patterns across parallel agents.

**Source files**:
- `bardo/apps/mori/src/orchestrator/gates.rs` — `extract_error_digest()`, `append_discovered_pattern()`, `read_discovered_patterns()`, `GateResult::is_mostly_passing()`

**Target**: `crates/roko-gate/src/error_patterns.rs` + wire into orchestrate.rs

**Prompt**:
> Port mori's error pattern discovery and sharing system.
> 1. **`extract_error_digest(output: &str) -> String`**: Parse cargo/test output, extract `error[E...]` blocks, deduplicate via HashSet, cap at 10 unique errors, cap each at 200 chars. Return compact digest.
> 2. **`append_discovered_pattern(repo_root, plan, error_digest)`**: Write to `.roko/learn/discovered-patterns.json`. Format: `{ "patterns": [{ "plan", "digest", "timestamp", "resolved": bool }] }`
> 3. **`read_discovered_patterns() -> Vec<DiscoveredPattern>`**: Read last 5 unresolved patterns (200 chars each). Used to inject into agent context so parallel agents learn from each other's failures without re-discovering.
> 4. **`GateResult::is_mostly_passing(results) -> bool`**: >90% pass rate with >20 tests and ≥1 failure = "mostly passing". This means the gate is close enough that a targeted fix (not full replan) should suffice.
> 5. **Integration**: In orchestrate.rs, after gate failure: call `extract_error_digest()` → `append_discovered_pattern()`. Before agent dispatch: call `read_discovered_patterns()` → inject into system prompt.
> Reference: bardo/apps/mori/src/orchestrator/gates.rs

**Acceptance criteria**:
- [ ] `extract_error_digest()` produces compact, deduped error signatures from real cargo output
- [ ] Patterns persisted to `.roko/learn/discovered-patterns.json`
- [ ] Parallel agents see each other's patterns (read from shared file)
- [ ] `is_mostly_passing()` returns true for 95% pass with 1 failure, false for 50% pass
- [ ] Pattern injection visible in agent system prompt (verified by reading composed prompt)

**Size**: M (2 days)

---

### Task 2.4: Port post-gate reflection loop

**Description**: After gate failure, spawn a lightweight agent to analyze what went wrong.

**Source files**:
- `bardo/apps/mori/src/orchestrator/reflection.rs` — `spawn_reflection()`, haiku-4-5 analysis, reflection field in episodes
- `bardo/apps/mori/src/orchestrator/iteration_memory.rs` — Per-plan iteration history

**Target**: `crates/roko-cli/src/orchestrate.rs` (new function) + `crates/roko-learn/src/episode_logger.rs` (add field)

**Prompt**:
> Implement a post-gate reflection loop in orchestrate.rs.
> 1. **Trigger**: After any gate failure (compile, test, clippy), before replanning
> 2. **Reflection agent**: Use cheapest model (haiku-4-5). Prompt: "Analyze this gate failure. What went wrong? What should the next attempt do differently? Gate output: {error_digest}. Files changed: {file_list}. Previous attempts: {iteration_count}."
> 3. **Output**: Store reflection text in episode's `reflection` field (add this field to Episode struct if missing)
> 4. **Injection**: On retry, inject last reflection into agent's system prompt as "Lessons from previous attempt: {reflection}"
> 5. **Deduplication**: If error_digest matches a previous reflection's error pattern, skip re-generating
> 6. **Cost guard**: Reflection must cost <$0.02 (cap max_tokens at 500)
> Reference: bardo/apps/mori/src/orchestrator/reflection.rs, iteration_memory.rs

**Acceptance criteria**:
- [ ] Reflection generated on gate failure (visible in episode log)
- [ ] Reflection injected into retry agent's prompt
- [ ] Deduplication: same error pattern doesn't trigger second reflection
- [ ] Cost capped: max_tokens=500, model=haiku
- [ ] Episode struct has `reflection: Option<String>` field

**Size**: M (2-3 days)

---

### Task 2.5: Port context injection scoping

**Description**: Scope playbook rules to plan's touched files and enable per-category toggles.

**Source files**:
- `bardo/apps/mori/src/orchestrator/inject.rs` — `ContextInjector`, `KnowledgeConfig`, `collect_plan_playbook_scope()`

**Target**: `crates/roko-compose/src/context_scoping.rs` + wire into orchestrate.rs

**Prompt**:
> Port mori's context injection scoping system.
> 1. **`KnowledgeConfig`** struct with toggles:
>    - `file_intel_enabled: bool` (default true), `file_intel_max_entries: usize` (default 5)
>    - `warnings_enabled: bool`, `warning_max_entries: usize`
>    - `error_patterns_enabled: bool`, `error_pattern_min_cluster: usize`
>    - `wave_context_enabled: bool` (read context from sibling tasks in same wave)
>    - `dynamic_budget_enabled: bool` (adjust context size per file difficulty)
> 2. **`collect_plan_playbook_scope(plan, tasks) -> PlaybookScope`**: Extract file globs + tags from task checklist. Only match playbook rules whose `trigger_files` overlap with plan's file scope.
> 3. **Role-filtered context**: Different roles get different context sizes. Implementer gets full file intel. Reviewer gets summary only. Strategist gets none (sees plan-level only).
> 4. **Integration**: In orchestrate.rs `dispatch_agent_with()`, apply KnowledgeConfig to filter playbook rules and context before prompt assembly.
> Reference: bardo/apps/mori/src/orchestrator/inject.rs

**Acceptance criteria**:
- [ ] `KnowledgeConfig` loadable from `roko.toml` (with defaults)
- [ ] `collect_plan_playbook_scope()` narrows rule matching to plan's files
- [ ] Implementer gets full context; reviewer gets summary; verified by prompt inspection
- [ ] Config toggles actually suppress sections (set `file_intel_enabled=false` → no file intel in prompt)

**Size**: M (2-3 days)

---

### Task 2.6: Port warm agent spawning

**Description**: Pre-spawn agents during gate execution for faster phase transitions.

**Source files**:
- `bardo/apps/mori/src/agent/mod.rs` — `MultiAgentPool`, `pre_spawn_warm()`, `promote_warm()`, `evict_warm()`

**Target**: `crates/roko-runtime/src/warm_pool.rs` + wire into orchestrate.rs

**Prompt**:
> Port mori's warm agent spawning system.
> 1. **WarmPool**: `HashMap<AgentRole, WarmAgent>` where `WarmAgent` = pre-spawned process ready for promotion
> 2. **`pre_spawn_warm(role, effort)`**: During gate pipeline execution, spawn the next phase's agent in the background. The agent initializes but doesn't receive a task yet.
> 3. **`promote_warm(role) -> AgentConnection`**: Swap warm agent to active. The agent receives its task and starts working immediately. Saves 5-15s vs cold spawn.
> 4. **`evict_warm(role)`**: Kill warm agent on gate failure (no point keeping it if plan is replanning)
> 5. **Integration**: In orchestrate.rs, after dispatching compile gate, call `pre_spawn_warm(Reviewer)`. When gate passes, call `promote_warm(Reviewer)`. When gate fails, call `evict_warm(Reviewer)`.
> Reference: bardo/apps/mori/src/agent/mod.rs (MultiAgentPool)

**Acceptance criteria**:
- [ ] Warm agent spawns in background during gate execution
- [ ] `promote_warm()` returns usable agent connection without re-spawn delay
- [ ] `evict_warm()` kills process and frees resources
- [ ] Timing test: promote is <100ms vs 5-15s for cold spawn
- [ ] No leaked processes on gate failure path

**Size**: M (2-3 days)

---

### Task 2.7: Port conductor watchers from mori

**Description**: Battle-tested detection rules for agent stalls, loops, and resource exhaustion.

**Source files**:
- `bardo/apps/mori/src/conductor/mod.rs` (600+ LOC) — Conductor struct, 10 watchers, 3-tier interventions
- `bardo/apps/mori/src/conductor/watchers.rs` — Individual watcher implementations

**Target**: Extend `crates/roko-conductor/src/` with mori's watcher implementations

**Prompt**:
> Port mori's 10 conductor watchers into roko-conductor. Roko already has a conductor framework — extend it with these detection rules:
> 1. **GhostTurn**: No output + fast turn (<5s) + not in gating → Restart agent
> 2. **ReviewLoop**: 3+ consecutive REVISE verdicts + gates pass → Skip remaining reviews
> 3. **IterationLoop**: Iteration ≥6 + cycling strategist/implementer → Force advance
> 4. **TestFailureBudget**: 70%+ tests pass but some fail → Force advance (good enough)
> 5. **SilenceTimeout**: No output for 180s → Restart agent
> 6. **CompileFailThreshold**: 3+ consecutive compile failures → Force advance
> 7. **TaskStall**: Single task blocking for 300s → Restart agent
> 8. **ContextPressure**: Prompt >80% of context window → Trim context
> 9. **PhaseTimeout**: Phase exceeds 30min wall-clock → Restart
> 10. **CooldownFilter**: Last intervention within 120s → Skip (debounce)
> Each watcher returns `Option<Intervention { tier, watcher, target_role, message, action }>`.
> Reference: bardo/apps/mori/src/conductor/

**Acceptance criteria**:
- [ ] All 10 watchers implemented and registered in conductor
- [ ] CooldownFilter prevents intervention storms (tested with rapid triggers)
- [ ] Each watcher's threshold configurable (in `roko.toml` or conductor config)
- [ ] Interventions logged with tier/watcher/target for observability
- [ ] Unit tests for each watcher with mock ConductorContext

**Size**: L (3-4 days)

---

## Phase 3: Learning Loop Gaps

### Task 3.1: Wire neuro store into cascade router

**Description**: Consult knowledge store at dispatch time for model selection.

**Source files**:
- `roko/crates/roko-neuro/src/` — Knowledge store (existing)
- `roko/crates/roko-learn/src/cascade_router.rs` — Model routing (existing)
- `bardo/crates/golem-grimoire/src/` — Grimoire retrieval scoring (reference)

**Target**: `crates/roko-learn/src/cascade_router.rs`

**Prompt**:
> Wire the neuro knowledge store into cascade_router::decide().
> Currently, the cascade router selects models based on observations (pass/fail history) but does NOT consult the neuro store. The grimoire in bardo queries episodic memory for context at dispatch time.
> 1. At `decide()` time, query `knowledge_store.query(task_description, limit=3)` for relevant prior knowledge
> 2. If knowledge entries mention specific model preferences (e.g., "opus handles X better"), bias model scoring by +0.1 for mentioned model
> 3. If knowledge entries describe failure patterns with a model, bias by -0.1
> 4. Add knowledge context to LinUCB feature vector (add 2 dims: `knowledge_match_score`, `knowledge_model_bias`)
> 5. Make this opt-in via `cascade_router.consult_knowledge: bool` in config (default true)
> Reference: roko/crates/roko-neuro/src/knowledge_store.rs, roko/crates/roko-learn/src/cascade_router.rs

**Acceptance criteria**:
- [ ] Cascade router queries neuro store at decide time
- [ ] Model bias applied based on knowledge entries
- [ ] LinUCB context vector extended with knowledge features
- [ ] Config toggle works (disabled = no knowledge query)
- [ ] No performance regression: knowledge query <10ms (cached or fast path)

**Size**: M (2 days)

---

### Task 3.2: Port episode clustering for error patterns

**Description**: Cluster failed episodes by error signature to recommend model fallbacks.

**Source files**:
- `bardo/apps/mori/src/orchestrator/pattern_learning.rs` — `cluster_episodes()`, `EpisodeCluster`

**Target**: Extend `crates/roko-learn/src/pattern_discovery.rs`

**Prompt**:
> Extend roko's pattern_discovery.rs with episode clustering from mori.
> 1. **`cluster_episodes(episodes: &[Episode]) -> Vec<EpisodeCluster>`**: Group by `error_signature` (failures) or `file_pattern` (successes). Minimum cluster size: 3.
> 2. **`EpisodeCluster`**: `{ key: String, count: usize, success_rate: f64, common_files: Vec<String>, best_model: String, best_provider: String, avg_cost_usd: f64 }`
> 3. **Model recommendation**: Per cluster, compute which model has highest success_rate. Store as `recommended_model` field.
> 4. **Integration**: Feed cluster recommendations into cascade_router as soft priors (not hard overrides). When a new task matches a cluster's file pattern, bias toward recommended_model.
> 5. **Cadence**: Run clustering every 10 new episodes (use existing `UpdateFrequency` mechanism)
> Reference: bardo/apps/mori/src/orchestrator/pattern_learning.rs (cluster_episodes, EpisodeCluster)

**Acceptance criteria**:
- [ ] `cluster_episodes()` groups episodes with matching error signatures
- [ ] Clusters with 3+ episodes produce model recommendations
- [ ] Recommendations integrated as soft bias in cascade_router
- [ ] Clustering runs on cadence (every 10 episodes)
- [ ] Test: 5 episodes with same error + model A succeeding → recommends model A

**Size**: M (2-3 days)

---

### Task 3.3: Port provider pass-rate into model scoring

**Description**: Bias model selection toward proven providers.

**Source files**:
- `bardo/apps/mori/src/orchestrator/pattern_learning.rs` — `compute_provider_metrics()`, `recommend_provider()`
- `roko/crates/roko-learn/src/provider_health.rs` — Existing provider health tracker

**Target**: `crates/roko-learn/src/cascade_router.rs`

**Prompt**:
> Integrate provider pass-rate into cascade router scoring.
> 1. `compute_provider_metrics(episodes)` → per-provider: pass_rate, avg_cost, avg_duration (min 5 episodes)
> 2. `recommend_provider(metrics)` → pick provider with highest pass_rate
> 3. In cascade_router Stage 2 (confidence) and Stage 3 (LinUCB): multiply model score by `provider_pass_rate` for the model's provider (so models from reliable providers score higher)
> 4. Use existing ProviderHealthTracker data if available, fall back to episode-derived metrics
> Reference: bardo/apps/mori/src/orchestrator/pattern_learning.rs, roko/crates/roko-learn/src/provider_health.rs

**Acceptance criteria**:
- [ ] Provider metrics computed from episode history
- [ ] Model scores multiplied by provider pass_rate in stages 2-3
- [ ] Provider with 0.9 pass_rate boosts its models vs provider with 0.6
- [ ] Minimum 5 episodes before provider metrics affect scoring
- [ ] Unit test verifies scoring bias

**Size**: S (1 day)

---

### Task 3.4: Port reflection-derived playbook rules

**Description**: Auto-generate playbook rules from agent reflections.

**Source files**:
- `bardo/apps/mori/src/orchestrator/pattern_learning.rs` — `build_reflection_playbook_rules()`

**Target**: Extend `crates/roko-learn/src/playbook_rules.rs`

**Prompt**:
> Extend roko's playbook system to auto-generate rules from agent reflections (Task 2.4).
> 1. After reflection is stored in episode, extract actionable patterns:
>    - If reflection mentions specific files → create rule with `trigger_files` glob
>    - If reflection mentions error type → create rule with `trigger_tags`
>    - Context injection = the reflection's key insight
> 2. **Confidence**: New rules start at 0.5 (neutral). Boost +0.05 on gate pass, penalize -0.10 on gate fail. Remove rules below 0.2 confidence (unless manually created).
> 3. **Cadence**: Run after every 3 new reflections (PLAYBOOK_REFRESH_NEW_EPISODE_FLOOR from mori)
> 4. **Persistence**: Append to `.roko/learn/playbook-rules.json` with `source: "reflection"` tag
> Reference: bardo/apps/mori/src/orchestrator/pattern_learning.rs (build_reflection_playbook_rules)

**Acceptance criteria**:
- [ ] Reflections with file mentions → playbook rules with trigger_files
- [ ] Confidence tracking: +0.05 on success, -0.10 on failure
- [ ] Rules below 0.2 auto-removed
- [ ] Manually created rules preserved (never auto-removed)
- [ ] Persistence in playbook-rules.json with `source: "reflection"` tag

**Size**: M (2 days)

---

### Task 3.5: Port A-MAC admission gate for neuro store

**Description**: Prevent hallucinated or contradictory knowledge from entering the store.

**Source files**:
- `bardo/crates/golem-grimoire/src/` — A-MAC 5-factor admission gate (similarity, novelty, contradiction, relevance, confidence)

**Target**: Extend `crates/roko-neuro/src/`

**Prompt**:
> Add an A-MAC (Anti-Misinformation Admission Control) gate to roko-neuro's knowledge store.
> Before any knowledge entry is stored, validate against 5 factors:
> 1. **Similarity**: Is this too similar to existing knowledge? (cosine sim > 0.95 → reject as duplicate)
> 2. **Novelty**: Does this add new information? (cosine sim < 0.3 to all existing → novel, good)
> 3. **Contradiction**: Does this contradict existing high-confidence entries? (semantic opposition check)
> 4. **Relevance**: Is this relevant to the agent's domain? (keyword match against domain tags)
> 5. **Confidence**: Does the source have sufficient credibility? (gate pass rate of the episode that generated this)
> Gate result: `Admit | Reject { reason }`. Log rejections for debugging.
> Reference: bardo/crates/golem-grimoire/src/ (A-MAC admission)

**Acceptance criteria**:
- [ ] Near-duplicate entries rejected (similarity > 0.95)
- [ ] Contradictory entries flagged (if existing entry has confidence > 0.8)
- [ ] Novel entries admitted with appropriate confidence score
- [ ] Rejections logged with reason
- [ ] Unit test: insert duplicate → rejected; insert novel fact → admitted; insert contradiction → flagged

**Size**: M (2-3 days)

---

## Phase 4: Heartbeat Pipeline

### Task 4.1: Port 9-step TickPipeline

**Description**: Full CoALA-style heartbeat tick for persistent agents.

**Source files**:
- `bardo/crates/golem-heartbeat/src/pipeline.rs` (3,019 LOC) — 9-step pipeline
- `bardo/crates/golem-heartbeat/src/engine.rs` (1,307 LOC) — HeartbeatEngine

**Target**: `crates/roko-conductor/src/tick_pipeline.rs`

**Prompt**:
> Port the 9-step CoALA tick pipeline from golem-heartbeat.
> Each step is a trait method on `TickStep`:
> 1. **Observe**: Poll external state (chain events, file changes, webhook payloads)
> 2. **Retrieve**: Query neuro store for relevant prior knowledge
> 3. **Analyze**: Parse observations + knowledge → decision signals
> 4. **Gate**: Check prediction error vs adaptive threshold → select T0/T1/T2
> 5. **Simulate**: Local simulation of proposed action (revm for chain, dry-run for code)
> 6. **Validate**: Safety checks + cost estimation + capability verification
> 7. **Execute**: Commit action (LLM call for T1/T2, or deterministic for T0)
> 8. **Verify**: Confirm outcome matches prediction (if applicable)
> 9. **Reflect**: Update adaptive clock, log episode, adjust thresholds
> Also port `HeartbeatEngine` which orchestrates the pipeline and manages `AdaptiveClock` with gamma/theta/delta cadences.
> Key: Steps 1-4 + 8-9 always run. Steps 5-7 only run if tier > T0.
> Reference: bardo/crates/golem-heartbeat/src/pipeline.rs, engine.rs

**Acceptance criteria**:
- [ ] All 9 steps defined as trait methods
- [ ] Pipeline short-circuits at step 4 for T0 (skips 5-7)
- [ ] T1 uses cheap model (haiku); T2 uses full model (opus/sonnet)
- [ ] AdaptiveClock adjusts gamma/theta/delta intervals based on prediction error
- [ ] Full pipeline cycle completes in <100ms for T0, <5s for T2
- [ ] Episode logged per tick with tier, prediction error, outcome

**Size**: L (4-5 days)

---

### Task 4.2: Wire T0/T1/T2 at dispatch time

**Description**: Enforce cognitive tier gating before agent dispatch.

**Source files**:
- `bardo/crates/golem-heartbeat/src/gating.rs` (481 LOC) — DailyCostAccumulator, GasGate, AdaptiveGate, PredictionError
- `roko/crates/roko-primitives/src/tier.rs` — Existing TierRouter (T0/T1/T2 types)

**Target**: `crates/roko-cli/src/orchestrate.rs` (modify `dispatch_agent_with()`)

**Prompt**:
> Wire tier enforcement into orchestrate.rs dispatch_agent_with().
> Before every agent dispatch, evaluate 3 gates (from bardo's gating.rs):
> 1. **DailyCostAccumulator**: If daily_cost_usd > max_daily_budget → force T0 (no inference)
> 2. **GasGate** (chain agents only): If last base_fee > 2× EMA base_fee → force T0
> 3. **AdaptiveGate**: Based on prediction_error EMA:
>    - error_rate < 0.1 → T0 sufficient (deterministic response)
>    - error_rate 0.1-0.25 → T1 (haiku)
>    - error_rate > 0.25 → T2 (full reasoning)
> 4. Use existing TierRouter in roko-primitives for model selection per tier
> 5. Add `PredictionError` tracking: rolling window of last 50 errors, EMA with alpha=0.1
> 6. Config: `[heartbeat] max_daily_budget_usd = 50.0`, `[heartbeat] gas_spike_multiplier = 2.0`
> Reference: bardo/crates/golem-heartbeat/src/gating.rs, roko/crates/roko-primitives/src/tier.rs

**Acceptance criteria**:
- [ ] Daily budget enforcement: after budget hit, all dispatches go T0
- [ ] Prediction error tracking: EMA updates after each task
- [ ] Tier selection matches error rate thresholds
- [ ] Config values loaded from roko.toml
- [ ] Tier logged in episode for observability
- [ ] Existing behavior preserved when heartbeat config is absent (default: always T2)

**Size**: M (2-3 days)

---

## Phase 5: Agent Modes + Profiles

### Task 5.1: Add AgentMode and AgentProfile enums

**Description**: Define the three agent lifecycle modes and five agent profiles.

**Target**: `crates/roko-core/src/config/schema.rs`

**Prompt**:
> Add agent mode and profile types to roko-core config schema.
> 1. **AgentMode** enum: `Persistent` (runs until stopped, heartbeat loop), `Ephemeral` (one task then stops), `Reactive` (sleeps, wakes on webhook/cron trigger)
> 2. **AgentProfile** enum: `Coding` (repo, task, runs tests, opens PRs), `Research` (web search, deep research, enhance PRDs), `Blockchain` (monitors chain, rebalances, alerts), `Security` (audits code, scans vulns), `Custom` (user-defined)
> 3. **ProfileDefaults** per profile: `default_model`, `default_tools: Vec<String>`, `heartbeat_interval: Duration`, `budget_cap_usd: f64`, `extensions: Vec<String>`
> 4. Wire into `CreateAgentRequest` in roko-serve/src/routes/agents.rs
> 5. Add `[agent.profiles.coding]`, `[agent.profiles.research]` etc. to roko.toml schema for customization
> These types must be Serialize/Deserialize/Clone/Debug and used everywhere agent config appears.

**Acceptance criteria**:
- [ ] `AgentMode` and `AgentProfile` compile and serialize to JSON/TOML
- [ ] `ProfileDefaults::for_profile(profile)` returns sensible defaults
- [ ] `CreateAgentRequest` accepts optional `mode` and `profile` fields
- [ ] Defaults applied when fields are omitted
- [ ] Config schema accepts `[agent.profiles.*]` sections

**Size**: S (1 day)

---

### Task 5.2: Wire ephemeral auto-stop

**Description**: When `mode == Ephemeral`, agent auto-stops after task completion.

**Target**: `crates/roko-serve/src/routes/agents.rs` + `crates/roko-runtime/src/process.rs`

**Prompt**:
> Wire ephemeral auto-stop into agent lifecycle.
> 1. In `start_agent()`: if mode is Ephemeral, register a completion callback
> 2. ProcessSupervisor detects process exit → check mode → if Ephemeral, mark agent as "completed" (not "failed")
> 3. Clean up resources (temp volumes, process entries) but preserve logs and episodes
> 4. Agent status transitions: Created → Running → Completed (not Stopped)
> 5. Dashboard shows ephemeral agents with "completed" badge after task finishes
> Reference: Agent lifecycle in roko-serve/src/routes/agents.rs, ProcessSupervisor in roko-runtime/src/process.rs

**Acceptance criteria**:
- [ ] Ephemeral agent auto-stops when spawned process exits normally
- [ ] Status = "completed" (not "stopped" or "failed")
- [ ] Logs and episodes preserved after completion
- [ ] Process resources freed (no zombie processes)
- [ ] `GET /api/agents` shows completed ephemeral agents with correct status

**Size**: S (1 day)

---

### Task 5.3: Wire reactive mode (webhook/cron triggers)

**Description**: Agents that sleep until triggered by webhooks or cron schedules.

**Source files**:
- `roko/crates/roko-serve/src/routes/webhooks.rs` — Existing webhook routes
- `roko/crates/roko-serve/src/routes/subscriptions.rs` — Existing subscription catalog (advertises cron/file_watch but doesn't implement runtime)

**Target**: `crates/roko-runtime/src/reactive.rs` + wire into agents.rs

**Prompt**:
> Implement reactive agent mode.
> 1. **Reactive agent state**: Created → Sleeping (no compute) → Waking → Running → Sleeping (cycle)
> 2. **Webhook trigger**: Existing webhook routes (`POST /webhooks/github`, `/webhooks/slack`) already parse events and publish to event bus. Add: event bus subscriber that checks if event matches a reactive agent's subscription filter → wake agent
> 3. **Cron trigger**: Add `tokio-cron-scheduler` or equivalent. Parse cron expression from agent config. On tick → wake agent.
> 4. **Wake flow**: Start agent process via ProcessSupervisor → agent processes event → agent exits → supervisor puts agent back to Sleeping
> 5. **Config**: `{ mode: "reactive", triggers: [{ type: "webhook", filter: { repo: "org/repo" } }, { type: "cron", schedule: "0 */6 * * *" }] }`
> Reference: roko-serve/src/routes/webhooks.rs, subscriptions.rs

**Acceptance criteria**:
- [ ] Reactive agent created without starting a process (Sleeping state)
- [ ] Webhook event matching agent's filter wakes the agent
- [ ] Cron schedule triggers agent on time
- [ ] Agent returns to Sleeping after processing
- [ ] No compute cost while sleeping (no process running)
- [ ] `GET /api/agents/:id` shows correct lifecycle state

**Size**: L (3-4 days)

---

## Phase 6: Dashboard Integration

### Task 6.1: Set up Next.js app in monorepo

**Source files**:
- `bardo/apps/dashboard/` — Full Next.js app
- `bardo/packages/ui/` — Shared component library

**Target**: `apps/dashboard/`, `packages/ui/`

**Prompt**:
> Copy the Next.js dashboard from bardo into roko monorepo.
> 1. Copy `bardo/apps/dashboard/` → `roko/apps/dashboard/`
> 2. Copy `bardo/packages/ui/` → `roko/packages/ui/`
> 3. Update `next.config.ts`: change basePath to `/dashboard`, update API rewrites from `/v1/*` to `/api/gateway/*` (to match roko-serve routes)
> 4. Update `package.json`: change name to `@roko/dashboard`, update `@bardo/ui` → `@roko/ui`
> 5. Add `pnpm-workspace.yaml` at repo root with `packages: ['apps/*', 'packages/*']`
> 6. Run `pnpm install` and verify `pnpm --filter @roko/dashboard dev` starts
> Reference: bardo/apps/dashboard/, bardo/packages/ui/

**Acceptance criteria**:
- [ ] `apps/dashboard/` exists with working Next.js app
- [ ] `packages/ui/` exists with shared component library
- [ ] `pnpm --filter @roko/dashboard dev` starts on localhost:3000
- [ ] Dashboard renders (even without backend — shows "connecting" state)
- [ ] No references to "bardo" in source code (renamed to "roko" or "nunchi")

**Size**: S (1 day)

---

### Task 6.2: Add Privy auth to dashboard

**Target**: `apps/dashboard/src/app/login/`, `apps/dashboard/src/components/AuthProvider.tsx`

**Prompt**:
> Add Privy authentication to the roko dashboard.
> 1. Install `@privy-io/react-auth`
> 2. Create `AuthProvider` wrapper component:
>    - Wraps app in `<PrivyProvider appId={process.env.NEXT_PUBLIC_PRIVY_APP_ID}>`
>    - Login methods: email, google, apple
>    - `createOnLogin: 'all-users'` (auto-create embedded wallet)
> 3. Create login page at `/login`:
>    - If not authenticated → show Privy login modal
>    - If authenticated → redirect to dashboard
>    - Store Privy JWT in localStorage as `nunchi_auth_token`
> 4. Add auth guard on all pages:
>    - Check `usePrivy()` authenticated state
>    - If not authenticated → redirect to `/login`
> 5. Add `Authorization: Bearer <jwt>` to all API calls (update useGateway hook)
> 6. Make Privy optional: if `NEXT_PUBLIC_PRIVY_APP_ID` not set, skip auth entirely (local dev mode)
> Reference: roko architecture redesign doc (Authentication section)

**Acceptance criteria**:
- [ ] Login page renders Privy modal
- [ ] Successful login stores JWT and redirects to dashboard
- [ ] API calls include Bearer token
- [ ] Without PRIVY_APP_ID env var, auth is skipped (local dev)
- [ ] Logout button in header clears session

**Size**: M (2-3 days)

---

### Task 6.3: Add agent management pages

**Target**: `apps/dashboard/src/app/agents/`

**Prompt**:
> Create agent management pages for the dashboard.
> 1. **Agent list page** (`/agents`):
>    - Card grid showing all agents from `GET /api/agents`
>    - Per card: name, profile badge, mode badge, status indicator, tier (T0/T1/T2), cost/hr, uptime
>    - [+ Create Agent] button → creation wizard
>    - [Stop] / [Restart] actions per agent
> 2. **Agent detail page** (`/agents/:id`):
>    - Header: name, status, profile, mode, execution tier, uptime, total cost
>    - Heartbeat timeline: horizontal strip showing T0/T1/T2 ticks over time (like bardo)
>    - Logs panel: scrollable, filterable live logs from `/api/agents/:id/logs`
>    - Episodes panel: recent episodes with success/fail, model used, cost
>    - [Stop] [Restart] [View Full Trace] buttons
> 3. **Agent creation wizard** (`/agents/new`):
>    - Step 1: Choose profile (Coding/Research/Blockchain/Security/Custom) — card selector
>    - Step 2: Configure (mode, task/strategy, model preference, budget cap, heartbeat interval)
>    - Step 3: Review + Launch
>    - POST to `/api/agents/create` on submit
> Use existing @roko/ui components (Panel, Kpi, Badge).
> Reference: Architecture redesign doc (Dashboard Layout section, Agent Creation wizard section)

**Acceptance criteria**:
- [ ] Agent list page renders all agents from API
- [ ] Agent detail page shows live status, heartbeat timeline, logs
- [ ] Creation wizard completes 3 steps and creates agent via API
- [ ] Stop/restart buttons work and update UI
- [ ] Responsive layout on desktop (1200px+)

**Size**: L (4-5 days)

---

### Task 6.4: Add settings page

**Target**: `apps/dashboard/src/app/settings/`

**Prompt**:
> Create settings page for the dashboard.
> 1. **Provider Keys** section:
>    - List all providers (Anthropic, Perplexity, Gemini, Moonshot, ZAI, OpenRouter, Ollama)
>    - Per provider: status indicator (connected/not set), [Add Key] / [Test] / [Update] / [Remove] buttons
>    - Test button calls `POST /api/secrets/:namespace/:key/test`
>    - Add key calls `POST /api/secrets/:namespace/:key` with value from input
> 2. **API Keys** section:
>    - List existing keys from `GET /api/api-keys` (metadata only)
>    - [+ Create Key] form: name, scope dropdown (admin/agent:write/read), expiry
>    - Shows key value ONCE on creation (copy button)
>    - [Revoke] button per key
> 3. **Account** section (if Privy):
>    - Profile from Privy (email, avatar)
>    - Wallet address (embedded wallet)
> Match the dashboard UI spec from the architecture redesign doc.
> Reference: Architecture redesign doc (Settings page section, Secret Management section)

**Acceptance criteria**:
- [ ] Provider keys page lists all supported providers
- [ ] Test button returns connected/invalid/error status
- [ ] API key creation shows value once (never again)
- [ ] Revoke removes key from list
- [ ] Account section shows Privy user info (if authenticated)

**Size**: M (3 days)

---

## Phase 7: DeFi Tools + Chain Runtime

### Task 7.1: Port ToolExecutor framework

**Source files**:
- `bardo/crates/golem-tools/src/executor.rs` — Full validation pipeline
- `bardo/crates/golem-tools/src/types.rs` — ToolDef, ToolContext, ToolResult, ToolCategory (17), ToolProfile (14)
- `bardo/crates/golem-tools/src/profiles.rs` — 72-tool Observatory allowlist, PolicyCage
- `bardo/crates/golem-tools/src/safety.rs` — CapabilitySpendTracker
- `bardo/crates/golem-tools/src/resilience.rs` — RateLimiter, CircuitBreaker
- `bardo/crates/golem-tools/src/config.rs` — SafetyConfig, RateLimitConfig

**Target**: `crates/roko-std/src/tool_executor.rs` (or new crate `crates/roko-tools/`)

**Prompt**:
> Port bardo's ToolExecutor framework. roko-std currently has 19 builtin tools with a simple executor. This adds capability-gated execution with safety enforcement.
> 1. **ToolDef**: name, description, category (17 variants), capability (Read/Write/Privileged), risk_tier (Layer1/2/3), tick_budget (Fast/Medium/Slow), requires_simulation, supported_chains, prompt_snippet
> 2. **ToolContext**: chain_provider, agent_id, tier, budget_remaining, capability token, audit_chain, mirage_client, tool_profile
> 3. **ToolExecutor pipeline**: Registry lookup → PolicyCage check → Capability validation → Rate limiter → Circuit breaker → Simulation gate → Dispatch → Audit append
> 4. **14 ToolProfiles**: Active (default), Observatory (72-tool read-only), Conservative, Trader, LP, Vault, VaultCurator, Intelligence, Learning, Identity, Full, Development, Evaluation, Minimal
> 5. **SafetyConfig**: max_value_per_tx ($10K), per_tick ($50K), per_day ($100K), max_slippage (1%), max_price_impact (3%), min_health_factor (1.2)
> 6. **Feature gates**: Cargo features per category (data, trading, lp, vault, safety, etc.)
> Reference: bardo/crates/golem-tools/src/ (all files listed above)

**Acceptance criteria**:
- [ ] All 17 ToolCategory variants defined
- [ ] All 14 ToolProfile variants with allowlists
- [ ] ToolExecutor pipeline enforces all 7 checks in order
- [ ] PolicyCage blocks Write tools in Observatory profile
- [ ] CapabilitySpendTracker enforces per-tx/tick/day limits
- [ ] Feature gates compile correctly (vault disabled by default)
- [ ] `cargo check --features vault,identity` passes

**Size**: XL (5-7 days)

---

### Task 7.2: Port vault tools (ERC-4626)

**Source files**:
- `bardo/crates/golem-tools/src/tools/vault/` — 32 tool definitions + stubbed handlers
- `bardo/crates/golem-tools/src/tools/vault/types.rs` — VaultCreateParams, VaultInfo, ExecutorPerformanceMetrics, etc.

**Target**: `crates/roko-std/src/tools/vault/` (or `crates/roko-tools/src/tools/vault/`)

**Prompt**:
> Port bardo's 32 vault tool definitions and types.
> Categories: lifecycle (12: create, deposit, withdraw, rebalance, list, info, snapshot, set_strategy, get_strategy, pause, unpause, emergency_withdraw), share accounting (7: approve, transfer, redeem, max_deposit, max_withdraw, preview_deposit, preview_redeem), proxy (6: set, revoke, list, delegate_vault, get_delegates, delegate_withdraw), executor (6: register, list, submit, status, cancel, performance).
> Port the ToolDef definitions with correct capability/risk/tick_budget/requires_simulation flags. Port all types from vault/types.rs. Wire into dispatch match statement under `#[cfg(feature = "vault")]` feature gate.
> Handlers can remain stubs (return error with "not implemented" message) — the definitions and types are what matter for now.
> Reference: bardo/crates/golem-tools/src/tools/vault/

**Acceptance criteria**:
- [ ] All 32 vault tools defined in registry
- [ ] Dispatch routes to each tool (even if handler is stubbed)
- [ ] Types (VaultCreateParams, VaultInfo, etc.) compile and serialize
- [ ] Feature-gated: only compiled when `vault` feature is enabled
- [ ] Tool profile filtering works (Vault profile allows vault tools, Observatory blocks them)

**Size**: L (3-4 days)

---

### Task 7.3: Port ProviderPool + SubgraphClient

**Source files**:
- `bardo/crates/golem-chain/src/provider.rs` — Alloy HTTP pool, moka cache, SubgraphClient

**Target**: Extend `crates/roko-chain/src/`

**Prompt**:
> Port bardo's chain ProviderPool and SubgraphClient into roko-chain.
> 1. **ProviderPool**: `HashMap<ChainId, Arc<HttpProvider>>` with lazy initialization from env var RPC URLs. Moka LRU cache (100 entries, 5min TTL). Cache keys: `EthCall { chain_id, to, data_hash }`, `GetLogs { chain_id, from_block, to_block, address, topic0 }`, `AccountBalance { chain_id, address, block }`, `StorageAt { chain_id, address, slot, block }`.
> 2. **SubgraphClient**: HTTP client for The Graph gateway. Dual cache: pool data (15s TTL) + metadata (5min TTL). Auto-pagination with first/skip. Query method: `query(query_str, variables) -> serde_json::Value`.
> 3. **12 supported networks**: Ethereum, Polygon, Arbitrum, Optimism, Base, Avalanche, Scroll, Blast, Celo, Gnosis, Sepolia, Goerli. Per-chain: RPC URL env var name, key contract addresses.
> roko-chain already has some alloy types — extend, don't duplicate.
> Reference: bardo/crates/golem-chain/src/provider.rs

**Acceptance criteria**:
- [ ] `ProviderPool::get(chain_id)` returns alloy HTTP provider
- [ ] Cache reduces RPC calls (test with mock provider counting calls)
- [ ] SubgraphClient queries The Graph with pagination
- [ ] All 12 chains registered with env var names
- [ ] `cargo test -p roko-chain` passes

**Size**: L (3-4 days)

---

### Task 7.4: Port Warden time-delay safety

**Source files**:
- `bardo/crates/golem-chain/src/warden.rs` (370 LOC)

**Target**: `crates/roko-chain/src/warden.rs`

**Prompt**:
> Port bardo's Warden time-delay safety mechanism.
> Actions that modify on-chain state must be announced, wait a delay, then executed:
> 1. **ActionType** enum with delays: PoolParameterUpdate (3600s), VaultRebalance (1800s), OrderCancel (300s), LargeSwap { threshold_usd } (600s), CrossChainBridge (7200s), Custom(String)
> 2. **WardenStatus** FSM: Announced → Waiting → Ready → Executed/Cancelled (terminal)
> 3. **API**: `announce(action_type, chain_id) -> Uuid`, `poll() -> Vec<Uuid>` (returns Ready actions), `execute(action_id) -> Result<()>`, `cancel(action_id) -> Result<()>`
> 4. Actions stored in `HashMap<Uuid, WardenAction>` with timestamps
> Reference: bardo/crates/golem-chain/src/warden.rs

**Acceptance criteria**:
- [ ] `announce()` creates action in Announced state
- [ ] Action transitions through Waiting → Ready after delay elapses
- [ ] `poll()` returns only Ready actions
- [ ] `execute()` only works on Ready actions
- [ ] `cancel()` works on non-terminal states
- [ ] Unit test: announce → sleep(delay) → poll returns action → execute succeeds

**Size**: S (1 day)

---

## Phase 8: TUI Enhancements

### Task 8.1: Add DaimonState visualization

**Source files**:
- `bardo/apps/bardo-terminal/src/screens/` — Emotions, Vitality screens
- `roko/crates/roko-daimon/src/` — DaimonState already loaded in orchestrate.rs

**Target**: `crates/roko-cli/src/tui/views/` (new view)

**Prompt**:
> Add a DaimonState (affect engine) visualization to the roko TUI.
> 1. New sub-view in Dashboard (F1) or new tab (F11 Affect):
>    - PAD vector display: Pleasure [-1,1], Arousal [-1,1], Dominance [-1,1] as horizontal gauges
>    - Current PadRegion label (Exuberant/Dependent/Relaxed/Docile/Hostile/Anxious/Disdainful/Bored)
>    - Somatic marker histogram: last 10 markers with valence coloring
>    - Behavioral bias indicators: which biases are active (AvoidTrade, SeekSafety, etc.)
> 2. Data source: DaimonState is already loaded in orchestrate.rs — pipe it through DashboardEvent to TUI state
> 3. Visual style: Use existing braille sparklines and gauge widgets
> Reference: bardo/apps/bardo-terminal/src/screens/ (emotions, vitality)

**Acceptance criteria**:
- [ ] PAD gauges render with correct values from DaimonState
- [ ] PadRegion label updates in real-time
- [ ] Somatic markers visible with positive (green) / negative (red) coloring
- [ ] View accessible via F-key or tab navigation

**Size**: M (2 days)

---

### Task 8.2: Add heartbeat status view to Learning tab

**Source files**:
- `bardo/apps/bardo-terminal/src/screens/heartbeat_status.rs`

**Target**: `crates/roko-cli/src/tui/views/learning_view.rs` (extend)

**Prompt**:
> Add a heartbeat status sub-view to the Learning tab (F10).
> 1. **Accuracy sparkline**: Rolling sparkline of prediction accuracy (correct/total per window)
> 2. **Recent predictions**: Table of last 10 prediction/outcome pairs with model, tier, cost
> 3. **Tier distribution**: Bar chart showing T0/T1/T2 tick percentages
> 4. **Cost trend**: Sparkline of per-tick cost over last hour
> 5. Data source: Efficiency events JSONL + episodes JSONL (already tailed by TUI)
> Reference: bardo/apps/bardo-terminal/src/screens/heartbeat_status.rs

**Acceptance criteria**:
- [ ] Accuracy sparkline updates as new episodes arrive
- [ ] Tier distribution shows percentage of T0/T1/T2 ticks
- [ ] Cost trend sparkline renders
- [ ] Accessible as sub-view within F10 (Learning tab)

**Size**: S (1 day)

---

### Task 8.3: Enhance Knowledge browser in Inspect tab

**Source files**:
- `bardo/apps/bardo-terminal/src/screens/knowledge.rs` — Grimoire stats, top-confidence entries

**Target**: `crates/roko-cli/src/tui/views/` (extend inspect view)

**Prompt**:
> Enhance the Knowledge sub-view in Inspect tab (F7).
> 1. **Store stats**: Total entries, per-tier counts (Tier1/2/3/Archive), health percentage
> 2. **Top entries**: Scrollable list of highest-confidence entries with: title, confidence, tier, last_accessed, type label
> 3. **Decay visualization**: Time-since-access color gradient (green=fresh, yellow=aging, red=decaying)
> 4. **Distillation events**: Recent distillation timeline (when knowledge was summarized/compressed)
> Data source: roko-neuro knowledge store API
> Reference: bardo/apps/bardo-terminal/src/screens/knowledge.rs

**Acceptance criteria**:
- [ ] Store stats render (entry counts, tier distribution)
- [ ] Top entries scrollable with confidence and tier info
- [ ] Decay visualization uses color gradients

**Size**: S (1 day)

---

## Phase 9: Operational Infrastructure

### Task 9.1: Create justfile for developer convenience

**Source files**:
- `bardo/justfile` (136 lines) — build, test, lint, fmt, coverage, docs, watch, release, CI

**Target**: `justfile` at repo root

**Prompt**:
> Create a justfile at the roko repo root with common development commands:
> ```
> build         := cargo build --workspace
> test          := cargo test --workspace
> lint          := cargo clippy --workspace --no-deps -- -D warnings
> fmt           := cargo +nightly fmt --all
> fmt-check     := cargo +nightly fmt --all -- --check
> check         := cargo check --workspace
> ci            := fmt-check && lint && test
> coverage      := cargo llvm-cov --workspace --html
> watch         := cargo watch -x 'check --workspace'
> deny          := cargo deny check
> doc           := cargo doc --workspace --no-deps
> clean         := cargo clean
> serve         := cargo run -p roko-cli -- serve
> dashboard     := cargo run -p roko-cli -- dashboard
> run           := cargo run -p roko-cli --
> ```
> Reference: bardo/justfile

**Acceptance criteria**:
- [ ] `just ci` runs fmt-check + lint + test
- [ ] `just serve` starts the server
- [ ] `just dashboard` starts the TUI
- [ ] All shortcuts work from repo root

**Size**: S (half day)

---

### Task 9.2: Create E2E test harness

**Source files**:
- `bardo/tests/harness/src/lib.rs` — BardoTestHarness, HealthReport, TerminalProbe

**Target**: `tests/harness/`

**Prompt**:
> Create an E2E test harness for multi-component integration tests.
> 1. **RokoTestHarness** struct: Manages spawning roko-serve + mirage-rs as child processes
> 2. **spawn_serve(config) -> ServerHandle**: Start roko-serve on random port, wait for health check
> 3. **spawn_mirage(config) -> MirageHandle**: Start mirage-rs on random port
> 4. **health_check(url) -> HealthReport**: Poll `/api/health` until ready or timeout
> 5. **cleanup()**: Kill all child processes on Drop (no leaked processes)
> 6. Add to workspace as `[dev-dependencies]` for integration tests
> Reference: bardo/tests/harness/src/lib.rs

**Acceptance criteria**:
- [ ] `RokoTestHarness::new()` spawns serve + mirage
- [ ] Health check waits up to 30s for services to be ready
- [ ] Drop impl kills all child processes
- [ ] Integration test using harness passes: spawn → health check → stop

**Size**: M (2 days)

---

### Task 9.3: Port self-healing supervisor script

**Source files**:
- `bardo/bardo-supervisor.sh` (381 LOC) — Crash recovery, error signature extraction, AI-driven auto-fix

**Target**: `scripts/roko-supervisor.sh`

**Prompt**:
> Port bardo's self-healing supervisor as a shell script for production deployments.
> 1. **Crash detection**: Monitor roko process exit code. On non-zero exit, extract panic signature from stderr.
> 2. **Error deduplication**: Track error signatures in `/tmp/roko-supervisor-errors.json`. Skip auto-fix for already-seen errors.
> 3. **Auto-fix** (optional, requires Claude CLI): Feed crash report + recent logs to Claude for diagnosis. Apply suggested fix. Restart.
> 4. **Circuit breaker**: After 3 consecutive restarts within 5 minutes, stop trying (prevent infinite loops). Alert via stderr.
> 5. **Signal handling**: Forward SIGTERM/SIGINT to child process. Clean shutdown.
> 6. **Configurable**: `ROKO_SUPERVISOR_MAX_RESTARTS=3`, `ROKO_SUPERVISOR_WINDOW_SECS=300`, `ROKO_SUPERVISOR_AUTOFIX=false`
> Reference: bardo/bardo-supervisor.sh

**Acceptance criteria**:
- [ ] Script restarts roko on crash
- [ ] Error signatures deduplicated (same crash doesn't trigger multiple auto-fix attempts)
- [ ] Circuit breaker stops after N restarts in window
- [ ] SIGTERM forwarded to child process
- [ ] Works without Claude CLI (autofix disabled by default)

**Size**: M (1-2 days)

---

## Phase 10: Fly Machines (Isolated Execution)

### Task 10.1: Fly Machines REST API client

**Target**: `crates/roko-runtime/src/fly.rs`

**Prompt**:
> Implement a Fly Machines REST API client.
> API base: `https://api.machines.dev/v1`
> Auth: `Authorization: Bearer {FLY_API_TOKEN}`
> Methods:
> 1. `create_machine(app, config) -> MachineId` — POST `/apps/{app}/machines`
>    Config: image, guest (cpus, memory_mb), env vars, volumes, services
> 2. `start_machine(app, machine_id)` — POST `/apps/{app}/machines/{id}/start`
> 3. `stop_machine(app, machine_id)` — POST `/apps/{app}/machines/{id}/stop`
> 4. `destroy_machine(app, machine_id)` — DELETE `/apps/{app}/machines/{id}`
> 5. `get_machine(app, machine_id) -> MachineStatus` — GET `/apps/{app}/machines/{id}`
> 6. `wait_for_state(app, machine_id, state, timeout)` — GET `/apps/{app}/machines/{id}/wait?state={state}&timeout={timeout}`
> Return typed results. Handle errors (404 = not found, 422 = invalid config).

**Acceptance criteria**:
- [ ] All 6 methods compile and return typed results
- [ ] Error handling for common failure modes
- [ ] FLY_API_TOKEN read from env
- [ ] Integration test with mock server (httptest crate)

**Size**: M (2-3 days)

---

### Task 10.2: Extend ProcessSupervisor for Fly Machines

**Target**: `crates/roko-runtime/src/process.rs`

**Prompt**:
> Extend ProcessSupervisor to support remote Fly Machine execution alongside local processes.
> 1. Add `ExecutionTier` to `SpawnConfig`: `InProcess` (local) or `Isolated { fly_app, cpus, memory_mb, volume_size_gb }`
> 2. When `Isolated`: call `FlyClient::create_machine()` instead of `tokio::process::Command`
> 3. Machine runs `roko agent run --managed --parent-url {parent_url}`
> 4. Track `MachineId` alongside local `ProcessId` in supervisor state
> 5. `shutdown()` calls `stop_machine()` then `destroy_machine()` for Fly processes
> 6. Health monitoring: poll `get_machine()` for status instead of checking local PID
> If `FLY_API_TOKEN` not set, `Isolated` tier falls back to local process with a warning.

**Acceptance criteria**:
- [ ] `SpawnConfig` accepts `ExecutionTier::Isolated`
- [ ] Supervisor creates Fly Machine when tier is Isolated
- [ ] Shutdown properly stops and destroys Fly Machine
- [ ] Fallback to local process when FLY_API_TOKEN missing
- [ ] Health monitoring works for both local and remote processes

**Size**: L (3-4 days)

---

## Phase 11: Clusters + Fleet Coordination

### Task 11.1: Wire FleetConductor (L4)

**Source files**:
- `roko/crates/roko-conductor/src/federation.rs` — L4 FleetConductor (stub, always returns continue())

**Target**: `crates/roko-conductor/src/federation.rs`

**Prompt**:
> Implement L4 FleetConductor evaluation logic. Currently a stub that always returns `continue()`.
> 1. **Cross-agent budget tracking**: Aggregate L3 plan-level budgets across all active agents. If total fleet spend > fleet_budget_usd → demote all agents to T0.
> 2. **Failure rate monitoring**: Track per-agent gate failure rates. If agent fails 3+ consecutive gates → recommend stop + replan.
> 3. **Tier rebalancing**: If fleet has >50% agents at T2 → downgrade lowest-priority to T1. Maintains fleet cost control.
> 4. **Fleet health summary**: Emit `FleetHealthEvent { total_agents, active, idle, failed, total_cost_usd, avg_gate_pass_rate }` periodically.
> 5. Integrate with existing L1-L3 conductor hierarchy.

**Acceptance criteria**:
- [ ] Fleet budget enforcement demotes all agents when exceeded
- [ ] Consecutive failure detection recommends stopping bad agents
- [ ] Tier rebalancing activates when >50% are T2
- [ ] FleetHealthEvent emitted for dashboard consumption
- [ ] L4 decisions don't conflict with L2/L3 (cascading hierarchy respected)

**Size**: M (3 days)

---

### Task 11.2: Cluster API routes

**Target**: `crates/roko-serve/src/routes/clusters.rs`

**Prompt**:
> Add cluster management routes to roko-serve.
> 1. `POST /api/clusters` — Create cluster with agents + pipeline definition
>    Body: `{ name, agents: [{ profile, name, mode, isolated }], pipeline: [{ stage, agents: [names], depends_on: [stage_names] }], shared_context: { prd, repo } }`
> 2. `GET /api/clusters` — List all clusters with status
> 3. `GET /api/clusters/:id` — Cluster detail (pipeline progress, per-agent status, cost)
> 4. `POST /api/clusters/:id/stop` — Stop all agents in cluster
> 5. `DELETE /api/clusters/:id` — Destroy cluster + all agents
> Storage: `.roko/clusters/{name}.toml`
> Cluster state tracks: stage progress (waiting/running/done), agent assignments, total cost.

**Acceptance criteria**:
- [ ] Create cluster with multi-stage pipeline
- [ ] Pipeline stages execute in dependency order
- [ ] Agent status visible per cluster
- [ ] Stop halts all agents
- [ ] Delete cleans up all resources
- [ ] `cargo test -p roko-serve` passes with cluster route tests

**Size**: L (3-4 days)

---

## Phase 12: Payments (Optional)

### Task 12.1: Port Machine Payment Protocol

**Source files**:
- `bardo/crates/mpp/src/` (988 LOC) — ERC-3009 USDC micropayments

**Target**: `crates/roko-mpp/`

**Prompt**:
> Port bardo's Machine Payment Protocol for USDC-based inference payments.
> 1. **ERC-3009 transferWithAuthorization**: Off-chain USDC signing (no on-chain approval tx needed)
> 2. **Session-based billing**: Create payment session → accumulate costs → settle batch
> 3. **Draw mechanics**: Gateway draws from pre-authorized USDC allowance per inference request
> 4. **Verification**: Validate EIP-712 signatures off-chain before accepting payment
> 5. Wire as optional middleware in roko-gateway: if MPP enabled, require USDC payment per request
> Reference: bardo/crates/mpp/src/

**Acceptance criteria**:
- [ ] ERC-3009 signature verification compiles
- [ ] Session creation and cost accumulation work
- [ ] Gateway middleware rejects requests without valid payment (when MPP enabled)
- [ ] MPP disabled by default (feature flag)

**Size**: M (2-3 days)

---

## Summary

| Phase | Tasks | Priority | Parallelizable |
|-------|-------|----------|----------------|
| 1. Inference Gateway | 12 | P0 | No (sequential foundation) |
| 2. Orchestrator Gaps | 7 | P0 | Yes (with Phase 1) |
| 3. Learning Loop Gaps | 5 | P1 | Yes (with Phases 1-2) |
| 4. Heartbeat Pipeline | 2 | P1 | Yes (with Phases 1-3) |
| 5. Agent Modes | 3 | P1 | After Phase 2 |
| 6. Dashboard | 4 | P1 | After Phase 1.11 |
| 7. DeFi Tools + Chain | 4 | P2 | Yes (standalone) |
| 8. TUI Enhancements | 3 | P2 | Yes (standalone) |
| 9. Operational Infra | 3 | P2 | Yes (standalone) |
| 10. Fly Machines | 2 | P2 | After Phase 5 |
| 11. Clusters | 2 | P3 | After Phases 4-5 |
| 12. Payments | 1 | P3 | After Phase 1 |
| **Total** | **48** | | |

**Critical path**: Phase 1 (Gateway) → Phase 6 (Dashboard) → Phase 10 (Fly)

**Parallel tracks**: Phases 2+3+4 (Orchestrator+Learning+Heartbeat) can run alongside Phase 1. Phases 7+8+9 (DeFi+TUI+Ops) are fully independent.

---

## Dependency Graph

```
Phase 1 (Gateway) ──→ Phase 6 (Dashboard)
     │               Phase 10 (Fly Machines)
     └─→ Phase 12 (Payments)

Phase 2 (Orchestrator) ──→ Phase 5 (Agent Modes) ──→ Phase 10 (Fly)
Phase 3 (Learning) ──→ (standalone)
Phase 4 (Heartbeat) ──→ Phase 11 (Clusters)
Phase 5 (Agent Modes) ──→ Phase 11 (Clusters)

Phase 7 (DeFi) ──→ (standalone, parallel with everything)
Phase 8 (TUI) ──→ (standalone)
Phase 9 (Ops) ──→ (standalone)
```
