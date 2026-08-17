# M022 — Verify Bus trait and BroadcastBus impl match unified spec

## Objective
The Bus trait and BroadcastBus implementation already exist in roko-core (`traits.rs`, `bus_backends.rs`, `pulse_bus.rs`). Verify they match the unified spec's requirements and fill any gaps. The spec requires `publish()` returning a sequence number and `subscribe()` with `TopicFilter`. Also verify that `BroadcastBus` and `PulseBus` have adequate test coverage.

## Scope
- Crates: `roko-core`
- Files:
  - `crates/roko-core/src/traits.rs` (Bus trait, line ~383)
  - `crates/roko-core/src/bus_backends.rs` (BroadcastBus, MemoryBus, MultiBus)
  - `crates/roko-core/src/pulse_bus.rs` (PulseBus — primary production impl)
- Phase ref: `tmp/unified-migration/02-PHASE-1-KERNEL.md` §1.2
- Spec ref: `tmp/unified/01-SIGNAL.md` §3.3 (Bus)

## Steps
1. Read current Bus trait definition:
   ```bash
   grep -n -A 15 'pub trait Bus' crates/roko-core/src/traits.rs
   ```

2. Compare with unified spec requirements (`tmp/unified/01-SIGNAL.md` §3.3):
   - `publish(pulse: Pulse) -> Result<u64>` — present
   - `subscribe(filter: TopicFilter) -> Result<Receiver>` — present
   - Associated `Receiver` type — present

3. Check existing tests for Bus implementations:
   ```bash
   grep -rn '#\[test\]\|#\[tokio::test\]' crates/roko-core/src/bus_backends.rs crates/roko-core/src/pulse_bus.rs | wc -l
   ```

4. If test coverage is thin, add these tests for `BroadcastBus`:
   - Publish 10 pulses, subscribe, verify all received
   - Subscribe with `TopicFilter::Prefix`, verify only matching pulses arrive
   - Subscribe with `TopicFilter::Exact`, verify exact match only
   - Multiple subscribers receive the same pulse (fan-out)

5. Add performance sanity test for `PulseBus`:
   ```rust
   #[tokio::test]
   async fn pulse_bus_throughput() {
       let bus = PulseBus::new(4096);
       let mut rx = bus.subscribe(TopicFilter::All).unwrap();
       // Publish 1000 pulses
       for i in 0..1000 {
           bus.publish(Pulse::new(i, Topic::new("test"), Kind::Metric, Body::text("x"))).unwrap();
       }
       // Verify subscriber can drain them
   }
   ```

6. Verify the `MemoryBus` (with replay) works correctly — replay_from should return pulses after a given sequence number.

7. Ensure all Bus implementations are exported from `crates/roko-core/src/lib.rs`.

## Verification
```bash
cargo check -p roko-core
cargo clippy -p roko-core --no-deps -- -D warnings
cargo test -p roko-core -- bus
cargo test -p roko-core -- pulse_bus
cargo test -p roko-core -- broadcast_bus
```

## What NOT to do
- Do NOT redesign the Bus trait — it already matches the spec
- Do NOT remove MemoryBus or MultiBus — they serve different use cases
- Do NOT change the Receiver associated type pattern — it allows different backends
- Do NOT add external dependencies for pub/sub — tokio::sync is sufficient
