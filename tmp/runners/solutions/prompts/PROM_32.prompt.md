# PROM_32: Content-Type-Aware Token Estimation

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#prom-32`](../ISSUE-TRACKER.md#prom-32)
- Source: `tmp/solutions/roko/tasks/06-PROMPT-ASSEMBLY.md` — Task 6.32
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PROM_32 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: The `TokenCounter` (line 9) already supports `Tiktoken` and
`HuggingFace` variants. Add a `ContentAware` variant that detects content
type for better budget accuracy than the flat 4.0 heuristic.

## Exact Changes

1. Add helper `fn content_aware_chars_per_token(content: &str) -> f64`:
   - Detect code indicators: `fn `, `struct `, `impl `, `pub `, `let `, `use `, `mod `
   - If code-heavy (> 5% of words are code keywords): return 3.0
   - If markdown-heavy (contains `##` or many `- ` lines): return 5.0
   - Otherwise: return 4.0 (prose default)
2. Add `ContentAware` variant to `TokenCounter` enum
3. Implement `count()` for `ContentAware`: calls `content_aware_chars_per_token()` then divides
4. Wire `ContentAware` as the default counter in `PromptAssemblyService` (line 473, currently `Heuristic { chars_per_token: 4.0 }`)
5. Keep `Heuristic` as fallback for callers that do not need accuracy

## Write Scope

_None — this is a documentation/verification-only batch._

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/06-PROMPT-ASSEMBLY.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Code-heavy content (Rust source) estimates ~3 chars/token
- [ ] Markdown documentation estimates ~5 chars/token
- [ ] Prose text estimates ~4 chars/token
- [ ] Budget enforcement is tighter (fewer over-budget assemblies)

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PROM_32 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Code-heavy content (Rust source) estimates ~3 chars/token
- Markdown documentation estimates ~5 chars/token
- Prose text estimates ~4 chars/token
- Budget enforcement is tighter (fewer over-budget assemblies)
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PROM_32 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
