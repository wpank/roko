//! Signal — the primary type name for the universal datum.
//!
//! `Signal` is now defined directly in the `engram` module as
//! `pub type Signal = Engram`. This module re-exports for convenience.

pub use crate::engram::{
    Engram as Signal, EngramBuilder as SignalBuilder, GraduationError, HdcFingerprint, SignalStatus,
};
