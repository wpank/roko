# M020 — Verify bus.rs module in roko-core

## Objective
Phase 0 §0.3 calls for creating `crates/roko-core/src/bus.rs` as a stub. However, the Bus trait already exists in `crates/roko-core/src/traits.rs` (line ~383) and implementations exist in `crates/roko-core/src/bus_backends.rs` and `crates/roko-core/src/pulse_bus.rs`. This migration verifies the existing bus infrastructure matches the unified spec and fills any gaps.

## Scope
- Crates: `roko-core`
- Files:
  - `crates/roko-core/src/traits.rs` (Bus trait definition)
  - `crates/roko-core/src/bus_backends.rs` (BroadcastBus, MemoryBus, MultiBus)
  - `crates/roko-core/src/pulse_bus.rs` (PulseBus wrapping EventBus)
  - `crates/roko-core/src/lib.rs` (exports)
- Phase ref: `tmp/unified-migration/01-PHASE-0-PREP.md` §0.3
- Spec ref: `tmp/unified/01-SIGNAL.md` §3.3 (Bus)

## Steps
1. Confirm Bus trait and implementations exist:
   ```bash
   grep -rn 'pub trait Bus' crates/roko-core/src/ --include='*.rs'
   grep -rn 'impl Bus for' crates/roko-core/src/ --include='*.rs'
   grep -rn 'pub struct.*Bus' crates/roko-core/src/ --include='*.rs'
   ```

2. Check if Bus, PulseBus, BroadcastBus are exported from lib.rs:
   ```bash
   grep -n 'Bus\|PulseBus\|BroadcastBus' crates/roko-core/src/lib.rs
   ```

3. Compare TopicFilter variants against the unified spec (`tmp/unified/01-SIGNAL.md` §3.3):
   - Spec requires: `Exact(Topic)`, `Prefix(String)`, `Glob(String)`, `AnyOf(Vec)`, `And(Box, Box)`, `Not(Box)`
   - Current has: `Exact(Topic)`, `Prefix(String)`, `All`
   - Missing: `Glob`, `AnyOf`, `And`, `Not`

4. Add the missing TopicFilter variants to `crates/roko-core/src/pulse.rs`:
   ```rust
   pub enum TopicFilter {
       Exact(Topic),
       Prefix(String),
       All,
       // New variants from unified spec:
       /// Glob pattern matching (e.g., "gate.*.emitted").
       Glob(String),
       /// Match any of the inner filters.
       AnyOf(Vec<TopicFilter>),
       /// Match when both inner filters match.
       And(Box<TopicFilter>, Box<TopicFilter>),
       /// Match when the inner filter does NOT match.
       Not(Box<TopicFilter>),
   }
   ```

5. Update `TopicFilter::matches()` to handle the new variants:
   - `Glob`: Use simple wildcard matching (split on `.`, match `*` as any segment)
   - `AnyOf`: `inner.iter().any(|f| f.matches(topic))`
   - `And`: `a.matches(topic) && b.matches(topic)`
   - `Not`: `!inner.matches(topic)`

6. Add tests for each new TopicFilter variant.

7. Verify exports are complete — Bus, PulseBus, BroadcastBus, TopicFilter, Topic should all be re-exported from roko-core's lib.rs.

## Verification
```bash
cargo check -p roko-core
cargo clippy -p roko-core --no-deps -- -D warnings
cargo test -p roko-core -- topic_filter
cargo test -p roko-core -- bus
cargo test -p roko-core -- pulse_bus
```

## What NOT to do
- Do NOT create a new `bus.rs` file — the bus infrastructure already exists across traits.rs, bus_backends.rs, and pulse_bus.rs
- Do NOT move the Bus trait out of traits.rs — keep it alongside the other protocol traits
- Do NOT add dependencies for glob matching — implement simple segment-based wildcard matching
- Do NOT change PulseBus internals — only extend TopicFilter
