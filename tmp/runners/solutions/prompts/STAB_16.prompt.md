# STAB_16: Inject validation diagnostics into plan regenerate

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-16`](../ISSUE-TRACKER.md#stab-16)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.16
- Priority: **P1**
- Effort: 3 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_16 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`plan regenerate` validates after generation but does NOT inject diagnostics into the
regeneration prompt. This is HOLLOW-3 from the audit. The validation-feedback loop is
missing.

## Exact Changes

1. After initial agent generation, validate the output using existing validation logic.
2. If validation fails:
   ```rust
   let mut retry_count = 0;
   let max_retries = 2;
   loop {
       let validation = validate_plan(&generated_output)?;
       if validation.is_ok() || retry_count >= max_retries {
           break;
       }
       let error_prompt = format!(
           "The generated plan has the following validation errors:\n{}\n\n\
            Fix these errors in the plan. Here is the original plan:\n{}",
           validation.errors_formatted(),
           generated_output
       );
       generated_output = run_agent_with_prompt(&error_prompt).await?;
       retry_count += 1;
   }
   ```
3. If still failing after retries, output the plan with warnings:
   ```
   WARNING: Plan has {n} validation errors after {max_retries} fix attempts:
   {errors}
   ```
4. Log the retry attempts in the episode for debugging.

## Design Guidance

The retry prompt should be concise -- include only the specific validation errors, not the
entire plan context. This keeps token cost proportional to the error, not the plan size.
Consider adding a `--no-fix` flag that skips the retry loop for users who want raw output.

## Write Scope

- `crates/roko-cli/src/commands/plan.rs`

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

- [ ] Plan with a file reference error triggers a fix attempt
- [ ] After fix, the plan is re-validated
- [ ] After 2 failed fix attempts, plan is output with warnings

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_16 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Plan with a file reference error triggers a fix attempt
- After fix, the plan is re-validated
- After 2 failed fix attempts, plan is output with warnings
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_16 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
