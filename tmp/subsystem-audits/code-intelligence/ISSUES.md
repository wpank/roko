# Code Intelligence: Issues

Verified against source code. Each issue cites the exact file and line where the problem lives.

---

## I-1: Duplicate `code_context_for_task()`

**Severity:** Medium

`code_context_for_task()` is defined identically in two files:
- `crates/roko-cli/src/prompt_helpers.rs:206`
- `crates/roko-cli/src/dispatch_helpers.rs:699`

Both have identical logic: build a `SearchStrategy::Hybrid` with keyword sub-query, `structural: None`, `hdc: None`, call `index.search()` for top 15 results, format as markdown bullets. `orchestrate.rs` imports from `prompt_helpers.rs` (line 180). The `dispatch_helpers.rs` copy is a dead clone.

**Fix:** Delete `dispatch_helpers.rs:699` version; update any callers to use the `prompt_helpers` version.

---

## I-2: HDC Disabled in Prompt Assembly

**Severity:** Medium

Both `code_context_for_task()` implementations use:
```rust
let strategy = roko_index::SearchStrategy::Hybrid {
    keyword: Some(...),
    structural: None,
    hdc: None,          // <-- HDC disabled
};
```

The `WorkspaceIndex` builds HDC fingerprints for every symbol at construction time
(`workspace.rs:symbol_fingerprints`). They are fully functional — `semantic_search()` and
`find_similar_patterns` MCP tool use them. But the prompt assembly path never uses them.

**Fix:** Pass an `HdcQuery` anchored on a fingerprint derived from the task description text.

---

## I-3: SQLite Persistent Backend Never Instantiated

**Severity:** Low

`roko-index/src/sqlite.rs` (500 LOC) implements a full SQLite-backed persistent index with:
- WAL mode, content-addressed file tracking (blake3), incremental updates
- Symbol and edge upserts
- `UpdateStats` tracking files_updated / files_skipped

It is feature-gated behind `sqlite` in `Cargo.toml`. It is never instantiated at runtime.
The `WorkspaceIndex::load()` path always rebuilds from scratch. The `CodeIndex` trait in
`workspace.rs` was presumably designed to allow plugging in a persistent backend, but
`SqliteIndex` does not implement `CodeIndex` yet.

**Fix:** Implement `CodeIndex` for `SqliteIndex`; expose `--persistent-index` flag or
auto-detect `.roko/index.db`.

---

## I-4: Personalized PageRank and Weighted PageRank Never Called

**Severity:** Low

`roko-index/src/graph.rs` implements three PageRank variants, all exported from `lib.rs`:
- `pagerank()` — used (called from `workspace.rs:1018`)
- `personalized_pagerank()` — exported, never called at runtime
- `weighted_pagerank()` — exported, never called at runtime

**Fix:** Wire `personalized_pagerank()` for task-specific seed symbols when a task description
references known symbols.

---

## I-5: 4 Undocumented MCP Tools

**Severity:** Low

`dispatch_tool_call()` in `roko-mcp-code/src/lib.rs` handles 14 tools but `handle_tools_list()`
only advertises 10. The 4 undocumented tools are:
- `symbol_lookup`
- `call_graph`
- `imports`
- `semantic_search`

Agents using `tools/list` to discover capabilities will not find these.

**Fix:** Either add them to `handle_tools_list()` with proper schemas, or remove them from the
dispatcher if they are superseded by the advertised equivalents.

---

## Sources

- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/prompt_helpers.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch_helpers.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-index/src/workspace.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-index/src/graph.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-index/src/sqlite.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-mcp-code/src/lib.rs`
