# Depth 13-05: Circuit Breaker Controls

> Three-stage overhead circuit breaker, budget enforcement, and Lens
> degradation protocol.

**Parent**: [13-TELEMETRY](../../13-TELEMETRY.md) -- Section 6

---

## 1. Design Principle

Telemetry is observational -- it must never slow down the observed
system. The `LensCircuitBreaker` enforces this invariant by tracking
consecutive overhead-budget violations and progressively degrading
Lens execution from full fidelity to complete disablement.

**Source**: `crates/roko-core/src/lens_circuit_breaker.rs`

---

## 2. Three Stages

```rust
pub enum LensBreakerStage {
    Active,    // Full fidelity: every event is processed
    Sampled,   // Degraded: events are selectively skipped
    Disabled,  // Off: no events are processed
}
```

### 2.1 Stage Transitions

```
Active ─── (sample_threshold consecutive violations) ──> Sampled
Sampled ── (disable_threshold consecutive violations) ─> Disabled
Disabled ─ (manual reset()) ─────────────────────────> Sampled
```

Transitions are monotonically degrading during violations. Recovery
from `Disabled` requires an explicit `reset()` call, which returns
the breaker to `Sampled` (not `Active`) -- the Lens must prove it
can stay within budget before being fully restored.

There is no automatic recovery from `Sampled` to `Active`. The
breaker clears the consecutive violation counter when an event
completes within budget, but does not promote the stage. This
prevents oscillation between `Active` and `Sampled`.

### 2.2 Default Thresholds

```rust
impl Default for LensCircuitBreaker {
    fn default() -> Self {
        Self::new(0.01)  // 1% overhead budget
    }
}
```

| Parameter | Default | Meaning |
|-----------|---------|---------|
| `budget_pct` | 0.01 (1%) | Max Lens time as fraction of Cell time |
| `sample_threshold` | 3 | Consecutive violations to enter Sampled |
| `disable_threshold` | 10 | Consecutive violations to enter Disabled |

Thresholds are configurable at construction via `with_thresholds()`:

```rust
let breaker = LensCircuitBreaker::new(0.02)
    .with_thresholds(5, 20);
```

---

## 3. Budget Check Protocol

The `check` method is called after each Lens observation with the
Lens execution time and the observed Cell/operation duration:

```rust
pub fn check(
    &mut self,
    lens_duration_ms: u64,
    cell_duration_ms: u64,
) -> LensBreakerAction
```

### 3.1 Budget Calculation

```
budget_ms = cell_duration_ms * budget_pct
```

If `lens_duration_ms > budget_ms`, the violation counter increments.
Otherwise, the counter resets to zero (the Lens is back within
budget).

### 3.2 Actions

The return value tells the Lens runtime what to do:

```rust
pub enum LensBreakerAction {
    Allow,    // Process the event normally
    Skip,     // Skip this event (Sampled stage)
    Disable,  // Shut down this Lens (Disabled stage)
}
```

| Stage | Action |
|-------|--------|
| `Active` | `Allow` |
| `Sampled` | `Skip` |
| `Disabled` | `Disable` |

### 3.3 Violation Logging

The first violation in a consecutive sequence logs a warning:

```
WARN Lens exceeded overhead budget
  lens_duration_ms=12 cell_duration_ms=100
```

Subsequent violations in the same sequence are silent to avoid log
flooding.

---

## 4. The `should_invoke` Guard

Before invoking a Lens at all, the runtime can check:

```rust
pub const fn should_invoke(&self) -> bool {
    !matches!(self.stage, LensBreakerStage::Disabled)
}
```

This is a fast constant-time check that avoids even calling the Lens's
observe method when it has been disabled. Both `Active` and `Sampled`
stages return `true` -- the `Sampled` stage still invokes the Lens
but skips the event after observing (to maintain the violation
tracking).

---

## 5. Reset Protocol

```rust
pub fn reset(&mut self) {
    self.stage = LensBreakerStage::Sampled;
    self.consecutive_violations = 0;
}
```

Reset always transitions to `Sampled`, never directly to `Active`.
The consecutive violation counter is cleared but the total violation
counter is preserved for diagnostics.

---

## 6. Diagnostic State

Two accessors expose internal state for telemetry-about-telemetry:

```rust
pub const fn stage(&self) -> LensBreakerStage { self.stage }
pub const fn total_violations(&self) -> u64 { self.total_violations }
```

The `total_violations` counter uses `saturating_add` so it cannot
overflow in any realistic runtime.

---

## 7. Duration-Bearing Events

The overhead budget only applies to events that close an operation
and carry a `duration_ms` field. The five duration-bearing events are:

1. `CellCompleted { duration_ms, .. }`
2. `GraphNodeCompleted { duration_ms, .. }`
3. `GraphCompleted { duration_ms, .. }`
4. `MemoryRetrieved { duration_ms, .. }`
5. `ExtensionHookCalled { duration_ms, .. }`

Events without a duration (start events, transitions, markers) return
`None` from `observed_duration_ms()` and bypass the overhead check
entirely -- the breaker cannot compute a meaningful budget without a
denominator.

---

## 8. Per-Lens Isolation

Each Lens has its own `LensCircuitBreaker` instance. A slow Lens does
not affect other Lenses. The runtime invokes breakers independently
and can disable one Lens while others continue at full fidelity.

---

## 9. Interaction with Bounded Delivery

The circuit breaker is the last line of defense. Before the breaker,
the telemetry runtime applies bounded delivery with drop-oldest
accounting: if the delivery queue is full, the oldest event is dropped
and a counter incremented. The breaker handles the case where events
are delivered but the Lens itself is too slow to process them within
the overhead budget.

---

## 10. Source References

| File | What it contains |
|------|------------------|
| `crates/roko-core/src/lens_circuit_breaker.rs` | `LensCircuitBreaker`, `LensBreakerStage`, `LensBreakerAction` |
| `crates/roko-core/src/telemetry_observe.rs` | `observed_duration_ms()` on `ObservableEvent` |

---

## Verification

```bash
# Circuit breaker tests
cargo test -p roko-core -- lens_circuit_breaker

# Duration-bearing events
grep -n 'observed_duration_ms' crates/roko-core/src/telemetry_observe.rs
```
