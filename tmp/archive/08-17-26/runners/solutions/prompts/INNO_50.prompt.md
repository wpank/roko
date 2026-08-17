# INNO_50: Add payload size guards to event logger

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-50`](../ISSUE-TRACKER.md#inno-50)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.50
- Priority: **P2**
- Effort: 4 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_50 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Research: LLM workloads easily blow Temporal's history-size budget. Payload
codecs and offloading are mandatory.

## Exact Changes

1. Define `MAX_INLINE_PAYLOAD_SIZE = 1_048_576` (1 MB).
2. Before writing an event, check payload size.
3. If > threshold: write payload to `.roko/events/payloads/{event_id}.json`,
   replace in event with `{ "$ref": "payloads/{event_id}.json" }`.
4. On event log read, resolve `$ref` references transparently.
5. Add payload GC: clean up unreferenced payload files older than 30 days.

## Write Scope

- `crates/roko-runtime/src/jsonl_logger.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/11-INNOVATIONS.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] An event with a 5 MB tool output stores the output in a separate file
- [ ] The event log entry contains a `$ref` instead of the full payload
- [ ] Reading the event log resolves references transparently

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_50 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- An event with a 5 MB tool output stores the output in a separate file
- The event log entry contains a `$ref` instead of the full payload
- Reading the event log resolves references transparently
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_50 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
