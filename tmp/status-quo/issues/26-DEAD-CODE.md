# Dead Code and Unused Features

## Critical — Compiled but never called

### `orchestrate.rs` — 23,676 lines (legacy-gated, dead-by-default)
- Behind `#[cfg(feature = "legacy-orchestrate")]` (non-default).
- Exists as reference. Not compiled in normal build.
- Contains cross-cutting ideas not ported to runner-v2.

### `roko-conductor` — 10,101 lines (unused dependency)
- Only imported from `orchestrate.rs` (legacy).
- `roko-serve` declares dep but has zero uses.
- `self_healing.rs`, `federation.rs`, 10 watchers — all dead at runtime.

### `roko-orchestrator` dead sub-modules — ~6,565 lines
- `coordination.rs` (1,991), `mesh_relay.rs` (308), `post_merge.rs` (500), `repair.rs` (956), `event_log.rs` (647), `progress.rs` (469)
- 5 safety sub-modules (`capability_tokens`, `loop_guard`, `sandboxing`, `taint_propagation`, `permit`) — 2,810 lines, zero callers ANYWHERE.

### `heartbeat_attention.rs` — 2,146 lines
- Declared in `roko-runtime/src/lib.rs:51`. Zero callers outside that declaration.

### `delta_consumer.rs` — 424 lines
- All three core methods are documented stubs: "will be connected to roko-dreams when wired."

## High — Feature flags that don't gate code

### `legacy-runner-v2` — no effect on binary
- `roko-cli/Cargo.toml:15`: Default feature, zero `#[cfg(feature = "legacy-runner-v2")]` in `src/`.
- Only gates 5 test files. Feature can be removed entirely.

### Blanket `#![allow(dead_code)]` on roko-cli
- `roko-cli/src/lib.rs:6`: Suppresses ALL dead-code warnings for 90K-line library.
- Makes it impossible to detect unused public functions via compiler.

## Medium

### Test utilities more complex than tested code
- `tests/common/mod.rs`: 476 lines with `#![allow(dead_code)]`. Individual helpers used by <2 tests each.

### Commented-out stub implementations
- `delta_consumer.rs:299-340`: Three methods with placeholder returns and documented intent.
