# S-safety-2: Wire RecoveryAction into dispatch failure path

## Task
When a tool call is denied or fails the safety contract, invoke the YAML-defined `RecoveryAction` instead of "log and proceed."

## Runner Context
Runner audit-2026-05-01, group S. Depends on S-safety-1. Wave 2.

## Source plan
`tmp/subsystem-audits/implementation-plans/28-safety-agent-hardening.md` § S-2.

## Read first

```bash
rg 'RecoveryAction|recovery_for' crates/roko-agent/src/safety/ -n
```

`RecoveryAction` enum likely already exists with variants `AskUser`, `FallbackReadOnly`, `Abort`, `Log`. The contract YAML maps each tool to a recovery action. The dispatch failure path needs to invoke it.

## Exact changes

### 1. Add `handle_violation` to `SafetyLayer`

```rust
impl SafetyLayer {
    pub async fn handle_violation(&self, violation: &SafetyViolation) -> RecoveryOutcome {
        let action = self.contract.recovery_for(&violation.tool);
        match action {
            RecoveryAction::AskUser => RecoveryOutcome::RequestPermission(violation.clone()),
            RecoveryAction::FallbackReadOnly => RecoveryOutcome::RetryWith(SafetyOverlay::read_only()),
            RecoveryAction::Abort => RecoveryOutcome::Abort,
            RecoveryAction::Log => {
                tracing::warn!(?violation, "safety violation logged but not blocked");
                RecoveryOutcome::Continue
            }
        }
    }
}

#[derive(Debug)]
pub enum RecoveryOutcome {
    Continue,
    RequestPermission(SafetyViolation),
    RetryWith(SafetyOverlay),
    Abort,
}
```

### 2. Call from dispatch failure path

In `roko-agent` dispatch (and `roko-acp` session that wraps it):

```rust
match safety_check(...).await {
    Ok(()) => proceed(),
    Err(violation) => match safety_layer.handle_violation(&violation).await {
        RecoveryOutcome::Continue => proceed(),
        RecoveryOutcome::RequestPermission(v) => {
            session.request_permission(v).await
        }
        RecoveryOutcome::RetryWith(overlay) => {
            // Apply overlay; retry once.
        }
        RecoveryOutcome::Abort => return Err(SafetyError::Aborted),
    }
}
```

### 3. Tests

Cover each `RecoveryAction` variant; assert the right `RecoveryOutcome` is returned.

## Write Scope
- `crates/roko-agent/src/safety/mod.rs`
- `crates/roko-agent/src/safety/recovery.rs` (new or extend)
- `crates/roko-agent/src/dispatch.rs` (or wherever the dispatch failure path is)

## Verify

```bash
rg 'handle_violation|RecoveryOutcome' crates/roko-agent/src/safety/
# Expect: at least 4 hits

rg 'RecoveryAction::AskUser|RecoveryAction::FallbackReadOnly' crates/roko-agent/
# Expect: at least 2 hits
```

## Do NOT

- Do NOT bundle with other S-safety batches.
- Do NOT silently ignore the recovery action.
- Do NOT introduce new `RecoveryAction` variants here.
- Do NOT make `handle_violation` infallible — it returns an outcome enum, not a Result.
