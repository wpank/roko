# S-acp-3: ClaudeCli vs Anthropic API identity regression tests

## Task
Add regression tests that prove `ProviderKind::ClaudeCli` does not require `ANTHROPIC_API_KEY`, and `ProviderKind::AnthropicApi` does. Audit `bridge_events.rs` for any `model_slug.starts_with("claude")` patterns; replace with explicit provider field checks.

## Runner Context
Runner audit-2026-05-01, group S. Depends on S-acp-1. Wave 2.

## Source plan
`tmp/subsystem-audits/implementation-plans/21-acp-protocol-completion.md` § ACP-3.

## Read first

```bash
rg 'starts_with\("claude"\)|ProviderKind::ClaudeCli|ANTHROPIC_API_KEY' crates/roko-acp/ -n
```

If you find `if model_slug.starts_with("claude") { use_anthropic_api() }`, that's the bug — it conflates the model name prefix with the provider kind.

## Exact changes

### 1. Audit and fix `bridge_events.rs` / `session.rs`

Replace prefix checks with provider kind checks:

```rust
// Bad
if req.model.starts_with("claude") {
    dispatch_via_anthropic_api(...)
}

// Good
match req.provider_kind {
    ProviderKind::ClaudeCli => dispatch_via_claude_cli(...),
    ProviderKind::AnthropicApi => dispatch_via_anthropic_api(...),
    other => return Err(DispatchValidationError::ProviderKindMismatch { ... }),
}
```

If the dispatch is centralized in `ModelCallService` (post S-acp-1), there shouldn't be any prefix checks in `roko-acp` at all. Confirm.

### 2. Tests

`crates/roko-acp/tests/provider_identity.rs` (new):

```rust
use roko_acp::session::AcpSession;
use roko_agent::dispatch_resolver::DispatchValidationError;

#[tokio::test]
async fn claude_cli_does_not_require_anthropic_api_key() {
    std::env::remove_var("ANTHROPIC_API_KEY");
    let session = AcpSession::test_with_provider("claude_cli", ProviderKind::ClaudeCli);
    let result = session.send_prompt("hello").await;
    if let Err(e) = result {
        let s = format!("{e:?}").to_lowercase();
        assert!(!s.contains("anthropic_api_key"),
            "ClaudeCli should not error about ANTHROPIC_API_KEY; got {e:?}");
    }
}

#[tokio::test]
async fn anthropic_api_requires_api_key() {
    std::env::remove_var("ANTHROPIC_API_KEY");
    let session = AcpSession::test_with_provider("anthropic", ProviderKind::AnthropicApi);
    let result = session.send_prompt("hello").await;
    let err = result.unwrap_err();
    let s = format!("{err:?}");
    assert!(s.contains("MissingApiKey") || s.contains("ANTHROPIC_API_KEY"),
        "AnthropicApi should error about missing API key; got {err:?}");
}

#[tokio::test]
async fn provider_kind_mismatch_returns_typed_error() {
    // Construct a request that selects provider="anthropic" (kind=AnthropicApi)
    // but expects kind=ClaudeCli. Validation should fail.
    let session = AcpSession::test_default();
    let plan_with_mismatch = test_plan_kind_mismatch();
    let r = session.dispatcher.validate_for_call(&plan_with_mismatch);
    assert!(matches!(r.unwrap_err(), DispatchValidationError::ProviderKindMismatch { .. }));
}
```

If `AcpSession::test_with_provider` doesn't exist, add it. If `send_prompt` is async and CLI binaries aren't available in CI, the test might error with "binary not found"; that's OK as long as it doesn't error with "missing ANTHROPIC_API_KEY."

### 3. Cleanup

```bash
rg 'starts_with\("claude"\)' crates/roko-acp/ crates/roko-agent/ -g '*.rs'
# Expect: 0 hits in production code
```

If a hit is in test code or in a model-name display formatter (not provider routing), it's OK. If a hit is in a routing decision, fix it as in step 1.

## Write Scope
- `crates/roko-acp/tests/provider_identity.rs` (new)
- `crates/roko-acp/src/session.rs` (only if a prefix check exists)
- `crates/roko-acp/src/bridge_events.rs` (only if a prefix check exists)

## Read-Only Context
- `crates/roko-agent/src/dispatch_resolver.rs`

## Verify

```bash
ls crates/roko-acp/tests/provider_identity.rs

rg 'starts_with\("claude"\)' crates/roko-acp/ -g '*.rs'
# Expect: 0 hits in non-test code
```

## Acceptance Criteria

- 3 tests cover ClaudeCli no-API-key, AnthropicApi requires-API-key, ProviderKindMismatch returns typed error.
- No `model_slug.starts_with("claude")` routing in production code.
- Tests run in CI (no `#[ignore]`).

## Do NOT

- Do NOT bundle with S-acp-1/2/4.
- Do NOT make tests depend on the `claude` CLI binary being installed; check error message string only.
- Do NOT remove the `ProviderKind::ClaudeCli` enum variant.
- Do NOT add fallback "if ClaudeCli unavailable, try Anthropic API" in production.
