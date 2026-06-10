# Inference Dispatch: Task Breakdown

> Unify the inference dispatch subsystem. Wire CascadeRouter to live paths,
> consolidate stream parsers, eliminate direct env key reads, enforce budgets,
> add provider health/retry, route ACP through providers, add thinking token
> accounting, and decompose the orchestrate.rs god object. 38 tasks across 10
> phases.
>
> Sources: `impl/03-INFERENCE-DISPATCH.md`, `19-DISPATCH-{AUDIT,ISSUES,PLAN}.md`,
> `03-PROVIDER-AND-AGENT-AUDIT.md`, codebase analysis

---

## Overview

The inference dispatch subsystem has 9 fragmented paths for calling LLMs. Only 2
go through the canonical `ModelCallService`. The `CascadeRouter` (a sophisticated
LinUCB contextual bandit) has zero live callers -- every call site passes `None`.
Four copies of the Claude stream-json parser exist with independent truncation
logic. Two live code paths read API keys directly from environment variables.
Budget enforcement defaults to `None` (unlimited spend). Provider health is
tracked in `roko-learn` but never consulted before dispatch.

**Target state**: All LLM calls route through `ModelCallService`. CascadeRouter
provides adaptive model selection for `roko run`, `roko chat`, and `roko plan
run`. One stream parser. One budget enforcement layer. Provider health gates
dispatch. No bare `Command::new("claude")` outside provider adapters. No direct
`std::env::var("API_KEY")` outside provider adapters.

| Current State | Component | Target |
|---|---|---|
| 9 dispatch paths | Invocation | 1 (`ModelCallService`) |
| `None` at all call sites | CascadeRouter | Loaded, consulted, persisted |
| 4 copies | Stream parser | 1 (`parse_stream_line`) |
| `BudgetCell::new(None)` | Budget | `BudgetConfig` from `roko.toml` |
| Not consulted | Provider health | Pre-dispatch check + failover |
| 2 bare subprocess spawns | ACP | Provider adapter system |
| Missing `thinking_tokens` | Usage tracking | Full thinking token accounting |
| 22,522 LOC dead code | orchestrate.rs | Valuable patterns extracted, dead code deleted |

---

## Anti-Patterns to Remove

| ID | Anti-Pattern | Where | Severity |
|---|---|---|---|
| AP-NOROUTER | CascadeRouter zero live callers; all sites pass `None` | `resolve_effective_model_key()` at `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/model_selection.rs:192` | Critical |
| AP-4PARSE | 4 copies of Claude stream-json parsing with independent 4096-byte truncation | `translate/mod.rs:186-187`, `translate/mod.rs:238-239`, `chat.rs:660-661`, `dispatch_direct.rs` (feature-gated) | High |
| AP-ENVKEY | Direct `std::env::var("API_KEY")` bypassing provider config | `episode_completion.rs:46`, `web_search.rs:332` | High |
| AP-NOBUDGET | `BudgetCell::new(None)` default = unlimited spend | `model_call_service.rs:126` | High |
| AP-BARESUBPROC | `Command::new("claude")` in ACP bypassing provider adapters | `roko-acp/src/runner.rs:1849`, `roko-acp/src/bridge_events.rs:1110` | High |
| AP-GODOBJ | 22,522-line orchestrate.rs with dead `PlanRunner` | `crates/roko-cli/src/orchestrate.rs` | Critical |
| AP-NOTHINK | `UsageObservation` missing `thinking_tokens` field | `crates/roko-agent/src/usage.rs` | Medium |
| AP-QUIRK | Per-provider boolean flags instead of structured quirks | `openai_compat_backend.rs:54,58,62` | Medium |
| AP-NOHEALTH | `ProviderHealthTracker` exists but never gates dispatch | `model_call_service.rs` has no health pre-check | High |
| AP-HARDCODE | 8 hardcoded model strings bypassing config | `run.rs:530`, `dispatch_direct.rs:208`, `auth_detect.rs:42` | High |

---

## Phase 0: Wire CascadeRouter to Live Paths

Everything adaptive depends on the router being loaded, consulted, and persisted.

### Task 3.1: Add load/save Helpers for CascadeRouter in model_selection.rs
**Priority**: P0
**Estimated Effort**: 2 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/model_selection.rs`
**Depends On**: none

#### Context
`CascadeRouter` is a 3-stage LinUCB contextual bandit at `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/cascade_router.rs`. It has `load_or_new(path, model_slugs)` (line 1596) and `save(path)` (line 1578) methods. The serve runtime loads it at `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/serve_runtime.rs:522`. The runner v2 loads it at `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/types.rs:1306`. Both use inline `load_or_new` calls with ad-hoc model slug extraction.

`model_selection.rs` already imports `CascadeRouter` at line 7 (`use roko_learn::cascade_router::CascadeRouter;`) and accepts `Option<&CascadeRouter>` in `resolve_effective_model` (line 144). But there is no centralized load/save utility, so every call site reimplements slug extraction + path construction.

`RokoConfig::effective_models()` returns the merged model map. The convention is `.roko/learn/cascade-router.json` for the persistence path.

#### Implementation Steps
1. Add `pub fn load_cascade_router(workdir: &Path, config: &RokoConfig) -> CascadeRouter`:
   - Build path as `workdir.join(".roko/learn/cascade-router.json")`
   - Extract model slugs from `config.effective_models().keys()`, sort and dedup
   - If slugs are empty and `config.agent.default_model` is non-empty, push it as the sole slug
   - Call `CascadeRouter::load_or_new(&path, model_slugs)`
   - This matches the pattern at `crates/roko-cli/src/commands/plan.rs:311-323`
2. Add `pub fn save_cascade_router(workdir: &Path, router: &CascadeRouter) -> std::io::Result<()>`:
   - Build path as `workdir.join(".roko/learn/cascade-router.json")`
   - Ensure parent directory exists via `std::fs::create_dir_all`
   - Call `router.save(&path)`
3. Add a `#[cfg(test)]` roundtrip test: create a temp dir, load (creates fresh), save, load again, verify model slugs are preserved

#### Design Guidance
Use the same path convention as `serve_runtime.rs:522` and `commands/plan.rs:311`. Do not introduce a new path constant -- inline the join. The function should be infallible (returns a fresh router if the file is missing or corrupt), matching `load_or_new` semantics.

#### Verification Criteria
- [ ] `cargo check -p roko-cli` compiles
- [ ] `cargo test -p roko-cli -- model_selection` passes (including new roundtrip test)
- [ ] `grep -n 'load_cascade_router' crates/roko-cli/src/model_selection.rs` shows the new function
- [ ] `grep -n 'save_cascade_router' crates/roko-cli/src/model_selection.rs` shows the companion

---

### Task 3.2: Thread CascadeRouter Through resolve_effective_model_key
**Priority**: P0
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/model_selection.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/plan.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/prd.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/config_cmd.rs`
**Depends On**: Task 3.1

#### Context
`resolve_effective_model_key()` at `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/model_selection.rs:184-196` is the convenience wrapper called by CLI command handlers. It currently hardcodes `None` for the cascade_router parameter at line 192:
```rust
resolve_effective_model(cli_model, None, role.map(str::to_string), None, &config)
```

Call sites (found via grep):
- `commands/plan.rs:559` -- `roko plan generate`
- `commands/plan.rs:608` -- `roko plan regenerate`
- `commands/prd.rs:351` -- `roko prd plan`
- `commands/prd.rs:672` -- `roko prd draft`
- `commands/config_cmd.rs:352` -- `roko config models route` (uses `resolve_effective_model` directly)
- `commands/config_cmd.rs:649` -- `roko config models list` (uses `resolve_effective_model` directly)

#### Implementation Steps
1. Change `resolve_effective_model_key()` signature from:
   ```rust
   pub fn resolve_effective_model_key(
       workdir: &Path, cli_model: Option<String>, role: Option<&str>, context: &str,
   ) -> anyhow::Result<String>
   ```
   to:
   ```rust
   pub fn resolve_effective_model_key(
       workdir: &Path, cli_model: Option<String>, role: Option<&str>, context: &str,
       cascade_router: Option<&CascadeRouter>,
   ) -> anyhow::Result<String>
   ```
2. Forward `cascade_router` to `resolve_effective_model()` instead of hardcoded `None`
3. Update all 4 call sites in `commands/plan.rs` and `commands/prd.rs`:
   - At each call site, the `CascadeRouter` is already loaded nearby (e.g., `plan.rs:322-323` loads the router). Pass a reference.
   - Where no router is loaded yet, call `load_cascade_router(workdir, &config)` and pass `Some(&router)`
4. For `config_cmd.rs` call sites (which call `resolve_effective_model` directly), add `Some(&router)` where the router is loaded at line 687
5. Run `cargo test -p roko-cli` to verify no regressions

#### Design Guidance
Adding the parameter to the convenience wrapper means every CLI command that resolves a model can benefit from adaptive routing with zero extra wiring per command. Pass `None` only in tests or where config is not available.

#### Verification Criteria
- [ ] `cargo check -p roko-cli` compiles
- [ ] `grep -n 'resolve_effective_model_key' crates/roko-cli/src/ -r` shows `cascade_router` parameter at all call sites
- [ ] `SelectionSource::CascadeRouter` variant is now reachable (verify with a test that passes a router with observations)
- [ ] Existing tests pass unchanged (callers that don't have a router pass `None`)

---

### Task 3.3: Wire CascadeRouter into `roko run` Entry Point
**Priority**: P0
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/run.rs`
**Depends On**: Task 3.1

#### Context
`roko run "<prompt>"` is the primary CLI dispatch entry point. The model selection call at `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/run.rs:452` currently passes `None` for cascade_router:
```rust
let selection = resolve_effective_model(cli_model_override, None, role, None, &model_config)
```

The `ServiceFactory::build()` call at line 466 feeds into `WorkflowEngine` but never loads a CascadeRouter. The dead `orchestrate.rs` was the only path that used the router with the workflow engine.

#### Implementation Steps
1. In the function that builds the model selection (around line 430-456), add:
   ```rust
   let cascade_router = load_cascade_router(&workdir, &model_config);
   ```
2. Change line 452 to pass `Some(&cascade_router)`:
   ```rust
   let selection = resolve_effective_model(cli_model_override, None, role, Some(&cascade_router), &model_config)
   ```
3. After the workflow engine completes (both success and error paths), persist the router:
   ```rust
   if let Err(e) = save_cascade_router(&workdir, &cascade_router) {
       tracing::warn!(error = %e, "failed to persist cascade router");
   }
   ```
4. Ensure the save happens even on early returns. Add save calls in all exit paths, or use a helper struct with `Drop` that calls save.
5. The `ServiceFactory` in `crates/roko-orchestrator/src/service_factory.rs:123` already loads a router. If the run.rs path builds its own `ServiceFactory`, verify it passes the same router to avoid double-loading.

#### Design Guidance
Load once, pass by reference everywhere, save once at exit. The router should be `Arc<CascadeRouter>` if it needs to be shared across async tasks; otherwise a plain reference suffices for single-dispatch `roko run`. Match the pattern at `commands/plan.rs:322-323` which already loads the router.

#### Verification Criteria
- [ ] `cargo run -p roko-cli -- run "echo hello"` completes successfully
- [ ] After running, `.roko/learn/cascade-router.json` exists and has been updated (check `mtime`)
- [ ] `grep -n 'load_cascade_router\|save_cascade_router' crates/roko-cli/src/run.rs` shows both calls
- [ ] Model selection log line shows correct `source` field (may be `CascadeRouter` or fall through to `ProjectDefault`)

---

### Task 3.4: Wire CascadeRouter into `roko plan run`
**Priority**: P0
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/plan.rs`
**Depends On**: Task 3.1, Task 3.2

#### Context
`roko plan run` already loads a `CascadeRouter` at `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/plan.rs:322-323`:
```rust
let cascade_router = std::sync::Arc::new(
    roko_learn::cascade_router::CascadeRouter::load_or_new(&router_path, model_slugs),
);
```

But this router is only passed to the Runner v2 infrastructure (runner/types.rs). The model selection calls at lines 559 and 608 use `resolve_effective_model_key()` which hardcodes `None`. After Task 3.2, those calls accept a router parameter.

The runner v2 at `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/types.rs:1306` loads its own router separately from the one in `plan.rs`. These should be unified.

#### Implementation Steps
1. Replace the inline `CascadeRouter::load_or_new` at plan.rs:322 with `load_cascade_router(&wd, &roko_config)` (from Task 3.1)
2. Pass `Some(&cascade_router)` to the `resolve_effective_model_key()` calls at lines 559 and 608
3. After plan execution completes, call `save_cascade_router(&wd, &cascade_router)`
4. Ensure runner/types.rs receives the same router instance (via `Arc`) rather than loading its own copy. The runner constructor at types.rs:1306 should accept an `Arc<CascadeRouter>` parameter.

#### Design Guidance
`roko plan run` executes many tasks. Each task's model selection should consult the router, and each task's result should feed back as an observation. The router must be `Arc<CascadeRouter>` since the runner holds it across async task boundaries. Save after all tasks complete, not after each task (the router's internal Mutex handles concurrent updates).

#### Verification Criteria
- [ ] `cargo check -p roko-cli` compiles
- [ ] `roko plan run plans/` uses CascadeRouter for model selection
- [ ] After plan execution, `.roko/learn/cascade-router.json` is updated with new observations
- [ ] The router loaded in plan.rs is the same instance used by runner/types.rs (no double-load)

---

### Task 3.5: Wire CascadeRouter into `roko chat` / chat_inline
**Priority**: P0
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/chat_inline.rs`
**Depends On**: Task 3.1

#### Context
The interactive chat REPL (`roko chat`, `roko <prompt>`) at `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/chat_inline.rs` calls `resolve_effective_model` at line 1466 with no cascade router. The `dispatch_via_model_call_service` call at line 1780 creates a `ModelCallService` without a router.

The chat session can last many turns. The router should be loaded at session start, consulted each turn, and persisted on session exit.

#### Implementation Steps
1. Load the CascadeRouter at chat session initialization (near the top of the main chat loop)
2. Pass the router reference to `resolve_effective_model()` at line 1466
3. For the `dispatch_via_model_call_service()` path at line 1780, the `ModelCallService` should receive the router via `with_cascade_router()`. Modify `dispatch_via_model_call_service()` in `dispatch_v2.rs` to accept an optional `Arc<CascadeRouter>` parameter.
4. Persist the router on session exit (both clean exit via `/exit` and Ctrl-C handler)
5. Use a `Drop` guard or explicit save in the session shutdown path

#### Design Guidance
Chat sessions are long-lived. The router learns within a session and carries those observations to the next session. Since `dispatch_via_model_call_service` at `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch_v2.rs:53` constructs a fresh `ModelCallService` per call, the router should be passed via `with_cascade_router(Arc::clone(&router))` each time rather than stored in the service.

#### Verification Criteria
- [ ] Interactive `roko chat` session uses CascadeRouter
- [ ] After several turns and exit, `.roko/learn/cascade-router.json` is updated
- [ ] `grep -n 'load_cascade_router\|save_cascade_router' crates/roko-cli/src/chat_inline.rs` shows both

---

### Task 3.6: Record CascadeRouter Observations After Each ModelCallService Call
**Priority**: P0
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/model_call_service.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/cascade_router.rs`
**Depends On**: Task 3.1

#### Context
`ModelCallService` at `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/model_call_service.rs` already has a `cascade_router` field (line 84) typed as `Option<Arc<dyn ForceBackendOverrideRecorder>>`. This trait only has `record_override_outcome(model_slug, success)` -- it was designed for UX34 force_backend learning, not general routing observations.

`CascadeRouter::record_observation()` at `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/cascade_router.rs:961` takes `(ctx: &RoutingContext, model_slug: &str, reward: f64, success: bool)`. This requires a `RoutingContext` which carries the 17-dimensional feature vector.

The gap: `ModelCallService` records override outcomes but not general routing observations. After each successful `call()`, no observation is fed back to the router.

#### Implementation Steps
1. Define a new trait in `model_call_service.rs` (or extend `ForceBackendOverrideRecorder`):
   ```rust
   pub trait RoutingObserver: Send + Sync {
       fn record_observation(&self, model_slug: &str, success: bool, latency_ms: u64, cost_usd: f64);
       fn record_override_outcome(&self, model_slug: &str, success: bool) -> bool;
   }
   ```
2. Implement `RoutingObserver` for `CascadeRouter` in `roko-learn` by delegating to `record_observation()` with a default `RoutingContext` (zero features, or minimal features from available info)
3. Replace the `cascade_router: Option<Arc<dyn ForceBackendOverrideRecorder>>` field with `routing_observer: Option<Arc<dyn RoutingObserver>>`
4. In `ModelCallService::call()`, after the agent returns, call:
   ```rust
   if let Some(observer) = &self.routing_observer {
       observer.record_observation(&model, success, latency_ms, cost_usd);
   }
   ```
5. Update `with_cascade_router()` to accept `Arc<dyn RoutingObserver>` instead
6. Update all existing callers (search for `with_cascade_router` in the codebase)

#### Design Guidance
The trait approach avoids adding a `roko-agent` -> `roko-learn` dependency. `RoutingObserver` is defined in `roko-agent`, implemented in `roko-learn`. The `roko-cli` layer wires them together. This is the existing pattern used for `ForceBackendOverrideRecorder`.

For the `RoutingContext`, a minimal version is acceptable initially (populate only the fields available: model slug, cost, latency). As more context is threaded through `ModelCallRequest`, richer features can be added.

#### Verification Criteria
- [ ] `cargo check -p roko-agent` and `cargo check -p roko-learn` compile
- [ ] `cargo test -p roko-agent` passes
- [ ] After `roko run "echo hello"`, the router's observation count has increased (check JSON file)
- [ ] `ForceBackendOverrideRecorder` callers are migrated to `RoutingObserver`

---

## Phase 1: Stream Parser Consolidation

Eliminate duplicated stream-json parsing logic. One parser, one truncation utility.

### Task 3.7: Extract Shared Truncation Utility
**Priority**: P1
**Estimated Effort**: 2 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/provider/claude_cli/stream.rs`
**Depends On**: none

#### Context
The 4096-byte truncation logic is inlined in 4 places:
1. `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/provider/claude_cli/stream.rs:125` (constant `TOOL_OUTPUT_TRUNCATE_AT`)
2. `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/translate/mod.rs:186-191` (inline 4096)
3. `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/translate/mod.rs:238-243` (inline 4096)
4. `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/chat.rs:660-665` (inline 4096)

All four implement the same pattern: check `len() > 4096`, walk backward to find `char_boundary`, append `"...[truncated]"`.

#### Implementation Steps
1. In `stream.rs`, make the existing `TOOL_OUTPUT_TRUNCATE_AT` constant `pub`
2. Add a `pub fn truncate_tool_output(content: &str, max_bytes: usize) -> String`:
   ```rust
   pub fn truncate_tool_output(content: &str, max_bytes: usize) -> String {
       if content.len() <= max_bytes {
           return content.to_string();
       }
       let mut end = max_bytes;
       while !content.is_char_boundary(end) && end > 0 {
           end -= 1;
       }
       format!("{}...[truncated]", &content[..end])
   }
   ```
3. Re-export from `crate::provider::claude_cli::stream` and from `crate::provider::claude_cli` module
4. Add tests: empty string, ASCII within limit, ASCII over limit, multi-byte UTF-8 boundary, exact boundary

#### Design Guidance
The function should be a pure utility with no side effects. `max_bytes` parameter allows callers to use different limits if needed (though 4096 is the standard). Return `String` always -- the caller can decide to borrow if needed later.

#### Verification Criteria
- [ ] `cargo test -p roko-agent` passes including new tests
- [ ] `truncate_tool_output` is publicly accessible from `roko_agent::provider::claude_cli::stream`
- [ ] Function handles multi-byte UTF-8 correctly (test with emoji or CJK characters)

---

### Task 3.8: Replace Inline Truncation in translate/mod.rs
**Priority**: P1
**Estimated Effort**: 2 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/translate/mod.rs`
**Depends On**: Task 3.7

#### Context
`BackendResponse::extract_text()` at line 186-191 and `BackendResponse::extract_tool_outputs()` at line 238-243 both inline the truncation logic for tool output content. Both operate on `serde_json::Value` event streams from `StreamJson` variant.

The `extract_text()` method formats tool output as `"\n[{tool_name}]\n{content}\n"` with truncation. The `extract_tool_outputs()` method returns `Vec<(Option<String>, String)>` with truncation.

#### Implementation Steps
1. Add import: `use crate::provider::claude_cli::stream::{truncate_tool_output, TOOL_OUTPUT_TRUNCATE_AT};`
2. In `extract_text()` (line 186-192), replace the inline block:
   ```rust
   // Before:
   if content.len() > 4096 {
       let mut end = 4096;
       while !content.is_char_boundary(end) { end -= 1; }
       buf.push_str(&content[..end]);
       buf.push_str("...[truncated]\n");
   } else {
       buf.push_str(content);
       buf.push('\n');
   }
   // After:
   let display = truncate_tool_output(content, TOOL_OUTPUT_TRUNCATE_AT);
   buf.push_str(&display);
   buf.push('\n');
   ```
3. In `extract_tool_outputs()` (line 238-246), replace the inline block:
   ```rust
   // Before:
   let truncated = if content.len() > 4096 { ... } else { content.to_string() };
   // After:
   let truncated = truncate_tool_output(content, TOOL_OUTPUT_TRUNCATE_AT);
   ```
4. Run existing tests to verify no behavior change

#### Design Guidance
This is a mechanical replacement. The output format should be identical -- verify by comparing test assertions before and after. The `truncate_tool_output` function appends `"...[truncated]"` which matches the existing inline format.

#### Verification Criteria
- [ ] `cargo test -p roko-agent` passes (all existing translate tests)
- [ ] `grep -n '4096' crates/roko-agent/src/translate/mod.rs` returns zero results
- [ ] `grep -n 'truncate_tool_output' crates/roko-agent/src/translate/mod.rs` shows 2 call sites

---

### Task 3.9: Replace extract_clean_text with Typed Parsing
**Priority**: P1
**Estimated Effort**: 6 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/chat.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch_direct.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/agent_serve.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/chat_inline.rs`
**Depends On**: Task 3.7

#### Context
`extract_clean_text()` at `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/chat.rs:570-697` is a 127-line format guesser handling 10 different response shapes. It is called from:
- `chat.rs:206` -- chat REPL display
- `dispatch_direct.rs:170` -- legacy dispatch (feature-gated behind `legacy-orchestrate`)
- `agent_serve.rs:514` -- sidecar agent serving
- `chat_inline.rs:4268` -- inline chat

The canonical parser `parse_stream_line()` at `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/provider/claude_cli/stream.rs:134` returns typed `AgentRuntimeEvent` variants. It is already used by:
- `chat_session.rs:1171` -- chat session
- `runner/agent_stream.rs:120` -- runner

#### Implementation Steps
1. For callers that receive Claude CLI stream-json output (JSONL lines), replace `extract_clean_text` with `parse_stream_line`:
   ```rust
   use roko_agent::provider::claude_cli::stream::parse_stream_line;
   let events: Vec<AgentRuntimeEvent> = raw.lines()
       .flat_map(|line| parse_stream_line(line))
       .collect();
   // Extract text from events
   let text = events.iter().filter_map(|e| match e {
       AgentRuntimeEvent::Text(t) => Some(t.as_str()),
       _ => None,
   }).collect::<Vec<_>>().join("");
   ```
2. For callers that receive single JSON objects (sidecar, API responses), keep a thin wrapper that checks for `result` or `content` fields -- but extract it from `extract_clean_text` into a smaller `extract_json_text(value: &serde_json::Value) -> Option<String>` function
3. Deprecate `extract_clean_text` with `#[deprecated]` annotation pointing to the typed alternatives
4. Update call sites one by one:
   - `chat.rs:206` -- use `parse_stream_line` for JSONL, `extract_json_text` for single objects
   - `agent_serve.rs:514` -- use `extract_json_text` (sidecar returns single JSON)
   - `chat_inline.rs:4268` -- use `parse_stream_line`
5. Keep existing tests passing by adding equivalent tests for the new functions

#### Design Guidance
Do NOT delete `extract_clean_text` in this task -- deprecate it. The `dispatch_direct.rs` caller is behind a feature gate and will be removed when the legacy flag is dropped. The goal is to stop adding new callers and migrate existing ones. The replacement should use typed `AgentRuntimeEvent` variants, not raw JSON guessing.

#### Verification Criteria
- [ ] `cargo check -p roko-cli` compiles
- [ ] `cargo test -p roko-cli` passes
- [ ] `extract_clean_text` has `#[deprecated]` annotation
- [ ] `grep -n 'extract_clean_text' crates/roko-cli/src/ -r | grep -v test | grep -v deprecated` shows only the deprecated definition and `dispatch_direct.rs` (feature-gated)
- [ ] All existing `extract_clean_text` tests have equivalents for the new functions

---

## Phase 2: Budget Enforcement

### Task 3.10: Wire BudgetConfig from roko.toml into ModelCallService
**Priority**: P1
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/budget.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/model_call_service.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch_v2.rs`
**Depends On**: none

#### Context
`BudgetConfig` already exists at `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/budget.rs` with `max_plan_usd: f32` (default $25), `max_turn_usd: f32` (default $3), and `prompt_token_budget: usize` (default 10,000). The `[budget]` section is already in the roko.toml schema.

`ModelCallService::new()` at line 126 creates `BudgetCell::new(None)` -- unlimited by default. The `with_cost_budget(max_cost_usd)` builder exists at line 233 but is never called from `dispatch_v2.rs`.

#### Implementation Steps
1. Add `per_session_usd: f32` field to `BudgetConfig` with default $10.00. This is the session-level budget for `roko run` and `roko chat`. The existing `max_plan_usd` covers plan execution and `max_turn_usd` covers per-turn limits.
2. In `dispatch_v2.rs`, after loading config (line 63-80), apply budget:
   ```rust
   let budget = config.budget.per_session_usd as f64;
   service = service.with_cost_budget(budget);
   ```
3. In `run.rs`, apply `config.budget.max_plan_usd` when building `ModelCallService` for plan execution
4. Add a `with_turn_budget(max_turn_usd: f64)` builder to `ModelCallService` that sets a per-call (not cumulative) cap. Implement by adding a `turn_budget` field and checking it in `call()` before dispatch.
5. Wire `config.budget.max_turn_usd` through all entry points

#### Design Guidance
The cumulative session budget (`per_session_usd`) tracks total spend across all calls in a session. The per-turn budget (`max_turn_usd`) caps a single call. Both should be configurable via roko.toml. When a budget is exceeded, fail with a clear error message including the budget amount, the current spend, and how to increase the limit.

#### Verification Criteria
- [ ] `cargo test -p roko-core` passes
- [ ] `BudgetConfig` has `per_session_usd` field
- [ ] `ModelCallService` receives a non-None budget from `dispatch_v2.rs`
- [ ] A test that sets `per_session_usd = 0.001` and dispatches a real model call gets a budget-exceeded error

---

### Task 3.11: Budget Graceful Degradation -- Cheaper Model Fallback
**Priority**: P2
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/model_call_service.rs`
**Depends On**: Task 3.10

#### Context
`ModelCallService` has `fallback_models: Vec<String>` (line 86) populated by `configured_fallback_models()` from workspace config. When budget is near exhaustion, instead of hard-failing, the service should attempt a cheaper model.

The `BudgetCell` at line 951 tracks cumulative cost and has a `check()` method. It needs a `remaining()` or `fraction_used()` accessor.

#### Implementation Steps
1. Add `remaining_usd(&self) -> Option<f64>` and `fraction_used(&self) -> Option<f64>` to `BudgetCell`
2. In `ModelCallService::call()`, before dispatch, check budget proximity:
   ```rust
   if let Some(fraction) = self.budget.fraction_used() {
       if fraction > 0.90 {
           // Find cheapest model from fallback list
           if let Some(cheaper) = self.cheapest_fallback_model(&effective_model) {
               tracing::warn!(
                   budget_used_pct = fraction * 100.0,
                   from = %effective_model, to = %cheaper,
                   "budget >90% used, downgrading model"
               );
               effective_model = cheaper;
           }
       }
   }
   ```
3. Add `cheapest_fallback_model(&self, current: &str) -> Option<String>` that uses `CostTable` to find the lowest-cost model from `fallback_models`
4. Emit a `RuntimeEvent::BudgetWarning` when degradation occurs
5. When budget is 100% exhausted, check if `config.budget.on_exceeded` is `"downgrade"` (try cheaper), `"warn"` (proceed but warn), or `"fail"` (hard error). Default to `"fail"`.

#### Design Guidance
Graceful degradation should be transparent but not silent. Always log when a model switch happens due to budget pressure. The user should see "budget 92% used, switching from claude-opus to claude-haiku" in stderr/logs. Never silently degrade without notification.

#### Verification Criteria
- [ ] `cargo test -p roko-agent` passes
- [ ] A unit test demonstrates model downgrade when budget fraction > 0.90
- [ ] Budget exhaustion produces a clear error message (not a panic)
- [ ] `RuntimeEvent::BudgetWarning` is emitted on degradation

---

## Phase 3: Direct Env Key Elimination

### Task 3.12: Migrate episode_completion.rs to Injected ModelCaller
**Priority**: P1
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-neuro/src/episode_completion.rs` (defined in `crates/roko-neuro/src/lib.rs`, search for `episode_completion` module)
**Depends On**: none

#### Context
The episode completion distillation at `/Users/will/dev/nunchi/roko/roko/crates/roko-neuro/src/episode_completion.rs` (re-exported from `lib.rs`) has a fallback path at line 46 that reads `ANTHROPIC_API_KEY` directly:
```rust
let Some(api_key) = std::env::var("ANTHROPIC_API_KEY")
    .ok()
    .map(|key| key.trim().to_owned())
    .filter(|key| !key.is_empty())
```

The function `distill_episode()` already accepts `model_caller: Option<Arc<dyn ModelCaller>>` and uses it when available (line 40-44). The env var fallback is only for when no `ModelCaller` is provided.

#### Implementation Steps
1. In `spawn_episode_distillation()`, ensure all callers provide a `model_caller`. Search for `spawn_episode_distillation` in the codebase to find all call sites.
2. If all callers can provide a `ModelCaller`, remove the `Option` wrapper -- make it `Arc<dyn ModelCaller>` (required). If some callers cannot, keep `Option` but emit a `tracing::warn!` and return early when `None`, rather than falling back to raw env var.
3. Remove the `std::env::var("ANTHROPIC_API_KEY")` fallback path
4. Remove the `Distiller::with_claude(api_key)` call that builds its own HTTP client
5. The `GATEWAY_DISTILLATION_MODEL` constant ("claude-haiku-3-5") should be configurable via config, not hardcoded

#### Design Guidance
The `ModelCaller` trait (`roko_core::foundation::ModelCaller`) is the correct abstraction for all LLM calls. The raw API key path was a bootstrap hack. All callers should construct a `ModelCaller` via `ModelCallService` which handles credential resolution through the provider system.

#### Verification Criteria
- [ ] `cargo check -p roko-neuro` compiles
- [ ] `grep -n 'std::env::var.*ANTHROPIC_API_KEY' crates/roko-neuro/src/` returns zero results
- [ ] `cargo test -p roko-neuro` passes
- [ ] Distillation still works when a `ModelCaller` is provided

---

### Task 3.13: Migrate web_search.rs to Injected Provider Config
**Priority**: P1
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-std/src/tool/builtin/web_search.rs`
**Depends On**: none

#### Context
The web search builtin tool at `/Users/will/dev/nunchi/roko/roko/crates/roko-std/src/tool/builtin/web_search.rs:332` reads `PERPLEXITY_API_KEY` directly:
```rust
let api_key = match std::env::var("PERPLEXITY_API_KEY") {
    Ok(k) if !k.is_empty() => k,
    _ => { return ToolResult::Err(...) }
};
```

There is already a TODO comment at line 330: `// TODO(gateway): wire ModelCaller from runtime ToolContext.`

The `ToolContext` passed to tool handlers carries metadata but currently lacks a reference to provider configuration or a pre-configured HTTP client.

#### Implementation Steps
1. Add an `api_keys: HashMap<String, String>` or `provider_config: Option<Arc<RokoConfig>>` field to the tool execution context (this may be in `ToolContext`, `ToolDispatchContext`, or the equivalent struct in `roko-std`)
2. If adding to `ToolContext` would cause too many downstream changes, add a simpler `perplexity_api_key: Option<String>` field that is populated from provider config at dispatch time
3. In the web_search handler, read the API key from context instead of env:
   ```rust
   let api_key = ctx.api_key("perplexity")
       .ok_or_else(|| ToolError::Other("Perplexity API key not configured".into()))?;
   ```
4. Populate the key from `RokoConfig::effective_providers()["perplexity"].resolve_api_key()` at the point where `ToolContext` is constructed
5. Remove the `std::env::var("PERPLEXITY_API_KEY")` call
6. Remove the `warn_direct_api_key_path_once()` helper since the direct path no longer exists

#### Design Guidance
Ideally the web_search tool would use `create_agent_for_model()` to get a Perplexity `Agent` and call it through the provider system. But the Perplexity adapter's `run()` returns agent results, not search results with citations. The current implementation does raw HTTP POST to the Perplexity chat/completions endpoint. A compromise is to pass the API key through `ToolContext` (injected from provider config) rather than reading it from env. Full provider integration is a follow-up.

#### Verification Criteria
- [ ] `cargo check -p roko-std` compiles
- [ ] `grep -n 'std::env::var.*PERPLEXITY_API_KEY' crates/roko-std/src/` returns zero results (outside tests)
- [ ] `cargo test -p roko-std` passes
- [ ] Web search works when API key is configured in roko.toml providers section

---

## Phase 4: Provider Health Integration

### Task 3.14: Wire ProviderHealthTracker into ModelCallService
**Priority**: P1
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/model_call_service.rs`
**Depends On**: none

#### Context
`ProviderHealthTracker` at `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/provider_health.rs:501` is a per-provider circuit breaker with `record_success()`, `record_failure()`, `is_healthy()`, and `filter_arms()`. It supports Healthy/Unhealthy/Probing states with configurable failure threshold (default 3) and recovery window (default 120s).

It is already used by:
- `roko-serve` state (`state.rs:377`)
- `roko-learn` runtime feedback (`runtime_feedback.rs:1251`)
- `roko-conductor` (`conductor.rs:70`)
- `roko-learn` model router (`model_router.rs:1241`)

But `ModelCallService` does not check provider health before dispatch. When a provider is down, every call fails immediately rather than trying a fallback.

#### Implementation Steps
1. Add a `health_tracker: Option<Arc<dyn HealthGate>>` field to `ModelCallService` where `HealthGate` is a new trait:
   ```rust
   pub trait HealthGate: Send + Sync {
       fn is_healthy(&self, provider_key: &str) -> bool;
       fn record_success(&self, provider_key: &str);
       fn record_failure(&self, provider_key: &str);
   }
   ```
2. Implement `HealthGate` for `ProviderHealthTracker` in `roko-learn` (trivial delegation)
3. Add `with_health_gate(gate: Arc<dyn HealthGate>)` builder method
4. In `ModelCallService::call()`, before dispatch:
   ```rust
   if let Some(gate) = &self.health_tracker {
       let provider_key = self.provider_key_for_model(&model);
       if !gate.is_healthy(&provider_key) {
           // Try fallback models
           for fb in &self.fallback_models {
               let fb_provider = self.provider_key_for_model(fb);
               if gate.is_healthy(&fb_provider) {
                   tracing::warn!(from = %model, to = %fb, "primary provider unhealthy, failing over");
                   model = fb.clone();
                   break;
               }
           }
       }
   }
   ```
5. After dispatch, record success/failure:
   ```rust
   if let Some(gate) = &self.health_tracker {
       if success { gate.record_success(&provider_key); }
       else { gate.record_failure(&provider_key); }
   }
   ```
6. Add a helper `provider_key_for_model(&self, model: &str) -> String` that maps model slug to provider key using `roko_core::agent::resolve_model` pattern matching

#### Design Guidance
Use the trait approach to avoid a `roko-agent` -> `roko-learn` dependency. The `HealthGate` trait is minimal (3 methods). `ProviderHealthTracker` implements it in `roko-learn`. `roko-cli` wires them together when constructing `ModelCallService`. This matches the existing `ForceBackendOverrideRecorder` pattern.

#### Verification Criteria
- [ ] `cargo check -p roko-agent` compiles
- [ ] `cargo test -p roko-agent` passes
- [ ] Unit test: when primary provider is marked unhealthy, dispatch uses fallback model
- [ ] Unit test: after successful call, health tracker records success
- [ ] Unit test: after failed call, health tracker records failure

---

### Task 3.15: Wire ProviderHealthTracker at CLI Entry Points
**Priority**: P1
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch_v2.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/run.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/chat_inline.rs`
**Depends On**: Task 3.14

#### Context
After Task 3.14, `ModelCallService` accepts a `HealthGate`. Now each CLI entry point that constructs a `ModelCallService` needs to create and pass a `ProviderHealthTracker`.

The health tracker should be shared across the session (not created per-call) so that a provider that fails mid-session gets tripped for subsequent calls.

#### Implementation Steps
1. In `dispatch_v2.rs:dispatch_via_model_call_service()`, create a `ProviderHealthTracker` and pass via `with_health_gate()`
2. In `run.rs`, create a session-scoped `ProviderHealthTracker`, share via `Arc`, pass to `ModelCallService`
3. In `chat_inline.rs`, create a session-scoped tracker that persists across the entire chat session
4. For `roko plan run`, the tracker should be shared across all task dispatches (same `Arc` passed to all `ModelCallService` instances)

#### Design Guidance
Use `Arc<ProviderHealthTracker>` since the tracker needs to be shared across async tasks. Create one per CLI session, not per call. The tracker is in-memory only -- it does not persist to disk. Provider health resets on each CLI invocation, which is the correct behavior (a provider that was down 5 minutes ago may be back).

#### Verification Criteria
- [ ] `cargo check -p roko-cli` compiles
- [ ] `grep -n 'with_health_gate\|ProviderHealthTracker' crates/roko-cli/src/ -r` shows wiring in all 3 entry points
- [ ] Integration: if a provider is unreachable, the second call in the same session uses a fallback

---

### Task 3.16: Add Retry Logic to ModelCallService
**Priority**: P2
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/model_call_service.rs`
**Depends On**: Task 3.14

#### Context
`RetryPolicy` at `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/retry.rs:45` already exists with full-jitter exponential backoff, retryable error classification, and `retry_after_ms` support. It is used by `ToolLoop` at `tool_loop/mod.rs:195` for multi-turn retries.

`ModelCallService::call()` does not retry on transient failures. When the agent returns an error, it immediately fails or falls back to a different model. For transient errors (rate limits, 500s, timeouts), retrying the same model with backoff is more efficient than failover.

#### Implementation Steps
1. Add `retry_policy: RetryPolicy` field to `ModelCallService` with `RetryPolicy::default()` (3 attempts, 1s base, 60s max)
2. Add `with_retry_policy(policy: RetryPolicy)` builder
3. In `call()`, wrap the dispatch in a retry loop:
   ```rust
   for attempt in 0..self.retry_policy.max_attempts {
       match self.dispatch_once(&model, &request).await {
           Ok(response) => return Ok(response),
           Err(e) if self.retry_policy.should_retry_mcs(&e, attempt) => {
               let delay = self.retry_policy.delay_for_attempt(attempt);
               tokio::time::sleep(Duration::from_millis(delay)).await;
               continue;
           }
           Err(e) => return Err(e),
       }
   }
   ```
4. Add `should_retry_mcs()` that classifies `RokoError` into retryable/non-retryable (similar to `should_retry()` for `ProviderError`)
5. Integrate with health tracker: record failure on final retry exhaustion, not on each attempt

#### Design Guidance
Retry before failover. Only fail over to a different model after all retries are exhausted. This prevents unnecessary model switches on transient network issues. The `retry_after_ms` from rate-limit headers should be honored (use `delay_with_retry_after`).

Rate limit retries should respect the provider's `retry_after_ms`. Server errors get exponential backoff. Timeouts get one retry with the same timeout. Auth failures, content policy, context overflow, and model-not-found are never retried.

#### Verification Criteria
- [ ] `cargo test -p roko-agent` passes
- [ ] Unit test: transient error followed by success completes without failover
- [ ] Unit test: 3 consecutive failures triggers final error (not infinite loop)
- [ ] Unit test: rate limit with `retry_after_ms=2000` waits approximately 2 seconds

---

## Phase 5: Thinking Token Accounting

### Task 3.17: Add thinking_tokens to UsageObservation
**Priority**: P2
**Estimated Effort**: 2 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/usage.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/chat_types.rs` (if `Usage` lives here)
**Depends On**: none

#### Context
`UsageObservation` at `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/usage.rs:17` tracks `input_tokens`, `output_tokens`, `cache_creation_tokens`, `cache_read_tokens`, `cost_usd` but not thinking/reasoning tokens.

The CascadeRouter already tracks thinking tokens in its Gemini observations (`cascade/types.rs:437`). The Gemini native backend extracts them (`gemini/native.rs:333`). But the canonical `UsageObservation` type -- which is used by `ModelCallService`, episode logging, and all feedback paths -- does not carry this data.

#### Implementation Steps
1. Add `pub thinking_tokens: Option<u64>` field to `UsageObservation` (after `cache_read_tokens`)
2. Update the `From<Usage> for UsageObservation` implementation to set `thinking_tokens: None` (legacy `Usage` struct doesn't have it)
3. Update the `From<UsageObservation> for Usage` implementation to ignore `thinking_tokens` (legacy struct can't represent it)
4. Ensure `#[serde(default)]` on the new field for backward-compatible deserialization
5. Update the Gemini adapter to populate `thinking_tokens` from `GeminiMetadata.thinking_tokens`
6. Update the Anthropic API adapter to populate from response when available (Claude extended thinking returns reasoning token counts)
7. Update `CostTable` pricing to account for thinking tokens (often priced differently than output tokens)

#### Design Guidance
`Option<u64>` matches the existing pattern for all fields. `None` means "this provider/model doesn't report thinking tokens" -- distinct from `Some(0)` meaning "thinking was enabled but produced zero tokens." This distinction matters for cost accounting: a model that doesn't support thinking should not have zero thinking cost attributed.

#### Verification Criteria
- [ ] `cargo check -p roko-agent` compiles
- [ ] `cargo test -p roko-agent` passes
- [ ] Serialization/deserialization roundtrip preserves `thinking_tokens`
- [ ] Existing usage data without `thinking_tokens` deserializes correctly (defaults to `None`)

---

## Phase 6: ACP Provider Migration

### Task 3.18: Replace run_claude_cli() in ACP Runner with Provider Adapter
**Priority**: P2
**Estimated Effort**: 6 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/runner.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/Cargo.toml`
**Depends On**: none

#### Context
`run_claude_cli()` at `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/runner.rs:1849` is the most bare-bones LLM invocation in the codebase. It spawns `claude --print --dangerously-skip-permissions` with no model flag, no streaming, no system prompt, and no feedback. Called from line 1629.

The `ClaudeCliAdapter` at `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/provider/claude_cli.rs` handles all subprocess construction properly: model selection, system prompt injection, tool allowlist, MCP config, effort settings, safety hooks, streaming output, error classification, and usage tracking.

#### Implementation Steps
1. Add `roko-agent = { path = "../roko-agent" }` to `crates/roko-acp/Cargo.toml` if not already present
2. Thread `RokoConfig` through the ACP runner context. It may already be available via `PipelineConfig` or similar.
3. Replace the `run_claude_cli()` function body with:
   ```rust
   let agent = create_agent_for_model(config, &model_key, &AgentOptions {
       system_prompt: Some(system_prompt),
       workdir: Some(workdir.to_path_buf()),
       ..Default::default()
   })?;
   let engram = Engram::new(Kind::Text, Body::from(prompt));
   let ctx = Context::for_workdir(workdir);
   let result = agent.run(&engram, &ctx).await;
   ```
4. Update the caller at line 1629 to pass model and config
5. Remove the `Command::new("claude")` import if no longer used
6. Preserve the cancellation token integration (`cancel_token` parameter)

#### Design Guidance
The ACP runner should not know about CLI subprocess details. It should request "run this prompt with this model" and get back a result. The provider adapter handles all the subprocess mechanics. If the ACP pipeline needs streaming output, use `AgentOptions { streaming: true }` and handle events through the existing `AgentRuntimeEvent` system.

Keep the `cancel_token` parameter -- wrap the agent call with `tokio::select!` against the cancellation.

#### Verification Criteria
- [ ] `cargo check -p roko-acp` compiles
- [ ] `grep -n 'Command::new("claude")' crates/roko-acp/src/runner.rs` returns zero results
- [ ] `grep -n 'create_agent_for_model' crates/roko-acp/src/runner.rs` shows at least one call
- [ ] ACP runner still works with cancellation

---

### Task 3.19: Replace run_claude_cognitive_task() in Bridge Events
**Priority**: P2
**Estimated Effort**: 5 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/bridge_events.rs`
**Depends On**: Task 3.18

#### Context
`run_claude_cognitive_task()` at `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/bridge_events.rs:1110` spawns `claude --print --output-format stream-json --model <m> --system-prompt <sp>` as a direct subprocess. Better than `run_claude_cli()` (has model and system prompt) but still bypasses the provider system entirely.

`run_openai_compat_cognitive_task()` at line 1140 uses `resolve_model()` from RokoConfig but builds its own HTTP client instead of going through the provider adapter.

Both are called from the cognitive task dispatcher around line 945-972 which switches on provider kind.

#### Implementation Steps
1. Replace `run_claude_cognitive_task()` with a call through `create_agent_for_model()`:
   ```rust
   let agent = create_agent_for_model(config, model, &AgentOptions {
       system_prompt: Some(system_prompt.to_string()),
       workdir: Some(workdir.to_path_buf()),
       ..Default::default()
   })?;
   ```
2. Replace `run_openai_compat_cognitive_task()` similarly -- the provider adapter handles HTTP client construction and all provider-specific quirks
3. The cognitive task dispatcher at line 945-972 should no longer switch on provider kind -- `create_agent_for_model()` handles routing internally
4. Remove the direct `Command::new("claude")` call and the manual `reqwest::Client` construction
5. Parse the agent result into the expected cognitive task output format

#### Design Guidance
The bridge events module should be provider-agnostic. It asks "run this prompt" and gets back a result. The provider adapter layer (7 adapters) handles all the provider-specific details. The cognitive task dispatcher should look like:
```rust
let agent = create_agent_for_model(config, model, &options)?;
let result = agent.run(&engram, &ctx).await?;
parse_cognitive_result(result)
```

No more switching on `ProviderKind` in the bridge.

#### Verification Criteria
- [ ] `cargo check -p roko-acp` compiles
- [ ] `grep -n 'Command::new("claude")' crates/roko-acp/src/bridge_events.rs` returns zero results
- [ ] `grep -n 'reqwest::Client' crates/roko-acp/src/bridge_events.rs` returns zero results (outside tests)
- [ ] Cognitive tasks work for both Claude and OpenAI-compatible models

---

## Phase 7: Provider Quirks Consolidation

### Task 3.20: Replace Boolean Flags with ProviderQuirks Struct
**Priority**: P2
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/openai_compat_backend.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/provider/cerebras.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/provider/openai_compat.rs`
**Depends On**: none

#### Context
`OpenAiCompatLlmBackend` at `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/openai_compat_backend.rs` has three boolean fields for provider-specific workarounds:
- `skip_session_fields: bool` (line 54) -- for Cerebras (rejects unknown fields)
- `disable_parallel_tool_calls: bool` (line 58) -- for small models
- `normalize_tool_call_content: bool` (line 62) -- for providers that reject empty content with tool_calls

These flags multiply with each new strict provider. The Kimi K2.5 documentation in the module header lists 7 additional constraints not yet encoded.

#### Implementation Steps
1. Define a `ProviderQuirks` struct in a new file `crates/roko-agent/src/provider_quirks.rs`:
   ```rust
   #[derive(Debug, Clone, Default)]
   pub struct ProviderQuirks {
       pub skip_session_fields: bool,
       pub disable_parallel_tool_calls: bool,
       pub normalize_tool_call_content: bool,
       pub max_tools: Option<usize>,
       pub strict_schemas: bool,
       pub few_shot_tool_examples: bool,
       pub thinking_locks_temperature: bool,
       pub reasoning_in_history: bool,
   }

   impl ProviderQuirks {
       pub fn cerebras() -> Self { Self { skip_session_fields: true, disable_parallel_tool_calls: true, normalize_tool_call_content: true, strict_schemas: true, few_shot_tool_examples: true, ..Self::default() } }
       pub fn kimi() -> Self { Self { thinking_locks_temperature: true, reasoning_in_history: true, ..Self::default() } }
   }
   ```
2. Replace the 3 boolean fields in `OpenAiCompatLlmBackend` with `quirks: ProviderQuirks`
3. Replace the 3 `with_*` builder methods with `with_quirks(quirks: ProviderQuirks)`
4. Update the Cerebras adapter to use `ProviderQuirks::cerebras()`
5. Update the OpenAI-compat adapter to use `ProviderQuirks::default()` (no quirks)
6. Update all callers of the removed builders

#### Design Guidance
`ProviderQuirks` is a value type -- `Clone`, `Debug`, `Default`. Named constructors (`cerebras()`, `kimi()`) make it easy to add new provider profiles. Adding a new quirk is one field addition + updating the relevant constructor. No new boolean flags needed on the backend.

The quirks struct can later be loaded from `ModelProfile` or `ProviderConfig` in roko.toml, allowing users to configure provider compatibility without code changes.

#### Verification Criteria
- [ ] `cargo check -p roko-agent` compiles
- [ ] `cargo test -p roko-agent` passes
- [ ] `grep -n 'skip_session_fields\|disable_parallel_tool_calls\|normalize_tool_call_content' crates/roko-agent/src/openai_compat_backend.rs` shows only the `ProviderQuirks` struct definition, not loose boolean fields
- [ ] Cerebras adapter uses `ProviderQuirks::cerebras()`

---

## Phase 8: Hardcoded Model String Elimination

### Task 3.21: Remove Hardcoded Model Strings from run.rs
**Priority**: P1
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/run.rs`
**Depends On**: Task 3.3

#### Context
`run.rs` has hardcoded model strings at:
- Line 530: `"claude-sonnet-4-6"` used as fallback
- Line 657: `"llama3.1:8b"` used for local model detection

These bypass the user's `default_model` configuration in `roko.toml`.

#### Implementation Steps
1. Replace the hardcoded `"claude-sonnet-4-6"` at line 530 with `config.agent.default_model.clone()` or the result from `resolve_effective_model()`
2. Replace `"llama3.1:8b"` at line 657 with a check against configured local models rather than a hardcoded string
3. Add constants for any fallback model strings that must remain (e.g., `const BUILT_IN_DEFAULT_MODEL: &str = "claude-sonnet-4-6";`) and use them from the `BuiltInDefault` precedence tier in `model_selection.rs`
4. Verify that changing `default_model` in `roko.toml` actually affects the `roko run` dispatch path

#### Design Guidance
Model strings should flow from config, not from source code. The only acceptable hardcoded model is the `BuiltInDefault` fallback (used when no config is available at all). Every other path should read from `RokoConfig`. This ensures `roko config set agent.default_model=my-model` actually takes effect.

#### Verification Criteria
- [ ] `cargo check -p roko-cli` compiles
- [ ] `grep -n '"claude-sonnet-4-6"\|"llama3.1:8b"' crates/roko-cli/src/run.rs` returns zero results (outside comments/constants)
- [ ] Changing `default_model` in `roko.toml` changes which model `roko run` uses
- [ ] `cargo test -p roko-cli` passes

---

### Task 3.22: Remove Hardcoded Model String from auth_detect.rs
**Priority**: P1
**Estimated Effort**: 2 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/auth_detect.rs`
**Depends On**: none

#### Context
`auth_detect.rs` at line 42 hardcodes `"claude-sonnet-4-6"`:
```rust
if let Ok(key) = std::env::var("ZAI_API_KEY") { ... }
if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") { ... }
if let Ok(key) = std::env::var("OPENAI_API_KEY") { ... }
```

This module detects available backends by probing env vars and CLI availability. It has a `Command::new("claude")` at line 102 for version detection.

#### Implementation Steps
1. Replace hardcoded model string with `config.agent.default_model` where config is available
2. If config is not available (auth_detect runs before config is loaded), use a constant from `model_selection.rs` rather than an inline string
3. The `Command::new("claude")` for version detection is acceptable (it's checking if the CLI is installed, not dispatching a model call)
4. Consolidate env var probing: instead of checking 3 env vars independently, check `config.effective_providers()` for providers with configured credentials

#### Design Guidance
Auth detection is a bootstrap step -- it runs before the full provider system is initialized. Some hardcoding is acceptable here, but it should use shared constants, not inline strings. The env var checks are also acceptable as a pre-flight, but they should not determine model selection (that's `model_selection.rs`'s job).

#### Verification Criteria
- [ ] `cargo check -p roko-cli` compiles
- [ ] `grep -n '"claude-sonnet-4-6"' crates/roko-cli/src/auth_detect.rs` returns zero results
- [ ] Auth detection still correctly identifies available backends
- [ ] `cargo test -p roko-cli` passes

---

## Phase 9: Rate Limiter Per-Provider

### Task 3.23: Wire Per-Provider Rate Limits from Config
**Priority**: P2
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/openai_compat_backend.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/schema.rs`
**Depends On**: none

#### Context
`shared_rate_limiter()` in `openai_compat_backend.rs` uses a `OnceLock` for a single global `ProviderRateLimiter` with 60 RPM default. All `OpenAiCompatLlmBackend` instances share this limiter unless `with_rate_limiter()` is called.

Different providers have different rate limits (Anthropic: 1000 RPM, Cerebras: 30 RPM, OpenRouter: varies by plan). The global 60 RPM unnecessarily throttles high-limit providers and may exceed low-limit ones.

#### Implementation Steps
1. Add `rate_limit_rpm: Option<u32>` field to `ProviderConfig` in the schema (`crates/roko-core/src/config/schema.rs`)
2. In the OpenAI-compat adapter (`provider/openai_compat.rs`), when creating an `OpenAiCompatLlmBackend`, check `provider_config.rate_limit_rpm`:
   ```rust
   let backend = if let Some(rpm) = provider_config.rate_limit_rpm {
       backend.with_rate_limiter(ProviderRateLimiter::new(rpm))
   } else {
       backend  // uses shared default
   };
   ```
3. Create per-provider `ProviderRateLimiter` instances keyed by provider name (use a `HashMap<String, Arc<ProviderRateLimiter>>` singleton or construct per backend)
4. Document the `rate_limit_rpm` config field in `roko.toml` comments

#### Design Guidance
Keep the `shared_rate_limiter()` as a fallback for providers without explicit config. Add per-provider overrides only when config specifies them. This is backward compatible -- existing setups get the same 60 RPM default, users who need different limits can configure them.

#### Verification Criteria
- [ ] `cargo check -p roko-core` and `cargo check -p roko-agent` compile
- [ ] `ProviderConfig` has `rate_limit_rpm` field
- [ ] A provider with `rate_limit_rpm = 1000` gets a 1000 RPM limiter
- [ ] A provider without `rate_limit_rpm` gets the default 60 RPM

---

## Phase 10: orchestrate.rs Decomposition

### Task 3.24: Identify Live Exports from orchestrate.rs
**Priority**: P1
**Estimated Effort**: 2 hours
**Files to Modify**: none (analysis only)
**Depends On**: none

#### Context
`orchestrate.rs` at `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs` is 22,522 lines. Most of it is dead code (the `PlanRunner` is never instantiated). But some exports are used by tests, other modules, or `lib.rs` re-exports.

#### Implementation Steps
1. Run a comprehensive grep for all imports from orchestrate.rs:
   ```bash
   grep -rn 'orchestrate::' crates/ --include='*.rs' | grep -v 'orchestrate.rs' | grep -v target/
   ```
2. Run a grep for items re-exported via lib.rs:
   ```bash
   grep -n 'orchestrate' crates/roko-cli/src/lib.rs
   ```
3. Categorize each export as:
   - **Live (used in production code)**: must be preserved or migrated
   - **Test-only**: can be moved to test helpers
   - **Dead (no callers)**: safe to delete
4. Document the categorization in a comment block at the top of orchestrate.rs
5. For each live export, identify the target module where it should live after decomposition

#### Design Guidance
This is an analysis task. Do not modify orchestrate.rs. Produce a categorized list that subsequent tasks use to plan extraction. The goal is to understand what is actually needed before deleting anything.

#### Verification Criteria
- [ ] A categorized list of all orchestrate.rs exports exists (in a comment or separate tracking doc)
- [ ] Each export is marked as live/test-only/dead with the importing file
- [ ] No code changes to orchestrate.rs

---

### Task 3.25: Extract Gate Failure Replan to roko-orchestrator
**Priority**: P2
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-orchestrator/src/replan.rs` (new file)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-orchestrator/src/lib.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs`
**Depends On**: Task 3.24

#### Context
`build_gate_failure_plan_revision()` in orchestrate.rs generates a revised plan when gate checks fail. This is a valuable pattern for the live `WorkflowEngine` path -- when a task fails gates, the system should be able to generate a fix-up plan automatically.

The function likely takes gate failure details (which gates failed, error messages, diff context) and produces a revised task list. It should live in `roko-orchestrator` alongside the existing plan execution infrastructure.

#### Implementation Steps
1. Create `/Users/will/dev/nunchi/roko/roko/crates/roko-orchestrator/src/replan.rs`
2. Extract `build_gate_failure_plan_revision()` from orchestrate.rs into the new module
3. Generalize the function signature to not depend on orchestrate.rs-specific types:
   ```rust
   pub struct GateFailureContext {
       pub task_id: String,
       pub gate_name: String,
       pub failure_message: String,
       pub diff_context: Option<String>,
       pub prior_attempts: u32,
   }

   pub fn build_replan(context: &GateFailureContext) -> Vec<TaskSpec> { ... }
   ```
4. Re-export from `crates/roko-orchestrator/src/lib.rs`
5. If orchestrate.rs still exports this function (for backward compat), delegate to the new location
6. Add a unit test for the replan function

#### Design Guidance
The extracted function should be pure (no I/O, no state mutation). It takes failure context and returns new task specs. The caller (WorkflowEngine or runner) handles persistence, state updates, and dispatch. This separation allows the replan logic to be tested independently.

#### Verification Criteria
- [ ] `cargo check -p roko-orchestrator` compiles
- [ ] `cargo test -p roko-orchestrator` passes including new replan test
- [ ] `crates/roko-orchestrator/src/replan.rs` exists and is re-exported
- [ ] orchestrate.rs callers (if any) delegate to the new location

---

### Task 3.26: Extract Context Bidding to roko-compose
**Priority**: P2
**Estimated Effort**: 5 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/context_bidding.rs` (may already exist at `context_provider.rs`)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/lib.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs`
**Depends On**: Task 3.24

#### Context
orchestrate.rs contains `AttentionBidder` variants (Neuro, Task, Research context bidders) and a VCG auction system for prompt assembly. This logic determines which context sections get included in the system prompt and at what priority. The `vcg_allocate` function computes optimal allocation.

`roko-compose` already has `context_provider.rs` at `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/context_provider.rs` (which has a `load_or_new` at line 462). The context bidding from orchestrate.rs should merge with the existing context provider infrastructure.

#### Implementation Steps
1. Identify the `AttentionBidder` types and `vcg_allocate` function in orchestrate.rs
2. Check if `context_provider.rs` already has equivalent functionality
3. If equivalent exists, verify feature parity and note any gaps
4. If not, extract the bidding types to `crates/roko-compose/src/context_bidding.rs`
5. Extract VCG auction to the same module or a submodule
6. Re-export from `crates/roko-compose/src/lib.rs`
7. Wire into `SystemPromptBuilder` as an optional enrichment step

#### Design Guidance
Context bidding is about selecting the most valuable context sections for a prompt within a token budget. This naturally belongs in `roko-compose` next to `SystemPromptBuilder`. The VCG auction is an allocation mechanism -- it should be a generic utility that bidders plug into, not tied to specific bidder implementations.

#### Verification Criteria
- [ ] `cargo check -p roko-compose` compiles
- [ ] `cargo test -p roko-compose` passes
- [ ] Context bidding is accessible from `roko_compose`
- [ ] `SystemPromptBuilder` can optionally use context bidding for section selection

---

### Task 3.27: Delete Dead PlanRunner from orchestrate.rs
**Priority**: P2
**Estimated Effort**: 8 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/lib.rs`
**Depends On**: Task 3.24, Task 3.25, Task 3.26

#### Context
After extracting valuable patterns (Tasks 3.25 and 3.26), the dead `PlanRunner` struct and its associated methods can be deleted. The goal is to reduce orchestrate.rs from 22,522 lines to under 5,000 (or less, ideally under 2,000).

The analysis from Task 3.24 identifies which exports are live. Everything else can be deleted.

#### Implementation Steps
1. Using the categorized export list from Task 3.24, identify all dead code
2. Delete the `PlanRunner` struct and all its methods
3. Delete `dispatch_agent_with()` and all private helpers that only serve PlanRunner
4. Delete `run_task_plans()` and callers
5. Keep all live exports (identified in Task 3.24)
6. For test-only exports, move them to a `#[cfg(test)]` block or a test helper module
7. Run `cargo check --workspace` after each major deletion to catch breakage early
8. Run `cargo test --workspace` after all deletions

#### Design Guidance
Delete in stages: start with the largest clearly-dead functions, verify compilation after each stage. The `#[cfg(feature = "legacy-orchestrate")]` feature gate may already protect some sections. Check if the feature is enabled in any CI configuration before removing gated code.

Do NOT delete anything that is imported by production code. The analysis task (3.24) must be completed first.

#### Verification Criteria
- [ ] `wc -l crates/roko-cli/src/orchestrate.rs` shows < 5000 lines
- [ ] `cargo check --workspace` compiles cleanly
- [ ] `cargo test --workspace` passes
- [ ] No live production imports are broken
- [ ] All extracted patterns (replan, context bidding) are accessible from their new locations

---

## Phase 11: Observability & Dashboard Integration

### Task 3.28: Emit RouterDecision Events from ModelCallService
**Priority**: P2
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/model_call_service.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/lib.rs` (or `events.rs`)
**Depends On**: Task 3.6

#### Context
`ModelCallService` emits `RuntimeEvent` variants through `event_consumers`. Currently it emits basic call completion events. After CascadeRouter integration (Task 3.6), it should also emit routing decision events so dashboards and the TUI can show why a particular model was selected.

#### Implementation Steps
1. Add a `RuntimeEvent::RouterDecision` variant (or equivalent) to `roko-core`:
   ```rust
   RouterDecision {
       model: String,
       source: String,  // "cascade_router", "role_config", "cli_override", etc.
       candidates: Vec<(String, f64)>,  // (model, score) pairs
       reason: String,
       estimated_cost_usd: f64,
   }
   ```
2. In `ModelCallService::call()`, after model resolution, emit the decision:
   ```rust
   self.emit(RuntimeEvent::RouterDecision { ... });
   ```
3. If a routing observer is available, include candidate scores from the router
4. Include the `SelectionSource` label in the event

#### Design Guidance
The event should be lightweight -- no large payloads. Include only what dashboards need: which model was chosen, why, what alternatives were considered, and expected cost. The TUI's router trace card can display this data.

#### Verification Criteria
- [ ] `cargo check -p roko-agent` and `cargo check -p roko-core` compile
- [ ] A test that dispatches through `ModelCallService` with an event consumer receives a `RouterDecision` event
- [ ] The event includes model, source, and reason fields

---

### Task 3.29: Expose Provider Health via HTTP API
**Priority**: P3
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/providers.rs`
**Depends On**: Task 3.15

#### Context
The serve runtime at `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/providers.rs` already has provider-related routes. The serve state at `state.rs:377` already holds a `ProviderHealthTracker`. A `/api/providers/health` endpoint should expose the tracker's state.

#### Implementation Steps
1. Add `GET /api/providers/health` route handler
2. Query `state.provider_health` for all known providers
3. Return JSON:
   ```json
   {
     "providers": {
       "anthropic_api": { "state": "healthy", "consecutive_failures": 0, "total_attempts": 42, "total_successes": 41 },
       "cerebras_api": { "state": "unhealthy", "consecutive_failures": 3, "total_attempts": 10, "total_successes": 7 }
     }
   }
   ```
4. Add `pub fn snapshot(&self) -> HashMap<String, ProviderStatusSnapshot>` to `ProviderHealthTracker` to export current state
5. Wire the route in the serve router

#### Design Guidance
The endpoint should be read-only and cheap. The `ProviderHealthTracker` uses a `RwLock` internally, so reading is non-blocking. Return only the fields that are useful for monitoring: state, failure count, success count, last success/failure timestamps.

#### Verification Criteria
- [ ] `cargo check -p roko-serve` compiles
- [ ] `curl localhost:6677/api/providers/health` returns valid JSON
- [ ] Health data reflects actual provider state

---

## Phase 12: Advanced Dispatch Strategies

### Task 3.30: Implement Speculative Decoding Pattern
**Priority**: P3
**Estimated Effort**: 6 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/model_call_service.rs`
**Depends On**: Task 3.6, Task 3.14

#### Context
For interactive paths, dispatching to a fast model (Haiku/Flash) while simultaneously starting a slower premium model can reduce perceived latency. If the fast model's output passes a quality check, cancel the slow model and return immediately.

The `ModelCallService` already has `fallback_models` and cost prediction. It needs a new `call_speculative()` method that races a fast model against a premium model.

#### Implementation Steps
1. Add `pub async fn call_speculative(&self, req: ModelCallRequest) -> Result<ModelCallResponse>`:
   ```rust
   let fast_model = self.fastest_viable_model(&req);
   let premium_model = self.resolve_model(&req);

   if fast_model == premium_model {
       return self.call(req).await;  // No speculative benefit
   }

   let (fast_result, premium_handle) = tokio::join!(
       self.call_model(&req, &fast_model),
       tokio::spawn(self.call_model(&req, &premium_model)),
   );
   ```
2. Add quality check: if fast result is successful and the response length > threshold (suggesting a complete answer), use it and cancel premium
3. If fast result fails or is too short, await premium
4. Track both calls in cost accounting (fast model cost is wasted if premium is used)
5. Add `with_speculative_threshold(min_quality_score: f64)` builder

#### Design Guidance
Speculative decoding is a latency optimization. It increases cost (~10-20% more from abandoned fast calls) but reduces p50 latency significantly for simple queries. Only enable for interactive paths (`roko chat`, `roko <prompt>`), not for plan execution where latency is less important.

The quality check should be simple: response length > 100 chars AND no error indicators. A more sophisticated check (confidence score, semantic similarity) can be added later.

#### Verification Criteria
- [ ] `cargo check -p roko-agent` compiles
- [ ] A test demonstrates speculative decoding: fast model returns quickly, premium is cancelled
- [ ] Cost accounting tracks both the fast model call and any premium model call
- [ ] `call_speculative` falls back to standard `call` when only one model is available

---

### Task 3.31: Implement Cost-Optimized Batch Routing
**Priority**: P3
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/model_call_service.rs`
**Depends On**: Task 3.10, Task 3.6

#### Context
For plan execution with many tasks, pre-computing an optimal routing plan within a budget constraint can reduce total cost while maintaining quality for critical tasks.

The `CostTable` has per-model pricing. The `CascadeRouter` has per-model quality estimates. Combined, they can produce a cost-quality Pareto-optimal assignment.

#### Implementation Steps
1. Add `pub fn plan_batch_routing(&self, tasks: &[BatchTask], budget_usd: f64) -> Vec<(String, String)>`:
   - `BatchTask` has `task_id: String`, `complexity: f64`, `required_quality: f64`
   - Returns `Vec<(task_id, model_slug)>` assignment
2. Sort tasks by complexity (descending)
3. For each task, find the cheapest model that meets the quality threshold
4. Track cumulative cost; if budget would be exceeded, downgrade remaining tasks
5. Return the assignment

#### Design Guidance
This is a greedy assignment algorithm -- not truly optimal, but good enough. The VCG auction from orchestrate.rs (if extracted in Task 3.26) could provide a more sophisticated allocation, but the greedy approach is simpler and sufficient for the initial implementation.

#### Verification Criteria
- [ ] `cargo check -p roko-agent` compiles
- [ ] Unit test: 10 tasks with $5 budget assigns cheap models to low-complexity tasks
- [ ] Unit test: budget exceeded triggers model downgrade for remaining tasks
- [ ] Total assigned cost does not exceed budget

---

## Phase 13: Episode/Feedback Unification

### Task 3.32: Emit Episode Records from One-Shot Paths
**Priority**: P1
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch_v2.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/chat_inline.rs`
**Depends On**: none

#### Context
The runner v2 path (`crates/roko-cli/src/runner/event_loop.rs`) writes episodes to `.roko/episodes.jsonl` and efficiency events to `.roko/learn/efficiency.jsonl`. But the one-shot paths (`roko <prompt>` via `dispatch_v2.rs`, `roko chat` via `chat_inline.rs`) write neither.

`ModelCallService` records `FeedbackEvent::ModelCall` via `FeedbackSink` (wired in `dispatch_v2.rs:88-92`). But `FeedbackService` writes to `.roko/learn/feedback.jsonl`, not to the episode log. The episode format expected by the learning subsystem is different.

#### Implementation Steps
1. In `dispatch_v2.rs`, after `ModelCallService::call()` returns, construct and append an episode record:
   ```rust
   let episode = Episode {
       timestamp: Utc::now(),
       run_id: format!("dispatch-v2:{}", uuid::Uuid::new_v4()),
       model: response.model.clone(),
       role: "inline".to_string(),
       input_tokens: response.usage.input_tokens,
       output_tokens: response.usage.output_tokens,
       cost_usd: response.usage.cost_usd,
       latency_ms: response.latency_ms,
       success: true,
       entry_point: "roko_inline".to_string(),
   };
   persist::append_jsonl(&episodes_path, &episode)?;
   ```
2. Similarly in `chat_inline.rs`, emit an episode after each turn
3. Use the same episode format as `runner/event_loop.rs` for consistency
4. Emit efficiency events alongside episodes (wall time, token efficiency)

#### Design Guidance
Episodes are the canonical learning signal. Without them, the CascadeRouter, prompt experiments, and efficiency analysis have no data from interactive use. The format must match what `roko-learn` expects -- check `roko_learn::episode_logger::Episode` for the canonical shape.

#### Verification Criteria
- [ ] `cargo run -p roko-cli -- run "echo hello"` produces an entry in `.roko/episodes.jsonl`
- [ ] `roko chat` produces episodes for each turn
- [ ] Episode format matches `Episode` struct serialization
- [ ] `cargo test -p roko-cli` passes

---

### Task 3.33: Persist CostMeter Data to Durable Log
**Priority**: P2
**Estimated Effort**: 2 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/chat_inline.rs`
**Depends On**: Task 3.32

#### Context
`CostMeter` in `chat_inline.rs` tracks in-memory cost data during a chat session. This data is displayed in the TUI but lost on exit. It should be persisted to `.roko/learn/costs.jsonl` so cumulative cost tracking works across sessions.

#### Implementation Steps
1. At chat session exit, serialize `CostMeter` summary to a cost record
2. Append to `.roko/learn/costs.jsonl`
3. Include: session_id, total_cost, model breakdown, turn count, duration
4. The `roko learn efficiency` command should include cost data in its report

#### Design Guidance
Cost records are append-only JSONL. Each session produces one summary record. Per-turn cost is already captured in episodes (Task 3.32). The session summary provides aggregate data for trend analysis.

#### Verification Criteria
- [ ] After a `roko chat` session, `.roko/learn/costs.jsonl` has a new entry
- [ ] The entry includes total cost and model breakdown
- [ ] `roko learn efficiency` can read and report on cost data

---

## Phase 14: Dispatch Path Unification

### Task 3.34: Route dispatch_direct.rs Through ModelCallService
**Priority**: P2
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch_direct.rs`
**Depends On**: Task 3.10, Task 3.14

#### Context
`dispatch_direct.rs` at `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch_direct.rs` is already gated behind `#[cfg(feature = "legacy-orchestrate")]`. It has three dispatch functions:
- `dispatch_claude_cli()` -- bare Claude CLI subprocess
- `dispatch_anthropic_api()` -- direct HTTP to Anthropic
- `dispatch_openai_compat()` -- direct HTTP to OpenAI

All three bypass the provider system, feedback, health tracking, and budget enforcement.

#### Implementation Steps
1. Check if the `legacy-orchestrate` feature is enabled in any production build configuration
2. If not enabled in production, this task is deprioritized -- the gated code is already dead
3. If enabled, replace each function's body with a delegation to `ModelCallService`:
   ```rust
   pub async fn dispatch_claude_cli(prompt: &str, config: &RokoConfig) -> Result<DispatchResult> {
       dispatch_via_model_call_service(prompt).await
   }
   ```
4. Remove the manual `Command::new("claude")` and `reqwest::Client` usage
5. Remove `extract_clean_text` import (uses `chat::extract_clean_text`)

#### Design Guidance
If the legacy feature gate is not enabled anywhere, leave this as-is -- it will be cleaned up when the feature gate is removed entirely. Do not spend effort on dead code migration unless the feature is actively used.

#### Verification Criteria
- [ ] `cargo check -p roko-cli` compiles (both with and without `legacy-orchestrate` feature)
- [ ] If feature is enabled, all dispatch goes through `ModelCallService`
- [ ] If feature is disabled, no behavior change

---

### Task 3.35: Unify dispatch_v2.rs Config Construction
**Priority**: P1
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch_v2.rs`
**Depends On**: Task 3.10

#### Context
`dispatch_via_model_call_service()` at `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch_v2.rs:53-100` manually constructs a `RokoConfig` by copying fields from the layered config one by one (lines 67-80). This is fragile -- new config fields must be added to this copy loop.

The same manual copy pattern exists in `run.rs` (lines 430-449). Both should use a shared config preparation function.

#### Implementation Steps
1. Extract the config construction logic (lines 62-86 of `dispatch_v2.rs`) into a shared function:
   ```rust
   pub fn prepare_model_config(workdir: &Path) -> anyhow::Result<(Config, RokoConfig)> {
       let config = crate::config::load_layered(workdir)
           .map(|r| r.config)
           .unwrap_or_default();
       let mut model_config = RokoConfig::default();
       model_config.merge_from(&config);  // or field-by-field, but in one place
       Ok((config, model_config))
   }
   ```
2. Use this shared function in both `dispatch_v2.rs` and `run.rs`
3. Verify that all config fields are properly forwarded (compare the field-by-field copy in both files)
4. Add any missing fields from `run.rs` that `dispatch_v2.rs` doesn't copy (or vice versa)

#### Design Guidance
Config preparation should happen in exactly one place. If `RokoConfig` gains a new field, only one function needs updating. The shared function should be in a `config_helpers.rs` or `config_prep.rs` module, not duplicated across dispatch files.

#### Verification Criteria
- [ ] `cargo check -p roko-cli` compiles
- [ ] `dispatch_v2.rs` and `run.rs` both use the shared config function
- [ ] No field-by-field config copying remains in dispatch files
- [ ] `cargo test -p roko-cli` passes

---

## Phase 15: CostTable Auto-Population

### Task 3.36: Auto-Populate CostTable from OpenRouter Metadata
**Priority**: P3
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/task_runner.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/provider/openrouter_meta.rs`
**Depends On**: none

#### Context
`CostTable` in `task_runner.rs` stores per-model pricing for cost calculation. It is currently populated manually. The `OpenRouter` metadata helper at `provider/openrouter_meta.rs` can fetch live pricing via the OpenRouter catalog API but is not wired to auto-populate the cost table.

#### Implementation Steps
1. Add `pub async fn fetch_pricing(models: &[String]) -> HashMap<String, ModelPricing>` to `openrouter_meta.rs`
2. Cache fetched pricing to `.roko/learn/model-pricing.json` with a 24-hour TTL
3. On `CostTable` construction, if a cached pricing file exists and is fresh, load it
4. If stale or missing and an OpenRouter API key is configured, fetch and cache
5. Merge fetched pricing with any hardcoded defaults (hardcoded takes precedence for known models)

#### Design Guidance
Pricing fetch should be opportunistic, not blocking. If the fetch fails (no API key, network error), use hardcoded defaults silently. The 24-hour TTL prevents excessive API calls while keeping prices reasonably current. Log when pricing is refreshed.

#### Verification Criteria
- [ ] `cargo check -p roko-agent` compiles
- [ ] With OpenRouter API key set, pricing is fetched and cached
- [ ] Without API key, hardcoded defaults are used
- [ ] Cached pricing is used on subsequent startups within 24 hours

---

### Task 3.37: Add Cache Hit Rate Metrics to CacheCell
**Priority**: P3
**Estimated Effort**: 2 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/model_call_service.rs`
**Depends On**: none

#### Context
`CacheCell` (L1 response cache, 128 entries, exact match) in `ModelCallService` has no metrics. Cannot measure cache hit rate, size, or savings.

#### Implementation Steps
1. Add counters to `CacheCell`: `hits: AtomicU64`, `misses: AtomicU64`, `evictions: AtomicU64`
2. Add `pub fn metrics(&self) -> CacheMetrics` returning a snapshot
3. Emit cache metrics via `RuntimeEvent` periodically or on session end
4. Log cache hit rate at `tracing::debug!` level on each lookup

#### Design Guidance
Use atomics for thread-safe counting without locks. The metrics snapshot should be cheap to produce. Include hit rate percentage and estimated cost savings (hits * average call cost) in the metrics.

#### Verification Criteria
- [ ] `cargo test -p roko-agent` passes
- [ ] After N calls with some duplicates, `CacheMetrics.hits` > 0
- [ ] Hit rate is calculated correctly: `hits / (hits + misses)`

---

### Task 3.38: Add ToolLoop Max Iterations to ModelProfile Config
**Priority**: P3
**Estimated Effort**: 2 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/schema.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/provider/cerebras.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/provider/openai_compat.rs`
**Depends On**: none

#### Context
Cerebras adapter hardcodes `tool_loop_max_iterations(50)` while OpenAI-compat uses `30`. These should be configurable per model/provider.

#### Implementation Steps
1. Add `max_tool_iterations: Option<u32>` to `ModelProfile` in `crates/roko-core/src/config/schema.rs`
2. In Cerebras adapter, use `model.max_tool_iterations.unwrap_or(50)`
3. In OpenAI-compat adapter, use `model.max_tool_iterations.unwrap_or(30)`
4. Document in roko.toml comments

#### Design Guidance
Use `Option` with provider-specific defaults. The per-model config takes precedence over the per-provider default. This allows users to set `max_tool_iterations = 100` for a specific model without affecting others.

#### Verification Criteria
- [ ] `cargo check -p roko-core` and `cargo check -p roko-agent` compile
- [ ] Setting `max_tool_iterations` in a model profile changes the iteration limit
- [ ] Default behavior unchanged when field is absent

---

## Dependency Graph

```
Phase 0 (CascadeRouter wiring)
  Task 3.1 ──→ Task 3.2 ──→ Task 3.4
  Task 3.1 ──→ Task 3.3
  Task 3.1 ──→ Task 3.5
  Task 3.1 ──→ Task 3.6

Phase 1 (Stream parser)          Phase 2 (Budget)            Phase 3 (Env keys)
  Task 3.7 ──→ Task 3.8         Task 3.10 ──→ Task 3.11     Task 3.12  (independent)
  Task 3.7 ──→ Task 3.9                                      Task 3.13  (independent)

Phase 4 (Provider health)        Phase 5 (Thinking)
  Task 3.14 ──→ Task 3.15       Task 3.17  (independent)
  Task 3.14 ──→ Task 3.16

Phase 6 (ACP)                    Phase 7 (Quirks)            Phase 8 (Hardcoded)
  Task 3.18 ──→ Task 3.19       Task 3.20  (independent)    Task 3.21  (needs 3.3)
                                                              Task 3.22  (independent)

Phase 9 (Rate limiter)           Phase 10 (orchestrate.rs decomp)
  Task 3.23  (independent)      Task 3.24 ──→ Task 3.25
                                 Task 3.24 ──→ Task 3.26
                                 Task 3.25 ──→ Task 3.27
                                 Task 3.26 ──→ Task 3.27

Phase 11 (Observability)         Phase 12 (Advanced)
  Task 3.28  (needs 3.6)        Task 3.30  (needs 3.6, 3.14)
  Task 3.29  (needs 3.15)       Task 3.31  (needs 3.10, 3.6)

Phase 13 (Episodes)              Phase 14 (Dispatch unify)   Phase 15 (Polish)
  Task 3.32  (independent)      Task 3.34  (needs 3.10)     Task 3.36  (independent)
  Task 3.33  (needs 3.32)       Task 3.35  (independent)    Task 3.37  (independent)
                                                              Task 3.38  (independent)
```

**Critical path**: 3.1 → 3.2 → 3.4 (CascadeRouter wired into plan run)
**Parallel tracks**: Phases 1-3 can all run independently of Phase 0.
**Estimated total**: 15-25 days for full completion.

Phases 0, 2, 3, 8 are P0/P1 -- essential for self-hosting quality.
Phases 1, 4, 5, 6, 7, 9 are P1/P2 -- reliability and correctness.
Phases 10-15 are P2/P3 -- polish and advanced features.

---

## Sources

| File | Purpose |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/model_call_service.rs` | ModelCallService (2,143 LOC) |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/provider/mod.rs` | Provider adapter registry |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/provider/claude_cli/stream.rs` | Canonical stream parser |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/translate/mod.rs` | Response translation with 4096 truncation |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/openai_compat_backend.rs` | OpenAI-compat backend with boolean flags |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/usage.rs` | UsageObservation (74 LOC) |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/retry.rs` | RetryPolicy with full-jitter backoff |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/provider/cerebras.rs` | Cerebras adapter (strict mode) |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/model_selection.rs` | 6-tier model selection (581 LOC) |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch_v2.rs` | v2 dispatch entry point (946 LOC) |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch_direct.rs` | Legacy dispatch (feature-gated) |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/run.rs` | `roko run` universal loop |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/chat.rs` | Chat REPL + extract_clean_text |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/chat_inline.rs` | Chat inline dispatch |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/auth_detect.rs` | Auth detection (hardcoded models) |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs` | God object (22,522 LOC) |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/plan.rs` | Plan commands with CascadeRouter |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/types.rs` | Runner v2 types (loads router) |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/serve_runtime.rs` | Serve runtime (loads router) |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/cascade_router.rs` | CascadeRouter LinUCB bandit |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/provider_health.rs` | ProviderHealthTracker circuit breaker |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/feedback_service.rs` | FeedbackService |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/budget.rs` | BudgetConfig (max_plan/turn_usd) |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/schema.rs` | Config schema |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-neuro/src/episode_completion.rs` | Direct env key read (ANTHROPIC_API_KEY) |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-std/src/tool/builtin/web_search.rs` | Direct env key read (PERPLEXITY_API_KEY) |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/runner.rs` | ACP bare subprocess |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/bridge_events.rs` | ACP event bridge |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-orchestrator/src/service_factory.rs` | ServiceFactory (loads router) |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/context_provider.rs` | Context bidder registry |
