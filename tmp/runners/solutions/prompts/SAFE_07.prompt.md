# SAFE_07: Wire `ResultFilter` Into Tool Output Processing

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#safe-07`](../ISSUE-TRACKER.md#safe-07)
- Source: `tmp/solutions/roko/tasks/17-SAFETY-SECURITY.md` — Task 17.7
- Priority: **P1**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: SAFE_07 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: The `ResultFilter` has working `sanitize()` that truncates oversized
output, strips secrets via `ScrubPolicy`, and annotates external tool output.
Wire it into the agent output processing path so agent responses are sanitized
before being stored as episodes or passed to downstream consumers.

## Exact Changes

1. Instantiate a `ResultFilter::with_defaults()` in the runner event loop
2. After receiving agent output in `handle_agent_event()`, run the output through
   `result_filter.sanitize(output, "agent_response")`
3. Apply the same filter to gate error output before injecting into retry context
4. Make `max_response_bytes` configurable via `[safety.tool_output.max_bytes]`
   in roko.toml (default: 100KB per the existing constant)
5. Log sanitization events (what was stripped) at `tracing::debug!` level

## Write Scope

- `crates/roko-cli/src/dispatch_v2.rs`
- `crates/roko-cli/src/runner/event_loop.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/17-SAFETY-SECURITY.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Agent output containing `sk-ant-api01-...` has the key redacted before episode storage
- [ ] Agent output exceeding 100KB is truncated with `[OUTPUT TRUNCATED]` marker
- [ ] Tool output from `bash` commands is annotated as external-source data
- [ ] Existing tests pass; new test confirms sanitization is applied

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: SAFE_07 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Agent output containing `sk-ant-api01-...` has the key redacted before episode storage
- Agent output exceeding 100KB is truncated with `[OUTPUT TRUNCATED]` marker
- Tool output from `bash` commands is annotated as external-source data
- Existing tests pass; new test confirms sanitization is applied
- No files outside the Write Scope are modified.
- Commit message contains `tracker: SAFE_07 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
