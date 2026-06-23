# INNO_11: Implement prompt compression pipeline

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-11`](../ISSUE-TRACKER.md#inno-11)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.11
- Priority: **P1**
- Effort: 8 hours
- Depends on: `INNO_05` (source 11.5), `INNO_06` (source 11.6)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_11 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

When the computed prompt exceeds the model's optimal context size (from
ModelContextProfile), tokens must be reduced. The compose crate already has
`compaction.rs` and `token_counter.rs`. Section effectiveness data is available
from `roko-learn/src/section_effect.rs`.

## Exact Changes

1. Create `crates/roko-compose/src/compressor.rs`.
2. Strategy 1 (regex-based): strip redundant whitespace, code comments in
   examples, duplicate section headers. No LLM needed.
3. Strategy 2 (code summarization): if a code block exceeds 200 tokens, replace
   with function signature + docstring + `// ... N lines`.
4. Strategy 3 (section-effectiveness-aware): if section_effect data is available
   (from `roko-learn`), drop sections with the lowest measured lift first.
   Never drop task description or tool instructions.
5. Implement `compress(prompt: &str, target_tokens: usize) -> String` applying
   strategies in sequence until target is reached.
6. Wire into prompt assembly when computed prompt exceeds
   `ModelContextProfile::optimal_size()`.
7. Add `pub mod compressor;` to `crates/roko-compose/src/lib.rs`.

## Write Scope

- `crates/roko-compose/src/lib.rs`

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

- [ ] A 15K-token prompt compressed for an 8K-context model produces output <= 5.6K tokens (70% of window)
- [ ] Compressed prompt retains task description and tool instructions verbatim
- [ ] Code blocks > 200 tokens are summarized to < 50 tokens
- [ ] Unit test: compress a known prompt, verify output is valid and smaller

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_11 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- A 15K-token prompt compressed for an 8K-context model produces output <= 5.6K tokens (70% of window)
- Compressed prompt retains task description and tool instructions verbatim
- Code blocks > 200 tokens are summarized to < 50 tokens
- Unit test: compress a known prompt, verify output is valid and smaller
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_11 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
