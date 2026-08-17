# ACPM_27: Build SharedContextStore for Cross-Agent Access

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#acpm-27`](../ISSUE-TRACKER.md#acpm-27)
- Source: `tmp/solutions/roko/tasks/09-ACP-MCP.md` — Task 9.27
- Priority: **P1**
- Effort: 4 hours
- Depends on: `ACPM_08` (source 9.8)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ACPM_27 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Parallel agents (from Phase 2) need a way to share intermediate results. If the Architect finishes before the Auditor, the Auditor should have access to the Architect's findings for the verdict merge.

## Exact Changes

1. Define `SharedContextStore` with `Arc<RwLock<HashMap<String, ContextEntry>>>`:
   ```rust
   pub(crate) struct SharedContextStore {
       entries: Arc<RwLock<HashMap<String, ContextEntry>>>,
   }

   struct ContextEntry {
       author_role: String,
       key: String,
       value: String,
       timestamp: Instant,
   }
   ```
2. Implement `publish(role: &str, key: &str, value: &str)` -- writes entry.
3. Implement `query(key_prefix: &str) -> Vec<ContextEntry>` -- reads entries matching prefix.
4. Implement `snapshot() -> String` -- renders all entries as markdown, sorted by timestamp.
5. Add `pub(crate) mod shared_context;` to `lib.rs`.

## Write Scope

- `crates/roko-acp/src/lib.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/09-ACP-MCP.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Unit test: two concurrent writers (via `tokio::spawn`), reader sees both entries
- [ ] Snapshot renders entries sorted by timestamp
- [ ] Empty store returns empty string from snapshot

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ACPM_27 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Unit test: two concurrent writers (via `tokio::spawn`), reader sees both entries
- Snapshot renders entries sorted by timestamp
- Empty store returns empty string from snapshot
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ACPM_27 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
