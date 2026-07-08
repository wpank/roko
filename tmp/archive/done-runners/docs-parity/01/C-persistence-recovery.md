# C — Persistence & Recovery (Docs 09-10)

Covers: snapshot recovery and event-log use in orchestration recovery.

The key audit correction is status: snapshot/resume is already wired. The remaining work is trust validation, not basic persistence activation.

---

## C.01 — Snapshot Recovery (Doc 09) — WIRED, WITH A TRUST GAP

The live runtime already supports:

- snapshot save
- `PlanRunner::from_snapshot()`
- `PlanRunner::from_snapshots()`
- executor restore
- resume flow through persisted state
- `save_snapshot_atomic()` on the write path

This pack should therefore stop describing snapshot recovery as if it were half-built.

What remains:

- persisted state is still easy to trust too early
- corruption and truncation handling can be stronger
- the recovery path should make an explicit integrity decision before restore

That is a good batch-01 target because it is small and testable.

## C.02 — Event Log (Doc 10) — REAL, BUT NARROWER THAN THE DOC STORY

The local orchestration event log is real:

- append-only log
- hash-chain integrity support
- snapshot and restore support

But this pack needs one explicit separation:

- the local orchestration event log is not the same thing as the shared runtime event bus,
- and the shared runtime bus still has only `PlanRevision` and `PrdPublished`.

For batch `01`, the useful event-log seam is narrow:

- use integrity verification in recovery where safe
- do not widen into event-taxonomy redesign

The event log is rich enough for local recovery, but that does not change the shared bus reality.

---

## Post-Audit Summary

| Item | Status | What matters now |
|------|--------|------------------|
| Snapshot save/resume | Done | treat it as live, then harden trust checks |
| Recovery helpers | Done | use them in the real recovery path |
| Event-log integrity | Done in code | call it from runtime recovery where it helps |
| Shared runtime bus | Narrow | still only two live event variants |

---

## Batch Guidance

### O1 — Recovery Trust Boundary

Good batch outcome:

- corrupted persisted state is rejected,
- event-log integrity is checked on the recovery path where safe,
- the remaining persistence roadmap stays deferred.

### Boundary To Keep Honest

- local orchestration event log: real, recovery-facing, hash-chained
- shared runtime event bus: real, but still only `PlanRevision` and `PrdPublished`
- distributed recovery architecture: deferred

### What To Defer

- delta snapshots
- Merkle verification
- CRDT recovery
- long-lived event-substrate redesign
