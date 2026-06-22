# ACPM_30: Wire ACP Elicitation for Structured Input

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#acpm-30`](../ISSUE-TRACKER.md#acpm-30)
- Source: `tmp/solutions/roko/tasks/09-ACP-MCP.md` — Task 9.30
- Priority: **P2**
- Effort: 4 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ACPM_30 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

ACP supports `elicitation/create` for structured form dialogs. When the pipeline's strategist encounters an ambiguous prompt, it could ask the user to choose between approaches.

## Exact Changes

1. When the strategist determines multiple valid approaches, construct an elicitation request:
   ```json
   {
     "method": "elicitation/create",
     "params": {
       "title": "Choose approach",
       "description": "The prompt is ambiguous. Please select your preferred approach.",
       "inputs": [
         { "id": "approach", "label": "Approach", "type": "select", "options": ["A: Trait-based", "B: Enum-based"], "default": "A: Trait-based" }
       ]
     }
   }
   ```
2. Send via `transport.send_request()` and await response.
3. Parse the user's selections from the response.
4. Feed into the pipeline as additional context for the implementer.
5. Timeout (60s) falls back to default selection.

## Write Scope

- `crates/roko-acp/src/handler.rs`
- `crates/roko-acp/src/runner.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/09-ACP-MCP.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Strategist can request user to choose between 2 approaches
- [ ] User selection appears in implementer's context
- [ ] Timeout falls back to default selection

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ACPM_30 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Strategist can request user to choose between 2 approaches
- User selection appears in implementer's context
- Timeout falls back to default selection
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ACPM_30 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
