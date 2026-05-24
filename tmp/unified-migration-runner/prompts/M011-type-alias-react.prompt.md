# M011 — Type alias: Policy → ReactProtocol in roko-core

## Objective
Add `ReactProtocol` as an alias for the existing `Policy` trait. This is the most
semantically significant rename — React operates on Pulses (ephemeral events), not Signals.

## Scope
- Crates: `roko-core`
- Files: `crates/roko-core/src/traits.rs`, `crates/roko-core/src/lib.rs`
- Phase ref: 02-PHASE-1-KERNEL.md §1.1

## Steps
1. Find the Policy trait:
   `grep -rn 'pub trait Policy' crates/roko-core/src/ --include='*.rs'`

2. Check if Policy already has a Pulse-based method:
   `grep -n 'Pulse\|pulse' crates/roko-core/src/traits.rs`

3. Add alias in roko-core:
   ```rust
   pub use traits::Policy as ReactProtocol;
   ```

4. Re-export from lib.rs.

5. Add a doc comment on the alias explaining the semantic shift:
   ```rust
   /// ReactProtocol — the canonical name for Policy.
   ///
   /// React operates on Pulses (ephemeral events on Bus), not Signals.
   /// The breaking change to take `&[Pulse]` instead of `&[Engram]` is
   /// tracked separately in the migration plan (§1.3).
   pub use traits::Policy as ReactProtocol;
   ```

## Verification
```bash
cargo check -p roko-core
cargo clippy -p roko-core --no-deps -- -D warnings
```

## What NOT to do
- Do NOT change the Policy trait signature yet — the Pulse breaking change is a later batch
- Do NOT rename implementations (CalibrationPolicy, DaimonPolicy, etc.) yet
