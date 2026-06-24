# TEST_34: CLI JSON output snapshot tests

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#test-34`](../ISSUE-TRACKER.md#test-34)
- Source: `tmp/solutions/roko/tasks/15-TESTING-VERIFICATION.md` — Task 15.34
- Priority: **P1**
- Effort: 3 hours
- Depends on: `TEST_01` (source 15.1), `TEST_02` (source 15.2)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: TEST_34 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

_(no context section in source)_

## Exact Changes

1. Test `roko status --json` on seeded workspace: output is valid JSON, contains expected top-level keys
2. Test `roko learn episodes --json` on seeded episodes.jsonl: output is valid JSON
3. Test `roko learn router --json` on seeded cascade-router.json: output is valid JSON
4. Test `roko config show --json` (if supported): output is valid JSON matching roko.toml structure
5. Test `roko plan list --json` on seeded plans: output is valid JSON array
6. Validate output parses with `serde_json::from_str::<Value>()` (schema validation via key presence)
7. Store expected key sets as test fixtures

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

- [ ] Every `--json` command produces valid JSON (parseable by serde_json)
- [ ] Output contains all expected fields
- [ ] No JSON output is mixed with log lines on stdout

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: TEST_34 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Every `--json` command produces valid JSON (parseable by serde_json)
- Output contains all expected fields
- No JSON output is mixed with log lines on stdout
- No files outside the Write Scope are modified.
- Commit message contains `tracker: TEST_34 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
