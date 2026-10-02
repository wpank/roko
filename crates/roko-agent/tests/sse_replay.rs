//! backlog 1112: one real GLM-4.7 agent turn, replayed through the SSE parser.
//!
//! `fixtures/sse/glm-4.7-agent-turn.sse` is the raw body Z.ai streamed for one
//! `glm-4.7` turn on 2026-10-02. `roko do` sent it with roko's Graph implementer
//! prompt, its eight tools and a task that needs `read_file`; a local proxy
//! between roko and api.z.ai teed the response. Only the response id was
//! scrubbed: every other byte is as received.
//!
//! The capture answers R4's open question with case (a): the tool call shares
//! its chunk with `"content": ""`, and the last chunk carries the usage and the
//! real `tool_calls` finish reason next to another `"content": ""`. The old
//! single-event parser read each of those chunks as an empty text delta, so the
//! turn reached the tool loop as reasoning with a blank answer and a made-up
//! `stop`.

#![allow(missing_docs)]

use std::time::Instant;

use roko_agent::Usage;
use roko_agent::streaming::parse_sse_line;
use roko_agent::tool_loop::{StreamEvent, StreamEventKind, collect_stream_to_response};
use roko_agent::translate::{OpenAiTranslator, Translator};
use serde_json::json;

static GLM_47_TURN: &str = include_str!("fixtures/sse/glm-4.7-agent-turn.sse");

/// The reasoning GLM-4.7 streamed, in 22 chunks, before its tool call.
const REASONING: &str =
    "The task is simple: read notes.txt and reply with the code word only. Let me read the file.";

/// A stable tag for each event kind, for the sequence assertion.
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

#[tokio::test]
async fn glm_47_recorded_stream_replays_without_loss() {
    // The chunks the single-event parser lost: the tool call, and the final
    // usage and finish reason, each share a chunk with `"content": ""`.
    let shared = GLM_47_TURN
        .lines()
        .filter(|line| line.contains(r#""content":"""#))
        .collect::<Vec<_>>();
    assert_eq!(shared.len(), 3, "{shared:#?}");
    assert!(shared[0].contains(r#""tool_calls":["#));
    assert!(shared[2].contains(r#""finish_reason":"tool_calls""#));
    assert!(shared[2].contains(r#""usage":{"#));

    let events = GLM_47_TURN
        .lines()
        .flat_map(parse_sse_line)
        .collect::<Vec<StreamEvent>>();

    // The reasoning, one tool call, the usage, the real finish reason, and
    // the `[DONE]` terminator, which names none. No text: every `content`
    // in the stream is empty.
    let mut expected = vec!["reasoning_delta"; 22];
    expected.extend(["tool_call_start", "usage", "done", "done"]);
    let tags = events
        .iter()
        .map(|event| event_tag(&event.kind))
        .collect::<Vec<_>>();
    assert_eq!(tags, expected);

    let reasoning = events
        .iter()
        .filter_map(|event| match &event.kind {
            StreamEventKind::ReasoningDelta(text) => Some(text.as_str()),
            _ => None,
        })
        .collect::<String>();
    assert_eq!(reasoning, REASONING);

    // The call arrives whole in one chunk: its id, name and arguments.
    let StreamEventKind::ToolCallStart { id, name } = &events[22].kind else {
        panic!("expected the tool call start, got {:?}", events[22].kind);
    };
    assert_eq!(
        id,
        "__idx_0\0call_0a817e07d49c4a15adef4ae4\x01{\"path\": \"notes.txt\"}"
    );
    assert_eq!(name, "read_file");

    let usage = Usage {
        input_tokens: 2279,
        output_tokens: 35,
        reasoning_tokens: 23,
        ..Usage::default()
    };
    let StreamEventKind::Usage(streamed) = &events[23].kind else {
        panic!("expected the usage, got {:?}", events[23].kind);
    };
    assert_eq!(*streamed, usage);

    let finish_reasons = events
        .iter()
        .filter_map(|event| match &event.kind {
            StreamEventKind::Done { finish_reason } => Some(finish_reason.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(finish_reasons, ["ToolCalls", "unknown"]);

    // Every chunk names the model; the `[DONE]` line names none.
    let (done_marker, chunks) = events.split_last().expect("a [DONE] event");
    for event in chunks {
        assert_eq!(event.model.as_deref(), Some("glm-4.7"), "{event:?}");
    }
    assert_eq!(done_marker.model, None);

    // The tool loop's collector keeps all of it, and `[DONE]`'s `unknown`
    // does not replace the finish reason the stream named.
    let stream = futures::stream::iter(events.into_iter().map(Ok));
    let response = collect_stream_to_response(Box::pin(stream), Instant::now())
        .await
        .expect("the recorded turn collects");
    let calls = OpenAiTranslator
        .parse_calls(&response)
        .expect("the collected tool calls parse");
    assert_eq!(calls.len(), 1, "{calls:?}");
    assert_eq!(calls[0].id, "call_0a817e07d49c4a15adef4ae4");
    assert_eq!(calls[0].name, "read_file");
    assert_eq!(calls[0].arguments, json!({ "path": "notes.txt" }));
    assert_eq!(response.extract_usage(), usage);
    assert_eq!(
        response.extract_finish_reason_raw().as_deref(),
        Some("ToolCalls")
    );
    assert_eq!(response.extract_text(), "");
    assert_eq!(response.extract_reasoning().as_deref(), Some(REASONING));
    assert_eq!(response.extract_model().as_deref(), Some("glm-4.7"));
}
