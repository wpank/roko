# PERF_39: Verify SHARED_HTTP_CLIENT Connection Reuse

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#perf-39`](../ISSUE-TRACKER.md#perf-39)
- Source: `tmp/solutions/roko/tasks/10-PERFORMANCE.md` — Task 10.39
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PERF_39 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Add logging and tests confirming connection reuse. The
`SHARED_HTTP_CLIENT` static at line 93 already exists with `pool_max_idle_per_host(10)`,
`pool_idle_timeout(90s)`, `tcp_keepalive(30s)`.

## Exact Changes

1. Add `tracing::debug!("SHARED_HTTP_CLIENT initialized")` in the `LazyLock`
   init closure
2. Add test: two requests to same mock server, verify single TCP connection
3. Verify current config is optimal (10 idle, 90s timeout, 30s keepalive)
4. Consider: for `roko serve`, increase `pool_idle_timeout` to 300s

## Write Scope

- `crates/roko-agent/src/provider/mod.rs`

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

- [ ] Test confirms connection reuse within idle timeout
- [ ] No unnecessary TLS handshakes for sequential same-provider requests
- [ ] Log shows SHARED_HTTP_CLIENT initialized exactly once per process

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PERF_39 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Test confirms connection reuse within idle timeout
- No unnecessary TLS handshakes for sequential same-provider requests
- Log shows SHARED_HTTP_CLIENT initialized exactly once per process
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PERF_39 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
