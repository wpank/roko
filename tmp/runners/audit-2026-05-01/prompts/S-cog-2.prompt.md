# S-cog-2: Implement FailureTracker in roko-learn

## Task
Implement a small `FailureTracker` (~200-500 LOC) in `crates/roko-learn/src/failure_tracker.rs` that replaces the parts of daimon actually used by orchestration / runner.

## Runner Context
Runner audit-2026-05-01, group S. Depends on S-cog-1 (inventory). Wave 2.

## Source plan
`tmp/subsystem-audits/implementation-plans/31-cognitive-layer-cleanup.md` § CL-2.

## Read first

Open `tmp/runners/audit-2026-05-01/logs/S-cog-1-inventory.md`. Read the daimon caller list to understand what API surface `FailureTracker` must provide.

## Exact changes

### `crates/roko-learn/src/failure_tracker.rs` (new)

```rust
//! Tracks recent agent failures by role / task kind. Provides retry-strategy
//! suggestions and surfaces alerts when consecutive failures exceed a
//! threshold.
//!
//! Replaces the parts of `daimon` that orchestration actually uses.
//! See tmp/runners/audit-2026-05-01/logs/S-cog-1-inventory.md for the
//! original use cases.

use std::collections::{HashMap, VecDeque};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

const RECENT_WINDOW: usize = 50;
const ESCALATE_AFTER: u32 = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureRecord {
    pub role: String,
    pub task_kind: String,
    pub model: String,
    pub error_class: ErrorClass,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ErrorClass {
    GateFailed,
    ModelError,
    Timeout,
    Cancelled,
    SafetyDenied,
    Unknown,
}

#[derive(Debug, Default)]
pub struct FailureTracker {
    by_role: HashMap<String, RoleFailureStats>,
    by_task_kind: HashMap<String, TaskKindFailureStats>,
    pending_alerts: Vec<Alert>,
}

#[derive(Debug, Default)]
pub struct RoleFailureStats {
    pub recent: VecDeque<FailureRecord>,
    pub consecutive: u32,
    pub last_seen: Option<DateTime<Utc>>,
}

#[derive(Debug, Default)]
pub struct TaskKindFailureStats {
    pub recent: VecDeque<FailureRecord>,
    pub error_class_counts: HashMap<ErrorClass, u32>,
}

#[derive(Debug)]
pub enum RetryStrategy {
    SameModel,
    EscalateModel,
    PauseAndAlert,
}

#[derive(Debug, Clone)]
pub struct Alert {
    pub kind: AlertKind,
    pub role: String,
    pub message: String,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy)]
pub enum AlertKind {
    ConsecutiveFailures,
    ErrorClassCluster,
}

impl FailureTracker {
    pub fn record(&mut self, record: FailureRecord) {
        let role_stats = self.by_role.entry(record.role.clone()).or_default();
        role_stats.recent.push_back(record.clone());
        if role_stats.recent.len() > RECENT_WINDOW {
            role_stats.recent.pop_front();
        }
        role_stats.consecutive += 1;
        role_stats.last_seen = Some(record.at);

        if role_stats.consecutive >= ESCALATE_AFTER {
            self.pending_alerts.push(Alert {
                kind: AlertKind::ConsecutiveFailures,
                role: record.role.clone(),
                message: format!("{} consecutive failures in role {}", role_stats.consecutive, record.role),
                at: record.at,
            });
        }

        let task_stats = self.by_task_kind.entry(record.task_kind.clone()).or_default();
        task_stats.recent.push_back(record.clone());
        if task_stats.recent.len() > RECENT_WINDOW { task_stats.recent.pop_front(); }
        *task_stats.error_class_counts.entry(record.error_class).or_default() += 1;
    }

    pub fn record_success(&mut self, role: &str) {
        if let Some(s) = self.by_role.get_mut(role) {
            s.consecutive = 0;
        }
    }

    pub fn suggest_retry_strategy(&self, role: &str) -> RetryStrategy {
        let s = self.by_role.get(role);
        match s.map(|s| s.consecutive).unwrap_or(0) {
            0 | 1 => RetryStrategy::SameModel,
            2 | 3 => RetryStrategy::EscalateModel,
            _ => RetryStrategy::PauseAndAlert,
        }
    }

    pub fn drain_alerts(&mut self) -> Vec<Alert> {
        std::mem::take(&mut self.pending_alerts)
    }
}
```

### Mount in `lib.rs`

```rust
pub mod failure_tracker;
```

### Tests

```rust
#[test]
fn consecutive_failures_escalate_then_alert() {
    let mut t = FailureTracker::default();
    t.record(rec("impl", "GateFailed"));
    assert!(matches!(t.suggest_retry_strategy("impl"), RetryStrategy::SameModel));
    t.record(rec("impl", "GateFailed"));
    assert!(matches!(t.suggest_retry_strategy("impl"), RetryStrategy::EscalateModel));
    t.record(rec("impl", "GateFailed"));
    let alerts = t.drain_alerts();
    assert_eq!(alerts.len(), 1);
}

#[test]
fn success_resets_consecutive() {
    let mut t = FailureTracker::default();
    t.record(rec("impl", "GateFailed"));
    t.record(rec("impl", "GateFailed"));
    t.record_success("impl");
    assert!(matches!(t.suggest_retry_strategy("impl"), RetryStrategy::SameModel));
}
```

## Write Scope
- `crates/roko-learn/src/failure_tracker.rs` (new)
- `crates/roko-learn/src/lib.rs`

## Read-Only Context
- `tmp/runners/audit-2026-05-01/logs/S-cog-1-inventory.md`

## Verify

```bash
ls crates/roko-learn/src/failure_tracker.rs

rg 'pub struct FailureTracker' crates/roko-learn/src/failure_tracker.rs
# Expect: 1 hit

rg 'failure_tracker' crates/roko-learn/src/lib.rs
# Expect: 1 hit
```

## Do NOT

- Do NOT bundle with S-cog-3/4/5.
- Do NOT exceed ~500 LOC. Simple is the point.
- Do NOT replicate daimon's full API; only what S-cog-1 inventoried as in-use.
- Do NOT add persistence in this batch (FailureTracker is in-memory; persist via existing JSONL infrastructure if needed later).
