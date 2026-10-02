//! Authored verify steps of a Graph task attempt, and the gate-dependent learning
//! records their verdict settles.

use super::tui_forward::append_jsonl_line_async;
use super::turn_policy::head_and_tail;
use super::*;

impl GraphTaskDispatcher {
    /// Screen an attempt, run its verify steps, and settle every
    /// gate-dependent learning record for it.
    ///
    /// Shared by the batch and streaming dispatch paths so both reach the same
    /// verdict. The pre-verify screen (`red_flags`) goes first: an attempt
    /// with runaway or malformed output, or one that tampered with its checks,
    /// is rejected before any step runs, as a `RokoError::Verify` of gate
    /// `pre_verify:<check>`, whether or not the task has verify steps. An
    /// implementer attempt that changed nothing is rejected there too, unless
    /// the task has authored verify steps: they probe the unchanged tree, and
    /// when they pass the task was already satisfied
    /// (`TaskGateVerdict::AlreadySatisfied`, gap-9eb1e1).
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn settle_task_verification(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        dispatch: &crate::dispatch_v2::AgentResultDispatch,
        effective_workdir: &Path,
        retry_key: &str,
        attempt_number: u32,
        attempt_key: &str,
        progress_tx: Option<&tokio::sync::mpsc::Sender<GraphTaskEvent>>,
    ) -> Result<TaskGateVerdict> {
        let screened = self
            .screen_attempt(
                spec,
                task,
                dispatch,
                effective_workdir,
                attempt_key,
                attempt_number,
                progress_tx,
            )
            .await?;
        let unchanged_tree = matches!(screened, red_flags::Screened::UnchangedTree(_));
        let verified = self
            .run_verify_steps(
                spec,
                task,
                dispatch,
                effective_workdir,
                retry_key,
                attempt_number,
                attempt_key,
                progress_tx,
                unchanged_tree,
            )
            .await;
        match screened {
            red_flags::Screened::Clear => verified,
            red_flags::Screened::UnchangedTree(unchanged) => {
                self.settle_unchanged_tree(
                    spec,
                    task,
                    attempt_number,
                    progress_tx,
                    unchanged,
                    verified,
                )
                .await
            }
        }
    }

    /// Run a task's verify steps, its authored `[[task.verify]]` steps and
    /// then the workspace's required `[[gates.rungs]]` unless its plan opts
    /// out ([`attempt_verify_steps`]), and settle every gate-dependent
    /// learning record for this attempt.
    ///
    /// Verify steps are deterministic: any
    /// failure returns `RokoError::Verify` (the Graph engine retries up to the task's
    /// `max_retries`, then fails the task) and is never force-accepted. Steps
    /// run fail-fast; the rest are reported as skipped. A step that fails
    /// while sibling tasks edit the same working tree waits for them to
    /// settle and re-runs once; only that result counts (`sibling_settle`).
    /// A step that ran out of time is recorded as a timeout. Once the plan
    /// run began to stop, a step that fails, or would start, ends the verify
    /// with a `RokoError::Cancelled` that no record or learner sees
    /// (bug-82cbef). The caller releases any worktree lease and settles
    /// episode feedback with the result.
    ///
    /// With `unchanged_tree` the attempt changed nothing, and its steps only
    /// probe whether the task's work was already there: their result stands
    /// as it is, with no auto-fix and no learning record (gap-9eb1e1).
    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    async fn run_verify_steps(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        dispatch: &crate::dispatch_v2::AgentResultDispatch,
        effective_workdir: &Path,
        retry_key: &str,
        attempt_number: u32,
        attempt_key: &str,
        progress_tx: Option<&tokio::sync::mpsc::Sender<GraphTaskEvent>>,
        unchanged_tree: bool,
    ) -> Result<TaskGateVerdict> {
        let effective_workdir = effective_workdir.to_path_buf();
        let retry_key = retry_key.to_string();
        let steps = self.verify_steps(spec, task);
        // `[gates] mode = "focused"` scopes authored Cargo tests (gap-1426e4).
        let steps = self
            .focus_verify_steps(&effective_workdir, task, steps)
            .await;
        // A step passed only because what failed in it failed on the plan
        // run's start commit too (gap-161be1).
        let mut preexisting_filtered = false;
        if !steps.is_empty() {
            let payload = GatePayload::in_dir(&effective_workdir)
                .with_label(format!("{}/{}", spec.plan_id, task.id))
                .with_env_passthrough(self.config.gates.env_passthrough.iter().cloned());
            let gate_signal = Signal::builder(Kind::Task)
                .body(
                    Body::from_json(&payload)
                        .unwrap_or_else(|_| Body::text("gate-payload-fallback")),
                )
                .build();
            let gate_ctx = Context::now();
            // While this attempt runs its verify steps it edits nothing, so
            // its siblings' verify steps do not wait for it (gap-1920ba).
            let verify_key = format!("{}/{}", spec.plan_id, task.id);
            let verifying = self.in_flight.begin_verify(&verify_key);
            let sibling_wait =
                std::time::Duration::from_secs(self.config.gates.sibling_settle_secs);

            let mut failures: Vec<String> = Vec::new();
            // Whether a failed step ran out of time. Its verdict says so even
            // when the step's authored `fail_msg` hides it in `failures`.
            let mut timed_out = false;
            // P2-LRN-6 Loop 1: Collect (phase, passed) for each verify step so
            // we can feed outcomes into GateThresholds::observe after all steps
            // complete (including any post-auto-fix re-run).
            let mut step_outcomes: Vec<(String, bool)> = Vec::new();
            // The verdicts of the steps that ran, with their phases: the run
            // the conductor's gate watchers read (gap-1a7f9c).
            let mut ran_steps: Vec<(String, roko_core::Verdict)> = Vec::new();
            // P1-08: the CodingOracle's test pass-rate forecast for this
            // attempt, taken before its steps feed the oracle below.
            let test_pass_forecast = self
                .feedback
                .coding_oracle
                .as_ref()
                .map(|oracle| oracle.predict_test_pass_rate());
            // P4-03: PromiseTracker for early termination of doomed attempts.
            let mut promise_tracker = crate::runner::promise_tracker::PromiseTracker::new();
            let mut promise_terminated = false;
            // Steps not run because an earlier step already failed.
            let mut skipped_steps: Vec<String> = Vec::new();
            // Siblings whose files hold every error of a failure that
            // persisted after they settled.
            let mut blocked_by_sibling: Option<String> = None;
            let total_steps = u32::try_from(steps.len()).unwrap_or(u32::MAX);

            for (i, (step_label, step)) in steps.iter().enumerate() {
                // How feedback and events quote the step: a pinned acceptance
                // step by its header line, not its generated script.
                let shown = crate::task_accept::prompt_command(&step.command);
                // Fail fast: once a step has failed (or P4-03 declared the
                // attempt doomed), report later steps as skipped instead of
                // paying for, say, a full compile after a cheap grep failed.
                if promise_terminated || !failures.is_empty() {
                    skipped_steps.push(format!("{step_label} (`{shown}`)"));
                    continue;
                }

                if let Some(progress_tx) = progress_tx {
                    let _ = progress_tx
                        .send(GraphTaskEvent::Progress {
                            message: format!("verify: {shown}"),
                            completed: Some(u32::try_from(i).unwrap_or(u32::MAX)),
                            total: Some(total_steps),
                        })
                        .await;
                }

                tracing::info!(
                    plan_id = %spec.plan_id,
                    task_id = %task.id,
                    step = i,
                    phase = %step.phase,
                    command = %step.command,
                    timeout_ms = step.timeout_ms,
                    "graph verify step starting"
                );

                // P2-TUI-4: Notify the TUI that a gate rung is starting so it
                // can show the active rung name and a spinner.
                if let Some(tui) = &self.tui_bridge {
                    tui.gate_rung_started(&spec.plan_id, &task.id, step_label);
                }

                let gate = ShellGate::new(
                    "bash",
                    vec![
                        "-o".into(),
                        "pipefail".into(),
                        "-c".into(),
                        self.verify_command(&step.command),
                    ],
                )
                .with_timeout_ms(step.timeout_ms)
                .with_name(step_label)
                .with_phase(&step.phase);

                // Wait until no sibling is mid-edit on what this step reads,
                // and keep siblings from starting to edit it while the step
                // runs (gap-1920ba). A run that begins to stop ends this wait
                // and the one for the compile lock (bug-3a3968).
                let step_scope = sibling_settle::StepScope::of(step, &effective_workdir);
                let reading = self
                    .unless_stopped(
                        spec,
                        task,
                        step_label,
                        self.in_flight.begin_step(&sibling_settle::StepRead {
                            plan_id: &spec.plan_id,
                            task_id: &task.id,
                            label: step_label,
                            workdir: &effective_workdir,
                            scope: &step_scope,
                            limit: sibling_wait,
                        }),
                    )
                    .await?;
                let compile_permit = self
                    .unless_stopped(
                        spec,
                        task,
                        step_label,
                        verify_compile_permit(
                            &effective_workdir,
                            self.config.gates.compile_concurrency,
                            step,
                            &spec.plan_id,
                            &task.id,
                        ),
                    )
                    .await?;
                // A run that began to stop starts no further step (bug-82cbef).
                if let Some(cancelled) = self.stopped_verify(spec, task, step_label) {
                    return Err(cancelled);
                }
                let mut verdict = gate.verify(&gate_signal, &gate_ctx).await;
                if !verdict.passed {
                    // A step that failed once its run began to stop was
                    // stopped with the run's commands: it says nothing about
                    // the work, so no sibling settle, record or learner sees
                    // it (bug-82cbef).
                    if let Some(cancelled) = self.stopped_verify(spec, task, step_label) {
                        return Err(cancelled);
                    }
                    // A sibling editing this working tree may have caused the
                    // failure: let it settle, then re-run the step. The
                    // compile lock is released meanwhile so the sibling's own
                    // cargo steps can finish.
                    drop(compile_permit);
                    let failed_step = sibling_settle::FailedStep {
                        plan_id: &spec.plan_id,
                        task_id: &task.id,
                        files: &task.files,
                        label: step_label,
                        workdir: &effective_workdir,
                        settle_limit: std::time::Duration::from_secs(
                            self.config.gates.sibling_settle_secs,
                        ),
                    };
                    (verdict, blocked_by_sibling) = self
                        .in_flight
                        .settle_failed_step(&failed_step, verdict, || async {
                            // Nothing re-runs once the run began to stop, and
                            // a stop ends the wait for the compile lock.
                            let permit = verify_compile_permit(
                                &effective_workdir,
                                self.config.gates.compile_concurrency,
                                step,
                                &spec.plan_id,
                                &task.id,
                            );
                            let Ok(_compile_permit) =
                                self.unless_stopped(spec, task, step_label, permit).await
                            else {
                                return roko_core::Verdict::fail(step_label, "stopping");
                            };
                            gate.verify(&gate_signal, &gate_ctx).await
                        })
                        .await;
                    if !verdict.passed
                        && let Some(cancelled) = self.stopped_verify(spec, task, step_label)
                    {
                        return Err(cancelled);
                    }
                }
                // A test step that still fails may fail only on tests that
                // failed on the plan run's start commit too (gap-161be1). The
                // step is done reading the tree: the runs that tell read it
                // again for themselves.
                if !verdict.passed && blocked_by_sibling.is_none() {
                    drop(reading);
                    if let Some(judgement) = self
                        .judge_against_baseline(
                            spec,
                            task,
                            attempt_key,
                            &effective_workdir,
                            step_label,
                            step,
                            &verdict,
                            unchanged_tree,
                        )
                        .await
                    {
                        preexisting_filtered |= baseline_verify::apply(
                            &mut verdict,
                            judgement,
                            &spec.plan_id,
                            &task.id,
                        );
                    }
                }
                // A passed `bench` step fails when its benchmarks got slower.
                verdict = bench_verify::judge_bench_step(
                    &self.workdir,
                    &spec.plan_id,
                    &task.id,
                    &step.phase,
                    verdict,
                );

                tracing::info!(
                    plan_id = %spec.plan_id,
                    task_id = %task.id,
                    step = i,
                    gate = %verdict.gate,
                    passed = verdict.passed,
                    duration_ms = verdict.duration_ms,
                    "graph verify step completed"
                );

                // P2-LRN-6 Loop 1: Record this step's (phase, passed) outcome
                // for gate threshold EMA update after the full verify sequence.
                step_outcomes.push((step.phase.clone(), verdict.passed));
                // P2-22: the gate pipeline's verdict metrics.
                gate_learning::record_gate_verdict_metrics(
                    self.metrics.as_deref(),
                    &step.phase,
                    &verdict,
                );

                // P2-TUI-4: Forward the gate verdict to the TUI so the
                // dashboard can display pass/fail status and captured output.
                // The command leads the output so failure summaries name it.
                if let Some(tui) = &self.tui_bridge {
                    tui.gate_result_with_output(
                        &spec.plan_id,
                        &task.id,
                        &verdict.gate,
                        verdict.passed,
                        Some(&published_gate_output(shown, &verdict)),
                    );
                }

                // ── P0-04: CodingOracle observations ────────────────────
                //
                // Feed each verdict into the CodingOracle so it can refine
                // its build-time and test-pass-rate predictions.
                if let Some(oracle) = &self.feedback.coding_oracle {
                    let now_ms = chrono::Utc::now().timestamp_millis();
                    let gate_lower = step.phase.to_ascii_lowercase();
                    if gate_lower.contains("compile")
                        || step.command.contains("cargo check")
                        || step.command.contains("cargo build")
                    {
                        oracle.observe_build(BuildRecord {
                            duration_secs: verdict.duration_ms as f64 / 1000.0,
                            success: verdict.passed,
                            warnings: 0,
                            ts_ms: now_ms,
                        });
                    }
                    if gate_lower.contains("test") || step.command.contains("cargo test") {
                        let (passed, failed, total) =
                            if verdict.passed { (1, 0, 1) } else { (0, 1, 1) };
                        oracle.observe_test(TestRecord {
                            passed,
                            failed,
                            total,
                            ts_ms: now_ms,
                        });
                    }
                }

                // ── P4-03: PromiseTracker per-step check ────────────────
                //
                // Build a TurnSnapshot from this verify step and check
                // whether the attempt should be terminated early.
                {
                    let core_verdict = if verdict.passed {
                        roko_core::Verdict::pass(step_label)
                    } else {
                        roko_core::Verdict::fail(step_label, &verdict.reason)
                    };
                    let snapshot = TurnSnapshot {
                        rung: i as u32,
                        verdicts: vec![core_verdict],
                        error_count: if verdict.passed { 0 } else { 1 },
                        diff_lines: 0,
                    };
                    let decision = promise_tracker.record_and_check(snapshot);
                    if let crate::runner::promise_tracker::PromiseDecision::Terminate {
                        promise,
                        consecutive_turns,
                    } = decision
                    {
                        tracing::warn!(
                            plan_id = %spec.plan_id,
                            task_id = %task.id,
                            step = i,
                            promise,
                            consecutive_turns,
                            "P4-03: PRM early termination — abandoning doomed verify sequence"
                        );
                        failures.push(format!(
                            "early termination: promise {promise:.3} below threshold \
                             for {consecutive_turns} consecutive verify steps"
                        ));
                        promise_terminated = true;
                        // Don't break here; fall through to record the current failure.
                    }
                }

                if !verdict.passed {
                    let fail_msg = step.fail_msg.as_deref().unwrap_or(&verdict.reason);
                    let detail_snippet = verdict
                        .detail
                        .as_deref()
                        .map(|d| {
                            // Include a bounded tail of the output for diagnostics.
                            let lines: Vec<&str> = d.lines().collect();
                            let start = lines.len().saturating_sub(30);
                            lines[start..].join("\n")
                        })
                        .unwrap_or_default();

                    timed_out |= roko_gate::verdict_timed_out(&verdict);
                    failures.push(format!(
                        "{step_label} (`{shown}`): {fail_msg}\n{detail_snippet}"
                    ));
                }
                ran_steps.push((step.phase.clone(), verdict));
            }

            // A probe of an unchanged tree settles here: nothing auto-fixes
            // the tree for it, and what it found teaches no learner.
            if unchanged_tree {
                self.publish_verify_run(spec, task, &effective_workdir, &ran_steps);
                if failures.is_empty() {
                    self.gate_retry_context.clear(&spec.plan_id, &task.id);
                    self.retrieval_ctx.lock().remove(&retry_key);
                    self.forget_diff_base(attempt_key);
                    return Ok(TaskGateVerdict::Passed);
                }
                let message =
                    verify_failure_summary(&spec.title, steps.len(), &failures, &skipped_steps);
                return Err(RokoError::Verify {
                    gate: "graph-verify".to_string(),
                    message,
                });
            }

            // ── P1-CLI-2: Compile auto-fix before agent retry ────────────
            //
            // Mirror the Runner-v2 path in gate_dispatch.rs: when verify steps
            // fail and `cargo_fix_enabled` is set (default: true), attempt
            // `cargo fix --allow-dirty` (or the build-system equivalent).
            // If the fix applies cleanly, re-run the verify steps once so the
            // caller sees the corrected result without waiting for a full agent
            // retry loop. Only applies when promise-tracker did NOT terminate
            // early (those failures are structural, not fixable by `cargo fix`).
            if !failures.is_empty() && !promise_terminated && self.auto_fix_enabled() {
                // The auto-fix and its re-run are verification too: a run
                // that began to stop starts neither (bug-82cbef).
                if let Some(cancelled) = self.stopped_verify(spec, task, "auto-fix") {
                    return Err(cancelled);
                }
                // Use the phase of the first failing step as the gate name so
                // `attempt_auto_fix` can pick the right fix command.
                let first_fail_phase = step_outcomes
                    .iter()
                    .find(|(phase, passed)| !passed && !phase.is_empty())
                    .map_or("compile", |(phase, _)| phase.as_str());
                let raw_failures = failures.join("\n---\n");
                // `cargo fix` writes files: meanwhile this attempt is editing.
                drop(verifying);
                match crate::runner::gate_dispatch::attempt_auto_fix(
                    &effective_workdir,
                    first_fail_phase,
                    &raw_failures,
                    crate::runner::gate_dispatch::AutoFixBounds::from_config(
                        &self.config,
                        &task.files,
                    ),
                )
                .await
                {
                    Ok(outcome) if outcome.fix_applied => {
                        tracing::info!(
                            plan_id = %spec.plan_id,
                            task_id = %task.id,
                            gate = first_fail_phase,
                            command = ?outcome.command,
                            "P1-CLI-2: auto-fix applied — re-running verify steps"
                        );
                        // Re-run verify steps with a fresh failures list.
                        // P2-LRN-6 Loop 1: Also collect retry outcomes to replace
                        // the original step_outcomes with post-fix results.
                        let mut retry_failures: Vec<String> = Vec::new();
                        let mut retry_step_outcomes: Vec<(String, bool)> = Vec::new();
                        let mut retry_ran_steps: Vec<(String, roko_core::Verdict)> = Vec::new();
                        let mut retry_skipped: Vec<String> = Vec::new();
                        let mut retry_timed_out = false;
                        let mut retry_preexisting_filtered = false;
                        let _verifying = self.in_flight.begin_verify(&verify_key);
                        for (i, (step_label, step)) in steps.iter().enumerate() {
                            let shown = crate::task_accept::prompt_command(&step.command);
                            if !retry_failures.is_empty() {
                                retry_skipped.push(format!("{step_label} (`{shown}`)"));
                                continue;
                            }
                            // P2-TUI-4: Notify the TUI of the post-fix re-run.
                            if let Some(tui) = &self.tui_bridge {
                                tui.gate_rung_started(&spec.plan_id, &task.id, step_label);
                            }
                            let retry_gate = ShellGate::new(
                                "bash",
                                vec![
                                    "-o".into(),
                                    "pipefail".into(),
                                    "-c".into(),
                                    step.command.clone(),
                                ],
                            )
                            .with_timeout_ms(step.timeout_ms)
                            .with_name(step_label)
                            .with_phase(&step.phase);
                            let step_scope =
                                sibling_settle::StepScope::of(step, &effective_workdir);
                            let reading = self
                                .unless_stopped(
                                    spec,
                                    task,
                                    step_label,
                                    self.in_flight.begin_step(&sibling_settle::StepRead {
                                        plan_id: &spec.plan_id,
                                        task_id: &task.id,
                                        label: step_label,
                                        workdir: &effective_workdir,
                                        scope: &step_scope,
                                        limit: sibling_wait,
                                    }),
                                )
                                .await?;
                            if let Some(cancelled) = self.stopped_verify(spec, task, step_label) {
                                return Err(cancelled);
                            }
                            // The re-run builds like the first run, so it
                            // queues on the same compile lock, and a stop
                            // ends that wait or keeps it from starting once
                            // it holds the lock (bug-c33c6e, bug-3a3968).
                            let Some(mut retry_verdict) = verify_step_locked(
                                &retry_gate,
                                &gate_signal,
                                &gate_ctx,
                                &effective_workdir,
                                self.config.gates.compile_concurrency,
                                step,
                                &spec.plan_id,
                                &task.id,
                                &self.stopping,
                            )
                            .await
                            else {
                                return Err(verify_cancelled(spec, task, step_label));
                            };
                            if !retry_verdict.passed {
                                if let Some(cancelled) = self.stopped_verify(spec, task, step_label)
                                {
                                    return Err(cancelled);
                                }
                                drop(reading);
                                if let Some(judgement) = self
                                    .judge_against_baseline(
                                        spec,
                                        task,
                                        attempt_key,
                                        &effective_workdir,
                                        step_label,
                                        step,
                                        &retry_verdict,
                                        unchanged_tree,
                                    )
                                    .await
                                {
                                    retry_preexisting_filtered |= baseline_verify::apply(
                                        &mut retry_verdict,
                                        judgement,
                                        &spec.plan_id,
                                        &task.id,
                                    );
                                }
                            }
                            retry_verdict = bench_verify::judge_bench_step(
                                &self.workdir,
                                &spec.plan_id,
                                &task.id,
                                &step.phase,
                                retry_verdict,
                            );
                            tracing::info!(
                                plan_id = %spec.plan_id,
                                task_id = %task.id,
                                step = i,
                                gate = %retry_verdict.gate,
                                passed = retry_verdict.passed,
                                duration_ms = retry_verdict.duration_ms,
                                "P1-CLI-2: post-fix verify step completed"
                            );
                            // P2-LRN-6 Loop 1: Record retry step outcome.
                            retry_step_outcomes.push((step.phase.clone(), retry_verdict.passed));
                            gate_learning::record_gate_verdict_metrics(
                                self.metrics.as_deref(),
                                &step.phase,
                                &retry_verdict,
                            );

                            // P2-TUI-4: Forward post-fix verdict to the TUI.
                            if let Some(tui) = &self.tui_bridge {
                                tui.gate_result_with_output(
                                    &spec.plan_id,
                                    &task.id,
                                    &retry_verdict.gate,
                                    retry_verdict.passed,
                                    Some(&published_gate_output(shown, &retry_verdict)),
                                );
                            }
                            if !retry_verdict.passed {
                                let fail_msg =
                                    step.fail_msg.as_deref().unwrap_or(&retry_verdict.reason);
                                let detail_snippet = retry_verdict
                                    .detail
                                    .as_deref()
                                    .map(|d| {
                                        let lines: Vec<&str> = d.lines().collect();
                                        let start = lines.len().saturating_sub(30);
                                        lines[start..].join("\n")
                                    })
                                    .unwrap_or_default();
                                retry_timed_out |= roko_gate::verdict_timed_out(&retry_verdict);
                                retry_failures.push(format!(
                                    "{step_label} (`{shown}`): {fail_msg}\n{detail_snippet}"
                                ));
                            }
                            retry_ran_steps.push((step.phase.clone(), retry_verdict));
                        }
                        // Replace the original failure list and step outcomes with
                        // the post-fix results. The retry outcomes are the ground
                        // truth for gate threshold EMA updates (P2-LRN-6 Loop 1).
                        failures = retry_failures;
                        timed_out = retry_timed_out;
                        step_outcomes = retry_step_outcomes;
                        ran_steps = retry_ran_steps;
                        preexisting_filtered = retry_preexisting_filtered;
                        skipped_steps = retry_skipped;
                        blocked_by_sibling = None;
                    }
                    Ok(outcome) => {
                        tracing::debug!(
                            plan_id = %spec.plan_id,
                            task_id = %task.id,
                            was_candidate = outcome.was_candidate,
                            fix_applied = outcome.fix_applied,
                            "P1-CLI-2: auto-fix not applied — proceeding to agent retry"
                        );
                    }
                    Err(err) => {
                        tracing::warn!(
                            plan_id = %spec.plan_id,
                            task_id = %task.id,
                            error = %err,
                            "P1-CLI-2: auto-fix error (non-fatal) — proceeding to agent retry"
                        );
                    }
                }
            }

            // gap-85f102: the opt-in LLM judge, once every step passed. A
            // blocking judge's failure fails the attempt like a failed step.
            if failures.is_empty() {
                let judged = self.judge_attempt(spec, task, attempt_key, &effective_workdir);
                failures.extend(judged.await);
            }

            self.publish_verify_run(spec, task, &effective_workdir, &ran_steps);

            if let Some(progress_tx) = progress_tx {
                let message = if failures.is_empty() {
                    "verify: all steps passed".to_string()
                } else {
                    format!(
                        "verify: {} failed, {} skipped of {total_steps}",
                        failures.len(),
                        skipped_steps.len()
                    )
                };
                let _ = progress_tx
                    .send(GraphTaskEvent::Progress {
                        message,
                        completed: Some(total_steps),
                        total: Some(total_steps),
                    })
                    .await;
            }

            // ── P2-LRN-6 Loop 1: Gate learning ──────────────────────────
            //
            // Feed each completed verify step's pass/fail outcome into the
            // persisted gate learning: the per-rung EMAs and the oracle
            // residual in `GateThresholds`, the task's profile priors and the
            // skip advisory (find-4b4344). Errors are logged and non-fatal;
            // the next task makes its own update.
            self.settle_gate_learning(spec, task, &step_outcomes, test_pass_forecast);
            // Steps an earlier attempt passed that fail now (gap-6dba88).
            let regressed = self.settle_step_regressions(spec, task, &steps, &step_outcomes);

            // ── Post-verify: GateGamingDetector + HoldoutExperiment ─────
            //
            // These run after all verify steps complete (or early-terminate)
            // regardless of pass/fail, matching the Runner-v2 gate completion
            // callback pattern.
            let all_passed = failures.is_empty();
            let model_slug = &dispatch.target.model_slug;

            // ── quality_judge + P1-01 GateGamingDetector (best-effort) ────
            //
            // The judge score only feeds the gaming detector: it never gates
            // the retry decision or the retry prompt, so it runs in the
            // background instead of blocking the retry. The LLM judge is only
            // consulted when verify failed and a detector is configured; all
            // passed maps to the deterministic high-quality score.
            if let Some(detector) = self.feedback.gate_gaming_detector.clone() {
                // P3-17: Modulate the judge score with daimon affect valence.
                let affect_bonus = self
                    .feedback
                    .daimon_state
                    .as_ref()
                    .and_then(|d| d.lock().ok())
                    .map(|state| state.state.alma.effective_affect().pleasure)
                    .unwrap_or(0.0);
                let judge = if all_passed { None } else { self.cheap_agent() };
                let judge_timeout = self.config.timeouts.llm_call();
                let agent_text = dispatch
                    .result
                    .output
                    .body
                    .as_text()
                    .unwrap_or("")
                    .to_string();
                let title = spec.title.clone();
                let plan_id = spec.plan_id.clone();
                let task_id = task.id.clone();
                let model_slug = model_slug.clone();
                tokio::spawn(async move {
                    let judge_quality_score: f64 = if all_passed {
                        0.9
                    } else if let Some(cheap_agent) = judge {
                        let rubric = "Did the agent make meaningful progress toward the task even though verify steps failed?";
                        match tokio::time::timeout(
                            judge_timeout,
                            roko_learn::quality_judge::judge_quality(
                                &cheap_agent,
                                &title,
                                &agent_text,
                                rubric,
                            ),
                        )
                        .await
                        {
                            Ok(score) => {
                                tracing::debug!(
                                    plan_id = %plan_id,
                                    task_id = %task_id,
                                    model = %model_slug,
                                    quality_score = score,
                                    "quality_judge: gate output scored"
                                );
                                score
                            }
                            Err(_) => {
                                tracing::warn!(
                                    plan_id = %plan_id,
                                    task_id = %task_id,
                                    "quality_judge timed out; using heuristic score"
                                );
                                0.2
                            }
                        }
                    } else {
                        // No model configured — fall through to heuristic score.
                        0.2
                    };
                    let quality_score = (judge_quality_score + affect_bonus * 0.1).clamp(0.0, 1.0);
                    let mut det = detector.lock().await;
                    if let Err(err) = det
                        .observe_and_detect(&model_slug, all_passed, quality_score)
                        .await
                    {
                        tracing::warn!(
                            error = %err,
                            model = %model_slug,
                            "P1-01: gate gaming detection I/O error (non-fatal)"
                        );
                    }
                });
            }

            // P1-04: HoldoutExperiment outcome recording and learning gate.
            if let Some(holdout) = &self.feedback.holdout_experiment {
                let holdout_task_key = format!("{}:{}", spec.plan_id, task.id);
                if let Ok(mut exp) = holdout.try_lock() {
                    exp.record_outcome(&holdout_task_key, all_passed, 0.0);
                    if let Some(alert) = exp.check_overfitting() {
                        tracing::warn!(
                            train_pass_rate = alert.train_pass_rate,
                            holdout_pass_rate = alert.holdout_pass_rate,
                            divergence_pp = alert.divergence_pp,
                            "P1-04: holdout overfitting detected"
                        );
                    }
                    // Gate learning updates: only Train partition tasks update
                    // the routing model; holdout tasks are observed but never
                    // feed back into learned state. This affects the playbook,
                    // efficiency, and experiment settlement paths above.
                    let should_update = exp.should_update_learning(&holdout_task_key);
                    tracing::debug!(
                        plan_id = %spec.plan_id,
                        task_id = %task.id,
                        should_update_learning = should_update,
                        "P1-04: holdout partition check"
                    );
                }
            }

            if !failures.is_empty() {
                // Verify steps are deterministic, so a failure is never
                // force-accepted: the Graph engine retries up to the task's
                // `max_retries` and then fails the task. (`gates.max_review_cycles`
                // may only bound non-deterministic review/judge verdicts, and
                // the Graph dispatcher gates on none.)
                let mut summary =
                    verify_failure_summary(&spec.title, steps.len(), &failures, &skipped_steps);
                // Lead with the blamed sibling so one-line failure reasons,
                // such as the episode's, keep it.
                if let Some(sibling) = &blocked_by_sibling {
                    summary = format!("blocked_by_sibling = {sibling}: {summary}");
                }
                tracing::warn!(
                    plan_id = %spec.plan_id,
                    task_id = %task.id,
                    failed_count = failures.len(),
                    skipped_count = skipped_steps.len(),
                    total_count = steps.len(),
                    attempt = attempt_number,
                    "graph verify steps failed"
                );
                // ── W12: Gate failure replan signal ───────────────────────
                if self.feedback.replan_on_gate_failure {
                    tracing::info!(
                        plan_id = %spec.plan_id,
                        task_id = %task.id,
                        failed_count = failures.len(),
                        "gate failure replan enabled; Graph engine will retry via max_retries"
                    );
                    // Update efficiency gate_passed if we wrote one.
                    if let Some(eff_path) = &self.feedback.efficiency_path {
                        // P3-02: Propagate actual turn count from the
                        // dispatch that preceded this gate failure; 0 marked
                        // unknown when it reported none (bug-ad5487).
                        let gate_turns = super::attempt::reported_turns(dispatch);
                        let gate_turn_number = gate_turns.unwrap_or(0);
                        let gate_event = roko_learn::efficiency::AgentEfficiencyEvent {
                            agent_id: format!("{}/{}", spec.plan_id, task.id),
                            role: task.role.as_deref().unwrap_or("implementer").to_string(),
                            backend: dispatch.target.provider_id.clone(),
                            model: dispatch.target.model_slug.clone(),
                            plan_id: spec.plan_id.clone(),
                            task_id: task.id.clone(),
                            attempt_id: format!("{attempt_key}/gate-fail"),
                            input_tokens: 0,
                            output_tokens: 0,
                            reasoning_tokens: 0,
                            cache_read_tokens: 0,
                            cache_write_tokens: 0,
                            cost_usd: 0.0,
                            cost_usd_without_cache: 0.0,
                            prompt_sections: vec![],
                            total_prompt_tokens: 0,
                            system_prompt_tokens: 0,
                            tools_available: 0,
                            tools_used: 0,
                            tool_calls: vec![],
                            wall_time_ms: 0,
                            duration_ms: 0,
                            time_to_first_token_ms: 0,
                            was_warm_start: false,
                            iteration: gate_turn_number,
                            turn_number: gate_turn_number,
                            is_final_turn: false,
                            gate_passed: Some(false),
                            outcome: "gate_failure".to_string(),
                            gate_errors: failures.clone(),
                            model_used: dispatch.target.model_slug.clone(),
                            frequency: roko_core::OperatingFrequency::Gamma,
                            strategy_attempted: "replan".to_string(),
                            timestamp: chrono::Utc::now().to_rfc3339(),
                        };
                        let row = AttemptKeyed {
                            attempt_key: attempt_key.to_string(),
                            row: roko_learn::efficiency::TurnsRow {
                                row: &gate_event,
                                turns_unknown: gate_turns.is_none(),
                            },
                        };
                        if let Ok(line) = serde_json::to_string(&row) {
                            let path = eff_path.clone();
                            crate::background_writes::spawn(&eff_path, async move {
                                if let Err(error) = append_jsonl_line_async(path, line).await {
                                    tracing::warn!(
                                        %error,
                                        "graph gate-failure efficiency event write failed"
                                    );
                                }
                            });
                        }
                    }
                }
                // ── error_enrichment: enrich gate failure before retry ───
                //
                // Ask a cheap judge model for a two-sentence diagnosis of the
                // raw failure so the retry prompt carries a focused summary
                // rather than raw compiler noise. Falls back deterministically
                // when no model is configured or the call fails.
                let raw_for_feedback = failures.join("\n---\n");
                // The diagnosis feeds the retry prompt, so it stays inline but
                // is bounded by `timeouts.llm_call_secs`; on timeout the retry
                // proceeds with the raw gate output alone.
                let enriched_diagnosis = if let Some(cheap_agent) = self.cheap_agent() {
                    tokio::time::timeout(
                        self.config.timeouts.llm_call(),
                        roko_learn::error_enrichment::enrich_error_digest(
                            &raw_for_feedback,
                            &cheap_agent,
                            &spec.title,
                        ),
                    )
                    .await
                    .unwrap_or_else(|_| {
                        tracing::warn!(
                            plan_id = %spec.plan_id,
                            task_id = %task.id,
                            "error enrichment timed out; retrying with raw gate output"
                        );
                        String::new()
                    })
                } else {
                    String::new()
                };

                // ── Store gate feedback for retry injection ─────────────
                //
                // Parse the raw failure text into structured GateFeedback,
                // with the diagnosis rendered ahead of the errors, and leave
                // it for the task's next attempt prompt. Plans with a Graph
                // checkpoint keep it on disk, so the attempt a resumed run
                // starts gets it even after this run's retries ran out.
                if let Some(feedback) = GateFeedback::from_raw(&raw_for_feedback) {
                    let diagnosis = step_ratchet::regression_note(&regressed, &enriched_diagnosis);
                    let feedback = feedback.with_diagnosis(&diagnosis);
                    let next_attempt = attempt_number.saturating_add(1);
                    let retries_left = spec
                        .max_retries
                        .saturating_sub(self.attempt_in_run(&retry_key));
                    tracing::info!(
                        plan_id = %spec.plan_id,
                        task_id = %task.id,
                        compile_errors = feedback.compile_errors.len(),
                        test_failures = feedback.test_failures.len(),
                        clippy_warnings = feedback.clippy_warnings.len(),
                        has_enriched_diagnosis = feedback.diagnosis.is_some(),
                        next_attempt,
                        retries_left,
                        "storing gate feedback for retry injection"
                    );
                    let kept_in = self.gate_retry_context.record(
                        &spec.plan_id,
                        &task.id,
                        feedback,
                        next_attempt,
                    );
                    if retries_left == 0 {
                        match kept_in {
                            Some(path) => tracing::warn!(
                                plan_id = %spec.plan_id,
                                task_id = %task.id,
                                attempt = attempt_number,
                                max_retries = spec.max_retries,
                                feedback = %path.display(),
                                "verify failed on the last attempt the retry budget allows; \
                                 `roko plan run --resume-plan` retries the task with this feedback"
                            ),
                            None => tracing::warn!(
                                plan_id = %spec.plan_id,
                                task_id = %task.id,
                                attempt = attempt_number,
                                max_retries = spec.max_retries,
                                "verify failed on the last attempt the retry budget allows; \
                                 its feedback is not kept for a resumed run"
                            ),
                        }
                    }
                }
                // ── W13: Persist structured gate failure record ──────────
                //
                // Classify the raw failure text and append a GateFailureRecord
                // to `.roko/learn/gate-failures.jsonl` for fast triage and
                // adaptive threshold learning (#218).
                if let Some(gf_path) = &self.feedback.gate_failures_path {
                    let raw_for_classification = failures.join("\n---\n");
                    // The failed step's phase says whether it ran tests
                    // (bug-386c9b).
                    let failed_phase = step_outcomes
                        .iter()
                        .find(|(_, passed)| !passed)
                        .map(|(phase, _)| phase.as_str());
                    let classification = roko_gate::classify_step_failure(
                        "graph-verify",
                        failed_phase,
                        &raw_for_classification,
                    );
                    // The failed step's verdict, not the failure text, says
                    // whether it ran out of time.
                    let classification = if timed_out {
                        classification.timed_out()
                    } else {
                        classification
                    };
                    // The summary leads with the failed step's label, which
                    // `roko diagnose` reads, then its failure message and
                    // output. A long command would crowd those out, and
                    // diagnose reads it from tasks.toml (bug-6f7f72).
                    let classification = match failed_step_summary(&steps, &ran_steps) {
                        Some(summary) => classification.with_summary(summary),
                        None => classification,
                    };
                    let record = roko_gate::GateFailureRecord::from_classification(
                        &spec.plan_id,
                        &task.id,
                        "graph-verify",
                        failed_step_rung(&step_outcomes),
                        &classification,
                    );
                    if let Ok(line) = serde_json::to_string(&record) {
                        let path = gf_path.clone();
                        crate::background_writes::spawn(&gf_path, async move {
                            if let Err(error) = append_jsonl_line_async(path, line).await {
                                tracing::warn!(
                                    %error,
                                    "gate failure record write failed (non-fatal)"
                                );
                            }
                        });
                    }
                }
                // ── P2-PLN-2: Post-gate LLM reflection ───────────────────
                //
                // When `replan_on_gate_failure` is enabled and a cheap
                // agent is available, ask the LLM for a one-sentence
                // reflection explaining the root cause. The lesson is
                // stored in the PostGateReflectionStore (at
                // `.roko/learn/post-gate-reflections.json`) so subsequent
                // retry prompts and playbook extraction see real LLM
                // analysis instead of the deterministic pattern template.
                if self.feedback.replan_on_gate_failure {
                    if let Some((reflection_path, cheap_agent)) = self
                        .feedback
                        .post_gate_reflection_path
                        .as_ref()
                        .cloned()
                        .zip(self.cheap_agent())
                    {
                        let raw_for_reflection = failures.join("\n---\n");
                        let task_desc = spec.title.clone();
                        let plan_id = spec.plan_id.clone();
                        let task_id = task.id.clone();
                        tokio::spawn(async move {
                            let lesson =
                                roko_learn::post_gate_reflection::generate_post_gate_reflection(
                                    &cheap_agent,
                                    &task_desc,
                                    "graph-verify",
                                    &raw_for_reflection,
                                )
                                .await;
                            tracing::info!(
                                plan_id = %plan_id,
                                task_id = %task_id,
                                lesson_chars = lesson.len(),
                                "post-gate LLM reflection generated"
                            );
                            let input = roko_learn::post_gate_reflection::ReflectionInput {
                                plan_id: Some(plan_id),
                                task_id: Some(task_id),
                                episode_id: None,
                                trigger_gate: "graph-verify".to_string(),
                                outcome:
                                    roko_learn::post_gate_reflection::ReflectionGateOutcome::Failed,
                                failure_pattern_ids: vec![],
                                pass_evidence: vec![],
                                proposed_lesson: lesson,
                            };
                            let mut store =
                                roko_learn::post_gate_reflection::PostGateReflectionStore::load(
                                    &reflection_path,
                                );
                            store.observe(
                                input,
                                roko_learn::post_gate_reflection::ReflectionPromotionConfig::default(),
                            );
                            if let Err(error) = store.save(&reflection_path) {
                                tracing::warn!(
                                    %error,
                                    "post-gate reflection store write failed (non-fatal)"
                                );
                            }
                        });
                    }
                }
                // ── RAG-10/11: Retrieval outcome settlement (gate fail) ──
                {
                    let ctx_snapshot = self.retrieval_ctx.lock().get(&retry_key).cloned();
                    if let Some((strategy, query, results_count, latency_ms)) = ctx_snapshot {
                        // RAG-11: update experiment store with gate-fail outcome.
                        if let Some(exp_path) = self.feedback.experiment_store_path.clone() {
                            // Locked: prompt treatments share the file. The
                            // store is read and written back whole, so off the
                            // reactor (gap-5e818f).
                            let outcome_strategy = strategy.clone();
                            let _ = tokio::task::spawn_blocking(move || {
                                roko_learn::prompt_experiment::ExperimentStore::transaction(
                                    &exp_path,
                                    |store| {
                                        store.record_retrieval_outcome(&outcome_strategy, false);
                                        Ok(())
                                    },
                                )
                            })
                            .await;
                        }
                        // RAG-10: write settled record.
                        if let Some(path) = self.feedback.retrieval_outcomes_path.clone() {
                            let record =
                                roko_learn::retrieval_outcome::RetrievalOutcomeRecord::settled(
                                    &spec.plan_id,
                                    &task.id,
                                    &query,
                                    &strategy,
                                    results_count,
                                    false,
                                )
                                .with_latency_ms(latency_ms);
                            crate::background_writes::spawn(&path.clone(), async move {
                                if let Err(error) =
                                    roko_learn::retrieval_outcome::RetrievalOutcomeStore::at(&path)
                                        .without_fsync()
                                        .append(&record)
                                        .await
                                {
                                    tracing::warn!(
                                        %error,
                                        "RAG-10: gate-fail retrieval outcome write failed (best-effort)"
                                    );
                                }
                            });
                        }
                    }
                }
                return Err(RokoError::Verify {
                    gate: "graph-verify".to_string(),
                    message: summary,
                });
            }

            tracing::info!(
                plan_id = %spec.plan_id,
                task_id = %task.id,
                step_count = steps.len(),
                "all graph verify steps passed"
            );
            // ── P0-GA-1: Emit gate-pass efficiency event ──────────────────
            //
            // The initial efficiency event (W05 in emit_feedback) is written
            // before gate execution with gate_passed: None, so the metric was
            // always 0%. Write a follow-up record now that we know all verify
            // steps passed so readers that filter by gate_passed == Some(true)
            // see the correct pass count.
            if let Some(eff_path) = &self.feedback.efficiency_path {
                // The attempt's reported turns; 0 marked unknown when it
                // reported none (bug-ad5487).
                let gate_turns = super::attempt::reported_turns(dispatch);
                let gate_turn_number = gate_turns.unwrap_or(0);
                let gate_pass_event = roko_learn::efficiency::AgentEfficiencyEvent {
                    agent_id: format!("{}/{}", spec.plan_id, task.id),
                    role: task.role.as_deref().unwrap_or("implementer").to_string(),
                    backend: dispatch.target.provider_id.clone(),
                    model: dispatch.target.model_slug.clone(),
                    plan_id: spec.plan_id.clone(),
                    task_id: task.id.clone(),
                    // Suffixed so it stays distinct from, yet joins, the
                    // attempt's dispatch event.
                    attempt_id: format!("{attempt_key}/gate-pass"),
                    input_tokens: 0,
                    output_tokens: 0,
                    reasoning_tokens: 0,
                    cache_read_tokens: 0,
                    cache_write_tokens: 0,
                    cost_usd: 0.0,
                    cost_usd_without_cache: 0.0,
                    prompt_sections: vec![],
                    total_prompt_tokens: 0,
                    system_prompt_tokens: 0,
                    tools_available: 0,
                    tools_used: 0,
                    tool_calls: vec![],
                    wall_time_ms: 0,
                    duration_ms: 0,
                    time_to_first_token_ms: 0,
                    was_warm_start: false,
                    iteration: gate_turn_number,
                    turn_number: gate_turn_number,
                    is_final_turn: true,
                    gate_passed: Some(true),
                    outcome: "gate_pass".to_string(),
                    gate_errors: vec![],
                    model_used: dispatch.target.model_slug.clone(),
                    frequency: roko_core::OperatingFrequency::Gamma,
                    strategy_attempted: String::new(),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                };
                let row = AttemptKeyed {
                    attempt_key: attempt_key.to_string(),
                    row: roko_learn::efficiency::TurnsRow {
                        row: &gate_pass_event,
                        turns_unknown: gate_turns.is_none(),
                    },
                };
                if let Ok(line) = serde_json::to_string(&row) {
                    let path = eff_path.clone();
                    let plan_id = spec.plan_id.clone();
                    let task_id = task.id.clone();
                    crate::background_writes::spawn(&eff_path, async move {
                        if let Err(error) = append_jsonl_line_async(path, line).await {
                            tracing::warn!(
                                plan_id = %plan_id,
                                task_id = %task_id,
                                %error,
                                "graph gate-pass efficiency event write failed (best-effort)"
                            );
                        }
                    });
                }
            }
            // ── RAG-10/11: Retrieval outcome settlement (gate pass) ───────
            {
                let ctx_snapshot = self.retrieval_ctx.lock().get(&retry_key).cloned();
                if let Some((strategy, query, results_count, latency_ms)) = ctx_snapshot {
                    // RAG-11: update experiment store with gate-pass outcome.
                    if let Some(exp_path) = self.feedback.experiment_store_path.clone() {
                        // Locked: prompt treatments share the file. The store
                        // is read and written back whole, so off the reactor
                        // (gap-5e818f).
                        let outcome_strategy = strategy.clone();
                        let _ = tokio::task::spawn_blocking(move || {
                            roko_learn::prompt_experiment::ExperimentStore::transaction(
                                &exp_path,
                                |store| {
                                    store.record_retrieval_outcome(&outcome_strategy, true);
                                    Ok(())
                                },
                            )
                        })
                        .await;
                    }
                    // RAG-10: write settled record.
                    if let Some(path) = self.feedback.retrieval_outcomes_path.clone() {
                        let record =
                            roko_learn::retrieval_outcome::RetrievalOutcomeRecord::settled(
                                &spec.plan_id,
                                &task.id,
                                &query,
                                &strategy,
                                results_count,
                                true,
                            )
                            .with_latency_ms(latency_ms);
                        crate::background_writes::spawn(&path.clone(), async move {
                            if let Err(error) =
                                roko_learn::retrieval_outcome::RetrievalOutcomeStore::at(&path)
                                    .without_fsync()
                                    .append(&record)
                                    .await
                            {
                                tracing::warn!(
                                    %error,
                                    "RAG-10: gate-pass retrieval outcome write failed (best-effort)"
                                );
                            }
                        });
                    }
                }
            }
            // Clear any stale gate retry context on success.
            self.gate_retry_context.clear(&spec.plan_id, &task.id);
            self.retrieval_ctx.lock().remove(&retry_key);
        }

        self.forget_diff_base(attempt_key);
        Ok(if steps.is_empty() {
            TaskGateVerdict::Unverified
        } else if preexisting_filtered {
            TaskGateVerdict::PassedWithPreexistingFailures
        } else {
            TaskGateVerdict::Passed
        })
    }

    /// The cancellation `task`'s verify ends in at `at`, a step or the
    /// auto-fix, once its plan run began to stop ([`Self::begin_stop`]). The
    /// run is stopping its commands, so a step that fails then says nothing
    /// about the attempt's work, and a step that would start then is not
    /// worth starting (bug-82cbef).
    fn stopped_verify(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        at: &str,
    ) -> Option<RokoError> {
        self.is_stopping().then(|| verify_cancelled(spec, task, at))
    }

    /// `wait`, unless `task`'s plan run begins to stop first: then the
    /// cancellation its verify ends in at `at` (bug-3a3968). A step waits for
    /// siblings editing what it reads and for the compile lock, and behind
    /// another process's long build either wait can outlast the run's drain.
    async fn unless_stopped<T>(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        at: &str,
        wait: impl std::future::Future<Output = T>,
    ) -> Result<T> {
        tokio::select! {
            biased;
            () = self.stopping.cancelled() => Err(verify_cancelled(spec, task, at)),
            value = wait => Ok(value),
        }
    }
}

/// The cancellation `task`'s verify ends in at `at`, a step or the auto-fix,
/// when its plan run stops it (bug-82cbef).
fn verify_cancelled(spec: &TaskExecutionSpec, task: &TaskDef, at: &str) -> RokoError {
    RokoError::cancelled(format!(
        "the plan run stopped during the verify of {}/{} at {at}",
        spec.plan_id, task.id
    ))
}

/// Queue a cargo verify step on the per-repository compile lock before its
/// timeout starts, so a build by a plan running beside this one, in this
/// process or another, cannot time the step out. Other steps take no permit.
pub(super) async fn verify_compile_permit(
    workdir: &Path,
    compile_concurrency: usize,
    step: &crate::task_parser::VerifyStep,
    plan_id: &str,
    task_id: &str,
) -> Option<crate::runner::gate_dispatch::CompileOwnership> {
    let runs_cargo = step
        .command
        .split(|c: char| c.is_whitespace() || "&|;({".contains(c))
        .any(|word| word == "cargo");
    if !runs_cargo {
        return None;
    }
    crate::runner::gate_dispatch::acquire_compile_ownership(
        workdir,
        compile_concurrency,
        std::time::Duration::from_millis(step.timeout_ms),
        plan_id,
        task_id,
        &step.command,
    )
    .await
    .inspect_err(|error| {
        tracing::warn!(%error, "running the cargo verify step without the compile lock");
    })
    .ok()
}

/// Run a verify step's gate while holding the compile lock its command needs
/// ([`verify_compile_permit`]). The post-auto-fix re-run goes through here
/// (bug-951930). A step whose plan run `stop`s while it waits for the lock
/// (bug-3a3968), or by the time it holds it (bug-c33c6e), does not start:
/// `None`.
pub(super) async fn verify_step_locked(
    gate: &ShellGate,
    signal: &Signal,
    ctx: &Context,
    workdir: &Path,
    compile_concurrency: usize,
    step: &crate::task_parser::VerifyStep,
    plan_id: &str,
    task_id: &str,
    stop: &tokio_util::sync::CancellationToken,
) -> Option<roko_core::Verdict> {
    let lock = verify_compile_permit(workdir, compile_concurrency, step, plan_id, task_id);
    let _compile_permit = tokio::select! {
        biased;
        () = stop.cancelled() => return None,
        permit = lock => permit,
    };
    if stop.is_cancelled() {
        return None;
    }
    Some(gate.verify(signal, ctx).await)
}

impl GraphTaskDispatcher {
    /// Whether `spec`'s plan runs the workspace's `[[gates.rungs]]`: its
    /// `[meta] workspace_rungs`, read once per plan from
    /// `<plan_dir>/tasks.toml`. An unreadable file counts as on.
    pub(super) fn plan_runs_workspace_rungs(&self, spec: &TaskExecutionSpec) -> bool {
        let mut plans = self.workspace_rung_plans.lock();
        if let Some(runs) = plans.get(&spec.plan_id) {
            return *runs;
        }
        let runs = self
            .read_plan_meta(spec)
            .is_none_or(|meta| meta.runs_workspace_rungs());
        if !runs {
            tracing::info!(
                plan_id = %spec.plan_id,
                "plan sets workspace_rungs = false: its tasks run only their own verify steps"
            );
        }
        plans.insert(spec.plan_id.clone(), runs);
        runs
    }

    /// The workspace rungs an attempt at `task` of `spec`'s plan faces: the
    /// `[[gates.rungs]]` [`task_runs_rung`] picks, none when the plan opts
    /// out.
    pub(super) fn task_rungs(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
    ) -> impl Iterator<Item = &roko_core::config::GateRungConfig> {
        let runs = self.plan_runs_workspace_rungs(spec);
        let rungs = self.config.gates.custom_rungs.iter();
        rungs.filter(move |rung| runs && task_runs_rung(task, rung))
    }

    /// The verify steps an attempt at `task` runs, labelled
    /// ([`attempt_verify_steps`]).
    pub(super) fn verify_steps(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
    ) -> Vec<(String, crate::task_parser::VerifyStep)> {
        attempt_verify_steps(task, self.task_rungs(spec, task))
    }

    /// `task` as its prompt shows it: with every verify step that will judge
    /// it, its own and then the workspace rungs, so the agent sees each check.
    pub(super) fn prompt_task(&self, spec: &TaskExecutionSpec, task: &TaskDef) -> TaskDef {
        let mut prompt_task = task.clone();
        prompt_task.verify = self
            .verify_steps(spec, task)
            .into_iter()
            .map(|(_, step)| step)
            .collect();
        prompt_task
    }
}

/// Whether an attempt at `task` runs the workspace rung `rung`: a required
/// rung with a command always, and an optional one when the task's
/// gate-profile hints ask for it (gap-69a56e). A `quality_profile =
/// "hardened"` task runs every declared rung, and a task that names
/// `test_invariants` also runs the rungs that run tests.
fn task_runs_rung(task: &TaskDef, rung: &roko_core::config::GateRungConfig) -> bool {
    if rung.command.trim().is_empty() {
        return false;
    }
    let invariants = task.hints.test_invariants.as_deref().unwrap_or_default();
    let test_rung = matches!(rung_for_gate_name(&rung.name), Some(roko_gate::Rung::Test));
    rung.required
        || task.hints.quality_profile == Some(roko_core::TaskQualityProfile::Hardened)
        || (test_rung && !invariants.is_empty())
}

/// The verify steps an attempt at `task` runs, each with its label: the
/// task's authored `[[task.verify]]` steps (`verify[i]` or `verify[i:phase]`),
/// then each of `rungs`, the workspace's `[[gates.rungs]]` that its plan runs
/// (`rung[name]`), whose command no authored step already runs. `roko run`
/// authors its task's steps from the same rungs, so they never run twice
/// there either.
fn attempt_verify_steps<'a>(
    task: &TaskDef,
    rungs: impl IntoIterator<Item = &'a roko_core::config::GateRungConfig>,
) -> Vec<(String, crate::task_parser::VerifyStep)> {
    let mut steps: Vec<_> = task
        .verify
        .iter()
        .enumerate()
        .map(|(index, step)| (verify_step_label(index, &step.phase), step.clone()))
        .collect();
    for rung in rungs {
        let command = rung.command.trim();
        if !task.verify.iter().any(|s| s.command.trim() == command) {
            steps.push((rung_step_label(&rung.name), rung.into()));
        }
    }
    steps
}

/// Stable label for the `index`-th verify step (`verify[i]` or `verify[i:phase]`).
pub(super) fn verify_step_label(index: usize, phase: &str) -> String {
    if phase.is_empty() {
        format!("verify[{index}]")
    } else {
        format!("verify[{index}:{phase}]")
    }
}

/// Stable label for the workspace gate rung `name` (`rung[name]`).
fn rung_step_label(name: &str) -> String {
    format!("rung[{name}]")
}

/// Maximum number of output lines kept from a gate's detail for dashboard
/// display.  Lines beyond this are replaced by a "… N earlier lines not shown"
/// leader.
const GATE_OUTPUT_TAIL_LINES: usize = 60;

/// Maximum byte size of the kept tail before a hard byte truncation kicks in.
/// When the 60-line tail still exceeds this, the last 16 KiB is kept (aligned
/// to a UTF-8 char boundary) and prefixed with "… earlier output not shown".
const GATE_OUTPUT_TAIL_BYTES: usize = 16 * 1024;

/// Gate output published to the dashboard: `$ {command}`, then (when
/// non-empty after trimming) the last 60 lines / 16 KiB of the gate's detail,
/// and finally — when the verdict failed — `✗ ` and how it ended.
pub(super) fn published_gate_output(command: &str, verdict: &roko_core::Verdict) -> String {
    let mut result = format!("$ {command}");

    let trimmed = verdict.detail.as_deref().unwrap_or("").trim_end();
    if !trimmed.is_empty() {
        result.push('\n');
        result.push_str(&gate_output_tail(trimmed));
    }

    if !verdict.passed {
        result.push_str("\n✗ ");
        result.push_str(&gate_how_ended(&verdict.reason));
    }

    result
}

/// Keep the last [`GATE_OUTPUT_TAIL_LINES`] lines of `detail`; prepend a
/// "… N earlier lines not shown" message when lines are dropped.  If the
/// resulting tail still exceeds [`GATE_OUTPUT_TAIL_BYTES`], truncate to the
/// last 16 KiB (aligned to a UTF-8 char boundary) and prepend
/// "… earlier output not shown".
fn gate_output_tail(detail: &str) -> String {
    let all_lines: Vec<&str> = detail.lines().collect();
    let total = all_lines.len();

    let (tail_lines, dropped) = if total > GATE_OUTPUT_TAIL_LINES {
        let dropped = total - GATE_OUTPUT_TAIL_LINES;
        (&all_lines[dropped..], dropped)
    } else {
        (&all_lines[..], 0)
    };

    let mut tail = tail_lines.join("\n");

    if tail.len() > GATE_OUTPUT_TAIL_BYTES {
        // Move the cut forward to a UTF-8 char boundary so we keep at most
        // GATE_OUTPUT_TAIL_BYTES bytes.
        let ideal = tail.len() - GATE_OUTPUT_TAIL_BYTES;
        let mut byte_pos = ideal;
        while !tail.is_char_boundary(byte_pos) {
            byte_pos += 1;
        }
        let kept = tail[byte_pos..].to_string();
        tail = format!("… earlier output not shown\n{kept}");
    } else if dropped > 0 {
        tail = format!("… {dropped} earlier lines not shown\n{tail}");
    }

    tail
}

/// Translate the verdict's `reason` field into a human-readable closing line.
///
/// - `"exit code: <n>"` → `"exit status <n>"`
/// - `"exit code: terminated by signal"` → `"terminated by a signal"`
/// - `""` (empty) → `"failed"`
/// - anything else → unchanged
fn gate_how_ended(reason: &str) -> String {
    if reason.is_empty() {
        "failed".to_string()
    } else if let Some(code) = reason.strip_prefix("exit code: ") {
        if code == "terminated by signal" {
            "terminated by a signal".to_string()
        } else {
            format!("exit status {code}")
        }
    } else {
        reason.to_string()
    }
}

/// Gate rung of the first failed step in `(phase, passed)` verify outcomes
/// ([`verify_step_rung`]); the custom shell gate's rung when none failed.
fn failed_step_rung(step_outcomes: &[(String, bool)]) -> u32 {
    let phase = step_outcomes
        .iter()
        .find(|(_, passed)| !passed)
        .map_or("", |(phase, _)| phase.as_str());
    verify_step_rung(phase)
}

/// Gate rung of a verify step of `phase`: its canonical rung (0 compile,
/// 1 clippy, 2 test), else the custom shell gate's rung, since every Graph
/// verify step is a shell command.
pub(super) fn verify_step_rung(phase: &str) -> u32 {
    rung_for_gate_name(phase).map_or_else(
        || {
            roko_gate::GateRegistry::new()
                .rung_for_name("custom")
                .map_or(0, u32::from)
        },
        |rung| rung.as_index(),
    )
}

/// Most bytes of a gate failure record's summary, as of an episode's failure
/// reason.
const GATE_FAILURE_SUMMARY_BYTES: usize = 2_048;

/// The summary of a failed verify run's gate failure record: the first
/// failed step's label, then its failure message (its authored `fail_msg`,
/// else how it ended) and its output, kept to
/// [`GATE_FAILURE_SUMMARY_BYTES`] by its head and tail. `None` when no step
/// failed.
fn failed_step_summary(
    steps: &[(String, crate::task_parser::VerifyStep)],
    ran_steps: &[(String, roko_core::Verdict)],
) -> Option<String> {
    let (_, verdict) = ran_steps.iter().find(|(_, verdict)| !verdict.passed)?;
    let fail_msg = steps
        .iter()
        .find(|(label, _)| *label == verdict.gate)
        .and_then(|(_, step)| step.fail_msg.as_deref())
        .unwrap_or(&verdict.reason);
    let output = verdict.detail.as_deref().unwrap_or_default().trim();
    let summary = format!("{}: {fail_msg}\n{output}", verdict.gate);
    Some(head_and_tail(
        summary.trim_end(),
        GATE_FAILURE_SUMMARY_BYTES,
    ))
}

/// Retry-facing summary of a failed verify run, including skipped steps.
pub(super) fn verify_failure_summary(
    title: &str,
    total: usize,
    failures: &[String],
    skipped: &[String],
) -> String {
    let summary = format!(
        "{n}/{total} verify step(s) failed for task `{title}`:\n\n{details}",
        n = failures.len(),
        details = failures.join("\n\n---\n\n"),
    );
    if skipped.is_empty() {
        summary
    } else {
        format!(
            "{summary}\n\nSkipped after the first failure: {}",
            skipped.join(", ")
        )
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;
    use crate::graph_task_dispatch::tests::{
        VERIFY_PROVIDER, batch_ctx, make_batch_dispatcher, make_spec, make_test_dispatcher,
        no_auto_fix, verify_step,
    };

    // ─── Verify verdict tests ───────────────────────────────────────────────

    /// Sorted `(attempt_id, outcome)` of every efficiency record, once
    /// `expected` records have landed from the background writers.
    async fn efficiency_records(path: &Path, expected: usize) -> Vec<(String, String)> {
        crate::background_writes::settled(path.parent().unwrap_or(path)).await;
        for _ in 0..600 {
            let mut records: Vec<(String, String)> = std::fs::read_to_string(path)
                .unwrap_or_default()
                .lines()
                .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
                .map(|record| {
                    let field = |name: &str| record[name].as_str().unwrap_or_default().to_string();
                    (field("attempt_id"), field("outcome"))
                })
                .collect();
            if records.len() >= expected {
                records.sort();
                return records;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        panic!("efficiency records were not written to {}", path.display());
    }

    #[tokio::test]
    async fn failing_authored_verify_is_never_force_accepted() {
        let temp = tempdir().expect("tempdir");
        let (dispatcher, mut task) = make_test_dispatcher(
            &temp,
            VERIFY_PROVIDER,
            |config| {
                // The retired review-cycle cap force-accepted at this count.
                config.gates.max_review_cycles = 1;
                no_auto_fix(config);
            },
            GraphFeedbackContext::default(),
        )
        .await;
        task.verify = vec![verify_step("structural", "exit 1")];
        let spec = make_spec(&task);

        for attempt in 0..3 {
            let error = dispatcher
                .dispatch(&spec, Vec::new(), &CellContext::new())
                .await
                .expect_err("a failing authored verify step must fail the attempt");
            assert!(
                matches!(error, RokoError::Verify { .. }),
                "attempt {attempt}: {error}"
            );
        }
    }

    #[tokio::test]
    async fn verify_fails_fast_and_reports_skipped_steps() {
        let temp = tempdir().expect("tempdir");
        let (dispatcher, mut task) = make_test_dispatcher(
            &temp,
            VERIFY_PROVIDER,
            no_auto_fix,
            GraphFeedbackContext::default(),
        )
        .await;
        let marker = temp.path().join("compile-ran");
        task.verify = vec![
            verify_step("structural", "grep -q missing-symbol /dev/null"),
            verify_step("compile", &format!("touch {}", marker.display())),
        ];

        let error = dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &CellContext::new())
            .await
            .expect_err("structural failure");
        let RokoError::Verify { message, .. } = error else {
            panic!("expected a verify failure, got {error}");
        };
        assert!(message.contains("1/2 verify step(s) failed"), "{message}");
        assert!(message.contains("grep -q missing-symbol"), "{message}");
        assert!(
            message.contains("Skipped after the first failure: verify[1:compile]"),
            "{message}"
        );
        assert!(!marker.exists(), "fail-fast must not run later steps");
    }

    /// Provider that answers diagnosis requests with a fixed diagnosis, logs
    /// every other prompt (argv, which carries the system prompt, then stdin)
    /// to `prompts/<n>.txt`, and writes `fixed` once a prompt carries
    /// previous-attempt feedback.
    const FEEDBACK_PROVIDER: &str = r#"#!/bin/sh
set -eu
input="$(cat)"
case "$input" in
  *"Diagnosis:"*) text="The fixture file named fixed was never created. Create it first." ;;
  *)
    mkdir -p prompts
    n=$(ls prompts | wc -l | tr -d ' ')
    printf '%s\n---\n%s\n' "$*" "$input" > "prompts/$n.txt"
    case "$*" in *"Your previous attempt FAILED verification"*) : > fixed ;; esac
    text="task-output" ;;
esac
printf '{"type":"content_block_delta","delta":{"text":"%s"}}\n' "$text"
printf '%s\n' '{"type":"result","session_id":"s","model":"claude-sonnet-4-6","total_cost_usd":0.01,"usage":{"input_tokens":5,"output_tokens":10}}'
"#;

    /// Lets the test model serve as the cheap diagnosis model too.
    fn diagnosing(config: &mut RokoConfig) {
        no_auto_fix(config);
        if let Some(model) = config.models.get_mut("stream-model") {
            model.supports_tools = true;
        }
    }

    #[tokio::test]
    async fn verify_feedback_outlives_the_process_and_reaches_the_resumed_prompt() {
        let temp = tempdir().expect("tempdir");
        let feedback_file = temp.path().join("state/retry-feedback.json");
        let (dispatcher, mut task) = make_test_dispatcher(
            &temp,
            FEEDBACK_PROVIDER,
            diagnosing,
            GraphFeedbackContext::default(),
        )
        .await;
        task.verify = vec![verify_step("structural", "test -f fixed")];
        let spec = make_spec(&task);
        dispatcher.attach_retry_feedback(&spec.plan_id, feedback_file.clone(), "run-1");

        let error = dispatcher
            .dispatch(&spec, Vec::new(), &CellContext::new())
            .await
            .expect_err("`fixed` does not exist yet");
        assert!(matches!(error, RokoError::Verify { .. }), "{error}");
        let persisted: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&feedback_file).expect("feedback persisted"))
                .expect("feedback json");
        let entry = &persisted["tasks"][task.id.as_str()];
        assert_eq!(entry["next_attempt"], 1, "{persisted}");
        assert_eq!(
            entry["feedback"]["diagnosis"],
            "The fixture file named fixed was never created. Create it first.",
            "{persisted}"
        );

        // Another process resumes the same checkpoint run.
        drop(dispatcher);
        let (resumed, _) = make_test_dispatcher(
            &temp,
            FEEDBACK_PROVIDER,
            diagnosing,
            GraphFeedbackContext::default(),
        )
        .await;
        resumed.attach_retry_feedback(&spec.plan_id, feedback_file.clone(), "run-1");
        resumed
            .dispatch(&spec, Vec::new(), &CellContext::new())
            .await
            .expect("the resumed attempt gets the feedback and passes");

        let first = std::fs::read_to_string(temp.path().join("prompts/0.txt")).expect("prompt 0");
        assert!(!first.contains("# Previous attempt feedback"), "{first}");
        let resumed_prompt =
            std::fs::read_to_string(temp.path().join("prompts/1.txt")).expect("prompt 1");
        assert!(
            resumed_prompt.contains("# Previous attempt feedback"),
            "{resumed_prompt}"
        );
        assert!(
            resumed_prompt.contains(
                "## Diagnosis\nThe fixture file named fixed was never created. Create it first."
            ),
            "{resumed_prompt}"
        );
        assert!(resumed_prompt.contains("test -f fixed"), "{resumed_prompt}");
        assert!(
            !feedback_file.exists(),
            "a pass clears the persisted feedback"
        );
    }

    #[tokio::test]
    async fn feedback_attempt_numbers_count_provider_failures_like_the_retry_budget() {
        let temp = tempdir().expect("tempdir");
        // Two provider failures, then a completed attempt whose verify fails:
        // the Graph engine's third attempt of a `max_retries = 2` task.
        let provider = r#"#!/bin/sh
set -eu
cat >/dev/null
calls=$(cat calls 2>/dev/null || echo 0)
echo $((calls + 1)) > calls
if [ "$calls" -lt 2 ]; then
  printf '%s\n' '{"type":"result","session_id":"s","model":"claude-sonnet-4-6","total_cost_usd":0.0,"usage":{"input_tokens":1,"output_tokens":1},"is_error":true}'
  exit 1
fi
printf '%s\n' '{"type":"content_block_delta","delta":{"text":"done"}}'
printf '%s\n' '{"type":"result","session_id":"s","model":"claude-sonnet-4-6","total_cost_usd":0.01,"usage":{"input_tokens":5,"output_tokens":10}}'
"#;
        let feedback_file = temp.path().join("retry-feedback.json");
        let (dispatcher, mut task) = make_test_dispatcher(
            &temp,
            provider,
            no_auto_fix,
            GraphFeedbackContext::default(),
        )
        .await;
        task.max_retries = 2;
        task.verify = vec![verify_step("structural", "exit 1")];
        let spec = make_spec(&task);
        dispatcher.attach_retry_feedback(&spec.plan_id, feedback_file.clone(), "run-1");

        for expected in ["provider", "provider", "verify"] {
            let error = dispatcher
                .dispatch(&spec, Vec::new(), &CellContext::new())
                .await
                .expect_err("every attempt fails");
            let kind = if matches!(error, RokoError::Verify { .. }) {
                "verify"
            } else {
                "provider"
            };
            assert_eq!(kind, expected, "{error}");
        }
        let persisted: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&feedback_file).expect("feedback persisted"))
                .expect("feedback json");
        assert_eq!(
            persisted["tasks"][task.id.as_str()]["next_attempt"],
            3,
            "the verify failure was attempt 2: {persisted}"
        );
    }

    #[test]
    fn a_failure_record_names_the_failed_steps_rung() {
        let outcomes = |steps: &[(&str, bool)]| {
            steps
                .iter()
                .map(|(phase, passed)| ((*phase).to_string(), *passed))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            failed_step_rung(&outcomes(&[("compile", true), ("test", false)])),
            2
        );
        assert_eq!(failed_step_rung(&outcomes(&[("clippy", false)])), 1);
        assert_eq!(
            failed_step_rung(&outcomes(&[("structural", false)])),
            5,
            "shell steps outside the canonical rungs use the custom gate's rung"
        );
    }

    /// Dispatches `task` beside a fake sibling `T12` of the same plan that
    /// edits `web/src/PlanView.tsx` in the same working tree and finishes its
    /// attempt, first creating `sibling_done`, once the dispatcher has begun
    /// to settle the step that failed beside it (it created `failed_once`).
    async fn dispatch_beside_editing_sibling(
        dispatcher: &GraphTaskDispatcher,
        task: &TaskDef,
        failed_once: &Path,
        sibling_done: &Path,
    ) -> Result<Vec<Signal>> {
        let spec = make_spec(task);
        let sibling = dispatcher.in_flight.register(
            &format!("{}/T12", spec.plan_id),
            &dispatcher.workdir,
            &["web/src/PlanView.tsx".to_string()],
        );
        let key = format!("{}/{}", spec.plan_id, task.id);
        let finish_sibling = async {
            // The failing step creates `failed_once` before the dispatcher
            // sees it fail: ending the sibling then could leave the settle no
            // writer to wait for (bug-779ae7).
            dispatcher.in_flight.settling_began(&key).await;
            assert!(
                failed_once.exists(),
                "the settled step failed beside the sibling"
            );
            std::fs::write(sibling_done, "").expect("sibling edit");
            drop(sibling);
        };
        let ctx = CellContext::new();
        // Only a hang guard: the sibling ends on the settle signal, not a clock.
        let (outcome, ()) = tokio::time::timeout(std::time::Duration::from_secs(300), async {
            tokio::join!(dispatcher.dispatch(&spec, Vec::new(), &ctx), finish_sibling)
        })
        .await
        .expect("the failed step settles once the sibling finishes");
        outcome
    }

    fn settle_quickly(config: &mut RokoConfig) {
        config.gates.cargo_fix_enabled = false;
        config.gates.sibling_settle_secs = 30;
    }

    #[tokio::test]
    async fn a_verify_failure_beside_an_editing_sibling_is_rerun_once_it_settles() {
        let temp = tempdir().expect("tempdir");
        let (dispatcher, mut task) = make_test_dispatcher(
            &temp,
            VERIFY_PROVIDER,
            settle_quickly,
            GraphFeedbackContext::default(),
        )
        .await;
        let failed_once = temp.path().join("failed-once");
        let sibling_done = temp.path().join("sibling-done");
        task.verify = vec![verify_step(
            "typecheck",
            &format!(
                "test -f {} || {{ touch {}; exit 2; }}",
                sibling_done.display(),
                failed_once.display()
            ),
        )];

        let outputs =
            dispatch_beside_editing_sibling(&dispatcher, &task, &failed_once, &sibling_done)
                .await
                .expect("the re-run after the sibling settled passes");

        assert!(
            failed_once.exists(),
            "the first run failed beside the sibling"
        );
        assert_eq!(
            TaskGateVerdict::from_signals(&outputs),
            Some(TaskGateVerdict::Passed)
        );
    }

    #[tokio::test]
    async fn a_verify_failure_left_in_a_sibling_file_blames_the_sibling() {
        let temp = tempdir().expect("tempdir");
        let (dispatcher, mut task) = make_test_dispatcher(
            &temp,
            VERIFY_PROVIDER,
            settle_quickly,
            GraphFeedbackContext::default(),
        )
        .await;
        let failed_once = temp.path().join("failed-once");
        let sibling_done = temp.path().join("sibling-done");
        task.verify = vec![verify_step(
            "typecheck",
            &format!(
                "touch {}; echo 'src/PlanView.tsx(448,24): error TS2304: \
                 Cannot find name formatDuration.' >&2; exit 2",
                failed_once.display()
            ),
        )];

        let error =
            dispatch_beside_editing_sibling(&dispatcher, &task, &failed_once, &sibling_done)
                .await
                .expect_err("the failure persists after the sibling settled");

        let RokoError::Verify { message, .. } = error else {
            panic!("expected a verify failure, got {error}");
        };
        assert!(
            message.starts_with("blocked_by_sibling = T12: 1/1 verify step(s) failed"),
            "{message}"
        );
        assert!(
            message.contains("first at src/PlanView.tsx:448:24"),
            "{message}"
        );
    }

    /// How long a dispatch that should finish may take on a loaded machine.
    /// Far below the limits [`wait_while_siblings_edit`] sets, so a step that
    /// waits when it should not fails the test here (bug-779ae7).
    const HANG_GUARD: std::time::Duration = std::time::Duration::from_secs(120);

    /// No auto-fix, and a sibling wait and provider timeouts far beyond
    /// [`HANG_GUARD`]: however loaded the machine, a step waits exactly as
    /// long as its siblings edit, and the fake provider never times out.
    /// The tests also give the task an attempt timeout beyond the guard.
    fn wait_while_siblings_edit(config: &mut RokoConfig) {
        config.gates.cargo_fix_enabled = false;
        config.gates.sibling_settle_secs = 3_600;
        for provider in config.providers.values_mut() {
            provider.timeout_ms = Some(600_000);
            provider.ttft_timeout_ms = Some(600_000);
        }
    }

    /// gap-1920ba: a verify step that reads the whole project (here hidden
    /// behind `bash -c`) waits until a sibling sharing the working tree has
    /// finished editing, so it never checks a half-written file. The sibling
    /// finishes only once the step has reached its wait without running;
    /// the step then runs once, after the sibling, and passes.
    #[tokio::test]
    async fn a_whole_project_verify_never_runs_while_a_sibling_edits() {
        let temp = tempdir().expect("tempdir");
        let (dispatcher, mut task) = make_test_dispatcher(
            &temp,
            VERIFY_PROVIDER,
            wait_while_siblings_edit,
            GraphFeedbackContext::default(),
        )
        .await;
        task.timeout_secs = 600;
        let sibling_done = temp.path().join("sibling-done");
        let runs = temp.path().join("verify-runs");
        let mut step = verify_step(
            "typecheck",
            &format!(
                "bash -c 'echo run >> {}; test -f {}'",
                runs.display(),
                sibling_done.display()
            ),
        );
        step.timeout_ms = 120_000;
        task.verify = vec![step];
        let spec = make_spec(&task);
        let sibling = dispatcher.in_flight.register(
            &format!("{}/T12", spec.plan_id),
            &dispatcher.workdir,
            &["web/src/PlanView.tsx".to_string()],
        );
        let key = format!("{}/{}", spec.plan_id, task.id);
        let finish_sibling = async {
            dispatcher.in_flight.reading_began(&key).await;
            // Time for a step that did not wait to have started.
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            assert!(!runs.exists(), "the step ran while its sibling edited");
            std::fs::write(&sibling_done, "").expect("sibling edit");
            drop(sibling);
        };

        let ctx = CellContext::new();
        let dispatch = dispatcher.dispatch(&spec, Vec::new(), &ctx);
        tokio::pin!(dispatch);
        let outputs = tokio::time::timeout(HANG_GUARD, async {
            tokio::select! {
                outcome = &mut dispatch => {
                    panic!("the attempt ended before its verify step ran: {:?}", outcome.err())
                }
                () = finish_sibling => dispatch.await,
            }
        })
        .await
        .expect("the step runs once its sibling is done")
        .expect("the step ran after the sibling's edit");

        assert_eq!(
            TaskGateVerdict::from_signals(&outputs),
            Some(TaskGateVerdict::Passed)
        );
        let runs = std::fs::read_to_string(&runs).expect("verify runs");
        assert_eq!(runs.lines().count(), 1, "the step ran once: {runs:?}");
    }

    /// A verify step whose scope the sibling does not write runs at once,
    /// while the sibling is still editing. The sibling edits until the
    /// dispatch returns, so a step that waited for it would hang.
    #[tokio::test]
    async fn a_scoped_verify_runs_beside_a_sibling_editing_elsewhere() {
        let temp = tempdir().expect("tempdir");
        let (dispatcher, mut task) = make_test_dispatcher(
            &temp,
            VERIFY_PROVIDER,
            wait_while_siblings_edit,
            GraphFeedbackContext::default(),
        )
        .await;
        task.timeout_secs = 600;
        let mut step = verify_step("typecheck", "true");
        step.scope = vec!["crates/own".to_string()];
        step.timeout_ms = 120_000;
        task.verify = vec![step];
        let spec = make_spec(&task);
        let sibling = dispatcher.in_flight.register(
            &format!("{}/T12", spec.plan_id),
            &dispatcher.workdir,
            &["web/src/PlanView.tsx".to_string()],
        );

        let ctx = CellContext::new();
        let outputs =
            tokio::time::timeout(HANG_GUARD, dispatcher.dispatch(&spec, Vec::new(), &ctx))
                .await
                .expect("the step ran while the sibling was still editing")
                .expect("dispatch");
        drop(sibling);

        assert_eq!(
            TaskGateVerdict::from_signals(&outputs),
            Some(TaskGateVerdict::Passed)
        );
    }

    /// bug-951930: the post-auto-fix re-run starts a cargo step only once it
    /// holds the compile lock, as the first run does.
    #[tokio::test]
    async fn post_fix_rerun_takes_compile_lock() {
        let temp = tempdir().expect("tempdir");
        let workdir = temp.path();
        let marker = workdir.join("ran.txt");
        let step = verify_step("compile", "echo ran > ran.txt # cargo check");
        let gate = ShellGate::new("bash", vec!["-c".into(), step.command.clone()])
            .with_timeout_ms(step.timeout_ms)
            .with_name("verify-1-compile")
            .with_phase(&step.phase);
        let signal = Signal::builder(Kind::Task)
            .body(Body::from_json(&GatePayload::in_dir(workdir)).expect("gate payload"))
            .build();
        let ctx = Context::now();

        // Another plan's build holds the repository's only compile permit.
        let held = crate::runner::gate_dispatch::acquire_compile_ownership(
            workdir,
            1,
            std::time::Duration::from_secs(5),
            "other-plan",
            "T9",
            "cargo build",
        )
        .await
        .expect("compile permit");
        let running = tokio_util::sync::CancellationToken::new();
        tokio::time::timeout(
            std::time::Duration::from_millis(100),
            verify_step_locked(
                &gate, &signal, &ctx, workdir, 1, &step, "plan", "T1", &running,
            ),
        )
        .await
        .expect_err("the re-run must wait for the compile lock");
        assert!(!marker.exists(), "the step ran without the compile lock");

        drop(held);
        let verdict = verify_step_locked(
            &gate, &signal, &ctx, workdir, 1, &step, "plan", "T1", &running,
        )
        .await
        .expect("the run is not stopping");
        assert!(verdict.passed, "{verdict:?}");
        assert!(marker.exists());
    }

    /// bug-c33c6e: a post-auto-fix re-run whose plan run begins to stop while
    /// it waits for the compile lock does not start (bug-3a3968 ends the wait
    /// itself on the stop).
    #[tokio::test]
    async fn auto_fix_rerun_stops_after_the_lock_wait() {
        let temp = tempdir().expect("tempdir");
        let workdir = temp.path();
        let marker = workdir.join("ran.txt");
        let step = verify_step("compile", "echo ran > ran.txt # cargo check");
        let gate = ShellGate::new("bash", vec!["-c".into(), step.command.clone()])
            .with_timeout_ms(step.timeout_ms)
            .with_name("verify-1-compile")
            .with_phase(&step.phase);
        let signal = Signal::builder(Kind::Task)
            .body(Body::from_json(&GatePayload::in_dir(workdir)).expect("gate payload"))
            .build();
        let ctx = Context::now();
        let held = crate::runner::gate_dispatch::acquire_compile_ownership(
            workdir,
            1,
            std::time::Duration::from_secs(5),
            "other-plan",
            "T9",
            "cargo build",
        )
        .await
        .expect("compile permit");

        let stop = tokio_util::sync::CancellationToken::new();
        let rerun =
            verify_step_locked(&gate, &signal, &ctx, workdir, 1, &step, "plan", "T1", &stop);
        // The run begins to stop while the re-run waits for the lock, then the
        // lock frees.
        let stop_then_release = async {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            stop.cancel();
            drop(held);
        };
        let (rerun, ()) = tokio::join!(rerun, stop_then_release);

        assert!(rerun.is_none(), "{rerun:?}");
        assert!(
            !marker.exists(),
            "the step started after its run began to stop"
        );
    }

    /// bug-3a3968: a verify step waiting for a sibling that edits what it
    /// reads, or for the compile lock another build holds, stops waiting when
    /// its plan run begins to stop, though the sibling still edits and the
    /// build still holds the lock. The step never runs, and the attempt
    /// settles as cancelled.
    #[tokio::test]
    async fn verify_waits_end_on_a_stop() {
        for blocker in ["sibling", "compile lock"] {
            let temp = tempdir().expect("tempdir");
            let runs = temp.path().join(".roko/runs");
            let feedback = GraphFeedbackContext {
                runs_dir: Some(runs.clone()),
                ..GraphFeedbackContext::default()
            };
            let (dispatcher, mut task) =
                make_test_dispatcher(&temp, VERIFY_PROVIDER, wait_while_siblings_edit, feedback)
                    .await;
            task.timeout_secs = 600;
            let marker = temp.path().join("ran.txt");
            // The step reads the whole project, so it waits for the sibling,
            // and it runs cargo, so it waits for the compile lock.
            let mut step = verify_step("compile", "bash -c 'echo ran > ran.txt' # cargo check");
            step.timeout_ms = 120_000;
            task.verify = vec![step];
            let spec = make_spec(&task);
            // What the step waits for, held until the case ends.
            let sibling = (blocker == "sibling").then(|| {
                dispatcher.in_flight.register(
                    &format!("{}/T12", spec.plan_id),
                    &dispatcher.workdir,
                    &["web/src/PlanView.tsx".to_string()],
                )
            });
            let build = if blocker == "compile lock" {
                let held = crate::runner::gate_dispatch::acquire_compile_ownership(
                    &dispatcher.workdir,
                    1,
                    std::time::Duration::from_secs(5),
                    "other-plan",
                    "T9",
                    "cargo build",
                )
                .await
                .expect("compile permit");
                Some(held)
            } else {
                None
            };

            let key = format!("{}/{}", spec.plan_id, task.id);
            let ctx = CellContext::new().with_run_id("stopped-run".to_string());
            let dispatched = dispatcher.dispatch(&spec, Vec::new(), &ctx);
            let stop = async {
                dispatcher.in_flight.reading_began(&key).await;
                dispatcher.begin_stop();
            };
            let (result, ()) =
                tokio::time::timeout(HANG_GUARD, async { tokio::join!(dispatched, stop) })
                    .await
                    .expect("the wait ends on the stop");
            let error = result.expect_err("the stopped wait fails the attempt");
            assert!(
                matches!(error, RokoError::Cancelled(_)),
                "{blocker}: {error}"
            );
            assert!(!marker.exists(), "{blocker}: the step ran");
            let verdicts = crate::graph_task_dispatch::tests::jsonl_rows_where(
                &runs.join("stopped-run").join("attempts.jsonl"),
                1,
                |row| row["schema_version"] == "roko.verdict/1",
            )
            .await;
            assert_eq!(verdicts[0]["outcome"], "cancelled", "{blocker}");
            drop((sibling, build));
        }
    }

    #[tokio::test]
    async fn verified_outcome_drives_output_verdict_and_feedback() {
        let temp = tempdir().expect("tempdir");
        let efficiency = temp.path().join("efficiency.jsonl");
        let feedback = GraphFeedbackContext {
            efficiency_path: Some(efficiency.clone()),
            ..GraphFeedbackContext::default()
        };
        let (dispatcher, mut task) =
            make_test_dispatcher(&temp, VERIFY_PROVIDER, no_auto_fix, feedback).await;

        let unverified = dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &CellContext::new())
            .await
            .expect("dispatch without verify steps");
        assert_eq!(
            TaskGateVerdict::from_signals(&unverified),
            Some(TaskGateVerdict::Unverified)
        );

        task.verify = vec![verify_step("structural", "true")];
        let passed = dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &CellContext::new())
            .await
            .expect("verified dispatch");
        assert_eq!(
            TaskGateVerdict::from_signals(&passed),
            Some(TaskGateVerdict::Passed)
        );

        task.verify = vec![verify_step("structural", "false")];
        dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &CellContext::new())
            .await
            .expect_err("verify failure");

        // The provider succeeded all three times; learning must record the
        // verified outcome, so the failed-verify attempt is a failure. Each
        // attempt gets its own key, which its gate-pass record extends; with
        // no Graph run in the cell context, the key names the dispatcher's
        // own run.
        let chain = format!(
            "{}:{}:{}",
            dispatcher.attempts.fallback_run_id(),
            make_spec(&task).plan_id,
            task.id
        );
        let attempt =
            |suffix: &str, outcome: &str| (format!("{chain}:{suffix}"), outcome.to_string());
        assert_eq!(
            efficiency_records(&efficiency, 4).await,
            vec![
                attempt("1", "success"),
                attempt("2", "success"),
                attempt("2/gate-pass", "gate_pass"),
                attempt("3", "failure"),
            ]
        );
    }

    #[test]
    fn published_gate_output_leads_with_the_command() {
        assert_eq!(
            published_gate_output(
                "cargo check -p roko-cli",
                &roko_core::Verdict::fail("verify[0]", "exit code: 101")
                    .with_detail("error[E0425]\n"),
            ),
            "$ cargo check -p roko-cli\nerror[E0425]\n✗ exit status 101"
        );
        assert_eq!(
            published_gate_output("true", &roko_core::Verdict::pass("verify[0]")),
            "$ true"
        );
        assert_eq!(verify_step_label(2, ""), "verify[2]");
        assert_eq!(verify_step_label(0, "compile"), "verify[0:compile]");
    }

    /// Every record of the gate-failure log at `path`, once `expected` have
    /// landed from the background writer.
    async fn gate_failure_records(
        path: &Path,
        expected: usize,
    ) -> Vec<roko_gate::GateFailureRecord> {
        crate::background_writes::settled(path.parent().unwrap_or(path)).await;
        for _ in 0..600 {
            let records: Vec<roko_gate::GateFailureRecord> = std::fs::read_to_string(path)
                .unwrap_or_default()
                .lines()
                .filter_map(|line| serde_json::from_str(line).ok())
                .collect();
            if records.len() >= expected {
                return records;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        panic!("no gate-failure record reached {}", path.display());
    }

    /// A verify step cut off at its `timeout_ms` is recorded as a timeout in
    /// `gate-failures.jsonl`, though its authored `fail_msg` stands in for
    /// the step's own "timed out" reason in the failure text. `roko diagnose`
    /// counts the attempt as timed out from that record (its test of the same
    /// name).
    #[tokio::test]
    async fn a_verify_step_timeout_is_recorded_as_a_timeout() {
        let temp = tempdir().expect("tempdir");
        let gate_failures = temp.path().join(".roko/learn/gate-failures.jsonl");
        let feedback = GraphFeedbackContext {
            gate_failures_path: Some(gate_failures.clone()),
            ..GraphFeedbackContext::default()
        };
        let (dispatcher, mut task) =
            make_test_dispatcher(&temp, VERIFY_PROVIDER, no_auto_fix, feedback).await;
        task.verify = vec![crate::task_parser::VerifyStep {
            fail_msg: Some("the check failed".to_string()),
            timeout_ms: 200,
            ..verify_step("structural", "sleep 30")
        }];

        let error = dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &CellContext::new())
            .await
            .expect_err("the step runs out of time");
        assert!(matches!(error, RokoError::Verify { .. }), "{error}");

        let records = gate_failure_records(&gate_failures, 1).await;
        assert_eq!(records.len(), 1);
        let record = &records[0];
        assert_eq!(record.task_id, task.id);
        assert_eq!(record.failure_kind, roko_gate::GateFailureKind::Timeout);
    }

    /// bug-6f7f72: a long verify command no longer crowds its failure
    /// message out of the gate failure record. The summary leads with the
    /// step's label, which `roko diagnose` reads, and leaves the command out.
    #[tokio::test]
    async fn a_long_verify_command_keeps_its_failure_message() {
        let temp = tempdir().expect("tempdir");
        let gate_failures = temp.path().join(".roko/learn/gate-failures.jsonl");
        let feedback = GraphFeedbackContext {
            gate_failures_path: Some(gate_failures.clone()),
            ..GraphFeedbackContext::default()
        };
        let (dispatcher, mut task) =
            make_test_dispatcher(&temp, VERIFY_PROVIDER, no_auto_fix, feedback).await;
        let padding = "a_long_filter_name".repeat(16);
        let command = format!(": {padding}; echo 'the widget count is off by one' >&2; exit 1");
        assert!(command.len() > 250);
        task.verify = vec![crate::task_parser::VerifyStep {
            fail_msg: Some("widgets do not add up".to_string()),
            ..verify_step("test", &command)
        }];

        dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &CellContext::new())
            .await
            .expect_err("the step fails");

        let records = gate_failure_records(&gate_failures, 1).await;
        let summary = &records[0].summary;
        assert!(
            summary.starts_with("verify[0:test]: widgets do not add up"),
            "{summary}"
        );
        assert!(
            summary.contains("the widget count is off by one"),
            "{summary}"
        );
        assert!(
            !summary.contains(&padding),
            "the command is left out: {summary}"
        );
    }

    /// bug-82cbef: a verify step that fails once its plan run began to stop,
    /// as a gate command does when an interrupt signals the run's commands,
    /// settles the attempt as cancelled. It leaves no gate-failure record,
    /// and the dispatch fails with a cancellation, which the task executor
    /// does not retry.
    #[tokio::test]
    async fn interrupted_verify_settles_as_cancelled() {
        let temp = tempdir().expect("tempdir");
        let runs = temp.path().join(".roko/runs");
        let gate_failures = temp.path().join(".roko/learn/gate-failures.jsonl");
        let feedback = GraphFeedbackContext {
            runs_dir: Some(runs.clone()),
            gate_failures_path: Some(gate_failures.clone()),
            ..GraphFeedbackContext::default()
        };
        let (dispatcher, mut task) =
            make_test_dispatcher(&temp, VERIFY_PROVIDER, no_auto_fix, feedback).await;
        let started = temp.path().join("verify-started");
        let stop_now = temp.path().join("stop-now");
        // The step runs until the run stops, then exits on SIGTERM.
        task.verify = vec![verify_step(
            "structural",
            &format!(
                "touch '{}'; until [ -e '{}' ]; do sleep 0.05; done; kill -TERM $$",
                started.display(),
                stop_now.display()
            ),
        )];
        let spec = make_spec(&task);
        let ctx = CellContext::new().with_run_id("interrupted-run".to_string());

        let dispatched = dispatcher.dispatch(&spec, Vec::new(), &ctx);
        let stop = async {
            for _ in 0..1_200 {
                if started.exists() {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(25)).await;
            }
            dispatcher.begin_stop();
            std::fs::write(&stop_now, "").expect("stop the step");
        };
        let (result, ()) = tokio::join!(dispatched, stop);
        let error = result.expect_err("the stopped verify fails the attempt");
        assert!(matches!(error, RokoError::Cancelled(_)), "{error}");

        let verdicts = crate::graph_task_dispatch::tests::jsonl_rows_where(
            &runs.join("interrupted-run").join("attempts.jsonl"),
            1,
            |row| row["schema_version"] == "roko.verdict/1",
        )
        .await;
        assert_eq!(verdicts[0]["outcome"], "cancelled", "{}", verdicts[0]);
        crate::background_writes::settled(gate_failures.parent().unwrap_or(temp.path())).await;
        assert!(
            !gate_failures.exists(),
            "a stopped step records no gate failure"
        );
    }

    fn rung(name: &str, command: &str, required: bool) -> roko_core::config::GateRungConfig {
        roko_core::config::GateRungConfig {
            name: name.to_string(),
            command: command.to_string(),
            timeout_secs: 10,
            required,
            parallel_with: Vec::new(),
        }
    }

    /// `[[gates.rungs]]` guard every plan task: once a task's authored steps
    /// pass, a failing required workspace rung fails the attempt. A rung an
    /// authored step already runs does not run twice, an optional rung never
    /// runs, and a task without authored steps is verified by the rungs.
    #[tokio::test]
    async fn plan_run_task_runs_the_workspace_gate_rungs() {
        let temp = tempdir().expect("tempdir");
        let lint_clean = temp.path().join("lint-clean");
        let docs_ran = temp.path().join("docs-ran");
        let rungs = vec![
            rung("same", "true", true),
            rung("lint", &format!("test -f {}", lint_clean.display()), true),
            rung("docs", &format!("touch {}", docs_ran.display()), false),
        ];
        let (dispatcher, mut task) = make_test_dispatcher(
            &temp,
            VERIFY_PROVIDER,
            |config| {
                no_auto_fix(config);
                config.gates.custom_rungs = rungs;
            },
            GraphFeedbackContext::default(),
        )
        .await;
        task.verify = vec![verify_step("structural", "true")];

        let error = dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &CellContext::new())
            .await
            .expect_err("the failing workspace rung fails the task");
        let RokoError::Verify { message, .. } = error else {
            panic!("expected a verify failure, got {error}");
        };
        // The authored step and `lint`: `same` repeats the authored command.
        assert!(message.contains("1/2 verify step(s) failed"), "{message}");
        assert!(message.contains("rung[lint] (`test -f"), "{message}");

        std::fs::write(&lint_clean, "").expect("lint passes");
        let passed = dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &CellContext::new())
            .await
            .expect("the authored step and the rungs pass");
        assert_eq!(
            TaskGateVerdict::from_signals(&passed),
            Some(TaskGateVerdict::Passed)
        );

        task.verify.clear();
        let verified = dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &CellContext::new())
            .await
            .expect("the rungs verify a task without authored steps");
        assert_eq!(
            TaskGateVerdict::from_signals(&verified),
            Some(TaskGateVerdict::Passed)
        );
        assert!(!docs_ran.exists(), "an optional rung never runs");
    }

    /// A plan dir holding a `tasks.toml` whose `[meta]` opts out of the
    /// workspace rungs, and a spec of `task` in it.
    fn opted_out_spec(temp: &tempfile::TempDir, task: &TaskDef) -> TaskExecutionSpec {
        let plan_dir = temp.path().join("plans/opted-out");
        std::fs::create_dir_all(&plan_dir).expect("plan dir");
        std::fs::write(
            plan_dir.join("tasks.toml"),
            r#"
[meta]
plan = "opted-out"
workspace_rungs = false

[[task]]
id = "T-STREAM"
title = "Streaming graph task"
"#,
        )
        .expect("tasks.toml");
        let mut spec = make_spec(task);
        spec.plan_id = "opted-out".to_string();
        spec.plan_dir = plan_dir.display().to_string();
        spec
    }

    /// A plan with `[meta] workspace_rungs = false` keeps its tasks to their
    /// own verify steps; a plan without it still runs the workspace rungs.
    #[tokio::test]
    async fn a_plan_can_opt_out_of_the_workspace_rungs() {
        let temp = tempdir().expect("tempdir");
        let (dispatcher, mut task) = make_test_dispatcher(
            &temp,
            VERIFY_PROVIDER,
            |config| {
                no_auto_fix(config);
                config.gates.custom_rungs = vec![rung("lint", "exit 3", true)];
            },
            GraphFeedbackContext::default(),
        )
        .await;
        task.verify = vec![verify_step("structural", "true")];
        let opted_out = opted_out_spec(&temp, &task);

        let outputs = dispatcher
            .dispatch(&opted_out, Vec::new(), &CellContext::new())
            .await
            .expect("the opted-out plan's task skips the failing rung");
        assert_eq!(
            TaskGateVerdict::from_signals(&outputs),
            Some(TaskGateVerdict::Passed)
        );

        let error = dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &CellContext::new())
            .await
            .expect_err("another plan runs the failing rung");
        let RokoError::Verify { message, .. } = error else {
            panic!("expected a verify failure, got {error}");
        };
        assert!(message.contains("rung[lint] (`exit 3`)"), "{message}");
    }

    /// A reflex serves only a task that no verify step checks, so a task the
    /// workspace rungs check is dispatched, and its rungs run, even when a
    /// reflex rule matches it.
    #[tokio::test]
    async fn a_reflex_pass_still_runs_the_workspace_rungs() {
        use roko_learn::reflex_store::{PromotionCandidate, ReflexAction, ReflexCondition};

        let store_dir = tempdir().expect("tempdir");
        let reflexes = ReflexStore::open(store_dir.path().join("reflexes.jsonl"));
        // A wildcard rule matches every task.
        let promoted = reflexes.try_promote(
            &PromotionCandidate {
                episode_id: "episode-reflex".to_string(),
                condition: ReflexCondition::default(),
                action: ReflexAction {
                    tool: "respond".to_string(),
                    args: "cached reflex output".to_string(),
                },
            },
            3,
        );
        assert!(promoted);
        let temp = tempdir().expect("tempdir");
        let rung_ran = temp.path().join("rung-ran");
        let check = rung("check", &format!("touch {}", rung_ran.display()), true);
        let (dispatcher, task) = make_batch_dispatcher(&temp, 0.01, |config| {
            no_auto_fix(config);
            config.learning.t0_reflexes = true;
            config.gates.custom_rungs = vec![check];
        })
        .await;
        let dispatcher = dispatcher.with_reflex_store(reflexes);
        assert!(task.verify.is_empty(), "only the rung checks the task");

        let outputs = dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &batch_ctx())
            .await
            .expect("the task passes its rung");
        assert_eq!(
            outputs[0].body.as_text().expect("output text"),
            "batch-output",
            "the agent ran, not the reflex"
        );
        assert!(rung_ran.exists(), "the workspace rung ran");
        assert_eq!(
            TaskGateVerdict::from_signals(&outputs),
            Some(TaskGateVerdict::Passed)
        );
    }

    /// Provider that saves its prompt (argv, which carries the system prompt,
    /// then stdin) to `prompt.txt` in its working directory.
    const PROMPT_LOG_PROVIDER: &str = r#"#!/bin/sh
set -eu
input="$(cat)"
printf '%s\n---\n%s\n' "$*" "$input" > prompt.txt
printf '%s\n' '{"type":"content_block_delta","delta":{"text":"done"}}'
printf '%s\n' '{"type":"result","session_id":"s","model":"claude-sonnet-4-6","total_cost_usd":0.01,"usage":{"input_tokens":5,"output_tokens":10}}'
"#;

    /// The prompt shows every check that will judge the task: its own verify
    /// steps, then the workspace rungs it faces, and no rung its plan opts out
    /// of.
    #[tokio::test]
    async fn task_prompts_list_the_workspace_rungs() {
        let temp = tempdir().expect("tempdir");
        let (dispatcher, mut task) = make_test_dispatcher(
            &temp,
            PROMPT_LOG_PROVIDER,
            |config| {
                no_auto_fix(config);
                config.gates.custom_rungs = vec![rung("lint", "true # the lint rung", true)];
            },
            GraphFeedbackContext::default(),
        )
        .await;
        task.verify = vec![verify_step("structural", "true # the own step")];
        let prompt = || std::fs::read_to_string(temp.path().join("prompt.txt")).expect("prompt");

        dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &CellContext::new())
            .await
            .expect("the task and its rung pass");
        let text = prompt();
        let own = text.find("true # the own step").expect("own step");
        let lint = text.find("true # the lint rung").expect("rung");
        assert!(own < lint, "rung after own steps:\n{text}");

        let opted_out = opted_out_spec(&temp, &task);
        dispatcher
            .dispatch(&opted_out, Vec::new(), &CellContext::new())
            .await
            .expect("the task passes its own step");
        let text = prompt();
        assert!(text.contains("true # the own step"), "{text}");
        assert!(!text.contains("the lint rung"), "opted out:\n{text}");
    }

    /// gap-69a56e: a task's gate-profile hints choose optional workspace
    /// rungs. A hardened task runs every declared rung, a task that names test
    /// invariants also the rungs that run tests, and any other task only the
    /// required ones.
    #[tokio::test]
    async fn quality_profile_and_test_invariants_select_gate_rungs() {
        let temp = tempdir().expect("tempdir");
        let (dispatcher, mut task) = make_test_dispatcher(
            &temp,
            VERIFY_PROVIDER,
            |config| {
                config.gates.custom_rungs = vec![
                    rung("compile", "true # compile", true),
                    rung("test", "true # test", false),
                    rung("audit", "true # audit", false),
                ];
            },
            GraphFeedbackContext::default(),
        )
        .await;
        task.verify = Vec::new();
        let rungs_of = |task: &TaskDef| -> Vec<String> {
            dispatcher
                .verify_steps(&make_spec(task), task)
                .into_iter()
                .map(|(label, _)| label)
                .collect()
        };
        assert_eq!(rungs_of(&task), ["rung[compile]"]);

        let mut invariants = task.clone();
        invariants.hints.test_invariants = Some(vec!["INV-1".to_string()]);
        assert_eq!(rungs_of(&invariants), ["rung[compile]", "rung[test]"]);

        let mut hardened = task.clone();
        hardened.hints.quality_profile = Some(roko_core::TaskQualityProfile::Hardened);
        assert_eq!(
            rungs_of(&hardened),
            ["rung[compile]", "rung[test]", "rung[audit]"]
        );
    }

    /// A pinned acceptance step is quoted by its header line, not its
    /// generated script, in the failure and skipped-step lines that become
    /// retry feedback.
    #[tokio::test]
    async fn pinned_steps_are_quoted_by_their_header_in_feedback() {
        let temp = tempdir().expect("tempdir");
        let (dispatcher, mut task) = make_test_dispatcher(
            &temp,
            VERIFY_PROVIDER,
            no_auto_fix,
            GraphFeedbackContext::default(),
        )
        .await;
        let pinned = |name: &str, script: &str| crate::task_parser::VerifyStep {
            fail_msg: Some(format!("the pinned acceptance test {name} failed")),
            ..verify_step("test", &format!("# roko accept: {name}\n{script}"))
        };
        task.verify = vec![
            pinned("one.sh", "roko_first_script_line=1\nexit 1"),
            pinned("two.sh", "roko_second_script_line=1\nexit 0"),
        ];

        let error = dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &CellContext::new())
            .await
            .expect_err("the first pinned step fails");
        let RokoError::Verify { message, .. } = error else {
            panic!("expected a verify failure, got {error}");
        };
        assert!(
            message.contains("verify[0:test] (`# roko accept: one.sh`): the pinned"),
            "{message}"
        );
        assert!(
            message.contains("verify[1:test] (`# roko accept: two.sh`)"),
            "{message}"
        );
        for script in ["roko_first_script_line", "roko_second_script_line"] {
            assert!(!message.contains(script), "{script}:\n{message}");
        }
    }
}
