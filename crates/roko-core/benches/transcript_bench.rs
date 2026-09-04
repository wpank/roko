#![allow(missing_docs)]

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use roko_core::tool::call::{ToolCall, ToolResult};
use roko_core::tool::def::ToolCategory;
use roko_core::tool::transcript::{
    ToolLifecycleStatus, TranscriptEvent, TranscriptEventMeta, TranscriptRecord,
};

fn make_meta(seq: u64, agent_id: &str) -> TranscriptEventMeta {
    TranscriptEventMeta {
        run_id: "bench-run-001".into(),
        turn_id: 0,
        agent_id: agent_id.into(),
        sequence: seq,
        timestamp_ms: 1_700_000_000_000 + (seq as i64),
        provider: "anthropic".into(),
        model: "claude-opus-4-6".into(),
        parent_event_id: None,
    }
}

fn make_text_delta(seq: u64) -> TranscriptRecord {
    TranscriptRecord {
        meta: make_meta(seq, "agent-bench"),
        event: TranscriptEvent::AssistantDelta {
            text: "The quick brown fox jumps over the lazy dog. ".into(),
        },
    }
}

fn make_tool_pair(seq_start: u64) -> (TranscriptRecord, TranscriptRecord) {
    let call = ToolCall::at(
        format!("call-{seq_start}"),
        "read_file",
        serde_json::json!({"path": "bench.rs"}),
        1_700_000_000_000 + (seq_start as i64),
    );
    let started = TranscriptRecord {
        meta: make_meta(seq_start, "agent-bench"),
        event: TranscriptEvent::ToolStarted {
            call,
            status: ToolLifecycleStatus::Pending,
            category: Some(ToolCategory::Read),
        },
    };
    let finished = TranscriptRecord {
        meta: make_meta(seq_start + 1, "agent-bench"),
        event: TranscriptEvent::ToolFinished {
            call_id: format!("call-{seq_start}"),
            result: ToolResult::text("fn main() {}"),
            status: ToolLifecycleStatus::Succeeded,
            execution_ms: Some(42),
        },
    };
    (started, finished)
}

// --- Benchmark: ingest 100k text deltas ---

fn bench_ingest_100k_text_deltas(c: &mut Criterion) {
    // Pre-build records
    let records: Vec<TranscriptRecord> = (0..100_000).map(make_text_delta).collect();

    c.bench_function("transcript_ingest_100k_text_deltas", |bencher| {
        bencher.iter(|| {
            // Simulate ingestion: serialize each record to JSON (the hot path)
            let mut total_bytes = 0usize;
            for record in &records {
                let json = serde_json::to_string(record).expect("serialize");
                total_bytes += json.len();
            }
            black_box(total_bytes)
        });
    });
}

// --- Benchmark: ingest 1k tool start/finish pairs ---

fn bench_ingest_1k_tool_pairs(c: &mut Criterion) {
    let pairs: Vec<(TranscriptRecord, TranscriptRecord)> =
        (0..1_000).map(|i| make_tool_pair(i * 2)).collect();

    c.bench_function("transcript_ingest_1k_tool_pairs", |bencher| {
        bencher.iter(|| {
            let mut total_bytes = 0usize;
            for (started, finished) in &pairs {
                let s = serde_json::to_string(started).expect("serialize");
                let f = serde_json::to_string(finished).expect("serialize");
                total_bytes += s.len() + f.len();
            }
            black_box(total_bytes)
        });
    });
}

// --- Benchmark: deserialize + filter by agent_id across 10k events ---

fn bench_query_by_agent_id_10k(c: &mut Criterion) {
    // Build 10k records from 3 different agents
    let agents = ["agent-alpha", "agent-beta", "agent-gamma"];
    let jsonl: Vec<String> = (0..10_000u64)
        .map(|i| {
            let record = TranscriptRecord {
                meta: make_meta(i, agents[(i % 3) as usize]),
                event: TranscriptEvent::AssistantDelta {
                    text: format!("delta {i}"),
                },
            };
            serde_json::to_string(&record).expect("serialize")
        })
        .collect();

    c.bench_function("transcript_query_by_agent_id_10k", |bencher| {
        bencher.iter(|| {
            let mut count = 0usize;
            for line in &jsonl {
                let record: TranscriptRecord = serde_json::from_str(line).expect("deserialize");
                if record.meta.agent_id == "agent-alpha" {
                    count += 1;
                }
            }
            black_box(count)
        });
    });
}

// --- Benchmark: replay from sequence across 10k events ---

fn bench_replay_from_sequence_10k(c: &mut Criterion) {
    let records: Vec<TranscriptRecord> = (0..10_000u64)
        .map(|i| TranscriptRecord {
            meta: make_meta(i, "agent-bench"),
            event: TranscriptEvent::AssistantDelta {
                text: format!("delta {i}"),
            },
        })
        .collect();

    // Serialize to JSONL
    let jsonl: Vec<String> = records
        .iter()
        .map(|r| serde_json::to_string(r).expect("serialize"))
        .collect();

    // Replay from sequence 5000
    let replay_from: u64 = 5_000;

    c.bench_function("transcript_replay_from_seq_10k", |bencher| {
        bencher.iter(|| {
            let mut replayed = Vec::new();
            for line in &jsonl {
                let record: TranscriptRecord = serde_json::from_str(line).expect("deserialize");
                if record.meta.sequence >= replay_from {
                    replayed.push(record);
                }
            }
            black_box(replayed.len())
        });
    });
}

// --- Benchmark: serde roundtrip for a single TranscriptRecord ---

fn bench_transcript_record_serde_roundtrip(c: &mut Criterion) {
    let record = TranscriptRecord {
        meta: make_meta(1, "agent-bench"),
        event: TranscriptEvent::ToolFinished {
            call_id: "call-1".into(),
            result: ToolResult::text("fn main() { println!(\"hello\"); }"),
            status: ToolLifecycleStatus::Succeeded,
            execution_ms: Some(150),
        },
    };

    c.bench_function("transcript_record_serde_roundtrip", |bencher| {
        bencher.iter(|| {
            let json = serde_json::to_string(&record).expect("serialize");
            black_box(serde_json::from_str::<TranscriptRecord>(&json).expect("deserialize"))
        });
    });
}

criterion_group!(
    benches,
    bench_ingest_100k_text_deltas,
    bench_ingest_1k_tool_pairs,
    bench_query_by_agent_id_10k,
    bench_replay_from_sequence_10k,
    bench_transcript_record_serde_roundtrip,
);
criterion_main!(benches);
