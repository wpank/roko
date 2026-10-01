//! Forwarding of agent events to the TUI dashboard, and the JSONL append helpers.

use super::*;

impl GraphTaskDispatcher {
    /// Forward dispatch events to the TUI bridge so the dashboard shows
    /// live agent output during Graph plan execution.
    ///
    /// Called after `run_shared_agent_bridge` returns. Each event in the
    /// dispatch result is mapped to the corresponding `TuiBridge` method
    /// which publishes a `DashboardEvent` through the StateHub.
    pub(super) fn forward_dispatch_events_to_tui(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        dispatch: &crate::dispatch_v2::AgentResultDispatch,
        ctx: &CellContext,
    ) {
        let Some(tui) = &self.tui_bridge else {
            return;
        };

        let agent_id = format!(
            "{}/{}",
            spec.plan_id,
            ctx.cell_id.as_deref().unwrap_or(&task.id)
        );
        let plan_id = &spec.plan_id;
        let task_id = &task.id;

        // `agent_spawned` is now published immediately before dispatch starts
        // (see the `dispatch` fn). Do not publish it here to avoid a duplicate.

        // Forward each provider event as a TUI stream record.
        for event in &dispatch.events {
            match event {
                roko_agent::AgentRuntimeEvent::MessageDelta { text } => {
                    tui.agent_text_delta(&agent_id, plan_id, task_id, 0, text);
                }
                roko_agent::AgentRuntimeEvent::ToolCall { id, name } => {
                    tui.tool_call(&agent_id, plan_id, task_id, 0, id, name);
                }
                roko_agent::AgentRuntimeEvent::ToolOutput { id, output } => {
                    // Truncate tool output for the TUI to avoid overwhelming
                    // the bounded stream ring buffer.
                    //
                    // Safety: floor the slice start to a char boundary so we
                    // never split a multi-byte character, which would panic.
                    let truncated = if output.len() > 2048 {
                        let raw_start = output.len().saturating_sub(1024);
                        // Walk backwards until we land on a char boundary.
                        let char_start = (0..=raw_start)
                            .rev()
                            .find(|&i| output.is_char_boundary(i))
                            .unwrap_or(0);
                        let tail = &output[char_start..];
                        format!("[...truncated]\n{tail}")
                    } else {
                        output.clone()
                    };
                    tui.tool_output(&agent_id, plan_id, task_id, 0, id, &truncated);
                }
                _ => {}
            }
        }

        // Emit agent completed event.
        tui.agent_completed(&agent_id, plan_id, task_id, 0);
    }
}

/// Forward a single [`roko_agent::live_output::LiveAgentEvent`] to the TUI
/// bridge.
///
/// Called from the forwarder task spawned alongside the heartbeat task.
/// Mapping:
/// - `ToolStep` → `TuiBridge::tool_step` (always; safe, scrubbed target)
/// - `Unscreened(TextDelta)` → unscreened `text` record
/// - `Unscreened(ReasoningDelta)` → unscreened `reasoning` record
/// - `Unscreened(ToolCallEnd)` → unscreened `tool_start` record with args
///   truncated to 2 048 bytes on a char boundary
/// - `Unscreened(ToolResult)` → unscreened `tool_result` record with output
///   truncated the same way `forward_dispatch_events_to_tui` truncates it
/// - All other `Unscreened` variants are silently ignored.
pub(super) fn forward_live_event_to_tui(
    tui: &TuiBridge,
    agent_id: &str,
    plan_id: &str,
    task_id: &str,
    event: roko_agent::live_output::LiveAgentEvent,
) {
    use roko_agent::StreamEventKind;
    use roko_agent::live_output::LiveAgentEvent;

    const ARGS_MAX_BYTES: usize = 2048;

    match event {
        LiveAgentEvent::ToolStep { id, name, target } => {
            tui.tool_step(agent_id, plan_id, task_id, 0, &id, &name, &target);
        }
        LiveAgentEvent::Unscreened(kind) => match kind {
            StreamEventKind::TextDelta(text) => {
                tui.publish_unscreened_stream_record(
                    agent_id,
                    plan_id,
                    task_id,
                    0,
                    "text",
                    serde_json::json!({"text": text}),
                );
            }
            StreamEventKind::ReasoningDelta(text) => {
                tui.publish_unscreened_stream_record(
                    agent_id,
                    plan_id,
                    task_id,
                    0,
                    "reasoning",
                    serde_json::json!({"text": text}),
                );
            }
            StreamEventKind::ToolCallEnd { id, name, args } => {
                // Serialize args and truncate to ARGS_MAX_BYTES on a char
                // boundary to avoid overwhelming the TUI ring buffer.
                let args_str = serde_json::to_string(&args).unwrap_or_default();
                let args_truncated = if args_str.len() > ARGS_MAX_BYTES {
                    let cut = (0..=ARGS_MAX_BYTES)
                        .rev()
                        .find(|&i| args_str.is_char_boundary(i))
                        .unwrap_or(0);
                    format!("{}…", &args_str[..cut])
                } else {
                    args_str
                };
                tui.publish_unscreened_stream_record(
                    agent_id,
                    plan_id,
                    task_id,
                    0,
                    "tool_start",
                    serde_json::json!({
                        "tool_id": id,
                        "tool": name,
                        "args": args_truncated,
                    }),
                );
            }
            StreamEventKind::ToolResult { id, output } => {
                // Truncate tool output the same way forward_dispatch_events_to_tui
                // does: keep the last 1 024 bytes (aligned to a char boundary).
                let truncated = if output.len() > 2048 {
                    let raw_start = output.len().saturating_sub(1024);
                    let char_start = (0..=raw_start)
                        .rev()
                        .find(|&i| output.is_char_boundary(i))
                        .unwrap_or(0);
                    let tail = &output[char_start..];
                    format!("[...truncated]\n{tail}")
                } else {
                    output
                };
                tui.publish_unscreened_stream_record(
                    agent_id,
                    plan_id,
                    task_id,
                    0,
                    "tool_result",
                    serde_json::json!({"tool_id": id, "output": truncated}),
                );
            }
            // All other stream event kinds (ToolCallStart, ToolCallDelta,
            // Usage, Done) are not forwarded as unscreened records.
            _ => {}
        },
    }
}

/// Append a single JSON line to a JSONL file, creating parent dirs as needed.
#[allow(dead_code)] // sync fallback; production paths use append_jsonl_line_async
fn append_jsonl_line(path: &std::path::Path, value: &impl serde::Serialize) -> std::io::Result<()> {
    use std::io::Write;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut line = serde_json::to_string(value)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    line.push('\n');
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    file.write_all(line.as_bytes())?;
    file.flush()?;
    Ok(())
}

/// Async wrapper for [`append_jsonl_line`].
///
/// Serializes `value` on the calling async task (cheap), then offloads the
/// blocking file I/O to a `spawn_blocking` thread so the Tokio reactor is
/// not stalled on disk writes inside `async fn emit_feedback`. The process's
/// secrets are redacted from the record first: efficiency and gate-failure
/// records carry agent and verify output.
pub(super) async fn append_jsonl_line_async(
    path: std::path::PathBuf,
    line: String,
) -> std::io::Result<()> {
    tokio::task::spawn_blocking(move || {
        use std::io::Write;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut line = roko_core::obs::scrub_secrets_in_jsonl(&line).into_owned();
        line.push('\n');
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        // One write per row, newline included: rows appended at once would
        // otherwise interleave a row with another's newline (bug-779ae7).
        file.write_all(line.as_bytes())?;
        file.flush()?;
        Ok(())
    })
    .await
    .unwrap_or_else(|join_err| Err(std::io::Error::new(std::io::ErrorKind::Other, join_err)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Rows appended at once each land whole, on a line of their own
    /// (bug-779ae7).
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn rows_appended_at_once_stay_whole() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("rows.jsonl");
        let appends: Vec<_> = (0..64_u64)
            .map(|id| {
                let row = serde_json::json!({ "id": id, "pad": "x".repeat(512) });
                tokio::spawn(append_jsonl_line_async(path.clone(), row.to_string()))
            })
            .collect();
        for append in appends {
            append.await.expect("join").expect("append");
        }

        let mut ids: Vec<u64> = std::fs::read_to_string(&path)
            .expect("rows")
            .lines()
            .map(|line| {
                let row: serde_json::Value = serde_json::from_str(line).expect("a whole row");
                row["id"].as_u64().expect("id")
            })
            .collect();
        ids.sort_unstable();
        assert_eq!(ids, (0..64).collect::<Vec<_>>());
    }
}
