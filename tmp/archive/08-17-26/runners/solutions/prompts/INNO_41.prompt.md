# INNO_41: Implement gate immutability from agent perspective

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-41`](../ISSUE-TRACKER.md#inno-41)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.41
- Priority: **P1**
- Effort: 8 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_41 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Research: Darwin Godel Machine -- reward-hacked by removing monitoring tokens
to fake perfect scores. Verify gates must live outside the agent's modifiable
surface. Anthropic alignment-faking paper: Claude strategically complies 12%
of the time.

## Exact Changes

1. Load gate configs from a path NOT writable by agents: system gates from
   compiled defaults, user gates from `roko.toml` (loaded at startup, not
   re-read during run), generated gates from `.roko/learn/gate-evolution.json`.
2. During agent dispatch, do NOT pass gate config paths as tool-accessible files.
3. Validate gate config integrity: hash gate configs at startup with BLAKE3,
   verify hash before each gate run.
4. Log any attempt to modify gate config paths via agent tools.

## Write Scope

- `crates/roko-gate/src/gate_service.rs`

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

- [ ] An agent that attempts to write to gate config files gets the modification ignored
- [ ] Gate config hash is verified before each gate run
- [ ] Integrity violation is logged as a warning

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_41 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- An agent that attempts to write to gate config files gets the modification ignored
- Gate config hash is verified before each gate run
- Integrity violation is logged as a warning
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_41 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
