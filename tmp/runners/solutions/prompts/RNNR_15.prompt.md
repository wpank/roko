# RNNR_15: Add context-pack directory support for shared agent knowledge

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#rnnr-15`](../ISSUE-TRACKER.md#rnnr-15)
- Source: `tmp/solutions/roko/tasks/14-RUNNER-PATTERNS.md` — Task 14.15
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: RNNR_15 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: Support the mega-parity runner's context-pack pattern: a directory
of markdown files prepended to every agent prompt. Rules, architecture,
anti-patterns, performance contracts.

## Exact Changes

1. Add `context_pack_dir: Option<PathBuf>` to `PromptAssemblyService` config
2. If set, read all `*.md` files from the directory, sorted by filename
   (00-RULES.md first, 05-NO-BUILD.md last)
3. Concatenate into a single context string with file separators
4. Inject as system prompt layer 2 (after role identity, before domain knowledge)
5. Track total token count; warn if context pack exceeds 8000 tokens
6. Add `[execution.context_pack_dir]` to `roko.toml`; per-plan overrides supported

## Write Scope

- `crates/roko-compose/src/prompt_assembly_service.rs`
- `crates/roko-core/src/config/mod.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/14-RUNNER-PATTERNS.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Files in context-pack directory injected into every agent prompt
- [ ] Files ordered by filename (numeric prefix sorting)
- [ ] Warning emitted when pack exceeds 8000 tokens
- [ ] Per-plan override works when specified in plan config

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: RNNR_15 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Files in context-pack directory injected into every agent prompt
- Files ordered by filename (numeric prefix sorting)
- Warning emitted when pack exceeds 8000 tokens
- Per-plan override works when specified in plan config
- No files outside the Write Scope are modified.
- Commit message contains `tracker: RNNR_15 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
