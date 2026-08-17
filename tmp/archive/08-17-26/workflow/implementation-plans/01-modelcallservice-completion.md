# 01 — ModelCallService: Finish the Unification

> Foundation Phase 0.1 of `tmp/workflow/UNIFIED-IMPLEMENTATION-PLAN.md`. Cross-references audit `tmp/workflow/09-inference-dispatch-audit.md`.

---

## Status (2026-05-01)

**PARTIAL.** The trait + service exist; ~3 of 7 entry points use them; ~4 still bypass.

**What's done:**

- `roko_core::foundation::ModelCaller` trait — `crates/roko-core/src/foundation.rs:230-243`
- `roko_core::foundation::ModelCallRequest` / `ModelCallResponse` / `ModelStreamEvent` / `BoxModelStream`
- `roko_agent::ModelCallService` (concrete impl) — `crates/roko-agent/src/model_call_service.rs`
- Used by: `roko-cli/chat_session.rs` (chat), `roko-acp/bridge_events.rs` (ACP cognitive dispatch via `run_anthropic_cognitive_task` and `run_openai_compat_cognitive_task`), `roko-serve` (gateway routes), `roko-orchestrator/service_factory.rs`
- One canonical Claude CLI stream-json line parser at `crates/roko-agent/src/provider/claude_cli/stream.rs:parse_stream_line`, re-exported from `crates/roko-cli/src/runner/agent_stream.rs`
- `crates/roko-cli/src/dispatch_direct.rs` is `#![cfg(feature = "legacy-orchestrate")]`-gated and excluded from default builds
- `crates/roko-core/src/dispatch_plan.rs` defines `DispatchPlan` / `DispatchRequest` types
- `crates/roko-agent/src/dispatch_resolver.rs` defines `DispatchResolver::resolve_existing(...)` returning a `DispatchPlan` (planning only, no execution)

**What's not:**

- `ModelCaller::stream` is **post-hoc chunked** (default trait impl waits for `call()` to complete, then emits a synthetic stream). True token-by-token streaming through `ModelCallService` is missing.
- **Bypassing entry points** (still call `spawn_agent_scoped` / `create_agent_for_model` directly):
  - `roko run` — `crates/roko-cli/src/run.rs` (search for `TODO(gateway): migrate to ModelCallService` ~line 1826)
  - `roko plan run` (default path) — `crates/roko-cli/src/runner/event_loop.rs` + `dispatch_v2.rs`
  - `roko prd` / research / plan-generate — `crates/roko-cli/src/agent_exec.rs`
  - `AgentJudgeOracle` (LLM judge gate) — `crates/roko-cli/src/orchestrate.rs:~3093-3159` (uses `run_prepared_agent`)
  - Episode distillation — `crates/roko-neuro/src/episode_completion.rs` (reads `ANTHROPIC_API_KEY` directly)
  - Web-search tool — `crates/roko-std/src/tool/builtin/web_search.rs`
  - Dreams runner — `crates/roko-dreams/src/runner.rs`
- **`InferenceGateway`** type from the v2 docs does not exist. Today the role is split between `ModelCallService` (per-call wiring) and `ServiceFactory` (build-time wiring).
- `extract_clean_text` (246-line monster in `crates/roko-cli/src/chat.rs:555-659`) still required by `dispatch_direct`, `chat_inline`, and `agent_serve` for legacy parsing.
- `OpenAiAgent` in `crates/roko-agent/src/openai_agent.rs` is a separate, non-streaming HTTP client — bypasses both `ModelCallService` and `OpenAiCompatAdapter` for some callers.

---

## Goal

**Every** LLM call in the binary goes through `roko_agent::ModelCallService` (or a thin facade implementing `roko_core::ModelCaller`). Entry-point bypasses listed above are migrated; legacy code paths are deleted or feature-gated such that they cannot be reached from the default build.

---

## Why This Exists (Anti-Patterns Eliminated)

1. **#1 Just Shell Out To Claude** — multiple `Command::new("claude")` invocations
2. **#7 Copy-Paste Between Runtimes** — duplicated stream-json parsing
3. **#6 Feedback as Afterthought** — bypass paths don't record episodes / cost / routing
4. **#10 God File** — fragments of dispatch live in 8+ files

See `tmp/workflow/ANTI-PATTERNS.md` for the full list.

---

## Existing Code — Read These First

```230:243:crates/roko-core/src/foundation.rs
#[async_trait]
pub trait ModelCaller: Send + Sync {
    async fn call(&self, req: ModelCallRequest) -> Result<ModelCallResponse>;

    async fn stream(&self, req: ModelCallRequest) -> Result<BoxModelStream> {
        Ok(match self.call(req).await {
            Ok(response) => model_call_response_to_stream(response),
            Err(error) => model_call_failure_to_stream(error),
        })
    }
}
```

The `ModelCallService` struct (in `crates/roko-agent/src/model_call_service.rs`) exposes builders:

- `pub fn new(default_model: String) -> Self`
- `pub fn with_config(self, config: RokoConfig) -> Self`
- `pub fn with_feedback_sink(self, sink: Arc<dyn FeedbackSink>) -> Self`
- `pub fn with_gateway_event_writer(self, writer: ...) -> Self`
- `pub fn with_cache(...)`, `with_router(...)`, `with_episode_logger(...)`

`impl ModelCaller for ModelCallService` implements `call` only; it does **not** override `stream`. The path inside `call`: route → `ProviderCallCell` → adapter → cost/feedback/event hooks.

`ServiceFactory` (`crates/roko-orchestrator/src/service_factory.rs`) is the build-time wiring point. Use it instead of constructing services manually. Look at `ServiceFactory::build()` to see how the production `EffectServices` is assembled.

---

## Implementation Steps

### Step 1 — Add true streaming to `ModelCallService`

The trait already supports streaming via `BoxModelStream`. Override `stream` in `ModelCallService` so it stops being the default post-hoc shim.

#### 1a. Define a per-adapter streaming surface

Each provider adapter currently implements `ProviderAdapter` (synchronous-style `call`). Extend with an optional streaming method:

```rust
// crates/roko-agent/src/provider/mod.rs (sketch)
#[async_trait]
pub trait StreamingProviderAdapter: ProviderAdapter {
    async fn stream(
        &self,
        req: AdapterRequest,
        ctx: AdapterCtx,
    ) -> ProviderResult<BoxAdapterStream>;
}

pub type BoxAdapterStream =
    Pin<Box<dyn Stream<Item = AdapterStreamChunk> + Send>>;

pub enum AdapterStreamChunk {
    Started { model: String },
    ContentDelta(String),
    ToolCallDelta { id: String, name: String, args_json: String },
    Usage(TokenUsage),
    Done { stop_reason: Option<String> },
    Error(String),
}
```

#### 1b. Implement `StreamingProviderAdapter` for the four real adapters

- **`ClaudeCliAdapter`** (`crates/roko-agent/src/provider/claude_cli/`): subprocess spawn with `--output-format stream-json`, parse via existing `parse_stream_line`, map `ClaudeStreamEvent` → `AdapterStreamChunk`.
- **`AnthropicApiAdapter`** (`crates/roko-agent/src/claude_agent.rs` is the source — extract HTTP+SSE streaming; the API supports it natively).
- **`OpenAiCompatAdapter`** (`crates/roko-agent/src/openai_compat_backend.rs` already has SSE delta logic — surface it through this trait).
- **`GeminiAdapter`** (`crates/roko-agent/src/gemini/native.rs`).

`PerplexityAdapter` and `OllamaAdapter` are HTTP-streamable too but lower priority; stub them to fall back to non-streaming until needed.

#### 1c. Override `ModelCallService::stream`

```rust
// crates/roko-agent/src/model_call_service.rs (sketch)
#[async_trait]
impl ModelCaller for ModelCallService {
    async fn call(&self, req: ModelCallRequest) -> Result<ModelCallResponse> { /* existing */ }

    async fn stream(&self, req: ModelCallRequest) -> Result<BoxModelStream> {
        let resolved = self.resolve(&req).await?;          // existing routing path
        let adapter = self.adapter_for(&resolved.provider)?;
        match adapter.streaming() {
            Some(streamer) => {
                let raw = streamer.stream(resolved.adapter_request, resolved.ctx).await?;
                Ok(self.bridge_stream(raw, resolved.metadata))   // converts AdapterStreamChunk → ModelStreamEvent and feeds FeedbackService once Done seen
            }
            None => {
                // Fall back to default shim: call(), then chunk
                Ok(crate::default_chunked_stream(self, req).await)
            }
        }
    }
}
```

The `bridge_stream` helper is critical: it must (a) emit `ModelStreamEvent`s in real time, (b) accumulate token usage, (c) call `FeedbackService::record(FeedbackEvent::ModelCall { ... })` once on `Done` or `Error`, (d) populate the `Caller`'s `request_id`. Do not call `FeedbackService` per chunk.

#### 1d. Wire chat path to use real streaming

`crates/roko-cli/src/chat_session.rs:send_turn_streaming` (~ line 611) already calls `.stream(...)`. Verify it now delivers `ContentDelta` events live (not all at the end). Add a regression test that asserts `ContentDelta` arrives **before** `Done` for a Claude CLI provider.

### Step 2 — Migrate `roko run` to `ModelCallService`

**File:** `crates/roko-cli/src/run.rs` (~1555 LOC). Search for `spawn_agent_scoped` and the `TODO(gateway)` comment around line 1826.

Replace `spawn_agent_scoped(spec, ...)` calls with:

```rust
let service = ServiceFactory::from_state(state).model_call_service()?;
let response = service
    .call(ModelCallRequest {
        model: resolved_model,
        system: Some(assembled_prompt),
        messages: vec![ChatMessage { role: MessageRole::User, content: user_prompt }],
        role: Some(role.to_string()),
        caller: Some(caller::CLI.to_string()),
        run_id: Some(run_id.clone()),
        prompt_section_ids,
        knowledge_ids,
        budget,
        budget_remaining,
        routing_hints,
        cache_policy: CachePolicy::Default,
        ..Default::default()
    })
    .await?;
```

**Important:** `roko run` runs gates after the agent. The current `RunReport` contract must continue to expose `output_text`, `gate_verdicts`, `episode_id`. The `ModelCallService` already records the episode via `FeedbackService` if attached; do not double-record.

**Plumbing needed:** Build the prompt via `PromptAssemblyService` (see plan 02). Until that lands, you can pass the prompt the existing `dispatch_helpers::build_system_prompt_with_context_validated` returns, but mark this with a `TODO(plan-02)` so it's clear what the next step is.

### Step 3 — Migrate `roko plan run` (default `event_loop`)

**Files:**

- `crates/roko-cli/src/runner/event_loop.rs` (~3000 LOC) — the active plan runner
- `crates/roko-cli/src/dispatch/mod.rs` + `dispatch/model_routing.rs` + `dispatch_v2.rs`

Today the runner spawns agents via `Dispatcher::spawn_agent` → `runner::agent_stream::spawn_agent` (subprocess). Replace this in two phases:

**Phase 3a (this plan):** Behind a runtime flag `roko.toml: [runner].use_workflow_engine = true`, route per-task spawns through `ModelCallService::call`. Keep the existing path active by default; gate the new path on the flag for safety.

```rust
// crates/roko-cli/src/runner/event_loop.rs (sketch)
let result = if state.use_workflow_engine {
    spawn_via_model_call_service(&state, &task, prompt).await?
} else {
    Dispatcher::spawn_agent_legacy(...)
};
```

**Phase 3b (deferred — covered by plan 11):** Replace the loop entirely with `WorkflowEngine` driving multi-task execution via `TaskScheduler` (plan 06).

**Why phase it:** The event loop owns ~12 things (streaming, gates, merge queue, replan, fingerprint resume). All must work after migration; partial migration is fine if both paths satisfy proof tests.

### Step 4 — Migrate `agent_exec.rs` callers (PRD / research / plan-generate)

**File:** `crates/roko-cli/src/agent_exec.rs`. Functions: `run_agent`, `run_agent_capture`, `run_agent_logged`.

Today these functions construct an `Agent` via `spawn_agent_scoped` → `create_agent_for_model` → `Agent::run`. They are short, surgical helpers used by:

- `roko prd` (PRD lifecycle)
- `roko plan generate`
- Several research subcommands

Refactor to call `ModelCallService` directly. The shape is one-shot, so `service.call(req).await` works without streaming.

```rust
// agent_exec.rs (sketch)
pub async fn run_agent(opts: AgentExecOpts) -> Result<AgentExecResult> {
    let service = ServiceFactory::from_workdir(&opts.workdir).await?.model_call_service();
    let req = build_request_from_opts(&opts)?;
    let response = service.call(req).await?;
    persist_capture_episode_if_needed(&opts, &response).await?;
    Ok(AgentExecResult::from_response(response))
}
```

**Things NOT to do:**

- Do not duplicate `persist_capture_episode` logic. Move it inside `FeedbackService` so `service.call(req)` automatically writes the episode if `with_feedback_sink` was attached. Then delete the helper.
- Do not pass `dangerously_skip_permissions: true` blindly. Use `role_allows_dangerous_skip_permissions(role)` from `run.rs` (lift to a shared util).

### Step 5 — Migrate `AgentJudgeOracle` (LLM judge gate)

**File:** `crates/roko-cli/src/orchestrate.rs:~3093-3159` (currently `legacy-orchestrate`-feature-gated).

`AgentJudgeOracle` hardcodes `command: "claude"` + `model: "claude-sonnet-4-20250514"` + `skip_permissions: true`. This is anti-pattern #1 in pure form.

Move the oracle to `crates/roko-gate/src/llm_judge_oracle.rs`:

```rust
pub struct LlmJudgeOracle {
    service: Arc<dyn ModelCaller>,
    judge_role: String,        // "judge"
    default_model: Option<String>,
}

impl LlmJudgeOracle {
    pub async fn judge(&self, prompt: String, context: String) -> JudgeVerdict {
        let req = ModelCallRequest {
            model: self.default_model.clone().unwrap_or_default(),
            system: Some(self.judge_system_prompt()),
            messages: vec![
                ChatMessage { role: MessageRole::User, content: prompt },
                ChatMessage { role: MessageRole::User, content: format!("Context:\n{context}") },
            ],
            role: Some(self.judge_role.clone()),
            caller: Some("gate.judge".to_string()),
            cache_policy: CachePolicy::Bypass,           // judge calls should not be cached
            ..Default::default()
        };
        match self.service.call(req).await {
            Ok(resp) => parse_judge_verdict(&resp.content),
            Err(e) => JudgeVerdict::Error(e.to_string()),
        }
    }
}
```

Wire it inside `crates/roko-gate/src/llm_judge_gate.rs::JudgeOracle`. Inject the `Arc<dyn ModelCaller>` from `ServiceFactory`.

Delete `AgentJudgeOracle` from `orchestrate.rs` once the legacy caller chain no longer references it.

### Step 6 — Migrate distillation, dreams, web-search

For each:

- `crates/roko-neuro/src/episode_completion.rs` — `Distiller` calls Claude API directly. Inject an `Arc<dyn ModelCaller>` (caller surface `"neuro.distillation"`, `cache_policy: ForceRefresh`).
- `crates/roko-dreams/src/runner.rs` — `dream_agent = create_agent_for_model(...)`. Replace with `ModelCallService` call (caller surface `"dreams"`).
- `crates/roko-std/src/tool/builtin/web_search.rs` — Perplexity HTTP call. Wire through `ModelCallService` with `caller: "tool.web_search"`. (Perplexity adapter likely already in `crates/roko-agent/src/perplexity/`; route through it.)

In each case **delete the direct env-var read** (`std::env::var("ANTHROPIC_API_KEY")`) — credential resolution lives in `ModelCallService` only. The `roko_core::caller` constants are the canonical caller-surface ids.

### Step 7 — Delete `extract_clean_text`

**File:** `crates/roko-cli/src/chat.rs:555-659+`.

This function handles 13 distinct response shapes because callers send raw JSON without typed parsing. After steps 1–6, all live callers receive `ModelCallResponse` (typed). Audit each call site:

- `crates/roko-cli/src/dispatch_direct.rs` — already feature-gated; either delete the file (Phase 6 retirement) or keep `extract_clean_text` only inside the `legacy-orchestrate` feature.
- `crates/roko-cli/src/chat_inline.rs` — switch to `ModelCallResponse.content`.
- `crates/roko-cli/src/agent_serve.rs` — switch to `ModelCallResponse.content`.

Once no live callers remain, move `extract_clean_text` behind `#[cfg(feature = "legacy-orchestrate")]` or delete outright.

### Step 8 — Make `dispatch_direct.rs` un-callable from default builds

It's already feature-gated. Verify with:

```
cargo build --bin roko --no-default-features
rg 'dispatch_direct' crates/roko-cli/src/ --type rust
```

The only references should be `#[cfg(feature = "legacy-orchestrate")]` modules and a `pub mod dispatch_direct;` line guarded the same way.

### Step 9 — Add `caller` plumbing tests

```rust
// tests/feedback_caller_surfaces.rs
#[tokio::test]
async fn every_caller_surface_is_covered() {
    for surface in [caller::CLI, caller::SERVE, caller::RESEARCH, caller::DREAMS,
                    "neuro.distillation", "tool.web_search", "gate.judge", "acp"] {
        let req = ModelCallRequest { caller: Some(surface.into()), ..req() };
        let captured = service_with_capture().call(req).await?;
        assert_eq!(captured.caller, Some(surface.into()));
    }
}
```

This forces every code path that wants to call an LLM to declare its caller surface, which feeds routing + cost analytics.

---

## Anti-Patterns To Avoid

| ID | Manifestation | How to avoid |
|---|---|---|
| #1 Just shell out | Adding a new `Command::new("claude")` anywhere outside `provider/claude_cli/` | All subprocess spawns live in one provider adapter |
| #2 Inline prompt strings | Hardcoding judge / distiller prompts inside the migration | Move prompts into `roko-compose/src/templates/` (see plan 02) |
| #3 Build another runtime | Creating a parallel "gateway" struct just for one caller | One `ModelCallService` for all callers |
| #4 Features in wrong layer | Adding budget enforcement inside `roko run` instead of `ModelCallService` | Budget logic lives once, in the service |
| #5 Hardcoded role behavior | If/elsing on role inside the service to choose providers | Routing decisions live in `CascadeRouter` (plan 08) |
| #6 Feedback afterthought | New caller forgetting to thread `with_feedback_sink` | `ServiceFactory::build` is the only constructor; it always attaches the sink |
| #7 Copy-paste between runtimes | Duplicating `parse_stream_line` for the new streaming path | Adapter calls the existing parser via `roko_agent::provider::claude_cli::stream::parse_stream_line` |
| #8 Prefixing unused params with `_` | `_caller: ModelCallCaller` because nothing reads it | The caller field MUST be propagated to the routing context and feedback record |

---

## Things NOT To Do

1. **Don't add a new `InferenceGateway` struct.** The audit imagined one; reality is `ModelCallService` + `ServiceFactory`. Don't make a third name.
2. **Don't create a per-adapter cache.** Cache lives on `ModelCallService` only, keyed by `(model, system, messages, cache_policy)`. Adapter caches diverge.
3. **Don't make `ModelCaller::stream` mandatory.** The default (post-hoc chunk) is the correct fallback for non-streaming providers (e.g. `OpenAiAgent`).
4. **Don't migrate `OpenAiAgent` to `ModelCaller` if it's deprecated.** First verify it still has callers; it's separate from `OpenAiCompatAdapter`. If dead, delete instead.
5. **Don't read API keys outside `ModelCallService::resolve`.** All `std::env::var("..._API_KEY")` calls outside this path are anti-pattern #1. The grep gate (below) enforces this.
6. **Don't break the chat session UX while migrating.** `chat_session.rs` is the most-used path. Add a feature flag if needed; ship the streaming change behind it until verified.
7. **Don't forget `cache_policy`**. Judge calls and distillation calls should be `Bypass` or `ForceRefresh` — never use cached judgements.

---

## Tests / Proof Criteria

```bash
# 1. No bare claude spawns outside the adapter
rg 'Command::new\("claude"\)' crates/ --type rust | grep -v 'provider/claude_cli' | grep -v test
# expected: 0 results

# 2. No direct API-key env reads outside the gateway
rg 'std::env::var\("(ANTHROPIC|OPENAI|PERPLEXITY|ZAI|MOONSHOT|GEMINI)_API_KEY' crates/ --type rust | grep -v 'roko-agent/src/(provider|secret)' | grep -v test
# expected: 0 results

# 3. extract_clean_text is gone (or feature-gated)
rg 'fn extract_clean_text' crates/ --type rust | grep -v '#\[cfg(feature ='
# expected: 0 results

# 4. dispatch_direct dispatch functions removed from live binary
rg 'dispatch_claude_cli|dispatch_anthropic_api|dispatch_openai_compat' crates/ --type rust | grep -v test | grep -v '#\[cfg(feature ='
# expected: 0 results

# 5. Default build compiles without legacy
cargo build --bin roko --no-default-features
# expected: success
```

Functional proofs:

- [ ] `roko run "add a comment to README"` produces a model call episode with `caller: "cli"` in `.roko/episodes.jsonl`
- [ ] `roko plan run plans/sample` (with `use_workflow_engine = true`) records one episode per task
- [ ] `roko prd refine prds/sample.md` records an episode with `caller: "research"`
- [ ] `roko knowledge dream run` records an episode with `caller: "dreams"`
- [ ] Gate judge invocation records an episode with `caller: "gate.judge"` and `cache_policy: bypass`
- [ ] Streaming proof: in `roko` interactive chat, first `ContentDelta` arrives within 500ms for a Claude CLI provider on a long response (verify visually in `chat_inline`)

---

## Dependencies

This plan blocks but does not depend on any other. It can start immediately. Plan 02 (PromptAssembly) and 03 (FeedbackService) consume the `caller`/`run_id`/`prompt_section_ids` fields and benefit from being done in parallel once those fields are stable.

---

## Estimated Effort

**XL.** ~2-3 weeks for one experienced engineer or 1-2 weeks for a small team. Sub-tasks:

- Step 1 (true streaming) — L (3-4 days; biggest unknown is provider SSE quirks)
- Step 2 (`roko run` migration) — M (2 days)
- Step 3 (event_loop migration behind flag) — L (3-4 days; lots of co-located behavior)
- Step 4 (`agent_exec`) — S (1 day)
- Step 5 (judge oracle) — S (1 day)
- Step 6 (distill/dreams/web-search) — M (2-3 days, three independent migrations)
- Step 7 (delete `extract_clean_text`) — S (1 day, mostly grep + replace)
- Step 8 (verify gating) — S (a few hours)
- Step 9 (caller surfaces test) — S (a few hours)
