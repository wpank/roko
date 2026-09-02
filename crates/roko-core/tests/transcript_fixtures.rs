//! Golden fixture tests for transcript JSONL format.
//!
//! These tests validate that the JSONL fixtures deserialize correctly
//! and that round-tripping through serde preserves all data.

use roko_core::tool::transcript::{TranscriptEvent, TranscriptRecord};

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

#[test]
fn basic_tool_transcript_loads() {
    let records = load_fixture("basic_tool_transcript.jsonl");
    assert_eq!(records.len(), 7);

    // First event is RunStarted.
    assert!(matches!(
        &records[0].event,
        TranscriptEvent::RunStarted { .. }
    ));
    // Last event is RunFinished.
    assert!(matches!(
        &records[6].event,
        TranscriptEvent::RunFinished { .. }
    ));

    // Sequences are monotonically increasing.
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

    // Count tool starts and finishes.
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

    // Parallel tools have the same timestamp_ms.
    let tool_start_times: Vec<i64> = records
        .iter()
        .filter(|r| matches!(&r.event, TranscriptEvent::ToolStarted { .. }))
        .map(|r| r.meta.timestamp_ms)
        .collect();
    // First two tool starts should be at the same time (parallel).
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
