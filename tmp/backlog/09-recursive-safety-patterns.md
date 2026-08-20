# 09 — Recursive Safety Continuous Monitoring

**Priority**: P3 — background safety monitoring for meta-agent lineages; no impact on current operation since roko has no deployed meta-agent lineages today
**Size**: L (4-5 days)
**Crates**: `crates/roko-agent/` (anomaly types + scan), `crates/roko-core/` (config schema + dashboard event), `crates/roko-serve/` (background monitor task), `crates/roko-cli/` (runner integration), `crates/roko-runtime/` (ProcessSupervisor API)
**Depends on**: None

---

## Background

Roko supports "meta-agents" — agents that can spawn other agents, creating recursive lineages. The R04 implementation in `crates/roko-agent/src/safety/recursive.rs` enforces strict per-activation invariants: non-widening delegation (child authority is always a subset of the parent's), a maximum delegation depth (`MAX_META_AGENT_DEPTH = 16`), a maximum fanout (`MAX_META_AGENT_FANOUT = 64`), a maximum lineage cost (`MAX_META_AGENT_LINEAGE_COST_USD = 10,000`), and a five-head corrigibility graph that must unanimously approve every activation. These checks happen at each agent creation or role-morph boundary.

What does not exist is a continuous, cross-lineage monitor that runs asynchronously during execution. Per-activation checks verify a single point in time; they cannot detect emergent patterns that only become visible across multiple agents over time — a child-creation rate accelerating toward the fanout limit, a progressive quality decline across generations, or a circular dependency formed through indirect lineage edges.

This item adds that continuous monitoring layer: a background task that periodically snapshots active meta-agent lineages, runs a pure anomaly-detection scan, and emits structured events when it finds problems. The per-activation safety contracts in `recursive.rs` are unchanged and remain the primary enforcement layer; this item is an observability and early-warning overlay on top of them.

## Current State

1. `RecursiveSafetyMonitor` is defined at line 182 of `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/safety/recursive.rs`. It is a zero-sized struct (`pub struct RecursiveSafetyMonitor;`) with two async methods: `validate_activation()` (line 187) and `validate_action()` (line 201). Both are point-in-time validators. There is no `scan()` method and no `SafetyAnomaly` type.

2. The lifecycle constants in `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/lifecycle.rs` are: `MAX_META_AGENT_DEPTH = 16` (line 408), `MAX_META_AGENT_FANOUT = 64` (line 410), `MAX_META_AGENT_RETRIES = 16` (line 412), `MAX_META_AGENT_LINEAGE_COST_USD = 10_000.0` (line 414). These are referenced directly from `recursive.rs` at import line 18.

3. `ProcessSupervisor` is defined in `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/process.rs` at line 839. It has a `handles: Arc<Mutex<HashMap<ProcessId, ProcessHandle>>>` field. It does not currently expose a method to enumerate live processes by meta-agent lineage or to send a stop signal to a specific named process.

4. `DashboardEvent` enum in `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/dashboard_snapshot.rs` (line 77) has many variants but no `RecursiveSafetyAnomaly` variant. Adding one requires also adding an arm to `DashboardSnapshot::apply_with_ts()` at line 1078.

5. `DashboardSnapshot` struct (line 915) has no field for tracking recursive safety anomaly history. It has a `diagnoses: VecDeque<DiagnosisSummary>` (line 926) ring buffer that could serve as a precedent for a safety anomaly ring buffer.

6. `RokoConfig` in `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/schema.rs` (line 89) has no `meta` or `recursive_safety` section. Adding one requires creating a new config struct and adding a field to `RokoConfig`.

7. The serve runtime at `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/runtime.rs` is a thin CLI-abstraction trait with no ongoing background tasks related to recursive safety.

8. The two existing test cases in `recursive.rs` (lines 259-321) cover `validate_delegation()` rejection and the corrigibility graph veto. They do not test any monitoring or anomaly-detection functionality.

## Implementation Plan

### Step 1: Define anomaly types in `recursive.rs`

Add to `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/safety/recursive.rs` after the existing type definitions (after line 94):

```rust
/// A snapshot of one active meta-agent lineage at a point in time.
/// Populated from `ProcessSupervisor` and agent health records.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecursiveProcess {
    /// Stable identifier for this meta-agent (not the ProcessId).
    pub meta_agent_id: String,
    /// Identifier of the parent meta-agent, if any.
    pub parent_id: Option<String>,
    /// Number of child agents created total.
    pub total_children_created: u32,
    /// Number of child agents created in the last hour.
    pub creations_this_hour: u32,
    /// Configured maximum children per hour for this agent.
    pub max_creations_per_hour: u32,
    /// Per-generation quality scores (index 0 = oldest generation).
    pub generation_quality_scores: Vec<f64>,
}

/// A detected anomaly in the recursive safety monitoring scan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SafetyAnomaly {
    /// A single meta-agent's child-creation rate exceeds its configured limit.
    RateLimitViolation {
        meta_agent_id: String,
        rate: u32,
        limit: u32,
    },
    /// Quality scores across generations trend significantly downward.
    QualityDegradation {
        meta_agent_id: String,
        generation: u32,
        quality_trend: Vec<f64>,
        /// Linear regression slope (negative = degrading).
        slope: f64,
    },
    /// Two or more agents form a cycle in the parent-child graph.
    CircularDependency { agents: Vec<String> },
    /// The sum of all meta-agent creation rates exceeds the global limit.
    GlobalRateExceeded { current_rate: u32, limit: u32 },
}

/// Recommended action for a detected anomaly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SafetyAction {
    /// Record the anomaly in the audit log only.
    Log,
    /// Pause the offending meta-agent until operator acknowledgment.
    Pause,
    /// Quarantine the agent and its children from further spawning.
    Quarantine,
    /// Terminate the agent.
    Terminate,
}
```

### Step 2: Implement `RecursiveSafetyMonitor::scan()` and `recommend_action()`

Add to the `impl RecursiveSafetyMonitor` block in `recursive.rs` (after line 229):

```rust
impl RecursiveSafetyMonitor {
    // ... existing methods ...

    /// Configuration for the continuous monitor.
    pub const DEFAULT_QUALITY_TREND_WINDOW: usize = 10;
    pub const DEFAULT_MIN_QUALITY_SLOPE: f64 = -0.05;
    pub const DEFAULT_GLOBAL_MAX_RATE_PER_HOUR: u32 = 50;

    /// Scan a snapshot of active recursive processes for anomalies.
    ///
    /// This is a pure function over a borrowed snapshot — it does not modify
    /// any state and can be called from any async context. The caller is
    /// responsible for constructing the snapshot from `ProcessSupervisor`.
    pub fn scan(
        &self,
        processes: &[RecursiveProcess],
        quality_trend_window: usize,
        min_quality_slope: f64,
        global_max_rate_per_hour: u32,
    ) -> Vec<SafetyAnomaly> {
        let mut anomalies = Vec::new();

        // Check per-agent rate limits.
        for process in processes {
            if process.creations_this_hour > process.max_creations_per_hour {
                anomalies.push(SafetyAnomaly::RateLimitViolation {
                    meta_agent_id: process.meta_agent_id.clone(),
                    rate: process.creations_this_hour,
                    limit: process.max_creations_per_hour,
                });
            }
        }

        // Check global rate across all agents.
        let total_rate: u32 = processes.iter().map(|p| p.creations_this_hour).sum();
        if total_rate > global_max_rate_per_hour {
            anomalies.push(SafetyAnomaly::GlobalRateExceeded {
                current_rate: total_rate,
                limit: global_max_rate_per_hour,
            });
        }

        // Check quality degradation trends.
        for process in processes {
            let scores = &process.generation_quality_scores;
            if scores.len() >= quality_trend_window.max(2) {
                let window = &scores[scores.len().saturating_sub(quality_trend_window)..];
                let slope = linear_slope(window);
                if slope < min_quality_slope {
                    anomalies.push(SafetyAnomaly::QualityDegradation {
                        meta_agent_id: process.meta_agent_id.clone(),
                        generation: scores.len() as u32,
                        quality_trend: window.to_vec(),
                        slope,
                    });
                }
            }
        }

        // Check for circular dependencies via DFS.
        if let Some(cycle) = detect_cycle(processes) {
            anomalies.push(SafetyAnomaly::CircularDependency { agents: cycle });
        }

        anomalies
    }

    /// Map an anomaly to its recommended action under the default policy.
    #[must_use]
    pub fn recommend_action(&self, anomaly: &SafetyAnomaly) -> SafetyAction {
        match anomaly {
            SafetyAnomaly::RateLimitViolation { .. } => SafetyAction::Pause,
            SafetyAnomaly::QualityDegradation { .. } => SafetyAction::Log,
            SafetyAnomaly::CircularDependency { .. } => SafetyAction::Quarantine,
            SafetyAnomaly::GlobalRateExceeded { .. } => SafetyAction::Pause,
        }
    }
}

/// Compute the linear regression slope over a slice of f64 values.
/// Returns 0.0 for slices shorter than 2 elements.
fn linear_slope(values: &[f64]) -> f64 {
    let n = values.len();
    if n < 2 {
        return 0.0;
    }
    let n_f = n as f64;
    let sum_x: f64 = (0..n).map(|i| i as f64).sum();
    let sum_y: f64 = values.iter().sum();
    let sum_xy: f64 = values.iter().enumerate().map(|(i, y)| i as f64 * y).sum();
    let sum_xx: f64 = (0..n).map(|i| (i as f64).powi(2)).sum();
    let denom = n_f * sum_xx - sum_x * sum_x;
    if denom.abs() < f64::EPSILON {
        return 0.0;
    }
    (n_f * sum_xy - sum_x * sum_y) / denom
}

/// DFS cycle detection on the parent-child lineage graph.
/// Returns `Some(Vec<agent_id>)` with the cycle members, or `None` if no cycle.
fn detect_cycle(processes: &[RecursiveProcess]) -> Option<Vec<String>> {
    use std::collections::{HashMap, HashSet};

    // Build adjacency: parent_id -> child_ids
    let mut children: HashMap<&str, Vec<&str>> = HashMap::new();
    for p in processes {
        if let Some(ref parent) = p.parent_id {
            children
                .entry(parent.as_str())
                .or_default()
                .push(p.meta_agent_id.as_str());
        }
    }

    let mut visited = HashSet::new();
    let mut stack = Vec::new();

    for p in processes {
        if !visited.contains(p.meta_agent_id.as_str()) {
            if dfs_cycle(
                p.meta_agent_id.as_str(),
                &children,
                &mut visited,
                &mut stack,
            ) {
                return Some(stack.into_iter().map(String::from).collect());
            }
        }
    }
    None
}

fn dfs_cycle<'a>(
    node: &'a str,
    children: &HashMap<&'a str, Vec<&'a str>>,
    visited: &mut std::collections::HashSet<&'a str>,
    stack: &mut Vec<&'a str>,
) -> bool {
    visited.insert(node);
    stack.push(node);
    if let Some(neighbors) = children.get(node) {
        for &child in neighbors {
            if stack.contains(&child) {
                stack.push(child); // include the cycle endpoint
                return true;
            }
            if !visited.contains(child)
                && dfs_cycle(child, children, visited, stack)
            {
                return true;
            }
        }
    }
    stack.pop();
    false
}
```

### Step 3: Add `MetaSafetyConfig` to the config schema

Create a new field in `RokoConfig` in `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/schema.rs`. First, define the config struct (can be placed in `crates/roko-core/src/config/agent.rs` alongside other agent config, or inline in `schema.rs`):

```rust
/// Configuration for the continuous recursive safety monitor.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MetaSafetyConfig {
    /// Number of generations to include in the quality trend window.
    #[serde(default = "MetaSafetyConfig::default_quality_trend_window")]
    pub quality_trend_window: usize,
    /// Minimum slope (negative) to flag as degradation.
    #[serde(default = "MetaSafetyConfig::default_min_quality_slope")]
    pub min_quality_slope: f64,
    /// Whether to detect circular lineage dependencies.
    #[serde(default = "MetaSafetyConfig::default_circular_detection")]
    pub circular_detection: bool,
    /// Whether to automatically execute the recommended action on anomaly detection.
    #[serde(default)]
    pub auto_pause_on_anomaly: bool,
    /// System-wide maximum hourly meta-agent creation rate.
    #[serde(default = "MetaSafetyConfig::default_global_max_rate_per_hour")]
    pub global_max_rate_per_hour: u32,
    /// How often the monitor polls in seconds.
    #[serde(default = "MetaSafetyConfig::default_monitor_interval_secs")]
    pub monitor_interval_secs: u64,
}

impl Default for MetaSafetyConfig {
    fn default() -> Self {
        Self {
            quality_trend_window: Self::default_quality_trend_window(),
            min_quality_slope: Self::default_min_quality_slope(),
            circular_detection: Self::default_circular_detection(),
            auto_pause_on_anomaly: false,
            global_max_rate_per_hour: Self::default_global_max_rate_per_hour(),
            monitor_interval_secs: Self::default_monitor_interval_secs(),
        }
    }
}

impl MetaSafetyConfig {
    fn default_quality_trend_window() -> usize { 10 }
    fn default_min_quality_slope() -> f64 { -0.05 }
    fn default_circular_detection() -> bool { true }
    fn default_global_max_rate_per_hour() -> u32 { 50 }
    fn default_monitor_interval_secs() -> u64 { 30 }
}
```

Add to `RokoConfig` (after the `resources` field, line 175):

```rust
/// Continuous recursive safety monitor configuration.
#[serde(default)]
pub meta_safety: MetaSafetyConfig,
```

The corresponding `roko.toml` section:

```toml
[meta_safety]
quality_trend_window     = 10
min_quality_slope        = -0.05
circular_detection       = true
auto_pause_on_anomaly    = false
global_max_rate_per_hour = 50
monitor_interval_secs    = 30
```

### Step 4: Add `DashboardEvent::RecursiveSafetyAnomaly` to `dashboard_snapshot.rs`

Add a new variant to `DashboardEvent` in `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/dashboard_snapshot.rs` (after the last existing variant, before the closing brace):

```rust
/// A recursive safety anomaly was detected by the continuous monitor.
RecursiveSafetyAnomaly {
    /// Stable identifier for the meta-agent involved (if applicable).
    #[serde(default)]
    meta_agent_id: String,
    /// Human-readable anomaly description.
    description: String,
    /// Recommended action: "log", "pause", "quarantine", or "terminate".
    recommended_action: String,
    /// ISO 8601 timestamp when the anomaly was detected.
    detected_at: String,
},
```

Add a `recursive_safety_anomalies: VecDeque<RecursiveSafetyAnomalyEntry>` field to `DashboardSnapshot` (after `diagnoses`, using ring size 50), and add the corresponding `AnomalyEntry` type:

```rust
/// One anomaly entry stored in the dashboard snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecursiveSafetyAnomalyEntry {
    pub meta_agent_id: String,
    pub description: String,
    pub recommended_action: String,
    pub detected_at: String,
    pub ts: u64,
}
```

Add the `apply_with_ts` arm for the new variant:

```rust
DashboardEvent::RecursiveSafetyAnomaly {
    meta_agent_id,
    description,
    recommended_action,
    detected_at,
} => {
    let entry = RecursiveSafetyAnomalyEntry {
        meta_agent_id: meta_agent_id.clone(),
        description: description.clone(),
        recommended_action: recommended_action.clone(),
        detected_at: detected_at.clone(),
        ts,
    };
    if self.recursive_safety_anomalies.len() >= 50 {
        self.recursive_safety_anomalies.pop_front();
    }
    self.recursive_safety_anomalies.push_back(entry);
}
```

### Step 5: Add the background monitor task in `roko-serve`

Create a new function in `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/runtime.rs` (or a new file `crates/roko-serve/src/recursive_safety_monitor.rs`):

```rust
use std::sync::Arc;
use roko_agent::safety::recursive::{RecursiveSafetyMonitor, RecursiveProcess, SafetyAction};
use roko_core::config::MetaSafetyConfig;
use roko_core::dashboard_snapshot::DashboardEvent;
use chrono::Utc;
use tokio::time::{Duration, interval};
use tracing::{info, warn};

/// Spawn the background recursive-safety monitor task.
///
/// The monitor polls at `config.monitor_interval_secs`, scans the active
/// process snapshot for anomalies, and publishes `DashboardEvent::RecursiveSafetyAnomaly`
/// for each detected anomaly.
pub fn spawn_recursive_safety_monitor(
    config: MetaSafetyConfig,
    // StateHub sender for publishing DashboardEvents.
    hub_sender: /* StateHub sender type */,
    // Optional JSONL path for audit logging.
    audit_path: Option<std::path::PathBuf>,
    cancel: tokio_util::sync::CancellationToken,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let monitor = RecursiveSafetyMonitor;
        let mut ticker = interval(Duration::from_secs(config.monitor_interval_secs));
        loop {
            tokio::select! {
                _ = cancel.cancelled() => break,
                _ = ticker.tick() => {}
            }

            // In a real integration, enumerate active meta-agent lineages
            // from the ProcessSupervisor. Currently returns empty because
            // no meta-agent lineages are tracked yet.
            let processes: Vec<RecursiveProcess> = collect_active_lineages();

            let anomalies = monitor.scan(
                &processes,
                config.quality_trend_window,
                config.min_quality_slope,
                config.global_max_rate_per_hour,
            );

            for anomaly in &anomalies {
                let action = monitor.recommend_action(anomaly);
                let description = format!("{anomaly:?}");
                let recommended_action = format!("{action:?}").to_lowercase();

                info!(
                    anomaly = %description,
                    action = %recommended_action,
                    "recursive safety monitor detected anomaly"
                );

                let event = DashboardEvent::RecursiveSafetyAnomaly {
                    meta_agent_id: anomaly_agent_id(anomaly),
                    description: description.clone(),
                    recommended_action: recommended_action.clone(),
                    detected_at: Utc::now().to_rfc3339(),
                };
                // Publish via hub_sender (exact API depends on StateHub type).
                let _ = hub_sender.send(event);

                // Append to JSONL audit log.
                if let Some(ref path) = audit_path {
                    append_anomaly_to_jsonl(path, &description, &recommended_action);
                }
            }
        }
    })
}

fn collect_active_lineages() -> Vec<RecursiveProcess> {
    // TODO: wire to ProcessSupervisor when meta-agent lineage tracking is added.
    Vec::new()
}

fn anomaly_agent_id(anomaly: &roko_agent::safety::recursive::SafetyAnomaly) -> String {
    use roko_agent::safety::recursive::SafetyAnomaly::*;
    match anomaly {
        RateLimitViolation { meta_agent_id, .. } => meta_agent_id.clone(),
        QualityDegradation { meta_agent_id, .. } => meta_agent_id.clone(),
        CircularDependency { agents } => agents.first().cloned().unwrap_or_default(),
        GlobalRateExceeded { .. } => "system".to_string(),
    }
}

fn append_anomaly_to_jsonl(
    path: &std::path::Path,
    description: &str,
    action: &str,
) {
    use std::io::Write;
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let entry = serde_json::json!({
            "ts": chrono::Utc::now().to_rfc3339(),
            "description": description,
            "recommended_action": action,
        });
        let _ = writeln!(file, "{entry}");
    }
}
```

### Step 6: Add unit tests

Add to `#[cfg(test)]` in `recursive.rs` (after line 321):

```rust
#[test]
fn scan_detects_rate_limit_violation() {
    let monitor = RecursiveSafetyMonitor;
    let processes = vec![RecursiveProcess {
        meta_agent_id: "agent-1".to_string(),
        parent_id: None,
        total_children_created: 30,
        creations_this_hour: 25,
        max_creations_per_hour: 20,
        generation_quality_scores: vec![],
    }];
    let anomalies = monitor.scan(&processes, 10, -0.05, 100);
    assert!(anomalies.iter().any(|a| matches!(a, SafetyAnomaly::RateLimitViolation { .. })));
}

#[test]
fn scan_detects_global_rate_exceeded() {
    let monitor = RecursiveSafetyMonitor;
    let processes = vec![
        RecursiveProcess {
            meta_agent_id: "agent-1".to_string(),
            parent_id: None,
            total_children_created: 0,
            creations_this_hour: 30,
            max_creations_per_hour: 100,
            generation_quality_scores: vec![],
        },
        RecursiveProcess {
            meta_agent_id: "agent-2".to_string(),
            parent_id: Some("agent-1".to_string()),
            total_children_created: 0,
            creations_this_hour: 30,
            max_creations_per_hour: 100,
            generation_quality_scores: vec![],
        },
    ];
    // Global limit = 50; total_rate = 60
    let anomalies = monitor.scan(&processes, 10, -0.05, 50);
    assert!(anomalies.iter().any(|a| matches!(a, SafetyAnomaly::GlobalRateExceeded { .. })));
}

#[test]
fn scan_detects_quality_degradation() {
    let monitor = RecursiveSafetyMonitor;
    // 10 generations declining from 0.9 to 0.5 → slope ≈ -0.044
    let scores: Vec<f64> = (0..10).map(|i| 0.9 - (i as f64) * 0.044).collect();
    let processes = vec![RecursiveProcess {
        meta_agent_id: "agent-1".to_string(),
        parent_id: None,
        total_children_created: 10,
        creations_this_hour: 1,
        max_creations_per_hour: 20,
        generation_quality_scores: scores,
    }];
    let anomalies = monitor.scan(&processes, 10, -0.03, 100);
    assert!(anomalies.iter().any(|a| matches!(a, SafetyAnomaly::QualityDegradation { .. })));
}

#[test]
fn scan_detects_circular_dependency() {
    let monitor = RecursiveSafetyMonitor;
    let processes = vec![
        RecursiveProcess {
            meta_agent_id: "agent-A".to_string(),
            parent_id: Some("agent-B".to_string()),
            total_children_created: 0,
            creations_this_hour: 0,
            max_creations_per_hour: 10,
            generation_quality_scores: vec![],
        },
        RecursiveProcess {
            meta_agent_id: "agent-B".to_string(),
            parent_id: Some("agent-A".to_string()),
            total_children_created: 0,
            creations_this_hour: 0,
            max_creations_per_hour: 10,
            generation_quality_scores: vec![],
        },
    ];
    let anomalies = monitor.scan(&processes, 10, -0.05, 100);
    assert!(anomalies.iter().any(|a| matches!(a, SafetyAnomaly::CircularDependency { .. })));
}

#[test]
fn scan_clean_processes_produces_no_anomalies() {
    let monitor = RecursiveSafetyMonitor;
    let processes = vec![RecursiveProcess {
        meta_agent_id: "agent-1".to_string(),
        parent_id: None,
        total_children_created: 5,
        creations_this_hour: 2,
        max_creations_per_hour: 20,
        generation_quality_scores: vec![0.8, 0.82, 0.81, 0.83],
    }];
    let anomalies = monitor.scan(&processes, 10, -0.05, 100);
    assert!(anomalies.is_empty());
}

#[test]
fn valid_dag_produces_no_circular_dependency() {
    let monitor = RecursiveSafetyMonitor;
    // A → B → C (no cycle)
    let processes = vec![
        RecursiveProcess {
            meta_agent_id: "A".to_string(),
            parent_id: None,
            total_children_created: 0, creations_this_hour: 0,
            max_creations_per_hour: 10, generation_quality_scores: vec![],
        },
        RecursiveProcess {
            meta_agent_id: "B".to_string(),
            parent_id: Some("A".to_string()),
            total_children_created: 0, creations_this_hour: 0,
            max_creations_per_hour: 10, generation_quality_scores: vec![],
        },
        RecursiveProcess {
            meta_agent_id: "C".to_string(),
            parent_id: Some("B".to_string()),
            total_children_created: 0, creations_this_hour: 0,
            max_creations_per_hour: 10, generation_quality_scores: vec![],
        },
    ];
    let anomalies = monitor.scan(&processes, 10, -0.05, 100);
    assert!(!anomalies.iter().any(|a| matches!(a, SafetyAnomaly::CircularDependency { .. })));
}
```

## Acceptance Criteria

1. `RecursiveSafetyMonitor::scan()` correctly identifies `RateLimitViolation` when `creations_this_hour > max_creations_per_hour` for any process in the snapshot.

2. `scan()` correctly identifies `GlobalRateExceeded` when the sum of all `creations_this_hour` values exceeds `global_max_rate_per_hour`.

3. `scan()` correctly identifies `QualityDegradation` when the per-generation quality score series has a negative slope steeper than `min_quality_slope` over the configured `quality_trend_window`.

4. `scan()` detects `CircularDependency` when the parent-child graph extracted from `RecursiveProcess.parent_id` fields contains a cycle; returns no `CircularDependency` for valid DAGs.

5. `scan()` returns an empty `Vec` for a healthy set of processes (no rate violations, positive quality trend, acyclic graph).

6. All six existing `RecursiveSafetyMonitor` tests in `recursive.rs` continue to pass without modification.

7. All six new unit tests (Steps 1-4 above) pass.

8. The background monitor task in `roko-serve` compiles, spawns without panicking, and emits a `DashboardEvent::RecursiveSafetyAnomaly` when `scan()` returns a non-empty result.

9. `MetaSafetyConfig` deserializes correctly from a `roko.toml` section with all fields specified, and from an empty config (all defaults).

10. `cargo test --workspace` passes with no regressions.

## Verification Checklist

- [ ] `cargo test -p roko-agent -- recursive` passes all 8 tests (2 existing + 6 new)
- [ ] `cargo test -p roko-core -- meta_safety` passes (config schema deserialization)
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes clean
- [ ] `cargo +nightly fmt --all` produces no diff
- [ ] Add a `[meta_safety]` section to `roko.toml` with `monitor_interval_secs = 5`; start `roko serve`; confirm no panics at startup; confirm the monitor task is spawned (log: "recursive safety monitor" span)
- [ ] Manually construct a test with two processes forming a cycle; confirm `DashboardEvent::RecursiveSafetyAnomaly` is published via `roko serve`'s SSE stream (`curl http://localhost:6677/api/events`)
- [ ] Confirm the audit JSONL at `.roko/learn/recursive-safety.jsonl` receives entries when anomalies are detected
- [ ] Confirm that with `auto_pause_on_anomaly = false` (default), no agents are stopped when anomalies are detected

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/safety/recursive.rs` | Add `RecursiveProcess`, `SafetyAnomaly`, `SafetyAction` types; add `scan()`, `recommend_action()` to `RecursiveSafetyMonitor`; add `linear_slope()` and `detect_cycle()` helpers; add 6 unit tests |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/schema.rs` | Add `MetaSafetyConfig` struct and `meta_safety: MetaSafetyConfig` field to `RokoConfig` |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/dashboard_snapshot.rs` | Add `DashboardEvent::RecursiveSafetyAnomaly` variant; add `RecursiveSafetyAnomalyEntry` type; add `recursive_safety_anomalies: VecDeque<RecursiveSafetyAnomalyEntry>` to `DashboardSnapshot`; add `apply_with_ts` arm |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/runtime.rs` (or new `recursive_safety_monitor.rs`) | Add `spawn_recursive_safety_monitor()` function; wire to serve startup |
