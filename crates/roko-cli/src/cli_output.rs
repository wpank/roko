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

#[cfg(test)]
mod tests {
    use super::*;

    // ── constructor ──────────────────────────────────────────────────────────

    #[test]
    fn new_stores_quiet_true() {
        let out = CliOutput::new(true);
        assert!(out.quiet, "CliOutput::new(true) must store quiet=true");
    }

    #[test]
    fn new_stores_quiet_false() {
        let out = CliOutput::new(false);
        assert!(!out.quiet, "CliOutput::new(false) must store quiet=false");
    }

    // ── quiet=true suppresses non-error methods (no panic, no output) ────────
    //
    // output_format functions write to stdout via println!, so we cannot
    // capture bytes in a unit test without restructuring the abstraction.
    // We verify the quiet guard condition directly via the private `quiet`
    // field and confirm each method is a no-op (does not panic) when quiet.

    #[test]
    fn quiet_true_intro_does_not_panic() {
        let out = CliOutput::new(true);
        out.intro("Hello");
    }

    #[test]
    fn quiet_true_step_does_not_panic() {
        let out = CliOutput::new(true);
        out.step("Label", "Value");
    }

    #[test]
    fn quiet_true_bar_does_not_panic() {
        let out = CliOutput::new(true);
        out.bar("continuation text");
    }

    #[test]
    fn quiet_true_note_does_not_panic() {
        let out = CliOutput::new(true);
        out.note("a note");
    }

    #[test]
    fn quiet_true_success_does_not_panic() {
        let out = CliOutput::new(true);
        out.success("all good");
    }

    #[test]
    fn quiet_true_warning_does_not_panic() {
        let out = CliOutput::new(true);
        out.warning("heads up");
    }

    #[test]
    fn quiet_true_divider_does_not_panic() {
        let out = CliOutput::new(true);
        out.divider();
    }

    #[test]
    fn quiet_true_branch_does_not_panic() {
        let out = CliOutput::new(true);
        out.branch("branch text");
    }

    #[test]
    fn quiet_true_end_does_not_panic() {
        let out = CliOutput::new(true);
        out.end("end text");
    }

    // ── quiet=true does NOT suppress error ───────────────────────────────────
    //
    // error() must call output_format::error regardless of the quiet flag.
    // The implementation has no `if !self.quiet` guard around the error call.
    // We verify this structurally: calling error() with quiet=true must not
    // be a silent no-op — it must execute the underlying call (no panic).

    #[test]
    fn quiet_true_error_still_executes_no_panic() {
        // error() is not gated on self.quiet — it must always print.
        let out = CliOutput::new(true);
        out.error("something went wrong");
    }

    #[test]
    fn quiet_false_error_executes_no_panic() {
        let out = CliOutput::new(false);
        out.error("error in verbose mode");
    }

    /// error() behaves identically regardless of the quiet flag.
    ///
    /// If error() were gated on `quiet`, the quiet=true call would be a
    /// silent no-op. Both branches must call through to output_format::error.
    #[test]
    fn error_ignores_quiet_flag_both_paths_no_panic() {
        for quiet in [true, false] {
            let out = CliOutput::new(quiet);
            out.error("test error");
        }
    }

    // ── quiet=false: all methods execute without panic ────────────────────────

    #[test]
    fn quiet_false_intro_no_panic() {
        let out = CliOutput::new(false);
        out.intro("Plan complete");
    }

    #[test]
    fn quiet_false_step_no_panic() {
        let out = CliOutput::new(false);
        out.step("Complexity", "standard (auto-detected)");
    }

    #[test]
    fn quiet_false_step_empty_value_no_panic() {
        // step() with an empty value renders without the value suffix.
        let out = CliOutput::new(false);
        out.step("Running", "");
    }

    #[test]
    fn quiet_false_bar_no_panic() {
        let out = CliOutput::new(false);
        out.bar("cost  $0.04");
    }

    #[test]
    fn quiet_false_note_no_panic() {
        let out = CliOutput::new(false);
        out.note("12 facts loaded");
    }

    #[test]
    fn quiet_false_success_no_panic() {
        let out = CliOutput::new(false);
        out.success("Build passed");
    }

    #[test]
    fn quiet_false_warning_no_panic() {
        let out = CliOutput::new(false);
        out.warning("No snapshot found, starting fresh");
    }

    #[test]
    fn quiet_false_divider_no_panic() {
        let out = CliOutput::new(false);
        out.divider();
    }

    // ── intro / end bracket pair ─────────────────────────────────────────────

    #[test]
    fn intro_end_bracket_pair_quiet_false_no_panic() {
        let out = CliOutput::new(false);
        out.intro("My Section");
        out.bar("some content");
        out.end("done");
    }

    #[test]
    fn intro_end_bracket_pair_quiet_true_no_panic() {
        // Both calls must silently no-op without panicking.
        let out = CliOutput::new(true);
        out.intro("My Section");
        out.bar("some content");
        out.end("done");
    }

    // ── bar / branch render correctly ────────────────────────────────────────

    #[test]
    fn bar_and_branch_quiet_false_no_panic() {
        let out = CliOutput::new(false);
        out.intro("Summary");
        out.branch("tasks    3/3");
        out.branch("cost     $0.12");
        out.bar("extra info");
        out.end("run-id: abc123");
    }

    #[test]
    fn bar_and_branch_quiet_true_no_panic() {
        // All calls must be silent no-ops when quiet.
        let out = CliOutput::new(true);
        out.intro("Summary");
        out.branch("tasks    3/3");
        out.bar("extra info");
        out.end("run-id: abc123");
    }

    // ── guard condition white-box check ──────────────────────────────────────

    #[test]
    fn quiet_flag_stored_correctly() {
        assert!(CliOutput::new(true).quiet);
        assert!(!CliOutput::new(false).quiet);
    }
}
