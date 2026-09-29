+++
id = "bug-1cb461"
kind = "bug"
title = "Aggregator truncates content by byte index and can panic on non-ASCII text"
status = "open"
triage = "verified"
severity = "p2"
size = "S"
goal = "visibility"
subsystem = ["roko-serve/aggregator"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "a17d9d766"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-serve/src/routes/aggregator.rs::list_knowledge_entries"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[repro]]
command = '! grep -q "content\[\.\.80\]" crates/roko-serve/src/routes/aggregator.rs'

[[verify]]
command = "! grep -q 'content\\[\\.\\.80\\]' crates/roko-serve/src/routes/aggregator.rs && grep -qw 'fn knowledge_label_truncates_on_char_boundary' crates/roko-serve/src/routes/aggregator.rs && cargo test -p roko-serve routes::aggregator::tests::knowledge_label_truncates_on_char_boundary"
+++

## Problem

`GET /api/knowledge/entries` (roko serve) builds a short label for each knowledge entry with
`format!("{}…", &e.content[..80])` when `e.content.len() > 80`. `len()` and `[..80]` count bytes. If byte 80
falls inside a multi-byte UTF-8 character (accented letters, CJK, emoji, `…` itself), the slice panics:
"byte index 80 is not a char boundary". The request then fails instead of returning the list.

Repro: store a knowledge entry whose content is 79 ASCII bytes followed by `é` (or any text with a multi-byte
character straddling byte 80), then request `GET /api/knowledge/entries`.
Expected: a label of at most 80 characters plus `…`, and a 200 response.

## Why it matters

- Goal `visibility` (serve, dashboard, portal). One entry with non-ASCII content breaks the whole knowledge
  list for the portal and any API client, and the response is cached under
  `aggregator:knowledge:entries`, so it cannot be served until the handler succeeds.
- Knowledge content comes from distilled agent output and user notes, so non-ASCII text is normal.

## Where

- `crates/roko-serve/src/routes/aggregator.rs::list_knowledge_entries` (line ~441; the slice is at line 455):
  reads all entries from `KnowledgeStore::for_layout(&state.layout)` and maps them to JSON rows
  `{id, domain, citations, label, confidence}`.
- Mounted by `aggregator::routes()` (`/knowledge/entries`, line 55), merged into the `/api` router in
  `crates/roko-serve/src/routes/mod.rs` (~line 340, nested at ~491).
- Tests module: `crates/roko-serve/src/routes/aggregator.rs` line ~1270 (`mod tests`).

## Current state

- Unchanged at HEAD; it is the only byte-index slice in `aggregator.rs`. Recent commits on the file
  (`5402a6567`, `c0f137075`) did not touch it.
- No shared char-safe truncation helper exists in `roko-core` or `roko-serve`; several crates have private
  copies (`truncate_chars` in roko-learn, roko-gate, roko-cli; `truncate_for_prompt` in
  `routes/research.rs:529`; `truncate_utf8` in `roko-graph/src/cells/task_executor.rs:553`).

## Plan

1. Extract the label into a small function in `aggregator.rs`, e.g.
   `fn knowledge_label(content: &str) -> String`:
   `match content.char_indices().nth(80) { Some((idx, _)) => format!("{}…", &content[..idx]), None => content.to_string() }`.
   This truncates at 80 characters, never inside a character. ASCII labels are unchanged.
2. Use it in `list_knowledge_entries` in place of the `if e.content.len() > 80 { .. }` block.
3. Add `knowledge_label_truncates_on_char_boundary` to `mod tests`: content with a multi-byte character across
   byte 80 (for example `"a".repeat(79) + "é" + "tail"`), plus a CJK or emoji string longer than 80 chars;
   assert no panic, the label ends with `…`, and it has at most 81 chars. Also assert a short string is
   returned unchanged.

## Done when

- `GET /api/knowledge/entries` returns 200 when an entry's content has a multi-byte character at byte 80.
- The new unit test passes.
- Verify (the current command runs a test filter that does not exist yet, so it would pass on the static check
  alone once the slice text changes). Suggested replacement:
  `! grep -q 'content\[\.\.80\]' crates/roko-serve/src/routes/aggregator.rs && grep -qw 'fn knowledge_label_truncates_on_char_boundary' crates/roko-serve/src/routes/aggregator.rs && cargo test -p roko-serve routes::aggregator::tests::knowledge_label_truncates_on_char_boundary`

## Notes

- Small, self-contained change in one handler; safe to do in parallel with other work unless someone else is
  editing `aggregator.rs`.
- Do not add a new workspace-wide helper for this; a local function is enough (and matches how other crates
  do it). A shared helper is a separate cleanup.

## Original notes

`format!("{}…", &e.content[..80])` (`routes/aggregator.rs:455`) slices a `String` at byte 80.
If byte 80 falls inside a multi-byte UTF-8 character (accents, CJK, emoji) the slice panics and the request fails.
Fix: truncate on a char boundary (e.g. `char_indices().nth(80)`) and add a test with multi-byte content.
