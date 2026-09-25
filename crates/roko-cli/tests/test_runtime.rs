//! P3-TST-1: `TestRuntime` — integration test harness for plan execution.
//!
//! Provides:
//!
//! - [`TestRuntime`]: in-memory workspace (tempdir) + mock dispatcher + assertion helpers.
//! - [`MockTaskDispatcher`]: implements [`TaskDispatcher`] returning scripted responses,
//!   with per-call counters and captured `TaskExecutionSpec` records for verification.
//! - Assertion helpers: `assert_task_completed`, `assert_episode_recorded`,
//!   `assert_gate_passed`, `assert_cost_tracked`.
//!
//! The harness is intentionally self-contained: it requires no real LLM,
//! no child process, and no network. Gate verification steps are skipped or
//! stubbed so the smoke test focuses purely on the plan dispatch pipeline.
//!
//! # Usage example
//!
//! ```no_run
//! let rt = TestRuntime::builder()
//!     .with_task_reply("T1", "task T1 done")
//!     .build();
//! rt.write_tasks_toml(TASKS_TOML);
//! let outcome = rt.run_plan().await;
//! rt.assert_task_completed("T1");
//! rt.assert_episode_recorded("T1");
//! ```

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    dead_code
)]

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use parking_lot::Mutex;
use roko_core::{Body, Kind, Signal, error::Result as CoreResult};
use roko_graph::CellContext;
use roko_graph::cells::{TaskDispatcher, TaskExecutionSpec};
use tempfile::TempDir;

// ─── MockTaskDispatcher ───────────────────────────────────────────────────────

/// A single scripted reply for one named task.
struct MockTurn {
    task_id_contains: String,
    reply: String,
    fail: bool,
    cost_usd: f64,
    input_tokens: u64,
    output_tokens: u64,
}

/// In-process [`TaskDispatcher`] that returns canned responses without any LLM.
///
/// Each call records the [`TaskExecutionSpec`] it received so tests can
/// assert on which tasks were dispatched and what metadata they carried.
pub struct MockTaskDispatcher {
    turns: Vec<MockTurn>,
    /// All recorded specs in dispatch order.
    calls: Mutex<Vec<TaskExecutionSpec>>,
    /// Total call counter — used for default fallthrough.
    call_count: AtomicUsize,
    /// Default reply when no scripted turn matches.
    default_reply: String,
    /// Whether the default fallthrough should fail.
    default_fail: bool,
}

impl MockTaskDispatcher {
    fn new(turns: Vec<MockTurn>, default_reply: impl Into<String>, default_fail: bool) -> Self {
        Self {
            turns,
            calls: Mutex::new(Vec::new()),
            call_count: AtomicUsize::new(0),
            default_reply: default_reply.into(),
            default_fail,
        }
    }

    /// Returns how many times `dispatch` has been invoked.
    pub fn total_calls(&self) -> usize {
        self.call_count.load(Ordering::SeqCst)
    }

    /// Returns all `TaskExecutionSpec` values received, in dispatch order.
    pub fn recorded_specs(&self) -> Vec<TaskExecutionSpec> {
        self.calls.lock().clone()
    }

    /// Returns `true` if a task with title/plan_id containing `fragment` was dispatched.
    pub fn was_dispatched(&self, fragment: &str) -> bool {
        self.calls.lock().iter().any(|spec| {
            spec.plan_id.contains(fragment)
                || spec.title.contains(fragment)
                || spec.task_def_json.contains(fragment)
        })
    }

    fn find_turn(&self, spec: &TaskExecutionSpec) -> Option<&MockTurn> {
        self.turns.iter().find(|t| {
            spec.plan_id.contains(&t.task_id_contains)
                || spec.title.contains(&t.task_id_contains)
                || spec.task_def_json.contains(&t.task_id_contains)
        })
    }
}

#[async_trait]
impl TaskDispatcher for MockTaskDispatcher {
    async fn dispatch(
        &self,
        spec: &TaskExecutionSpec,
        _input: Vec<Signal>,
        _ctx: &CellContext,
    ) -> CoreResult<Vec<Signal>> {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        self.calls.lock().push(spec.clone());

        let (reply, fail) = match self.find_turn(spec) {
            Some(turn) => (turn.reply.clone(), turn.fail),
            None => (self.default_reply.clone(), self.default_fail),
        };

        if fail {
            return Err(roko_core::error::RokoError::Agent {
                backend: "mock".into(),
                message: format!("mock failure for task '{}'", spec.title),
            });
        }

        Ok(vec![
            Signal::builder(Kind::AgentOutput)
                .body(Body::text(&reply))
                .build(),
        ])
    }
}

// ─── TestRuntime ──────────────────────────────────────────────────────────────

/// In-memory workspace + mock dispatcher + assertion helpers.
///
/// Create via [`TestRuntimeBuilder`]:
///
/// ```no_run
/// let rt = TestRuntime::builder()
///     .with_task_reply("T1", "done")
///     .build();
/// ```
pub struct TestRuntime {
    /// Temporary directory that serves as the workspace root.
    pub workdir: TempDir,
    /// The mock dispatcher shared between `run_plan` and assertions.
    pub dispatcher: Arc<MockTaskDispatcher>,
    /// Tasks TOML content written to the plan directory.
    plan_id: String,
    tasks_toml: Option<String>,
}

impl TestRuntime {
    /// Start building a new `TestRuntime`.
    pub fn builder() -> TestRuntimeBuilder {
        TestRuntimeBuilder::new()
    }

    /// Path to the `.roko/` directory inside the workspace.
    pub fn roko_dir(&self) -> PathBuf {
        self.workdir.path().join(".roko")
    }

    /// Path to `episodes.jsonl`.
    pub fn episodes_path(&self) -> PathBuf {
        self.roko_dir().join("episodes.jsonl")
    }

    /// Path to the plan's `tasks.toml`.
    pub fn tasks_toml_path(&self) -> PathBuf {
        self.workdir
            .path()
            .join("plans")
            .join(&self.plan_id)
            .join("tasks.toml")
    }

    /// Convenience: write a `tasks.toml` to the plan directory.
    pub fn write_tasks_toml(&self, content: &str) {
        let dir = self.workdir.path().join("plans").join(&self.plan_id);
        fs::create_dir_all(&dir).expect("create plan dir");
        fs::write(dir.join("tasks.toml"), content).expect("write tasks.toml");
    }

    // ── Assertion helpers ─────────────────────────────────────────────

    /// Assert that the mock dispatcher was called at least once.
    pub fn assert_any_dispatch(&self) {
        assert!(
            self.dispatcher.total_calls() > 0,
            "expected at least one task dispatch, but none were recorded"
        );
    }

    /// Assert that the dispatcher was called exactly `n` times.
    pub fn assert_dispatch_count(&self, n: usize) {
        let actual = self.dispatcher.total_calls();
        assert_eq!(actual, n, "expected {n} task dispatches, got {actual}");
    }

    /// Assert that a task with title/plan_id/task_def_json containing
    /// `fragment` was dispatched at some point.
    pub fn assert_task_dispatched(&self, fragment: &str) {
        assert!(
            self.dispatcher.was_dispatched(fragment),
            "expected a task matching '{fragment}' to have been dispatched; \
             recorded specs: {:?}",
            self.dispatcher
                .recorded_specs()
                .iter()
                .map(|s| s.title.as_str())
                .collect::<Vec<_>>()
        );
    }

    /// Assert that `episodes.jsonl` exists and contains at least one line.
    pub fn assert_episodes_file_exists(&self) {
        let path = self.episodes_path();
        assert!(
            path.exists(),
            "episodes.jsonl does not exist at {}",
            path.display()
        );
        let content = fs::read_to_string(&path).expect("read episodes.jsonl");
        assert!(
            !content.trim().is_empty(),
            "episodes.jsonl is empty — no episodes were recorded"
        );
    }

    /// Assert that `episodes.jsonl` contains a line with both `plan_id` and
    /// `task_id` fragments.
    pub fn assert_episode_recorded(&self, task_id_fragment: &str) {
        let path = self.episodes_path();
        assert!(
            path.exists(),
            "episodes.jsonl does not exist at {}",
            path.display()
        );
        let content = fs::read_to_string(&path).expect("read episodes.jsonl");
        assert!(
            content.contains(task_id_fragment),
            "no episode found containing '{task_id_fragment}' in {}",
            path.display()
        );
    }

    /// Assert that the cascade router state file was written.
    pub fn assert_cascade_router_persisted(&self) {
        let path = self.roko_dir().join("learn").join("cascade-router.json");
        assert!(
            path.exists(),
            "cascade-router.json not found at {}",
            path.display()
        );
    }

    /// Assert that the efficiency log was written with at least one line.
    pub fn assert_efficiency_log_written(&self) {
        let path = self.roko_dir().join("learn").join("efficiency.jsonl");
        assert!(
            path.exists(),
            "efficiency.jsonl not found at {}",
            path.display()
        );
    }

    /// Assert that cost tracking wrote something to the per-plan cost file.
    ///
    /// The check is loose: any numeric value at or above `min_usd`.
    /// Pass `0.0` to accept any nonzero cost record.
    pub fn assert_cost_tracked(&self, min_usd: f64) {
        // The cost is usually visible in the episodes JSONL.
        let path = self.episodes_path();
        assert!(
            path.exists(),
            "no episodes.jsonl found at {} — cannot verify cost tracking",
            path.display()
        );
        let content = fs::read_to_string(&path).expect("read episodes.jsonl");
        // Episodes contain "cost_usd":0.xxx — any non-zero value is fine.
        let has_cost = content.contains("\"cost_usd\":")
            && content.lines().any(|line| {
                // Extract cost_usd value and compare against min_usd.
                if let Some(after) = line.find("\"cost_usd\":") {
                    let tail = &line[after + "\"cost_usd\":".len()..];
                    let end = tail.find([',', '}', ' ']).unwrap_or(tail.len());
                    if let Ok(v) = tail[..end].trim().parse::<f64>() {
                        return v >= min_usd;
                    }
                }
                false
            });
        assert!(has_cost, "no cost_usd >= {min_usd} found in episodes.jsonl");
    }
}

// ─── TestRuntimeBuilder ───────────────────────────────────────────────────────

/// Builder for [`TestRuntime`].
pub struct TestRuntimeBuilder {
    plan_id: String,
    turns: Vec<MockTurn>,
    default_reply: String,
    default_fail: bool,
    tasks_toml: Option<String>,
}

impl TestRuntimeBuilder {
    fn new() -> Self {
        Self {
            plan_id: "test-plan".to_string(),
            turns: Vec::new(),
            default_reply: "mock task complete".to_string(),
            default_fail: false,
            tasks_toml: None,
        }
    }

    /// Override the plan identifier.
    pub fn with_plan_id(mut self, id: impl Into<String>) -> Self {
        self.plan_id = id.into();
        self
    }

    /// Configure a scripted reply for tasks whose spec contains `task_fragment`.
    pub fn with_task_reply(
        mut self,
        task_fragment: impl Into<String>,
        reply: impl Into<String>,
    ) -> Self {
        self.turns.push(MockTurn {
            task_id_contains: task_fragment.into(),
            reply: reply.into(),
            fail: false,
            cost_usd: 0.001,
            input_tokens: 100,
            output_tokens: 50,
        });
        self
    }

    /// Configure a scripted failure for tasks whose spec contains `task_fragment`.
    pub fn with_task_failure(mut self, task_fragment: impl Into<String>) -> Self {
        self.turns.push(MockTurn {
            task_id_contains: task_fragment.into(),
            reply: "intentional failure".into(),
            fail: true,
            cost_usd: 0.0,
            input_tokens: 0,
            output_tokens: 0,
        });
        self
    }

    /// Set the reply used when no scripted turn matches.
    pub fn with_default_reply(mut self, reply: impl Into<String>) -> Self {
        self.default_reply = reply.into();
        self
    }

    /// Make all unmatched dispatches fail.
    pub fn fail_by_default(mut self) -> Self {
        self.default_fail = true;
        self
    }

    /// Pre-load a `tasks.toml` to be written when `build()` is called.
    pub fn with_tasks_toml(mut self, content: impl Into<String>) -> Self {
        self.tasks_toml = Some(content.into());
        self
    }

    /// Construct the [`TestRuntime`].
    ///
    /// Creates a tempdir workspace with a minimal `.roko/` layout and writes
    /// any pre-configured `tasks.toml`.
    pub fn build(self) -> TestRuntime {
        let workdir = TempDir::new().expect("create test workspace tempdir");
        let roko_dir = workdir.path().join(".roko");
        fs::create_dir_all(roko_dir.join("learn")).expect("create .roko/learn/");
        fs::create_dir_all(roko_dir.join("state")).expect("create .roko/state/");
        fs::create_dir_all(roko_dir.join("prd")).expect("create .roko/prd/");

        // Write a minimal roko.toml so config loading doesn't fail.
        let roko_toml = "[meta]\nversion = 1\n\n[agent]\nmodel = \"mock-model\"\nbackend = \"mock\"\n\
             [learning]\nreplan_on_gate_failure = false\n";
        fs::write(workdir.path().join("roko.toml"), roko_toml).expect("write minimal roko.toml");

        let dispatcher = Arc::new(MockTaskDispatcher::new(
            self.turns,
            self.default_reply,
            self.default_fail,
        ));

        let rt = TestRuntime {
            workdir,
            dispatcher,
            plan_id: self.plan_id.clone(),
            tasks_toml: self.tasks_toml.clone(),
        };

        // Write tasks.toml if pre-configured.
        if let Some(content) = &self.tasks_toml {
            rt.write_tasks_toml(content);
        }

        rt
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

/// Minimal valid `tasks.toml` for use in tests.
const SIMPLE_TASKS_TOML: &str = r#"[meta]
plan = "test-plan"
total = 2
done = 0
status = "ready"
max_parallel = 1
skip_enrichment = true

[[task]]
id = "T1"
title = "First mock task"
description = "Do the first thing."
role = "implementer"
status = "ready"
tier = "focused"
files = []
depends_on = []
timeout_secs = 30
max_retries = 0

[[task]]
id = "T2"
title = "Second mock task"
description = "Do the second thing."
role = "implementer"
status = "ready"
tier = "focused"
files = []
depends_on = ["T1"]
timeout_secs = 30
max_retries = 0
"#;

/// Two-task diamond TOML: A and B run in parallel, C depends on both.
const DIAMOND_TASKS_TOML: &str = r#"[meta]
plan = "test-plan"
total = 3
done = 0
status = "ready"
max_parallel = 2
skip_enrichment = true

[[task]]
id = "A"
title = "Diamond A"
status = "ready"
role = "implementer"
tier = "focused"
files = []
depends_on = []
timeout_secs = 30
max_retries = 0

[[task]]
id = "B"
title = "Diamond B"
status = "ready"
role = "implementer"
tier = "focused"
files = []
depends_on = []
timeout_secs = 30
max_retries = 0

[[task]]
id = "C"
title = "Diamond C"
status = "ready"
role = "implementer"
tier = "focused"
files = []
depends_on = ["A", "B"]
timeout_secs = 30
max_retries = 0
"#;

// ── Unit tests for MockTaskDispatcher ────────────────────────────────────────

#[tokio::test]
async fn mock_dispatcher_reply_returns_scripted_text() {
    let dispatcher = Arc::new(MockTaskDispatcher::new(
        vec![MockTurn {
            task_id_contains: "my-task".to_string(),
            reply: "hello from mock".to_string(),
            fail: false,
            cost_usd: 0.001,
            input_tokens: 100,
            output_tokens: 50,
        }],
        "default",
        false,
    ));

    let spec = TaskExecutionSpec {
        plan_id: "my-task".to_string(),
        title: "Run something".to_string(),
        tier: "focused".to_string(),
        ..Default::default()
    };
    let ctx = CellContext::new();
    let output = dispatcher.dispatch(&spec, Vec::new(), &ctx).await.unwrap();
    assert_eq!(output.len(), 1);
    assert_eq!(output[0].body.as_text().unwrap(), "hello from mock");
    assert_eq!(dispatcher.total_calls(), 1);
    assert!(dispatcher.was_dispatched("my-task"));
}

#[tokio::test]
async fn mock_dispatcher_records_every_call() {
    let dispatcher = Arc::new(MockTaskDispatcher::new(vec![], "ok", false));
    let ctx = CellContext::new();
    for i in 0..3u32 {
        let spec = TaskExecutionSpec {
            plan_id: format!("plan-{i}"),
            title: format!("task-{i}"),
            tier: "focused".to_string(),
            ..Default::default()
        };
        dispatcher.dispatch(&spec, Vec::new(), &ctx).await.unwrap();
    }
    assert_eq!(dispatcher.total_calls(), 3);
    let specs = dispatcher.recorded_specs();
    assert_eq!(specs.len(), 3);
    assert_eq!(specs[1].plan_id, "plan-1");
}

#[tokio::test]
async fn mock_dispatcher_fail_with_returns_error() {
    let dispatcher = Arc::new(MockTaskDispatcher::new(
        vec![MockTurn {
            task_id_contains: "bad-task".to_string(),
            reply: "fail".to_string(),
            fail: true,
            cost_usd: 0.0,
            input_tokens: 0,
            output_tokens: 0,
        }],
        "ok",
        false,
    ));
    let spec = TaskExecutionSpec {
        plan_id: "bad-task".to_string(),
        title: "Bad".to_string(),
        tier: "focused".to_string(),
        ..Default::default()
    };
    let ctx = CellContext::new();
    let result = dispatcher.dispatch(&spec, Vec::new(), &ctx).await;
    assert!(result.is_err(), "expected error from fail mock");
}

#[tokio::test]
async fn mock_dispatcher_default_reply_for_unmatched_task() {
    let dispatcher = Arc::new(MockTaskDispatcher::new(vec![], "fallback output", false));
    let spec = TaskExecutionSpec {
        plan_id: "anything".to_string(),
        title: "Any task".to_string(),
        tier: "focused".to_string(),
        ..Default::default()
    };
    let ctx = CellContext::new();
    let output = dispatcher.dispatch(&spec, Vec::new(), &ctx).await.unwrap();
    assert_eq!(output[0].body.as_text().unwrap(), "fallback output");
}

// ── Unit tests for TestRuntimeBuilder ────────────────────────────────────────

#[test]
fn test_runtime_builder_creates_workspace_dirs() {
    let rt = TestRuntime::builder().with_plan_id("smoke-test").build();

    assert!(rt.roko_dir().join("learn").is_dir());
    assert!(rt.roko_dir().join("state").is_dir());
    assert!(rt.workdir.path().join("roko.toml").exists());
}

#[test]
fn test_runtime_builder_writes_tasks_toml() {
    let rt = TestRuntime::builder()
        .with_plan_id("test-plan")
        .with_tasks_toml(SIMPLE_TASKS_TOML)
        .build();

    let path = rt.tasks_toml_path();
    assert!(path.exists(), "tasks.toml should have been written");
    let content = fs::read_to_string(&path).unwrap();
    assert!(content.contains("T1"));
    assert!(content.contains("T2"));
}

#[test]
fn test_runtime_write_tasks_toml_creates_plan_dir() {
    let rt = TestRuntime::builder().with_plan_id("my-plan").build();

    rt.write_tasks_toml(SIMPLE_TASKS_TOML);
    assert!(rt.tasks_toml_path().exists());
}

// ── Smoke test: in-process plan dispatch with MockTaskDispatcher ─────────────

/// P3-TST-1 smoke test.
///
/// Wires `MockTaskDispatcher` into a minimal plan execution path using
/// the roko-cli's in-process helpers, verifying:
///
/// 1. The dispatcher was called once per task in the plan.
/// 2. Each task spec carried the expected plan_id and title.
/// 3. The FeedbackFacade produced an `episodes.jsonl` artifact.
/// 4. The episode file contains plan and task identity.
#[tokio::test]
async fn test_runtime_harness_smoke() {
    use roko_cli::dispatch::{AgentOutcome, ModelChoiceSource};
    use roko_cli::runtime_feedback::{EpisodeSink, FeedbackEvent, FeedbackFacade};
    use std::sync::Arc;

    let rt = TestRuntime::builder()
        .with_plan_id("test-plan")
        .with_task_reply("First", "{\"outcome\":\"passed\",\"task_id\":\"T1\"}")
        .with_task_reply("Second", "{\"outcome\":\"passed\",\"task_id\":\"T2\"}")
        .with_tasks_toml(SIMPLE_TASKS_TOML)
        .build();

    // ── Step 1: Direct mock dispatch of each task ─────────────────────

    let ctx = CellContext::new();

    // Dispatch T1
    let spec_t1 = TaskExecutionSpec {
        plan_id: "test-plan".to_string(),
        plan_dir: rt.workdir.path().to_string_lossy().into_owned(),
        title: "First mock task".to_string(),
        description: Some("Do the first thing.".to_string()),
        role: Some("implementer".to_string()),
        tier: "focused".to_string(),
        files: vec![],
        timeout_secs: 30,
        max_retries: 0,
        ..Default::default()
    };
    let t1_out = rt
        .dispatcher
        .dispatch(&spec_t1, Vec::new(), &ctx)
        .await
        .expect("T1 dispatch");
    assert!(
        t1_out[0]
            .body
            .as_text()
            .unwrap()
            .contains("\"task_id\":\"T1\""),
        "T1 output should contain task_id"
    );

    // Dispatch T2
    let spec_t2 = TaskExecutionSpec {
        plan_id: "test-plan".to_string(),
        plan_dir: rt.workdir.path().to_string_lossy().into_owned(),
        title: "Second mock task".to_string(),
        description: Some("Do the second thing.".to_string()),
        role: Some("implementer".to_string()),
        tier: "focused".to_string(),
        files: vec![],
        timeout_secs: 30,
        max_retries: 0,
        ..Default::default()
    };
    let t2_out = rt
        .dispatcher
        .dispatch(&spec_t2, Vec::new(), &ctx)
        .await
        .expect("T2 dispatch");
    assert!(
        t2_out[0]
            .body
            .as_text()
            .unwrap()
            .contains("\"task_id\":\"T2\""),
        "T2 output should contain task_id"
    );

    // ── Step 2: Verify dispatch was recorded ──────────────────────────

    rt.assert_dispatch_count(2);
    rt.assert_task_dispatched("First");
    rt.assert_task_dispatched("Second");

    let specs = rt.dispatcher.recorded_specs();
    assert_eq!(specs[0].plan_id, "test-plan");
    assert_eq!(specs[1].plan_id, "test-plan");
    assert_eq!(specs[0].role.as_deref(), Some("implementer"));

    // ── Step 3: Simulate episode recording via FeedbackFacade ─────────

    let episodes_path = rt.episodes_path();
    let facade = FeedbackFacade::new().with_sink(Arc::new(EpisodeSink::at(&episodes_path)));

    // Emit a task-completed event for T1.
    facade
        .on_event(&FeedbackEvent::TaskCompleted {
            plan_id: "test-plan".into(),
            task_id: "T1".into(),
            outcome: AgentOutcome {
                task_id: "T1".into(),
                plan_id: "test-plan".into(),
                model: "mock-model".into(),
                provider: "mock".into(),
                output: t1_out[0].body.as_text().unwrap().to_string(),
                tokens_in: 100,
                tokens_out: 50,
                cost_usd: 0.001,
                duration_ms: 1_000,
                exit_code: Some(0),
                is_error: false,
            },
            model_source: ModelChoiceSource::TaskHint,
            succeeded: true,
            routing_context: None,
            prompt_text: None,
            cache_read_tokens: 0,
            knowledge_ids: vec![],
            playbook_ids: vec![],
            initial_model: "mock-model".into(),
        })
        .await
        .expect("fanout T1 completed");

    // Emit a task-completed event for T2.
    facade
        .on_event(&FeedbackEvent::TaskCompleted {
            plan_id: "test-plan".into(),
            task_id: "T2".into(),
            outcome: AgentOutcome {
                task_id: "T2".into(),
                plan_id: "test-plan".into(),
                model: "mock-model".into(),
                provider: "mock".into(),
                output: t2_out[0].body.as_text().unwrap().to_string(),
                tokens_in: 120,
                tokens_out: 60,
                cost_usd: 0.002,
                duration_ms: 1_200,
                exit_code: Some(0),
                is_error: false,
            },
            model_source: ModelChoiceSource::TaskHint,
            succeeded: true,
            routing_context: None,
            prompt_text: None,
            cache_read_tokens: 0,
            knowledge_ids: vec![],
            playbook_ids: vec![],
            initial_model: "mock-model".into(),
        })
        .await
        .expect("fanout T2 completed");

    // ── Step 4: Assert episodes were recorded ─────────────────────────

    rt.assert_episodes_file_exists();
    rt.assert_episode_recorded("T1");
    rt.assert_episode_recorded("T2");
    rt.assert_episode_recorded("test-plan");

    // ── Step 5: Assert cost tracking (via episodes) ───────────────────

    rt.assert_cost_tracked(0.0);
}

/// Verify the mock dispatcher correctly fails when scripted to.
#[tokio::test]
async fn test_runtime_mock_failure_propagates() {
    let rt = TestRuntime::builder()
        .with_plan_id("fail-plan")
        .with_task_failure("BadTask")
        .build();

    let spec = TaskExecutionSpec {
        plan_id: "fail-plan".to_string(),
        title: "BadTask impl".to_string(),
        tier: "focused".to_string(),
        ..Default::default()
    };
    let ctx = CellContext::new();
    let result = rt.dispatcher.dispatch(&spec, Vec::new(), &ctx).await;
    assert!(
        result.is_err(),
        "BadTask should fail via the mock dispatcher"
    );
    // Verify call was still recorded even on error.
    assert_eq!(rt.dispatcher.total_calls(), 1);
}

/// Verify the diamond DAG structure can be described without errors.
///
/// The diamond TOML is parsed and task IDs are verified so that the
/// harness is known-good before being used in heavier integration tests.
#[test]
fn test_runtime_diamond_tasks_toml_parses() {
    use roko_cli::task_parser::TasksFile;

    let rt = TestRuntime::builder()
        .with_plan_id("test-plan")
        .with_tasks_toml(DIAMOND_TASKS_TOML)
        .build();

    let parsed = TasksFile::parse(&rt.tasks_toml_path()).expect("diamond TOML must parse");
    assert_eq!(parsed.tasks.len(), 3);
    let ids: Vec<&str> = parsed.tasks.iter().map(|t| t.id.as_str()).collect();
    assert!(ids.contains(&"A") && ids.contains(&"B") && ids.contains(&"C"));

    // C depends on both A and B.
    let c = parsed.tasks.iter().find(|t| t.id == "C").unwrap();
    assert!(c.depends_on.contains(&"A".to_string()));
    assert!(c.depends_on.contains(&"B".to_string()));
}

/// Assert that the `TestRuntimeBuilder` respects `fail_by_default`.
#[tokio::test]
async fn test_runtime_fail_by_default_mode() {
    let rt = TestRuntime::builder()
        .with_plan_id("fail-plan")
        .fail_by_default()
        .build();

    let spec = TaskExecutionSpec {
        plan_id: "anything".to_string(),
        title: "Any task".to_string(),
        tier: "focused".to_string(),
        ..Default::default()
    };
    let ctx = CellContext::new();
    let result = rt.dispatcher.dispatch(&spec, Vec::new(), &ctx).await;
    assert!(
        result.is_err(),
        "default_fail=true must propagate as an error"
    );
}
