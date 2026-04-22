# Code Intelligence Subsystem Audit

Symbol graphs, HDC fingerprints, 3 language providers, 10 MCP tools — a solid code intelligence system that's under-utilized at runtime.

## The Problem

The code intelligence subsystem (~9,000 LOC, 164 tests) is well-built: symbol extraction, PageRank scoring, HDC similarity, 10 MCP tools. But at runtime, the prompt context injection path uses Hybrid search with HDC disabled (hdc: None). HDC similarity in the primary code path, personalized PageRank, and structural search are built but not wired. Also: `code_context_for_task()` is duplicated in two files.

---

## 1. Crate Overview

| Crate | LOC | Files | Purpose | Status |
|---|---|---|---|---|
| roko-index | ~4,575 | 7 | Parser, symbol graph, HDC, workspace index, SQLite backend | Active |
| roko-lang-rust | ~905 | 2 | Rust language provider (heuristic) | Active, 39 tests |
| roko-lang-typescript | ~938 | 1 | TypeScript/JS provider (heuristic) | Active, 33 tests |
| roko-lang-go | ~673 | 1 | Go provider (heuristic) | Active, 25 tests |
| roko-mcp-code | ~1,935 | 2 | Code intelligence MCP server (10 advertised + 4 undocumented tools) | Active |
| **Total** | **~9,026** | | | **164 tests** |

---

## 2. LanguageProvider Trait

```rust
trait LanguageProvider {
    fn language_name(&self) -> &str;
    fn file_extensions(&self) -> &[&str];
    fn parse_imports(&self, source: &str) -> Vec<Import>;
    fn extract_symbols(&self, source: &str) -> Vec<Symbol>;
}
```

### Implementations

| Language | Provider | Parsing | Symbol Types | Limitations |
|---|---|---|---|---|
| Rust | RustLanguageProvider | Line-by-line heuristic | fn, struct, enum, trait, impl, const, type, mod | Single-line only; misses multi-line signatures |
| TypeScript | TypeScriptLanguageProvider | Line-by-line heuristic | function, class, interface, type, const, enum, export | No JSX/TSX attribute extraction |
| Go | GoLanguageProvider | Line-by-line heuristic | func (with receiver), type, const, var | No interface method extraction |

**Missing:** Python, C/C++, Java providers not built.

**Tree-sitter:** Feature-gated for Rust (`tree_sitter_parser.rs`). The `tree-sitter` feature is defined in `roko-lang-rust/Cargo.toml` but not in `default = []`, so it is not compiled unless explicitly enabled.

---

## 3. Symbol Graph (roko-index/graph.rs)

**Edge kinds:** Calls, Imports, Implements, Contains, TypeRef

**Edge detection:** Regex-based heuristic:
- `CALL_RE`: `\b([A-Za-z_][A-Za-z0-9_]*)\s*\(` — function calls
- `TYPE_REF_RE`: `\b([A-Z][A-Za-z0-9_]*)\b` — type references

**PageRank scoring:**
- Standard unweighted PageRank (default)
- `personalized_pagerank()` — seed-aware variant (built, never called)
- `weighted_pagerank()` — weighted variant (built, unused)

**Accuracy:** High false-negative rate (misses indirect calls, dynamic dispatch), very low false-positive rate.

---

## 4. HDC Fingerprints (roko-index/hdc.rs)

**Vector:** 10,240-bit (160 × u64 words)

**Computation:**
1. Role vector from symbol kind (fn/struct/trait/etc.)
2. Name vector from character trigrams (FNV-1a hash → Splitmix64 PRNG)
3. Context vector from surrounding code
4. Bind (XOR) role + name + context
5. Bundle (majority vote) all symbol fingerprints for file-level fingerprint

**Similarity:** Normalized Hamming distance: `1.0 - (hamming_dist / 10240)`

**Used in:** MCP `find_similar_patterns` tool (via `hdc_search()`) and MCP `semantic_search` tool (via `semantic_search()` which computes a query fingerprint from the raw text and uses HDC for scoring). **Disabled** in prompt assembly code path (`hdc: None` in both `code_context_for_task()` implementations).

---

## 5. Workspace Index (roko-index/workspace.rs)

**In-memory index:**
```rust
WorkspaceIndex {
    root: PathBuf,
    files_by_path: HashMap,      // path -> SourceFile
    file_paths: HashSet,
    imports_by_file: HashMap,    // path -> Vec<Import>
    symbols_by_name: HashMap,    // name -> Vec<SymbolInfo>
    functions_by_name: HashMap,  // name -> Vec<SymbolInfo> (functions only)
    symbols_by_id: HashMap,      // SymbolId -> SymbolInfo
    file_fingerprints: HashMap,  // path -> HdcFingerprint
    symbol_fingerprints: HashMap,// SymbolId -> HdcFingerprint
    pagerank_scores: HashMap,    // SymbolId -> f64
    graph: SymbolGraph,
}
```

Note: there is no `symbols_by_kind` field; kind-based filtering happens at query time inside `structural_search()`.

### 5 Search Strategies

| Strategy | How | Used at Runtime | Notes |
|---|---|---|---|
| Keyword | Substring match + PageRank bonus (up to +0.2) | Via Hybrid path | Direct `keyword_search()` also callable |
| Structural | Filter by kind, visibility, min PageRank | MCP/CLI only | Not in prompt assembly |
| HDC | Hamming distance similarity | MCP `find_similar_patterns`, `semantic_search` | Disabled in prompt assembly (hdc=None in Hybrid) |
| Embedding | Falls back to keyword (no dense embeddings) | Placeholder | Not implemented |
| Hybrid | Keyword + structural + HDC with RRF ranking | prompt assembly, MCP `get_context` | Used in both `prompt_helpers.rs` and `dispatch_helpers.rs`, but hdc sub-query is None so effective search is keyword-only |

### Index Building

- `WorkspaceIndex::load()` scans workspace, parses all .rs/.ts/.js/.go files
- Always built fresh in-memory — no persistent cache
- orchestrate.rs: 60-second staleness cache
- CLI commands: rebuilt every invocation
- MCP server: once at startup

---

## 6. MCP Server (roko-mcp-code)

10 tools advertised via `tools/list` (JSON-RPC 2.0 over stdio). 4 additional tools are handled by the dispatcher but not advertised:

| Tool | Advertised | Purpose |
|---|---|---|
| `search_code` | Yes | 5 search strategies (keyword, structural, HDC, embedding, hybrid) |
| `get_symbol_context` | Yes | Definition + call graph (callers/callees) for uniquely-resolved function symbols |
| `get_file_ast` | Yes | Symbol-level file structure |
| `find_similar_patterns` | Yes | HDC similarity search via `semantic_search()` |
| `get_index_stats` | Yes | File/symbol/edge counts, languages, top PageRank |
| `find_references` | Yes | Graph-backed reference finder |
| `find_implementations` | Yes | Trait/interface implementors (impl-block heuristic) |
| `get_callers` | Yes | Direct + transitive caller chain |
| `workspace_map` | Yes | Crate/module/symbol navigation |
| `get_context` | Yes | Auto-assemble relevant context for a task (Hybrid keyword + semantic) |
| `symbol_lookup` | No | Exact symbol name lookup |
| `call_graph` | No | Call graph for a function with depth |
| `imports` | No | List imports for a file |
| `semantic_search` | No | Direct `semantic_search()` call |

**Entry point:** `ROKO_WORKSPACE_ROOT` env var or cwd. Binary: `roko-mcp-code`.

---

## 7. Runtime Integration

### How Index Is Used Today

1. **orchestrate.rs** (active): `cached_code_index()` with 60s TTL → `code_context_for_task()` (from `prompt_helpers.rs`) → `SearchStrategy::Hybrid` with keyword sub-query, hdc=None → top 15 results → inject into prompt
2. **CLI:** `roko index build/search/stats` commands
3. **MCP server:** 10 advertised tools + 4 undocumented tools available to agents

### What's NOT Used

- HDC similarity in prompt assembly (`hdc: None` in both `prompt_helpers.rs` and `dispatch_helpers.rs` hybrid queries)
- Personalized PageRank (built in `graph.rs`, exported from `lib.rs`, never called at runtime)
- Weighted PageRank (same — built, exported, never called)
- Structural search in prompt assembly
- Dense embeddings (not computed; `embedding_search()` falls back to keyword)
- Persistent index (SQLite backend in `sqlite.rs` is feature-gated behind `sqlite`, never instantiated at runtime)

---

## 8. Anti-Patterns

| Anti-Pattern | Where | Impact |
|---|---|---|
| **Duplicate `code_context_for_task()`** | `prompt_helpers.rs:206` AND `dispatch_helpers.rs:699` | Identical implementations in 2 files — fixes must be applied twice |
| **HDC disabled in prompt assembly** | Both `code_context_for_task()` impls use `hdc: None` in Hybrid query | HDC similarity never used in prompt context injection |
| **Regex-based edge detection** | `graph.rs` | Brittle; misses indirect calls, dynamic dispatch |
| **No persistent index at runtime** | `workspace.rs` + `sqlite.rs` | SQLite backend exists but is never instantiated; index is rebuilt from scratch on every call when not cached |
| **No incremental updates** | `WorkspaceIndex::load()` | Full re-index even if one file changed |
| **Single workspace** | MCP server reads one env var | No multi-workspace support |

---

## 9. File Inventory

| File | LOC | Status |
|---|---|---|
| `roko-index/src/lib.rs` | 47 | Module exports, re-exports |
| `roko-index/src/parser.rs` | 142 | Language-agnostic parser delegation |
| `roko-index/src/symbol.rs` | 215 | SymbolId, SymbolRef, find_symbol |
| `roko-index/src/graph.rs` | 1,400 | Symbol graph, 3 PageRank variants, edge kinds, rkyv snapshots |
| `roko-index/src/hdc.rs` | 355 | 10,240-bit HDC fingerprints |
| `roko-index/src/workspace.rs` | 1,916 | WorkspaceIndex, 5 search strategies, CodeIndex trait, CallGraph |
| `roko-index/src/sqlite.rs` | 500 | SQLite persistent backend (feature-gated, not used at runtime) |
| `roko-lang-rust/src/lib.rs` | 905 | CargoBuildSystem + RustLanguageProvider |
| `roko-lang-rust/src/tree_sitter_parser.rs` | (feature-gated) | Tree-sitter Rust parser (feature `tree-sitter`, not in defaults) |
| `roko-lang-typescript/src/lib.rs` | 938 | Npm/Pnpm/Yarn + TypeScriptLanguageProvider |
| `roko-lang-go/src/lib.rs` | 673 | GoBuildSystem + GoLanguageProvider |
| `roko-mcp-code/src/lib.rs` | 1,930 | MCP server, 10 advertised + 4 undocumented tools |
| `roko-mcp-code/src/main.rs` | 5 | Binary entry point |

---

## Sources

- `/Users/will/dev/nunchi/roko/roko/crates/roko-index/src/lib.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-index/src/graph.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-index/src/hdc.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-index/src/workspace.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-index/src/parser.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-index/src/symbol.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-index/src/sqlite.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-index/Cargo.toml`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-lang-rust/src/lib.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-lang-rust/Cargo.toml`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-lang-typescript/src/lib.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-lang-go/src/lib.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-mcp-code/src/lib.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-primitives/src/hdc.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/prompt_helpers.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch_helpers.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs`
