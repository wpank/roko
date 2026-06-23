# INNO_28: Implement SpeculativeFixRunner for parallel gate fixes

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-28`](../ISSUE-TRACKER.md#inno-28)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.28
- Priority: **P2**
- Effort: 16 hours
- Depends on: `INNO_08` (source 11.8), `INNO_17` (source 11.17)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_28 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Research: Speculative Actions (arxiv 2510.04371) -- up to 55% accuracy in
next-action prediction, significant latency reductions.

CancelToken at `crates/roko-runtime/src/cancel.rs` enables first-to-finish semantics.

## Exact Changes

1. Create `crates/roko-orchestrator/src/speculative.rs`.
2. Define `SpeculativeFixRunner` with `max_parallel_fixes: usize` (default 3).
3. Implement error complexity classifier:
   - Trivial (unused import, format) -> single haiku agent, no speculation.
   - Moderate (type mismatch, missing impl) -> 2 parallel: haiku + sonnet.
   - Complex (logic error, architectural) -> 3 parallel: sonnet x2 + opus.
4. On `GateFailed`, classify error and spawn parallel fix agents.
5. Use `CancelToken` from roko-runtime: first agent to pass gates cancels others.
6. Feed failed attempt context as anti-pattern to surviving agents.
7. Track speculation outcomes for learning.

## Write Scope

- `crates/roko-orchestrator/src/lib.rs`

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

- [ ] A compile error fix spawns 2 parallel agents. The faster fix passes first, the other is cancelled
- [ ] Speculation cost for 3 parallel haiku runs < 1 sonnet run
- [ ] After 10+ speculative runs, the system learns which error categories benefit from speculation

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_28 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- A compile error fix spawns 2 parallel agents. The faster fix passes first, the other is cancelled
- Speculation cost for 3 parallel haiku runs < 1 sonnet run
- After 10+ speculative runs, the system learns which error categories benefit from speculation
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_28 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
