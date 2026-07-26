# Wave 7 cleanup pass evidence

Assignment:
- Wave: 7 (reconcile E01 and cleanup)
- Base SHA: 719e0f9d7 (status-quo/wave-4-6-telemetry tip)
- Branch: status-quo/wave-7-cleanup
- Integration branch: main (via PR)

## Changes made

### Dead code removal
- Deleted `crates/roko-core/src/state_hub.rs` (18.4K, orphan — not in lib.rs)
- Deleted `crates/roko-core/src/pulse_bus.rs` (6.3K, orphan — not in lib.rs)
- Both were duplicates of wired copies in roko-runtime

### Doc-link repairs
- `bus_backends.rs:3` — `crate::PulseBus` → `PulseBus (in roko-runtime)`
- `traits.rs:384` — `PulseBus (roko-core)` → `PulseBus (roko-runtime)`
- `dashboard_snapshot.rs:5,802` — `super::state_hub::StateHub` → `StateHub (in roko-cli)`

### Clippy fix
- `roko-orchestrator/src/worktree.rs:3640` — removed useless `.into()` on `std::io::Error`

### Documentation improvements
- `roko-cli/Cargo.toml` — corrected misleading `legacy-runner-v2` feature comment
- `roko-daimon/src/phase2_stubs.rs` — added module-level doc clarifying phase-2 status
- `roko-dreams/src/replay.rs:297` — documented retained-for-diagnostics field
- `roko-core/src/loop_tick.rs` — added runtime status section (not yet wired, tracked E01/E22)
- `roko-compose/src/strategy.rs` — documented VCG cold-start behavior

### Test additions
- `roko-compose/src/strategy.rs` — 4 new tests: empty bidders, WeightedSum normalization, explicit Vcg passthrough, is_density_greedy alias coverage
- `roko-compose/src/cost_attribution.rs` — 4 new tests: effectiveness before stamp, gate failure, empty sections, VCG payment preservation

### Gap tracking
- Updated `.roko/GAPS.md` with cleanup findings and pre-existing issues

## Verification

```
cargo check -p roko-core         # OK
cargo check -p roko-compose      # OK
cargo check -p roko-daimon       # OK
cargo check -p roko-dreams       # OK
cargo check -p roko-orchestrator # OK
cargo clippy -p roko-orchestrator --no-deps  # 0 warnings (was 1)
cargo test -p roko-compose -- strategy::tests     # 6/6 pass
cargo test -p roko-compose -- cost_attribution     # 6/6 pass
```
