//! Live agent event types and the tool-step target projection (§03c T15).
//!
//! [`LiveAgentEvent`] is the discriminated union that a portal UI or any
//! consumer receives over an [`mpsc`] channel while an agent is running.
//! [`LiveOutput`] is the cheap, `Clone`able handle that agent internals hold
//! to emit those events.
//!
//! [`tool_step_target`] extracts the single human-readable "what is this call
//! touching" string from a tool's JSON input object, scrubs secrets from it,
//! collapses whitespace, and truncates to 120 characters.

use tokio::sync::mpsc;

use crate::{
    safety::scrub::{ScrubPolicy, scrub_secrets},
    tool_loop::StreamEventKind,
};

// ─── Public types ─────────────────────────────────────────────────────────────

/// An event emitted by the agent runtime while a run is in progress.
#[derive(Debug, Clone)]
pub enum LiveAgentEvent {
    /// A tool call is about to be (or is being) dispatched.
    ///
    /// Only the identity and target of the call are exposed here; the full
    /// arguments are intentionally excluded so that secrets that might appear
    /// in other fields never leak into the live feed.
    ToolStep {
        /// The tool-use ID assigned by the model.
        id: String,
        /// The tool name (e.g. `"Write"`, `"Bash"`).
        name: String,
        /// The projected, scrubbed, and truncated target string produced by
        /// [`tool_step_target`].
        target: String,
    },

    /// A raw streaming event that has not been screened for display safety.
    ///
    /// Consumers that want to render live token output should handle this
    /// variant but must be prepared for it to contain sensitive content.
    Unscreened(StreamEventKind),
}

/// A cheap, cloneable handle that agent code holds to emit [`LiveAgentEvent`]s.
///
/// When `trusted` is `false` the consumer should treat [`LiveAgentEvent::Unscreened`]
/// events as potentially sensitive and may choose to suppress them.
#[derive(Debug, Clone)]
pub struct LiveOutput {
    /// Channel to send events on.
    pub sink: mpsc::Sender<LiveAgentEvent>,
    /// `true` when the receiver is a trusted local consumer (e.g. the
    /// developer's TUI); `false` for remote / portal consumers.
    pub trusted: bool,
}

// ─── Tool step target projection ─────────────────────────────────────────────

/// Maximum byte length for the target string (before the `…` suffix is added).
const TARGET_MAX_CHARS: usize = 120;

/// Extract the single "what is this call touching" value from a tool's JSON
/// input object.
///
/// The fields are checked in this priority order and the first present
/// non-empty string value wins:
///
/// 1. `file_path`
/// 2. `notebook_path`
/// 3. `path`
/// 4. `command` — only the **first line** is kept
/// 5. `pattern`
/// 6. `url`
/// 7. `query`
/// 8. `description`
///
/// The result is then:
/// - scrubbed for secrets via [`scrub_secrets`] with the default [`ScrubPolicy`];
/// - whitespace-collapsed (runs of `\t`, `\n`, `\r`, etc. → single space, then trimmed);
/// - truncated to 120 characters on a char boundary, with a trailing `…` when cut.
///
/// If none of the fields are present or all are empty, an empty string is returned.
/// Nothing else from `input` is allowed to appear in the output.
#[must_use]
pub fn tool_step_target(_name: &str, input: &serde_json::Value) -> String {
    // Priority-ordered field names.
    const FIELDS: &[&str] = &[
        "file_path",
        "notebook_path",
        "path",
        "command",
        "pattern",
        "url",
        "query",
        "description",
    ];

    let raw: Option<String> = FIELDS.iter().find_map(|&field| {
        let s = input.get(field)?.as_str()?;
        if s.is_empty() {
            return None;
        }
        // For `command`, keep only the first line.
        if field == "command" {
            let first = s.lines().next().unwrap_or("").trim().to_string();
            if first.is_empty() { None } else { Some(first) }
        } else {
            Some(s.to_string())
        }
    });

    let raw = match raw {
        Some(v) => v,
        None => return String::new(),
    };

    // Scrub secrets.
    let scrubbed = scrub_secrets(&raw, &ScrubPolicy::default());

    // Collapse whitespace: replace each run of whitespace chars with a single
    // space, then trim leading/trailing spaces.
    let collapsed: String = scrubbed.split_whitespace().collect::<Vec<_>>().join(" ");

    // Truncate to TARGET_MAX_CHARS characters on a char boundary.
    if collapsed.chars().count() <= TARGET_MAX_CHARS {
        collapsed
    } else {
        // Find the byte offset of the TARGET_MAX_CHARS-th char.
        let cut = collapsed
            .char_indices()
            .nth(TARGET_MAX_CHARS)
            .map(|(i, _)| i)
            .unwrap_or(collapsed.len());
        let mut out = collapsed[..cut].to_string();
        out.push('…');
        out
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    // Helper: call with a dummy tool name.
    fn target(input: serde_json::Value) -> String {
        tool_step_target("dummy", &input)
    }

    // ── Write path ───────────────────────────────────────────────────────────

    #[test]
    fn write_path_returned() {
        let t = target(json!({ "file_path": "src/main.rs" }));
        assert_eq!(t, "src/main.rs");
    }

    // ── Read path ────────────────────────────────────────────────────────────

    #[test]
    fn read_path_returned() {
        let t = target(json!({ "file_path": "/etc/hosts" }));
        assert_eq!(t, "/etc/hosts");
    }

    // ── Edit path ────────────────────────────────────────────────────────────

    #[test]
    fn edit_path_returned() {
        // Edit tools pass file_path; path is the fallback.
        let t = target(json!({ "path": "lib/foo.rs" }));
        assert_eq!(t, "lib/foo.rs");
    }

    // ── Bash command — first line only ────────────────────────────────────────

    #[test]
    fn bash_command_first_line_only() {
        let t = target(json!({ "command": "echo hello\nrm -rf /tmp" }));
        assert_eq!(t, "echo hello");
    }

    // ── Bash command — truncated at 120 characters ────────────────────────────

    #[test]
    fn bash_command_truncated_at_120_chars() {
        // Build a command whose first line exceeds 120 characters.
        let long_cmd = "x".repeat(150);
        let t = target(json!({ "command": long_cmd }));
        // Should end with ellipsis and be 121 chars (120 + …).
        assert!(t.ends_with('…'), "expected '…' suffix, got: {t}");
        assert_eq!(
            t.chars().count(),
            TARGET_MAX_CHARS + 1,
            "expected {} chars + ellipsis, got: {} chars",
            TARGET_MAX_CHARS,
            t.chars().count()
        );
    }

    // ── API-key-shaped secret is scrubbed ─────────────────────────────────────

    #[test]
    fn api_key_in_command_is_scrubbed() {
        // sk- prefix with 20+ alphanumeric chars matches the OpenAI key pattern.
        let secret = "sk-".to_string() + &"A".repeat(25);
        let t = target(json!({ "command": format!("curl -H 'Authorization: {secret}'") }));
        assert!(!t.contains(&secret), "secret must be scrubbed: {t}");
        assert!(t.contains("[REDACTED]"), "expected [REDACTED] marker: {t}");
    }

    // ── Unknown tool gives empty string ───────────────────────────────────────

    #[test]
    fn unknown_tool_gives_empty_string() {
        let t = target(json!({ "foo": "bar", "baz": 42 }));
        assert_eq!(t, "", "expected empty string for unknown tool input");
    }

    // ── Multi-byte text is cut safely ─────────────────────────────────────────

    #[test]
    fn multibyte_text_cut_safely() {
        // Japanese characters are 3 bytes each in UTF-8.
        // Build a string of 130 × '文' (each 3 bytes).
        let long_text = "文".repeat(130);
        let t = target(json!({ "description": long_text }));
        assert!(t.ends_with('…'), "expected '…' suffix, got: {t}");
        // Must be valid UTF-8 (no panic, no mojibake).
        assert!(
            t.is_char_boundary(t.len()),
            "cut must land on char boundary"
        );
        assert_eq!(
            t.chars().count(),
            TARGET_MAX_CHARS + 1,
            "expected {} chars + ellipsis",
            TARGET_MAX_CHARS
        );
    }

    // ── Priority order ────────────────────────────────────────────────────────

    #[test]
    fn file_path_wins_over_path() {
        let t = target(json!({ "file_path": "a.rs", "path": "b.rs" }));
        assert_eq!(t, "a.rs");
    }

    // ── Empty field is skipped ────────────────────────────────────────────────

    #[test]
    fn empty_field_falls_through_to_next() {
        let t = target(json!({ "file_path": "", "path": "fallback.rs" }));
        assert_eq!(t, "fallback.rs");
    }
}
