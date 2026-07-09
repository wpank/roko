# C -- Tool Loop + Format Translation (Docs 07, 09)

Parity analysis of `docs/02-agents/07-tool-loop.md`, `09-format-translation.md` vs actual codebase.

---

## C.01 -- ToolLoop struct and construction (Doc 07)

- **Status**: DONE
- **Priority**: P0
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
The `ToolLoop` struct holds three `Arc`-wrapped dependencies: `translator`, `dispatcher`, `backend`, plus `max_iterations` (default 25) and `context_token_limit`. Constructed via `ToolLoop::new(translator, dispatcher, backend)`.

### What exists
The `ToolLoop` struct at `crates/roko-agent/src/tool_loop/mod.rs:159` matches and exceeds the spec:

```rust
pub struct ToolLoop {
    translator: Arc<dyn Translator>,
    dispatcher: Arc<ToolDispatcher>,
    backend: Arc<dyn LlmBackend>,
    max_iterations: usize,         // Default: 25 (mod.rs:183)
    context_token_limit: usize,    // Default: 102_400 (prune.rs:9)
    checkpoint_path: Option<PathBuf>,
    model_profile: Option<ModelProfile>,
    retry_policy: RetryPolicy,
    monitor: Option<MetacognitiveMonitor>,
}
```

Additional fields beyond doc: `checkpoint_path`, `model_profile`, `retry_policy`, `monitor`. All are optional builder-pattern additions. The file is 1686 lines (doc claimed ~769 but that was written earlier).

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| -- | None | -- | -- |

### Verify
```bash
grep -n "pub struct ToolLoop" crates/roko-agent/src/tool_loop/mod.rs
```

---

## C.02 -- Stop conditions (Doc 07)

- **Status**: DONE
- **Priority**: P0
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
Four stop conditions: Stop (no tool calls), MaxIterations, Cancelled, BackendError.

### What exists
`StopReason` enum at `crates/roko-agent/src/tool_loop/mod.rs:116`:

```rust
pub enum StopReason {
    Stop,
    MaxIterations,
    Cancelled,
    BackendError(String),
}
```

Exact match with the doc. All four variants are exercised in the run loop and covered by tests.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| -- | None | -- | -- |

### Verify
```bash
grep -n "pub enum StopReason" crates/roko-agent/src/tool_loop/mod.rs
```

---

## C.03 -- Iteration cap (max_iter submodule) (Doc 07)

- **Status**: DONE
- **Priority**: P0
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
`max_iter` submodule with `DEFAULT_MAX_ITERATIONS = 25` and `is_exhausted(iterations, max)`.

### What exists
`crates/roko-agent/src/tool_loop/max_iter.rs` (39 lines):
- `DEFAULT_MAX_ITERATIONS: usize = 25` at line 7
- `pub const fn is_exhausted(iterations: usize, max: usize) -> bool` at line 12
- 4 unit tests covering zero, at-limit, past-limit, zero-limit cases.

Exact match.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| -- | None | -- | -- |

### Verify
```bash
grep -n "DEFAULT_MAX_ITERATIONS" crates/roko-agent/src/tool_loop/max_iter.rs
```

---

## C.04 -- Context pruning (prune submodule) (Doc 07)

- **Status**: DONE
- **Priority**: P0
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
`prune` submodule implements context-growth guards. `prune_if_needed(messages, token_limit)` keeps system + first user message, preserves most recent, drops oldest tool results.

### What exists
`crates/roko-agent/src/tool_loop/prune.rs` (108 lines):
- `DEFAULT_CONTEXT_TOKEN_LIMIT: usize = 102_400` at line 9
- `HEAD_KEEP: usize = 2` at line 12 (system + user)
- `TAIL_KEEP: usize = 3` at line 15 (most recent context)
- `estimate_message_tokens()` at line 21 (bytes/4 heuristic)
- `pub fn prune_if_needed(messages: &mut Vec<Value>, token_limit: usize)` at line 38
- 4 unit tests covering under-limit, minimum-size, oldest-pruning, head-tail preservation.

Exact match with doc's description.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| -- | None | -- | -- |

### Verify
```bash
grep -n "pub fn prune_if_needed" crates/roko-agent/src/tool_loop/prune.rs
```

---

## C.05 -- Compaction (compaction submodule) (Doc 07)

- **Status**: DONE
- **Priority**: P1
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
Doc mentions the `compaction` submodule at line 38 as "gentle tool-result truncation".

### What exists
`crates/roko-agent/src/tool_loop/compaction.rs` (164 lines):
- `RECENT_TOOL_GROUPS_TO_KEEP: usize = 2` at line 9
- `TOOL_RESULT_COMPACTION_THRESHOLD_CHARS: usize = 500` at line 10
- `TOOL_RESULT_PREVIEW_CHARS: usize = 200` at line 11
- `pub fn compact_tool_results(messages: &mut Vec<Value>)` at line 19
- Groups tool messages by contiguous `role="tool"` runs. Keeps newest 2 groups verbatim. Older groups: content > 500 chars is replaced with 200-char preview + char count.
- 3 unit tests covering truncation of old groups, preservation of recent groups, and Unicode char boundaries.

Beyond what the doc describes -- the doc only names the submodule, but the implementation is complete and tested.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| -- | None | -- | -- |

### Verify
```bash
grep -n "pub fn compact_tool_results" crates/roko-agent/src/tool_loop/compaction.rs
```

---

## C.06 -- Tool-result message construction (result_msg submodule) (Doc 07)

- **Status**: DONE
- **Priority**: P0
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
`result_msg` submodule constructs tool-result messages and appends them to the conversation.

### What exists
`crates/roko-agent/src/tool_loop/result_msg.rs` (110 lines):
- `pub fn initial_messages(system: &str, user: &str) -> Vec<Value>` at line 13
- `pub fn append_results(messages: &mut Vec<Value>, rendered: RenderedResults)` at line 31
- Handles all three `RenderedResults` variants: `JsonMessages` (each element becomes a message), `TextBlock` (wrapped as user-role message), `HandledByBackend` (no-op).
- 4 unit tests.

Exact match.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| -- | None | -- | -- |

### Verify
```bash
grep -n "pub fn append_results" crates/roko-agent/src/tool_loop/result_msg.rs
```

---

## C.07 -- Checkpoint and resume (Doc 07)

- **Status**: DONE
- **Priority**: P0
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
`Checkpoint` struct with `iterations`, `tool_calls`, `messages`. Serializable. Loop produces checkpoint on non-Stop stops. `tool_loop.resume(checkpoint, tools, ctx)` continues from where it left off.

### What exists
`crates/roko-agent/src/tool_loop/checkpoint.rs` (140 lines):
- `Checkpoint { iterations: usize, tool_calls: Vec<ToolCall>, messages: Vec<Value> }` at line 18
- `Checkpoint::new()`, `to_bytes()`, `from_bytes()`, `save(path)`, `load(path)` -- all implemented
- Serde `Serialize` + `Deserialize` derived
- `ToolLoop::resume()` method exists at `tool_loop/mod.rs` -- resumes from checkpoint state
- `ToolLoop::run()` auto-loads checkpoint from `checkpoint_path` if the file exists (mod.rs:242)
- 3 unit tests: round-trip serde, empty checkpoint, save+load file round-trip.

Matches and exceeds spec. The `save`/`load` methods for file persistence go beyond what the doc describes.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| -- | None | -- | -- |

### Verify
```bash
grep -n "pub struct Checkpoint" crates/roko-agent/src/tool_loop/checkpoint.rs
grep -n "pub async fn resume" crates/roko-agent/src/tool_loop/mod.rs
```

---

## C.08 -- LlmBackend trait (Doc 07)

- **Status**: DONE
- **Priority**: P0
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
```rust
#[async_trait]
pub trait LlmBackend: Send + Sync {
    async fn send_turn(
        &self,
        messages: &[serde_json::Value],
        tools: &RenderedTools,
    ) -> Result<BackendResponse, LlmError>;
}
```

With `LlmError { Backend(String), Network(String) }`.

### What exists
`crates/roko-agent/src/tool_loop/mod.rs:61`:

```rust
#[async_trait]
pub trait LlmBackend: Send + Sync {
    async fn send_turn(
        &self,
        messages: &[serde_json::Value],
        tools: &RenderedTools,
        session: &SessionState,     // extra param vs doc
    ) -> Result<BackendResponse, LlmError>;

    fn extract_session(&self, response: &BackendResponse) -> SessionState { ... }

    async fn send_turn_streaming(
        &self,
        messages: &[serde_json::Value],
        tools: &RenderedTools,
        session: &SessionState,
        event_tx: mpsc::UnboundedSender<StreamChunk>,
    ) -> Result<BackendResponse, LlmError> { ... }
}
```

`LlmError` at line 97:
```rust
pub enum LlmError {
    Backend(String),
    Network(String),
    Provider(ProviderError),    // extra vs doc
    RetriesExhausted,           // extra vs doc
}
```

The trait and error type have been extended beyond the doc: `session` parameter added, `extract_session` default method, `send_turn_streaming` default method, two extra error variants. All are additive, not breaking.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| C.08a | Doc claims 2 LlmError variants; code has 4 | `tool_loop/mod.rs:97-110` | Low (doc stale, code is richer) |

### Verify
```bash
grep -n "pub trait LlmBackend" crates/roko-agent/src/tool_loop/mod.rs
grep -n "pub enum LlmError" crates/roko-agent/src/tool_loop/mod.rs
```

---

## C.09 -- LlmBackend: OllamaLlmBackend (Doc 07)

- **Status**: DONE
- **Priority**: P0
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
`OllamaLlmBackend` at `crates/roko-agent/src/ollama_backend.rs` proves the `LlmBackend` pattern works. Implements the trait for Ollama's HTTP API.

### What exists
`crates/roko-agent/src/ollama_backend.rs` (340 lines):
- `pub struct OllamaLlmBackend` at line 17
- `impl LlmBackend for OllamaLlmBackend` at line 92
- Builder: `new(model)`, `with_base_url()`, `with_timeout_ms()`, `with_poster()`, `with_response_cache()`, `without_response_cache()`
- Forces `"stream": false` in request body (enforcing M21)
- Content-addressed response caching via `ResponseCache`
- 7 unit tests.

Fully implemented and tested.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| -- | None | -- | -- |

### Verify
```bash
grep -n "impl LlmBackend for OllamaLlmBackend" crates/roko-agent/src/ollama_backend.rs
```

---

## C.10 -- LlmBackend: OpenAiCompatBackend (Doc 07)

- **Status**: DONE
- **Priority**: P0
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
The doc proposes an `OpenAiCompatBackend` as the missing piece that would wrap HTTP-based providers. The doc describes it as unimplemented.

### What exists
`crates/roko-agent/src/openai_compat_backend.rs` (973 lines):
- `pub struct OpenAiCompatLlmBackend` at line 33
- `impl LlmBackend for OpenAiCompatLlmBackend` at line 222
- Full `send_turn()` implementation: builds JSON body, rate-limits, posts, returns `BackendResponse::Json`
- Full `send_turn_streaming()` implementation: SSE streaming, `StreamAccumulator`, chunk-by-chunk delivery
- `extract_session()`: extracts `session_id`, `thread_id`, `conversation_id` from response
- Builder methods: `with_base_url`, `with_timeout_ms`, `with_max_tokens`, `with_extra_headers`, `with_extra_body_params`, `with_rate_limiter`, `with_poster`
- Re-exported as `OpenAiCompatBackend` at `tool_loop/backends/openai_compat.rs:11`
- 7 unit tests including a full tool-loop integration test.

The doc's proposed gap has been filled. This is a fully functional backend.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| -- | None (doc gap is closed) | -- | -- |

### Verify
```bash
grep -n "impl LlmBackend for OpenAiCompatLlmBackend" crates/roko-agent/src/openai_compat_backend.rs
```

---

## C.11 -- LlmBackend: HedgedBackend (Doc 07)

- **Status**: DONE
- **Priority**: P1
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
Not explicitly described in doc 07. Implied by the model-routing documentation.

### What exists
`crates/roko-agent/src/tool_loop/backends/hedged.rs` (174 lines):
- `pub struct HedgedBackend { primary, backup, hedge_after_ms }` at line 12
- `impl LlmBackend for HedgedBackend` at line 34
- Sends to primary first. If primary exceeds `hedge_after_ms`, fires backup in parallel. Returns whichever responds first.
- Uses `tokio::select! { biased; }` for deterministic primary preference.
- 2 unit tests: primary-fast (no backup fired), primary-slow (backup wins).

Fully functional tail-latency hedging.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| C.11a | Doc 07 does not document the HedgedBackend | `tool_loop/backends/hedged.rs` | Low (doc gap only) |

### Verify
```bash
grep -n "pub struct HedgedBackend" crates/roko-agent/src/tool_loop/backends/hedged.rs
```

---

## C.12 -- LlmBackend: GeminiNativeBackend (Doc 07)

- **Status**: DONE
- **Priority**: P1
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
Not explicitly described in doc 07. The backend factory at `tool_loop/backends/mod.rs:88` routes `GeminiApi` + `gemini_native` tool format to this backend.

### What exists
`crates/roko-agent/src/tool_loop/backends/gemini_native.rs` (358 lines):
- `pub struct GeminiNativeBackend` at line 27
- `impl LlmBackend for GeminiNativeBackend` (via the file)
- Translates standard messages to Gemini's `Content`/`Part` format
- Posts to `generateContent` endpoint
- Handles thinking levels, cached content, system instructions
- Factory function at `backends/mod.rs:88-98` wires it in.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| C.12a | Doc 07 does not document the GeminiNativeBackend | `backends/gemini_native.rs` | Low (doc gap only) |

### Verify
```bash
grep -n "pub struct GeminiNativeBackend" crates/roko-agent/src/tool_loop/backends/gemini_native.rs
```

---

## C.13 -- Backend factory (create_tool_loop_backend) (Doc 07)

- **Status**: PARTIAL
- **Priority**: P1
- **Estimated LOC**: ~60
- **Dependencies**: None
- **Files to modify**: `crates/roko-agent/src/tool_loop/backends/mod.rs`

### What the doc says
The doc proposes wiring HTTP backends through `create_agent_for_model -> provider adapter -> LlmBackend -> ToolLoop -> ToolDispatcher -> SafetyLayer`.

### What exists
`crates/roko-agent/src/tool_loop/backends/mod.rs:80`:
```rust
pub fn create_tool_loop_backend(
    provider: &ProviderConfig,
    model: &ModelProfile,
    options: &AgentOptions,
    poster: Arc<dyn HttpPoster>,
) -> Result<Arc<dyn LlmBackend>, AgentCreationError>
```

Routes:
- `OpenAiCompat` -> `OpenAiCompatBackend` (DONE)
- `GeminiApi` + `gemini_native` -> `GeminiNativeBackend` (DONE)
- `AnthropicApi` -> Error: "not implemented yet" (line 102)
- `ClaudeCli | CursorAcp` -> Error: "don't use LlmBackend" (line 105)
- `PerplexityApi` -> Error: "not implemented yet" (line 110)

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| C.13a | Anthropic HTTP API backend not implemented | `backends/mod.rs:102` | Medium -- blocks Anthropic API tool loop |
| C.13b | Perplexity backend not implemented | `backends/mod.rs:110` | Low -- Perplexity is research-only |
| C.13c | GeminiApi without `gemini_native` format errors | `backends/mod.rs:99-101` | Low -- fallback path |

### Verify
```bash
grep -n "not implemented yet" crates/roko-agent/src/tool_loop/backends/mod.rs
```

---

## C.14 -- ToolDispatcher struct and 7-step pipeline (Doc 07)

- **Status**: DONE
- **Priority**: P0
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
Doc claims an 8-step pipeline:
1. VALIDATE args against JSON schema
2. RESOLVE ToolDef for canonical name
3. FILTER task-level allowed/denied tool lists
4. AUTHORIZE def.permission.satisfied_by(&role_perms)
5. SAFETY SafetyLayer pre-execution checks
6. EXECUTE handler.execute() with timeout + cancellation
7. TRUNCATE oversized Ok content
8. SCRUB SafetyLayer post-execution secret scrubbing

### What exists
`crates/roko-agent/src/dispatcher/mod.rs` (1069 lines):

The `dispatch()` method at line 135 implements:
1. **VALIDATE** (line 140): `validate(&call, self.registry.as_ref())`
2. **RESOLVE** (line 156): `self.registry.get(&call.name)`
3. **FILTER** (line 172): `tool_filter_block_reason()` -- checks `ctx.allowed_tools` / `ctx.denied_tools`
4. **AUTHORIZE** (line 198): `def.permission.satisfied_by(&role_perms)` -- builds `ToolPermissions` from `ctx.capabilities`
5. **SAFETY** (line 237): `safety.check_pre_execution(&call, ctx)` -- only if `self.safety.is_some()`
6. **RESOLVE HANDLER** (line 254): `self.resolver.resolve(&call.name)` via `HandlerResolver` trait
7. **EXECUTE** (line 283): `handler.execute()` raced against `with_timeout(timeout, ...)` and `wait_cancelled(ctx.cancel_token)`
8. **TRUNCATE** (line 290): `truncate_result(result, self.max_result_bytes)`
9. **SCRUB** (line 292): `safety.scrub_output(result)` -- only if safety layer attached

Each phase emits audit signals via `emit_audit()` (Engram signals with phase/status tags).

Actual pipeline is 9 steps (resolve handler is a separate step from resolve def). Doc numbered 8 steps. The code matches the doc's intent precisely -- it just has slightly different numbering because handler resolution is explicit.

12 unit tests covering: unknown tool, invalid args, permission denied, allowlist/denylist blocking, timeout, cancellation, successful call, audit signal emission per phase, oversized truncation, UTF-8 boundary truncation, parallel batch, serial batch.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| C.14a | Doc says "7-step" but code has 9 distinct phases | `dispatcher/mod.rs:135-298` | None (doc numbering off, all steps present) |

### Verify
```bash
grep -n "fn dispatch\b" crates/roko-agent/src/dispatcher/mod.rs
```

---

## C.15 -- ToolDispatcher batch dispatch (Doc 07)

- **Status**: DONE
- **Priority**: P0
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
`dispatch_batch` groups calls by concurrency policy. Parallel tools run via `join_all`, serial tools run sequentially.

### What exists
`crates/roko-agent/src/dispatcher/mod.rs:307`:
```rust
pub async fn dispatch_batch(
    &self,
    calls: Vec<ToolCall>,
    ctx: &ToolContext,
) -> Vec<(ToolCall, ToolResult)>
```

Uses `partition_by_concurrency(calls, self.registry.as_ref())` from `dispatcher/parallel.rs`. Parallel bucket: `futures::future::join_all`. Serial bucket: sequential `for` loop.

Tests at lines 980-1068 verify: 3 parallel sleep-100ms calls complete in ~100ms wall time; 3 serial calls take ~180ms with strictly increasing counter observations.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| -- | None | -- | -- |

### Verify
```bash
grep -n "pub async fn dispatch_batch" crates/roko-agent/src/dispatcher/mod.rs
```

---

## C.16 -- SafetyLayer struct and 6 policies (Doc 07)

- **Status**: DONE
- **Priority**: P0
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
```rust
pub struct SafetyLayer {
    pub bash_policy: BashPolicy,
    pub git_policy: GitPolicy,
    pub network_policy: NetworkPolicy,
    pub path_policy: PathPolicy,
    pub scrub_policy: ScrubPolicy,
    pub rate_limiter: Option<Arc<RateLimiter>>,
    pub role: String,
}
```

Six policy families: bash (command allowlist/denylist), git (branch protection), network (outbound allowlist), path (worktree escape prevention), scrub (secret scrubbing), rate_limit (per-tool rate limits).

### What exists
`crates/roko-agent/src/safety/mod.rs:77`:
```rust
pub struct SafetyLayer {
    pub bash_policy: BashPolicy,
    pub git_policy: GitPolicy,
    pub network_policy: NetworkPolicy,
    pub path_policy: PathPolicy,
    pub scrub_policy: ScrubPolicy,
    pub rate_limiter: Option<Arc<RateLimiter>>,
    pub role: String,
    pub warrant: Option<AgentWarrant>,  // extra vs doc
}
```

All 6 doc-specified policy families are implemented in their own submodules:
- `safety/bash.rs` (397 lines) -- command allowlist/denylist with regex patterns
- `safety/git.rs` (719 lines) -- branch protection, force-push blocking
- `safety/network.rs` (464 lines) -- URL destination allowlist, private IP blocking
- `safety/path.rs` (487 lines) -- worktree-relative canonicalization, escape prevention
- `safety/scrub.rs` (472 lines) -- secret/API-key pattern scrubbing from output
- `safety/rate_limit.rs` (508 lines) -- per-role per-tool call rate limits

Additional submodules beyond doc:
- `safety/capabilities.rs` (188 lines) -- OCaps-style `AgentWarrant` / `Capability` system
- `safety/contract.rs` (173 lines) -- contract types

`check_pre_execution()` at line 136 chains policies in order:
1. Rate limit (all tools)
2. OCaps warrant check (if warrant attached)
3. Bash/git policy (bash, run_tests tools)
4. Network policy (web_fetch, web_search tools)
5. Path policy (file tools: read_file, write_file, edit_file, etc.)

`scrub_output()` at line 223 scrubs secrets from `ToolResult::Ok` variants.
`check_exec_command()` at line 199 checks raw subprocess launches.

12 unit tests in safety/mod.rs covering all policy families.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| C.16a | Doc omits `warrant: Option<AgentWarrant>` field and OCaps system | `safety/mod.rs:94`, `safety/capabilities.rs` | None (additive feature) |

### Verify
```bash
grep -n "pub struct SafetyLayer" crates/roko-agent/src/safety/mod.rs
grep -c "pub mod" crates/roko-agent/src/safety/mod.rs
```

---

## C.17 -- ToolDispatcher + SafetyLayer integration into orchestrate.rs (Doc 07)

- **Status**: PARTIAL
- **Priority**: P0
- **Estimated LOC**: ~100
- **Dependencies**: C.13
- **Files to modify**: `crates/roko-cli/src/orchestrate.rs`

### What the doc says
Doc explicitly calls out: "The ToolDispatcher is never called from `orchestrate.rs`." The orchestrator constructs `ClaudeCliAgent` directly, and Claude CLI drives its own internal tool loop. Roko's `ToolDispatcher` + `SafetyLayer` + `ToolLoop` are bypassed for the primary execution path. The doc identifies this as the **#1 integration gap**.

The fix: wire HTTP backends through `create_agent_for_model -> provider adapter -> LlmBackend -> ToolLoop -> ToolDispatcher -> SafetyLayer`.

### What exists
The `ToolLoopAgent` wrapper at `crates/roko-agent/src/tool_loop/agent_wrapper.rs` (359 lines) bridges the `Agent` trait and the `ToolLoop`:
- `pub struct ToolLoopAgent` wraps a `ToolLoop` + `translator` + `tools`
- Implements `Agent::run()` by calling `tool_loop.run()`
- Factory function `create_tool_loop_agent()` at the crate level builds the full stack

The `SafetyLayer` is attachable to `ToolDispatcher` via `.with_safety()` (dispatcher/mod.rs:110).

However, the orchestrator at `crates/roko-cli/src/orchestrate.rs` still uses `ClaudeCliAgent` as the primary path. When HTTP backends are used through the tool loop, the safety layer IS available, but the integration path from orchestrate.rs to ToolLoopAgent for non-CLI backends needs verification.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| C.17a | orchestrate.rs primary path bypasses ToolLoop/SafetyLayer for Claude CLI | `crates/roko-cli/src/orchestrate.rs` | Medium -- by design for CLI, but HTTP backends need safety |
| C.17b | ToolLoopAgent wiring from orchestrate.rs not yet exercised for HTTP models | `crates/roko-cli/src/orchestrate.rs` | Medium -- blocks HTTP-backend safety guarantees |

### Verify
```bash
grep -n "ToolLoop\|ToolDispatcher\|SafetyLayer" crates/roko-cli/src/orchestrate.rs
grep -n "ClaudeCliAgent" crates/roko-cli/src/orchestrate.rs
```

---

## C.18 -- Translator trait (Doc 09)

- **Status**: DONE
- **Priority**: P0
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
```rust
pub trait Translator: Send + Sync {
    fn format(&self) -> ToolFormat;
    fn render_tools(&self, tools: &[ToolDef]) -> RenderedTools;
    fn parse_calls(&self, response: &BackendResponse) -> Result<Vec<ToolCall>, TranslatorError>;
    fn render_results(&self, results: &[(ToolCall, ToolResult)]) -> RenderedResults;
    fn render_assistant_message(&self, response: &BackendResponse) -> Option<Value> { None }
}
```

### What exists
`crates/roko-agent/src/translate/mod.rs:98`:
```rust
pub trait Translator: Send + Sync {
    fn format(&self) -> ToolFormat;
    fn render_tools(&self, tools: &[ToolDef]) -> RenderedTools;
    fn parse_calls(&self, response: &BackendResponse) -> Result<Vec<ToolCall>, TranslatorError>;
    fn render_results(&self, results: &[(ToolCall, ToolResult)]) -> RenderedResults;
    fn render_assistant_message(&self, _response: &BackendResponse) -> Option<serde_json::Value> { None }
}
```

Exact match. All 5 methods present with identical signatures.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| -- | None | -- | -- |

### Verify
```bash
grep -n "pub trait Translator" crates/roko-agent/src/translate/mod.rs
```

---

## C.19 -- Wire format enums (RenderedTools, RenderedResults, BackendResponse) (Doc 09)

- **Status**: DONE
- **Priority**: P0
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
Three enums:
- `RenderedTools { JsonArray, CliFlag, SystemPromptBlock }`
- `RenderedResults { JsonMessages, TextBlock, HandledByBackend }`
- `BackendResponse { Json, StreamJson, Text }`

### What exists
`crates/roko-agent/src/translate/mod.rs`:
- `RenderedTools` at line 131: `JsonArray(Value)`, `CliFlag(String)`, `SystemPromptBlock(String)` -- exact match
- `RenderedResults` at line 144: `JsonMessages(Value)`, `TextBlock(String)`, `HandledByBackend` -- exact match
- `BackendResponse` at line 159: `Json(Value)`, `StreamJson(Vec<Value>)`, `Text(String)` -- exact match

All three have `Debug, Clone` derives. `BackendResponse` also has helper methods: `extract_text()` (line 177), `extract_reasoning()` (line 208), `extract_usage()` (line 230).

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| -- | None | -- | -- |

### Verify
```bash
grep -n "pub enum Rendered\|pub enum BackendResponse" crates/roko-agent/src/translate/mod.rs
```

---

## C.20 -- Translator: OpenAiTranslator (Doc 09)

- **Status**: DONE
- **Priority**: P0
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
Handles OpenAI chat completions tool format. `render_tools` -> `type: function` JSON array. `parse_calls` -> reads `choices[0].message.tool_calls`. Arguments always JSON-stringified. `render_results` -> `role: "tool"` messages.

### What exists
`crates/roko-agent/src/translate/openai.rs` (844 lines):
- `pub struct OpenAiTranslator` at line 34 (zero-sized)
- `format()` -> `ToolFormat::OpenAiJson` (line 48)
- `render_tools()` at line 52: maps `ToolDef` to `{type: "function", function: {name, description, parameters}}`. Also handles `ToolSource::WebSearch` and `ToolSource::Retrieval` variants.
- `parse_calls()` at line 57: reads `choices[0].message.tool_calls`. Parses stringified `arguments`. Preserves backend-issued IDs verbatim (including Kimi's `functions.Read:0` format).
- `render_results()` at line 95: produces `{role: "tool", tool_call_id, content}` messages.
- `render_assistant_message()` at line 113: returns `choices[0].message`.
- Additional helpers: `parse_usage()` (line 151), `parse_glm_response()` (line 179), `parse_glm_metadata()` (line 187), `build_partial_continuation()` (line 39).
- 23 unit tests covering: format, render, parse single/multiple/no calls, error cases, round-trip, Kimi IDs, GLM reasoning, cached tokens.

Fully implemented and exceeds doc spec.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| -- | None | -- | -- |

### Verify
```bash
grep -n "impl Translator for OpenAiTranslator" crates/roko-agent/src/translate/openai.rs
```

---

## C.21 -- Translator: OllamaTranslator (Doc 09)

- **Status**: DONE
- **Priority**: P0
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
Similar to OpenAI but handles Ollama's slightly different JSON structure (messages under `message` instead of `choices[0].message`).

### What exists
`crates/roko-agent/src/translate/ollama.rs` (562 lines):
- `pub struct OllamaTranslator` at line 52 (zero-sized)
- `format()` -> `ToolFormat::OpenAiJson` (line 57) -- same format as OpenAI (both use OpenAI-compatible wire shape)
- `render_tools()` at line 60: identical structure to OpenAI's `type: function` array
- `parse_calls()` at line 77: reads `message.tool_calls` (NOT `choices[0].message.tool_calls`). Handles both stringified and inline-object arguments.
- `render_results()` at line 126: same `role: "tool"` messages as OpenAI
- `render_assistant_message()` at line 119: returns `message` directly (not `choices[0].message`)
- 15 unit tests.

The key differentiator from OpenAI is the JSON pointer path for parsing: `/message/tool_calls` vs `/choices/0/message/tool_calls`.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| -- | None | -- | -- |

### Verify
```bash
grep -n "impl Translator for OllamaTranslator" crates/roko-agent/src/translate/ollama.rs
```

---

## C.22 -- Translator: ClaudeTranslator (Doc 09)

- **Status**: DONE
- **Priority**: P0
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
Handles Claude CLI's stream-JSON protocol. `render_tools` -> `RenderedTools::CliFlag("Read,Edit,Bash,...")`. `parse_calls` -> parses `tool_use` blocks from stream-json events. `render_results` -> `RenderedResults::HandledByBackend`.

### What exists
`crates/roko-agent/src/translate/claude.rs` (517 lines):
- `pub struct ClaudeTranslator` at line 31 (zero-sized)
- `format()` -> `ToolFormat::AnthropicBlocks` (line 35)
- `render_tools()` at line 39: maps canonical snake_case names to Claude PascalCase via `claude_of_canonical()`, joins as CSV -> `RenderedTools::CliFlag(csv)`
- `parse_calls()` at line 48: filters `content_block_start` events with `content_block.type == "tool_use"`, maps Claude PascalCase names back via `canonical_of_claude()`, extracts `id`, `name`, `input`
- `render_results()` at line 93: returns `RenderedResults::HandledByBackend` unconditionally
- Additional: `inject_cache_markers()` / `inject_cache_markers_into_content()` functions for Anthropic prompt caching with `<!-- cache:system -->` / `<!-- cache:session -->` markers
- 14 unit tests.

Matches doc exactly plus adds cache marker support.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| -- | None | -- | -- |

### Verify
```bash
grep -n "impl Translator for ClaudeTranslator" crates/roko-agent/src/translate/claude.rs
```

---

## C.23 -- Translator: ReActTranslator (Doc 09)

- **Status**: DONE
- **Priority**: P0
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
Fallback for models without native function calling. `render_tools` -> `RenderedTools::SystemPromptBlock(...)` with `### tool_name` headings and `Action:` / `Action Input:` / `Observation:` / `Final Answer:` markers. `parse_calls` -> regex-extracts `Action:` and `Action Input:` from text. `render_results` -> `RenderedResults::TextBlock("Observation: ...")`.

### What exists
`crates/roko-agent/src/translate/react.rs` (422 lines):
- `pub struct ReActTranslator` at line 26 (zero-sized)
- `format()` -> `ToolFormat::ReActText` (line 29)
- `render_tools()` at line 33: builds system prompt block with `### tool_name`, description, fenced JSON schema, and instruction text for `Action:` / `Action Input:` / `Observation:` / `Final Answer:` markers
- `parse_calls()` at line 57: uses `rfind("Action:")` (last occurrence, not first -- avoids quoted mentions in reasoning), extracts tool name up to newline, then looks for `Action Input:` with JSON args up to `\n\n`
- `render_results()` at line 93: produces `"Observation: content\n\n"` text blocks
- 12 unit tests covering: format, render, parse single/final-answer/malformed/multiple-action/blank-line/no-input cases, round-trip.

Exact match with doc.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| -- | None | -- | -- |

### Verify
```bash
grep -n "impl Translator for ReActTranslator" crates/roko-agent/src/translate/react.rs
```

---

## C.24 -- Translator: GeminiTranslator (Doc 09)

- **Status**: DONE
- **Priority**: P1
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
Doc 09 lists four translators (Claude, Ollama, OpenAI, ReAct). Gemini is not mentioned in doc 09 -- it was added after doc 09 was written.

### What exists
`crates/roko-agent/src/translate/gemini.rs` (416 lines):
- `pub struct GeminiTranslator` at line 16 (zero-sized)
- `format()` -> `ToolFormat::Custom("gemini_native")` (line 21)
- `render_tools()` at line 24: wraps tools as `functionDeclarations` inside a JSON array
- `parse_calls()` at line 39: reads `candidates[0].content.parts` looking for `functionCall` entries. Also handles SDK-style `functionCalls` array at top level. Generates sequential IDs if backend omits them.
- `render_results()` at line 80: produces `{role: "user", parts: [{functionResponse: {name, response, id}}]}` messages
- `render_assistant_message()` at line 99: returns `candidates[0].content`
- 9 unit tests.

A 5th translator beyond what doc 09 describes. Fully functional.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| C.24a | Doc 09 says "four translator implementations" but code has five (+ Gemini) | `translate/gemini.rs` | Low (doc stale) |

### Verify
```bash
grep -n "impl Translator for GeminiTranslator" crates/roko-agent/src/translate/gemini.rs
```

---

## C.25 -- Capability detection and translator selection (Doc 09)

- **Status**: DONE
- **Priority**: P0
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
`ModelCapabilities` struct with `supports_tools`, `supports_thinking`, `supports_vision`, `tool_format`, `max_tools`. `translator_for(capabilities)` selects translator based on `tool_format`. `capabilities_from_profile(profile)` derives capabilities from `ModelProfile`.

### What exists
`crates/roko-agent/src/translate/capability.rs` (420 lines):
- `ModelCapabilities` at line 35:
  ```rust
  pub struct ModelCapabilities {
      pub supports_tools: bool,
      pub supports_parallel_tool_calls: bool,
      pub tool_format: ToolFormat,
      pub max_tools_before_degrade: u8,
      pub supports_thinking: bool,
      pub supports_vision: bool,
      pub supports_web_search: bool,
      pub supports_mcp_tools: bool,
      pub supports_partial: bool,
      pub supports_tool_streaming: bool,
  }
  ```
  10 fields vs doc's 5 -- all additive.

- `capabilities_for(slug: &str)` at line 63: hardcoded overrides for `glm-5.*` and `kimi-k2*`, fallback to `profile_for_model(slug)`
- `capabilities_from_profile(profile: &ModelProfile)` at line 111: maps full `ModelProfile` to `ModelCapabilities`
- `translator_for(slug: &str)` at line 153: `AnthropicBlocks -> ClaudeTranslator`, `OpenAiJson -> OllamaTranslator`, everything else -> `ReActTranslator`
- `translator_name_for(slug: &str)` at line 176: returns `"claude"`, `"openai"`, or `"react"`
- `tool_format_from_str()` at line 132: maps string tool format to `ToolFormat` enum (9 known formats + `Custom`)
- 14 unit tests.

Note: `translator_for` uses slug-based lookup, not capabilities struct. Doc describes `translator_for(capabilities)` but code uses `translator_for(slug)` internally calling `capabilities_for(slug)` first. Same result.

Note: `translator_for` routes `OpenAiJson` to `OllamaTranslator` (not `OpenAiTranslator`). The capability module docs explain this is deliberate -- both share the same wire shape. The `OpenAiTranslator` exists but is used only when explicitly constructed (e.g., by `OpenAiCompatBackend` tests).

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| C.25a | `translator_for()` routes OpenAiJson to OllamaTranslator, not OpenAiTranslator | `capability.rs:160` | Low -- deliberate design, documented in module docs |
| C.25b | GeminiTranslator not routed by `translator_for()` | `capability.rs:153-165` | Medium -- gemini_native format falls through to ReAct. The `GeminiNativeBackend` uses `GeminiTranslator` directly, bypassing `translator_for()`. |

### Verify
```bash
grep -n "pub fn translator_for" crates/roko-agent/src/translate/capability.rs
grep -n "pub fn capabilities_from_profile" crates/roko-agent/src/translate/capability.rs
```

---

## C.26 -- Reasoning extraction (BackendResponse::extract_reasoning) (Doc 09)

- **Status**: DONE
- **Priority**: P1
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
`BackendResponse` provides `extract_reasoning()` handling four different reasoning wire formats. Used by ChatResponse construction, episode logging, and cost computation.

### What exists
`crates/roko-agent/src/translate/mod.rs:208`:
```rust
pub fn extract_reasoning(&self) -> Option<String>
```

Handles:
1. **OpenAI/GLM**: `choices[0].message.reasoning_content` (line 211-213)
2. **Ollama**: `message.reasoning_content` (line 213)
3. **Anthropic blocks**: `content` array with `type: "thinking"` blocks (line 265-283)
4. **Stream events**: `delta.reasoning_content`, `delta.thinking`, `content_block.reasoning_content`, `content_block` with `type: "thinking"`, `delta` with `type: "thinking_delta"` (line 285-327)

Tested with: OpenAI JSON, Claude JSON blocks, stream-json events. All 4 wire formats covered.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| -- | None | -- | -- |

### Verify
```bash
grep -n "fn extract_reasoning" crates/roko-agent/src/translate/mod.rs
```

---

## C.27 -- FinishReason normalization (Doc 09)

- **Status**: DONE
- **Priority**: P1
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
Implied by doc 09's mention of reasoning extraction and ChatResponse construction.

### What exists
`crates/roko-agent/src/translate/mod.rs:82`:
```rust
pub fn normalize_finish_reason(raw: &str) -> FinishReason {
    match raw {
        "stop" | "end_turn" => FinishReason::Stop,
        "length" | "max_tokens" => FinishReason::Length,
        "tool_calls" | "tool_use" => FinishReason::ToolCalls,
        "content_filter" | "sensitive" => FinishReason::ContentFilter,
        "network_error" => FinishReason::Error("network_error"),
        "model_context_window_exceeded" => FinishReason::Error("context_overflow"),
        other => FinishReason::Error(other),
    }
}
```

Maps 10+ provider-specific strings to 5 canonical `FinishReason` variants. 11 test assertions.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| -- | None | -- | -- |

### Verify
```bash
grep -n "pub fn normalize_finish_reason" crates/roko-agent/src/translate/mod.rs
```

---

## C.28 -- ChatResponse canonical type (Doc 09)

- **Status**: DONE
- **Priority**: P1
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
Implied by doc 09.

### What exists
`crates/roko-agent/src/translate/mod.rs:58`:
```rust
pub struct ChatResponse {
    pub content: String,
    pub reasoning: Option<String>,
    pub tool_calls: Vec<ToolCall>,
    pub usage: Usage,
    pub finish_reason: FinishReason,
    pub metadata: ResponseMetadata,
}
```

`ResponseMetadata` at line 69: `response_id`, `model_used`, `cached_tokens`, `content_filter`, `web_search`, `extra`, `provider_latency_ms`, `raw_finish_reason`.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| -- | None | -- | -- |

### Verify
```bash
grep -n "pub struct ChatResponse" crates/roko-agent/src/translate/mod.rs
```

---

## C.29 -- Reasoning patterns: ReAct (Doc 07)

- **Status**: DONE
- **Priority**: P0
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
The ToolLoop implements the basic ReAct pattern. The `prompt -> LLM -> tool_calls -> dispatch -> results -> LLM` cycle.

### What exists
This is exactly what the `ToolLoop::run()` method implements at `tool_loop/mod.rs:235`. The loop is the ReAct pattern: reasoning (LLM response), acting (tool dispatch), observing (results fed back).

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| -- | None | -- | -- |

### Verify
```bash
grep -n "pub async fn run" crates/roko-agent/src/tool_loop/mod.rs
```

---

## C.30 -- Reasoning patterns: Reflexion (Doc 07)

- **Status**: NOT DONE
- **Priority**: P2
- **Estimated LOC**: ~200
- **Dependencies**: C.17
- **Files to modify**: New file or extension to `crates/roko-agent/src/tool_loop/`

### What the doc says
Doc proposes `ReflexionContext` and `Reflection` structs. When a gate rejects an agent's output, the gate result should be converted to a verbal reflection and injected into the next agent dispatch. Includes `from_gate_failure()` factory and `as_prompt_section()` formatter.

### What exists
No code exists. Searched for `ReflexionContext`, `Reflexion`, `Reflection` across all crates -- zero matches.

The orchestrator does re-run tasks on gate failure (retry logic in orchestrate.rs), but it does NOT generate verbal reflections or inject gate failure summaries into the next attempt's context.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| C.30a | ReflexionContext struct not implemented | crates/roko-agent/ | Medium -- improves retry quality |
| C.30b | Gate failure to verbal reflection pipeline not wired | orchestrate.rs | Medium -- closes feedback loop |

### Verify
```bash
grep -rn "ReflexionContext\|Reflexion\|verbal_reflection" crates/ --include='*.rs' | grep -v target/
```

---

## C.31 -- Reasoning patterns: TreeOfThought / MCTS / LATS (Doc 07)

- **Status**: NOT DONE
- **Priority**: P3
- **Estimated LOC**: ~500+
- **Dependencies**: C.30
- **Files to modify**: New crate or module

### What the doc says
Doc proposes `ReasoningStrategy` enum with variants `Direct`, `ReAct`, `Reflexion`, `TreeOfThought`, `Mcts`, plus `EvaluationMethod` enum. The `CascadeRouter` would pick the reasoning pattern based on task complexity.

### What exists
No code exists. Searched for `ReasoningStrategy`, `TreeOfThought`, `MCTS`, `LATS`, `EvaluationMethod` across all crates -- zero matches. These are research proposals in the doc, not implementation plans.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| C.31a | ReasoningStrategy enum not implemented | N/A | Low -- future research feature |
| C.31b | TreeOfThought/MCTS execution engines not built | N/A | Low -- future research feature |
| C.31c | CascadeRouter integration with reasoning strategies not wired | N/A | Low -- requires C.31a/b first |

### Verify
```bash
grep -rn "ReasoningStrategy\|TreeOfThought\|MCTS\|LATS" crates/ --include='*.rs' | grep -v target/
```

---

## C.32 -- Tool RAG (retrieval-augmented tool selection) (Doc 07)

- **Status**: NOT DONE
- **Priority**: P3
- **Estimated LOC**: ~150
- **Dependencies**: roko-index HDC embeddings
- **Files to modify**: New module in `crates/roko-agent/src/`

### What the doc says
`ToolRag` struct using HDC embeddings from `roko-index` to retrieve only relevant tools per query. Reduces token consumption by up to 99.6%.

### What exists
No code exists. Searched for `ToolRag`, `tool_rag`, `tool_embeddings` -- zero matches. The `roko-index` crate exists but the `bardo-primitives` HDC fingerprints are listed as "Built, not called" in CLAUDE.md.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| C.32a | ToolRag struct not implemented | N/A | Low -- optimization, not required for function |

### Verify
```bash
grep -rn "ToolRag\|tool_rag\|tool_embeddings" crates/ --include='*.rs' | grep -v target/
```

---

## C.33 -- AutoTool (graph-based tool prediction) (Doc 07)

- **Status**: NOT DONE
- **Priority**: P3
- **Estimated LOC**: ~150
- **Dependencies**: Episode data
- **Files to modify**: New module in `crates/roko-agent/src/` or `crates/roko-learn/src/`

### What the doc says
`ToolTransitionGraph` with Markov chain transitions mined from `EpisodeLogger` data. `from_episodes()` builder, `predict_next(current_tool, k)` predictor.

### What exists
No code exists. Searched for `ToolTransitionGraph`, `AutoTool`, `predict_next` -- zero matches.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| C.33a | ToolTransitionGraph not implemented | N/A | Low -- optimization, not required |

### Verify
```bash
grep -rn "ToolTransitionGraph\|AutoTool\|predict_next" crates/ --include='*.rs' | grep -v target/
```

---

## C.34 -- Speculative tool execution (PASTE) (Doc 07)

- **Status**: NOT DONE
- **Priority**: P3
- **Estimated LOC**: ~200
- **Dependencies**: C.33
- **Files to modify**: New module in `crates/roko-agent/src/`

### What the doc says
`SpeculativeExecutor` that runs predicted tools in parallel with LLM reasoning. Uses `ToolTransitionGraph` for predictions. Only speculates on read-only tools.

### What exists
No code exists. Searched for `SpeculativeExecutor`, `speculate`, `speculation_threshold` -- zero matches.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| C.34a | SpeculativeExecutor not implemented | N/A | Low -- advanced optimization |

### Verify
```bash
grep -rn "SpeculativeExecutor\|speculative_cache\|speculation_threshold" crates/ --include='*.rs' | grep -v target/
```

---

## C.35 -- Tool result caching (Doc 07)

- **Status**: PARTIAL
- **Priority**: P2
- **Estimated LOC**: ~150
- **Dependencies**: None
- **Files to modify**: `crates/roko-agent/src/cache.rs`

### What the doc says
`ToolResultCache` with per-tool `CachePolicy` (cacheable, ttl, invalidated_by). Includes `default_cache_policies()` for 19 builtin tools. Separate from provider response caching.

### What exists
`crates/roko-agent/src/cache.rs` implements a `ResponseCache` -- this is a **provider-level** HTTP response cache (content-addressed by request hash), used by `OllamaLlmBackend`. It caches full backend responses, not individual tool results.

The doc proposes a different, tool-level cache that would cache individual tool results (e.g., caching `read_file` output and invalidating on `write_file`). This tool-level cache does NOT exist.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| C.35a | ToolResultCache struct not implemented | N/A | Medium -- reduces latency and token cost |
| C.35b | Per-tool CachePolicy with TTL and invalidation not built | N/A | Medium |
| C.35c | default_cache_policies() for builtin tools not implemented | N/A | Low -- needs C.35a first |

### Verify
```bash
grep -rn "ToolResultCache\|CachePolicy\|default_cache_policies" crates/ --include='*.rs' | grep -v target/
```

---

## C.36 -- Format switching / max_tools truncation (Doc 09)

- **Status**: PARTIAL
- **Priority**: P2
- **Estimated LOC**: ~50
- **Dependencies**: None
- **Files to modify**: `crates/roko-agent/src/translate/capability.rs`

### What the doc says
The `max_tools` field in `ModelProfile` addresses format switching. When set, the adapter truncates the tool array to the specified size, keeping only relevant tools. Truncation happens at the adapter level before the Translator sees the tools.

### What exists
- `ModelCapabilities.max_tools_before_degrade` exists at `capability.rs:43` -- populated from `ModelProfile.max_tools` or profile defaults
- `capabilities_from_profile()` correctly maps `profile.max_tools` to `max_tools_before_degrade` at line 114-117
- Profile defaults: Claude=32, GPT=64, Qwen3=5, unknown=3

However, there is no code that actually TRUNCATES the tool array based on `max_tools_before_degrade`. The field is read and stored but never enforced. The `ToolLoop::run()` passes the full tool array through `translator.render_tools(tools)` without checking the count.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| C.36a | max_tools truncation not enforced anywhere | `tool_loop/mod.rs` | Medium -- causes degraded performance for small models |
| C.36b | Tool relevance ranking for truncation not implemented | N/A | Medium -- depends on C.36a |

### Verify
```bash
grep -rn "max_tools_before_degrade" crates/roko-agent/src/ --include='*.rs' | grep -v target/ | grep -v "test"
```

---

## C.37 -- Translator count: doc says 4, code has 5 (Doc 09)

- **Status**: DONE (code exceeds doc)
- **Priority**: P1
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: `docs/02-agents/09-format-translation.md`

### What the doc says
"The `Translator` trait, the four translator implementations (Claude, Ollama, OpenAI, ReAct)."

### What exists
Five translator implementations:
1. `ClaudeTranslator` -- `translate/claude.rs` (517 lines)
2. `OllamaTranslator` -- `translate/ollama.rs` (562 lines)
3. `OpenAiTranslator` -- `translate/openai.rs` (844 lines)
4. `ReActTranslator` -- `translate/react.rs` (422 lines)
5. `GeminiTranslator` -- `translate/gemini.rs` (416 lines)

All five are re-exported from `translate/mod.rs`:
```rust
pub use claude::ClaudeTranslator;
pub use gemini::GeminiTranslator;
pub use ollama::OllamaTranslator;
pub use openai::OpenAiTranslator;
pub use react::ReActTranslator;
```

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| C.37a | Doc 09 says "four translators" but code has five | `docs/02-agents/09-format-translation.md` line 7 | Low (doc stale) |

### Verify
```bash
grep -c "impl Translator for" crates/roko-agent/src/translate/*.rs
```

---

## C.38 -- The core loop flow (Doc 07)

- **Status**: DONE
- **Priority**: P0
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says
9-step pseudocode:
1. Check iteration cap
2. Check cancellation
3. Send turn to LLM
4. Parse tool calls
5. If no tool calls, return final answer
6. Inject assistant message into history
7. Dispatch tool calls (parallel + serial)
8. Format results as messages
9. Prune context if needed

### What exists
The `run_inner()` private method in `tool_loop/mod.rs` (approximately lines 290-400 in the full file) implements exactly this flow:

1. `max_iter::is_exhausted()` check
2. `ctx.cancel_token` cancellation check
3. `self.backend.send_turn()` with retry policy
4. `self.translator.parse_calls()` on the response
5. Empty calls -> return `ToolLoopOutput` with `StopReason::Stop`
6. `self.translator.render_assistant_message()` -> push to messages
7. `self.dispatcher.dispatch_batch()` with parallel/serial grouping
8. `self.translator.render_results()` + `result_msg::append_results()`
9. Context overflow detection + `prune::prune_if_needed()` + `compaction::compact_tool_results()`

The code also adds: checkpoint persistence on non-Stop exits, usage accumulation across turns, metacognitive monitor integration, retry policy for transient backend errors.

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| -- | None | -- | -- |

### Verify
```bash
grep -n "fn run_inner\|fn run_streaming_inner" crates/roko-agent/src/tool_loop/mod.rs
```

---

## C.39 -- ToolLoopAgent wrapper (not in doc)

- **Status**: DONE (undocumented)
- **Priority**: P1
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: `docs/02-agents/07-tool-loop.md`

### What the doc says
Not documented.

### What exists
`crates/roko-agent/src/tool_loop/agent_wrapper.rs` (359 lines):
- `pub struct ToolLoopAgent` -- bridges the `Agent` trait and the `ToolLoop`
- Implements `Agent::run()` by calling the inner `ToolLoop::run()` and converting `ToolLoopOutput` to `AgentOutput`
- Enables HTTP-backed models to be used interchangeably with CLI-based agents in orchestrate.rs

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| C.39a | ToolLoopAgent not documented in doc 07 | `tool_loop/agent_wrapper.rs` | Low (doc gap) |

### Verify
```bash
grep -n "pub struct ToolLoopAgent" crates/roko-agent/src/tool_loop/agent_wrapper.rs
```

---

## C.40 -- Streaming support (not in doc)

- **Status**: DONE (undocumented)
- **Priority**: P1
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: `docs/02-agents/07-tool-loop.md`

### What the doc says
Not documented. Doc 07 only describes the non-streaming flow.

### What exists
- `LlmBackend::send_turn_streaming()` default method at `tool_loop/mod.rs:83`
- `ToolLoop::run_streaming()` method at `tool_loop/mod.rs` -- full streaming variant of the tool loop
- `OpenAiCompatLlmBackend::send_turn_streaming()` at `openai_compat_backend.rs:249` -- SSE parsing, `StreamAccumulator`, chunk-by-chunk delivery via `mpsc::UnboundedSender<StreamChunk>`
- Full integration test at `openai_compat_backend.rs:757` with a real TCP server emitting SSE events

### Gaps
| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| C.40a | Streaming tool loop not documented in doc 07 | `tool_loop/mod.rs` | Low (doc gap) |

### Verify
```bash
grep -n "pub async fn run_streaming" crates/roko-agent/src/tool_loop/mod.rs
```

---

# Summary

## Tally

| Status | Count | IDs |
|--------|-------|-----|
| **DONE** | 29 | C.01-C.12, C.14-C.16, C.18-C.24, C.26-C.29, C.38-C.40 |
| **PARTIAL** | 4 | C.13, C.17, C.35, C.36 |
| **NOT DONE** | 5 | C.30, C.31, C.32, C.33, C.34 |

## Priority breakdown

| Priority | DONE | PARTIAL | NOT DONE |
|----------|------|---------|----------|
| P0 | 19 | 1 | 0 |
| P1 | 9 | 1 | 0 |
| P2 | 0 | 2 | 1 |
| P3 | 0 | 0 | 4 |

## Key findings

1. **ToolLoop, ToolDispatcher, SafetyLayer, all Translators: fully implemented and tested.** The core tool-loop infrastructure is production-quality with comprehensive test suites. The doc's claims about these components being "Shipping" are accurate.

2. **Five LlmBackend implementations exist** (OllamaLlmBackend, OpenAiCompatLlmBackend, HedgedBackend, GeminiNativeBackend, plus ToolLoopAgent wrapper), exceeding the doc which only describes Ollama as "proving the pattern works" and OpenAiCompat as "missing."

3. **Five Translators exist** (Claude, Ollama, OpenAI, Gemini, ReAct), exceeding doc 09's claim of four. The Gemini translator was added after doc 09 was written.

4. **The #1 integration gap identified in doc 07 (orchestrate.rs bypassing ToolLoop)** is partially addressed via `ToolLoopAgent`, but the primary CLI execution path still bypasses Roko's safety layer (by design for Claude CLI, which has its own safety).

5. **All research proposals (Reflexion, TreeOfThought, MCTS, ToolRAG, AutoTool, SpeculativeExecution, ToolResultCache) are NOT DONE.** These are P2-P3 future work items described as research directions, not implementation commitments.

6. **max_tools truncation is specified but not enforced.** The `max_tools_before_degrade` field is correctly populated from model profiles but never used to actually truncate tool arrays before passing them to translators.

## LOC estimates for remaining work

| ID | What | LOC |
|----|------|-----|
| C.13a | Anthropic HTTP API LlmBackend | ~200 |
| C.17b | Wire ToolLoopAgent into orchestrate.rs for HTTP models | ~100 |
| C.30a+b | Reflexion context and gate-failure feedback | ~200 |
| C.35a+b | Tool-level result cache with invalidation | ~150 |
| C.36a+b | max_tools truncation enforcement | ~50 |
| C.31-C.34 | Research patterns (ToT, MCTS, ToolRAG, speculative) | ~1000+ |
| **Total (excluding research)** | | **~700** |

---

## Agent Execution Notes

### C.13 — Backend Coverage

If you need tool-loop universality, prefer filling the missing backend seam over adding provider-specific bypasses.

Good outcome:

- the backend factory can represent the real provider surface,
- at least one test proves the missing provider path now works,
- no duplicate backend abstraction is introduced.

### C.17 — Dispatcher / ToolLoop Universality

This is the highest-value executable item in the file.

Recommended slice:

1. choose one real plan-execution path in `orchestrate.rs`,
2. route tool-capable HTTP-backed agents through `ToolLoopAgent` and `ToolDispatcher`,
3. reuse the same safety-layer construction approach already visible in `run.rs`,
4. stop after one production path is proven.

Acceptance criteria:

- the chosen orchestrator path uses `ToolDispatcher`,
- tools on that path flow through safety and translation,
- the path is covered by tests or dry-run evidence.

### C.36 — Tool Count Enforcement

Keep this practical:

- enforce the cap deterministically,
- do not pretend semantic ranking exists if it does not,
- defer ToolRAG or graph-based selection work.

Acceptance criteria:

- model/tool caps affect runtime behavior,
- truncation policy is explicit and tested,
- future agents can discover the policy from code or tests.
