//! OpenAI's Responses API (`/responses`) for roko's OpenAI-compatible tool
//! loop (gap-1ba0ac).
//!
//! On chat completions OpenAI refuses a `reasoning_effort` beside function
//! tools for the GPT-5.4 family (bug-29014c), so no agentic call to it could
//! reason. The Responses API takes both. [`OpenAiCompatLlmBackend`] speaks it
//! to a model whose wire is [`OpenAiApi::Responses`], and this module
//! translates: the tool loop's chat-completions history becomes a Responses
//! request, and a Responses answer, whole or streamed, becomes the
//! chat-completions JSON and stream events the loop and its translator read.
//! Nothing above the backend changes.
//!
//! The calls are stateless (`store: false`): roko's own history stays the
//! conversation, and nothing is kept at OpenAI. A reasoning model's reasoning
//! comes back encrypted (`include: ["reasoning.encrypted_content"]`), and a
//! [`ReasoningLedger`] keeps each turn's output items, so the next request
//! replays a turn's reasoning before its function calls.
//!
//! [`OpenAiCompatLlmBackend`]: crate::openai_compat_backend::OpenAiCompatLlmBackend
//! [`OpenAiApi::Responses`]: roko_core::config::schema::OpenAiApi::Responses

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use serde_json::{Map, Value, json};

use crate::provider::openai_compat::REASONING_EFFORT;
use crate::streaming::SseError;
use crate::tool_loop::{LlmError, StreamEvent, StreamEventKind};

/// The Responses endpoint of the OpenAI provider at `base_url`.
#[must_use]
pub(crate) fn responses_endpoint(base_url: &str) -> String {
    format!("{}/responses", base_url.trim_end_matches('/'))
}

/// What a Responses request is built from: the tool loop's turn, as the
/// backend would send it to chat completions.
pub(crate) struct ResponsesRequest<'a> {
    pub(crate) model: &'a str,
    /// The chat-completions history: system, user, assistant and tool
    /// messages.
    pub(crate) messages: &'a [Value],
    /// The chat-completions `tools` array; `None` sends no tools.
    pub(crate) tools: Option<&'a Value>,
    pub(crate) max_output_tokens: Option<u32>,
    /// The model's extra body params (`build_extra_body_params`).
    pub(crate) extra: &'a Map<String, Value>,
    /// `Some(false)` asks for one tool call at a time.
    pub(crate) parallel_tool_calls: Option<bool>,
    pub(crate) stream: bool,
}

/// The Responses body for `request`, with the reasoning `ledger` kept of the
/// run's earlier turns replayed before their function calls.
#[must_use]
pub(crate) fn request_body(request: &ResponsesRequest<'_>, ledger: &ReasoningLedger) -> Value {
    let mut instructions = Vec::new();
    let mut input = Vec::new();
    for message in request.messages {
        match message.get("role").and_then(Value::as_str) {
            Some("system" | "developer") => {
                let text = content_text(message.get("content"));
                if !text.is_empty() {
                    instructions.push(text);
                }
            }
            Some("user") => input.push(json!({
                "role": "user",
                "content": user_content(message.get("content")),
            })),
            Some("assistant") => match ledger.replay(message) {
                Some(items) => input.extend(items),
                None => input.extend(assistant_items(message)),
            },
            Some("tool") => input.push(json!({
                "type": "function_call_output",
                "call_id": message.get("tool_call_id").cloned().unwrap_or(Value::Null),
                "output": content_text(message.get("content")),
            })),
            _ => {}
        }
    }

    let mut body = json!({
        "model": request.model,
        "input": input,
        // roko's history is the conversation: nothing is kept at OpenAI, and
        // a reasoning model's reasoning comes back to replay.
        "store": false,
        "include": ["reasoning.encrypted_content"],
    });
    let Some(object) = body.as_object_mut() else {
        return body;
    };
    if !instructions.is_empty() {
        object.insert(
            "instructions".to_string(),
            Value::String(instructions.join("\n\n")),
        );
    }
    let tools = request.tools.map(flat_tools).unwrap_or_default();
    if !tools.is_empty() {
        object.insert("tools".to_string(), Value::Array(tools));
        object.insert("tool_choice".to_string(), Value::from("auto"));
        if let Some(parallel) = request.parallel_tool_calls {
            object.insert("parallel_tool_calls".to_string(), Value::Bool(parallel));
        }
    }
    if let Some(max_output_tokens) = request.max_output_tokens {
        object.insert(
            "max_output_tokens".to_string(),
            Value::from(max_output_tokens),
        );
    }
    for (key, value) in request.extra {
        match key.as_str() {
            REASONING_EFFORT => {
                object.insert("reasoning".to_string(), json!({ "effort": value }));
            }
            "temperature" | "top_p" => {
                object.insert(key.clone(), value.clone());
            }
            // A seed, provider routing and other providers' thinking switches
            // have no Responses equivalent, and an unknown key is refused.
            _ => {}
        }
    }
    if request.stream {
        object.insert("stream".to_string(), Value::Bool(true));
    }
    body
}

/// The text of a chat-completions `content`: a string, or the text of its
/// parts.
fn content_text(content: Option<&Value>) -> String {
    match content {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Array(parts)) => parts
            .iter()
            .filter_map(|part| part.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

/// A user message's content as Responses input parts: its text, and its
/// images as `input_image`.
fn user_content(content: Option<&Value>) -> Value {
    let parts = match content {
        Some(Value::String(text)) => vec![json!({ "type": "input_text", "text": text })],
        Some(Value::Array(parts)) => parts
            .iter()
            .filter_map(|part| match part.get("type").and_then(Value::as_str) {
                Some("text") => Some(json!({
                    "type": "input_text",
                    "text": part.get("text").cloned().unwrap_or_default(),
                })),
                Some("image_url") => {
                    let url = part
                        .pointer("/image_url/url")
                        .or_else(|| part.get("image_url"))
                        .and_then(Value::as_str)?;
                    Some(json!({ "type": "input_image", "image_url": url }))
                }
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    };
    Value::Array(parts)
}

/// An assistant message without a ledger record, as Responses items: its
/// text, then its function calls, with no reasoning to replay.
fn assistant_items(message: &Value) -> Vec<Value> {
    let mut items = Vec::new();
    let text = content_text(message.get("content"));
    if !text.is_empty() {
        items.push(json!({
            "type": "message",
            "role": "assistant",
            "content": [{ "type": "output_text", "text": text }],
        }));
    }
    for call in tool_calls(message) {
        let arguments = match call.pointer("/function/arguments") {
            Some(Value::String(arguments)) => arguments.clone(),
            Some(other) => other.to_string(),
            None => "{}".to_string(),
        };
        items.push(json!({
            "type": "function_call",
            "call_id": call.get("id").cloned().unwrap_or(Value::Null),
            "name": call.pointer("/function/name").cloned().unwrap_or(Value::Null),
            "arguments": arguments,
        }));
    }
    items
}

fn tool_calls(message: &Value) -> &[Value] {
    message
        .get("tool_calls")
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

/// Chat-completions tools as Responses function tools: flat, and not strict,
/// which Responses makes the default and roko's schemas with optional
/// properties do not meet.
fn flat_tools(tools: &Value) -> Vec<Value> {
    tools
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .filter_map(|tool| {
            let function = tool.get("function")?;
            let mut flat = json!({
                "type": "function",
                "name": function.get("name")?,
                "parameters": function
                    .get("parameters")
                    .cloned()
                    .unwrap_or_else(|| json!({ "type": "object", "properties": {} })),
                "strict": function.get("strict").and_then(Value::as_bool).unwrap_or(false),
            });
            if let Some(description) = function.get("description") {
                flat["description"] = description.clone();
            }
            Some(flat)
        })
        .collect()
}

/// A Responses `usage` block as the chat-completions one, which
/// `translate::openai::parse_usage` reads: input with its cached part, and
/// output with its reasoning part.
#[must_use]
pub(crate) fn chat_usage(usage: &Value) -> Option<Value> {
    usage.as_object()?;
    let count = |pointer: &str| usage.pointer(pointer).and_then(Value::as_u64).unwrap_or(0);
    let input = count("/input_tokens");
    let output = count("/output_tokens");
    Some(json!({
        "prompt_tokens": input,
        "completion_tokens": output,
        "total_tokens": usage.get("total_tokens").and_then(Value::as_u64).unwrap_or(input + output),
        "prompt_tokens_details": { "cached_tokens": count("/input_tokens_details/cached_tokens") },
        "completion_tokens_details": {
            "reasoning_tokens": count("/output_tokens_details/reasoning_tokens"),
        },
    }))
}

/// The finish reason, in chat-completions terms, of a response whose
/// `output` and `status` these are.
fn finish_reason(response: &Value, output: &[Value]) -> &'static str {
    if response.get("status").and_then(Value::as_str) == Some("incomplete") {
        return match response
            .pointer("/incomplete_details/reason")
            .and_then(Value::as_str)
        {
            Some("content_filter") => "content_filter",
            _ => "length",
        };
    }
    if output
        .iter()
        .any(|item| item.get("type").and_then(Value::as_str) == Some("function_call"))
    {
        "tool_calls"
    } else {
        "stop"
    }
}

/// The text of a response's output: its messages' text, and a refusal's
/// words.
fn output_text(output: &[Value]) -> String {
    output
        .iter()
        .filter(|item| item.get("type").and_then(Value::as_str) == Some("message"))
        .flat_map(|item| {
            item.get("content")
                .and_then(Value::as_array)
                .map(Vec::as_slice)
                .unwrap_or_default()
        })
        .filter_map(|part| match part.get("type").and_then(Value::as_str) {
            Some("output_text") => part.get("text").and_then(Value::as_str),
            Some("refusal") => part.get("refusal").and_then(Value::as_str),
            _ => None,
        })
        .collect()
}

/// A whole Responses answer as the chat-completions JSON the tool loop and
/// its translator read. A failed response is an error.
pub(crate) fn chat_json(response: &Value) -> Result<Value, LlmError> {
    if response.get("status").and_then(Value::as_str) == Some("failed") {
        let message = response
            .pointer("/error/message")
            .and_then(Value::as_str)
            .unwrap_or("the response failed");
        return Err(LlmError::Backend(format!("responses: {message}")));
    }
    let output = response
        .get("output")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    let text = output_text(output);
    let calls: Vec<Value> = output
        .iter()
        .filter(|item| item.get("type").and_then(Value::as_str) == Some("function_call"))
        .map(|item| {
            json!({
                "id": item.get("call_id").cloned().unwrap_or(Value::Null),
                "type": "function",
                "function": {
                    "name": item.get("name").cloned().unwrap_or(Value::Null),
                    "arguments": item.get("arguments").cloned().unwrap_or_else(|| json!("{}")),
                },
            })
        })
        .collect();
    let mut message = json!({ "role": "assistant", "content": text });
    if !calls.is_empty() {
        message["tool_calls"] = Value::Array(calls);
    }
    let mut chat = json!({
        "object": "chat.completion",
        "choices": [{
            "index": 0,
            "message": message,
            "finish_reason": finish_reason(response, output),
        }],
    });
    for key in ["id", "model"] {
        if let Some(value) = response.get(key) {
            chat[key] = value.clone();
        }
    }
    if let Some(usage) = response.get("usage").and_then(chat_usage) {
        chat["usage"] = usage;
    }
    Ok(chat)
}

/// The most turns a [`ReasoningLedger`] keeps: far more than a try takes.
const LEDGER_TURNS: usize = 256;

/// One turn's output items, as the model produced them.
#[derive(Debug, Clone)]
struct LedgerTurn {
    /// Its function calls' ids, in order.
    call_ids: Vec<String>,
    /// Its text.
    text: String,
    items: Vec<Value>,
}

/// The output items of the turns a run's Responses calls produced, so each
/// next request replays a turn's encrypted reasoning before its function
/// calls (gap-1ba0ac). A turn without reasoning to replay is not kept: it is
/// rebuilt from the history as it stands.
#[derive(Debug, Default)]
pub(crate) struct ReasoningLedger {
    turns: parking_lot::Mutex<VecDeque<LedgerTurn>>,
}

impl ReasoningLedger {
    /// Keep a response's `output` items, when they hold reasoning to replay.
    pub(crate) fn record(&self, output: &[Value]) {
        let reasoned = output.iter().any(|item| {
            item.get("type").and_then(Value::as_str) == Some("reasoning")
                && item.get("encrypted_content").is_some_and(Value::is_string)
        });
        if !reasoned {
            return;
        }
        let call_ids = output
            .iter()
            .filter(|item| item.get("type").and_then(Value::as_str) == Some("function_call"))
            .filter_map(|item| item.get("call_id").and_then(Value::as_str))
            .map(str::to_string)
            .collect();
        let mut turns = self.turns.lock();
        turns.push_back(LedgerTurn {
            call_ids,
            text: output_text(output),
            items: output.to_vec(),
        });
        while turns.len() > LEDGER_TURNS {
            turns.pop_front();
        }
    }

    /// The items to send for the assistant turn `message` of the history:
    /// those the model produced for it, its reasoning first, when the ledger
    /// kept them. A turn with function calls is found by their ids, a turn
    /// of text alone by its text. `None` rebuilds it from the message.
    pub(crate) fn replay(&self, message: &Value) -> Option<Vec<Value>> {
        let call_ids: Vec<&str> = tool_calls(message)
            .iter()
            .filter_map(|call| call.get("id").and_then(Value::as_str))
            .collect();
        let text = content_text(message.get("content"));
        let turns = self.turns.lock();
        turns
            .iter()
            .rev()
            .find(|turn| {
                if call_ids.is_empty() {
                    turn.call_ids.is_empty() && turn.text == text
                } else {
                    turn.call_ids == call_ids
                }
            })
            .map(|turn| turn.items.clone())
    }
}

/// What one Responses turn's SSE lines become: text, tool calls and usage as
/// the tool loop's stream events, a failure as an [`SseError`], and an empty
/// reasoning delta while the model reasons or OpenAI sends a keepalive, so
/// the stall watchdog sees the call alive (gap-1ba0ac, gap-a94400). The
/// completed response's items go to the ledger before its `Done` is sent,
/// so the next request can replay them.
pub(crate) struct ResponsesTurn {
    ledger: Arc<ReasoningLedger>,
    /// Each function call's id and name, by its output item's id.
    calls: HashMap<String, (String, String)>,
    /// A reasoning item has started and not finished.
    reasoning_open: bool,
    /// A `Done` was sent.
    done_sent: bool,
    /// The model the response named.
    model: Option<String>,
}

impl ResponsesTurn {
    pub(crate) fn new(ledger: Arc<ReasoningLedger>) -> Self {
        Self {
            ledger,
            calls: HashMap::new(),
            reasoning_open: false,
            done_sent: false,
            model: None,
        }
    }

    /// Whether the model is reasoning: a reasoning item started and has
    /// not finished. It sends nothing until it has.
    pub(crate) const fn reasoning_open(&self) -> bool {
        self.reasoning_open
    }

    fn event(&self, kind: StreamEventKind) -> StreamEvent {
        StreamEvent::now(kind).with_model(self.model.clone())
    }

    /// The items to send for one SSE line.
    pub(crate) fn line(&mut self, line: &str) -> Vec<Result<StreamEvent, SseError>> {
        let Some(data) = roko_core::sse::extract_sse_data(line) else {
            return Vec::new();
        };
        let Ok(json) = serde_json::from_str::<Value>(data) else {
            return Vec::new();
        };
        if let Some(model) = json.pointer("/response/model").and_then(Value::as_str) {
            self.model = Some(model.to_string());
        }
        let item = || json.get("item").cloned().unwrap_or(Value::Null);
        let text = |key: &str| {
            json.get(key)
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string()
        };
        match json.get("type").and_then(Value::as_str).unwrap_or_default() {
            // OpenAI's keepalives, sent while the model works and streams
            // nothing else, say the call is alive too (gap-a94400).
            "response.in_progress" | "keepalive" => {
                vec![Ok(
                    self.event(StreamEventKind::ReasoningDelta(String::new()))
                )]
            }
            "response.output_item.added" => {
                let item = item();
                match item.get("type").and_then(Value::as_str) {
                    Some("reasoning") => {
                        self.reasoning_open = true;
                        vec![Ok(
                            self.event(StreamEventKind::ReasoningDelta(String::new()))
                        )]
                    }
                    Some("function_call") => {
                        let id = item_str(&item, "call_id");
                        let name = item_str(&item, "name");
                        self.calls
                            .insert(item_str(&item, "id"), (id.clone(), name.clone()));
                        vec![Ok(self.event(StreamEventKind::ToolCallStart { id, name }))]
                    }
                    _ => Vec::new(),
                }
            }
            "response.output_item.done" => {
                let item = item();
                match item.get("type").and_then(Value::as_str) {
                    Some("reasoning") => {
                        self.reasoning_open = false;
                        Vec::new()
                    }
                    Some("function_call") => {
                        let arguments = item_str(&item, "arguments");
                        let args =
                            serde_json::from_str(&arguments).unwrap_or(Value::String(arguments));
                        vec![Ok(self.event(StreamEventKind::ToolCallEnd {
                            id: item_str(&item, "call_id"),
                            name: item_str(&item, "name"),
                            args,
                        }))]
                    }
                    _ => Vec::new(),
                }
            }
            "response.function_call_arguments.delta" => {
                let Some((id, _)) = self.calls.get(&text("item_id")).cloned() else {
                    return Vec::new();
                };
                vec![Ok(self.event(StreamEventKind::ToolCallDelta {
                    id,
                    json_fragment: text("delta"),
                }))]
            }
            "response.output_text.delta" | "response.refusal.delta" => {
                vec![Ok(self.event(StreamEventKind::TextDelta(text("delta"))))]
            }
            "response.reasoning_summary_text.delta" | "response.reasoning_text.delta" => {
                vec![Ok(
                    self.event(StreamEventKind::ReasoningDelta(text("delta")))
                )]
            }
            "response.completed" | "response.incomplete" => {
                let response = json.get("response").cloned().unwrap_or(Value::Null);
                let output = response
                    .get("output")
                    .and_then(Value::as_array)
                    .map(Vec::as_slice)
                    .unwrap_or_default();
                self.ledger.record(output);
                self.reasoning_open = false;
                self.done_sent = true;
                let mut events = Vec::new();
                if let Some(usage) = response.get("usage").and_then(chat_usage) {
                    let usage = crate::translate::openai::parse_usage(&json!({ "usage": usage }));
                    events.push(Ok(self.event(StreamEventKind::Usage(usage))));
                }
                events.push(Ok(self.event(StreamEventKind::Done {
                    finish_reason: finish_reason(&response, output).to_string(),
                })));
                events
            }
            "response.failed" => vec![Err(sse_error(
                json.pointer("/response/error").unwrap_or(&Value::Null),
                "the response failed",
            ))],
            "error" => vec![Err(sse_error(&json, "the stream sent an error"))],
            _ => Vec::new(),
        }
    }

    /// The `Done` to send when the stream ended, unless one was sent: a
    /// stream that ended without completing names no finish reason.
    pub(crate) fn end(&mut self) -> Option<StreamEvent> {
        if std::mem::replace(&mut self.done_sent, true) {
            return None;
        }
        Some(self.event(StreamEventKind::Done {
            finish_reason: crate::streaming::UNKNOWN_FINISH_REASON.to_string(),
        }))
    }
}

fn item_str(item: &Value, key: &str) -> String {
    item.get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

/// A Responses error object (`{code, message}`) as an [`SseError`].
fn sse_error(error: &Value, fallback: &str) -> SseError {
    let code = match error.get("code") {
        Some(Value::String(code)) => Some(code.clone()),
        Some(Value::Number(code)) => Some(code.to_string()),
        _ => None,
    };
    SseError {
        code,
        error_type: error
            .get("type")
            .and_then(Value::as_str)
            .filter(|kind| *kind != "error")
            .map(str::to_string),
        message: error
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or(fallback)
            .to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A turn's output with encrypted reasoning, then a call of `echo`.
    fn reasoned_echo_output() -> Vec<Value> {
        vec![
            json!({
                "type": "reasoning", "id": "rs_1", "summary": [],
                "encrypted_content": "gAAAA-enc-1",
            }),
            json!({
                "type": "function_call", "id": "fc_1", "call_id": "call_1",
                "name": "echo", "arguments": "{\"text\":\"hi\"}", "status": "completed",
            }),
        ]
    }

    fn chat_tools() -> Value {
        json!([{
            "type": "function",
            "function": {
                "name": "read_file",
                "description": "Read a file",
                "parameters": {
                    "type": "object",
                    "properties": { "path": {"type": "string"}, "limit": {"type": "integer"} },
                    "required": ["path"],
                },
            },
        }])
    }

    /// gap-1ba0ac: the tool loop's chat history becomes a stateless Responses
    /// request: the system prompt as instructions, each message as its item,
    /// flat tools that are not strict, the effort as `reasoning.effort`, and
    /// no key Responses would refuse.
    #[test]
    fn a_chat_turn_becomes_a_stateless_responses_request() {
        let messages = [
            json!({ "role": "system", "content": "You are roko's implementer." }),
            json!({ "role": "user", "content": [
                { "type": "text", "text": "Look at this." },
                { "type": "image_url", "image_url": { "url": "data:image/png;base64,AAAA" } },
            ]}),
            json!({ "role": "assistant", "content": "Reading it.", "tool_calls": [{
                "id": "call_9", "type": "function",
                "function": { "name": "read_file", "arguments": "{\"path\":\"a.rs\"}" },
            }]}),
            json!({ "role": "tool", "tool_call_id": "call_9", "content": "fn main() {}" }),
        ];
        let tools = chat_tools();
        let extra = Map::from_iter([
            (REASONING_EFFORT.to_string(), json!("medium")),
            ("seed".to_string(), json!(7)),
            ("temperature".to_string(), json!(1)),
        ]);
        let body = request_body(
            &ResponsesRequest {
                model: "gpt-5.4-mini",
                messages: &messages,
                tools: Some(&tools),
                max_output_tokens: Some(32_768),
                extra: &extra,
                parallel_tool_calls: None,
                stream: true,
            },
            &ReasoningLedger::default(),
        );

        assert_eq!(body["model"], "gpt-5.4-mini");
        assert_eq!(body["instructions"], "You are roko's implementer.");
        assert_eq!(body["store"], false);
        assert_eq!(body["include"], json!(["reasoning.encrypted_content"]));
        assert_eq!(body["reasoning"], json!({ "effort": "medium" }));
        assert_eq!(body["max_output_tokens"], 32_768);
        assert_eq!(body["stream"], true);
        assert_eq!(body["temperature"], 1);
        assert_eq!(body["tool_choice"], "auto");
        for refused in [
            "seed",
            "reasoning_effort",
            "messages",
            "max_completion_tokens",
        ] {
            assert!(body.get(refused).is_none(), "{refused}: {body}");
        }
        assert_eq!(
            body["tools"],
            json!([{
                "type": "function",
                "name": "read_file",
                "description": "Read a file",
                "parameters": tools[0]["function"]["parameters"],
                "strict": false,
            }])
        );
        assert_eq!(
            body["input"],
            json!([
                { "role": "user", "content": [
                    { "type": "input_text", "text": "Look at this." },
                    { "type": "input_image", "image_url": "data:image/png;base64,AAAA" },
                ]},
                { "type": "message", "role": "assistant",
                  "content": [{ "type": "output_text", "text": "Reading it." }] },
                { "type": "function_call", "call_id": "call_9", "name": "read_file",
                  "arguments": "{\"path\":\"a.rs\"}" },
                { "type": "function_call_output", "call_id": "call_9", "output": "fn main() {}" },
            ])
        );
    }

    /// gap-1ba0ac: a turn the ledger kept is replayed as the model produced
    /// it, its encrypted reasoning before its function call; a turn it did
    /// not keep is rebuilt from the history, without reasoning.
    #[test]
    fn a_kept_turn_replays_its_reasoning_before_its_function_call() {
        let ledger = ReasoningLedger::default();
        ledger.record(&reasoned_echo_output());
        let messages = [
            json!({ "role": "user", "content": "Call echo." }),
            json!({ "role": "assistant", "content": "", "tool_calls": [{
                "id": "call_1", "type": "function",
                "function": { "name": "echo", "arguments": "{\"text\":\"hi\"}" },
            }]}),
            json!({ "role": "tool", "tool_call_id": "call_1", "content": "hi" }),
            json!({ "role": "assistant", "content": "", "tool_calls": [{
                "id": "call_2", "type": "function",
                "function": { "name": "echo", "arguments": "{}" },
            }]}),
            json!({ "role": "tool", "tool_call_id": "call_2", "content": "" }),
        ];
        let body = request_body(
            &ResponsesRequest {
                model: "gpt-5.4",
                messages: &messages,
                tools: None,
                max_output_tokens: None,
                extra: &Map::new(),
                parallel_tool_calls: None,
                stream: false,
            },
            &ledger,
        );
        let input = body["input"].as_array().expect("input items");
        let kinds: Vec<&str> = input
            .iter()
            .map(|item| item["type"].as_str().unwrap_or("message"))
            .collect();
        assert_eq!(
            kinds,
            [
                "message",
                "reasoning",
                "function_call",
                "function_call_output",
                "function_call",
                "function_call_output"
            ]
        );
        assert_eq!(input[1]["encrypted_content"], "gAAAA-enc-1");
        assert_eq!(input[2]["id"], "fc_1");
        assert_eq!(input[4]["call_id"], "call_2");
        assert!(input[4].get("id").is_none(), "{}", input[4]);
        assert!(body.get("tools").is_none());
        assert!(body.get("reasoning").is_none());
    }

    /// gap-1ba0ac: a whole answer reads as chat completions: its text, its
    /// calls by `call_id`, its finish reason, and usage with the cached input
    /// and the reasoning inside the output.
    #[test]
    fn a_responses_answer_reads_as_chat_completions() {
        let response = json!({
            "id": "resp_1", "object": "response", "status": "completed",
            "model": "gpt-5.4-mini-2026-03-17",
            "output": reasoned_echo_output(),
            "usage": {
                "input_tokens": 2554, "input_tokens_details": { "cached_tokens": 1024 },
                "output_tokens": 300, "output_tokens_details": { "reasoning_tokens": 256 },
                "total_tokens": 2854,
            },
        });
        let chat = chat_json(&response).expect("a completed response");
        assert_eq!(chat["model"], "gpt-5.4-mini-2026-03-17");
        assert_eq!(chat["choices"][0]["finish_reason"], "tool_calls");
        assert_eq!(
            chat["choices"][0]["message"]["tool_calls"],
            json!([{ "id": "call_1", "type": "function",
                     "function": { "name": "echo", "arguments": "{\"text\":\"hi\"}" } }])
        );
        let usage = crate::translate::openai::parse_usage(&chat);
        assert_eq!(usage.input_tokens, 2554 - 1024);
        assert_eq!(usage.cache_read_tokens, 1024);
        assert_eq!(usage.output_tokens, 300, "reasoning is inside the output");
        assert_eq!(usage.reasoning_tokens, 256);

        let answered = json!({
            "status": "completed",
            "output": [{ "type": "message", "role": "assistant", "content": [
                { "type": "output_text", "text": "OK" },
            ]}],
        });
        let chat = chat_json(&answered).expect("an answer");
        assert_eq!(chat["choices"][0]["message"]["content"], "OK");
        assert_eq!(chat["choices"][0]["finish_reason"], "stop");
        assert!(chat.get("usage").is_none());

        let cut = json!({
            "status": "incomplete",
            "incomplete_details": { "reason": "max_output_tokens" },
            "output": [],
        });
        let chat = chat_json(&cut).expect("a cut answer");
        assert_eq!(chat["choices"][0]["finish_reason"], "length");

        let failed = json!({ "status": "failed", "error": { "message": "server overloaded" } });
        assert!(chat_json(&failed).is_err());
    }

    fn sse(event: &Value) -> String {
        format!("data: {event}")
    }

    fn kinds(events: &[Result<StreamEvent, SseError>]) -> Vec<StreamEventKind> {
        events
            .iter()
            .map(|event| event.as_ref().expect("an event").kind.clone())
            .collect()
    }

    /// gap-1ba0ac: a streamed turn's events become the tool loop's, the
    /// reasoning model's silence is marked by empty reasoning deltas, and
    /// the completed response's items reach the ledger before its `Done`.
    #[test]
    fn a_streamed_turn_maps_onto_the_tool_loops_events() {
        let ledger = Arc::new(ReasoningLedger::default());
        let mut turn = ResponsesTurn::new(Arc::clone(&ledger));
        let output = reasoned_echo_output();
        let mut events = Vec::new();
        for line in [
            "event: response.created".to_string(),
            sse(&json!({ "type": "response.created",
                         "response": { "model": "gpt-5.4-mini-2026-03-17" } })),
            sse(&json!({ "type": "response.in_progress", "response": {} })),
            sse(&json!({ "type": "response.output_item.added",
                         "item": { "type": "reasoning", "id": "rs_1" } })),
            sse(&json!({ "type": "response.output_item.done", "item": output[0] })),
            sse(&json!({ "type": "response.output_item.added", "item": {
                "type": "function_call", "id": "fc_1", "call_id": "call_1",
                "name": "echo", "arguments": "" } })),
            sse(&json!({ "type": "response.function_call_arguments.delta",
                         "item_id": "fc_1", "delta": "{\"text\":" })),
            sse(&json!({ "type": "response.function_call_arguments.delta",
                         "item_id": "fc_1", "delta": "\"hi\"}" })),
            sse(&json!({ "type": "response.output_item.done", "item": output[1] })),
            sse(&json!({ "type": "response.output_text.delta", "delta": "done" })),
        ] {
            if line.contains("output_item.added") && line.contains("reasoning") {
                assert!(!turn.reasoning_open());
            }
            events.extend(turn.line(&line));
            if line.contains("output_item.added") && line.contains("reasoning") {
                assert!(turn.reasoning_open(), "reasoning started");
            }
        }
        assert!(!turn.reasoning_open(), "reasoning ended");
        assert!(
            ledger.replay(&json!({})).is_none(),
            "nothing kept before completion"
        );
        events.extend(turn.line(&sse(&json!({
            "type": "response.completed",
            "response": {
                "model": "gpt-5.4-mini-2026-03-17", "status": "completed", "output": output,
                "usage": { "input_tokens": 200, "output_tokens": 40,
                           "output_tokens_details": { "reasoning_tokens": 32 } },
            },
        }))));

        let kinds = kinds(&events);
        assert!(
            matches!(&kinds[0], StreamEventKind::ReasoningDelta(text) if text.is_empty()),
            "{kinds:?}"
        );
        assert!(
            matches!(&kinds[1], StreamEventKind::ReasoningDelta(text) if text.is_empty()),
            "{kinds:?}"
        );
        assert!(
            matches!(&kinds[2], StreamEventKind::ToolCallStart { id, name }
                if id == "call_1" && name == "echo"),
            "{kinds:?}"
        );
        assert!(
            matches!(&kinds[3], StreamEventKind::ToolCallDelta { id, json_fragment }
                if id == "call_1" && json_fragment == "{\"text\":"),
            "{kinds:?}"
        );
        assert!(
            matches!(&kinds[5], StreamEventKind::ToolCallEnd { id, name, args }
                if id == "call_1" && name == "echo" && *args == json!({ "text": "hi" })),
            "{kinds:?}"
        );
        assert!(
            matches!(&kinds[6], StreamEventKind::TextDelta(text) if text == "done"),
            "{kinds:?}"
        );
        let StreamEventKind::Usage(usage) = &kinds[7] else {
            panic!("usage, got {:?}", kinds[7]);
        };
        assert_eq!(
            (
                usage.input_tokens,
                usage.output_tokens,
                usage.reasoning_tokens
            ),
            (200, 40, 32)
        );
        assert!(
            matches!(&kinds[8], StreamEventKind::Done { finish_reason } if finish_reason == "tool_calls"),
            "{kinds:?}"
        );
        let last = events.last().expect("an event").as_ref().expect("done");
        assert_eq!(last.model.as_deref(), Some("gpt-5.4-mini-2026-03-17"));
        assert!(turn.end().is_none(), "one Done a turn");
        let kept = ledger
            .replay(&json!({ "role": "assistant", "tool_calls": [{ "id": "call_1" }] }))
            .expect("the turn was kept");
        assert_eq!(kept[0]["encrypted_content"], "gAAAA-enc-1");

        // A failure, in the stream or of the response, ends the turn.
        let mut turn = ResponsesTurn::new(Arc::new(ReasoningLedger::default()));
        let failed = turn.line(&sse(&json!({ "type": "response.failed", "response": {
            "error": { "code": "server_error", "message": "overloaded" } } })));
        let Err(error) = &failed[0] else {
            panic!("a failure");
        };
        assert_eq!(error.message, "overloaded");
        let error = turn.line(&sse(
            &json!({ "type": "error", "code": "rate_limit_exceeded",
                                            "message": "slow down" }),
        ));
        assert!(error[0].is_err());
        let mut silent = ResponsesTurn::new(Arc::new(ReasoningLedger::default()));
        let end = silent.end().map(|event| event.kind);
        assert!(
            matches!(&end, Some(StreamEventKind::Done { finish_reason })
                if finish_reason == crate::streaming::UNKNOWN_FINISH_REASON),
            "{end:?}"
        );
    }
}
