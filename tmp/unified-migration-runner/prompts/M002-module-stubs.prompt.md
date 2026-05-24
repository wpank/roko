# M002 — Create module stubs for unified type names

## Objective
Create empty module files in roko-core that will hold the unified type names (Signal, Cell).
This is scaffolding — later batches will populate them with type aliases and trait definitions.

## Scope
- Crates: `roko-core`
- Files: `crates/roko-core/src/signal.rs`, `crates/roko-core/src/cell.rs`
- Phase ref: 01-PHASE-0-PREP.md §0.3

## Steps
1. Check if `crates/roko-core/src/signal.rs` already exists:
   `ls crates/roko-core/src/signal.rs 2>/dev/null`
   If it does, skip creating it.

2. Create `crates/roko-core/src/signal.rs`:
   ```rust
   //! Unified Signal type — the universal datum.
   //!
   //! Signal is the canonical name for what was previously called Engram.
   //! This module re-exports the core type under the unified name.
   //!
   //! See: tmp/unified/01-SIGNAL.md
   ```

3. Create `crates/roko-core/src/cell.rs`:
   ```rust
   //! Unified Cell trait — the universal computation unit.
   //!
   //! Cell is the canonical name for atomic computation in the unified model.
   //! Every operator (Scorer, Gate, Router, Composer, Policy) is a Cell.
   //!
   //! See: tmp/unified/02-CELL.md
   ```

4. Add `pub mod signal;` and `pub mod cell;` to `crates/roko-core/src/lib.rs`
   (find the existing `pub mod` block and add at the appropriate alphabetical position).

## Verification
```bash
test -f crates/roko-core/src/signal.rs
test -f crates/roko-core/src/cell.rs
cargo check -p roko-core
```

## What NOT to do
- Do NOT add any types or traits yet — those come in M005 and M012
- Do NOT modify any existing modules
- Do NOT add dependencies
