# Tree-Sitter Incremental Parsing

> **Parent:** [34-CODE-INTELLIGENCE](../../34-CODE-INTELLIGENCE.md) Sections 3, 5

---

## The LanguageProvider Trait

All parsing in `roko-index` goes through a single trait defined in
`roko-core`:

```rust
pub trait LanguageProvider: Send + Sync {
    fn language_name(&self) -> &str;
    fn file_extensions(&self) -> &[&str];
    fn parse_imports(&self, source: &str) -> Vec<Import>;
    fn extract_symbols(&self, source: &str) -> Vec<Symbol>;
}
```

The `parse_source()` function in `roko-index/src/parser.rs` delegates
entirely to the provider:

```rust
pub fn parse_source(
    path: &str,
    content: &str,
    provider: &dyn LanguageProvider,
) -> SourceFile {
    let symbols = provider.extract_symbols(content);
    let imports = provider.parse_imports(content);
    SourceFile {
        path: path.to_string(),
        language: provider.language_name().to_string(),
        content: content.to_string(),
        symbols,
        imports,
    }
}
```

This trait-based design means `roko-index` never needs to know which
language it is working with. The graph builder, PageRank, HDC fingerprinter,
and search layer all operate on `SourceFile`, `Symbol`, and `Import` --
language-neutral abstractions. Adding Python support means writing a
`PythonLanguageProvider`; the rest of the stack works unchanged.

---

## Current Heuristic Parsers

All three shipped language providers use line-by-line heuristic parsers that
scan source text for patterns like `fn `, `struct `, `pub enum`, `function `,
`class `, `type `, and `func `. These heuristics work well for common cases
-- they correctly extract top-level definitions, handle visibility modifiers,
and parse import statements across three languages in under 3,000 lines of
code combined.

### Heuristic limitations

Heuristic parsers have fundamental limitations. They cannot handle:

- Nested function definitions (closures, inner functions)
- Multi-line signatures where `fn` and the name are on different lines
- Macro-generated code or conditional compilation
- Scope-aware symbol lookup
- Call graph extraction (no `call_expression` node traversal)
- Column-level source locations

See [language-providers.md](./language-providers.md) for details on each
provider.

---

## What Tree-Sitter Provides

Tree-sitter (Brunsfeld 2018) is an incremental parsing framework that
generates parsers from grammar specifications. It provides five key
capabilities:

### 1. Concrete Syntax Trees (CSTs)

Tree-sitter produces a CST, not an AST. The CST preserves every token
including punctuation, operators, and keywords as anonymous nodes alongside
semantic named nodes. Named nodes (e.g., `function_item`, `identifier`)
represent semantic constructs; anonymous nodes (e.g., `"+"`, `"{"`, `"fn"`)
preserve delimiters and keywords. The `node.named_child()` API filters to
named-only traversal, approximating AST behavior.

### 2. Incremental updates

When source text changes, tree-sitter re-parses only the affected regions.
A single-character edit re-parses in microseconds, not milliseconds. The
algorithm (based on Wagner and Graham 1998) reuses unchanged subtrees via a
`ReusableNode` tracker that checks whether old tree nodes at the current
parse position remain valid.

### 3. Error-tolerant parsing

Tree-sitter always produces a valid tree, even for syntactically broken
input. Two special node types handle errors:

- **ERROR nodes** -- Inserted when the parser encounters tokens with no
  valid parse action. Detected via `node.is_error()`.
- **MISSING nodes** -- Zero-width synthetic nodes inserted when the grammar
  expects a token that is absent (e.g., a missing semicolon). Detected via
  `node.is_missing()`.

### 4. GLR error recovery

When a parse error occurs, tree-sitter forks its parse stack into up to
`MAX_VERSION_COUNT = 6` concurrent branches, each attempting a different
recovery strategy. Branches are evaluated by an `ErrorStatus` cost model.
As valid nodes accumulate past the error site, the pruning threshold
tightens until only one branch survives.

### 5. Consistent API

The same query and traversal APIs work across all 300+ supported language
grammars.

---

## Current Tree-Sitter Implementation

The `TreeSitterRustProvider` in `crates/roko-lang-rust/src/tree_sitter_parser.rs`
(485 lines, 7 tests) is a working implementation behind the `tree-sitter`
feature flag:

```rust
pub struct TreeSitterRustProvider;

impl LanguageProvider for TreeSitterRustProvider {
    fn language_name(&self) -> &str { "rust" }
    fn file_extensions(&self) -> &[&str] { &["rs"] }

    fn parse_imports(&self, source: &str) -> Vec<Import> {
        let Some(tree) = parse_source(source) else { return Vec::new() };
        let mut imports = Vec::new();
        collect_imports(tree.root_node(), source, &mut imports);
        imports
    }

    fn extract_symbols(&self, source: &str) -> Vec<Symbol> {
        let Some(tree) = parse_source(source) else { return Vec::new() };
        let mut symbols = Vec::new();
        collect_symbols(tree.root_node(), source, &mut symbols);
        symbols
    }
}
```

The implementation handles:

- All symbol kinds (functions, structs, enums, traits, consts, types,
  modules, impl blocks)
- Impl blocks with trait names (`impl Display for Foo`)
- Methods within impl bodies via recursive `collect_impl_methods()`
- Visibility modifiers
- `use` declarations with brace expansion and wildcards
- `mod` declarations and `extern crate`
- Graceful error recovery (parse errors do not prevent extraction of valid
  regions)

A parity test verifies that tree-sitter extracts at least as many symbols
as the heuristic parser for standard cases.

---

## What Tree-Sitter Enables Beyond Heuristics

| Capability | Heuristic | Tree-sitter |
|-----------|-----------|-------------|
| Nested function definitions | Missed | Captured at correct scope |
| Multi-line signatures | Fragile | Robust |
| Macro-generated items | Invisible | Visible (if expanded) |
| Scope-aware symbol lookup | Impossible | Natural via AST traversal |
| Call graph extraction | Impossible | Via function call node traversal |
| `impl Trait for Type` edges | Partial | Complete |
| Error recovery | Crashes or misparses | Partial tree with ERROR nodes |
| Column-level source locations | Not tracked | Exact byte offsets |

The most critical capability tree-sitter enables is **call graph
extraction**. The current graph builder uses regex heuristics for call
edges. Tree-sitter's AST contains `call_expression` nodes that identify
exactly which function is being called -- enabling precise `Calls` edges.

---

## Tree-Sitter Query Language

Tree-sitter queries use S-expression patterns with captures (`@name`),
field names, wildcards, and predicates:

```scheme
;; Match function definitions with full context
(function_item
  name: (identifier) @name
  parameters: (parameters) @params
  return_type: (_)? @return_type
  body: (block) @body
) @definition.function

;; Match method definitions in impl blocks
(impl_item
  type: (_) @impl_type
  body: (declaration_list
    (function_item
      name: (identifier) @method_name
    ) @definition.method
  )
)

;; Match call expressions for call graph edges
(call_expression
  function: [
    (identifier) @callee
    (field_expression field: (field_identifier) @callee)
    (scoped_identifier name: (identifier) @callee)
  ]
) @call_site
```

Key predicates:

| Predicate | Purpose | Example |
|-----------|---------|---------|
| `#eq?` | Text equality | `((identifier) @x (#eq? @x "self"))` |
| `#match?` | Regex match | `((identifier) @c (#match? @c "^[A-Z]"))` |
| `#any-of?` | Multi-string match | `((identifier) @kw (#any-of? @kw "self" "super"))` |

---

## Incremental Parsing Workflow

```
                    Initial Parse
  Source text --> tree_sitter::Parser --> Tree (full CST)
                                           |
                                    Store tree + hash

                    Incremental Update
  Git diff --> compute edit ranges --> tree.edit(InputEdit)
                                           |
                                  parser.parse(text, Some(old_tree))
                                           |
                                  New tree (partial re-parse)
                                           |
                                  ts_tree_get_changed_ranges(old, new)
                                           |
                                  Re-extract symbols in changed ranges only
```

The key insight: `parser.parse()` accepts an optional `old_tree`. When
provided, tree-sitter reuses unchanged subtrees -- no lexing, no parsing.
For a typical single-function edit, this means re-parsing a few hundred
bytes rather than the entire file.

### The InputEdit struct

Before reparsing, the application describes the edit:

```rust
pub struct InputEdit {
    pub start_byte: usize,
    pub old_end_byte: usize,
    pub new_end_byte: usize,
    pub start_position: Point,
    pub old_end_position: Point,
    pub new_end_position: Point,
}
```

Both byte offsets and row/column positions are required because after an
edit, the tree cannot re-read source text to derive positions.

---

## Error-Tolerant Parsing for Coding Agents

Error tolerance is critical because agents frequently work with incomplete
code:

1. Agent modifies a function signature (code is temporarily invalid)
2. Tree-sitter produces a partial tree with ERROR nodes around the broken
   signature
3. All other symbols in the file remain correctly parsed
4. Agent continues modifying the function body
5. Tree re-parses incrementally -- ERROR nodes resolve as the code becomes
   valid

Without error tolerance, a single syntax error would prevent parsing the
entire file, losing all code intelligence until the error is fixed.

---

## Performance Characteristics

| Operation | Heuristic (current) | Tree-sitter (target) | Notes |
|-----------|--------------------|--------------------|-------|
| Initial parse (10K line file) | ~2ms | ~10ms | Tree-sitter slower but more accurate |
| Incremental re-parse (1 line) | N/A (full re-parse) | ~50us | Tree-sitter's key advantage |
| Full workspace parse (~1M lines) | ~300ms | ~1.5s | Initial; subsequent passes incremental |
| Memory per 10K-line tree | N/A | ~1--3 MB | Trees can be dropped and re-parsed |

The initial parse is slower with tree-sitter, but the incremental advantage
is overwhelming: re-parsing a single-line change is ~40x faster than the
heuristic approach (which re-parses the entire file).

---

## Verified Behaviors (7 tests)

- Basic function extraction from Rust source
- Struct and enum extraction
- Impl block extraction (including `impl Trait for Type`)
- Use import extraction (including brace expansion, mods, extern crate)
- Trait and const extraction
- Graceful handling of parse errors
- Heuristic vs tree-sitter parity

---

## Academic Foundations

- **Tree-sitter**: Brunsfeld (2018). Incremental parsing framework. 300+
  language grammars.
- **Efficient and Flexible Incremental Parsing**: Wagner and Graham (1998).
  *ACM TOPLAS* 20(5). Foundation for tree-sitter's incremental algorithm.
- **Augmenting Parsers to Support Incrementality**: Ghezzi and Mandrioli
  (1980). *Journal of the ACM* 27(3). First incremental LR parsing.
- **Principles of Program Analysis**: Nielson, Nielson, and Hankin (1999).
  Foundational text on static analysis.
- **Aider repository map**: Gauthier (2024). Uses tree-sitter for coding
  agent context.
- **mcp-server-tree-sitter**: (2025). Tree-sitter as MCP tool server for AI
  agents.

---

## Cross-References

- See [vision.md](./vision.md) for why code intelligence matters
- See [symbol-extraction.md](./symbol-extraction.md) for what parsers
  extract and how symbols are represented
- See [dependency-graph.md](./dependency-graph.md) for how parsed symbols
  become a graph
- See [language-providers.md](./language-providers.md) for Rust, TypeScript,
  and Go provider details
