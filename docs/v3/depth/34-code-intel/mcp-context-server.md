# MCP Context Server

> **Parent:** [34-CODE-INTELLIGENCE](../../34-CODE-INTELLIGENCE.md) Section 11

---

## Overview

The `roko-mcp-code` crate (2,330 lines, 16 tests) exposes code intelligence
to agents via Model Context Protocol (MCP) tools. Rather than requiring
agents to understand the `SymbolGraph` API, PageRank algorithm, or HDC
fingerprint system directly, the MCP server presents ten high-level tools
accessible via stdio JSON-RPC.

---

## The Ten MCP Tools

| Tool | Purpose | Key params |
|------|---------|-----------|
| `search_code` | Multi-strategy code search | `query`, `strategy`, `max_results`, `file_pattern`, `kind_filter` |
| `get_symbol` | Detailed symbol context | `name`, `file_path`, `include_dependencies`, `include_callers` |
| `get_file_ast` | Symbol-level file structure | `file_path`, `include_bodies` |
| `find_similar` | HDC similarity search | `reference`, `min_similarity`, `max_results` |
| `get_stats` | Index health and coverage | (none) |
| `find_references` | Symbol usage sites | `symbol_name`, `file_path`, `include_definitions` |
| `find_implementations` | Trait/interface implementations | `trait_name`, `include_methods` |
| `get_callers` | Call graph traversal | `function_name`, `transitive`, `max_depth` |
| `workspace_map` | Structural workspace overview | `depth`, `focus_path` |
| `get_context` | Automated context assembly | `task`, `token_budget`, `include_tests` |

### Tool 1: search_code

The primary entry point for code search. Combines multiple search strategies
via RRF. Supports `keyword`, `structural`, `hdc`, `embedding`, and `hybrid`
strategies. Returns ranked results with file paths, line numbers, scores,
and code snippets.

### Tool 2: get_symbol

Retrieves full context for a specific symbol including definition,
dependencies (forward edges), callers (reverse edges), PageRank score, and
HDC fingerprint similarity to task context. Supports disambiguation via
`file_path` when multiple symbols share a name.

### Tool 3: get_file_ast

Returns the symbol-level structure of a source file -- a "table of contents"
view. Optionally includes function bodies. Useful for understanding file
organization without reading every line.

### Tool 4: find_similar

Finds code structurally similar to a given snippet or symbol via HDC
fingerprint comparison. Configurable minimum similarity threshold and
maximum results.

### Tool 5: get_stats

Reports index health and coverage: indexed file count, total symbols, total
edges (broken down by kind), language distribution, top symbols by
PageRank, last indexed timestamp, and index size.

### Tool 6: find_references

Finds all locations where a symbol is referenced (imported, called, or
mentioned). Optionally includes definition sites alongside usage sites.

### Tool 7: find_implementations

Finds all types that implement a given trait or interface. Optionally
includes method signatures for each implementor. Critical for understanding
trait hierarchies.

### Tool 8: get_callers

Finds all symbols that call a given function. Supports transitive mode
(callers of callers) with configurable `max_depth`. Results ordered by
graph distance.

### Tool 9: workspace_map

Generates a high-level map of the workspace structure -- inspired by
Aider's "repository map" concept. Supports three depth levels: `crate`,
`module`, and `symbol`. Optional `focus_path` narrows the map to a specific
area.

### Tool 10: get_context

The meta-tool: given a task description and token budget, automatically
assembles the optimal context block. Runs the full context assembly pipeline
(search, rank, expand, slice, budget). Returns a ready-to-insert context
block.

---

## Server Architecture

The MCP server follows the standard MCP stdio transport pattern:

```toml
[agent.mcp_config.servers.code-intelligence]
command = "roko-mcp-code"
```

### Startup sequence

1. Discover workspace root (via `.roko/` marker or `--root` flag)
2. Build a `WorkspaceIndex` from source files (enumerates providers,
   parses all files, builds graph, computes fingerprints and PageRank)
3. Enter stdio JSON-RPC loop: read requests, dispatch to tool handlers,
   write responses

### Request handling

```rust
impl CodeIntelligenceServer {
    pub async fn handle_tool_call(
        &self,
        tool_name: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value> {
        match tool_name {
            "search_code" => self.search_code(params).await,
            "get_symbol" => self.get_symbol_context(params).await,
            "get_file_ast" => self.get_file_ast(params).await,
            "find_similar" => self.find_similar(params).await,
            "get_stats" => self.get_stats(params).await,
            "find_references" => self.find_refs(params).await,
            "find_implementations" => self.find_impls(params).await,
            "get_callers" => self.get_callers(params).await,
            "workspace_map" => self.workspace_map(params).await,
            "get_context" => self.get_context(params).await,
            _ => Err(Error::UnknownTool(tool_name.into())),
        }
    }
}
```

### Index lifecycle

- **Startup** -- Build fresh index from workspace sources
- **Query serving** -- Handle tool calls from agents
- **Planned: background re-index** -- Watch for file changes and update
  incrementally via `notify::RecommendedWatcher`
- **Planned: snapshot persistence** -- Persist index to disk on shutdown

---

## Security

All tool inputs are validated:

- **File paths** -- Must be within the workspace directory. Path traversal
  (`../../../etc/passwd`) is blocked via `canonicalize()` comparison.
- **Result limits** -- Capped to prevent response explosion.
- **Token budgets** -- Capped to prevent memory exhaustion.
- **FTS5 queries** -- Sanitized for SQL injection (alphanumeric, spaces,
  quotes, asterisks, dashes only).

### Rate limiting (planned)

Per-agent sliding window rate limiter to prevent excessive queries:

```toml
[mcp.code_intelligence]
max_concurrent_queries = 16
query_timeout_ms = 5000
rate_limit_per_agent = 100
debounce_ms = 500
```

---

## Comparison with Built-in Tools

| Existing tool | MCP equivalent | Difference |
|--------------|---------------|-----------|
| `file_read` | `get_file_ast` | MCP returns structured AST, not raw text |
| `file_search` | `search_code` | MCP uses multi-strategy search with ranking |
| `grep_search` | `search_code` (keyword) | MCP adds structural and similarity search |
| (none) | `get_callers` | New: graph-based caller analysis |
| (none) | `find_implementations` | New: trait implementation discovery |
| (none) | `workspace_map` | New: structural workspace overview |
| (none) | `get_context` | New: automated context assembly |

The MCP tools augment built-in tools. An agent might use `file_read` for
raw file access and `get_symbol` for structured code understanding in the
same task.

---

## Index Sharing Between Agents

Multiple agents can query the same index concurrently via separate MCP
server instances or (planned) shared-memory access:

```rust
pub struct CodeIndex {
    pool: r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>,
    cache: parking_lot::RwLock<IndexCache>,
}
```

Read operations acquire a shared read lock (concurrent). Write operations
(re-indexing) acquire an exclusive write lock (brief blocking). SQLite WAL
mode ensures reads never block writes.

---

## Verified Behaviors (16 tests)

- Tool listing and schema validation
- Each of the 10 tools returns expected structure
- Error handling for unknown tools
- File path validation (workspace containment)
- Parameter validation (missing required fields)

---

## Academic Foundations

- **Model Context Protocol**: Anthropic (2024). Standardized tool interface
  between LLMs and external services.
- **Aider repository map**: Gauthier (2024). `workspace_map` is directly
  inspired by Aider's tree-sitter-based repository maps.
- **Language Server Protocol**: Microsoft (2016). LSP tools
  (`textDocument/references`, `textDocument/implementation`) inspired
  `find_references`, `find_implementations`, and `get_symbol`.
- **code2seq**: Alon, Brody, Levy, and Yahav (2018). *ICLR*. Structured
  code representations improve code understanding.

---

## Cross-References

- See [context-assembly-from-code.md](./context-assembly-from-code.md) for
  the assembly pipeline that `get_context` triggers
- See [index-db-scaling.md](./index-db-scaling.md) for the storage backend
- See [dependency-graph.md](./dependency-graph.md) for graph queries behind
  `get_callers` and `find_implementations`
- See Chapter 19 (Tools) for the broader tool system architecture
