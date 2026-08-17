# M012 — Define Cell trait skeleton in roko-core

## Objective
Define the Cell trait as the universal computation unit per the unified spec.
Cell unifies all protocol implementations — every Scorer, Gate, Router, Composer,
and Policy is a Cell. This batch defines the trait skeleton; later batches add
blanket impls.

## Scope
- Crates: `roko-core`
- Files: `crates/roko-core/src/cell.rs`, `crates/roko-core/src/lib.rs`
- Phase ref: 02-PHASE-1-KERNEL.md §1.4

## Steps
1. Read the Cell spec for reference:
   `cat tmp/unified/02-CELL.md` (the Cell trait section)

2. In `crates/roko-core/src/cell.rs` (created in M002), define:
   ```rust
   use std::fmt;

   /// A content-addressed identifier for a Cell.
   #[derive(Debug, Clone, PartialEq, Eq, Hash)]
   pub struct CellId(pub String);

   impl fmt::Display for CellId {
       fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
           write!(f, "{}", self.0)
       }
   }

   /// The universal computation unit.
   ///
   /// Every operator in the system (Scorer, Gate, Router, Composer, Policy)
   /// is a Cell. Cells have typed I/O, protocol conformance declarations,
   /// and cost estimates.
   ///
   /// See: tmp/unified/02-CELL.md
   pub trait Cell: Send + Sync + 'static {
       /// Content-addressed identifier.
       fn id(&self) -> CellId;

       /// Human-readable name.
       fn name(&self) -> &str;

       /// Which protocols this Cell conforms to.
       fn protocols(&self) -> Vec<String> {
           vec![]
       }

       /// Estimated cost per invocation (in microcents).
       fn estimated_cost(&self) -> Option<u64> {
           None
       }
   }
   ```

3. Export from lib.rs:
   ```rust
   pub use cell::{Cell, CellId};
   ```

4. Do NOT add blanket impls for existing traits yet — that's a follow-up batch to
   avoid breaking the existing trait hierarchy in one step.

## Verification
```bash
cargo check -p roko-core
cargo clippy -p roko-core --no-deps -- -D warnings
cargo test -p roko-core --lib --no-run
```

## What NOT to do
- Do NOT add `execute()` method yet — that requires the full CellContext which is Phase 2
- Do NOT add blanket impls (`impl Cell for T where T: Scorer`) yet
- Do NOT add TypeSchema — that's M014
- Do NOT change existing trait definitions
