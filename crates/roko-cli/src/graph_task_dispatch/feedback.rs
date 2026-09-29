//! The learning/feedback context of a Graph plan run and the feedback each task
//! attempt emits once its outcome is settled.

use super::tui_forward::append_jsonl_line_async;
use super::verification::verify_step_label;
use super::*;

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
    /// P1-01: GateGamingDetector for flagging gaming patterns.
    pub gate_gaming_detector: Option<Arc<tokio::sync::Mutex<roko_learn::GateGamingDetector>>>,
    /// P1-04: HoldoutExperiment for gating learning updates (80/20 train/holdout split).
    pub holdout_experiment: Option<Arc<tokio::sync::Mutex<roko_learn::HoldoutExperiment>>>,
    /// P2-01: ShadowRunner for recording shadow dispatch decisions.
    pub shadow_runner: Option<Arc<ShadowRunner>>,
    /// P0-02: Whether eval generation is enabled for standard+ tier tasks.
    pub eval_generation_enabled: bool,
    /// P2-LRN-6 Loop 1: Path to `.roko/learn/gate-thresholds.json` for
    /// adaptive EMA threshold updates after each verify run.
    ///
    /// When set, each verify step outcome is fed into `GateThresholds::observe`
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
            .field("gate_gaming_detector", &self.gate_gaming_detector.is_some())
            .field("holdout_experiment", &self.holdout_experiment.is_some())
            .field("shadow_runner", &self.shadow_runner.is_some())
            .field("eval_generation_enabled", &self.eval_generation_enabled)
            .field("gate_thresholds_path", &self.gate_thresholds_path)
            .field("retrieval_outcomes_path", &self.retrieval_outcomes_path)
            .field("runs_dir", &self.runs_dir)
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
            gate_gaming_detector: None,
            holdout_experiment: None,
            shadow_runner: None,
            eval_generation_enabled: false,
            gate_thresholds_path: None,
            retrieval_outcomes_path: None,
            runs_dir: None,
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
    /// The sinks that predate S01 read its success flag and its bounded
    /// class-prefixed failure reason, which lands on the episode together
    /// with the provider-reported turn count; the episode, efficiency and
    /// cost rows carry its attempt key. A success of a task with authored
    /// verify steps also emits [`FeedbackEvent::TaskVerified`], which grows
    /// durable knowledge. The settlement itself goes out as
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
        let experiment_settlement =
            prompt_experiment::settlement(succeeded, failure_reason.as_deref());
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

            // Authored verify steps are deterministic and never
            // force-accepted, so a success of a task that declares them is a
            // gate-backed pass (`TaskGateVerdict::Passed`). Only those grow
            // durable knowledge.
            if succeeded && !task.verify.is_empty() {
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

        // ── W05: Efficiency event ────────────────────────────────────────
        if let Some(eff_path) = &self.feedback.efficiency_path {
            // Gather prompt diagnostics for the efficiency event so
            // telemetry reflects what the agent actually received.
            let eff_prompt_sections: Vec<roko_learn::efficiency::PromptSectionMeta> = dispatch_plan
                .prompt
                .diagnostics
                .included_sections
                .iter()
                .map(|name| roko_learn::efficiency::PromptSectionMeta {
                    name: name.clone(),
                    tokens: 0,
                    priority: 0,
                    was_truncated: false,
                    was_dropped: false,
                })
                .collect();
            let eff_system_prompt_tokens = dispatch_plan.prompt.diagnostics.estimated_tokens;
            let eff_tool_calls: Vec<roko_learn::efficiency::ToolCallMeta> = dispatch
                .events
                .iter()
                .filter_map(|ev| match ev {
                    roko_agent::AgentRuntimeEvent::ToolCall { name, .. } => {
                        Some(roko_learn::efficiency::ToolCallMeta {
                            tool_name: name.clone(),
                            duration_ms: 0,
                            result_tokens: 0,
                            succeeded: true,
                            advanced_task: false,
                            was_redundant: false,
                            error_category: None,
                        })
                    }
                    _ => None,
                })
                .collect();
            let eff_tools_used = eff_tool_calls.len() as u32;
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
                reasoning_tokens: 0,
                cache_read_tokens: u64::from(dispatch.result.usage.cache_read_tokens),
                cache_write_tokens: u64::from(dispatch.result.usage.cache_create_tokens),
                cost_usd,
                cost_usd_without_cache: cost_usd,
                prompt_sections: eff_prompt_sections,
                total_prompt_tokens: tokens_in,
                system_prompt_tokens: u64::from(eff_system_prompt_tokens),
                tools_available: eff_tool_calls.len() as u32,
                tools_used: eff_tools_used,
                tool_calls: eff_tool_calls,
                wall_time_ms: duration_ms,
                duration_ms,
                time_to_first_token_ms: 0,
                // No provider process is pre-spawned or reused, so every
                // dispatch is a cold start.
                was_warm_start: false,
                iteration: agent_num_turns,
                turn_number: agent_num_turns,
                is_final_turn: true,
                gate_passed: None,
                outcome: if succeeded {
                    "success".to_string()
                } else {
                    "failure".to_string()
                },
                gate_errors: vec![],
                model_used: model_slug.clone(),
                frequency: roko_core::OperatingFrequency::Gamma,
                strategy_attempted: String::new(),
                timestamp: chrono::Utc::now().to_rfc3339(),
            };
            let row = AttemptKeyed {
                attempt_key: attempt_key.to_string(),
                row: roko_learn::efficiency::ExecutedRow::new(&event, &settled.verdict.executed),
            };
            match serde_json::to_string(&row) {
                Ok(line) => {
                    let path = eff_path.clone();
                    let plan_id = spec.plan_id.clone();
                    let task_id = task.id.clone();
                    tokio::spawn(async move {
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
            };
            let row = AttemptKeyed {
                attempt_key: attempt_key.to_string(),
                row: roko_learn::efficiency::ExecutedRow::new(
                    &cost_record,
                    &settled.verdict.executed,
                ),
            };
            match serde_json::to_string(&row) {
                Ok(line) => {
                    let path = costs_path.clone();
                    let plan_id = spec.plan_id.clone();
                    let task_id = task.id.clone();
                    tokio::spawn(async move {
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
        // Credit the playbooks prompt assembly actually injected.
        if let Some(playbook_dir) = &self.feedback.playbook_dir {
            let store = roko_learn::playbook::PlaybookStore::new(playbook_dir);
            for playbook_id in &diagnostics.playbook_ids {
                if let Err(error) = store.record_outcome(playbook_id, succeeded).await {
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
        if let Some(daimon) = &self.feedback.daimon_state {
            use roko_daimon::AffectEngine;
            let event = roko_daimon::AffectEvent::TaskOutcome {
                task_id: task.id.clone(),
                succeeded,
            };
            if let Ok(mut state) = daimon.lock() {
                let _ = state.appraise(event);
            }
        }

        // ── W14: Experiment settlement ───────────────────────────────────
        //
        // Settles this attempt's prompt treatments (prepared at prompt
        // assembly, bound to the launched prompt) with its outcome.
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
    /// per attempt.
    pub(super) async fn publish_settlement(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        settled: &SettledAttempt,
    ) {
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

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;
    use crate::graph_task_dispatch::tests::{
        VERIFY_PROVIDER, make_spec, make_test_dispatcher, no_auto_fix, verify_step,
    };

    /// Through the batch dispatch path: a verified attempt grows durable
    /// knowledge and credits its prompt treatment and the playbook its prompt
    /// used with a success; a verify failure credits both with a failure and
    /// keeps the failing step and its output on the episode.
    #[tokio::test]
    async fn dispatch_outcomes_feed_knowledge_experiments_playbooks_and_episodes() {
        use roko_learn::prompt_experiment::{ExperimentStore, PromptExperiment, PromptVariant};

        let temp = tempdir().expect("tempdir");
        let store_path = temp.path().join(".roko/learn/experiments.json");
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
        store.save(&store_path).unwrap();
        let playbook_dir = temp.path().join(".roko/learn/playbooks");
        let playbooks = roko_learn::playbook::PlaybookStore::new(&playbook_dir);
        playbooks
            .save(&roko_learn::playbook::Playbook::new(
                "banner-steps",
                "Render a greeting banner",
            ))
            .await
            .unwrap();
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

        let stats = ExperimentStore::load_strict(&store_path)
            .unwrap()
            .get("role-ab")
            .unwrap()
            .stats
            .values()
            .fold((0, 0), |(trials, successes), stats| {
                (trials + stats.trials, successes + stats.successes)
            });
        assert_eq!(stats, (2, 1), "one observed success and one failure");
        let playbook = playbooks.load("banner-steps").await.unwrap().unwrap();
        assert_eq!((playbook.success_count, playbook.failure_count), (1, 1));

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
