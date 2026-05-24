# M010 — Type alias: Composer → ComposeProtocol in roko-core

## Objective
Add `ComposeProtocol` as an alias for the existing `Composer` trait.

## Scope
- Crates: `roko-core`, `roko-compose`
- Files: `crates/roko-core/src/traits.rs`, `crates/roko-core/src/lib.rs`
- Phase ref: 02-PHASE-1-KERNEL.md §1.1

## Steps
1. Find the Composer trait:
   `grep -rn 'pub trait Composer' crates/roko-core/src/ --include='*.rs'`

2. Add alias in roko-core:
   ```rust
   pub use traits::Composer as ComposeProtocol;
   ```

3. Re-export from lib.rs.

## Verification
```bash
cargo check -p roko-core -p roko-compose
cargo clippy -p roko-core -p roko-compose --no-deps -- -D warnings
```

## What NOT to do
- Do NOT rename the trait — just alias
- Do NOT change prompt assembly logic or VCG auction path
