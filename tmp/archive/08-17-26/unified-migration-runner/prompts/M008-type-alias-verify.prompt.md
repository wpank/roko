# M008 — Type alias: Gate → VerifyProtocol in roko-core

## Objective
Add `VerifyProtocol` as an alias for the existing `Gate` trait. Also alias key
Gate implementations (CompileGate → CompileVerify, TestGate → TestVerify, etc.).

## Scope
- Crates: `roko-core`, `roko-gate`
- Files: `crates/roko-core/src/traits.rs`, `crates/roko-core/src/lib.rs`, `crates/roko-gate/src/lib.rs`
- Phase ref: 02-PHASE-1-KERNEL.md §1.1

## Steps
1. Find the Gate trait and all Gate implementations:
   ```bash
   grep -rn 'pub trait Gate' crates/roko-core/src/ --include='*.rs'
   grep -rn 'impl.*Gate for' crates/roko-gate/src/ --include='*.rs' | grep -v target/
   ```

2. Add alias in roko-core:
   ```rust
   pub use traits::Gate as VerifyProtocol;
   ```

3. In roko-gate, add aliases for each implementation:
   ```rust
   pub use compile_gate::CompileGate as CompileVerify;
   pub use test_gate::TestGate as TestVerify;
   // ... for each gate implementation
   ```
   First find them: `grep -rn 'pub struct.*Gate' crates/roko-gate/src/ --include='*.rs'`

4. Re-export aliases from roko-gate's lib.rs.

## Verification
```bash
cargo check -p roko-core -p roko-gate
cargo clippy -p roko-core -p roko-gate --no-deps -- -D warnings
```

## What NOT to do
- Do NOT rename structs — just alias
- Do NOT change the gate pipeline implementation
- Do NOT modify adaptive threshold logic
