# Roko-Trustworthy Branch: Exhaustive Audit Catalogue

> **Branch**: `wp-arch2` (28 commits, RT00–RT23)
> **Date**: 2026-04-26
> **Purpose**: Reference document for upcoming refactor/redesign. No code changes.

---

## Table of Contents

- [Category A: Dead Code — Built but Never Wired](#category-a-dead-code--built-but-never-wired)
- [Category B: Implementation Gaps & Stubs](#category-b-implementation-gaps--stubs)
- [Category C: Quality & Maintainability](#category-c-quality--maintainability)
- [Category D: Correctness Concerns](#category-d-correctness-concerns)
- [Category E: Extensibility Improvements](#category-e-extensibility-improvements)
- [Implementation Priority](#implementation-priority)

---

## Category A: Dead Code — Built but Never Wired

These modules exist as standalone implementations with types, tests, and persistence but are **not called from any production code path** (orchestrate.rs, agent dispatch, or CLI commands).

### A1. Extension Framework (`roko-core/src/extension.rs`)

- **Issue**: `ExtensionChain`, `Extension` trait, all 8 layers — zero imports outside the file itself. Not called from orchestrate.rs, agent dispatch, or any runtime path.
- **Impact**: 539 lines of dead code. The extension hook system is well-designed but entirely disconnected.
- **Fix**: Wire `ExtensionChain` into the agent dispatch loop in orchestrate.rs. At minimum, call `run_pre_inference`/`run_post_inference` around agent invocations.
- **Files**:
  - `crates/roko-core/src/extension.rs`
  - `crates/roko-cli/src/orchestrate.rs`
- **Priority**: P0

### A2. Knowledge Admission Controller (`roko-neuro/src/admission.rs`)

- **Issue**: `KnowledgeAdmissionController` is exported from roko-neuro but never instantiated in orchestrate.rs or any CLI command. Candidates are not evaluated against admission thresholds.
- **Impact**: ~1,285 lines. The admission gate exists but knowledge writes bypass it entirely.
- **Fix**: Wire admission evaluation into the post-gate reflection path in orchestrate.rs where knowledge candidates are generated.
- **Files**:
  - `crates/roko-neuro/src/admission.rs`
  - `crates/roko-cli/src/orchestrate.rs`
- **Priority**: P0

### A3. Connector and Feed Registries (`roko-core`)

- **Issue**: `ConnectorRegistry` and `FeedRegistry` are defined in roko-core and used by roko-serve route handlers, but the registries are only in-memory within AppState. No connectors or feeds are ever registered by orchestrate.rs or any agent. The HTTP routes work but serve empty registries.
- **Impact**: ~493 lines of types + routes that can't produce real data.
- **Fix**: Add connector/feed registration during `roko serve` startup from config, or during agent dispatch.
- **Files**:
  - `crates/roko-core/src/connector.rs`
  - `crates/roko-core/src/feed.rs`
  - `crates/roko-serve/src/state.rs`
- **Priority**: Defer

### A4. Contextual Bandit Policy (`roko-learn/src/contextual_bandit.rs`)

- **Issue**: The bandit is imported in `routes/learning.rs` for stats exposure but never called for actual decision-making in orchestrate.rs or gateway routing. `select_model_via_router` in gateway.rs uses the old `CascadeRouter` directly, not the bandit layer.
- **Impact**: ~1,372 lines of sophisticated ML code that doesn't influence any runtime decision.
- **Fix**: Integrate `ContextualBanditPolicy::select` into the model routing path in gateway.rs and/or orchestrate.rs dispatch.
- **Files**:
  - `crates/roko-learn/src/contextual_bandit.rs`
  - `crates/roko-serve/src/routes/gateway.rs`
- **Priority**: P0

---

## Category B: Implementation Gaps & Stubs

### B1. Gateway token tracking is placeholder

- **File**: `crates/roko-serve/src/routes/gateway.rs:269-270`
- **Issue**: `estimate_tokens()` uses `len / 4` heuristic. `ModelStats` in `gateway_stats` always reports `tokens_in: 0, tokens_out: 0, cost_usd: 0.0`. Cache hit rate is hardcoded to 0.
- **Fix**: Accumulate actual token counts in AppState from completion responses. Add per-model token counters.
- **Priority**: P3

### B2. Gateway batch requests process sequentially

- **File**: `crates/roko-serve/src/routes/gateway.rs:437-487`
- **Issue**: `batch_submit` spawns one task that processes requests in a `for` loop. No parallelism for batch inference.
- **Fix**: Use `tokio::JoinSet` or `futures::stream::FuturesUnordered` for concurrent dispatch within a batch, with configurable concurrency limit.
- **Priority**: Defer

### B3. Batch status total/completed are redundant

- **File**: `crates/roko-serve/src/routes/gateway.rs:552-553`
- **Issue**: `completed` is calculated as `successes + failures` which always equals `total` when the batch is done. The `total: 0` while processing gives no progress indication.
- **Fix**: Track completed count incrementally in the OperationHandle so polling shows real progress.
- **Priority**: Defer

### B4. Gateway hardcodes routing context

- **File**: `crates/roko-serve/src/routes/gateway.rs:615-632`
- **Issue**: `select_model_via_router` creates a fixed `RoutingContext` with `TaskCategory::Implementation`, `complexity: Standard`, etc. regardless of the actual request.
- **Fix**: Derive routing context from the completion request metadata (agent_id, optional task hints in request body).
- **Priority**: P1

### B5. CognitiveWorkspace not attached to episodes

- **Issue**: CognitiveWorkspace is imported in roko-compose and roko-orchestrator/event_log but never persisted alongside episodes in orchestrate.rs.
- **Fix**: Serialize workspace to the episode record or sidecar file.
- **Priority**: P3

---

## Category C: Quality & Maintainability

### C1. Duplicate atomic-write helpers

- Multiple modules implement `path.with_extension("json.tmp")` + write + rename:
  - `orchestrate.rs` (at least 3 locations)
  - `error_pattern_store.rs` (custom `unique_tmp_path`)
  - `contextual_bandit.rs`
- **Fix**: Extract a shared `atomic_write_json(path, &T) -> io::Result<()>` utility into roko-core or roko-fs.
- **Priority**: P2

### C2. `serde_json::Value` overuse in Extension trait

- **File**: `crates/roko-core/src/extension.rs`
- **Issue**: Every hook parameter is `&mut serde_json::Value`. This loses type safety — extensions can't know the shape of observations, requests, or responses at compile time.
- **Fix**: Define typed hook parameters (e.g., `InferenceRequest`, `InferenceResponse`, `Observation`) and use those instead of `Value`. Keep `Value` only as a fallback for truly dynamic data.
- **Priority**: Defer

### C3. Large monolithic files

- `orchestrate.rs` is enormous (22K+ lines based on test line numbers). The trustworthy work added more code to it.
- `contextual_bandit.rs` is 1,372 lines mixing policy logic, persistence, and stats.
- **Fix**: Extract submodules. For orchestrate.rs: split gate classification, failure pattern handling, and reflection into separate files. For bandit: separate persistence from policy logic.
- **Priority**: P3

### C4. Error pattern store uses Vec scan for lookup

- **File**: `crates/roko-learn/src/error_pattern_store.rs:313`
- **Issue**: `observe_gate_failure` does a linear scan `self.patterns.iter_mut().find(|p| p.key == key)` on every observation.
- **Fix**: Use a `HashMap<String, usize>` index alongside the Vec, or switch the store to `HashMap<String, ErrorPattern>`.
- **Priority**: P2

### C5. `plan_ids`/`task_ids` dedup uses linear scan

- **File**: `crates/roko-learn/src/error_pattern_store.rs:498-502`
- **Issue**: `push_unique` does `Vec::contains` (O(n)) for dedup.
- **Fix**: Use `BTreeSet<String>` or `HashSet<String>` for `plan_ids`/`task_ids`.
- **Priority**: P2

---

## Category D: Correctness Concerns

### D1. Gateway loads CascadeRouter from disk on every request

- **File**: `crates/roko-serve/src/routes/gateway.rs:613-614`
- **Issue**: `select_model_via_router` calls `CascadeRouter::load_or_new(&cascade_path, ...)` on every inference request. File I/O on every request is slow and could race with concurrent writes.
- **Fix**: Cache the CascadeRouter in AppState with periodic reload, or load once at startup and refresh on config change.
- **Priority**: P1

### D2. Gateway loads config on every request

- **File**: `crates/roko-serve/src/routes/gateway.rs:234, 343, 387`
- **Issue**: `state.load_roko_config()` is called multiple times per handler. If this reads from disk, it's wasteful.
- **Fix**: Ensure `load_roko_config()` is cached (it may already be — verify).
- **Priority**: P1

### D3. Knowledge admission thresholds not validated

- **File**: `crates/roko-neuro/src/admission.rs`
- **Issue**: `min_admission_confidence` is configurable but there's no validation that it's in [0.0, 1.0]. A value > 1.0 would block all admissions; a negative value would admit everything.
- **Fix**: Add bounds validation in the controller constructor.
- **Priority**: P3

---

## Category E: Extensibility Improvements

### E1. Extension trait should be async

- **File**: `crates/roko-core/src/extension.rs`
- **Issue**: All hooks are synchronous (`fn` not `async fn`). Extensions that need network calls (MCP, knowledge queries, tool validation) can't be implemented without blocking.
- **Fix**: Make hooks async using `async_trait` or return `Pin<Box<dyn Future>>`. This is a breaking change but the trait has zero external implementations today.
- **Priority**: Defer

### E2. Acceptance contract should support custom gate kinds

- **File**: `crates/roko-gate/src/acceptance_contract.rs:355-368`
- **Issue**: `GateRequirementKind` is a closed enum (Compile/Test/Lint/Review/Custom). Future gate types (security scan, coverage, perf benchmark) require adding variants.
- **Fix**: Add a `Named(String)` variant or use a string-based kind field alongside the enum.
- **Priority**: Defer

### E3. Error pattern store needs eviction

- **File**: `crates/roko-learn/src/error_pattern_store.rs`
- **Issue**: Patterns accumulate forever. No TTL, no max-size, no resolved-pattern cleanup.
- **Fix**: Add `gc(max_age: Duration, max_patterns: usize)` that removes old resolved patterns and caps total count.
- **Priority**: P3

---

## Implementation Priority

| Priority | Items | Rationale |
|----------|-------|-----------|
| **P0 — Wire dead code** | A1, A2, A4 | Highest-value changes: 3K+ lines of code that should be influencing runtime behavior |
| **P1 — Fix correctness** | D1, D2, B4 | Gateway per-request file I/O and hardcoded routing context undermine production reliability |
| **P2 — Dedup utilities** | C1, C4, C5 | Quick wins for maintainability |
| **P3 — Quality** | B1, B5, C3, D3, E3 | Token tracking, workspace persistence, file splitting, validation, eviction |
| **Defer** | A3, B2, B3, C2, E1, E2 | Connector/feed wiring, batch parallelism, async extensions — lower urgency |

---

## Summary

| Category | Count | Est. Dead/Stub LOC |
|----------|-------|--------------------|
| A — Dead code | 4 | ~3,689 |
| B — Gaps/stubs | 5 | — |
| C — Quality | 5 | — |
| D — Correctness | 3 | — |
| E — Extensibility | 3 | — |
| **Total findings** | **20** | |
