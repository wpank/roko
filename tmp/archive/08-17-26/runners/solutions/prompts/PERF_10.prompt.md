# PERF_10: Source Context Collection Cache

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#perf-10`](../ISSUE-TRACKER.md#perf-10)
- Source: `tmp/solutions/roko/tasks/10-PERFORMANCE.md` — Task 10.10
- Priority: **??**
- Effort: ?
- Depends on: `PERF_09` (source 10.9)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PERF_10 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Cache `collect_source_context()` / `collect_source_context_from()`
with `src/` directory mtime invalidation. Convert blocking `std::fs` to
`tokio::fs` in the hot path.

## Exact Changes

1. Add `source_context_cache: Mutex<Option<SourceContextCacheEntry>>` to
   `PromptAssemblyService`
2. Define `SourceContextCacheEntry { workdir: PathBuf, context: (Vec<String>, Vec<String>), dir_mtime: SystemTime }`
3. Before calling `collect_source_context()` at line 513, check cache using
   `src/` directory mtime
4. Convert `std::fs::read_dir()` and `std::fs::read_to_string()` in
   `collect_source_context_from()` (line 681) to `tokio::fs` equivalents.
   Note: `collect_source_context_from` is called synchronously from
   `detect_workdir_conventions`, so this may require making it async or using
   `spawn_blocking` for the recursive walk
5. Cap source file reads at 12 files / 64KB total (preserve existing limits)

## Write Scope

- `crates/roko-compose/src/prompt_assembly_service.rs`

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

- [ ] Unit test: repeated `assemble()` uses cached source context
- [ ] No blocking `std::fs` in the hot prompt assembly path (or wrapped in

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PERF_10 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Unit test: repeated `assemble()` uses cached source context
- No blocking `std::fs` in the hot prompt assembly path (or wrapped in
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PERF_10 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
