//! Helper model calls an attempt makes outside its agent run (bug-62e3f4).
//!
//! After a failed gate, verification asks the cheap helper model
//! ([`select_cheap_model_key`]) for an error diagnosis and, in the
//! background, a gate reflection. Each goes through a [`HelperAgent`],
//! which counts its call toward the attempt whose verify steps are settling
//! ([`HelperCalls::scope`]). The attempt then waits for
//! them and accounts every call like a dispatch
//! ([`GraphTaskDispatcher::settle_helper_calls`]): its spend on the task and
//! plan budgets, a cost row and an efficiency row keyed by the attempt, and
//! their totals on the attempt's verdict and episode.

use roko_core::pricing_snapshot::PriceSnapshot;
use roko_learn::efficiency::ExecutedRow;
use roko_learn::telemetry::{CostSource, HelperCallsUsage};

use super::served_model::{is_cli_backend, same_model};
use super::tui_forward::append_jsonl_line_async;
use super::*;

/// `role` of a helper call's cost and efficiency rows.
const HELPER_ROLE: &str = "helper";

/// How much longer than one helper call's own timeout an attempt waits for
/// its helper calls to finish.
const HELPER_SETTLE_GRACE: std::time::Duration = std::time::Duration::from_secs(5);

tokio::task_local! {
    /// The helper calls of the attempt whose verify steps are settling.
    static HELPER_CALLS: HelperCalls;
}

/// One provider call an attempt made beside its agent run's result: a
/// helper call after a failed gate, or a call provider failover refused
/// (bug-220385). Each gets its own cost and efficiency rows
/// ([`GraphTaskDispatcher::write_side_call_rows`]).
#[derive(Debug, Clone)]
pub(super) struct SideCall {
    provider_id: String,
    /// Slug the bridge launched.
    model_slug: String,
    /// Model the provider reported serving, when it named one.
    model_reported: Option<String>,
    usage: roko_core::Usage,
    /// Where `usage` came from (S01 §4.4): `estimated` for a call that
    /// streamed it and was cut off (gap-288e38).
    cost_source: CostSource,
    /// Model calls the agent reported, when it reported a count.
    turns: Option<u32>,
    duration_ms: u64,
    success: bool,
    /// Whether roko could price the call (backlog 2109).
    priced: bool,
}

impl SideCall {
    /// The call behind `dispatch`, which took `duration_ms`, in a run that
    /// prices from `snapshot`.
    pub(super) fn of(
        dispatch: &crate::dispatch_v2::AgentResultDispatch,
        duration_ms: u64,
        snapshot: Option<&PriceSnapshot>,
    ) -> Self {
        Self::from_result(
            &dispatch.target.provider_id,
            &dispatch.target.model_slug,
            dispatch.target.model_profile.as_ref(),
            is_cli_backend(dispatch.target.provider_kind),
            &dispatch.result,
            duration_ms,
            snapshot,
        )
    }

    /// The call of `result` to `model_slug` with `profile`, on a CLI agent
    /// backend when `cli_backend`, in a run that prices from `snapshot`.
    fn from_result(
        provider_id: &str,
        model_slug: &str,
        profile: Option<&roko_core::config::schema::ModelProfile>,
        cli_backend: bool,
        result: &roko_agent::AgentResult,
        duration_ms: u64,
        snapshot: Option<&PriceSnapshot>,
    ) -> Self {
        Self {
            provider_id: provider_id.to_string(),
            model_slug: model_slug.to_string(),
            model_reported: result
                .usage_obs
                .as_ref()
                .and_then(|usage| usage.model.clone()),
            usage: result.usage,
            cost_source: result
                .usage_obs
                .as_ref()
                .map_or(CostSource::Unknown, |usage| {
                    CostSource::from_usage_source(&usage.source, cli_backend)
                }),
            turns: result
                .output
                .tag("num_turns")
                .and_then(|turns| turns.parse().ok()),
            duration_ms,
            success: result.success,
            priced: crate::dispatch_v2::usage_is_priced(
                &result.usage,
                snapshot,
                profile,
                model_slug,
            ),
        }
    }

    /// `row` with the model this call's provider reported serving.
    fn served_row<T>(&self, row: T) -> ExecutedRow<T> {
        ExecutedRow {
            row,
            model_reported: self.model_reported.clone(),
            model_mismatch: self
                .model_reported
                .as_deref()
                .is_some_and(|reported| !same_model(&self.model_slug, reported)),
            models_reported: Vec::new(),
            substituted_from: None,
            substitution_reason: None,
            turns_unknown: self.turns.is_none(),
        }
    }
}

/// The helper calls of one attempt: the helper agents still out, and the
/// calls they completed.
#[derive(Clone, Default)]
pub(super) struct HelperCalls(Arc<HelperCallsState>);

#[derive(Default)]
struct HelperCallsState {
    ledger: parking_lot::Mutex<HelperLedger>,
    /// Signalled when the last helper agent out is dropped.
    idle: tokio::sync::Notify,
}

#[derive(Default)]
struct HelperLedger {
    agents_out: usize,
    completed: Vec<SideCall>,
}

impl HelperCalls {
    /// Run `future`, an attempt's verify steps, with the helper agents it
    /// creates counting their calls here.
    pub(super) async fn scope<F: std::future::Future>(&self, future: F) -> F::Output {
        HELPER_CALLS.scope(self.clone(), future).await
    }

    /// The helper calls of the attempt being verified, when there is one.
    fn current() -> Option<Self> {
        HELPER_CALLS.try_with(Self::clone).ok()
    }

    fn agent_out(&self) {
        self.0.ledger.lock().agents_out += 1;
    }

    fn agent_back(&self) {
        let mut ledger = self.0.ledger.lock();
        ledger.agents_out = ledger.agents_out.saturating_sub(1);
        if ledger.agents_out == 0 {
            self.0.idle.notify_one();
        }
    }

    fn record(&self, call: SideCall) {
        self.0.ledger.lock().completed.push(call);
    }

    /// Wait up to `limit` for the helper agents still out, then take the
    /// calls completed so far. A call still running at `limit` is not
    /// counted: roko never saw its usage.
    async fn settle(&self, limit: std::time::Duration) -> Vec<SideCall> {
        let idle = async {
            loop {
                let agents_out = self.0.ledger.lock().agents_out;
                if agents_out == 0 {
                    return;
                }
                self.0.idle.notified().await;
            }
        };
        if tokio::time::timeout(limit, idle).await.is_err() {
            tracing::warn!(
                limit_ms = u64::try_from(limit.as_millis()).unwrap_or(u64::MAX),
                "a helper model call outlived its attempt; its usage is not recorded"
            );
        }
        std::mem::take(&mut self.0.ledger.lock().completed)
    }
}

/// The cheap helper agent ([`CheapFactoryAgent`], which it derefs to),
/// counting each call that reaches a provider toward the attempt whose
/// verify steps are settling, when there is one.
pub(super) struct HelperAgent {
    agent: CheapFactoryAgent,
    provider_id: String,
    model_slug: String,
    /// The helper model's profile, which prices its calls.
    model_profile: Option<roko_core::config::schema::ModelProfile>,
    /// The run's price snapshot, which prices them first (backlog 2114).
    pricing_snapshot: Option<Arc<PriceSnapshot>>,
    /// The helper model runs on a CLI agent backend.
    cli_backend: bool,
    /// The attempt's helper calls; the agent is out until it is dropped.
    calls: Option<HelperCalls>,
}

impl HelperAgent {
    pub(super) fn new(
        agent: CheapFactoryAgent,
        target: crate::dispatch_v2::ProviderDispatchSpec,
        pricing_snapshot: Option<Arc<PriceSnapshot>>,
    ) -> Self {
        let calls = HelperCalls::current();
        if let Some(calls) = &calls {
            calls.agent_out();
        }
        Self {
            agent,
            cli_backend: is_cli_backend(target.provider_kind),
            provider_id: target.provider_id,
            model_slug: target.model_slug,
            model_profile: target.model_profile,
            pricing_snapshot,
            calls,
        }
    }
}

impl std::ops::Deref for HelperAgent {
    type Target = CheapFactoryAgent;

    fn deref(&self) -> &CheapFactoryAgent {
        &self.agent
    }
}

impl Drop for HelperAgent {
    fn drop(&mut self) {
        if let Some(calls) = &self.calls {
            calls.agent_back();
        }
    }
}

#[async_trait::async_trait]
impl roko_agent::Agent for HelperAgent {
    async fn run(&self, input: &Signal, ctx: &Context) -> roko_agent::AgentResult {
        let started = Instant::now();
        let result = roko_agent::Agent::run(&self.agent, input, ctx).await;
        // A result without a usage observation never reached a provider.
        if let Some(calls) = &self.calls
            && result.usage_obs.is_some()
        {
            calls.record(SideCall::from_result(
                &self.provider_id,
                &self.model_slug,
                self.model_profile.as_ref(),
                self.cli_backend,
                &result,
                u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
                self.pricing_snapshot.as_deref(),
            ));
        }
        result
    }

    fn name(&self) -> &str {
        roko_agent::Agent::name(&self.agent)
    }
}

impl GraphTaskDispatcher {
    /// Settle the helper calls of the attempt `attempt_key`: wait for the
    /// helper agents still out (bounded by `timeouts.llm_call_secs`), then
    /// account each completed call like a dispatch. Its spend goes on the
    /// task's and the plan's budgets, and a cost row and an efficiency row
    /// (`role = "helper"`) keyed by the attempt record it. Returns their
    /// totals for the attempt's verdict and episode.
    pub(super) async fn settle_helper_calls(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        attempt_key: &str,
        helpers: &HelperCalls,
    ) -> HelperCallsUsage {
        let limit = self
            .config
            .timeouts
            .llm_call()
            .saturating_add(HELPER_SETTLE_GRACE);
        let calls = helpers.settle(limit).await;
        let mut totals = HelperCallsUsage::default();
        for (index, call) in calls.iter().enumerate() {
            let cost_usd = f64::from(call.usage.cost_usd);
            totals.calls = totals.calls.saturating_add(1);
            totals.tokens_in += u64::from(call.usage.input_tokens);
            totals.tokens_out += u64::from(call.usage.output_tokens);
            totals.tokens_cache_read += u64::from(call.usage.cache_read_tokens);
            totals.cost_usd += cost_usd;
            if !call.usage.has_known_cost() {
                totals.unpriced_calls = totals.unpriced_calls.saturating_add(1);
            }

            self.record_task_spend(&spec.plan_id, &task.id, &call.usage);
            if let Err(error) = self.budget_ledger.settle(&spec.plan_id, 0, cost_usd) {
                tracing::warn!(
                    plan_id = %spec.plan_id,
                    task_id = %task.id,
                    %error,
                    "helper call spend not recorded on the plan's cost ledger"
                );
            }
            self.write_side_call_rows(
                spec,
                &task.id,
                attempt_key,
                &format!("{attempt_key}/helper-{}", index + 1),
                HELPER_ROLE,
                call,
            )
            .await;
        }
        totals
    }

    /// The cost row and efficiency row of one side call of task `task_id`'s
    /// attempt `attempt_key`, with `role`. `attempt_id` names the efficiency
    /// row uniquely and joins it to the attempt's dispatch row.
    pub(super) async fn write_side_call_rows(
        &self,
        spec: &TaskExecutionSpec,
        task_id: &str,
        attempt_key: &str,
        attempt_id: &str,
        role: &str,
        call: &SideCall,
    ) {
        let timestamp = chrono::Utc::now().to_rfc3339();
        let (input_tokens, output_tokens) = (
            u64::from(call.usage.input_tokens),
            u64::from(call.usage.output_tokens),
        );
        let cost_usd = f64::from(call.usage.cost_usd);
        let mut lines = Vec::new();
        if let Some(path) = &self.feedback.costs_path {
            let record = CostRecord {
                timestamp: timestamp.clone(),
                model: call.model_slug.clone(),
                provider: call.provider_id.clone(),
                role: role.to_string(),
                plan_id: spec.plan_id.clone(),
                task_id: task_id.to_string(),
                complexity_band: spec.tier.clone(),
                input_tokens,
                output_tokens,
                cached_tokens: u64::from(call.usage.cache_read_tokens),
                cost_usd,
                duration_ms: call.duration_ms,
                success: call.success,
                session_id: String::new(),
                cost_source: call.cost_source,
                priced: Some(call.priced),
            };
            let row = AttemptKeyed {
                attempt_key: attempt_key.to_string(),
                row: call.served_row(&record),
            };
            lines.push((path, serde_json::to_string(&row)));
        }
        if let Some(path) = &self.feedback.efficiency_path {
            let event = roko_learn::efficiency::AgentEfficiencyEvent {
                agent_id: format!("{}/{task_id}", spec.plan_id),
                role: role.to_string(),
                backend: call.provider_id.clone(),
                model: call.model_slug.clone(),
                plan_id: spec.plan_id.clone(),
                task_id: task_id.to_string(),
                attempt_id: attempt_id.to_string(),
                input_tokens,
                output_tokens,
                cache_read_tokens: u64::from(call.usage.cache_read_tokens),
                cache_write_tokens: u64::from(call.usage.cache_create_tokens),
                cost_usd,
                cost_usd_without_cache: cost_usd,
                total_prompt_tokens: input_tokens,
                wall_time_ms: call.duration_ms,
                duration_ms: call.duration_ms,
                iteration: call.turns.unwrap_or(0),
                turn_number: call.turns.unwrap_or(0),
                outcome: if call.success { "success" } else { "failure" }.to_string(),
                model_used: call.model_slug.clone(),
                frequency: roko_core::OperatingFrequency::Gamma,
                timestamp,
                ..Default::default()
            };
            let row = AttemptKeyed {
                attempt_key: attempt_key.to_string(),
                row: call.served_row(&event),
            };
            lines.push((path, serde_json::to_string(&row)));
        }
        for (path, line) in lines {
            let written = match line {
                Ok(line) => append_jsonl_line_async(path.clone(), line)
                    .await
                    .map_err(|error| error.to_string()),
                Err(error) => Err(error.to_string()),
            };
            if let Err(error) = written {
                tracing::warn!(
                    plan_id = %spec.plan_id,
                    task_id,
                    role,
                    path = %path.display(),
                    %error,
                    "side call row not written (best-effort)"
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use roko_core::agent::ProviderKind;
    use roko_core::config::schema::{ModelProfile, ProviderConfig};
    use tempfile::tempdir;

    use super::*;
    use crate::graph_task_dispatch::tests::{
        FIXTURE_PROVIDER_TIMEOUT_MS, VERIFY_PROVIDER, final_turn, jsonl_rows_where, make_spec,
        make_test_dispatcher, no_auto_fix, recording_feedback, spawn_openai_mock, verify_step,
    };

    const RUN: &str = "graph-helper-run";

    /// A dispatcher whose helper model, `helper-1` on an OpenAI-compatible
    /// mock ($1 in and $2 out per 1M tokens), answers up to three calls, one
    /// more than the error diagnosis and the gate reflection make, so a stray
    /// call is seen; its task's verify step fails on its first run and
    /// passes on its second. Returns the requests the helper model saw.
    async fn helper_fixture(
        temp: &tempfile::TempDir,
        feedback: GraphFeedbackContext,
    ) -> (
        Arc<GraphTaskDispatcher>,
        TaskDef,
        Arc<parking_lot::Mutex<Vec<serde_json::Value>>>,
    ) {
        let mut answer = final_turn("0.5");
        answer["model"] = serde_json::json!("helper-1");
        answer["usage"] = serde_json::json!({ "prompt_tokens": 100, "completion_tokens": 20, "total_tokens": 120 });
        let (base_url, requests) = spawn_openai_mock(vec![answer; 3]);
        let feedback = GraphFeedbackContext {
            replan_on_gate_failure: true,
            post_gate_reflection_path: Some(temp.path().join(".roko/learn/reflections.json")),
            ..feedback
        };
        let (dispatcher, mut task) = make_test_dispatcher(
            temp,
            VERIFY_PROVIDER,
            |config| {
                no_auto_fix(config);
                // The helper model: $1 in and $2 out per 1M tokens.
                config.providers.insert(
                    "helper_api".to_string(),
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
                    },
                );
                config.models.insert(
                    "helper-model".to_string(),
                    ModelProfile {
                        provider: "helper_api".to_string(),
                        slug: "helper-1".to_string(),
                        context_window: 128_000,
                        max_output: Some(1_024),
                        max_tools: Some(32),
                        supports_tools: true,
                        tool_format: "openai_json".to_string(),
                        cost_input_per_m: Some(1.0),
                        cost_output_per_m: Some(2.0),
                        ..ModelProfile::default()
                    },
                );
                config.routing.fast_task_model = "helper-model".to_string();
            },
            feedback,
        )
        .await;
        // Fails on its first run, passes on its second.
        task.verify = vec![verify_step(
            "check",
            "test -f passed-once || { touch passed-once; exit 1; }",
        )];
        (dispatcher, task, requests)
    }

    /// A task fails its gate once, then passes. After the failure the error
    /// diagnosis and the gate reflection each call the helper model once;
    /// both calls are on the first attempt's cost and efficiency rows,
    /// verdict and episode, and on the plan's spend.
    #[tokio::test]
    async fn helper_calls_after_a_failed_gate_are_costed() {
        let temp = tempdir().expect("tempdir");
        let (dispatcher, task, requests) =
            helper_fixture(&temp, recording_feedback(temp.path())).await;
        let spec = make_spec(&task);
        let ctx = CellContext::new().with_run_id(RUN.to_string());
        dispatcher
            .dispatch(&spec, Vec::new(), &ctx)
            .await
            .expect_err("the first attempt fails its gate");
        dispatcher
            .dispatch(&spec, Vec::new(), &ctx)
            .await
            .expect("the second attempt passes");
        assert_eq!(requests.lock().len(), 2, "the helper model saw two calls");
        // Two agent calls at $0.01, two helper calls at $0.00014.
        let spent = dispatcher.plan_budget_snapshot(&spec.plan_id).spent_usd;
        assert!((spent - 0.020_28).abs() < 1e-5, "{spent}");
        drop(dispatcher);

        let failed_key = format!("{RUN}:{}:{}:1", spec.plan_id, task.id);
        let is_helper = |row: &serde_json::Value| row["role"] == HELPER_ROLE;
        let costs =
            jsonl_rows_where(&temp.path().join(".roko/learn/costs.jsonl"), 2, is_helper).await;
        assert_eq!(costs.len(), 2);
        for row in &costs {
            assert_eq!(row["attempt_key"], failed_key.as_str());
            assert_eq!(row["model"], "helper-1");
            assert_eq!(row["provider"], "helper_api");
            assert_eq!(row["model_reported"], "helper-1");
            assert_eq!(row["input_tokens"], 100);
            let cost = row["cost_usd"].as_f64().expect("cost");
            assert!((cost - 0.000_14).abs() < 1e-7, "{row}");
        }
        let efficiency = jsonl_rows_where(
            &temp.path().join(".roko/learn/efficiency.jsonl"),
            2,
            |row| {
                row["schema"] == roko_learn::efficiency::AGENT_EFFICIENCY_EVENT_SCHEMA
                    && is_helper(row)
            },
        )
        .await;
        let mut ids: Vec<&str> = efficiency
            .iter()
            .map(|row| row["attempt_id"].as_str().unwrap_or_default())
            .collect();
        ids.sort_unstable();
        let expected: Vec<String> = (1..=2)
            .map(|n| format!("{failed_key}/helper-{n}"))
            .collect();
        assert_eq!(ids, expected);

        let verdicts = jsonl_rows_where(
            &temp
                .path()
                .join(".roko/runs")
                .join(RUN)
                .join("attempts.jsonl"),
            2,
            |row| row["schema_version"] == "roko.verdict/1",
        )
        .await;
        assert_eq!(verdicts[0]["helpers"]["calls"], 2);
        assert_eq!(verdicts[0]["helpers"]["tokens_in"], 200);
        assert!(
            verdicts[1].get("helpers").is_none(),
            "the pass made no helper call"
        );

        let episodes = roko_learn::episode_logger::EpisodeLogger::read_all(
            &temp.path().join(".roko/episodes.jsonl"),
        )
        .await
        .expect("episodes");
        let failed = episodes
            .iter()
            .find(|episode| !episode.success)
            .expect("failed attempt");
        assert_eq!(failed.extra["helper_calls"], 2);
        assert_eq!(failed.extra["helper_tokens_in"], 200);
        let helper_cost = failed.extra["helper_cost_usd"]
            .as_f64()
            .expect("helper cost");
        assert!((helper_cost - 0.000_28).abs() < 1e-7, "{helper_cost}");
        let passed = episodes
            .iter()
            .find(|episode| episode.success)
            .expect("passed attempt");
        assert!(!passed.extra.contains_key("helper_calls"));
    }

    /// Helper calls give the cascade router no credit (bug-b8af02). The
    /// router learns from each attempt's settled verdict alone
    /// (`RoutingObservationSink`), and the provider bridge keeps no router
    /// of its own, so after a failed gate and its two helper calls it
    /// holds the attempt's failure and nothing for the helper model.
    #[tokio::test]
    async fn helper_calls_give_the_cascade_router_no_credit() {
        let temp = tempdir().expect("tempdir");
        let router = Arc::new(roko_learn::cascade_router::CascadeRouter::new(vec![
            "claude-sonnet-4-6".into(),
            "helper-1".into(),
        ]));
        let facade = crate::runtime_feedback::FeedbackFacade::new().with_sink(Arc::new(
            crate::runtime_feedback::RoutingObservationSink::new(Arc::clone(&router)),
        ));
        let feedback = GraphFeedbackContext {
            feedback_facade: Some(Arc::new(facade)),
            ..recording_feedback(temp.path())
        };
        let (dispatcher, task, requests) = helper_fixture(&temp, feedback).await;
        dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &CellContext::new())
            .await
            .expect_err("the first attempt fails its gate");
        assert_eq!(requests.lock().len(), 2, "the helper model saw two calls");

        let snapshot = router.confidence_snapshot();
        assert_eq!(
            snapshot.get("helper-1").copied().unwrap_or_default(),
            (0, 0),
            "a helper call is not a routing trial"
        );
        assert_eq!(
            snapshot.get("claude-sonnet-4-6").copied(),
            Some((1, 0)),
            "the attempt's settled failure is"
        );
        assert!(
            !temp.path().join(".roko/learn/cascade-router.json").exists(),
            "the provider bridge trained a router of its own"
        );
        let helper_costs =
            jsonl_rows_where(&temp.path().join(".roko/learn/costs.jsonl"), 2, |row| {
                row["role"] == HELPER_ROLE
            })
            .await;
        assert_eq!(helper_costs.len(), 2, "the helper calls stay helper rows");
    }

    /// Nothing an attempt's verify settles feeds the gate-gaming detector
    /// (S05 F1). A task fails its gate once, then passes, with a helper
    /// model set: the helper model sees the error diagnosis and the gate
    /// reflection, no quality judge asks it to rate the attempt, and no
    /// alerts file appears.
    #[tokio::test]
    async fn verify_feeds_the_gaming_detector_nothing() {
        let temp = tempdir().expect("tempdir");
        let (dispatcher, task, requests) =
            helper_fixture(&temp, recording_feedback(temp.path())).await;
        let spec = make_spec(&task);
        let ctx = CellContext::new().with_run_id(RUN.to_string());
        dispatcher
            .dispatch(&spec, Vec::new(), &ctx)
            .await
            .expect_err("the first attempt fails its gate");
        dispatcher
            .dispatch(&spec, Vec::new(), &ctx)
            .await
            .expect("the second attempt passes");
        drop(dispatcher);

        let seen = requests.lock();
        assert_eq!(seen.len(), 2, "the diagnosis and the reflection alone");
        for request in seen.iter() {
            let body = request.to_string();
            assert!(
                !body.contains("Rate the quality of this response"),
                "{body}"
            );
        }
        let alerts = temp.path().join(".roko/learn/gate-gaming-alerts.jsonl");
        assert!(!alerts.exists(), "{} was written", alerts.display());
    }
}
