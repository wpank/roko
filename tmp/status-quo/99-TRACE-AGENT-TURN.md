# 99 — TRACE: One Agent Turn, End to End

> **Verification header**
> - Repo HEAD: `5852c93c05a4f1bda8ff880fc752d9fba2ba453e` (branch `main`)
> - Date: 2026-07-08
> - Method: read the real call path with `file:line` at every hop. No inference from doc titles.
> - Scope: a single agent turn from dispatch request → provider → LLM call → tool loop → tool
>   dispatch → safety → result → usage/cost. Deeper second pass over docs 33/38/48.
> - Status tags: **[WIRED]** runs at runtime · **[PARTIAL]** built, conditionally reached ·
>   **[BUG]** confirmed defect · **[BYPASS]** funnel not applied on this path.

---

## 0. TL;DR — the two turns that actually exist

There is no single "agent turn." There are **two disjoint execution shapes**, chosen at
`classify_runtime` (`crates/roko-cli/src/dispatch_v2.rs:1202`):

| Shape | Providers | Who drives the tool loop | Roko `SafetyLayer` + `ToolDispatcher` run? |
|---|---|---|---|
| **A. CLI subprocess** | Claude CLI, Codex | the **subprocess** (Claude/Codex internal loop) | **No — [BYPASS]** |
| **B. AgentResultBridge** | OpenAI-compat, Anthropic API, Cerebras, Gemini, Perplexity, Cursor ACP/CLI, Hermes, OpenClaw | depends on backend (see §5) | only when a **roko `ToolLoop`** is built (OpenAI-compat + `supports_tools`) |

The most consequential fact in this document: **the entire safety/tool-dispatch funnel
(`ToolDispatcher` → `SafetyLayer::check_pre_execution`) only runs for the roko-driven
`ToolLoop`, which is reachable only for OpenAI-compatible providers whose model has
`supports_tools = true`.** Claude CLI (the default provider) drives its own loop inside the
subprocess and never touches roko's `SafetyLayer` for individual tool calls.

---

## 1. Turn sequence diagram (Shape B, OpenAI-compat + tools — the only fully-instrumented path)

```
caller (orchestrate.rs / dispatch_v2)
  │
  │ AgentDispatchRequest { model_key, prompt, system_prompt, workdir, tools, mcp_config, … }
  ▼
AgentDispatcherV2::run_agent_result_bridge            dispatch_v2.rs:795
  ├─ resolve(model_key) ─────────────────────────────► ProviderDispatchResolver::resolve   dispatch_v2.rs:653
  │     └─ resolve_model + effective_{models,providers} → ProviderDispatchSpec { runtime }
  ├─ if Unsupported → early Err                          dispatch_v2.rs:770
  ├─ create_agent(request) ──────────────────────────► create_agent_for_model             provider/mod.rs:182
  │     ├─ mock_agent_from_env? (ROKO_DISPATCHER=mock-*) provider/mod.rs:276
  │     ├─ SafetyLayer::from_config → thread-local scope provider/mod.rs:191, 269
  │     ├─ adapter_for_kind(kind) ───────────────────► provider/mod.rs:163  (10 kinds)
  │     └─ adapter.create_agent(provider, model, opts)   OpenAiCompatAdapter  openai_compat.rs:371
  │           ├─ if model.supports_tools:  build ToolLoop  openai_compat.rs:388
  │           │     ├─ tool_registry_for_options ──────► openai_compat.rs:313  (MCP + allowlist filter)
  │           │     ├─ build_tool_dispatcher (SafetyLayer attached) provider/mod.rs:328
  │           │     ├─ create_openai_compat_backend                openai_compat.rs:397
  │           │     └─ ToolLoopAgent::new(tool_loop)               openai_compat.rs:406
  │           └─ else: CodexAgent (single-shot, NO tools)          openai_compat.rs:419
  ▼
agent.run(input, ctx)                                   → ToolLoopAgent → ToolLoop::run
  ▼
ToolLoop::run_inner  (the loop)                         tool_loop/mod.rs:918
  loop:
   1. prune_context_if_needed                           tool_loop/mod.rs:935
   2. iteration cap? → StopReason::MaxIterations        tool_loop/mod.rs:938
   3. ctx.is_cancelled()? → StopReason::Cancelled       tool_loop/mod.rs:954
   4. budget.check()==Exhausted? → BudgetExhausted      tool_loop/mod.rs:970
   5. send_turn[_streaming]_with_retry ────────────────► backend.send_turn  openai_compat_backend.rs:442
   │     ├─ build_body (messages+tools+session)         openai_compat_backend.rs:273
   │     ├─ rate_limiter.acquire(provider_id)           openai_compat_backend.rs:449
   │     ├─ poster.post_json → HTTP                      openai_compat_backend.rs:451
   │     └─ err → LlmError::Network(decorate_error)  ◄── [BUG] 429 lands here  :460
   6. merge_session_state + extract_usage + fill_cost   tool_loop/mod.rs:1020-1034
   7. budget.record_turn                                tool_loop/mod.rs:1037
   8. translator.parse_calls(response) ────────────────► OpenAiTranslator  translate/openai.rs:64
   9. calls.is_empty()?  ── yes ─► StopReason::Stop (final_text)   tool_loop/mod.rs:1081
      │                    (length-limit → BackendError)          tool_loop/mod.rs:1119
      └─ no ▼
  10. render_assistant_message → push to history         tool_loop/mod.rs:1139
  11. dispatcher.dispatch_batch(calls, ctx) ───────────► ToolDispatcher   dispatcher/mod.rs:517
      │     partition parallel/serial → per-call dispatch() dispatcher/mod.rs:230
      │     ┌─ 0. __truncated arg guard                     :238
      │     ├─ 1. validate(schema)                          :269
      │     ├─ 2. registry.get(def)                         :286
      │     ├─ 2b. tool_selector.is_allowed                 :302
      │     ├─ 3. tool_filter (allowed_/denied_tools)       :324
      │     ├─ 4. def.permission.satisfied_by(role_perms)   :357  ← capability authz
      │     ├─ 3b. safety.check_pre_execution ────────────► SafetyLayer  safety/mod.rs:377  ← ALL policies
      │     ├─ 3c. hook_chain.evaluate (optional)           :413
      │     ├─ 4. resolver.resolve(name) → handler          :462  → roko_std handler_for
      │     ├─ 5. handler.execute  race(timeout, cancel)    :493
      │     ├─ 6. truncate_result                           :500
      │     ├─ 7. safety.scrub_output   (mutate, no block)  :502  ← POST check #1
      │     └─   safety.check_recovery  (contract recovery) :503  ← POST check #2
  12. render_results → append_results to messages        tool_loop/mod.rs:1157
  13. on_turn callback + metacognitive monitor           tool_loop/mod.rs:1169, 1183
  14. iterations++ ; save_checkpoint_snapshot            tool_loop/mod.rs:1219
   └─ back to top
  ▼
ToolLoopOutput → AgentResult
  ▼
fill_cost_from_profile + record_agent_dispatch_feedback  dispatch_v2.rs:806-813
  → .roko/learn/efficiency.jsonl + cascade-router.json + provider-health.json
```

---

## 2. Dispatch request construction & provider resolution

### 2.1 The request object
`AgentDispatchRequest` (`dispatch_v2.rs:1027`) is the provider-neutral turn envelope: `model_key`,
`prompt`, `system_prompt`, `workdir`, `agent_id`, `command`, `timeout_ms`, `mcp_config`, `env`,
`extra_args`, `effort`, `tools` (CSV allowlist — **note this field, it is the alias bug's fuel**),
`bare_mode`, `dangerously_skip_permissions`. Validated at `:1060` (non-empty prompt/model, workdir
exists).

### 2.2 Resolution — `ProviderDispatchResolver::resolve` (`dispatch_v2.rs:653`)
1. `resolve_model(config, model_key)` → base slug + provider kind.
2. Pick `ModelProfile` from `resolved.profile` or `effective_models()` (`:658`).
3. Derive `model_slug` (`:669`) and `requested_provider_id` (`:674`).
4. Match a `ProviderConfig` by id, or (only when no profile) by first provider of matching kind
   (`:679`).
5. `classify_runtime(provider_id, kind, provider)` (`:705`) → `ProviderRuntime`.

### 2.3 `classify_runtime` (`dispatch_v2.rs:1202`) — the fork
- No provider config → `Unsupported::MissingProvider` (`:1207`).
- `CliProviderConfig::from_provider_config` succeeds → **`ProviderRuntime::Cli`** (`:1215`).
  - `from_provider_config` (`:296`): `ClaudeCli` → Claude/Codex-by-exe-name; `OpenAiCompat` only
    accepts CLI when the command name contains `codex` (`:314`), else `UnsupportedCommand`; API
    kinds → `UnsupportedCliProvider` (`:335`).
- Otherwise → **`ProviderRuntime::AgentResultBridge { provider_kind }`** for every API-ish kind
  (`:1241`), or `Unsupported` for `ClaudeCli`-with-no-command.

So `ProviderRuntime` is `Cli` **only** for Claude CLI and Codex; all ~8 remaining providers route
through the bridge and `Agent::run`.

---

## 3. Provider backend selection

`adapter_for_kind` (`provider/mod.rs:163`) is an exhaustive match over **10** `ProviderKind`s →
static adapter singletons:

| ProviderKind | Adapter | Turn engine | file:line |
|---|---|---|---|
| `OpenAiCompat` | `OpenAiCompatAdapter` | roko `ToolLoop` (if tools) / `CodexAgent` | openai_compat.rs:366 |
| `ClaudeCli` | `ClaudeCliAdapter` | subprocess stream-json (own loop) | provider/claude_cli.rs |
| `AnthropicApi` | `AnthropicApiAdapter` | own tool loop | provider/anthropic_api/tool_loop.rs |
| `CursorAcp` | `CursorAcpAdapter` | ACP subprocess | provider/cursor_acp.rs |
| `CursorCli` | `CursorCliAdapter` | CLI subprocess | provider/cursor_cli.rs |
| `PerplexityApi` | `PerplexityAdapter` | search chat (no tools) | perplexity.rs |
| `GeminiApi` | `GeminiAdapter` | gemini native backend | gemini/ |
| `CerebrasApi` | `CerebrasAdapter` | OpenAI-compat-strict | provider/cerebras.rs |
| `Hermes` | `HermesProviderAdapter` | one-shot / CLI | provider/hermes.rs |
| `OpenClaw` | `OpenClawProviderAdapter` | gateway | provider/openclaw.rs |

**Correction to prior docs (38-AGENT-PROVIDERS-TOOLS):** the "~10 providers" count is exactly the
`adapter_for_kind` arm count = **10 `ProviderKind`s** dispatching to **10 static adapters**
(`provider/mod.rs:90-99`). "Codex", "Cursor" and "Perplexity" are not their own `ProviderKind`:
Codex is an `OpenAiCompat` command whose exe name contains `codex`; the CLI-vs-API split is a
runtime classification, not a kind. The often-cited backend list conflates `ProviderKind` (10) with
concrete `Agent` impls (ClaudeCliAgent, CodexAgent, CursorAgent, OllamaAgent, ToolLoopAgent, …).

---

## 4. Request translation (per-backend)

For **CLI providers**, translation is CLI-flag assembly, not message translation:
- Claude: `build_claude_invocation` (`dispatch_v2.rs:377`) — `--print --output-format stream-json
  --verbose --model … --max-turns … --settings <json> [--append-system-prompt] [--mcp-config]
  [--resume]`. System prompt is a native flag (`:399`); MCP config passthrough at `:411`.
- Codex: `build_codex_invocation` (`dispatch_v2.rs:428`) — `exec --json --cd … --sandbox
  workspace-write`; system prompt is **folded into stdin** (`:454`), no MCP flag.

For **OpenAI-compat + tools**, translation is real: `OpenAiTranslator`
(`translate/openai.rs`) does `render_tools` (JSON function specs) / `parse_calls` / `render_results`
/ `render_assistant_message`. `build_body` (`openai_compat_backend.rs:273`) assembles
`{model, messages, tools, max_tokens|max_completion_tokens, session fields, extra_body_params}`.
Provider quirks are injected via `build_extra_body_params` (`openai_compat.rs:227`): GLM thinking
(`:62`), Kimi thinking (`:179`), OpenRouter provider routing (`:81`).

- **Images:** Kimi requires base64 `data:image/...;base64,` — `validate_vision_input`
  (`openai_compat.rs:156`) rejects plain URLs. **[PARTIAL]** — this validator is `#[allow(dead_code)]`
  and not called from `create_agent`/`build_body`; the constraint is documented and unit-tested but
  not enforced on the live request path.
- **Tool-name sanitization:** dots → `__DOT__` on the wire (`translate/openai.rs:166`), reversed on
  parse (`:175`) so `chain.balance` survives OpenAI's `^[A-Za-z0-9_-]+$` name rule.

---

## 5. The LLM call, response parse, tool-call detection

`ToolLoop::run_inner` (`tool_loop/mod.rs:918`) is the roko-owned loop. Per the module header
(`tool_loop/mod.rs:1-6`): **"Claude CLI drives its own internal loop and bypasses this entirely."**
It is reached only through `ToolLoopAgent`, built only by `OpenAiCompatAdapter` when
`model.supports_tools` (`openai_compat.rs:388`). This confirms the ground-truth claim:
**the session/MCP tool loop is OpenAI-compat only.** Anthropic-API has a *separate* loop
(`provider/anthropic_api/tool_loop.rs`); Gemini has its own native backend.

Per-turn detail inside the loop:
- LLM call: `send_turn_with_retry` (`:1269`) or `send_turn_streaming_with_retry` (`:1303`).
- Response parse: `translator.parse_calls(&response)` (`:1062`). Empty → final answer
  (`StopReason::Stop`, `:1081`). A `finish_reason` of `length`/`max_tokens` with empty text becomes
  `BackendError` (`:1119`) — surfaces the "increase max_output" failure explicitly.
- Assistant tool-call message pushed to history (`:1139`) before dispatch so the next turn sees it.

---

## 6. Tool loop iteration → dispatch → safety → result

### 6.1 `ToolDispatcher::dispatch` decision path (`dispatcher/mod.rs:230`)
Order is fixed; **first failure short-circuits and returns `ToolResult::err`** (each stage emits an
audit `Signal` via `emit_audit`):

| # | Stage | file:line | On fail |
|---|---|---|---|
| 0 | `__truncated` arg salvage guard | :238 | `Other` (model hit token limit mid-JSON) |
| 1 | `validate` args vs registry schema | :269 | `SchemaInvalid` |
| 2 | `registry.get(name)` def exists | :286 | `Other("unknown tool")` |
| 2b | `tool_selector.is_allowed` (profile) | :302 | `PermissionDenied` |
| 3 | `tool_filter_block_reason` (allowed/denied lists) | :324 | `PermissionDenied` |
| 4 | `def.permission.satisfied_by(role_perms)` | :357 | `PermissionDenied` — **capability authz** |
| 3b | `safety.check_pre_execution` | :395 | first policy err (see §6.2) — **role/rate/bash/net/path/budget/temporal/contract** |
| 3c | `hook_chain.evaluate` (optional) | :413 | `PermissionDenied` |
| 4 | `resolver.resolve(name)` → handler | :462 | `Other("no handler")` |
| 5 | `handler.execute` raced vs `timeout` + `cancel_token` | :493 | `Timeout` / `Cancelled` |
| 6 | `truncate_result` (`max_result_bytes`) | :500 | (always Ok) |
| 7 | `safety.scrub_output` | :502 | **mutates**, never blocks — POST |
| 8 | `safety.check_recovery` (contract recovery) | :503 | may convert Ok→Err — POST |

Batch grouping: `dispatch_batch` (`:517`) partitions by `ToolConcurrency`; parallel tools run
`buffer_unordered(DEFAULT_MAX_CONCURRENT_TOOLS)`, serial tools sequentially.

### 6.2 `SafetyLayer::check_pre_execution` — pre-check order (`safety/mod.rs:377`)
1. Role-tools whitelist (`role_tools[role].matches(name)`) — `:380`.
2. Rate limiter (`RateLimitKey{role,tool}`) — `:390`.
3. OCaps warrant capability check — `:399`.
4. Bash/`run_tests` command policy + git policy — `:411` (matches `BASH_TOOLS`).
5. Network policy on `url` arg — `:419` (matches `NETWORK_TOOLS`).
6. Path policy on `file_path`/`path`/`pattern` — `:426` (matches `FILE_TOOLS`).
7. Safety budget consume — `:440`.
8. Temporal LTL monitor — `:454`.
9. **Agent contract** invariants + governance — `:460`.

All nine are **blocking** (`Result<(), ToolError>`, first err returned). The `default` role uses
`AgentContract::hardened_default` (`safety/mod.rs:251`). Unknown roles fail closed via
`contract_for_role` → `RestrictedFallback` (`:929`), **except** the fallback clears `allowed_tools`
when the operator configured TOML role-tools/overrides (`:949`) — i.e. an operator-defined role with
no tools list is intentionally permissive. This is the **[PARTIAL]** "falls back to permissive
default when YAML missing" behavior CLAUDE.md flags.

---

## 7. Safety enforcement-point table (enforced vs bypassed vs Warn-only)

| Point | Location | Blocks? | Runs on which turn shape |
|---|---|---|---|
| Capability authz (`satisfied_by`) | dispatcher/mod.rs:357 | **Block** | Shape B + roko `ToolLoop` only |
| `check_pre_execution` (9 policies) | safety/mod.rs:377 | **Block** | Shape B + roko `ToolLoop` only |
| `hook_chain` | dispatcher/mod.rs:413 | **Block** (if attached) | as above |
| `scrub_output` (secret scrub) | safety/mod.rs:638 | mutate, no block | as above (post) |
| `check_recovery` (contract recovery) | safety/mod.rs:657 | may Ok→Err | as above (post) |
| `check_exec_command` (subprocess) | safety/mod.rs:569 | **Block** | `ExecAgent` fallback + raw spawns |
| `pre_dispatch_check` (AGT-01, orchestrator) | safety/mod.rs:680 | **Block** | per-task, before agent run |
| `post_dispatch_check` (AGT-01, orchestrator) | safety/mod.rs:749 | **Warn-only** (`Vec<SafetyViolation>`, `Severity::Warn`) | per-task, after agent run |
| Claude CLI / Codex per-tool safety | — | **[BYPASS]** | Shape A — subprocess owns tools |

Two distinct "post" checks exist and prior docs conflate them:
- **In-dispatch post** = `scrub_output` + `check_recovery` (mutation/recovery, not a Warn list).
- **Orchestrator post** = `post_dispatch_check` (`safety/mod.rs:749`) — this is the **Warn-only**
  one: secret-leak, path-escape, forbidden-tool checks all push `ViolationSeverity::Warn` and the
  caller decides. It never blocks. This matches the ground-truth "post = Warn-only," but note it is
  the *orchestrator-level* post, not the dispatcher's per-call post.

**[BYPASS] finding:** For the default provider (Claude CLI) none of the per-tool safety points run.
Tool safety on that path is delegated to Claude's own permission system, governed only by
`build_settings_json` and the `--dangerously-skip-permissions` flag (whose default-`true` audit is
catalogued in `provider/mod.rs:541-577`). `roko-std` handlers are never invoked; roko's
`SafetyLayer` never sees the calls.

---

## 8. Per-provider capability matrix (as coded)

| Provider (kind) | Streaming | Tools via roko `ToolLoop` | Images | Retry on 429 |
|---|---|---|---|---|
| Claude CLI | subprocess stream-json | no (own loop) | via CLI | subprocess-internal |
| Codex (OpenAiCompat+`codex` exe) | subprocess `--json` | no (own loop) | n/a | subprocess-internal |
| OpenAI-compat + `supports_tools` | yes (`stream_turn`) | **yes** | Kimi base64 (validator dead-code) | **No — [BUG] §9** |
| OpenAI-compat, no tools | via CodexAgent | no | no | No (same map path) |
| Anthropic API | own loop | own loop (not §5) | yes | own loop policy |
| Cerebras (strict OpenAI-compat) | yes | yes (strict tools) | no | No — [BUG] §9 |
| Gemini | native backend | native | yes | native |
| Perplexity | search | no (search only) | no | n/a |
| Cursor ACP/CLI | ACP/subprocess | own | n/a | own |
| Hermes / OpenClaw | one-shot/gateway | via ExecAgent-ish | n/a | provider-specific |

Provider concurrency is capped by `ProviderSemaphores` (`provider/mod.rs:472`), default
`DEFAULT_PROVIDER_MAX_CONCURRENT` permits per provider id; the OpenAI-compat backend additionally
throttles via a process-wide `ProviderRateLimiter` (`openai_compat_backend.rs:449`).

---

## 9. [BUG] 429 rate-limit misclassification → no retry

**Confirmed.** On the OpenAI-compat send path every HTTP/transport error, including HTTP 429, is
wrapped as `LlmError::Network`, not `LlmError::Provider(ProviderError::RateLimit)`:

- `send_turn`: `poster.post_json(...).map_err(|e| LlmError::Network(self.decorate_error(&e)))`
  — `openai_compat_backend.rs:460`.
- `stream_turn`: non-success status → `HttpPostError::http(status, text)` →
  `LlmError::Network(self.decorate_error(&raw_err))` — `openai_compat_backend.rs:519-524`.
- `send_turn_streaming`: same, `:742-745`.

The retry driver only retries **`LlmError::Provider`**:

```rust
// tool_loop/mod.rs:1276
match self.backend.send_turn(...).await {
    Ok(r) => return Ok(r),
    Err(LlmError::Provider(ref error)) if self.retry_policy.should_retry(error, attempt) => { backoff }
    Err(error) => return Err(error),   // ← Network(429) falls here: NO retry
}
```

So a 429 → `LlmError::Network` → immediate `Err` → loop terminates with
`StopReason::BackendError` (`tool_loop/mod.rs:1013`). The turn is abandoned, not backed off.

The machinery to do the right thing **exists but is unwired on this path**:
- `OpenAiCompatAdapter::classify_error` correctly maps `429 → RateLimit{retry_after_ms}` and Z.AI
  business codes (`openai_compat.rs:437-467`) — but is only called by health tracking, **never on
  the send path**.
- `should_retry`/`RetryAction::WaitAndRetry` (`provider/mod.rs:748`) and
  `map_provider_error` (`provider/mod.rs:652`, which produces a friendly "Rate limited … Wait and
  retry" string) both exist; the string is embedded in the `Network` error, so the user sees the
  right message but the code still doesn't retry.

**Fix shape:** in the backend's error path, run `classify_error(status, body)` on non-2xx and emit
`LlmError::Provider(ProviderError::RateLimit{..})` (and `ServerError`, `AuthFailure`, etc.) so
`send_turn_with_retry` engages backoff. Network/DNS failures should remain `Network`.

---

## 10. [BUG] Tool-alias strip on non-Claude backends (PascalCase vs snake_case)

**Confirmed.** `tool_registry_for_options` (`openai_compat.rs:313`) filters the built-in registry by
the CSV allowlist in `AgentDispatchRequest.tools`:

```rust
// openai_compat.rs:341 (parse) + 348 (filter)
let allowed = parse_allowed_tools_csv(options.tools.as_deref());   // openai_compat.rs:252
let tools = registry.all().iter()
    .filter(|tool| allowed.as_ref().is_none_or(|a| a.contains(tool.name.as_str())))  // :348
    .cloned().collect();
```

The roko-std registry names are **snake_case** — confirmed in `handler_for`
(`roko-std/.../tool/handlers.rs:28-43`): `read_file`, `write_file`, `edit_file`, `multi_edit`,
`glob`, `ls`, `grep`, `bash`, `run_tests`, `apply_patch`, `notebook_edit`, `todo_write`,
`exit_plan_mode`, `web_fetch`, `web_search`, `task_agent` (16 resolvable handlers).

If a caller passes a **Claude-style PascalCase** allowlist (`Read,Edit,Bash,Grep` — the names Claude
CLI uses and that appear throughout the codebase's Claude-facing config), `allowed.contains("read_file")`
is `false` for every tool → **the filter strips the entire tool set** and the OpenAI-compat model is
handed zero tools. It then either loops uselessly or returns a no-tool answer. There is **no alias
normalization** between Claude PascalCase and roko-std snake_case at this filter (nor anywhere on the
build path — `sanitize_tool_name` only handles dots, not case). The Claude CLI path is unaffected
because it never consults this registry.

**Note on count:** ground truth cites `TOOL_COUNT=37` builtins; that is the registry-level `ToolDef`
population. Only **16** have executable handlers via `handler_for` — a call to any of the other ~21
defs resolves a def (dispatcher stage 2 passes) but fails at stage 4 handler resolution
(`Other("no handler")`, `dispatcher/mod.rs:462`). Worth reconciling in 38-AGENT-PROVIDERS-TOOLS.

---

## 11. Usage & cost recording (turn close-out)

- Per-turn usage extracted from the response and accumulated: `extract_usage` +
  `total_usage.add` (`tool_loop/mod.rs:1022-1034`); cost back-filled from the model profile pricing
  when the provider reports no dollar amount (`fill_cost_from_pricing`, `:1026`).
- Budget tracker records the turn cost after the LLM call (`TurnCostRecord`, `tool_loop/mod.rs:1044`).
- At bridge close-out, `fill_cost_from_profile` (`dispatch_v2.rs:806`, `:1099`) and
  `record_agent_dispatch_feedback` (`dispatch_v2.rs:989`) write `ModelCallFeedback` to
  `.roko/learn/efficiency.jsonl` (`role: "dispatch_v2"`), plus `provider-health.json` and
  `cascade-router.json` (verified by the in-file test at `dispatch_v2.rs:1478-1494`).
- Streaming path emits `AgentRuntimeEvent::TokenUsage` / `TurnCompleted` / `Exited`
  (`dispatch_v2.rs:876-907`).

---

## 12. Corrections to prior docs

1. **33-AGENT-SAFETY:** "post-check = Warn-only" is precise but ambiguous. There are two posts.
   The Warn-only one is the *orchestrator* `post_dispatch_check` (`safety/mod.rs:749`). The
   *dispatcher* post is `scrub_output`+`check_recovery` (mutation/recovery), which can turn an Ok
   into an Err via a contract recovery rule — not "Warn-only." Both should be listed separately.
2. **38-AGENT-PROVIDERS-TOOLS:** the safety/tool-dispatch funnel is **not** provider-agnostic. It is
   reached only by the OpenAI-compat `ToolLoop`. Claude CLI (default) and Codex bypass it entirely
   ([BYPASS], §7). Any doc implying roko safety-gates every tool call is wrong for the default path.
3. **48-MCP-CRATES:** MCP tool discovery on the OpenAI-compat path uses a `block_on` + spawned OS
   thread (`openai_compat.rs:262-293`) unless `pre_discovered_mcp_tools` is supplied; Claude CLI
   passes MCP straight through as `--mcp-config` with no roko-side discovery. Two entirely different
   MCP wiring paths.
4. Provider count: **10 `ProviderKind`s / 10 adapters** (`adapter_for_kind`), not "~10 backends" —
   the concrete `Agent` impl count is larger and orthogonal to `ProviderKind`.

---

## 13. Checklist (verify against HEAD 5852c93c05)

- [x] `AgentDispatchRequest` → `run_agent_result_bridge` → `resolve` (dispatch_v2.rs:795, 653)
- [x] `classify_runtime` forks Cli (Claude/Codex) vs AgentResultBridge (rest) (dispatch_v2.rs:1202)
- [x] `adapter_for_kind` is a 10-arm exhaustive match (provider/mod.rs:163)
- [x] OpenAI-compat builds `ToolLoop` only when `model.supports_tools` (openai_compat.rs:388)
- [x] `ToolLoop::run_inner` per-turn order: prune→cap→cancel→budget→send→parse→dispatch (tool_loop/mod.rs:918)
- [x] `ToolDispatcher::dispatch` 11-stage order, first-fail short-circuit (dispatcher/mod.rs:230)
- [x] `check_pre_execution` 9 blocking policies, contract last (safety/mod.rs:377)
- [x] in-dispatch post = scrub_output + check_recovery (dispatcher/mod.rs:502-503)
- [x] orchestrator post = post_dispatch_check, Warn-only (safety/mod.rs:749)
- [x] Claude CLI / Codex bypass roko SafetyLayer per-tool ([BYPASS])
- [x] **[BUG]** 429 → LlmError::Network → no retry (openai_compat_backend.rs:460; tool_loop/mod.rs:1276)
- [x] classify_error 429→RateLimit exists but unused on send path (openai_compat.rs:437)
- [x] **[BUG]** allowlist filter strips PascalCase tools; registry is snake_case (openai_compat.rs:252,348; handlers.rs:28)
- [x] 16 handlers in handler_for vs cited 37 registry defs (reconcile)
- [x] usage/cost → efficiency.jsonl + cascade-router.json (dispatch_v2.rs:989)

---

## 14. Roadmap (turn-path hardening)

1. **Wire error classification into the send path** — call `classify_error` on non-2xx in
   `send_turn`/`stream_turn`/`send_turn_streaming` and emit `LlmError::Provider(..)` so retry works
   (fixes §9). Keep DNS/connection failures as `Network`.
2. **Add a tool-name alias layer** — normalize Claude PascalCase ↔ roko-std snake_case before the
   allowlist filter in `tool_registry_for_options` (fixes §10), or reject unknown allowlist names
   loudly instead of silently stripping.
3. **Unify safety across turn shapes** — either route Claude/Codex tool calls through a roko-side
   pre-check (hard given subprocess ownership) or document the [BYPASS] as an accepted boundary and
   ensure `build_settings_json` encodes the equivalent policy.
4. **Reconcile tool count** — make `handler_for` and the registry `ToolDef` set agree, or document
   the def-without-handler set explicitly.
5. **Activate `validate_vision_input`** on the OpenAI-compat request path so the Kimi base64 rule is
   enforced, not just unit-tested.
```
