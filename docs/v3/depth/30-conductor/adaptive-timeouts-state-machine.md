# Adaptive Timeouts and the State Machine

> Depth file for [30-CONDUCTOR.md](../../30-CONDUCTOR.md) section 11.
> Source: `crates/roko-conductor/src/state_machine.rs`,
>         `crates/roko-learn/src/latency.rs`

---

## 1. The State Machine

Each plan progresses through a well-defined set of phases:

```
Queued -> Implementing -> Gating -> Reviewing -> Done -> Merging -> Complete
                           |         |
                           v         v
                       AutoFixing  (re-implement)
                           |
                           v
                       (back to Gating)
```

Invalid transitions are structurally impossible. A plan cannot jump from Queued to
Reviewing. The state machine is a DATA STRUCTURE, not code paths.

This is Hard Guarantee 1: "Explicit State Machine with Compile-Time Transition
Validation."

---

## 2. Phase Timeouts

Every phase has a hard wall-clock timeout. When the timeout fires, the plan
transitions to Failed. No exceptions.

```rust
pub fn phase_timeout(phase: PlanPhase, complexity: Complexity) -> Duration {
    match (phase, complexity) {
        (Implementing, Complex)  => Duration::from_secs(600),  // 10 min
        (Implementing, Standard) => Duration::from_secs(300),  // 5 min
        (Implementing, Fast)     => Duration::from_secs(120),  // 2 min

        (Gating, _)              => Duration::from_secs(300),  // 5 min
        (Reviewing, _)           => Duration::from_secs(300),  // 5 min
        (Merging, _)             => Duration::from_secs(60),   // 1 min
    }
}
```

### 2.1 Why Hard Timeouts

Soft timeouts (conductor detects timeout -> decides whether to intervene) do not
work. Production experience demonstrated the nudge loop: conductor detects timeout
-> nudges agent -> agent ignores nudge -> conductor detects again -> 10 minutes
wasted.

Hard timeouts are enforced by the state machine:

```rust
for plan in active_plans {
    let elapsed = plan.phase_entered_at.elapsed();
    let timeout = phase_timeout(plan.phase, plan.complexity);
    if elapsed > timeout {
        transition(plan, Failed(Timeout));  // HARD. No negotiation.
    }
}
```

This is Hard Guarantee 2: "Every Phase Has a Hard Timeout."

### 2.2 Complexity-Based Scaling

| Complexity | Typical Plans | Implementation Timeout |
|-----------|--------------|----------------------|
| Fast | Typo fixes, const additions, doc updates | 120s (2 min) |
| Standard | Function implementations, module additions | 300s (5 min) |
| Complex | Multi-crate features, architectural changes | 600s (10 min) |

Other phases (Gating, Reviewing, Merging) do not scale with complexity because
their duration depends on codebase size and test suite speed, not plan complexity.

---

## 3. PhaseTransition Records

Every phase transition produces an audit record:

```rust
pub struct PhaseTransition {
    pub plan_id: String,
    pub from: PlanPhase,
    pub to: PlanPhase,
    pub timestamp: String,   // ISO 8601
    pub reason: String,
}
```

Example audit trail:

```
plan-42: Queued -> Implementing    (2026-04-09T10:00:00Z, "dependencies met")
plan-42: Implementing -> Gating    (2026-04-09T10:03:22Z, "all tasks complete")
plan-42: Gating -> Implementing    (2026-04-09T10:04:15Z, "gate failed: 2 errors")
plan-42: Implementing -> Gating    (2026-04-09T10:06:48Z, "all tasks complete")
plan-42: Gating -> Reviewing       (2026-04-09T10:07:30Z, "all gates passed")
plan-42: Reviewing -> Merging      (2026-04-09T10:08:45Z, "review approved")
plan-42: Merging -> Complete       (2026-04-09T10:09:02Z, "merge successful")
```

This enables post-mortem analysis, performance optimization, anomaly detection, and
learning system input.

---

## 4. Adaptive Timeout Computation

### 4.1 P95-Based Adaptive Timeout

```rust
impl LatencyStats {
    /// Recommended timeout = 2x the observed p95 latency,
    /// clamped to [5s, 300s].
    pub fn adaptive_timeout_ms(&self) -> u64 {
        if self.observations < 10 { return 120_000; }  // Not enough data
        let p95 = self.p95_ms();
        let timeout = (p95 * 2.0) as u64;
        timeout.clamp(5_000, 300_000)
    }
}
```

With enough observations, the timeout automatically adjusts:

- Complex plans consistently finishing in 4 minutes -> timeout settles at ~8 min
- Model upgrades making agents faster -> timeout tightens
- Codebase growth slowing compilation -> timeout widens

### 4.2 Cold Start Behavior

With fewer than 10 observations, the system uses the static default (120 seconds).
This prevents the adaptive system from setting unreasonable timeouts based on small,
unrepresentative samples.

### 4.3 Per-Phase Adaptive Timeouts

| Phase | Duration Determined By |
|-------|----------------------|
| Implementing | Agent reasoning speed, codebase complexity |
| Gating | Compile time, test suite size |
| Reviewing | Reviewer model speed, number of reviewers |
| Merging | Git merge speed, post-merge test time |

Each phase has its own adaptive timeout from its own observation window.

---

## 5. TTFT Timeout

Time-to-first-token timeout provides early detection of stalled providers:

```
Request sent
    |
    | <- connect_timeout_ms (5s): TCP connection must be established
    |
    | <- ttft_timeout_ms (15s): first token must arrive
    |
    | <- timeout_ms (120s): complete response must arrive
    |
Response received
```

Each layer catches a different failure mode:
- Connection timeout -> DNS failure, firewall, provider down
- TTFT timeout -> provider overloaded, queue backed up
- Full timeout -> response generation taking too long

---

## 6. Graceful Shutdown Sequence

Four-phase shutdown on Ctrl+C or budget exhaustion:

```
Phase 1: Stop Accepting (immediate)
    +-- Set accepting_spawns = false

Phase 2: Drain Active (30s timeout)
    +-- Send SIGTERM to all Running processes
    +-- Wait for clean exit

Phase 3: Force Kill (5s)
    +-- kill_all_descendants() for remaining processes

Phase 4: Checkpoint and Flush (2s)
    +-- Write final state to disk
    +-- Flush logs
    +-- Exit
```

### 6.1 Atomic Checkpoint Writes

Temp-file-then-rename prevents corruption from mid-write crashes:

```rust
fn save_snapshot_atomic(snapshot: &Snapshot, path: &Path) -> Result<()> {
    let tmp = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(snapshot)?;
    std::fs::write(&tmp, &json)?;
    std::fs::rename(&tmp, path)?;  // Atomic on POSIX
    Ok(())
}
```

---

## 7. Timeout Layering

Phase timeouts in the Conductor complement process-level supervision:

| Layer | Timeout Type | What It Catches |
|-------|-------------|----------------|
| Process (roko-runtime) | Process timeout | Agent process hangs |
| Task (Conductor) | Phase timeout | Task too long in any phase |
| Plan (Conductor) | Wall-clock limit | Total plan execution exceeds limit |
| Batch (Orchestrator) | Budget limit | Total batch cost exceeds limit |

Each layer catches problems at a different granularity.

---

## 8. File Reference

| File | What |
|------|------|
| `crates/roko-conductor/src/state_machine.rs` | Phase timeouts, PhaseTransition |
| `crates/roko-learn/src/latency.rs` | LatencyStats, adaptive_timeout_ms() |
| `crates/roko-core/src/config/schema.rs` | ProviderConfig with timeout fields |
