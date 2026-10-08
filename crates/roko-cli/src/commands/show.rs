//! `roko show` state inspection command.

use crate::*;
use chrono::{DateTime, NaiveDate, Utc};
use roko_cli::DashboardData;
use roko_cli::tui::dashboard::AgentSummary;
use roko_fs::RokoLayout;
use roko_learn::efficiency::AgentEfficiencyEvent;
use roko_learn::run_metrics::RunMetricsRecord;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShowSubject {
    Overview,
    Costs,
    Agents,
    Knowledge,
    Plans,
    Learning,
    History,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ShowTarget {
    Subject(ShowSubject),
    WorkId(String),
}

impl ShowTarget {
    fn parse(subject: Option<String>) -> Self {
        let Some(subject) = subject else {
            return Self::Subject(ShowSubject::Overview);
        };
        let normalized = subject.trim().to_ascii_lowercase();
        match normalized.as_str() {
            "" | "overview" | "summary" => Self::Subject(ShowSubject::Overview),
            "cost" | "costs" => Self::Subject(ShowSubject::Costs),
            "agent" | "agents" => Self::Subject(ShowSubject::Agents),
            "knowledge" | "know" | "neuro" => Self::Subject(ShowSubject::Knowledge),
            "plan" | "plans" => Self::Subject(ShowSubject::Plans),
            "learning" | "learn" | "router" | "routing" => Self::Subject(ShowSubject::Learning),
            "history" | "events" | "log" => Self::Subject(ShowSubject::History),
            _ => Self::WorkId(subject),
        }
    }
}

/// Default `--since` span of the activity views.
const DEFAULT_ACTIVITY_WINDOW: &str = "7d";

/// How many of the latest plan runs `roko show costs` lists (backlog 2122).
const RECENT_RUNS: usize = 10;

/// The efficiency events the activity views count, from `--since`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActivityWindow {
    /// Every recorded event (`--since all`).
    All,
    /// Events at or after this time.
    Since(DateTime<Utc>),
}

impl ActivityWindow {
    /// Parse `--since`: `all`, a span back from `now` (`30m`, `24h`, `7d`, `2w`), a
    /// `YYYY-MM-DD` date (its midnight UTC) or an RFC 3339 time. `None` is the default span.
    fn parse(value: Option<&str>, now: DateTime<Utc>) -> Result<Self> {
        let value = value.unwrap_or(DEFAULT_ACTIVITY_WINDOW).trim();
        if value.eq_ignore_ascii_case("all") {
            return Ok(Self::All);
        }
        if let Some(span) = parse_span(value) {
            // A span reaching past the earliest representable time covers everything.
            return Ok(now.checked_sub_signed(span).map_or(Self::All, Self::Since));
        }
        if let Ok(time) = DateTime::parse_from_rfc3339(value) {
            return Ok(Self::Since(time.with_timezone(&Utc)));
        }
        NaiveDate::parse_from_str(value, "%Y-%m-%d")
            .ok()
            .and_then(|date| date.and_hms_opt(0, 0, 0))
            .map(|midnight| Self::Since(midnight.and_utc()))
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "--since {value}: expected a span such as 24h or 7d, YYYY-MM-DD, an RFC 3339 \
                     time, or all"
                )
            })
    }

    /// Whether an event stamped `timestamp` is in the window.
    fn contains(self, timestamp: &str) -> bool {
        self.includes(event_time(timestamp))
    }

    /// Whether an event at `time` is in the window. Under a cutoff, an event whose time is
    /// unknown is out: nothing shows that it is recent.
    fn includes(self, time: Option<DateTime<Utc>>) -> bool {
        match self {
            Self::All => true,
            Self::Since(cutoff) => time.is_some_and(|time| time >= cutoff),
        }
    }

    fn label(self) -> String {
        match self {
            Self::All => String::from("all time"),
            Self::Since(cutoff) => format!("since {}", cutoff.format("%Y-%m-%d %H:%M UTC")),
        }
    }
}

/// A `--since` span such as `30m`, `24h`, `7d` or `2w`.
fn parse_span(value: &str) -> Option<chrono::Duration> {
    let unit = value.chars().last()?;
    let count = value[..value.len() - unit.len_utf8()]
        .parse::<i64>()
        .ok()
        .filter(|count| *count > 0)?;
    match unit {
        'm' => chrono::Duration::try_minutes(count),
        'h' => chrono::Duration::try_hours(count),
        'd' => chrono::Duration::try_days(count),
        'w' => chrono::Duration::try_weeks(count),
        _ => None,
    }
}

/// The time of an efficiency event (its RFC 3339 `timestamp`), if it parses.
fn event_time(timestamp: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(timestamp)
        .ok()
        .map(|time| time.with_timezone(&Utc))
}

#[derive(Debug)]
struct ShowState {
    workdir: PathBuf,
    layout: RokoLayout,
    data: DashboardData,
    work_items: Vec<WorkItemSummary>,
    /// The `--since` window of the activity views.
    window: ActivityWindow,
    /// The latest plan runs' summaries in `.roko/learn/run-metrics.jsonl`,
    /// newest first.
    recent_runs: Vec<RunMetricsRecord>,
}

#[derive(Debug, Clone)]
struct WorkItemSummary {
    id: String,
    kind: String,
    status: String,
    prompt: String,
    tasks_done: Option<usize>,
    tasks_total: Option<usize>,
    cost_usd: Option<f64>,
    created: Option<String>,
    source: PathBuf,
    modified_ms: u64,
}

#[derive(Debug, Default)]
struct CostAggregate {
    turns: usize,
    input_tokens: u64,
    output_tokens: u64,
    cost_usd: f64,
    /// Turns that carry a gate verdict, and how many of those passed.
    verdicts: usize,
    passed: usize,
}

impl CostAggregate {
    fn record(&mut self, event: &AgentEfficiencyEvent) {
        self.turns += 1;
        self.input_tokens += event.input_tokens;
        self.output_tokens += event.output_tokens;
        self.cost_usd += event.cost_usd;
        if let Some(passed) = event.gate_passed {
            self.verdicts += 1;
            self.passed += usize::from(passed);
        }
    }
}

pub(crate) async fn cmd_show(
    cli: &Cli,
    workdir: Option<PathBuf>,
    live: bool,
    follow: bool,
    serve_url: String,
    subject: Option<String>,
    since: Option<String>,
) -> Result<i32> {
    if live {
        return super::dashboard::cmd_dashboard(cli, workdir, None, false, false, None).await;
    }

    // --follow: stream live SSE events from a running roko serve instance.
    if follow {
        let color = cli.color.should_color();
        let client = roko_cli::runner::SseStreamClient::new(&serve_url, color);
        let cancel = tokio_util::sync::CancellationToken::new();
        let cancel_for_signal = cancel.clone();
        tokio::spawn(async move {
            let _ = tokio::signal::ctrl_c().await;
            cancel_for_signal.cancel();
        });
        // User-facing progress output (streaming mode indicator)
        eprintln!("Streaming events from {serve_url}/api/events (Ctrl+C to stop)...");
        return match client.stream(cancel).await {
            Ok(()) => Ok(EXIT_SUCCESS),
            Err(err) => {
                tracing::error!(error = %err, "SSE stream error");
                Ok(1)
            }
        };
    }

    let workdir = workdir.unwrap_or_else(|| resolve_workdir(cli));
    let window = ActivityWindow::parse(since.as_deref(), Utc::now())?;
    // Read-only state inspection: shared lock allows coexistence with an
    // active plan runner (which holds only the runner lock, not the workspace
    // lock).
    let _lock = roko_cli::workspace_lock::acquire_workspace_lock_shared(&workdir.join(".roko"))?;
    let state = load_show_state(&workdir, window);
    let target = ShowTarget::parse(subject);
    if cli.json {
        let value = render_json(&state, target)?;
        println!("{}", serde_json::to_string_pretty(&value)?);
        return Ok(EXIT_SUCCESS);
    }
    let output = match target {
        ShowTarget::Subject(ShowSubject::Overview) => render_overview(&state),
        ShowTarget::Subject(ShowSubject::Costs) => render_costs(&state),
        ShowTarget::Subject(ShowSubject::Agents) => render_agents(&state),
        ShowTarget::Subject(ShowSubject::Knowledge) => render_knowledge(&state),
        ShowTarget::Subject(ShowSubject::Plans) => render_plans(&state),
        ShowTarget::Subject(ShowSubject::Learning) => render_learning(&state),
        ShowTarget::Subject(ShowSubject::History) => render_history(&state),
        ShowTarget::WorkId(work_id) => render_work_detail(&state, &work_id)?,
    };

    print!("{output}");
    Ok(EXIT_SUCCESS)
}

fn load_show_state(workdir: &Path, window: ActivityWindow) -> ShowState {
    let layout = RokoLayout::for_project(workdir);
    let data = DashboardData::load_best_effort(workdir);
    let work_items = collect_work_items(workdir, &layout, &data);
    let run_metrics = layout.learn_dir().join("run-metrics.jsonl");
    let recent_runs = roko_learn::run_metrics::read_recent(&run_metrics, RECENT_RUNS)
        .unwrap_or_else(|error| {
            tracing::warn!(path = %run_metrics.display(), %error, "cannot read recent runs");
            Vec::new()
        });
    ShowState {
        workdir: workdir.to_path_buf(),
        layout,
        data,
        work_items,
        window,
        recent_runs,
    }
}

fn render_overview(state: &ShowState) -> String {
    let mut out = header(state, "overview");
    push_window(&mut out, state.window);
    push_section(&mut out, "work items");
    if state.work_items.is_empty() {
        push_empty(
            &mut out,
            "No work items found in .roko/work, .roko/jobs, or plans.",
        );
    } else {
        for item in state.work_items.iter().take(8) {
            push_work_item_row(&mut out, item);
        }
    }

    push_section(&mut out, "agents");
    let agents = agent_rows(state);
    if agents.rows.is_empty() && agents.stale == 0 {
        push_empty(
            &mut out,
            "No agents found in the durable Runner projection or efficiency events.",
        );
    } else {
        for row in agents.rows.iter().take(6) {
            push_kv(&mut out, &row.id, &row.summary);
        }
        push_stale_agents(&mut out, &agents, state.window);
    }

    push_section(&mut out, "costs");
    let total = cost_total(window_events(state));
    push_kv(
        &mut out,
        "total",
        &format!(
            "{} across {} turn(s)",
            format_cost(total.cost_usd),
            total.turns
        ),
    );
    push_kv(
        &mut out,
        "tokens",
        &format!(
            "{} input, {} output",
            format_count(total.input_tokens),
            format_count(total.output_tokens)
        ),
    );

    push_section(&mut out, "learning");
    push_learning_summary(state, &mut out);
    push_section(&mut out, "try");
    push_empty(
        &mut out,
        "roko show costs | roko show agents | roko show learning | roko show <work-id>",
    );
    out
}

fn render_costs(state: &ShowState) -> String {
    let mut out = header(state, "costs");
    push_window(&mut out, state.window);
    let events = window_events(state);
    let total = cost_total(events.iter().copied());
    push_section(&mut out, "summary");
    push_kv(&mut out, "turns", &total.turns.to_string());
    push_kv(&mut out, "total cost", &format_cost(total.cost_usd));
    push_kv(
        &mut out,
        "avg turn cost",
        &format_cost(if total.turns == 0 {
            0.0
        } else {
            total.cost_usd / total.turns as f64
        }),
    );
    push_kv(&mut out, "gate pass rate", &format_pass_rate(&total));
    if state.window != ActivityWindow::All {
        let all_time = cost_total(&state.data.efficiency_events);
        push_kv(
            &mut out,
            "all time",
            &format!(
                "{} across {} turn(s)",
                format_cost(all_time.cost_usd),
                all_time.turns
            ),
        );
    }

    let by_model = cost_by_model(&events);
    push_section(&mut out, "by model");
    if by_model.is_empty() {
        push_empty(&mut out, &no_cost_events("model", state.window));
    } else {
        for (model, aggregate) in by_model {
            push_kv(
                &mut out,
                &model,
                &format!(
                    "{} | {} turn(s) | {} in / {} out",
                    format_cost(aggregate.cost_usd),
                    aggregate.turns,
                    format_count(aggregate.input_tokens),
                    format_count(aggregate.output_tokens)
                ),
            );
        }
    }

    let by_task = cost_by_task(&events);
    push_section(&mut out, "by task");
    if by_task.is_empty() {
        push_empty(&mut out, &no_cost_events("task", state.window));
    } else {
        for (task, aggregate) in by_task.into_iter().take(12) {
            push_kv(
                &mut out,
                &task,
                &format!(
                    "{} | {} turn(s)",
                    format_cost(aggregate.cost_usd),
                    aggregate.turns
                ),
            );
        }
    }

    let by_day = cost_by_day(&events);
    push_section(&mut out, "by day");
    if by_day.is_empty() {
        push_empty(&mut out, &no_cost_events("dated", state.window));
    } else {
        for (day, aggregate) in by_day {
            push_kv(
                &mut out,
                &day,
                &format!(
                    "{} | {} turn(s)",
                    format_cost(aggregate.cost_usd),
                    aggregate.turns
                ),
            );
        }
    }

    push_recent_runs(&mut out, state);
    out
}

/// The latest plan runs in the window, newest first, from the summary each
/// run appends to `.roko/learn/run-metrics.jsonl` (backlog 2122).
fn push_recent_runs(out: &mut String, state: &ShowState) {
    push_section(out, "recent runs");
    let runs: Vec<&RunMetricsRecord> = state
        .recent_runs
        .iter()
        .filter(|run| state.window.contains(&run.timestamp))
        .collect();
    if runs.is_empty() {
        push_empty(
            out,
            &format!(
                "No plan run {} in .roko/learn/run-metrics.jsonl.",
                state.window.label()
            ),
        );
        return;
    }
    for run in runs {
        let started = event_time(&run.timestamp).map_or_else(
            || run.timestamp.clone(),
            |time| time.format("%Y-%m-%d %H:%M").to_string(),
        );
        push_kv(
            out,
            &started,
            &format!(
                "{} | {} completed, {} failed, {} unverified | {} in / {} out | {} | {}",
                run.run_id,
                run.tasks_completed,
                run.tasks_failed,
                run.tasks_unverified,
                format_count(run.total_tokens_in),
                format_count(run.total_tokens_out),
                format_cost(run.total_cost_usd),
                format_duration_ms(run.duration_ms as f64)
            ),
        );
    }
}

fn render_agents(state: &ShowState) -> String {
    let mut out = header(state, "agents");
    push_window(&mut out, state.window);
    let agents = agent_rows(state);
    push_section(&mut out, "agents");
    if agents.rows.is_empty() && agents.stale == 0 {
        push_empty(
            &mut out,
            "No agents found in the durable Runner projection or .roko/learn/efficiency.jsonl.",
        );
    } else {
        for row in &agents.rows {
            push_kv(&mut out, &row.id, &row.summary);
        }
        push_stale_agents(&mut out, &agents, state.window);
    }
    out
}

fn render_knowledge(state: &ShowState) -> String {
    let mut out = header(state, "knowledge");
    let entries = &state.data.knowledge_entries;
    push_section(&mut out, "store");
    push_kv(
        &mut out,
        "path",
        &display_rel(
            &state.workdir,
            &state.layout.root().join("neuro").join("knowledge.jsonl"),
        ),
    );
    push_kv(&mut out, "entries", &entries.len().to_string());

    push_section(&mut out, "recent entries");
    if entries.is_empty() {
        push_empty(
            &mut out,
            "No knowledge entries found in .roko/neuro/knowledge.jsonl.",
        );
    } else {
        let mut sorted = entries.clone();
        sorted.sort_by_key(|e| std::cmp::Reverse(e.created_at));
        for entry in sorted.iter().take(12) {
            let tags = if entry.tags.is_empty() {
                String::from("no tags")
            } else {
                entry.tags.join(", ")
            };
            push_kv(
                &mut out,
                &entry.id,
                &format!(
                    "{} | {} | conf {:.2} | {} | {}",
                    entry.kind, entry.tier, entry.confidence, tags, entry.content_preview
                ),
            );
        }
    }
    out
}

fn render_plans(state: &ShowState) -> String {
    let mut out = header(state, "plans");
    if let Some(current) = &state.data.current_plan_execution {
        push_section(&mut out, "current");
        push_kv(
            &mut out,
            "plan",
            &format!("{} | {}", current.plan_id, current.plan_title),
        );
        push_kv(
            &mut out,
            "tasks",
            &format!("{}/{}", current.tasks_done, current.tasks_total),
        );
        if let Some(task) = &current.current_task {
            push_kv(
                &mut out,
                "current task",
                &format!("{} | {}", task.task_id, task.description),
            );
        }
    }

    push_section(&mut out, "all plans");
    if state.data.plans.is_empty() {
        push_empty(&mut out, "No plans found in plans/ or .roko/plans.");
    } else {
        for plan in &state.data.plans {
            let status = plan_status(plan.completed, plan.tasks_done, plan.tasks_failed);
            push_kv(
                &mut out,
                &plan.id,
                &format!(
                    "{} | {} | tasks {}/{} done, {} failed",
                    status, plan.title, plan.tasks_done, plan.task_count, plan.tasks_failed
                ),
            );
            if let Some(error) = plan.last_error.as_deref().filter(|error| !error.is_empty()) {
                push_kv(&mut out, "last error", error);
            }
        }
    }
    out
}

fn render_learning(state: &ShowState) -> String {
    let mut out = header(state, "learning");
    push_section(&mut out, "routing");
    let router = &state.data.cascade_router;
    if router.model_slugs.is_empty() && router.confidence_stats.is_empty() {
        push_empty(
            &mut out,
            "No cascade router state found in .roko/learn/cascade-router.json.",
        );
    } else {
        push_kv(&mut out, "models", &router.model_slugs.join(", "));
        for (model, stats) in router.confidence_stats.iter() {
            push_kv(
                &mut out,
                model,
                &format!(
                    "{} success over {} trial(s)",
                    format_percent(ratio(stats.successes as usize, stats.trials as usize)),
                    stats.trials
                ),
            );
        }
    }

    push_section(&mut out, "experiments");
    if state.data.experiments.is_empty() {
        push_empty(
            &mut out,
            "No prompt experiments found in .roko/learn/experiments.json.",
        );
    } else {
        for experiment in state.data.experiments.iter().take(10) {
            let winner = experiment.winner_id.as_deref().unwrap_or("none");
            push_kv(
                &mut out,
                &experiment.experiment_id,
                &format!(
                    "{} | {} variant(s) | {} trial(s) | winner {}",
                    experiment.status, experiment.active_variants, experiment.total_trials, winner
                ),
            );
        }
    }

    push_section(&mut out, "gates");
    if state.data.gate_results_page.gate_rows.is_empty() {
        push_empty(
            &mut out,
            "No gate signal rows found in .roko/engrams.jsonl.",
        );
    } else {
        for gate in state.data.gate_results_page.gate_rows.iter().take(10) {
            push_kv(
                &mut out,
                &gate.gate_name,
                &format!(
                    "{} pass | {} run(s) | avg {}",
                    format_percent(gate.pass_rate),
                    gate.total_runs,
                    format_duration_ms(gate.avg_duration_ms)
                ),
            );
        }
    }

    push_section(&mut out, "c-factor");
    if let Some(cfactor) = &state.data.cfactor {
        push_kv(&mut out, "overall", &format!("{:.2}", cfactor.overall));
        push_kv(
            &mut out,
            "cost efficiency",
            &format_percent(cfactor.components.cost_efficiency),
        );
        push_kv(
            &mut out,
            "knowledge growth",
            &format_percent(cfactor.components.knowledge_growth),
        );
    } else {
        push_empty(
            &mut out,
            "No C-Factor snapshots found in .roko/learn/c-factor.jsonl.",
        );
    }
    out
}

fn render_history(state: &ShowState) -> String {
    let mut out = header(state, "history");
    push_section(&mut out, "state events");
    if state.data.event_log.is_empty() {
        push_empty(
            &mut out,
            "No event log entries found in .roko/state/events.json.",
        );
    } else {
        let mut events = state.data.event_log.clone();
        events.sort_by_key(|event| event.timestamp_ms);
        for event in events.iter().rev().take(20).rev() {
            let scope = event_scope(&event.plan_id, &event.task_id);
            push_kv(
                &mut out,
                &event.event_type,
                &format!("{} | {}", scope, event.message),
            );
        }
    }

    push_section(&mut out, "recent turns");
    if state.data.efficiency_events.is_empty() {
        push_empty(
            &mut out,
            "No efficiency events found in .roko/learn/efficiency.jsonl.",
        );
    } else {
        for event in state.data.efficiency_events.iter().rev().take(12).rev() {
            push_kv(
                &mut out,
                &event.timestamp,
                &format!(
                    "{}:{} | {} | {} | {}",
                    event.plan_id,
                    event.task_id,
                    event.agent_id,
                    event.model,
                    format_cost(event.cost_usd)
                ),
            );
        }
    }
    out
}

fn render_work_detail(state: &ShowState, work_id: &str) -> Result<String> {
    let Some(item) = state.work_items.iter().find(|item| item.id == work_id) else {
        anyhow::bail!(
            "`{work_id}` is not a recognised subject or work-item ID.\n\
            Valid subjects: overview, costs, agents, knowledge, plans, learning, history\n\
            Run `roko show` with no argument for an overview, or `roko show <subject>` to inspect a section."
        );
    };

    let mut out = header(state, work_id);
    push_section(&mut out, "work item");
    push_kv(&mut out, "id", &item.id);
    push_kv(&mut out, "kind", &item.kind);
    push_kv(&mut out, "status", &item.status);
    push_kv(
        &mut out,
        "source",
        &display_rel(&state.workdir, &item.source),
    );
    if !item.prompt.is_empty() {
        push_kv(&mut out, "prompt", &item.prompt);
    }
    if let Some(created) = &item.created {
        push_kv(&mut out, "created", created);
    }
    if item.tasks_done.is_some() || item.tasks_total.is_some() {
        push_kv(
            &mut out,
            "tasks",
            &format!(
                "{}/{}",
                item.tasks_done.unwrap_or_default(),
                item.tasks_total.unwrap_or_default()
            ),
        );
    }
    if let Some(cost) = item.cost_usd {
        push_kv(&mut out, "cost", &format_cost(cost));
    }

    if let Some(plan) = state.data.plans.iter().find(|plan| plan.id == item.id) {
        push_section(&mut out, "plan");
        push_kv(&mut out, "title", &plan.title);
        push_kv(
            &mut out,
            "tasks",
            &format!(
                "{}/{} done, {} failed",
                plan.tasks_done, plan.task_count, plan.tasks_failed
            ),
        );
        if let Some(error) = plan.last_error.as_deref().filter(|error| !error.is_empty()) {
            push_kv(&mut out, "last error", error);
        }
    }

    let related_costs = state
        .data
        .efficiency_events
        .iter()
        .filter(|event| event.plan_id == item.id || event.task_id == item.id)
        .fold(CostAggregate::default(), |mut aggregate, event| {
            aggregate.turns += 1;
            aggregate.input_tokens += event.input_tokens;
            aggregate.output_tokens += event.output_tokens;
            aggregate.cost_usd += event.cost_usd;
            aggregate
        });
    push_section(&mut out, "costs");
    if related_costs.turns == 0 {
        push_empty(&mut out, "No related efficiency events found.");
    } else {
        push_kv(&mut out, "turns", &related_costs.turns.to_string());
        push_kv(&mut out, "cost", &format_cost(related_costs.cost_usd));
        push_kv(
            &mut out,
            "tokens",
            &format!(
                "{} input, {} output",
                format_count(related_costs.input_tokens),
                format_count(related_costs.output_tokens)
            ),
        );
    }

    push_section(&mut out, "history");
    let mut wrote_event = false;
    for event in state
        .data
        .event_log
        .iter()
        .filter(|event| event.plan_id == item.id || event.task_id == item.id)
        .take(12)
    {
        wrote_event = true;
        push_kv(
            &mut out,
            &event.event_type,
            &format!(
                "{} | {}",
                event_scope(&event.plan_id, &event.task_id),
                event.message
            ),
        );
    }
    if !wrote_event {
        push_empty(
            &mut out,
            "No related events found in .roko/state/events.json.",
        );
    }

    Ok(out)
}

/// `roko show --json`: each view as one JSON object, over the same data and `--since` window
/// as its text, with the subject, the workspace and the window at the top level.
fn render_json(state: &ShowState, target: ShowTarget) -> Result<Value> {
    let (subject, body) = match target {
        ShowTarget::Subject(ShowSubject::Overview) => ("overview", overview_json(state)),
        ShowTarget::Subject(ShowSubject::Costs) => ("costs", costs_json(state)),
        ShowTarget::Subject(ShowSubject::Agents) => {
            ("agents", agents_json(&agent_rows(state), usize::MAX))
        }
        ShowTarget::Subject(ShowSubject::Knowledge) => ("knowledge", knowledge_json(state)),
        ShowTarget::Subject(ShowSubject::Plans) => ("plans", plans_json(state)),
        ShowTarget::Subject(ShowSubject::Learning) => ("learning", learning_json(state)),
        ShowTarget::Subject(ShowSubject::History) => ("history", history_json(state)),
        ShowTarget::WorkId(work_id) => ("work-item", work_detail_json(state, &work_id)?),
    };
    let mut out = serde_json::Map::new();
    out.insert("subject".into(), subject.into());
    out.insert("workdir".into(), state.workdir.display().to_string().into());
    out.insert("window".into(), window_json(state.window));
    if let Value::Object(body) = body {
        out.extend(body);
    }
    Ok(Value::Object(out))
}

fn window_json(window: ActivityWindow) -> Value {
    let since = match window {
        ActivityWindow::All => Value::Null,
        ActivityWindow::Since(cutoff) => cutoff.to_rfc3339().into(),
    };
    serde_json::json!({ "label": window.label(), "since": since })
}

fn cost_json(aggregate: &CostAggregate) -> Value {
    serde_json::json!({
        "turns": aggregate.turns,
        "cost_usd": aggregate.cost_usd,
        "input_tokens": aggregate.input_tokens,
        "output_tokens": aggregate.output_tokens,
        "gate_verdicts": aggregate.verdicts,
        "gate_passed": aggregate.passed,
    })
}

fn work_item_json(state: &ShowState, item: &WorkItemSummary) -> Value {
    serde_json::json!({
        "id": item.id,
        "kind": item.kind,
        "status": item.status,
        "prompt": item.prompt,
        "created": item.created,
        "tasks_done": item.tasks_done,
        "tasks_total": item.tasks_total,
        "cost_usd": item.cost_usd,
        "source": display_rel(&state.workdir, &item.source),
    })
}

/// The first `limit` agent rows, and how many older agents the window hides.
fn agents_json(agents: &AgentRows, limit: usize) -> Value {
    let rows: Vec<Value> = agents
        .rows
        .iter()
        .take(limit)
        .map(|row| serde_json::json!({ "id": row.id, "summary": row.summary }))
        .collect();
    serde_json::json!({ "agents": rows, "stale_agents_hidden": agents.stale })
}

fn event_log_json(event: &roko_cli::tui::dashboard::EventLogEntry) -> Value {
    serde_json::json!({
        "timestamp_ms": event.timestamp_ms,
        "event_type": event.event_type,
        "plan_id": event.plan_id,
        "task_id": event.task_id,
        "message": event.message,
    })
}

fn learning_summary_json(state: &ShowState) -> Value {
    let router = &state.data.cascade_router;
    let trials: u64 = router
        .confidence_stats
        .values()
        .map(|stats| stats.trials)
        .sum();
    let successes: u64 = router
        .confidence_stats
        .values()
        .map(|stats| stats.successes)
        .sum();
    serde_json::json!({
        "routing_trials": trials,
        "routing_successes": successes,
        "experiments": state.data.experiments.len(),
        "cfactor": state.data.cfactor.as_ref().map(|cfactor| cfactor.overall),
    })
}

fn overview_json(state: &ShowState) -> Value {
    let work_items: Vec<Value> = state
        .work_items
        .iter()
        .take(8)
        .map(|item| work_item_json(state, item))
        .collect();
    let agents = agents_json(&agent_rows(state), 6);
    serde_json::json!({
        "work_items": work_items,
        "agents": agents["agents"],
        "stale_agents_hidden": agents["stale_agents_hidden"],
        "costs": cost_json(&cost_total(window_events(state))),
        "learning": learning_summary_json(state),
    })
}

fn costs_json(state: &ShowState) -> Value {
    let events = window_events(state);
    let total = cost_total(events.iter().copied());
    let avg_turn_cost_usd = if total.turns == 0 {
        0.0
    } else {
        total.cost_usd / total.turns as f64
    };
    let all_time = (state.window != ActivityWindow::All)
        .then(|| cost_json(&cost_total(&state.data.efficiency_events)));
    let by_model: Vec<Value> = cost_by_model(&events)
        .iter()
        .map(|(model, aggregate)| keyed(cost_json(aggregate), "model", model))
        .collect();
    let by_task: Vec<Value> = cost_by_task(&events)
        .iter()
        .take(12)
        .map(|(task, aggregate)| keyed(cost_json(aggregate), "task", task))
        .collect();
    let by_day: Vec<Value> = cost_by_day(&events)
        .iter()
        .map(|(day, aggregate)| keyed(cost_json(aggregate), "day", day))
        .collect();
    let recent_runs: Vec<Value> = state
        .recent_runs
        .iter()
        .filter(|run| state.window.contains(&run.timestamp))
        .map(|run| {
            serde_json::json!({
                "run_id": run.run_id,
                "started": run.timestamp,
                "tasks_completed": run.tasks_completed,
                "tasks_failed": run.tasks_failed,
                "tasks_unverified": run.tasks_unverified,
                "input_tokens": run.total_tokens_in,
                "output_tokens": run.total_tokens_out,
                "cost_usd": run.total_cost_usd,
                "duration_ms": run.duration_ms,
            })
        })
        .collect();
    serde_json::json!({
        "summary": cost_json(&total),
        "avg_turn_cost_usd": avg_turn_cost_usd,
        "all_time": all_time,
        "by_model": by_model,
        "by_task": by_task,
        "by_day": by_day,
        "recent_runs": recent_runs,
    })
}

/// `value` with one more field in front of the rest, for a keyed row.
fn keyed(value: Value, key: &str, name: &str) -> Value {
    let mut row = serde_json::Map::new();
    row.insert(key.into(), name.into());
    if let Value::Object(fields) = value {
        row.extend(fields);
    }
    Value::Object(row)
}

fn knowledge_json(state: &ShowState) -> Value {
    let entries = &state.data.knowledge_entries;
    let mut recent: Vec<_> = entries.iter().collect();
    recent.sort_by_key(|entry| std::cmp::Reverse(entry.created_at));
    let recent: Vec<Value> = recent
        .into_iter()
        .take(12)
        .map(|entry| {
            serde_json::json!({
                "id": entry.id,
                "kind": entry.kind,
                "tier": entry.tier,
                "confidence": entry.confidence,
                "tags": entry.tags,
                "created_at": entry.created_at.to_rfc3339(),
                "preview": entry.content_preview,
            })
        })
        .collect();
    let store = state.layout.root().join("neuro").join("knowledge.jsonl");
    serde_json::json!({
        "store": display_rel(&state.workdir, &store),
        "entries": entries.len(),
        "recent_entries": recent,
    })
}

fn plans_json(state: &ShowState) -> Value {
    let current = state.data.current_plan_execution.as_ref().map(|current| {
        serde_json::json!({
            "plan_id": current.plan_id,
            "title": current.plan_title,
            "tasks_done": current.tasks_done,
            "tasks_total": current.tasks_total,
            "current_task": current.current_task.as_ref().map(|task| {
                serde_json::json!({ "task_id": task.task_id, "description": task.description })
            }),
        })
    });
    let plans: Vec<Value> = state
        .data
        .plans
        .iter()
        .map(|plan| {
            serde_json::json!({
                "id": plan.id,
                "title": plan.title,
                "status": plan_status(plan.completed, plan.tasks_done, plan.tasks_failed),
                "task_count": plan.task_count,
                "tasks_done": plan.tasks_done,
                "tasks_failed": plan.tasks_failed,
                "last_error": plan.last_error.as_deref().filter(|error| !error.is_empty()),
            })
        })
        .collect();
    serde_json::json!({ "current": current, "plans": plans })
}

fn learning_json(state: &ShowState) -> Value {
    let router = &state.data.cascade_router;
    let mut stats: Vec<_> = router.confidence_stats.iter().collect();
    stats.sort_by(|left, right| left.0.cmp(right.0));
    let routing: Vec<Value> = stats
        .into_iter()
        .map(|(model, stats)| {
            serde_json::json!({
                "model": model,
                "trials": stats.trials,
                "successes": stats.successes,
                "success_rate": ratio(stats.successes as usize, stats.trials as usize),
            })
        })
        .collect();
    let experiments: Vec<Value> = state
        .data
        .experiments
        .iter()
        .take(10)
        .map(|experiment| {
            serde_json::json!({
                "id": experiment.experiment_id,
                "status": experiment.status,
                "active_variants": experiment.active_variants,
                "total_trials": experiment.total_trials,
                "winner": experiment.winner_id,
            })
        })
        .collect();
    let gates: Vec<Value> = state
        .data
        .gate_results_page
        .gate_rows
        .iter()
        .take(10)
        .map(|gate| {
            serde_json::json!({
                "gate": gate.gate_name,
                "pass_rate": gate.pass_rate,
                "runs": gate.total_runs,
                "avg_duration_ms": gate.avg_duration_ms,
            })
        })
        .collect();
    let cfactor = state.data.cfactor.as_ref().map(|cfactor| {
        serde_json::json!({
            "overall": cfactor.overall,
            "cost_efficiency": cfactor.components.cost_efficiency,
            "knowledge_growth": cfactor.components.knowledge_growth,
        })
    });
    serde_json::json!({
        "models": router.model_slugs,
        "routing": routing,
        "experiments": experiments,
        "gates": gates,
        "cfactor": cfactor,
    })
}

fn history_json(state: &ShowState) -> Value {
    let mut events: Vec<_> = state.data.event_log.iter().collect();
    events.sort_by_key(|event| event.timestamp_ms);
    let state_events: Vec<Value> = events
        .iter()
        .rev()
        .take(20)
        .rev()
        .copied()
        .map(event_log_json)
        .collect();
    let recent_turns: Vec<Value> = state
        .data
        .efficiency_events
        .iter()
        .rev()
        .take(12)
        .rev()
        .map(|event| {
            serde_json::json!({
                "timestamp": event.timestamp,
                "plan_id": event.plan_id,
                "task_id": event.task_id,
                "agent_id": event.agent_id,
                "model": event.model,
                "cost_usd": event.cost_usd,
            })
        })
        .collect();
    serde_json::json!({ "state_events": state_events, "recent_turns": recent_turns })
}

fn work_detail_json(state: &ShowState, work_id: &str) -> Result<Value> {
    let Some(item) = state.work_items.iter().find(|item| item.id == work_id) else {
        anyhow::bail!(
            "`{work_id}` is not a recognised subject or work-item ID.\n\
            Valid subjects: overview, costs, agents, knowledge, plans, learning, history"
        );
    };
    let plan = state
        .data
        .plans
        .iter()
        .find(|plan| plan.id == item.id)
        .map(|plan| {
            serde_json::json!({
                "title": plan.title,
                "task_count": plan.task_count,
                "tasks_done": plan.tasks_done,
                "tasks_failed": plan.tasks_failed,
                "last_error": plan.last_error.as_deref().filter(|error| !error.is_empty()),
            })
        });
    let related = |plan_id: &str, task_id: &str| plan_id == item.id || task_id == item.id;
    let costs = cost_total(
        state
            .data
            .efficiency_events
            .iter()
            .filter(|event| related(&event.plan_id, &event.task_id)),
    );
    let history: Vec<Value> = state
        .data
        .event_log
        .iter()
        .filter(|event| related(&event.plan_id, &event.task_id))
        .take(12)
        .map(event_log_json)
        .collect();
    Ok(serde_json::json!({
        "item": work_item_json(state, item),
        "plan": plan,
        "costs": cost_json(&costs),
        "history": history,
    }))
}

fn collect_work_items(
    workdir: &Path,
    layout: &RokoLayout,
    data: &DashboardData,
) -> Vec<WorkItemSummary> {
    let mut items = BTreeMap::<String, WorkItemSummary>::new();
    let plan_costs = cost_by_plan(data);

    for plan in &data.plans {
        let plan_path = roko_cli::plan::plans_dir(workdir).join(&plan.id);
        items.insert(
            plan.id.clone(),
            WorkItemSummary {
                id: plan.id.clone(),
                kind: String::from("plan"),
                status: plan_status(plan.completed, plan.tasks_done, plan.tasks_failed),
                prompt: plan.title.clone(),
                tasks_done: Some(plan.tasks_done),
                tasks_total: Some(plan.task_count),
                cost_usd: plan_costs.get(&plan.id).copied(),
                created: None,
                source: plan_path.clone(),
                modified_ms: path_modified_ms(&plan_path),
            },
        );
    }

    for path in read_json_paths(&layout.root().join("jobs")) {
        if let Some(item) = work_item_from_json_path(&path, "job") {
            items.insert(item.id.clone(), item);
        }
    }

    for path in read_json_paths(&layout.root().join("work")) {
        if let Some(item) = work_item_from_json_path(&path, "work") {
            items.insert(item.id.clone(), item);
        }
    }

    if let Some(current) = &data.current_plan_execution
        && !current.plan_id.is_empty()
    {
        let runner_source = data
            .runner_projection_path()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| workdir.join(roko_runtime::STATE_SNAPSHOT_RELATIVE_PATH));
        items.insert(
            current.plan_id.clone(),
            WorkItemSummary {
                id: current.plan_id.clone(),
                kind: String::from("current-plan"),
                status: String::from("running"),
                prompt: current.plan_title.clone(),
                tasks_done: Some(current.tasks_done),
                tasks_total: Some(current.tasks_total),
                cost_usd: plan_costs.get(&current.plan_id).copied(),
                created: None,
                source: runner_source.clone(),
                modified_ms: path_modified_ms(&runner_source),
            },
        );
    }

    let mut values = items.into_values().collect::<Vec<_>>();
    values.sort_by(|left, right| {
        right
            .modified_ms
            .cmp(&left.modified_ms)
            .then_with(|| left.id.cmp(&right.id))
    });
    values
}

fn work_item_from_json_path(path: &Path, kind: &str) -> Option<WorkItemSummary> {
    let text = fs::read_to_string(path).ok()?;
    let value = serde_json::from_str::<Value>(&text).ok()?;
    let id = value_string(&value, &["id", "work_id", "job_id"])
        .or_else(|| file_stem(path))
        .unwrap_or_else(|| String::from("unknown"));
    let status =
        value_string(&value, &["status", "state"]).unwrap_or_else(|| String::from("recorded"));
    let prompt =
        value_string(&value, &["prompt", "intent", "title", "description"]).unwrap_or_default();
    let tasks_done = value_usize_path(
        &value,
        &[
            &["tasks_completed"][..],
            &["tasks_done"][..],
            &["progress", "done"][..],
            &["cost", "tasks_completed"][..],
        ],
    );
    let tasks_total = value_usize_path(
        &value,
        &[
            &["tasks_total"][..],
            &["task_count"][..],
            &["progress", "total"][..],
            &["cost", "tasks_total"][..],
        ],
    );
    let cost_usd = value_f64_path(
        &value,
        &[
            &["cost_usd"][..],
            &["total_cost_usd"][..],
            &["cost", "total_usd"][..],
            &["cost", "usd"][..],
            &["cost", "total"][..],
            &["cost_summary", "total_usd"][..],
        ],
    );
    let created = value_string(
        &value,
        &["created", "created_at", "started_at", "updated_at"],
    );
    Some(WorkItemSummary {
        id,
        kind: String::from(kind),
        status,
        prompt,
        tasks_done,
        tasks_total,
        cost_usd,
        created,
        source: path.to_path_buf(),
        modified_ms: path_modified_ms(path),
    })
}

fn read_json_paths(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut paths = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
        .collect::<Vec<_>>();
    paths.sort();
    paths
}

/// The efficiency events in the `--since` window.
fn window_events(state: &ShowState) -> Vec<&AgentEfficiencyEvent> {
    state
        .data
        .efficiency_events
        .iter()
        .filter(|event| state.window.contains(&event.timestamp))
        .collect()
}

fn cost_total<'a>(events: impl IntoIterator<Item = &'a AgentEfficiencyEvent>) -> CostAggregate {
    let mut total = CostAggregate::default();
    for event in events {
        total.record(event);
    }
    total
}

/// The gate pass rate over the turns that carry a verdict. Most turns carry none (a task's
/// earlier turns, provider failures, ungated runs), so dividing by every turn understates it.
fn format_pass_rate(total: &CostAggregate) -> String {
    if total.verdicts == 0 {
        return String::from("no gate verdicts");
    }
    format!(
        "{} ({} of {} gate verdict(s))",
        format_percent(ratio(total.passed, total.verdicts)),
        total.passed,
        total.verdicts
    )
}

fn no_cost_events(kind: &str, window: ActivityWindow) -> String {
    format!(
        "No {kind} cost events in .roko/learn/efficiency.jsonl ({}).",
        window.label()
    )
}

fn cost_by_model(events: &[&AgentEfficiencyEvent]) -> BTreeMap<String, CostAggregate> {
    let mut rows = BTreeMap::<String, CostAggregate>::new();
    for event in events {
        let aggregate = rows.entry(non_empty(&event.model, "unknown")).or_default();
        aggregate.record(event);
    }
    rows
}

fn cost_by_task(events: &[&AgentEfficiencyEvent]) -> Vec<(String, CostAggregate)> {
    let mut rows = BTreeMap::<String, CostAggregate>::new();
    for event in events {
        let task = if event.plan_id.is_empty() {
            non_empty(&event.task_id, "unknown-task")
        } else {
            format!("{}:{}", event.plan_id, non_empty(&event.task_id, "task"))
        };
        rows.entry(task).or_default().record(event);
    }
    let mut rows = rows.into_iter().collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .1
            .cost_usd
            .partial_cmp(&left.1.cost_usd)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.0.cmp(&right.0))
    });
    rows
}

fn cost_by_day(events: &[&AgentEfficiencyEvent]) -> BTreeMap<String, CostAggregate> {
    let mut rows = BTreeMap::<String, CostAggregate>::new();
    for event in events {
        // The event's UTC date. A timestamp that does not parse is undated,
        // never cut at a byte offset, which panics inside a multi-byte char.
        let day = match event_time(&event.timestamp) {
            Some(time) => time.format("%Y-%m-%d").to_string(),
            None => String::from("undated"),
        };
        rows.entry(day).or_default().record(event);
    }
    rows
}

fn cost_by_plan(data: &DashboardData) -> HashMap<String, f64> {
    let mut rows = HashMap::<String, f64>::new();
    for event in &data.efficiency_events {
        if !event.plan_id.is_empty() {
            *rows.entry(event.plan_id.clone()).or_default() += event.cost_usd;
        }
    }
    rows
}

#[derive(Debug)]
struct AgentRow {
    id: String,
    summary: String,
}

/// The agents an activity view lists, and how many it leaves out.
#[derive(Debug, Default)]
struct AgentRows {
    /// Agents in the durable Runner projection, then agents whose last efficiency event is in
    /// the window, most recently active first.
    rows: Vec<AgentRow>,
    /// Agents whose last efficiency event is before the window.
    stale: usize,
}

fn agent_rows(state: &ShowState) -> AgentRows {
    agent_rows_in_window(
        &state.data.agents,
        &state.data.efficiency_events,
        state.window,
    )
}

fn agent_rows_in_window(
    agents: &[AgentSummary],
    events: &[AgentEfficiencyEvent],
    window: ActivityWindow,
) -> AgentRows {
    let mut projected = BTreeMap::<String, AgentRow>::new();
    for agent in agents {
        let scope = agent.plan_id.as_deref().unwrap_or("workspace");
        projected.insert(
            agent.id.clone(),
            AgentRow {
                id: agent.id.clone(),
                summary: format!("{} | {} | {}", agent.label, agent.status, scope),
            },
        );
    }

    // Each other agent's most recent event; of two with the same time, the later line wins.
    let mut latest = BTreeMap::<&str, (Option<DateTime<Utc>>, &AgentEfficiencyEvent)>::new();
    for event in events {
        if projected.contains_key(event.agent_id.as_str()) {
            continue;
        }
        let time = event_time(&event.timestamp);
        let entry = latest
            .entry(event.agent_id.as_str())
            .or_insert((time, event));
        if time >= entry.0 {
            *entry = (time, event);
        }
    }

    let mut active = Vec::new();
    let mut stale = 0;
    for (agent_id, (time, event)) in latest {
        if !window.includes(time) {
            stale += 1;
            continue;
        }
        let row = AgentRow {
            id: agent_id.to_string(),
            summary: format!(
                "{} | {} | {} | last {}",
                non_empty(&event.role, "agent"),
                non_empty(&event.model, "unknown-model"),
                non_empty(&event.plan_id, "workspace"),
                non_empty(&event.timestamp, "unknown-time")
            ),
        };
        active.push((time, row));
    }
    // `latest` iterates in id order and the sort is stable, so equal times stay in id order.
    active.sort_by_key(|(time, _)| std::cmp::Reverse(*time));

    AgentRows {
        rows: projected
            .into_values()
            .chain(active.into_iter().map(|(_, row)| row))
            .collect(),
        stale,
    }
}

/// Count the agents the window leaves out, so they are hidden rather than lost.
fn push_stale_agents(out: &mut String, agents: &AgentRows, window: ActivityWindow) {
    if agents.stale > 0 {
        push_empty(
            out,
            &format!(
                "{} older agent(s) hidden (no activity {}); use --since all to list them.",
                agents.stale,
                window.label()
            ),
        );
    }
}

fn push_learning_summary(state: &ShowState, out: &mut String) {
    let router = &state.data.cascade_router;
    let mut trials = 0usize;
    let mut successes = 0usize;
    for stats in router.confidence_stats.values() {
        trials += stats.trials as usize;
        successes += stats.successes as usize;
    }
    push_kv(
        out,
        "routing confidence",
        &format_percent(ratio(successes, trials)),
    );
    push_kv(out, "routing trials", &trials.to_string());
    push_kv(
        out,
        "experiments",
        &state.data.experiments.len().to_string(),
    );
    if let Some(cfactor) = &state.data.cfactor {
        push_kv(out, "c-factor", &format!("{:.2}", cfactor.overall));
    } else {
        push_kv(out, "c-factor", "no snapshots");
    }
}

fn push_work_item_row(out: &mut String, item: &WorkItemSummary) {
    let tasks = match (item.tasks_done, item.tasks_total) {
        (Some(done), Some(total)) => format!(" | tasks {done}/{total}"),
        _ => String::new(),
    };
    let cost = item
        .cost_usd
        .map(|cost| format!(" | {}", format_cost(cost)))
        .unwrap_or_default();
    let prompt = if item.prompt.is_empty() {
        String::new()
    } else {
        format!(" | {}", truncate(&item.prompt, 64))
    };
    push_kv(
        out,
        &item.id,
        &format!("{} | {}{}{}{}", item.status, item.kind, tasks, cost, prompt),
    );
}

fn header(state: &ShowState, title: &str) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "roko show {title}");
    let _ = writeln!(out, "workspace: {}", state.workdir.display());
    let _ = writeln!(out, "state: {}", state.layout.root().display());
    let _ = writeln!(
        out,
        "runner projection: {}",
        state.data.runner_projection_status()
    );
    if let Some(path) = state.data.runner_projection_path() {
        let _ = writeln!(out, "runner source: {}", path.display());
    }
    if let Some(generation) = state.data.runner_projection_generation() {
        let _ = writeln!(out, "runner generation: {generation}");
    }
    if let Some(error) = state.data.runner_projection_error() {
        let _ = writeln!(out, "runner error: {error}");
    }
    out
}

/// The header line naming the `--since` window of an activity view.
fn push_window(out: &mut String, window: ActivityWindow) {
    let _ = writeln!(out, "window: {}", window.label());
}

fn push_section(out: &mut String, title: &str) {
    let _ = writeln!(out);
    let _ = writeln!(out, "{}", title.to_ascii_uppercase());
}

fn push_kv(out: &mut String, key: &str, value: &str) {
    let _ = writeln!(out, "  {:<22} {}", truncate(key, 22), value);
}

fn push_empty(out: &mut String, message: &str) {
    let _ = writeln!(out, "  {message}");
}

fn plan_status(completed: bool, tasks_done: usize, tasks_failed: usize) -> String {
    if completed {
        String::from("done")
    } else if tasks_failed > 0 {
        String::from("failed")
    } else if tasks_done > 0 {
        String::from("running")
    } else {
        String::from("pending")
    }
}

fn event_scope(plan_id: &str, task_id: &str) -> String {
    match (plan_id.is_empty(), task_id.is_empty()) {
        (true, true) => String::from("workspace"),
        (false, true) => plan_id.to_string(),
        (true, false) => task_id.to_string(),
        (false, false) => format!("{plan_id}:{task_id}"),
    }
}

fn display_rel(workdir: &Path, path: &Path) -> String {
    path.strip_prefix(workdir)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn path_modified_ms(path: &Path) -> u64 {
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
}

fn file_stem(path: &Path) -> Option<String> {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .map(ToOwned::to_owned)
}

fn value_string(value: &Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        let Some(found) = value.get(*key) else {
            continue;
        };
        if let Some(text) = found.as_str().filter(|text| !text.trim().is_empty()) {
            return Some(text.to_string());
        }
        if found.is_number() || found.is_boolean() {
            return Some(found.to_string());
        }
    }
    None
}

fn value_usize_path(value: &Value, paths: &[&[&str]]) -> Option<usize> {
    value_number_path(value, paths).and_then(|number| {
        if number >= 0.0 {
            Some(number as usize)
        } else {
            None
        }
    })
}

fn value_f64_path(value: &Value, paths: &[&[&str]]) -> Option<f64> {
    value_number_path(value, paths)
}

fn value_number_path(value: &Value, paths: &[&[&str]]) -> Option<f64> {
    for path in paths {
        let mut current = value;
        let mut missing = false;
        for key in *path {
            if let Some(next) = current.get(*key) {
                current = next;
            } else {
                missing = true;
                break;
            }
        }
        if missing {
            continue;
        }
        if let Some(number) = current.as_f64() {
            return Some(number);
        }
        if let Some(text) = current.as_str()
            && let Ok(number) = text.parse::<f64>()
        {
            return Some(number);
        }
    }
    None
}

fn ratio(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn format_percent(value: f64) -> String {
    format!("{:.1}%", value * 100.0)
}

fn format_cost(value: f64) -> String {
    format!("${value:.4}")
}

fn format_count(value: u64) -> String {
    let text = value.to_string();
    let mut out = String::new();
    for (idx, ch) in text.chars().rev().enumerate() {
        if idx > 0 && idx % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out.chars().rev().collect()
}

fn format_duration_ms(value: f64) -> String {
    if value >= 1000.0 {
        format!("{:.2}s", value / 1000.0)
    } else {
        format!("{value:.0}ms")
    }
}

fn non_empty(value: &str, fallback: &str) -> String {
    if value.trim().is_empty() {
        fallback.to_string()
    } else {
        value.to_string()
    }
}

fn truncate(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let mut out = value
        .chars()
        .take(max_chars.saturating_sub(1))
        .collect::<String>();
    out.push('.');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(text: &str) -> DateTime<Utc> {
        event_time(text).expect("an RFC 3339 test time")
    }

    fn since(text: &str) -> ActivityWindow {
        ActivityWindow::Since(at(text))
    }

    fn efficiency_event(agent_id: &str, timestamp: &str) -> AgentEfficiencyEvent {
        AgentEfficiencyEvent {
            agent_id: agent_id.to_string(),
            role: String::from("implementer"),
            model: String::from("claude-sonnet-4-6"),
            plan_id: String::from("plan-a"),
            task_id: String::from("T01"),
            timestamp: timestamp.to_string(),
            ..AgentEfficiencyEvent::default()
        }
    }

    fn live_agent(id: &str) -> AgentSummary {
        AgentSummary {
            id: id.to_string(),
            label: id.to_string(),
            plan_id: Some(String::from("plan-a")),
            status: String::from("agent_running"),
        }
    }

    fn row_ids(agents: &AgentRows) -> Vec<&str> {
        agents.rows.iter().map(|row| row.id.as_str()).collect()
    }

    fn show_state(events: &[AgentEfficiencyEvent], window: ActivityWindow) -> ShowState {
        let mut data = DashboardData::default();
        data.efficiency_events = events.to_vec();
        ShowState {
            workdir: PathBuf::from("/workspace"),
            layout: RokoLayout::for_project("/workspace"),
            data,
            work_items: Vec::new(),
            window,
            recent_runs: Vec::new(),
        }
    }

    /// One `push_kv` line, as the views print it.
    fn kv(key: &str, value: &str) -> String {
        let mut out = String::new();
        push_kv(&mut out, key, value);
        out
    }

    #[test]
    fn show_costs_excludes_events_outside_window() {
        let now = at("2026-10-01T12:00:00Z");
        let window = ActivityWindow::parse(None, now).expect("default");
        let events = [
            AgentEfficiencyEvent {
                cost_usd: 100.0,
                gate_passed: Some(false),
                ..efficiency_event("H11:12", "2026-05-10T09:00:00+00:00")
            },
            // A task's earlier turn carries no verdict, so it leaves the pass rate alone.
            AgentEfficiencyEvent {
                cost_usd: 0.75,
                ..efficiency_event("T02:1", "2026-09-30T09:00:00+00:00")
            },
            AgentEfficiencyEvent {
                cost_usd: 0.25,
                gate_passed: Some(true),
                ..efficiency_event("T02:1", "2026-09-30T10:00:00+00:00")
            },
        ];

        let costs = render_costs(&show_state(&events, window));
        assert!(
            costs.contains("window: since 2026-09-24 12:00 UTC\n"),
            "{costs}"
        );
        for (key, value) in [
            ("turns", "2"),
            ("total cost", "$1.0000"),
            ("gate pass rate", "100.0% (1 of 1 gate verdict(s))"),
            ("all time", "$101.0000 across 3 turn(s)"),
            ("plan-a:T01", "$1.0000 | 2 turn(s)"),
            ("2026-09-30", "$1.0000 | 2 turn(s)"),
        ] {
            assert!(costs.contains(&kv(key, value)), "{key}: {costs}");
        }
        assert!(!costs.contains("2026-05-10"), "{costs}");
        let overview = render_overview(&show_state(&events, window));
        assert!(
            overview.contains(&kv("total", "$1.0000 across 2 turn(s)")),
            "{overview}"
        );

        let costs = render_costs(&show_state(&events, ActivityWindow::All));
        for (key, value) in [
            ("turns", "3"),
            ("total cost", "$101.0000"),
            ("gate pass rate", "50.0% (1 of 2 gate verdict(s))"),
            ("2026-05-10", "$100.0000 | 1 turn(s)"),
        ] {
            assert!(costs.contains(&kv(key, value)), "{key}: {costs}");
        }
        assert!(!costs.contains("all time  "), "{costs}");
    }

    /// backlog 2122: `roko show costs` lists the latest plan runs from
    /// `.roko/learn/run-metrics.jsonl`, newest first, with their tasks,
    /// tokens, cost and duration, within the `--since` window.
    #[test]
    fn show_costs_lists_recent_runs() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join(".roko/learn/run-metrics.jsonl");
        for (run_id, timestamp, cost) in [
            ("run-old", "2026-09-30T08:00:00Z", 0.25),
            ("run-new", "2026-10-01T09:30:00Z", 1.5),
        ] {
            let record = RunMetricsRecord {
                run_id: run_id.to_string(),
                timestamp: timestamp.to_string(),
                duration_ms: 90_000,
                total_tasks: 3,
                tasks_completed: 2,
                tasks_already_satisfied: 0,
                tasks_failed: 1,
                tasks_unverified: 0,
                tasks_skipped: 0,
                total_cost_usd: cost,
                total_tokens_in: 12_000,
                total_tokens_out: 3_400,
                total_agent_calls: 4,
                budget_exhausted: false,
                plans: Vec::new(),
            };
            roko_learn::run_metrics::append_run_metrics(&path, &record).expect("append");
        }

        let costs = render_costs(&load_show_state(temp.path(), ActivityWindow::All));
        let newest = kv(
            "2026-10-01 09:30",
            "run-new | 2 completed, 1 failed, 0 unverified | 12,000 in / 3,400 out | $1.5000 \
             | 90.00s",
        );
        let (Some(new_at), Some(old_at)) = (costs.find(&newest), costs.find("run-old")) else {
            panic!("both runs are listed: {costs}");
        };
        assert!(new_at < old_at, "newest first: {costs}");

        let window = since("2026-10-01T00:00:00Z");
        let recent = render_costs(&load_show_state(temp.path(), window));
        assert!(recent.contains("run-new"), "{recent}");
        assert!(!recent.contains("run-old"), "{recent}");
    }

    #[test]
    fn show_since_parses_spans_dates_times_and_all() {
        let now = at("2026-10-01T12:00:00Z");
        let parse = |value| ActivityWindow::parse(Some(value), now).expect(value);
        assert_eq!(
            ActivityWindow::parse(None, now).expect("default"),
            since("2026-09-24T12:00:00Z")
        );
        assert_eq!(parse("all"), ActivityWindow::All);
        assert_eq!(parse("24h"), since("2026-09-30T12:00:00Z"));
        assert_eq!(parse("2w"), since("2026-09-17T12:00:00Z"));
        assert_eq!(parse("2026-09-29"), since("2026-09-29T00:00:00Z"));
        assert_eq!(
            parse("2026-09-29T06:30:00+02:00"),
            since("2026-09-29T04:30:00Z")
        );
        for bad in ["yesterday", "0d", "-7d", "7y", ""] {
            assert!(ActivityWindow::parse(Some(bad), now).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn show_agents_marks_or_omits_stale_agents() {
        let now = at("2026-10-01T12:00:00Z");
        let window = ActivityWindow::parse(None, now).expect("default");
        let live = [live_agent("live-agent")];
        let events = [
            efficiency_event("H11:12", "2026-05-10T09:00:00+00:00"),
            efficiency_event("T02:1", "2026-09-29T08:00:00+00:00"),
            efficiency_event("T02:1", "2026-09-30T10:00:00+00:00"),
            // An older event later in the file does not replace the latest one.
            efficiency_event("T02:1", "2026-09-28T07:00:00+00:00"),
            efficiency_event("undated", ""),
        ];

        let agents = agent_rows_in_window(&live, &events, window);
        assert_eq!(row_ids(&agents), ["live-agent", "T02:1"]);
        let summary = &agents.rows[1].summary;
        assert!(
            summary.ends_with("| last 2026-09-30T10:00:00+00:00"),
            "{summary}"
        );
        assert_eq!(agents.stale, 2, "the May and undated agents");
        let mut out = String::new();
        push_stale_agents(&mut out, &agents, window);
        assert!(
            out.contains("2 older agent(s) hidden (no activity since 2026-09-24 12:00 UTC)"),
            "{out}"
        );

        let everything = agent_rows_in_window(&live, &events, ActivityWindow::All);
        assert_eq!(
            row_ids(&everything),
            ["live-agent", "T02:1", "H11:12", "undated"]
        );
        assert_eq!(everything.stale, 0);
    }

    #[test]
    fn show_json_renders_every_subject_over_the_window() {
        let now = at("2026-10-01T12:00:00Z");
        let window = ActivityWindow::parse(None, now).expect("default");
        let events = [
            AgentEfficiencyEvent {
                cost_usd: 100.0,
                ..efficiency_event("H11:12", "2026-05-10T09:00:00+00:00")
            },
            AgentEfficiencyEvent {
                cost_usd: 0.25,
                input_tokens: 400,
                output_tokens: 100,
                gate_passed: Some(true),
                ..efficiency_event("T02:1", "2026-09-30T10:00:00+00:00")
            },
        ];
        let state = show_state(&events, window);
        let json = |subject: &str| {
            render_json(&state, ShowTarget::parse(Some(subject.to_string()))).expect(subject)
        };

        let costs = json("costs");
        assert_eq!(costs["subject"], "costs");
        assert_eq!(costs["workdir"], "/workspace");
        assert_eq!(costs["window"]["since"], "2026-09-24T12:00:00+00:00");
        assert_eq!(costs["summary"]["turns"], 1);
        assert_eq!(costs["summary"]["cost_usd"], 0.25);
        assert_eq!(costs["summary"]["gate_passed"], 1);
        assert_eq!(costs["all_time"]["turns"], 2);
        assert_eq!(costs["by_model"][0]["model"], "claude-sonnet-4-6");
        assert_eq!(costs["by_task"][0]["task"], "plan-a:T01");
        assert_eq!(costs["by_day"][0]["day"], "2026-09-30");

        let overview = json("overview");
        assert_eq!(overview["costs"]["input_tokens"], 400);
        assert_eq!(overview["agents"][0]["id"], "T02:1");
        assert_eq!(overview["stale_agents_hidden"], 1);

        let agents = json("agents");
        assert_eq!(agents["agents"].as_array().map(Vec::len), Some(1));

        for (subject, key) in [
            ("knowledge", "entries"),
            ("plans", "plans"),
            ("learning", "routing"),
            ("history", "recent_turns"),
        ] {
            let value = json(subject);
            assert_eq!(value["subject"], subject);
            assert!(value.get(key).is_some(), "{subject}: {value}");
        }
        assert_eq!(json("history")["recent_turns"][1]["cost_usd"], 0.25);

        let all = render_json(
            &show_state(&events, ActivityWindow::All),
            ShowTarget::parse(None),
        )
        .expect("overview");
        assert_eq!(all["window"]["since"], Value::Null);
        assert_eq!(all["costs"]["turns"], 2);
    }

    #[test]
    fn show_json_rejects_an_unknown_work_id() {
        let state = show_state(&[], ActivityWindow::All);
        let error = render_json(&state, ShowTarget::WorkId(String::from("nope")))
            .expect_err("no such work item");
        assert!(
            error
                .to_string()
                .contains("`nope` is not a recognised subject")
        );
    }

    #[test]
    fn cost_by_day_never_slices_a_timestamp_by_bytes() {
        let events = [
            efficiency_event("a", "2026-09-30T10:00:00+00:00"),
            // Late on the 30th west of UTC is the 1st in UTC.
            efficiency_event("b", "2026-09-30T23:30:00-02:00"),
            // A two-byte character across byte 10 used to panic the slice.
            efficiency_event("c", "2026-09-3éT10:00:00Z"),
            efficiency_event("d", ""),
        ];
        let refs: Vec<&AgentEfficiencyEvent> = events.iter().collect();
        let by_date = cost_by_day(&refs);
        let turns_by_day: Vec<(&str, usize)> = by_date
            .iter()
            .map(|(day, aggregate)| (day.as_str(), aggregate.turns))
            .collect();
        assert_eq!(
            turns_by_day,
            [("2026-09-30", 1), ("2026-10-01", 1), ("undated", 2)]
        );
    }
}
