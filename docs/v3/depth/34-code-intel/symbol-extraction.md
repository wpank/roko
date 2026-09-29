# Symbol Extraction

> **Parent:** [34-CODE-INTELLIGENCE](../../34-CODE-INTELLIGENCE.md) Section 6

---

## The Symbol Type System

### Core types from roko-core

The foundational types live in `roko_core::language` and are re-exported by
`roko-index`:

```rust
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub visibility: Visibility,
    pub line: usize,        // 1-based line number
}

#[non_exhaustive]
pub enum SymbolKind {
    Function,   // fn, async fn, const fn (Rust); function (TS); func (Go)
    Struct,     // struct (Rust); class (TS, mapped); type struct (Go)
    Enum,       // enum (Rust, TS); not directly in Go
    Trait,      // trait (Rust); interface (TS, mapped); type interface (Go)
    Const,      // const (Rust, TS, Go); var (Go)
    Type,       // type alias (Rust, TS); type (Go, non-struct/interface)
    Module,     // mod (Rust); namespace (TS); package (Go)
    Impl,       // impl (Rust only)
}

pub enum Visibility {
    Public,     // pub (Rust); export (TS); capitalized name (Go)
    Private,    // no modifier (Rust, Go lowercase); no export (TS)
    Crate,      // pub(crate) (Rust); internal (Go package-level)
}
```

The `#[non_exhaustive]` attribute on `SymbolKind` is intentional -- new
kinds can be added without breaking downstream code as language support
expands.

### Import types

```rust
pub struct Import {
    pub path: String,       // e.g., "std::collections::HashMap"
    pub alias: Option<String>,
    pub kind: ImportKind,
}

pub enum ImportKind {
    Use, Require, Import, TypeOnly, Mod, ExternCrate,
}
```

### Cross-language mapping conventions

| Language construct | Mapped SymbolKind | Rationale |
|-------------------|-------------------|-----------|
| Rust `fn` / `async fn` | `Function` | Direct |
| Rust `struct` | `Struct` | Direct |
| Rust `trait` | `Trait` | Direct |
| Rust `impl` / `impl Trait for Type` | `Impl` | Unique to Rust |
| TypeScript `class` | `Struct` | Classes = data + methods |
| TypeScript `interface` | `Trait` | Interfaces define contracts |
| Go `func` | `Function` | Methods also map here |
| Go `type X struct` | `Struct` | Direct |
| Go `type X interface` | `Trait` | Direct |
| Go uppercase name | `Visibility::Public` | Go capitalization convention |

This mapping means the graph and fingerprint layers treat symbols uniformly
regardless of source language.

---

## Symbol Identification: SymbolId

### The unique key

`SymbolId` provides a unique identifier for a symbol within an index:

```rust
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SymbolId {
    pub file_path: String,
    pub symbol_name: String,
    pub kind: SymbolKind,
}
```

The triple `(file_path, symbol_name, kind)` uniquely identifies a symbol:

1. Two symbols with the same name but different kinds are distinct -- a
   `struct Config` and a `fn Config` in the same file are different symbols.
2. Two symbols with the same name and kind in different files are distinct.
3. Re-indexing produces the same `SymbolId` for unchanged symbols.

### Construction

```rust
impl SymbolId {
    pub fn new(
        file_path: impl Into<String>,
        symbol_name: impl Into<String>,
        kind: SymbolKind,
    ) -> Self { ... }

    pub fn from_symbol(symbol: &Symbol, file_path: &str) -> Self {
        Self {
            file_path: file_path.to_string(),
            symbol_name: symbol.name.clone(),
            kind: symbol.kind.clone(),
        }
    }
}
```

### Display format

The `Display` implementation produces: `file_path::symbol_name(Kind)`.
Example: `handler.rs::process(Function)`. Used in debugging, graph
visualization, and error messages.

### Hash-based identity

`SymbolId` derives `Hash`, making it suitable as a key in `HashMap` and
`HashSet`. This is critical for `SymbolGraph`, which uses
`HashSet<SymbolId>` for nodes and `HashMap<SymbolId, Vec<...>>` for
adjacency lists.

---

## Symbol References: SymbolRef

While `SymbolId` identifies where a symbol is defined, `SymbolRef`
identifies where a symbol is used:

```rust
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SymbolRef {
    pub file: String,
    pub line: usize,   // 1-based
    pub column: usize,  // 0-based
}
```

`SymbolRef` tracks three dimensions: file (which file contains the usage),
line (1-based, matching editor conventions), and column (0-based, matching
LSP conventions).

Tree-sitter integration enables reference tracking by traversing
`identifier` and `call_expression` nodes in the AST. When fully
implemented, this enables find-all-references, impact analysis, and dead
code detection.

---

## Symbol Lookup

### The find_symbol function

```rust
pub fn find_symbol<'a>(files: &'a [SourceFile], name: &str) -> Vec<&'a Symbol> {
    files
        .iter()
        .flat_map(|f| f.symbols.iter())
        .filter(|s| s.name == name)
        .collect()
}
```

This is intentionally simple -- a linear scan -- because:

1. Symbol counts are manageable: even a ~1M-line workspace produces
   ~10,000--20,000 top-level symbols. Linear scan over 20K items is
   sub-millisecond.
2. Name collisions are informative: the caller can distinguish by kind
   and file path.
3. The function is a building block for more sophisticated queries.

### SQLite-backed lookup

With the `sqlite` feature enabled, symbol lookup is backed by FTS5:

```sql
CREATE INDEX idx_symbols_name ON symbols(name);
CREATE VIRTUAL TABLE symbols_fts USING fts5(
    name, doc_comment,
    content='symbols', content_rowid='id',
    tokenize='unicode61 remove_diacritics 2'
);
```

This enables prefix search, fuzzy search (FTS5 tokenization handles
camelCase and snake_case splitting), and indexed lookup by kind.

---

## The Extraction Pipeline

```
  Source file (path + content)
        |
  LanguageProvider.extract_symbols()
        |
  Vec<Symbol> --> SourceFile { symbols, imports, ... }
        |                           |
  SymbolId::from_symbol()    build_graph() uses symbols as nodes
        |                           |
  Graph nodes                  Import/Call/TypeRef edges
        |
  fingerprint_symbol() --> HdcFingerprint (10,240 bits)
```

Each step enriches the raw data:

1. **Extraction** -- `LanguageProvider` turns text into `Vec<Symbol>` and
   `Vec<Import>`.
2. **Packaging** -- `parse_source()` bundles symbols and imports into a
   `SourceFile`.
3. **Identification** -- `SymbolId::from_symbol()` creates unique keys for
   graph nodes.
4. **Graphing** -- `build_graph()` registers symbols as nodes and creates
   edges from imports, calls, and type references.
5. **Fingerprinting** -- `fingerprint_symbol()` encodes each symbol into a
   10,240-bit HDC vector.

### What each language provider extracts

| Construct | Rust | TypeScript | Go |
|-----------|------|-----------|-----|
| Functions | `fn`, `async fn`, `unsafe fn`, `const fn` | `function`, `export function` | `func`, `func (r T) method` |
| Structs | `struct Name` | `class Name` | `type Name struct` |
| Enums | `enum Name` | `enum Name` | -- |
| Traits/Interfaces | `trait Name` | `interface Name` | `type Name interface` |
| Constants | `const NAME` | `const NAME`, `export const` | `const Name`, `var Name` |
| Type aliases | `type Name` | `type Name =` | `type Name` |
| Modules | `mod name` | -- | -- |
| Impl blocks | `impl Name`, `impl Trait for Type` | -- | -- |
| Imports | `use path::Item` | `import { X } from "path"` | `import "path"` |

---

## Planned Enrichments

### Rich symbol metadata (tree-sitter enabled)

With tree-sitter, symbols can carry additional information:

- Byte range (exact start/end in source)
- Column offset
- End line (for multi-line definitions)
- Function signature text
- Associated doc comment
- Parent symbol (containing impl block or module)
- Generic type parameters
- Attributes/decorators

### Scope-aware nesting

Currently, all symbols are extracted as a flat list. Tree-sitter enables
hierarchical extraction where impl blocks contain their methods and modules
contain their children. This nesting information is critical for scope
resolution and context-aware retrieval.

### Content hashing for incremental updates

Each symbol's surrounding content can be hashed with BLAKE3 for fine-grained
change detection: only symbols whose `content_hash` changed need
re-fingerprinting.

---

## Verified Behaviors (8 tests)

- SymbolId construction and display format
- Hash-based identity (HashMap key behavior)
- find_symbol across multiple files
- Serialization round-trip
- SymbolRef construction and equality
- Multiple matches for same name across files

---

## Cross-References

- See [tree-sitter-parsing.md](./tree-sitter-parsing.md) for how symbols
  are extracted from source code
- See [dependency-graph.md](./dependency-graph.md) for how symbols become
  graph nodes
- See [hdc-fingerprints.md](./hdc-fingerprints.md) for how symbols are
  encoded into fingerprint vectors
- See [index-db-scaling.md](./index-db-scaling.md) for persistent symbol
  storage design
- See [language-providers.md](./language-providers.md) for per-language
  extraction details
