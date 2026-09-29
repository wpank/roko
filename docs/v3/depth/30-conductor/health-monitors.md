# Health Monitors -- SystemSnapshot and 4 Checks

> Depth file for [30-CONDUCTOR.md](../../30-CONDUCTOR.md) section 7.
> Source: `crates/roko-conductor/src/health.rs`

---

## 1. SystemSnapshot

The health monitor operates on a point-in-time snapshot of system state:

```rust
pub struct SystemSnapshot {
    pub active_agents: usize,
    pub expected_agents: usize,
    pub last_agent_heartbeat_ms: Option<u64>,
    pub chain_connected: bool,
    pub chain_expected: bool,
    pub spec_hash_expected: Option<String>,
    pub spec_hash_actual: Option<String>,
    pub coverage_history: Vec<f64>,
}
```

The snapshot captures infrastructure health, not task health. Task health is the
watcher ensemble's domain. Infrastructure health is about whether the foundation
the tasks run on is solid.

---

## 2. HealthStatus

```rust
pub enum HealthStatus {
    Healthy,
    Degraded,
    Critical,
}
```

**Healthy**: All checks pass. System operating normally.

**Degraded**: One or more non-critical issues. System continues with reduced
capacity. Operator attention recommended.

**Critical**: Fundamental infrastructure problem. System cannot reliably continue.
Operator intervention required.

---

## 3. The Four Checks

### 3.1 Terminal Liveness

**What it checks**: Is the agent process still responsive?

Compares `last_agent_heartbeat_ms` against a liveness threshold. If the most recent
heartbeat is older than the threshold, the terminal is considered unresponsive.

| Condition | Status |
|-----------|--------|
| Heartbeat within threshold (or no agents expected) | Healthy |
| Heartbeat exceeds threshold | Degraded |
| No heartbeat received and agents expected | Critical |

Agent processes can become unresponsive without crashing. The process is alive (PID
exists, no exit code) but the agent has stopped producing output. Without heartbeat
monitoring, this condition is invisible.

### 3.2 Agent Status

**What it checks**: Are the expected number of agents running?

Compares `active_agents` against `expected_agents`.

| Condition | Status |
|-----------|--------|
| `active_agents >= expected_agents` | Healthy |
| `active_agents < expected_agents` | Degraded |
| `active_agents == 0` and `expected_agents > 0` | Critical |

In a batch run with 5 parallel plans, the expected count is 5. If only 3 agents are
active, 2 plans are stalled. This detects the shortfall before the stalled plans'
timeout watchers fire.

**Self-healing trigger**: When Degraded, the orchestrator can proactively respawn
missing agents rather than waiting for affected plans' time-overrun watchers.

### 3.3 Spec Drift

**What it checks**: Has the implementation diverged from its specification?

Compares `spec_hash_expected` against `spec_hash_actual`. If the hashes differ, the
specification has changed or the implementation has drifted.

| Condition | Status |
|-----------|--------|
| Hashes match (or no spec tracking) | Healthy |
| Hashes differ | Degraded |

System-level spec drift is distinct from the spec-drift watcher (which monitors
individual task file scope). The health monitor checks the entire system
specification.

### 3.4 Coverage Trend

**What it checks**: Is test coverage trending down over time?

Examines the `coverage_history` vector. Uses simple regression on the coverage
history. If the slope is negative and the recent average is below the earlier
average by more than a threshold (e.g., 2 percentage points), the status is
Degraded.

| Condition | Status |
|-----------|--------|
| Coverage stable or increasing | Healthy |
| Coverage declining | Degraded |

Coverage drops compound: less-tested code is harder for future agents to modify
correctly, leading to more failures, leading to more corner-cutting, leading to
less coverage. This implements Design Principle 12: "The agent builds the world it
operates in."

---

## 4. HealthMonitor API

```rust
pub struct HealthMonitor {
    // Configuration: thresholds for each check
}

impl HealthMonitor {
    pub fn check(&self, snapshot: &SystemSnapshot) -> HealthStatus {
        let liveness = self.terminal_liveness(snapshot);
        let agents = self.agent_status(snapshot);
        let drift = self.spec_drift(snapshot);
        let coverage = self.coverage_trend(snapshot);

        // Worst status wins
        [liveness, agents, drift, coverage]
            .into_iter()
            .max()
            .unwrap_or(HealthStatus::Healthy)
    }
}
```

Like the intervention policy, the health monitor uses worst-status-wins aggregation.

---

## 5. Health vs. Watcher Ensemble

| Dimension | Health Monitor | Watcher Ensemble |
|-----------|--------------|-----------------|
| **Scope** | System infrastructure | Individual plan/task execution |
| **Input** | SystemSnapshot | Signal stream |
| **Output** | HealthStatus | WatcherOutput (per-watcher severity) |
| **Frequency** | Periodic (every 10 seconds) | Every conductor evaluation |
| **Trigger** | Infrastructure problems | Execution anomalies |

A system can be Healthy (all infrastructure checks pass) while individual plans are
failing. Conversely, all plans can be proceeding normally while the system is
Degraded (an expected agent has died).

The health monitor's Critical status can override watcher-based decisions -- if the
infrastructure is failing, task-level interventions are pointless.

---

## 6. Snapshot Collection

| Field | Source |
|-------|--------|
| `active_agents` | ProcessSupervisor agent count |
| `expected_agents` | Plans in Implementing phase |
| `last_agent_heartbeat_ms` | ProcessSupervisor heartbeat tracker |
| `chain_connected` | Reserved for future chain integration |
| `chain_expected` | Configuration flag |
| `spec_hash_expected` | Plan TOML frontmatter |
| `spec_hash_actual` | Computed from current codebase state |
| `coverage_history` | Gate results over recent builds |

The orchestrator constructs the snapshot periodically (every 10 seconds) and passes
it to the health monitor. The snapshot is a read-only copy -- computing the health
check does not hold any locks.

---

## 7. VSM Mapping

In Beer's Viable System Model (Beer, 1972), the health monitor maps to
**System 3*** (System Three-Star) -- the audit channel:

| VSM Component | Roko Equivalent |
|--------------|----------------|
| System 1 | Individual agents executing tasks |
| System 2 | Conventions, templates, shared protocols |
| System 3 | Orchestrator (internal oversight) |
| **System 3*** | **Health monitor (sporadic audit)** |
| System 4 | Learning system (external adaptation) |
| System 5 | Configuration and policy |

System 3* checks whether System 3's model of reality matches actual reality.

---

## 8. File Reference

| File | What |
|------|------|
| `crates/roko-conductor/src/health.rs` | HealthMonitor, SystemSnapshot, HealthStatus, 4 check methods |
