# 21 — ACP Protocol Completion

The ACP wire blockers from doc 36 (5-link failure chain) are mostly
fixed (A0-3 through A0-5 in the agent-packet ledger). What remains:

1. Provider auth/capability validation in `DispatchResolver` (P2).
2. End-to-end transcript proof (assert against a mocked model stream).
3. Verify `ClaudeCli` vs `Anthropic API` identity is preserved across the
   ACP path.
4. Rename or remove stale wrapper functions that look like raw HTTP/SSE
   sites (e.g. `run_anthropic_cognitive_task`).

Source: doc 36 (ACP/terminal/safety deep audit), doc 35 still-issue
checklist § ACP and chat state.

---

## Anti-Patterns To Enforce

1. **Don't reintroduce raw provider streaming inside ACP.** The
   migration to `ModelCallService::stream` (D6 in the packet ledger) is
   complete; do not regress.
2. **Don't synthesize `ClaudeCli` → `Anthropic API` mapping when API key
   is absent.** Return a typed error.
3. **Don't construct ACP `session/update` payloads with `format!` or
   string concatenation.** Use the typed `SessionUpdate` struct.
4. **Don't trim conversation history silently.** The 40-turn / 64K-char
   FIFO is a known constraint; if a turn is dropped, log it.
5. **Don't use `unwrap()` in the ACP bridge.** A panic crashes the
   editor's stdio session.

---

## [ ] ACP-1: Add provider auth/capability validation to `DispatchResolver`

**File**: `crates/roko-agent/src/dispatch_resolver.rs`

**Why**: Today `DispatchResolver::resolve_existing` returns
`Unvalidated` diagnostics for auth/capability/model. The ACP path then
proceeds, hits the wire, and the user sees a confusing "stream failed"
instead of "missing API key."

### Implementation

1. Define typed errors:

```rust
#[derive(Debug, thiserror::Error)]
pub enum DispatchValidationError {
    #[error("provider {provider} requires auth via {env_var} (not set)")]
    MissingApiKey { provider: String, env_var: String },

    #[error("provider {provider} kind is {actual_kind:?}, but request requires {expected_kind:?}")]
    ProviderKindMismatch { provider: String, expected_kind: ProviderKind, actual_kind: ProviderKind },

    #[error("model {model} is not registered (under any provider)")]
    UnknownModel { model: String },

    #[error("model {model} on provider {provider} does not support streaming")]
    StreamingNotSupported { model: String, provider: String },
}
```

2. Add validation methods to `DispatchResolver`:

```rust
impl DispatchResolver {
    pub fn validate_for_call(&self, plan: &DispatchPlan) -> Result<(), DispatchValidationError> {
        // Check provider exists in config
        let provider = self.providers.get(&plan.provider)
            .ok_or(DispatchValidationError::UnknownProvider { /* ... */ })?;

        // Check provider's required env var is set (or api_key is in config)
        if provider.kind.requires_api_key() && provider.api_key.is_empty()
            && std::env::var(&provider.api_key_env).is_err() {
            return Err(DispatchValidationError::MissingApiKey {
                provider: plan.provider.clone(),
                env_var: provider.api_key_env.clone(),
            });
        }

        // Check provider kind matches request
        if plan.requires_kind != provider.kind {
            return Err(DispatchValidationError::ProviderKindMismatch { /* ... */ });
        }

        // Check model is registered
        let model = self.models.get(&plan.model)
            .ok_or(DispatchValidationError::UnknownModel { model: plan.model.clone() })?;

        // Check streaming support if required
        if plan.streaming && !model.supports_streaming {
            return Err(DispatchValidationError::StreamingNotSupported {
                model: plan.model.clone(),
                provider: plan.provider.clone(),
            });
        }

        Ok(())
    }
}
```

3. Call `validate_for_call` in every site that constructs a
   `DispatchPlan` from `DispatchResolver`:
   - `roko-agent/src/model_call_service.rs::call`
   - `roko-agent/src/model_call_service.rs::stream`
   - ACP session dispatch entry (`crates/roko-acp/src/session.rs`)

4. Tests:

```rust
#[test]
fn rejects_missing_api_key() {
    let r = DispatchResolver::test_setup_with_provider("anthropic", "ANTHROPIC_API_KEY");
    let plan = DispatchPlan::for_model("claude-opus", "anthropic");
    let err = r.validate_for_call(&plan).unwrap_err();
    assert!(matches!(err, DispatchValidationError::MissingApiKey { .. }));
}

#[test]
fn rejects_claudecli_over_anthropic_api_route() {
    let r = test_resolver();
    let plan = DispatchPlan::for_model("claude-cli-sonnet", "anthropic_api");
    let err = r.validate_for_call(&plan).unwrap_err();
    assert!(matches!(err, DispatchValidationError::ProviderKindMismatch { .. }));
}
```

### Verify

```bash
cargo test -p roko-agent dispatch_resolver --lib
rg 'Unvalidated' crates/roko-agent/src/dispatch_resolver.rs
# Should be empty or only in test fixtures
```

### Do not

- Add a fallback path that synthesizes a different provider/model when
  validation fails.
- Skip validation for "internal" calls.
- Make the error variants part of `roko-cli`'s error type — they belong
  in `roko-agent`.

**Estimated effort**: 4-6 hours.

---

## [ ] ACP-2: Add end-to-end transcript proof

**File**: `crates/roko-acp/tests/transcript_e2e.rs` (new)

**Why**: ACP unit tests cover individual events; nothing asserts the
end-to-end transcript shape against a mocked model stream. A
regression in the bridge would not be caught until an editor user
reports it.

### Implementation

1. Construct a fake `ModelCallService` that emits a deterministic
   sequence of `ModelStreamEvent`s.
2. Wire it into a real `AcpSession`.
3. Send a `prompt` request via the ACP transport (use the in-process
   pipe).
4. Assert the JSON-RPC frames received include:
   - One `session/update` with `kind: "agent_message_chunk"` per text
     delta.
   - One `session/update` with `kind: "tool_call_update"` for any tool
     call.
   - A final `session/prompt` response with the full text.
   - On a `Failed` stream event, a `tool_call_update` with
     `status: "failed"` and a final response with the error.

```rust
#[tokio::test]
async fn happy_path_transcript_matches_spec() {
    let stream = vec![
        ModelStreamEvent::TextDelta("Hello ".into()),
        ModelStreamEvent::TextDelta("world.".into()),
        ModelStreamEvent::Completed { text: "Hello world.".into(), usage: None },
    ];
    let session = AcpSession::test_with_stream(stream).await;
    let frames = session.collect_outbound_frames(/* prompt */).await;

    let text_deltas: Vec<_> = frames.iter()
        .filter(|f| f.method == "session/update")
        .filter(|f| f.params["update"]["kind"] == "agent_message_chunk")
        .collect();
    assert_eq!(text_deltas.len(), 2);
    assert_eq!(text_deltas[0].params["update"]["text"], "Hello ");
    assert_eq!(text_deltas[1].params["update"]["text"], "world.");

    let final_resp = frames.iter().rfind(|f| f.method == "session/prompt").unwrap();
    assert_eq!(final_resp.result["text"], "Hello world.");
}

#[tokio::test]
async fn failed_stream_surfaces_typed_failure() {
    let stream = vec![
        ModelStreamEvent::TextDelta("Working...".into()),
        ModelStreamEvent::Failed { reason: "rate_limit".into(), code: Some(429) },
    ];
    let session = AcpSession::test_with_stream(stream).await;
    let frames = session.collect_outbound_frames(/* prompt */).await;

    let failed_update = frames.iter()
        .find(|f| f.params["update"]["kind"] == "tool_call_update"
                && f.params["update"]["status"] == "failed");
    assert!(failed_update.is_some());
}
```

### Verify

```bash
cargo test -p roko-acp transcript_e2e
```

**Estimated effort**: 4 hours (mostly test scaffolding for stream stubbing).

---

## [ ] ACP-3: Verify ClaudeCli vs Anthropic API identity

**File**: `crates/roko-acp/src/session.rs` (and `tests/`)

**Why**: A model `claude-sonnet-4-6` resolved to `provider = "claude_cli"`
must use the `claude` CLI binary. The same model resolved to
`provider = "anthropic"` must use the Anthropic API and require
`ANTHROPIC_API_KEY`. The audit suspects there are still places that
collapse these.

### Implementation

1. Add a regression test:

```rust
#[tokio::test]
async fn claude_cli_does_not_require_anthropic_api_key() {
    // Construct a session with claude_cli provider; ensure ANTHROPIC_API_KEY is unset
    std::env::remove_var("ANTHROPIC_API_KEY");
    let session = AcpSession::test_with_model("claude-cli-sonnet", "claude_cli").await;
    let result = session.send_prompt("hello").await;
    // The CLI binary may not be present in CI; expect either ok or "binary not found"
    // but NOT a "missing ANTHROPIC_API_KEY" error.
    if let Err(e) = result {
        assert!(!format!("{e:?}").to_lowercase().contains("anthropic_api_key"));
    }
}

#[tokio::test]
async fn anthropic_api_requires_api_key() {
    std::env::remove_var("ANTHROPIC_API_KEY");
    let session = AcpSession::test_with_model("claude-sonnet-4-6", "anthropic").await;
    let result = session.send_prompt("hello").await;
    let err = result.unwrap_err();
    assert!(matches!(err, AcpError::Validation(DispatchValidationError::MissingApiKey { .. })));
}
```

2. Audit `bridge_events.rs` and `session.rs` for any:
   ```rust
   if model_slug.starts_with("claude") { use_anthropic_api() }
   ```
   patterns. Replace with explicit provider field check.

### Verify

```bash
cargo test -p roko-acp claude_cli_identity
rg 'starts_with\("claude"\)|.*ProviderKind::ClaudeCli' crates/roko-acp/
```

**Estimated effort**: 2-3 hours.

---

## [ ] ACP-4: Rename or remove stale dispatch wrappers

**File**: `crates/roko-acp/src/bridge_events.rs`,
`crates/roko-acp/src/session.rs`

**Why**: Functions like `run_anthropic_cognitive_task` look like they
own the Anthropic API call but actually delegate to
`ModelCallService::stream`. The fitness inventory still flags them as
potential raw-HTTP sites. Rename for clarity or remove.

### Approach

```bash
rg 'run_(anthropic|openai|claude_cli)_(cognitive_task|stream|agent)' crates/roko-acp/
```

For each match:

- If still used: rename to e.g. `dispatch_via_model_call_service` or
  fold into the calling site (no need for a separate wrapper).
- If unused: delete.

The fitness inventory's allowlist should then drop the entries for
these functions.

### Verify

```bash
rg 'run_anthropic_cognitive_task|run_openai_cognitive_task' crates/
# Empty
cargo test -p roko-acp
```

**Estimated effort**: 1-2 hours.

---

## Combined Verification

```bash
cargo test -p roko-acp
cargo clippy -p roko-acp --no-deps -- -D warnings
rg 'Unvalidated' crates/roko-agent/src/dispatch_resolver.rs   # only test fixtures
rg 'run_anthropic_cognitive_task' crates/                      # 0 matches
ls crates/roko-acp/tests/transcript_e2e.rs                     # exists
```

---

## Status

- [ ] ACP-1 — Add provider auth/capability validation to `DispatchResolver`
- [ ] ACP-2 — Add end-to-end transcript proof
- [ ] ACP-3 — Verify ClaudeCli vs Anthropic API identity
- [ ] ACP-4 — Rename or remove stale dispatch wrappers
