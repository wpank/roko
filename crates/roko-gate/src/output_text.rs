//! A command's captured output as the gates read it.

use crate::verify_chain_gate::strip_ansi;

/// `bytes`, a command's captured stdout or stderr, as plain text: invalid
/// UTF-8 replaced, and escape sequences stripped (bug-4debc8).
///
/// A tool that colors its output starts each diagnostic line with an escape
/// sequence. cargo does under `CARGO_TERM_COLOR=always`, which CI sets, or
/// under `--color always`. That hides the diagnostic from the reasons and the
/// failure classification, which read the output line by line, and leaves
/// escapes in the detail a retry prompt shows.
#[must_use]
pub fn output_text(bytes: &[u8]) -> String {
    strip_ansi(&String::from_utf8_lossy(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// cargo's colored clippy output reads as the plain one.
    #[test]
    fn colored_cargo_output_reads_as_plain_text() {
        let colored = b"\x1b[1m\x1b[92m    Checking\x1b[0m fixture v0.1.0\n\x1b[1m\x1b[91merror\x1b[0m\x1b[1m: unused variable: `unused_value`\x1b[0m\n";
        assert_eq!(
            output_text(colored),
            "    Checking fixture v0.1.0\nerror: unused variable: `unused_value`\n"
        );
        assert_eq!(output_text(b"plain \xff"), "plain \u{fffd}");
    }
}
