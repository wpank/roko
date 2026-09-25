# Gate Ratcheting -- Monotonic Progress Tracking

> Depth file for [07-GATES.md](../../07-GATES.md) section 7.
> Source: `crates/roko-gate/src/ratchet.rs`

---

## 1. Overview

The `GateRatchet` prevents verification regression. Once a plan passes
rung N, it should never regress to rung N-1. The ratchet tracks the
highest rung each plan has passed and provides a `can_regress()` check
that the conductor uses before accepting a lower verdict.

---

## 2. The Thrashing Problem

Without a ratchet, an agent can oscillate indefinitely:

```
Attempt 1: Compile PASS, Lint FAIL
  Agent receives: "warning: unused variable"
  Agent fixes lint issue

Attempt 2: Compile FAIL, (Lint/Test never run)
  Agent's lint fix introduced a type error
  Agent receives: "error[E0308]: mismatched types"
  Agent fixes type error

Attempt 3: Compile PASS, Lint FAIL
  Agent's type fix reintroduced the lint issue
  ... (infinite loop)
```

Each attempt passes one rung and fails the next. The agent is doing work
but making no *progress*. The net verification state oscillates between
"compiles but doesn't lint" and "lints but doesn't compile."

The ratchet breaks this cycle: "You passed Compile on Attempt 1. You are
not allowed to regress below Compile on Attempt 2." If Attempt 2 fails
compile, the ratchet flags regression and the system can:

- Reject the attempt outright
- Flag it for human review
- Give the agent a targeted prompt: "Your previous attempt passed compile.
  Your new attempt broke compile. Fix without regressing."

---

## 3. Data Structure

```rust
pub struct GateRatchet {
    passes: HashMap<String, u8>,  // plan_id -> highest rung passed
}
```

A map from plan identifier to the highest rung number (u8) that plan has
passed. The rung number corresponds to the `Rung` enum's discriminant (0-6).

### Why u8

Seven rungs fit in 3 bits. Using `u8` is the natural Rust choice for a
small non-negative integer, avoids enum overhead in a `HashMap`, and allows
simple comparison operators (`>`, `>=`).

---

## 4. Operations

### 4.1 Record a Pass

```rust
pub fn record_pass(&mut self, plan_id: impl Into<String>, rung: u8) {
    let entry = self.passes.entry(plan_id.into()).or_insert(0);
    if rung > *entry {
        *entry = rung;
    }
}
```

Records that `plan_id` passed `rung`. Only advances the watermark -- if
the plan has already passed a higher rung, this is a no-op.

**Monotonic property:** The stored value for any plan ID can only increase
or stay the same. It never decreases. This is the core ratchet invariant.

### 4.2 Query Highest Pass

```rust
pub fn highest_pass(&self, plan_id: &str) -> Option<u8> {
    self.passes.get(plan_id).copied()
}
```

Returns the highest rung passed, or `None` if no recorded passes. `None`
means the ratchet has no opinion -- the plan is free to pass or fail any
rung.

### 4.3 Check for Regression

```rust
pub fn can_regress(&self, plan_id: &str, rung: u8) -> bool {
    match self.passes.get(plan_id) {
        None => true,                    // Unknown plan: no regression possible
        Some(&highest) => rung >= highest, // OK if same or higher
    }
}
```

Returns `false` if accepting `rung` as the new highest would be a
regression. Returns `true` if no regression would occur.

### 4.4 Plan Count and Clear

```rust
pub fn plan_count(&self) -> usize { self.passes.len() }
pub fn clear(&mut self) { self.passes.clear(); }
```

`clear()` resets entirely, used when starting a fresh execution session.

---

## 5. Usage Pattern in the Orchestrator

```rust
let rung = verdict_rung;
let plan_id = &task.plan_id;

if verdict.passed {
    ratchet.record_pass(plan_id, rung);
} else {
    if !ratchet.can_regress(plan_id, rung) {
        // Regression detected!
        // Options: reject, feed back, trigger re-planning
    }
}
```

---

## 6. Ratchet + Escalation Interaction

| Mechanism | Direction | Purpose |
|-----------|-----------|---------|
| Escalation | Forward (adds rungs) | Failed -> try harder |
| Ratchet | Backward (blocks regression) | Passed -> do not lose progress |

Together they create a monotonically advancing verification frontier:

```
Attempt 1: Complexity=Simple, Rungs=[Compile, Lint]
  -> Compile PASS (ratchet records rung 0)
  -> Lint FAIL
  -> Escalate to Standard

Attempt 2: Complexity=Standard, Rungs=[Compile, Lint, Test, Symbol]
  -> Compile must still pass (ratchet enforces)
  -> Lint PASS (ratchet records rung 1)
  -> Test FAIL
  -> Escalate to Complex

Attempt 3: Complexity=Complex, Rungs=[all]
  -> Compile must still pass (ratchet enforces)
  -> Lint must still pass (ratchet enforces)
  -> Test PASS (ratchet records rung 2)
  -> ... and so on
```

Each attempt can only move the verification frontier forward.

---

## 7. Per-Plan Isolation

Each plan has its own ratchet entry. Plan A's progress has no effect on
Plan B's ratchet. Plans are independent units of work, may be at different
verification stages, and one plan's failure does not constrain another.

The `HashMap<String, u8>` keying on plan ID provides this isolation.

---

## 8. Edge Cases

### 8.1 Rung 0 Ratchet

If a plan passes Rung 0 (Compile), `highest = 0`. On the next attempt,
`can_regress("plan", 0)` returns `true` (rung 0 >= 0). There is no rung
below 0. The ratchet's `record_pass()` only fires on success; a failure
at Rung 0 does not call `record_pass()`, so the stored value stays at 0.

### 8.2 Full Pipeline Pass

When all 7 rungs pass, `highest_pass` = 6. Any subsequent failure at any
rung is a regression (all rungs 0-5 are below 6). Only passing rung 6
again is non-regressive.

### 8.3 Unknown Plans

Plans not in the ratchet (`highest_pass` returns `None`) have no
constraints. `can_regress` returns `true` for any rung.

---

## 9. Ratchet in Process Reward Context

The ratchet provides raw data for the process reward model:

- **Promise score:** How likely is this plan to eventually pass all rungs,
  given it has passed rung N so far?
- **Progress score:** Is the plan advancing (reaching higher rungs on
  successive attempts) or stalling?

A plan that reaches Rung 3 (Compile + Lint + Test) has demonstrated more
progress than one at Rung 1 (Compile only).

---

## 10. Persistence

The current ratchet is in-memory and ephemeral. For resumable executions,
the ratchet state is part of the executor snapshot:

```json
// .roko/state/gate-ratchet.json
{
  "plan-42": 3,
  "plan-43": 1,
  "plan-44": 5
}
```

Loaded on `--resume` alongside the executor snapshot.

---

## 11. Testing

| Test | Property |
|------|----------|
| `ratchet_new_is_empty` | Default state has no entries |
| `ratchet_record_and_query` | Basic store/query roundtrip |
| `ratchet_only_advances` | Lower rung does not overwrite higher |
| `ratchet_can_regress_prevents_regression` | Detects regression correctly |
| `ratchet_can_regress_allows_same_or_higher` | Non-regression permitted |
| `ratchet_can_regress_unknown_plan_returns_true` | No constraint on unknown plans |
| `ratchet_multiple_plans_independent` | Per-plan isolation |
| `ratchet_clear_resets_all` | Clear removes all entries |
| `ratchet_record_pass_zero_rung` | Edge case: rung 0 |
| `ratchet_monotonic_sequence` | Rungs 0->6 all recorded correctly |
| `ratchet_default_is_new` | `Default` trait works |

---

## 12. Correctness by Construction

The `GateRatchet` is correct by construction. There are no complex
algorithms, no statistical models, no heuristics. Just: the highest rung
you have passed is the floor you cannot drop below. A `HashMap<String, u8>`
with monotonic updates.

---

## Verification

```bash
cargo test -p roko-gate -- ratchet
```
