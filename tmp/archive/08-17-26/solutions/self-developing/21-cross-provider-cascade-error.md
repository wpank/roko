# 21: Cross-Provider Cascade Error (Gemini Key Error While Using OpenAI)

## Problem

User selected OpenAI/gpt54-mini in Zed, ran `/plan-generate`. Got:
```
Error: model stream failed: agent error (gpt54-mini): Missing API key: env var GEMINI_API_KEY not set
```

This is confusing — the user explicitly chose OpenAI, but the error is about Gemini.

---

## Root Cause: Exact Code Path

The error is NOT from a cascade router or fallback mechanism. It is from the CLI's own model resolution reading the workspace `roko.toml`, which has Gemini configured as the **`default_model`** or first model that the "strategist" role maps to.

### Step-by-step trace

1. User types `/plan-generate describe the feature` in Zed
2. `bridge_events.rs:run_slash_command()` matches `"plan-generate"`:
   ```rust
   "plan-generate" => {
       require_args!("plan-generate", "<description>");
       vec!["plan".into(), "generate".into(), args.into()]
       // model_slug is available here but NOT used
   }
   ```
3. The roko binary is spawned as: `roko plan generate "describe the feature"`
4. `commands/plan.rs:PlanCmd::Generate` runs:
   ```rust
   let model_key = roko_cli::model_selection::resolve_effective_model_key(
       &workdir,
       cli.model.clone(),  // None — no --model flag passed
       Some("strategist"),
       "plan generate",
   )?;
   ```
5. `model_selection.rs:resolve_effective_model_key()` calls `select_candidate()`:
   ```rust
   fn select_candidate(cli_model, task_hint, role, cascade_router, config, cli_provider) {
       // 1. cli_model is None — skip
       // 2. cli_provider is None — skip
       // 3. task_hint is None — skip
       // 4. role is "strategist" → look for [agent.roles.strategist] override
       //    → may find a model, or fall through
       // 5. If CascadeRouter is loaded, use it
       // 6. Otherwise use config.agent.default_model
   }
   ```
6. The workspace `roko.toml` has this (from the actual roko.toml):
   ```toml
   [agent]
   default_model = "gemini-2-5-flash"  # or whatever the first entry is
   ```
   OR the roko.toml has Gemini as one of the models and the cascade router's arm selection happens to pick it.
7. `resolve_model(config, "gemini-2-5-flash")` → `ResolvedModel { provider_kind: GeminiApi, ... }`
8. `run_openai_compat_cognitive_task()` is called for GeminiApi
9. `ModelCallService::stream()` tries to construct the Gemini adapter
10. `gemini/adapter.rs:146-152`:
    ```rust
    if std::env::var(
        provider.api_key_env
            .as_deref()
            .unwrap_or_else(|| "GEMINI_API_KEY".into()),
    ).is_err() {
        return Err(AgentCreationError::MissingApiKey(
            provider.api_key_env.clone().unwrap_or_else(|| "GEMINI_API_KEY".into()),
        ));
    }
    ```
11. `GEMINI_API_KEY` is not set → `AgentCreationError::MissingApiKey("GEMINI_API_KEY")`
12. This propagates up through `ModelCallService` → `BridgeEventsError::Pipeline(e)` → `emit_dispatch_failure("Error: model stream failed: ... Missing API key: env var GEMINI_API_KEY not set")`

**The disconnect**: The ACP session has `model_key = "gpt54-mini"` (user's selection). But the CLI subprocess re-reads `roko.toml` independently and selects a different model based on `default_model` or role overrides. The two model resolution systems are completely disconnected.

### Why the error says "agent error (gpt54-mini)"

The error text `agent error (gpt54-mini)` in the outer error is from the `ModelCallService`'s request label — the `model_key` field of the `ModelCallRequest`, which is set to whatever was passed to the CLI (which may have been "gpt54-mini" in the original prompt context). The inner error chain contains the actual model that failed (Gemini). This makes the error particularly misleading: it looks like gpt54-mini itself is missing GEMINI_API_KEY.

---

## Complete Error Flow for This Bug

```
Zed: /plan-generate "feature"
  → bridge_events.rs:run_slash_command("plan-generate", args, session_id, workdir,
                                       model_slug="gpt54-mini", ...)
    → cli_args = ["plan", "generate", "feature"]
       Note: model_slug NOT included in cli_args
    → tokio::process::Command::new(roko_bin).args(&cli_args).spawn()
    → [subprocess] roko plan generate "feature"
        → commands/plan.rs:PlanCmd::Generate
            → resolve_effective_model_key(workdir, cli.model=None, role="strategist", ...)
                → config.agent.default_model = "gemini-2-5-flash" (from roko.toml)
                → returns "gemini-2-5-flash"
            → run_agent_logged(AgentExecOpts { model: Some("gemini-2-5-flash"), ... })
                → agent dispatcher resolves GeminiApi provider
                → Gemini adapter checks GEMINI_API_KEY
                → Err(AgentCreationError::MissingApiKey("GEMINI_API_KEY"))
        → [subprocess] exits with error text on stderr
    → [parent] slash command reads stderr, sends as TokenChunk:
        "--- stderr ---\nError: ... Missing API key: env var GEMINI_API_KEY not set"
    → Zed shows this inline in chat
```

---

## Why Gemini Error Appears When Using OpenAI: Summary

Three independent factors combine to produce this bug:

1. **No --model passthrough**: The ACP slash command dispatcher builds CLI args without `--model <model_slug>`, even though `model_slug` is available in scope.

2. **Independent model resolution in CLI subprocess**: The `roko plan generate` CLI subcommand calls `resolve_effective_model_key()` which reads `roko.toml` from scratch, not from the ACP session's selected model.

3. **roko.toml default_model points to Gemini**: The workspace `roko.toml` (or the `~/.roko/config.toml` merged into it) has `default_model = "gemini-2-5-flash"` or similar, and `GEMINI_API_KEY` is not set.

None of these individual issues is a cascade router bug. There is no fallback or retry happening. The CLI just picks the wrong model from the start because it has no information about the user's ACP session selection.

---

## The Cascade Router's Role (It's Not the Problem Here)

The `CascadeRouter` in `crates/roko-learn/src/cascade_router.rs` IS consulted during model selection in the CLI, but only if:
1. A cascade router state file exists at `.roko/learn/cascade-router.json`
2. AND `cascade_router` is Some in `select_candidate()`

In `commands/plan.rs:783-788`, `resolve_effective_model_key()` is called WITHOUT a cascade router:
```rust
let model_key = roko_cli::model_selection::resolve_effective_model_key(
    &workdir,
    cli.model.clone(),
    Some("strategist"),
    "plan generate",
)?;
```

This calls the two-argument form of `resolve_effective_model_key` which passes `None` for cascade_router. So the cascade router is NOT involved in this bug.

However, the cascade router DOES have a related issue: it selects from `config.model_keys_for_cascade()` which includes ALL configured models regardless of provider credential availability (this is by design, documented in the source). If a cascade router state existed and selected Gemini, it would also produce this error.

---

## Fix 1: Pass --model to All Agent-Dispatching Slash Commands (Critical)

In `crates/roko-acp/src/bridge_events.rs`, in `run_slash_command()`, the `model_slug: String` parameter must be passed to all agent-dispatching commands. The `model_slug` here is the ACP session's resolved model slug (e.g. `"gpt-5-4-mini-openai"` or the API slug like `"gpt-4o-mini"`).

However, the CLI expects a **model key** (the `[models.KEY]` name), not a slug. The safest approach is to pass the `model_key` stored in the session, which is what the user selected from the dropdown.

In `bridge_events.rs:handle_session_prompt_inner()`, the `model_key` is:
```rust
let model_key = session.config_state.model.clone();
```

This needs to be threaded into `run_slash_command()`. Currently the function receives `resolved.slug.clone()` (the API slug, not the config key):
```rust
return run_slash_command(
    &session_id,
    prompt_text.trim(),
    &workdir,
    resolved.slug.clone(),  // <- this is the API slug, not the model key
    cancel_token,
    event_sender,
    shared_run,
)
.await;
```

The fix is to pass `model_key` instead of `resolved.slug`:

```rust
// In handle_session_prompt_inner():
return run_slash_command(
    &session_id,
    prompt_text.trim(),
    &workdir,
    model_key.clone(),     // <- pass the config model key, not the slug
    cancel_token,
    event_sender,
    shared_run,
)
.await;
```

Then in `run_slash_command()`, the `model_slug` parameter becomes `model_key`, and all agent-dispatching commands become:

```rust
"plan-generate" => {
    require_args!("plan-generate", "<description>");
    vec!["plan".into(), "generate".into(), "--model".into(), model_key.clone(), args.into()]
}
"plan-regenerate" => {
    require_args!("plan-regenerate", "<plan-dir>");
    vec!["plan".into(), "regenerate".into(), "--model".into(), model_key.clone(), args.into()]
}
"prd-draft" => {
    require_args!("prd-draft", "<slug>");
    vec!["prd".into(), "draft".into(), "new".into(), "--model".into(), model_key.clone(), args.into()]
}
"prd-plan" => {
    require_args!("prd-plan", "<slug>");
    vec!["prd".into(), "plan".into(), "--model".into(), model_key.clone(), args.into()]
}
"run" => {
    require_args!("run", "<prompt>");
    vec!["run".into(), "--model".into(), model_key.clone(), args.into()]
}
"research" => {
    require_args!("research", "<topic>");
    vec!["research".into(), "topic".into(), "--model".into(), model_key.clone(), args.into()]
}
"enhance-prd" => {
    require_args!("enhance-prd", "<slug>");
    vec!["research".into(), "enhance-prd".into(), "--model".into(), model_key.clone(), args.into()]
}
"agent-chat" => {
    require_args!("agent-chat", "<agent name>");
    vec!["agent".into(), "chat".into(), "--agent".into(), args.into(),
         "--model".into(), model_key.clone()]
}
```

**Commands that do NOT need --model** (they don't dispatch agents):
- `status`, `doctor`, `config`, `learn`, `learn-router`, `learn-episodes`, `learn-tune`
- `build`, `test`, `clippy`, `fmt`, `gate`, `review-this`, `review`
- `prd-list`, `prd-status`, `plan-list`, `plan-validate`, `plan-validate`
- `knowledge-stats`, `knowledge-gc`, `knowledge-backup`
- `agents` (list only), `audit`
- `index stats`

---

## Fix 2: Cascade Router Should Filter Unavailable Providers

Even after Fix 1, the cascade router can still select models whose providers don't have API keys. This is by design (the router's arm set must be stable across restarts), but the dispatch path should guard against it.

In the CLI's plan run orchestration (`commands/plan.rs:421-432`):
```rust
let cascade_router = std::sync::Arc::new(
    roko_learn::cascade_router::CascadeRouter::load_or_new(&router_path, model_slugs),
);
```

The model selection from the cascade router flows into the orchestrator, which then tries to dispatch. If the selected model's provider is unavailable, the task fails.

### Fix: Pre-flight check in model selection

In `model_selection.rs:select_candidate()`, after a cascade router selection, verify the selected model's provider is available:

```rust
if let Some(router) = cascade_router {
    let model = router.select(Vec::new()).model.slug;
    let model = model.trim();
    if !model.is_empty() {
        // Check if this model's provider is available before committing to it
        if config.provider_available_for_model_key(model) {
            return Ok(ModelCandidate {
                source: SelectionSource::CascadeRouter,
                model: model.to_owned(),
            });
        } else {
            // Cascade router selected an unavailable model — warn and fall through
            warn!(
                model,
                "cascade router selected model with unavailable provider; \
                 falling back to default_model"
            );
        }
    }
}
```

This uses the already-existing `RokoConfig::provider_available_for_model_key()` method from `schema.rs:489-502`.

### Fix: Default model selection should also guard

In `model_selection.rs`, when falling back to `config.agent.default_model`, also check availability:

```rust
// If the default_model's provider is unavailable, find the first available model
let default = config.agent.default_model.trim().to_owned();
if !default.is_empty() {
    if config.provider_available_for_model_key(&default) {
        return Ok(ModelCandidate { source: SelectionSource::DefaultModel, model: default });
    } else {
        // Find the first model with an available provider
        let fallback = config.models.keys()
            .find(|k| config.provider_available_for_model_key(k))
            .cloned();
        if let Some(fallback_model) = fallback {
            eprintln!(
                "warning: default_model '{}' provider is unavailable; \
                 using '{}' instead",
                default, fallback_model
            );
            return Ok(ModelCandidate {
                source: SelectionSource::DefaultModel,
                model: fallback_model,
            });
        }
    }
}
```

---

## Fix 3: Error Message Should Explain the Cascade

When the error "Missing API key: env var GEMINI_API_KEY not set" is produced in a context where the user selected OpenAI, the error chain should include context about why Gemini was chosen.

In `run_slash_command()`, after the child process exits with an error, extract the model from stderr and explain:

```rust
// Parse the error to detect cross-provider confusion
if stderr_trimmed.contains("Missing API key") && stderr_trimmed.contains("GEMINI_API_KEY") {
    let _ = event_sender.send(CognitiveEvent::TokenChunk(format!(
        "\nNote: The command ran with a different model than your session selection.\n\
         Your session: {model_key}\n\
         The subcommand used Gemini because roko.toml default_model points to it.\n\
         Fix: the --model flag is now being passed to fix this automatically.",
    ))).await;
}
```

Long-term, the right fix is Fix 1 above — once `--model` is passed through, this cross-provider confusion cannot occur.

---

## Files to Modify

| File | Change | Priority |
|------|--------|----------|
| `crates/roko-acp/src/bridge_events.rs:1143-1152` | Pass `model_key` (not `resolved.slug`) to `run_slash_command()` | Critical |
| `crates/roko-acp/src/bridge_events.rs:2726-3188` | Add `--model {model_key}` to all agent-dispatching slash commands | Critical |
| `crates/roko-cli/src/model_selection.rs` | Skip unavailable providers in cascade router and default_model fallback | High |
| `crates/roko-acp/src/bridge_events.rs:3191-3293` | Parse child stderr for cross-provider errors and add explanatory note | Medium |

### Concrete code sketch for bridge_events.rs

The function signature changes from:
```rust
async fn run_slash_command(
    session_id: &str,
    raw_input: &str,
    workdir: &Path,
    model_slug: String,     // <- currently the API slug
    ...
```

To:
```rust
async fn run_slash_command(
    session_id: &str,
    raw_input: &str,
    workdir: &Path,
    model_key: String,      // <- the config key (what CLI --model expects)
    ...
```

And the call site in `handle_session_prompt_inner()`:
```rust
// Before (passes API slug):
return run_slash_command(
    &session_id,
    prompt_text.trim(),
    &workdir,
    resolved.slug.clone(),
    ...
).await;

// After (passes model key):
return run_slash_command(
    &session_id,
    prompt_text.trim(),
    &workdir,
    model_key.clone(),  // model_key = session.config_state.model
    ...
).await;
```

Note: `model_key` is already in scope at line 969:
```rust
let model_key = session.config_state.model.clone();
```

The call at line 1143 uses `resolved.slug.clone()` — this needs to change to `model_key.clone()`.

---

## Testing the Fix

After applying Fix 1, the user's scenario should work:
1. User selects `openai` provider and `gpt54-mini` model in Zed
2. Types `/plan-generate add authentication to the API`
3. The CLI runs: `roko plan generate --model gpt54-mini "add authentication to the API"`
4. `resolve_effective_model_key()` finds `cli.model = Some("gpt54-mini")` and returns it
5. The OpenAI provider is used, `OPENAI_API_KEY` is set, dispatch succeeds

Verification command:
```bash
# Simulate what the ACP slash command does today (broken):
roko plan generate "add authentication"

# Simulate what it should do after the fix:
roko plan generate --model gpt54-mini "add authentication"
```

Run both and compare — the first should fail with a Gemini error (if that's the default_model), the second should succeed.
