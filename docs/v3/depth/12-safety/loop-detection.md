# Loop Detection and Non-Widening Validation

> **v3 depth file** -- `/docs/v3/depth/12-safety/loop-detection.md`
> Canonical source: v1 `docs/v1/11-safety/05-loop-detection.md`
> Status: **Shipping**. Rate limiter, circuit breaker, ghost turn detection,
> secret scrubbing, and adaptive gate thresholds are live. Zeroize-on-drop
> for `TaintedString` is target-state.

---

## 1. The Loop Problem

An autonomous agent can enter infinite or near-infinite loops through several mechanisms:

**Tool call loops.** The agent calls a tool, the result triggers another call, which
triggers another. Example: a bash command fails, the agent retries with a slightly
modified command, which also fails, triggering another retry.

**Reasoning loops.** The agent enters a circular pattern: consider option A, reject for
B, reject B for C, reject C for A. The LLM sees the same context and produces the same
cycling pattern.

**Escalation loops.** The agent encounters a permission error, attempts to work around it,
encounters another error, and spirals into increasingly creative (and dangerous) attempts.

**Resource exhaustion.** A loop that individually seems harmless but cumulatively exhausts
token budget, API rate limits, or disk space.

---

## 2. Rate Limiting as Loop Defense

The primary loop defense is the sliding-window rate limiter in
`roko-agent/src/safety/rate_limit.rs`:

```rust
pub struct RateLimiter {
    policy: RateLimitPolicy,
    state: Mutex<HashMap<RateLimitKey, VecDeque<Instant>>>,
}

pub struct RateLimitPolicy {
    pub max_calls_per_window: usize,    // Default: 60
    pub window_duration: Duration,       // Default: 60s
}
```

The implementation is `Send + Sync` with a single-lock critical section.
`check_and_record()` atomically checks the cap and records the timestamp under one
mutex acquisition, preventing TOCTOU races.

A legitimate agent making rapid parallel reads approaches the limit but rarely exceeds
it. An agent stuck in a retry loop exhausts the budget for that tool.

---

## 3. Circuit Breaker Pattern

The `roko-conductor` crate implements a circuit breaker that monitors agent health:

**States:**
- **Closed** (normal): all tool calls proceed.
- **Half-open** (testing): limited calls allowed to test recovery.
- **Open** (broken): tool calls rejected, agent paused.

**Triggers for opening:**
- Gate failure rate exceeds threshold (default 50% over last 10 tasks).
- Error rate on tool calls exceeds threshold (default 30% over last 20 calls).
- Session duration exceeds maximum.
- Drawdown metric exceeds safety threshold (chain-domain agents).

When the circuit opens, the orchestrator pauses the current task and records the
intervention as a Signal with `Kind::InterventionReceived`.

---

## 4. Diagnosis Engine

The `DiagnosisEngine` performs root cause analysis when the circuit breaker triggers:

1. **Tail signal analysis** -- examines the last N signals (default 200) for patterns.
2. **Ghost turn detection** -- identifies turns with no meaningful output.
3. **Efficiency degradation** -- compares recent metrics against historical baselines.
4. **Phase stuck detection** -- identifies when the agent has been in the same phase too long.

The diagnosis produces a `ConductorDecision`:

| Decision | Action |
|---|---|
| Continue | Normal operation |
| Pause | Wait for human review |
| Retry | Retry with fresh agent instance |
| Skip | Mark task as failed, move to next |
| Abort | Abort entire plan execution |

---

## 5. Ghost Turn Detection

A ghost turn is a turn that produces no meaningful output:

1. **Empty turn** -- zero content or whitespace only.
2. **Repeat turn** -- identical or near-identical to previous turn (edit distance).
3. **Failure-only turn** -- all tool calls failed with the same error class.
4. **No-progress turn** -- tokens produced per token consumed below minimum threshold.

Ghost turns are tracked per task. When they exceed threshold (default: 3 consecutive),
the conductor transitions Closed to Half-Open, Half-Open to Open.

### Liveness enforcement

Ghost turn detection enforces the liveness property:

```
G(task_started -> F(task_completed | task_failed))
    "Every started task eventually completes or fails"
```

Without detection, a looping agent violates this: the task is started but never
completes. The conductor's intervention ensures liveness by forcing Skip or Abort.

---

## 6. Non-Widening Validation

The non-widening invariant: no action in the safety pipeline can increase an agent's
capability beyond what was initially granted.

This applies at multiple levels:

- **Warrant delegation** -- `delegate()` produces strictly weaker sub-warrants.
- **Sandbox levels** -- monotonic from None to Quarantine; escalation only.
- **Taint levels** -- monotonic from Trusted to Untrusted; join only goes up.
- **Capability intersection** -- Cell x Graph x Space is conjunctive; no layer widens another.
- **R04 meta-agent lineage** -- authority is non-widening across activation/morph/rollback.

---

## 7. Secret Scrubbing

`ScrubPolicy` in `roko-agent/src/safety/scrub.rs` applies regex-based secret scrubbing
on tool output before it enters the LLM context:

```rust
pub fn scrub_secrets(content: &str, policy: &ScrubPolicy) -> String {
    let mut result = content.to_string();
    if !policy.disable_defaults {
        for pattern in default_patterns() {
            result = apply_pattern(&result, pattern);
        }
    }
    for raw in &policy.extra_patterns {
        let Ok(re) = Regex::new(raw) else { continue; };
        let extra = Pattern { re, replace_group: None };
        result = apply_pattern(&result, &extra);
    }
    result
}
```

The scrubber is pure -- it allocates a new `String` and never mutates shared state.
Default patterns cover API keys, AWS credentials, private keys, and bearer tokens.

### Zeroize-on-drop (target-state)

The `zeroize` crate can provide memory-level protection via `Zeroizing<String>`:
when the value is dropped, every byte is overwritten with zeros using volatile writes.
This prevents secrets from persisting in memory after use.

---

## 8. Adaptive Gate Thresholds

The adaptive gate system in `roko-learn` creates a self-regulating feedback loop:

- Gate pass rates are tracked per rung using EMA.
- When EMA drops below threshold, the gate tightens (higher confidence required).
- When EMA rises above threshold, the gate loosens.
- Thresholds persist to `.roko/learn/gate-thresholds.json` and load on restart.

An agent that repeatedly fails gates gets harder gates. An agent that consistently
passes earns slightly looser thresholds.

---

## 9. Integration with Efficiency Events

The efficiency event stream (`.roko/learn/efficiency.jsonl`) provides raw data for
ghost turn detection:

```json
{
  "task_id": "T-42",
  "turn": 7,
  "tokens_in": 1250,
  "tokens_out": 340,
  "tool_calls": 2,
  "tool_failures": 0,
  "duration_ms": 8200
}
```

The DiagnosisEngine aggregates these events to detect efficiency degradation over
time, complementing per-turn ghost detection with trend-based anomaly detection.

---

## 10. Tool Cooldown and Isolation

E34 added tool cooldown and isolation controls:

- **Cooldown** -- after repeated failures, a tool is placed in a cooldown period.
  Further calls are rejected until the cooldown expires.
- **Isolation** -- provider-specific tool results are isolated. A tool result from
  one provider session cannot influence another provider's safety state.

These controls prevent cascading failures where one tool's repeated errors cause
the agent to retry in increasingly dangerous ways.

---

## Academic References

| Paper | Contribution |
|---|---|
| Nygard (2018) | Release It! -- circuit breaker pattern |
| Vaucher et al. (2018) | Zeroization patterns for sensitive data |
| Barthe et al. (2014) | Verified security of crypto implementations |

---

## Implementation References

| Component | Location |
|---|---|
| RateLimiter | `crates/roko-agent/src/safety/rate_limit.rs` |
| ScrubPolicy | `crates/roko-agent/src/safety/scrub.rs` |
| DiagnosisEngine | `crates/roko-conductor/` |
| Efficiency events | `.roko/learn/efficiency.jsonl` |
| Gate thresholds | `.roko/learn/gate-thresholds.json` |
| ProcessSupervisor | `crates/roko-runtime/src/process.rs` |
