# PAJ_03: Wire provider health persistence to disk

## Task
Persist `ProviderHealthTracker` state to disk so provider health data survives process restarts.

## Runner Context
Runner PAJ (Projection & Background Tasks), batch 3 of 3. No dependencies.

## Problem
PB-3 anti-pattern: "Health data lost on restart." `ProviderHealthTracker` (provider_health.rs:501-550) tracks circuit breaker state, failure counts, and cooldowns in memory. On process restart, all health data is lost — a provider that was failing badly appears healthy again.

## Current Code

**ProviderHealthTracker** — `crates/roko-learn/src/provider_health.rs:501-550`:
In-memory runtime tracker with `RwLock`. Uses `Instant` for timers.

**ProviderHealth** — fields use `Instant` and `VecDeque<FailureRecord>` which are NOT directly serializable.

## Exact Changes

### Step 1: Add serializable snapshot type

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct ProviderHealthSnapshot {
    pub providers: HashMap<String, ProviderHealthState>,
    pub snapshot_at: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProviderHealthState {
    pub provider_id: String,
    pub state: CircuitState,
    pub consecutive_failures: u32,
    pub total_requests: u64,
    pub total_failures: u64,
    pub last_failure_at: Option<i64>,
    pub cooldown_until: Option<i64>,
    // Note: failure_window (VecDeque<FailureRecord>) with Instants
    // can't be directly serialized — use timestamps instead
    pub recent_failure_timestamps: Vec<i64>,
}
```

### Step 2: Add save/load methods to ProviderHealthTracker

```rust
impl ProviderHealthTracker {
    pub fn save(&self, path: &Path) -> Result<()> {
        let snapshot = self.snapshot();
        let json = serde_json::to_string_pretty(&snapshot)?;
        persist::atomic_write(path, json.as_bytes())
    }

    pub fn load(path: &Path) -> Result<Self> {
        let json = std::fs::read_to_string(path)?;
        let snapshot: ProviderHealthSnapshot = serde_json::from_str(&json)?;
        Self::from_snapshot(snapshot)
    }

    fn snapshot(&self) -> ProviderHealthSnapshot {
        let providers = self.providers.read().unwrap();
        let mut states = HashMap::new();
        for (id, health) in providers.iter() {
            states.insert(id.clone(), ProviderHealthState {
                provider_id: health.provider_id.clone(),
                state: health.state.clone(),
                consecutive_failures: health.consecutive_failures,
                total_requests: health.total_requests,
                total_failures: health.total_failures,
                last_failure_at: health.last_failure_at,
                cooldown_until: health.cooldown_until,
                recent_failure_timestamps: health.failure_window.iter()
                    .map(|f| f.timestamp_ms)
                    .collect(),
            });
        }
        ProviderHealthSnapshot {
            providers: states,
            snapshot_at: chrono::Utc::now().timestamp_millis() as u64,
        }
    }
}
```

### Step 3: Save on significant state changes

```rust
// After recording failures that change circuit state:
pub fn record_failure_and_persist(&mut self, provider: &str, path: &Path) {
    self.record_failure(provider);
    // Only persist on circuit state transitions (not every failure)
    if self.state_changed_since_last_persist(provider) {
        let _ = self.save(path);
    }
}
```

### Step 4: Load at startup

```rust
// In runner/serve startup:
let health_path = workdir.join(".roko/learn/provider-health.json");
let health_tracker = ProviderHealthTracker::load(&health_path)
    .unwrap_or_else(|_| ProviderHealthTracker::new());
```

## Write Scope
- `crates/roko-learn/src/provider_health.rs` (ProviderHealthSnapshot, save/load)
- `crates/roko-cli/src/runner/event_loop.rs` (load at startup, save on transitions)

## Read-Only Context
- `crates/roko-learn/src/provider_health.rs` (ProviderHealth, ProviderHealthTracker)


## Verify
```bash
cargo build --workspace 2>&1 | head -30
cargo test --workspace 2>&1 | tail -20
```
## Acceptance Criteria
- Provider health persisted to `.roko/learn/provider-health.json`
- State survives process restarts
- Only persisted on circuit state transitions (not every call)
- Stale health data (>1h) marked as expired on load
- Missing file → fresh tracker (no crash)

## Do NOT
- Change the in-memory ProviderHealth struct
- Serialize `Instant` values (convert to timestamps)
- Persist on every success (too frequent)
