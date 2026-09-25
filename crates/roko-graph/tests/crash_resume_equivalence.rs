//! P0-GE-1: Crash/resume equivalence test harness for the Graph engine.
//!
//! This file proves a core durability invariant:
//!
//!   After a crash mid-execution, resuming from an Activity checkpoint
//!   must produce **identical final state** to a clean full run.
//!
//! # Test structure
//!
//! The harness builds a 3-node Activity DAG with linear dependencies:
//!
//!   task_a  →  task_b  →  task_c
//!
//! Each node is a deterministic mock cell that:
//! - Writes a unique tag signal ("output_a", "output_b", "output_c").
//! - Increments a shared atomic counter so we can detect re-executions.
//!
//! Crash simulation: a `CrashAtB` cell variant returns an error on its
//! first invocation, simulating a mid-run crash after `task_a` has
//! already completed and been persisted to the Activity recorder, but
//! before `task_b` finishes.
//!
//! Resume path: the test loads the partially-filled JSONL checkpoint into
//! an `ActivityReplayer`, re-attaches it to a fresh engine, and runs
//! again.  The replayer substitutes `task_a`'s recorded output without
//! re-executing the cell, then executes `task_b` and `task_c` normally.
//!
//! Assertions (the equivalence property):
//! 1. `task_a` is NOT re-executed after resume (counter stays at 1).
//! 2. `task_b` IS retried after resume (counter goes from 0 to 1).
//! 3. `task_c` executes once in both runs.
//! 4. The final output signals from the resumed run match those of a
//!    clean full run of the same plan.

use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};

use async_trait::async_trait;
use roko_core::{Body, Kind, Signal, error::Result as CoreResult};
use roko_graph::{
    CellContext, CellRegistry,
    cell::Cell,
    engine::GraphEngine,
    registry::CellDescriptor,
    replay::{ActivityRecorder, ActivityReplayer},
    types::{Edge, ExecutionClass, FailureStrategy, Graph, GraphMetadata, GraphPolicy, Node},
};
use tempfile::NamedTempFile;

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn signal(tag: &str) -> Signal {
    Signal::builder(Kind::Task).body(Body::text(tag)).build()
}

fn empty_config() -> toml::Value {
    toml::Value::Table(toml::map::Map::new())
}

fn activity_node(id: &str, cell_type: &str) -> Node {
    Node {
        id: id.to_string(),
        cell_type: cell_type.to_string(),
        config: empty_config(),
        inputs: Vec::new(),
        outputs: Vec::new(),
        execution_class: ExecutionClass::Activity,
    }
}

/// Build a linear 3-node Activity graph: task_a → task_b → task_c.
fn build_linear_graph(failure_strategy: FailureStrategy) -> Graph {
    let mut g = Graph::new(GraphMetadata {
        name: "crash-resume-test".to_string(),
        ..Default::default()
    });
    g.policy = GraphPolicy {
        failure_strategy,
        max_concurrent_nodes: 1, // sequential so ordering is deterministic
        ..Default::default()
    };
    g.add_node(activity_node("task_a", "mock.a")).unwrap();
    g.add_node(activity_node("task_b", "mock.b")).unwrap();
    g.add_node(activity_node("task_c", "mock.c")).unwrap();
    g.add_edge(Edge {
        from: "task_a".into(),
        to: "task_b".into(),
        condition: None,
    })
    .unwrap();
    g.add_edge(Edge {
        from: "task_b".into(),
        to: "task_c".into(),
        condition: None,
    })
    .unwrap();
    g
}

// ─── Mock cells ───────────────────────────────────────────────────────────────

/// A simple mock cell that succeeds immediately, emitting a tagged signal
/// and incrementing `call_count`.
struct MockCell {
    id: String,
    tag: String,
    call_count: Arc<AtomicU32>,
}

impl MockCell {
    fn new(id: &str, tag: &str, counter: Arc<AtomicU32>) -> Self {
        Self {
            id: id.to_string(),
            tag: tag.to_string(),
            call_count: counter,
        }
    }
}

#[async_trait]
impl Cell for MockCell {
    fn cell_id(&self) -> &str {
        &self.id
    }

    fn cell_name(&self) -> &str {
        &self.id
    }

    async fn execute(&self, _input: Vec<Signal>, _ctx: &CellContext) -> CoreResult<Vec<Signal>> {
        self.call_count.fetch_add(1, Ordering::Relaxed);
        Ok(vec![signal(&self.tag)])
    }
}

/// A cell that fails on the first invocation to simulate a mid-run crash,
/// and succeeds on subsequent invocations.
struct CrashFirstCell {
    id: String,
    tag: String,
    call_count: Arc<AtomicU32>,
}

impl CrashFirstCell {
    fn new(id: &str, tag: &str, counter: Arc<AtomicU32>) -> Self {
        Self {
            id: id.to_string(),
            tag: tag.to_string(),
            call_count: counter,
        }
    }
}

#[async_trait]
impl Cell for CrashFirstCell {
    fn cell_id(&self) -> &str {
        &self.id
    }

    fn cell_name(&self) -> &str {
        &self.id
    }

    async fn execute(&self, _input: Vec<Signal>, _ctx: &CellContext) -> CoreResult<Vec<Signal>> {
        let prev = self.call_count.fetch_add(1, Ordering::Relaxed);
        if prev == 0 {
            Err(roko_core::error::RokoError::invalid(
                "simulated crash at task_b (first call)",
            ))
        } else {
            Ok(vec![signal(&self.tag)])
        }
    }
}

// ─── Registry builders ────────────────────────────────────────────────────────

/// Registry for the clean (no-crash) full run.
/// Each node succeeds on every invocation.
fn full_run_registry(
    count_a: Arc<AtomicU32>,
    count_b: Arc<AtomicU32>,
    count_c: Arc<AtomicU32>,
) -> CellRegistry {
    let mut reg = CellRegistry::new();

    let ca = count_a.clone();
    reg.register_with_descriptor(
        "mock.a",
        CellDescriptor::test_stub("mock.a"),
        move |_config| Box::new(MockCell::new("mock.a", "output_a", ca.clone())),
    );

    let cb = count_b.clone();
    reg.register_with_descriptor(
        "mock.b",
        CellDescriptor::test_stub("mock.b"),
        move |_config| Box::new(MockCell::new("mock.b", "output_b", cb.clone())),
    );

    let cc = count_c.clone();
    reg.register_with_descriptor(
        "mock.c",
        CellDescriptor::test_stub("mock.c"),
        move |_config| Box::new(MockCell::new("mock.c", "output_c", cc.clone())),
    );

    reg
}

/// Registry for the crash run: task_b crashes on first call.
fn crash_run_registry(
    count_a: Arc<AtomicU32>,
    count_b: Arc<AtomicU32>,
    count_c: Arc<AtomicU32>,
) -> CellRegistry {
    let mut reg = CellRegistry::new();

    let ca = count_a.clone();
    reg.register_with_descriptor(
        "mock.a",
        CellDescriptor::test_stub("mock.a"),
        move |_config| Box::new(MockCell::new("mock.a", "output_a", ca.clone())),
    );

    let cb = count_b.clone();
    reg.register_with_descriptor(
        "mock.b",
        CellDescriptor::test_stub("mock.b"),
        move |_config| Box::new(CrashFirstCell::new("mock.b", "output_b", cb.clone())),
    );

    let cc = count_c.clone();
    reg.register_with_descriptor(
        "mock.c",
        CellDescriptor::test_stub("mock.c"),
        move |_config| Box::new(MockCell::new("mock.c", "output_c", cc.clone())),
    );

    reg
}

/// Registry for the resume run: all nodes succeed (task_b is no longer crashing).
fn resume_run_registry(
    count_a: Arc<AtomicU32>,
    count_b: Arc<AtomicU32>,
    count_c: Arc<AtomicU32>,
) -> CellRegistry {
    // After the crash, we want task_a to be replayed from the checkpoint
    // and task_b / task_c to be re-executed normally.
    full_run_registry(count_a, count_b, count_c)
}

// ─── Tests ────────────────────────────────────────────────────────────────────

/// Core equivalence test:
///
/// 1. Run a graph to completion cleanly, recording all Activity outputs.
/// 2. Simulate a crash: run the same graph again with task_b crashing,
///    but with a fresh recorder that captures task_a before the crash.
/// 3. Resume from the crash checkpoint.
/// 4. Assert equivalence: final output signals match the clean run, and
///    task_a was not re-executed (its count stays at 1 from the crash run).
#[tokio::test]
async fn crash_resume_produces_identical_final_state() {
    // ── Phase 0: clean full run to establish expected final state ─────────────

    let clean_count_a = Arc::new(AtomicU32::new(0));
    let clean_count_b = Arc::new(AtomicU32::new(0));
    let clean_count_c = Arc::new(AtomicU32::new(0));

    let clean_recording = NamedTempFile::new().expect("tempfile for clean recording");
    let clean_recorder = ActivityRecorder::create_fresh("clean-run", clean_recording.path())
        .expect("create clean recorder");

    let clean_graph = build_linear_graph(FailureStrategy::SkipFailed);
    let clean_registry = full_run_registry(
        clean_count_a.clone(),
        clean_count_b.clone(),
        clean_count_c.clone(),
    );

    let clean_engine = GraphEngine::new(clean_graph, clean_registry)
        .with_allow_test_stubs(true)
        .with_recorder(clean_recorder);

    let ctx = CellContext::new().with_run_id("clean-run".to_string());
    let clean_output = clean_engine
        .execute(&ctx)
        .await
        .expect("clean run must succeed");

    assert!(clean_output.success, "clean run must succeed");
    assert_eq!(clean_count_a.load(Ordering::Relaxed), 1, "task_a ran once");
    assert_eq!(clean_count_b.load(Ordering::Relaxed), 1, "task_b ran once");
    assert_eq!(clean_count_c.load(Ordering::Relaxed), 1, "task_c ran once");

    // Collect the clean run's final outputs per node for comparison later.
    let clean_node_outputs: std::collections::HashMap<String, Vec<Signal>> = {
        let replayer =
            ActivityReplayer::load(clean_recording.path()).expect("load clean recording");
        // The replayer stores by (node_id, tick); extract tick 0 for each node.
        [
            ("task_a".to_string(), replayer.lookup("task_a", 0).cloned()),
            ("task_b".to_string(), replayer.lookup("task_b", 0).cloned()),
            ("task_c".to_string(), replayer.lookup("task_c", 0).cloned()),
        ]
        .into_iter()
        .filter_map(|(k, v)| v.map(|sigs| (k, sigs)))
        .collect()
    };

    assert_eq!(
        clean_node_outputs.len(),
        3,
        "clean run must record all 3 Activity nodes"
    );

    // ── Phase 1: crash run – task_a succeeds, task_b crashes ─────────────────

    let crash_count_a = Arc::new(AtomicU32::new(0));
    let crash_count_b = Arc::new(AtomicU32::new(0));
    let crash_count_c = Arc::new(AtomicU32::new(0));

    let crash_recording = NamedTempFile::new().expect("tempfile for crash recording");
    let crash_recorder = ActivityRecorder::create_fresh("crash-run", crash_recording.path())
        .expect("create crash recorder");

    let crash_graph = build_linear_graph(FailureStrategy::SkipFailed);
    let crash_registry = crash_run_registry(
        crash_count_a.clone(),
        crash_count_b.clone(),
        crash_count_c.clone(),
    );

    let crash_engine = GraphEngine::new(crash_graph, crash_registry)
        .with_allow_test_stubs(true)
        .with_recorder(crash_recorder);

    let ctx = CellContext::new().with_run_id("crash-run".to_string());
    let crash_output = crash_engine.execute(&ctx).await.expect("execute crash run");

    // task_a succeeded and was recorded; task_b failed; task_c was skipped.
    assert!(
        !crash_output.success,
        "crash run must not report success because task_b failed"
    );

    // Verify exactly what was executed before the crash.
    assert_eq!(
        crash_count_a.load(Ordering::Relaxed),
        1,
        "task_a must have run once before the crash"
    );
    assert_eq!(
        crash_count_b.load(Ordering::Relaxed),
        1,
        "task_b must have been attempted once (the crashing call)"
    );
    assert_eq!(
        crash_count_c.load(Ordering::Relaxed),
        0,
        "task_c must not have run (upstream task_b failed)"
    );

    // The crash recording must contain task_a's output (persisted before crash),
    // but NOT task_b's (it crashed before returning success).
    {
        let partial_replay =
            ActivityReplayer::load(crash_recording.path()).expect("load crash recording");
        assert_eq!(
            partial_replay.entry_count(),
            1,
            "crash checkpoint must contain exactly 1 entry (task_a)"
        );
        assert!(
            partial_replay.lookup("task_a", 0).is_some(),
            "crash checkpoint must contain task_a output"
        );
        assert!(
            partial_replay.lookup("task_b", 0).is_none(),
            "crash checkpoint must NOT contain task_b output (it crashed)"
        );
    }

    // ── Phase 2: resume from crash checkpoint ────────────────────────────────

    // Fresh counters – we want to count only resume-run executions.
    let resume_count_a = Arc::new(AtomicU32::new(0));
    let resume_count_b = Arc::new(AtomicU32::new(0));
    let resume_count_c = Arc::new(AtomicU32::new(0));

    // Load the partial (crash) checkpoint for replay.
    let replayer =
        ActivityReplayer::load(crash_recording.path()).expect("load crash checkpoint for resume");

    // New recording for the resume run.
    let resume_recording = NamedTempFile::new().expect("tempfile for resume recording");
    let resume_recorder = ActivityRecorder::create_fresh("resume-run", resume_recording.path())
        .expect("create resume recorder");

    let resume_graph = build_linear_graph(FailureStrategy::SkipFailed);
    let resume_registry = resume_run_registry(
        resume_count_a.clone(),
        resume_count_b.clone(),
        resume_count_c.clone(),
    );

    let resume_engine = GraphEngine::new(resume_graph, resume_registry)
        .with_allow_test_stubs(true)
        .with_replayer(replayer)
        .with_recorder(resume_recorder);

    let ctx = CellContext::new().with_run_id("resume-run".to_string());
    let resume_output = resume_engine
        .execute(&ctx)
        .await
        .expect("resume run must succeed");

    // ── Phase 3: assert equivalence ───────────────────────────────────────────

    // The resumed run must complete successfully.
    assert!(
        resume_output.success,
        "resumed run must succeed; node_results: {:#?}",
        resume_output.node_results
    );

    // P0-GE-1 Property 1: task_a must NOT be re-executed – the replayer
    // substituted its recorded output from the crash checkpoint.
    assert_eq!(
        resume_count_a.load(Ordering::Relaxed),
        0,
        "P0-GE-1[1]: task_a must NOT be re-executed after resume (output replayed from checkpoint)"
    );

    // P0-GE-1 Property 2: task_b MUST be retried in the resume run.
    assert_eq!(
        resume_count_b.load(Ordering::Relaxed),
        1,
        "P0-GE-1[2]: task_b must be retried exactly once during resume"
    );

    // P0-GE-1 Property 3: task_c executes once in the resume run.
    assert_eq!(
        resume_count_c.load(Ordering::Relaxed),
        1,
        "P0-GE-1[3]: task_c must execute exactly once during resume"
    );

    // P0-GE-1 Property 4: final outputs must match the clean run.
    //
    // The resume run only re-records nodes that were re-executed (task_b, task_c).
    // task_a's output was replayed from the crash checkpoint and is NOT re-recorded
    // by the engine (replayed outputs are substituted, not written to the new recorder).
    //
    // The complete final state is:
    //   crash_checkpoint: task_a (carried forward from crash)
    //   resume_recording: task_b, task_c (re-executed during resume)
    //
    // We merge both to reconstruct the full final state and compare against the clean run.
    let resume_new_recording =
        ActivityReplayer::load(resume_recording.path()).expect("load resume recording");
    let crash_checkpoint =
        ActivityReplayer::load(crash_recording.path()).expect("reload crash checkpoint");

    // The resume run's new recorder must contain exactly task_b and task_c.
    assert_eq!(
        resume_new_recording.entry_count(),
        2,
        "resume recording must contain exactly 2 newly-executed nodes (task_b, task_c)"
    );
    assert!(
        resume_new_recording.lookup("task_a", 0).is_none(),
        "task_a must NOT appear in the resume recording (it was replayed, not re-executed)"
    );
    assert!(
        resume_new_recording.lookup("task_b", 0).is_some(),
        "task_b must appear in the resume recording (it was re-executed)"
    );
    assert!(
        resume_new_recording.lookup("task_c", 0).is_some(),
        "task_c must appear in the resume recording (it was re-executed)"
    );

    // Merge: prefer the resume recorder's outputs for re-executed nodes; fall
    // back to the crash checkpoint for replayed nodes (task_a).
    let merge_outputs = |node_id: &str| -> Vec<Signal> {
        // Prefer freshly-recorded output (from re-execution during resume).
        if let Some(sigs) = resume_new_recording.lookup(node_id, 0) {
            return sigs.clone();
        }
        // Fall back to the crash checkpoint (replayed, not re-executed).
        crash_checkpoint
            .lookup(node_id, 0)
            .cloned()
            .unwrap_or_default()
    };

    for node_id in ["task_a", "task_b", "task_c"] {
        let clean_sigs = clean_node_outputs
            .get(node_id)
            .unwrap_or_else(|| panic!("clean run missing output for {node_id}"));
        let resume_sigs = merge_outputs(node_id);

        assert_eq!(
            clean_sigs.len(),
            resume_sigs.len(),
            "P0-GE-1[4]: signal count mismatch for {node_id}: clean={}, resume={}",
            clean_sigs.len(),
            resume_sigs.len()
        );

        // Compare body text for each signal (deterministic mock cells produce
        // the same content every time, so the texts must match exactly).
        for (i, (clean, resumed)) in clean_sigs.iter().zip(resume_sigs.iter()).enumerate() {
            let clean_text = clean.body.as_text().unwrap_or("");
            let resumed_text = resumed.body.as_text().unwrap_or("");
            assert_eq!(
                clean_text, resumed_text,
                "P0-GE-1[4]: signal[{i}] body mismatch for {node_id}: \
                 clean={clean_text:?}, resume={resumed_text:?}"
            );
        }
    }
}

/// Verify that a fully-completed checkpoint (all 3 nodes recorded) causes
/// none of the nodes to re-execute on a subsequent replay pass.
///
/// This is the "all already done" resume case: the entire run is replayed
/// from the checkpoint without invoking any real cells.
#[tokio::test]
async fn full_checkpoint_replays_without_any_reexecution() {
    // Record a complete run.
    let recording = NamedTempFile::new().expect("tempfile");
    {
        let count_a = Arc::new(AtomicU32::new(0));
        let count_b = Arc::new(AtomicU32::new(0));
        let count_c = Arc::new(AtomicU32::new(0));

        let recorder =
            ActivityRecorder::create_fresh("full-run", recording.path()).expect("create recorder");

        let engine = GraphEngine::new(
            build_linear_graph(FailureStrategy::FailFast),
            full_run_registry(count_a, count_b, count_c),
        )
        .with_allow_test_stubs(true)
        .with_recorder(recorder);

        let output = engine
            .execute(&CellContext::new().with_run_id("full-run".to_string()))
            .await
            .expect("execute");

        assert!(output.success, "initial run must succeed");
    }

    // Replay the complete run – every Activity node should be substituted.
    let replay_count_a = Arc::new(AtomicU32::new(0));
    let replay_count_b = Arc::new(AtomicU32::new(0));
    let replay_count_c = Arc::new(AtomicU32::new(0));

    let replayer = ActivityReplayer::load(recording.path()).expect("load replayer");
    assert_eq!(replayer.entry_count(), 3, "must have 3 checkpoint entries");

    let engine = GraphEngine::new(
        build_linear_graph(FailureStrategy::FailFast),
        resume_run_registry(
            replay_count_a.clone(),
            replay_count_b.clone(),
            replay_count_c.clone(),
        ),
    )
    .with_allow_test_stubs(true)
    .with_replayer(replayer);

    let output = engine
        .execute(&CellContext::new().with_run_id("replay-run".to_string()))
        .await
        .expect("replay");

    assert!(output.success, "replay run must succeed");

    // No cells should have been called – all outputs come from the checkpoint.
    assert_eq!(
        replay_count_a.load(Ordering::Relaxed),
        0,
        "task_a must not be called during full-checkpoint replay"
    );
    assert_eq!(
        replay_count_b.load(Ordering::Relaxed),
        0,
        "task_b must not be called during full-checkpoint replay"
    );
    assert_eq!(
        replay_count_c.load(Ordering::Relaxed),
        0,
        "task_c must not be called during full-checkpoint replay"
    );
}

/// Verify that running without a checkpoint (no replayer) executes all
/// nodes exactly once, establishing the baseline for the equivalence claim.
#[tokio::test]
async fn clean_run_executes_every_node_exactly_once() {
    let count_a = Arc::new(AtomicU32::new(0));
    let count_b = Arc::new(AtomicU32::new(0));
    let count_c = Arc::new(AtomicU32::new(0));

    let engine = GraphEngine::new(
        build_linear_graph(FailureStrategy::FailFast),
        full_run_registry(count_a.clone(), count_b.clone(), count_c.clone()),
    )
    .with_allow_test_stubs(true);

    let output = engine.execute(&CellContext::new()).await.expect("execute");

    assert!(output.success, "clean run must succeed");
    assert_eq!(count_a.load(Ordering::Relaxed), 1, "task_a called once");
    assert_eq!(count_b.load(Ordering::Relaxed), 1, "task_b called once");
    assert_eq!(count_c.load(Ordering::Relaxed), 1, "task_c called once");

    // All 3 nodes appear in results as Complete.
    assert_eq!(output.node_results.len(), 3);
    for result in &output.node_results {
        assert_eq!(
            result.status,
            roko_graph::NodeStatus::Complete,
            "node {} must be Complete in clean run",
            result.node_id
        );
    }
}

/// Verify that the crash checkpoint contains only the output of nodes that
/// successfully completed before the crash, not the crashing node's output.
///
/// This is the atomicity property: the recorder flushes after each successful
/// Activity node, so a crash mid-node leaves the checkpoint in the state just
/// before that node started.
#[tokio::test]
async fn checkpoint_atomicity_crash_node_not_recorded() {
    let count_a = Arc::new(AtomicU32::new(0));
    let count_b = Arc::new(AtomicU32::new(0));
    let count_c = Arc::new(AtomicU32::new(0));

    let recording = NamedTempFile::new().expect("tempfile");
    let recorder =
        ActivityRecorder::create_fresh("atomicity-run", recording.path()).expect("recorder");

    let engine = GraphEngine::new(
        build_linear_graph(FailureStrategy::SkipFailed),
        crash_run_registry(count_a, count_b, count_c),
    )
    .with_allow_test_stubs(true)
    .with_recorder(recorder);

    let output = engine
        .execute(&CellContext::new().with_run_id("atomicity-run".to_string()))
        .await
        .expect("execute");

    // The run does not succeed because task_b fails.
    assert!(!output.success);

    // Load checkpoint and verify only task_a is present.
    let checkpoint = ActivityReplayer::load(recording.path()).expect("load checkpoint");
    assert_eq!(
        checkpoint.entry_count(),
        1,
        "atomicity: only successfully-completed nodes are checkpointed"
    );
    assert!(
        checkpoint.lookup("task_a", 0).is_some(),
        "task_a output must be in checkpoint"
    );
    assert!(
        checkpoint.lookup("task_b", 0).is_none(),
        "task_b output must NOT be in checkpoint (it crashed)"
    );
    assert!(
        checkpoint.lookup("task_c", 0).is_none(),
        "task_c output must NOT be in checkpoint (skipped after task_b crashed)"
    );
}
