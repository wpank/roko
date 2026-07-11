# State Persistence and Resume Issues

## High

### `write_checkpoint` never called — integrity check is a no-op
- `persist.rs:606`: Function exists but zero callers.
- `verify_checkpoint` at `event_loop.rs:628` returns `Ok(true)` when file missing (line 630). State integrity is never verified.

### `failed_tasks` not persisted or restored on resume
- `state.rs:105`: `RunState.failed_tasks` drives DAG dependency skipping.
- `RunStateSnapshot` has no `failed_tasks` field. After resume, previously-failed tasks appear non-failed.
- Dependents of failed tasks are not pre-skipped → may hang forever or dispatch incorrectly.

### Legacy state files lag behind unified snapshot
- `snapshot_writer.rs:189-193`: `write_all_files` only writes `state-snapshot.json`.
- Individual `executor.json`, `orchestrator.json`, `run-state.json` are never updated after the snapshot refactor.
- Fallback load chain (`orchestrator.json` → `state-snapshot.json` → `executor.json`) can load stale data.

### Graph Engine ignores all snapshots and skips workspace lock
- `commands/plan.rs:258-271`: `--resume-plan` prints a warning and is discarded.
- No workspace lock acquired on Graph path (lock is inside `else` block at line 272).

## Medium

### All-invalid JSONL file left untruncated
- `persist.rs:494-505`: When `last_good_byte == 0` and `dropped_lines > 0`, returns `DroppedInvalid` without modifying file. Subsequent appends sit on top of malformed content.

### `tasks_total` not restored from snapshot
- `event_loop.rs:776`: Always recomputed from current plan set. If tasks were dynamically added/removed during original run, progress percentage is wrong after resume.

### Unbounded `.bak.` file proliferation
- `commands/plan.rs:280-297`: Every `--fresh` archives state files with timestamp suffix. No pruning. Currently 27 `.bak.` files on disk.

### `PlanMissing` check skipped when snapshot has empty fingerprints
- `resume.rs:231-239`: Iterates caller-supplied slice. If empty, check is silently bypassed.
