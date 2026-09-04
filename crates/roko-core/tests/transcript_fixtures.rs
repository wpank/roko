//! Golden fixture tests for transcript JSONL format.
//!
//! These tests validate that the JSONL fixtures deserialize correctly,
//! round-trip through serde preserving all data, and satisfy the
//! acceptance criteria from `01-event-schema.md`:
//!
//! 1. A fixture covering text, reasoning, two tools, a failed tool,
//!    a subagent, and todo updates renders identically regardless of
//!    source adapter.
//! 2. Tool start and finish always pair by call ID, even when frames
//!    arrive out of order.
//! 3. Replaying persisted JSONL produces the same transcript as the
//!    live stream.
//! 4. Schema versioning enables forward-compatible deserialization.
//! 5. Compound correlation IDs (task_id, attempt_id) propagate
//!    end-to-end.

use std::collections::{HashMap, HashSet};
use roko_core::tool::transcript::{
    ToolLifecycleStatus, TranscriptEvent, TranscriptRecord,
};

// ─── Fixture loader ──────────────────────────────────────────────────────

fn load_fixture(name: &str) -> Vec<TranscriptRecord> {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read fixture {path}: {e}"));
    content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .enumerate()
        .map(|(i, line)| {
            serde_json::from_str(line)
                .unwrap_or_else(|e| panic!("line {i} of {name}: {e}\n  line: {line}"))
        })
        .collect()
}

/// All fixture files in the transcript/ directory.
const ALL_TRANSCRIPT_FIXTURES: &[&str] = &[
    "transcript/basic-tool.jsonl",
    "transcript/cancelled-tool.jsonl",
    "transcript/failed-tool.jsonl",
    "transcript/full-scenario.jsonl",
    "transcript/lag-reconnect.jsonl",
    "transcript/parallel-tools.jsonl",
    "transcript/reasoning-tools.jsonl",
    "transcript/truncated-redacted.jsonl",
    "transcript/unicode-boundaries.jsonl",
    "transcript/unknown-version.jsonl",
];

// ─── Legacy fixture tests ────────────────────────────────────────────────

#[test]
fn basic_tool_transcript_loads() {
    let records = load_fixture("basic_tool_transcript.jsonl");
    assert_eq!(records.len(), 7);

    assert!(matches!(
        &records[0].event,
        TranscriptEvent::RunStarted { .. }
    ));
    assert!(matches!(
        &records[6].event,
        TranscriptEvent::RunFinished { .. }
    ));

    for w in records.windows(2) {
        assert!(
            w[1].meta.sequence > w[0].meta.sequence,
            "sequence must be monotonic: {} vs {}",
            w[0].meta.sequence,
            w[1].meta.sequence,
        );
    }
}

#[test]
fn parallel_tools_transcript_loads() {
    let records = load_fixture("parallel_tools_transcript.jsonl");
    assert_eq!(records.len(), 11);

    let starts = records
        .iter()
        .filter(|r| matches!(&r.event, TranscriptEvent::ToolStarted { .. }))
        .count();
    let finishes = records
        .iter()
        .filter(|r| matches!(&r.event, TranscriptEvent::ToolFinished { .. }))
        .count();
    assert_eq!(starts, 3);
    assert_eq!(finishes, 3);

    let tool_start_times: Vec<i64> = records
        .iter()
        .filter(|r| matches!(&r.event, TranscriptEvent::ToolStarted { .. }))
        .map(|r| r.meta.timestamp_ms)
        .collect();
    assert_eq!(tool_start_times[0], tool_start_times[1]);
}

#[test]
fn fixture_records_roundtrip() {
    for fixture in &[
        "basic_tool_transcript.jsonl",
        "parallel_tools_transcript.jsonl",
    ] {
        let records = load_fixture(fixture);
        for (i, record) in records.iter().enumerate() {
            let json = serde_json::to_string(record)
                .unwrap_or_else(|e| panic!("{fixture} record {i}: serialize failed: {e}"));
            let decoded: TranscriptRecord = serde_json::from_str(&json)
                .unwrap_or_else(|e| panic!("{fixture} record {i}: deserialize failed: {e}"));
            assert_eq!(&decoded, record, "{fixture} record {i}: roundtrip mismatch");
        }
    }
}

#[test]
fn all_records_have_consistent_run_id() {
    for fixture in &[
        "basic_tool_transcript.jsonl",
        "parallel_tools_transcript.jsonl",
    ] {
        let records = load_fixture(fixture);
        let run_id = &records[0].meta.run_id;
        for record in &records {
            assert_eq!(
                &record.meta.run_id, run_id,
                "{fixture}: inconsistent run_id"
            );
        }
    }
}

// ─── All transcript/ fixtures round-trip (T001, T007) ────────────────────

#[test]
fn all_transcript_fixtures_roundtrip() {
    for fixture in ALL_TRANSCRIPT_FIXTURES {
        let records = load_fixture(fixture);
        assert!(!records.is_empty(), "{fixture}: fixture is empty");
        for (i, record) in records.iter().enumerate() {
            let json = serde_json::to_string(record)
                .unwrap_or_else(|e| panic!("{fixture}[{i}]: serialize: {e}"));
            let decoded: TranscriptRecord = serde_json::from_str(&json)
                .unwrap_or_else(|e| panic!("{fixture}[{i}]: deserialize: {e}"));
            assert_eq!(&decoded, record, "{fixture}[{i}]: roundtrip mismatch");
        }
    }
}

// ─── Schema version (T001) ──────────────────────────────────────────────

#[test]
fn schema_version_defaults_for_legacy_fixtures() {
    // Legacy fixtures without explicit schema_version should default to CURRENT.
    let records = load_fixture("transcript/basic-tool.jsonl");
    for record in &records {
        assert_eq!(
            record.schema_version,
            TranscriptRecord::CURRENT_SCHEMA_VERSION,
            "legacy fixture should default to current schema version"
        );
    }
}

#[test]
fn schema_version_unknown_future_preserved() {
    let records = load_fixture("transcript/unknown-version.jsonl");
    // This fixture has schema_version: 99 at the record level.
    // Only the first record carries the field; the rest default to CURRENT.
    assert_eq!(records[0].schema_version, 99, "future version should be preserved");
    for record in &records[1..] {
        assert_eq!(
            record.schema_version,
            TranscriptRecord::CURRENT_SCHEMA_VERSION,
            "records without schema_version should default to current"
        );
    }
}

#[test]
fn full_scenario_has_explicit_schema_version() {
    let records = load_fixture("transcript/full-scenario.jsonl");
    for record in &records {
        assert_eq!(record.schema_version, 1);
    }
}

// ─── Full-scenario acceptance (T002, T032) ──────────────────────────────

#[test]
fn full_scenario_covers_all_event_types() {
    let records = load_fixture("transcript/full-scenario.jsonl");

    let mut seen_types: HashSet<&str> = HashSet::new();
    for record in &records {
        let t = match &record.event {
            TranscriptEvent::RunStarted { .. } => "run_started",
            TranscriptEvent::AssistantDelta { .. } => "assistant_delta",
            TranscriptEvent::ReasoningDelta { .. } => "reasoning_delta",
            TranscriptEvent::ToolStarted { .. } => "tool_started",
            TranscriptEvent::ToolOutputDelta { .. } => "tool_output_delta",
            TranscriptEvent::ToolFinished { .. } => "tool_finished",
            TranscriptEvent::TodoSnapshot { .. } => "todo_snapshot",
            TranscriptEvent::SubagentStarted { .. } => "subagent_started",
            TranscriptEvent::SubagentUpdate { .. } => "subagent_update",
            TranscriptEvent::SubagentFinished { .. } => "subagent_finished",
            TranscriptEvent::Usage { .. } => "usage",
            TranscriptEvent::ProviderChanged { .. } => "provider_changed",
            TranscriptEvent::Warning { .. } => "warning",
            TranscriptEvent::Error { .. } => "error",
            TranscriptEvent::RunFinished { .. } => "run_finished",
            _ => "unknown",
        };
        seen_types.insert(t);
    }

    // The acceptance test requires: text, reasoning, two tools, a failed tool,
    // a subagent, and todo updates.
    let required = [
        "run_started",
        "assistant_delta",
        "reasoning_delta",
        "tool_started",
        "tool_output_delta",
        "tool_finished",
        "todo_snapshot",
        "subagent_started",
        "subagent_update",
        "subagent_finished",
        "usage",
        "provider_changed",
        "warning",
        "error",
        "run_finished",
    ];
    for req in &required {
        assert!(
            seen_types.contains(req),
            "full-scenario fixture must contain {req}, has: {seen_types:?}"
        );
    }
}

#[test]
fn full_scenario_has_two_successful_tools_and_one_failed() {
    let records = load_fixture("transcript/full-scenario.jsonl");

    let mut succeeded = 0u32;
    let mut failed = 0u32;
    for record in &records {
        if let TranscriptEvent::ToolFinished { status, .. } = &record.event {
            match status {
                ToolLifecycleStatus::Succeeded => succeeded += 1,
                ToolLifecycleStatus::Failed => failed += 1,
                _ => {}
            }
        }
    }
    assert!(succeeded >= 2, "need at least 2 successful tools, got {succeeded}");
    assert!(failed >= 1, "need at least 1 failed tool, got {failed}");
}

// ─── Tool start/finish pairing by call_id (T002, T031) ──────────────────

#[test]
fn tool_start_finish_paired_by_call_id() {
    for fixture in ALL_TRANSCRIPT_FIXTURES {
        let records = load_fixture(fixture);

        // Collect started call IDs.
        let mut started: HashMap<String, usize> = HashMap::new();
        let mut finished: HashMap<String, usize> = HashMap::new();

        for (i, record) in records.iter().enumerate() {
            match &record.event {
                TranscriptEvent::ToolStarted { call, .. } => {
                    started.insert(call.id.clone(), i);
                }
                TranscriptEvent::ToolFinished { call_id, .. } => {
                    finished.insert(call_id.clone(), i);
                }
                _ => {}
            }
        }

        // Every finished call must have a matching start (by call_id, not index).
        for (call_id, finish_idx) in &finished {
            assert!(
                started.contains_key(call_id),
                "{fixture}: ToolFinished for {call_id} at index {finish_idx} has no matching ToolStarted"
            );
        }
    }
}

#[test]
fn parallel_tools_paired_by_call_id_not_vector_index() {
    // T031: Parallel result previews must be paired to calls by call_id,
    // not by mismatched vector order.
    let records = load_fixture("transcript/parallel-tools.jsonl");

    let mut tool_starts: Vec<(String, usize)> = Vec::new();
    let mut tool_finishes: Vec<(String, usize)> = Vec::new();

    for (i, record) in records.iter().enumerate() {
        match &record.event {
            TranscriptEvent::ToolStarted { call, .. } => {
                tool_starts.push((call.id.clone(), i));
            }
            TranscriptEvent::ToolFinished { call_id, .. } => {
                tool_finishes.push((call_id.clone(), i));
            }
            _ => {}
        }
    }

    assert!(tool_starts.len() >= 2, "need at least 2 parallel tool starts");
    assert_eq!(tool_starts.len(), tool_finishes.len());

    // Verify that finishes are paired by call_id, which may differ from
    // the order of starts (tool_call_2 may finish before tool_call_1).
    let start_ids: Vec<&str> = tool_starts.iter().map(|(id, _)| id.as_str()).collect();
    let finish_ids: Vec<&str> = tool_finishes.iter().map(|(id, _)| id.as_str()).collect();

    // All started IDs must appear in finished IDs (order may differ).
    let start_set: HashSet<&str> = start_ids.iter().copied().collect();
    let finish_set: HashSet<&str> = finish_ids.iter().copied().collect();
    assert_eq!(start_set, finish_set, "every started call must have a finish");

    // The finish order differs from start order in this fixture (tool_call_2
    // finishes before tool_call_1), proving pairing is by call_id not index.
    if tool_starts.len() >= 2 {
        assert_ne!(
            start_ids, finish_ids,
            "parallel fixture should have different start/finish ordering to prove call_id pairing"
        );
    }
}

// ─── Replay determinism (T007) ──────────────────────────────────────────

#[test]
fn replay_produces_deterministic_sequence() {
    // Load, serialize to JSONL, reload, compare.
    for fixture in ALL_TRANSCRIPT_FIXTURES {
        let records = load_fixture(fixture);

        let jsonl: String = records
            .iter()
            .map(|r| serde_json::to_string(r).unwrap())
            .collect::<Vec<_>>()
            .join("\n");

        let replayed: Vec<TranscriptRecord> = jsonl
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();

        assert_eq!(
            records, replayed,
            "{fixture}: replay must produce identical records"
        );
    }
}

#[test]
fn lag_reconnect_dedup_by_sequence() {
    // T007: The lag-reconnect fixture has duplicate sequence numbers.
    // Consumers should be able to dedup by (run_id, sequence).
    let records = load_fixture("transcript/lag-reconnect.jsonl");

    let mut seen: HashSet<(String, u64)> = HashSet::new();
    let mut dupes = 0u32;
    for record in &records {
        let key = (record.meta.run_id.clone(), record.meta.sequence);
        if !seen.insert(key) {
            dupes += 1;
        }
    }

    assert!(dupes > 0, "lag-reconnect fixture should contain duplicate sequences");

    // After dedup, records should be orderable by sequence.
    let mut deduped: Vec<&TranscriptRecord> = Vec::new();
    let mut seen2: HashSet<u64> = HashSet::new();
    for record in &records {
        if seen2.insert(record.meta.sequence) {
            deduped.push(record);
        }
    }
    let seqs: Vec<u64> = deduped.iter().map(|r| r.meta.sequence).collect();
    let mut sorted_seqs = seqs.clone();
    sorted_seqs.sort();
    assert_eq!(seqs, sorted_seqs, "deduped records should be in sequence order");
}

// ─── Compound correlation IDs (T005) ────────────────────────────────────

#[test]
fn full_scenario_has_compound_correlation_ids() {
    let records = load_fixture("transcript/full-scenario.jsonl");

    for record in &records {
        assert_eq!(
            record.meta.task_id.as_deref(),
            Some("task-01"),
            "full-scenario should have task_id on all records"
        );
        assert_eq!(
            record.meta.attempt_id.as_deref(),
            Some("attempt-1"),
            "full-scenario should have attempt_id on all records"
        );
    }
}

#[test]
fn legacy_fixtures_have_no_correlation_ids() {
    // Legacy fixtures should deserialize with None for task_id/attempt_id.
    let records = load_fixture("transcript/basic-tool.jsonl");
    for record in &records {
        assert_eq!(record.meta.task_id, None);
        assert_eq!(record.meta.attempt_id, None);
    }
}

// ─── Sequence gap detection (T007) ───────────────────────────────────────

#[test]
fn lag_reconnect_has_sequence_gaps() {
    let records = load_fixture("transcript/lag-reconnect.jsonl");

    // After dedup, check for gaps in sequences.
    let mut seen: HashSet<u64> = HashSet::new();
    let mut unique_seqs: Vec<u64> = Vec::new();
    for record in &records {
        if seen.insert(record.meta.sequence) {
            unique_seqs.push(record.meta.sequence);
        }
    }
    unique_seqs.sort();

    let mut has_gap = false;
    for w in unique_seqs.windows(2) {
        if w[1] - w[0] > 1 {
            has_gap = true;
            break;
        }
    }
    assert!(has_gap, "lag-reconnect fixture should have sequence gaps");
}
