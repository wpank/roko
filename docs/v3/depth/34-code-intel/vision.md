# Vision: Why Code Intelligence Matters for Cognitive Agents

> **Parent:** [34-CODE-INTELLIGENCE](../../34-CODE-INTELLIGENCE.md) Section 1

---

## The Context Window Problem

### Token budgets are scarce

Modern LLMs offer context windows ranging from 128K to 2M tokens. This
sounds abundant, but consider the math for a real codebase:

| Metric | Roko workspace | Typical enterprise |
|--------|---------------|-------------------|
| Lines of code | ~1M | 500K--5M |
| Estimated tokens (raw) | ~1.5M | 1.5M--15M |
| Context budget (128K model) | 128K | 128K |
| Usable for code (after prompt/history) | ~80K | ~80K |
| Coverage without intelligence | ~5% | 0.5--5% |

Without code intelligence, an agent can see at most 5% of even a modest
codebase in a single turn. For enterprise codebases, that drops below 1%.
The agent is effectively blind to 95--99% of the code it is modifying.

### Blind agents make expensive mistakes

When agents lack structural understanding, they exhibit predictable failure
modes:

1. **Duplicate implementations** -- The agent writes code that already exists
   elsewhere in the codebase because it never saw the existing
   implementation. This is the single most common failure mode in the Roko
   codebase itself, catalogued in `MISTAKES-LEARNED.md` as mistake #1.

2. **Broken dependencies** -- The agent modifies a function signature without
   knowing that dozens of other call sites depend on the old signature.
   Impact analysis requires graph traversal, not text search.

3. **Misunderstood abstractions** -- The agent reimplements a capability
   because it does not understand the trait hierarchy or generic patterns in
   play. Understanding that `Gate` is a composable trait across layers
   requires structural comprehension.

4. **Token waste on irrelevant context** -- Without ranking, the agent
   includes entire files when it only needs three functions. Measurements
   from the Aider project (Gauthier 2024) show that intelligent context
   selection reduces token consumption by 10x for search tasks and up to 75x
   for impact analysis.

---

## The Niche Construction Thesis

The concept of niche construction from evolutionary biology (Odling-Smee,
Laland, and Feldman 2003) provides the theoretical foundation for code
intelligence. In biology, organisms do not just adapt to their environment
-- they actively modify it to improve their fitness. Beavers build dams.
Earthworms transform soil chemistry. These modifications create a "cognitive
niche" that makes the organism more effective.

Coding agents operate analogously. The codebase is the agent's environment.
Code intelligence is the mechanism by which the agent constructs its
cognitive niche -- building indexes, computing importance scores, maintaining
symbol graphs -- so that future interactions with the codebase are more
productive. Each indexing pass makes the agent more effective at its next
task.

This is not a metaphor. It is a design principle. The `roko-index` crate is
literally the niche construction machinery for Roko's coding agents. Every
parse, every graph edge, every HDC fingerprint is a modification to the
agent's cognitive environment that improves future performance.

---

## What Code Intelligence Provides

### The four pillars

The `roko-index` crate implements four core capabilities, each building on
the previous:

| Pillar | Module | What it does | Why it matters |
|--------|--------|-------------|---------------|
| **Parsing** | `parser` | Extracts symbols and imports via `LanguageProvider` | Raw structural data |
| **Graph** | `graph` | Directed dependency graph, PageRank scoring | Understands relationships and importance |
| **Fingerprints** | `hdc` | 10,240-bit HDC vectors for structural similarity | Finds similar code without embeddings |
| **Search** | `workspace` | Hybrid search combining keyword, structural, and HDC strategies | Retrieves precisely the right context |

These pillars serve the composition system at two critical steps:

- **Perceive** -- Code intelligence enables queries to return not just raw
  files but parsed, ranked, similarity-scored code fragments. The agent
  perceives code structure, not text.

- **Compose** -- PageRank scores and dependency information assemble context
  windows that prioritize the most important symbols for the current task.
  Budget-aware composition means every token counts.

### Token savings: empirical evidence

Measurements from Aider's repository map feature (Gauthier 2024) and
Meta-Harness experiments (Lee et al. 2026, arXiv:2603.28052) demonstrate the
impact of code intelligence:

| Scenario | Without intelligence | With intelligence | Savings |
|----------|---------------------|-------------------|---------|
| Code search (find relevant function) | ~50K tokens | ~5K tokens | **10x** |
| Impact analysis (who calls this?) | ~150K tokens | ~2K tokens | **75x** |
| Similar pattern finding | ~100K tokens | ~3K tokens | **33x** |
| Context for modification | ~40K tokens | ~8K tokens | **5x** |

These savings are not just cost reduction. They directly improve agent
quality because more of the context budget is spent on relevant code rather
than noise.

---

## Design Principles

### 1. Language-agnostic core, language-specific providers

The `roko-index` crate defines no language-specific parsing logic. All
language knowledge lives in `LanguageProvider` implementations:

- `roko-lang-rust` -- Rust parsing (heuristic + feature-gated tree-sitter)
- `roko-lang-typescript` -- TypeScript/JavaScript parsing
- `roko-lang-go` -- Go parsing

Adding a new language requires only implementing the `LanguageProvider`
trait. The graph, fingerprint, and search layers work unchanged.

### 2. Incremental by design

Codebases change incrementally -- a commit typically touches 1--5 files out
of thousands. BLAKE3 content hashing detects which files actually changed
(not just which were touched by git). The SQLite backend supports true
incremental updates. Target: sub-second re-indexing for typical commits,
even on workspaces with 100K+ symbols.

### 3. Zero-copy where possible

Performance matters because indexing runs in the agent's critical path.
The design favors:

- **Bitwise operations** for HDC similarity (XOR + popcount, ~50ns per
  comparison)
- **In-place graph traversal** rather than materializing intermediate
  collections
- **Planned rkyv snapshots** for zero-copy deserialization via
  memory-mapped files

### 4. Composable with the Signal architecture

Code intelligence is not a standalone system. It produces and consumes
Signals:

- A parsed symbol can be stored as a Signal with `kind: CodeSymbol`
- A PageRank score maps to a Signal's `utility` axis
- An HDC fingerprint similarity maps to the `salience` axis
- The dependency graph itself is a form of lineage tracking

---

## Relationship to the Broader Architecture

### Layer placement

Code intelligence spans two layers:

| Layer | Component | Role |
|-------|-----------|------|
| **L0 Runtime** | File watching, incremental triggers | Detects when code changes |
| **L2 Scaffold** | `roko-index`, `roko-lang-*` | Parsing, graphs, fingerprints, search |
| **L2 Scaffold** | `roko-compose` | Assembles code context into prompts |
| **L3 Harness** | `roko-mcp-code` | Exposes code intelligence to agents via MCP tools |

### Agent types that depend on code intelligence

- **Coding agents** -- Primary consumers. Use code intelligence for every
  task: understanding existing code, finding modification points, assessing
  impact, generating context-aware patches.
- **Research agents** -- Use code intelligence to understand codebase
  structure during analysis tasks.
- **Custom agents** -- Domain-specific agents may use code intelligence when
  their domain involves code (e.g., a security audit agent).

---

## Current Implementation Status

The `roko-index` crate ships 6 modules (parser, symbol, graph, hdc,
workspace, sqlite) at ~5,822 lines with 88 tests. Three language providers
ship (Rust at 1,387 lines, TypeScript at 939, Go at 672) totaling 104 tests.
The `roko-mcp-code` MCP server implements all 10 planned tools (2,330 lines,
16 tests). SQLite persistence is feature-gated and enabled by `roko-cli`.
CLI commands (`roko index build/rebuild/search/stats`) are wired.

### What is missing

1. Dense embeddings (no fastembed/BGE-small integration)
2. rkyv zero-copy snapshots
3. Salsa memoization
4. HNSW approximate nearest-neighbor index
5. Additional language providers (Python, Java, C++)
6. `roko-compose` integration for automatic context enrichment

---

## Academic Foundations

- **Niche construction**: Odling-Smee, Laland, and Feldman (2003). *Niche
  Construction: The Neglected Process in Evolution*. Princeton University
  Press. Environmental modification by cognitive agents.
- **Principles of Program Analysis**: Nielson, Nielson, and Hankin (1999).
  Foundational text on extracting structural information from source code.
- **Program slicing**: Weiser (1981). "Program Slicing." *ICSE*. Extracting
  minimal relevant code subsets.
- **Code property graphs**: Yamaguchi, Golde, Arp, and Rieck (2014).
  "Modeling and Discovering Vulnerabilities with Code Property Graphs."
  *IEEE S&P*. IEEE Test-of-Time Award 2024.
- **Hyperdimensional computing**: Kanerva (2009). *Cognitive Computation*
  1(2). Mathematical foundation for HDC fingerprints.
- **code2vec**: Alon, Zilberstein, Levy, and Brody (2019). *POPL*.
  Distributed code representations.
- **Meta-Harness**: Lee et al. (2026). arXiv:2603.28052. +7.7 points from
  harness optimization including context engineering.
- **Aider repository map**: Gauthier (2024). Tree-sitter-based repository
  maps for coding agent context quality.

---

## Cross-References

- See [tree-sitter-parsing.md](./tree-sitter-parsing.md) for incremental
  parsing design
- See [dependency-graph.md](./dependency-graph.md) for the `SymbolGraph`
  architecture
- See [hdc-fingerprints.md](./hdc-fingerprints.md) for HDC similarity search
- See [context-assembly-from-code.md](./context-assembly-from-code.md) for
  how indexed code becomes LLM context
- See Chapter 06 (Composition) for context engineering and prompt assembly
- See Chapter 05 (Agent) for agent types and their use of code intelligence
