# M005 — Type alias: Engram → Signal in roko-core

## Objective
Add `pub type Signal = Engram;` to roko-core's public API, making Signal the canonical
name while preserving backward compatibility. This is the bridge step — callers can use
either name, and later batches will migrate callers to Signal.

## Scope
- Crates: `roko-core`
- Files: `crates/roko-core/src/signal.rs`, `crates/roko-core/src/lib.rs`
- Phase ref: 02-PHASE-1-KERNEL.md §1.1

## Steps
1. Find the Engram type definition:
   `grep -rn 'pub struct Engram' crates/roko-core/src/ --include='*.rs'`

2. In `crates/roko-core/src/signal.rs` (created in M002), add:
   ```rust
   pub use crate::engram::Engram as Signal;
   ```
   (Adjust the path based on where Engram is actually defined.)

3. In `crates/roko-core/src/lib.rs`, ensure `signal` module is exported and add a
   re-export at the crate root:
   ```rust
   pub use signal::Signal;
   ```

4. Also add convenience aliases for related types:
   ```rust
   // In signal.rs
   pub use crate::engram::Engram as Signal;
   // If there's an EngramId, alias it too:
   // pub use crate::engram::EngramId as SignalId;
   ```
   But ONLY alias types that actually exist. `grep -rn 'pub.*Engram' crates/roko-core/src/ --include='*.rs'` to find them all.

5. Update the roko-core README.md if it references Engram in code examples — add a note that Signal is the preferred name.

## Verification
```bash
cargo check -p roko-core
cargo clippy -p roko-core --no-deps -- -D warnings
# Verify the alias works:
grep -q 'pub use.*Signal' crates/roko-core/src/signal.rs
```

## What NOT to do
- Do NOT rename the Engram struct itself — that's a much larger change for later
- Do NOT update other crates to use Signal yet — they'll migrate in subsequent batches
- Do NOT add new fields or methods — pure aliasing only
