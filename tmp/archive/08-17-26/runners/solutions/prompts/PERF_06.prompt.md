# PERF_06: Lazy Event Serialization + Buffer Reuse

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#perf-06`](../ISSUE-TRACKER.md#perf-06)
- Source: `tmp/solutions/roko/tasks/10-PERFORMANCE.md` — Task 10.6
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PERF_06 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Use a thread-local buffer to avoid per-event `String` allocation in
`write_event()`. Serialize directly into a reusable `Vec<u8>` instead of
allocating a new `String` per event.

## Exact Changes

1. Add `thread_local! { static BUF: RefCell<Vec<u8>> = RefCell::new(Vec::with_capacity(512)); }`
2. In `write_event()`, replace `serde_json::to_string(&envelope)` (line 72)
   with:
   ```
   BUF.with(|buf| {
       let mut buf = buf.borrow_mut();
       buf.clear();
       serde_json::to_writer(&mut *buf, &envelope)?;
       buf.push(b'\n');
       // write buf to file
   })
   ```
3. Remove the separate `writeln!(w, "{json}")` -- the buffer already has the
   newline
4. Verify `event_bus.rs` does not eagerly serialize before passing to consumers

## Write Scope

- `crates/roko-runtime/src/jsonl_logger.rs`

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

- [ ] Events round-trip correctly (envelope schema unchanged)
- [ ] Benchmark: 30 sequential `write_event()` calls complete in <20ms

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PERF_06 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Events round-trip correctly (envelope schema unchanged)
- Benchmark: 30 sequential `write_event()` calls complete in <20ms
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PERF_06 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
