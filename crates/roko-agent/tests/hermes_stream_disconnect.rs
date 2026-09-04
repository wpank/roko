//! T6: Hermes stream disconnect fixture test.
//!
//! Feeds the `chat_stream_disconnect.sse` fixture through the SSE parser
//! and verifies graceful handling: no panic, partial content is captured,
//! and no `[DONE]` marker appears.

#![allow(missing_docs)]

use roko_agent::streaming::parse_sse_line;
use roko_agent::tool_loop::StreamEventKind;

static DISCONNECT_FIXTURE: &str = include_str!("fixtures/hermes/http/chat_stream_disconnect.sse");

/// The disconnect fixture should produce partial content but no [DONE].
#[test]
fn disconnect_fixture_produces_partial_content_no_done() {
    let mut content = String::new();
    let mut saw_done = false;
    let mut event_count = 0u32;

    for line in DISCONNECT_FIXTURE.lines() {
        if let Some(event) = parse_sse_line(line) {
            event_count += 1;
            match &event.kind {
                StreamEventKind::TextDelta(delta) => content.push_str(delta),
                StreamEventKind::Done { .. } => saw_done = true,
                _ => {}
            }
        }
    }

    // Must NOT have a [DONE] — the stream was disconnected.
    assert!(
        !saw_done,
        "disconnect fixture should NOT contain [DONE] marker"
    );

    // Must have captured partial content.
    assert_eq!(content, "I'm working on your request");

    // Must have parsed at least one event.
    assert!(
        event_count >= 2,
        "expected at least 2 events, got {event_count}"
    );
}

/// Each line of the disconnect fixture parses without panic.
#[test]
fn disconnect_fixture_all_lines_parse_without_panic() {
    // This test verifies graceful error handling: feeding every line
    // (including empty lines and non-data lines) through parse_sse_line
    // should never panic.
    let mut parsed = 0u32;
    let mut skipped = 0u32;

    for line in DISCONNECT_FIXTURE.lines() {
        match parse_sse_line(line) {
            Some(_) => parsed += 1,
            None => skipped += 1,
        }
    }

    // We expect some lines to be parsed (data lines) and some skipped
    // (empty lines between SSE events).
    assert!(parsed > 0, "expected at least one parsed SSE event");
    assert!(skipped > 0, "expected at least one skipped line (empty)");
}

/// An abruptly truncated SSE chunk (no newline at end) is handled gracefully.
#[test]
fn truncated_sse_chunk_does_not_panic() {
    // Simulate a mid-chunk disconnect: the JSON is cut off.
    let truncated =
        r#"data: {"id":"chatcmpl-hermes-crash","choices":[{"index":0,"delta":{"content":"partial"#;
    let result = parse_sse_line(truncated);
    // Should return None (invalid JSON), not panic.
    assert!(
        result.is_none(),
        "truncated JSON should fail gracefully, got: {result:?}"
    );
}

/// An empty data line returns None.
#[test]
fn empty_data_line_returns_none() {
    assert!(parse_sse_line("data:").is_none() || parse_sse_line("data: ").is_none());
}

/// A standard [DONE] line produces a Done event.
#[test]
fn done_marker_produces_done_event() {
    let event = parse_sse_line("data: [DONE]").expect("should parse [DONE]");
    assert!(
        matches!(event.kind, StreamEventKind::Done { .. }),
        "expected Done event, got: {:?}",
        event.kind
    );
}
