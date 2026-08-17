# Runner 01 — ModelCallService Completion

> **Give this entire file to a fresh agent.** It contains everything needed to execute Plan 01.

---

## Context

You are working on the `roko` codebase at `/Users/will/dev/nunchi/roko/roko`. It is a Rust workspace with ~30 crates. The goal of this task is to make **every LLM call in the binary** go through one unified service (`ModelCallService`), instead of the current state where ~4 of 7 entry points bypass it.

**You MUST read these files first (in order) before making any changes:**

1. `tmp/workflow/ANTI-PATTERNS.md` — development anti-patterns to avoid
2. `tmp/workflow/implementation-plans/01-modelcallservice-completion.md` — the full plan with status, steps, code sketches, and proof criteria
3. `crates/roko-core/src/foundation.rs` — the `ModelCaller` trait, `ModelCallRequest`, `ModelCallResponse`, `ModelStreamEvent` types
4. `crates/roko-agent/src/model_call_service.rs` — the existing `ModelCallService` implementation
5. `crates/roko-agent/src/provider/mod.rs` — `ProviderAdapter` trait and adapter registry
6. `crates/roko-agent/src/provider/claude_cli/stream.rs` — canonical stream-json parser (`parse_stream_line`)
7. `crates/roko-orchestrator/src/service_factory.rs` — `ServiceFactory::build()` (how services are wired)

**Also skim these for current bypass locations:**

8. `crates/roko-cli/src/run.rs` — search for `spawn_agent_scoped` and `TODO(gateway)`
9. `crates/roko-cli/src/agent_exec.rs` — search for `spawn_agent_scoped` and `create_agent_for_model`
10. `crates/roko-neuro/src/episode_completion.rs` — search for `ANTHROPIC_API_KEY`
11. `crates/roko-dreams/src/runner.rs` — search for `create_agent_for_model`
12. `crates/roko-std/src/tool/builtin/web_search.rs` — search for `PERPLEXITY_API_KEY`

---

## Work Items (Execute In Order)

**Use `yarn` not `npm`. Use `cargo check` frequently. Run `cargo clippy` after each major step.**

### Step 1: True Streaming (01-A)

**Goal:** Override `ModelCallService::stream()` so it does real token-by-token streaming instead of waiting for `call()` to finish.

1. Open `crates/roko-agent/src/provider/mod.rs`. Add a `StreamingProviderAdapter` trait:

```rust
#[async_trait]
pub trait StreamingProviderAdapter: ProviderAdapter {
    async fn stream(&self, req: AdapterRequest, ctx: AdapterCtx) -> ProviderResult<BoxAdapterStream>;
}
```

Also add `AdapterStreamChunk` enum with variants: `Started { model }`, `ContentDelta(String)`, `ToolCallDelta { id, name, args_json }`, `Usage(TokenUsage)`, `Done { stop_reason }`, `Error(String)`.

2. Implement for `ClaudeCliAdapter` in `crates/roko-agent/src/provider/claude_cli/mod.rs`:
   - Spawn subprocess with `--output-format stream-json`
   - Read stdout line by line
   - Use existing `parse_stream_line` from `stream.rs` to parse each line
   - Map `ClaudeStreamEvent` → `AdapterStreamChunk`

3. Implement for `OpenAiCompatAdapter` in `crates/roko-agent/src/openai_compat_backend.rs`:
   - Use existing SSE delta logic
   - Map to `AdapterStreamChunk`

4. Stub implementations for `AnthropicApiAdapter` and `GeminiAdapter` that fall back to `call()` + synthetic stream.

5. Override `stream()` in `crates/roko-agent/src/model_call_service.rs`:
   - Check if adapter implements `StreamingProviderAdapter`
   - If yes: call `adapter.stream()`, bridge via helper that maps `AdapterStreamChunk` → `ModelStreamEvent`
   - If no: use default trait fallback (call + chunk)
   - The bridge helper MUST: accumulate content, track usage, call `FeedbackService::record()` once on `Done/Error`

6. Verify: `cargo check --workspace`

### Step 2: Migrate `roko run` (01-B)

**Goal:** `crates/roko-cli/src/run.rs` uses `ModelCallService` instead of `spawn_agent_scoped`.

1. Search `run.rs` for `spawn_agent_scoped`. Note every call site.
2. Replace each with:
   ```rust
   let service = ServiceFactory::from_state(state).model_call_service()?;
   let response = service.call(ModelCallRequest {
       model: resolved_model,
       system: Some(assembled_prompt),
       messages: vec![ChatMessage { role: MessageRole::User, content: user_prompt }],
       role: Some(role.to_string()),
       caller: Some(caller::CLI.to_string()),
       run_id: Some(run_id.clone()),
       ..Default::default()
   }).await?;
   ```
3. Preserve the `RunReport` return contract.
4. Remove the `TODO(gateway)` comment.
5. Verify: `cargo check -p roko-cli`

### Step 3: Migrate `roko plan run` behind flag (01-C)

**Goal:** Add `[runner].use_workflow_engine` config flag; when true, plan runner dispatches via `ModelCallService`.

1. Add field to config schema in `crates/roko-core/src/config/schema.rs`
2. In `crates/roko-cli/src/runner/event_loop.rs`, before each `Dispatcher::spawn_agent` call, check the flag
3. If true: build `ModelCallRequest` and call `ModelCallService`
4. Keep the default as `false` (opt-in)
5. Verify: existing integration tests still pass with flag=false

### Step 4: Migrate `agent_exec.rs` (01-D)

**Goal:** `run_agent`, `run_agent_capture`, `run_agent_logged` use `ModelCallService`.

1. Replace `spawn_agent_scoped` / `create_agent_for_model` with `ServiceFactory.model_call_service().call(req)`
2. Delete `persist_capture_episode` if `FeedbackService` now handles it
3. Lift `role_allows_dangerous_skip_permissions` to `crates/roko-agent/src/safety/utils.rs`
4. Verify: `rg 'spawn_agent_scoped|create_agent_for_model' crates/roko-cli/src/agent_exec.rs` returns 0

### Step 5: Migrate LLM Judge (01-E)

**Goal:** Gate LLM judge uses `ModelCallService` instead of hardcoded `Command::new("claude")`.

1. Create `crates/roko-gate/src/llm_judge_oracle.rs` with `LlmJudgeOracle { service: Arc<dyn ModelCaller> }`
2. Implement `JudgeOracle` trait for `LlmJudgeOracle` (call `service.call(req)` with `caller: "gate.judge"`, `cache_policy: Bypass`)
3. Wire into `crates/roko-gate/src/gate_service.rs` via `GateRunContext`
4. Wire `ServiceFactory` to inject `ModelCaller` into `LlmJudgeOracle`
5. Verify: `rg 'AgentJudgeOracle' crates/ --type rust | grep -v '#\[cfg(feature'` returns 0

### Step 6: Migrate distillation/dreams/web-search (01-F)

For each of these three files, the pattern is identical:
1. Add `service: Arc<dyn ModelCaller>` field
2. Replace direct API call / `create_agent_for_model` with `service.call(req)`
3. Delete direct `std::env::var("*_API_KEY")` reads
4. Set appropriate `caller` string

**Files:**
- `crates/roko-neuro/src/episode_completion.rs` → `caller: "neuro.distillation"`, `cache_policy: ForceRefresh`
- `crates/roko-dreams/src/runner.rs` → `caller: "dreams"`
- `crates/roko-std/src/tool/builtin/web_search.rs` → `caller: "tool.web_search"`

### Step 7: Cleanup (01-G)

1. Feature-gate or delete `extract_clean_text` in `crates/roko-cli/src/chat.rs`
2. Verify `dispatch_direct.rs` unreachable: `cargo build --bin roko --no-default-features`
3. Run all verification commands from the plan's "Tests / Proof Criteria" section
4. Add caller surface coverage test

---

## Verification Checklist (Run After ALL Steps Complete)

```bash
# No bare claude spawns outside the adapter
rg 'Command::new\("claude"\)' crates/ --type rust | grep -v 'provider/claude_cli' | grep -v test
# MUST return 0 results

# No direct API-key env reads outside the gateway
rg 'std::env::var.*API_KEY' crates/ --type rust | grep -v 'roko-agent/src/(provider|secret)' | grep -v test
# MUST return 0 results

# extract_clean_text gone or gated
rg 'fn extract_clean_text' crates/ --type rust | grep -v '#\[cfg(feature ='
# MUST return 0 results

# dispatch_direct removed from live binary
rg 'dispatch_claude_cli|dispatch_anthropic_api|dispatch_openai_compat' crates/ --type rust | grep -v test | grep -v '#\[cfg(feature ='
# MUST return 0 results

# Default build works
cargo build --bin roko --no-default-features
# MUST succeed

# All tests pass
cargo test --workspace
# MUST succeed
```

---

## Critical Rules

1. **NEVER add `Command::new("claude")` anywhere.** All subprocess spawns live inside `crates/roko-agent/src/provider/claude_cli/`.
2. **NEVER read API keys with `std::env::var`.** All credential resolution lives inside `ModelCallService::resolve`.
3. **NEVER add inline prompt strings.** Use `roko-compose` templates or `PromptAssemblyService`.
4. **ALWAYS set `caller` on every `ModelCallRequest`.** Use constants from `roko_core::foundation::caller`.
5. **ALWAYS set `cache_policy: Bypass` for judge and distillation calls.**
6. **Use `ServiceFactory::build()` to construct all services.** Never construct `ModelCallService::new()` manually in production code.
7. **Use `yarn` not `npm` for any JS work.**
