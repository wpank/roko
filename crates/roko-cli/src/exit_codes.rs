//! Named CLI exit codes.
//!
//! Shared between the binary (`main.rs`) and library modules (e.g.
//! `custody.rs`) so all exit paths use symbolic constants instead of
//! bare `1` or `2`.

/// Successful execution.
pub const EXIT_SUCCESS: i32 = 0;
/// Agent or gate failure (logical error in the build).
pub const EXIT_FAILURE: i32 = 1;
/// Agent or gate failure (alias for `EXIT_FAILURE`, kept for backward compat).
pub const EXIT_AGENT_FAILURE: i32 = 1;
/// System error (I/O, config, infrastructure).
pub const EXIT_SYSTEM_ERROR: i32 = 2;
