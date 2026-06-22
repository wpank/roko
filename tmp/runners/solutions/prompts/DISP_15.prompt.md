# DISP_15: Wire ProviderHealthTracker at CLI Entry Points

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#disp-15`](../ISSUE-TRACKER.md#disp-15)
- Source: `tmp/solutions/roko/tasks/03-INFERENCE-DISPATCH.md` — Task 3.15
- Priority: **P1**
- Effort: 3 hours
- Depends on: `DISP_14` (source 3.14)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: DISP_15 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

After Task 3.14, `ModelCallService` accepts a `HealthGate`. Now each CLI entry point that constructs a `ModelCallService` needs to create and pass a `ProviderHealthTracker`.

The health tracker should be shared across the session (not created per-call) so that a provider that fails mid-session gets tripped for subsequent calls.

## Exact Changes

1. In `dispatch_v2.rs:dispatch_via_model_call_service()`, create a `ProviderHealthTracker` and pass via `with_health_gate()`
2. In `run.rs`, create a session-scoped `ProviderHealthTracker`, share via `Arc`, pass to `ModelCallService`
3. In `chat_inline.rs`, create a session-scoped tracker that persists across the entire chat session
4. For `roko plan run`, the tracker should be shared across all task dispatches (same `Arc` passed to all `ModelCallService` instances)

## Design Guidance

Use `Arc<ProviderHealthTracker>` since the tracker needs to be shared across async tasks. Create one per CLI session, not per call. The tracker is in-memory only -- it does not persist to disk. Provider health resets on each CLI invocation, which is the correct behavior (a provider that was down 5 minutes ago may be back).

## Write Scope

- `crates/roko-cli/src/dispatch_v2.rs`
- `crates/roko-cli/src/run.rs`
- `crates/roko-cli/src/chat_inline.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/03-INFERENCE-DISPATCH.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `grep -n 'with_health_gate\|ProviderHealthTracker' crates/roko-cli/src/ -r` shows wiring in all 3 entry points
- [ ] Integration: if a provider is unreachable, the second call in the same session uses a fallback

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: DISP_15 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `grep -n 'with_health_gate\|ProviderHealthTracker' crates/roko-cli/src/ -r` shows wiring in all 3 entry points
- Integration: if a provider is unreachable, the second call in the same session uses a fallback
- No files outside the Write Scope are modified.
- Commit message contains `tracker: DISP_15 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
