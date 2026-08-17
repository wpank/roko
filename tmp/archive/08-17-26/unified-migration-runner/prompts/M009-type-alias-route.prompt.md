# M009 — Type alias: Router → RouteProtocol in roko-core

## Objective
Add `RouteProtocol` as an alias for the existing `Router` trait. Also alias
CascadeRouter → CascadeRoute, etc.

## Scope
- Crates: `roko-core`, `roko-learn`
- Files: `crates/roko-core/src/traits.rs`, `crates/roko-core/src/lib.rs`
- Phase ref: 02-PHASE-1-KERNEL.md §1.1

## Steps
1. Find the Router trait and implementations:
   ```bash
   grep -rn 'pub trait Router' crates/roko-core/src/ --include='*.rs'
   grep -rn 'impl.*Router for\|pub struct.*Router' crates/ --include='*.rs' | grep -v target/
   ```

2. Add alias in roko-core:
   ```rust
   pub use traits::Router as RouteProtocol;
   ```

3. Add implementation aliases where Router implementations live:
   ```rust
   // In roko-learn or wherever CascadeRouter is defined:
   pub type CascadeRoute = CascadeRouter;
   ```

## Verification
```bash
cargo check -p roko-core
cargo clippy -p roko-core --no-deps -- -D warnings
```

## What NOT to do
- Do NOT rename structs — just alias
- Do NOT change the cascade router logic or learning persistence
