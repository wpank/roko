# 00-ARCH -- Error Handling and Recovery Patterns

> **Parent**: [00-ARCHITECTURE](../../00-ARCHITECTURE.md)
>
> Every subsystem in Roko can fail. This depth file catalogs the failure modes across
> all subsystems, specifies the recovery strategy for each, defines the error
> propagation rules, the circuit breaker pattern, the graceful degradation ladder,
> and the crash recovery protocol. Updated for the current architecture with Graph as
> the sole engine, E34 safety enforcement, and E42 config evolution.

---

## 1. Error Classification

All errors in Roko fall into four categories:

| Category | Retry? | Escalate? | Examples |
|---|---|---|---|
| **Transient** | Yes (with backoff) | After N retries | Network timeout, rate limit, temporary API error |
| **Deterministic** | No (same input = same failure) | Yes | Compile error, invalid config, schema mismatch |
| **Resource** | Yes (after resource freed) | After timeout | Disk full, memory pressure, too many open files |
| **Catastrophic** | No | Immediate | Data corruption, missing critical files, auth revocation |

```rust
/// Error classification used by the recovery engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorClass {
    Transient,
    Deterministic,
    Resource,
    Catastrophic,
}

impl ErrorClass {
    pub fn should_retry(&self) -> bool {
        matches!(self, Self::Transient | Self::Resource)
    }

    pub fn should_escalate(&self) -> bool {
        matches!(self, Self::Deterministic | Self::Catastrophic)
    }
}
```

---

## 2. Retry Policy

### 2.1 Exponential Backoff with Jitter

Transient errors use truncated exponential backoff with jitter:

```
delay = min(base_ms * 2^attempt + random(0..jitter_ms), max_delay_ms)
```

| Parameter | Default | Range | Description |
|---|---|---|---|
| `base_ms` | 500 | 100-5,000 | Base delay for exponential backoff |
| `max_delay_ms` | 30,000 | 5,000-120,000 | Maximum retry delay |
| `max_retries` | 3 | 0-10 | Maximum retry attempts for transient errors |
| `jitter_ms` | 200 | 0-1,000 | Random jitter added to retry delay |

```rust
pub struct RetryPolicy {
    pub base_ms: u64,
    pub max_delay_ms: u64,
    pub max_retries: u32,
    pub jitter_ms: u64,
}

impl RetryPolicy {
    pub fn delay_for(&self, attempt: u32) -> Duration {
        let exp_delay = self.base_ms.saturating_mul(2u64.saturating_pow(attempt));
        let jitter = rand::thread_rng().gen_range(0..=self.jitter_ms);
        let total = exp_delay.saturating_add(jitter).min(self.max_delay_ms);
        Duration::from_millis(total)
    }
}
```

### 2.2 Rate Limit Handling

When a provider returns HTTP 429 (rate limit), the retry uses the `Retry-After`
header if present, otherwise falls back to exponential backoff:

```rust
fn handle_rate_limit(response: &Response, policy: &RetryPolicy, attempt: u32) -> Duration {
    if let Some(retry_after) = response.header("Retry-After") {
        Duration::from_secs(retry_after.parse().unwrap_or(60))
    } else {
        policy.delay_for(attempt)
    }
}
```

---

## 3. Per-Subsystem Failure Modes

### 3.1 Agent Dispatch (roko-agent)

| Failure mode | Class | Recovery |
|---|---|---|
| LLM API timeout | Transient | Retry with backoff; after 3 failures, route to different provider |
| LLM API 429 (rate limit) | Transient | Use Retry-After header; circuit breaker if persistent |
| LLM API 500 (server error) | Transient | Retry with backoff; circuit breaker after threshold |
| LLM API 401 (auth) | Catastrophic | Halt task; notify user; do not retry |
| Agent process crash | Transient | Restart agent; replay last turn from episode log |
| Agent exceeds max_turns | Deterministic | Stop agent; mark task as failed; escalate to replanning |
| MCP server unavailable | Transient | Retry connection; fall back to non-MCP tools |
| Tool execution timeout | Transient | Kill tool process; retry once; skip tool on second failure |
| Provider health degraded | Transient | Route to alternative provider via health registry |
| Safety contract violation (E34) | Catastrophic | Deny tool use; quarantine; log incident with evidence |

### 3.2 Gate Pipeline (roko-gate)

| Failure mode | Class | Recovery |
|---|---|---|
| Compile failure | Deterministic | Feed error to agent; retry task (new agent turn) |
| Test failure | Deterministic | Feed test output to agent; retry task |
| Clippy failure | Deterministic | Feed warnings to agent; retry task |
| Gate process timeout | Transient | Kill process; retry gate once; skip on second timeout |
| Gate binary not found | Catastrophic | Halt task; log error; skip gate for this run |
| Diff gate: no changes | Deterministic | Pass (no-op task); log warning |
| Adaptive threshold breach | Deterministic | Use EMA-adjusted threshold instead of static |

Gate retries are bounded by `gates.max_iterations` (default 5). After max iterations,
the task is marked as failed and, if `learning.replan_on_gate_failure` is true, the
gate-failure cascade (E44) fires the synchronous cross-cut updates.

### 3.3 Graph Execution (roko-graph)

| Failure mode | Class | Recovery |
|---|---|---|
| Cell execution failure | Depends on cell type | Cell-specific retry; fail the cell, not the graph |
| Cost budget exceeded | Resource | Block new cells; complete running cells; checkpoint state |
| Topology cycle detected | Deterministic | Reject graph; show cycle to user |
| Resume checkpoint corrupt | Resource | Delete checkpoint; restart graph from beginning |
| Wave timeout | Transient | Kill wave; retry or mark timed-out cells as failed |
| Immune graph rejection | Deterministic | Deny output; log five-head evidence; do not retry |

Graph uses resume-durable schema-v2 cost state and graph-fingerprinted Activity
resume. Checkpoints are written atomically.

### 3.4 Substrate / File System (roko-fs)

| Failure mode | Class | Recovery |
|---|---|---|
| JSONL write failure (disk full) | Resource | Buffer in memory (up to 1,000 signals); retry on next prune |
| JSONL read failure (corrupt line) | Deterministic | Skip corrupt line; log warning; continue reading |
| File lock contention | Transient | Retry with backoff; timeout after 10s |
| Missing .roko directory | Resource | Create directory; initialize empty files |
| Signal too large (> 1MB) | Deterministic | Reject signal; log error |
| Rotation-bounded generation overflow | Resource | Prune oldest generation; proceed |

JSONL corruption recovery: the reader skips lines that fail JSON deserialization. Each
line is independent, so one corrupt line does not invalidate the file. The skipped
count is logged.

### 3.5 Learning Subsystem (roko-learn)

| Failure mode | Class | Recovery |
|---|---|---|
| Episode log write failure | Resource | Buffer in memory; retry on next tick |
| Playbook extraction failure | Deterministic | Skip extraction; use existing playbook |
| Bandit state corruption | Deterministic | Reset arm to prior distribution; log warning |
| Experiment store write failure | Resource | Buffer update; retry on next tick |
| Cascade router state corruption | Deterministic | Delete state file; reinitialize from defaults |
| Significance test inconclusive | -- | Continue collecting samples; do not early-stop |

Learning subsystem failures are **non-blocking**. The system continues to function
without learning -- it just does not improve. All learning state files can be deleted
and regenerated.

### 3.6 Prompt Composition (roko-compose)

| Failure mode | Class | Recovery |
|---|---|---|
| Token budget exceeded | Deterministic | Drop lowest-priority sections until within budget |
| Section content missing | Deterministic | Skip section; log warning |
| Template parse error | Catastrophic | Fall back to minimal template (role + task only) |
| Encoding failure | Deterministic | Fall back to raw text (no template formatting) |
| E44 functor error | Transient | Skip the failing functor; proceed with remaining |
| VCG auction deadlock | Transient | Fall back to greedy composition strategy |

Composition failures should never prevent an agent from running. The minimal fallback
prompt is just the role description and task description.

### 3.7 Configuration (roko-core/config)

| Failure mode | Class | Recovery |
|---|---|---|
| Missing roko.toml | Resource | Use `RokoConfig::default()` |
| TOML parse error | Deterministic | Show error with line/column; refuse to start |
| Schema version mismatch | Deterministic | Run migration chain; fail if no path exists |
| Validation warning | -- | Print warning; continue |
| Validation error | Deterministic | Refuse to start; show error |
| Transactional reload failure (E42) | Deterministic | Keep previous config; log error |

---

## 4. Error Propagation Rules

### 4.1 Propagation Hierarchy

```
Agent error
    |
    v
Gate pipeline catches -> retry or fail task
    |
    v
Graph execution catches -> retry cell or fail graph
    |
    v
CLI catches -> show error to user, save state
```

Each layer catches errors from the layer below and decides: retry, escalate, or absorb.

### 4.2 Absorption Rules

Some errors are absorbed (logged but not propagated):

| Error | Absorbed by | Rationale |
|---|---|---|
| Episode log write failure | Learning subsystem | Learning is optional |
| Skill extraction failure | Learning subsystem | Skills improve future tasks, not current |
| Dashboard render failure | TUI | Display errors should not affect execution |
| Metric emission failure | Conductor | Metrics are observability, not core function |
| Dream consolidation failure | Dreams subsystem | Consolidation can be retried next cycle |
| E44 functor short-circuit | Compose subsystem | Enrichment is additive, not required |

### 4.3 Escalation Rules

Some errors escalate immediately:

| Error | Escalated to | Rationale |
|---|---|---|
| Auth failure (401) | User | Cannot be fixed by retry |
| Budget exceeded | Graph executor | Policy decision, not a technical failure |
| State corruption | User | Risk of data loss requires human decision |
| Config parse error | User | Cannot start without valid config |
| Safety contract violation (E34) | Quarantine + incident log | Non-negotiable safety boundary |

---

## 5. Circuit Breaker Pattern

The circuit breaker prevents cascading failures when a provider is degraded:

```rust
pub struct CircuitBreaker {
    state: CircuitState,
    failure_count: u32,
    threshold: u32,           // default: 5
    reset_timeout: Duration,  // default: 300s
    last_failure: Instant,
}

#[derive(Debug, Clone, Copy)]
pub enum CircuitState {
    Closed,   // Normal operation
    Open,     // All requests rejected
    HalfOpen, // One test request allowed
}

impl CircuitBreaker {
    pub fn can_execute(&mut self) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                if self.last_failure.elapsed() >= self.reset_timeout {
                    self.state = CircuitState::HalfOpen;
                    true
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => true,
        }
    }

    pub fn record_success(&mut self) {
        self.failure_count = 0;
        self.state = CircuitState::Closed;
    }

    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.last_failure = Instant::now();
        if self.failure_count >= self.threshold {
            self.state = CircuitState::Open;
        }
    }
}
```

Provider outcome feedback is wired: live workflow attempts plus CLI/bridge outcomes
update one persisted health registry. Unhealthy providers are filtered during learned
routing.

---

## 6. Graceful Degradation Ladder

When resources are constrained, the system degrades through a defined sequence:

```
Level 0: Normal operation
    All features active. Full model selection. All learning loops.

Level 1: Budget pressure (warn_threshold reached)
    Route to cheaper models. Disable experiment exploration. Log warning.

Level 2: Budget critical (block_threshold reached)
    Block new tasks. Complete running tasks. Save state.

Level 3: Provider degraded (circuit breaker open)
    Route to alternative providers. Fall back to local models if available.

Level 4: All providers degraded
    Queue tasks. Retry periodically. Notify user.

Level 5: Disk pressure (disk_warn_mb reached)
    Reduce logging verbosity. Prune aggressively. Warn user.

Level 6: Unrecoverable
    Save state. Print diagnostic. Exit with non-zero code.
```

---

## 7. State Recovery After Crash

If the process crashes, the next run recovers via:

1. **Graph checkpoints**: load from `.roko/state/graph/` (atomic-write protected,
   schema-v2 cost state, graph-fingerprinted Activity resume)
2. **Legacy executor state**: load from `.roko/state/state-snapshot.json` (Runner-v2
   snapshot, used with `--engine legacy`)
3. **Episode log**: read from `.roko/episodes.jsonl` (append-only, skip corrupt lines)
4. **Signal log**: read from `.roko/engrams.jsonl` (append-only, skip corrupt lines)
5. **Learning state**: load from `.roko/learn/*.json` files. Missing files initialize
   to defaults
6. **Agent state**: not recoverable. Running agents are lost on crash. The graph
   executor re-dispatches tasks that were in-progress

State persistence uses atomic writes (write to temp file, then rename) to prevent
corruption:

```rust
fn atomic_write(path: &Path, data: &[u8]) -> io::Result<()> {
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, data)?;
    fs::rename(&tmp, path)?;
    Ok(())
}
```

The `--resume-plan` flag loads the graph checkpoint and skips completed cells:

```bash
roko plan run plans/ --resume-plan
```

---

## 8. Configuration Parameters

| Parameter | Default | Range | Description |
|---|---|---|---|
| `retry_base_ms` | 500 | 100-5,000 | Base delay for exponential backoff |
| `retry_max_delay_ms` | 30,000 | 5,000-120,000 | Maximum retry delay |
| `retry_max_retries` | 3 | 0-10 | Maximum retry attempts |
| `retry_jitter_ms` | 200 | 0-1,000 | Random jitter |
| `circuit_breaker_threshold` | 5 | 1-50 | Failures before circuit opens |
| `circuit_breaker_reset_secs` | 300 | 30-3,600 | Seconds before half-open |
| `state_write_timeout_ms` | 5,000 | 1,000-30,000 | Timeout for state file writes |
| `jsonl_max_corrupt_lines` | 100 | 1-10,000 | Max corrupt lines before rejecting |
| `signal_max_size_bytes` | 1,048,576 | 1,024-10,485,760 | Max single signal size |
| `memory_buffer_max` | 1,000 | 100-100,000 | Max signals buffered during I/O failure |

---

## 9. Test Criteria

1. Transient error retries with exponential backoff (verify delay sequence: 500ms, 1s, 2s)
2. Circuit breaker opens after threshold failures and rejects subsequent requests
3. Circuit breaker transitions to half-open after reset_timeout
4. Successful request in half-open state closes the circuit
5. Atomic write prevents state corruption (kill process mid-write, verify temp file exists)
6. JSONL reader skips corrupt lines and reports count
7. Budget exceeded blocks new tasks but does not kill running agents
8. Auth failure (401) escalates immediately without retry
9. Missing .roko directory is created on first access
10. Full crash recovery: kill process, restart with --resume-plan, verify completed cells skipped
11. Degradation ladder: simulate budget pressure and verify model tier downgrade
12. E44 functor failure does not prevent prompt composition
13. Graph checkpoint resume skips completed cells and retries in-progress cells
14. E34 safety violation triggers quarantine, not retry

---

## Cross-References

- [00-ARCHITECTURE](../../00-ARCHITECTURE.md) -- Parent chapter
- [configuration-schema.md](configuration-schema.md) -- Config parameters
- [cognitive-cross-cuts.md](cognitive-cross-cuts.md) -- E44 gate-failure cascade
- `crates/roko-conductor/` -- Circuit breaker and health monitoring
- `crates/roko-agent/src/dispatcher/mod.rs` -- Agent dispatch with retry logic
- `crates/roko-graph/` -- Graph execution, checkpoints, cost state
- `crates/roko-fs/` -- FileSubstrate error handling
