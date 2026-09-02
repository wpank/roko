//! Convert [`TranscriptRecord`] sequences into [`TranscriptBlock`]s.
//!
//! The converter correlates tool start/finish events by call ID, accumulates
//! text deltas into complete blocks, and synthesizes terminal events for
//! orphaned tool calls (disconnection, missing result).

use std::collections::HashMap;

use roko_core::tool::transcript::{ToolLifecycleStatus, TranscriptEvent, TranscriptRecord};

use super::block::{MessageLevel, SubagentBlockStatus, ToolBlockStatus, TranscriptBlock};

/// Maximum preview length for tool results and arguments.
const MAX_PREVIEW_LEN: usize = 512;

/// Convert a lifecycle status from the core model into a display status.
fn map_tool_status(status: ToolLifecycleStatus) -> ToolBlockStatus {
    match status {
        ToolLifecycleStatus::Pending | ToolLifecycleStatus::Admitted => ToolBlockStatus::Pending,
        ToolLifecycleStatus::Executing => ToolBlockStatus::Running,
        ToolLifecycleStatus::Succeeded => ToolBlockStatus::Succeeded,
        ToolLifecycleStatus::Failed | ToolLifecycleStatus::Panicked => ToolBlockStatus::Failed,
        ToolLifecycleStatus::TimedOut => ToolBlockStatus::TimedOut,
        ToolLifecycleStatus::Cancelled | ToolLifecycleStatus::Interrupted => {
            ToolBlockStatus::Cancelled
        }
        ToolLifecycleStatus::Denied => ToolBlockStatus::Denied,
        _ => ToolBlockStatus::Failed, // future variants treated as failure
    }
}

/// Truncate a string to `max_len`, appending an ellipsis if truncated.
fn preview(text: &str, max_len: usize) -> (String, bool) {
    if text.len() <= max_len {
        (text.to_string(), false)
    } else {
        let mut boundary = max_len.min(text.len());
        while boundary > 0 && !text.is_char_boundary(boundary) {
            boundary -= 1;
        }
        (format!("{}...", &text[..boundary]), true)
    }
}

// ─── Stateful builder ───────────────────────────────────────────────────

/// Intermediate state for building blocks from a record stream.
struct BlockBuilder {
    blocks: Vec<TranscriptBlock>,
    /// Accumulated assistant text (streamed deltas).
    assistant_text: String,
    assistant_streaming: bool,
    /// Accumulated reasoning text.
    reasoning_text: String,
    reasoning_streaming: bool,
    /// In-flight tool calls by call_id.
    open_tools: HashMap<String, ToolBlockState>,
    /// In-flight subagents by subagent_id.
    open_subagents: HashMap<String, SubagentState>,
    /// Accumulated tool output deltas by call_id.
    tool_output_deltas: HashMap<String, String>,
}

struct ToolBlockState {
    call_id: String,
    tool_name: String,
    arguments_preview: Option<String>,
    status: ToolBlockStatus,
}

struct SubagentState {
    agent_id: String,
    task: String,
    children: Vec<TranscriptBlock>,
}

impl BlockBuilder {
    fn new() -> Self {
        Self {
            blocks: Vec::new(),
            assistant_text: String::new(),
            assistant_streaming: false,
            reasoning_text: String::new(),
            reasoning_streaming: false,
            open_tools: HashMap::new(),
            open_subagents: HashMap::new(),
            tool_output_deltas: HashMap::new(),
        }
    }

    /// Flush any accumulated assistant text into a block.
    fn flush_assistant(&mut self) {
        if !self.assistant_text.is_empty() {
            self.blocks.push(TranscriptBlock::AssistantText {
                text: std::mem::take(&mut self.assistant_text),
                is_streaming: false,
            });
            self.assistant_streaming = false;
        }
    }

    /// Flush any accumulated reasoning text into a block.
    fn flush_reasoning(&mut self) {
        if !self.reasoning_text.is_empty() {
            self.blocks.push(TranscriptBlock::Reasoning {
                text: std::mem::take(&mut self.reasoning_text),
                is_streaming: false,
            });
            self.reasoning_streaming = false;
        }
    }

    /// Process a single transcript record.
    fn process(&mut self, record: &TranscriptRecord) {
        match &record.event {
            TranscriptEvent::AssistantDelta { text } => {
                self.flush_reasoning();
                self.assistant_text.push_str(text);
                self.assistant_streaming = true;
            }

            TranscriptEvent::ReasoningDelta { text } => {
                self.flush_assistant();
                self.reasoning_text.push_str(text);
                self.reasoning_streaming = true;
            }

            TranscriptEvent::ToolStarted { call, status, .. } => {
                self.flush_assistant();
                self.flush_reasoning();
                let args_preview = if call.arguments.is_null() {
                    None
                } else {
                    let s = serde_json::to_string(&call.arguments).unwrap_or_default();
                    let (p, _) = preview(&s, MAX_PREVIEW_LEN);
                    Some(p)
                };
                self.open_tools.insert(
                    call.id.clone(),
                    ToolBlockState {
                        call_id: call.id.clone(),
                        tool_name: call.name.clone(),
                        arguments_preview: args_preview,
                        status: map_tool_status(*status),
                    },
                );
            }

            TranscriptEvent::ToolOutputDelta { call_id, text } => {
                self.tool_output_deltas
                    .entry(call_id.clone())
                    .or_default()
                    .push_str(text);
            }

            TranscriptEvent::ToolFinished {
                call_id,
                result,
                status,
                execution_ms,
            } => {
                // Merge any accumulated output deltas.
                let delta_output = self.tool_output_deltas.remove(call_id);

                let (result_text, error_text) = match result {
                    roko_core::tool::ToolResult::Ok { content, .. } => {
                        (Some(content.clone()), None)
                    }
                    roko_core::tool::ToolResult::Err(err) => (None, Some(err.to_string())),
                };

                // Use delta output if the result output is empty.
                let effective_result = result_text.filter(|s| !s.is_empty()).or(delta_output);

                let (result_preview, truncated) = effective_result
                    .as_deref()
                    .map(|s| preview(s, MAX_PREVIEW_LEN))
                    .unwrap_or((String::new(), false));

                let result_preview = if result_preview.is_empty() {
                    None
                } else {
                    Some(result_preview)
                };

                // Check if we have an open tool for this call_id.
                let (tool_name, arguments_preview) =
                    if let Some(state) = self.open_tools.remove(call_id) {
                        (state.tool_name, state.arguments_preview)
                    } else {
                        // Orphan result — no matching start event.
                        (String::from("<unknown>"), None)
                    };

                self.blocks.push(TranscriptBlock::ToolCall {
                    call_id: call_id.clone(),
                    tool_name,
                    arguments_preview,
                    status: map_tool_status(*status),
                    duration_ms: *execution_ms,
                    result_preview,
                    error: error_text,
                    truncated,
                    redacted: false,
                });
            }

            TranscriptEvent::TodoSnapshot { items } => {
                self.flush_assistant();
                self.flush_reasoning();
                // Extract individual items from the array, or emit a single block.
                if let Some(arr) = items.as_array() {
                    for item in arr {
                        let id = item.get("id").map(|v| v.to_string()).unwrap_or_default();
                        let title = item
                            .get("text")
                            .or_else(|| item.get("title"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let status = item
                            .get("status")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                            .to_string();
                        let progress = item.get("progress").and_then(|v| v.as_f64());
                        self.blocks.push(TranscriptBlock::TodoUpdate {
                            todo_id: id,
                            title,
                            status,
                            progress,
                        });
                    }
                }
            }

            TranscriptEvent::SubagentStarted { subagent_id, task } => {
                self.flush_assistant();
                self.flush_reasoning();
                self.open_subagents.insert(
                    subagent_id.clone(),
                    SubagentState {
                        agent_id: subagent_id.clone(),
                        task: task.clone(),
                        children: Vec::new(),
                    },
                );
            }

            TranscriptEvent::SubagentUpdate { subagent_id, .. } => {
                // Subagent updates are accumulated as child events; for now
                // we mark the subagent as running (children come from nested
                // records via parent_event_id).
                if let Some(state) = self.open_subagents.get_mut(subagent_id) {
                    // Mark running (no extra block needed).
                    let _ = state;
                }
            }

            TranscriptEvent::SubagentFinished {
                subagent_id,
                success,
                ..
            } => {
                let status = if *success {
                    SubagentBlockStatus::Completed
                } else {
                    SubagentBlockStatus::Failed
                };
                let state = self.open_subagents.remove(subagent_id);
                let (agent_name, children) = state
                    .map(|s| (s.task, s.children))
                    .unwrap_or_else(|| (String::from("<unknown>"), Vec::new()));
                self.blocks.push(TranscriptBlock::SubagentBlock {
                    agent_id: subagent_id.clone(),
                    agent_name,
                    status,
                    children,
                });
            }

            TranscriptEvent::Usage {
                input_tokens,
                output_tokens,
                cache_read_tokens,
                ..
            } => {
                let cache = if *cache_read_tokens > 0 {
                    Some(*cache_read_tokens)
                } else {
                    None
                };
                self.blocks.push(TranscriptBlock::UsageReport {
                    input_tokens: *input_tokens,
                    output_tokens: *output_tokens,
                    cache_tokens: cache,
                });
            }

            TranscriptEvent::ProviderChanged {
                from_provider,
                to_provider,
                reason,
                ..
            } => {
                self.flush_assistant();
                self.flush_reasoning();
                self.blocks.push(TranscriptBlock::ProviderChange {
                    from: from_provider.clone().unwrap_or_default(),
                    to: to_provider.clone(),
                    reason: reason.clone(),
                });
            }

            TranscriptEvent::Warning { message, .. } => {
                self.blocks.push(TranscriptBlock::SystemMessage {
                    level: MessageLevel::Warning,
                    text: message.clone(),
                });
            }

            TranscriptEvent::Error { message, .. } => {
                self.blocks.push(TranscriptBlock::SystemMessage {
                    level: MessageLevel::Error,
                    text: message.clone(),
                });
            }

            TranscriptEvent::RunStarted { .. } => {
                self.blocks.push(TranscriptBlock::SystemMessage {
                    level: MessageLevel::Info,
                    text: "Run started".to_string(),
                });
            }

            TranscriptEvent::RunFinished { success, .. } => {
                self.flush_assistant();
                self.flush_reasoning();
                let msg = if *success {
                    "Run completed successfully"
                } else {
                    "Run failed"
                };
                self.blocks.push(TranscriptBlock::SystemMessage {
                    level: if *success {
                        MessageLevel::Info
                    } else {
                        MessageLevel::Error
                    },
                    text: msg.to_string(),
                });
            }

            // Future event variants — emit a diagnostic block.
            _ => {}
        }
    }

    /// Finalize: flush remaining text, close orphaned tool calls.
    fn finish(mut self) -> Vec<TranscriptBlock> {
        self.flush_assistant();
        self.flush_reasoning();

        // Synthesize terminal events for tool calls that never got a result
        // (disconnection, provider error, etc.).
        for (_, state) in self.open_tools.drain() {
            self.blocks.push(TranscriptBlock::ToolCall {
                call_id: state.call_id,
                tool_name: state.tool_name,
                arguments_preview: state.arguments_preview,
                status: ToolBlockStatus::Failed,
                duration_ms: None,
                result_preview: None,
                error: Some("Tool call never received a result (orphaned)".to_string()),
                truncated: false,
                redacted: false,
            });
        }

        // Close orphaned subagents.
        for (_, state) in self.open_subagents.drain() {
            self.blocks.push(TranscriptBlock::SubagentBlock {
                agent_id: state.agent_id,
                agent_name: state.task,
                status: SubagentBlockStatus::Failed,
                children: state.children,
            });
        }

        self.blocks
    }
}

// ─── Public API ─────────────────────────────────────────────────────────

/// Convert a sequence of [`TranscriptRecord`]s into semantic [`TranscriptBlock`]s.
///
/// Records should be sorted by `meta.sequence`. The converter:
/// - Correlates tool start/finish events by call ID
/// - Accumulates text and reasoning deltas into complete blocks
/// - Synthesizes terminal events for orphaned tool calls
pub fn blocks_from_records(records: &[TranscriptRecord]) -> Vec<TranscriptBlock> {
    let mut builder = BlockBuilder::new();
    for record in records {
        builder.process(record);
    }
    builder.finish()
}
