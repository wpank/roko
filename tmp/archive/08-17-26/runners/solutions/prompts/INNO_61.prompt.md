# INNO_61: Add multi-dimensional collective intelligence measurement

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-61`](../ISSUE-TRACKER.md#inno-61)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.61
- Priority: **P3**
- Effort: 8 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_61 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

CFactorSummary at `crates/roko-core/src/cfactor.rs` is a single scalar.

Research: PLOS One 2024 replication failure -- single scalar is unreliable.
Williams-Beer/Broja PID: decompose into synergy (true collective), redundancy
(wasted), unique (specialization). Caveat: PID for n>=3 is mathematically
broken; use binary-only.

## Exact Changes

1. Implement binary PID (Williams-Beer): decompose mutual information between
   two agent outputs into synergy, redundancy, and unique.
2. For n >= 3 agents, use pairwise PID (binary-only).
3. Add `synergy`, `redundancy`, `unique_info` fields to `CFactorSummary`.
4. Gate multi-agent scaling on synergy threshold: if synergy < 0.1, recommend
   reducing agent count.
5. Log PID components in efficiency events.

## Write Scope

- `crates/roko-core/src/cfactor.rs`
- `crates/roko-cli/src/orchestrate.rs`

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

- [ ] After a multi-agent run with 3 agents, CFactorSummary includes synergy, redundancy, and unique components
- [ ] High-redundancy runs produce a recommendation to reduce agent count
- [ ] PID components are visible in `roko learn all` output

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_61 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- After a multi-agent run with 3 agents, CFactorSummary includes synergy, redundancy, and unique components
- High-redundancy runs produce a recommendation to reduce agent count
- PID components are visible in `roko learn all` output
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_61 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
