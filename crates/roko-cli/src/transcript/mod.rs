//! Shared semantic transcript model for tool-audit UX.
//!
//! This module owns the data model that both inline (CLI) and TUI renderers
//! consume. It bridges the low-level [`TranscriptRecord`] stream from
//! `roko-core` into a set of semantic blocks that renderers can iterate,
//! fold, search, and project without knowing about provider-specific wire
//! formats.
//!
//! # Key types
//!
//! | Type | Purpose |
//! |---|---|
//! | [`TranscriptBlock`] | A renderable semantic unit |
//! | [`FoldState`] | Collapse/expand state per block |
//! | [`TranscriptProjection`] | Query/iteration API over blocks |
//! | [`BlockFilter`] | Filter predicate for queries |

mod block;
mod convert;
mod fold;
mod projection;

pub use block::{MessageLevel, SubagentBlockStatus, ToolBlockStatus, TranscriptBlock};
pub use convert::blocks_from_records;
pub use fold::{FoldRule, FoldState};
pub use projection::{BlockFilter, BlockQuery, TranscriptProjection};

#[cfg(test)]
mod tests;
