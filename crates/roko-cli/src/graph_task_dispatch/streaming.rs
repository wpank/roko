//! The streaming dispatch path (#274) and crash-safe attempt reconciliation.

use super::*;

/// Bounded channel capacity for streaming graph task events (#233).
const STREAMING_EVENT_CHANNEL_CAPACITY: usize = 256;

/// Recommended channel capacity for callers constructing an event sender.
///
/// Text/progress events may coalesce across sends. Tool boundaries, usage,
/// attempt receipt, and terminal outcome events are reliable and must not be
/// dropped.
#[must_use]
pub const fn streaming_event_channel_capacity() -> usize {
    STREAMING_EVENT_CHANNEL_CAPACITY
}

#[async_trait::async_trait]
impl StreamingTaskDispatcher for GraphTaskDispatcher {
    async fn dispatch_streaming(
        &self,
        spec: &TaskExecutionSpec,
        input: Vec<Signal>,
        ctx: &CellContext,
        lease: &TaskLease,
        event_tx: tokio::sync::mpsc::Sender<GraphTaskEvent>,
        recorder: &dyn ProviderAttemptRecorder,
    ) -> Result<TaskDispatchOutcome> {
        // ── Lease validation ─────────────────────────────────────────────
        if !lease.path.exists() {
            return Err(RokoError::Agent {
                backend: "graph-task-executor".to_string(),
                message: format!(
                    "lease path `{}` does not exist; the caller must acquire the lease before dispatch",
                    lease.path.display()
                ),
            });
        }
        // When worktree isolation is active, the lease path is the
        // worktree (not the repo root), so the mismatch is expected.
        // Only enforce the strict check when no workspace provider is set.
        if self.workspace_provider.is_none() && lease.path != self.workdir {
            return Err(RokoError::Agent {
                backend: "graph-task-executor".to_string(),
                message: format!(
                    "lease path `{}` does not match the dispatcher workdir `{}`; shared-checkout workdirs are rejected",
                    lease.path.display(),
                    self.workdir.display()
                ),
            });
        }

        // ── Budget reservation ───────────────────────────────────────────
        let budget_reservation = self
            .budget_ledger
            .reserve(&spec.plan_id, self.budget_policy)?;

        // ── Attempt identity ─────────────────────────────────────────────
        let attempt_id = format!(
            "{}/{}/{}",
            spec.plan_id,
            ctx.cell_id.as_deref().unwrap_or("unknown"),
            uuid::Uuid::new_v4()
        );

        // Record attempt-start receipt before provider launch.
        recorder.record_start(&attempt_id, spec).await?;

        // Notify the event channel that the attempt has started.
        let _ = event_tx
            .send(GraphTaskEvent::AttemptStarted {
                attempt_id: attempt_id.clone(),
            })
            .await;

        let started_at = Instant::now();

        // ── Task + dispatch context ──────────────────────────────────────
        let task: TaskDef = serde_json::from_str(&spec.task_def_json).map_err(|error| {
            RokoError::Planning(format!(
                "decode task definition for `{}`: {error}",
                spec.title
            ))
        })?;
        let role = task.role.as_deref().unwrap_or("implementer");
        let _in_flight = self.in_flight.register(
            &format!("{}/{}", spec.plan_id, task.id),
            &lease.path,
            &task.files,
        );

        // ── P3-AGT-2: Express mode check (streaming) ─────────────────────
        let express_active = is_express_task(&self.config, &task);
        let max_turns = task_turn_limit(&self.config, &task, express_active);
        if express_active {
            tracing::info!(
                plan_id = %spec.plan_id,
                task_id = %task.id,
                tier = %task.tier,
                fast_model = %self.config.routing.fast_task_model,
                max_turns,
                "P3-AGT-2: express mode active (streaming) — routing to fast model with reduced turn limit"
            );
        }

        // ── W10: Enrichment pipeline (streaming) ─────────────────────────
        let routing_ctx = build_routing_context(role, &task, &self.feedback.daimon_state);
        // Clone before the move into DispatchContext so emit_feedback can pass
        // the real dispatch-time context to the routing observation sink.
        let routing_ctx_for_feedback = routing_ctx.clone();

        let (cached_workspace_map, cached_workspace_context, cached_cfactor_context) =
            self.static_prompt_cache.get_or_init(|| {
                let ws_map =
                    crate::dispatch::prompt_builder::generate_workspace_map_pub(&self.workdir);
                let ws_ctx =
                    crate::dispatch::prompt_builder::generate_workspace_context_pub(&self.workdir);
                let cf_ctx =
                    crate::dispatch::prompt_builder::generate_cfactor_context_pub(&self.workdir);
                tracing::debug!(
                    ws_map_bytes = ws_map.len(),
                    ws_ctx_bytes = ws_ctx.len(),
                    cf_ctx_bytes = cf_ctx.len(),
                    "static_prompt_cache: computed once for this run (streaming path)"
                );
                (ws_map, ws_ctx, cf_ctx)
            });
        let express_force_backend_streaming = if express_active && self.cli_model_override.is_none()
        {
            Some(self.config.routing.fast_task_model.clone())
        } else {
            None
        };
        let retry_key = format!("{}/{}", spec.plan_id, task.id);
        // The attempt opens before prompt assembly (S01 §4.2).
        let mut attempt = self.open_attempt(spec, &task, ctx);
        let attempt_key = attempt.key.attempt_key();
        let prompt_experiment = self
            .feedback
            .experiment_store_path
            .as_deref()
            .and_then(|store| prompt_experiment::context(store, &attempt.key));
        let mut dispatch_ctx = DispatchContext {
            plan_id: spec.plan_id.clone(),
            role: role.to_string(),
            workdir: lease.path.clone(),
            model_hint: task.model_hint.clone(),
            force_backend: self
                .cli_model_override
                .clone()
                .or(express_force_backend_streaming),
            budget_remaining_usd: effective_routing_budget(
                ctx.budget_remaining,
                budget_reservation.routing_budget_usd(),
            ),
            attempt: 0,
            prompt_experiment: prompt_experiment.clone(),
            gate_feedback: None,
            routing_context: Some(routing_ctx),
            routing_bias: None,
            dependency_outputs: upstream_outputs(&input),
            error_patterns_context: self.factory.format_error_patterns_for_prompt(5),
            cached_workspace_map: cached_workspace_map.clone(),
            cached_workspace_context: cached_workspace_context.clone(),
            cached_cfactor_context: cached_cfactor_context.clone(),
        };
        let dispatch_plan = match self.plan_dispatch(spec, &task, &mut dispatch_ctx) {
            Ok(dispatch_plan) => dispatch_plan,
            Err(error) => return Err(self.fail_attempt(spec, &task, attempt, None, error).await),
        };
        attempt.prompt_assembled();
        let contract = effective_agent_contract(role, &task);
        let timeout_ms = base_attempt_timeout_ms(&self.config, spec);
        let request = AgentDispatchRequest {
            model_key: dispatch_plan.model.slug.clone(),
            prompt: dispatch_plan.prompt.user_prompt.clone(),
            system_prompt: dispatch_plan.prompt.system_prompt.clone(),
            workdir: lease.path.clone(),
            immune_root: Some(lease.path.clone()),
            agent_id: format!(
                "{}/{}",
                spec.plan_id,
                ctx.cell_id.as_deref().unwrap_or(&task.id)
            ),
            command: None,
            timeout_ms: Some(timeout_ms),
            mcp_config: self.config.agent.mcp_config.clone(),
            env: Vec::new(),
            extra_args: Vec::new(),
            effort: Some(self.config.agent.default_effort.clone()),
            tools: None,
            agent_contract: Some(contract),
            bare_mode: self.config.agent.bare_mode,
            dangerously_skip_permissions: self.dangerously_skip_permissions,
            max_turns: Some(max_turns),
            live_output: None,
            attempt_key: Some(attempt_key.clone()),
        };
        let _launched_treatments = prompt_experiment::LaunchedTreatments::bind(
            prompt_experiment,
            &dispatch_plan.prompt.diagnostics.experiment_assignments,
            &request.system_prompt,
            &request.prompt,
        )
        .await;

        // ── Live output forwarder (streaming path) ────────────────────────
        let request = {
            let mut req = request;
            if let (Some(tui), Some(live_setting)) = (&self.tui_bridge, &self.live_agent_output) {
                let (live_tx, mut live_rx) =
                    tokio::sync::mpsc::channel::<roko_agent::live_output::LiveAgentEvent>(64);
                let trusted = matches!(live_setting, LiveAgentOutput::Trusted);
                req.live_output = Some(roko_agent::live_output::LiveOutput {
                    sink: live_tx,
                    trusted,
                });
                let tui_clone = tui.clone();
                let agent_id_s = format!(
                    "{}/{}",
                    spec.plan_id,
                    ctx.cell_id.as_deref().unwrap_or(&task.id)
                );
                let plan_id_s = spec.plan_id.clone();
                let task_id_s = task.id.clone();
                tokio::spawn(async move {
                    while let Some(event) = live_rx.recv().await {
                        forward_live_event_to_tui(
                            &tui_clone,
                            &agent_id_s,
                            &plan_id_s,
                            &task_id_s,
                            event,
                        );
                    }
                });
            }
            req
        };

        // ── Provider invocation ──────────────────────────────────────────
        attempt.dispatch_started();
        let mut dispatch_result = self.factory.run_shared_agent_bridge(request).await;
        attempt.dispatch_ended();
        // A model the provider substituted is priced by the model that
        // served (bug-31438d). This path runs no failover, and a `--model`
        // pin fails only the batch path's attempt.
        if let Ok(dispatch) = dispatch_result.as_mut() {
            let _ = self.check_served_model(spec, &task.id, dispatch);
        }

        let wall_duration = started_at.elapsed();

        // ── TUI streaming output (streaming path) ──────────────────────
        // Forward provider events to the TUI bridge in the streaming path too.
        if let Ok(dispatch) = &dispatch_result {
            self.forward_dispatch_events_to_tui(spec, &task, dispatch, ctx);
        }

        // ── Map provider events to graph events ──────────────────────────
        // Forward provider dispatch events as streaming graph events.
        match &dispatch_result {
            Ok(dispatch) => {
                for event in &dispatch.events {
                    let graph_event = match event {
                        roko_agent::AgentRuntimeEvent::MessageDelta { text } => {
                            Some(GraphTaskEvent::Text { text: text.clone() })
                        }
                        roko_agent::AgentRuntimeEvent::ToolCall { id, name } => {
                            Some(GraphTaskEvent::ToolCall {
                                id: id.clone(),
                                name: name.clone(),
                            })
                        }
                        roko_agent::AgentRuntimeEvent::ToolOutput { id, output } => {
                            Some(GraphTaskEvent::ToolOutput {
                                id: id.clone(),
                                output: output.clone(),
                            })
                        }
                        roko_agent::AgentRuntimeEvent::TokenUsage {
                            input_tokens,
                            output_tokens,
                            ..
                        } => Some(GraphTaskEvent::Usage {
                            input_tokens: *input_tokens,
                            output_tokens: *output_tokens,
                            cost_usd: None,
                        }),
                        _ => None,
                    };
                    if let Some(event) = graph_event {
                        // Best-effort send; text/progress may coalesce.
                        let _ = event_tx.send(event).await;
                    }
                }
            }
            Err(_) => {}
        }

        // ── Settle cost and build outcome ────────────────────────────────
        let (outcome, output_signals, verification) = match dispatch_result {
            Ok(dispatch) => {
                let cost_usd = f64::from(dispatch.result.usage.cost_usd);
                let actual_cost = if cost_usd.is_finite() && cost_usd > 0.0 {
                    Some(cost_usd)
                } else {
                    tracing::debug!(
                        attempt = %attempt_id,
                        "provider did not report cost; recording None"
                    );
                    None
                };
                self.task_spend
                    .record(&format!("{}/{}", spec.plan_id, task.id), cost_usd);
                if let Err(error) = budget_reservation.settle(cost_usd.max(0.0)) {
                    let routed = Some((dispatch_plan.model.slug.as_str(), &dispatch));
                    return Err(self.fail_attempt(spec, &task, attempt, routed, error).await);
                }

                // Forward final usage event with cost.
                let _ = event_tx
                    .send(GraphTaskEvent::Usage {
                        input_tokens: u64::from(dispatch.result.usage.input_tokens),
                        output_tokens: u64::from(dispatch.result.usage.output_tokens),
                        cost_usd: actual_cost,
                    })
                    .await;

                // ── Verify steps (streaming) ─────────────────────────────
                //
                // Same verdict logic as the batch path; gates run in the lease
                // path and progress streams through the event channel.
                let helper_calls = HelperCalls::default();
                let verification = if dispatch.result.success {
                    let attempt_number = self.next_retry_attempt(&spec.plan_id, &task.id).attempt;
                    Some(
                        helper_calls
                            .scope(self.settle_task_verification(
                                spec,
                                &task,
                                &dispatch,
                                &lease.path,
                                &retry_key,
                                attempt_number,
                                &attempt_key,
                                Some(&event_tx),
                            ))
                            .await,
                    )
                } else {
                    None
                };
                attempt.record_helper_calls(
                    self.settle_helper_calls(spec, &task, &attempt_key, &helper_calls)
                        .await,
                );
                let verified = matches!(verification, Some(Ok(_)));

                // ── Learning/feedback pipeline (streaming) ───────────────
                //
                // Settled after the gate so learning sees the verified outcome.
                let settlement = match &verification {
                    Some(verification) => Settlement::verified(verification),
                    None => Settlement::provider_failure(
                        dispatch.result.output.body.as_text().unwrap_or_default(),
                        first_token_seen(&dispatch),
                    ),
                };
                let settled =
                    attempt.settle(settlement, &dispatch_plan.model.slug, Some(&dispatch));
                self.emit_feedback(
                    spec,
                    &task,
                    &settled,
                    &dispatch,
                    wall_duration,
                    &dispatch_plan,
                    Some(routing_ctx_for_feedback),
                )
                .await;

                let outcome_kind = if verified {
                    TaskDispatchOutcomeKind::Succeeded
                } else {
                    TaskDispatchOutcomeKind::Failed
                };

                let output_signals = match &verification {
                    Some(Ok(verdict)) => {
                        let mut output = dispatch.result.output;
                        if output.body.as_text().is_err() {
                            output = Signal::builder(Kind::AgentOutput)
                                .body(Body::text(format!(
                                    "provider `{}` completed task `{}`",
                                    dispatch.target.provider_id, spec.title
                                )))
                                .build();
                        }
                        let mut outputs = vec![output];
                        verdict.stamp(&mut outputs);
                        outputs
                    }
                    _ => Vec::new(),
                };

                let dispatch_outcome = TaskDispatchOutcome {
                    attempt_id: attempt_id.clone(),
                    outcome: outcome_kind,
                    provider_id: dispatch.target.provider_id.clone(),
                    model: dispatch.target.model_slug.clone(),
                    input_tokens: Some(u64::from(dispatch.result.usage.input_tokens)),
                    output_tokens: Some(u64::from(dispatch.result.usage.output_tokens)),
                    cost_usd: actual_cost,
                    changed_files: Vec::new(), // Changed files computed relative to lease base.
                    wall_duration,
                    output: output_signals.clone(),
                };

                (dispatch_outcome, output_signals, verification)
            }
            Err(error) => {
                // No provider result reached the sinks that predate S01; the
                // attempt's verdict is recorded.
                let settlement = Settlement::provider_failure(&error.to_string(), false);
                let settled = attempt.settle(settlement, &dispatch_plan.model.slug, None);
                self.publish_settlement(spec, &task, &settled).await;

                let dispatch_outcome = TaskDispatchOutcome {
                    attempt_id: attempt_id.clone(),
                    outcome: TaskDispatchOutcomeKind::Failed,
                    provider_id: "graph-task-executor".to_string(),
                    model: String::new(),
                    input_tokens: None,
                    output_tokens: None,
                    cost_usd: None,
                    changed_files: Vec::new(),
                    wall_duration,
                    output: Vec::new(),
                };

                // Record terminal receipt before propagating the error.
                let _ = recorder
                    .record_terminal(&attempt_id, &dispatch_outcome)
                    .await;

                // Send terminal event.
                let _ = event_tx
                    .send(GraphTaskEvent::AttemptTerminal {
                        attempt_id,
                        outcome: TaskDispatchOutcomeKind::Failed,
                    })
                    .await;

                return Err(RokoError::Agent {
                    backend: "graph-task-executor".to_string(),
                    message: error.to_string(),
                });
            }
        };

        // ── Terminal receipt and event ────────────────────────────────────
        recorder.record_terminal(&attempt_id, &outcome).await?;

        let _ = event_tx
            .send(GraphTaskEvent::AttemptTerminal {
                attempt_id: attempt_id.clone(),
                outcome: outcome.outcome,
            })
            .await;

        // A verify failure is reported as such; otherwise an unsuccessful
        // provider result fails the task even though cost was settled
        // (callers still incur the charge).
        if let Some(Err(error)) = verification {
            return Err(error);
        }
        if outcome.outcome == TaskDispatchOutcomeKind::Failed {
            return Err(RokoError::Agent {
                backend: outcome.provider_id.clone(),
                message: "provider returned an unsuccessful result".to_string(),
            });
        }

        Ok(TaskDispatchOutcome {
            output: output_signals,
            ..outcome
        })
    }

    async fn reconcile_attempt(
        &self,
        _spec: &TaskExecutionSpec,
        previous_attempt_id: &str,
        recorder: &dyn ProviderAttemptRecorder,
    ) -> AttemptReconciliation {
        // Check if the previous attempt has terminal evidence: if so, the
        // caller should reuse the committed result without re-invoking the
        // provider.
        if recorder.has_terminal_evidence(previous_attempt_id).await {
            return AttemptReconciliation::ReuseCommitted {
                attempt_id: previous_attempt_id.to_string(),
            };
        }

        // Check if the previous attempt started but never reached a terminal
        // state. This is ambiguous: the provider may have been invoked and we
        // cannot know whether it completed. Do not retry.
        if recorder.has_started_evidence(previous_attempt_id).await {
            return AttemptReconciliation::FailAmbiguous {
                attempt_id: previous_attempt_id.to_string(),
                reason: format!(
                    "attempt `{previous_attempt_id}` started but has no terminal evidence; \
                     the provider may have been invoked and cannot be safely retried"
                ),
            };
        }

        // No evidence that the previous attempt ever started. Allocate a
        // fresh attempt ID for a new dispatch.
        AttemptReconciliation::AllocateNew {
            attempt_id: format!("{previous_attempt_id}-retry-{}", uuid::Uuid::new_v4()),
        }
    }
}

#[cfg(test)]
mod tests {
    use roko_graph::cells::{AttemptReconciliation, NoopAttemptRecorder};
    use tempfile::tempdir;

    use super::*;
    use crate::graph_task_dispatch::tests::{
        VERIFY_PROVIDER, make_spec, make_test_dispatcher, no_auto_fix, verify_step,
    };

    #[tokio::test]
    async fn streaming_verify_failure_is_a_failed_terminal_attempt() {
        let temp = tempdir().expect("tempdir");
        let (dispatcher, mut task) = make_test_dispatcher(
            &temp,
            VERIFY_PROVIDER,
            no_auto_fix,
            GraphFeedbackContext::default(),
        )
        .await;
        task.verify = vec![
            verify_step("structural", "false"),
            verify_step("compile", "true"),
        ];
        let lease = TaskLease {
            path: temp.path().to_path_buf(),
            fingerprint: "test-fingerprint".to_string(),
        };
        let (event_tx, mut event_rx) =
            tokio::sync::mpsc::channel(streaming_event_channel_capacity());

        let error = dispatcher
            .dispatch_streaming(
                &make_spec(&task),
                Vec::new(),
                &CellContext::new().with_cell_id("T-STREAM".to_string()),
                &lease,
                event_tx,
                &NoopAttemptRecorder,
            )
            .await
            .expect_err("streaming verify failure");
        assert!(matches!(error, RokoError::Verify { .. }), "{error}");

        let mut events = Vec::new();
        while let Ok(event) = event_rx.try_recv() {
            events.push(event);
        }
        let terminal: Vec<_> = events
            .iter()
            .filter_map(|event| match event {
                GraphTaskEvent::AttemptTerminal { outcome, .. } => Some(*outcome),
                _ => None,
            })
            .collect();
        assert_eq!(terminal, vec![TaskDispatchOutcomeKind::Failed]);
    }

    // ─── Streaming dispatch tests (#274) ─────────────────────────────────────

    /// Helper: create a `GraphTaskDispatcher` with a fake CLI provider.
    async fn make_streaming_dispatcher(
        temp: &tempfile::TempDir,
        script_content: &str,
    ) -> (Arc<GraphTaskDispatcher>, TaskDef) {
        make_test_dispatcher(
            temp,
            script_content,
            |_| {},
            GraphFeedbackContext::default(),
        )
        .await
    }

    #[tokio::test]
    async fn streaming_dispatch_sends_attempt_started_and_terminal_events() {
        let temp = tempdir().expect("tempdir");
        let (dispatcher, task) = make_streaming_dispatcher(
            &temp,
            r#"#!/bin/sh
set -eu
cat >/dev/null
printf '%s\n' '{"type":"content_block_delta","delta":{"text":"streaming-output"}}'
printf '%s\n' '{"type":"result","session_id":"sess-s1","model":"claude-sonnet-4-6","total_cost_usd":0.10,"usage":{"input_tokens":5,"output_tokens":10}}'
"#,
        )
        .await;

        let spec = make_spec(&task);
        let lease = TaskLease {
            path: temp.path().to_path_buf(),
            fingerprint: "test-fingerprint".to_string(),
        };
        let (event_tx, mut event_rx) =
            tokio::sync::mpsc::channel(streaming_event_channel_capacity());
        let recorder = NoopAttemptRecorder;

        let outcome = dispatcher
            .dispatch_streaming(
                &spec,
                Vec::new(),
                &CellContext::new().with_cell_id("T-STREAM".to_string()),
                &lease,
                event_tx,
                &recorder,
            )
            .await
            .expect("streaming dispatch");

        assert_eq!(outcome.outcome, TaskDispatchOutcomeKind::Succeeded);
        assert!(!outcome.attempt_id.is_empty());
        assert!(outcome.cost_usd.is_some());
        assert!(!outcome.output.is_empty());

        // Collect all events.
        let mut events = Vec::new();
        while let Ok(event) = event_rx.try_recv() {
            events.push(event);
        }

        // Must have at least AttemptStarted and AttemptTerminal.
        let started = events
            .iter()
            .any(|e| matches!(e, GraphTaskEvent::AttemptStarted { .. }));
        let terminal = events.iter().any(|e| {
            matches!(
                e,
                GraphTaskEvent::AttemptTerminal {
                    outcome: TaskDispatchOutcomeKind::Succeeded,
                    ..
                }
            )
        });
        assert!(started, "must emit AttemptStarted event");
        assert!(terminal, "must emit AttemptTerminal(Succeeded) event");

        // Must have usage event with cost.
        let usage = events.iter().any(|e| {
            matches!(
                e,
                GraphTaskEvent::Usage {
                    cost_usd: Some(_),
                    ..
                }
            )
        });
        assert!(usage, "must emit Usage event with actual cost");
    }

    #[tokio::test]
    async fn streaming_dispatch_rejects_missing_lease_path() {
        let temp = tempdir().expect("tempdir");
        let (dispatcher, task) = make_streaming_dispatcher(&temp, "#!/bin/sh\nexit 0\n").await;

        let spec = make_spec(&task);
        let lease = TaskLease {
            path: temp.path().join("nonexistent-lease"),
            fingerprint: "fp".to_string(),
        };
        let (event_tx, _event_rx) = tokio::sync::mpsc::channel(streaming_event_channel_capacity());
        let recorder = NoopAttemptRecorder;

        let error = dispatcher
            .dispatch_streaming(
                &spec,
                Vec::new(),
                &CellContext::new(),
                &lease,
                event_tx,
                &recorder,
            )
            .await
            .expect_err("missing lease path must fail");

        assert!(error.to_string().contains("does not exist"));
    }

    #[tokio::test]
    async fn streaming_dispatch_rejects_mismatched_workdir() {
        let temp = tempdir().expect("tempdir");
        let other_dir = tempdir().expect("other tempdir");
        let (dispatcher, task) = make_streaming_dispatcher(&temp, "#!/bin/sh\nexit 0\n").await;

        let spec = make_spec(&task);
        let lease = TaskLease {
            path: other_dir.path().to_path_buf(),
            fingerprint: "fp".to_string(),
        };
        let (event_tx, _event_rx) = tokio::sync::mpsc::channel(streaming_event_channel_capacity());
        let recorder = NoopAttemptRecorder;

        let error = dispatcher
            .dispatch_streaming(
                &spec,
                Vec::new(),
                &CellContext::new(),
                &lease,
                event_tx,
                &recorder,
            )
            .await
            .expect_err("mismatched workdir must fail");

        assert!(error.to_string().contains("shared-checkout"));
    }

    #[tokio::test]
    async fn streaming_dispatch_settles_cost_on_provider_failure() {
        let temp = tempdir().expect("tempdir");
        let (dispatcher, task) = make_streaming_dispatcher(
            &temp,
            r#"#!/bin/sh
set -eu
cat >/dev/null
printf '%s\n' '{"type":"content_block_delta","delta":{"text":"fail-output"}}'
printf '%s\n' '{"type":"result","session_id":"sess-f1","model":"claude-sonnet-4-6","total_cost_usd":0.15,"usage":{"input_tokens":3,"output_tokens":4},"is_error":true}'
exit 1
"#,
        )
        .await;

        let spec = make_spec(&task);
        let lease = TaskLease {
            path: temp.path().to_path_buf(),
            fingerprint: "fp".to_string(),
        };
        let (event_tx, mut event_rx) =
            tokio::sync::mpsc::channel(streaming_event_channel_capacity());
        let recorder = NoopAttemptRecorder;

        let error = dispatcher
            .dispatch_streaming(
                &spec,
                Vec::new(),
                &CellContext::new().with_cell_id("T-FAIL".to_string()),
                &lease,
                event_tx,
                &recorder,
            )
            .await
            .expect_err("failed provider must error");

        assert!(matches!(error, RokoError::Agent { .. }));

        // Terminal event must still be emitted for failures.
        let mut events = Vec::new();
        while let Ok(event) = event_rx.try_recv() {
            events.push(event);
        }
        let terminal = events.iter().any(|e| {
            matches!(
                e,
                GraphTaskEvent::AttemptTerminal {
                    outcome: TaskDispatchOutcomeKind::Failed,
                    ..
                }
            )
        });
        assert!(terminal, "must emit AttemptTerminal(Failed) event");
    }

    #[tokio::test]
    async fn reconcile_returns_reuse_committed_for_terminal_evidence() {
        use std::sync::atomic::{AtomicBool, Ordering};

        /// Test recorder that reports terminal evidence for a specific attempt.
        struct TerminalRecorder {
            terminal: AtomicBool,
        }

        #[async_trait::async_trait]
        impl ProviderAttemptRecorder for TerminalRecorder {
            async fn record_start(&self, _id: &str, _spec: &TaskExecutionSpec) -> Result<()> {
                Ok(())
            }
            async fn record_terminal(
                &self,
                _id: &str,
                _outcome: &TaskDispatchOutcome,
            ) -> Result<()> {
                self.terminal.store(true, Ordering::SeqCst);
                Ok(())
            }
            async fn has_terminal_evidence(&self, _id: &str) -> bool {
                self.terminal.load(Ordering::SeqCst)
            }
            async fn has_started_evidence(&self, _id: &str) -> bool {
                false
            }
        }

        let temp = tempdir().expect("tempdir");
        let (dispatcher, task) = make_streaming_dispatcher(&temp, "#!/bin/sh\nexit 0\n").await;
        let spec = make_spec(&task);

        let recorder = TerminalRecorder {
            terminal: AtomicBool::new(true),
        };

        let result = StreamingTaskDispatcher::reconcile_attempt(
            &*dispatcher,
            &spec,
            "prev-attempt-1",
            &recorder,
        )
        .await;

        assert!(
            matches!(result, AttemptReconciliation::ReuseCommitted { .. }),
            "terminal evidence must return ReuseCommitted"
        );
    }

    #[tokio::test]
    async fn reconcile_returns_fail_ambiguous_for_started_evidence() {
        /// Recorder that reports started-but-not-terminal evidence.
        struct StartedRecorder;

        #[async_trait::async_trait]
        impl ProviderAttemptRecorder for StartedRecorder {
            async fn record_start(&self, _id: &str, _spec: &TaskExecutionSpec) -> Result<()> {
                Ok(())
            }
            async fn record_terminal(
                &self,
                _id: &str,
                _outcome: &TaskDispatchOutcome,
            ) -> Result<()> {
                Ok(())
            }
            async fn has_terminal_evidence(&self, _id: &str) -> bool {
                false
            }
            async fn has_started_evidence(&self, _id: &str) -> bool {
                true
            }
        }

        let temp = tempdir().expect("tempdir");
        let (dispatcher, task) = make_streaming_dispatcher(&temp, "#!/bin/sh\nexit 0\n").await;
        let spec = make_spec(&task);

        let result = StreamingTaskDispatcher::reconcile_attempt(
            &*dispatcher,
            &spec,
            "prev-attempt-ambig",
            &StartedRecorder,
        )
        .await;

        assert!(
            matches!(result, AttemptReconciliation::FailAmbiguous { .. }),
            "started-but-not-terminal must return FailAmbiguous"
        );
    }

    #[tokio::test]
    async fn reconcile_returns_allocate_new_for_no_evidence() {
        let temp = tempdir().expect("tempdir");
        let (dispatcher, task) = make_streaming_dispatcher(&temp, "#!/bin/sh\nexit 0\n").await;
        let spec = make_spec(&task);
        let recorder = NoopAttemptRecorder;

        let result = StreamingTaskDispatcher::reconcile_attempt(
            &*dispatcher,
            &spec,
            "prev-never-started",
            &recorder,
        )
        .await;

        match result {
            AttemptReconciliation::AllocateNew { attempt_id } => {
                assert!(
                    attempt_id.contains("prev-never-started"),
                    "new attempt ID must reference the original"
                );
            }
            other => panic!("expected AllocateNew, got {other:?}"),
        }
    }

    #[test]
    fn streaming_event_channel_capacity_is_bounded() {
        let capacity = streaming_event_channel_capacity();
        assert!(
            capacity > 0 && capacity <= 1024,
            "channel capacity {capacity} must be bounded and reasonable"
        );
    }

    #[tokio::test]
    async fn streaming_dispatch_respects_budget_exhaustion() {
        let temp = tempdir().expect("tempdir");
        let (dispatcher, task) = make_streaming_dispatcher(
            &temp,
            r#"#!/bin/sh
set -eu
cat >/dev/null
printf '%s\n' '{"type":"content_block_delta","delta":{"text":"expensive"}}'
printf '%s\n' '{"type":"result","session_id":"sess-x","model":"claude-sonnet-4-6","total_cost_usd":1.00,"usage":{"input_tokens":50,"output_tokens":100}}'
"#,
        )
        .await;

        let spec = make_spec(&task);
        let lease = TaskLease {
            path: temp.path().to_path_buf(),
            fingerprint: "fp".to_string(),
        };
        let recorder = NoopAttemptRecorder;

        // First dispatch exhausts the $1.00 budget.
        let (event_tx, _) = tokio::sync::mpsc::channel(streaming_event_channel_capacity());
        let _ = dispatcher
            .dispatch_streaming(
                &spec,
                Vec::new(),
                &CellContext::new().with_cell_id("T-EXP-1".to_string()),
                &lease,
                event_tx,
                &recorder,
            )
            .await;

        // Second dispatch must fail with budget exhaustion.
        let (event_tx2, _) = tokio::sync::mpsc::channel(streaming_event_channel_capacity());
        let error = dispatcher
            .dispatch_streaming(
                &spec,
                Vec::new(),
                &CellContext::new().with_cell_id("T-EXP-2".to_string()),
                &lease,
                event_tx2,
                &recorder,
            )
            .await
            .expect_err("budget-exhausted dispatch must fail");

        assert!(
            matches!(error, RokoError::BudgetExceeded { .. }),
            "error must be BudgetExceeded, got: {error:?}"
        );
    }
}
