//! The opt-in LLM-judge step of a Graph task attempt (gap-85f102).
//!
//! With `[gates] llm_judge = true`, an attempt whose verify steps all passed
//! is scored by the cheap helper model through the LLM-judge gate rung: its
//! task against its diff, an [`AgentJudgeOracle`] behind an
//! [`LlmJudgeGate`]. The verdict is advisory: it is logged, shown on the
//! dashboard, counted in the gate metrics and appended to
//! `.roko/learn/judge-calibration.jsonl`, and the attempt's verdict stands.
//! With `llm_judge_blocking = true`, a score below `llm_judge_min_score`, or
//! a judge that cannot answer, fails the attempt like a failed verify step,
//! and the next attempt's feedback carries the judge's reason. An attempt
//! with no diff, or a run with no helper model, is not judged.

use roko_gate::AgentJudgeOracle;
use roko_gate::llm_judge_gate::{JudgePayload, LlmJudgeGate};

use super::*;

/// The judge step's gate name, in logs, on the dashboard and in failures.
const JUDGE_GATE: &str = "llm-judge";

impl GraphTaskDispatcher {
    /// Judge the attempt `attempt_key` at `task` in `workdir`, whose verify
    /// steps all passed, when `[gates] llm_judge` is on. Returns the failure
    /// to report when a blocking judge failed it; an advisory verdict never
    /// fails it.
    pub(super) async fn judge_attempt(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        attempt_key: &str,
        workdir: &Path,
    ) -> Option<String> {
        let gates = &self.config.gates;
        if !gates.llm_judge {
            return None;
        }
        let Some(agent) = self.cheap_agent() else {
            tracing::warn!(
                plan_id = %spec.plan_id,
                task_id = %task.id,
                "[gates] llm_judge is on, but no helper model can run: the attempt is not judged"
            );
            return None;
        };
        let diff = match self.attempt_diff(spec, task, attempt_key, workdir).await {
            Some(diff) => diff.patch().await.unwrap_or_default(),
            None => String::new(),
        };
        if diff.trim().is_empty() {
            tracing::info!(
                plan_id = %spec.plan_id,
                task_id = %task.id,
                "the attempt left no diff the LLM judge can read: it is not judged"
            );
            return None;
        }
        let payload = JudgePayload {
            task_description: judge_task_description(spec, task),
            diff,
        };
        let signal = Signal::builder(Kind::Task)
            .body(Body::from_json(&payload).unwrap_or_else(|_| Body::empty()))
            .build();
        let oracle = Arc::new(AgentJudgeOracle::new(Arc::new(agent)));
        let calibration = roko_fs::RokoLayout::for_project(&self.workdir)
            .learn_dir()
            .join("judge-calibration.jsonl");
        let mut gate = LlmJudgeGate::new(oracle, gates.llm_judge_min_score)
            .with_name(JUDGE_GATE)
            .with_calibration_log(calibration);
        if gates.llm_judge_blocking {
            gate = gate.blocking();
        }
        let verdict = gate.verify(&signal, &Context::now()).await;

        gate_learning::record_gate_verdict_metrics(self.metrics.as_deref(), JUDGE_GATE, &verdict);
        if let Some(tui) = &self.tui_bridge {
            tui.gate_result(&spec.plan_id, &task.id, JUDGE_GATE, verdict.passed);
        }
        let blocks = gates.llm_judge_blocking && !verdict.passed;
        tracing::info!(
            plan_id = %spec.plan_id,
            task_id = %task.id,
            passed = verdict.passed,
            score = verdict.score,
            blocking = gates.llm_judge_blocking,
            reason = %verdict.reason,
            detail = ?verdict.detail,
            "LLM judge scored the attempt"
        );
        blocks.then(|| format!("{JUDGE_GATE}: {}", verdict.reason))
    }
}

/// What the judge is told the attempt was for: the task's title and, when it
/// has one, its description.
fn judge_task_description(spec: &TaskExecutionSpec, task: &TaskDef) -> String {
    match task
        .description
        .as_deref()
        .map(str::trim)
        .filter(|description| !description.is_empty())
    {
        Some(description) => format!("{}\n\n{description}", spec.title),
        None => spec.title.clone(),
    }
}

#[cfg(test)]
mod tests {
    use roko_core::agent::ProviderKind;
    use roko_core::config::schema::{ModelProfile, ProviderConfig};
    use tempfile::tempdir;

    use super::*;
    use crate::graph_task_dispatch::diff_snapshot::tests::commit_repo;
    use crate::graph_task_dispatch::tests::{
        FIXTURE_PROVIDER_TIMEOUT_MS, final_turn, make_spec, make_test_dispatcher, no_auto_fix,
        spawn_openai_mock, verify_step,
    };

    /// A fake Claude CLI for the task's attempt: it adds a function to
    /// `src/lib.rs`, so the attempt has a diff to judge.
    const EDITING_PROVIDER: &str = r#"#!/bin/sh
set -eu
cat >/dev/null
printf 'pub fn two() -> u8 {\n    2\n}\n' >> src/lib.rs
printf '%s\n' '{"type":"content_block_delta","delta":{"text":"Added two()."}}'
printf '%s\n' '{"type":"result","session_id":"s","model":"claude-sonnet-4-6","total_cost_usd":0.01,"usage":{"input_tokens":5,"output_tokens":10}}'
"#;

    /// Dispatch one attempt at a task whose only verify step passes, in a
    /// git repository, with the LLM judge configured by `judge` and a helper
    /// model on an OpenAI-compatible mock that scores every call `score`.
    /// Returns the attempt's result and the requests the helper model saw.
    async fn judged_attempt(
        judge: impl FnOnce(&mut RokoConfig),
        score: &str,
    ) -> (Result<Vec<Signal>>, Vec<serde_json::Value>) {
        let temp = tempdir().expect("tempdir");
        commit_repo(
            temp.path(),
            &[("src/lib.rs", "pub fn one() -> u8 {\n    1\n}\n")],
        );
        // The judge's answer, and one more for the error diagnosis a failed
        // attempt asks the helper model for.
        let (base_url, requests) = spawn_openai_mock(vec![final_turn(score); 2]);
        let (dispatcher, mut task) = make_test_dispatcher(
            &temp,
            EDITING_PROVIDER,
            |config| {
                no_auto_fix(config);
                config.providers.insert(
                    "judge_api".to_string(),
                    ProviderConfig {
                        kind: ProviderKind::OpenAiCompat,
                        base_url: Some(base_url),
                        api_key_env: Some("PATH".to_string()),
                        command: None,
                        args: None,
                        timeout_ms: Some(FIXTURE_PROVIDER_TIMEOUT_MS),
                        ttft_timeout_ms: Some(FIXTURE_PROVIDER_TIMEOUT_MS),
                        connect_timeout_ms: Some(FIXTURE_PROVIDER_TIMEOUT_MS),
                        extra_headers: None,
                        max_concurrent: None,
                        limits: None,
                        require_confirmation: false,
                        stream_usage: None,
                        billing: None,
                    },
                );
                config.models.insert(
                    "judge-model".to_string(),
                    ModelProfile {
                        provider: "judge_api".to_string(),
                        slug: "judge-1".to_string(),
                        context_window: 128_000,
                        max_output: Some(1_024),
                        max_tools: Some(32),
                        supports_tools: true,
                        tool_format: "openai_json".to_string(),
                        ..ModelProfile::default()
                    },
                );
                config.routing.fast_task_model = "judge-model".to_string();
                judge(config);
            },
            GraphFeedbackContext::default(),
        )
        .await;
        task.files = vec!["src/lib.rs".to_string()];
        task.verify = vec![verify_step("structural", "grep -q 'fn two' src/lib.rs")];

        let result = dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &CellContext::new())
            .await;
        drop(dispatcher);
        let requests = requests.lock().clone();
        (result, requests)
    }

    /// gap-85f102: the judge runs only when `[gates] llm_judge` is on, scores
    /// the attempt's diff against its task, and fails a passing attempt only
    /// when it is blocking.
    #[tokio::test]
    async fn a_failing_judge_blocks_only_when_configured() {
        // Off: the helper model is never asked.
        let (result, requests) = judged_attempt(|_| {}, "0.2").await;
        result.expect("an unjudged attempt passes");
        assert!(requests.is_empty(), "{requests:?}");

        // Advisory: a low score is recorded, and the attempt passes.
        let (result, requests) =
            judged_attempt(|config| config.gates.llm_judge = true, "0.2").await;
        result.expect("an advisory judge never fails the attempt");
        assert_eq!(requests.len(), 1, "{requests:?}");
        let prompt = requests[0].to_string();
        assert!(prompt.contains("Score this implementation"), "{prompt}");
        assert!(prompt.contains("Streaming graph task"), "{prompt}");
        assert!(
            prompt.contains("fn two"),
            "the judge sees the diff: {prompt}"
        );

        // Blocking: the same score fails the attempt.
        let (result, _) = judged_attempt(
            |config| {
                config.gates.llm_judge = true;
                config.gates.llm_judge_blocking = true;
            },
            "0.2",
        )
        .await;
        let error = result.expect_err("a blocking judge fails a low score");
        let RokoError::Verify { message, .. } = error else {
            panic!("expected a verify failure, got {error}");
        };
        assert!(message.contains("llm-judge"), "{message}");
        assert!(message.contains("below threshold"), "{message}");

        // Blocking, but a passing score: the attempt passes.
        let (result, _) = judged_attempt(
            |config| {
                config.gates.llm_judge = true;
                config.gates.llm_judge_blocking = true;
            },
            "0.9",
        )
        .await;
        result.expect("a passing score keeps the attempt passing");
    }
}
