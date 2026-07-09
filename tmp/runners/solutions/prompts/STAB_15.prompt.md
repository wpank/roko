# STAB_15: Wire `build_repo_context` into plan generate

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-15`](../ISSUE-TRACKER.md#stab-15)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.15
- Priority: **P1**
- Effort: 2 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_15 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`build_repo_context()` at line 282 of `repo_context.rs` accepts a workdir and feature keywords,
returns a `RepoContextPack` with workspace map, crate structure, existing implementations.
It is called from:
- `prd.rs` line 877 (PRD draft generation)
- `commands/prd.rs` line 383 (PRD draft new)

It is NOT called from `commands/plan.rs` for plan generate, plan regenerate, or prd plan.

## Exact Changes

1. In `plan.rs`, in the `plan generate` handler:
   - Extract task keywords from the plan prompt or PRD content
   - Call `build_repo_context(&workdir, &keywords).await`
   - Inject the context pack into the agent prompt as a "Repository Structure" section
2. In `plan.rs`, in the `plan regenerate` handler: same treatment.
3. In `prd.rs` or `plan.rs`, in the `prd plan` handler: same treatment.
4. The context injection should use the `RepoContextPack::to_prompt_section()` method
   (or format it inline if no such method exists).

## Design Guidance

The repo context should be positioned early in the system prompt (before task-specific
instructions) so the agent has structural awareness when generating plans. Keep the keyword
extraction simple: split the prompt on whitespace, filter stopwords, take the top 5 by
tf-idf or frequency.

## Write Scope

- `crates/roko-cli/src/commands/plan.rs`
- `crates/roko-cli/src/commands/prd.rs`
- `crates/roko-cli/src/repo_context.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `roko plan generate` on a workspace with 18 crates includes crate names in the agent prompt
- [ ] Generated plan references existing crate names
- [ ] `roko prd plan` also includes repo context

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_15 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `roko plan generate` on a workspace with 18 crates includes crate names in the agent prompt
- Generated plan references existing crate names
- `roko prd plan` also includes repo context
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_15 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
