//! T024: Streaming contract tests verifying sequence ordering and backpressure.
//!
//! These tests exercise the `StreamEvent` / `StreamEventKind` protocol at the
//! contract level, verifying that:
//! - Events arrive in a valid ordering (TextDelta before Done, ToolCallStart
//!   before ToolCallDelta/End, etc.)
//! - Backpressure from a slow consumer does not drop events or reorder them
//! - Concurrent tool-call streams interleave correctly
//! - Usage events arrive before Done

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use roko_agent::tool_loop::{StreamEvent, StreamEventKind};
use roko_agent::usage::Usage;
use tokio::sync::mpsc;

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Collect all events from an mpsc channel into a vec.
async fn collect_events(mut rx: mpsc::Receiver<StreamEvent>) -> Vec<StreamEvent> {
    let mut events = Vec::new();
    while let Some(ev) = rx.recv().await {
        events.push(ev);
    }
    events
}

/// Classify a StreamEventKind into a stable tag for ordering assertions.
fn event_tag(kind: &StreamEventKind) -> &'static str {
    match kind {
        StreamEventKind::TextDelta(_) => "text_delta",
        StreamEventKind::ReasoningDelta(_) => "reasoning_delta",
        StreamEventKind::ToolCallStart { .. } => "tool_call_start",
        StreamEventKind::ToolCallDelta { .. } => "tool_call_delta",
        StreamEventKind::ToolCallEnd { .. } => "tool_call_end",
        StreamEventKind::ToolResult { .. } => "tool_result",
        StreamEventKind::Usage(_) => "usage",
        StreamEventKind::Done { .. } => "done",
    }
}

// ── Sequence ordering ────────────────────────────────────────────────────────

/// `Done` must be the last event in any valid stream.
#[tokio::test]
async fn done_is_terminal() {
    let (tx, rx) = mpsc::channel(32);

    tx.send(StreamEvent::now(StreamEventKind::TextDelta("hello".into())))
        .await
        .unwrap();
    tx.send(StreamEvent::now(StreamEventKind::Usage(Usage::default())))
        .await
        .unwrap();
    tx.send(StreamEvent::now(StreamEventKind::Done {
        finish_reason: "stop".into(),
    }))
    .await
    .unwrap();
    drop(tx);

    let events = collect_events(rx).await;
    let last = events.last().expect("non-empty event stream");
    assert_eq!(event_tag(&last.kind), "done", "Done must be the last event");
}

/// `TextDelta` events must come before `Done`.
#[tokio::test]
async fn text_deltas_precede_done() {
    let (tx, rx) = mpsc::channel(32);

    let deltas = vec!["Hello", ", ", "world", "!"];
    for d in &deltas {
        tx.send(StreamEvent::now(StreamEventKind::TextDelta(
            (*d).to_string(),
        )))
        .await
        .unwrap();
    }
    tx.send(StreamEvent::now(StreamEventKind::Done {
        finish_reason: "stop".into(),
    }))
    .await
    .unwrap();
    drop(tx);

    let events = collect_events(rx).await;
    let done_idx = events
        .iter()
        .position(|e| matches!(e.kind, StreamEventKind::Done { .. }))
        .expect("Done must be present");
    let last_text_idx = events
        .iter()
        .rposition(|e| matches!(e.kind, StreamEventKind::TextDelta(_)))
        .expect("TextDelta must be present");

    assert!(last_text_idx < done_idx, "all TextDeltas must precede Done");
}

/// `ToolCallStart` must precede its `ToolCallDelta` and `ToolCallEnd`.
#[tokio::test]
async fn tool_call_start_precedes_delta_and_end() {
    let (tx, rx) = mpsc::channel(32);

    tx.send(StreamEvent::now(StreamEventKind::ToolCallStart {
        id: "c1".into(),
        name: "read_file".into(),
    }))
    .await
    .unwrap();
    tx.send(StreamEvent::now(StreamEventKind::ToolCallDelta {
        id: "c1".into(),
        json_fragment: r#"{"path":"#.into(),
    }))
    .await
    .unwrap();
    tx.send(StreamEvent::now(StreamEventKind::ToolCallDelta {
        id: "c1".into(),
        json_fragment: r#""f.rs"}"#.into(),
    }))
    .await
    .unwrap();
    tx.send(StreamEvent::now(StreamEventKind::ToolCallEnd {
        id: "c1".into(),
        name: "read_file".into(),
        args: serde_json::json!({"path": "f.rs"}),
    }))
    .await
    .unwrap();
    tx.send(StreamEvent::now(StreamEventKind::Done {
        finish_reason: "tool_calls".into(),
    }))
    .await
    .unwrap();
    drop(tx);

    let events = collect_events(rx).await;
    let start_idx = events
        .iter()
        .position(|e| matches!(e.kind, StreamEventKind::ToolCallStart { .. }))
        .expect("ToolCallStart must be present");
    let first_delta_idx = events
        .iter()
        .position(|e| matches!(e.kind, StreamEventKind::ToolCallDelta { .. }))
        .expect("ToolCallDelta must be present");
    let end_idx = events
        .iter()
        .position(|e| matches!(e.kind, StreamEventKind::ToolCallEnd { .. }))
        .expect("ToolCallEnd must be present");

    assert!(
        start_idx < first_delta_idx,
        "ToolCallStart must precede ToolCallDelta"
    );
    assert!(
        first_delta_idx < end_idx,
        "ToolCallDelta must precede ToolCallEnd"
    );
}

// ── Interleaved parallel tool calls ──────────────────────────────────────────

/// Interleaved deltas for multiple tool calls preserve per-call ordering.
#[tokio::test]
async fn interleaved_tool_calls_preserve_per_call_ordering() {
    let (tx, rx) = mpsc::channel(32);

    // Start two tool calls.
    tx.send(StreamEvent::now(StreamEventKind::ToolCallStart {
        id: "a".into(),
        name: "read_file".into(),
    }))
    .await
    .unwrap();
    tx.send(StreamEvent::now(StreamEventKind::ToolCallStart {
        id: "b".into(),
        name: "write_file".into(),
    }))
    .await
    .unwrap();

    // Interleave deltas.
    tx.send(StreamEvent::now(StreamEventKind::ToolCallDelta {
        id: "a".into(),
        json_fragment: r#"{"path":"#.into(),
    }))
    .await
    .unwrap();
    tx.send(StreamEvent::now(StreamEventKind::ToolCallDelta {
        id: "b".into(),
        json_fragment: r#"{"path":"b.txt"}"#.into(),
    }))
    .await
    .unwrap();
    tx.send(StreamEvent::now(StreamEventKind::ToolCallDelta {
        id: "a".into(),
        json_fragment: r#""a.txt"}"#.into(),
    }))
    .await
    .unwrap();

    // End both.
    tx.send(StreamEvent::now(StreamEventKind::ToolCallEnd {
        id: "a".into(),
        name: "read_file".into(),
        args: serde_json::json!({"path": "a.txt"}),
    }))
    .await
    .unwrap();
    tx.send(StreamEvent::now(StreamEventKind::ToolCallEnd {
        id: "b".into(),
        name: "write_file".into(),
        args: serde_json::json!({"path": "b.txt"}),
    }))
    .await
    .unwrap();

    tx.send(StreamEvent::now(StreamEventKind::Done {
        finish_reason: "tool_calls".into(),
    }))
    .await
    .unwrap();
    drop(tx);

    let events = collect_events(rx).await;

    // Per-call ordering: for each id, starts come before deltas come before ends.
    for call_id in &["a", "b"] {
        let indices: Vec<usize> = events
            .iter()
            .enumerate()
            .filter(|(_, e)| match &e.kind {
                StreamEventKind::ToolCallStart { id, .. }
                | StreamEventKind::ToolCallDelta { id, .. }
                | StreamEventKind::ToolCallEnd { id, .. } => id == call_id,
                _ => false,
            })
            .map(|(i, _)| i)
            .collect();

        assert!(indices.len() >= 2, "at least start + end for {call_id}");

        let tags: Vec<&str> = indices
            .iter()
            .map(|&i| event_tag(&events[i].kind))
            .collect();

        // First must be start, last must be end.
        assert_eq!(
            tags.first().copied(),
            Some("tool_call_start"),
            "first event for {call_id} must be start"
        );
        assert_eq!(
            tags.last().copied(),
            Some("tool_call_end"),
            "last event for {call_id} must be end"
        );
    }
}

// ── Usage precedes Done ──────────────────────────────────────────────────────

/// Usage events, when present, arrive before Done.
#[tokio::test]
async fn usage_arrives_before_done() {
    let (tx, rx) = mpsc::channel(32);

    tx.send(StreamEvent::now(StreamEventKind::TextDelta("hi".into())))
        .await
        .unwrap();
    tx.send(StreamEvent::now(StreamEventKind::Usage(Usage {
        input_tokens: 100,
        output_tokens: 50,
        ..Default::default()
    })))
    .await
    .unwrap();
    tx.send(StreamEvent::now(StreamEventKind::Done {
        finish_reason: "stop".into(),
    }))
    .await
    .unwrap();
    drop(tx);

    let events = collect_events(rx).await;
    let usage_idx = events
        .iter()
        .position(|e| matches!(e.kind, StreamEventKind::Usage(_)))
        .expect("Usage must be present");
    let done_idx = events
        .iter()
        .position(|e| matches!(e.kind, StreamEventKind::Done { .. }))
        .expect("Done must be present");

    assert!(usage_idx < done_idx, "Usage must arrive before Done");
}

// ── Backpressure ─────────────────────────────────────────────────────────────

/// A slow consumer (bounded channel of 1) does not drop or reorder events.
#[tokio::test]
async fn bounded_channel_backpressure_preserves_order() {
    let (tx, rx) = mpsc::channel(1); // minimal buffer

    let expected_count = Arc::new(AtomicUsize::new(0));
    let count_clone = Arc::clone(&expected_count);

    let producer = tokio::spawn(async move {
        for i in 0..20 {
            let event = StreamEvent::now(StreamEventKind::TextDelta(format!("chunk-{i}")));
            tx.send(event).await.unwrap();
            count_clone.fetch_add(1, Ordering::SeqCst);
        }
        tx.send(StreamEvent::now(StreamEventKind::Done {
            finish_reason: "stop".into(),
        }))
        .await
        .unwrap();
        count_clone.fetch_add(1, Ordering::SeqCst);
    });

    // Slow consumer: pause 1ms between reads.
    let consumer = tokio::spawn(async move {
        let mut received = Vec::new();
        let mut rx = rx;
        while let Some(ev) = rx.recv().await {
            received.push(ev);
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        received
    });

    producer.await.unwrap();
    let received = consumer.await.unwrap();

    let total_sent = expected_count.load(Ordering::SeqCst);
    assert_eq!(
        received.len(),
        total_sent,
        "all sent events must be received (no drops)"
    );

    // Verify ordering: text deltas in sequence, Done last.
    let text_chunks: Vec<String> = received
        .iter()
        .filter_map(|e| match &e.kind {
            StreamEventKind::TextDelta(t) => Some(t.clone()),
            _ => None,
        })
        .collect();
    for (i, chunk) in text_chunks.iter().enumerate() {
        assert_eq!(
            chunk,
            &format!("chunk-{i}"),
            "text deltas must arrive in order"
        );
    }

    assert!(
        matches!(
            received.last().map(|e| &e.kind),
            Some(StreamEventKind::Done { .. })
        ),
        "Done must be the last event"
    );
}

// ── Timestamp monotonicity ───────────────────────────────────────────────────

/// Timestamps across events are monotonically non-decreasing.
#[tokio::test]
async fn timestamps_are_monotonically_nondecreasing() {
    let (tx, rx) = mpsc::channel(32);

    for i in 0..10 {
        tx.send(StreamEvent::now(StreamEventKind::TextDelta(format!(
            "d{i}"
        ))))
        .await
        .unwrap();
    }
    tx.send(StreamEvent::now(StreamEventKind::Done {
        finish_reason: "stop".into(),
    }))
    .await
    .unwrap();
    drop(tx);

    let events = collect_events(rx).await;
    for window in events.windows(2) {
        assert!(
            window[0].timestamp <= window[1].timestamp,
            "timestamps must be non-decreasing"
        );
    }
}

// ── Empty stream ─────────────────────────────────────────────────────────────

/// A stream with only Done is valid (empty content).
#[tokio::test]
async fn done_only_stream_is_valid() {
    let (tx, rx) = mpsc::channel(32);

    tx.send(StreamEvent::now(StreamEventKind::Done {
        finish_reason: "stop".into(),
    }))
    .await
    .unwrap();
    drop(tx);

    let events = collect_events(rx).await;
    assert_eq!(events.len(), 1);
    assert_eq!(event_tag(&events[0].kind), "done");
}

// ── ReasoningDelta ordering ──────────────────────────────────────────────────

/// ReasoningDelta events can precede TextDelta events (thinking before speaking).
#[tokio::test]
async fn reasoning_before_text_is_valid() {
    let (tx, rx) = mpsc::channel(32);

    tx.send(StreamEvent::now(StreamEventKind::ReasoningDelta(
        "thinking...".into(),
    )))
    .await
    .unwrap();
    tx.send(StreamEvent::now(StreamEventKind::TextDelta(
        "answer".into(),
    )))
    .await
    .unwrap();
    tx.send(StreamEvent::now(StreamEventKind::Done {
        finish_reason: "stop".into(),
    }))
    .await
    .unwrap();
    drop(tx);

    let events = collect_events(rx).await;
    let reasoning_idx = events
        .iter()
        .position(|e| matches!(e.kind, StreamEventKind::ReasoningDelta(_)))
        .expect("ReasoningDelta must be present");
    let text_idx = events
        .iter()
        .position(|e| matches!(e.kind, StreamEventKind::TextDelta(_)))
        .expect("TextDelta must be present");
    let done_idx = events
        .iter()
        .position(|e| matches!(e.kind, StreamEventKind::Done { .. }))
        .expect("Done must be present");

    assert!(reasoning_idx < text_idx);
    assert!(text_idx < done_idx);
}
