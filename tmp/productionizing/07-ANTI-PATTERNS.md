# Anti-Patterns and Things NOT To Do

Read this BEFORE making any changes. These are hard-learned patterns from the codebase.

## 1. DO NOT add new providers or models to roko.toml without key validation

**The problem**: roko.toml has 50+ model entries across 10+ providers. Most have no API keys.
The CascadeRouter can learn to prefer any of them, then fail at dispatch.

**DO**: Only add models for providers you have keys for.
**DO NOT**: Add a provider entry "just in case" or "for completeness."
**DO NOT**: Assume `default_backend = "anthropic"` means only Anthropic is used — the cascade
router, role configs, and task hints can all override it.

## 2. DO NOT use `unwrap()` or `expect()` in runtime code

**The problem**: 92 `expect()` calls in orchestrate.rs. Any one of them panics = process crash.

**DO**: Use `?` operator for Result propagation. Use `anyhow::Context` for descriptive errors.
**DO NOT**: Use `.unwrap()` or `.expect("should exist")` in any non-test code.
**DO NOT**: Use `.lock().unwrap()` on mutexes — use `.lock().map_err(|e| ...)` or
`match self.lock() { Ok(guard) => ..., Err(poisoned) => poisoned.into_inner() }`.

```rust
// BAD:
let guard = self.stats.lock().expect("stats lock");

// GOOD:
let guard = self.stats.lock().unwrap_or_else(|poisoned| poisoned.into_inner());

// ALSO GOOD (if you want to propagate):
let guard = self.stats.lock().map_err(|_| anyhow!("stats lock poisoned"))?;
```

## 3. DO NOT silently swallow errors with `.ok()` or `let _ = ...`

**The problem**: 25+ silent error swallows in production paths. Failures become invisible.

**DO**: Log before swallowing: `if let Err(e) = result { tracing::warn!("...: {e}"); }`
**DO NOT**: Write `let _ = important_operation();` without at least a `tracing::warn!`.
**DO NOT**: Use `if let Ok(x) = fallible_thing()` without an `else` branch that logs.

```rust
// BAD:
let _ = state.state_hub.bootstrap_from_workdir(&state.workdir);

// GOOD:
if let Err(e) = state.state_hub.bootstrap_from_workdir(&state.workdir) {
    tracing::warn!(error = %e, "state_hub bootstrap failed, starting fresh");
}
```

## 4. DO NOT add `eprintln!()` for production output

**The problem**: ~50 `eprintln!()` calls bypass structured logging. No log levels, no
integration with observability systems, no alerting.

**DO**: Use `tracing::info!()`, `tracing::warn!()`, `tracing::error!()`.
**DO NOT**: Use `println!()` or `eprintln!()` in library/server code.
**EXCEPTION**: CLI user-facing output in `main.rs` can use `println!()` for direct user feedback.

## 5. DO NOT hardcode model names as fallbacks

**The problem**: `"claude-sonnet-4-6"` appears as a hardcoded fallback in 7 locations.
When Anthropic releases a new model, all these become stale. When the user doesn't have
an Anthropic key, all these fail.

**DO**: Read from config: `config.agent.default_model`
**DO**: Validate the model's provider has a key before using it
**DO NOT**: Write `unwrap_or_else(|| "claude-sonnet-4-6".into())`

```rust
// BAD:
let model = config.agent.model.clone()
    .unwrap_or_else(|| "claude-sonnet-4-6".into());

// GOOD:
let model = config.agent.default_model.clone();
if !config.provider_available_for_model(&model) {
    return Err(anyhow!(
        "default model '{}' requires provider '{}' but no API key is set",
        model, config.provider_for_model(&model)
    ));
}
```

## 6. DO NOT skip the pre-commit checks

**Mandatory before any commit**:
```bash
cargo +nightly fmt --all
cargo clippy --workspace --no-deps -- -D warnings
cargo test --workspace
```

CI uses latest stable rustc which may have stricter lints. Code that compiles locally can
fail in CI.

## 7. DO NOT create new files when you can wire existing code

**The codebase pattern**: "built but never connected." Before building anything new,
`grep -rn 'FunctionName\|StructName' crates/ --include='*.rs' | grep -v target/`

Many features already exist but aren't called from the runtime. Check before building.

## 8. DO NOT assume single-process access to `.roko/` state

**The problem**: All mutexes are process-local. If two roko instances run (e.g., `roko serve`
+ `roko plan run`), they will corrupt shared state files.

**DO**: Use file-level locking (`flock()`) for any `.roko/` state write.
**DO**: Document that only one writer should exist per `.roko/` directory.
**DO NOT**: Add more `tokio::sync::Mutex` and assume that's sufficient for concurrency.

## 9. DO NOT expose the server without auth

**The problem**: `serve.auth.enabled = false` is the default. ALL routes are unprotected.
The terminal endpoint allows arbitrary command execution.

**DO**: Set `serve.auth.enabled = true` + configure an API key before any non-localhost deploy.
**DO NOT**: Deploy to Railway/Fly/any public host with `auth.enabled = false`.
**DO NOT**: Rely on the "auto-enable" logic for Privy — it only works if Privy credentials exist.

## 10. DO NOT add provider configs without `api_key_env`

Every provider entry in roko.toml should have an `api_key_env` field pointing to the
env var name. Without it, the provider appears "always available" and routing can
select it without any key.

```toml
# BAD:
[[providers]]
slug = "my-provider"
name = "My Provider"
base_url = "https://api.example.com"

# GOOD:
[[providers]]
slug = "my-provider"
name = "My Provider"
base_url = "https://api.example.com"
api_key_env = "MY_PROVIDER_API_KEY"
```

Exception: local providers like Ollama that don't need keys.

## 11. DO NOT buffer entire SSE/WebSocket streams for scrubbing

**The problem**: The secret scrubbing middleware intentionally skips `text/event-stream`
to avoid buffering the entire stream. Don't "fix" this by buffering SSE.

**DO**: Scrub secrets at the event production site (before they enter the broadcast channel).
**DO NOT**: Wrap SSE responses in the body scrubber middleware — it will deadlock or OOM.

## 12. DO NOT use `dir.join(user_input)` without path validation

**The problem**: `PathBuf::join()` with `../` sequences can escape the intended directory.

**DO**: Canonicalize and verify the result is within the expected directory:
```rust
let full = dir.join(user_path);
let canonical = full.canonicalize()?;
if !canonical.starts_with(dir.canonicalize()?) {
    return Err(anyhow!("path traversal attempt"));
}
```

## 13. DO NOT rely on `AgentBackend::from_model()` for unknown slugs

**The problem**: Unknown model slugs fall through to `Self::Codex` (OpenAI routing).
A typo like `"claude-sonet"` (missing 'n') silently routes to OpenAI.

**DO**: Validate model slug against configured models before dispatch.
**DO NOT**: Rely on prefix matching to infer the provider.
