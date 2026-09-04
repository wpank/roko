//! Code intelligence for Roko: source parsing, symbol graphs, and HDC
//! fingerprints.
//!
//! This crate provides the building blocks for code understanding:
//!
//! - **[`parser`]** — Parse source files via a language-agnostic
//!   [`LanguageProvider`](roko_core::LanguageProvider) trait.
//! - **[`symbol`]** — Symbol identifiers, references, and lookup utilities.
//! - **[`graph`]** — Directed symbol dependency graph with `PageRank` scoring.
//! - **[`hdc`]** — 10,240-bit hyperdimensional fingerprints for similarity
//!   search.
//! - **[`sqlite`]** *(feature `sqlite`)* — Versioned persistent index store
//!   backed by SQLite at `<root>/.roko/index.db`.
//!
//! # Feature disposition (backlog #362)
//!
//! - **SQLite** (`sqlite`): canonical CLI persistence. Enabled by `roko-cli`.
//! - **rkyv** (`rkyv`): library-only opt-in. CLI does not enable it.
//! - **tree-sitter** (`roko-lang-rust/tree-sitter`): disabled, experimental.
//! - **HDC search**: library-only per backlog #335.

#![allow(
    clippy::cast_precision_loss,
    clippy::doc_markdown,
    clippy::expect_used,
    clippy::map_unwrap_or,
    clippy::missing_const_for_fn,
    clippy::needless_pass_by_value,
    clippy::similar_names,
    clippy::too_many_lines
)]

pub mod graph;
pub mod hdc;
pub mod parser;
#[cfg(feature = "sqlite")]
pub mod sqlite;
pub mod symbol;
pub mod workspace;

// ─── Convenience re-exports ─────────────────────────────────────────────

pub use graph::{
    EdgeKind, SymbolEdge, SymbolGraph, build_graph, pagerank, personalized_pagerank,
    weighted_pagerank,
};
pub use hdc::{HdcFingerprint, fingerprint_file, fingerprint_symbol, similarity};
pub use parser::{SourceFile, parse_source};
#[cfg(feature = "sqlite")]
pub use sqlite::{
    FileRecord, IndexMeta, IndexStore, IndexStoreError, RankingRecord, SqliteIndex, UpdateStats,
};
pub use symbol::{SymbolId, SymbolRef, find_symbol};
pub use workspace::{
    AssembledContext, CallGraph, CallGraphDirection, CallGraphEdge, CodeIndex, CodeSlice,
    ContextOverlay, EmbeddingQuery, FileAst, HdcQuery, ImplementationMatch, IndexQuery, IndexStats,
    KeywordQuery, PrivacyConfig, ReferenceKind, ReferenceMatch, SearchResult, SearchScope,
    SearchStrategy, StructuralQuery, SymbolContext, SymbolInfo, WorkspaceIndex, WorkspaceMap,
    WorkspaceMapEntry,
};
