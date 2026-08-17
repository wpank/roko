# 12: ACP / Zed Integration Errors

## Bug: `max_tokens` rejected by OpenAI gpt-5.x models

### Error

```
Error: model stream failed: agent error (gpt54-mini): network error: Provider 'openai'
(openai_compat) error: http 400: { "error": { "message": "Unsupported parameter:
'max_tokens' is not supported with this model. Use 'max_completion_tokens' instead.",
"type": "invalid_request_error", "param": "max_tokens", "code": "unsupported_parameter" } }
```

### Root Cause

OpenAI's newer models (gpt-4o, gpt-5.x, o3, o4) reject the `max_tokens` parameter and require `max_completion_tokens` instead. The roko codebase already supports this via `use_max_completion_tokens = true` in model config, but none of the OpenAI model entries in `roko.toml` had it set.

### Fix Applied

Added `use_max_completion_tokens = true` to all OpenAI gpt-5.x, o3, o4, and gpt-4o model entries in `roko.toml`.

### What Should Have Prevented This

1. **Auto-detection**: If the model slug starts with `gpt-5`, `o3`, `o4`, or `gpt-4o`, automatically use `max_completion_tokens`. No config needed.

```rust
fn should_use_max_completion_tokens(slug: &str) -> bool {
    slug.starts_with("gpt-5")
        || slug.starts_with("gpt-4o")
        || slug.starts_with("o3")
        || slug.starts_with("o4")
        || slug.starts_with("codex")
}
```

2. **Error recovery**: When the API returns `"code": "unsupported_parameter"` with `"param": "max_tokens"`, automatically retry with `max_completion_tokens` and remember this for future calls.

3. **Config templates**: When `roko config models add` creates a new OpenAI model entry, auto-set `use_max_completion_tokens = true` for any gpt-4o+ model.

---

## Complete Error Flow Diagrams

### Path 1: Transport-level error (JSON-RPC parse)

```
stdin → StdioTransport::read_message()
  → Err(TransportError::Json(e))
    → send_error(JsonRpcId::Null, PARSE_ERROR, "failed to parse JSON-RPC message: {e}")
    → continue loop (not fatal)
  → Err(other)
    → return Err(e).context("failed to read ACP message")
    → handler.rs top-level trap:
        send_error_response({ "code": -32603, "message": "ACP server failed to start: {e:#}" })
        → printed to stdout
        → server exits
```

### Path 2: session/prompt dispatch error

```
Zed sends session/prompt
  → handle_request() → "session/prompt" branch
    → handle_session_prompt_inner()
      → resolve_model(roko_config, model_key) → ResolvedModel { provider_kind, slug, ... }
      → cognitive_task = tokio::spawn(async {
          match provider_kind {
            AnthropicApi → run_anthropic_cognitive_task(...)
            _ → run_openai_compat_cognitive_task(...)
          }
        })
      → stream_events_to_editor() polls event_receiver
          CognitiveEvent::Failure { message } →
            dispatch_failure_update(message) as AgentMessageChunk →
            send_session_update(transport, session_id, update) →
            Zed receives session/update notification with the text
          CognitiveEvent::Complete { stop_reason } → returns StreamResult
      → cognitive_task.await
          Ok(Ok(())) → no error
          Ok(Err(e)) → task_error = Some(e.to_string()) (logged, NOT sent to Zed directly)
          Err(join_err) → task_join_error = Some(...) (will be returned as Err to handler)
      → if task_join_error → return Err(join_error)  [BridgeEventsError bubbles to handler]
      → else → stream_result.map(|sr| sr.prompt_result)
```

The critical insight: **model errors reach Zed via CognitiveEvent::Failure messages, NOT via JSON-RPC error responses**. By the time the error occurs, the method has already returned a success response (the streaming updates follow). The user sees the error inline in the chat, not as a dialog.

### Path 3: Stream-level model error

```
run_anthropic_cognitive_task() or run_openai_compat_cognitive_task()
  → ModelCallService::stream(request)
      Err(e) →
        emit_dispatch_failure(event_sender, "Error: model stream failed: {e}")
          → CognitiveEvent::Failure { message: "Error: model stream failed: ..." }
            → stream_events_to_editor receives it
              → dispatch_failure_update(message) → SessionUpdate::AgentMessageChunk
              → StdioTransport sends session/update to Zed
              → Zed shows the text inline in chat
        return Err(anyhow!("model stream failed: {error}").into())
      Ok(stream) →
        loop forward_model_stream_event()
          ModelStreamEvent::Failed { error } →
            emit_dispatch_failure("Error: model stream failed: {error}")  [same path above]
          ModelStreamEvent::AttemptFailed { model, error } →
            warn!() only — NOT forwarded to Zed (silent retry signal)
```

### Path 4: Provider not configured (Anthropic with no API key)

```
resolve_model(roko_config, model_key) → ResolvedModel { provider_kind: AnthropicApi, ... }
  → run_anthropic_cognitive_task()
    → anthropic_model_call_config(roko_config, model_key, slug)
        → config.providers.iter().find_map(|(id, p)| p.kind == AnthropicApi) → None
        → return None
      → emit_dispatch_failure("Error: Anthropic provider is not configured for ACP dispatch.")
      → return Err(anyhow!("Anthropic provider is not configured"))
```

The error "Anthropic provider is not configured for ACP dispatch." comes from this `None` path when there is NO `[providers.*]` entry with `kind = "anthropic_api"` in roko.toml.

The error "Missing API key: env var ANTHROPIC_API_KEY not set" comes from a DIFFERENT path — it's `AgentCreationError::MissingApiKey(env_var)` from `crates/roko-agent/src/provider/anthropic_api.rs:85`, which fires when the provider IS configured but the env var is absent.

### Path 5: Server startup failure

```
run_acp_server(config)
  → run_acp_server_inner(config) → Err(e)
    → handler.rs:32-47:
        write to stdout:
          { "jsonrpc": "2.0", "id": null,
            "error": { "code": -32603,
                       "message": "ACP server failed to start: {e:#}" } }
        return Err(e)
```

This is the only place a startup error reaches Zed as a JSON-RPC response. Everything else arrives as streaming `session/update` notifications inline in chat.

---

## Every Error Message the ACP Can Produce

| Error text | Source | Root cause |
|---|---|---|
| `"ACP server failed to start: {e:#}"` | `handler.rs:42` | Fatal startup error (e.g. can't create `.roko/`, logging init failed) |
| `"failed to parse JSON-RPC message: {error}"` | `handler.rs:155-158` | Zed sent malformed JSON |
| `"method '{method}' is not supported"` | `handler.rs:446-450` | Zed sent an unknown JSON-RPC method |
| `"session '{id}' already has an active prompt"` | `BridgeEventsError::SessionBusy` / `bridge_events.rs:947` | User sent a second prompt before first completed |
| `"Error: Anthropic provider is not configured for ACP dispatch."` | `bridge_events.rs:1474-1478` | No `[providers.*]` entry with `kind = "anthropic_api"` in roko.toml |
| `"Error: model stream failed: {e}"` | `bridge_events.rs:1597-1602` | `ModelCallService::stream()` returned Err — includes nested provider error |
| `"Error: model stream failed: {error}"` | `bridge_events.rs:1668-1670` | `ModelStreamEvent::Failed` from the stream loop |
| `"Error: session MCP tools require an explicitly configured provider for model '{}'."` | `bridge_events.rs:1833-1844` | MCP tool loop started but provider_config is None |
| `"Error: session MCP tools require an explicitly configured model profile for '{}'."` | `bridge_events.rs:1847-1859` | MCP tool loop started but profile is None |
| `"Error: MCP tool loop stopped because the model-call budget was exhausted."` | `bridge_events.rs:1969-1975` | Tool loop hit budget limit |
| `"Error: MCP tool loop failed: {error}"` | `bridge_events.rs:1978-1983` | Backend error in tool loop |
| `"Safety check blocked this action: {message}"` | `bridge_events.rs:1129-1133` | Pre-dispatch safety violation (Block severity) |
| `"Missing API key: env var {env_var} not set"` | `AgentCreationError::MissingApiKey` | Provider configured but env var absent |
| `"Missing required config field: {message}"` | `AgentCreationError::MissingConfig` | Provider config missing required field |
| `"ACP pipeline error: {e}"` | `BridgeEventsError::Pipeline` | Pipeline runner failed |

All model/API errors arrive in Zed as inline chat text via `SessionUpdate::AgentMessageChunk`, not as JSON-RPC error responses. This means they look like assistant text to the user, which is confusing.

---

## Provider Availability Check: Exact Logic

The `check_provider_readiness()` function in `handler.rs:235-263` runs at startup and after config reloads:

```rust
fn check_provider_readiness(config: &RokoConfig) -> Option<String> {
    if config.providers.is_empty() {
        return Some("no providers configured in roko.toml — agent dispatch will fail; ...");
    }
    for (_name, provider) in &config.providers {
        if provider.kind == ProviderKind::ClaudeCli {
            return None;  // ClaudeCli always OK — no key needed
        }
        if let Some(env_var) = &provider.api_key_env
            && std::env::var(env_var).ok().filter(|k| !k.is_empty()).is_some()
        {
            return None;  // First provider with a valid key = ready
        }
    }
    Some("no provider has resolvable credentials — check api_key_env vars ...")
}
```

This is a **coarse check** — it returns a warning string or None. It does NOT:
- Check which models are linked to which providers
- Return per-provider status
- Block dispatch (it only warns to logs)

The **fine-grained check** lives in `RokoConfig::is_provider_available()` in `crates/roko-core/src/config/schema.rs:473-485`:

```rust
pub fn is_provider_available(&self, provider: &ProviderConfig) -> bool {
    if matches!(provider.kind, ProviderKind::ClaudeCli | ProviderKind::CursorAcp) {
        return true;  // CLI providers don't need an API key
    }
    match provider.api_key_env.as_ref().map(|s| s.trim()) {
        None => false,          // No env var configured = not available
        Some("") => true,       // Empty string = skip key check (unusual)
        Some(name) => std::env::var(name).is_ok() || self.agent_env_value(name).is_some(),
    }
}
```

This is called from `session.rs:build_config_options()` to set `ready:` on each provider/model option in the Zed dropdown. So the dropdown already KNOWS which providers are available — it marks them with `ready: false`. But Zed's UI shows all options, including not-ready ones, without filtering them out.

The `ready` field in `ConfigOptionValue` is already computed correctly. The gap is in Zed's rendering, not in roko's availability logic.

---

## Design: Only Show Working Providers

The infrastructure is already in place. The fix is:

### Option A: Filter at source (in session.rs)

```rust
// In build_config_options(), change the provider list to exclude unavailable ones:
let mut provider_options: Vec<ConfigOptionValue> = roko_config
    .providers
    .iter()
    .filter(|(_, provider)| roko_config.is_provider_available(provider))  // ADD THIS
    .map(|(key, provider)| ConfigOptionValue {
        value: key.clone(),
        name: capitalize_model_key(key),
        description: provider_option_description(roko_config, provider),
        ready: true,  // All remaining are available
    })
    .collect();
```

This removes unavailable providers from the dropdown entirely.

### Option B: Health check on startup (prefer this)

Add a pre-session validation pass that emits a `session/update` with the availability status as inline text:

```rust
// In handle_session_prompt_inner(), before dispatch:
if !roko_config.is_provider_available(&resolved_provider) {
    let env_var = resolved_provider.api_key_env.as_deref().unwrap_or("(not configured)");
    emit_dispatch_failure(
        &event_sender,
        format!(
            "Provider '{}' is not available. Set {} in your environment.\n\
             Available providers: {}",
            provider_name,
            env_var,
            roko_config.available_provider_ids().join(", ")
        ),
    ).await;
    return Err(...);
}
```

### Option C: Only auto-select working providers (default model selection)

In `session.rs`, when building the initial `default_model` / `default_provider`, only pick from providers where `is_provider_available()` returns true:

```rust
fn select_default_model(roko_config: &RokoConfig) -> String {
    // Prefer the configured default_model if its provider is available
    let default = &roko_config.agent.default_model;
    if roko_config.provider_available_for_model_key(default) {
        return default.clone();
    }
    // Fall back to the first available model
    roko_config
        .models
        .keys()
        .find(|k| roko_config.provider_available_for_model_key(k))
        .cloned()
        .unwrap_or_default()
}
```

---

## Design: Passing --model Through Slash Commands

The slash command dispatcher in `bridge_events.rs:run_slash_command()` receives `model_slug: String` but does NOT pass it to the CLI. Every agent-dispatching slash command shells out to `roko <subcommand>` without a `--model` flag, so the CLI uses its own default model resolution (via `resolve_effective_model_key()` in `model_selection.rs`).

This means the user's model choice in Zed is ignored for all slash commands.

### Fix: Pass model_slug to all agent-dispatching slash commands

In `run_slash_command()`, the `model_slug` parameter is already available. Apply it to these commands:

```rust
// Current (wrong):
"plan-generate" => {
    require_args!("plan-generate", "<description>");
    vec!["plan".into(), "generate".into(), args.into()]
}

// Fixed:
"plan-generate" => {
    require_args!("plan-generate", "<description>");
    vec!["plan".into(), "generate".into(), "--model".into(), model_slug.clone(), args.into()]
}
```

**Commands that dispatch agents and need `--model` propagation:**

| Slash command | CLI command | Needs --model? |
|---|---|---|
| `plan-generate` | `plan generate` | Yes |
| `plan-regenerate` | `plan regenerate` | Yes |
| `plan-run` | `plan run` | Yes (uses default_model for agent dispatch) |
| `prd-draft` | `prd draft new` | Yes |
| `prd-plan` | `prd plan` | Yes |
| `run` | `run` | Yes |
| `research` | `research topic` | Yes |
| `search` | `research search` | No (uses Perplexity always) |
| `enhance-prd` | `research enhance-prd` | Yes |
| `agent-chat` | `agent chat` | Yes |
| `analyze` | `research analyze` | Maybe |
| `status` | `status` | No (no agent dispatch) |
| `doctor` | `doctor` | No |
| `config` | `config show` | No |
| `learn` | `learn all` | No |
| `build`, `test`, `clippy`, `fmt`, `gate` | shell commands | No |
| `review-this`, `review` | shell commands | No |

The `express` and `full` slash commands bypass `run_slash_command()` entirely — they call `run_with_workflow_engine()` or `crate::runner::run_workflow_pipeline()` directly, which receive `model_slug` via `PipelineConfig`. Those are already wired correctly in the `ROKO_ACP_LEGACY` path. In the non-legacy path (`run_with_workflow_engine`), the workflow engine reads the model from roko.toml, not from the session — that is also a gap (see doc 21).

---

## How Error Messages Should Be Reformatted for Zed

Currently, model errors arrive as raw Rust `format!()` strings like:

```
Error: model stream failed: agent error (gpt54-mini): network error: Provider 'openai'
(openai_compat) error: http 400: { "error": { "message": "...", "type": "...", ... } }
```

This is a dump of the Rust error chain. The user sees a wall of technical text and cannot tell:
- Which config key to fix
- What command to run
- Whether this is transient (retry works) or permanent (fix config)

### Reformatting approach

Create a `format_acp_error_for_user(error: &str) -> String` function that pattern-matches on the error string and returns an actionable message:

```rust
fn format_acp_error_for_user(error: &str, model_key: &str, provider_key: &str) -> String {
    // Pattern: max_tokens rejection
    if error.contains("unsupported_parameter") && error.contains("max_tokens") {
        return format!(
            "Model '{}' requires max_completion_tokens instead of max_tokens.\n\
             Fix: add `use_max_completion_tokens = true` to [models.{}] in roko.toml.",
            model_key, model_key
        );
    }
    // Pattern: missing API key
    if let Some(env_var) = extract_missing_key_env_var(error) {
        return format!(
            "Missing API key for provider '{}'.\n\
             Fix: export {}=<your-key>  (or add to ~/.roko/.env)\n\
             Or switch models: /config set model <other-model>",
            provider_key, env_var
        );
    }
    // Pattern: HTTP 401
    if error.contains("http 401") || error.contains("401 Unauthorized") {
        return format!(
            "Authentication failed for provider '{}'.\n\
             Check that {} is set to a valid API key.",
            provider_key,
            /* env var */ "..."
        );
    }
    // Pattern: HTTP 429 (rate limit)
    if error.contains("http 429") || error.contains("rate limit") {
        return format!(
            "Rate limited by '{}'. Try again in a moment, or switch to another model.",
            provider_key
        );
    }
    // Default: show the original but add a hint
    format!(
        "{}\n\nHint: check .roko/acp.log for details, or run /doctor",
        error
    )
}
```

Apply this in `stream_model_call_to_cognitive_events()` at lines 1596 and 1668 in `bridge_events.rs`.

---

## UX Issue: ACP defaults to weak model

The Zed screenshot shows `gpt54-mini` being used by default in ACP mode. This is the same model that fails at plan generation. The ACP/editor workflow should default to a stronger model (sonnet or gpt-5.5) since:
- Editor interactions are typically complex (not batch processing)
- Users expect high-quality responses in real-time
- The cost difference is negligible for interactive use

### Fix

Either:
- Add `[acp] default_model = "sonnet"` config option
- Or pass `--model sonnet` when configuring roko in Zed: `"args": ["acp", "--model", "sonnet"]`

---

## Bug: Prompt echoed/repeated instead of responding

### Symptom

User asks a question in Zed agent panel. Instead of responding, roko echoes the prompt 3 times with "Decision provenance" headers and never actually answers.

### Root Cause

The LLM call fails (e.g. max_tokens error → HTTP 400) but:
1. The failure is delivered as a `CognitiveEvent::Failure` text block
2. The session history still gets the user turn pushed to it (at `bridge_events.rs:995`)
3. The assistant turn is NOT pushed on failure (the push at line 1444 requires non-empty `sr.assistant_text`)
4. On the NEXT prompt, the accumulated failed history (3 user turns with no assistant response) is fed to the model, which produces confusing output

The "repeated 3 times" symptom is multiple failed attempts each pushing a user turn without a corresponding assistant turn.

### Fix

When `handle_session_prompt` gets an LLM error:
1. Pop the user turn if the assistant never responded (undo `push_user_turn`)
2. Show the error to the user immediately with actionable guidance
3. Never echo the user's prompt back as "output"

```rust
// In handle_session_prompt_inner(), after error detection:
if !sr.assistant_text.is_empty() {
    session.push_assistant_turn(...);
} else {
    // Roll back the user turn — the exchange failed
    session.pop_last_user_turn();
}
```

---

## Bug: Model dropdown shows "No matches"

### Symptom

After config changes, Zed's model/provider dropdown shows "No matches" — user can't select any model.

### Root Cause (hypothesis)

The ACP process caches config at startup. After `roko.toml` changes, the running ACP process has stale config. Zed may also cache the model list from the `initialize` response.

### Fix

1. **Immediate**: User must restart the agent panel in Zed (closes and restarts the ACP process)
2. **Proper fix**: The ACP should watch `roko.toml` for changes and send a `config/updated` notification to Zed with the new model list
3. **The `ConfigWatcher` already exists** in `crates/roko-acp/src/config_watch.rs` — verify it sends model list updates to Zed

The config reload is already wired at `handler.rs:166-220`: when `config_watcher.changed()`, the config is reloaded and `send_config_options_notification()` is sent for all active sessions. This should fix the dropdown, but only triggers on the NEXT request. A test with a deliberate config change between prompts is needed to verify it works.

---

## Bug: Anthropic selectable in Zed but requires API key user doesn't have

### Symptom

User selects "Anthropic" + "Claude Opus" in Zed's agent dropdown. Gets:
```
Missing API key: env var ANTHROPIC_API_KEY not set
```

But user has Claude Code authenticated and working right there.

### Root Cause

The ACP server **cannot use `claude_cli`** because the ACP itself IS a CLI subprocess — you can't spawn a CLI inside the CLI. It requires `AnthropicApi` (direct HTTP), which needs `ANTHROPIC_API_KEY`.

`bridge_events.rs:1508-1511`:
```rust
// 1. Prefer an existing AnthropicApi provider (NOT ClaudeCli — ACP IS the CLI subprocess).
let anthropic_provider_id = config.providers.iter().find_map(|(id, provider)| {
    (provider.kind == ProviderKind::AnthropicApi).then(|| id.clone())
})?;
```

This is a fundamental architectural constraint but it's INVISIBLE to the user.

### What Should Happen

1. **Don't show Anthropic in the dropdown** if `ANTHROPIC_API_KEY` isn't set and no `anthropic-api` provider is configured
2. **Explain the situation clearly**: "Anthropic models in Zed require ANTHROPIC_API_KEY (the Claude CLI auth can't be used inside ACP). Export it or use OpenAI models."
3. **Long-term**: Consider proxying through the authenticated `claude` CLI session that launched the ACP, or using the Claude Max OAuth token directly

### User's Options Right Now

- Export `ANTHROPIC_API_KEY` (requires API access, not just Max subscription)
- Use OpenAI models in Zed (already working with `OPENAI_API_KEY`)
- Use `--model gpt54-mini` or `--model gpt55` in ACP args

---

## Bug: EVERY provider fails or is useless in Zed

### What the user tried (in order)

| Provider | Model | Result |
|----------|-------|--------|
| Anthropic | claude-opus | `Missing API key: ANTHROPIC_API_KEY not set` |
| Claude_cli | (unknown) | `Missing required config field: explicit [providers] and [models] entries are required for protocol command claude and model ''` |
| Gemini | gemini-2-5-flash | `Missing API key: GEMINI_API_KEY not set` |
| Moonshot | kimi-k2-6 | Works but model doesn't know about roko — gives generic "what are you looking for?" response |
| OpenAI | gpt54-mini | Was broken (max_tokens), now fixed but user didn't know to try it |

**5 attempts, 0 successes for the user's actual task.** The UX is "try providers until one works." This is unacceptable.

### What Should Happen

1. **On ACP startup**: Test which providers actually work (have keys, respond to health check)
2. **Only show working providers** in the dropdown — hide Anthropic, Claude_cli, Gemini if keys aren't set
3. **Auto-select the best working provider** — don't default to one that fails
4. **If NO providers work**: Show a clear setup message, not a cryptic error per attempt:
   ```
   No AI providers available. Set one of:
     export OPENAI_API_KEY=...    (for GPT-5.5)
     export ANTHROPIC_API_KEY=... (for Claude)
     export GEMINI_API_KEY=...    (for Gemini)
   ```
5. **When a provider works but the model doesn't know about roko** (Moonshot/Kimi): inject roko's system prompt so the model knows about slash commands, PRDs, plans, etc.

### The ACP System Prompt Problem

Screenshot 3 shows Kimi responding with "I don't have a directory listing tool" — a generic LLM response. This means the ACP isn't injecting roko's context/capabilities into the system prompt for that provider, OR the model is too weak to follow the instructions.

The ACP should always include:
- List of available slash commands
- Workspace context (what project, what tools exist)
- How to use roko features (plans, PRDs, knowledge)

---

## Bug: ACP fails to load global config in non-roko directories

### Symptom

When Zed is open in a directory without `roko.toml` (e.g. "nu" project), the ACP shows:
```
Missing required config field: explicit [providers] and [models] entries are required
for protocol command claude and model ''
```

### Root Cause

The code at `crates/roko-acp/src/config.rs:209-230` (`load_roko_config()`):

```rust
pub fn load_roko_config(&self) -> RokoConfig {
    let opts = LoadOptions::acp();
    let mut local_opts = opts.clone();
    // Disable standard global merge when explicit --global-config is provided.
    local_opts.merge_global = self.global_config_path.is_none();
    let mut cfg = match self.config_path.as_deref() {
        Some(path) => load_config_file(path, &local_opts),
        None => load_config_with_options(&self.workdir, &local_opts),
    }
    .unwrap_or_default();
    // Then merge explicit global config on top.
    if let Some(global_path) = self.global_config_path.as_deref() {
        let mut global_opts = opts;
        global_opts.merge_global = false;
        if let Ok(global_cfg) = load_config_file(global_path, &global_opts) {
            merge_inherited_config(&mut cfg, global_cfg);
        }
    }
    cfg
}
```

When `global_config_path` is None and `workdir` has no `roko.toml`:
- `local_opts.merge_global = true` (the standard core loader merge is active)
- `load_config_with_options(&self.workdir, &local_opts)` walks up from workdir looking for `roko.toml`, finds none, returns default config
- The core loader's global merge (`~/.roko/config.toml`) SHOULD trigger via `merge_global = true`

**The real gap**: `load_config_with_options()` may not find or apply `~/.roko/config.toml` if the path resolution in the core loader differs from what the ACP expects. The ACP logs a warning at `handler.rs:88-116` when no roko.toml is found, and it includes which global config path was found or not found. Check the ACP log (`.roko/acp.log`) first.

### The merge_inherited_config function

The `merge_inherited_config()` function at `config.rs:240-288` uses `entry().or_insert()` semantics:

```rust
fn merge_inherited_config(config: &mut RokoConfig, global: RokoConfig) {
    // providers: global fills gaps, does NOT override local
    for (name, provider) in providers {
        config.providers.entry(name).or_insert(provider);
    }
    // models: same
    for (name, model) in models {
        config.models.entry(name).or_insert(model);
    }
    // default_model: inherit only if local is empty or still at built-in default
    if should_inherit_default_model(config, local_default_model_declared_before_merge) {
        config.agent.default_model = default_model;
    }
    // default_backend: inherit only if local is empty
    if config.agent.default_backend.is_empty() && !default_backend.is_empty() {
        config.agent.default_backend = default_backend;
    }
}
```

**The global config merge bug**: When the explicit `--global-config` path is provided AND the workdir also has a `roko.toml`, the project config is loaded first (with `local_opts.merge_global = false` because `global_config_path.is_some()`). The global config is then merged AFTER. This is correct.

But when NO explicit global config path is provided, the global merge happens inside `load_config_with_options()` via `merge_global = true`. This path uses the core loader's standard global path (`roko_core::config::loader::global_config_path()` → `~/.roko/config.toml`). If the core loader's global merge doesn't trigger (e.g. because no ancestor walk found a roko.toml to anchor to), the global config may be silently skipped.

**Fix**:
1. Add explicit logging: after `load_roko_config()`, log how many providers/models were loaded
2. If zero providers and zero models, and `~/.roko/config.toml` exists, try loading it directly as a fallback
3. Never show "Missing required config field" to users — if the global config works, use it; if it doesn't, explain what's missing from `~/.roko/config.toml`

---

## UX Issue: Error message not actionable in Zed

When the error appears in Zed's agent panel, the user sees a wall of JSON. They can't:
- Tell what to fix
- Know it's a config issue
- Fix it without switching to terminal

### Fix

The ACP error response should return a human-readable message:

```json
{
  "error": "OpenAI rejected 'max_tokens' for gpt-5.4-mini. Add 'use_max_completion_tokens = true' to [models.gpt54-mini] in roko.toml, or switch to a Claude model with: /config set model sonnet"
}
```

See the "How Error Messages Should Be Reformatted for Zed" section above for the implementation approach.
