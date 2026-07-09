# INNO_56: Emit gate results as structured compliance events

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-56`](../ISSUE-TRACKER.md#inno-56)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.56
- Priority: **P3**
- Effort: 4 hours
- Depends on: `INNO_54` (source 11.54)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_56 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

_(no context section in source)_

## Exact Changes

1. After each gate rung completes, emit an OTel span with structured attributes:
   `gate.rung.id`, `gate.verdict`, `gate.agent_id`, `gate.evidence`, etc.
2. For Article 50 compliance: include `ai.provenance.model`,
   `ai.provenance.timestamp`, `ai.provenance.confidence`.

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

- [ ] Gate results appear as OTel spans with all structured attributes
- [ ] A SIEM tool can filter for `gate.verdict = fail` events

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_56 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Gate results appear as OTel spans with all structured attributes
- A SIEM tool can filter for `gate.verdict = fail` events
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_56 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
