# PERF_08: Async Feedback Flush

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#perf-08`](../ISSUE-TRACKER.md#perf-08)
- Source: `tmp/solutions/roko/tasks/10-PERFORMANCE.md` — Task 10.8
- Priority: **??**
- Effort: ?
- Depends on: `PERF_06` (source 10.6)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PERF_08 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Remove per-event `w.flush()` from `write_event()`. Add explicit
`flush()` method. Call it at workflow completion.

## Exact Changes

1. In `jsonl_logger.rs` line 81, remove `w.flush()?;` after `writeln!(w, "{json}")?;`
2. Change `BufWriter::new(file)` at line 55 to `BufWriter::with_capacity(8192, file)`
   for explicit sizing
3. Add public method:
   ```rust
   pub fn flush(&self) -> std::io::Result<()> {
       let mut writer = self.writer.lock().unwrap_or_else(|e| e.into_inner());
       if let Some(ref mut w) = *writer {
           w.flush()?;
       }
       Ok(())
   }
   ```
4. In `WorkflowEngine::run()` (at `workflow_engine.rs`), call the logger's
   `flush()` after the workflow completes, before returning the result
5. Verify `BufWriter` flushes on `Drop` as a safety net

## Write Scope

- `crates/roko-runtime/src/jsonl_logger.rs`
- `crates/roko-runtime/src/workflow_engine.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/10-PERFORMANCE.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Test: write 100 events, call `flush()`, read back all 100
- [ ] No data loss: `roko run` still produces events in `runtime-events.jsonl`
- [ ] Per-event write latency drops (no sync I/O per event)

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PERF_08 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Test: write 100 events, call `flush()`, read back all 100
- No data loss: `roko run` still produces events in `runtime-events.jsonl`
- Per-event write latency drops (no sync I/O per event)
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PERF_08 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
