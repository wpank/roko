# EVAL_13: `AstCollector` -- tree-sitter AST extraction

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#eval-13`](../ISSUE-TRACKER.md#eval-13)
- Source: `tmp/solutions/roko/tasks/05-GATE-EVOLUTION.md` — Task 5.13
- Priority: **P2**
- Effort: 8 hours
- Depends on: `EVAL_07` (source 5.7)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: EVAL_13 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Uses `tree-sitter` + `tree-sitter-rust` to parse source files and produce `EvidenceKind::Ast` evidence. The existing `crates/roko-lang-rust/` already has tree-sitter integration for the code intelligence indexer. Reuse patterns from there.

## Exact Changes

1. Define `FileAst` struct: `{ path: String, items: Vec<AstItem>, complexity: Vec<FunctionComplexity> }`.
2. Define `AstItem`: `{ kind: String, name: String, visibility: String, span: (usize, usize), children: Vec<AstItem>, body_text: Option<String> }`.
3. Define `FunctionComplexity`: `{ function: String, cyclomatic: u32, cognitive: u32, body_lines: u32 }`.
4. Implement `AstCollector` implementing `EvidenceCollector`:
   - `produces()` = `[EvidenceKind::Ast]`
   - `collect()`: parse source files using tree-sitter, walk the AST, extract items and complexity.
   - Factory method: `AstCollector::for_changed_files(workdir: &Path)` -- uses `git diff --name-only` to find changed files, parses only those.
5. Gate behind `#[cfg(feature = "ast")]`.

## Design Guidance

Tree-sitter C bindings can be slow to compile. Gate behind `ast` feature flag so the default workspace build is unaffected. Complexity calculation: cyclomatic = number of branch points (if, match arm, while, for, &&, ||); cognitive = cyclomatic + nesting depth bonus; body_lines = line count excluding braces.

## Write Scope

- `crates/roko-eval-metrics/src/lib.rs`
- `crates/roko-eval-metrics/Cargo.toml`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/05-GATE-EVOLUTION.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Parse a small Rust file, assert item count and kinds
- [ ] Complexity calculation for a function with 5 nested if-else branches

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: EVAL_13 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Parse a small Rust file, assert item count and kinds
- Complexity calculation for a function with 5 nested if-else branches
- No files outside the Write Scope are modified.
- Commit message contains `tracker: EVAL_13 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
