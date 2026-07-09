# Prompt: 15-code-intelligence

You are a fresh Claude Opus agent. Zero prior context. Read every file this prompt references before writing.

## Your mission

Generate `/Users/will/dev/nunchi/roko/roko/docs/15-code-intelligence/`. Covers incremental code indexing (tree-sitter), symbol extraction and directed dependency graph, PageRank for symbol importance, HDC fingerprints for structural similarity, context assembly from code search, MCP context server design, index.db scaling and snapshot optimization. Framed as `roko-index` crate.

## Step 1 — Context pack (MANDATORY)

Read all 7 files in `/Users/will/dev/nunchi/roko/roko/tmp/prd-migration/context-pack/` in order.

## Step 2 — refactoring-prd canonical sources

1. `/Users/will/dev/nunchi/roko/refactoring-prd/05-agent-types.md` §2 Coding Agent, §Niche Construction
2. `/Users/will/dev/nunchi/roko/refactoring-prd/00-overview.md` §Crate Map (roko-index status)
3. `/Users/will/dev/nunchi/roko/refactoring-prd/08-translation-guide.md`

## Step 3 — SOURCE-INDEX entry `## 15-code-intelligence.md`

Key legacy:
- `bardo-backup/prd/15-dev/06-indexer.md`
- `bardo-backup/prd/07-tools/13-tools-intelligence.md`
- `bardo-backup/tmp/mori-refactor/10-code-intelligence.md`
- `bardo-backup/tmp/mori-agents/18-code-intelligence-and-gateway.md`
- `bardo-backup/tmp/death/tools/02-code-index.md` (extract mechanism)
- `bardo-backup/tmp/death/docs/30-index-performance.md` (extract mechanism)

## Step 4 — active code

- Glob `/Users/will/dev/nunchi/roko/roko/crates/roko-index/src/**/*.rs`
- Read: `lib.rs`, `hdc.rs`, `parser.rs` (if exists), `graph.rs` (if exists), language-specific files
- Glob `/Users/will/dev/nunchi/roko/roko/crates/roko-lang-rust/src/**/*.rs`
- Glob `/Users/will/dev/nunchi/roko/roko/crates/roko-lang-typescript/src/**/*.rs`
- Glob `/Users/will/dev/nunchi/roko/roko/crates/roko-lang-go/src/**/*.rs`

## Step 5 — Output and sub-doc plan

```bash
mkdir -p /Users/will/dev/nunchi/roko/roko/docs/15-code-intelligence
```

Write **11 sub-docs** plus `INDEX.md`:

| # | Filename | Content |
|---|---|---|
| 00 | `00-vision.md` | roko-index vision. Why code intelligence matters for coding agents. Relationship to context engineering (03-composition) and niche construction (01-orchestration §07). |
| 01 | `01-tree-sitter-parsing.md` | Tree-sitter incremental parsing. Per-language grammar. Language-specific sub-crates: roko-lang-rust, roko-lang-typescript, roko-lang-go. How each language adapts the base interface. |
| 02 | `02-symbol-extraction.md` | Extracting symbols (functions, types, traits, methods, modules) from the AST. Symbol metadata (signature, location, visibility, doc comments). Symbol IDs. |
| 03 | `03-dependency-graph.md` | Directed dependency graph between symbols. Call graphs. Import graphs. Type dependency. Trait implementation relationships. Graph storage format. |
| 04 | `04-pagerank-symbol-importance.md` | PageRank over the dependency graph. Why PageRank works: important symbols are referenced by other important symbols. Use cases: ranking context search results by importance, identifying "god classes." |
| 05 | `05-hdc-fingerprints.md` | HDC fingerprints for structural similarity. BIND(symbol_name, symbol_kind, containing_scope) → vector. Why HDC for code: detect refactors, find similar functions, cross-language structural search. Integration with 10,240-bit BSC from roko-primitives. |
| 06 | `06-context-assembly-from-code.md` | How indexed code becomes LLM context. Query by task description → semantic + HDC search → rank by PageRank + relevance → pack under budget. Integration with roko-compose Composer (cross-reference 03-composition.md). |
| 07 | `07-mcp-context-server.md` | MCP context server design. Exposing code intelligence as MCP tools for agents. `roko-mcp-stdio` integration. Tool schemas. |
| 08 | `08-index-db-scaling.md` | Index.db storage format. Scaling to large codebases. Update strategies (incremental vs full rebuild). |
| 09 | `09-snapshot-optimization.md` | Snapshot format for persistence. Fast load. Differential updates. |
| 10 | `10-current-status-and-gaps.md` | roko-index built. Language crates built for rust/ts/go. HDC fingerprints implemented. What's missing: MCP context server, benchmarks on large codebases, integration with roko-compose. |

Plus `INDEX.md`.

## Step 7-9 — Rules, INDEX, self-check

Per context-pack rules. ≥200 lines per sub-doc, ≥2500 total (allow lower citation count — this topic is implementation-focused). Citations: PageRank (Page et al.), tree-sitter paper, HDC/VSA (Kanerva, Kleyko).

Cross-reference 00-architecture (HDC), 03-composition (code-as-context), 18-tools (MCP servers).

## CRITICAL REMINDERS

- DO NOT SUMMARIZE. DO NOT TRUNCATE.
- Frame as `roko-index` crate design, not `mori-index`.
- Apply naming map: mori-index → roko-index; mori-context → roko-compose/roko-index; mori-mcp → roko MCP.
- Use Write tool. Don't ask questions.
