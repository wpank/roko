//! Per-platform message formatters (#430).
//!
//! Each formatter converts a [`RichMessage`] into platform-specific markup
//! that can be sent directly to the wire.  Three formatters are provided:
//!
//! | Formatter | Platform | Markup |
//! |---|---|---|
//! | [`DiscordFormatter`] | Discord | CommonMark Markdown (subset) |
//! | [`SlackFormatter`] | Slack | `mrkdwn` notation |
//! | [`PlainTextFormatter`] | All | Plain text, no markup |
//!
//! Use [`PlatformFormatter`] to select the right formatter from a platform
//! `kind` string.
//!
//! # Example
//!
//! ```rust
//! use roko_runtime::message_formatter::{MessageFormatter, PlatformFormatter, FormattedMessage};
//! use roko_runtime::platforms::RichMessage;
//!
//! let msg = RichMessage::markdown("**hello**", "**hello**");
//! let fmt = PlatformFormatter::for_kind("discord");
//! let out: FormattedMessage = fmt.format(&msg);
//! assert!(out.body.contains("**hello**"));
//! ```

use crate::platforms::RichMessage;

// ---------------------------------------------------------------------------
// FormattedMessage — output type
// ---------------------------------------------------------------------------

/// The rendered body for a single platform message send.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormattedMessage {
    /// The formatted message body ready to send to the platform API.
    pub body: String,
    /// Whether the formatter applied rich markup (false = plain text).
    pub is_rich: bool,
}

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// Formats a [`RichMessage`] for a specific platform's wire protocol.
pub trait MessageFormatter: Send + Sync {
    /// Render `msg` into platform-specific markup.
    fn format(&self, msg: &RichMessage) -> FormattedMessage;
}

// ---------------------------------------------------------------------------
// Discord formatter
// ---------------------------------------------------------------------------

/// Formats messages as Discord-compatible CommonMark Markdown.
///
/// Discord supports a subset of Markdown: `**bold**`, `*italic*`, `` `code` ``,
/// blockquotes, and bare URLs are all rendered.  When the message includes a
/// `markdown` field that field is used as-is; otherwise `text` is sent.
pub struct DiscordFormatter;

impl MessageFormatter for DiscordFormatter {
    fn format(&self, msg: &RichMessage) -> FormattedMessage {
        let body = msg
            .markdown
            .clone()
            .unwrap_or_else(|| msg.text.clone());
        FormattedMessage { body, is_rich: true }
    }
}

// ---------------------------------------------------------------------------
// Slack formatter
// ---------------------------------------------------------------------------

/// Formats messages using Slack `mrkdwn` notation.
///
/// Slack `mrkdwn` uses `*bold*`, `_italic_`, `` `code` ``, `~strike~`, and
/// `<URL|text>` link syntax — distinct from CommonMark.  This formatter
/// attempts a best-effort conversion from Markdown (stripping double-asterisks
/// to single and converting `**…**` to `*…*`) when a `markdown` field is
/// present; if not it wraps the plain `text` field unchanged.
pub struct SlackFormatter;

impl MessageFormatter for SlackFormatter {
    fn format(&self, msg: &RichMessage) -> FormattedMessage {
        let body = if let Some(md) = &msg.markdown {
            // Convert the most common Markdown → mrkdwn differences.
            markdown_to_mrkdwn(md)
        } else {
            msg.text.clone()
        };
        FormattedMessage { body, is_rich: true }
    }
}

/// Best-effort Markdown → Slack `mrkdwn` conversion.
///
/// Converts: `**text**` → `*text*`, `__text__` → `_text_`.
/// Leaves `*text*` and `_text_` unchanged (already mrkdwn).
fn markdown_to_mrkdwn(md: &str) -> String {
    // Replace **bold** with *bold* (Slack mrkdwn for bold).
    let s = md.replace("**", "*");
    // Replace __italic__ with _italic_ (Slack mrkdwn for italic).
    s.replace("__", "_")
}

// ---------------------------------------------------------------------------
// Plain-text formatter
// ---------------------------------------------------------------------------

/// Always returns the plain `text` field of the message.
///
/// Used as a safe fallback for unknown platform kinds, terminals, and
/// platforms that do not support any markup.
pub struct PlainTextFormatter;

impl MessageFormatter for PlainTextFormatter {
    fn format(&self, msg: &RichMessage) -> FormattedMessage {
        FormattedMessage {
            body: msg.text.clone(),
            is_rich: false,
        }
    }
}

// ---------------------------------------------------------------------------
// PlatformFormatter — selector
// ---------------------------------------------------------------------------

/// Selects the appropriate formatter from a platform `kind` string.
///
/// Falls back to [`PlainTextFormatter`] for unknown kinds.
pub enum PlatformFormatter {
    /// Discord platform formatter (preserves markdown).
    Discord(DiscordFormatter),
    /// Slack platform formatter (converts to mrkdwn).
    Slack(SlackFormatter),
    /// Plain-text fallback formatter.
    Plain(PlainTextFormatter),
}

impl PlatformFormatter {
    /// Select a formatter based on the platform `kind` field.
    ///
    /// Known kinds: `"discord"`, `"slack"`.  All others use plain text.
    #[must_use]
    pub fn for_kind(kind: &str) -> Self {
        match kind {
            "discord" => Self::Discord(DiscordFormatter),
            "slack" => Self::Slack(SlackFormatter),
            _ => Self::Plain(PlainTextFormatter),
        }
    }
}

impl MessageFormatter for PlatformFormatter {
    fn format(&self, msg: &RichMessage) -> FormattedMessage {
        match self {
            Self::Discord(f) => f.format(msg),
            Self::Slack(f) => f.format(msg),
            Self::Plain(f) => f.format(msg),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platforms::RichMessage;

    #[test]
    fn discord_uses_markdown_field() {
        let msg = RichMessage::markdown("plain", "**bold**");
        let out = DiscordFormatter.format(&msg);
        assert_eq!(out.body, "**bold**");
        assert!(out.is_rich);
    }

    #[test]
    fn discord_falls_back_to_text() {
        let msg = RichMessage::plain("hello world");
        let out = DiscordFormatter.format(&msg);
        assert_eq!(out.body, "hello world");
    }

    #[test]
    fn slack_converts_bold() {
        let msg = RichMessage::markdown("plain", "**important** note");
        let out = SlackFormatter.format(&msg);
        // **important** becomes *important* in mrkdwn.
        assert_eq!(out.body, "*important* note");
        assert!(out.is_rich);
    }

    #[test]
    fn slack_falls_back_to_text_when_no_markdown() {
        let msg = RichMessage::plain("just text");
        let out = SlackFormatter.format(&msg);
        assert_eq!(out.body, "just text");
    }

    #[test]
    fn plain_always_returns_text_field() {
        let msg = RichMessage::markdown("plain text", "**markdown**");
        let out = PlainTextFormatter.format(&msg);
        assert_eq!(out.body, "plain text");
        assert!(!out.is_rich);
    }

    #[test]
    fn platform_formatter_discord() {
        let msg = RichMessage::markdown("plain", "_italic_");
        let fmt = PlatformFormatter::for_kind("discord");
        let out = fmt.format(&msg);
        assert_eq!(out.body, "_italic_");
        assert!(out.is_rich);
    }

    #[test]
    fn platform_formatter_slack() {
        let msg = RichMessage::markdown("p", "**title**");
        let fmt = PlatformFormatter::for_kind("slack");
        let out = fmt.format(&msg);
        assert_eq!(out.body, "*title*");
    }

    #[test]
    fn platform_formatter_unknown_kind_falls_back_to_plain() {
        let msg = RichMessage::markdown("plain", "**md**");
        let fmt = PlatformFormatter::for_kind("matrix");
        let out = fmt.format(&msg);
        assert_eq!(out.body, "plain");
        assert!(!out.is_rich);
    }

    #[test]
    fn markdown_to_mrkdwn_double_bold() {
        assert_eq!(
            markdown_to_mrkdwn("**hello** and __world__"),
            "*hello* and _world_"
        );
    }
}
