//! Quiet-aware output wrapper for CLI commands.
//!
//! `CliOutput` delegates to `output_format` primitives and suppresses all
//! non-error output when the `--quiet` flag is set. Error messages always
//! print regardless of the quiet flag.

use crate::output_format;

/// Thin wrapper around the `output_format` primitives that respects `--quiet`.
///
/// All methods other than [`CliOutput::error`] are silenced when
/// `quiet` is `true`. Errors always print so the user knows something went
/// wrong even in scripted/CI contexts.
pub struct CliOutput {
    quiet: bool,
}

impl CliOutput {
    /// Create a new `CliOutput`. When `quiet` is `true`, all output except
    /// [`error`][Self::error] is suppressed.
    pub fn new(quiet: bool) -> Self {
        Self { quiet }
    }

    /// Print the intro line `◆  <title>` in bold. Suppressed when quiet.
    pub fn intro(&self, title: &str) {
        if !self.quiet {
            output_format::intro(title);
        }
    }

    /// Print a step line `◇  <label>  <value>`. Suppressed when quiet.
    pub fn step(&self, label: &str, value: &str) {
        if !self.quiet {
            output_format::step(label, value);
        }
    }

    /// Print a continuation line `│  <text>`. Suppressed when quiet.
    pub fn bar(&self, text: &str) {
        if !self.quiet {
            output_format::bar(text);
        }
    }

    /// Print a note line `│  <text>` in dim style. Suppressed when quiet.
    pub fn note(&self, text: &str) {
        if !self.quiet {
            output_format::note(text);
        }
    }

    /// Print a success line `✔  <msg>` in green. Suppressed when quiet.
    pub fn success(&self, msg: &str) {
        if !self.quiet {
            output_format::success(msg);
        }
    }

    /// Print an error line `✖  <msg>` in red. Always shown, ignores quiet.
    pub fn error(&self, msg: &str) {
        output_format::error(msg);
    }

    /// Print a warning line `⚠  <msg>` in yellow. Suppressed when quiet.
    pub fn warning(&self, msg: &str) {
        if !self.quiet {
            output_format::warning(msg);
        }
    }

    /// Print an empty `│` line (visual spacer). Suppressed when quiet.
    pub fn divider(&self) {
        if !self.quiet {
            output_format::divider();
        }
    }

    /// Print a branch line `├  <text>`. Suppressed when quiet.
    pub fn branch(&self, text: &str) {
        if !self.quiet {
            output_format::branch(text);
        }
    }

    /// Print an end line `└  <text>`. Suppressed when quiet.
    pub fn end(&self, text: &str) {
        if !self.quiet {
            output_format::end(text);
        }
    }
}
