# Innovations and New Features: Task Breakdown

> 65 tasks across 4 phases and 15 feature areas. Wire agent memory, adaptive
> context, cost optimization, self-improving gates, agent debugging, interactive
> steering, multi-agent collaboration, speculative execution, cross-project
> knowledge, A2A interoperability, CaMeL safety, OTel observability, and
> research-derived enhancements.
>
> Sources: `impl/11-INNOVATIONS.md`, `10-INNOVATIONS-AND-NEW-FEATURES.md`,
> `07-RESEARCH-SYNTHESIS-1.md`, `08-RESEARCH-SYNTHESIS-2.md`, codebase analysis
>
> Research citations: Mem0/ActiveContext (arxiv 2604.11462), CaMeL (arxiv 2503.18813),
> Speculative Actions (arxiv 2510.04371), AXIOM BMR (arxiv 2505.24784), DCILP (AAAI 2025),
> PID (Williams-Beer/Broja), HGM CMP (ICLR 2026), CodeCRDT (arxiv 2510.18893),
> RouteLLM, FrugalGPT, A2A v1.0, ERC-8004, OTel gen_ai.* v1.37+

---

## Overview

Roko's core plan-execute-gate-persist loop works end-to-end. The learning
subsystem records episodes, routes models, runs experiments, and adapts gate
thresholds. But the loop operates in a cold-start mode: agents never consult
prior learning at spawn time, gates run uniform checks regardless of what
changed, and cost tracking exists only in scattered efficiency events. These 65
tasks wire the built-but-disconnected subsystems into a compounding intelligence
layer.

**Key principle**: Every innovation task wires existing infrastructure into a
live code path. The codebase already has EpisodeLogger, KnowledgeStore,
PlaybookStore, ErrorPatternStore, HDC fingerprints, CascadeRouter with LinUCB,
ExperimentStore, AdaptiveThresholds, DreamCycle, CFactorSummary, and
WorktreeManager. The work is connection, not construction.

### Existing infrastructure (verified in codebase)

| Component | Location | Status |
|---|---|---|
| EpisodeLogger | `crates/roko-learn/src/episode_logger.rs` | Wired, records episodes with `hdc_fingerprint` field |
| KnowledgeStore | `crates/roko-neuro/src/knowledge_store.rs` | Wired, has `ingest()`, `query()`, `query_similar()`, `query_kind()`, anti-knowledge |
| PlaybookStore | `crates/roko-learn/src/playbook.rs` | Wired, has `Playbook`, `PlaybookStep`, `PlaybookStore` |
| ErrorPatternStore | `crates/roko-learn/src/error_pattern_store.rs` | Wired, accumulates error patterns with categories |
| CascadeRouter | `crates/roko-learn/src/cascade_router.rs` | Wired, has `route()`, `route_with_knowledge()`, `route_with_cfactor()` |
| ExperimentStore | `crates/roko-learn/src/prompt_experiment.rs` | Wired, A/B experiments |
| BudgetGuardrail | `crates/roko-learn/src/budget.rs` | Exists but task/session/day granularity only, no plan-level |
| HdcVector | `crates/roko-primitives/src/hdc.rs` | Wired, `fingerprint()`, `hamming_similarity()` |
| DreamCycle | `crates/roko-dreams/src/cycle.rs` | Built, no runtime trigger (CLAUDE.md item 14) |
| DreamRunner | `crates/roko-dreams/src/runner.rs` | Built, has `DreamRuntimeControls`, no cron/schedule |
| CFactorSummary | `crates/roko-core/src/cfactor.rs` | Wired, single scalar |
| WorktreeManager | `crates/roko-orchestrator/src/worktree.rs` | Wired, 1,203 LOC |
| CancelToken | `crates/roko-runtime/src/cancel.rs` | Wired |
| PromptAssemblyService | `crates/roko-compose/src/prompt_assembly_service.rs` | Wired, 9-layer builder |
| SystemPromptBuilder | `crates/roko-compose/src/system_prompt_builder.rs` | Wired |
| AttentionBidder | `crates/roko-compose/src/attention.rs` + 12 files | Wired but VCG path not dominant |
| GateService | `crates/roko-gate/src/gate_service.rs` | Wired, 7-rung pipeline |
| RuntimeFeedback | `crates/roko-learn/src/runtime_feedback.rs` | Wired |
| WorkflowEngine | `crates/roko-runtime/src/workflow_engine.rs` | Wired |
| AgentContract | `crates/roko-agent/src/safety/contract.rs` | Partial, falls back to permissive default |
| tool_loop/result_msg.rs | `crates/roko-agent/src/tool_loop/result_msg.rs` | Wired, tool output formatting |
| KnowledgeRoutingAdvice | `crates/roko-learn/src/cascade/types.rs` | Type exists, wired via `route_with_knowledge()` |

---

## Anti-Patterns to Remove

| ID | Anti-Pattern | Where | Severity |
|---|---|---|---|
| AP-COLD | Agents start cold every run; never consult prior learning at spawn time | Dispatch path in `crates/roko-cli/src/dispatch/mod.rs` | Critical |
| AP-UNIFORM | Gate pipeline runs identical checks regardless of diff content | `crates/roko-gate/src/gate_service.rs` `run_gates()` | High |
| AP-NOCOST | No plan-level budget cap; `--max-cost` flag missing | `crates/roko-cli/src/main.rs` | High |
| AP-NOLEARN | `force_backend` overrides not fed back to CascadeRouter (CLAUDE.md UX34) | `crates/roko-learn/src/cascade_router.rs` | Medium |
| AP-NEURO | KnowledgeStore not consulted for model selection (CLAUDE.md item 13) | `crates/roko-learn/src/cascade_router.rs` | Medium |
| AP-NODREAM | Dream consolidation built but no runtime trigger (CLAUDE.md item 14) | `crates/roko-dreams/src/runner.rs` | Medium |
| AP-SAMEFAM | Oracle gate judges can use same model family as task agent | `crates/roko-gate/src/gate_service.rs` | Medium |
| AP-VERBOSE | Tool outputs included in agent context without truncation or sanitization | `crates/roko-agent/src/tool_loop/result_msg.rs` | Medium |
| AP-SINGULAR | CFactorSummary is single scalar; PID replication failure argues for multi-dimensional | `crates/roko-core/src/cfactor.rs` | Low |
| AP-NOEXP | ExperimentStore outcomes not fed back to CascadeRouter routing weights | `crates/roko-learn/src/prompt_experiment.rs` / `cascade_router.rs` | Medium |

---

## Phase 1: Foundations (Memory, Context, Cost)

Deliver immediate value by wiring existing infrastructure into live paths. No
new architectural patterns -- just connections between built-but-disconnected
subsystems.

---

### Task 11.1: Create MemoryLayer struct wrapping three memory tiers
**Priority**: P0
**Estimated Effort**: 4 hours
**Removes**: AP-COLD (partial)
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/memory_layer.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/lib.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/Cargo.toml` (if roko-neuro dep needed)
**Depends On**: none

#### Context
Agents are stateless between invocations. EpisodeLogger (`.roko/episodes.jsonl`),
KnowledgeStore (`.roko/neuro/knowledge.jsonl`), and PlaybookStore
(`.roko/learn/playbooks/`) all exist and are populated during runs, but nothing
unifies them or queries them at dispatch time.

Research: Mem0, JetBrains Research, ActiveContext (arxiv 2604.11462) -- vanilla
RAG fails for agentic use cases; agents need stateful persistence that recalls
context on demand, not just one-shot retrieval.

`EpisodeLogger` is at `crates/roko-learn/src/episode_logger.rs` with
`hdc_fingerprint: Option<String>` on episodes. `KnowledgeStore` is at
`crates/roko-neuro/src/knowledge_store.rs` with `ingest()`, `query()`,
`query_similar()`, and anti-knowledge support. `PlaybookStore` is at
`crates/roko-learn/src/playbook.rs` with `Playbook`, `PlaybookStep`.

#### Implementation Steps
1. Create `crates/roko-learn/src/memory_layer.rs`.
2. Define `MemoryLayer` struct holding owned instances (not references) of
   `EpisodeLogger`, `KnowledgeStore`, and `PlaybookStore`. Use `Arc` wrappers
   if shared ownership is needed.
3. Define `MemoryInjection` struct:
   ```rust
   pub struct MemoryInjection {
       pub playbooks: Vec<PlaybookEntry>,
       pub anti_patterns: Vec<String>,
       pub relevant_episodes: Vec<EpisodeSummary>,
       pub knowledge_entries: Vec<KnowledgeEntry>,
       pub total_tokens: usize,
   }
   ```
4. Define `EpisodeSummary` and `PlaybookEntry` as lightweight summary types
   (task_id, outcome, key_insight, confidence).
5. Implement `MemoryLayer::new(roko_dir: &Path) -> Result<Self>` that loads all
   three stores from their standard paths.
6. Stub `query_for_task(&self, ctx: &TaskContext) -> Result<MemoryInjection>`
   returning empty injection (implemented in Task 11.2).
7. Add `pub mod memory_layer;` to `crates/roko-learn/src/lib.rs`.
8. Ensure `roko-learn` can depend on `roko-neuro` for `KnowledgeStore` -- check
   `Cargo.toml` for existing dependency; add if missing.

#### Verification Criteria
- [ ] `cargo check -p roko-learn` compiles without errors
- [ ] `MemoryLayer::new()` succeeds on a `.roko/` directory with episode and knowledge data
- [ ] Unit test: construct MemoryLayer with empty stores, call `query_for_task`, get empty MemoryInjection
- [ ] `MemoryInjection` is importable from `roko_learn::memory_layer`

---

### Task 11.2: Implement memory retrieval with token budget
**Priority**: P0
**Estimated Effort**: 8 hours
**Removes**: AP-COLD (partial)
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/memory_layer.rs`
**Depends On**: Task 11.1

#### Context
The `query_for_task()` stub needs three-tier retrieval logic with a 2K token
budget. EpisodeLogger already has `hdc_fingerprint` per episode
(`crates/roko-learn/src/episode_logger.rs:247`). KnowledgeStore has
`query_similar()` for HDC-based retrieval
(`crates/roko-neuro/src/knowledge_store.rs:651`). PlaybookStore has search by
category.

HdcVector at `crates/roko-primitives/src/hdc.rs` provides `fingerprint()` and
`hamming_similarity()`. Use these for Tier 2 matching.

#### Implementation Steps
1. In `query_for_task()`, accept a `TaskContext` with `task_id`, `domain_tags`,
   `description`, and optional `hdc_fingerprint`.
2. Tier 1 (exact match): query EpisodeLogger for episodes matching `task_id`.
   Return the most recent attempt's outcome, error patterns, and tool calls.
3. Tier 2 (HDC similarity): if `hdc_fingerprint` is available on the task
   context, scan recent episodes (last 100) for `hamming_similarity > 0.7`.
   Weight by recency using half-life decay (configurable, default 7 days).
4. Tier 3 (semantic): query KnowledgeStore by domain tags using `query_kind()`.
   Include anti-knowledge entries (where `is_anti_knowledge == true`) in the
   `anti_patterns` field.
5. Query PlaybookStore for playbooks matching task category. Include top-3 by
   confidence score.
6. Enforce 2048-token budget: rank all results by relevance score (exact match
   > HDC similarity > semantic > playbook). Estimate token count per item (use
   `roko_compose::token_counter` if available, else heuristic of
   `text.len() / 4`). Truncate to fit budget.
7. Return populated `MemoryInjection` with `total_tokens` field.

#### Verification Criteria
- [ ] After 10+ recorded episodes, `query_for_task` returns the 3-5 most relevant items, not all items
- [ ] Anti-knowledge entries appear in the `anti_patterns` field
- [ ] Total token count of returned injection is <= 2048 tokens
- [ ] Unit test: insert 20 episodes, query, verify budget constraint
- [ ] Unit test: verify recency weighting -- recent episodes rank higher than old ones

---

### Task 11.3: Wire MemoryLayer into SystemPromptBuilder
**Priority**: P0
**Estimated Effort**: 4 hours
**Removes**: AP-COLD (complete)
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/prompt_assembly_service.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch/mod.rs` (or equivalent dispatch path)
**Depends On**: Task 11.2

#### Context
SystemPromptBuilder at `crates/roko-compose/src/system_prompt_builder.rs`
assembles 9 layers. Layers 6 (Techniques) and 7 (Anti-patterns) are the natural
injection points for memory-derived content. PromptAssemblyService at
`crates/roko-compose/src/prompt_assembly_service.rs` orchestrates the assembly.

The dispatch path (search `dispatch` in `crates/roko-cli/src/dispatch/mod.rs`
and `crates/roko-cli/src/runner/event_loop.rs`) is where the system prompt is
built before agent invocation.

#### Implementation Steps
1. In the dispatch path, construct or receive a `MemoryLayer` instance.
2. Call `memory_layer.query_for_task(&task_context)` to get `MemoryInjection`.
3. Format playbooks as layer 6 content: brief summaries with confidence scores.
   Example: "Playbook: {name} (confidence: {score})\n{steps_summary}".
4. Format anti-patterns as layer 7 content: "AVOID: {pattern} (seen {count}
   times, last failure: {date})".
5. Format relevant episodes as layer 4 supplemental content: "Prior attempt on
   similar task {task_id}: {outcome}. Key insight: {insight}."
6. Pass formatted sections to `PromptAssemblyService` as additional layer
   content. Respect existing token budget: if VCG auction is active, memory
   sections participate as bidders; otherwise, append within budget.
7. Add `--verbose` output showing memory injection contents for debugging.

#### Verification Criteria
- [ ] Run `roko plan run` on a plan where prior runs recorded episodes
- [ ] Inspect the system prompt (via `--verbose` or episode log): layers 6/7 contain memory-derived content
- [ ] A task that previously failed sees the failure pattern in its anti-patterns section
- [ ] Memory injection does not exceed the overall prompt token budget

---

### Task 11.4: Wire memory update on task completion
**Priority**: P0
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/runtime_feedback.rs`
**Depends On**: Task 11.1

#### Context
`RuntimeFeedback` at `crates/roko-learn/src/runtime_feedback.rs` handles
post-task feedback. This is the natural place to update the memory layer after
each task attempt. KnowledgeStore has `ingest()` and anti-knowledge support.
PlaybookStore has `Playbook` and `PlaybookStep` types.

Research: Dohmatob 2025 -- accumulate (synthetic added to real) gives bounded
error vs replace scenarios. All new knowledge must be additive.

#### Implementation Steps
1. In the post-task feedback handler, check task outcome (success/failure).
2. On success:
   - Call `PlaybookStore::upsert()` (or create equivalent) with the successful
     approach as a new playbook.
   - Call `KnowledgeStore::ingest()` with extracted facts at Transient tier.
3. On failure:
   - Call `KnowledgeStore::ingest()` with error pattern as anti-knowledge
     (set `is_anti_knowledge: true`).
4. On either outcome:
   - Compute HDC fingerprint from task context + outcome using
     `roko_primitives::hdc::fingerprint()`.
   - Store fingerprint on the episode via `EpisodeLogger`.
5. If the ingested knowledge matches an existing entry (HDC similarity > 0.9),
   boost the existing entry's confidence instead of creating a duplicate.

#### Verification Criteria
- [ ] Run a plan with 5 tasks. 3 succeed, 2 fail
- [ ] After run: KnowledgeStore has 3 new entries (successes) and 2 anti-knowledge entries (failures)
- [ ] PlaybookStore has at least 1 new playbook from a successful task
- [ ] A second run on the same plan shows memory injection from first run's data
- [ ] Duplicate knowledge entries are merged (confidence boosted), not duplicated

---

### Task 11.5: Implement progressive disclosure context levels
**Priority**: P1
**Estimated Effort**: 8 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/prompt_assembly_service.rs`
**Depends On**: none

#### Context
SystemPromptBuilder assembles 9 layers but treats every model the same.
BenchLM.ai's 2026 comparison: effective context can fall 99% below advertised
maximum on complex tasks. Anthropic's context engineering guide (2026):
"context engineering is the discipline of building dynamic systems that provide
the right information at the right time."

JetBrains Research: observation masking (showing agents only relevant
observations while preserving action history) is the single most effective
strategy for software engineering agents.

#### Implementation Steps
1. Define `DisclosureLevel` enum: `Essential`, `Standard`, `Extended`, `Full`.
2. Tag each SystemPromptBuilder section with a disclosure level:
   - Essential: task description, tool instructions, critical constraints
   - Standard: + role context, code context, recent history
   - Extended: + knowledge injection, playbooks, full file contents
   - Full: + anti-patterns, experimental sections, verbose examples
3. Implement `select_disclosure_level(model_context_window: usize,
   content_tokens: usize) -> DisclosureLevel`:
   - If content fits in 30% of window: Full
   - If content fits in 50% of window: Extended
   - If content fits in 70% of window: Standard
   - Otherwise: Essential (with aggressive trimming)
4. Wire into the prompt assembly path: before assembling, compute total content
   tokens, select level, filter sections by level.
5. Log the selected disclosure level in verbose output.

#### Verification Criteria
- [ ] Dispatching to a model with 8K context window produces a prompt at Essential or Standard level
- [ ] Dispatching to Claude Opus (200K) produces a Full-level prompt
- [ ] The section-level filtering is visible in verbose output
- [ ] No prompt exceeds 70% of the model's context window
- [ ] `cargo test -p roko-compose` passes with disclosure level tests

---

### Task 11.6: Define ModelContextProfile and calibration data
**Priority**: P1
**Estimated Effort**: 4 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/context_profile.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/lib.rs`
**Depends On**: none

#### Context
Models have wildly different effective context windows. A prompt that works well
with Claude's 200K window may be catastrophically pruned for a Cerebras 8B
model with an 8K effective window. The compose crate needs per-model calibration
data.

#### Implementation Steps
1. Define `ModelContextProfile` struct: `model_slug: String`,
   `context_window: usize`, `sweet_spot_range: (usize, usize)`,
   `degradation_threshold: usize`, `calibrated: bool`.
2. Implement `ModelContextProfile::default_for(slug: &str)` with known values:
   - `claude-opus-4-*`: context_window = 200_000, sweet_spot = (4000, 40_000)
   - `claude-sonnet-4-*`: context_window = 200_000, sweet_spot = (3000, 30_000)
   - `claude-haiku-4-*`: context_window = 200_000, sweet_spot = (2000, 20_000)
   - `gpt-4o*`: context_window = 128_000, sweet_spot = (3000, 30_000)
   - `gemini-*`: context_window = 1_000_000, sweet_spot = (5000, 50_000)
   - `cerebras-*`: context_window = 8_192, sweet_spot = (1000, 5_000)
   - Default fallback: context_window = 128_000, sweet_spot = (2000, 20_000)
3. Implement serde for persistence to `.roko/learn/model-profiles/{slug}.json`.
4. Implement `optimal_size(&self, content_tokens: usize) -> usize` that returns
   the ideal context size: `min(content_tokens, sweet_spot.1)`, clamped to
   `[sweet_spot.0, degradation_threshold]`.
5. Add `pub mod context_profile;` to `crates/roko-compose/src/lib.rs`.

#### Verification Criteria
- [ ] `ModelContextProfile::default_for("claude-sonnet-4-6")` returns a profile with context_window = 200_000
- [ ] Profile serializes to and deserializes from JSON
- [ ] Unit test: `optimal_size` returns a value within the sweet spot range for moderate content sizes
- [ ] `cargo check -p roko-compose` compiles

---

### Task 11.7: Wire cost-aware Pareto routing into CascadeRouter
**Priority**: P0
**Estimated Effort**: 8 hours
**Removes**: partial AP-NOCOST
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/cascade_router.rs`
**Depends On**: none

#### Context
CascadeRouter at `crates/roko-learn/src/cascade_router.rs` has `route()`,
`route_with_knowledge()`, `route_with_cfactor()`, and
`route_with_knowledge_among()` methods. It already computes Pareto frontier via
`crates/roko-learn/src/pareto.rs` but does not use cost data for model selection.

Research: RouteLLM -- 85% cost cut on MT-Bench retaining 95% of GPT-4 quality.
FrugalGPT -- 98% cost reduction. Princeton HAL: 50x cost variation between
agents at similar accuracy.

#### Implementation Steps
1. In `CascadeRouter::route()`, retrieve the Pareto frontier of non-dominated
   models (quality vs cost).
2. Accept `budget_pressure: Option<f64>` parameter (computed as
   `remaining_budget / remaining_tasks` by caller).
3. Filter candidates to `expected_quality >= quality_floor` (configurable,
   default 0.7). Use existing `quality_judge.rs` or bandit observations.
4. Among viable candidates, sort by cost-per-token ascending when budget
   pressure is high (budget_pressure < 1.0), by quality descending when budget
   is unconstrained.
5. Return `CascadeModel` with the selected model. The existing `escalation`
   field on the return type can encode the fallback model.
6. Add `quality_floor: f64` to `CascadeRouterConfig` with default 0.7.

#### Verification Criteria
- [ ] With a tight budget ($0.10/task), the router selects Haiku or Cerebras over Opus/Sonnet
- [ ] With an unconstrained budget, the router selects the highest-quality model
- [ ] After 50+ observations, dominated models (high cost, low quality) are not selected
- [ ] Verify via `cascade-router.json` that Pareto routing data is persisted
- [ ] `cargo test -p roko-learn` passes

---

### Task 11.8: Implement per-plan budget manager
**Priority**: P0
**Estimated Effort**: 8 hours
**Removes**: AP-NOCOST
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/plan_budget.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/lib.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/main.rs` (add `--max-cost` flag)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs` (wire budget check)
**Depends On**: none

#### Context
`BudgetGuardrail` at `crates/roko-learn/src/budget.rs` has task/session/day
granularity with `record_cost()` and `BudgetAction` (Ok/Warn/Block). But there
is no plan-level budget cap and no `--max-cost` CLI flag.

Research: Cost visibility is #1 developer pain. Cursor's pinned forum thread
has 117K views. Princeton HAL: 50x cost variation between agents at similar accuracy.

#### Implementation Steps
1. Create `crates/roko-learn/src/plan_budget.rs`.
2. Define `PlanBudgetManager` struct: `plan_budget_usd: f64`, `spent_usd: f64`,
   `remaining_tasks: usize`, `task_costs: HashMap<String, f64>`.
3. Implement `budget_for_task(task: &str, complexity_multiplier: f64) -> TaskBudget`:
   - `target_usd = (plan_budget_usd - spent_usd) / remaining_tasks * complexity_multiplier`
   - `hard_cap_usd = target_usd * 3.0`
   - `allow_escalation = spent_usd < plan_budget_usd * 0.7`
4. Implement `record_cost(task_id: &str, cost_usd: f64)` and
   `is_exceeded() -> bool`.
5. Implement `budget_pressure(&self) -> f64` returning
   `remaining_budget / remaining_tasks` (input to Task 11.7).
6. Implement serde for persistence: serialize to `.roko/state/plan-budget.json`
   so budget state survives `--resume`.
7. Add `--max-cost <USD>` flag to `roko plan run` and `roko run` in
   `crates/roko-cli/src/main.rs`.
8. Wire `PlanBudgetManager` into the runner event loop: before dispatching each
   task, call `is_exceeded()`. If true, halt with error message:
   "Budget exceeded: ${spent} spent of ${budget} budget."
9. Add `pub mod plan_budget;` to `crates/roko-learn/src/lib.rs`.

#### Verification Criteria
- [ ] `roko plan run --max-cost 1.00` halts when cumulative cost reaches $1.00
- [ ] Error message shows: "Budget exceeded: $X.XX spent of $1.00 budget."
- [ ] Budget state persists across `--resume` runs
- [ ] Without `--max-cost`, no budget enforcement (backward compatible)
- [ ] `cargo test -p roko-learn` passes with budget tests

---

### Task 11.9: Implement semantic cache with BLAKE3 exact match
**Priority**: P1
**Estimated Effort**: 12 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/semantic_cache.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/lib.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/Cargo.toml` (add blake3 dep if not present)
**Depends On**: none

#### Context
When the same fix is applied multiple times (e.g., re-running a plan after
partial failure), the LLM is called even if the prompt is byte-identical.
BLAKE3 hashing of the full prompt provides exact-match deduplication with
zero false positives.

Research: Augment Code SWE-bench analysis -- 50-60% of tokens are removable.
Prompt-cache alone is 0.20x cost multiplier.

#### Implementation Steps
1. Create `crates/roko-learn/src/semantic_cache.rs`.
2. Define `SemanticCache` struct with `exact: HashMap<[u8; 32], CachedResponse>`.
3. Define `CachedResponse`: `response: String`, `model: String`,
   `created_at: DateTime<Utc>`, `ttl: Duration`, `task_category: String`.
4. Implement `check(prompt: &str) -> Option<CachedResponse>`:
   - Compute BLAKE3 hash of prompt.
   - Look up in exact map. If found and not expired, return.
5. Implement `store(prompt: &str, response: &str, model: &str, ttl: Duration,
   task_category: &str)`:
   - Only cache deterministic tasks (compile fixes, format, simple edits).
   - Never cache creative/architectural tasks (check `task_category`).
6. Implement `evict_expired()` to remove stale entries.
7. Persist cache to `.roko/cache/semantic.json` with TTL-based eviction on load.
8. Wire into dispatch path: check cache before LLM call, store after successful call.
9. Add `pub mod semantic_cache;` to `crates/roko-learn/src/lib.rs`.

#### Verification Criteria
- [ ] Run the same fix task twice. Second run hits exact cache, zero LLM cost
- [ ] Second run response time < 100ms (cache lookup only)
- [ ] Cache does not store creative/architectural task responses
- [ ] Expired entries are evicted on next check
- [ ] `cargo test -p roko-learn` passes with cache tests

---

### Task 11.10: Add HDC fuzzy matching to semantic cache
**Priority**: P2
**Estimated Effort**: 8 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/semantic_cache.rs`
**Depends On**: Task 11.9

#### Context
Exact-match caching misses prompts that differ only in line numbers, variable
names, or whitespace. HDC fingerprints at `crates/roko-primitives/src/hdc.rs`
provide `hamming_similarity()` for fuzzy matching.

#### Implementation Steps
1. Add `fuzzy: Vec<(HdcVector, CachedResponse)>` to `SemanticCache`.
2. On cache miss for exact match, compute HDC fingerprint of prompt.
3. Scan fuzzy entries for `hamming_similarity > 0.95` (configurable threshold).
4. If match found, validate applicability: check that the cached response's
   code context overlaps with the current context (file paths, function names).
5. On store, also insert into fuzzy index.
6. Limit fuzzy index to 1000 entries (LRU eviction).

#### Verification Criteria
- [ ] A compile fix for `foo.rs:42` gets cached. A subsequent fix for `foo.rs:45` with the same error type hits the fuzzy cache
- [ ] False positive rate < 5% on a test suite of 50 similar-but-different prompts
- [ ] Fuzzy match latency < 10ms for 1000 entries
- [ ] `cargo test -p roko-learn` passes

---

### Task 11.11: Implement prompt compression pipeline
**Priority**: P1
**Estimated Effort**: 8 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/compressor.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/lib.rs`
**Depends On**: Task 11.5, Task 11.6

#### Context
When the computed prompt exceeds the model's optimal context size (from
ModelContextProfile), tokens must be reduced. The compose crate already has
`compaction.rs` and `token_counter.rs`. Section effectiveness data is available
from `roko-learn/src/section_effect.rs`.

#### Implementation Steps
1. Create `crates/roko-compose/src/compressor.rs`.
2. Strategy 1 (regex-based): strip redundant whitespace, code comments in
   examples, duplicate section headers. No LLM needed.
3. Strategy 2 (code summarization): if a code block exceeds 200 tokens, replace
   with function signature + docstring + `// ... N lines`.
4. Strategy 3 (section-effectiveness-aware): if section_effect data is available
   (from `roko-learn`), drop sections with the lowest measured lift first.
   Never drop task description or tool instructions.
5. Implement `compress(prompt: &str, target_tokens: usize) -> String` applying
   strategies in sequence until target is reached.
6. Wire into prompt assembly when computed prompt exceeds
   `ModelContextProfile::optimal_size()`.
7. Add `pub mod compressor;` to `crates/roko-compose/src/lib.rs`.

#### Verification Criteria
- [ ] A 15K-token prompt compressed for an 8K-context model produces output <= 5.6K tokens (70% of window)
- [ ] Compressed prompt retains task description and tool instructions verbatim
- [ ] Code blocks > 200 tokens are summarized to < 50 tokens
- [ ] Unit test: compress a known prompt, verify output is valid and smaller
- [ ] `cargo test -p roko-compose` passes

---

### Task 11.12: Add `roko learn costs` CLI command
**Priority**: P1
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/learn.rs`
**Depends On**: none

#### Context
Cost data is recorded in `.roko/learn/efficiency.jsonl` via the efficiency
events system. No CLI command aggregates and displays it.

#### Implementation Steps
1. Read `.roko/learn/efficiency.jsonl` for cost data.
2. Aggregate: total cost, per-task cost, per-model cost distribution.
3. Compute cost-per-gate-pass: total cost / number of gate passes.
4. Display as a formatted table: Task, Model, Cost, Gate Pass, Cost/Pass.
5. Show model distribution as a simple bar chart (Unicode blocks).
6. Wire into the `learn` subcommand as `roko learn costs`.

#### Verification Criteria
- [ ] `roko learn costs` displays a table after at least one run with cost data
- [ ] Per-model breakdown sums to total cost (within rounding)
- [ ] Cost-per-gate-pass is computed correctly
- [ ] `cargo build -p roko-cli` compiles

---

## Phase 2: Self-Improvement (Gates, Debugging, Steering)

Build on Phase 1 foundations. Require the memory layer and cost infrastructure
to be in place.

---

### Task 11.13: Create GateEvolver for failure-pattern-driven gate generation
**Priority**: P1
**Estimated Effort**: 12 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/gate_evolver.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/lib.rs`
**Depends On**: none

#### Context
`ErrorPatternStore` at `crates/roko-learn/src/error_pattern_store.rs`
accumulates patterns with error_hash, category, message, frequency, last_seen.
These are never fed back into gate construction.

Research: Darwin Godel Machine (Sakana, May 2025) -- self-improving agent that
also reward-hacked by removing monitoring tokens. Gates must evolve BUT remain
immutable from the agent's perspective (see Task 11.41).

#### Implementation Steps
1. Create `crates/roko-gate/src/gate_evolver.rs`.
2. Define `GateEvolver` struct holding a reference path to `ErrorPatternStore`
   data (`.roko/learn/error-patterns.json`).
3. Define `GeneratedGate`: `name: String`, `shell_command: String`,
   `target_pattern: String`, `created_from: String`, `effectiveness: f64`,
   `retired: bool`.
4. Implement `evolve_gates(&self, patterns: &[ErrorPattern]) -> Vec<GeneratedGate>`:
   - For each pattern with count >= 3, generate a ShellGate:
     - "unused import" -> `grep -rn "^use.*unused" {files}`
     - "missing semicolon" -> targeted syntax check
     - "type mismatch" -> focused `cargo check` on changed files only
5. Implement `should_retire(&self, gate: &GeneratedGate) -> bool`:
   - Retire if 3+ consecutive false positives.
6. Persist generated gates to `.roko/learn/gate-evolution.json`.
7. Add `pub mod gate_evolver;` to `crates/roko-gate/src/lib.rs`.

#### Verification Criteria
- [ ] After 5 runs with recurring "unused import" failures, a targeted grep-based gate exists in `gate-evolution.json`
- [ ] Generated gate runs in < 100ms vs clippy's 3-8 seconds
- [ ] A gate with 3+ consecutive false positives is marked `retired: true`
- [ ] `cargo test -p roko-gate` passes

---

### Task 11.14: Wire generated gates into GateService runtime
**Priority**: P1
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/gate_service.rs`
**Depends On**: Task 11.13

#### Context
GateService at `crates/roko-gate/src/gate_service.rs` runs the 7-rung pipeline.
Generated gates need to be inserted as pre-flight checks before the standard
rung pipeline (rung 0).

#### Implementation Steps
1. In `GateService::run_gates()`, load generated gates from
   `.roko/learn/gate-evolution.json`.
2. Filter to non-retired gates whose target pattern matches the current diff.
3. Run matching generated gates as rung 0 (before compile).
4. If a generated gate catches the issue, skip the expensive standard rung
   that would have caught it (e.g., skip clippy if a grep-based gate already
   found unused imports).
5. Record generated gate outcomes for effectiveness tracking.

#### Verification Criteria
- [ ] A generated "unused import" gate fires before clippy and catches the issue
- [ ] Gate report shows the generated gate ran at rung 0
- [ ] If the generated gate passes but clippy later catches the same issue, the generated gate's effectiveness score decreases
- [ ] `cargo test -p roko-gate` passes

---

### Task 11.15: Implement DiffAnalyzer for rung relevance
**Priority**: P1
**Estimated Effort**: 8 hours
**Removes**: AP-UNIFORM
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/diff_analyzer.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/lib.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/gate_service.rs`
**Depends On**: none

#### Context
Gate pipeline runs the same 7 rungs regardless of what changed. A diff touching
only documentation files should skip compile, clippy, and test gates.

#### Implementation Steps
1. Create `crates/roko-gate/src/diff_analyzer.rs`.
2. Define `DiffAnalysis` struct: `files_changed: Vec<PathBuf>`,
   `categories: HashSet<FileCategory>`, `estimated_complexity: Complexity`.
3. `FileCategory` enum: `Source`, `Test`, `Documentation`, `Config`, `Build`.
4. `Complexity` enum: `Trivial`, `Moderate`, `Complex`.
5. Implement `analyze_diff(diff: &str) -> DiffAnalysis` using file extension
   and path heuristics:
   - `.rs` in `src/` -> Source
   - `.rs` in `tests/` or `*_test.rs` -> Test
   - `.md`, `.txt`, `.adoc` -> Documentation
   - `.toml`, `.yaml`, `.json` in root or config dirs -> Config
   - `Cargo.toml`, `build.rs` -> Build
6. Implement `relevant_rungs(analysis: &DiffAnalysis) -> Vec<RungId>`:
   - Documentation-only: format + diff only
   - Config-only: format + diff + validate
   - Test-only: format + test + diff
   - Source: all rungs
7. Wire into `GateService::run_gates()`: skip irrelevant rungs.

#### Verification Criteria
- [ ] A diff touching only `.md` files skips compile, clippy, and test gates
- [ ] A diff touching only test files skips clippy but runs test and format
- [ ] Gate report shows which rungs were skipped with reason "irrelevant to diff"
- [ ] `cargo test -p roko-gate` passes

---

### Task 11.16: Track gate effectiveness metrics
**Priority**: P1
**Estimated Effort**: 8 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/gate_effectiveness.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/lib.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/learn.rs`
**Depends On**: none

#### Context
No tracking of precision/recall per gate rung. Without this data, gates cannot
self-improve.

#### Implementation Steps
1. Create `crates/roko-learn/src/gate_effectiveness.rs`.
2. Define `GateEffectiveness` struct: `rung_id: String`, `true_positives: u64`,
   `false_positives: u64`, `true_negatives: u64`, `false_negatives: u64`.
3. Compute precision = TP / (TP + FP), recall = TP / (TP + FN), F1 score.
4. A "true positive" = gate fails AND the issue was real (confirmed by
   autofix succeeding after addressing the flagged issue).
5. A "false positive" = gate fails AND the fix attempt succeeds without
   addressing the flagged issue (the gate was wrong).
6. Persist to `.roko/learn/gate-effectiveness.json`.
7. Add `roko learn gates` CLI showing effectiveness report.
8. Add `pub mod gate_effectiveness;` to `crates/roko-learn/src/lib.rs`.

#### Verification Criteria
- [ ] After 20+ runs, `roko learn gates` shows precision/recall per rung
- [ ] At least one gate with precision < 0.5 is flagged for review
- [ ] Effectiveness data persists across runs
- [ ] `cargo test -p roko-learn` passes

---

### Task 11.17: Define FailureKind taxonomy for agent debugging
**Priority**: P1
**Estimated Effort**: 4 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/failure_taxonomy.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/lib.rs`
**Depends On**: none

#### Context
Agent failures are currently unclassified. A failure from token exhaustion looks
the same as a failure from wrong model selection. Systematic debugging requires
a taxonomy.

#### Implementation Steps
1. Create `crates/roko-learn/src/failure_taxonomy.rs`.
2. Define `FailureKind` enum:
   - `QualityFailure { gate_rung: String, error_hash: String, is_recurring: bool }`
   - `ConvergenceFailure { iterations: usize, repeated_error_hashes: Vec<String> }`
   - `ResourceFailure { kind: ResourceKind, used: f64, limit: f64 }`
   - `ToolFailure { tool_name: String, error: String, is_permission: bool }`
   - `ComprehensionFailure { evidence: Vec<String> }`
3. Define `ResourceKind` enum: `Tokens`, `Budget`, `Time`, `Context`.
4. Implement `classify(task_result: &TaskResult, episodes: &[Episode]) -> FailureKind`:
   - Check for repeated error hashes -> ConvergenceFailure
   - Check for budget/context exceeded -> ResourceFailure
   - Check for tool errors -> ToolFailure
   - Check for wrong-direction changes -> ComprehensionFailure
   - Default to QualityFailure
5. Add `is_recurring` check: compare error hash against past episodes.
6. Add `pub mod failure_taxonomy;` to `crates/roko-learn/src/lib.rs`.

#### Verification Criteria
- [ ] A task that fails 3 times with the same error is classified as ConvergenceFailure
- [ ] A task that runs out of tokens is classified as ResourceFailure
- [ ] A task where `bash` returns permission denied is classified as ToolFailure with `is_permission: true`
- [ ] Unit tests for each variant
- [ ] `cargo test -p roko-learn` passes

---

### Task 11.18: Implement debugger hypothesis generation
**Priority**: P2
**Estimated Effort**: 8 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/debug_engine.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/lib.rs`
**Depends On**: Task 11.17

#### Context
Once failures are classified, the system should generate ranked hypotheses
about root cause and propose interventions.

#### Implementation Steps
1. Create `crates/roko-learn/src/debug_engine.rs`.
2. Define `Hypothesis` struct: `cause: String`, `confidence: f64`,
   `intervention: Intervention`, `evidence: Vec<String>`.
3. Define `Intervention` enum: `RouteToModel(String)`, `AddContext(String)`,
   `FixPermissions(String)`, `AdjustPrompt(String)`, `TuneGate(String)`.
4. Implement `generate_hypotheses(failure: &FailureKind, context: &TaskContext,
   history: &[Episode]) -> Vec<Hypothesis>`:
   - ConvergenceFailure -> ["missing context", "wrong model", "prompt interference"]
   - ResourceFailure -> ["context too large", "model too expensive"]
   - ToolFailure -> ["permission mismatch", "tool not available"]
   - QualityFailure -> ["wrong model tier", "missing relevant code"]
5. Rank hypotheses by: recurrence in history, similarity to past interventions
   that worked (from PlaybookStore).
6. Add `pub mod debug_engine;` to `crates/roko-learn/src/lib.rs`.

#### Verification Criteria
- [ ] A ConvergenceFailure produces at least 3 ranked hypotheses
- [ ] Hypotheses include actionable interventions, not just descriptions
- [ ] A previously successful intervention ranks higher than novel ones
- [ ] `cargo test -p roko-learn` passes

---

### Task 11.19: Wire debugging into gate failure handler
**Priority**: P2
**Estimated Effort**: 12 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/workflow_engine.rs` (or runner gate handler)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/debug_engine.rs`
**Depends On**: Task 11.17, Task 11.18, Task 11.3

#### Context
When gate failures exhaust the autofix budget, execution currently halts. The
debug engine should classify the failure, generate hypotheses, apply the top
intervention, and retry.

`build_gate_failure_plan_revision` exists at `crates/roko-cli/src/orchestrate.rs`
but is in the legacy monolith. The WorkflowEngine at
`crates/roko-runtime/src/workflow_engine.rs` is the active runtime.

#### Implementation Steps
1. After autofix budget is exhausted, call `classify()` on the failure.
2. Call `generate_hypotheses()` with the classified failure.
3. Apply the top hypothesis's intervention:
   - `RouteToModel` -> override model for retry
   - `AddContext` -> inject additional context into prompt
   - `FixPermissions` -> adjust role for retry
   - `AdjustPrompt` -> modify section weights
4. Retry the task with the intervention applied.
5. If retry succeeds: record the intervention as a playbook entry.
6. If retry fails: try next hypothesis (up to 3 attempts).
7. If all hypotheses fail: generate a debug report and write to
   `.roko/debug/{task_id}.md`.

#### Verification Criteria
- [ ] A task that fails 3 times with "missing module" -> debug engine adds repo tree context -> retry succeeds
- [ ] Successful intervention is saved as a playbook
- [ ] Debug report is written for failures that exhaust all hypotheses
- [ ] `cargo test -p roko-runtime` passes

---

### Task 11.20: Define SteeringAction primitives
**Priority**: P2
**Estimated Effort**: 4 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/steering.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/lib.rs`
**Depends On**: none

#### Context
Interactive steering allows humans to redirect, skip, or adjust running agents
without stopping the plan.

#### Implementation Steps
1. Create `crates/roko-core/src/steering.rs`.
2. Define `SteeringAction` enum:
   - `Redirect { guidance: String, model_override: Option<String> }`
   - `Skip { reason: String }`
   - `Split { sub_tasks: Vec<String> }`
   - `BudgetAdjust { remaining_budget_usd: f64, model_preference: Option<String> }`
   - `InjectContext { content: String, priority: ContextPriority }`
   - `ReviewVerdict { task_id: String, verdict: Verdict, notes: String }`
3. Define `ContextPriority` enum: `Override`, `Append`, `Background`.
4. Define `ConfidenceThresholds` struct:
   - `auto_proceed: f64` (default 0.85)
   - `suggest_review: f64` (default 0.50)
   - `require_approval: f64` (default 0.50)
5. Define `SteeringAuditEntry` for the audit trail.
6. Implement serde for all types.
7. Add `pub mod steering;` to `crates/roko-core/src/lib.rs`.

#### Verification Criteria
- [ ] All types compile and serialize to/from JSON
- [ ] Unit test: round-trip serialization for each SteeringAction variant
- [ ] `cargo check -p roko-core` compiles

---

### Task 11.21: Implement steering channel
**Priority**: P2
**Estimated Effort**: 8 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/steering.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/lib.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/workflow_engine.rs`
**Depends On**: Task 11.20

#### Context
`WorkflowEngine` at `crates/roko-runtime/src/workflow_engine.rs` runs the main
execution loop. It needs a channel to receive steering actions without blocking.

#### Implementation Steps
1. Create `crates/roko-runtime/src/steering.rs`.
2. Define `SteeringChannel` wrapping `(mpsc::Sender<SteeringAction>,
   mpsc::Receiver<SteeringAction>)`.
3. Implement `SteeringChannel::new(buffer: usize) -> (SteeringSender,
   SteeringReceiver)`.
4. In the workflow engine's main loop, poll the steering receiver at each
   iteration alongside the agent task using `tokio::select!`.
5. On receiving a `SteeringAction`:
   - `Redirect` -> inject guidance into the agent's next prompt
   - `Skip` -> mark task as deferred, move to next
   - `BudgetAdjust` -> update PlanBudgetManager
   - `InjectContext` -> append to current prompt context
6. Record every steering action to `.roko/steer/audit.jsonl`.
7. Add `pub mod steering;` to `crates/roko-runtime/src/lib.rs`.

#### Verification Criteria
- [ ] Sending a `Redirect` action via the channel injects guidance into the next agent prompt iteration
- [ ] Sending a `Skip` action stops the current task and moves to the next
- [ ] Audit trail records all steering actions with timestamps
- [ ] Channel is non-blocking: the execution loop continues if no steering action is pending
- [ ] `cargo test -p roko-runtime` passes

---

### Task 11.22: Implement confidence scoring for tasks
**Priority**: P2
**Estimated Effort**: 8 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/confidence.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/lib.rs`
**Depends On**: Task 11.2, Task 11.20

#### Context
Confidence scoring enables the steering system to auto-proceed on high-
confidence tasks, suggest review on medium, and require approval on low.

#### Implementation Steps
1. Create `crates/roko-learn/src/confidence.rs`.
2. Define `ConfidenceScore` struct: `value: f64`, `components: Vec<(String, f64)>`.
3. Implement `compute_confidence(task_description: &str, memory: &MemoryInjection,
   thresholds: &AdaptiveThresholds) -> ConfidenceScore`:
   - Component 1: task complexity vs model capability
   - Component 2: similarity to past successes (from memory layer)
   - Component 3: expected gate pass probability (from adaptive thresholds)
   - Component 4: error pattern match (similar task failed before)
4. Weighted average of components (configurable weights).
5. Compare against `ConfidenceThresholds` to determine action:
   `AutoProceed`, `SuggestReview`, `RequireApproval`.
6. Add `pub mod confidence;` to `crates/roko-learn/src/lib.rs`.

#### Verification Criteria
- [ ] A task with many similar past successes scores > 0.85
- [ ] A task touching unfamiliar code with no history scores < 0.5
- [ ] Confidence is logged per task in the episode data
- [ ] `cargo test -p roko-learn` passes

---

### Task 11.23: Add HTTP steering endpoints
**Priority**: P2
**Estimated Effort**: 8 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/steering.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/mod.rs`
**Depends On**: Task 11.21, Task 11.22

#### Context
`roko-serve` at `crates/roko-serve/src/routes/` has ~85 routes. Steering
endpoints allow external clients (web dashboards, CI systems) to steer
running agents.

#### Implementation Steps
1. Create `crates/roko-serve/src/routes/steering.rs`.
2. `POST /api/steer/{task_id}` -- accept `SteeringAction` JSON body, send
   to steering channel.
3. `GET /api/confidence` -- return `Vec<ConfidenceReport>` for all active tasks.
4. `POST /api/approve/{task_id}` -- shorthand for `ReviewVerdict` with approve.
5. Wire into existing `roko-serve` router in `routes/mod.rs`.
6. Return 404 if task_id is not active, 409 if task already completed.
7. Respect existing auth middleware.

#### Verification Criteria
- [ ] `POST /api/steer/task-07 {"action":"redirect","guidance":"..."}` injects context into the running agent
- [ ] `GET /api/confidence` returns confidence scores for active tasks
- [ ] 401 returned for unauthenticated requests (when auth enabled)
- [ ] `cargo check -p roko-serve` compiles

---

### Task 11.24: Add TUI steering panel (F8)
**Priority**: P3
**Estimated Effort**: 12 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/modals/steering.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/input.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/mod.rs`
**Depends On**: Task 11.21, Task 11.22

#### Context
TUI at `crates/roko-cli/src/tui/` has F1-F7 tabs and a `modals/` directory.
F8 is the natural binding for the steering panel.

#### Implementation Steps
1. Bind F8 to open the steering panel.
2. Panel shows: current task, confidence score, agent state.
3. Key bindings within panel:
   - `s`: redirect (text input for guidance)
   - `k`: skip current task
   - `b`: adjust budget (numeric input)
   - `c`: inject context (text input)
   - `Esc`: close panel
4. On action, send via `SteeringSender` to the execution loop.
5. Show confirmation: "Steering action applied: Redirect sent to task-07".

#### Verification Criteria
- [ ] F8 opens the steering panel during a running plan
- [ ] Pressing `s` and typing guidance redirects the running agent
- [ ] Panel shows current task confidence score
- [ ] Esc closes without action

---

## Phase 3: Multi-Agent and Speculative Execution

Require parallel infrastructure and are higher complexity. Build on the memory,
cost, and steering foundations from Phases 1-2.

---

### Task 11.25: Implement competitive proposals (Best-of-N)
**Priority**: P2
**Estimated Effort**: 16 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-orchestrator/src/competitive.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-orchestrator/src/lib.rs`
**Depends On**: Task 11.8

#### Context
WorktreeManager at `crates/roko-orchestrator/src/worktree.rs` (1,203 LOC) can
isolate parallel attempts. CancelToken at `crates/roko-runtime/src/cancel.rs`
can stop losing agents.

Research: up to 70% higher success rates with multi-agent collaboration vs
single-agent (AWS Strands Agents, OpenAI Agents SDK, Swarms framework).
But Princeton NLP: single well-tooled agent matches or outperforms multi-agent
on 64% of tasks. Competitive proposals are the exception where multi-agent
reliably wins.

#### Implementation Steps
1. Create `crates/roko-orchestrator/src/competitive.rs`.
2. Define `CompetitiveRunner` struct with `proposal_count: usize` (default 3).
3. Implement `run_competitive(task: &Task, n: usize) -> Vec<ProposalResult>`:
   - Allocate N worktrees via WorktreeManager.
   - Spawn N agents concurrently (different models or different prompts).
   - Run gate pipeline on each completed proposal.
   - Rank by gate score. Select winner.
   - Clean up losing worktrees.
4. Wire into the dispatch path: if `--collaboration=competitive` is set,
   use CompetitiveRunner instead of single dispatch.
5. Track all proposals in the episode log with a `proposal_group` field.

#### Verification Criteria
- [ ] `roko plan run --collaboration=competitive --proposals=3` spawns 3 implementers in separate worktrees
- [ ] Gate pipeline scores all 3. Best wins. Others are cleaned up
- [ ] TUI shows all proposals with scores
- [ ] Episode log records all 3 proposals with the same `proposal_group`

---

### Task 11.26: Implement swarm collaboration pattern
**Priority**: P3
**Estimated Effort**: 16 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-orchestrator/src/swarm.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-orchestrator/src/lib.rs`
**Depends On**: Task 11.21

#### Context
EventBus at `crates/roko-runtime/src/event_bus.rs` provides typed broadcast
channels with replay support. The swarm pattern uses this for inter-agent
communication.

Research: CodeCRDT (arxiv 2510.18893) -- 600-trial study shows up to 21.1%
speedup but also 39.4% slowdown depending on task structure. Parallelism is
not a free lunch.

#### Implementation Steps
1. Create `crates/roko-orchestrator/src/swarm.rs`.
2. Define `SwarmRunner` with `roles: Vec<AgentRole>`.
3. Implement shared message channel (tokio broadcast) for inter-agent communication.
4. Implementer agent runs in its worktree. Reviewer watches the diff stream.
5. If reviewer detects an issue mid-implementation, inject a `Redirect`
   steering action into the implementer's context.
6. Task completes when implementer finishes AND reviewer approves.

#### Verification Criteria
- [ ] Swarm mode: reviewer catches a bug mid-implementation, implementer receives the correction before finishing
- [ ] If reviewer never objects, task completes at normal speed (no overhead)
- [ ] Signal bus messages are recorded in the episode log

---

### Task 11.27: Implement specialist mode for multi-crate changes
**Priority**: P3
**Estimated Effort**: 16 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-orchestrator/src/specialist.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-orchestrator/src/lib.rs`
**Depends On**: Task 11.25

#### Implementation Steps
1. Create `crates/roko-orchestrator/src/specialist.rs`.
2. Define `SpecialistRunner` that analyzes the task to identify affected crates.
3. Spawn one specialist agent per crate, each in its own worktree.
4. Each specialist works only on its crate's files.
5. A merge coordinator agent reconciles cross-crate interface changes.
6. Gate pipeline runs on the merged result.

#### Verification Criteria
- [ ] A 5-file cross-crate change spawns 3 specialists plus a merge agent
- [ ] Total wall time < 1.5x single-agent time
- [ ] Cross-crate type mismatches are resolved by the merge coordinator

---

### Task 11.28: Implement SpeculativeFixRunner for parallel gate fixes
**Priority**: P2
**Estimated Effort**: 16 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-orchestrator/src/speculative.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-orchestrator/src/lib.rs`
**Depends On**: Task 11.8, Task 11.17

#### Context
Research: Speculative Actions (arxiv 2510.04371) -- up to 55% accuracy in
next-action prediction, significant latency reductions.

CancelToken at `crates/roko-runtime/src/cancel.rs` enables first-to-finish semantics.

#### Implementation Steps
1. Create `crates/roko-orchestrator/src/speculative.rs`.
2. Define `SpeculativeFixRunner` with `max_parallel_fixes: usize` (default 3).
3. Implement error complexity classifier:
   - Trivial (unused import, format) -> single haiku agent, no speculation.
   - Moderate (type mismatch, missing impl) -> 2 parallel: haiku + sonnet.
   - Complex (logic error, architectural) -> 3 parallel: sonnet x2 + opus.
4. On `GateFailed`, classify error and spawn parallel fix agents.
5. Use `CancelToken` from roko-runtime: first agent to pass gates cancels others.
6. Feed failed attempt context as anti-pattern to surviving agents.
7. Track speculation outcomes for learning.

#### Verification Criteria
- [ ] A compile error fix spawns 2 parallel agents. The faster fix passes first, the other is cancelled
- [ ] Speculation cost for 3 parallel haiku runs < 1 sonnet run
- [ ] After 10+ speculative runs, the system learns which error categories benefit from speculation

---

### Task 11.29: Implement speculative prefetch for DAG tasks
**Priority**: P2
**Estimated Effort**: 12 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-orchestrator/src/dag.rs`
**Depends On**: none

#### Context
DAG executor at `crates/roko-orchestrator/src/dag.rs` (2,557 LOC) manages task
ordering. While task N executes, task N+1's context can be pre-built.

#### Implementation Steps
1. In the DAG executor's main loop, identify the next task(s) after the current batch.
2. For each candidate next task:
   - Resolve dependencies from the DAG.
   - Pre-build system prompt layers 1-3 (stable across tasks).
   - Pre-spawn a warm agent (connect to LLM, don't send prompt yet).
   - Pre-fetch code context for the candidate task.
3. If current task succeeds, hand off to the pre-warmed agent immediately.
4. If current task fails, discard the prefetch (context may have changed).
5. Add `--speculate` flag to `plan run` to enable speculative prefetch.

#### Verification Criteria
- [ ] DAG with 5 sequential tasks: speculative prefetch prepares task N+1 while N executes
- [ ] Discarded prefetches do not leak resources (agents, worktrees)
- [ ] Prefetch is disabled by default; enabled with `--speculate`

---

### Task 11.30: Implement density-threshold gating for multi-agent
**Priority**: P2
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-orchestrator/src/dag.rs`
**Depends On**: none

#### Context
Research: Stigmergic phase transition (arxiv 2512.10166) -- above agent density
rho_c = 0.230, trace-based coordination dominates by 36-41%. Below rho_c = 0.10,
stigmergy fails completely.

MAST taxonomy (NeurIPS 2025): 41-86.7% failure rates across multi-agent systems.
Princeton NLP: single agent matches multi-agent on 64% of tasks.

#### Implementation Steps
1. Define `agent_density(num_agents: usize, num_tasks: usize,
   interaction_edges: usize) -> f64`.
2. Before spawning parallel agents, compute density.
3. If density < 0.23, log warning and fall back to sequential execution.
4. Track density vs outcome in efficiency events for future calibration.
5. Add `multi_agent.density_threshold` to configuration (default 0.23).

#### Verification Criteria
- [ ] A plan with 2 agents and 20 tasks (density ~0.1) falls back to sequential
- [ ] A plan with 5 agents and 10 tasks (density ~0.5) proceeds with multi-agent
- [ ] Warning logged when density is below threshold

---

## Phase 4: Cross-Cutting, Safety, and Interoperability

Strategic features requiring earlier phases to be stable. Includes cross-project
learning, A2A interoperability, CaMeL safety, and research-derived enhancements.

---

### Task 11.31: Implement cross-project global config directory
**Priority**: P2
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/schema.rs` (or equivalent)
**Depends On**: none

#### Implementation Steps
1. Define `GlobalConfig` struct with paths: `domains/`, `meta/`, `cache/`, `community/`.
2. Implement `GlobalConfig::ensure(home_dir: &Path) -> Result<Self>` that creates
   the directory structure at `~/.roko/`.
3. Wire into CLI startup: ensure global dir exists before loading project config.
4. Add `global_dir` to the paths available in the runtime context.

#### Verification Criteria
- [ ] After running any `roko` command, `~/.roko/` exists with subdirectories
- [ ] Global config is loadable from any project
- [ ] Does not interfere with project-local `.roko/` directory

---

### Task 11.32: Implement domain detection for projects
**Priority**: P2
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/` (or new file)
**Depends On**: none

#### Implementation Steps
1. Define `DomainTag` enum: `Rust`, `TypeScript`, `JavaScript`, `Python`,
   `Go`, `Blockchain`, `React`, `WebApp`, etc.
2. Implement `detect_domains(workdir: &Path) -> Vec<DomainTag>`:
   - `Cargo.toml` -> Rust
   - `package.json` -> JavaScript
   - `tsconfig.json` -> TypeScript
   - `pyproject.toml` or `requirements.txt` -> Python
   - `go.mod` -> Go
   - `foundry.toml` -> Blockchain
3. Cache the result for the session.
4. Make domain tags available to dispatch path and memory layer.

#### Verification Criteria
- [ ] In the roko project (Rust), `detect_domains()` returns `[Rust]`
- [ ] In a project with both `Cargo.toml` and `package.json`, returns both tags
- [ ] Domain tags are logged in verbose output

---

### Task 11.33: Implement tiered KnowledgeStore for cross-project sharing
**Priority**: P3
**Estimated Effort**: 12 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-neuro/src/tiered_store.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-neuro/src/lib.rs`
**Depends On**: Task 11.31, Task 11.32

#### Implementation Steps
1. Define `TieredKnowledgeStore` wrapping:
   - Tier 0: project-specific (`.roko/neuro/knowledge.jsonl`)
   - Tier 1: domain-specific (`~/.roko/domains/{domain}/knowledge.jsonl`)
   - Tier 2: model meta-knowledge (`~/.roko/meta/model-knowledge.jsonl`)
2. Implement `query(topic: &str, domains: &[DomainTag]) -> Vec<KnowledgeEntry>`:
   - Always include Tier 0 and Tier 2.
   - Include Tier 1 only if domain tags match.
   - Rank by confidence, Tier 0 gets a small boost.
3. Implement conflict resolution: if entries conflict across tiers, use
   confidence score. If tied, prefer the more specific tier.

#### Verification Criteria
- [ ] Query from a Rust project returns Rust-specific Tier 1 knowledge
- [ ] Query from a TypeScript project does NOT return Rust-specific knowledge
- [ ] Model meta-knowledge (Tier 2) is available in all projects
- [ ] `cargo test -p roko-neuro` passes

---

### Task 11.34: Implement knowledge tier promotion logic
**Priority**: P3
**Estimated Effort**: 8 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-neuro/src/tiered_store.rs`
**Depends On**: Task 11.33

#### Implementation Steps
1. On run completion, scan new knowledge entries from Tier 0.
2. Filter: exclude entries containing project-specific paths, variable names, or secrets.
3. Classify remaining by tier:
   - About model/tool behavior -> Tier 2
   - About language/framework patterns -> Tier 1
   - Project-specific -> stay at Tier 0
4. Promote if: confidence > 0.8, pattern matches domain tags, not path-dependent,
   for Tier 2: confirmed across 2+ domains.
5. Implement path/secret scrubbing: remove absolute paths, replace with
   placeholders, strip anything matching known secret patterns.

#### Verification Criteria
- [ ] After roko learns "Cerebras fails on async trait impls" in project A, the entry appears in `~/.roko/meta/model-knowledge.jsonl`
- [ ] Promoted entries contain no absolute paths or project-specific identifiers
- [ ] An entry with confidence < 0.8 is not promoted

---

### Task 11.35: Add `roko knowledge export/import` commands
**Priority**: P3
**Estimated Effort**: 8 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/knowledge.rs`
**Depends On**: Task 11.33, Task 11.34

#### Implementation Steps
1. `roko knowledge export [--domain <tag>] [--tier <n>] -o <file>`:
   - Export entries matching filters with full scrubbing.
   - Output as JSON.
2. `roko knowledge import <file> [--tier <n>]`:
   - Validate entries. Import at specified tier (default: Tier 1).
   - Merge with existing (boost confidence if duplicate).
3. `roko knowledge domains`: list all domain stores with entry counts.

#### Verification Criteria
- [ ] `roko knowledge export` produces a JSON file with no absolute paths
- [ ] `roko knowledge import` adds entries to the appropriate store
- [ ] `roko knowledge domains` lists domains with counts

---

### Task 11.36: Define A2A core types
**Priority**: P2
**Estimated Effort**: 8 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/a2a.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/lib.rs`
**Depends On**: none

#### Context
A2A v1.0 is stable with Signed Agent Cards, 150+ organizations, JSON-RPC + gRPC
bindings. Effectively unopposed as the cross-vendor agent bus.

#### Implementation Steps
1. Define `AgentCard` struct matching A2A v1.0 spec: `name`, `description`,
   `url`, `version`, `capabilities`, `skills`, `authentication`.
2. Define `A2ASkill`: `id`, `name`, `description`, `input_modes`, `output_modes`.
3. Define `A2ATask`: `id`, `status`, `messages`, `artifacts`.
4. Define `A2AArtifact`: `name`, `content_type`, `data`.
5. Implement JSON-RPC 2.0 request/response types for A2A methods:
   `tasks/send`, `tasks/get`, `tasks/cancel`, `tasks/sendSubscribe`.
6. Implement serde for all types, validated against A2A v1.0 spec.
7. AgentCard includes roko's 4 skills: code-implementation, code-review,
   gate-verification, knowledge-query.
8. Add `pub mod a2a;` to `crates/roko-core/src/lib.rs`.

#### Verification Criteria
- [ ] All types serialize to JSON matching the A2A v1.0 spec
- [ ] Round-trip test: serialize, deserialize, compare equality
- [ ] AgentCard includes roko's 4 skills
- [ ] `cargo test -p roko-core` passes

---

### Task 11.37: Implement A2A Agent Card endpoint
**Priority**: P2
**Estimated Effort**: 4 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/a2a.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/mod.rs`
**Depends On**: Task 11.36

#### Implementation Steps
1. Implement `GET /.well-known/agent.json` route.
2. Generate AgentCard from roko-serve configuration.
3. Add route to the serve router.

#### Verification Criteria
- [ ] `GET http://localhost:6677/.well-known/agent.json` returns a valid A2A Agent Card
- [ ] Card includes 4 skills with correct descriptions
- [ ] `cargo check -p roko-serve` compiles

---

### Task 11.38: Implement A2A JSON-RPC endpoint (server)
**Priority**: P3
**Estimated Effort**: 12 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/a2a.rs`
**Depends On**: Task 11.36, Task 11.37

#### Implementation Steps
1. Implement `POST /a2a` route accepting JSON-RPC 2.0 requests.
2. Dispatch based on method: `tasks/send`, `tasks/get`, `tasks/cancel`, `tasks/sendSubscribe`.
3. Map A2A skills to internal AgentRole dispatch.
4. Stream progress updates via SSE for `sendSubscribe`.
5. Return A2A-compliant task completion with artifacts.

#### Verification Criteria
- [ ] External A2A client sends `tasks/send` with code-implementation skill, roko executes and returns result
- [ ] `tasks/cancel` correctly cancels a running agent
- [ ] SSE stream shows progress updates for subscribed tasks
- [ ] Invalid JSON-RPC returns proper error response

---

### Task 11.39: Implement A2A client for external agent delegation
**Priority**: P3
**Estimated Effort**: 12 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/a2a_client.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/lib.rs`
**Depends On**: Task 11.36

#### Implementation Steps
1. Define `A2AClient` struct with `reqwest::Client`.
2. Implement `discover(card_url: &str) -> Result<AgentCard>`.
3. Implement `send_task(card: &AgentCard, skill_id: &str, input: &str)
   -> Result<A2ATask>`.
4. Add `[a2a.agents]` config section to roko.toml.
5. At dispatch time, check if task domain matches an A2A agent's skills.
   If so, delegate via A2A. Fallback to local agent on failure.

#### Verification Criteria
- [ ] Configure an external agent in roko.toml. When a matching task appears, roko delegates via A2A
- [ ] Delegation failure falls back to local agent
- [ ] `roko agent discover <url>` displays the external agent's capabilities

---

### Task 11.40: Implement CaMeL-style privileged/quarantined LLM split
**Priority**: P1
**Estimated Effort**: 12 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/safety/mod.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/safety/contract.rs`
**Depends On**: none

#### Context
Research: CaMeL (arxiv 2503.18813) -- solves 77% of AgentDojo with provable
security vs 84% undefended utility (7-point tax). Blocks ~67% of injections.
AutoInject (arxiv 2602.05746): 1.5B suffix-generator achieves 77.96% ASR.

AgentContract at `crates/roko-agent/src/safety/contract.rs` exists but falls
back to permissive default when YAML missing (CLAUDE.md: "Safety contracts
enforcement -- Partial").

#### Implementation Steps
1. Define `TrustDomain` enum: `Privileged`, `Quarantined`.
2. In the dispatch path, tag each LLM call with its trust domain:
   - Quarantined: agent implementation, tool output processing, user message
     handling, web content processing.
   - Privileged: gate evaluation, policy enforcement, system prompt assembly.
3. Enforce: quarantined LLM's chain-of-thought never influences privileged
   LLM's decisions (reasoning-blind classifier pattern).
4. Privileged calls should use a different model lineage from quarantined
   calls when feasible.
5. Log trust domain per LLM call in efficiency events.

#### Verification Criteria
- [ ] Gate evaluation calls are tagged as Privileged
- [ ] Agent implementation calls are tagged as Quarantined
- [ ] If the agent used Claude, the gate judge uses a different model family (when configured)
- [ ] Trust domain is visible in episode logs

---

### Task 11.41: Implement gate immutability from agent perspective
**Priority**: P1
**Estimated Effort**: 8 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/gate_service.rs`
**Depends On**: none

#### Context
Research: Darwin Godel Machine -- reward-hacked by removing monitoring tokens
to fake perfect scores. Verify gates must live outside the agent's modifiable
surface. Anthropic alignment-faking paper: Claude strategically complies 12%
of the time.

#### Implementation Steps
1. Load gate configs from a path NOT writable by agents: system gates from
   compiled defaults, user gates from `roko.toml` (loaded at startup, not
   re-read during run), generated gates from `.roko/learn/gate-evolution.json`.
2. During agent dispatch, do NOT pass gate config paths as tool-accessible files.
3. Validate gate config integrity: hash gate configs at startup with BLAKE3,
   verify hash before each gate run.
4. Log any attempt to modify gate config paths via agent tools.

#### Verification Criteria
- [ ] An agent that attempts to write to gate config files gets the modification ignored
- [ ] Gate config hash is verified before each gate run
- [ ] Integrity violation is logged as a warning

---

### Task 11.42: Implement pheromone signal sanitization
**Priority**: P1
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs` (or signal handling path)
**Depends On**: none

#### Context
Research: Multi-agent attack amplification (arxiv 2504.16489) -- structured
prompt rewriting raises mean harmfulness from 28.14% to 80.34% in Multi-Agent
Debate. Infectious Jailbreak: one adversarial image propagates to ~100% of
agents.

#### Implementation Steps
1. Before injecting pheromone signals into an agent's context, pass through
   a sanitization pipeline:
   - Strip any executable content (code blocks that look like tool calls).
   - Validate signal format against expected schema.
   - Truncate to maximum pheromone size (configurable, default 500 tokens).
2. Log sanitization events: what was stripped, from which source.
3. If a pheromone fails validation entirely, quarantine it and log a warning.

#### Verification Criteria
- [ ] A pheromone containing a fake tool call is sanitized (tool call stripped)
- [ ] A pheromone exceeding 500 tokens is truncated
- [ ] An invalid pheromone is quarantined, not injected
- [ ] Sanitization events are logged

---

### Task 11.43: Add knowledge provenance tags
**Priority**: P2
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-neuro/src/knowledge_store.rs`
**Depends On**: none

#### Context
Research: Karpathy's LLM Wiki pattern (April 2026) -- provenance tags
(extracted, inferred, ambiguous) and a lint pass that flags drift to speculation.

#### Implementation Steps
1. Add `provenance: Provenance` field to `KnowledgeEntry`.
2. Define `Provenance` enum: `Extracted`, `Inferred`, `Ambiguous`.
3. Default all new entries to `Extracted`.
4. When dream consolidation synthesizes knowledge, tag as `Inferred`.
5. When two sources disagree (HDC similarity > 0.9 but contradictory), tag
   as `Ambiguous`.
6. Implement `lint_provenance() -> Vec<ProvenanceWarning>` flagging entries
   drifting from Extracted to Inferred without acknowledgment.

#### Verification Criteria
- [ ] New knowledge entries from direct observation are tagged `Extracted`
- [ ] Synthesized entries from dream cycle are tagged `Inferred`
- [ ] `lint_provenance()` returns warnings for unacknowledged Inferred entries
- [ ] Provenance is visible in `roko knowledge query` output

---

### Task 11.44: Wire RLAIF/RLSF pattern into learning loop
**Priority**: P2
**Estimated Effort**: 8 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/feedback_service.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/runtime_feedback.rs`
**Depends On**: none

#### Context
Research: Absolute Zero Reasoner (NeurIPS 2025 Spotlight) -- trains from identity
seed with executor as only reward. Roko's Verify gate pipeline is a strict
superset. Dohmatob 2025: accumulate-only constraint prevents model collapse.

#### Implementation Steps
1. After gate passes, record the full trajectory as a positive training signal.
2. After gate fails, apply AgentHER (Hindsight Experience Replay): ask "what
   sub-goals did this trajectory actually achieve?" and record those as positive
   episodes for sub-goals.
3. Store trajectory quality scores alongside episodes.
4. Feed trajectory quality into CascadeRouter observations.
5. Implement accumulate-only constraint: synthetic/relabeled data always added
   to real data, never replaces it. Tag synthetic entries.

#### Verification Criteria
- [ ] After a gate pass, a positive trajectory is recorded with quality score
- [ ] After a gate fail, at least one sub-goal is identified and recorded
- [ ] CascadeRouter observations include trajectory quality
- [ ] Synthetic entries are tagged and never replace real entries

---

### Task 11.45: Wire dream consolidation cron trigger
**Priority**: P2
**Estimated Effort**: 8 hours
**Removes**: AP-NODREAM
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-dreams/src/runner.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/` (daemon or post-run hook)
**Depends On**: none

#### Context
DreamRunner at `crates/roko-dreams/src/runner.rs` is built (line 721:
`pub struct DreamRunner`) with `DreamRuntimeControls` but has no runtime
trigger (CLAUDE.md item 14: "Cold substrate archival -- built but not
instantiated at runtime (no cron/trigger)").

Research: AXIOM (arxiv 2505.24784) -- BMR with 7.6x sample efficiency.

#### Implementation Steps
1. Add `dream.schedule` to roko.toml config:
   `schedule = "after_10_runs"` or `"daily"` or `"manual"`.
2. In the daemon or post-run hook, check if dream cycle should trigger:
   count completed runs since last dream cycle.
3. If count >= threshold, spawn dream cycle as a background task.
4. Dream cycle performs: load episodes, run hypnagogia, run imagination,
   run consolidation, persist distilled knowledge.
5. Log dream cycle outcomes.

#### Verification Criteria
- [ ] After 10 completed runs, the dream cycle triggers automatically
- [ ] Dream cycle output appears in `.roko/neuro/knowledge.jsonl`
- [ ] Dream cycle does not block the next run (runs in background)
- [ ] `roko knowledge dream run` still works for manual triggering

---

### Task 11.46: Enhance HDC fingerprinting for routing
**Priority**: P2
**Estimated Effort**: 8 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-primitives/src/hdc.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/cascade_router.rs`
**Depends On**: none

#### Context
HdcVector at `crates/roko-primitives/src/hdc.rs` provides `fingerprint()` and
`hamming_similarity()`. CascadeRouter at `crates/roko-learn/src/cascade_router.rs`
does not use HDC for task-to-model matching.

Research: IBM NorthPole projects >100M HDC similarity searches/s on a single
chip. HRR-VSA (arxiv 2502.01657): 82.86% lower cross-entropy loss.

#### Implementation Steps
1. Compute capability fingerprints per model from historical episode data:
   aggregate HDC fingerprints of tasks where the model succeeded.
2. Compute requirement fingerprints per task from task context.
3. In CascadeRouter, add a routing stage: compute Hamming distance between
   task requirement fingerprint and each model's capability fingerprint.
4. Use HDC distance as a feature in the LinUCB context vector.
5. Track HDC routing accuracy.

#### Verification Criteria
- [ ] After 50+ episodes, models have non-trivial capability fingerprints
- [ ] HDC routing selects the model whose capability profile best matches the task
- [ ] HDC distance is included in the LinUCB context vector

---

### Task 11.47: Add HDC consistency check for adversarial detection
**Priority**: P3
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-primitives/src/hdc.rs`
**Depends On**: none

#### Context
Research: HDXpose -- 85.7% non-targeted ASR via Differential Evolution on
10,240-bit binary VSAs. Defense: bind HDC fingerprint to code hash.

#### Implementation Steps
1. When computing an HDC fingerprint for an agent/skill, also compute a
   BLAKE3 hash of the agent's source code or configuration.
2. Store `(hdc_fingerprint, code_hash)` pairs.
3. On subsequent runs, recompute both. If HDC has drifted (Hamming distance
   > threshold) but code hash is unchanged, flag as `AdversarialDriftWarning`.
4. Log the warning. Configurable threshold (default: Hamming > 500 of 10240 bits).

#### Verification Criteria
- [ ] A stable agent produces the same `(fingerprint, hash)` pair across runs
- [ ] Manually corrupting the fingerprint triggers an AdversarialDriftWarning
- [ ] Warning is informational, does not block execution

---

### Task 11.48: Implement tool-output sanitization
**Priority**: P0
**Estimated Effort**: 4 hours
**Removes**: AP-VERBOSE
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/tool_loop/result_msg.rs`
**Depends On**: none

#### Context
Tool outputs at `crates/roko-agent/src/tool_loop/result_msg.rs` are included
in agent context without truncation or filtering. `result_msg.rs` has
`initial_messages()`, `initial_messages_with_few_shot()`, and `append_results()`.

Research: Augment Code SWE-bench analysis -- 30-40% of wasted tokens come from
verbose tool outputs. MCPTox: tool-poisoning 84.2% with auto-approve.

#### Implementation Steps
1. Define `ToolOutputSanitizer` with configurable max output size
   (default: 4096 tokens).
2. In `append_results()`, before appending tool output:
   - Truncate to max size with "... (truncated, {N} tokens omitted)" suffix.
   - Strip ANSI escape codes.
   - Filter known injection patterns (tool calls embedded in output).
   - Validate UTF-8 encoding.
3. Log sanitization events when content is modified.
4. Make max size configurable per tool (some tools like `read_file` need
   larger output than `bash`).

#### Verification Criteria
- [ ] A `bash` tool output of 10K tokens is truncated to 4K with a truncation notice
- [ ] ANSI codes are stripped from all tool outputs
- [ ] A tool output containing a fake tool call has it sanitized
- [ ] Sanitization is visible in verbose logging

---

### Task 11.49: Implement event log fork-from-checkpoint
**Priority**: P2
**Estimated Effort**: 8 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/workflow_engine.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/main.rs`
**Depends On**: none

#### Context
Research: LangGraph 1.0 GA made durable state and fork-from-checkpoint
first-class. AGDebugger (CHI 2025): counterfactual log editing is the UX
developers actually want.

#### Implementation Steps
1. Ensure each task completion writes a checkpoint to the event log.
2. Add `--fork-from <task_id>` flag to `roko plan run`.
3. When specified, load event log up to the named task's last checkpoint,
   replay state, and continue from there.
4. Fork creates a new `run_id` but shares the event log prefix.
5. Forked runs preserve all learning data from the original run.

#### Verification Criteria
- [ ] `roko plan run --fork-from task-05` starts execution from after task-05
- [ ] The fork has a new run_id visible in logs
- [ ] Learning data from the original run is available in the fork

---

### Task 11.50: Add payload size guards to event logger
**Priority**: P2
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/jsonl_logger.rs`
**Depends On**: none

#### Context
Research: LLM workloads easily blow Temporal's history-size budget. Payload
codecs and offloading are mandatory.

#### Implementation Steps
1. Define `MAX_INLINE_PAYLOAD_SIZE = 1_048_576` (1 MB).
2. Before writing an event, check payload size.
3. If > threshold: write payload to `.roko/events/payloads/{event_id}.json`,
   replace in event with `{ "$ref": "payloads/{event_id}.json" }`.
4. On event log read, resolve `$ref` references transparently.
5. Add payload GC: clean up unreferenced payload files older than 30 days.

#### Verification Criteria
- [ ] An event with a 5 MB tool output stores the output in a separate file
- [ ] The event log entry contains a `$ref` instead of the full payload
- [ ] Reading the event log resolves references transparently

---

### Task 11.51: Add model-heterogeneity enforcement in gate judges
**Priority**: P1
**Estimated Effort**: 4 hours
**Removes**: AP-SAMEFAM
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/gate_service.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/gate_runner.rs` (enrich_rung_config callsite)
**Depends On**: none

#### Context
`enrich_rung_config` is called from `crates/roko-cli/src/orchestrate.rs` and
`crates/roko-cli/src/gate_runner.rs`. Oracle gate rungs (4-6) can currently
use the same model family as the task agent.

Research: ICLR 2026 (arxiv 2502.01534) -- evaluation breaks when judge and
generator share a lineage. "Great Models Think Alike" (ICML 2025) -- debate
value collapses to zero when debater models share weights.

#### Implementation Steps
1. In `enrich_rung_config()` or its caller, accept the agent's model slug.
2. Determine the agent's model family (Claude, GPT, Gemini, etc.) by prefix.
3. For oracle rungs (4-6), select a judge model from a different family.
4. If no alternative model is configured, log a warning and proceed with
   the same family (degraded mode, not a hard failure).
5. Record judge model in the gate verdict for auditability.

#### Verification Criteria
- [ ] Agent uses Claude Sonnet -> oracle judge uses GPT or Gemini
- [ ] If only Claude models are configured, a warning is logged
- [ ] Judge model is visible in gate verdict logs

---

### Task 11.52: Implement force_backend override learning (UX34)
**Priority**: P1
**Estimated Effort**: 4 hours
**Removes**: AP-NOLEARN
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/cascade_router.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch/model_routing.rs`
**Depends On**: none

#### Context
`force_backend` appears in 13 files across the codebase. CascadeRouter does
not learn from manual overrides (CLAUDE.md item 15: "UX34: force_backend
override learning").

#### Implementation Steps
1. In the dispatch path, detect when `force_backend` is set.
2. Record the override as a strong observation in CascadeRouter: the user
   explicitly chose this model for this task type.
3. Weight override observations 3x compared to automatic observations
   (configurable multiplier).
4. After accumulating 5+ overrides for the same task category, adjust
   CascadeRouter's static routing table to prefer the user's choice.
5. Add `roko learn tune routing --show-overrides` to display learned overrides.

#### Verification Criteria
- [ ] After 5 `--force-backend cerebras` overrides on "simple fix" tasks, CascadeRouter routes "simple fix" tasks to Cerebras by default
- [ ] Override learning is visible in `cascade-router.json` observations
- [ ] `roko learn tune routing --show-overrides` lists learned preferences

---

### Task 11.53: Wire knowledge store consultation into CascadeRouter
**Priority**: P1
**Estimated Effort**: 4 hours
**Removes**: AP-NEURO
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/cascade_router.rs`
**Depends On**: Task 11.1

#### Context
CascadeRouter already has `route_with_knowledge()` at line 856 and
`KnowledgeRoutingAdvice` at `crates/roko-learn/src/cascade/types.rs`. But the
live dispatch path does not construct `KnowledgeRoutingAdvice` from the neuro
store (CLAUDE.md item 13: "Knowledge-informed agent routing -- neuro store not
yet consulted for model selection").

#### Implementation Steps
1. Before routing, query `KnowledgeStore` for entries about model performance
   on the current task's domain/type using `query_kind()`.
2. Construct `KnowledgeRoutingAdvice` from matching entries.
3. Pass to `route_with_knowledge()` (already implemented).
4. If knowledge contradicts bandit observations, weight bandit higher.

#### Verification Criteria
- [ ] If knowledge store contains "cerebras fails on async Rust" with high confidence, CascadeRouter avoids cerebras for async Rust tasks
- [ ] Knowledge consultation adds < 5ms to routing latency
- [ ] Bandit observations override stale knowledge

---

### Task 11.54: Implement OTel gen_ai.* span emission
**Priority**: P3
**Estimated Effort**: 12 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/otel_emitter.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/lib.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/Cargo.toml`
**Depends On**: none

#### Context
OTel gen_ai.* semantic conventions (v1.37+) define standard attributes for LLM
observability. Langfuse, Phoenix, Honeycomb all support OTLP ingestion.

#### Implementation Steps
1. Add `opentelemetry`, `opentelemetry-otlp`, `opentelemetry-sdk` to
   roko-runtime dependencies (feature-gated under `otel`).
2. Define `OtelEmitter` struct wrapping a tracer provider.
3. On each agent dispatch, create a span with gen_ai.* attributes.
4. On each gate evaluation, create a child span.
5. Emit to configurable OTLP endpoint (from roko.toml).
6. Feature-gate: only active when `[observability]` config is present.

#### Verification Criteria
- [ ] With `[observability] provider = "otlp-generic"` configured, OTel spans are emitted
- [ ] Spans include all gen_ai.* attributes per v1.37+ spec
- [ ] Without observability config, no OTel overhead (feature-gated)
- [ ] JSONL logging continues alongside OTel (not replaced)

---

### Task 11.55: Add vendor-neutral observability config
**Priority**: P3
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/schema.rs` (or serve.rs)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/otel_emitter.rs`
**Depends On**: Task 11.54

#### Implementation Steps
1. Add `[observability]` section to roko.toml schema:
   `provider`, `endpoint`, `protocol`, `api_key_env`.
2. Configure OTLP exporter based on provider.
3. Validate config at startup.

#### Verification Criteria
- [ ] Changing `provider` from `langfuse` to `honeycomb` requires only config change
- [ ] Missing API key produces a clear error at startup
- [ ] `roko config validate` checks observability config

---

### Task 11.56: Emit gate results as structured compliance events
**Priority**: P3
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/gate_service.rs`
**Depends On**: Task 11.54

#### Implementation Steps
1. After each gate rung completes, emit an OTel span with structured attributes:
   `gate.rung.id`, `gate.verdict`, `gate.agent_id`, `gate.evidence`, etc.
2. For Article 50 compliance: include `ai.provenance.model`,
   `ai.provenance.timestamp`, `ai.provenance.confidence`.

#### Verification Criteria
- [ ] Gate results appear as OTel spans with all structured attributes
- [ ] A SIEM tool can filter for `gate.verdict = fail` events

---

### Task 11.57: Wire experiment feedback into CascadeRouter
**Priority**: P1
**Estimated Effort**: 4 hours
**Removes**: AP-NOEXP
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/cascade_router.rs`
**Depends On**: none

#### Context
ExperimentStore at `crates/roko-learn/src/prompt_experiment.rs` runs A/B
experiments but outcomes are not fed back to CascadeRouter routing weights.

#### Implementation Steps
1. After an experiment arm concludes, extract the model and prompt variant.
2. Feed the outcome into CascadeRouter as an observation with experiment context.
3. Winning experiment arms boost the associated model's routing weight.
4. Losing arms reduce the weight.

#### Verification Criteria
- [ ] An experiment with 3 arms on the same task type: after 10 trials, the winning model has the highest routing weight
- [ ] Experiment observations are visible in `cascade-router.json`
- [ ] The experiment store and router are no longer operating independently

---

### Task 11.58: Implement Bayesian Model Reduction in dream cycle
**Priority**: P3
**Estimated Effort**: 8 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-dreams/src/cycle.rs`
**Depends On**: Task 11.45

#### Context
Research: AXIOM (arxiv 2505.24784) -- BMR with 7.6x sample efficiency, 39x
faster wall-clock.

DreamCycle at `crates/roko-dreams/src/cycle.rs` has `pub fn run_dream`.

#### Implementation Steps
1. During consolidation, compute evidence for each knowledge entry: how many
   episodes support vs contradict it.
2. Apply BMR: score candidate knowledge models from accumulated posteriors.
3. Prune low-evidence entries (evidence < threshold).
4. Merge near-duplicate entries (HDC similarity > 0.95).
5. Log pruning decisions.

#### Verification Criteria
- [ ] After dream consolidation, knowledge store has fewer entries but higher average confidence
- [ ] Entries with zero supporting episodes are pruned
- [ ] Near-duplicate entries are merged

---

### Task 11.59: Implement hindsight relabeling in dream cycle
**Priority**: P3
**Estimated Effort**: 8 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-dreams/src/cycle.rs`
**Depends On**: Task 11.45

#### Context
Research: AgentHER -- +7-12 percentage points and 2x data efficiency on
WebArena/ToolBench. With Verify gates as the relabeling oracle, failed runs
become positive episodes for sub-goals.

#### Implementation Steps
1. Load failed episodes from the episode log.
2. For each failed episode, analyze the trajectory to identify sub-goals
   actually achieved.
3. Create new positive episodes for those sub-goals with reduced scope.
4. Store relabeled episodes with `provenance: Inferred` and
   `source: hindsight_relabeling`.
5. Feed relabeled episodes into the learning loop.

#### Verification Criteria
- [ ] A failed episode that correctly identified right files produces a positive sub-goal episode
- [ ] Relabeled episodes are tagged as Inferred
- [ ] The learning loop's positive trajectory count increases after dream consolidation

---

### Task 11.60: Implement CMP scoring for agent variants
**Priority**: P3
**Estimated Effort**: 8 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs` (or `crates/roko-learn/`)
**Depends On**: none

#### Context
Research: Huxley Godel Machine (ICLR 2026 oral) -- CMP scores agent variants
by aggregate descendant performance, not the variant's own output.

#### Implementation Steps
1. Track agent lineage: which configuration produced which outcomes, and which
   configurations descended from which.
2. Define CMP score: average gate pass rate of all tasks dispatched by agents
   using this configuration AND all descendant configurations.
3. When evaluating which configuration to use, prefer higher CMP scores.
4. Store CMP scores in `.roko/learn/agent-variants.json`.
5. Add `roko learn agents` CLI showing variant CMP scores.

#### Verification Criteria
- [ ] An agent configuration with good outcomes AND good descendant performance has a higher CMP score than one with only good individual performance
- [ ] CMP scores persist across runs
- [ ] `roko learn agents` displays variant lineage with CMP scores

---

### Task 11.61: Add multi-dimensional collective intelligence measurement
**Priority**: P3
**Estimated Effort**: 8 hours
**Removes**: AP-SINGULAR
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/cfactor.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs`
**Depends On**: none

#### Context
CFactorSummary at `crates/roko-core/src/cfactor.rs` is a single scalar.

Research: PLOS One 2024 replication failure -- single scalar is unreliable.
Williams-Beer/Broja PID: decompose into synergy (true collective), redundancy
(wasted), unique (specialization). Caveat: PID for n>=3 is mathematically
broken; use binary-only.

#### Implementation Steps
1. Implement binary PID (Williams-Beer): decompose mutual information between
   two agent outputs into synergy, redundancy, and unique.
2. For n >= 3 agents, use pairwise PID (binary-only).
3. Add `synergy`, `redundancy`, `unique_info` fields to `CFactorSummary`.
4. Gate multi-agent scaling on synergy threshold: if synergy < 0.1, recommend
   reducing agent count.
5. Log PID components in efficiency events.

#### Verification Criteria
- [ ] After a multi-agent run with 3 agents, CFactorSummary includes synergy, redundancy, and unique components
- [ ] High-redundancy runs produce a recommendation to reduce agent count
- [ ] PID components are visible in `roko learn all` output

---

### Task 11.62: Implement distributed causal discovery over episodes
**Priority**: P3
**Estimated Effort**: 12 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-neuro/src/episode_completion.rs`
**Depends On**: none

#### Context
Research: DCILP (AAAI 2025) -- ~270x speedup over DAGMA. Each Block estimates
its local Markov blanket; a merge produces the global structural causal model.

`episode_completion.rs` exists at `crates/roko-neuro/src/episode_completion.rs`.

#### Implementation Steps
1. For each task, estimate its local Markov blanket from episode outcomes:
   which other tasks' outcomes statistically predict this task's success.
2. Merge local blankets into a global structural causal model (DAG).
3. Compare causal DAG with the declared dependency DAG in plans.
4. Flag spurious dependencies: tasks declared as dependent but with no causal
   relationship in the data.
5. Flag missing dependencies: tasks with causal relationships not declared.
6. Output recommendations: "task-07 does not actually depend on task-05
   (p = 0.02); consider parallelizing."
7. Persist to `.roko/learn/causal-model.json`.

#### Verification Criteria
- [ ] After 20+ plan runs, the causal model identifies at least one spurious dependency
- [ ] Recommendations are actionable: include task IDs and p-values
- [ ] `cargo test -p roko-neuro` passes

---

### Task 11.63: Add Article 50 compliance posture endpoint
**Priority**: P3
**Estimated Effort**: 4 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/compliance.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/mod.rs`
**Depends On**: Task 11.54

#### Implementation Steps
1. `GET /api/compliance/article50` returns AI disclosure status, logging status,
   retention period, provenance metadata status.
2. `GET /api/compliance/report` generates a compliance report.
3. Add `transparency_mode` flag to agent configuration.

#### Verification Criteria
- [ ] `GET /api/compliance/article50` returns a structured JSON report
- [ ] Report includes logging status and retention period
- [ ] `transparency_mode` is configurable in roko.toml

---

### Task 11.64: Add C2PA-aligned metadata to agent outputs
**Priority**: P3
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/jsonl_logger.rs`
**Depends On**: none

#### Implementation Steps
1. On each agent output, attach metadata fields: `ai.generated: true`,
   `ai.model`, `ai.timestamp`, `ai.agent_id`, `ai.confidence`,
   `ai.provenance_version: "c2pa-draft-2026"`.
2. Include metadata in JSONL events.
3. Include metadata in A2A task artifacts (if A2A wired).

#### Verification Criteria
- [ ] Every agent output event in JSONL includes provenance metadata
- [ ] Metadata fields match C2PA-aligned naming conventions

---

### Task 11.65: Add ERC-8004 identity fields to agent configuration
**Priority**: P3
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/agent.rs`
**Depends On**: Task 11.36

#### Context
ERC-8004 is live on Ethereum mainnet (Jan 29, 2026). ~80-150K agents registered
by April 2026.

#### Implementation Steps
1. Add optional fields to agent config in roko.toml:
   ```toml
   [agent.identity]
   erc8004_id = "0x..."
   capabilities = ["code-implementation", "code-review"]
   reputation_tier = "verified"
   ```
2. If `erc8004_id` is set, include it in the A2A Agent Card.
3. Validate format (Ethereum address format).
4. Field is entirely optional.

#### Verification Criteria
- [ ] Agent config accepts `[agent.identity]` section without error
- [ ] ERC-8004 ID appears in A2A Agent Card when set
- [ ] Invalid Ethereum address format produces a config validation error
- [ ] Field is entirely optional; omitting it changes nothing

---

## Summary

| Phase | Tasks | Effort (hours) | Key Deliverables |
|-------|-------|---------------|-----------------|
| 1: Foundations | 11.1 -- 11.12 | ~80 | Memory layer, context adaptation, cost optimization, semantic cache |
| 2: Self-Improvement | 11.13 -- 11.24 | ~92 | Self-improving gates, agent debugging, interactive steering |
| 3: Multi-Agent | 11.25 -- 11.30 | ~80 | Competitive proposals, swarm, speculative execution |
| 4: Cross-Cutting | 11.31 -- 11.65 | ~208 | Cross-project learning, A2A, CaMeL safety, OTel, compliance, research features |
| **Total** | **65** | **~460** | |

### Dependency Graph (Critical Path)

```
11.1 (MemoryLayer) -> 11.2 (Retrieval) -> 11.3 (Wire to prompt)
                                        -> 11.22 (Confidence)
11.1 -> 11.4 (Memory update)
11.1 -> 11.53 (Knowledge routing)

11.5 (Disclosure) + 11.6 (Profile) -> 11.11 (Compression)

11.9 (Cache exact) -> 11.10 (Cache fuzzy)

11.13 (GateEvolver) -> 11.14 (Wire gates)

11.17 (Taxonomy) -> 11.18 (Hypotheses) -> 11.19 (Wire debug)

11.20 (Steering types) -> 11.21 (Channel) -> 11.23 (HTTP)
                                           -> 11.24 (TUI)
11.20 -> 11.22 (Confidence) -> 11.23, 11.24

11.31 (Global dir) + 11.32 (Domain) -> 11.33 (Tiered store)
  -> 11.34 (Promotion) -> 11.35 (Export/import)

11.36 (A2A types) -> 11.37 (Card) -> 11.38 (Server)
11.36 -> 11.39 (Client)
11.36 -> 11.65 (ERC-8004)

11.54 (OTel) -> 11.55 (Config) -> 11.56 (Compliance events)
                                -> 11.63 (Article 50)

11.45 (Dream trigger) -> 11.58 (BMR) + 11.59 (Hindsight)
```

### Implementation Priority Order

**Immediate (P0)**:
11.48 (Sanitize), 11.8 (Budget), 11.1 (Memory struct), 11.7 (Pareto routing),
11.2 (Retrieval), 11.3 (Wire memory), 11.4 (Memory update)

**Week 1-2 (P1)**:
11.5 (Disclosure), 11.6 (Profile), 11.9 (Cache), 11.11 (Compress),
11.12 (Learn costs), 11.13 (GateEvolver), 11.15 (DiffAnalyzer),
11.16 (Effectiveness), 11.17 (Taxonomy), 11.40 (CaMeL), 11.41 (Gate immutability),
11.42 (Pheromone sanitize), 11.51 (Judge heterogeneity), 11.52 (UX34),
11.53 (Knowledge routing), 11.57 (Experiment feedback)

**Week 3-4 (P2)**:
11.10 (Fuzzy cache), 11.14 (Wire gates), 11.18 (Hypotheses),
11.19 (Debug), 11.20 (Steering types), 11.21 (Channel),
11.22 (Confidence), 11.23 (HTTP steer), 11.25 (Competitive),
11.28 (Speculative fix), 11.29 (Prefetch), 11.30 (Density threshold),
11.31 (Global dir), 11.32 (Domain), 11.36 (A2A types), 11.37 (Card),
11.43 (Provenance), 11.44 (RLAIF), 11.45 (Dream trigger),
11.46 (HDC routing), 11.49 (Fork), 11.50 (Payload guards)

**Month 2 (P3)**:
11.24 (TUI steer), 11.26 (Swarm), 11.27 (Specialist),
11.33-35 (Cross-project learning), 11.38-39 (A2A server/client),
11.47 (HDC adversarial), 11.54-56 (OTel), 11.58-59 (BMR, hindsight),
11.60-62 (CMP, PID, causal), 11.63-65 (Compliance, C2PA, ERC-8004)
