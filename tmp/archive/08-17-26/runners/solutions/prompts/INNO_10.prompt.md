# INNO_10: Add HDC fuzzy matching to semantic cache

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-10`](../ISSUE-TRACKER.md#inno-10)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.10
- Priority: **P2**
- Effort: 8 hours
- Depends on: `INNO_09` (source 11.9)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_10 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Exact-match caching misses prompts that differ only in line numbers, variable
names, or whitespace. HDC fingerprints at `crates/roko-primitives/src/hdc.rs`
provide `hamming_similarity()` for fuzzy matching.

## Exact Changes

1. Add `fuzzy: Vec<(HdcVector, CachedResponse)>` to `SemanticCache`.
2. On cache miss for exact match, compute HDC fingerprint of prompt.
3. Scan fuzzy entries for `hamming_similarity > 0.95` (configurable threshold).
4. If match found, validate applicability: check that the cached response's
   code context overlaps with the current context (file paths, function names).
5. On store, also insert into fuzzy index.
6. Limit fuzzy index to 1000 entries (LRU eviction).

## Write Scope

- `crates/roko-learn/src/semantic_cache.rs`

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

- [ ] A compile fix for `foo.rs:42` gets cached. A subsequent fix for `foo.rs:45` with the same error type hits the fuzzy cache
- [ ] False positive rate < 5% on a test suite of 50 similar-but-different prompts
- [ ] Fuzzy match latency < 10ms for 1000 entries

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_10 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- A compile fix for `foo.rs:42` gets cached. A subsequent fix for `foo.rs:45` with the same error type hits the fuzzy cache
- False positive rate < 5% on a test suite of 50 similar-but-different prompts
- Fuzzy match latency < 10ms for 1000 entries
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_10 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
