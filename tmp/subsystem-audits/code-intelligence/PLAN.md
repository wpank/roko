# Code Intelligence: Work Plan

Ordered by impact/effort. All tasks derived from verified gaps in ISSUES.md.

---

## P-1: Deduplicate `code_context_for_task()` (Easy, High Impact)

**Issue:** I-1

1. Delete `dispatch_helpers.rs:699–773` (the duplicate).
2. Add `pub(crate) use crate::prompt_helpers::code_context_for_task;` in `dispatch_helpers.rs`
   if any caller there needs it, or update callers to import from `prompt_helpers`.
3. Verify `orchestrate.rs` still compiles and uses the `prompt_helpers` version (it already does
   at line 180).

**Verification:** `cargo build -p roko-cli`; grep confirms only one definition remains.

---

## P-2: Enable HDC in Prompt Assembly (Medium, High Impact)

**Issue:** I-2

In `prompt_helpers.rs::code_context_for_task()`:

1. Derive an anchor fingerprint from the task description:
   ```rust
   use roko_index::{fingerprint_file, SourceFile};
   let query_file = SourceFile { content: task_description.to_string(), .. };
   let anchor = fingerprint_file(&query_file);
   ```
2. Pass the anchor as the `hdc` sub-query:
   ```rust
   hdc: Some(roko_index::HdcQuery {
       anchor,
       min_similarity: 0.55,
       max_results: 15,
   }),
   ```
3. Remove the identical change from `dispatch_helpers.rs` after P-1 (only one copy to update).

**Verification:** Run `roko plan run` on a small plan; confirm prompt Layer 3 includes HDC hits.

---

## P-3: Advertise Undocumented MCP Tools (Easy, Low Impact)

**Issue:** I-5

In `handle_tools_list()` in `roko-mcp-code/src/lib.rs`, add `tool_spec` entries for:
- `symbol_lookup` (name: str) → exact symbol name lookup
- `call_graph` (function: str, depth: u32) → call graph neighborhood
- `imports` (file: str) → list imports for a file
- `semantic_search` (query: str, limit: u32) → HDC-based semantic search

**Verification:** MCP `tools/list` returns 14 tools.

---

## P-4: Instantiate SQLite Backend (Large, Medium Impact)

**Issue:** I-3

1. Implement `CodeIndex` trait for `SqliteIndex` in `sqlite.rs`.
2. In `WorkspaceIndex::load()`, check for `.roko/index.db`; if present, open `SqliteIndex`
   and run incremental update instead of full re-parse.
3. Fall back to in-memory full parse when no database exists.
4. Wire `roko index build` CLI command to create/update the SQLite index.

**Verification:** Second call to `cached_code_index()` returns immediately (no re-parse).

---

## P-5: Wire Personalized PageRank (Medium, Low Impact)

**Issue:** I-4

In `code_context_for_task()`, after extracting keywords, look up seed symbols and pass them
to `personalized_pagerank()` to boost PageRank for symbols related to the task context.

This requires passing the `SymbolGraph` to a re-ranked copy of search results before
truncation.

**Verification:** Top results for a task mentioning `PlanRunner` should rank `PlanRunner`
and its callees higher than unrelated symbols with similar names.

---

## Sources

- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/prompt_helpers.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch_helpers.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-index/src/workspace.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-index/src/graph.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-index/src/sqlite.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-index/src/hdc.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-mcp-code/src/lib.rs`
