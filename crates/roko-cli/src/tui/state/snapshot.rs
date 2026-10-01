//! Dashboard snapshot application.
//!
//! Contains `update_from_snapshot` (offline file-based data loading) and
//! `update_from_dashboard_snapshot` (live push-based snapshot protocol),
//! plus all the helper functions they call.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;
use std::time::{Duration, Instant};

use super::super::dashboard::{
    AgentSummary, CascadeRouterState, DashboardData, EfficiencySummary, GateResultSummary,
    KnowledgeBrowseEntry, PlanTaskListSnapshot,
};
use super::learning::{
    build_token_samples, compute_token_rate, compute_windowed_token_rate, current_epoch_ms,
    extract_episode_output, fallback_route_metrics_for_agent, latest_agent_events,
    latest_route_metrics, plan_is_active, populate_provider_statuses, route_tier_label_for_model,
    sum_costs,
};
use super::{
    AgentRow, AgentStatus, AgentTopologyStatus, CANONICAL_PHASES, GateResultEntry, MAX_GATE_LINES,
    PhaseStep, PlanEntry, PlanPhase, TaskEntry, TaskRow, TaskStatus, TuiState, Wave,
    bounded_output_lines, count_online_from_files, derive_topology_from_agents, gate_pass_rate,
    is_online_agent_status, model_context_limit, snapshot_gate_pass_rate,
};
use crate::plan::{PlanSummary, plans_dir};
use crate::task_parser::{TaskDef, TasksFile};

impl TuiState {
    pub fn update_from_snapshot(&mut self, data: &DashboardData) {
        let executor_summary = data.executor_summary();
        if executor_summary.orchestrator_state.is_empty() {
            // Try to derive orchestrator state from status.json when the
            // executor summary is empty (standalone TUI without a live hub).
            if !self.workdir.as_os_str().is_empty() {
                let state_dir = self.workdir.join(".roko").join("state");
                let runner_read = crate::runner::status_file::read_runner_status(&state_dir);
                match &runner_read {
                    crate::runner::status_file::RunnerStatusRead::Live(s) => {
                        self.orchestrator_state = if s.current_phase.is_empty() {
                            s.phase.clone()
                        } else {
                            s.current_phase.clone()
                        };
                    }
                    // A finished run shows how it ended (bug-f7f3bb).
                    crate::runner::status_file::RunnerStatusRead::Finished(s) => {
                        self.orchestrator_state = s.phase.clone();
                    }
                    crate::runner::status_file::RunnerStatusRead::Stale(_) => {
                        self.orchestrator_state = String::from("stale/offline");
                    }
                    crate::runner::status_file::RunnerStatusRead::Missing => {
                        if self.orchestrator_state.is_empty() {
                            self.orchestrator_state = String::from("idle");
                        }
                    }
                }
            } else if self.orchestrator_state.is_empty() {
                self.orchestrator_state = String::from("idle");
            }
        } else {
            self.orchestrator_state = executor_summary.orchestrator_state;
        }
        self.current_iteration = executor_summary.current_iteration;
        self.current_phase = executor_summary.current_phase;
        let latest_events = latest_agent_events(&data.efficiency_events);
        let latest_route_metrics = latest_route_metrics(&data.efficiency_events, data);
        self.experiment_winners = data.experiment_winners.clone();
        self.gate_trends.clear();
        self.gate_recent_failures.clear();

        let mut tasks_by_plan: HashMap<String, Vec<TaskEntry>> = HashMap::new();
        for task in &data.active_tasks {
            tasks_by_plan
                .entry(task.plan_id.clone())
                .or_default()
                .push(TaskEntry {
                    id: task.task_id.clone(),
                    name: task.latest_gate.as_ref().map_or_else(
                        || task.task_id.clone(),
                        |gate| format!("{} ({gate})", task.task_id),
                    ),
                    status: TaskStatus::from(task.status.as_str()),
                    agent_id: task.assigned_agents.first().cloned(),
                    ..Default::default()
                });
        }

        if let Some(exec) = &data.current_plan_execution {
            let entry = tasks_by_plan.entry(exec.plan_id.clone()).or_default();
            if entry.is_empty() {
                entry.extend(exec.tasks.iter().map(|task| TaskEntry {
                    id: task.task_id.clone(),
                    name: if task.title.is_empty() {
                        task.task_id.clone()
                    } else {
                        task.title.clone()
                    },
                    status: TaskStatus::from(task.phase.as_str()),
                    agent_id: None,
                    ..Default::default()
                }));
            }
        }

        // Plans
        let expanded_by_plan: HashMap<String, bool> = self
            .plans
            .iter()
            .map(|plan| (plan.id.clone(), plan.expanded))
            .collect();
        // P5.5: preserve existing started_at so the timer is not reset on
        // every data refresh for plans that are already active.
        let existing_plan_started_at: HashMap<String, Instant> = self
            .plans
            .iter()
            .filter_map(|plan| plan.started_at.map(|t| (plan.id.clone(), t)))
            .collect();
        let plan_waves = derive_plan_waves(data.root(), &data.plans);
        let plan_snapshots = data.plan_task_snapshots();
        self.plans = data
            .plans
            .iter()
            .map(|p| {
                let completed = p.completed;
                let snapshot = plan_snapshots.get(&p.id);
                let phase = snapshot.map(|plan| plan.phase.clone()).unwrap_or_else(|| {
                    if completed {
                        String::from("done")
                    } else {
                        String::from("pending")
                    }
                });
                let status = PlanPhase::from(phase.as_str());
                let tasks_total = snapshot
                    .map(|plan| plan.tasks.len())
                    .filter(|count| *count > 0)
                    .unwrap_or(p.task_count);
                let (tasks_done, derived_failed) = plan_task_counts(p, snapshot, tasks_total);
                let tasks_failed = snapshot
                    .map(|plan| {
                        usize::try_from(plan.failed_count)
                            .unwrap_or(tasks_total)
                            .min(tasks_total)
                    })
                    .unwrap_or(derived_failed);
                let elapsed_secs = snapshot
                    .map(|plan| {
                        if plan.elapsed_ms > 0 {
                            plan.elapsed_ms as f64 / 1000.0
                        } else {
                            plan.elapsed_secs
                        }
                    })
                    .unwrap_or(0.0);
                PlanEntry {
                    id: p.id.clone(),
                    name: p.title.clone(),
                    status,
                    active: snapshot.map(|plan| plan.active).unwrap_or(!completed),
                    phase,
                    tasks_total,
                    tasks_done,
                    tasks_failed,
                    elapsed_secs,
                    wave: snapshot
                        .map(|plan| usize::try_from(plan.wave).unwrap_or_default())
                        .or_else(|| plan_waves.get(&p.id).copied()),
                    expanded: expanded_by_plan.get(&p.id).copied().unwrap_or(false),
                    tasks: snapshot
                        .map(|plan| {
                            plan.tasks
                                .iter()
                                .map(|task| TaskEntry {
                                    id: task.id.clone(),
                                    name: task.title.clone(),
                                    status: TaskStatus::from(task.status.as_str()),
                                    agent_id: task.agent_id.clone(),
                                    depends_on: task.dependencies.clone(),
                                    acceptance_text: task.acceptance_text.clone(),
                                    verify_command: task.verify_command.clone(),
                                    started_at: task.started_at.clone(),
                                    files: task.files.clone(),
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                    branch: Some(crate::orchestrator::worktree::format_branch_name(&p.id)),
                    worktree_path: None,
                    last_commit: None,
                    files_modified: None,
                    insertions: None,
                    deletions: None,
                    // P5.5: reuse existing Instant when the plan is already
                    // tracked as active; only create a new one on first
                    // transition into the active state.
                    started_at: if snapshot.map(|plan| plan.active).unwrap_or(!completed) {
                        Some(
                            existing_plan_started_at
                                .get(&p.id)
                                .copied()
                                .unwrap_or_else(Instant::now),
                        )
                    } else {
                        None
                    },
                }
            })
            .collect();

        // Agents
        self.agents.clear();
        let mut agent_ids = data
            .agents
            .iter()
            .map(|agent| agent.id.clone())
            .collect::<Vec<_>>();
        if agent_ids.is_empty() {
            agent_ids.extend(latest_events.keys().cloned());
            agent_ids.sort();
            agent_ids.dedup();
        }
        for agent_id in agent_ids {
            let summary = data.agents.iter().find(|agent| agent.id == agent_id);
            let latest = latest_events.get(&agent_id);
            let label = summary
                .map(|agent| agent.label.clone())
                .filter(|label| !label.is_empty())
                .or_else(|| latest.map(|event| event.role.clone()))
                .unwrap_or_else(|| agent_id.clone());
            let status = summary
                .map(|agent| agent.status.clone())
                .or_else(|| latest.map(|event| event.status.clone()))
                .unwrap_or_else(|| "idle".to_string());
            let current_plan = summary
                .and_then(|agent| agent.plan_id.clone())
                .or_else(|| latest.and_then(|event| event.plan_id.clone()))
                .unwrap_or_default();
            let current_task = latest
                .map(|event| event.task_id.clone())
                .unwrap_or_default();
            let is_active = plan_is_active(&status);
            self.agents.push(AgentRow {
                id: agent_id.clone(),
                active: is_active,
                status: AgentStatus::from(status.as_str()),
                role: label.clone(),
                model: latest.map(|event| event.model.clone()).unwrap_or_default(),
                input_tokens: latest.map_or(0, |event| event.input_tokens),
                output_tokens: latest.map_or(0, |event| event.output_tokens),
                context_limit: model_context_limit(latest.map(|e| e.model.as_str()).unwrap_or("")),
                current_plan: current_plan.clone(),
                current_task: current_task.clone(),
                attempt: 0,
                spawned_at_ms: 0,
                last_event_at_ms: 0,
                output_lines: Vec::new(),
                last_output_line: String::new(),
            });
        }

        // Populate agent output from episodes (Task 2)
        for episode in data.episodes() {
            if let Some(row) = self.agents.iter_mut().find(|a| a.id == episode.agent_id) {
                let output_text = extract_episode_output(episode);
                if !output_text.is_empty() {
                    row.output_lines = output_text.lines().map(String::from).collect();
                }
                if let Some(last_line) = output_text.lines().last() {
                    row.last_output_line = last_line.to_string();
                }
                // Also populate model and task from episode
                if !episode.model.is_empty() {
                    row.model = episode.model.clone();
                    row.context_limit = model_context_limit(&episode.model);
                }
                if !episode.task_id.is_empty() {
                    row.current_task = episode.task_id.clone();
                }
                row.input_tokens = row.input_tokens.max(episode.usage.input_tokens);
                row.output_tokens = row.output_tokens.max(episode.usage.output_tokens);
            }
        }

        // Supplement agent output from task-outputs files (Task 2 continued)
        for (task_id, lines) in data.task_outputs() {
            // Find agent working on this task and add output if empty
            if let Some(row) = self.agents.iter_mut().find(|a| a.current_task == *task_id) {
                if row.output_lines.is_empty() && !lines.is_empty() {
                    row.output_lines = lines.clone();
                }
                if row.last_output_line.is_empty() {
                    if let Some(last) = lines.last() {
                        row.last_output_line = last.clone();
                    }
                }
            }
        }
        self.route_metrics = self
            .agents
            .iter()
            .map(|agent| {
                let metrics = latest_route_metrics
                    .get(&agent.id)
                    .cloned()
                    .map(|mut metrics| {
                        if metrics.model.is_empty() && !agent.model.is_empty() {
                            metrics.model = agent.model.clone();
                        }
                        if metrics.context_limit == 0 {
                            metrics.context_limit = agent.context_limit.max(1);
                        }
                        metrics
                    })
                    .unwrap_or_else(|| fallback_route_metrics_for_agent(agent));
                (agent.id.clone(), metrics)
            })
            .collect();
        self.prune_agent_output_cache();
        self.prune_agent_streams();

        self.cost_dollars = data.efficiency.total_cost_usd;
        self.max_plan_budget_usd = f64::from(data.budget.max_plan_usd);
        self.cumulative_input_tokens = data.efficiency.total_input_tokens;
        self.cumulative_output_tokens = data.efficiency.total_output_tokens;
        self.token_total = self.cumulative_input_tokens + self.cumulative_output_tokens;
        // Snapshot events arrive much more often than token/cost events. Do
        // not feed zero-delta UI updates into the EMA between real turns.
        if self.last_rate_sample_at.is_none()
            || self.token_total != self.last_token_total_sample
            || self.cost_dollars != self.last_cost_dollars_sample
        {
            self.update_efficiency_rates();
        }
        self.gate_results = data
            .gate_results
            .iter()
            .map(GateResultEntry::from)
            .collect();
        sum_costs(data, &mut self.cost_per_plan, &mut self.cost_per_task);
        self.task_budget_usd.clear();
        for (plan_id, snapshot) in &plan_snapshots {
            for task in &snapshot.tasks {
                self.task_budget_usd.insert(
                    format!("{plan_id}:{}", task.id),
                    data.budget
                        .task_limit_usd(&task.tier, task.model_hint.as_deref()),
                );
            }
        }

        self.phase_pipeline = build_phase_pipeline(&data.active_tasks);

        // Populate phase elapsed times from episodes (Task 7)
        populate_phase_elapsed(&mut self.phase_pipeline, data.episodes());

        // Build current_task_checklist from active_tasks + task-trackers (Task 3)
        self.current_task_checklist = build_task_checklist_from_execution(data);

        // Populate task detail fields (P5.1-P5.2) from plan TaskEntry data.
        populate_task_row_details(&mut self.current_task_checklist, &self.plans);

        self.execution_waves = rebuild_execution_waves(&self.plans, &self.execution_waves);

        // Sync filter alias
        self.filter = self.filter_text.clone();

        // Clamp selections
        if !self.plans.is_empty() {
            if self.selected_plan_idx >= self.plans.len() {
                self.selected_plan_idx = self.plans.len() - 1;
            }
            if self.current_plan_idx >= self.plans.len() {
                self.current_plan_idx = self.plans.len() - 1;
            }
        } else {
            self.selected_plan_idx = 0;
            self.current_plan_idx = 0;
        }

        if !self.agents.is_empty() && self.selected_agent >= self.agents.len() {
            self.selected_agent = self.agents.len() - 1;
        }
        self.selected_agent_tab = self.selected_agent_tab.min(6);
        self.selected_wave_idx =
            clamp_selected_wave_idx(self.selected_wave_idx, self.execution_waves.len());

        let token_samples = build_token_samples(data);
        self.token_history = token_samples
            .iter()
            .map(|(agent_id, samples)| {
                (
                    agent_id.clone(),
                    samples.iter().map(|(_, total)| *total).collect(),
                )
            })
            .collect();
        self.token_rate = self
            .agents
            .get(self.selected_agent)
            .and_then(|agent| token_samples.get(&agent.id))
            .map_or_else(
                || compute_token_rate(&data.efficiency_events),
                compute_windowed_token_rate,
            );

        // -- view data (migrated from DashboardData) --
        self.workdir = data.root().to_path_buf();
        self.efficiency_summary = data.efficiency.clone();
        self.efficiency_events = data.efficiency_events.clone();
        self.rev_efficiency.bump();
        self.efficiency_trend = data.efficiency_trend.clone();
        self.cfactor_trend_buckets = data.cfactor_trend.clone();
        self.cascade_router = data.cascade_router.clone();
        self.recent_signals = data.recent_signals.clone();
        self.rev_signals.bump();
        self.current_plan_execution = data.current_plan_execution.clone();
        self.conductor_alerts = data.conductor_alerts.clone();
        self.cfactor = data.cfactor.clone();
        self.gate_results_page = data.gate_results_page.clone();
        self.rev_gate_results.bump();
        self.experiments = data.experiments.clone();
        self.task_output_tails = data.task_outputs().clone();
        self.git_diff = data.git_diff.clone();
        self.plan_summaries = data.plans.clone();
        self.agent_summaries = data.agents.clone();
        self.active_task_summaries = data.active_tasks.clone();
        self.gate_result_summaries = data.gate_results.clone();
        self.episodes_cache = data.episodes().to_vec();
        self.rev_episodes.bump();
        self.refresh_cached_unified_log();

        // -- knowledge browse --
        self.knowledge_entries = data.knowledge_entries.clone();

        // -- network stats --
        self.agents_online = if data.agents.is_empty() {
            count_online_from_files(data.root())
        } else {
            data.agents
                .iter()
                .filter(|agent| is_online_agent_status(&agent.status))
                .count()
        };
        self.gate_pass_rate = gate_pass_rate(&data.gate_results);

        // -- marketplace / atelier --
        self.marketplace_jobs = data.marketplace_jobs.clone();
        self.atelier_prds = data.atelier_prds.clone();
        self.atelier_tasks_by_slug = data.atelier_tasks_by_slug.clone();

        // Clamp marketplace/atelier selections to valid range after data refresh.
        if self.marketplace_jobs.is_empty() {
            self.marketplace_selected_job = 0;
        } else if self.marketplace_selected_job >= self.marketplace_jobs.len() {
            self.marketplace_selected_job = self.marketplace_jobs.len() - 1;
        }
        if self.atelier_prds.is_empty() {
            self.atelier_selected_prd = 0;
        } else if self.atelier_selected_prd >= self.atelier_prds.len() {
            self.atelier_selected_prd = self.atelier_prds.len() - 1;
        }

        // -- provider statuses (F11) --
        self.provider_statuses = populate_provider_statuses(&self.workdir, &self.efficiency_events);
        if !self.provider_statuses.is_empty()
            && self.providers_selected >= self.provider_statuses.len()
        {
            self.providers_selected = self.provider_statuses.len() - 1;
        }
    }

    /// Populate state from a connected-mode `DashboardSnapshot`.
    ///
    /// This mirrors the live state published by `StateHub` without touching
    /// navigation or scroll state. Learning data arrives through two channels:
    /// pushed snapshot payloads (efficiency trend buckets, cascade-router and
    /// gate-threshold JSON) are parsed into the typed view structs here, and
    /// per-event payloads the snapshot cannot carry (efficiency events,
    /// experiment store) are tailed from the local `.roko/learn/` files by
    /// [`Self::sync_connected_learning_files`].
    pub fn update_from_dashboard_snapshot(&mut self, snap: &roko_core::DashboardSnapshot) {
        // One run clock spans the whole announced plan set, including the
        // gaps between plans when nothing is active.
        self.plan_set_running = snap.plan_set.is_some() && !snap.plan_set_complete();
        if let Some(duration_ms) = snap.run_duration_ms {
            self.run_duration_secs = Some(duration_ms as f64 / 1_000.0);
            self.run_started = None;
        } else if snap.stats.plans_active > 0 || self.plan_set_running {
            self.run_duration_secs = None;
            if self.run_started.is_none() {
                self.run_started = Some(
                    snap.plan_set
                        .as_ref()
                        .and_then(|set| instant_at_unix_ms(set.loaded_at_ms))
                        .unwrap_or_else(Instant::now),
                );
            }
        } else if snap.plan_set.is_some() {
            // The whole set finished: freeze the clock at its final value.
            if let Some(started) = self.run_started.take() {
                self.run_duration_secs = Some(started.elapsed().as_secs_f64());
            }
        } else {
            self.run_duration_secs = None;
            self.run_started = None;
        }
        let prev_selected_plan_id = self
            .plans
            .get(self.selected_plan_idx)
            .map(|plan| plan.id.clone());
        let prev_current_plan_id = self
            .plans
            .get(self.current_plan_idx)
            .map(|plan| plan.id.clone());
        let prev_selected_agent_id = self
            .agents
            .get(self.selected_agent)
            .map(|agent| agent.id.clone());
        let prev_plan_order: HashMap<String, usize> = self
            .plans
            .iter()
            .enumerate()
            .map(|(idx, plan)| (plan.id.clone(), idx))
            .collect();
        let prev_agent_order: HashMap<String, usize> = self
            .agents
            .iter()
            .enumerate()
            .map(|(idx, agent)| (agent.id.clone(), idx))
            .collect();
        let prev_plan_expanded: HashMap<String, bool> = self
            .plans
            .iter()
            .map(|plan| (plan.id.clone(), plan.expanded))
            .collect();
        let prev_plan_elapsed: HashMap<String, f64> = self
            .plans
            .iter()
            .map(|plan| (plan.id.clone(), plan.elapsed_secs))
            .collect();
        // P5.5: preserve per-plan started_at so the live elapsed timer is not
        // reset to Instant::now() on every snapshot update for active plans.
        let prev_plan_started_at: HashMap<String, Instant> = self
            .plans
            .iter()
            .filter_map(|plan| plan.started_at.map(|t| (plan.id.clone(), t)))
            .collect();
        let prev_plan_wave: HashMap<String, Option<usize>> = self
            .plans
            .iter()
            .map(|plan| (plan.id.clone(), plan.wave))
            .collect();
        let prev_task_elapsed: HashMap<String, f64> = self
            .current_task_checklist
            .iter()
            .map(|task| (task.id.clone(), task.elapsed_secs))
            .collect();
        let prev_agent_rows: HashMap<String, AgentRow> = self
            .agents
            .iter()
            .cloned()
            .map(|agent| (agent.id.clone(), agent))
            .collect();
        let prev_route_metrics = self.route_metrics.clone();
        let mut snapshot_tasks: Vec<&roko_core::dashboard_snapshot::TaskState> =
            snap.tasks.values().collect();
        snapshot_tasks.sort_by(|lhs, rhs| {
            lhs.plan_id
                .cmp(&rhs.plan_id)
                .then_with(|| lhs.task_id.cmp(&rhs.task_id))
        });

        let mut tasks_by_plan: HashMap<String, Vec<TaskEntry>> = HashMap::new();
        self.current_task_checklist = snapshot_tasks
            .iter()
            .map(|task| {
                let status = snapshot_task_status(task);
                tasks_by_plan
                    .entry(task.plan_id.clone())
                    .or_default()
                    .push(TaskEntry {
                        id: task.task_id.clone(),
                        name: if task.title.is_empty() {
                            task.task_id.clone()
                        } else {
                            task.title.clone()
                        },
                        status,
                        agent_id: None,
                        ..Default::default()
                    });
                TaskRow {
                    id: task.task_id.clone(),
                    title: if task.title.is_empty() {
                        task.task_id.clone()
                    } else {
                        task.title.clone()
                    },
                    status,
                    elapsed_secs: prev_task_elapsed.get(&task.task_id).copied().unwrap_or(0.0),
                    depends_on: Vec::new(),
                    acceptance_text: None,
                    verify_command: None,
                    files: Vec::new(),
                }
            })
            .collect();

        let mut plan_ids: Vec<String> = snap.plans.keys().cloned().collect();
        // Announced plan-set members keep their execution order.
        let plan_set_position = |plan_id: &str| {
            snap.plan_set
                .as_ref()
                .and_then(|set| set.position(plan_id))
                .unwrap_or(usize::MAX)
        };
        plan_ids.sort_by(|lhs, rhs| {
            plan_set_position(lhs)
                .cmp(&plan_set_position(rhs))
                .then_with(|| {
                    prev_plan_order
                        .get(lhs)
                        .copied()
                        .unwrap_or(usize::MAX)
                        .cmp(&prev_plan_order.get(rhs).copied().unwrap_or(usize::MAX))
                })
                .then_with(|| lhs.cmp(rhs))
        });
        // Announced plan-set members group by their dependency wave, so the
        // plan tree shows which plans can run side by side.
        let plan_set_wave = |plan_id: &str| {
            snap.plan_set.as_ref().and_then(|set| {
                set.plans
                    .iter()
                    .find(|entry| entry.plan_id == plan_id)
                    .map(|entry| entry.wave)
            })
        };

        self.plans = plan_ids
            .iter()
            .map(|plan_id| {
                let plan = &snap.plans[plan_id];
                let tasks = tasks_by_plan.remove(plan_id).unwrap_or_default();
                let tasks_total = plan.tasks_total.max(tasks.len());
                PlanEntry {
                    id: plan.plan_id.clone(),
                    name: plan.plan_id.clone(),
                    status: snapshot_plan_status(plan),
                    active: plan.active,
                    phase: snapshot_plan_phase(plan),
                    tasks_total,
                    tasks_done: plan.tasks_done.min(tasks_total),
                    tasks_failed: plan.tasks_failed.min(tasks_total),
                    elapsed_secs: prev_plan_elapsed.get(plan_id).copied().unwrap_or(0.0),
                    wave: plan_set_wave(plan_id)
                        .or_else(|| prev_plan_wave.get(plan_id).copied().flatten()),
                    expanded: prev_plan_expanded.get(plan_id).copied().unwrap_or(false),
                    tasks,
                    branch: Some(crate::orchestrator::worktree::format_branch_name(plan_id)),
                    worktree_path: None,
                    last_commit: None,
                    files_modified: None,
                    insertions: None,
                    deletions: None,
                    // P5.5: reuse the existing Instant if the plan is already
                    // tracked and active; only create a new one when the plan
                    // transitions into the active state for the first time.
                    started_at: if plan.active {
                        Some(
                            prev_plan_started_at
                                .get(plan_id)
                                .copied()
                                .unwrap_or_else(Instant::now),
                        )
                    } else {
                        None
                    },
                }
            })
            .collect();

        let mut orphaned_plan_ids: Vec<String> = tasks_by_plan.keys().cloned().collect();
        orphaned_plan_ids.sort();
        for plan_id in orphaned_plan_ids {
            let tasks = tasks_by_plan.remove(&plan_id).unwrap_or_default();
            let tasks_total = tasks.len();
            let tasks_done = tasks.iter().filter(|task| task.status.is_done()).count();
            let tasks_failed = tasks.iter().filter(|task| task.status.is_failed()).count();
            let active = tasks.iter().any(|task| task.status.is_active());
            self.plans.push(PlanEntry {
                id: plan_id.clone(),
                name: plan_id.clone(),
                status: if active {
                    PlanPhase::Active
                } else if tasks_failed > 0 {
                    PlanPhase::Failed
                } else if tasks_total > 0 && tasks_done == tasks_total {
                    PlanPhase::Done
                } else {
                    PlanPhase::Pending
                },
                active,
                phase: if active {
                    String::from("active")
                } else if tasks_failed > 0 {
                    String::from("failed")
                } else if tasks_total > 0 && tasks_done == tasks_total {
                    String::from("completed")
                } else {
                    String::from("pending")
                },
                tasks_total,
                tasks_done,
                tasks_failed,
                elapsed_secs: prev_plan_elapsed.get(&plan_id).copied().unwrap_or(0.0),
                wave: prev_plan_wave.get(&plan_id).copied().flatten(),
                expanded: prev_plan_expanded.get(&plan_id).copied().unwrap_or(false),
                tasks,
                branch: Some(crate::orchestrator::worktree::format_branch_name(&plan_id)),
                worktree_path: None,
                last_commit: None,
                files_modified: None,
                insertions: None,
                deletions: None,
                // P5.5: preserve existing Instant for plans already tracked.
                started_at: if active {
                    Some(
                        prev_plan_started_at
                            .get(&plan_id)
                            .copied()
                            .unwrap_or_else(Instant::now),
                    )
                } else {
                    None
                },
            });
        }

        // Populate task detail fields (P5.1-P5.2) from plan TaskEntry data.
        populate_task_row_details(&mut self.current_task_checklist, &self.plans);

        let mut agent_ids: Vec<String> = snap.agents.keys().cloned().collect();
        agent_ids.sort_by(|lhs, rhs| {
            prev_agent_order
                .get(lhs)
                .copied()
                .unwrap_or(usize::MAX)
                .cmp(&prev_agent_order.get(rhs).copied().unwrap_or(usize::MAX))
                .then_with(|| lhs.cmp(rhs))
        });

        self.agents = agent_ids
            .iter()
            .map(|agent_id| {
                let agent = &snap.agents[agent_id];
                let prev_row = prev_agent_rows.get(agent_id);
                let model = prev_row.map(|row| row.model.clone()).unwrap_or_default();
                let context_limit = prev_row
                    .map(|row| row.context_limit)
                    .filter(|limit| *limit > 0)
                    .unwrap_or_else(|| model_context_limit(&model));

                // Prefer snapshot values for model/tokens/cost/task/plan, fall
                // back to previously cached values.
                let snap_model = if agent.model.is_empty() {
                    model
                } else {
                    agent.model.clone()
                };
                let snap_input_tokens = if agent.input_tokens > 0 {
                    agent.input_tokens
                } else {
                    prev_row.map(|row| row.input_tokens).unwrap_or(0)
                };
                let snap_output_tokens = if agent.output_tokens > 0 {
                    agent.output_tokens
                } else {
                    prev_row
                        .map(|row| row.output_tokens)
                        .unwrap_or(0)
                        .max(agent.output_bytes as u64)
                };
                let snap_current_plan = if agent.current_plan.is_empty() {
                    prev_row
                        .map(|row| row.current_plan.clone())
                        .unwrap_or_default()
                } else {
                    agent.current_plan.clone()
                };
                let snap_current_task = if agent.current_task.is_empty() {
                    prev_row
                        .map(|row| row.current_task.clone())
                        .unwrap_or_default()
                } else {
                    agent.current_task.clone()
                };

                // `task_outputs` is an authoritative ring, not an event delta.
                // Re-appending it on every snapshot duplicates the same lines
                // forever, while retaining an absent or explicitly empty ring
                // leaks output across task transitions. Replace the cached
                // value on every connected snapshot, including with empty.
                let output_lines = snap
                    .task_outputs
                    .get(&snap_current_task)
                    .map(bounded_output_lines)
                    .unwrap_or_default();
                let last_output_line = output_lines.last().cloned().unwrap_or_default();

                let snap_attempt = if agent.attempt > 0 {
                    agent.attempt
                } else {
                    prev_row.map(|row| row.attempt).unwrap_or(0)
                };

                AgentRow {
                    id: agent.agent_id.clone(),
                    active: agent.active,
                    status: if agent.active {
                        AgentStatus::Active
                    } else {
                        AgentStatus::Idle
                    },
                    role: agent.role.clone(),
                    model: snap_model,
                    input_tokens: snap_input_tokens,
                    output_tokens: snap_output_tokens,
                    context_limit,
                    current_plan: snap_current_plan,
                    current_task: snap_current_task,
                    attempt: snap_attempt,
                    spawned_at_ms: agent.spawned_at_ms,
                    last_event_at_ms: agent.last_event_at_ms,
                    output_lines,
                    last_output_line,
                }
            })
            .collect();
        self.route_metrics = self
            .agents
            .iter()
            .map(|agent| {
                let metrics = prev_route_metrics
                    .get(&agent.id)
                    .cloned()
                    .map(|mut metrics| {
                        if metrics.model.is_empty() && !agent.model.is_empty() {
                            metrics.model = agent.model.clone();
                        }
                        metrics.context_used =
                            agent.input_tokens.saturating_add(agent.output_tokens);
                        if metrics.context_limit == 0 {
                            metrics.context_limit = agent.context_limit.max(1);
                        }
                        if metrics.tier.is_empty() {
                            metrics.tier = route_tier_label_for_model(&metrics.model).to_string();
                        }
                        metrics
                    })
                    .unwrap_or_else(|| fallback_route_metrics_for_agent(agent));
                (agent.id.clone(), metrics)
            })
            .collect();
        // Ingest agent output lines into the structured history (#367).
        // Only ingest when there are output_lines that haven't been seen yet,
        // deduplicating against the history's existing records.
        for agent in &self.agents {
            if !agent.output_lines.is_empty() && self.agent_output_history.len(&agent.id) == 0 {
                self.agent_output_history
                    .ingest_lines(&agent.id, &agent.output_lines, "assistant");
            }
        }

        self.prune_agent_output_cache();
        self.prune_agent_streams();

        self.gate_results = snap
            .gates
            .iter()
            .map(|gate_result| GateResultEntry {
                gate: gate_result.gate.clone(),
                plan_id: gate_result.plan_id.clone(),
                task_id: gate_result.task_id.clone(),
                passed: gate_result.passed,
                output: if gate_result.task_id.is_empty() {
                    String::new()
                } else {
                    format!("task {}", gate_result.task_id)
                },
            })
            .collect();
        self.rev_gate_results.bump();
        self.current_gate_rung = snap.active_gate_rung.as_ref().map(|rung| {
            let elapsed_ms = current_epoch_ms().saturating_sub(rung.started_at_ms);
            let started_at = Instant::now()
                .checked_sub(Duration::from_millis(elapsed_ms))
                .unwrap_or_else(Instant::now);
            (rung.rung_name.clone(), started_at)
        });
        self.diagnoses = snap.diagnoses.iter().cloned().collect();
        // Nothing publishes `ExperimentWinnersUpdated` today, so an empty push
        // must not clobber winners synced from the local experiment store.
        if !snap.experiment_winners.is_empty() {
            self.experiment_winners = snap.experiment_winners.clone();
        }
        self.gate_trends = snap.gate_trends.clone();
        self.gate_recent_failures = snap.gate_recent_failures.clone();
        self.task_gate_outputs = snap.task_gate_outputs.iter().cloned().collect();
        self.affect = snap.affect.clone();
        self.critical_path_eta_minutes = snap.critical_path_eta_minutes.map(|v| v as f64);
        if !snap.agent_topology.is_empty() {
            self.agent_topology = snap.agent_topology.clone();
            self.agent_topology_status = AgentTopologyStatus::Ready;
        } else if !snap.agents.is_empty() {
            // Derive a minimal topology from active agents when the
            // explicit topology is empty (item 41).  Each agent becomes a
            // node; agents sharing a plan_id get edges between them.
            let derived = derive_topology_from_agents(&snap.agents);
            if !derived.is_empty() {
                self.agent_topology = derived;
                self.agent_topology_status = AgentTopologyStatus::Ready;
            }
        } else if !matches!(self.agent_topology_status, AgentTopologyStatus::Ready) {
            self.agent_topology = snap.agent_topology.clone();
        }

        // --- Event log from snapshot ---
        self.event_log = snap.event_log.iter().cloned().collect();
        self.rev_event_log.bump();

        // --- Learning data (pushed JSON parsed into the typed view structs) ---
        // Re-parse only on change; the stored strings double as change detectors.
        if snap.cascade_router_json != self.cascade_router_json {
            self.cascade_router_json
                .clone_from(&snap.cascade_router_json);
            if !self.cascade_router_json.is_empty() {
                match serde_json::from_str::<CascadeRouterState>(&self.cascade_router_json) {
                    Ok(parsed) => self.cascade_router = parsed,
                    Err(error) => tracing::warn!(
                        error = %error,
                        "failed to parse pushed cascade router snapshot"
                    ),
                }
            }
        }
        if snap.gate_thresholds_json != self.gate_thresholds_json {
            self.gate_thresholds_json
                .clone_from(&snap.gate_thresholds_json);
            if !self.gate_thresholds_json.is_empty() {
                match serde_json::from_str::<roko_gate::adaptive_threshold::AdaptiveThresholds>(
                    &self.gate_thresholds_json,
                ) {
                    Ok(thresholds) => {
                        self.gate_results_page.threshold_rows =
                            super::super::dashboard::gate_threshold_rows(&thresholds);
                    }
                    Err(error) => tracing::warn!(
                        error = %error,
                        "failed to parse pushed gate thresholds snapshot"
                    ),
                }
            }
        }

        // --- Token/cost aggregation across agents ---
        self.cumulative_input_tokens = self.agents.iter().map(|a| a.input_tokens).sum();
        self.cumulative_output_tokens = self.agents.iter().map(|a| a.output_tokens).sum();
        self.token_total = self.cumulative_input_tokens + self.cumulative_output_tokens;
        if snap.stats.cost_usd_total > 0.0 {
            self.cost_dollars = snap.stats.cost_usd_total;
        }
        // Connected snapshots carry token deltas in a bounded event ring.
        // Convert them into a cumulative series aligned to the current total.
        let recent_token_total = snap.token_event_ring.iter().copied().sum::<u64>();
        let mut running_total = self.token_total.saturating_sub(recent_token_total);
        let connected_history = snap
            .token_event_ring
            .iter()
            .map(|delta| {
                running_total = running_total.saturating_add(*delta);
                running_total
            })
            .collect::<VecDeque<_>>();
        self.token_history.clear();
        if !connected_history.is_empty() {
            self.token_history
                .insert("connected".to_string(), connected_history);
        } else if self.token_total > 0 {
            self.token_history
                .insert("connected".to_string(), VecDeque::from([self.token_total]));
        }
        if self.last_rate_sample_at.is_none()
            || self.token_total != self.last_token_total_sample
            || self.cost_dollars != self.last_cost_dollars_sample
        {
            self.update_efficiency_rates();
        }

        // -- network stats --
        self.agents_online = snap.agents.values().filter(|agent| agent.active).count();
        self.gate_pass_rate = snapshot_gate_pass_rate(&snap.gates);

        // Synthesize plan_summaries from snapshot-built plans so the F2 left
        // panel works in approval mode (where DashboardData is never loaded).
        // The snapshot carries no plan set, so each plan keeps the group disk
        // discovery gave it: from the disk-loaded summaries when there are
        // any, else from one workspace scan per unseen plan id.
        let discovered_groups: HashMap<String, String> = self
            .plan_summaries
            .iter()
            .filter_map(|summary| Some((summary.id.clone(), summary.group.clone()?)))
            .collect();
        if !self.workdir.as_os_str().is_empty()
            && self
                .plans
                .iter()
                .any(|plan| !self.plan_groups.contains_key(&plan.id))
        {
            let plan_dirs = crate::plan::plan_dirs_by_id(&self.workdir);
            for plan in &self.plans {
                let group = plan_dirs.get(&plan.id).and_then(|dir| dir.group.clone());
                self.plan_groups.insert(plan.id.clone(), group);
            }
        }
        self.plan_summaries = self
            .plans
            .iter()
            .map(|plan| PlanSummary {
                id: plan.id.clone(),
                title: plan.name.clone(),
                task_count: plan.tasks_total,
                tasks_done: plan.tasks_done,
                tasks_failed: plan.tasks_failed,
                completed: plan.status.is_done(),
                status: plan.phase.clone(),
                superseded_by: None,
                old_format: false,
                last_error: None,
                group: discovered_groups
                    .get(&plan.id)
                    .cloned()
                    .or_else(|| self.plan_groups.get(&plan.id).cloned().flatten()),
            })
            .collect();

        // Synthesize agent_summaries from snapshot-built agents so the F3 left
        // panel works in approval mode.
        self.agent_summaries = self
            .agents
            .iter()
            .map(|agent| AgentSummary {
                id: agent.id.clone(),
                label: format!("{} ({})", agent.role, agent.model),
                plan_id: Some(agent.current_plan.clone()).filter(|s| !s.is_empty()),
                status: if agent.active {
                    "running".into()
                } else {
                    "idle".into()
                },
            })
            .collect();

        // Synthesize gate_result_summaries from snapshot gates so the F2 right
        // panel shows gate verdicts in approval mode.
        self.gate_result_summaries = snap
            .gates
            .iter()
            .map(|g| GateResultSummary {
                plan_id: g.plan_id.clone(),
                gate_name: g.gate.clone(),
                passed: g.passed,
                rung: 0,
                duration_ms: 0,
                summary: if g.task_id.is_empty() {
                    String::new()
                } else {
                    format!("task {}", g.task_id)
                },
            })
            .collect();

        self.phase_pipeline = build_phase_pipeline_from_dashboard_snapshot(snap);
        self.execution_waves = rebuild_execution_waves(&self.plans, &self.execution_waves);

        self.orchestrator_state = if snap.stats.plans_active > 0 {
            String::from("running")
        } else if snap.stats.plans_failed > 0 {
            String::from("failed")
        } else if self.orchestrator_state.is_empty() {
            String::from("idle")
        } else {
            self.orchestrator_state.clone()
        };
        self.current_phase = self
            .plans
            .iter()
            .find(|plan| plan.active)
            .or_else(|| self.plans.first())
            .map(|plan| plan.phase.clone())
            .unwrap_or_default();
        self.filter = self.filter_text.clone();

        restore_selected_plan_idx(
            &self.plans,
            &mut self.selected_plan_idx,
            prev_selected_plan_id,
        );
        restore_selected_plan_idx(
            &self.plans,
            &mut self.current_plan_idx,
            prev_current_plan_id,
        );
        restore_selected_agent_idx(
            &self.agents,
            &mut self.selected_agent,
            prev_selected_agent_id,
        );
        self.selected_agent_tab = self.selected_agent_tab.min(6);
        self.selected_wave_idx =
            clamp_selected_wave_idx(self.selected_wave_idx, self.execution_waves.len());

        // The connected snapshot is authoritative. Rebuild instead of only
        // inserting so completed/removed tasks do not leak entries forever.
        self.task_output_tails = snap
            .task_outputs
            .iter()
            .map(|(task_id, lines)| (task_id.clone(), bounded_output_lines(lines)))
            .collect();

        // --- Marketplace / Atelier from snapshot ---
        if !snap.marketplace_jobs.is_empty() {
            self.marketplace_jobs = snap.marketplace_jobs.clone();
        }
        if !snap.atelier_prds.is_empty() {
            self.atelier_prds = snap.atelier_prds.clone();
            self.atelier_tasks_by_slug = snap.atelier_tasks.clone();
        }
        if self.marketplace_jobs.is_empty() {
            self.marketplace_selected_job = 0;
        } else if self.marketplace_selected_job >= self.marketplace_jobs.len() {
            self.marketplace_selected_job = self.marketplace_jobs.len() - 1;
        }
        if self.atelier_prds.is_empty() {
            self.atelier_selected_prd = 0;
        } else if self.atelier_selected_prd >= self.atelier_prds.len() {
            self.atelier_selected_prd = self.atelier_prds.len() - 1;
        }

        // --- Knowledge entries from snapshot ---
        if !snap.knowledge_entries.is_empty() {
            self.knowledge_entries = snap
                .knowledge_entries
                .iter()
                .map(|entry| KnowledgeBrowseEntry {
                    id: entry.id.clone(),
                    kind: entry.kind.clone(),
                    content_preview: entry.content_preview.clone(),
                    confidence: entry.confidence,
                    tier: entry.tier.clone(),
                    tags: entry.tags.clone(),
                    created_at: entry.created_at,
                    frozen: entry.frozen,
                })
                .collect();
        }

        // --- Efficiency trend → efficiency summary for bottom bar ---
        if !snap.efficiency_trend.is_empty() {
            let total_cost_cents: u64 =
                snap.efficiency_trend.iter().map(|b| b.cost_usd_cents).sum();
            let total_turns: u64 = snap.efficiency_trend.iter().map(|b| b.turns).sum();
            let total_in: u64 = snap.efficiency_trend.iter().map(|b| b.tokens_in).sum();
            let total_out: u64 = snap.efficiency_trend.iter().map(|b| b.tokens_out).sum();
            let avg_latency: f64 = if total_turns > 0 {
                snap.efficiency_trend
                    .iter()
                    .map(|b| b.latency_ms_avg * b.turns as f64)
                    .sum::<f64>()
                    / total_turns as f64
            } else {
                0.0
            };
            self.efficiency_summary = EfficiencySummary {
                event_count: total_turns as usize,
                total_cost_usd: total_cost_cents as f64 / 100.0,
                total_input_tokens: total_in,
                total_output_tokens: total_out,
                passed_count: 0,
                average_wall_time_ms: avg_latency,
            };
            // Keep the chart-facing trend in the learning crate's bucket type.
            self.efficiency_trend = snap
                .efficiency_trend
                .iter()
                .map(|bucket| roko_learn::aggregate::EfficiencyBucket {
                    start: bucket.start,
                    turns: bucket.turns,
                    tokens_in: bucket.tokens_in,
                    tokens_out: bucket.tokens_out,
                    cost_usd_cents: bucket.cost_usd_cents,
                    latency_ms_avg: bucket.latency_ms_avg,
                })
                .collect();
        }

        // --- C-factor trend buckets ---
        if !snap.cfactor_trend.is_empty() {
            self.cfactor_trend_buckets = snap
                .cfactor_trend
                .iter()
                .map(|b| roko_learn::aggregate::CFactorBucket {
                    start: b.start,
                    samples: b.samples,
                    avg: b.avg,
                    p50: b.p50,
                    p95: b.p95,
                })
                .collect();
        }

        // --- Gate output lines from snapshot ---
        // The connected snapshot is authoritative. Replacing with an empty
        // ring is important when a new rung starts, otherwise the previous
        // gate's output leaks into the active panel.
        let prev_gate_len = self.gate_output_lines.len();
        self.gate_output_lines = snap.gate_output_lines.clone();
        // Enforce bounded gate output.
        while self.gate_output_lines.len() > MAX_GATE_LINES {
            self.gate_output_lines.pop_front();
            self.eviction_counters.gate_output += 1;
        }
        if self.gate_output_lines.len() < prev_gate_len {
            // Lines replaced with a shorter ring — no eviction to count.
        }

        // --- Inbox items from snapshot ---
        // Replace the entire list; items are authoritative from the snapshot.
        // Sort by received_at_ms so the oldest pending items appear at the top.
        self.inbox_items = snap.inbox_items.values().cloned().collect();
        self.inbox_items.sort_by_key(|item| item.received_at_ms);

        // --- Learning files the snapshot cannot carry (per-event payloads) ---
        self.sync_connected_learning_files();

        // --- Provider statuses (F11) ---
        if !self.workdir.as_os_str().is_empty() {
            self.provider_statuses =
                populate_provider_statuses(&self.workdir, &self.efficiency_events);
            if !self.provider_statuses.is_empty()
                && self.providers_selected >= self.provider_statuses.len()
            {
                self.providers_selected = self.provider_statuses.len() - 1;
            }
        }

        self.refresh_cached_unified_log();
    }
}

// ---------------------------------------------------------------------------
// Free helper functions
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

pub(super) fn plan_task_counts(
    summary: &PlanSummary,
    snapshot: Option<&PlanTaskListSnapshot>,
    tasks_total: usize,
) -> (usize, usize) {
    if let Some(snapshot) = snapshot {
        let tasks_done = if snapshot.tasks.is_empty() {
            snapshot.tasks_done
        } else {
            snapshot
                .tasks
                .iter()
                .filter(|task| TaskStatus::from(task.status.as_str()).is_done())
                .count()
        };
        let tasks_failed = if snapshot.tasks.is_empty() {
            snapshot.tasks_failed
        } else {
            snapshot
                .tasks
                .iter()
                .filter(|task| TaskStatus::from(task.status.as_str()).is_failed())
                .count()
        };
        return (tasks_done.min(tasks_total), tasks_failed.min(tasks_total));
    }

    (
        summary.tasks_done.min(tasks_total),
        summary.tasks_failed.min(tasks_total),
    )
}

fn snapshot_plan_phase(plan: &roko_core::dashboard_snapshot::PlanState) -> String {
    if plan.phase.is_empty() {
        if plan.active {
            String::from("active")
        } else if plan.tasks_failed > 0 {
            String::from("failed")
        } else if plan.tasks_done >= plan.tasks_total && plan.tasks_total > 0 {
            String::from("completed")
        } else {
            String::from("pending")
        }
    } else {
        plan.phase.clone()
    }
}

fn snapshot_plan_status(plan: &roko_core::dashboard_snapshot::PlanState) -> PlanPhase {
    if plan.active {
        PlanPhase::Active
    } else if plan.tasks_failed > 0 {
        PlanPhase::Failed
    } else {
        match snapshot_plan_phase(plan).as_str() {
            "completed" | "done" => PlanPhase::Done,
            "failed" | "error" => PlanPhase::Failed,
            phase => PlanPhase::from(phase),
        }
    }
}

/// Map a past Unix-millisecond timestamp onto the monotonic clock.
fn instant_at_unix_ms(unix_ms: u64) -> Option<Instant> {
    if unix_ms == 0 {
        return None;
    }
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_millis();
    let elapsed_ms = u64::try_from(now_ms).ok()?.checked_sub(unix_ms)?;
    Instant::now().checked_sub(std::time::Duration::from_millis(elapsed_ms))
}

fn snapshot_task_status(task: &roko_core::dashboard_snapshot::TaskState) -> TaskStatus {
    use roko_core::dashboard_snapshot::{TaskOutcomeClass, classify_task_outcome};

    match task.outcome.as_deref().map(classify_task_outcome) {
        Some(TaskOutcomeClass::Failed) => TaskStatus::Failed,
        Some(TaskOutcomeClass::AcceptedWithFailures) => TaskStatus::AcceptedWithFailures,
        Some(TaskOutcomeClass::Passed) => TaskStatus::Done,
        None => TaskStatus::from(task.phase.as_str()),
    }
}

fn canonical_phase_index_for_snapshot_task(
    task: &roko_core::dashboard_snapshot::TaskState,
) -> Option<usize> {
    let phase = task.phase.trim().to_ascii_lowercase();
    if phase.is_empty() {
        return None;
    }

    if matches!(phase.as_str(), "completed" | "done") {
        return Some(CANONICAL_PHASES.len().saturating_sub(1));
    }

    CANONICAL_PHASES.iter().position(|candidate| {
        *candidate == phase
            || (*candidate == "compile-gate" && phase.contains("compile"))
            || (*candidate == "test-gate" && (phase.contains("test") || phase.contains("verif")))
            || (*candidate == "critic-review" && phase.contains("critic"))
    })
}

fn build_phase_pipeline_from_dashboard_snapshot(
    snap: &roko_core::dashboard_snapshot::DashboardSnapshot,
) -> Vec<PhaseStep> {
    #[derive(Clone, Copy, Default)]
    struct PhaseTaskCounts {
        total: usize,
        done: usize,
        active: usize,
        failed: usize,
    }

    let mut counts = vec![PhaseTaskCounts::default(); CANONICAL_PHASES.len()];

    for task in snap.tasks.values() {
        let Some(current_idx) = canonical_phase_index_for_snapshot_task(task) else {
            continue;
        };

        let failed = snapshot_task_status(task).is_failed();
        let done = snapshot_task_status(task).is_done();

        for (phase_idx, phase_counts) in counts.iter_mut().enumerate() {
            phase_counts.total += 1;

            if phase_idx < current_idx {
                phase_counts.done += 1;
            } else if phase_idx == current_idx {
                if failed {
                    phase_counts.failed += 1;
                } else if done {
                    phase_counts.done += 1;
                } else {
                    phase_counts.active += 1;
                }
            }
        }
    }

    CANONICAL_PHASES
        .iter()
        .enumerate()
        .map(|(idx, phase)| {
            let counts = counts[idx];
            let status = if counts.failed > 0 {
                PlanPhase::Failed
            } else if counts.active > 0 {
                PlanPhase::Active
            } else if counts.total > 0 && counts.done == counts.total {
                PlanPhase::Done
            } else {
                PlanPhase::Pending
            };
            let pct = if counts.total == 0 {
                0.0
            } else {
                (counts.done as f64 / counts.total as f64) * 100.0
            };

            PhaseStep {
                name: (*phase).to_string(),
                status,
                elapsed_secs: 0.0,
                pct,
            }
        })
        .collect()
}

fn restore_selected_plan_idx(
    plans: &[PlanEntry],
    selected: &mut usize,
    previous_id: Option<String>,
) {
    match previous_id {
        Some(previous_id) => {
            if let Some(idx) = plans.iter().position(|plan| plan.id == previous_id) {
                *selected = idx;
            } else if plans.is_empty() {
                *selected = 0;
            } else {
                *selected = (*selected).min(plans.len() - 1);
            }
        }
        None if plans.is_empty() => *selected = 0,
        None if *selected >= plans.len() => *selected = plans.len() - 1,
        None => {}
    }
}

fn restore_selected_agent_idx(
    agents: &[AgentRow],
    selected: &mut usize,
    previous_id: Option<String>,
) {
    match previous_id {
        Some(previous_id) => {
            if let Some(idx) = agents.iter().position(|agent| agent.id == previous_id) {
                *selected = idx;
            } else if agents.is_empty() {
                *selected = 0;
            } else {
                *selected = (*selected).min(agents.len() - 1);
            }
        }
        None if agents.is_empty() => *selected = 0,
        None if *selected >= agents.len() => *selected = agents.len() - 1,
        None => {}
    }
}

/// Build the canonical 9-phase pipeline, inferring status from active tasks.
pub(super) fn build_phase_pipeline(
    active_tasks: &[super::super::dashboard::TaskSummary],
) -> Vec<PhaseStep> {
    #[derive(Clone, Copy, Default)]
    struct PhaseTaskCounts {
        total: usize,
        done: usize,
        active: usize,
        failed: usize,
    }

    let mut counts = vec![PhaseTaskCounts::default(); CANONICAL_PHASES.len()];

    for task in active_tasks {
        let Some(current_idx) = canonical_phase_index_for_task(task) else {
            continue;
        };

        let failed = task_status_is_failed(&task.status);
        let done = task_status_is_done(&task.status);
        let active = !failed && !done;

        for (phase_idx, phase_counts) in counts.iter_mut().enumerate() {
            phase_counts.total += 1;

            if phase_idx < current_idx {
                phase_counts.done += 1;
                continue;
            }

            if phase_idx == current_idx {
                if failed {
                    phase_counts.failed += 1;
                } else if done {
                    phase_counts.done += 1;
                } else if active {
                    phase_counts.active += 1;
                }
            }
        }
    }

    CANONICAL_PHASES
        .iter()
        .enumerate()
        .map(|(idx, &name)| {
            let phase_counts = counts[idx];
            let pct = if phase_counts.total > 0 {
                (phase_counts.done as f64 / phase_counts.total as f64) * 100.0
            } else {
                0.0
            };
            let status = if phase_counts.failed > 0 {
                PlanPhase::Failed
            } else if phase_counts.total > 0 && phase_counts.done == phase_counts.total {
                PlanPhase::Done
            } else if phase_counts.active > 0 {
                PlanPhase::Active
            } else {
                PlanPhase::Pending
            };

            PhaseStep {
                name: name.to_string(),
                status,
                elapsed_secs: 0.0,
                pct,
            }
        })
        .collect()
}

fn canonical_phase_index_for_task(task: &super::super::dashboard::TaskSummary) -> Option<usize> {
    let phase_name = canonical_phase_name_for_task(task)?;
    CANONICAL_PHASES.iter().position(|&name| name == phase_name)
}

fn canonical_phase_name_for_task(
    task: &super::super::dashboard::TaskSummary,
) -> Option<&'static str> {
    let status = task.status.to_ascii_lowercase();

    match status.as_str() {
        "preflight" => Some("preflight"),
        "strategist" => Some("strategist"),
        "implementer" => Some("implementer"),
        "compile-gate" | "compile_gate" => Some("compile-gate"),
        "test-gate" | "test_gate" => Some("test-gate"),
        "reviewing" => Some("reviewing"),
        "critic-review" | "critic_review" => Some("critic-review"),
        "verdict" => Some("verdict"),
        "committing" => Some("committing"),
        "queued" => Some("preflight"),
        "enriching" => Some("strategist"),
        "implementing" | "auto-fixing" | "auto_fixing" => Some("implementer"),
        "gating" => Some(classify_gate_phase(task)),
        "verifying" | "regenerating-verify" | "regenerating_verify" => Some("test-gate"),
        "review" => Some("reviewing"),
        "doc-revision" | "doc_revision" => Some("critic-review"),
        "done" | "passed" => Some("verdict"),
        "merging" | "commit" | "complete" | "completed" => Some("committing"),
        "failed" | "error" => Some(classify_failed_phase(task)),
        "running" | "active" | "executing" | "in_progress" => Some(classify_phase_from_hints(task)),
        _ => Some(classify_phase_from_hints(task)),
    }
}

fn classify_gate_phase(task: &super::super::dashboard::TaskSummary) -> &'static str {
    let latest_gate = task
        .latest_gate
        .as_deref()
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    let task_id = task.task_id.to_ascii_lowercase();

    if latest_gate.contains("test")
        || latest_gate.contains("verify")
        || task_id.contains("test")
        || task_id.contains("verify")
    {
        "test-gate"
    } else {
        "compile-gate"
    }
}

fn classify_failed_phase(task: &super::super::dashboard::TaskSummary) -> &'static str {
    let hint = classify_phase_from_hints(task);
    if hint == "preflight" || hint == "strategist" {
        hint
    } else if task.latest_gate.as_deref().is_some_and(|gate| {
        let gate = gate.to_ascii_lowercase();
        gate.contains("test") || gate.contains("verify")
    }) {
        "test-gate"
    } else if task.latest_gate.is_some() {
        "compile-gate"
    } else {
        hint
    }
}

fn classify_phase_from_hints(task: &super::super::dashboard::TaskSummary) -> &'static str {
    let task_id = task.task_id.to_ascii_lowercase();
    let assigned_agents = task
        .assigned_agents
        .iter()
        .map(|agent| agent.to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join(" ");
    let latest_gate = task
        .latest_gate
        .as_deref()
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();

    if task_id.contains("preflight") {
        "preflight"
    } else if task_id.contains("strateg") || assigned_agents.contains("strateg") {
        "strategist"
    } else if task_id.contains("critic") || assigned_agents.contains("critic") {
        "critic-review"
    } else if task_id.contains("review") {
        "reviewing"
    } else if task_id.contains("merge")
        || task_id.contains("commit")
        || latest_gate.contains("merge")
    {
        "committing"
    } else if task_id.contains("verdict") {
        "verdict"
    } else if latest_gate.contains("test")
        || latest_gate.contains("verify")
        || task_id.contains("test")
        || task_id.contains("verify")
    {
        "test-gate"
    } else if latest_gate.contains("compile")
        || latest_gate.contains("clippy")
        || task_id.contains("compile")
        || task_id.contains("clippy")
        || task_id.contains("build")
    {
        "compile-gate"
    } else {
        "implementer"
    }
}

fn task_status_is_failed(status: &str) -> bool {
    TaskStatus::from(status).is_failed()
}

fn task_status_is_done(status: &str) -> bool {
    TaskStatus::from(status).is_done()
}

/// Build execution waves from plan entries.
///
/// Groups by `wave` field if set, otherwise places all plans in wave 0.
fn build_execution_waves(plans: &[PlanEntry]) -> Vec<Wave> {
    if plans.is_empty() {
        return Vec::new();
    }

    let has_waves = plans.iter().any(|p| p.wave.is_some());
    if !has_waves {
        let done = plans.iter().filter(|plan| plan_is_complete(plan)).count();
        return vec![Wave {
            index: 0,
            plans: plans.iter().map(|plan| plan.id.clone()).collect(),
            done,
            total: plans.len(),
            expanded: true,
            blocked_by_waves: Vec::new(),
        }];
    }

    let mut wave_map: std::collections::BTreeMap<usize, Vec<&PlanEntry>> =
        std::collections::BTreeMap::new();
    for plan in plans {
        let wave_index = plan.wave.unwrap_or(0);
        wave_map.entry(wave_index).or_default().push(plan);
    }

    // Collect all wave indices for blocker computation.
    let wave_indices: Vec<usize> = wave_map.keys().copied().collect();

    wave_map
        .into_iter()
        .map(|(idx, wave_plans)| {
            let done = wave_plans
                .iter()
                .filter(|plan| plan_is_complete(plan))
                .count();
            // A wave is blocked by all waves with a lower index.
            let blocked_by: Vec<usize> =
                wave_indices.iter().copied().filter(|&w| w < idx).collect();
            Wave {
                index: idx,
                plans: wave_plans.iter().map(|plan| plan.id.clone()).collect(),
                done,
                total: wave_plans.len(),
                expanded: true,
                blocked_by_waves: blocked_by,
            }
        })
        .collect()
}

pub(super) fn rebuild_execution_waves(plans: &[PlanEntry], previous: &[Wave]) -> Vec<Wave> {
    let prev_wave_expanded: std::collections::HashMap<usize, bool> = previous
        .iter()
        .map(|wave| (wave.index, wave.expanded))
        .collect();

    let mut waves = build_execution_waves(plans);
    for wave in &mut waves {
        if let Some(expanded) = prev_wave_expanded.get(&wave.index).copied() {
            wave.expanded = expanded;
        }
    }

    waves
}

fn clamp_selected_wave_idx(selected_wave_idx: usize, wave_count: usize) -> usize {
    if wave_count == 0 {
        0
    } else {
        selected_wave_idx.min(wave_count - 1)
    }
}

fn plan_is_complete(plan: &PlanEntry) -> bool {
    !plan.active && !plan.status.is_failed()
}

pub(super) fn derive_plan_waves(root: &Path, plans: &[PlanSummary]) -> HashMap<String, usize> {
    if plans.is_empty() {
        return HashMap::new();
    }

    let known_plan_ids: HashSet<String> = plans.iter().map(|plan| plan.id.clone()).collect();
    let mut deps_by_plan: HashMap<String, Vec<String>> = HashMap::new();
    let mut saw_dependency = false;

    for plan in plans {
        let tasks_path = plans_dir(root).join(&plan.id).join("tasks.toml");
        let mut deps: HashSet<String> = HashSet::new();

        if let Ok(tasks_file) = TasksFile::parse(&tasks_path) {
            for task in &tasks_file.tasks {
                deps.extend(task_plan_dependencies(task, &plan.id, &known_plan_ids));
            }
        }

        let mut deps = deps.into_iter().collect::<Vec<_>>();
        deps.sort();
        saw_dependency |= !deps.is_empty();
        deps_by_plan.insert(plan.id.clone(), deps);
    }

    if !saw_dependency {
        return HashMap::new();
    }

    let mut plan_waves = HashMap::new();
    for plan in plans {
        let mut visiting = HashSet::new();
        let wave = resolve_plan_wave(&plan.id, &deps_by_plan, &mut plan_waves, &mut visiting);
        plan_waves.insert(plan.id.clone(), wave);
    }
    plan_waves
}

fn task_plan_dependencies(
    task: &TaskDef,
    current_plan_id: &str,
    known_plan_ids: &HashSet<String>,
) -> Vec<String> {
    let mut deps = HashSet::new();

    for dep in &task.depends_on_plan {
        let plan_id = dep.trim();
        if !plan_id.is_empty() && plan_id != current_plan_id && known_plan_ids.contains(plan_id) {
            deps.insert(plan_id.to_string());
        }
    }

    for dep in &task.depends_on {
        let Some((plan_id, _task_id)) = dep.split_once(':') else {
            continue;
        };
        let plan_id = plan_id.trim();
        if !plan_id.is_empty() && plan_id != current_plan_id && known_plan_ids.contains(plan_id) {
            deps.insert(plan_id.to_string());
        }
    }

    let mut deps = deps.into_iter().collect::<Vec<_>>();
    deps.sort();
    deps
}

fn resolve_plan_wave(
    plan_id: &str,
    deps_by_plan: &HashMap<String, Vec<String>>,
    cache: &mut HashMap<String, usize>,
    visiting: &mut HashSet<String>,
) -> usize {
    if let Some(&wave) = cache.get(plan_id) {
        return wave;
    }

    if !visiting.insert(plan_id.to_string()) {
        return 0;
    }

    let wave = deps_by_plan
        .get(plan_id)
        .map(|deps| {
            deps.iter()
                .map(|dep| resolve_plan_wave(dep, deps_by_plan, cache, visiting) + 1)
                .max()
                .unwrap_or(0)
        })
        .unwrap_or(0);

    visiting.remove(plan_id);
    cache.insert(plan_id.to_string(), wave);
    wave
}

fn build_task_checklist_from_execution(data: &DashboardData) -> Vec<TaskRow> {
    if let Some(exec) = &data.current_plan_execution {
        return exec
            .tasks
            .iter()
            .map(|t| {
                let status = if t.is_current {
                    TaskStatus::Active
                } else {
                    TaskStatus::from(t.phase.as_str())
                };
                let elapsed_secs = parse_duration_to_secs(&t.duration);
                TaskRow {
                    id: t.task_id.clone(),
                    title: t.title.clone(),
                    status,
                    elapsed_secs,
                    depends_on: Vec::new(),
                    acceptance_text: None,
                    verify_command: None,
                    files: Vec::new(),
                }
            })
            .collect();
    }

    // Fallback: build from active_tasks
    data.active_tasks
        .iter()
        .map(|t| {
            let status = TaskStatus::from(t.status.as_str());
            TaskRow {
                id: t.task_id.clone(),
                title: t.task_id.clone(),
                status,
                elapsed_secs: 0.0,
                depends_on: Vec::new(),
                acceptance_text: None,
                verify_command: None,
                files: Vec::new(),
            }
        })
        .collect()
}

/// Populate task detail fields on `TaskRow` by looking up the corresponding
/// `TaskEntry` from the plan's task list.  This enriches the task checklist
/// with depends_on, acceptance criteria, verify command, and file lists
/// that originate from `tasks.toml` (P5.1-P5.2).
fn populate_task_row_details(checklist: &mut [TaskRow], plans: &[PlanEntry]) {
    // Build a lookup: task_id -> &TaskEntry (across all plans).
    let mut task_entry_by_id: HashMap<&str, &TaskEntry> = HashMap::new();
    for plan in plans {
        for task_entry in &plan.tasks {
            task_entry_by_id.insert(&task_entry.id, task_entry);
        }
    }
    for row in checklist.iter_mut() {
        if let Some(entry) = task_entry_by_id.get(row.id.as_str()) {
            if row.depends_on.is_empty() && !entry.depends_on.is_empty() {
                row.depends_on.clone_from(&entry.depends_on);
            }
            if row.acceptance_text.is_none() && entry.acceptance_text.is_some() {
                row.acceptance_text.clone_from(&entry.acceptance_text);
            }
            if row.verify_command.is_none() && entry.verify_command.is_some() {
                row.verify_command.clone_from(&entry.verify_command);
            }
            if row.files.is_empty() && !entry.files.is_empty() {
                row.files.clone_from(&entry.files);
            }
        }
    }
}

/// Parse a duration string like "5s", "2m 30s", "120ms" into seconds.
fn parse_duration_to_secs(duration: &str) -> f64 {
    if duration == "--" || duration.is_empty() {
        return 0.0;
    }
    if let Some(ms) = duration.strip_suffix("ms") {
        return ms.parse::<f64>().unwrap_or(0.0) / 1000.0;
    }
    let mut total = 0.0;
    for part in duration.split_whitespace() {
        if let Some(m) = part.strip_suffix('m') {
            total += m.parse::<f64>().unwrap_or(0.0) * 60.0;
        } else if let Some(s) = part.strip_suffix('s') {
            total += s.parse::<f64>().unwrap_or(0.0);
        }
    }
    total
}

/// Populate phase_pipeline elapsed times from episode timestamps (Task 7).
fn populate_phase_elapsed(
    pipeline: &mut [PhaseStep],
    episodes: &[roko_learn::episode_logger::Episode],
) {
    if episodes.is_empty() || pipeline.is_empty() {
        return;
    }

    // Compute per-phase elapsed from episodes that have a matching trigger_kind
    // or kind. Map episode kinds to canonical phase names.
    let mut phase_durations: std::collections::HashMap<String, f64> =
        std::collections::HashMap::new();

    for episode in episodes {
        let phase_name = episode_to_phase_name(episode);
        if !phase_name.is_empty() {
            *phase_durations.entry(phase_name).or_default() += episode.duration_secs;
        }
    }

    for step in pipeline.iter_mut() {
        if let Some(&elapsed) = phase_durations.get(&step.name) {
            step.elapsed_secs = elapsed;
        }
    }
}

/// Map an episode's kind/trigger to a canonical phase name.
fn episode_to_phase_name(episode: &roko_learn::episode_logger::Episode) -> String {
    let kind = episode.kind.to_ascii_lowercase();
    let trigger = episode.trigger_kind.to_ascii_lowercase();
    let template = episode.agent_template.to_ascii_lowercase();

    // Direct kind matches
    match kind.as_str() {
        "preflight" => return "preflight".to_string(),
        "compile" | "compile-gate" | "compile_gate" => return "compile-gate".to_string(),
        "test" | "test-gate" | "test_gate" => return "test-gate".to_string(),
        "review" | "reviewing" | "critic-review" | "critic_review" => {
            return "reviewing".to_string();
        }
        "verdict" => return "verdict".to_string(),
        "commit" | "committing" => return "committing".to_string(),
        _ => {}
    }

    // Template-based inference
    if template.contains("strategist") {
        return "strategist".to_string();
    }
    if template.contains("implementer") || template.contains("implement") {
        return "implementer".to_string();
    }
    if template.contains("reviewer") || template.contains("critic") {
        return "critic-review".to_string();
    }

    // Trigger-based inference
    if trigger.contains("gate") {
        if trigger.contains("compile") {
            return "compile-gate".to_string();
        }
        if trigger.contains("test") {
            return "test-gate".to_string();
        }
    }

    // Agent turn episodes map to implementer by default
    if kind == "agent_turn" {
        return "implementer".to_string();
    }

    String::new()
}
