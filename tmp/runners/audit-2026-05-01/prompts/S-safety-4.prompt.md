# S-safety-4: ACP dispatch enforces SafetyContract — regression tests

## Task
Add regression tests that prove ACP dispatch enforces `AgentContract`: a blacklisted tool returns `SafetyError::ToolDenied`, not silently dispatched.

## Runner Context
Runner audit-2026-05-01, group S. Depends on S-safety-1. Wave 2.

## Source plan
`tmp/subsystem-audits/implementation-plans/28-safety-agent-hardening.md` § S-4.

## Read first

```bash
rg 'safety_layer|SafetyLayer|AgentContract' crates/roko-acp/src/ -n | head -10
```

R3_F03 (mega-parity) wired safety contract enforcement into ACP dispatch. Verify with regression tests.

## Exact changes

### `crates/roko-acp/tests/safety_enforcement.rs` (new)

```rust
use roko_acp::session::AcpSession;
use roko_agent::safety::{AgentContract, SafetyError};

#[tokio::test]
async fn acp_denies_blacklisted_tool() {
    let contract = AgentContract::restricted("test").deny_tool("bash");
    let session = AcpSession::test_with_contract(contract).await;
    let result = session.dispatch_tool("bash", serde_json::json!({"cmd": "ls"})).await;
    let err = result.unwrap_err();
    assert!(matches!(err, SafetyError::ToolDenied { tool, .. } if tool == "bash"),
        "expected ToolDenied for bash; got {err:?}");
}

#[tokio::test]
async fn acp_allows_allowlisted_tool() {
    let contract = AgentContract::restricted("test")
        .allow_tool("read_file");
    let session = AcpSession::test_with_contract(contract).await;
    let result = session.dispatch_tool("read_file", serde_json::json!({"path": "/tmp/test"})).await;
    // Either Ok or some non-Safety error (path not found etc.); ensure NOT a SafetyError.
    if let Err(e) = result {
        assert!(!matches!(e, SafetyError::ToolDenied { .. }),
            "read_file should be allowed; got {e:?}");
    }
}

#[tokio::test]
async fn acp_overlay_intersects_with_contract() {
    let base = AgentContract::restricted("test").allow_tool("read_file").allow_tool("write_file");
    let overlay = roko_agent::safety::SafetyOverlay::read_only();
    let session = AcpSession::test_with_contract_and_overlay(base, overlay).await;

    let read_r = session.dispatch_tool("read_file", json!({"path": "/tmp"})).await;
    let write_r = session.dispatch_tool("write_file", json!({"path": "/tmp", "content": "x"})).await;

    // read_file: not blocked by overlay.
    if let Err(e) = read_r {
        assert!(!matches!(e, SafetyError::ToolDenied { .. }));
    }
    // write_file: blocked by read_only overlay.
    let werr = write_r.unwrap_err();
    assert!(matches!(werr, SafetyError::ToolDenied { tool, .. } if tool == "write_file"));
}
```

If `AcpSession::test_with_contract_and_overlay` doesn't exist, depend on S-safety-3 having landed first; otherwise leave the third test out and log "blocked on S-safety-3."

## Write Scope
- `crates/roko-acp/tests/safety_enforcement.rs` (new)
- `crates/roko-acp/src/session.rs` (only test helpers, `#[cfg(test)]`)

## Verify

```bash
ls crates/roko-acp/tests/safety_enforcement.rs

rg 'acp_denies_blacklisted_tool|acp_allows_allowlisted_tool' crates/roko-acp/tests/
# Expect: 2+ hits
```

## Do NOT

- Do NOT mock the contract logic; use real `AgentContract`.
- Do NOT bundle with other S-safety batches.
- Do NOT skip if S-safety-3 isn't done; just skip the overlay test.
- Do NOT use `#[ignore]`.
