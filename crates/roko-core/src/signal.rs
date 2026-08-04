//! Signal — forward-compatible alias for `Engram`.
//!
//! The full Engram→Signal rename happens in Phase 1. This module provides
//! the new import path so downstream code can start using `Signal` today.

pub use crate::engram::{
    Engram as Signal, EngramBuilder as SignalBuilder, GraduationError, HdcFingerprint, SignalStatus,
};
