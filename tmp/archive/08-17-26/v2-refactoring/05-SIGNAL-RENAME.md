# Phase 1B: Signal Rename (from Engram)

> **Status: DONE (2026-08-12)**
>
> The rename is complete. The Rust struct is now `pub struct Signal` in `engram.rs` (file
> retains old name). `pub type Engram = Signal` backward-compat alias is retained for
> downstream crate compilation. New code uses `Signal` everywhere. The `balance: f64`
> field (QW-3) was also added during this work.
>
> **What remains:** Mark `Engram` alias as `#[deprecated]`. Update remaining crate-level
> references that still use `Engram` directly (via the alias). Cosmetic: rename
> `engram.rs` file to `signal.rs`.
>
> **Last updated: 2026-08-13**

## What Changed

`Signal` is now both the public API name and the underlying struct name (renamed from
Engram in 2026-08-12). This aligns with the v2 spec where Signal is the universal durable datum.

## Current State

- `roko-core/src/engram.rs`: defines `pub struct Signal` + `pub type Engram = Signal` (backward-compat alias)
- `roko-core/src/signal.rs`: re-exports `Signal`, `SignalBuilder`
- `balance: f64` field added with `#[serde(default)]` for backwards compatibility
- New code across the codebase uses `Signal`; some older crate code still references `Engram` via the alias

## Strategy: Gradual, Non-Breaking

### Step 1: Flip the canonical direction (1 hour)

In `engram.rs`:
```rust
// Before
pub struct Engram { ... }

// After
pub struct Signal { ... }
```

In `signal.rs` or `lib.rs`:
```rust
// Before
pub type Signal = Engram;

// After
#[deprecated(note = "Use Signal instead")]
pub type Engram = Signal;
```

### Step 2: Update roko-core internals (1 hour)

Change all `Engram` references within roko-core to `Signal`. The deprecated alias
means external crates still compile.

### Step 3: Update dependent crates (2-3 hours, can be incremental)

For each crate that uses `Engram`:
```bash
grep -rn 'Engram' crates/ --include='*.rs' | grep -v target/ | grep -v 'type Engram'
```
Replace with `Signal`. This can be done crate-by-crate over multiple PRs.

### Step 4: Add `balance` field (part of QW-3)

While renaming, also add the demurrage balance field:

```rust
pub struct Signal {
    // ... existing fields ...
    /// Demurrage balance. Starts at 1.0, decays over time, refreshed on access.
    /// Signals with balance below pruning threshold are eligible for GC.
    #[serde(default = "default_balance")]
    pub balance: f64,
}

fn default_balance() -> f64 { 1.0 }
```

## Wire Target

Everything that uses `Signal` (or the backward-compat `Engram` alias) -- this was a rename, not new functionality.

## Verification

```bash
cargo build --workspace  # Deprecated alias ensures compilation
cargo test --workspace   # All tests pass
```

## Files to Change

| File | Change |
|------|--------|
| `crates/roko-core/src/engram.rs` | Rename struct |
| `crates/roko-core/src/signal.rs` or `lib.rs` | Flip alias direction |
| `crates/roko-core/src/lib.rs` | Export Signal as primary |
| All crates using `Engram` alias | Incremental migration to `Signal` (non-breaking) |
