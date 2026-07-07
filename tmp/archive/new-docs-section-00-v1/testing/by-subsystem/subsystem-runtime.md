# roko-runtime — Test Coverage

> Tests for the process supervisor, event bus, and cancellation infrastructure.

**Status**: Shipping (used by the orchestrator; 0 dedicated unit tests)
**Crate**: `roko-runtime`
**Section**: Cross-cut infrastructure
**Last reviewed**: 2026-04-19

---

## Test Count: 0 dedicated unit tests

`roko-runtime` has no standalone unit tests as of the 2026-04-17 audit. Its correctness is verified through orchestrator integration tests.

---

## Tested Indirectly

The following `roko-runtime` behaviours are exercised by `roko-orchestrator` integration tests:

### ProcessSupervisor

- Launching a subprocess and capturing its stdout/stderr.
- Killing a subprocess on timeout.
- Detecting a crashed subprocess and emitting a crash event.
- Restarting a subprocess after a crash (supervisor policy).

### Event Bus

- Publishing an event reaches all current subscribers.
- A subscriber added after an event does not receive the past event (no replay by default).
- A slow subscriber does not block fast subscribers (non-blocking publish).
- Cancellation propagates to all subscribers.

### Cancellation Tokens

- A cancellation token can be cancelled exactly once.
- Cancelling a token wakes all tasks awaiting it.
- A child token is cancelled when its parent is cancelled.
- Cancelling a child does not cancel the parent.

Key property: [../by-property/cancellation-token-propagation.md](../by-property/cancellation-token-propagation.md).

---

## Known Gaps

- `roko-runtime` is the only shipping crate with 0 unit tests.
- The event bus backpressure behavior under slow consumers is not tested.
- No chaos tests for supervisor crash loops (supervisor itself crashes).

## See also

- [subsystem-orchestrator.md](subsystem-orchestrator.md) — exercises runtime via integration tests
- [../gaps-and-roadmap.md](../gaps-and-roadmap.md) — roko-runtime unit tests listed as a gap
