//! Shared Server-Sent Events (SSE) line and frame parser.
//!
//! All SSE parsing in the workspace converges here so that whitespace handling,
//! comment stripping, and frame-accumulation logic are consistent.
//!
//! # SSE wire format (RFC 8895)
//!
//! An SSE stream is a sequence of UTF-8 lines separated by `\n` or `\r\n`.
//! Each line has the form `<field>:<optional single space><value>`.  A blank
//! line dispatches the accumulated event.  Lines starting with `:` are
//! comments and are ignored.
//!
//! Per the spec, the value starts immediately after the `:` separator; if the
//! first character of the value is a space (U+0020) it is stripped — exactly
//! one space.  We follow this rule for all fields rather than using
//! `str::trim_start()` (which would strip multiple spaces and tabs).
//!
//! ## Recognised field names
//!
//! | Field    | Handling                                  |
//! |----------|-------------------------------------------|
//! | `data`   | Accumulated; lines joined with `\n`        |
//! | `event`  | Sets the event type for the current frame |
//! | `id`     | Sets the last-event-ID for reconnection   |
//! | `retry`  | Ignored (reconnection timing)             |
//! | (other)  | Ignored                                   |

/// Strip exactly one leading space from a field value, per RFC 8895 §9.2.6.
///
/// This is the correct whitespace treatment for SSE field values.  `trim_start`
/// is intentionally avoided because it would silently consume multiple spaces
/// or tabs, which is incorrect for the spec and can corrupt intentionally
/// indented JSON values.
#[inline]
fn strip_one_space(value: &str) -> &str {
    value.strip_prefix(' ').unwrap_or(value)
}

/// A fully-parsed SSE frame dispatched by a blank line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SseFrame {
    /// The event type.  Defaults to `"message"` when no `event:` field is
    /// present in the frame, matching RFC 8895 §9.2.5.
    pub event: String,
    /// The accumulated data payload.  Multiple `data:` lines are joined with
    /// `\n`.
    pub data: String,
    /// The last-event-ID value, if present.
    pub id: Option<String>,
}

/// Parse a batch of SSE *lines* (already split on `\n`/`\r\n`) into complete
/// [`SseFrame`]s.
///
/// Unterminated frames (no trailing blank line) are flushed at the end so that
/// callers do not need to append a synthetic blank line to process the final
/// event in a body.
///
/// # Arguments
///
/// * `lines` — an iterator of `&str` slices, one per SSE line (without the
///   trailing newline character).
pub fn parse_sse_lines<'a>(lines: impl IntoIterator<Item = &'a str>) -> Vec<SseFrame> {
    let mut frames = Vec::new();
    let mut current_event = String::new();
    let mut current_data: Vec<String> = Vec::new();
    let mut current_id: Option<String> = None;

    for line in lines {
        if line.is_empty() {
            // Blank line — dispatch the accumulated frame.
            if !current_data.is_empty() {
                let event = if current_event.is_empty() {
                    "message".to_string()
                } else {
                    current_event.clone()
                };
                frames.push(SseFrame {
                    event,
                    data: current_data.join("\n"),
                    id: current_id.clone(),
                });
            }
            current_event.clear();
            current_data.clear();
            // Note: `current_id` persists across frames per spec — it is only
            // replaced when a new `id:` field arrives.
            continue;
        }

        if line.starts_with(':') {
            // Comment line — ignore.
            continue;
        }

        if let Some(rest) = line.strip_prefix("data:") {
            current_data.push(strip_one_space(rest).to_string());
        } else if let Some(rest) = line.strip_prefix("event:") {
            current_event = strip_one_space(rest).to_string();
        } else if let Some(rest) = line.strip_prefix("id:") {
            let id_val = strip_one_space(rest);
            if id_val.is_empty() {
                // Empty `id:` resets the last-event-ID per spec.
                current_id = None;
            } else {
                current_id = Some(id_val.to_string());
            }
        }
        // `retry:` and unknown fields are ignored.
    }

    // Flush any unterminated frame.
    if !current_data.is_empty() {
        let event = if current_event.is_empty() {
            "message".to_string()
        } else {
            current_event
        };
        frames.push(SseFrame {
            event,
            data: current_data.join("\n"),
            id: current_id,
        });
    }

    frames
}

/// Parse a raw SSE text body (containing embedded newlines) into complete
/// [`SseFrame`]s.
///
/// This is a thin wrapper around [`parse_sse_lines`] that handles `\r\n` line
/// endings as required by RFC 8895 §9.1.
pub fn parse_sse_text(text: &str) -> Vec<SseFrame> {
    parse_sse_lines(text.lines())
}

/// Extract the `data:` value from a single SSE line, if present.
///
/// Returns `None` when:
/// - the line does not start with `data:`, or
/// - the line is `data: [DONE]` (a streaming-protocol sentinel used by
///   OpenAI-compatible APIs).
///
/// This is the single-line variant used by streaming byte-stream consumers
/// that process lines one at a time rather than accumulating full frames.
#[must_use]
pub fn extract_sse_data(line: &str) -> Option<&str> {
    let value = strip_one_space(line.strip_prefix("data:")?);
    if value == "[DONE]" {
        return None;
    }
    Some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_sse_lines / parse_sse_text ─────────────────────────────────────

    #[test]
    fn single_data_frame() {
        let input = "data: hello\n\n";
        let frames = parse_sse_text(input);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].event, "message");
        assert_eq!(frames[0].data, "hello");
        assert!(frames[0].id.is_none());
    }

    #[test]
    fn frame_with_event_type() {
        let input = "event: ping\ndata: {}\n\n";
        let frames = parse_sse_text(input);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].event, "ping");
        assert_eq!(frames[0].data, "{}");
    }

    #[test]
    fn frame_with_id() {
        let input = "id: 42\ndata: hello\n\n";
        let frames = parse_sse_text(input);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].id.as_deref(), Some("42"));
    }

    #[test]
    fn id_persists_across_frames() {
        let input = "id: 1\ndata: a\n\ndata: b\n\n";
        let frames = parse_sse_text(input);
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].id.as_deref(), Some("1"));
        // id persists to the second frame because no new id: was sent.
        assert_eq!(frames[1].id.as_deref(), Some("1"));
    }

    #[test]
    fn empty_id_resets() {
        let input = "id: 1\ndata: a\n\nid:\ndata: b\n\n";
        let frames = parse_sse_text(input);
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].id.as_deref(), Some("1"));
        assert!(frames[1].id.is_none());
    }

    #[test]
    fn multiple_frames() {
        let input = concat!(
            "id: 1\n",
            "data: first\n",
            "\n",
            "id: 2\n",
            "data: second\n",
            "\n",
        );
        let frames = parse_sse_text(input);
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].data, "first");
        assert_eq!(frames[1].data, "second");
    }

    #[test]
    fn multiline_data_joined_with_newline() {
        let input = "data: line1\ndata: line2\n\n";
        let frames = parse_sse_text(input);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].data, "line1\nline2");
    }

    #[test]
    fn comment_lines_ignored() {
        let input = ": keepalive\ndata: hello\n\n";
        let frames = parse_sse_text(input);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].data, "hello");
    }

    #[test]
    fn unterminated_frame_flushed() {
        // No trailing blank line — the frame should still be emitted.
        let input = "data: last";
        let frames = parse_sse_text(input);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].data, "last");
    }

    #[test]
    fn empty_frame_skipped() {
        // Consecutive blank lines should not emit empty frames.
        let input = "\n\ndata: hello\n\n";
        let frames = parse_sse_text(input);
        assert_eq!(frames.len(), 1);
    }

    #[test]
    fn crlf_line_endings() {
        // `str::lines()` handles both \n and \r\n.
        let input = "data: hello\r\n\r\n";
        let frames = parse_sse_text(input);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].data, "hello");
    }

    #[test]
    fn exactly_one_leading_space_stripped() {
        // RFC 8895 strips exactly one space after the colon.
        let input = "data:  two spaces\n\n";
        let frames = parse_sse_text(input);
        assert_eq!(frames[0].data, " two spaces"); // one space stripped, one remains
    }

    #[test]
    fn no_space_after_colon() {
        let input = "data:nospace\n\n";
        let frames = parse_sse_text(input);
        assert_eq!(frames[0].data, "nospace");
    }

    // ── extract_sse_data ─────────────────────────────────────────────────────

    #[test]
    fn extract_sse_data_basic() {
        assert_eq!(extract_sse_data("data: hello"), Some("hello"));
    }

    #[test]
    fn extract_sse_data_no_space() {
        assert_eq!(extract_sse_data("data:hello"), Some("hello"));
    }

    #[test]
    fn extract_sse_data_done_sentinel() {
        assert_eq!(extract_sse_data("data: [DONE]"), None);
        assert_eq!(extract_sse_data("data:[DONE]"), None);
    }

    #[test]
    fn extract_sse_data_non_data_line() {
        assert!(extract_sse_data("event: ping").is_none());
        assert!(extract_sse_data(": comment").is_none());
        assert!(extract_sse_data("").is_none());
    }
}
