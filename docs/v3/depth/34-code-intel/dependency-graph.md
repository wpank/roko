# Dependency Graph

> **Parent:** [34-CODE-INTELLIGENCE](../../34-CODE-INTELLIGENCE.md) Section 7

---

## Graph Data Structure

### SymbolGraph

The core data structure uses adjacency lists for both forward and reverse
edges:

```rust
#[derive(Clone, Debug)]
pub struct SymbolGraph {
    nodes: HashSet<SymbolId>,
    forward: HashMap<SymbolId, Vec<(SymbolId, EdgeKind)>>,
    reverse: HashMap<SymbolId, Vec<(SymbolId, EdgeKind)>>,
}
```

The dual-adjacency design is intentional:

- **Forward edges** answer "what does symbol X depend on?" -- critical for
  transitive dependency analysis.
- **Reverse edges** answer "what depends on symbol X?" -- critical for
  impact analysis and finding callers.

Maintaining both directions in sync costs 2x memory for edges but enables
O(1) lookup in either direction. For a workspace with ~10K symbols and ~30K
edges, this is ~2MB -- negligible.

---

## Edge Types

```rust
#[non_exhaustive]
pub enum EdgeKind {
    Calls,       // Function/method call
    Imports,     // use/import/require statement
    Implements,  // Trait/interface implementation
    Contains,    // Scope nesting (method in impl block)
    TypeRef,     // Type reference in signature or body
}
```

Each edge kind represents a different structural relationship:

| Edge kind | Meaning | Example | Source |
|-----------|---------|---------|--------|
| `Imports` | A imports B | `use crate::config::Config;` | Import parsing (built) |
| `Calls` | A calls B | `config.validate()` | Regex heuristic `CALL_RE` |
| `TypeRef` | A references type B | `fn process(c: Config)` | Regex heuristic `TYPE_REF_RE` |
| `Implements` | A implements B | `impl Gate for CompileGate` | Tree-sitter (planned) |
| `Contains` | A contains B | `impl Config { fn new() }` | Tree-sitter (planned) |

The current `build_graph()` creates `Imports` edges from parsed imports.
`Calls` and `TypeRef` edges are extracted using regex heuristics over
function bodies. `Implements` and `Contains` edges require tree-sitter AST
analysis.

---

## Graph Construction

Graph construction is a multi-phase process:

### Phase 1: Node registration

Every symbol in every file becomes a node:

```rust
for file in files {
    for sym in &file.symbols {
        nodes.insert(SymbolId::from_symbol(sym, &file.path));
    }
}
```

### Phase 2: Name-to-ID lookup table

A reverse index maps symbol names to their `SymbolId`s across all files:

```rust
let mut name_to_ids: HashMap<&str, Vec<SymbolId>> = HashMap::new();
```

Names can map to multiple IDs because the same name may appear in different
files (e.g., `fn new()` in multiple modules).

### Phase 3: Import edge creation

For each file, the algorithm matches import paths against the name lookup
table by last segment. Multi-separator support (`::`/`/`/`.`) enables
cross-language import resolution. Self-file edges are excluded.

### Phase 4: Call and TypeRef edges

Regex-based extraction from function bodies:

- `CALL_RE` pattern `\b([A-Za-z_][A-Za-z0-9_]*)\s*\(` identifies call sites
- `TYPE_REF_RE` pattern `\b([A-Z][A-Za-z0-9_]*)\b` identifies type
  references (capitalized identifiers)

These heuristics produce useful edges but have false positives. Tree-sitter
AST analysis will provide precise call site and type reference identification.

### Complexity analysis

| Phase | Time | Space |
|-------|------|-------|
| Node registration | O(S) | O(S) |
| Name lookup table | O(S) | O(S) |
| Edge creation | O(F x I x M) | O(E) |
| Total | O(S + F x I x M) | O(S + E) |

For the Roko workspace: completes in under 1ms.

---

## Graph Traversal Operations

### Forward neighbors

```rust
pub fn neighbors(&self, id: &SymbolId) -> Vec<&SymbolId>
```

Returns all symbols that `id` depends on. Use: understanding dependencies
before modification, assessing coupling.

### Reverse neighbors

```rust
pub fn reverse_neighbors(&self, id: &SymbolId) -> Vec<&SymbolId>
```

Returns all symbols that depend on `id`. Use: impact analysis for API
changes, safe refactoring.

### Transitive closure (BFS)

```rust
pub fn transitive(&self, start: &SymbolId, max_depth: usize)
    -> Vec<(SymbolId, usize)>
```

BFS traversal following forward edges to find all transitive dependencies
up to a configurable depth. Returns each reached symbol paired with its
distance.

- Depth 1: direct dependencies only (fast, focused)
- Depth 2: dependencies of dependencies (neighborhood understanding)
- Depth 3+: extended reach (use with budget constraints)

### All edges

```rust
pub fn all_edges(&self) -> Vec<SymbolEdge>
```

Returns all edges as `SymbolEdge` triples for serialization and external
representation.

---

## Practical Examples

### Star topology (hub with spokes)

```
    A --imports--> Hub <--imports-- B
                    ^
                    |
              C --imports--'
```

Three files all import `Hub`. Hub gets highest PageRank. This pattern is
common: core types like `Config`, `Error`, `Context` are imported by many
modules.

### Chain topology

```
    Top --imports--> Mid --imports--> Core
```

`Top.transitive(depth=2)` returns `[(Mid, 1), (Core, 2)]`. Core gets
highest PageRank (end of chain, no outgoing edges).

### Cycle

```
    A --> B --> C --> A
```

All three get roughly equal PageRank (verified by test: diff < 0.01). The
BFS `visited` set prevents infinite loops.

---

## Program Dependence Graphs

### Beyond import edges

The current `SymbolGraph` captures module-level dependencies via import
edges. Program Dependence Graphs (PDGs) (Ferrante, Ottenstein, and Warren
1987) extend this with two additional relationship types:

- **Control dependence edges**: Statement Y is control-dependent on predicate
  X if Y's execution depends on which branch is taken.
- **Data dependence edges**: Statement S1 defines variable `v`; statement S2
  uses `v`; there is a def-clear path from S1 to S2.

### Program slicing via graph traversal

Program slicing (Weiser 1981) extracts the minimal set of statements
relevant to a computation:

- **Backward slice**: follow dependence edges backward to find all
  statements affecting a value (debugging, root-cause analysis).
- **Forward slice**: follow edges forward to find all affected statements
  (impact analysis).
- **Thin slice** (Sridharan et al. 2007): backward slice excluding
  data-structure routing dependences (1.5--5x smaller).

### LLM-aware program analysis

- **BugLens** (OOPSLA 2024): PDG slices as structured LLM context improve
  precision from 0.10 to 0.72.
- **LLMxCPG** (USENIX Security 2025): LLM-generated CPG traversal queries
  achieve 67--91% code reduction while preserving vulnerability-relevant
  structure.

This validates the design: graph-based slicing produces compact,
dependency-aware context that LLMs reason about more effectively than raw
file dumps.

---

## Planned Enhancements

### Call graph edges via tree-sitter

Tree-sitter enables extraction of function call sites, producing precise
`Calls` edges instead of the current regex heuristics. Call edges are
dramatically more informative than import edges because they capture actual
runtime dependencies.

### Weighted edges

Edge weighting for more accurate PageRank:

| Condition | Weight | Rationale |
|-----------|--------|-----------|
| Symbol in current task | 10x | Direct relevance |
| File in agent's context | 50x | Active working set |
| Private symbol | 0.1x | Less external relevance |
| Recently modified | 5x | Recency bias |
| Test file dependency | 0.5x | Tests depend on code, not reverse |

### Impact analysis

Combining reverse traversal with edge types enables structured impact
reports: not just that 47 files mention a symbol, but that 12 directly call
it, 8 import it, and 3 implement it as a trait method.

### Subgraph extraction

For context assembly, extract relevant subgraphs around focal symbols:

```rust
pub fn extract_subgraph(
    graph: &SymbolGraph,
    focal: &[SymbolId],
    radius: usize,
) -> SymbolGraph
```

BFS from each focal symbol (both forward and reverse), returning all nodes
within radius and all edges between them.

---

## Verified Behaviors (22 tests)

- Empty graph: no panics, empty rank map
- Star topology: hub gets highest PageRank
- Cycle topology: roughly equal ranks (diff < 0.01)
- Forward/reverse neighbor queries return correct sets
- Transitive BFS respects depth limits
- Self-file import edges are excluded
- Regex-based call and type-ref edge extraction
- Weighted and personalized PageRank variants

---

## Academic Foundations

- **Program Dependence Graph**: Ferrante, Ottenstein, and Warren (1987).
  *ACM TOPLAS* 9(3). Control and data dependence edges.
- **System Dependence Graph**: Horwitz, Reps, and Binkley (1990). *ACM
  TOPLAS* 12(1). Interprocedural slicing with summary edges.
- **Program slicing**: Weiser (1981). *ICSE*. Minimal relevant code subsets.
- **Thin slicing**: Sridharan, Fink, and Bodik (2007). *PLDI*. 1.5--5x
  smaller slices.
- **Code Property Graphs**: Yamaguchi et al. (2014). *IEEE S&P*. Unified
  AST/CFG/PDG. IEEE Test-of-Time Award 2024.
- **PageRank**: Page, Brin, Motwani, and Winograd (1999). Stanford InfoLab.
- **LLMxCPG**: Lekssays et al. (2025). *USENIX Security*. LLM-generated
  CPG queries.
- **BugLens**: (OOPSLA 2024). PDG slices improve LLM vulnerability analysis.

---

## Cross-References

- See [symbol-extraction.md](./symbol-extraction.md) for how symbols become
  graph nodes
- See [pagerank-symbol-importance.md](./pagerank-symbol-importance.md) for
  importance scoring over the graph
- See [context-assembly-from-code.md](./context-assembly-from-code.md) for
  how graph structure informs context selection
- See [index-db-scaling.md](./index-db-scaling.md) for persistent graph
  storage
