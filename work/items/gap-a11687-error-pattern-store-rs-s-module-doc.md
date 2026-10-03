+++
id = "gap-a11687"
kind = "gap"
title = "error_pattern_store.rs's module doc comment still calls record_resolution mark_resolved"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-learn/error-patterns"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "wave-6 follow-up reports 2026-10-03 (PK35 gap-943046)"
discovered_from = "gap-943046"
anchors = ["crates/roko-learn/src/error_pattern_store.rs::record_resolution"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'mark_resolved' crates/roko-learn/src/error_pattern_store.rs"
+++

## Problem

`crates/roko-learn/src/error_pattern_store.rs`'s own module-level doc comment describes a `mark_resolved`
method that no longer exists:

- Line 21: "5. After a fix is confirmed, `ErrorPatternStore::mark_resolved` annotates the..."
- Line 51: "...on crash. There is no upper bound on pattern count, but `mark_resolved` and..."

The actual method is `record_resolution` (line 501: `pub fn record_resolution(&mut self, key: &str, resolution:
&str, resolved_by: &str) -> bool`). There is no `fn mark_resolved` anywhere in the file; the method was renamed
(or replaced) without updating either doc-comment reference.

## Why it matters

The module doc comment is the first thing anyone reading `error_pattern_store.rs` (or generating docs from it)
sees; it currently describes an API surface that doesn't exist, which sends the next reader looking for a
method that was never there under that name.

## Where

- `crates/roko-learn/src/error_pattern_store.rs` lines 21 and 51 (module doc comment, the numbered lifecycle
  list and the storage-bound note).
- The real method: `record_resolution` (line 501).

## Current state

Unfixed — confirmed by direct read at HEAD; `grep -rn "fn mark_resolved" crates/` matches nothing in the
workspace.

## Plan

Replace both `mark_resolved` references in the module doc comment with `record_resolution`, matching the
method's actual name and signature.

## Done when

- `error_pattern_store.rs`'s module doc comment names `record_resolution`, not `mark_resolved`.
- The `[[verify]]` command passes.

## Notes

- Docs-only (a doc comment inside the source file, not `docs/v3/` or `CLAUDE.md`) — this is a different, smaller
  finding from PK35's CLAUDE.md/docs/v3 gaps (`[experiments]`, `--no-holdout`, `roko learn patterns`), which are
  noted separately on `gap-d2c64f` and `gap-fd9dfb`.
