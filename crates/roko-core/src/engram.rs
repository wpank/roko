//! Backward-compatible re-export module.
//!
//! The canonical definitions now live in [`crate::signal`]. This module
//! re-exports everything so that existing `use crate::engram::*` paths
//! continue to compile.

pub use crate::signal::*;
