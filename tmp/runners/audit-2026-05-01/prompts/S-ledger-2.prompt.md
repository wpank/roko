# S-ledger-2: Report builder reads from RunLedger; delete event-replay paths

## Task
Make the workflow report builder construct gate / artifact / command status from `RunLedger` typed entries instead of replaying string events. Delete the legacy event-replay path.

## Runner Context
Runner audit-2026-05-01, group S. Depends on T5-40a + T5-40b. Wave 3.

## Source plan
`tmp/subsystem-audits/implementation-plans/24-runtime-ledger-migration.md` § Phase 4 (slice cleanup).

## Read first

```bash
rg 'fn build_report|fn finalize|event\.kind\s*==' crates/roko-runtime/src/workflow_engine.rs -n
```

Identify the legacy paths that infer status from string events.

## Exact changes

### 1. Replace event-replay reads with ledger queries

In the report builder:

```rust
// Before
let gate_status: HashMap<Rung, bool> = events.iter()
    .filter(|e| e.kind.starts_with("gate."))
    .filter_map(|e| parse_gate_event(e))
    .collect();

// After
let gate_status: HashMap<Rung, GateStatus> = ledger.gate_verdicts_by_rung();
```

Same for artifacts:

```rust
let invalid_required = ledger.artifacts()
    .filter(|a| a.required && matches!(a.outcome, ArtifactOutcome::Invalid | ArtifactOutcome::Missing))
    .count();
```

### 2. Delete the legacy parse helpers

```bash
rg 'fn parse_gate_event|fn parse_artifact_event|fn replay_to_report' crates/roko-runtime/
```

For each, confirm no other callers; delete.

### 3. Tests

Update existing report-builder tests to use a `RunLedger` fixture instead of an event log.

## Write Scope
- `crates/roko-runtime/src/workflow_engine.rs`
- `crates/roko-runtime/src/run_ledger.rs` (only if missing query helpers)

## Verify

```bash
rg 'event\.kind\s*==' crates/roko-runtime/
# Expect: 0 hits in report-builder code paths

rg 'ledger\.gate_verdicts_by_rung|ledger\.artifacts' crates/roko-runtime/
# Expect: at least 2 hits
```

## Do NOT

- Do NOT bundle with S-ledger-1.
- Do NOT keep both old and new readers. Delete the old.
- Do NOT change the report struct fields visible to consumers (that's a separate batch).
- Do NOT touch resume / checkpoint logic (T5-40d territory).
