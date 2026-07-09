# M025 — Update React/Policy trait to take Pulses (breaking change)

## Objective
The Policy trait (which will be renamed to React) already has a `decide_with_pulses()` method that accepts both Engrams and Pulses. The unified spec (§1.3) requires the primary method signature to be `react(pulses: &[Pulse], ctx: &CellContext) -> ReactOutput`. Promote `decide_with_pulses` to be the canonical method and deprecate the engram-only `decide()`.

## Scope
- Crates: `roko-core`, `roko-daimon`, `roko-conductor`, `roko-learn`
- Files:
  - `crates/roko-core/src/traits.rs` (Policy trait, line ~337)
  - `crates/roko-core/src/pulse.rs` (PolicyOutputs struct)
  - All implementors of the Policy trait
- Phase ref: `tmp/unified-migration/02-PHASE-1-KERNEL.md` §1.3
- Spec ref: `tmp/unified/02-CELL.md` §7 (React protocol)

## Steps
1. Find all Policy trait implementations:
   ```bash
   grep -rn 'impl Policy for\|impl.*Policy.*for' crates/ --include='*.rs' | grep -v target/
   ```

2. Read the current trait definition:
   ```bash
   grep -n -A 30 'pub trait Policy' crates/roko-core/src/traits.rs
   ```

3. The trait currently has:
   - `decide(&self, stream: &[Engram], ctx: &Context) -> Vec<Engram>` (primary)
   - `decide_with_pulses(&self, engrams, pulses, ctx) -> PolicyOutputs` (secondary, with default impl)

4. Restructure:
   - Keep both methods for backward compatibility
   - Mark `decide()` as `#[deprecated(note = "Use decide_with_pulses() instead")]`
   - Make `decide_with_pulses()` the required method (remove default impl)
   - Add a default impl for `decide()` that calls `decide_with_pulses()`:
     ```rust
     #[deprecated(note = "Use decide_with_pulses() instead")]
     fn decide(&self, stream: &[Engram], ctx: &Context) -> Vec<Engram> {
         self.decide_with_pulses(stream, &[], ctx).engrams
     }
     ```

5. For each existing Policy implementation, ensure it implements `decide_with_pulses()`. If the implementation only used `decide()`, move the logic to `decide_with_pulses()` and have it return `PolicyOutputs` wrapping the engrams.

6. Update call sites that currently call `.decide()` to call `.decide_with_pulses()`:
   ```bash
   grep -rn '\.decide(' crates/ --include='*.rs' | grep -v target/ | grep -v 'decide_with_pulses'
   ```

7. Rename `PolicyOutputs` to `ReactOutput` (with a type alias for backward compatibility):
   ```rust
   pub type PolicyOutputs = ReactOutput;
   ```

## Verification
```bash
cargo check --workspace
cargo clippy --workspace --no-deps -- -D warnings
cargo test --workspace
# Verify no remaining calls to the deprecated decide():
grep -rn '\.decide(' crates/ --include='*.rs' | grep -v target/ | grep -v 'decide_with_pulses\|#\[deprecated'
```

## What NOT to do
- Do NOT rename Policy to React yet — that's a separate rename migration
- Do NOT remove `decide()` — deprecate it so callers can migrate gradually
- Do NOT change the Context type to CellContext yet — that depends on Cell trait (M012)
- Do NOT add new Pulse handling logic to existing implementations — just restructure the trait; actual Pulse handling is per-implementation and done separately
