# S-cog-3: Migrate daimon callers to FailureTracker

## Task
Replace daimon caller sites (per S-cog-1 inventory) with `FailureTracker` calls. After this batch, `roko-daimon` (or daimon modules in `roko-cognitive`) has no production callers.

## Runner Context
Runner audit-2026-05-01, group S. Depends on S-cog-2. Wave 3.

## Source plan
`tmp/subsystem-audits/implementation-plans/31-cognitive-layer-cleanup.md` § CL-2 (caller migration phase).

## Read first

`tmp/runners/audit-2026-05-01/logs/S-cog-1-inventory.md` — the daimon caller list.

## Exact changes

For each caller listed in the inventory:

### Pattern: record a failure

```rust
// Before
self.daimon.record_failure(role, task);

// After
self.failure_tracker.record(FailureRecord {
    role: role.to_string(),
    task_kind: task.kind.clone(),
    model: task.model.clone(),
    error_class: ErrorClass::from(/* ... */),
    at: chrono::Utc::now(),
});
```

### Pattern: query for retry strategy

```rust
// Before
let policy = self.daimon.policy_for(role);

// After
let strategy = self.failure_tracker.suggest_retry_strategy(role);
```

### Pattern: drain alerts

```rust
// Before
self.daimon.drain_alerts()

// After
self.failure_tracker.drain_alerts()
```

### Field migration on the orchestrator

```rust
pub struct Orchestrator {
    // ... existing
    failure_tracker: Arc<Mutex<FailureTracker>>,
    // (Remove `daimon: Arc<...>` after all sites migrated)
}
```

### Special case: F04's `FatigueDetector`

If a caller uses `FatigueDetector` (per F04 already shipped), **do not** migrate that — the inventory marked it as kept. Leave `FatigueDetector` references intact for now. Plan 31 § CL-2 may address it later.

## Write Scope
- `crates/roko-cli/src/orchestrate.rs` (or post-T5-35 split)
- `crates/roko-cli/src/runner/event_loop.rs`
- (Other migration sites per inventory)

## Read-Only Context
- `tmp/runners/audit-2026-05-01/logs/S-cog-1-inventory.md`
- `crates/roko-learn/src/failure_tracker.rs`

## Verify

```bash
# All daimon callers migrated
rg 'use roko_daimon|roko_daimon::' crates/ -g '*.rs' \
  | rg -v 'crates/roko-daimon/' \
  | rg -v 'FatigueDetector'  # F04 keeps this
# Expect: 0 hits

# FailureTracker in product paths
rg 'failure_tracker|FailureTracker' crates/roko-cli/src/
# Expect: at least 3 hits
```

## Do NOT

- Do NOT migrate `FatigueDetector` callers.
- Do NOT bundle with S-cog-2/4/5.
- Do NOT delete `roko-daimon` here (S-cog-4 does).
- Do NOT change `FailureTracker` API (S-cog-2 owns).
