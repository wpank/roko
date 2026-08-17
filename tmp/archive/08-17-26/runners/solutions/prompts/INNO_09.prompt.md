# INNO_09: Implement semantic cache with BLAKE3 exact match

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-09`](../ISSUE-TRACKER.md#inno-09)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.9
- Priority: **P1**
- Effort: 12 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_09 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

When the same fix is applied multiple times (e.g., re-running a plan after
partial failure), the LLM is called even if the prompt is byte-identical.
BLAKE3 hashing of the full prompt provides exact-match deduplication with
zero false positives.

Research: Augment Code SWE-bench analysis -- 50-60% of tokens are removable.
Prompt-cache alone is 0.20x cost multiplier.

## Exact Changes

1. Create `crates/roko-learn/src/semantic_cache.rs`.
2. Define `SemanticCache` struct with `exact: HashMap<[u8; 32], CachedResponse>`.
3. Define `CachedResponse`: `response: String`, `model: String`,
   `created_at: DateTime<Utc>`, `ttl: Duration`, `task_category: String`.
4. Implement `check(prompt: &str) -> Option<CachedResponse>`:
   - Compute BLAKE3 hash of prompt.
   - Look up in exact map. If found and not expired, return.
5. Implement `store(prompt: &str, response: &str, model: &str, ttl: Duration,
   task_category: &str)`:
   - Only cache deterministic tasks (compile fixes, format, simple edits).
   - Never cache creative/architectural tasks (check `task_category`).
6. Implement `evict_expired()` to remove stale entries.
7. Persist cache to `.roko/cache/semantic.json` with TTL-based eviction on load.
8. Wire into dispatch path: check cache before LLM call, store after successful call.
9. Add `pub mod semantic_cache;` to `crates/roko-learn/src/lib.rs`.

## Write Scope

- `crates/roko-learn/src/lib.rs`
- `crates/roko-learn/Cargo.toml`

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

- [ ] Run the same fix task twice. Second run hits exact cache, zero LLM cost
- [ ] Second run response time < 100ms (cache lookup only)
- [ ] Cache does not store creative/architectural task responses
- [ ] Expired entries are evicted on next check

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_09 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Run the same fix task twice. Second run hits exact cache, zero LLM cost
- Second run response time < 100ms (cache lookup only)
- Cache does not store creative/architectural task responses
- Expired entries are evicted on next check
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_09 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
