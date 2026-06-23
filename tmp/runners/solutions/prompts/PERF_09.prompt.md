# PERF_09: Workspace Convention Cache

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#perf-09`](../ISSUE-TRACKER.md#perf-09)
- Source: `tmp/solutions/roko/tasks/10-PERFORMANCE.md` — Task 10.9
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PERF_09 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Cache `detect_workdir_conventions()` result with `Cargo.toml` mtime
invalidation. Conventions (language, build system, naming style) are stable
within a run.

## Exact Changes

1. Add field `convention_cache: Mutex<Option<ConventionCacheEntry>>` to
   `PromptAssemblyService` struct (line 47)
2. Define `ConventionCacheEntry { workdir: PathBuf, conventions: String, mtime: SystemTime }`
3. In `assemble()`, before `detect_workdir_conventions()` at line 507:
   - Lock cache, check if entry exists for same workdir + matching
     `Cargo.toml` mtime
   - On hit: return cached conventions string
   - On miss: call `detect_workdir_conventions()`, store result
4. Use `std::fs::metadata(workdir.join("Cargo.toml")).and_then(|m| m.modified())`
   for mtime comparison
5. For non-Rust workspaces: try `package.json`, `go.mod`, `pyproject.toml` as
   cache key file

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

- [ ] Unit test: two `assemble()` for same workdir with unchanged Cargo.toml --
- [ ] Unit test: touch Cargo.toml between calls -- cache invalidated, fresh detect

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PERF_09 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Unit test: two `assemble()` for same workdir with unchanged Cargo.toml --
- Unit test: touch Cargo.toml between calls -- cache invalidated, fresh detect
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PERF_09 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
