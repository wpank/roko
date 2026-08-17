# M007 — Type alias: Scorer → ScoreProtocol in roko-core

## Objective
Add `ScoreProtocol` as an alias for the existing `Scorer` trait per unified vocabulary.

## Scope
- Crates: `roko-core`
- Files: `crates/roko-core/src/traits.rs`, `crates/roko-core/src/lib.rs`
- Phase ref: 02-PHASE-1-KERNEL.md §1.1

## Steps
1. Find the Scorer trait:
   `grep -rn 'pub trait Scorer' crates/roko-core/src/ --include='*.rs'`

2. Add alias near the trait definition or in lib.rs:
   ```rust
   pub use traits::Scorer as ScoreProtocol;
   ```

3. Re-export from lib.rs if not already visible.

## Verification
```bash
cargo check -p roko-core
cargo clippy -p roko-core --no-deps -- -D warnings
```

## What NOT to do
- Do NOT rename the trait — just alias
- Do NOT change method signatures or implementations
