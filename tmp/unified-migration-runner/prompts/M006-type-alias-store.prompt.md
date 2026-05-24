# M006 — Type alias: Substrate → Store in roko-core

## Objective
Add `pub type Store = Substrate;` and `pub type FileStore = FileSubstrate;` aliases,
making Store the canonical protocol name per the unified spec.

## Scope
- Crates: `roko-core`, `roko-fs`
- Files: `crates/roko-core/src/traits.rs`, `crates/roko-core/src/lib.rs`, `crates/roko-fs/src/lib.rs`
- Phase ref: 02-PHASE-1-KERNEL.md §1.1

## Steps
1. Find the Substrate trait:
   `grep -rn 'pub trait Substrate' crates/roko-core/src/ --include='*.rs'`

2. In the same file (or in a new `store.rs` module), add:
   ```rust
   /// Store protocol — the canonical name for Substrate.
   /// See: tmp/unified/02-CELL.md §2.1
   pub trait StoreProtocol: Substrate {}
   impl<T: Substrate> StoreProtocol for T {}
   ```
   OR simpler (trait alias pattern):
   ```rust
   pub use crate::traits::Substrate as StoreProtocol;
   ```

3. Find FileSubstrate:
   `grep -rn 'pub struct FileSubstrate' crates/ --include='*.rs' | grep -v target/`

4. Add alias in roko-fs:
   ```rust
   pub type FileStore = FileSubstrate;
   ```

5. Re-export from roko-core lib.rs:
   ```rust
   pub use traits::Substrate as StoreProtocol;
   ```

## Verification
```bash
cargo check -p roko-core -p roko-fs
cargo clippy -p roko-core -p roko-fs --no-deps -- -D warnings
```

## What NOT to do
- Do NOT rename the trait itself — just alias
- Do NOT update downstream callers yet
- Do NOT change method signatures
