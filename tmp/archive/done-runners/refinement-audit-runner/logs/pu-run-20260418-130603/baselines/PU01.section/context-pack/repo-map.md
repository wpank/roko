# Repo Map — Batch 01

Quick reference for agents working on orchestration parity and follow-on code batches.

## Workspace Root

`/Users/will/dev/nunchi/roko/roko/`

## Baseline Numbers

- workspace members: **36**
- total Rust LOC: **322,088**
- `crates/roko-cli/src/orchestrate.rs`: **17,087** lines
- `roko-learn`: **42 modules**, **35,847 LOC**
- `roko-serve`: **200+ routes**
- TUI surface: **~58K LOC**

## High-Value Paths

| What | Path | Why It Matters In Batch 01 |
|------|------|----------------------------|
| Main orchestration harness | `crates/roko-cli/src/orchestrate.rs` | main loop, dispatch, recovery, worktrees, conductor wiring |
| Executor core | `crates/roko-orchestrator/src/executor/` | live state machine, snapshot, recovery helpers, speculative actions |
| DAG support | `crates/roko-orchestrator/src/dag.rs` | shipped graph logic that is not yet the main runtime owner |
| Worktree manager | `crates/roko-orchestrator/src/worktree.rs` | isolated execution lifecycle |
| Merge queue | `crates/roko-orchestrator/src/merge_queue.rs` | conflict-aware merge serialization |
| Event log | `crates/roko-orchestrator/src/event_log.rs` | integrity checks for recovery hardening |
| Conductor | `crates/roko-conductor/src/` | watchers, decisions, layer boundary |
| Learned conductor policy | `crates/roko-learn/src/conductor.rs` | source of the current layering seam |
| Runtime event bus | `crates/roko-runtime/src/event_bus.rs` | still only two live `RokoEvent` variants |
| Orchestration docs | `docs/01-orchestration/` | source docs being refreshed |
| Parity pack | `tmp/docs-parity/01/` | execution contract and carry-forward notes |

## Runtime Boundary Notes

- Local orchestration recovery uses the event log in `roko-orchestrator`.
- Cross-cutting runtime pub/sub uses `roko-runtime::event_bus`.
- Those are both real, but they are not the same surface and should not be merged in prose.

## Practical Warnings

1. `orchestrate.rs` is the main conflict hotspot.
2. `UnifiedTaskDag` being shipped does not mean it already owns runtime scheduling.
3. Snapshot/resume and worktree lifecycle are already wired; do not reopen them as if they were missing.
4. Docs `12-13` are deferred. Do not pull them back into active batch-01 implementation.
5. The conductor/learn dependency is real and narrow; record it honestly, but do not try to solve it inside an orchestration patch.
