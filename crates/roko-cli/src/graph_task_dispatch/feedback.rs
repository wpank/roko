//! The learning/feedback context of a Graph plan run and the feedback each task
//! attempt emits once its outcome is settled.

use roko_agent::safety::provenance_sink::arguments_digest;
use roko_agent::safety::{
    ProvenanceCall, ProvenanceOutcome, ProvenanceVerdict, SafetyProvenanceSink,
};
use roko_core::ContentHash;
use roko_core::extension::CamelTaintLevel;

use super::tui_forward::append_jsonl_line_async;
use super::verification::verify_step_label;
use super::*;
use crate::dispatch_v2::ToolCallRecord;

/// Learning/feedback subsystem context for the Graph engine.
///
/// Constructed once in `cmd_plan_run_engine()` and shared by all tasks in
/// the plan run. Each subsystem is optional so the dispatcher degrades
/// gracefully when a component cannot be initialized.
#[derive(Clone)]
pub struct GraphFeedbackContext {
    /// Feedback facade that fans task-completion events to episode and routing sinks.
    pub feedback_facade: Option<Arc<FeedbackFacade>>,
    /// Path to `.roko/learn/efficiency.jsonl` for efficiency event writes.
    pub efficiency_path: Option<PathBuf>,
    /// Path to `.roko/learn/costs.jsonl` for per-task cost record writes.
    ///
    /// When set, each completed dispatch appends one [`roko_learn::costs_db::CostRecord`]
    /// so that `roko status` cost summary reads from the same source as the
    /// efficiency events produced by the Graph engine.
    pub costs_path: Option<PathBuf>,
    /// Path to `.roko/learn/playbooks/` for playbook outcome recording.
    pub playbook_dir: Option<PathBuf>,
    /// Shared daimon affect state, loaded from `.roko/daimon/state.json`.
    pub daimon_state: Option<Arc<std::sync::Mutex<roko_daimon::DaimonState>>>,
    /// Path to `.roko/learn/experiments.json` for experiment settlement.
    pub experiment_store_path: Option<PathBuf>,
    /// Path to `.roko/learn/gate-failures.jsonl` for structured gate failure records.
    pub gate_failures_path: Option<PathBuf>,
    /// Path to `.roko/learn/post-gate-reflections.json` for LLM-generated gate reflection store.
    pub post_gate_reflection_path: Option<PathBuf>,
    /// Whether gate failure replanning is enabled (`learning.replan_on_gate_failure`).
    pub replan_on_gate_failure: bool,
    /// P0-04: CodingOracle for post-gate build/test observations.
    pub coding_oracle: Option<Arc<CodingOracle>>,
    /// P1-04: HoldoutExperiment for gating learning updates (80/20 train/holdout split).
    pub holdout_experiment: Option<Arc<tokio::sync::Mutex<roko_learn::HoldoutExperiment>>>,
    /// P2-01: ShadowRunner for recording shadow dispatch decisions.
    pub shadow_runner: Option<Arc<ShadowRunner>>,
    /// P2-LRN-6 Loop 1: Path to `.roko/learn/gate-thresholds.json` for
    /// adaptive EMA threshold updates after each verify run.
    ///
    /// When set, each verify step outcome is fed into `GateThresholds::observe_with_alpha`
    /// so the EMA pass-rate converges toward the workspace's real gate history.
    /// The file is written atomically after every task's verify sequence
    /// completes (both pass and fail), and a `GateThresholdsUpdated` event is
    /// published to the TUI bridge.
    pub gate_thresholds_path: Option<PathBuf>,

    /// RAG-10: Path to `.roko/learn/retrieval-outcomes.jsonl`.
    ///
    /// When set, each task dispatch appends one pre-gate
    /// [`roko_learn::retrieval_outcome::RetrievalOutcomeRecord`] immediately
    /// after prompt assembly (strategy + result count known, gate unknown), and
    /// a second settled record once all verify steps complete so the gate-pass
    /// correlation is durably captured.
    pub retrieval_outcomes_path: Option<PathBuf>,

    /// S01: `.roko/runs/`, where each run's `<run_id>/attempts.jsonl` records
    /// every attempt's open line and settled verdict.
    ///
    /// Attempt ordinals continue from that file, so a resumed run never
    /// reuses an attempt key. When unset, ordinals live in memory and no
    /// attempt is recorded.
    pub runs_dir: Option<PathBuf>,

    /// The safety provenance sinks of the runs in flight (gap-ca8022): a
    /// CLI provider's dispatch turn leaves one record with its run's sink.
    pub provenance_sinks: Option<crate::safety_provenance::ProvenanceSinks>,
}

/// A `costs.jsonl` row with its attempt's settled verdict beside it: the
/// outcome and the learning label, `null` when the attempt teaches nothing
/// (S01 §4.3). The row's own `success` keeps its meaning, which `roko
/// status` and `roko show costs` read. `R` is the [`CostRecord`] with the
/// verdict's executed-model columns ([`roko_learn::efficiency::ExecutedRow`]);
/// the record's `cost_source` is the verdict's `cost.source`.
#[derive(serde::Serialize)]
struct SettledCostRow<R> {
    outcome: AttemptOutcome,
    learning_label: Option<u8>,
    #[serde(flatten)]
    row: R,
}

impl std::fmt::Debug for GraphFeedbackContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GraphFeedbackContext")
            .field("feedback_facade", &self.feedback_facade.is_some())
            .field("efficiency_path", &self.efficiency_path)
            .field("costs_path", &self.costs_path)
            .field("playbook_dir", &self.playbook_dir)
            .field("daimon_state", &self.daimon_state.is_some())
            .field("experiment_store_path", &self.experiment_store_path)
            .field("gate_failures_path", &self.gate_failures_path)
            .field("post_gate_reflection_path", &self.post_gate_reflection_path)
            .field("replan_on_gate_failure", &self.replan_on_gate_failure)
            .field("coding_oracle", &self.coding_oracle.is_some())
            .field("holdout_experiment", &self.holdout_experiment.is_some())
            .field("shadow_runner", &self.shadow_runner.is_some())
            .field("gate_thresholds_path", &self.gate_thresholds_path)
            .field("retrieval_outcomes_path", &self.retrieval_outcomes_path)
            .field("runs_dir", &self.runs_dir)
            .field("provenance_sinks", &self.provenance_sinks.is_some())
            .finish()
    }
}

impl Default for GraphFeedbackContext {
    fn default() -> Self {
        Self {
            feedback_facade: None,
            efficiency_path: None,
            costs_path: None,
            playbook_dir: None,
            daimon_state: None,
            experiment_store_path: None,
            gate_failures_path: None,
            post_gate_reflection_path: None,
            replan_on_gate_failure: false,
            coding_oracle: None,
            holdout_experiment: None,
            shadow_runner: None,
            gate_thresholds_path: None,
            retrieval_outcomes_path: None,
            runs_dir: None,
            provenance_sinks: None,
        }
    }
}

impl GraphTaskDispatcher {
    /// Emit all feedback events after a task dispatch completes.
    ///
    /// This is the Graph engine equivalent of Runner-v2's post-dispatch
    /// feedback pipeline. Each subsystem is best-effort: failures are logged
    /// but do not block the task result.
    ///
    /// `settled` is the attempt's settlement ([`AttemptContext::settle`]).
    /// Learners read only its learning label (S01 §4.1): the router, through
    /// the facade's routing sink, the playbooks, the daimon, the prompt
    /// experiments and durable knowledge, which only a pass grows
    /// ([`FeedbackEvent::TaskVerified`]). An attempt without a label
    /// (unverified, provider or harness failures) updates none of them. The
    /// analysis rows record every attempt under its attempt key: the
    /// episode, with the bounded class-prefixed failure reason and the
    /// provider-reported turn count, and the efficiency and cost rows. Their
    /// success flag keeps its meaning (the provider call succeeded and no
    /// verify step failed); the cost row adds the verdict's outcome and
    /// label. The settlement itself goes out as
    /// [`FeedbackEvent::AttemptSettled`].
    ///
    /// [`AttemptContext::settle`]: super::attempt::AttemptContext::settle
    pub(super) async fn emit_feedback(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        settled: &SettledAttempt,
        dispatch: &crate::dispatch_v2::AgentResultDispatch,
        wall_duration: std::time::Duration,
        dispatch_plan: &crate::dispatch::RunnerDispatchPlan,
        routing_context: Option<roko_learn::model_router::RoutingContext>,
    ) {
        let succeeded = settled.succeeded();
        // What learners record: a pass, a failure, or nothing.
        let learning = settled.learning_success();
        let failure_reason = settled.failure_reason.clone();
        let attempt_key = settled.attempt_key();
        let role = task.role.as_deref().unwrap_or("implementer");
        // P3-02: Agent turns as the agent reported them (the Claude CLI's
        // `num_turns`, the model calls of roko's tool loop), so episodes and
        // efficiency records carry real counts. 0 when it did not say: the
        // count is unknown, not one turn (bug-55fd84).
        let agent_num_turns = dispatch
            .events
            .iter()
            .rev()
            .find_map(|ev| match ev {
                roko_agent::AgentRuntimeEvent::TurnCompleted { num_turns, .. } => *num_turns,
                _ => None,
            })
            .unwrap_or(0);
        let provider_id = &dispatch.target.provider_id;
        let model_slug = &dispatch.target.model_slug;
        let cost_usd = f64::from(dispatch.result.usage.cost_usd);
        let tokens_in = u64::from(dispatch.result.usage.input_tokens);
        let tokens_out = u64::from(dispatch.result.usage.output_tokens);
        let duration_ms = wall_duration.as_millis() as u64;

        // Accumulate run-level aggregates for RunMetricsRecord (#169).
        self.agg_tokens_in.fetch_add(tokens_in, Ordering::Relaxed);
        self.agg_tokens_out.fetch_add(tokens_out, Ordering::Relaxed);
        self.agg_dispatch_count.fetch_add(1, Ordering::Relaxed);

        // Determine model choice source for feedback routing.
        let model_source = if dispatch_plan.forced {
            ModelChoiceSource::Override
        } else if self.cli_model_override.is_some() {
            ModelChoiceSource::Override
        } else if task.model_hint.is_some() {
            ModelChoiceSource::TaskHint
        } else {
            ModelChoiceSource::Router
        };
        let experiment_settlement = prompt_experiment::settlement(learning);
        let diagnostics = &dispatch_plan.prompt.diagnostics;

        // ── W04: FeedbackFacade (episodes + routing + knowledge) ─────────
        if let Some(facade) = &self.feedback.feedback_facade {
            let outcome = crate::dispatch::AgentOutcome {
                task_id: task.id.clone(),
                plan_id: spec.plan_id.clone(),
                model: model_slug.clone(),
                provider: provider_id.clone(),
                output: dispatch
                    .result
                    .output
                    .body
                    .as_text()
                    .ok()
                    .unwrap_or("")
                    .chars()
                    .take(2048)
                    .collect(),
                tokens_in,
                tokens_out,
                cost_usd,
                duration_ms,
                exit_code: if succeeded { Some(0) } else { Some(1) },
                is_error: !succeeded,
            };
            let event = FeedbackEvent::TaskCompleted {
                plan_id: spec.plan_id.clone(),
                task_id: task.id.clone(),
                outcome,
                model_source,
                succeeded,
                routing_context,
                prompt_text: Some(dispatch_plan.prompt.system_prompt.clone()),
                cache_read_tokens: u64::from(dispatch.result.usage.cache_read_tokens),
                knowledge_ids: diagnostics.knowledge_ids.clone(),
                playbook_ids: diagnostics.playbook_ids.clone(),
                initial_model: model_slug.clone(),
                turns: u64::from(agent_num_turns),
                failure_reason,
                settled: Some(Arc::clone(&settled.verdict)),
            };
            if let Err(error) = facade.on_event(&event).await {
                tracing::warn!(
                    plan_id = %spec.plan_id,
                    task_id = %task.id,
                    %error,
                    "graph feedback facade error (best-effort)"
                );
            }

            // Only a pass carries the learning label 1: every authored
            // verify step passed (`TaskGateVerdict::Passed`), or failed only
            // on tests that failed before the run too
            // (`PassedWithPreexistingFailures`). Only those grow durable
            // knowledge.
            if learning == Some(true) {
                let verified = crate::runtime_feedback::VerifiedAttempt {
                    plan_id: spec.plan_id.clone(),
                    task_id: task.id.clone(),
                    attempt_id: attempt_key.to_string(),
                    title: task.title.clone(),
                    task_type: task.tier.clone(),
                    role: role.to_string(),
                    model: model_slug.clone(),
                    files: task.files.clone(),
                    verify_steps: task
                        .verify
                        .iter()
                        .enumerate()
                        .map(|(index, step)| {
                            (verify_step_label(index, &step.phase), step.command.clone())
                        })
                        .collect(),
                    knowledge_ids: diagnostics.knowledge_ids.clone(),
                    agent_output: dispatch
                        .result
                        .output
                        .body
                        .as_text()
                        .unwrap_or_default()
                        .to_string(),
                };
                if let Err(error) = facade
                    .on_event(&FeedbackEvent::TaskVerified(verified))
                    .await
                {
                    tracing::warn!(
                        plan_id = %spec.plan_id,
                        task_id = %task.id,
                        %error,
                        "graph verified-knowledge feedback error (best-effort)"
                    );
                }
            }
        }
        self.publish_settlement(spec, task, settled).await;
        self.record_cli_turn_provenance(settled, dispatch).await;

        // ── W05: Efficiency event ────────────────────────────────────────
        if let Some(eff_path) = &self.feedback.efficiency_path {
            // Gather prompt diagnostics for the efficiency event so
            // telemetry reflects what the agent actually received.
            let eff_prompt_sections = efficiency_prompt_sections(&dispatch_plan.prompt.diagnostics);
            let eff_system_prompt_tokens = dispatch_plan.prompt.diagnostics.estimated_tokens;
            let live_tool_calls = settled.live_tool_calls.finish().await;
            let eff_tool_calls =
                efficiency_tool_calls(&dispatch.events, &live_tool_calls, &dispatch.tool_calls);
            let eff_tools_used = eff_tool_calls.len() as u32;
            // The tools the contract allows, or 0 (unknown) when the
            // provider's own tool set applies (gap-7a8474).
            let eff_tools_available = effective_agent_contract(role, task, &self.config)
                .allowed_tools
                .map_or(0, |tools| tools.len() as u32);
            // Without pricing for the model the saving is unknown, and the
            // row records none; a price list never undercuts the reported cost.
            let eff_cost_without_cache = crate::dispatch_v2::usage_cost_without_cache(
                &dispatch.result.usage,
                dispatch.target.model_profile.as_ref(),
                &dispatch.target.model_slug,
            )
            .map_or(cost_usd, |uncached| uncached.max(cost_usd));
            // The task's first attempt, or a retry of a failed one, which
            // a replan follows when gate failures trigger one.
            let eff_strategy = if settled.key().attempt <= 1 {
                "initial"
            } else if self.feedback.replan_on_gate_failure {
                "replan"
            } else {
                "retry"
            };
            let event = roko_learn::efficiency::AgentEfficiencyEvent {
                agent_id: format!("{}/{}", spec.plan_id, task.id),
                role: role.to_string(),
                backend: provider_id.clone(),
                model: model_slug.clone(),
                plan_id: spec.plan_id.clone(),
                task_id: task.id.clone(),
                attempt_id: attempt_key.to_string(),
                input_tokens: tokens_in,
                output_tokens: tokens_out,
                reasoning_tokens: reported_reasoning_tokens(
                    &dispatch.result.usage,
                    dispatch.result.usage_obs.as_ref(),
                    &dispatch.events,
                ),
                cache_read_tokens: u64::from(dispatch.result.usage.cache_read_tokens),
                cache_write_tokens: u64::from(dispatch.result.usage.cache_create_tokens),
                cost_usd,
                cost_usd_without_cache: eff_cost_without_cache,
                prompt_sections: eff_prompt_sections,
                total_prompt_tokens: tokens_in,
                system_prompt_tokens: u64::from(eff_system_prompt_tokens),
                tools_available: eff_tools_available,
                tools_used: eff_tools_used,
                tool_calls: eff_tool_calls,
                wall_time_ms: duration_ms,
                duration_ms,
                // 0 when no stream showed model output (gap-7a8474), which
                // the row marks `ttft_unknown`.
                time_to_first_token_ms: dispatch.result.ttft_ms.unwrap_or(0),
                // No provider process is pre-spawned or reused, so every
                // dispatch is a cold start.
                was_warm_start: false,
                iteration: agent_num_turns,
                turn_number: agent_num_turns,
                is_final_turn: true,
                // The attempt's one settled row carries its verify verdict;
                // no gate row follows it (backlog 2107).
                gate_passed: efficiency_gate_passed(settled.verdict.outcome),
                outcome: if succeeded {
                    "success".to_string()
                } else {
                    "failure".to_string()
                },
                gate_errors: failed_step_summaries(&settled.verdict.steps),
                model_used: model_slug.clone(),
                frequency: roko_core::OperatingFrequency::Gamma,
                strategy_attempted: eff_strategy.to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
            };
            let row = AttemptKeyed {
                attempt_key: attempt_key.to_string(),
                row: roko_learn::efficiency::TtftRow {
                    row: roko_learn::efficiency::ExecutedRow::new(
                        &event,
                        &settled.verdict.executed,
                    ),
                    ttft_unknown: dispatch.result.ttft_ms.is_none(),
                },
            };
            match serde_json::to_string(&row) {
                Ok(line) => {
                    let path = eff_path.clone();
                    let plan_id = spec.plan_id.clone();
                    let task_id = task.id.clone();
                    crate::background_writes::spawn(&eff_path, async move {
                        if let Err(error) = append_jsonl_line_async(path, line).await {
                            tracing::warn!(
                                plan_id = %plan_id,
                                task_id = %task_id,
                                %error,
                                "graph efficiency event write failed (best-effort)"
                            );
                        }
                    });
                }
                Err(error) => {
                    tracing::warn!(
                        plan_id = %spec.plan_id,
                        task_id = %task.id,
                        %error,
                        "graph efficiency event serialization failed (best-effort)"
                    );
                }
            }
        }

        // ── W05b: Cost record to costs.jsonl ─────────────────────────────
        //
        // `roko status` reads cost totals from `.roko/learn/costs.jsonl` via
        // `CostsLog::total_cost()`. The Graph engine only writes efficiency
        // events (above), so the status cost summary always showed $0.0000.
        // This block bridges the gap: one `CostRecord` per dispatch, written
        // asynchronously alongside the efficiency event.
        if let Some(costs_path) = &self.feedback.costs_path {
            let cost_record = CostRecord {
                timestamp: chrono::Utc::now().to_rfc3339(),
                model: model_slug.clone(),
                provider: provider_id.clone(),
                role: role.to_string(),
                plan_id: spec.plan_id.clone(),
                task_id: task.id.clone(),
                complexity_band: task.tier.clone(),
                input_tokens: tokens_in,
                output_tokens: tokens_out,
                cached_tokens: u64::from(dispatch.result.usage.cache_read_tokens),
                cost_usd,
                duration_ms,
                success: succeeded,
                session_id: String::new(),
                // `estimated` for usage a call streamed before it was
                // cancelled or timed out (bug-aa2044), so readers of
                // `costs.jsonl` show it apart (gap-288e38).
                cost_source: settled.verdict.cost.source,
                // An unknown cost reads as unknown, not as $0 (backlog 2109).
                priced: Some(crate::dispatch_v2::usage_is_priced(
                    &dispatch.result.usage,
                    dispatch.target.model_profile.as_ref(),
                    &dispatch.target.model_slug,
                )),
            };
            let row = AttemptKeyed {
                attempt_key: attempt_key.to_string(),
                row: SettledCostRow {
                    outcome: settled.verdict.outcome,
                    learning_label: settled.verdict.learning_label,
                    row: roko_learn::efficiency::ExecutedRow::new(
                        &cost_record,
                        &settled.verdict.executed,
                    ),
                },
            };
            match serde_json::to_string(&row) {
                Ok(line) => {
                    let path = costs_path.clone();
                    let plan_id = spec.plan_id.clone();
                    let task_id = task.id.clone();
                    crate::background_writes::spawn(&costs_path, async move {
                        if let Err(error) = append_jsonl_line_async(path, line).await {
                            tracing::warn!(
                                plan_id = %plan_id,
                                task_id = %task_id,
                                %error,
                                "graph cost record write failed (best-effort)"
                            );
                        }
                    });
                }
                Err(error) => {
                    tracing::warn!(
                        plan_id = %spec.plan_id,
                        task_id = %task.id,
                        %error,
                        "graph cost record serialization failed (best-effort)"
                    );
                }
            }
        }

        // ── W05c: Publish token usage and cost to the TUI dashboard ──────
        //
        // `token_usage` and `efficiency_event("cost_usd")` feed the snapshot's
        // per-agent counters and the overall `SnapshotStats` totals.  Kept
        // outside the `if let Some(costs_path)` block above so they fire even
        // when `.roko/learn/costs.jsonl` is not configured.  The fold in
        // `DashboardSnapshot::apply` attributes the numbers to the agent that
        // `forward_dispatch_events_to_tui` announced for the task, so publish
        // after that call.
        if let Some(tui) = &self.tui_bridge {
            tui.token_usage(
                &spec.plan_id,
                &task.id,
                tokens_in,
                tokens_out,
                u64::from(dispatch.result.usage.cache_read_tokens),
                u64::from(dispatch.result.usage.cache_create_tokens),
            );
            tui.efficiency_event(&spec.plan_id, &task.id, "cost_usd", cost_usd);
        }

        // ── W07: Playbook outcome recording ──────────────────────────────
        //
        // Credit the playbooks prompt assembly actually injected with the
        // attempt's learning label; an attempt without one credits none.
        if let (Some(playbook_dir), Some(success)) = (&self.feedback.playbook_dir, learning) {
            let store = roko_learn::playbook::PlaybookStore::new(playbook_dir);
            for playbook_id in &diagnostics.playbook_ids {
                if let Err(error) = store.record_outcome(playbook_id, success).await {
                    tracing::warn!(
                        plan_id = %spec.plan_id,
                        task_id = %task.id,
                        %playbook_id,
                        %error,
                        "graph playbook outcome recording failed (best-effort)"
                    );
                }
            }
        }

        // ── W09: DaimonState affect feedback ─────────────────────────────
        //
        // Only an attempt with a learning label moves affect, and none does
        // while learning is frozen (decision 2218).
        if let (Some(daimon), Some(success)) = (&self.feedback.daimon_state, learning)
            && !self.learning_frozen()
        {
            use roko_daimon::AffectEngine;
            let event = roko_daimon::AffectEvent::TaskOutcome {
                task_id: task.id.clone(),
                succeeded: success,
            };
            if let Ok(mut state) = daimon.lock() {
                let _ = state.appraise(event);
            }
        }

        // ── W14: Experiment settlement ───────────────────────────────────
        //
        // Settles this attempt's prompt treatments (prepared at prompt
        // assembly, bound to the launched prompt) with its learning label;
        // an attempt without one abandons them.
        if let Some(store_path) = &self.feedback.experiment_store_path {
            prompt_experiment::settle(
                store_path,
                settled.key().to_prompt_attempt_key(),
                experiment_settlement,
            )
            .await;
        }
    }

    /// Publish an attempt's settlement through the feedback facade as
    /// [`FeedbackEvent::AttemptSettled`]. The facade delivers one settlement
    /// per attempt. First, the T0 reflex rule that served the attempt, if
    /// one did, learns from it ([`Self::credit_reflex_rule`]), and the
    /// settlement counts toward the task's standing on the model ladder
    /// (gap-460230).
    /// Record a CLI provider's dispatch turn with its run's safety
    /// provenance sink (gap-ca8022). A CLI agent such as the Claude CLI runs
    /// its tools itself, outside roko's tool dispatcher, so nothing records
    /// an intent before they run. The turn instead leaves one outcome
    /// afterwards: whether it succeeded, and a keyed digest of the tool calls
    /// its live output showed. A failed write is logged: the tools have run.
    async fn record_cli_turn_provenance(
        &self,
        settled: &SettledAttempt,
        dispatch: &crate::dispatch_v2::AgentResultDispatch,
    ) {
        use roko_core::agent::ProviderKind;

        let cli = matches!(
            dispatch.target.provider_kind,
            ProviderKind::ClaudeCli | ProviderKind::CodexCli
        );
        let Some(sinks) = self.feedback.provenance_sinks.as_ref().filter(|_| cli) else {
            return;
        };
        let Some(sink) = sinks.for_run(&settled.key().run_id) else {
            return;
        };
        let calls = settled.live_tool_calls.finish().await;
        let outcome = cli_turn_outcome(sink.as_ref(), settled, dispatch, &calls);
        if let Err(error) = sink.record_outcome(&outcome) {
            tracing::warn!(
                attempt_key = settled.attempt_key(),
                %error,
                "safety provenance: the CLI turn was not recorded"
            );
        }
    }

    pub(super) async fn publish_settlement(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        settled: &SettledAttempt,
    ) {
        // Spend the attempt settled on any path, its helper calls included,
        // raises the plan's budget alerts it crossed (backlog 2116).
        self.announce_budget_alerts(&spec.plan_id);
        self.credit_reflex_rule(spec, task, settled).await;
        self.note_ladder_outcome(spec, task, settled);
        let Some(facade) = &self.feedback.feedback_facade else {
            return;
        };
        let event = FeedbackEvent::AttemptSettled(Arc::clone(&settled.verdict));
        if let Err(error) = facade.on_event(&event).await {
            tracing::warn!(
                plan_id = %spec.plan_id,
                task_id = %task.id,
                %error,
                "graph attempt settlement feedback error (best-effort)"
            );
        }
    }
}

/// The prompt sections an efficiency row lists (backlog 2108). With the
/// composition manifest: each included section with the composer's token
/// estimate, ranked by inclusion order, then each section the budget
/// dropped. Without one only the included names are known, and their tokens
/// stay 0 for unknown.
fn efficiency_prompt_sections(
    diagnostics: &crate::dispatch::PromptDiagnostics,
) -> Vec<roko_learn::efficiency::PromptSectionMeta> {
    let section = |name: &str, tokens: usize, priority: usize, was_dropped: bool| {
        roko_learn::efficiency::PromptSectionMeta {
            name: name.to_string(),
            tokens: u64::try_from(tokens).unwrap_or(u64::MAX),
            priority: u8::try_from(priority).unwrap_or(u8::MAX),
            was_truncated: false,
            was_dropped,
        }
    };
    let Some(manifest) = &diagnostics.composition_manifest else {
        return diagnostics
            .included_sections
            .iter()
            .map(|name| section(name, 0, 0, false))
            .collect();
    };
    let included = manifest
        .included
        .iter()
        .enumerate()
        .map(|(order, meta)| section(&meta.name, meta.estimated_tokens, order, false));
    // A dropped section ranks below every included one.
    let last = usize::from(u8::MAX);
    let dropped = manifest
        .excluded
        .iter()
        .map(|meta| section(&meta.name, meta.estimated_tokens, last, true));
    included.chain(dropped).collect()
}

/// What an attempt's verify steps said, for its efficiency row (backlog
/// 2107): a pass, a failed gate, or nothing when no verify step judged it.
const fn efficiency_gate_passed(outcome: AttemptOutcome) -> Option<bool> {
    match outcome {
        AttemptOutcome::Passed => Some(true),
        AttemptOutcome::GateFailed => Some(false),
        _ => None,
    }
}

/// One line per failed verify step, for the efficiency row's `gate_errors`:
/// the step's rung and how it failed. No command or output.
fn failed_step_summaries(steps: &[roko_learn::telemetry::VerifyStepVerdict]) -> Vec<String> {
    steps
        .iter()
        .filter(|step| step.passed == Some(false))
        .map(|step| match (step.timed_out, step.exit_code) {
            (true, _) => format!("{}: timed out", step.rung),
            (false, Some(code)) => format!("{}: exit code {code}", step.rung),
            (false, None) => format!("{}: failed", step.rung),
        })
        .collect()
}

/// Reasoning (thinking) tokens a dispatch reported (gap-7a8474): its usage,
/// else its usage observation, else the sum of its token-usage events, which
/// some CLI providers stream.
fn reported_reasoning_tokens(
    usage: &roko_core::Usage,
    observed: Option<&roko_core::UsageObservation>,
    events: &[roko_agent::AgentRuntimeEvent],
) -> u64 {
    let reported = u64::from(usage.reasoning_tokens);
    if reported > 0 {
        return reported;
    }
    if let Some(observed) = observed.and_then(|observed| observed.reasoning_tokens)
        && observed > 0
    {
        return observed;
    }
    events
        .iter()
        .map(|event| match event {
            roko_agent::AgentRuntimeEvent::TokenUsage {
                reasoning_tokens, ..
            } => *reasoning_tokens,
            _ => 0,
        })
        .sum()
}

/// The tool calls a dispatch made, one per call (bug-f9ae3e), with each
/// one's outcome where something recorded it.
///
/// A streaming provider sends a call's start, each argument delta and its
/// end as separate tool-call events under one id, and a delta may name no
/// tool; a call without an id counts once for each event that names a
/// tool. A call's tool output gives its result size. The events don't say
/// whether a call succeeded, and the result bridge sends none. Two records
/// do: the calls the attempt's live output showed (`live`, bug-264c41),
/// with the outcome a CLI provider's tool result reports, and the calls
/// roko's own tool loop ran (`audited`, gap-4d5e2d), with the outcome the
/// tool audit holds, which wins. Each record settles the earliest call under
/// its id that the record hasn't settled yet, or is added as a call of its
/// own. A call no record settled keeps an unknown outcome.
fn efficiency_tool_calls(
    events: &[roko_agent::AgentRuntimeEvent],
    live: &[crate::dispatch_v2::ToolCallRecord],
    audited: &[crate::dispatch_v2::ToolCallRecord],
) -> Vec<roko_learn::efficiency::ToolCallMeta> {
    let mut calls: Vec<roko_learn::efficiency::ToolCallMeta> = Vec::new();
    // The call id of each entry of `calls`; empty for a call without one.
    let mut ids: Vec<String> = Vec::new();
    let mut by_id: HashMap<&str, usize> = HashMap::new();
    for event in events {
        match event {
            roko_agent::AgentRuntimeEvent::ToolCall { id, name } if !id.is_empty() => {
                match by_id.entry(id.as_str()) {
                    Entry::Occupied(entry) => {
                        let call = &mut calls[*entry.get()];
                        if call.tool_name.is_empty() {
                            call.tool_name.clone_from(name);
                        }
                    }
                    Entry::Vacant(entry) => {
                        entry.insert(calls.len());
                        calls.push(unobserved_tool_call(name));
                        ids.push(id.clone());
                    }
                }
            }
            roko_agent::AgentRuntimeEvent::ToolCall { name, .. } if !name.is_empty() => {
                calls.push(unobserved_tool_call(name));
                ids.push(String::new());
            }
            roko_agent::AgentRuntimeEvent::ToolOutput { id, output } => {
                if let Some(&index) = by_id.get(id.as_str()) {
                    calls[index].result_tokens += roko_compose::estimate_tokens(output) as u64;
                }
            }
            _ => {}
        }
    }
    settle_tool_calls(&mut calls, &mut ids, live);
    settle_tool_calls(&mut calls, &mut ids, audited);
    calls
}

/// An efficiency record of a call to `name` whose outcome is unknown.
fn unobserved_tool_call(name: &str) -> roko_learn::efficiency::ToolCallMeta {
    roko_learn::efficiency::ToolCallMeta {
        tool_name: name.to_string(),
        duration_ms: 0,
        result_tokens: 0,
        succeeded: None,
        advanced_task: false,
        was_redundant: false,
        error_category: None,
    }
}

/// Settle `calls`, whose call ids are `ids`, with `records`: each record
/// settles the earliest call under its id that no earlier record of
/// `records` settled, giving it the record's outcome when it has one and
/// its tool name when it lacks one. A record with no such call is added.
fn settle_tool_calls(
    calls: &mut Vec<roko_learn::efficiency::ToolCallMeta>,
    ids: &mut Vec<String>,
    records: &[crate::dispatch_v2::ToolCallRecord],
) {
    let mut settled = vec![false; calls.len()];
    for record in records {
        let earliest = (0..calls.len()).find(|&index| !settled[index] && ids[index] == record.id);
        match earliest {
            Some(index) => {
                settled[index] = true;
                let call = &mut calls[index];
                if record.succeeded.is_some() {
                    call.succeeded = record.succeeded;
                }
                if call.tool_name.is_empty() {
                    call.tool_name.clone_from(&record.name);
                }
            }
            None => {
                calls.push(roko_learn::efficiency::ToolCallMeta {
                    succeeded: record.succeeded,
                    ..unobserved_tool_call(&record.name)
                });
                ids.push(record.id.clone());
                settled.push(true);
            }
        }
    }
}

/// The provenance record of a CLI provider's dispatch turn `dispatch`, for
/// attempt `settled` (gap-ca8022). It is an outcome with no intent, since the
/// CLI ran its tools before roko saw them, and its argument digest commits to
/// the tool calls `calls` the turn's live output showed. A turn that made tool
/// calls carries external taint: roko cannot see where their inputs came from.
fn cli_turn_outcome(
    sink: &dyn SafetyProvenanceSink,
    settled: &SettledAttempt,
    dispatch: &crate::dispatch_v2::AgentResultDispatch,
    calls: &[ToolCallRecord],
) -> ProvenanceOutcome {
    let identity = &settled.verdict.identity;
    let verdict = if dispatch.result.success {
        ProvenanceVerdict::Succeeded
    } else {
        ProvenanceVerdict::Failed
    };
    let taint = if calls.is_empty() {
        CamelTaintLevel::Trusted
    } else {
        CamelTaintLevel::External
    };
    ProvenanceOutcome {
        call: ProvenanceCall {
            run_id: identity.run_id.clone(),
            task_id: identity.task_id.clone(),
            attempt_id: identity.attempt_key.clone(),
            turn_id: String::new(),
            call_id: "cli-turn".to_string(),
            tool: format!("cli:{}", dispatch.target.provider_id),
            args_digest: cli_tool_calls_digest(&sink.digest_key(), calls),
        },
        intent: None,
        verdict,
        reason: None,
        result_digest: None,
        taint,
    }
}

/// Keyed digest of the tool calls `calls` a CLI turn's live output showed:
/// each call's id, tool and outcome, in the order the provider made them.
fn cli_tool_calls_digest(key: &[u8; 32], calls: &[ToolCallRecord]) -> ContentHash {
    let observed: Vec<serde_json::Value> = calls
        .iter()
        .map(|call| {
            serde_json::json!({ "id": call.id, "name": call.name, "succeeded": call.succeeded })
        })
        .collect();
    arguments_digest(key, &serde_json::Value::Array(observed))
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use roko_learn::cascade_router::CascadeRouter;
    use roko_learn::playbook::PlaybookStore;
    use roko_learn::prompt_experiment::ExperimentStore;

    use super::*;
    use crate::graph_task_dispatch::tests::{
        VERIFY_PROVIDER, jsonl_rows_where, make_spec, make_test_dispatcher, no_auto_fix,
        verify_step,
    };

    /// Save a prompt experiment on the implementer's role section at
    /// `store_path`, so every implementer prompt gets a treatment.
    fn save_role_experiment(store_path: &Path) {
        use roko_learn::prompt_experiment::{PromptExperiment, PromptVariant};

        std::fs::create_dir_all(store_path.parent().unwrap()).unwrap();
        let mut experiment = PromptExperiment::new(
            "role-ab",
            "role_identity",
            ["terse", "thorough"]
                .into_iter()
                .map(|id| PromptVariant {
                    id: id.into(),
                    name: id.into(),
                    section_name: "role_identity".into(),
                    content: format!("You are the Implementer ({id} variant)."),
                    slug: None,
                    active: true,
                })
                .collect(),
        );
        experiment.role = Some("implementer".into());
        let mut store = ExperimentStore::new();
        store.register(experiment);
        store.save(store_path).unwrap();
    }

    /// Trials and successes over every treatment of the role experiment.
    fn experiment_trials(store_path: &Path) -> (u64, u64) {
        ExperimentStore::load_strict(store_path)
            .unwrap()
            .get("role-ab")
            .unwrap()
            .stats
            .values()
            .fold((0, 0), |(trials, successes), stats| {
                (trials + stats.trials, successes + stats.successes)
            })
    }

    /// Save the playbook that prompt assembly injects into a task titled
    /// "Render the greeting banner".
    async fn save_banner_playbook(playbook_dir: &Path) -> PlaybookStore {
        let playbooks = PlaybookStore::new(playbook_dir);
        playbooks
            .save(&roko_learn::playbook::Playbook::new(
                "banner-steps",
                "Render a greeting banner",
            ))
            .await
            .unwrap();
        playbooks
    }

    /// The banner playbook's success and failure counts.
    async fn playbook_counts(playbooks: &PlaybookStore) -> (u64, u64) {
        let playbook = playbooks.load("banner-steps").await.unwrap().unwrap();
        (playbook.success_count, playbook.failure_count)
    }

    /// The router's confidence trials and successes for the dispatched
    /// model, and its bandit observations.
    fn router_counts(router: &CascadeRouter) -> ((u64, u64), u64) {
        let confidence = router
            .confidence_snapshot()
            .get("claude-sonnet-4-6")
            .copied();
        (confidence.unwrap_or_default(), router.total_observations())
    }

    /// Provider that answers like [`VERIFY_PROVIDER`], except that a call
    /// finding `fail-next` beside it fails with a transport error, and one
    /// finding `exhaust-next` reports exhausted usage. Each marker fails one
    /// call.
    const FLAKY_PROVIDER: &str = r#"#!/bin/sh
set -eu
cat >/dev/null
dir=$(dirname -- "$0")
if [ -f "$dir/fail-next" ]; then
  rm -f "$dir/fail-next"
  echo 'upstream connect error: connection refused' >&2
  exit 1
fi
if [ -f "$dir/exhaust-next" ]; then
  rm -f "$dir/exhaust-next"
  echo "You've hit your usage limit" >&2
  exit 1
fi
printf '%s\n' '{"type":"content_block_delta","delta":{"text":"verify-output"}}'
printf '%s\n' '{"type":"result","session_id":"sess-v1","model":"claude-sonnet-4-6","total_cost_usd":0.01,"usage":{"input_tokens":5,"output_tokens":10}}'
"#;

    /// bug-c34782 through the batch dispatch path: the router learns a
    /// success from a passing verify step and a failure from a failing one,
    /// and nothing from an attempt without verify steps, a provider transport
    /// error or exhausted usage, although the provider call succeeded or the
    /// model never got to work.
    #[tokio::test]
    async fn routing_learns_only_from_gate_verdicts() {
        let temp = tempdir().expect("tempdir");
        let router = Arc::new(CascadeRouter::new(vec!["claude-sonnet-4-6".into()]));
        let facade = FeedbackFacade::new().with_sink(Arc::new(
            crate::runtime_feedback::RoutingObservationSink::new(Arc::clone(&router)),
        ));
        let feedback = GraphFeedbackContext {
            feedback_facade: Some(Arc::new(facade)),
            ..GraphFeedbackContext::default()
        };
        let (dispatcher, mut task) =
            make_test_dispatcher(&temp, FLAKY_PROVIDER, no_auto_fix, feedback).await;
        let ctx = CellContext::new();

        task.verify = vec![verify_step("check", "true")];
        dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &ctx)
            .await
            .expect("the verify step passes");
        assert_eq!(router_counts(&router), ((1, 1), 1), "a pass is a success");

        task.verify = vec![verify_step("check", "false")];
        dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &ctx)
            .await
            .expect_err("the verify step fails");
        assert_eq!(router_counts(&router), ((2, 1), 2), "a gate failure");

        task.verify.clear();
        dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &ctx)
            .await
            .expect("an attempt without verify steps completes unverified");
        std::fs::write(temp.path().join("fail-next"), "").unwrap();
        dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &ctx)
            .await
            .expect_err("the provider call fails");
        std::fs::write(temp.path().join("exhaust-next"), "").unwrap();
        dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &ctx)
            .await
            .expect_err("the provider is out of usage");
        assert_eq!(
            router_counts(&router),
            ((2, 1), 2),
            "unverified, provider-failed and exhausted attempts are not quality evidence"
        );
    }

    /// bug-f9ae3e: a streamed call's start, argument deltas and end make one
    /// efficiency record, sized by its tool output and with its outcome
    /// unknown; a call without an id counts once per event naming a tool.
    #[test]
    fn efficiency_counts_distinct_tool_call_ids() {
        use roko_agent::AgentRuntimeEvent as Event;

        let call = |id: &str, name: &str| Event::ToolCall {
            id: id.to_string(),
            name: name.to_string(),
        };
        let events = vec![
            call("tu_1", "Read"),
            call("tu_1", ""),
            call("tu_1", ""),
            call("tu_1", "Read"),
            Event::ToolOutput {
                id: "tu_1".to_string(),
                output: "x".repeat(40),
            },
            call("tu_2", "Bash"),
            call("", ""),
            call("", "Grep"),
            Event::MessageDelta {
                text: "done".to_string(),
            },
        ];

        let calls = efficiency_tool_calls(&events, &[], &[]);
        let names: Vec<&str> = calls.iter().map(|call| call.tool_name.as_str()).collect();
        assert_eq!(names, ["Read", "Bash", "Grep"]);
        assert_eq!(calls[0].result_tokens, 10, "40 bytes of tool output");
        assert_eq!(calls[1].result_tokens, 0, "no output observed");
        assert!(calls.iter().all(|call| call.succeeded.is_none()));
    }

    /// gap-4d5e2d: a call the tool audit recorded takes its audited outcome.
    /// An audited call the events missed is added with its outcome, and a
    /// call only the events saw, or one the audit holds no result for, keeps
    /// an unknown outcome.
    #[test]
    fn efficiency_tool_calls_record_outcome_from_the_tool_audit() {
        use crate::dispatch_v2::ToolCallRecord;
        use roko_agent::AgentRuntimeEvent as Event;

        let call = |id: &str, name: &str| Event::ToolCall {
            id: id.to_string(),
            name: name.to_string(),
        };
        let audited = |id: &str, name: &str, succeeded: Option<bool>| ToolCallRecord {
            id: id.to_string(),
            name: name.to_string(),
            succeeded,
        };
        let events = [call("call-1", "read_file"), call("call-2", "Bash")];

        let calls = efficiency_tool_calls(
            &events,
            &[],
            &[
                audited("call-1", "read_file", Some(false)),
                audited("call-3", "write_file", Some(true)),
                audited("call-4", "grep", None),
            ],
        );
        let outcomes: Vec<(&str, Option<bool>)> = calls
            .iter()
            .map(|call| (call.tool_name.as_str(), call.succeeded))
            .collect();
        assert_eq!(
            outcomes,
            [
                ("read_file", Some(false)),
                ("Bash", None),
                ("write_file", Some(true)),
                ("grep", None),
            ]
        );
    }

    /// bug-264c41: the calls a CLI provider streamed take the outcomes its
    /// tool results reported. A call roko's tool loop ran shows up in the
    /// live output too: the audit settles that one call instead of adding a
    /// second, and its outcome wins; a call it left open keeps the live one.
    #[test]
    fn efficiency_tool_calls_record_outcome_from_the_live_output() {
        use crate::dispatch_v2::ToolCallRecord;

        let record = |id: &str, name: &str, succeeded: Option<bool>| ToolCallRecord {
            id: id.to_string(),
            name: name.to_string(),
            succeeded,
        };
        let live = [
            record("tu_1", "Read", Some(true)),
            record("tu_2", "Bash", Some(false)),
            record("call-1", "read_file", None),
            record("call-2", "grep", Some(true)),
            record("tu_3", "Grep", None),
        ];
        let audited = [
            record("call-1", "read_file", Some(false)),
            record("call-2", "grep", None),
        ];

        let calls = efficiency_tool_calls(&[], &live, &audited);
        let outcomes: Vec<(&str, Option<bool>)> = calls
            .iter()
            .map(|call| (call.tool_name.as_str(), call.succeeded))
            .collect();
        assert_eq!(
            outcomes,
            [
                ("Read", Some(true)),
                ("Bash", Some(false)),
                ("read_file", Some(false)),
                ("grep", Some(true)),
                ("Grep", None),
            ]
        );
    }

    /// A fake Claude CLI whose agent reads a file and runs a command that
    /// fails; the failed call's tool result is marked `is_error`.
    const TOOL_CALLING_PROVIDER: &str = r#"#!/bin/sh
set -eu
cat >/dev/null
printf '%s\n' '{"type":"assistant","message":{"id":"msg_1","model":"claude-sonnet-4-6","content":[{"type":"tool_use","id":"tu_1","name":"Read","input":{"file_path":"notes.txt"}},{"type":"tool_use","id":"tu_2","name":"Bash","input":{"command":"false"}}],"usage":{"input_tokens":10,"output_tokens":5}}}'
printf '%s\n' '{"type":"user","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"tu_1","content":"notes"},{"type":"tool_result","tool_use_id":"tu_2","content":"Exit code 1","is_error":true}]}}'
printf '%s\n' '{"type":"result","subtype":"success","is_error":false,"session_id":"sess-tools","model":"claude-sonnet-4-6","total_cost_usd":0.01,"num_turns":2,"usage":{"input_tokens":5,"output_tokens":10}}'
"#;

    /// bug-264c41: a Claude CLI attempt's efficiency row lists the tool calls
    /// its agent made, which the attempt's live output showed, each with the
    /// outcome its tool result reported.
    #[tokio::test]
    async fn cli_attempt_records_its_tool_calls() {
        let temp = tempdir().expect("tempdir");
        let efficiency_path = temp.path().join(".roko/learn/efficiency.jsonl");
        let feedback = GraphFeedbackContext {
            efficiency_path: Some(efficiency_path.clone()),
            ..GraphFeedbackContext::default()
        };
        // The attempt tracks its progress (bug-3a3b0f), so its live-output
        // tap is open and sees the calls.
        let (dispatcher, task) =
            make_test_dispatcher(&temp, TOOL_CALLING_PROVIDER, no_auto_fix, feedback).await;
        dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &CellContext::new())
            .await
            .expect("the attempt completes");

        let rows = jsonl_rows_where(&efficiency_path, 1, |row| {
            row["schema"] == roko_learn::efficiency::AGENT_EFFICIENCY_EVENT_SCHEMA
                && !row["attempt_id"].as_str().unwrap_or("/").contains('/')
        })
        .await;
        let calls: Vec<(&str, Option<bool>)> = rows[0]["tool_calls"]
            .as_array()
            .expect("tool calls")
            .iter()
            .map(|call| {
                (
                    call["tool_name"].as_str().unwrap_or_default(),
                    call["succeeded"].as_bool(),
                )
            })
            .collect();
        assert_eq!(
            calls,
            [("Read", Some(true)), ("Bash", Some(false))],
            "{:#}",
            rows[0]
        );
        assert_eq!(rows[0]["tools_used"], 2, "{:#}", rows[0]);
    }

    /// gap-ca8022: a Claude CLI attempt runs its tools itself, outside roko's
    /// tool dispatcher, and its dispatch turn leaves one provenance record:
    /// an outcome with no intent, whose argument digest commits to the tool
    /// calls the attempt's live output showed.
    #[tokio::test]
    async fn cli_attempts_record_provenance() {
        use roko_agent::safety::{ProvenanceRecord, WitnessLogger};

        use crate::safety_provenance::{GraphProvenanceSink, ProvenanceSinks};

        let temp = tempdir().expect("tempdir");
        let sinks = ProvenanceSinks::default();
        let feedback = GraphFeedbackContext {
            provenance_sinks: Some(sinks.clone()),
            ..GraphFeedbackContext::default()
        };
        let (dispatcher, task) =
            make_test_dispatcher(&temp, TOOL_CALLING_PROVIDER, no_auto_fix, feedback).await;
        let sink = Arc::new(GraphProvenanceSink::start(temp.path()).expect("provenance sink"));
        let _registered = sinks.register("run-cli", Arc::clone(&sink));
        let ctx = CellContext::new().with_run_id("run-cli".to_string());
        dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &ctx)
            .await
            .expect("the attempt completes");

        let summary = sink.summary();
        assert_eq!(summary.records, 1, "one record for the turn");
        let head = summary.witness_head.expect("the turn's vertex");
        let dag = WitnessLogger::new(temp.path().join(".roko/witness.jsonl"))
            .read_all()
            .expect("witness log");
        let vertex = dag.get(&head).expect("the turn's vertex");
        let record: ProvenanceRecord =
            serde_json::from_value(vertex.content["record"].clone()).expect("a record");
        let ProvenanceRecord::Outcome(outcome) = record else {
            panic!("expected an outcome: {record:?}");
        };
        assert_eq!(outcome.intent, None);
        assert_eq!(outcome.verdict, ProvenanceVerdict::Succeeded);
        assert_eq!(outcome.call.tool, "cli:stream-cli");
        assert_eq!(outcome.call.run_id, "run-cli");
        assert_eq!(outcome.taint, CamelTaintLevel::External);
        let calls = [
            ToolCallRecord {
                id: "tu_1".to_string(),
                name: "Read".to_string(),
                succeeded: Some(true),
            },
            ToolCallRecord {
                id: "tu_2".to_string(),
                name: "Bash".to_string(),
                succeeded: Some(false),
            },
        ];
        assert_eq!(
            outcome.call.args_digest,
            cli_tool_calls_digest(&sink.digest_key(), &calls)
        );
    }

    /// A fake Claude CLI that answers after 100 ms.
    const SLOW_FIRST_TOKEN_PROVIDER: &str = r#"#!/bin/sh
set -eu
cat >/dev/null
sleep 0.1
printf '%s\n' '{"type":"assistant","message":{"id":"msg_1","model":"claude-sonnet-4-6","content":[{"type":"text","text":"done"}],"usage":{"input_tokens":10,"output_tokens":5}}}'
printf '%s\n' '{"type":"result","subtype":"success","is_error":false,"session_id":"sess-ttft","model":"claude-sonnet-4-6","total_cost_usd":0.01,"num_turns":1,"usage":{"input_tokens":10,"output_tokens":5}}'
"#;

    /// gap-7a8474: a Graph attempt's efficiency row records its time to
    /// first token, from the provider call's start to its first streamed
    /// output.
    #[tokio::test]
    async fn cli_attempt_records_its_time_to_first_token() {
        let temp = tempdir().expect("tempdir");
        let efficiency_path = temp.path().join(".roko/learn/efficiency.jsonl");
        let feedback = GraphFeedbackContext {
            efficiency_path: Some(efficiency_path.clone()),
            ..GraphFeedbackContext::default()
        };
        let (dispatcher, task) =
            make_test_dispatcher(&temp, SLOW_FIRST_TOKEN_PROVIDER, no_auto_fix, feedback).await;
        dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &CellContext::new())
            .await
            .expect("the attempt completes");

        let rows = jsonl_rows_where(&efficiency_path, 1, |row| {
            row["schema"] == roko_learn::efficiency::AGENT_EFFICIENCY_EVENT_SCHEMA
                && !row["attempt_id"].as_str().unwrap_or("/").contains('/')
        })
        .await;
        let ttft_ms = rows[0]["time_to_first_token_ms"]
            .as_u64()
            .expect("time to first token");
        assert!(ttft_ms >= 100, "{ttft_ms} ms: {:#}", rows[0]);
    }

    /// backlog 2107: an attempt writes one efficiency row, its settled one,
    /// whatever ended it: a pass, a verify failure with replanning on, or a
    /// provider failure. The pass row carries the verdict and the call's
    /// tokens, the failed gate's row names the failed step, and a call that
    /// streamed nothing marks its time to first token unknown.
    #[tokio::test]
    async fn efficiency_writes_one_keyed_row_per_attempt() {
        let temp = tempdir().expect("tempdir");
        let efficiency_path = temp.path().join(".roko/learn/efficiency.jsonl");
        let feedback = GraphFeedbackContext {
            efficiency_path: Some(efficiency_path.clone()),
            replan_on_gate_failure: true,
            ..GraphFeedbackContext::default()
        };
        let (dispatcher, mut task) =
            make_test_dispatcher(&temp, FLAKY_PROVIDER, no_auto_fix, feedback).await;
        let ctx = CellContext::new();
        task.verify = vec![verify_step("check", "true")];
        dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &ctx)
            .await
            .expect("the verify step passes");
        task.verify = vec![verify_step("check", "false")];
        dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &ctx)
            .await
            .expect_err("the verify step fails");
        std::fs::write(temp.path().join("fail-next"), "").expect("fail the next call");
        dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &ctx)
            .await
            .expect_err("the provider call fails");

        let rows = jsonl_rows_where(&efficiency_path, 3, |row| {
            row["schema"] == roko_learn::efficiency::AGENT_EFFICIENCY_EVENT_SCHEMA
                && row["role"] != "helper"
        })
        .await;
        assert_eq!(rows.len(), 3, "one row per attempt: {rows:#?}");
        let mut keys: Vec<&str> = rows
            .iter()
            .map(|row| row["attempt_key"].as_str().unwrap_or_default())
            .collect();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), 3, "{rows:#?}");
        let row = |gate_passed: serde_json::Value| {
            rows.iter()
                .find(|row| row["gate_passed"] == gate_passed)
                .unwrap_or_else(|| panic!("no row with gate_passed {gate_passed}: {rows:#?}"))
        };
        let passed = row(serde_json::Value::Bool(true));
        assert_eq!(passed["attempt_id"], passed["attempt_key"], "{passed}");
        assert!(passed["input_tokens"].as_u64() > Some(0), "{passed}");
        assert!(passed.get("ttft_unknown").is_none(), "{passed}");
        let failed = row(serde_json::Value::Bool(false));
        assert_eq!(
            failed["gate_errors"],
            serde_json::json!(["custom:check: exit code 1"]),
            "{failed}"
        );
        let provider_failed = row(serde_json::Value::Null);
        assert_eq!(provider_failed["ttft_unknown"], true, "{provider_failed}");
    }

    /// backlog 2108: an efficiency row lists each prompt section with the
    /// composer's token estimate, ranked by inclusion order, and each section
    /// the budget dropped. Without a manifest only the names are known.
    #[test]
    fn efficiency_prompt_sections_carry_token_counts() {
        use roko_compose::{
            AttentionBidder, CompositionManifest, CompositionStrategy, ExcludedSectionMeta,
            IncludedSectionMeta,
        };

        let mut diagnostics = crate::dispatch::PromptDiagnostics {
            included_sections: vec!["role".to_string(), "task".to_string()],
            ..crate::dispatch::PromptDiagnostics::default()
        };
        let names_only: Vec<(String, u64)> = efficiency_prompt_sections(&diagnostics)
            .into_iter()
            .map(|section| (section.name, section.tokens))
            .collect();
        assert_eq!(
            names_only,
            [("role".to_string(), 0), ("task".to_string(), 0)]
        );

        let included = |name: &str, tokens: usize| IncludedSectionMeta {
            section_id: name.to_string(),
            action_id: name.to_string(),
            name: name.to_string(),
            bidder: AttentionBidder::default(),
            estimated_tokens: tokens,
            score: 1.0,
            bid_value: 1.0,
            vcg_payment: None,
            reason: "selected".to_string(),
        };
        diagnostics.composition_manifest = Some(CompositionManifest {
            requested_strategy: CompositionStrategy::Auto,
            selected_strategy: CompositionStrategy::DensityGreedy,
            included: vec![included("role", 120), included("task", 30)],
            excluded: vec![ExcludedSectionMeta {
                section_id: "episodes".to_string(),
                action_id: "episodes".to_string(),
                name: "episodes".to_string(),
                bidder: AttentionBidder::default(),
                estimated_tokens: 80,
                score: 0.1,
                bid_value: 0.1,
                reason: "over budget".to_string(),
            }],
            scored_signals: Vec::new(),
            vcg_diagnostics: None,
            total_tokens: 150,
            token_budget_limit: Some(160),
        });
        let sections: Vec<(String, u64, u8, bool)> = efficiency_prompt_sections(&diagnostics)
            .into_iter()
            .map(|section| {
                (
                    section.name,
                    section.tokens,
                    section.priority,
                    section.was_dropped,
                )
            })
            .collect();
        assert_eq!(
            sections,
            [
                ("role".to_string(), 120, 0, false),
                ("task".to_string(), 30, 1, false),
                ("episodes".to_string(), 80, u8::MAX, true),
            ]
        );
    }

    /// gap-7a8474: the Graph efficiency row's usage fields come from what the
    /// dispatch reported. Reasoning tokens fall back from the usage to its
    /// observation to the streamed token events, and cache reads on a priced
    /// model make the uncached cost higher than the cost.
    #[test]
    fn graph_efficiency_event_populates_usage_fields() {
        use crate::dispatch_v2::usage_cost_without_cache;
        use roko_agent::AgentRuntimeEvent as Event;
        use roko_core::config::schema::ModelProfile;

        let streamed = |reasoning_tokens: u64| Event::TokenUsage {
            input_tokens: 0,
            output_tokens: 0,
            cache_read_tokens: 0,
            cache_write_tokens: 0,
            reasoning_tokens,
        };
        let events = [streamed(40), streamed(2)];
        let mut usage = roko_core::Usage::zero();
        assert_eq!(reported_reasoning_tokens(&usage, None, &events), 42);
        let observed = roko_core::UsageObservation {
            reasoning_tokens: Some(7),
            ..roko_core::UsageObservation::default()
        };
        assert_eq!(
            reported_reasoning_tokens(&usage, Some(&observed), &events),
            7
        );
        usage.reasoning_tokens = 9;
        assert_eq!(
            reported_reasoning_tokens(&usage, Some(&observed), &events),
            9
        );

        usage.input_tokens = 1_000;
        usage.output_tokens = 500;
        usage.cache_read_tokens = 9_000;
        usage.fill_cost_from_pricing(Some(3.0), Some(15.0), None, None);
        let profile = ModelProfile {
            cost_input_per_m: Some(3.0),
            cost_output_per_m: Some(15.0),
            ..ModelProfile::default()
        };
        let uncached = usage_cost_without_cache(&usage, Some(&profile), "unpriced-model")
            .expect("the profile prices the model");
        assert!(
            uncached > f64::from(usage.cost_usd),
            "{uncached} against {}",
            usage.cost_usd
        );
        assert_eq!(
            usage_cost_without_cache(&usage, None, "unpriced-model"),
            None
        );
    }

    /// bug-07bc75: a Graph dispatch teaches the router only through its
    /// settled verdict. The provider bridge still records every call's
    /// efficiency row and the provider's health, but it no longer observes
    /// or saves `cascade-router.json` from the provider's own success,
    /// before any gate ran.
    #[tokio::test]
    async fn graph_dispatch_router_learns_only_from_settled_verdicts() {
        let temp = tempdir().expect("tempdir");
        let router = Arc::new(CascadeRouter::new(vec!["claude-sonnet-4-6".into()]));
        let facade = FeedbackFacade::new().with_sink(Arc::new(
            crate::runtime_feedback::RoutingObservationSink::new(Arc::clone(&router)),
        ));
        let feedback = GraphFeedbackContext {
            feedback_facade: Some(Arc::new(facade)),
            ..GraphFeedbackContext::default()
        };
        let (dispatcher, mut task) =
            make_test_dispatcher(&temp, FLAKY_PROVIDER, no_auto_fix, feedback).await;
        let ctx = CellContext::new();

        dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &ctx)
            .await
            .expect("an attempt without verify steps completes unverified");
        std::fs::write(temp.path().join("fail-next"), "").unwrap();
        dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &ctx)
            .await
            .expect_err("the provider call fails");
        task.verify = vec![verify_step("check", "true")];
        dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &ctx)
            .await
            .expect("the verify step passes");

        assert_eq!(router_counts(&router), ((1, 1), 1), "only the settled pass");
        let learn = temp.path().join(".roko/learn");
        assert!(
            !learn.join("cascade-router.json").exists(),
            "the provider bridge trained the router"
        );
        let model_calls = std::fs::read_to_string(learn.join("efficiency.jsonl"))
            .unwrap_or_default()
            .lines()
            .filter(|line| line.contains(r#""kind":"model_call""#))
            .count();
        assert!(model_calls >= 3, "one efficiency row per provider call");
        assert!(learn.join("provider-health.json").exists());
    }

    /// S01 §4.1 through the batch dispatch path: an unverified attempt, a
    /// provider transport error and a prompt-assembly failure carry no
    /// learning label, so none of them moves a learner (router, playbooks,
    /// daimon, prompt experiments, durable knowledge). The first two still
    /// leave episodes, labelled `null`, and all three leave verdicts, so none
    /// reads as abandoned. A pass then moves every learner.
    #[tokio::test]
    async fn learning_sinks_skip_attempts_without_a_learning_label() {
        let temp = tempdir().expect("tempdir");
        let roko = temp.path().join(".roko");
        let store_path = roko.join("learn/experiments.json");
        save_role_experiment(&store_path);
        let playbooks = save_banner_playbook(&roko.join("learn/playbooks")).await;
        let router = Arc::new(CascadeRouter::new(vec!["claude-sonnet-4-6".into()]));
        let daimon = Arc::new(std::sync::Mutex::new(roko_daimon::DaimonState::new()));
        let episodes_path = roko.join("episodes.jsonl");
        let facade = FeedbackFacade::new()
            .with_sink(Arc::new(crate::runtime_feedback::EpisodeSink::at(
                &episodes_path,
            )))
            .with_sink(Arc::new(
                crate::runtime_feedback::RoutingObservationSink::new(Arc::clone(&router)),
            ))
            .with_sink(Arc::new(
                crate::runtime_feedback::VerifiedKnowledgeSink::for_workdir(temp.path()),
            ));
        let feedback = GraphFeedbackContext {
            feedback_facade: Some(Arc::new(facade)),
            experiment_store_path: Some(store_path.clone()),
            playbook_dir: Some(roko.join("learn/playbooks")),
            daimon_state: Some(Arc::clone(&daimon)),
            runs_dir: Some(roko.join("runs")),
            ..GraphFeedbackContext::default()
        };
        let (dispatcher, mut task) =
            make_test_dispatcher(&temp, FLAKY_PROVIDER, no_auto_fix, feedback).await;
        task.title = "Render the greeting banner".into();
        let ctx = CellContext::new().with_run_id("run-labels".to_string());
        let affect_ticks = || daimon.lock().unwrap().state.tick_count;
        let knowledge = || {
            roko_neuro::KnowledgeStore::for_workdir(temp.path())
                .read_all()
                .unwrap()
                .len()
        };

        // No verify steps: the provider call succeeds, and nothing checks it.
        dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &ctx)
            .await
            .expect("the attempt completes unverified");
        // A transport error is the provider's, not the agent's.
        std::fs::write(temp.path().join("fail-next"), "").unwrap();
        dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &ctx)
            .await
            .expect_err("the provider call fails");
        // A declared context file that does not exist fails prompt assembly.
        let mut unassembled = task.clone();
        unassembled.context = Some(crate::task_parser::TaskContext {
            read_files: vec![crate::task_parser::ReadFile {
                path: "src/missing.rs".to_string(),
                lines: None,
                why: "context".to_string(),
            }],
            ..crate::task_parser::TaskContext::default()
        });
        dispatcher
            .dispatch(&make_spec(&unassembled), Vec::new(), &ctx)
            .await
            .expect_err("prompt assembly fails");

        assert_eq!(router_counts(&router), ((0, 0), 0), "router");
        assert_eq!(playbook_counts(&playbooks).await, (0, 0), "playbooks");
        assert_eq!(affect_ticks(), 0, "daimon");
        assert_eq!(experiment_trials(&store_path), (0, 0), "experiments");
        assert_eq!(knowledge(), 0, "durable knowledge");
        let episodes = roko_learn::episode_logger::EpisodeLogger::read_all(&episodes_path)
            .await
            .unwrap();
        let labels: Vec<(&str, &serde_json::Value)> = episodes
            .iter()
            .map(|episode| {
                let outcome = episode.extra["outcome"].as_str().unwrap_or_default();
                (outcome, &episode.extra["learning_label"])
            })
            .collect();
        let null = serde_json::Value::Null;
        assert_eq!(labels, [("unverified", &null), ("provider_error", &null)]);

        // A pass is evidence for every learner.
        task.verify = vec![verify_step("check", "true")];
        dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &ctx)
            .await
            .expect("the verify step passes");
        assert_eq!(router_counts(&router), ((1, 1), 1), "router");
        assert_eq!(playbook_counts(&playbooks).await, (1, 0), "playbooks");
        assert!(affect_ticks() > 0, "daimon");
        assert_eq!(experiment_trials(&store_path), (1, 1), "experiments");
        assert!(knowledge() > 0, "durable knowledge");

        // Every attempt settled, so none reads as abandoned. Closing the
        // run's writer flushes its lines.
        drop(dispatcher);
        let attempts = roko.join("runs/run-labels/attempts.jsonl");
        let mut verdicts = Vec::new();
        for _ in 0..600 {
            verdicts = std::fs::read_to_string(&attempts)
                .unwrap_or_default()
                .lines()
                .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
                .filter(|row| row["schema_version"] == "roko.verdict/1")
                .map(|row| {
                    (
                        row["outcome"].to_string(),
                        row["learning_label"].to_string(),
                    )
                })
                .collect::<Vec<_>>();
            if verdicts.len() >= 4 {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        let expected = [
            ("unverified", "null"),
            ("provider_error", "null"),
            ("harness_error", "null"),
            ("passed", "1"),
        ]
        .map(|(outcome, label)| (format!("\"{outcome}\""), label.to_string()));
        assert_eq!(verdicts, expected);
    }

    /// Through the batch dispatch path: a verified attempt grows durable
    /// knowledge and credits its prompt treatment and the playbook its prompt
    /// used with a success; a verify failure credits both with a failure and
    /// keeps the failing step and its output on the episode.
    #[tokio::test]
    async fn dispatch_outcomes_feed_knowledge_experiments_playbooks_and_episodes() {
        let temp = tempdir().expect("tempdir");
        let store_path = temp.path().join(".roko/learn/experiments.json");
        save_role_experiment(&store_path);
        let playbook_dir = temp.path().join(".roko/learn/playbooks");
        let playbooks = save_banner_playbook(&playbook_dir).await;
        let episodes_path = temp.path().join(".roko/episodes.jsonl");
        let facade = crate::runtime_feedback::FeedbackFacade::new()
            .with_sink(Arc::new(crate::runtime_feedback::EpisodeSink::at(
                &episodes_path,
            )))
            .with_sink(Arc::new(
                crate::runtime_feedback::VerifiedKnowledgeSink::for_workdir(temp.path()),
            ));
        let feedback = GraphFeedbackContext {
            feedback_facade: Some(Arc::new(facade)),
            experiment_store_path: Some(store_path.clone()),
            playbook_dir: Some(playbook_dir),
            ..GraphFeedbackContext::default()
        };
        let (dispatcher, mut task) =
            make_test_dispatcher(&temp, VERIFY_PROVIDER, no_auto_fix, feedback).await;
        task.title = "Render the greeting banner".into();
        task.verify = vec![verify_step("structural", "true")];
        dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &CellContext::new())
            .await
            .expect("the verified attempt passes");

        let mut failing = task.clone();
        failing.id = "T-FAIL".into();
        failing.verify = vec![verify_step(
            "check",
            "echo 'checking the banner'; echo 'banner.txt: the greeting is missing' >&2; exit 3",
        )];
        dispatcher
            .dispatch(&make_spec(&failing), Vec::new(), &CellContext::new())
            .await
            .expect_err("the failing verify step fails the attempt");

        let knowledge = roko_neuro::KnowledgeStore::for_workdir(temp.path())
            .read_all()
            .unwrap();
        assert!(
            knowledge.iter().any(|entry| {
                entry.source.as_deref() == Some("runtime:gate_verdict")
                    && entry.content.contains("Render the greeting banner")
            }),
            "{knowledge:#?}"
        );

        assert_eq!(
            experiment_trials(&store_path),
            (2, 1),
            "one observed success and one failure"
        );
        assert_eq!(playbook_counts(&playbooks).await, (1, 1));

        let episodes = roko_learn::episode_logger::EpisodeLogger::read_all(&episodes_path)
            .await
            .unwrap();
        let failed = episodes
            .iter()
            .find(|episode| episode.task_id == "T-FAIL")
            .expect("failure episode");
        let reason = failed.failure_reason.as_deref().unwrap_or_default();
        assert!(
            reason.starts_with("verify: 1/1 verify step(s) failed for task"),
            "{reason}"
        );
        assert!(reason.contains("verify[0:check]"), "{reason}");
        assert!(reason.contains("the greeting is missing"), "{reason}");
    }
}
