# Context Assembly from Code Search

> **Parent:** [34-CODE-INTELLIGENCE](../../34-CODE-INTELLIGENCE.md) Section 10

---

## The Five Search Strategies

The `WorkspaceIndex` facade in `roko-index/src/workspace.rs` implements
the `CodeIndex` trait that combines five complementary search strategies:

| # | Strategy | What it finds | Speed |
|---|----------|-------------- |-------|
| 1 | **Keyword** | Exact text matches on symbol names | Fast (FTS5) |
| 2 | **Structural** | Symbols by kind, visibility, file pattern | Fast (SQL) |
| 3 | **HDC Similarity** | Structurally similar symbols | Fast (Hamming) |
| 4 | **Embedding** | Semantically similar code (planned) | Medium (ANN) |
| 5 | **Hybrid (RRF)** | Combined ranking from all strategies | Medium |

### Strategy 1: Keyword search

Traditional text search over symbol names, backed by SQLite FTS5 when
persistent storage is enabled. Excels when the agent knows the exact name:
"find the `build_graph` function."

### Strategy 2: Structural search

Query symbols by structural properties: kind, visibility, file pattern,
minimum PageRank score. Example: "find all public traits in `roko-core`."

### Strategy 3: HDC similarity search

Find symbols structurally similar to a query symbol or code snippet.
Fingerprints are compared via Hamming distance at ~50ns per comparison.

### Strategy 4: Embedding similarity search (planned)

Dense embeddings via fastembed/BGE-small-en-v1.5 (384 dimensions) for
semantic queries beyond structural similarity. "Find error handling code"
matches functions dealing with errors even if names do not contain "error."

### Strategy 5: Hybrid search with Reciprocal Rank Fusion

RRF (Cormack, Clarke, and Butt 2009) combines ranked lists:

```
RRF_score(symbol) = sum_strategy 1 / (k + rank_strategy(symbol))
```

Where `k = 60`. Symbols appearing in multiple result lists rise to the top.

---

## The Context Assembly Pipeline

```
  Task description
        |
  1. PARSE QUERY --> Extract search terms, intent, focal symbols
        |
  2. MULTI-STRATEGY SEARCH --> Run applicable strategies
        |
  3. RANK (RRF) --> Combine into single ranked list
        |
  4. EXPAND GRAPH --> Add graph neighbors (1-2 hops)
        |
  5. SLICE --> Extract relevant code fragments
        |
  6. BUDGET --> Fit into token budget, prioritized by rank
        |
  Context block (ready for prompt assembly)
```

### Step 1: Parse query

The task description is analyzed to extract:
- **Explicit symbol mentions** -- `"modify build_graph"` triggers keyword
  search for `build_graph`
- **Kind hints** -- `"add a new trait"` triggers structural search for
  existing traits
- **Similarity intent** -- `"like the Gate implementation"` triggers HDC
  similarity anchored on `Gate`
- **Scope hints** -- `"in roko-index"` triggers file pattern filter

### Step 2: Multi-strategy search

Applicable strategies run in parallel. A specific name query uses keyword
only (fast path). A "find similar" query uses HDC + embedding. An
open-ended exploration uses all strategies.

### Step 3: Rank via RRF

Results from all strategies combine into a unified ranking.

### Step 4: Expand graph

Top-ranked symbols expand using the dependency graph:
- **Forward expansion (depth 1)** -- Include dependencies
- **Reverse expansion (depth 1)** -- Include dependents
- **Contextual expansion** -- Include symbols in the same file

This fills in structural context: a function without its type definitions,
or a struct without its constructor.

### Step 5: Slice

Extract minimal code fragments rather than entire files:

```rust
pub struct CodeSlice {
    pub file_path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub content: String,
    pub symbols_included: Vec<SymbolId>,
    pub token_estimate: usize,
}
```

Program slicing (Weiser 1981) provides the theoretical foundation: include
only the code relevant to the current computation.

### Step 6: Budget-aware composition

Token estimation uses ~4 characters per token for code. Budget allocation
is proportional to PageRank score:

```
token_budget(symbol) = total_budget * (pagerank(symbol) / sum(pagerank(included)))
```

---

## The CodeIndex Trait

The `WorkspaceIndex` implements the `CodeIndex` trait that provides all
ten query types consumed by both the MCP server and the CLI:

```rust
pub trait CodeIndex {
    fn search(&self, query: &IndexQuery) -> Result<Vec<SearchResult>>;
    fn symbol_context(&self, name: &str, file: Option<&str>,
                      depth: u32) -> Result<SymbolContext>;
    fn file_ast(&self, path: &str, bodies: bool) -> Result<FileAst>;
    fn find_similar(&self, reference: &str, min_sim: f64,
                    max: usize) -> Result<Vec<SearchResult>>;
    fn stats(&self) -> Result<IndexStats>;
    fn find_references(&self, name: &str, file: Option<&str>,
                       include_defs: bool) -> Result<Vec<ReferenceMatch>>;
    fn find_implementations(&self, trait_name: &str,
                            methods: bool) -> Result<Vec<ImplementationMatch>>;
    fn callers(&self, name: &str, file: Option<&str>,
               transitive: bool, depth: u32) -> Result<CallGraph>;
    fn workspace_map(&self, depth: &str,
                     focus: Option<&str>) -> Result<WorkspaceMap>;
    fn assemble_context(&self, task: &str, budget: u64,
                        tests: bool) -> Result<AssembledContext>;
}
```

---

## Context Overlay System

Per-agent customization of index views:

```rust
pub struct ContextOverlay {
    pub pinned_files: Vec<String>,
    pub excluded_patterns: Vec<String>,
    pub importance_overrides: HashMap<SymbolId, f64>,
    pub max_expansion_depth: usize,
}
```

Use cases:
- A coding agent working on `roko-gate` pins gate files and excludes chain
  crate files
- A research agent pins documentation and suppresses test utilities
- A security audit agent boosts symbols with `unsafe` in their context

### Privacy configuration

```rust
pub struct PrivacyConfig {
    pub redact_patterns: Vec<String>,
    pub ignore_files: Vec<String>,
    pub blocked_symbols: Vec<String>,
}
```

Redaction happens after search and ranking but before prompt composition,
ensuring sensitive data never enters LLM prompts.

---

## Token Savings: Measured Impact

### Without code intelligence

An agent tasked with "add error handling to `process_input`" must:
1. Search (grep-like): 20--50 candidate files
2. Include full files: ~50K tokens
3. Risk missing callers, trait implementations, type definitions

### With code intelligence

1. Keyword search for `process_input`: 1 result (5ms)
2. Graph expansion: 3 dependencies, 7 callers (1ms)
3. Code slicing: 11 focused slices, ~5K tokens
4. Context includes exactly the function, its dependencies, and callers

**Result: 10x fewer tokens, higher-quality context.**

### Impact analysis scenario

"What breaks if I change `Verdict`?"

- Without: `grep -rn "Verdict"` gives 47 files, ~150K tokens, no structure
- With: `reverse_neighbors(Verdict)` gives 12 direct dependents. Transitive
  closure (depth 2) gives 23 affected symbols. Code slices total ~2K tokens.

**Result: 75x fewer tokens, structured impact understanding.**

---

## Integration with roko-compose

Code intelligence output becomes part of the `SystemPromptBuilder` context
layer. Structured context is far more useful than raw file dumps:

```markdown
## Relevant Code Context

### Core types (PageRank > 0.01)
// crates/roko-index/src/graph.rs:46
pub struct SymbolGraph { ... }

### Focal function
// crates/roko-index/src/graph.rs:118
pub fn build_graph(files: &[SourceFile]) -> SymbolGraph { ... }

### Callers (7 found)
- orchestrate.rs:142
- search.rs:89
```

---

## Verified Behaviors (21 workspace tests)

- WorkspaceIndex build from directory
- Keyword/structural/HDC/hybrid search
- Symbol context with dependencies and callers
- File AST extraction
- Similar pattern finding
- Reference and implementation finding
- Call graph traversal (forward/reverse, transitive)
- Workspace map generation
- Context assembly with token budgets
- Index statistics

---

## Academic Foundations

- **Program slicing**: Weiser (1981). *ICSE*. Minimal relevant code subsets.
- **Reciprocal Rank Fusion**: Cormack, Clarke, and Butt (2009). *SIGIR*.
  Multi-list fusion.
- **Hoogle**: Mitchell (2004). Haskell API search by type signature.
- **Meta-Harness**: Lee et al. (2026). arXiv:2603.28052. +7.7 points from
  harness optimization.

---

## Cross-References

- See [pagerank-symbol-importance.md](./pagerank-symbol-importance.md) for
  the ranking algorithm used in context prioritization
- See [hdc-fingerprints.md](./hdc-fingerprints.md) for structural similarity
  search
- See [mcp-context-server.md](./mcp-context-server.md) for the agent-facing
  API that triggers context assembly
- See [dependency-graph.md](./dependency-graph.md) for graph expansion
