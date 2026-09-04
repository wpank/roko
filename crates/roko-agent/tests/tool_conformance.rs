//! T016: Tool conformance tests verifying that all tool categories produce
//! consistent `TranscriptEvent` records.
//!
//! Each tool category (Read, Write, Exec, Git, Network, Meta, Notebook, Mcp)
//! must produce a matching `ToolStarted` + `ToolFinished` pair with correct
//! lifecycle statuses and consistent `call_id` threading.

use roko_core::tool::call::{ToolCall, ToolError, ToolResult};
use roko_core::tool::def::ToolCategory;
use roko_core::tool::transcript::{
    ToolLifecycleStatus, TranscriptEvent, TranscriptEventMeta, TranscriptRecord,
};

// ── Helpers ──────────────────────────────────────────────────────────────────

fn meta_at(seq: u64) -> TranscriptEventMeta {
    TranscriptEventMeta {
        run_id: "conformance-run".into(),
        turn_id: 0,
        agent_id: "conformance-agent".into(),
        sequence: seq,
        timestamp_ms: 1_700_000_000_000 + (seq as i64 * 100),
        provider: "test".into(),
        model: "test-model".into(),
        parent_event_id: None,
        task_id: Some("task-conformance".into()),
        attempt_id: Some("attempt-0".into()),
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

/// Build a ToolStarted + ToolFinished pair for a given category and terminal status.
fn make_tool_pair(
    category: ToolCategory,
    call_id: &str,
    tool_name: &str,
    terminal_status: ToolLifecycleStatus,
    result: ToolResult,
    base_seq: u64,
) -> (TranscriptRecord, TranscriptRecord) {
    let started = TranscriptRecord {
        schema_version: TranscriptRecord::CURRENT_SCHEMA_VERSION,
        meta: meta_at(base_seq),
        event: TranscriptEvent::ToolStarted {
            call: make_call(call_id, tool_name),
            status: ToolLifecycleStatus::Admitted,
            category: Some(category),
        },
    };
    let finished = TranscriptRecord {
        schema_version: TranscriptRecord::CURRENT_SCHEMA_VERSION,
        meta: meta_at(base_seq + 1),
        event: TranscriptEvent::ToolFinished {
            call_id: call_id.to_string(),
            result,
            status: terminal_status,
            execution_ms: Some(10),
        },
    };
    (started, finished)
}

/// Assert that a ToolStarted event has the expected category and call_id.
fn assert_started(record: &TranscriptRecord, expected_category: ToolCategory, expected_id: &str) {
    match &record.event {
        TranscriptEvent::ToolStarted {
            call, category, status, ..
        } => {
            assert_eq!(call.id, expected_id, "call_id mismatch in ToolStarted");
            assert_eq!(
                *category,
                Some(expected_category),
                "category mismatch in ToolStarted for call {expected_id}"
            );
            assert!(
                !status.is_terminal(),
                "ToolStarted must have non-terminal status"
            );
        }
        other => panic!("expected ToolStarted, got {other:?}"),
    }
}

/// Assert that a ToolFinished event has the expected terminal status and call_id.
fn assert_finished(
    record: &TranscriptRecord,
    expected_id: &str,
    expected_status: ToolLifecycleStatus,
) {
    match &record.event {
        TranscriptEvent::ToolFinished {
            call_id, status, ..
        } => {
            assert_eq!(call_id, expected_id, "call_id mismatch in ToolFinished");
            assert_eq!(
                *status, expected_status,
                "terminal status mismatch in ToolFinished for call {expected_id}"
            );
            assert!(
                status.is_terminal(),
                "ToolFinished must have terminal status"
            );
        }
        other => panic!("expected ToolFinished, got {other:?}"),
    }
}

// ── Category completeness ────────────────────────────────────────────────────

/// Every ToolCategory variant produces a conformant ToolStarted/ToolFinished pair.
#[test]
fn every_tool_category_produces_conformant_started_finished_pair() {
    let categories = [
        (ToolCategory::Read, "read_call", "read_file"),
        (ToolCategory::Write, "write_call", "write_file"),
        (ToolCategory::Exec, "exec_call", "bash"),
        (ToolCategory::Git, "git_call", "git_commit"),
        (ToolCategory::Network, "net_call", "http_get"),
        (ToolCategory::Meta, "meta_call", "agent_delegate"),
        (ToolCategory::Notebook, "nb_call", "notebook_edit"),
        (ToolCategory::Mcp, "mcp_call", "mcp_tool"),
    ];

    for (seq_base, (category, call_id, tool_name)) in categories.iter().enumerate() {
        let (started, finished) = make_tool_pair(
            *category,
            call_id,
            tool_name,
            ToolLifecycleStatus::Succeeded,
            ToolResult::text("ok"),
            (seq_base as u64) * 2 + 1,
        );

        assert_started(&started, *category, call_id);
        assert_finished(&finished, call_id, ToolLifecycleStatus::Succeeded);

        // Serde roundtrip must be deterministic.
        let started_json = serde_json::to_string(&started).unwrap();
        let finished_json = serde_json::to_string(&finished).unwrap();
        let started_rt: TranscriptRecord = serde_json::from_str(&started_json).unwrap();
        let finished_rt: TranscriptRecord = serde_json::from_str(&finished_json).unwrap();
        assert_eq!(started_rt, started, "started roundtrip for {category:?}");
        assert_eq!(finished_rt, finished, "finished roundtrip for {category:?}");
    }
}

// ── Terminal status consistency ───────────────────────────────────────────────

/// Each terminal ToolLifecycleStatus is representable in ToolFinished events.
#[test]
fn all_terminal_statuses_produce_valid_finished_events() {
    let terminal_statuses = [
        (ToolLifecycleStatus::Succeeded, ToolResult::text("done")),
        (
            ToolLifecycleStatus::Failed,
            ToolResult::err(ToolError::Other("handler error".into())),
        ),
        (
            ToolLifecycleStatus::TimedOut,
            ToolResult::err(ToolError::Timeout { after_ms: 30_000 }),
        ),
        (
            ToolLifecycleStatus::Cancelled,
            ToolResult::err(ToolError::Cancelled),
        ),
        (
            ToolLifecycleStatus::Denied,
            ToolResult::err(ToolError::PermissionDenied(
                "bash: missing exec".into(),
            )),
        ),
        (
            ToolLifecycleStatus::Panicked,
            ToolResult::err(ToolError::Other("panic caught".into())),
        ),
        (
            ToolLifecycleStatus::Interrupted,
            ToolResult::err(ToolError::Other("SIGINT".into())),
        ),
    ];

    for (idx, (status, result)) in terminal_statuses.iter().enumerate() {
        let call_id = format!("terminal-{idx}");
        let (_, finished) = make_tool_pair(
            ToolCategory::Exec,
            &call_id,
            "bash",
            *status,
            result.clone(),
            (idx as u64) * 2 + 100,
        );

        assert_finished(&finished, &call_id, *status);
        assert!(
            status.is_terminal(),
            "{status:?} must be classified as terminal"
        );

        // Roundtrip preserves the status.
        let json = serde_json::to_string(&finished).unwrap();
        let rt: TranscriptRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(rt, finished, "roundtrip for terminal status {status:?}");
    }
}

// ── call_id threading ────────────────────────────────────────────────────────

/// The call_id in ToolStarted must match the call_id in ToolFinished.
#[test]
fn call_id_is_threaded_from_started_to_finished() {
    let call_ids = ["call-alpha", "call-beta", "call-gamma"];

    for (idx, call_id) in call_ids.iter().enumerate() {
        let (started, finished) = make_tool_pair(
            ToolCategory::Read,
            call_id,
            "read_file",
            ToolLifecycleStatus::Succeeded,
            ToolResult::text("contents"),
            (idx as u64) * 2 + 200,
        );

        let started_call_id = match &started.event {
            TranscriptEvent::ToolStarted { call, .. } => call.id.clone(),
            _ => panic!("expected ToolStarted"),
        };
        let finished_call_id = match &finished.event {
            TranscriptEvent::ToolFinished { call_id, .. } => call_id.clone(),
            _ => panic!("expected ToolFinished"),
        };

        assert_eq!(
            started_call_id, finished_call_id,
            "call_id must be consistent between started and finished"
        );
    }
}

// ── Sequence ordering ────────────────────────────────────────────────────────

/// ToolStarted always has a lower sequence than its paired ToolFinished.
#[test]
fn started_precedes_finished_in_sequence_order() {
    let (started, finished) = make_tool_pair(
        ToolCategory::Write,
        "call-ordering",
        "write_file",
        ToolLifecycleStatus::Succeeded,
        ToolResult::text("written"),
        300,
    );

    assert!(
        started.meta.sequence < finished.meta.sequence,
        "started.sequence ({}) must be < finished.sequence ({})",
        started.meta.sequence,
        finished.meta.sequence
    );
}

// ── ToolOutputDelta call_id consistency ───────────────────────────────────────

/// ToolOutputDelta events reference the same call_id as the enclosing pair.
#[test]
fn tool_output_delta_references_correct_call_id() {
    let call_id = "call-delta";

    let started = TranscriptRecord {
        schema_version: TranscriptRecord::CURRENT_SCHEMA_VERSION,
        meta: meta_at(400),
        event: TranscriptEvent::ToolStarted {
            call: make_call(call_id, "bash"),
            status: ToolLifecycleStatus::Admitted,
            category: Some(ToolCategory::Exec),
        },
    };
    let delta = TranscriptRecord {
        schema_version: TranscriptRecord::CURRENT_SCHEMA_VERSION,
        meta: meta_at(401),
        event: TranscriptEvent::ToolOutputDelta {
            call_id: call_id.to_string(),
            text: "partial output".into(),
        },
    };
    let finished = TranscriptRecord {
        schema_version: TranscriptRecord::CURRENT_SCHEMA_VERSION,
        meta: meta_at(402),
        event: TranscriptEvent::ToolFinished {
            call_id: call_id.to_string(),
            result: ToolResult::text("full output"),
            status: ToolLifecycleStatus::Succeeded,
            execution_ms: Some(50),
        },
    };

    // Verify call_id threading across all three records.
    let started_id = match &started.event {
        TranscriptEvent::ToolStarted { call, .. } => &call.id,
        _ => panic!("expected ToolStarted"),
    };
    let delta_id = match &delta.event {
        TranscriptEvent::ToolOutputDelta { call_id, .. } => call_id,
        _ => panic!("expected ToolOutputDelta"),
    };
    let finished_id = match &finished.event {
        TranscriptEvent::ToolFinished { call_id, .. } => call_id,
        _ => panic!("expected ToolFinished"),
    };

    assert_eq!(started_id, delta_id);
    assert_eq!(delta_id, finished_id);

    // Ordering: started < delta < finished.
    assert!(started.meta.sequence < delta.meta.sequence);
    assert!(delta.meta.sequence < finished.meta.sequence);
}

// ── JSONL roundtrip for mixed-category transcript ────────────────────────────

/// A transcript containing all categories roundtrips through JSONL deterministically.
#[test]
fn mixed_category_transcript_jsonl_roundtrip() {
    let categories = [
        ToolCategory::Read,
        ToolCategory::Write,
        ToolCategory::Exec,
        ToolCategory::Git,
        ToolCategory::Network,
        ToolCategory::Meta,
        ToolCategory::Notebook,
        ToolCategory::Mcp,
    ];

    let mut records = Vec::new();
    for (idx, cat) in categories.iter().enumerate() {
        let call_id = format!("mixed-{idx}");
        let (started, finished) = make_tool_pair(
            *cat,
            &call_id,
            "tool",
            ToolLifecycleStatus::Succeeded,
            ToolResult::text("ok"),
            (idx as u64) * 2 + 500,
        );
        records.push(started);
        records.push(finished);
    }

    let jsonl: String = records
        .iter()
        .map(|r| serde_json::to_string(r).unwrap())
        .collect::<Vec<_>>()
        .join("\n");

    let decoded: Vec<TranscriptRecord> = jsonl
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();

    assert_eq!(decoded.len(), records.len());
    assert_eq!(decoded, records, "JSONL roundtrip must be deterministic");
}

// ── Correlation consistency ──────────────────────────────────────────────────

/// All records in a tool pair share the same correlation fields.
#[test]
fn tool_pair_shares_correlation_fields() {
    let (started, finished) = make_tool_pair(
        ToolCategory::Read,
        "call-cor",
        "read_file",
        ToolLifecycleStatus::Succeeded,
        ToolResult::text("contents"),
        600,
    );

    assert_eq!(started.meta.run_id, finished.meta.run_id);
    assert_eq!(started.meta.agent_id, finished.meta.agent_id);
    assert_eq!(started.meta.task_id, finished.meta.task_id);
    assert_eq!(started.meta.attempt_id, finished.meta.attempt_id);
    assert_eq!(started.meta.provider, finished.meta.provider);
    assert_eq!(started.meta.model, finished.meta.model);
}

// ── Category discriminant stability ──────────────────────────────────────────

/// ToolCategory serde names are stable strings used by downstream consumers.
#[test]
fn tool_category_serde_names_are_stable() {
    let cases = [
        (ToolCategory::Read, "\"read\""),
        (ToolCategory::Write, "\"write\""),
        (ToolCategory::Exec, "\"exec\""),
        (ToolCategory::Git, "\"git\""),
        (ToolCategory::Network, "\"network\""),
        (ToolCategory::Meta, "\"meta\""),
        (ToolCategory::Notebook, "\"notebook\""),
        (ToolCategory::Mcp, "\"mcp\""),
    ];

    for (cat, expected_json) in cases {
        let json = serde_json::to_string(&cat).unwrap();
        assert_eq!(
            json, expected_json,
            "serde name for {cat:?} must be stable"
        );
        let rt: ToolCategory = serde_json::from_str(&json).unwrap();
        assert_eq!(rt, cat);
    }
}
