# W2-E: Fix Negative Cost Display

**Priority**: P2 — cosmetic
**Effort**: 5 minutes
**Files to modify**: 1 file
**Dependencies**: None

## Problem

`roko status` shows `$-0.0000`. Either unsigned integer underflow or float precision issue with zero-usage models.

## Root Cause

In `crates/roko-cli/src/commands/util.rs`, cost values are `f64` from `CostsLog`. No clamping is applied before formatting. The cost data comes from `.roko/learn/costs.jsonl`.

## Exact Code to Change

### File: `crates/roko-cli/src/commands/util.rs`

### Change 1: Total cost display (line ~759)

```rust
// BEFORE:
if let Some(total_cost_usd) = total_cost_usd {
    println!("  Total:    ${total_cost_usd:.4}");
}

// AFTER:
if let Some(total_cost_usd) = total_cost_usd {
    println!("  Total:    ${:.4}", total_cost_usd.max(0.0));
}
```

### Change 2: Today cost display (line ~762)

```rust
// BEFORE:
if let Some(today_cost_usd) = today_cost_usd {
    println!("  Today:    ${today_cost_usd:.4}");
}

// AFTER:
if let Some(today_cost_usd) = today_cost_usd {
    println!("  Today:    ${:.4}", today_cost_usd.max(0.0));
}
```

### Change 3: format_cost_breakdown (line ~854)

```rust
// BEFORE:
.map(|(name, cost)| format!("{name}=${cost:.4}"))

// AFTER:
.map(|(name, cost)| format!("{name}=${:.4}", cost.max(0.0)))
```

## Agent Prompt

```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W2-E-negative-cost.md and implement all changes described in it. Three .max(0.0) additions in crates/roko-cli/src/commands/util.rs. Do NOT run cargo build/test/clippy/fmt — compilation is deferred. Mark the checklist items as done.
```

## Commit

This batch is committed with all Wave 2 batches together. Do not commit individually.

## Checklist

- [x] Clamp total cost to >= 0.0
- [x] Clamp today cost to >= 0.0
- [x] Clamp per-model costs to >= 0.0 in format_cost_breakdown
- [x] Verify: `roko status` never shows negative
- [x] Pre-commit checks pass
