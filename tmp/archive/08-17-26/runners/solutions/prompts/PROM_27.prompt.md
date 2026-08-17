# PROM_27: Add Prompt Version Tagging

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#prom-27`](../ISSUE-TRACKER.md#prom-27)
- Source: `tmp/solutions/roko/tasks/06-PROMPT-ASSEMBLY.md` — Task 6.27
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PROM_27 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: Tag each assembled prompt with a version identifier so learning
data can be attributed to specific prompt versions.

## Exact Changes

1. Add `prompt_version: Option<String>` field to `SystemPromptBuilder`
2. Derive the version from a hash of: template version, role identity text, section names, ordering strategy
3. Use a simple hash (e.g., first 8 hex chars of SHA-256 of the concatenated inputs)
4. Include the version in the output via a `<!-- prompt_version:abc12345 -->` comment
5. Expose the version via `pub fn prompt_version(&self) -> Option<&str>`

## Write Scope

_None — this is a documentation/verification-only batch._

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/06-PROMPT-ASSEMBLY.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Every assembled prompt has a non-empty prompt_version string
- [ ] Changing the role identity text changes the version
- [ ] Changing the section set changes the version

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PROM_27 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Every assembled prompt has a non-empty prompt_version string
- Changing the role identity text changes the version
- Changing the section set changes the version
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PROM_27 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
