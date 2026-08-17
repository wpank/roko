# INNO_31: Implement cross-project global config directory

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-31`](../ISSUE-TRACKER.md#inno-31)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.31
- Priority: **P2**
- Effort: 4 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_31 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

_(no context section in source)_

## Exact Changes

1. Define `GlobalConfig` struct with paths: `domains/`, `meta/`, `cache/`, `community/`.
2. Implement `GlobalConfig::ensure(home_dir: &Path) -> Result<Self>` that creates
   the directory structure at `~/.roko/`.
3. Wire into CLI startup: ensure global dir exists before loading project config.
4. Add `global_dir` to the paths available in the runtime context.

## Write Scope

- `crates/roko-core/src/config/schema.rs`

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

- [ ] After running any `roko` command, `~/.roko/` exists with subdirectories
- [ ] Global config is loadable from any project
- [ ] Does not interfere with project-local `.roko/` directory

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_31 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- After running any `roko` command, `~/.roko/` exists with subdirectories
- Global config is loadable from any project
- Does not interfere with project-local `.roko/` directory
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_31 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
