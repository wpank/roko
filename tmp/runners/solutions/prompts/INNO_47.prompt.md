# INNO_47: Add HDC consistency check for adversarial detection

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-47`](../ISSUE-TRACKER.md#inno-47)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.47
- Priority: **P3**
- Effort: 4 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_47 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Research: HDXpose -- 85.7% non-targeted ASR via Differential Evolution on
10,240-bit binary VSAs. Defense: bind HDC fingerprint to code hash.

## Exact Changes

1. When computing an HDC fingerprint for an agent/skill, also compute a
   BLAKE3 hash of the agent's source code or configuration.
2. Store `(hdc_fingerprint, code_hash)` pairs.
3. On subsequent runs, recompute both. If HDC has drifted (Hamming distance
   > threshold) but code hash is unchanged, flag as `AdversarialDriftWarning`.
4. Log the warning. Configurable threshold (default: Hamming > 500 of 10240 bits).

## Write Scope

- `crates/roko-primitives/src/hdc.rs`

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

- [ ] A stable agent produces the same `(fingerprint, hash)` pair across runs
- [ ] Manually corrupting the fingerprint triggers an AdversarialDriftWarning
- [ ] Warning is informational, does not block execution

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_47 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- A stable agent produces the same `(fingerprint, hash)` pair across runs
- Manually corrupting the fingerprint triggers an AdversarialDriftWarning
- Warning is informational, does not block execution
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_47 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
