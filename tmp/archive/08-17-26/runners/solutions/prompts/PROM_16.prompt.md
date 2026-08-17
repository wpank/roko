# PROM_16: Replace ACP Inline Prompts with Template Calls

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#prom-16`](../ISSUE-TRACKER.md#prom-16)
- Source: `tmp/solutions/roko/tasks/06-PROMPT-ASSEMBLY.md` — Task 6.16
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PROM_16 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: Replace the `format!()` role descriptions in
`run_multi_role_review()` (lines 1527-1544) with `ReviewerTemplate` calls.

## Exact Changes

1. Import `roko_compose::templates::reviewer::{ReviewerTemplate, ReviewerInput, Reviewer}`
2. Replace the hardcoded "Architect Reviewer" format string (line 1527-1533):
   ```rust
   let template = ReviewerTemplate::new(Reviewer::Architect);
   let sections = template.render(&ReviewerInput { ... });
   let architect_prompt = sections_to_prompt_string(&sections);
   ```
3. Replace the hardcoded "Security & Correctness Auditor" format string (line 1537-1543) similarly, using `Reviewer::Auditor`
4. Remove the now-dead inline strings
5. Ensure the review JSON schema instruction (`REVIEW_JSON_SCHEMA`) is still appended

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

- [ ] `run_multi_role_review()` no longer contains `format!()` role descriptions
- [ ] ACP reviews produce the same structured output format as before
- [ ] `grep -n 'You are the.*Reviewer\|You are the.*Auditor' crates/roko-acp/src/runner.rs` returns 0 results

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PROM_16 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `run_multi_role_review()` no longer contains `format!()` role descriptions
- ACP reviews produce the same structured output format as before
- `grep -n 'You are the.*Reviewer\|You are the.*Auditor' crates/roko-acp/src/runner.rs` returns 0 results
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PROM_16 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
