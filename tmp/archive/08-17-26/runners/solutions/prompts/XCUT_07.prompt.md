# XCUT_07: Add Request-Scoped Correlation IDs

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#xcut-07`](../ISSUE-TRACKER.md#xcut-07)
- Source: `tmp/solutions/roko/tasks/19-CROSS-CUTTING.md` — Task 19.7
- Priority: **P5**
- Effort: 3 hours
- Depends on: `XCUT_06` (source 19.6)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: XCUT_07 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`crates/roko-serve/src/routes/middleware.rs` has no correlation ID middleware. When `roko serve` handles concurrent HTTP requests, log lines from different requests interleave without correlation. `RuntimeEventEnvelope` (line 11 of `runtime_event.rs`) has `run_id`, `seq`, `ts`, `schema_version`, `source`, and `payload` but no `correlation_id`. SSE and WebSocket routes emit events without request IDs.

## Exact Changes

1. Add Axum middleware that generates or extracts `X-Request-ID` header, stores in request extensions.
2. Create a tracing span `roko.http[request_id]` per request in the middleware.
3. Add optional `correlation_id: Option<String>` to `RuntimeEventEnvelope`.
4. When events are emitted from an HTTP-initiated context, populate `correlation_id` from the request extension.
5. SSE/WebSocket routes include `correlation_id` in event payloads.

## Write Scope

- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-core/src/runtime_event.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/19-CROSS-CUTTING.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `curl -H "X-Request-ID: test-123" http://localhost:6677/api/status` logs contain `request_id=test-123`
- [ ] Without the header, a UUID is auto-generated
- [ ] `RuntimeEventEnvelope` emitted from HTTP routes carry the correlation ID

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: XCUT_07 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `curl -H "X-Request-ID: test-123" http://localhost:6677/api/status` logs contain `request_id=test-123`
- Without the header, a UUID is auto-generated
- `RuntimeEventEnvelope` emitted from HTTP routes carry the correlation ID
- No files outside the Write Scope are modified.
- Commit message contains `tracker: XCUT_07 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
