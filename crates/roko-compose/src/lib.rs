//! [`Composer`](roko_core::Composer) implementations — assemble signals into
//! structured outputs under resource budgets.
//!
//! This crate provides:
//!
//! - [`PromptSection`] — typed payload for a labeled, priority-tagged prompt fragment
//! - [`PromptComposer`] — combines sections into a final prompt under a token budget
//! - [`SectionScorer`] — ranks sections by priority × recency × relevance
//!
//! # Design principle
//!
//! Composers do not read files. The app layer reads files, converts them
//! into `Signal<PromptSection>`s, and passes them to the composer. This is
//! the strict I/O boundary from `02-anti-patterns.md` (rule #8).

#![allow(clippy::module_name_repetitions)]

pub mod prompt;
pub mod scorer;

pub use prompt::{
    estimate_tokens, CacheLayer, ContextStrategy, Placement, PromptBuild,
    PromptComposer, PromptSection, SectionPriority,
};
pub use scorer::SectionScorer;
