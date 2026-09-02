//! Tests and fixtures for the transcript module.

use roko_core::tool::call::{ToolCall, ToolResult};
use roko_core::tool::def::ToolCategory;
use roko_core::tool::transcript::{
    ToolLifecycleStatus, TranscriptEvent, TranscriptEventMeta, TranscriptRecord,
};

use super::block::{MessageLevel, SubagentBlockStatus, ToolBlockStatus, TranscriptBlock};
use super::convert::blocks_from_records;
use super::fold::{FoldRule, FoldState};
use super::projection::{BlockFilter, BlockQuery, TranscriptProjection};

// ─── Helpers ────────────────────────────────────────────────────────────

fn meta(seq: u64) -> TranscriptEventMeta {
    TranscriptEventMeta {
        run_id: "run-test".into(),
        turn_id: 0,
        agent_id: "agent-test".into(),
        sequence: seq,
        timestamp_ms: 1_700_000_000_000 + (seq as i64 * 100),
        provider: "anthropic".into(),
        model: "claude-opus-4-6".into(),
        parent_event_id: None,
    }
}

fn record(seq: u64, event: TranscriptEvent) -> TranscriptRecord {
    TranscriptRecord {
        meta: meta(seq),
        event,
    }
}

fn make_call(id: &str, name: &str) -> ToolCall {
    ToolCall::at(
        id,
        name,
        serde_json::json!({"path": "src/main.rs"}),
        1_700_000_000_000,
    )
}

// ─── Fixture: two interleaved tools (one success, one failure) ──────────

fn fixture_interleaved_tools() -> Vec<TranscriptRecord> {
    vec![
        record(
            1,
            TranscriptEvent::RunStarted {
                system_prompt_hash: Some("abc".into()),
                tools_offered: 16,
            },
        ),
        record(
            2,
            TranscriptEvent::AssistantDelta {
                text: "Let me read the file.".into(),
            },
        ),
        record(
            3,
            TranscriptEvent::ToolStarted {
                call: make_call("call-1", "read_file"),
                status: ToolLifecycleStatus::Admitted,
                category: Some(ToolCategory::Read),
            },
        ),
        record(
            4,
            TranscriptEvent::ToolStarted {
                call: make_call("call-2", "bash"),
                status: ToolLifecycleStatus::Admitted,
                category: Some(ToolCategory::Exec),
            },
        ),
        record(
            5,
            TranscriptEvent::ToolFinished {
                call_id: "call-1".into(),
                result: ToolResult::text("fn main() { }"),
                status: ToolLifecycleStatus::Succeeded,
                execution_ms: Some(42),
            },
        ),
        record(
            6,
            TranscriptEvent::ToolFinished {
                call_id: "call-2".into(),
                result: ToolResult::err(roko_core::tool::ToolError::Other(
                    "command not found".into(),
                )),
                status: ToolLifecycleStatus::Failed,
                execution_ms: Some(100),
            },
        ),
        record(
            7,
            TranscriptEvent::RunFinished {
                success: true,
                total_turns: 1,
                total_tool_calls: 2,
                wall_ms: 500,
            },
        ),
    ]
}

// ─── Fixture: reasoning interleaved with tool calls ─────────────────────

fn fixture_reasoning_with_tools() -> Vec<TranscriptRecord> {
    vec![
        record(
            1,
            TranscriptEvent::ReasoningDelta {
                text: "I need to check the code first.".into(),
            },
        ),
        record(
            2,
            TranscriptEvent::ReasoningDelta {
                text: " Let me look at the imports.".into(),
            },
        ),
        record(
            3,
            TranscriptEvent::ToolStarted {
                call: make_call("call-r1", "grep"),
                status: ToolLifecycleStatus::Admitted,
                category: Some(ToolCategory::Read),
            },
        ),
        record(
            4,
            TranscriptEvent::ToolFinished {
                call_id: "call-r1".into(),
                result: ToolResult::text("use std::io;"),
                status: ToolLifecycleStatus::Succeeded,
                execution_ms: Some(15),
            },
        ),
        record(
            5,
            TranscriptEvent::AssistantDelta {
                text: "Found the import.".into(),
            },
        ),
    ]
}

// ─── Fixture: subagent with child tool calls ────────────────────────────

fn fixture_subagent_with_children() -> Vec<TranscriptRecord> {
    vec![
        record(
            1,
            TranscriptEvent::SubagentStarted {
                subagent_id: "sub-1".into(),
                task: "Research authentication patterns".into(),
            },
        ),
        record(
            2,
            TranscriptEvent::SubagentUpdate {
                subagent_id: "sub-1".into(),
                payload: serde_json::json!({"progress": 0.5}),
            },
        ),
        record(
            3,
            TranscriptEvent::SubagentFinished {
                subagent_id: "sub-1".into(),
                success: true,
                summary: Some("Found 3 auth patterns".into()),
            },
        ),
    ]
}

// ─── Fixture: truncated and redacted results ────────────────────────────

fn fixture_truncated_and_large_results() -> Vec<TranscriptRecord> {
    let large_output = "x".repeat(2000);
    vec![
        record(
            1,
            TranscriptEvent::ToolStarted {
                call: make_call("call-big", "bash"),
                status: ToolLifecycleStatus::Admitted,
                category: Some(ToolCategory::Exec),
            },
        ),
        record(
            2,
            TranscriptEvent::ToolFinished {
                call_id: "call-big".into(),
                result: ToolResult::text(&large_output),
                status: ToolLifecycleStatus::Succeeded,
                execution_ms: Some(1500),
            },
        ),
    ]
}

// ═══ Tests ══════════════════════════════════════════════════════════════

#[test]
fn interleaved_tools_produce_correct_block_sequence() {
    let records = fixture_interleaved_tools();
    let blocks = blocks_from_records(&records);

    // Expected: SystemMessage(RunStarted), AssistantText, ToolCall(success),
    //           ToolCall(failure), SystemMessage(RunFinished)
    assert!(
        blocks.len() >= 4,
        "expected at least 4 blocks, got {}",
        blocks.len()
    );

    // Find tool blocks
    let tool_blocks: Vec<_> = blocks
        .iter()
        .filter(|b| matches!(b, TranscriptBlock::ToolCall { .. }))
        .collect();
    assert_eq!(tool_blocks.len(), 2, "expected 2 tool blocks");

    // First tool succeeded
    if let TranscriptBlock::ToolCall {
        call_id,
        status,
        duration_ms,
        ..
    } = &tool_blocks[0]
    {
        assert_eq!(call_id, "call-1");
        assert_eq!(*status, ToolBlockStatus::Succeeded);
        assert_eq!(*duration_ms, Some(42));
    } else {
        panic!("expected ToolCall block");
    }

    // Second tool failed
    if let TranscriptBlock::ToolCall {
        call_id,
        status,
        error,
        ..
    } = &tool_blocks[1]
    {
        assert_eq!(call_id, "call-2");
        assert_eq!(*status, ToolBlockStatus::Failed);
        assert!(error.is_some(), "failed tool must have error");
    } else {
        panic!("expected ToolCall block");
    }
}

#[test]
fn reasoning_is_flushed_before_tool_call() {
    let records = fixture_reasoning_with_tools();
    let blocks = blocks_from_records(&records);

    // First block should be reasoning
    assert!(
        matches!(&blocks[0], TranscriptBlock::Reasoning { text, .. } if text.contains("I need to check")),
        "first block should be reasoning: {:?}",
        blocks[0]
    );
}

#[test]
fn subagent_lifecycle_produces_subagent_block() {
    let records = fixture_subagent_with_children();
    let blocks = blocks_from_records(&records);

    let subagent_blocks: Vec<_> = blocks
        .iter()
        .filter(|b| matches!(b, TranscriptBlock::SubagentBlock { .. }))
        .collect();
    assert_eq!(subagent_blocks.len(), 1);

    if let TranscriptBlock::SubagentBlock {
        agent_id, status, ..
    } = &subagent_blocks[0]
    {
        assert_eq!(agent_id, "sub-1");
        assert_eq!(*status, SubagentBlockStatus::Completed);
    }
}

#[test]
fn large_tool_result_is_truncated_in_preview() {
    let records = fixture_truncated_and_large_results();
    let blocks = blocks_from_records(&records);

    let tool_blocks: Vec<_> = blocks
        .iter()
        .filter(|b| matches!(b, TranscriptBlock::ToolCall { .. }))
        .collect();
    assert_eq!(tool_blocks.len(), 1);

    if let TranscriptBlock::ToolCall {
        truncated,
        result_preview,
        ..
    } = &tool_blocks[0]
    {
        assert!(*truncated, "large result must be marked truncated");
        assert!(
            result_preview.as_ref().map_or(false, |p| p.len() < 2000),
            "preview must be shorter than full result"
        );
    }
}

#[test]
fn orphaned_tool_call_gets_synthetic_terminal_event() {
    // A tool start with no matching finish
    let records = vec![record(
        1,
        TranscriptEvent::ToolStarted {
            call: make_call("orphan-1", "bash"),
            status: ToolLifecycleStatus::Executing,
            category: None,
        },
    )];
    let blocks = blocks_from_records(&records);

    let tool_blocks: Vec<_> = blocks
        .iter()
        .filter(|b| matches!(b, TranscriptBlock::ToolCall { .. }))
        .collect();
    assert_eq!(tool_blocks.len(), 1);

    if let TranscriptBlock::ToolCall {
        call_id,
        status,
        error,
        ..
    } = &tool_blocks[0]
    {
        assert_eq!(call_id, "orphan-1");
        assert_eq!(*status, ToolBlockStatus::Failed);
        assert!(error.as_ref().map_or(false, |e| e.contains("orphaned")));
    }
}

// ─── Fold tests ─────────────────────────────────────────────────────────

#[test]
fn fold_state_toggle_round_trips() {
    assert_eq!(FoldRule::toggle(FoldState::Expanded), FoldState::Collapsed);
    assert_eq!(FoldRule::toggle(FoldState::Collapsed), FoldState::Expanded);
    assert_eq!(FoldRule::toggle(FoldState::AutoFolded), FoldState::Expanded);
}

#[test]
fn error_tool_calls_are_always_expanded() {
    let rule = FoldRule::default();
    let block = TranscriptBlock::ToolCall {
        call_id: "err-1".into(),
        tool_name: "bash".into(),
        arguments_preview: None,
        status: ToolBlockStatus::Failed,
        duration_ms: None,
        result_preview: None,
        error: Some("panic".into()),
        truncated: false,
        redacted: false,
    };
    assert_eq!(rule.initial_state(&block), FoldState::Expanded);
}

#[test]
fn large_successful_result_is_auto_folded() {
    let rule = FoldRule {
        auto_fold_bytes: 100,
        errors_always_expanded: true,
    };
    let block = TranscriptBlock::ToolCall {
        call_id: "big-1".into(),
        tool_name: "read_file".into(),
        arguments_preview: None,
        status: ToolBlockStatus::Succeeded,
        duration_ms: Some(10),
        result_preview: Some("x".repeat(200)),
        error: None,
        truncated: true,
        redacted: false,
    };
    assert_eq!(rule.initial_state(&block), FoldState::AutoFolded);
}

// ─── Projection tests ──────────────────────────────────────────────────

#[test]
fn projection_search_finds_text_across_blocks() {
    let records = fixture_interleaved_tools();
    let blocks = blocks_from_records(&records);
    let proj = TranscriptProjection::new(blocks);

    let hits = proj.search("read the file");
    assert!(
        !hits.is_empty(),
        "should find 'read the file' in assistant text"
    );
}

#[test]
fn projection_filter_by_tool_status() {
    let records = fixture_interleaved_tools();
    let blocks = blocks_from_records(&records);
    let proj = TranscriptProjection::new(blocks);

    let failed = proj.tool_calls_by_status(ToolBlockStatus::Failed);
    assert_eq!(failed.len(), 1, "expected 1 failed tool call");

    let succeeded = proj.tool_calls_by_status(ToolBlockStatus::Succeeded);
    assert_eq!(succeeded.len(), 1, "expected 1 succeeded tool call");
}

#[test]
fn projection_page_respects_cursor_and_limit() {
    let records = fixture_interleaved_tools();
    let blocks = blocks_from_records(&records);
    let proj = TranscriptProjection::new(blocks);

    let total = proj.len();
    let page1 = proj.page(&BlockQuery {
        filter: BlockFilter::default(),
        cursor: 0,
        limit: 2,
    });
    assert_eq!(page1.len(), 2);

    let page2 = proj.page(&BlockQuery {
        filter: BlockFilter::default(),
        cursor: 2,
        limit: 100,
    });
    assert_eq!(page2.len(), total - 2);
}

#[test]
fn projection_toggle_fold_persists() {
    let records = fixture_truncated_and_large_results();
    let blocks = blocks_from_records(&records);
    let mut proj = TranscriptProjection::with_fold_rule(
        blocks,
        FoldRule {
            auto_fold_bytes: 100,
            errors_always_expanded: true,
        },
    );

    // Find the tool block index
    let tool_idx = proj
        .blocks()
        .iter()
        .position(|b| matches!(b, TranscriptBlock::ToolCall { .. }))
        .expect("tool block must exist");

    // Should be auto-folded initially
    assert!(
        proj.fold_state(tool_idx).is_collapsed(),
        "large result should be auto-folded"
    );

    // Toggle to expanded
    proj.toggle_fold(tool_idx);
    assert_eq!(proj.fold_state(tool_idx), FoldState::Expanded);

    // Toggle back to collapsed (user choice, not auto)
    proj.toggle_fold(tool_idx);
    assert_eq!(proj.fold_state(tool_idx), FoldState::Collapsed);
}

#[test]
fn projection_push_and_replace_last() {
    let mut proj = TranscriptProjection::new(Vec::new());
    proj.push(TranscriptBlock::AssistantText {
        text: "hello".into(),
        is_streaming: true,
    });
    assert_eq!(proj.len(), 1);

    // Replace the last streaming block with an updated version
    let replaced = proj.replace_last_if(
        TranscriptBlock::AssistantText {
            text: "hello world".into(),
            is_streaming: true,
        },
        "assistant_text",
    );
    assert!(replaced);
    assert_eq!(proj.len(), 1);
    if let TranscriptBlock::AssistantText { text, .. } = &proj.blocks()[0] {
        assert_eq!(text, "hello world");
    }

    // Trying to replace with wrong type should not replace
    let not_replaced = proj.replace_last_if(
        TranscriptBlock::Reasoning {
            text: "thinking".into(),
            is_streaming: false,
        },
        "reasoning",
    );
    assert!(!not_replaced);
    assert_eq!(proj.len(), 1);
}

#[test]
fn block_type_returns_stable_strings() {
    let blocks = vec![
        TranscriptBlock::AssistantText {
            text: "hi".into(),
            is_streaming: false,
        },
        TranscriptBlock::Reasoning {
            text: "hmm".into(),
            is_streaming: false,
        },
        TranscriptBlock::ToolCall {
            call_id: "c".into(),
            tool_name: "t".into(),
            arguments_preview: None,
            status: ToolBlockStatus::Succeeded,
            duration_ms: None,
            result_preview: None,
            error: None,
            truncated: false,
            redacted: false,
        },
        TranscriptBlock::TodoUpdate {
            todo_id: "1".into(),
            title: "x".into(),
            status: "done".into(),
            progress: None,
        },
        TranscriptBlock::SubagentBlock {
            agent_id: "a".into(),
            agent_name: "n".into(),
            status: SubagentBlockStatus::Completed,
            children: vec![],
        },
        TranscriptBlock::SystemMessage {
            level: MessageLevel::Info,
            text: "ok".into(),
        },
        TranscriptBlock::UsageReport {
            input_tokens: 100,
            output_tokens: 50,
            cache_tokens: None,
        },
        TranscriptBlock::ProviderChange {
            from: "a".into(),
            to: "b".into(),
            reason: "rate limit".into(),
        },
    ];

    let expected_types = [
        "assistant_text",
        "reasoning",
        "tool_call",
        "todo_update",
        "subagent",
        "system_message",
        "usage_report",
        "provider_change",
    ];

    for (block, expected) in blocks.iter().zip(expected_types.iter()) {
        assert_eq!(block.block_type(), *expected);
    }
}

#[test]
fn contains_text_searches_recursively_in_subagent() {
    let block = TranscriptBlock::SubagentBlock {
        agent_id: "s".into(),
        agent_name: "Research".into(),
        status: SubagentBlockStatus::Completed,
        children: vec![TranscriptBlock::AssistantText {
            text: "Found important result".into(),
            is_streaming: false,
        }],
    };

    assert!(block.contains_text("important"));
    assert!(block.contains_text("Research"));
    assert!(!block.contains_text("missing"));
}

#[test]
fn tool_output_deltas_are_merged_into_result() {
    let records = vec![
        record(
            1,
            TranscriptEvent::ToolStarted {
                call: make_call("delta-1", "bash"),
                status: ToolLifecycleStatus::Admitted,
                category: None,
            },
        ),
        record(
            2,
            TranscriptEvent::ToolOutputDelta {
                call_id: "delta-1".into(),
                text: "line 1\n".into(),
            },
        ),
        record(
            3,
            TranscriptEvent::ToolOutputDelta {
                call_id: "delta-1".into(),
                text: "line 2\n".into(),
            },
        ),
        record(
            4,
            TranscriptEvent::ToolFinished {
                call_id: "delta-1".into(),
                result: ToolResult::text(""),
                status: ToolLifecycleStatus::Succeeded,
                execution_ms: Some(50),
            },
        ),
    ];

    let blocks = blocks_from_records(&records);
    let tool_blocks: Vec<_> = blocks
        .iter()
        .filter(|b| matches!(b, TranscriptBlock::ToolCall { .. }))
        .collect();
    assert_eq!(tool_blocks.len(), 1);

    if let TranscriptBlock::ToolCall { result_preview, .. } = &tool_blocks[0] {
        let preview = result_preview.as_deref().unwrap_or("");
        assert!(
            preview.contains("line 1"),
            "delta output should be merged: {preview}"
        );
        assert!(
            preview.contains("line 2"),
            "delta output should be merged: {preview}"
        );
    }
}

#[test]
fn provider_change_becomes_block() {
    let records = vec![record(
        1,
        TranscriptEvent::ProviderChanged {
            from_provider: Some("anthropic".into()),
            to_provider: "openai".into(),
            from_model: Some("opus".into()),
            to_model: "gpt-4o".into(),
            reason: "rate limited".into(),
        },
    )];

    let blocks = blocks_from_records(&records);
    let changes: Vec<_> = blocks
        .iter()
        .filter(|b| matches!(b, TranscriptBlock::ProviderChange { .. }))
        .collect();
    assert_eq!(changes.len(), 1);

    if let TranscriptBlock::ProviderChange { from, to, reason } = &changes[0] {
        assert_eq!(from, "anthropic");
        assert_eq!(to, "openai");
        assert_eq!(reason, "rate limited");
    }
}

#[test]
fn usage_report_includes_cache_only_when_nonzero() {
    let records = vec![
        record(
            1,
            TranscriptEvent::Usage {
                input_tokens: 100,
                output_tokens: 50,
                cache_read_tokens: 0,
                cache_creation_tokens: 0,
                cost_usd: None,
            },
        ),
        record(
            2,
            TranscriptEvent::Usage {
                input_tokens: 200,
                output_tokens: 80,
                cache_read_tokens: 30,
                cache_creation_tokens: 10,
                cost_usd: Some(0.01),
            },
        ),
    ];

    let blocks = blocks_from_records(&records);
    let usage_blocks: Vec<_> = blocks
        .iter()
        .filter(|b| matches!(b, TranscriptBlock::UsageReport { .. }))
        .collect();
    assert_eq!(usage_blocks.len(), 2);

    if let TranscriptBlock::UsageReport { cache_tokens, .. } = &usage_blocks[0] {
        assert_eq!(*cache_tokens, None, "zero cache should be None");
    }
    if let TranscriptBlock::UsageReport { cache_tokens, .. } = &usage_blocks[1] {
        assert_eq!(*cache_tokens, Some(30), "nonzero cache should be Some");
    }
}
