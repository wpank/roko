# Code Intelligence: Goals

## End State

Persistent code index with symbol graphs, PageRank, and HDC fingerprints. MCP server exposes all 10 tools. Code context automatically injected into agent prompts based on task relevance.

## Key Properties

- **Persistent index**: Symbol graph survives across sessions (not rebuilt every time).
- **Auto-enrichment**: Agent prompts automatically include relevant code context from the index.
- **HDC similarity active**: Fingerprint-based retrieval for "code like this" in prompt assembly.
- **10 MCP tools fully wired**: All code-intelligence MCP tools accessible from ACP and CLI agents.
- **Cross-language support**: Rust, TypeScript, Go parsers all feeding the same unified index.
- **Agent following trace**: When agent navigates codebase, build call graph synthesis from traversal pattern.

## What Exists Today

- roko-index: parser + graph + HDC indexing (~9,026 LOC, 164 tests)
- roko-mcp-code: 10 advertised MCP tools + 4 undocumented tools (symbol_lookup, call_graph, imports, semantic_search)
- Symbol graphs + 3 PageRank variants (standard, personalized, weighted) + HDC fingerprints (built)
- SQLite persistent backend (built in `sqlite.rs`, feature-gated, not instantiated at runtime)
- `code_context_for_task()` exists but is duplicated identically in `prompt_helpers.rs:206` and `dispatch_helpers.rs:699`
- Language support: roko-lang-rust (+ tree-sitter feature), roko-lang-typescript, roko-lang-go
- Auto-enrichment wired: `cached_code_index()` (60s TTL) feeds `code_context_for_task()` at dispatch time
- Hybrid search used in prompt assembly, but HDC sub-query is disabled (`hdc: None`)

## From v2 UX Showcase (9 Scenarios)

- **CallGraph card** (follow): Tree visualization synthesized from agent's file traversal: LoginForm.handleSubmit → api.login → POST /api/auth/login → users.findByEmail → bcrypt.compare → signToken. Each node has fn name + file:line reference.
- **Agent cursor tracking** (follow): EditorPeek shows "following · L11" with crosshair icon, highlighted line in code, agent-traced info marks on gutter.
- **AGENT NAVIGATES dividers** (follow): Between file reads, "→ SRC/AUTH/API.TS" markers showing agent's navigation path.
- **Editor gutter marks** (pipeline, follow): Gate-derived or trace-derived marks at specific lines — ok/error/warn/info dots with tooltip messages.

### Data Feeds Required
- `CallGraphNode` — fn_name, file_path, line_number, depth (tree level)
- `AgentTraversal` — ordered list of (file, line_range, action, timestamp)
- `CursorPosition` — current file, current line, following indicator
- `GutterMark` — line_number, kind (ok/error/warn/info), message

## Gap

- No persistent index at runtime (SQLite backend built but never instantiated; index rebuilt from scratch unless cached)
- HDC sub-query disabled in prompt assembly (`hdc: None` in both `code_context_for_task()` copies)
- `code_context_for_task()` duplicated in `prompt_helpers.rs` and `dispatch_helpers.rs` — needs a single canonical source
- MCP tools not accessible from ACP (only from direct MCP connection)
- No agent traversal trace / call graph synthesis
- Personalized PageRank and weighted PageRank built and exported but never called at runtime

---

## Sources

- `/Users/will/dev/nunchi/roko/roko/crates/roko-index/src/workspace.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-index/src/sqlite.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-mcp-code/src/lib.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/prompt_helpers.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch_helpers.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs`
