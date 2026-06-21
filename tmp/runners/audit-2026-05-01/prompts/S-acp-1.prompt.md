# S-acp-1: Add typed validation errors to DispatchResolver

## Task
Replace `DispatchResolver::resolve_existing` returning `Unvalidated` diagnostics with typed `DispatchValidationError` variants (`MissingApiKey`, `ProviderKindMismatch`, `UnknownModel`, `StreamingNotSupported`). Call `validate_for_call` from every site that builds a `DispatchPlan`.

## Runner Context
Runner audit-2026-05-01, group S. No dependencies. Wave 1.

## Source plan
`tmp/subsystem-audits/implementation-plans/21-acp-protocol-completion.md` § ACP-1.

## Why
Today, missing API keys / unsupported capabilities silently proceed to dispatch. The user gets "model error" or a 401 from the provider instead of "missing ANTHROPIC_API_KEY (set env var)." Typed validation catches this before any wire activity.

## Read first

```bash
rg 'Unvalidated|DispatchPlan|fn resolve_existing|fn resolve_for_call|fn validate' crates/roko-agent/src/dispatch_resolver.rs -n
rg 'pub enum DispatchValidationError' crates/roko-agent/src/ -n
rg 'ProviderKind' crates/roko-core/src/config/ -n
```

Identify:
- The current `DispatchResolver` API.
- All the callers of `resolve_existing` / equivalent (`ModelCallService::call`, `ModelCallService::stream`, ACP `session.rs`).
- The `ProviderKind` enum.

## Exact changes

### 1. Define typed error

`crates/roko-agent/src/dispatch_resolver.rs` (or new `dispatch_validation.rs`):

```rust
#[derive(Debug, thiserror::Error)]
pub enum DispatchValidationError {
    #[error("provider {provider} is not configured")]
    UnknownProvider { provider: String },

    #[error("provider {provider} requires auth via env var {env_var} (not set)")]
    MissingApiKey { provider: String, env_var: String },

    #[error("provider {provider} kind mismatch: expected {expected:?}, got {actual:?}")]
    ProviderKindMismatch {
        provider: String,
        expected: ProviderKind,
        actual: ProviderKind,
    },

    #[error("model {model} is not registered (under any provider)")]
    UnknownModel { model: String },

    #[error("model {model} on provider {provider} does not support streaming")]
    StreamingNotSupported { model: String, provider: String },
}
```

### 2. Implement `validate_for_call`

```rust
impl DispatchResolver {
    pub fn validate_for_call(&self, plan: &DispatchPlan) -> Result<(), DispatchValidationError> {
        let provider = self.providers.get(&plan.provider)
            .ok_or_else(|| DispatchValidationError::UnknownProvider { provider: plan.provider.clone() })?;

        if provider.kind.requires_api_key() {
            let env_set = std::env::var(&provider.api_key_env).is_ok();
            let inline_set = !provider.api_key.is_empty();
            if !env_set && !inline_set {
                return Err(DispatchValidationError::MissingApiKey {
                    provider: plan.provider.clone(),
                    env_var: provider.api_key_env.clone(),
                });
            }
        }

        if let Some(expected_kind) = plan.requires_kind {
            if expected_kind != provider.kind {
                return Err(DispatchValidationError::ProviderKindMismatch {
                    provider: plan.provider.clone(),
                    expected: expected_kind,
                    actual: provider.kind,
                });
            }
        }

        let model = self.models.get(&plan.model)
            .ok_or_else(|| DispatchValidationError::UnknownModel { model: plan.model.clone() })?;

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

`ProviderKind::requires_api_key()`: implement on the enum if not present.

```rust
impl ProviderKind {
    pub fn requires_api_key(&self) -> bool {
        matches!(self,
            ProviderKind::AnthropicApi
            | ProviderKind::OpenAi
            | ProviderKind::Cerebras
            | ProviderKind::Cursor
            | ProviderKind::Gemini
            | ProviderKind::Perplexity
        )
        // ClaudeCli, Ollama, Local: no API key needed
    }
}
```

### 3. Call from every dispatch site

`ModelCallService::call`:

```rust
async fn call(&self, req: ModelCallRequest) -> Result<ModelCallResponse, ModelCallError> {
    let plan = self.resolver.resolve_for_call(&req)?;
    self.resolver.validate_for_call(&plan)
        .map_err(ModelCallError::Validation)?;
    // ... dispatch as before
}
```

Same in `stream`. ACP session dispatch (`crates/roko-acp/src/session.rs`) calls into `ModelCallService` already; the validation propagates automatically.

If any caller bypasses `ModelCallService` (e.g. `dispatch_direct.rs`), this batch does NOT migrate them; T5-37 quarantines those.

### 4. Map validation error to `ModelCallError`

```rust
#[derive(Debug, thiserror::Error)]
pub enum ModelCallError {
    #[error("dispatch validation failed: {0}")]
    Validation(#[from] DispatchValidationError),
    // ... existing variants
}
```

### 5. Tests

```rust
#[test]
fn rejects_missing_api_key() {
    let resolver = DispatchResolver::test_with_provider("anthropic", "ANTHROPIC_API_KEY", ProviderKind::AnthropicApi);
    std::env::remove_var("ANTHROPIC_API_KEY");
    let plan = DispatchPlan::for_test("claude-opus", "anthropic", false);
    let err = resolver.validate_for_call(&plan).unwrap_err();
    assert!(matches!(err, DispatchValidationError::MissingApiKey { .. }));
}

#[test]
fn rejects_streaming_when_not_supported() {
    let resolver = test_resolver_with_no_streaming_model();
    let plan = DispatchPlan::streaming("model-no-stream", "provider");
    let err = resolver.validate_for_call(&plan).unwrap_err();
    assert!(matches!(err, DispatchValidationError::StreamingNotSupported { .. }));
}

#[test]
fn rejects_claudecli_route_via_anthropic_api() {
    let resolver = test_resolver_anthropic_api();
    let plan = DispatchPlan::expecting_kind("claude-cli-sonnet", "anthropic_api", ProviderKind::ClaudeCli);
    let err = resolver.validate_for_call(&plan).unwrap_err();
    assert!(matches!(err, DispatchValidationError::ProviderKindMismatch { .. }));
}

#[test]
fn accepts_valid_plan() {
    std::env::set_var("ANTHROPIC_API_KEY", "test-key");
    let resolver = DispatchResolver::test_with_provider("anthropic", "ANTHROPIC_API_KEY", ProviderKind::AnthropicApi);
    let plan = DispatchPlan::for_test("claude-opus", "anthropic", false);
    assert!(resolver.validate_for_call(&plan).is_ok());
}
```

## Write Scope
- `crates/roko-agent/src/dispatch_resolver.rs`
- `crates/roko-agent/src/lib.rs` (re-exports)
- `crates/roko-agent/src/model_call_service.rs`
- `crates/roko-acp/src/session.rs` (if it calls validate directly)

## Read-Only Context
- `crates/roko-core/src/config/`

## Verify

```bash
rg 'DispatchValidationError|validate_for_call' crates/roko-agent/src/
# Expect: at least 6 hits (definition + variants + call sites)

rg 'Unvalidated' crates/roko-agent/src/dispatch_resolver.rs
# Expect: 0 hits in production code (or only in test fixtures)

rg 'rejects_missing_api_key|rejects_streaming_when_not_supported|rejects_claudecli_route_via_anthropic_api' crates/roko-agent/src/
# Expect: 3 hits
```

## Acceptance Criteria

- `DispatchValidationError` enum with 5 typed variants.
- `validate_for_call` implemented; called by `ModelCallService::call` and `::stream`.
- `ProviderKind::requires_api_key` helper defined.
- 4 tests cover validation paths.
- No `Unvalidated` diagnostics in production.

## Do NOT

- Do NOT add silent fallback "if validation fails, try the next model."
- Do NOT validate at proxy / dispatch time only — validate before any wire activity.
- Do NOT bundle with S-acp-2/3/4.
- Do NOT change `ProviderKind` enum variants (separate concern).
- Do NOT make validation async unless it actually needs async (env var check is sync).
