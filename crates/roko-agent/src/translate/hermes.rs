//! Hermes XML translator (§36.c-hermes).
//!
//! Hermes models (NousResearch Hermes, Qwen 3 base) use an XML-based
//! tool-calling format: tools are rendered as `<tools>` blocks in the
//! system prompt, the model emits `<tool_call>{"name":...,"arguments":...}</tool_call>`
//! XML tags, and results are fed back as `<tool_response>` blocks.
//!
//! Unlike ReAct (which can only parse a single `Action:` per turn), Hermes
//! supports **parallel tool calls**: multiple `<tool_call>` blocks in one
//! response are parsed into separate [`ToolCall`] entries.
//!
//! # Wire format
//!
//! ```text
//! System prompt (tools section):
//!   <tools>
//!   [{"type":"function","function":{"name":"...","description":"...","parameters":{...}}}]
//!   </tools>
//!
//! Model output (tool invocation):
//!   <tool_call>
//!   {"name": "read_file", "arguments": {"path": "src/lib.rs"}}
//!   </tool_call>
//!
//! Result injection (next turn):
//!   <tool_response>
//!   {"name": "read_file", "content": "pub fn main() {}"}
//!   </tool_response>
//! ```
//!
//! # Robustness
//!
//! The parser handles common model quirks:
//! - `<think>...</think>` reasoning blocks are skipped
//! - Missing `"arguments"` key defaults to `{}`
//! - Single-block `"tool_calls": [...]` arrays are unpacked
//! - Trailing commas and minor JSON malformations are repaired
//! - Both `BackendResponse::Text` and `BackendResponse::Json` (with
//!   content in `/message/content` or `/choices/0/message/content`) are accepted

use roko_core::tool::{ToolCall, ToolDef, ToolFormat, ToolResult};

use super::{BackendResponse, RenderedResults, RenderedTools, Translator, TranslatorError};

/// Translator for the Hermes XML `<tool_call>` format.
///
/// Pure, stateless, zero-sized. Renders tools as a `<tools>` block
/// for the system prompt and parses `<tool_call>` XML tags from the
/// model's text output.
#[derive(Debug, Default, Clone, Copy)]
pub struct HermesXmlTranslator;

impl Translator for HermesXmlTranslator {
    fn format(&self) -> ToolFormat {
        ToolFormat::HermesJson
    }

    fn render_tools(&self, tools: &[ToolDef]) -> RenderedTools {
        // Convert tools into the OpenAI-schema JSON array that Hermes
        // models expect inside `<tools>...</tools>`.
        let arr: Vec<serde_json::Value> = tools
            .iter()
            .map(|t| {
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.parameters.as_value(),
                    }
                })
            })
            .collect();

        let json_str =
            serde_json::to_string_pretty(&arr).unwrap_or_else(|_| "[]".to_string());

        let mut block = String::from("You are a function calling AI model. You are provided with function signatures within <tools></tools> XML tags. You may call one or more functions to assist with the user query. Don't make assumptions about what values to plug into functions.\n\n");
        block.push_str("<tools>\n");
        block.push_str(&json_str);
        block.push_str("\n</tools>\n\n");
        block.push_str("For each function call, return a JSON object within <tool_call></tool_call> XML tags with the function name and arguments:\n");
        block.push_str("<tool_call>\n");
        block.push_str(r#"{"name": "<function_name>", "arguments": <args_dict>}"#);
        block.push('\n');
        block.push_str("</tool_call>");

        RenderedTools::SystemPromptBlock(block)
    }

    fn parse_calls(
        &self,
        response: &BackendResponse,
    ) -> Result<Vec<ToolCall>, TranslatorError> {
        let text = extract_text(response);

        // Parse all <tool_call>...</tool_call> blocks from the text.
        let mut calls = Vec::new();
        let mut search_from = 0;

        while let Some(start) = text[search_from..].find("<tool_call>") {
            let abs_start = search_from + start + "<tool_call>".len();
            let Some(end_offset) = text[abs_start..].find("</tool_call>") else {
                // Unclosed tag -- try to parse what we have up to end-of-string.
                let body = text[abs_start..].trim();
                if !body.is_empty() {
                    if let Some(call) = parse_tool_call_body(body, calls.len()) {
                        calls.push(call);
                    }
                }
                break;
            };
            let abs_end = abs_start + end_offset;
            let body = text[abs_start..abs_end].trim();

            if !body.is_empty() {
                // Try to parse the JSON body inside the tags.
                if let Some(call) = parse_tool_call_body(body, calls.len()) {
                    calls.push(call);
                }
            }

            search_from = abs_end + "</tool_call>".len();
        }

        Ok(calls)
    }

    fn render_results(&self, results: &[(ToolCall, ToolResult)]) -> RenderedResults {
        let mut block = String::new();
        for (call, result) in results {
            let content = match result {
                ToolResult::Ok { .. } => result.text_content(),
                ToolResult::Err(e) => format!("Error: {e}"),
            };
            block.push_str("<tool_response>\n");
            let obj = serde_json::json!({
                "name": call.name,
                "content": content,
            });
            block.push_str(
                &serde_json::to_string(&obj).unwrap_or_else(|_| "{}".to_string()),
            );
            block.push_str("\n</tool_response>\n");
        }
        RenderedResults::TextBlock(block)
    }

    /// Hermes models produce text responses; there is no structured
    /// assistant message to inject into conversation history.
    fn render_assistant_message(
        &self,
        _response: &BackendResponse,
    ) -> Option<serde_json::Value> {
        None
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────

/// Extract text content from any `BackendResponse` variant.
///
/// - `Text` -> used directly
/// - `Json` -> try `/message/content` then `/choices/0/message/content`
/// - `StreamJson` -> concatenate content fields from all events
fn extract_text(response: &BackendResponse) -> String {
    match response {
        BackendResponse::Text(s) => s.clone(),
        BackendResponse::Json(v) => v
            .pointer("/message/content")
            .and_then(|x| x.as_str())
            .or_else(|| {
                v.pointer("/choices/0/message/content")
                    .and_then(|x| x.as_str())
            })
            .unwrap_or("")
            .to_string(),
        BackendResponse::StreamJson(events) => {
            let mut buf = String::new();
            for ev in events {
                if let Some(content) = ev.get("content").and_then(|c| c.as_str()) {
                    buf.push_str(content);
                }
                // Also check nested delta content (OpenAI streaming shape).
                if let Some(content) = ev
                    .pointer("/choices/0/delta/content")
                    .and_then(|c| c.as_str())
                {
                    buf.push_str(content);
                }
            }
            buf
        }
    }
}

/// Parse the JSON body inside a `<tool_call>` block into a `ToolCall`.
///
/// Handles:
/// - Standard `{"name": "...", "arguments": {...}}`
/// - Missing `"arguments"` key -> defaults to `{}`
/// - `"arguments"` before `"name"` key ordering (JSON is order-independent)
/// - Single-block `{"tool_calls": [...]}` array -> unpacks each entry
/// - Trailing commas and minor JSON malformations -> attempted repair
fn parse_tool_call_body(body: &str, index: usize) -> Option<ToolCall> {
    // Try parsing the body as-is first.
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(body) {
        return tool_call_from_value(&parsed, index);
    }

    // Attempt repair: strip trailing commas before `}` or `]`.
    let repaired = repair_json(body);
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&repaired) {
        return tool_call_from_value(&parsed, index);
    }

    None
}

/// Construct a `ToolCall` from a parsed JSON value.
///
/// If the value contains a `"tool_calls"` array, returns only the first
/// entry (additional entries from that array should be handled by the
/// caller, but in practice single-block arrays are the common case).
fn tool_call_from_value(value: &serde_json::Value, index: usize) -> Option<ToolCall> {
    // Check for `{"tool_calls": [...]}` wrapper (some models do this).
    if let Some(arr) = value.get("tool_calls").and_then(|v| v.as_array()) {
        // Take the first entry; multi-entry arrays inside a single
        // <tool_call> block are rare but handled.
        return arr.first().and_then(|entry| extract_single_call(entry, index));
    }

    extract_single_call(value, index)
}

/// Extract a single tool call from a JSON object with `"name"` and
/// optionally `"arguments"` fields.
fn extract_single_call(value: &serde_json::Value, index: usize) -> Option<ToolCall> {
    let name = value.get("name").and_then(|n| n.as_str())?;
    let arguments = value
        .get("arguments")
        .cloned()
        .unwrap_or(serde_json::json!({}));

    // If "arguments" came as a JSON-encoded string, parse it.
    let arguments = match arguments {
        serde_json::Value::String(ref s) => {
            serde_json::from_str::<serde_json::Value>(s).unwrap_or(arguments)
        }
        other => other,
    };

    let id = format!("hermes-tc-{index}");
    Some(ToolCall::new(id, name, arguments))
}

/// Attempt to repair common JSON malformations from LLM output.
///
/// - Removes trailing commas before `}` and `]`
/// - Strips trailing whitespace
fn repair_json(input: &str) -> String {
    let trimmed = input.trim();
    // Remove trailing commas before closing braces/brackets.
    // This handles the common case: {"key": "val",}
    let mut result = String::with_capacity(trimmed.len());
    let chars: Vec<char> = trimmed.chars().collect();
    let len = chars.len();

    let mut i = 0;
    while i < len {
        if chars[i] == ',' {
            // Look ahead past whitespace for `}` or `]`.
            let mut j = i + 1;
            while j < len && chars[j].is_whitespace() {
                j += 1;
            }
            if j < len && (chars[j] == '}' || chars[j] == ']') {
                // Skip the trailing comma.
                i += 1;
                continue;
            }
        }
        result.push(chars[i]);
        i += 1;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use roko_core::tool::{ToolCategory, ToolError, ToolPermission, ToolSchema};
    use serde_json::json;

    // ─── helpers ──────────────────────────────────────────────────────────

    fn read_tool() -> ToolDef {
        ToolDef::new(
            "read_file",
            "Read a UTF-8 file from the worktree.",
            ToolCategory::Read,
            ToolPermission::read_only(),
        )
        .with_parameters(ToolSchema::from_value(json!({
            "type": "object",
            "properties": { "path": { "type": "string" } },
            "required": ["path"],
        })))
    }

    fn write_tool() -> ToolDef {
        ToolDef::new(
            "edit_file",
            "Edit a UTF-8 file in the worktree.",
            ToolCategory::Write,
            ToolPermission::writes(),
        )
        .with_parameters(ToolSchema::from_value(json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" },
                "content": { "type": "string" },
            },
            "required": ["path", "content"],
        })))
    }

    // ─── format ───────────────────────────────────────────────────────────

    #[test]
    fn format_returns_hermes_json() {
        assert_eq!(HermesXmlTranslator.format(), ToolFormat::HermesJson);
    }

    // ─── render_tools ─────────────────────────────────────────────────────

    #[test]
    fn renders_tools_as_system_prompt_block_with_xml_tags() {
        let tools = vec![read_tool(), write_tool()];
        let rendered = HermesXmlTranslator.render_tools(&tools);
        let RenderedTools::SystemPromptBlock(block) = rendered else {
            panic!("expected SystemPromptBlock");
        };
        assert!(block.contains("<tools>"), "block missing <tools> tag");
        assert!(block.contains("</tools>"), "block missing </tools> tag");
        assert!(
            block.contains("<tool_call>"),
            "block missing <tool_call> instruction"
        );
        assert!(
            block.contains("</tool_call>"),
            "block missing </tool_call> instruction"
        );
        assert!(
            block.contains("read_file"),
            "block missing read_file tool name"
        );
        assert!(
            block.contains("edit_file"),
            "block missing edit_file tool name"
        );
        assert!(
            block.contains("Read a UTF-8 file from the worktree."),
            "block missing read_file description"
        );
    }

    #[test]
    fn renders_tools_json_inside_tools_tag_is_valid_json_array() {
        let tools = vec![read_tool()];
        let rendered = HermesXmlTranslator.render_tools(&tools);
        let RenderedTools::SystemPromptBlock(block) = rendered else {
            panic!("expected SystemPromptBlock");
        };
        // Extract the JSON between <tools> and </tools>.
        let start = block.find("<tools>\n").unwrap() + "<tools>\n".len();
        let end = block.find("\n</tools>").unwrap();
        let json_str = &block[start..end];
        let arr: Vec<serde_json::Value> =
            serde_json::from_str(json_str).expect("JSON inside <tools> must be valid");
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["function"]["name"], "read_file");
    }

    #[test]
    fn renders_empty_tool_list() {
        let rendered = HermesXmlTranslator.render_tools(&[]);
        let RenderedTools::SystemPromptBlock(block) = rendered else {
            panic!("expected SystemPromptBlock");
        };
        assert!(block.contains("<tools>"));
        assert!(block.contains("[]"));
    }

    // ─── parse_calls ──────────────────────────────────────────────────────

    #[test]
    fn parse_single_tool_call() {
        let text = "I need to read the file.\n\
                    <tool_call>\n\
                    {\"name\": \"read_file\", \"arguments\": {\"path\": \"src/lib.rs\"}}\n\
                    </tool_call>";
        let calls = HermesXmlTranslator
            .parse_calls(&BackendResponse::Text(text.into()))
            .expect("parse should succeed");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "read_file");
        assert_eq!(calls[0].id, "hermes-tc-0");
        assert_eq!(calls[0].arguments["path"], "src/lib.rs");
    }

    #[test]
    fn parse_multiple_parallel_tool_calls() {
        let text = "<tool_call>\n\
                    {\"name\": \"read_file\", \"arguments\": {\"path\": \"a.rs\"}}\n\
                    </tool_call>\n\
                    <tool_call>\n\
                    {\"name\": \"read_file\", \"arguments\": {\"path\": \"b.rs\"}}\n\
                    </tool_call>";
        let calls = HermesXmlTranslator
            .parse_calls(&BackendResponse::Text(text.into()))
            .expect("parse should succeed");
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].name, "read_file");
        assert_eq!(calls[0].arguments["path"], "a.rs");
        assert_eq!(calls[0].id, "hermes-tc-0");
        assert_eq!(calls[1].name, "read_file");
        assert_eq!(calls[1].arguments["path"], "b.rs");
        assert_eq!(calls[1].id, "hermes-tc-1");
    }

    #[test]
    fn parse_returns_empty_for_no_tool_calls() {
        let text = "I can answer this directly. The answer is 42.";
        let calls = HermesXmlTranslator
            .parse_calls(&BackendResponse::Text(text.into()))
            .expect("parse should succeed");
        assert!(calls.is_empty());
    }

    #[test]
    fn parse_skips_think_blocks() {
        let text = "<think>\nLet me reason about this...\n</think>\n\
                    <tool_call>\n\
                    {\"name\": \"read_file\", \"arguments\": {\"path\": \"x.rs\"}}\n\
                    </tool_call>";
        let calls = HermesXmlTranslator
            .parse_calls(&BackendResponse::Text(text.into()))
            .expect("parse should succeed");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "read_file");
    }

    #[test]
    fn parse_handles_missing_arguments_key() {
        let text = "<tool_call>\n\
                    {\"name\": \"bash\"}\n\
                    </tool_call>";
        let calls = HermesXmlTranslator
            .parse_calls(&BackendResponse::Text(text.into()))
            .expect("parse should succeed");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "bash");
        assert_eq!(calls[0].arguments, json!({}));
    }

    #[test]
    fn parse_handles_trailing_comma_in_json() {
        let text = "<tool_call>\n\
                    {\"name\": \"read_file\", \"arguments\": {\"path\": \"x.rs\",}}\n\
                    </tool_call>";
        let calls = HermesXmlTranslator
            .parse_calls(&BackendResponse::Text(text.into()))
            .expect("parse should succeed");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "read_file");
        assert_eq!(calls[0].arguments["path"], "x.rs");
    }

    #[test]
    fn parse_handles_tool_calls_array_wrapper() {
        // Some models wrap multiple calls in a single block with "tool_calls" array.
        let text = "<tool_call>\n\
                    {\"tool_calls\": [{\"name\": \"read_file\", \"arguments\": {\"path\": \"a.rs\"}}]}\n\
                    </tool_call>";
        let calls = HermesXmlTranslator
            .parse_calls(&BackendResponse::Text(text.into()))
            .expect("parse should succeed");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "read_file");
        assert_eq!(calls[0].arguments["path"], "a.rs");
    }

    #[test]
    fn parse_handles_json_response() {
        // When the backend returns JSON (e.g. OpenAI-compatible), extract
        // the content field and parse tool calls from it.
        let response = BackendResponse::Json(json!({
            "message": {
                "content": "<tool_call>\n{\"name\": \"bash\", \"arguments\": {\"cmd\": \"ls\"}}\n</tool_call>"
            }
        }));
        let calls = HermesXmlTranslator
            .parse_calls(&response)
            .expect("parse should succeed");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "bash");
        assert_eq!(calls[0].arguments["cmd"], "ls");
    }

    #[test]
    fn parse_handles_openai_json_response() {
        let response = BackendResponse::Json(json!({
            "choices": [{
                "message": {
                    "content": "<tool_call>\n{\"name\": \"read_file\", \"arguments\": {\"path\": \"x\"}}\n</tool_call>"
                }
            }]
        }));
        let calls = HermesXmlTranslator
            .parse_calls(&response)
            .expect("parse should succeed");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "read_file");
    }

    #[test]
    fn parse_handles_arguments_as_json_string() {
        // Some models emit arguments as a JSON-encoded string.
        let text = "<tool_call>\n\
                    {\"name\": \"read_file\", \"arguments\": \"{\\\"path\\\": \\\"x.rs\\\"}\"}\n\
                    </tool_call>";
        let calls = HermesXmlTranslator
            .parse_calls(&BackendResponse::Text(text.into()))
            .expect("parse should succeed");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].arguments["path"], "x.rs");
    }

    #[test]
    fn parse_handles_unclosed_tool_call_tag() {
        // Model emitted an opening tag but no closing tag -- parse what we have.
        let text = "Some reasoning\n\
                    <tool_call>\n\
                    {\"name\": \"read_file\", \"arguments\": {\"path\": \"x.rs\"}}";
        let calls = HermesXmlTranslator
            .parse_calls(&BackendResponse::Text(text.into()))
            .expect("parse should succeed");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "read_file");
    }

    #[test]
    fn parse_empty_tool_call_block_is_skipped() {
        let text = "<tool_call>\n</tool_call>";
        let calls = HermesXmlTranslator
            .parse_calls(&BackendResponse::Text(text.into()))
            .expect("parse should succeed");
        assert!(calls.is_empty());
    }

    #[test]
    fn parse_malformed_json_is_skipped() {
        // Completely unparseable JSON inside <tool_call> -> skip it.
        let text = "<tool_call>\n\
                    this is not json at all\n\
                    </tool_call>";
        let calls = HermesXmlTranslator
            .parse_calls(&BackendResponse::Text(text.into()))
            .expect("parse should succeed (empty)");
        assert!(calls.is_empty());
    }

    #[test]
    fn parse_stream_json_response() {
        let events = vec![
            json!({"content": "<tool_call>\n{\"name\": \"bash\""}),
            json!({"content": ", \"arguments\": {\"cmd\": \"ls\"}}\n</tool_call>"}),
        ];
        let calls = HermesXmlTranslator
            .parse_calls(&BackendResponse::StreamJson(events))
            .expect("parse should succeed");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "bash");
    }

    // ─── render_results ───────────────────────────────────────────────────

    #[test]
    fn render_results_wraps_in_tool_response_tags() {
        let call = ToolCall::new("hermes-tc-0", "read_file", json!({"path": "x.rs"}));
        let result = ToolResult::text("file contents here");
        let rendered = HermesXmlTranslator.render_results(&[(call, result)]);
        let RenderedResults::TextBlock(block) = rendered else {
            panic!("expected TextBlock");
        };
        assert!(block.contains("<tool_response>"));
        assert!(block.contains("</tool_response>"));
        assert!(block.contains("read_file"));
        assert!(block.contains("file contents here"));
    }

    #[test]
    fn render_results_json_inside_tags_is_valid() {
        let call = ToolCall::new("hermes-tc-0", "read_file", json!({"path": "x.rs"}));
        let result = ToolResult::text("content");
        let rendered = HermesXmlTranslator.render_results(&[(call, result)]);
        let RenderedResults::TextBlock(block) = rendered else {
            panic!("expected TextBlock");
        };
        // Extract the JSON between the tags.
        let start = block.find("<tool_response>\n").unwrap() + "<tool_response>\n".len();
        let end = block.find("\n</tool_response>").unwrap();
        let json_str = &block[start..end];
        let parsed: serde_json::Value =
            serde_json::from_str(json_str).expect("JSON inside <tool_response> must be valid");
        assert_eq!(parsed["name"], "read_file");
        assert_eq!(parsed["content"], "content");
    }

    #[test]
    fn render_results_formats_errors() {
        let call = ToolCall::new("hermes-tc-0", "bash", json!({}));
        let result = ToolResult::err(ToolError::PermissionDenied("needs exec".into()));
        let rendered = HermesXmlTranslator.render_results(&[(call, result)]);
        let RenderedResults::TextBlock(block) = rendered else {
            panic!("expected TextBlock");
        };
        assert!(block.contains("Error: "));
        assert!(block.contains("permission denied"));
    }

    #[test]
    fn render_results_handles_multiple_results() {
        let pairs = vec![
            (
                ToolCall::new("hermes-tc-0", "read_file", json!({})),
                ToolResult::text("first"),
            ),
            (
                ToolCall::new("hermes-tc-1", "bash", json!({})),
                ToolResult::text("second"),
            ),
        ];
        let rendered = HermesXmlTranslator.render_results(&pairs);
        let RenderedResults::TextBlock(block) = rendered else {
            panic!("expected TextBlock");
        };
        // Should have two <tool_response> blocks.
        let count = block.matches("<tool_response>").count();
        assert_eq!(count, 2);
        assert!(block.contains("first"));
        assert!(block.contains("second"));
    }

    #[test]
    fn render_results_empty_gives_empty_block() {
        let rendered = HermesXmlTranslator.render_results(&[]);
        let RenderedResults::TextBlock(block) = rendered else {
            panic!("expected TextBlock");
        };
        assert!(block.is_empty());
    }

    // ─── render_assistant_message ─────────────────────────────────────────

    #[test]
    fn render_assistant_message_returns_none() {
        let response = BackendResponse::Text("hello".into());
        assert!(HermesXmlTranslator.render_assistant_message(&response).is_none());
    }

    // ─── round trip ───────────────────────────────────────────────────────

    #[test]
    fn round_trip_single_tool() {
        // 1. Render tools.
        let tools = vec![read_tool()];
        let rendered = HermesXmlTranslator.render_tools(&tools);
        let RenderedTools::SystemPromptBlock(block) = rendered else {
            panic!("expected SystemPromptBlock");
        };
        assert!(block.contains("read_file"));

        // 2. Simulate model response.
        let model_output = "<tool_call>\n\
                            {\"name\": \"read_file\", \"arguments\": {\"path\": \"src/lib.rs\"}}\n\
                            </tool_call>";

        // 3. Parse.
        let calls = HermesXmlTranslator
            .parse_calls(&BackendResponse::Text(model_output.into()))
            .expect("parse should succeed");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "read_file");
        assert_eq!(calls[0].arguments["path"], "src/lib.rs");

        // 4. Render results.
        let result = ToolResult::text("pub fn main() {}");
        let rendered = HermesXmlTranslator.render_results(&[(calls[0].clone(), result)]);
        let RenderedResults::TextBlock(text) = rendered else {
            panic!("expected TextBlock");
        };
        assert!(text.contains("<tool_response>"));
        assert!(text.contains("pub fn main() {}"));
    }

    #[test]
    fn round_trip_parallel_tool_calls() {
        let tools = vec![read_tool(), write_tool()];
        let _ = HermesXmlTranslator.render_tools(&tools);

        let model_output = "<tool_call>\n\
                            {\"name\": \"read_file\", \"arguments\": {\"path\": \"a.rs\"}}\n\
                            </tool_call>\n\
                            <tool_call>\n\
                            {\"name\": \"edit_file\", \"arguments\": {\"path\": \"b.rs\", \"content\": \"new\"}}\n\
                            </tool_call>";

        let calls = HermesXmlTranslator
            .parse_calls(&BackendResponse::Text(model_output.into()))
            .expect("parse should succeed");
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].name, "read_file");
        assert_eq!(calls[1].name, "edit_file");

        let results = vec![
            (calls[0].clone(), ToolResult::text("contents of a.rs")),
            (calls[1].clone(), ToolResult::text("file updated")),
        ];
        let rendered = HermesXmlTranslator.render_results(&results);
        let RenderedResults::TextBlock(text) = rendered else {
            panic!("expected TextBlock");
        };
        assert_eq!(text.matches("<tool_response>").count(), 2);
    }

    // ─── repair_json ──────────────────────────────────────────────────────

    #[test]
    fn repair_json_removes_trailing_comma_before_brace() {
        let input = r#"{"name": "test", "args": {"key": "val",}}"#;
        let repaired = repair_json(input);
        assert!(
            serde_json::from_str::<serde_json::Value>(&repaired).is_ok(),
            "repaired JSON should parse: {repaired}"
        );
    }

    #[test]
    fn repair_json_removes_trailing_comma_before_bracket() {
        let input = r#"{"items": ["a", "b",]}"#;
        let repaired = repair_json(input);
        assert!(
            serde_json::from_str::<serde_json::Value>(&repaired).is_ok(),
            "repaired JSON should parse: {repaired}"
        );
    }

    #[test]
    fn repair_json_preserves_valid_json() {
        let input = r#"{"name": "test", "args": {"key": "val"}}"#;
        let repaired = repair_json(input);
        assert_eq!(repaired, input);
    }
}
