# TEST_33: CLI help text snapshot tests

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#test-33`](../ISSUE-TRACKER.md#test-33)
- Source: `tmp/solutions/roko/tasks/15-TESTING-VERIFICATION.md` — Task 15.33
- Priority: **P1**
- Effort: 3 hours
- Depends on: `TEST_01` (source 15.1), `TEST_02` (source 15.2)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: TEST_33 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

_(no context section in source)_

## Exact Changes

1. Generate `--help` output for each top-level command: `run`, `plan`, `prd`, `chat`, `serve`, `status`, `doctor`, `init`, `config`, `learn`, `knowledge`, `research`, `agent`, `job`, `explain`, `dashboard`, `bench`, `deploy`, `replay`, `history`, `up`
2. Generate `--help` for key subcommands: `plan list`, `plan show`, `plan create`, `plan run`, `plan validate`, `config show`, `config providers`, `config models`
3. Store snapshots in `crates/roko-cli/tests/snapshots/help/` as `.txt` files
4. Compare output against stored snapshots using simple string comparison
5. Fail on unexpected changes (forces intentional help text updates)
6. Provide an update mechanism: set `UPDATE_SNAPSHOTS=1` env var to overwrite stored snapshots

## Design Guidance

Do NOT use `insta` (not in workspace dependencies). Use simple `assert_eq!` with file read/write. The update mechanism: if `std::env::var("UPDATE_SNAPSHOTS").is_ok()`, write current output to snapshot file and pass; otherwise, read snapshot file and compare.

## Write Scope

_None — this is a documentation/verification-only batch._

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/15-TESTING-VERIFICATION.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Every top-level command has a help snapshot
- [ ] Key subcommands have snapshots
- [ ] Adding a new flag without updating snapshots causes test failure
- [ ] `UPDATE_SNAPSHOTS=1` updates all snapshot files

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: TEST_33 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Every top-level command has a help snapshot
- Key subcommands have snapshots
- Adding a new flag without updating snapshots causes test failure
- `UPDATE_SNAPSHOTS=1` updates all snapshot files
- No files outside the Write Scope are modified.
- Commit message contains `tracker: TEST_33 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
