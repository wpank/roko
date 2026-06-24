# XCUT_01: Audit and Migrate anyhow Usage at Crate Boundaries

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#xcut-01`](../ISSUE-TRACKER.md#xcut-01)
- Source: `tmp/solutions/roko/tasks/19-CROSS-CUTTING.md` — Task 19.1
- Priority: **P2**
- Effort: 6 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: XCUT_01 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`anyhow::Error` appears at 218+ call sites across 30+ files in `crates/`. The error philosophy in `crates/roko-core/src/error/mod.rs` explicitly defines a structured `RokoError` enum with 17 `ErrorKind` discriminants and `thiserror` derivation. At public API boundaries, `anyhow::Result` erases this structure, preventing callers from matching on failure modes or building retry logic. Internal crate usage of `anyhow` is acceptable; only `pub fn` / `pub async fn` return types need migration.

The heaviest offenders: `roko-demo` (41 occurrences across 14 files, but demo code is lower priority), `roko-mcp-scripts` (7 occurrences), `roko-dreams` (cycle.rs + runner.rs), `roko-neuro`, `roko-index`.

## Exact Changes

1. For each crate with public `anyhow::Result` returns, create a crate-level error enum deriving `thiserror::Error` with `#[non_exhaustive]`. Example for `roko-neuro`:
   ```rust
   #[derive(Debug, thiserror::Error)]
   #[non_exhaustive]
   pub enum NeuroError {
       #[error("knowledge store: {0}")]
       Store(String),
       #[error("query failed: {0}")]
       Query(String),
       #[error(transparent)]
       Io(#[from] std::io::Error),
       #[error(transparent)]
       Other(#[from] anyhow::Error),
   }
   ```
2. Convert public signatures from `anyhow::Result<T>` to `Result<T, CrateError>`.
3. Implement `From<CrateError>` for `RokoError` on each new error type to maintain the unified taxonomy.
4. Keep `anyhow` for internal-only functions -- only public boundaries matter.
5. Verify no public function in the target crates returns `anyhow::Result`.

## Design Guidance

The `From<anyhow::Error>` variant provides a migration escape hatch -- internal functions that still use `anyhow` can be wrapped with `?` at the boundary. Over time, these `Other` variants should be replaced with specific error kinds. Do not attempt to migrate all 218 sites at once; focus on the 5 crates listed above which are imported by `roko-cli` and `roko-serve`.

## Write Scope

- `crates/roko-neuro/src/lib.rs`
- `crates/roko-dreams/src/cycle.rs`
- `crates/roko-dreams/src/runner.rs`
- `crates/roko-index/src/workspace.rs`
- `crates/roko-demo/src/deploy.rs`
- `crates/roko-mcp-scripts/src/main.rs`

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

- [ ] Zero `pub` functions in `roko-neuro`, `roko-dreams`, `roko-index` return `anyhow::Result`
- [ ] All new error types implement `Into<RokoError>`

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: XCUT_01 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Zero `pub` functions in `roko-neuro`, `roko-dreams`, `roko-index` return `anyhow::Result`
- All new error types implement `Into<RokoError>`
- No files outside the Write Scope are modified.
- Commit message contains `tracker: XCUT_01 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
