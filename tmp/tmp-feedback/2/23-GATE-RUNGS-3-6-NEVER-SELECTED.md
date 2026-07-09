# Gate Rungs 3-6 Never Selected

## Problem

The gate pipeline has 7 rungs (0-6) with increasing strictness, but rungs 3-6 are never
selected even when `enable_advanced_rungs = true` is set in config. The rung selection
logic has a bug where both branches of the condition skip advanced rungs.

Additionally, `VerifyChainGate` always returns stub pass.

## Root Cause

### A. Rung selection logic bug

**File:** `crates/roko-cli/src/orchestrate.rs`

In `enrich_rung_config()`:
```rust
fn select_rung(task: &Task, config: &GateConfig) -> u8 {
    if config.enable_advanced_rungs {
        // BUG: this branch still caps at rung 2
        match task.priority {
            Priority::Critical => 2,
            Priority::High => 1,
            Priority::Normal => 0,
            _ => 0,
        }
    } else {
        match task.priority {
            Priority::Critical => 2,
            Priority::High => 1,
            _ => 0,
        }
    }
}
```

Both the `if` and `else` branches return the same values (0, 1, 2). When
`enable_advanced_rungs` is true, it should return higher rungs:

### B. VerifyChainGate stub

**File:** `crates/roko-gate/src/gates/verify_chain.rs`

```rust
impl Gate for VerifyChainGate {
    async fn check(&self, _artifact: &Artifact) -> GateResult {
        GateResult::Pass  // ← always passes, no chain verification
    }
}
```

This gate is supposed to verify that an artifact has a valid chain witness (cryptographic
proof of provenance). It always passes because the chain runtime isn't integrated yet.

### C. EMA adaptive thresholds work correctly

The adaptive threshold system (EMA per rung) is correctly wired and functioning. It adjusts
pass/fail thresholds based on historical gate outcomes. This is one of the few learning
subsystems that works end-to-end.

## Fix

### Fix 1: Fix rung selection for advanced rungs (~5 min)

**File:** `crates/roko-cli/src/orchestrate.rs`

```rust
fn select_rung(task: &Task, config: &GateConfig) -> u8 {
    if config.enable_advanced_rungs {
        match task.priority {
            Priority::Critical => 6,  // Full pipeline: compile+test+clippy+diff+lint+chain+review
            Priority::High => 4,      // compile+test+clippy+diff+lint
            Priority::Normal => 2,    // compile+test+clippy
            _ => 1,                   // compile+test
        }
    } else {
        match task.priority {
            Priority::Critical => 2,
            Priority::High => 1,
            _ => 0,
        }
    }
}
```

### Fix 2: Mark VerifyChainGate as Phase 2 (~2 min)

**File:** `crates/roko-gate/src/gates/verify_chain.rs`

Add a log warning when the gate is invoked:
```rust
async fn check(&self, _artifact: &Artifact) -> GateResult {
    tracing::warn!("VerifyChainGate: stub pass (chain runtime not integrated)");
    GateResult::Pass
}
```

## Files to Modify

| File | Change |
|------|--------|
| `crates/roko-cli/src/orchestrate.rs` | Fix rung selection for advanced rungs |
| `crates/roko-gate/src/gates/verify_chain.rs` | Add warning log for stub |

## Priority

**P1** — Gate strictness is supposed to scale with task priority. Currently every task gets
the same weak validation (rungs 0-2) regardless of config. A critical refactor task gets the
same gates as a documentation update.
