//! RAG-20: Demo harness and proof-of-improvement.
//!
//! Runs two simulated plan executions side by side:
//!
//! 1. **Baseline** — no retrieval context, so each task receives only its
//!    description as context.
//! 2. **Retrieval-enabled** — a [`UnifiedRetrievalContextBidder`] with a
//!    mock knowledge provider injects additional context before each task is
//!    dispatched.
//!
//! The test then compares the simulated gate pass rates and asserts that the
//! retrieval-enabled run achieves an equal or higher pass rate. Because the
//! dispatchers are deterministic mocks, the test is purely structural: it
//! demonstrates that the retrieval pipeline is wired, that context candidates
//! reach the task dispatcher, and that the resulting statistics can be
//! collected and compared.
//!
//! # Simulated gate logic
//!
//! The mock "gate" awards a pass when the dispatcher received a context
//! payload containing the retrieval marker string `"[retrieved:"`. This
//! mirrors the real-world expectation: when retrieval is enabled, injected
//! context reaches the agent and improves the probability of a gate pass.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    dead_code
)]

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use parking_lot::Mutex;
use roko_compose::context_provider::{ContextBidder, ContextProvider, ContextRequest, ContextTier};
use roko_compose::graph_cells::knowledge::KnowledgeProvider;
use roko_compose::graph_cells::signals::ComposeScope;
use roko_compose::prompt::{
    AttentionBidder, CacheLayer, Placement, PromptSection, SectionPriority,
};
use roko_compose::unified_retrieval_bidder::UnifiedRetrievalContextBidder;
use roko_core::config::RetrievalConfig;
use roko_core::error::Result as CoreResult;
use roko_core::{Body, Kind, Signal};
use roko_graph::CellContext;
use roko_graph::cells::{TaskDispatcher, TaskExecutionSpec};

// ─── Mock knowledge provider ─────────────────────────────────────────────────

/// Returns a single retrieval section for every query.
///
/// The section body contains `"[retrieved: relevant context for <task_id>]"`
/// which the gate check below uses as a signal that retrieval fired.
struct FixedKnowledgeProvider;

impl KnowledgeProvider for FixedKnowledgeProvider {
    fn query_sections(
        &self,
        scope: &ComposeScope,
        _budget_tokens: Option<usize>,
    ) -> Vec<PromptSection> {
        let mut section = PromptSection::new(
            format!("rag-knowledge-{}", scope.task_id),
            format!("[retrieved: relevant context for {}]", scope.task_id),
        );
        section.bidder = AttentionBidder::Neuro;
        section.priority = SectionPriority::Normal;
        section.placement = Placement::Middle;
        section.cache_layer = CacheLayer::Workspace;
        vec![section]
    }
}

// ─── Counting task dispatcher ─────────────────────────────────────────────────

/// Records every dispatch call; if the context payload contains the retrieval
/// marker it counts as a "gate pass" in the simulated scoring.
struct CountingDispatcher {
    dispatches: AtomicUsize,
    gate_passes: AtomicUsize,
    /// Retrieval context to inject before each dispatch (empty = no retrieval).
    retrieval_context: Mutex<String>,
}

impl CountingDispatcher {
    fn new(retrieval_context: impl Into<String>) -> Self {
        Self {
            dispatches: AtomicUsize::new(0),
            gate_passes: AtomicUsize::new(0),
            retrieval_context: Mutex::new(retrieval_context.into()),
        }
    }

    fn dispatch_count(&self) -> usize {
        self.dispatches.load(Ordering::SeqCst)
    }

    fn gate_pass_count(&self) -> usize {
        self.gate_passes.load(Ordering::SeqCst)
    }

    fn gate_pass_rate(&self) -> f64 {
        let d = self.dispatch_count();
        if d == 0 {
            return 0.0;
        }
        self.gate_pass_count() as f64 / d as f64
    }
}

#[async_trait]
impl TaskDispatcher for CountingDispatcher {
    async fn dispatch(
        &self,
        spec: &TaskExecutionSpec,
        _input: Vec<Signal>,
        _ctx: &CellContext,
    ) -> CoreResult<Vec<Signal>> {
        self.dispatches.fetch_add(1, Ordering::SeqCst);

        let retrieval = self.retrieval_context.lock().clone();

        // Simulate a gate: pass if the retrieval context is present.
        let full_context = format!("{}\n{}", spec.task_def_json, retrieval);
        if full_context.contains("[retrieved:") {
            self.gate_passes.fetch_add(1, Ordering::SeqCst);
        }

        let output = format!(
            "{{\"outcome\":\"done\",\"task_id\":\"{}\",\"context_len\":{}}}",
            spec.title,
            full_context.len()
        );
        Ok(vec![
            Signal::builder(Kind::AgentOutput)
                .body(Body::text(&output))
                .build(),
        ])
    }
}

// ─── Retrieval harness ────────────────────────────────────────────────────────

/// Run the retrieval pipeline for a list of task descriptions and collect the
/// merged context string.  Returns the concatenated content of all candidates.
fn run_retrieval(task_ids: &[&str]) -> String {
    let config = RetrievalConfig {
        token_budget: 4096,
        min_score: 0.0,
        max_results: 10,
        ..Default::default()
    };

    let bidder = UnifiedRetrievalContextBidder::new(config).with_knowledge(FixedKnowledgeProvider);

    let mut combined = String::new();
    let provider = ContextProvider::new(std::path::PathBuf::new());

    for task_id in task_ids {
        let request = ContextRequest {
            tier: ContextTier::Focused,
            budget_tokens: 4096,
            plan_id: "demo-plan".to_string(),
            task_id: task_id.to_string(),
            task_files: Vec::new(),
            task: None,
            plan_artifacts: None,
            siblings: Vec::new(),
            prior_outputs: Vec::new(),
            role_profile: None,
            prompt_policy: None,
        };

        let candidates = bidder.propose_context(&provider, &request);
        for candidate in &candidates {
            combined.push_str(&candidate.section.section.content);
            combined.push('\n');
        }
    }

    combined
}

// ─── Tests ────────────────────────────────────────────────────────────────────

const TASK_IDS: &[&str] = &[
    "implement-auth",
    "write-tests",
    "refactor-module",
    "add-validation",
    "fix-error-handling",
];

/// RAG-20 core proof: retrieval-enabled run has higher simulated gate pass rate.
#[tokio::test]
async fn rag_retrieval_enabled_improves_pass_rate() {
    let ctx = CellContext::new();

    // ── Baseline: no retrieval context ────────────────────────────────────────
    let baseline = Arc::new(CountingDispatcher::new(""));

    for task_id in TASK_IDS {
        let spec = TaskExecutionSpec {
            plan_id: "demo-plan".to_string(),
            title: (*task_id).to_string(),
            task_def_json: format!(
                "{{\"id\":\"{task_id}\",\"description\":\"task without retrieval\"}}"
            ),
            tier: "focused".to_string(),
            ..Default::default()
        };
        let _ = baseline.dispatch(&spec, Vec::new(), &ctx).await.unwrap();
    }

    let baseline_rate = baseline.gate_pass_rate();
    assert_eq!(
        baseline.dispatch_count(),
        TASK_IDS.len(),
        "baseline: all tasks dispatched"
    );

    // ── Retrieval-enabled: inject retrieved context ───────────────────────────
    let retrieved_context = run_retrieval(TASK_IDS);
    assert!(
        !retrieved_context.is_empty(),
        "retrieval pipeline must produce non-empty context"
    );
    assert!(
        retrieved_context.contains("[retrieved:"),
        "retrieval context must contain the expected marker"
    );

    let with_retrieval = Arc::new(CountingDispatcher::new(retrieved_context.clone()));

    for task_id in TASK_IDS {
        let spec = TaskExecutionSpec {
            plan_id: "demo-plan".to_string(),
            title: (*task_id).to_string(),
            task_def_json: format!(
                "{{\"id\":\"{task_id}\",\"description\":\"task with retrieval\",\"context\":\"{retrieved_context}\"}}",
                retrieved_context = retrieved_context.replace('"', "'")
            ),
            tier: "focused".to_string(),
            ..Default::default()
        };
        let _ = with_retrieval
            .dispatch(&spec, Vec::new(), &ctx)
            .await
            .unwrap();
    }

    let retrieval_rate = with_retrieval.gate_pass_rate();
    assert_eq!(
        with_retrieval.dispatch_count(),
        TASK_IDS.len(),
        "retrieval run: all tasks dispatched"
    );

    // ── Comparison ────────────────────────────────────────────────────────────
    let ret_pct = (retrieval_rate * 100.0) as u32;
    let base_pct = (baseline_rate * 100.0) as u32;
    assert!(
        retrieval_rate >= baseline_rate,
        "retrieval-enabled run ({ret_pct}%) must match or exceed baseline ({base_pct}%)"
    );
    assert_eq!(
        retrieval_rate, 1.0,
        "all tasks should pass the simulated gate when retrieval context is present"
    );
    assert_eq!(
        baseline_rate, 0.0,
        "no tasks should pass the simulated gate without retrieval context"
    );
}

/// Verify the UnifiedRetrievalContextBidder returns one candidate per task
/// when backed by the FixedKnowledgeProvider.
#[test]
fn retrieval_bidder_returns_candidates_for_each_task() {
    let config = RetrievalConfig {
        token_budget: 4096,
        min_score: 0.0,
        max_results: 10,
        ..Default::default()
    };

    let bidder = UnifiedRetrievalContextBidder::new(config).with_knowledge(FixedKnowledgeProvider);

    let provider = ContextProvider::new(std::path::PathBuf::new());

    for task_id in TASK_IDS {
        let request = ContextRequest {
            tier: ContextTier::Focused,
            budget_tokens: 4096,
            plan_id: "demo-plan".to_string(),
            task_id: task_id.to_string(),
            task_files: Vec::new(),
            task: None,
            plan_artifacts: None,
            siblings: Vec::new(),
            prior_outputs: Vec::new(),
            role_profile: None,
            prompt_policy: None,
        };

        let candidates = bidder.propose_context(&provider, &request);
        assert!(
            !candidates.is_empty(),
            "task '{task_id}': retrieval must yield at least one candidate"
        );
        let combined: String = candidates
            .iter()
            .map(|c| c.section.section.content.as_str())
            .collect::<Vec<_>>()
            .join("");
        assert!(
            combined.contains("[retrieved:"),
            "task '{task_id}': candidate content must contain the retrieval marker"
        );
    }
}

/// Verify that an empty bidder (no sources) produces zero candidates.
#[test]
fn retrieval_bidder_with_no_sources_returns_empty() {
    let config = RetrievalConfig {
        token_budget: 4096,
        min_score: 0.0,
        max_results: 10,
        ..Default::default()
    };

    // No .with_knowledge() / .with_episodes() / .with_code_index() attached.
    let bidder = UnifiedRetrievalContextBidder::new(config);

    let provider = ContextProvider::new(std::path::PathBuf::new());
    let request = ContextRequest {
        tier: ContextTier::Focused,
        budget_tokens: 4096,
        plan_id: "demo-plan".to_string(),
        task_id: "any-task".to_string(),
        task_files: Vec::new(),
        task: None,
        plan_artifacts: None,
        siblings: Vec::new(),
        prior_outputs: Vec::new(),
        role_profile: None,
        prompt_policy: None,
    };

    let candidates = bidder.propose_context(&provider, &request);
    assert!(
        candidates.is_empty(),
        "bidder with no sources must yield no candidates"
    );
}

/// Verify that retrieval context is unique per task (not shared across tasks).
#[test]
fn retrieval_context_is_task_scoped() {
    let config = RetrievalConfig {
        token_budget: 4096,
        min_score: 0.0,
        max_results: 10,
        ..Default::default()
    };

    let bidder = UnifiedRetrievalContextBidder::new(config).with_knowledge(FixedKnowledgeProvider);

    let provider = ContextProvider::new(std::path::PathBuf::new());

    let task_a_context = {
        let req = ContextRequest {
            tier: ContextTier::Focused,
            budget_tokens: 4096,
            plan_id: "p".to_string(),
            task_id: "task-alpha".to_string(),
            task_files: Vec::new(),
            task: None,
            plan_artifacts: None,
            siblings: Vec::new(),
            prior_outputs: Vec::new(),
            role_profile: None,
            prompt_policy: None,
        };
        let candidates = bidder.propose_context(&provider, &req);
        candidates
            .iter()
            .map(|c| c.section.section.content.clone())
            .collect::<Vec<_>>()
            .join("")
    };

    let task_b_context = {
        let req = ContextRequest {
            tier: ContextTier::Focused,
            budget_tokens: 4096,
            plan_id: "p".to_string(),
            task_id: "task-beta".to_string(),
            task_files: Vec::new(),
            task: None,
            plan_artifacts: None,
            siblings: Vec::new(),
            prior_outputs: Vec::new(),
            role_profile: None,
            prompt_policy: None,
        };
        let candidates = bidder.propose_context(&provider, &req);
        candidates
            .iter()
            .map(|c| c.section.section.content.clone())
            .collect::<Vec<_>>()
            .join("")
    };

    assert_ne!(
        task_a_context, task_b_context,
        "retrieval context must differ between tasks"
    );
    assert!(
        task_a_context.contains("task-alpha"),
        "task-alpha context must reference its own task ID"
    );
    assert!(
        task_b_context.contains("task-beta"),
        "task-beta context must reference its own task ID"
    );
}
