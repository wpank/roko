# S-ledger-1: Fail-closed persistence for gate + artifact ledger writes

## Task
Make `RunLedger::record_gate` and `record_artifact` fail-closed: a write failure surfaces as `WorkflowOutcome::LedgerFailure`, not log-and-continue.

## Runner Context
Runner audit-2026-05-01, group S. Depends on T5-40a + T5-40b. Wave 3.

## Source plan
`tmp/subsystem-audits/implementation-plans/24-runtime-ledger-migration.md` § Phase 5.

## Why
For correctness-critical entries (gate verdicts, artifact outcomes), a silent persistence failure means the workflow report could lie about what actually happened. Surface the failure.

## Exact changes

### 1. Typed error + fsync

```rust
// crates/roko-runtime/src/run_ledger.rs

#[derive(Debug, thiserror::Error)]
pub enum LedgerWriteError {
    #[error("ledger io: {0}")]
    Io(#[from] std::io::Error),
    #[error("ledger serialize: {0}")]
    Serialize(#[from] serde_json::Error),
}

impl RunLedger {
    pub fn record_gate(&mut self, ...) -> Result<(), LedgerWriteError> {
        let entry = RunLedgerEntry::Gate { ... };
        self.append(entry)?;
        self.fsync()?;
        Ok(())
    }

    pub fn record_artifact(&mut self, ...) -> Result<(), LedgerWriteError> {
        let entry = RunLedgerEntry::Artifact { ... };
        self.append(entry)?;
        self.fsync()?;
        Ok(())
    }

    /// Non-critical writes (commands, checkpoints): no fsync. Continue on error.
    pub(crate) fn append(&mut self, entry: RunLedgerEntry) -> Result<(), LedgerWriteError> { ... }
    fn fsync(&mut self) -> Result<(), LedgerWriteError> { ... }
}
```

### 2. Workflow engine consumes the error

```rust
match ledger.record_gate(...) {
    Ok(()) => {}
    Err(e) => {
        return WorkflowOutcome::LedgerFailure {
            reason: format!("gate ledger write failed: {e}"),
        };
    }
}
```

Add `WorkflowOutcome::LedgerFailure { reason: String }` if not present.

### 3. Non-critical writes stay log-and-continue

`record_command` and `record_checkpoint` keep the existing log-and-continue behavior (their data is non-critical for outcome correctness).

## Write Scope
- `crates/roko-runtime/src/run_ledger.rs`
- `crates/roko-runtime/src/workflow_engine.rs`

## Verify

```bash
rg 'WorkflowOutcome::LedgerFailure' crates/roko-runtime/
# Expect: 2+ hits (definition + use)

rg 'fn fsync|self.fsync\(\)' crates/roko-runtime/src/run_ledger.rs
# Expect: at least 1 implementation + calls
```

## Do NOT

- Do NOT fsync on every command-event append (perf hit; not critical).
- Do NOT make `record_gate` async (`std::fs::File::sync_all` is sync; if using `tokio::fs`, use its `sync_all`).
- Do NOT bundle with S-ledger-2.
- Do NOT delete legacy log-and-continue paths for non-critical writes.
