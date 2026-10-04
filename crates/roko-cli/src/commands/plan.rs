//! plan command handlers.

use crate::*;
use anyhow::Context as _;
use roko_cli::plan_validate;
use roko_fs::RokoLayout;

/// Execution engine for `roko plan run`.
///
/// The Graph Engine is the sole execution engine. The `--engine legacy` and
/// `--engine runner-v2` values are still accepted to avoid breaking existing
/// scripts, but they print a deprecation error and exit.
///
/// **#336**: The legacy engine variant is scheduled for removal. Scripts using
/// `--engine legacy` or `--engine runner-v2` should remove the flag entirely
/// (the graph engine is the default and sole engine).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum PlanEngine {
    /// Graph Engine (default and sole engine).
    #[default]
    #[value(name = "graph")]
    Graph,
    /// Legacy Runner-v2 (REMOVED). Accepted for backward compatibility but
    /// prints a deprecation error and exits. Use `--engine graph` (the default).
    /// Scheduled for removal in the next release (#336).
    #[value(name = "legacy", alias = "runner-v2", hide = true)]
    RunnerV2,
}

#[derive(Debug, Subcommand)]
pub(crate) enum PlanCmd {
    /// List all plans in the workspace.
    List {
        /// Working directory.
        #[arg(long)]
        workdir: Option<PathBuf>,
        /// Group plans by execution wave (cross-plan dependency analysis).
        #[arg(long)]
        waves: bool,
    },
    /// Show details of a specific plan.
    Show {
        /// Plan ID.
        plan_id: String,
        /// Working directory.
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Create a new plan.
    Create {
        /// Plan ID.
        plan_id: String,
        /// Plan title.
        #[arg(long)]
        title: String,
        /// Plan description.
        #[arg(long, default_value = "")]
        description: String,
        /// Working directory.
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Lint every `tasks.toml` under a plans directory without executing it.
    Validate {
        /// Plans root directory.
        #[arg(default_value = "plans/")]
        dir: PathBuf,
        /// Fail on warnings, not only errors.
        #[arg(long)]
        strict: bool,
        /// Output machine-readable JSON instead of text.
        #[arg(long)]
        json: bool,
        /// Show DAG analysis: plan/task/edge counts, wave breakdown,
        /// critical path, and dangling dependency references.
        #[arg(long)]
        dag: bool,
        /// Score each task's spec with the speclint rules (`sq-2`): score,
        /// band, rule scores and hard fails. With `--strict`, a hard fail
        /// exits 1.
        #[arg(long)]
        spec_quality: bool,
        /// With `--spec-quality`: first run each implementer task's verify
        /// steps twice on a clean checkout of its base commit, so the scores
        /// count SQ06 (red on base) and find HF3 (a check that already
        /// passes). No step runs in this checkout.
        #[arg(long, requires = "spec_quality")]
        dynamic: bool,
        /// With `--dynamic`: the commit to check every plan against (default:
        /// HEAD for a plan that has not run; none for the rest).
        #[arg(long, requires = "dynamic", value_name = "REV")]
        base: Option<String>,
        /// With `--dynamic`: the most seconds one verify step may run
        /// (default: `[spec_quality] red_on_base_timeout_secs`).
        #[arg(
            long,
            requires = "dynamic",
            value_name = "SECONDS",
            value_parser = clap::value_parser!(u64).range(1..)
        )]
        timeout: Option<u64>,
        /// With `--dynamic`: an existing directory outside every checkout for
        /// the base checkouts (default: the system temp directory).
        #[arg(long, requires = "dynamic", value_name = "DIR")]
        scratch: Option<PathBuf>,
        /// With `--dynamic`: the workspace is a plain directory; check it
        /// against a one-commit snapshot of itself.
        #[arg(long, requires = "dynamic")]
        fixture: bool,
    },
    /// Write a plan's companion documents beside its `tasks.toml`: `brief.md`
    /// (its artifacts, task map and risks), which dispatch adds to each of its
    /// task prompts. No model runs unless `--full`. Documents that exist are
    /// kept unless `--force`.
    Prepare {
        /// The plan directory, holding `tasks.toml`.
        plan_dir: PathBuf,
        /// Also have the planner model write `decomposition.md` (numbered
        /// steps with checkpoints) and `rubric.md` (review criteria).
        #[arg(long)]
        full: bool,
        /// Overwrite companion documents that exist.
        #[arg(long)]
        force: bool,
        /// Working directory.
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Rebuild or verify the deterministic plans index.
    Index {
        /// Verify exact generated content without writing any files.
        #[arg(long)]
        check: bool,
        /// Working directory.
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Run a plan directory through the orchestration loop.
    #[command(after_help = "\
Examples:
  roko plan run plans/              Run all plans (graph engine, default)
  roko plan run plans/my-plan       Run a specific plan
  roko plan run plans/ --no-tui     Plain log output instead of the inline TUI
  roko plan run plans/ --dry-run    Preview without executing
  roko plan run plans/ --fresh      Archive old state and start clean
  roko plan run plans/ --max-parallel-plans 3   Run up to 3 independent plans at once
  roko plan run plans/ --resume-plan .roko/state/graph                              Resume Graph Activities

A plan with [meta] approval = \"per_task\" holds each verified task until it is approved or
rejected: in the inline TUI (y or n), with `roko plan review`, or with the portal's Review action.

The legacy Runner-v2 engine has been removed. --engine legacy is accepted but exits with an error.")]
    Run {
        /// Path to the plans directory.
        plans_dir: PathBuf,
        /// Execution engine to use for plan execution.
        ///
        /// The Graph engine is the sole engine. `--engine legacy` and
        /// `--engine runner-v2` are accepted for backward compatibility
        /// but print a deprecation error and exit.
        #[arg(long, default_value = "graph", value_enum)]
        engine: PlanEngine,
        /// Working directory (repo root). Defaults to current directory.
        #[arg(long)]
        workdir: Option<PathBuf>,
        /// Resume from engine state. Bare `--resume-plan` resumes the canonical
        /// Graph checkpoints under `.roko/state/graph/` (any number of plans);
        /// pass a checkpoint directory, or a checkpoint file for a single plan.
        #[arg(long = "resume-plan", visible_alias = "resume-state", num_args = 0..=1, default_missing_value = ".roko/state/state-snapshot.json")]
        resume_plan: Option<PathBuf>,
        /// Open the inline TUI, which shows agent output, tokens and gate
        /// progress live and closes when the run ends. The run already opens it
        /// whenever stdout is a terminal, unless `--no-tui`, `--quiet` or
        /// `--json` is given, so this flag changes nothing; scripts may pass it.
        ///
        /// A plan with `[meta] approval = "per_task"` holds each verified task
        /// until someone approves or rejects it: in this TUI (y or n), with
        /// `roko plan review <plan> <task> --approve|--reject`, or with the
        /// Review action in the portal.
        #[arg(long, visible_alias = "tui")]
        approval: bool,
        /// Disable the inline TUI even in interactive terminals.
        ///
        /// By default, the TUI is auto-enabled when stdout is a TTY.
        /// Pass `--no-tui` to suppress it and use plain log output instead.
        #[arg(long)]
        no_tui: bool,
        /// Maximum retry attempts per task (overrides per-task and config values).
        #[arg(long)]
        max_retries: Option<u32>,
        /// Maximum concurrent tasks per plan (0 keeps the config/default value).
        #[arg(long, default_value_t = 0)]
        max_tasks: usize,
        /// Parse and display the plan without executing. Shows tasks, dependencies, and estimates.
        #[arg(long)]
        dry_run: bool,
        /// Archive old run state and start from scratch (ignores the unified state snapshot and legacy files).
        #[arg(long)]
        fresh: bool,
        /// Re-queue drifted tasks instead of aborting when resuming from a snapshot.
        #[arg(long)]
        force_resume: bool,
        /// Override the plan cost ceiling for this run.
        ///
        /// `--budget-override 50.0` sets the per-plan USD ceiling to $50.00,
        /// replacing whatever is configured in roko.toml. Once the plan has
        /// spent it, no further task starts, as with a configured ceiling.
        /// `--budget-override 0` removes the plan ceiling; the per-task and
        /// daily ceilings still apply.
        #[arg(long, value_name = "AMOUNT")]
        budget_override: Option<f64>,
        /// Disable budget enforcement entirely for this run.
        ///
        /// No plan, per-task or daily ceiling stops a dispatch; spend is still
        /// recorded.
        #[arg(long, conflicts_with = "budget_override")]
        no_budget: bool,
        /// Hold learned state fixed for this run (decision 2218): it reads
        /// learned state as usual and writes none, while telemetry stays on.
        /// `[learning] frozen = true` in roko.toml does the same for every
        /// run. The run manifest records `ablation_flags = ["learning_frozen"]`.
        #[arg(long)]
        frozen_learning: bool,
        /// Run in maximize mode (decision 4115): no learning loop is
        /// withheld and no route explores, for this run alone, while every
        /// decision is still logged. `[experiments] maximize = true` in
        /// roko.toml does the same for every run.
        #[arg(long)]
        no_holdout: bool,
        /// Skip the disk-space pre-check and start the plan even when free disk
        /// is below `resources.min_free_disk_mb`. Use with caution: the plan
        /// may fail mid-run if disk space is exhausted.
        #[arg(long)]
        force: bool,
        /// Skip agent permission prompts for this run. UNSAFE: agents will execute
        /// tools without approval. Prefer setting `runner.dangerously_skip_permissions = true`
        /// in roko.toml for persistent use.
        #[arg(long)]
        dangerously_skip_permissions: bool,
        /// Write structured JSONL event log to this file during execution.
        ///
        /// Every runner lifecycle event (task start, gate result, agent dispatch,
        /// run completion, etc.) is serialized as a single JSON line and flushed.
        #[arg(long, value_name = "PATH")]
        log_file: Option<PathBuf>,
        /// Rejected: the Graph engine always runs its provider preflight. Hidden
        /// from `--help`; parsed so `plan run` can say so (gap-d60281).
        #[arg(long, hide = true)]
        skip_preflight: bool,
        // NOTE: `--force-backend` was removed from this subcommand and
        // consolidated into the global `--model` flag (hidden alias).
        // Use `roko plan run --model <slug> plans/` instead.
        /// Rejected: plan runs take no screenshots (use `roko screenshot`).
        /// Hidden from `--help`; parsed so `plan run` can say so (gap-d60281).
        #[arg(long, hide = true)]
        screenshots: bool,
        /// Rejected with `--screenshots`; hidden from `--help`.
        #[arg(
            long,
            hide = true,
            value_name = "SECONDS",
            default_value_t = 60,
            value_parser = clap::value_parser!(u64).range(1..=86_400)
        )]
        screenshot_interval: u64,
        /// Rejected with `--screenshots`; hidden from `--help`.
        #[arg(long, hide = true, value_name = "PATH")]
        screenshot_dir: Option<PathBuf>,
        /// Rejected: plan runs do not pause after N plans. Hidden from
        /// `--help`; parsed so `plan run` can say so (gap-d60281).
        #[arg(long, hide = true, value_name = "N")]
        batch_size: Option<usize>,
        /// Run each task in an isolated git worktree so agents cannot
        /// interfere with each other or the user's working tree. This is the
        /// default (`[runner] worktree_per_task = true`); the flag overrides
        /// a config that turns it off.
        ///
        /// Each task dispatch creates a fresh worktree, runs the agent and
        /// verify steps inside it, and cleans it up on completion. Failed
        /// worktrees are retained for post-mortem. Finished plans are
        /// delivered into the run's batch branch, `roko/batch/<run-id>`; your
        /// checkout is never changed, and the run ends with the command that
        /// takes the work into it. Without this flag, a workdir that is not
        /// the top level of a git checkout with a commit runs its tasks in
        /// the shared working tree. Only applies to the Graph engine.
        #[arg(long)]
        worktree_per_task: bool,
        /// Run every task in the shared working tree, whatever
        /// `[runner] worktree_per_task` says: tasks edit your checkout
        /// directly.
        #[arg(long, conflicts_with = "worktree_per_task")]
        no_worktree_per_task: bool,
        /// Use the rich 5-node-per-task production topology instead of the
        /// simple single-Activity-per-task converter.
        ///
        /// When enabled, each task becomes a subgraph of:
        ///   [TaskContext] -> [Compose] -> [TaskExecutor] -> [Gate] -> [SuccessBoundary]
        ///
        /// Each task's [Gate] runs the compile, lint and test rungs in the
        /// worktree its attempt ran in, and accepts the attempt onto the plan
        /// branch when they pass, so this needs per-task worktrees (the
        /// default; see `--worktree-per-task`). Only applies to the Graph
        /// engine.
        #[arg(long)]
        rich_topology: bool,
        /// After every plan is delivered into the run's batch branch
        /// (`roko/batch/<run-id>`), promote the batch into BRANCH and tag it
        /// `roko/run/<run-id>`. Never pushes. A BRANCH checked out anywhere,
        /// such as your own checkout's, is not moved: the promotion is parked
        /// at `refs/roko/delivered/run-<run-id>` for you to fast-forward.
        /// Needs per-task worktrees, the default (see `--worktree-per-task`).
        #[arg(long, value_name = "BRANCH", conflicts_with = "no_worktree_per_task")]
        promote: Option<String>,
        /// Run up to N plans of a plan set at the same time.
        ///
        /// Plans start in execution order once their `depends_on_plan`
        /// prerequisites have succeeded. Plans that write or build
        /// overlapping parts of the working tree never run at the same time.
        /// Defaults to `[conductor] max_parallel_plans` (1: one plan at a
        /// time). With per-task worktrees, each plan that finishes is
        /// delivered into the run's batch branch in turn.
        #[arg(
            long,
            value_name = "N",
            value_parser = clap::value_parser!(u64).range(1..=64)
        )]
        max_parallel_plans: Option<u64>,
        /// Start no further plans after the first plan fails. Plans already
        /// running finish; the rest are reported as blocked.
        #[arg(long)]
        fail_fast: bool,
    },
    /// Generate a plan from a prompt or a file (a spec, requirements, notes).
    Generate {
        /// Source: free-text prompt, or path to a file (spec, requirements, etc).
        source: Vec<String>,
        /// Treat source as a file path to read (instead of inline text).
        #[arg(long)]
        from_file: Option<PathBuf>,
        /// Additional context files/dirs/globs to include in the prompt.
        #[arg(long = "context", value_name = "PATH")]
        context: Vec<PathBuf>,
        /// Read notes from .roko/notes/ and generate one plan per cluster.
        #[arg(long)]
        from_notes: bool,
        /// Filter notes by tag when using --from-notes.
        #[arg(long)]
        tag: Option<String>,
        /// Generate plan(s) from backlog spec(s). Accepts a single ID or
        /// comma-separated IDs: `--from-backlog 206` or `--from-backlog 206,120,119`.
        /// Reads the spec from `tmp/backlog/<id>-*.md`, generates a deterministic
        /// slug, and writes the plan to `plans/<slug>/tasks.toml`.
        #[arg(long, value_name = "IDS")]
        from_backlog: Option<String>,
    },
    /// Pause the plan run in this workspace: no new plan, task or retry
    /// starts until resume, and running attempts finish. Prints the run's
    /// answer; fails when no run is listening.
    Pause {
        /// Working directory.
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Resume a paused plan run. Prints the run's answer; fails when no run
    /// is listening.
    Resume {
        /// Working directory.
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Cancel a plan of the running plan run, or every running plan. Prints
    /// the run's answer; fails when no run is listening or it refuses.
    Cancel {
        /// Plan ID to cancel. If omitted, cancels the current run.
        #[arg(long)]
        plan_id: Option<String>,
        /// Working directory.
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Run a plan that failed or was cancelled in the running plan run again,
    /// from its checkpoint. Prints the run's answer; fails when no run is
    /// listening or it refuses.
    Retry {
        /// Task ID. A Graph run reruns the whole plan from its checkpoint, so
        /// its passed tasks stay done.
        task_id: Option<String>,
        /// The plan to run again.
        #[arg(long)]
        plan_id: Option<String>,
        /// Working directory.
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Act on the budget of a running plan (backlog 2118).
    Budget {
        #[command(subcommand)]
        cmd: PlanBudgetCmd,
    },
    /// Approve or reject a task held for review (`[meta] approval =
    /// "per_task"`). The plan run holding it merges the task on approval; a
    /// rejection fails the attempt, and the note is the next attempt's
    /// feedback.
    Review {
        /// Plan id.
        plan_id: String,
        /// Task id.
        task_id: String,
        /// Approve the task's held attempt.
        #[arg(long, conflicts_with = "reject", required_unless_present = "reject")]
        approve: bool,
        /// Reject the task's held attempt.
        #[arg(long)]
        reject: bool,
        /// The reviewer's note, which a rejected task's next attempt gets.
        #[arg(long, default_value = "")]
        note: String,
        /// Working directory.
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Regenerate an existing plan from its `plan.md`.
    Regenerate {
        /// Path to the plan directory (containing tasks.toml).
        plan_dir: PathBuf,
        /// Preview changes without overwriting.
        #[arg(long)]
        dry_run: bool,
    },
    /// Revise a plan from feedback with the planner model and print what
    /// changed, task by task. The planner also sees how the plan's last run
    /// failed, gate output included. The revision is written only when it
    /// validates; otherwise the command exits 1 and the plan is untouched.
    Revise {
        /// The plan: its directory, or its id.
        plan: String,
        /// What to change.
        #[arg(long)]
        feedback: String,
    },
    /// Queue manifest operations: show, validate, and init milestone definitions.
    Queue {
        #[command(subcommand)]
        cmd: QueueCmd,
    },
    /// Show the lightweight runner status from `.roko/state/status.json`.
    ///
    /// Reads the < 500 byte status file written by the runner on every tick
    /// (debounced 1/sec). This is fast because it does not require
    /// deserializing the full executor snapshot.
    ///
    /// When a plan directory is provided, shows task-level status for that
    /// specific plan (from its tasks.toml and executor snapshot).
    /// When omitted, shows the global runner status (phase, plans, agents).
    #[command(after_help = "\
Examples:
  roko plan status                          Show global runner status
  roko plan status plans/demos/demo-hello   Show task status for a specific plan")]
    Status {
        /// Optional plan directory to show status for (e.g. plans/my-plan).
        /// When provided, shows task-level status for that specific plan.
        /// When omitted, shows the global runner status.
        plan_dir: Option<PathBuf>,
        /// Working directory.
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Shorthand: `roko plan "add cursor support"` routes to plan generate.
    #[command(external_subcommand)]
    Shorthand(Vec<String>),
}

#[derive(Debug, Subcommand)]
pub(crate) enum PlanBudgetCmd {
    /// Raise the budget ceiling of a running plan for the rest of its run.
    /// The run keeps the new ceiling in the plan's costs.json, so a resume
    /// keeps it, and arms the plan's budget alerts again against it. Refused
    /// when the amount is not above the plan's ceiling and its spend. Prints
    /// the run's answer; fails when no run is listening or it refuses.
    Raise {
        /// The running plan.
        plan_id: String,
        /// The new ceiling, in USD.
        #[arg(long, value_name = "USD")]
        to: f64,
        /// Working directory.
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum QueueCmd {
    /// Display milestone status and plan assignments.
    Show {
        /// Path to queue manifest file.
        #[arg(long, default_value = ".roko/queue.toml")]
        file: PathBuf,
        /// Working directory.
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Validate queue manifest structure and plan references.
    Validate {
        /// Path to queue manifest file.
        #[arg(long, default_value = ".roko/queue.toml")]
        file: PathBuf,
        /// Working directory.
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Generate a starter queue.toml from discovered plans.
    Init {
        /// Output path for the generated manifest.
        #[arg(long, default_value = ".roko/queue.toml")]
        output: PathBuf,
        /// Working directory.
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
}

fn join_approval_tui_thread(handle: Option<std::thread::JoinHandle<anyhow::Result<()>>>) {
    let Some(handle) = handle else {
        return;
    };

    match handle.join() {
        Ok(Ok(())) => {}
        Ok(Err(err)) => {
            tracing::error!(error = %err, "approval TUI exited with error");
        }
        Err(_) => {
            tracing::error!("approval TUI thread panicked");
        }
    }
}

/// Send plan control command `kind` (`pause`, `resume`, `cancel` or `retry`)
/// for plan `plan_id`, or for the whole run, to the plan run listening in
/// `workdir`, over the socket `roko inject` uses, and print the run's answer
/// (1209). The exit code is non-zero when no run is listening or the run
/// refuses: nothing is written for a run that may never read it.
async fn send_plan_control(
    cli: &Cli,
    workdir: &Path,
    kind: &str,
    plan_id: Option<String>,
) -> Result<i32> {
    send_plan_control_with(cli, workdir, kind, plan_id, String::new()).await
}

/// `roko plan budget` (backlog 2118).
async fn cmd_plan_budget(cli: &Cli, cmd: PlanBudgetCmd) -> Result<i32> {
    match cmd {
        PlanBudgetCmd::Raise {
            plan_id,
            to,
            workdir,
        } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            raise_plan_budget(cli, &wd, plan_id, to).await
        }
    }
}

/// `roko plan budget raise` (backlog 2118): refuse a ceiling that is not
/// above what the plan's checkpoint says it has spent, then send the raise
/// to the plan run, which also refuses one that is not above the plan's
/// ceiling, and print its answer.
async fn raise_plan_budget(
    cli: &Cli,
    workdir: &Path,
    plan_id: String,
    ceiling_usd: f64,
) -> Result<i32> {
    let checkpoint = roko_cli::graph_checkpoint::inspect_canonical_checkpoint(workdir, &plan_id)?;
    let spent_micro_usd = checkpoint
        .and_then(|checkpoint| checkpoint.spent_micro_usd)
        .unwrap_or(0);
    let refusal = match roko_cli::graph_task_dispatch::plan_ceiling_micro_usd(ceiling_usd) {
        None => Some(format!(
            "--to {ceiling_usd} is not a ceiling: give a positive amount in USD"
        )),
        Some(ceiling) if ceiling <= spent_micro_usd => {
            let spent_usd = spent_micro_usd as f64 / 1_000_000.0;
            Some(format!(
                "plan {plan_id} has spent ${spent_usd:.4}: raise its ceiling above that"
            ))
        }
        Some(_) => None,
    };
    if let Some(refusal) = refusal {
        if cli.json {
            println!(
                "{}",
                serde_json::json!({
                    "code": "plan_control_refused",
                    "command": "raise_budget",
                    "plan_id": plan_id,
                    "message": refusal,
                })
            );
        } else {
            eprintln!("Error: {refusal}");
        }
        return Ok(EXIT_FAILURE);
    }
    let payload = ceiling_usd.to_string();
    send_plan_control_with(cli, workdir, "raise_budget", Some(plan_id), payload).await
}

/// [`send_plan_control`], with `payload`: for `raise_budget`, the plan's new
/// ceiling in USD.
async fn send_plan_control_with(
    cli: &Cli,
    workdir: &Path,
    kind: &str,
    plan_id: Option<String>,
    payload: String,
) -> Result<i32> {
    let request = roko_cli::inject::InjectWireRequest {
        request_id: uuid::Uuid::new_v4().to_string(),
        session: plan_id.clone().unwrap_or_default(),
        kind: kind.to_string(),
        payload,
    };
    let reply = roko_cli::inject::deliver(workdir, &request).await;
    let (code, message) = match &reply {
        Some(reply) if reply.outcome == roko_cli::inject::InjectOutcome::Accepted => {
            ("plan_control_accepted", reply.message.clone())
        }
        Some(reply) => ("plan_control_refused", reply.message.clone()),
        None => (
            "plan_control_no_run",
            format!("no plan run is listening in {}", workdir.display()),
        ),
    };
    let accepted = code == "plan_control_accepted";
    if cli.json {
        println!(
            "{}",
            serde_json::json!({
                "code": code,
                "command": kind,
                "plan_id": plan_id,
                "message": message,
            })
        );
    } else if accepted {
        if !cli.quiet {
            println!("{kind}: {message}");
        }
    } else if reply.is_none() {
        eprintln!("Error: {kind} was not delivered: {message}");
        eprintln!("Hint: start a run with `roko plan run`, or pass the run's --workdir.");
    } else {
        eprintln!("Error: the plan run refused {kind}: {message}");
    }
    Ok(if accepted { EXIT_SUCCESS } else { EXIT_FAILURE })
}

pub(crate) async fn cmd_plan(cli: &Cli, cmd: PlanCmd) -> Result<i32> {
    match cmd {
        PlanCmd::List { workdir, waves } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            // Read-only: skip the lock when a server owns the workspace (the
            // server is the only writer and operates atomically); otherwise
            // take the shared lock as usual.
            let _lock = roko_cli::serve_client::read_lock_unless_served(&wd)?;
            let summaries =
                roko_cli::plan::summarize_discovered_plans(&wd).map_err(|e| anyhow!("{e}"))?;
            let executor_state = read_executor_state(&wd);
            // A plan has run state if either the legacy executor snapshot
            // exists OR any graph-engine checkpoint directory is present.
            let has_graph_checkpoints = {
                let graph_root = wd.join(".roko/state/graph");
                graph_root.is_dir()
                    && std::fs::read_dir(&graph_root)
                        .map(|mut rd| rd.next().is_some())
                        .unwrap_or(false)
            };
            let has_run_state = executor_state.is_some() || has_graph_checkpoints;
            let state_entries = executor_state.clone().unwrap_or_default();
            let state_map: std::collections::HashMap<String, (usize, usize)> = state_entries
                .iter()
                .cloned()
                .map(|(id, done, total)| (id, (done, total)))
                .collect();

            let mut summaries = summaries;
            for summary in &mut summaries {
                if let Some((tasks_done, tasks_total)) = state_map.get(&summary.id).copied() {
                    summary.tasks_done = tasks_done;
                    summary.task_count = tasks_total;
                    summary.completed = tasks_total > 0 && tasks_done == tasks_total;
                }
            }

            if waves {
                // Load plans via runner plan_loader for cross-plan DAG analysis.
                let plans_dir = wd.join("plans");
                match roko_cli::runner::plan_loader::load_plans(&plans_dir) {
                    Ok(loaded_plans) => {
                        match roko_cli::runner::plan_dag::CrossPlanDag::compute(&loaded_plans) {
                            Ok(dag) => {
                                if cli.json {
                                    let dag_summary = dag.summary();
                                    println!("{}", serde_json::to_string_pretty(&dag_summary)?);
                                } else {
                                    for (wave_idx, wave_plans) in dag.waves.iter().enumerate() {
                                        let plan_count = wave_plans.len();
                                        println!(
                                            "\nWave {wave_idx} ({plan_count} plan{}):",
                                            if plan_count == 1 { "" } else { "s" }
                                        );
                                        for plan_id in wave_plans {
                                            let summary_info =
                                                summaries.iter().find(|s| s.id == *plan_id);
                                            let progress = summary_info.map_or_else(
                                                || "?/?".to_string(),
                                                |s| format!("{}/{}", s.tasks_done, s.task_count),
                                            );
                                            let status = summary_info.map_or_else(
                                                || "unknown".to_string(),
                                                |s| s.status_label().to_string(),
                                            );
                                            println!(
                                                "  {:<16} {:<12} {}",
                                                plan_id, progress, status
                                            );
                                        }
                                    }
                                    if !dag.critical_path.is_empty() {
                                        println!(
                                            "\nCritical path: {} (est. {} min)",
                                            dag.critical_path.join(" -> "),
                                            dag.critical_path_minutes,
                                        );
                                    }
                                    for overlap in &dag.crate_overlaps {
                                        println!("warning: {overlap}");
                                    }
                                    for dangling in &dag.dangling_refs {
                                        println!("warning: {dangling}");
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::error!(error = %e, "DAG error");
                                return Ok(EXIT_FAILURE);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "failed to load plans for wave analysis");
                        return Ok(EXIT_FAILURE);
                    }
                }
                return Ok(EXIT_SUCCESS);
            }

            let reporter = roko_cli::cli_reporter::CliReporter::from_flags(false, cli.json);

            if cli.json {
                let entries: Vec<serde_json::Value> = summaries
                    .iter()
                    .map(|summary| {
                        serde_json::json!({
                            "id": summary.id.as_str(),
                            "title": summary.title.as_str(),
                            "task_count": summary.task_count,
                            "tasks_done": summary.tasks_done,
                            "tasks_failed": summary.tasks_failed,
                            "completed": summary.completed,
                            "status": summary.status.as_str(),
                            "status_label": summary.status_label(),
                            "superseded_by": summary.superseded_by.as_deref(),
                            "has_run_state": has_run_state,
                        })
                    })
                    .collect();
                let total = summaries.len();
                let complete = summaries.iter().filter(|s| s.completed).count();
                let failed = summaries
                    .iter()
                    .filter(|s| s.tasks_failed > 0 && !s.completed)
                    .count();
                let running = total.saturating_sub(complete).saturating_sub(failed);
                let payload = json!({
                    "plans": entries,
                    "summary": {
                        "total": total,
                        "complete": complete,
                        "running": running,
                        "failed": failed,
                        "has_run_state": has_run_state,
                    }
                });
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                if summaries.is_empty() {
                    if has_run_state {
                        reporter.note("no plans found in discovery path");
                    } else {
                        reporter.note("no run state found");
                    }
                } else {
                    let rows: Vec<Vec<String>> = summaries
                        .iter()
                        .map(|s| {
                            vec![
                                s.id.to_string(),
                                s.title.to_string(),
                                format!("{}/{}", s.tasks_done, s.task_count),
                                s.status_label().to_string(),
                            ]
                        })
                        .collect();
                    reporter.table(&["ID", "TITLE", "PROGRESS", "STATUS"], &rows);
                    if !has_run_state {
                        reporter.note("(no run state found — counts from tasks.toml files)");
                    }
                }

                for (plan_id, _, _) in &state_entries {
                    if !plan_path_exists(&wd, plan_id) {
                        reporter.warn(&format!(
                            "state references missing plan: {plan_id} (not found in plans/ or .roko/plans/)"
                        ));
                    }
                }
            }
            Ok(EXIT_SUCCESS)
        }
        PlanCmd::Show { plan_id, workdir } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            // Read-only: skip the lock when a server owns the workspace;
            // otherwise take the shared lock as usual.
            let _lock = roko_cli::serve_client::read_lock_unless_served(&wd)?;
            let plan_id = plan_id
                .strip_prefix("plans/")
                .or_else(|| plan_id.strip_prefix("plans\\"))
                .unwrap_or(&plan_id);
            let Some(plan_info) =
                roko_cli::plan::discover_plan_by_id(&wd, plan_id).map_err(|e| anyhow!("{e}"))?
            else {
                anyhow::bail!(
                    "plan '{plan_id}' not found.\n  hint: run `roko plan list` to see available plans, or `roko plan create` to create a new one"
                );
            };
            let summary = roko_cli::plan::summarize_plan_info(&plan_info);
            let tasks_path = roko_cli::plan::tasks_path(&plan_info);
            let stable_id = roko_cli::plan::stable_plan_id(&plan_info);

            if cli.json {
                let task_entries: Vec<serde_json::Value> = tasks_path
                    .as_deref()
                    .filter(|p| p.is_file())
                    .and_then(|p| roko_cli::task_parser::TasksFile::parse(p).ok())
                    .map(|tf| {
                        tf.tasks
                            .iter()
                            .map(|t| {
                                json!({
                                    "id": t.id,
                                    "title": t.title,
                                    "status": t.status,
                                    "role": t.role,
                                    "tier": t.tier,
                                    "depends_on": t.depends_on,
                                    "files": t.files,
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                let payload = json!({
                    "plan_id": stable_id,
                    "base": plan_info.base,
                    "title": summary.title,
                    "status": summary.status,
                    "status_label": summary.status_label(),
                    "task_count": summary.task_count,
                    "tasks_done": summary.tasks_done,
                    "tasks_failed": summary.tasks_failed,
                    "completed": summary.completed,
                    "plan_path": plan_info.path,
                    "tasks_path": tasks_path,
                    "frontmatter": plan_info.frontmatter,
                    "tasks": task_entries,
                });
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("plan: {stable_id}");
                println!("base: {}", plan_info.base);
                println!("title: {}", summary.title);
                println!("plan file: {}", plan_info.path.display());
                println!(
                    "tasks file: {}",
                    tasks_path
                        .as_deref()
                        .filter(|path| path.is_file())
                        .map_or_else(|| "(none)".to_string(), |path| path.display().to_string())
                );
                println!("task count: {}", summary.task_count);
                if let Some(frontmatter) = plan_info.frontmatter.as_ref() {
                    if !frontmatter.depends_on.is_empty() {
                        println!("depends_on: {}", frontmatter.depends_on.join(", "));
                    }
                    if !frontmatter.parallel_with.is_empty() {
                        println!("parallel_with: {}", frontmatter.parallel_with.join(", "));
                    }
                    if let Some(priority) = frontmatter.priority {
                        println!("priority: {priority}");
                    }
                    if !frontmatter.tags.is_empty() {
                        println!("tags: {}", frontmatter.tags.join(", "));
                    }
                    if let Some(milestone) = frontmatter.milestone.as_deref() {
                        println!("milestone: {milestone}");
                    }
                }
            }
            Ok(EXIT_SUCCESS)
        }
        PlanCmd::Create {
            plan_id,
            title,
            description,
            workdir,
        } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            let plan_id = plan_id
                .strip_prefix("plans/")
                .or_else(|| plan_id.strip_prefix("plans\\"))
                .unwrap_or(&plan_id)
                .to_string();
            let _workspace_lock =
                roko_cli::workspace_lock::acquire_workspace_lock(&wd.join(".roko"))?;
            let plan = Plan::new(plan_id.clone(), title, description);
            plan.validate()
                .map_err(|errs| anyhow!("plan validation failed: {}", errs.join("; ")))?;

            let plans_dir = roko_cli::plan::plans_dir(&wd);
            std::fs::create_dir_all(&plans_dir).map_err(|e| anyhow!("create plans dir: {e}"))?;
            let plan_dir = plans_dir.join(&plan_id);
            let legacy_plan = plans_dir.join(format!("{plan_id}.md"));
            if plan_dir.exists() || legacy_plan.exists() {
                bail!("plan '{plan_id}' already exists");
            }
            std::fs::create_dir_all(&plan_dir).map_err(|e| anyhow!("create plan dir: {e}"))?;
            let plan_md_path = plan_dir.join("plan.md");
            let tasks_path = plan_dir.join("tasks.toml");

            let yaml_plan_id = serde_json::to_string(&plan.id)?;
            let plan_md = format!(
                "---\nplan: {yaml_plan_id}\n---\n# {}\n\n{}\n",
                plan.title,
                if plan.description.is_empty() {
                    "Describe the plan here.".to_string()
                } else {
                    plan.description.clone()
                }
            );
            let tasks_toml = format!(
                "[meta]\nplan = {:?}\n\n# Add [[task]] entries below.\n",
                plan.id
            );
            std::fs::write(&plan_md_path, plan_md)
                .map_err(|e| anyhow!("write {}: {e}", plan_md_path.display()))?;
            std::fs::write(&tasks_path, tasks_toml)
                .map_err(|e| anyhow!("write {}: {e}", tasks_path.display()))?;

            if cli.json {
                let payload = json!({
                    "created": plan_id,
                    "plan_dir": plan_dir,
                    "plan_path": plan_md_path,
                    "tasks_path": tasks_path,
                });
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else if !cli.quiet {
                tracing::info!(plan_id = %plan_id, path = %plan_dir.display(), "Created plan");
                crate::commands::util::print_next_step_hint(&format!(
                    "Next: edit {tasks} and run with `roko plan run {}`",
                    plan_dir.display(),
                    tasks = tasks_path.display()
                ));
            }
            Ok(EXIT_SUCCESS)
        }
        PlanCmd::Validate {
            dir,
            strict,
            json,
            dag,
            spec_quality,
            dynamic,
            base,
            timeout,
            scratch,
            fixture,
        } => {
            let workdir = resolve_workdir(cli);
            // Read-only lint: skip the lock when a server owns the workspace;
            // otherwise take the shared lock as usual.
            let _lock = roko_cli::serve_client::read_lock_unless_served(&workdir)?;
            let plans_dir = if dir.is_absolute() {
                dir.clone()
            } else {
                workdir.join(&dir)
            };
            // 3214: `--dynamic` proves each task's checks red on the base.
            let dynamic = dynamic.then(|| roko_cli::spec_red_on_base::RedOnBaseOptions {
                base,
                timeout: timeout.map(std::time::Duration::from_secs),
                scratch,
                fixture,
                ..Default::default()
            });
            let exit = cmd_plan_validate(
                &plans_dir,
                &workdir,
                strict,
                json || cli.json,
                spec_quality,
                dynamic,
            )?;

            if dag {
                // Run DAG analysis on top of the lint output.
                let dag_dir = if dir.is_absolute() {
                    dir
                } else {
                    workdir.join(dir)
                };
                match roko_cli::runner::plan_loader::load_plans(&dag_dir) {
                    Ok(loaded_plans) => {
                        match roko_cli::runner::plan_dag::CrossPlanDag::compute(&loaded_plans) {
                            Ok(plan_dag) => {
                                let dag_summary = plan_dag.summary();
                                if json || cli.json {
                                    println!("{}", serde_json::to_string_pretty(&dag_summary)?);
                                } else {
                                    println!("\nDAG Analysis");
                                    println!("============");
                                    println!(
                                        "Plans: {}   Tasks: {}   Edges: {}",
                                        dag_summary.total_plans,
                                        dag_summary.total_tasks,
                                        dag_summary.total_edges,
                                    );
                                    println!();
                                    for wave in &dag_summary.waves {
                                        println!(
                                            "Wave {} ({} plan{}, {} task{}):  {}",
                                            wave.index,
                                            wave.parallelism_width,
                                            if wave.parallelism_width == 1 { "" } else { "s" },
                                            wave.total_tasks,
                                            if wave.total_tasks == 1 { "" } else { "s" },
                                            wave.plan_ids.join(", "),
                                        );
                                    }
                                    if !dag_summary.critical_path.is_empty() {
                                        println!(
                                            "\nCritical path: {} (est. {} min)",
                                            dag_summary.critical_path.join(" -> "),
                                            dag_summary.critical_path_minutes,
                                        );
                                    }
                                    if !dag_summary.dangling_refs.is_empty()
                                        || !dag_summary.crate_overlaps.is_empty()
                                    {
                                        println!("\nWarnings:");
                                        for dangling in &dag_summary.dangling_refs {
                                            println!("  - {dangling}");
                                        }
                                        for overlap in &dag_summary.crate_overlaps {
                                            println!("  - {overlap}");
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::error!(error = %e, "DAG error");
                                return Ok(EXIT_FAILURE);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "failed to load plans for DAG analysis");
                        return Ok(EXIT_FAILURE);
                    }
                }
            }
            Ok(exit)
        }
        PlanCmd::Index { check, workdir } => {
            let workdir = workdir.unwrap_or_else(|| resolve_workdir(cli));
            if check {
                // Read-only check: skip the lock when a server owns the
                // workspace; otherwise take the shared lock as usual.
                let _lock = roko_cli::serve_client::read_lock_unless_served(&workdir)?;
                roko_cli::index::check_plans_index(&workdir)?;
            } else {
                // Rebuild writes the index: exclusive workspace lock.
                let _lock =
                    roko_cli::workspace_lock::acquire_workspace_lock(&workdir.join(".roko"))?;
                roko_cli::index::rebuild_plans_index(&workdir)?;
            }
            if !cli.quiet {
                let status = if check { "current" } else { "rebuilt" };
                println!("plans index {status}");
            }
            Ok(EXIT_SUCCESS)
        }
        PlanCmd::Prepare {
            plan_dir,
            full,
            force,
            workdir,
        } => {
            let workdir = workdir.unwrap_or_else(|| resolve_workdir(cli));
            let plan_dir = if plan_dir.is_absolute() {
                plan_dir
            } else {
                workdir.join(plan_dir)
            };
            let _lock = roko_cli::workspace_lock::acquire_workspace_lock(&workdir.join(".roko"))?;
            let prepared = if full {
                let model = cli.model.clone();
                roko_cli::plan_brief::prepare_full(&plan_dir, &workdir, force, model).await?
            } else {
                roko_cli::plan_brief::prepare(&plan_dir, &workdir, force)?
            };
            if !cli.quiet {
                for path in &prepared.written {
                    println!("wrote {}", path.display());
                }
                for path in &prepared.kept {
                    println!("kept {} (it exists; --force overwrites it)", path.display());
                }
            }
            Ok(EXIT_SUCCESS)
        }
        PlanCmd::Run {
            plans_dir,
            engine,
            workdir,
            resume_plan,
            // `--approval` (`--tui`) names the default: the Graph run opens
            // the inline TUI itself whenever stdout is a terminal.
            approval: _,
            no_tui,
            max_retries,
            max_tasks,
            dry_run,
            fresh,
            force_resume,
            budget_override,
            no_budget,
            frozen_learning,
            no_holdout,
            force,
            dangerously_skip_permissions,
            log_file,
            skip_preflight,
            screenshots,
            screenshot_interval,
            screenshot_dir,
            batch_size,
            worktree_per_task,
            no_worktree_per_task,
            rich_topology,
            promote,
            max_parallel_plans,
            fail_fast,
        } => {
            let _t_setup = std::time::Instant::now();
            // `--worktree-per-task` / `--no-worktree-per-task`; neither leaves
            // it to `[runner] worktree_per_task` (gap-4ec59f).
            let worktree_flag = match (worktree_per_task, no_worktree_per_task) {
                (true, _) => Some(true),
                (_, true) => Some(false),
                _ => None,
            };

            // The global `--model` flag (with `--force-model` and
            // `--force-backend` as aliases) is the single model override.
            // It populates `RunConfig.cli_model_override`, which the event
            // loop maps to `DispatchContext.force_backend`.
            let effective_model_override = cli.model.clone();

            // `--config` names the run's config (bug-4ed3c2): a file that does
            // not exist is an error, not a fall back to the workspace's.
            if let Some(config) = &cli.config
                && !config.is_file()
            {
                anyhow::bail!("--config {}: no such file", config.display());
            }

            // gap-d60281: stop on a flag the Graph engine does not implement
            // rather than run without it.
            let unsupported = graph_unsupported_flags(
                cli.resume.as_deref(),
                cli.effort.as_ref(),
                skip_preflight,
                screenshots,
                screenshot_interval,
                screenshot_dir.as_deref(),
                batch_size,
            );
            if !unsupported.is_empty() {
                anyhow::bail!(
                    "plan run does not support these flags:\n  - {}",
                    unsupported.join("\n  - ")
                );
            }

            // Resolve workdir FIRST (before using plans_dir)
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            let layout = RokoLayout::for_project(&wd);

            // Resolve plans_dir relative to workdir if not absolute
            let resolved_plans_dir = if plans_dir.is_absolute() {
                plans_dir.clone()
            } else {
                wd.join(&plans_dir)
            };

            // ── Wrong cwd detection ────────────────────────────────────────
            // If plans dir doesn't exist but cwd looks like a plan directory,
            // give the user a clear hint to run from the workspace root.
            if !resolved_plans_dir.exists()
                && (wd.join("tasks.toml").exists() || wd.join("plan.md").exists())
            {
                let workspace_root = wd
                    .ancestors()
                    .find(|p| p.join("Cargo.toml").exists() || p.join(".roko").exists())
                    .unwrap_or(&wd);
                anyhow::bail!(
                    "plans directory not found at {}.\n\
                     It looks like you're inside a plan directory. \
                     Run from the workspace root instead:\n  cd {}",
                    resolved_plans_dir.display(),
                    workspace_root.display()
                );
            }

            // ── Mandatory validation: reject malformed plans before execution ──
            // Runs in both normal and `--dry-run` mode.
            if let Some(exit_code) = validate_before_run(&resolved_plans_dir, &wd, no_holdout) {
                return Ok(exit_code);
            }

            // Cross-plan Graph semantics belong to the exact set selected by
            // `plan_loader` (one root plan, or the root's immediate plans),
            // not to the generic validator's recursive file discovery. Run
            // this preflight before both dry-run and workspace-lock mutation.
            validate_graph_selected_plans_before_run(engine, &wd, &resolved_plans_dir)?;

            // ── Dry-run mode: parse plans + show summary without executing ──
            if dry_run {
                return cmd_plan_dry_run(
                    &resolved_plans_dir,
                    cli,
                    &wd,
                    max_parallel_plans.map(|limit| usize::try_from(limit).unwrap_or(usize::MAX)),
                    &roko_cli::graph_checkpoint::ResumeOptions {
                        resume_plan: resume_plan.as_deref(),
                        fresh,
                        force_resume,
                        max_tasks,
                        max_retries,
                        rich_topology,
                    },
                )
                .await;
            }

            // ── Workspace server check ────────────────────────────────────
            // When a live `roko-serve` process owns this workspace, forward
            // the run to the server rather than executing locally.  The
            // server already holds the runner lock so we must not acquire it.
            if let Some(endpoint) = roko_cli::serve_client::discover_workspace_server(&wd) {
                // The server runs the plan under its own config (bug-4ed3c2).
                if let Some(config) = &cli.config {
                    anyhow::bail!(
                        "--config {} cannot be used when a server owns this workspace: the \
                         server runs the plan under its own config; stop the server first",
                        config.display()
                    );
                }
                // A frozen run must not share its workspace with a server,
                // whose own runs and timers write learned state (decision 2218).
                if frozen_learning {
                    anyhow::bail!(
                        "--frozen-learning cannot be used when a server owns this workspace: \
                         the server's runs and timers write learned state; stop the server first"
                    );
                }
                if no_holdout {
                    anyhow::bail!(
                        "--no-holdout cannot be used when a server owns this workspace: the \
                         server runs the plan under its own config; set [experiments] maximize \
                         = true there, or stop the server first"
                    );
                }
                return roko_cli::serve_client::run_plan_via_server(
                    &wd,
                    &resolved_plans_dir,
                    &endpoint,
                    fresh,
                    no_tui,
                    cli.json,
                    max_parallel_plans.map(|limit| usize::try_from(limit).unwrap_or(usize::MAX)),
                    // server-incompatible flags
                    max_retries,
                    max_tasks,
                    budget_override,
                    no_budget,
                    effective_model_override.clone(),
                    dangerously_skip_permissions,
                    log_file.clone(),
                    worktree_flag.is_some(),
                    rich_topology,
                    resume_plan.clone(),
                )
                .await;
            }

            // Both execution engines mutate shared workspace/runtime state.
            // Use the runner-exclusive lock (roko.runner.lock) so that
            // read-only commands holding a shared workspace lock (roko.lock)
            // can coexist with an active plan run without being blocked.
            let _lock = roko_cli::workspace_lock::acquire_runner_lock(layout.root())?;

            // ── Graph Engine path (explicit opt-in) ──
            if matches!(engine, PlanEngine::Graph) {
                return cmd_plan_run_engine(
                    &resolved_plans_dir,
                    &wd,
                    cli,
                    resume_plan.as_deref(),
                    fresh,
                    force_resume,
                    max_retries,
                    max_tasks,
                    budget_override,
                    no_budget,
                    frozen_learning,
                    no_holdout,
                    effective_model_override.clone(),
                    dangerously_skip_permissions,
                    log_file.as_deref(),
                    worktree_flag,
                    rich_topology,
                    promote.clone(),
                    no_tui,
                    max_parallel_plans.map(|limit| usize::try_from(limit).unwrap_or(usize::MAX)),
                    fail_fast,
                    force,
                )
                .await;
            }

            // ── Runner v2 path (REMOVED) ──
            //
            // The legacy Runner-v2 event loop has been deleted. The Graph engine
            // is the sole execution engine. The --engine legacy/runner-v2 flag
            // is kept parseable so existing scripts do not hard-fail, but the
            // engine itself is no longer available.
            {
                tracing::error!(
                    "the legacy Runner-v2 engine has been removed; \
                     the Graph engine is now the sole execution engine; \
                     remove --engine legacy (or --engine runner-v2) from your command; \
                     your plans will run with the Graph engine by default"
                );
                Ok(EXIT_FAILURE)
            }
        }
        PlanCmd::Generate {
            source,
            from_file,
            context,
            from_notes,
            tag,
            from_backlog,
        } => {
            let workdir = std::env::current_dir().context("resolve cwd")?;
            // Plan generation is read-only on workspace state: it reads source
            // code and writes one plan to the workspace plans directory
            // (per-slug, non-overlapping).
            // No workspace lock needed (#226) — allows generating plans while
            // other plans are running.

            // --from-backlog: resolve backlog specs by numeric ID and generate
            // plans with deterministic slugs written to plans/ (#227).
            if let Some(ref backlog_ids_str) = from_backlog {
                use roko_cli::plan_generate::{
                    DEFAULT_BACKLOG_DIR, parse_backlog_ids, resolve_backlog_spec,
                    slug_from_backlog_stem,
                };

                let ids = parse_backlog_ids(backlog_ids_str)?;
                let backlog_dir = workdir.join(DEFAULT_BACKLOG_DIR);
                let model_key = roko_cli::model_selection::resolve_planner_model(
                    &workdir,
                    cli.model.clone(),
                    "plan generate --from-backlog",
                )?;

                let mut results: Vec<(u32, String, &str)> = Vec::new();

                for id in &ids {
                    let spec = match resolve_backlog_spec(&backlog_dir, *id) {
                        Ok(spec) => spec,
                        Err(err) => {
                            tracing::error!(id, error = %err, "backlog spec error");
                            results.push((*id, format!("backlog-{id}"), "error"));
                            continue;
                        }
                    };
                    let slug = slug_from_backlog_stem(&spec.file_stem);
                    let plan_dir = workdir.join("plans").join(&slug);

                    // Check if plan already exists.
                    if plan_dir.exists() {
                        tracing::info!(id, %slug, "skipped (plan already exists)");
                        results.push((*id, slug, "skipped"));
                        continue;
                    }

                    tracing::info!(id, %slug, "generating plan from backlog spec");

                    // 3220: the one plan generator (gap-2623b2) repairs,
                    // validates, scores and writes the plan to plans/<slug>/,
                    // as every other plan-writing path does.
                    let request = roko_cli::plan_generate::PlanRequest {
                        model: Some(model_key.as_str()),
                        effort: Some("high"),
                        ..roko_cli::plan_generate::PlanRequest::new(
                            roko_cli::plan_generate::PlanSource::Text {
                                text: &spec.source_text,
                                kind: "backlog spec",
                            },
                            &slug,
                            &workdir,
                        )
                    };
                    match roko_cli::plan_generate::generate_plan(request).await {
                        Ok((_, outcome)) if outcome.artifact_valid => {
                            tracing::info!(id, %slug, "plan generated");
                            results.push((*id, slug, "generated"));
                        }
                        Ok(_) => {
                            tracing::warn!(id, %slug, "plan generated but validation failed");
                            results.push((*id, slug, "validation-failed"));
                        }
                        Err(err) => {
                            let error = format!("{err:#}");
                            tracing::error!(id, %slug, %error, "plan generation failed");
                            results.push((*id, slug, "error"));
                        }
                    }
                }

                // Log summary for batch mode.
                if ids.len() > 1 {
                    for (id, slug, status) in &results {
                        match *status {
                            "generated" | "skipped" => {
                                tracing::info!(
                                    id,
                                    slug = slug.as_str(),
                                    status,
                                    "batch plan generate result"
                                );
                            }
                            _ => {
                                tracing::warn!(
                                    id,
                                    slug = slug.as_str(),
                                    status,
                                    "batch plan generate result"
                                );
                            }
                        }
                    }
                }

                return Ok(EXIT_SUCCESS);
            }

            // --from-notes: read .roko/notes/, cluster, generate one plan per cluster.
            if from_notes {
                let notes_dir = workdir.join(".roko").join("notes");
                let notes = roko_cli::note_cluster::load_notes(&notes_dir, tag.as_deref());
                if notes.is_empty() {
                    tracing::warn!(path = %notes_dir.display(), "no notes found");
                    return Ok(1);
                }
                let clusters = roko_cli::note_cluster::cluster_notes(notes);
                tracing::info!(count = clusters.len(), "found note clusters");
                for (i, cluster) in clusters.iter().enumerate() {
                    tracing::info!(
                        index = i + 1,
                        name = %cluster.notes[0].path.file_stem().unwrap_or_default().to_string_lossy(),
                        note_count = cluster.notes.len(),
                        theme = %cluster.theme,
                        "note cluster"
                    );
                }

                let model_key = roko_cli::model_selection::resolve_planner_model(
                    &workdir,
                    cli.model.clone(),
                    "plan generate --from-notes",
                )?;

                for cluster in &clusters {
                    let combined: String = cluster
                        .notes
                        .iter()
                        .map(|n| n.text.as_str())
                        .collect::<Vec<_>>()
                        .join("\n\n---\n\n");
                    let slug = cluster.theme.replace(' ', "-");
                    tracing::info!(%slug, "generating plan for cluster");

                    let request = roko_cli::plan_generate::PlanRequest {
                        model: Some(model_key.as_str()),
                        effort: Some("high"),
                        ..roko_cli::plan_generate::PlanRequest::new(
                            roko_cli::plan_generate::PlanSource::Text {
                                text: &combined,
                                kind: "notes",
                            },
                            &slug,
                            &workdir,
                        )
                    };
                    match roko_cli::plan_generate::generate_plan(request).await {
                        Ok(_) => {
                            tracing::info!(%slug, "plan generated from notes cluster");
                        }
                        Err(err) => {
                            tracing::warn!(%slug, error = %format!("{err:#}"), "plan generate for cluster failed");
                        }
                    }
                }

                return Ok(0);
            }

            // Get the source content: either from a file or inline text
            let source_text = if let Some(ref path) = from_file {
                let content = std::fs::read_to_string(path)
                    .with_context(|| format!("read {}", path.display()))?;
                tracing::info!(path = %path.display(), "generating plans from file");
                content
            } else {
                let text = source.join(" ");
                if text.is_empty() {
                    anyhow::bail!("Provide a prompt or --from-file <path>");
                }
                tracing::info!("generating plans from prompt");
                text
            };

            let source_type = if from_file.is_some() {
                "file"
            } else {
                "prompt"
            };
            // The plan's slug: the file's stem, else the prompt's words.
            let slug = from_file
                .as_ref()
                .and_then(|path| path.file_stem())
                .and_then(|stem| stem.to_str())
                .map_or_else(
                    || roko_cli::plan_generate::slugify(&source_text),
                    roko_cli::plan_generate::slugify,
                );
            let model_key = roko_cli::model_selection::resolve_planner_model(
                &workdir,
                cli.model.clone(),
                "plan generate",
            )?;

            let context_block = if context.is_empty() {
                String::new()
            } else {
                let loaded = roko_cli::context_loader::load_context_files(
                    &context,
                    roko_cli::context_loader::DEFAULT_BUDGET,
                    &workdir,
                );
                if !loaded.is_empty() {
                    format!("<context>\n{loaded}</context>")
                } else {
                    String::new()
                }
            };

            // The one plan generator (gap-2623b2) validates the plan and
            // writes it to the workspace plans directory, where every other
            // command looks for plans (bug-e3df7d).
            let request = roko_cli::plan_generate::PlanRequest {
                context: Some(context_block.as_str()),
                model: Some(model_key.as_str()),
                effort: Some("high"),
                ..roko_cli::plan_generate::PlanRequest::new(
                    roko_cli::plan_generate::PlanSource::Text {
                        text: &source_text,
                        kind: source_type,
                    },
                    &slug,
                    &workdir,
                )
            };
            let (_, outcome) = roko_cli::plan_generate::generate_plan(request).await?;
            if outcome.artifact_valid {
                Ok(EXIT_SUCCESS)
            } else {
                tracing::error!(
                    "plan generate: the generated plans failed validation (see warnings above)"
                );
                Ok(1)
            }
        }
        PlanCmd::Regenerate { plan_dir, dry_run } => {
            let workdir = std::env::current_dir().context("resolve cwd")?;
            // Plan regeneration writes only to the target plan directory,
            // which is per-slug and non-overlapping with active plan runs.
            // No workspace lock needed (#226).
            let plan_dir = if plan_dir.is_absolute() {
                plan_dir
            } else {
                workdir.join(plan_dir)
            };
            let tasks_path = plan_dir.join("tasks.toml");
            if !tasks_path.exists() {
                anyhow::bail!("No tasks.toml found in {}", plan_dir.display());
            }
            let source_path = roko_cli::plan_generate::plan_source_document(&plan_dir)?;
            let model_key = roko_cli::model_selection::resolve_planner_model(
                &workdir,
                cli.model.clone(),
                "plan regenerate",
            )?;

            // Collect pre-existing validation diagnostics so the planner knows what was wrong.
            let pre_validation_context =
                format_pre_validation_context(&tasks_path, &plan_validate::validate_plans_dir);

            if dry_run {
                tracing::info!(
                    tasks = %tasks_path.display(),
                    source = %source_path.display(),
                    model = %model_key,
                    "[dry-run] would regenerate plan"
                );
                return Ok(EXIT_SUCCESS);
            }

            // The one plan generator (gap-2623b2) rewrites tasks.toml only
            // once the regenerated plan passes validation, keeping done
            // tasks done.
            let slug = roko_cli::plan_generate::plan_dir_slug(&plan_dir);
            // 3228: the planner sees how the plan's last run failed (3215),
            // and the command prints what the regeneration changed (3216).
            let before = std::fs::read_to_string(&tasks_path).unwrap_or_default();
            let failure = last_run_failure_for(&workdir, &slug, &model_key);
            let request = roko_cli::plan_generate::PlanRequest {
                context: Some(pre_validation_context.as_str()),
                model: Some(model_key.as_str()),
                effort: Some("high"),
                failure_context: failure.as_deref(),
                ..roko_cli::plan_generate::PlanRequest::new(
                    roko_cli::plan_generate::PlanSource::Regenerate(&plan_dir),
                    &slug,
                    &workdir,
                )
            };
            let (_, outcome) = roko_cli::plan_generate::generate_plan(request).await?;
            if outcome.artifact_valid {
                let after = std::fs::read_to_string(&tasks_path).unwrap_or_default();
                print_plan_diff(
                    &roko_cli::plan_authoring::plan_diff(&before, &after),
                    cli.json,
                )?;
                Ok(EXIT_SUCCESS)
            } else {
                tracing::error!(
                    "plan regenerate: the regenerated plan failed validation (see warnings above)"
                );
                Ok(1)
            }
        }
        PlanCmd::Revise { plan, feedback } => cmd_plan_revise(cli, &plan, &feedback).await,
        PlanCmd::Queue { cmd } => cmd_plan_queue(cli, cmd).await,

        // ── Plan control commands (#146, 1209) ──────────────────────
        PlanCmd::Pause { workdir } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            send_plan_control(cli, &wd, "pause", None).await
        }
        PlanCmd::Resume { workdir } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            send_plan_control(cli, &wd, "resume", None).await
        }
        PlanCmd::Cancel { plan_id, workdir } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            send_plan_control(cli, &wd, "cancel", plan_id).await
        }
        PlanCmd::Retry {
            task_id: _,
            plan_id,
            workdir,
        } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            send_plan_control(cli, &wd, "retry", plan_id).await
        }
        PlanCmd::Budget { cmd } => cmd_plan_budget(cli, cmd).await,
        PlanCmd::Review {
            plan_id,
            task_id,
            approve,
            reject: _,
            note,
            workdir,
        } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            let decision = if approve { "approved" } else { "rejected" };
            let attempt_key = roko_cli::graph_task_dispatch::record_review(
                &wd, &plan_id, &task_id, decision, &note,
            )?;
            if cli.json {
                println!(
                    "{}",
                    serde_json::json!({
                        "plan_id": plan_id,
                        "task_id": task_id,
                        "decision": decision,
                        "attempt_key": attempt_key,
                    })
                );
            } else if !cli.quiet {
                println!("{decision} task {task_id} of plan {plan_id} (attempt {attempt_key})");
            }
            Ok(EXIT_SUCCESS)
        }

        PlanCmd::Status { plan_dir, workdir } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            // Read-only status inspection: skip the lock when a server owns
            // the workspace; otherwise take the shared lock as usual.
            let _lock = roko_cli::serve_client::read_lock_unless_served(&wd)?;

            // When a plan directory is provided, show task-level status for
            // that specific plan instead of the global runner status.
            if let Some(raw_plan_dir) = plan_dir {
                return cmd_plan_dir_status(cli, &wd, &raw_plan_dir).await;
            }

            let status_path = wd.join(".roko").join("state").join("status.json");
            if !status_path.is_file() {
                if cli.json {
                    println!(
                        "{}",
                        serde_json::json!({"error": "no status file found — is a plan running?"})
                    );
                } else {
                    tracing::warn!(path = %status_path.display(), "no status file found; start a plan run with: roko plan run plans/");
                }
                return Ok(EXIT_FAILURE);
            }
            let raw = std::fs::read_to_string(&status_path)
                .with_context(|| format!("read {}", status_path.display()))?;
            if cli.json {
                // Pass through as-is; the file is already JSON.
                println!("{raw}");
            } else {
                let value: serde_json::Value = serde_json::from_str(&raw)
                    .with_context(|| format!("parse {}", status_path.display()))?;
                println!(
                    "run_id:          {}",
                    value.get("run_id").and_then(|v| v.as_str()).unwrap_or("?")
                );
                println!(
                    "phase:           {}",
                    value.get("phase").and_then(|v| v.as_str()).unwrap_or("?")
                );
                println!(
                    "plans:           {}/{} completed",
                    value
                        .get("completed_plans")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0),
                    value
                        .get("total_plans")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0)
                );
                println!(
                    "active agents:   {}",
                    value
                        .get("active_agents")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0)
                );
                println!(
                    "elapsed:         {}s",
                    value
                        .get("elapsed_secs")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0)
                );
                println!(
                    "last event:      {}",
                    value
                        .get("last_event")
                        .and_then(|v| v.as_str())
                        .unwrap_or("none")
                );
            }
            Ok(EXIT_SUCCESS)
        }

        PlanCmd::Shorthand(words) => {
            // `roko plan "add cursor support"` → delegate to plan generate
            Box::pin(cmd_plan(
                cli,
                PlanCmd::Generate {
                    source: words,
                    from_file: None,
                    context: vec![],
                    from_notes: false,
                    tag: None,
                    from_backlog: None,
                },
            ))
            .await
        }
    }
}

// ---------------------------------------------------------------------------
// Plan-directory status helper
// ---------------------------------------------------------------------------

/// `roko plan status`'s label for a plan: its Graph checkpoint's status when
/// it has one (`succeeded` reads `complete`, and an interrupted or cancelled
/// run shows as such, gap-20ab07), else what its task counts say.
fn plan_status_label(
    checkpoint: Option<roko_cli::graph_checkpoint::GraphCheckpointStatus>,
    done_tasks: usize,
    total_tasks: usize,
) -> &'static str {
    use roko_cli::graph_checkpoint::GraphCheckpointStatus;

    match checkpoint {
        Some(GraphCheckpointStatus::Succeeded) => "complete",
        Some(status) => status.as_str(),
        None if total_tasks > 0 && done_tasks == total_tasks => "complete",
        None if done_tasks == 0 => "not started",
        None => "in progress",
    }
}

/// Show task-level status for a specific plan directory.
///
/// Reads `tasks.toml` in the plan directory and, when executor state is
/// available, overlays runtime completion counts from the snapshot.
async fn cmd_plan_dir_status(
    cli: &Cli,
    workdir: &std::path::Path,
    raw_plan_dir: &std::path::Path,
) -> Result<i32> {
    // Resolve the plan directory (may be relative to workdir).
    let plan_dir = if raw_plan_dir.is_absolute() {
        raw_plan_dir.to_path_buf()
    } else {
        workdir.join(raw_plan_dir)
    };

    // Strip a leading "plans/" prefix from the string form to extract the
    // plan ID, mirroring how `roko plan show` handles it.
    let plan_id_raw = plan_dir.file_name().and_then(|n| n.to_str()).unwrap_or("");

    // Locate the tasks.toml inside this plan directory.
    let tasks_path = plan_dir.join("tasks.toml");
    if !tasks_path.is_file() {
        if cli.json {
            println!(
                "{}",
                serde_json::json!({
                    "error": format!("no tasks.toml found in {}", plan_dir.display()),
                    "hint": "run `roko plan list` to see available plans"
                })
            );
        } else {
            eprintln!(
                "error: no tasks.toml found in {}\n  hint: run `roko plan list` to see available plans",
                plan_dir.display()
            );
        }
        return Ok(EXIT_FAILURE);
    }

    let tasks_file = roko_cli::task_parser::TasksFile::parse(&tasks_path)
        .with_context(|| format!("parse {}", tasks_path.display()))?;

    // Build per-plan completion counts from the executor snapshot.
    let state_map: std::collections::HashMap<String, (usize, usize)> = read_executor_state(workdir)
        .unwrap_or_default()
        .into_iter()
        .map(|(id, done, total)| (id, (done, total)))
        .collect();

    let plan_id_from_toml = tasks_file.meta.plan.as_str();
    let plan_id = if plan_id_from_toml.is_empty() {
        plan_id_raw
    } else {
        plan_id_from_toml
    };

    let total_tasks = tasks_file.tasks.len();
    let done_from_toml = tasks_file
        .tasks
        .iter()
        .filter(|t| {
            t.status.eq_ignore_ascii_case("done") || t.status.eq_ignore_ascii_case("complete")
        })
        .count();

    let (mut done_tasks, total_from_state) = state_map
        .get(plan_id)
        .copied()
        .unwrap_or((done_from_toml, total_tasks));

    let effective_total = if total_from_state > 0 {
        total_from_state
    } else {
        total_tasks
    };

    // Overlay the Graph engine checkpoint status. The graph engine writes
    // terminal state to `.roko/state/graph/<safe-plan-id>/checkpoint.json`
    // rather than updating tasks.toml or the legacy executor snapshot. Read it
    // here so that `plan status` reflects the same data as `plan list`.
    let graph_status = roko_cli::graph_checkpoint::canonical_checkpoint_status(workdir, plan_id);
    let checkpoint_succeeded =
        graph_status == Some(roko_cli::graph_checkpoint::GraphCheckpointStatus::Succeeded);
    if checkpoint_succeeded && done_tasks == 0 && effective_total > 0 {
        // The graph engine completed all tasks; tasks.toml wasn't updated.
        done_tasks = effective_total;
    }

    // Derive the human-readable status string.
    let status_str = plan_status_label(graph_status, done_tasks, effective_total);

    // Why the plan's whole-plan check failed, when it did (gap-60233f).
    let plan_check_failure =
        roko_cli::graph_checkpoint::recorded_plan_check_failure(workdir, &plan_id);
    // Where a delivered plan's work is, and how to take it into the checkout,
    // which the run never changes (gap-4ec59f).
    let delivery = roko_cli::graph_checkpoint::recorded_batch_delivery(workdir, plan_id);
    let merge_command = match &delivery {
        Some(delivery) => {
            roko_cli::graph_execution::batch::merge_command(workdir, &delivery.branch).await
        }
        None => None,
    };

    if cli.json {
        let task_entries: Vec<serde_json::Value> = tasks_file
            .tasks
            .iter()
            .map(|t| {
                serde_json::json!({
                    "id": t.id,
                    "title": t.title,
                    "status": t.status,
                    "role": t.role,
                    "tier": t.tier,
                    "depends_on": t.depends_on,
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "plan_id": plan_id,
                "plan_dir": plan_dir,
                "tasks_done": done_tasks,
                "tasks_total": effective_total,
                "completed": status_str == "complete",
                "status": status_str,
                "plan_check_failure": plan_check_failure,
                "delivery": delivery.as_ref().map(|delivery| serde_json::json!({
                    "branch": delivery.branch,
                    "merge_commit": delivery.merge_commit,
                    "merge_command": merge_command,
                    // The whole-plan checks the delivery ran (backlog 3111).
                    "checks": delivery
                        .checks
                        .iter()
                        .map(|check| serde_json::json!({
                            "command": check.command,
                            "source": check.source,
                            "exit_code": check.exit_code,
                        }))
                        .collect::<Vec<_>>(),
                    "check_log": delivery.check_log,
                })),
                "tasks": task_entries,
            }))?
        );
    } else {
        println!("plan:            {plan_id}");
        println!("directory:       {}", plan_dir.display());
        println!("tasks:           {done_tasks}/{effective_total}");
        println!("status:          {status_str}");
        if let Some(failure) = &plan_check_failure {
            println!("plan check:      {failure}");
        }
        if let Some(delivery) = &delivery {
            println!(
                "delivered:       {} at {}",
                delivery.branch, delivery.merge_commit
            );
            if let Some(command) = &merge_command {
                println!("take it with:    {command}");
            }
            if let Some(log) = &delivery.check_log {
                println!("plan check log:  {log}");
            }
        }
        println!();
        if tasks_file.tasks.is_empty() {
            println!("  (no tasks)");
        } else {
            let id_width = tasks_file
                .tasks
                .iter()
                .map(|t| t.id.len())
                .max()
                .unwrap_or(2)
                .max(2);
            for task in &tasks_file.tasks {
                // For display, show "done" for all tasks when the graph
                // checkpoint reports success but tasks.toml wasn't updated.
                // When the graph checkpoint succeeded, promote any
                // pre-execution task status to "done".  The graph engine
                // does not write back to tasks.toml, so statuses like
                // "ready", "pending", and "todo" all mean "not yet done
                // according to the file" even though the run completed.
                let is_pre_execution_status = task.status.is_empty()
                    || task.status.eq_ignore_ascii_case("pending")
                    || task.status.eq_ignore_ascii_case("todo")
                    || task.status.eq_ignore_ascii_case("ready");
                let display_status = if checkpoint_succeeded && is_pre_execution_status {
                    "done"
                } else {
                    task.status.as_str()
                };
                let title = task.title.as_str();
                println!("  {:<id_width$}  {:<10}  {title}", task.id, display_status);
            }
        }
    }
    Ok(EXIT_SUCCESS)
}

// ---------------------------------------------------------------------------
// Queue manifest subcommands
// ---------------------------------------------------------------------------

async fn cmd_plan_queue(cli: &Cli, cmd: QueueCmd) -> Result<i32> {
    match cmd {
        QueueCmd::Show { file, workdir } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            let manifest_path = if file.is_absolute() {
                file
            } else {
                wd.join(file)
            };
            let manifest =
                roko_cli::runner::queue_manifest::QueueManifest::from_file(&manifest_path)?;

            // Graph runs write checkpoints, not the old executor snapshot.
            let completed = manifest.completed_plans(&wd);

            if cli.json {
                let milestones: Vec<serde_json::Value> = manifest
                    .milestone_order()
                    .iter()
                    .filter_map(|&name| manifest.milestones.iter().find(|m| m.name == name))
                    .map(|ms| {
                        let done = ms.plans.iter().filter(|p| completed.contains(*p)).count();
                        json!({
                            "name": ms.name,
                            "description": ms.description,
                            "plans": ms.plans,
                            "depends_on": ms.depends_on,
                            "completed": done,
                            "total": ms.plans.len(),
                            "all_done": done == ms.plans.len() && !ms.plans.is_empty(),
                        })
                    })
                    .collect();
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({ "milestones": milestones }))?
                );
            } else {
                print!("{}", manifest.render_show(&completed));
            }
            Ok(EXIT_SUCCESS)
        }
        QueueCmd::Validate { file, workdir } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            let manifest_path = if file.is_absolute() {
                file
            } else {
                wd.join(file)
            };
            let manifest =
                roko_cli::runner::queue_manifest::QueueManifest::from_file(&manifest_path)?;

            // Collect available plan IDs from disk.
            let available: std::collections::HashSet<String> =
                roko_cli::plan::summarize_discovered_plans(&wd)
                    .map_err(|e| anyhow!("{e}"))?
                    .into_iter()
                    .map(|s| s.id)
                    .collect();

            let issues = manifest.validate(Some(&available));
            if issues.is_empty() {
                if !cli.quiet {
                    println!(
                        "queue manifest valid: {} milestone(s), {} plan(s)",
                        manifest.milestones.len(),
                        manifest
                            .milestones
                            .iter()
                            .map(|m| m.plans.len())
                            .sum::<usize>()
                    );
                }
                Ok(EXIT_SUCCESS)
            } else {
                for issue in &issues {
                    tracing::error!(issue = %issue, "queue manifest validation error");
                }
                Ok(EXIT_FAILURE)
            }
        }
        QueueCmd::Init { output, workdir } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            let output_path = if output.is_absolute() {
                output
            } else {
                wd.join(output)
            };

            if output_path.exists() {
                bail!("queue manifest already exists at {}", output_path.display());
            }

            let plan_ids: Vec<String> = roko_cli::plan::summarize_discovered_plans(&wd)
                .map_err(|e| anyhow!("{e}"))?
                .into_iter()
                .map(|s| s.id)
                .collect();

            if plan_ids.is_empty() {
                bail!("no plans found in workspace");
            }

            let manifest =
                roko_cli::runner::queue_manifest::QueueManifest::generate_starter(&plan_ids);
            let toml_content = manifest.to_toml()?;

            if let Some(parent) = output_path.parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("create directory {}", parent.display()))?;
            }
            std::fs::write(&output_path, &toml_content)
                .with_context(|| format!("write {}", output_path.display()))?;

            if !cli.quiet {
                println!(
                    "created queue manifest at {} with {} plan(s)",
                    output_path.display(),
                    plan_ids.len()
                );
            }
            Ok(EXIT_SUCCESS)
        }
    }
}

/// Handle `roko resume [run-id]` by locating the snapshot and delegating
/// to `cmd_plan` with a synthesized `PlanCmd::Run`. `max_tasks` is the run's
/// `--max-tasks`, which never stops a checkpoint from resuming.
pub(crate) async fn cmd_resume(
    cli: &Cli,
    run_id: Option<String>,
    workdir: Option<std::path::PathBuf>,
    max_tasks: usize,
) -> Result<i32> {
    let workdir = workdir.unwrap_or_else(|| resolve_workdir(cli));
    let snapshot = if let Some(ref id) = run_id {
        // Try a named checkpoint, then the authoritative unified snapshot,
        // then the legacy executor only when the unified file is absent.
        let specific = workdir.join(format!(".roko/state/{id}.json"));
        if specific.exists() {
            specific
        } else if workdir.join(".roko/state/state-snapshot.json").exists() {
            workdir.join(".roko/state/state-snapshot.json")
        } else {
            workdir.join(".roko/state/executor.json")
        }
    } else {
        let unified = workdir.join(".roko/state/state-snapshot.json");
        // Graph engine runs checkpoint each plan under this root.
        let graph_root = workdir.join(".roko/state/graph");
        if unified.exists() {
            unified
        } else if graph_root.is_dir() {
            graph_root
        } else {
            workdir.join(".roko/state/executor.json")
        }
    };

    if !snapshot.exists() {
        eprintln!("no snapshot found at {}", snapshot.display());
        eprintln!("hint: run `roko plan run <dir>` first to create a checkpoint");
        return Ok(1);
    }

    // Print resume header using inline primitives
    if roko_cli::inline::should_use_inline() {
        let theme = roko_cli::tui::Theme::from_env();
        let id_display = run_id.as_deref().unwrap_or("latest");
        let lines = vec![roko_cli::inline::styled::section_start(
            &theme,
            "resume",
            id_display,
            Some(&format!("from {}", snapshot.display())),
        )];
        roko_cli::inline::plaintext::print_plain(&lines);
    }

    // Delegate to plan run with resume
    // Use canonical `./plans/` first, fall back to `.roko/plans/` with a note.
    let plan_dir = super::super::resolve_plans_dir(&workdir, None);
    if !plan_dir.exists() {
        let canonical = workdir.join("plans");
        let fallback = workdir.join(".roko").join("plans");
        eprintln!(
            "error: no plans directory found. Checked:\n  canonical: {}\n  fallback: {}",
            canonical.display(),
            fallback.display(),
        );
        return Ok(1);
    }
    let plan_cmd = PlanCmd::Run {
        plans_dir: plan_dir,
        engine: PlanEngine::default(),
        resume_plan: Some(snapshot),
        workdir: Some(workdir),
        approval: false,
        no_tui: false,
        max_retries: None,
        max_tasks,
        dry_run: false,
        fresh: false,
        force_resume: false,
        budget_override: None,
        no_budget: false,
        frozen_learning: false,
        no_holdout: false,
        force: false,
        dangerously_skip_permissions: false,
        log_file: None,
        skip_preflight: false,
        screenshots: false,
        screenshot_interval: 60,
        screenshot_dir: None,
        batch_size: None,
        // Neither flag: the resumed run follows `[runner] worktree_per_task`.
        worktree_per_task: false,
        no_worktree_per_task: false,
        rich_topology: false,
        promote: None,
        max_parallel_plans: None,
        fail_fast: false,
    };
    cmd_plan(cli, plan_cmd).await
}

/// Parse and display a plan directory without executing anything.
///
/// Plans are loaded with the same loader as `plan run`, so the preview shows
/// exactly what a real run would execute: the plan itself when `plans_dir` is
/// a plan directory, or every plan of a plan set. Finding no plans is an
/// error here, as it is for a real run.
pub(crate) async fn cmd_plan_dry_run(
    plans_dir: &Path,
    cli: &Cli,
    workdir: &Path,
    max_parallel_plans: Option<usize>,
    resume: &roko_cli::graph_checkpoint::ResumeOptions<'_>,
) -> Result<i32> {
    let plans = roko_cli::runner::plan_loader::load_plans(plans_dir)?;
    let schedule = parallel_schedule(workdir, plans_dir, &plans, max_parallel_plans).await?;
    // What each plan's Graph checkpoint would restore in a real run.
    let checkpoints: Vec<_> = plans
        .iter()
        .map(|plan| {
            roko_cli::graph_checkpoint::preview_plan_resume(workdir, plan, plans.len(), resume)
        })
        .collect();
    // Optional scheduling hints live in each plan's `plan.md` frontmatter.
    let frontmatters: Vec<Option<roko_cli::orchestrator::PlanFrontmatter>> = plans
        .iter()
        .map(|plan| {
            std::fs::read_to_string(plan.dir.join("plan.md"))
                .ok()
                .and_then(|content| roko_cli::orchestrator::parse_frontmatter(&content))
        })
        .collect();

    let mut plan_summaries: Vec<serde_json::Value> = Vec::new();
    let mut total_tasks: usize = 0;
    let mut total_estimated_minutes: u32 = 0;

    for ((plan, frontmatter), checkpoint) in plans.iter().zip(&frontmatters).zip(&checkpoints) {
        let task_details: Vec<serde_json::Value> = plan
            .tasks
            .tasks
            .iter()
            .map(|t| {
                json!({
                    "id": t.id,
                    "title": t.title,
                    "status": t.status,
                    "tier": t.tier,
                    "depends_on": t.depends_on,
                    "files": t.files.len(),
                })
            })
            .collect();

        total_tasks += task_details.len();
        if let Some(mins) = frontmatter.as_ref().and_then(|f| f.estimated_minutes) {
            total_estimated_minutes += mins;
        }

        plan_summaries.push(json!({
            "plan": plan.id,
            "num": plan.id.split('-').next().unwrap_or(&plan.id),
            "dir": plan.dir,
            "task_count": task_details.len(),
            "estimated_minutes": frontmatter.as_ref().and_then(|f| f.estimated_minutes),
            "parallel_width": frontmatter.as_ref().and_then(|f| f.estimated_parallel_width),
            "priority": frontmatter.as_ref().and_then(|f| f.priority),
            "tags": frontmatter.as_ref().map(|f| &f.tags),
            "tasks": task_details,
            "checkpoint": match checkpoint {
                Ok(preview) => serde_json::to_value(preview)?,
                Err(error) => json!({ "error": format!("{error:#}") }),
            },
        }));
    }

    if cli.json {
        let payload = json!({
            "dry_run": true,
            "plans_dir": plans_dir,
            "total_plans": plans.len(),
            "total_tasks": total_tasks,
            "total_estimated_minutes": total_estimated_minutes,
            "plans": plan_summaries,
            "max_parallel_plans": schedule.max_parallel_plans,
            "execution_order": schedule.order,
            "plan_conflicts": schedule.conflicts,
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!(
            "Dry run: {} plan(s), {} task(s) in {}\n",
            plans.len(),
            total_tasks,
            plans_dir.display()
        );

        for (i, ((plan, frontmatter), checkpoint)) in plans
            .iter()
            .zip(&frontmatters)
            .zip(&checkpoints)
            .enumerate()
        {
            let est = frontmatter
                .as_ref()
                .and_then(|f| f.estimated_minutes)
                .map(|m| format!(" (~{m} min)"))
                .unwrap_or_default();
            let priority = frontmatter
                .as_ref()
                .and_then(|f| f.priority)
                .map(|p| format!(" [priority={p}]"))
                .unwrap_or_default();
            println!("  {}. {}{}{}", i + 1, plan.id, est, priority);

            for t in &plan.tasks.tasks {
                let status = if t.status.is_empty() {
                    "pending"
                } else {
                    t.status.as_str()
                };
                let deps = if t.depends_on.is_empty() {
                    String::new()
                } else {
                    format!(" (after {})", t.depends_on.join(", "))
                };
                println!("     {}: {} [{}, {status}]{deps}", t.id, t.title, t.tier);
            }
            match checkpoint {
                Ok(preview) => {
                    for line in preview.describe(workdir) {
                        println!("     {line}");
                    }
                }
                Err(error) => println!("     checkpoint: unknown ({error:#})"),
            }
        }

        if total_estimated_minutes > 0 {
            println!("\nEstimated total: ~{total_estimated_minutes} min");
        }
        print_parallel_schedule(&schedule);
        println!("\nNo tasks were executed. Remove --dry-run to run the plan.");
    }

    Ok(EXIT_SUCCESS)
}

/// 3228: `roko plan revise`: revise the plan with the planner model
/// ([`roko_cli::plan_authoring::revise_plan_source`]), then print the plan
/// diff and the validation result, or with `--json` both as one object. Exit
/// 1 when the revision was rejected, which leaves `tasks.toml` as it was.
async fn cmd_plan_revise(cli: &Cli, plan: &str, feedback: &str) -> Result<i32> {
    let workdir = resolve_workdir(cli);
    let tasks_path = plan_tasks_path(&workdir, plan)?;
    let plan_id = roko_cli::task_parser::TasksFile::parse(&tasks_path)?
        .meta
        .plan;
    let resolved = roko_cli::load_resolved_config(&workdir)?;
    let outcome = roko_cli::plan_authoring::revise_plan_source(
        &workdir,
        &plan_id,
        &tasks_path,
        feedback,
        &resolved.config.models,
        None,
    )
    .await?;
    let diagnostics: Vec<serde_json::Value> = outcome
        .report
        .diagnostics
        .iter()
        .map(|diagnostic| {
            let severity = match diagnostic.severity {
                plan_validate::Severity::Error => "error",
                plan_validate::Severity::Warning => "warning",
            };
            serde_json::json!({
                "severity": severity,
                "rule_id": diagnostic.rule_id,
                "task_id": diagnostic.task_id,
                "message": diagnostic.message,
            })
        })
        .collect();
    if cli.json {
        let output = serde_json::json!({
            "revised": outcome.written,
            "task_count": outcome.task_count,
            "diff": outcome.diff,
            "diagnostics": diagnostics,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        if outcome.written {
            println!(
                "plan revise: wrote {} ({} tasks)",
                tasks_path.display(),
                outcome.task_count
            );
            if let Some(diff) = &outcome.diff {
                println!("{}", diff.render_text());
            }
        } else {
            println!(
                "plan revise: the revision was rejected; {} is unchanged",
                tasks_path.display()
            );
        }
        println!(
            "validation: {} errors, {} warnings",
            outcome.report.errors, outcome.report.warnings
        );
        for diagnostic in &diagnostics {
            println!(
                "  {} {} [{}]: {}",
                diagnostic["severity"].as_str().unwrap_or_default(),
                diagnostic["rule_id"].as_str().unwrap_or_default(),
                diagnostic["task_id"].as_str().unwrap_or("-"),
                diagnostic["message"].as_str().unwrap_or_default()
            );
        }
    }
    Ok(if outcome.written { EXIT_SUCCESS } else { 1 })
}

/// The `tasks.toml` of `plan`: a plan directory, relative to `workdir` or
/// absolute, or a plan id.
fn plan_tasks_path(workdir: &Path, plan: &str) -> Result<PathBuf> {
    let dir = workdir.join(plan);
    if dir.join("tasks.toml").is_file() {
        return Ok(dir.join("tasks.toml"));
    }
    let info = roko_cli::plan::discover_plan_by_id(workdir, plan)
        .with_context(|| format!("find plan `{plan}` in {}", workdir.display()))?
        .ok_or_else(|| anyhow!("no plan `{plan}` in {}", workdir.display()))?;
    roko_cli::plan::tasks_path(&info)
        .filter(|path| path.is_file())
        .ok_or_else(|| anyhow!("plan `{plan}` has no tasks.toml"))
}

/// How `plan_id`'s last run failed, for its planner, within a quarter of the
/// planner model's context window (3215); `None` when it has no failed run.
fn last_run_failure_for(workdir: &Path, plan_id: &str, model_key: &str) -> Option<String> {
    let window = roko_cli::load_resolved_config(workdir)
        .ok()
        .and_then(|resolved| {
            resolved
                .config
                .models
                .get(model_key)
                .map(|model| model.context_window)
        })
        .filter(|window| *window > 0);
    let budget = roko_cli::plan_authoring::revision_failure_budget(window);
    roko_cli::plan_authoring::last_run_failure_context(workdir, plan_id, budget)
}

/// Print a plan diff: its text, or with `--json` the diff itself.
fn print_plan_diff(diff: &roko_cli::plan_authoring::PlanDiff, json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(diff)?);
    } else {
        println!("{}", diff.render_text());
    }
    Ok(())
}

/// Run plan validation before `plan run` starts any agents; `no_holdout`
/// (`--no-holdout`) turns on maximize mode for the spec gate's holdout.
///
/// Returns `Some(exit_code)` when validation fails, or `None` when the plan
/// set is valid enough to continue.
fn validate_before_run(plans_dir: &Path, workdir: &Path, no_holdout: bool) -> Option<i32> {
    // If the plans directory doesn't exist yet (e.g. before `plan generate` runs),
    // skip pre-flight validation — the run path will report "No plans found".
    if !plans_dir.exists() {
        return None;
    }

    let config_path = workdir.join("roko.toml");
    let models = if config_path.is_file() {
        std::fs::read_to_string(&config_path)
            .ok()
            .and_then(|text| toml::from_str::<roko_core::config::schema::RokoConfig>(&text).ok())
            .map(|config| crate::commands::config_cmd::configured_models(&config))
    } else {
        None
    };

    let report = match plan_validate::validate_plans_dir_with_workdir(
        plans_dir,
        models.as_ref(),
        Some(workdir),
    ) {
        Ok(report) => report,
        Err(error) => {
            tracing::error!(error = %error, "plan validation failed");
            return Some(1);
        }
    };

    // If no tasks.toml files were found, skip validation — the run path will
    // report "No plans found" with better context.
    if report.totals.plans_checked == 0 {
        return None;
    }

    // An advisory finding does not stop the run: tasks that could run
    // together but write overlapping files only cost parallelism, since the
    // engine runs them one after the other.
    let (advisory, blocking): (Vec<_>, Vec<_>) = report
        .plans
        .iter()
        .flat_map(|plan| &plan.diagnostics)
        .filter(|diagnostic| diagnostic.severity == plan_validate::Severity::Error)
        .partition(|diagnostic| roko_cli::plan_policy::is_advisory_code(&diagnostic.rule_id));
    for diagnostic in advisory {
        tracing::warn!(
            rule = %diagnostic.rule_id,
            plan_id = diagnostic.plan_id.as_deref().unwrap_or_default(),
            message = %diagnostic.message,
            "plan validation finding; the plan still runs"
        );
    }
    if blocking.is_empty() {
        spec_gate_before_run(plans_dir, workdir, no_holdout)
    } else {
        let rendered = plan_validate::render_text(&report);
        // On stderr as well as in the log: without `--verbose` an operator
        // sees no tracing output.
        eprintln!("plan validation failed; fix these errors before running:\n{rendered}");
        tracing::error!(report = %rendered, "plan validation failed — fix the errors above before running");
        Some(1)
    }
}

/// 3211: run the spec gate ([`roko_cli::spec_gate`]) over the plans `plan
/// run` is about to start, with the `[spec_quality]` settings of the
/// workspace's `roko.toml` (the defaults when it has none or does not
/// parse). Maximize mode (`no_holdout`, or `[experiments] maximize`) holds no
/// task out, as in the plan-load gate (gap-29fe0a). Logs each finding with
/// its task, rule and detail, and returns `Some(1)` when a task is blocked,
/// before any agent starts.
fn spec_gate_before_run(plans_dir: &Path, workdir: &Path, no_holdout: bool) -> Option<i32> {
    let mut config = std::fs::read_to_string(workdir.join("roko.toml"))
        .ok()
        .and_then(|text| toml::from_str::<roko_core::config::schema::RokoConfig>(&text).ok())
        .unwrap_or_default();
    roko_cli::graph_execution::plan_runner::apply_run_switches(&mut config, false, no_holdout);
    let config = config.spec_quality;
    let files = match plan_validate::collect_tasks_files(plans_dir) {
        Ok(files) => files,
        Err(error) => {
            tracing::error!(error = %error, "spec gate: cannot list the plans to check");
            return Some(1);
        }
    };
    // A vacuous check (HF2) is refused here, before the run starts. The
    // red-on-base check (gap-0ee70b) runs once, in the plan-load gate every
    // run passes before its first dispatch (3231), so it is not repeated.
    let red_on_base = std::collections::BTreeMap::new();
    let mut report = roko_cli::spec_gate::check_plans(&files, workdir, &config, &red_on_base);
    // The same holdout draw as the plan-load gate (3232), so a held-out task
    // is not refused here on its score.
    let epoch = roko_cli::spec_gate::holdout_epoch();
    roko_cli::spec_gate::apply_holdout(&mut report, config.holdout_frac, &epoch);
    if report.blocks() {
        // On stderr as well as in the log: without `--verbose` an operator
        // sees no tracing output.
        roko_cli::spec_gate::log_blocked(&report);
        Some(1)
    } else {
        None
    }
}

/// `plan validate --json` output: the report, with the `--spec-quality`
/// report when asked for and the workspace rungs when there are any.
#[derive(serde::Serialize)]
struct ValidateJson<'a> {
    #[serde(flatten)]
    report: &'a plan_validate::ValidationReport,
    #[serde(skip_serializing_if = "Option::is_none")]
    spec_quality: Option<&'a roko_gate::spec_quality::SpecQualityReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    red_on_base: Option<&'a roko_cli::spec_red_on_base::RedOnBaseReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    workspace_rungs: Option<&'a plan_validate::WorkspaceRungs>,
}

pub(crate) fn cmd_plan_validate(
    dir: &Path,
    workdir: &Path,
    strict: bool,
    json_output: bool,
    spec_quality: bool,
    dynamic: Option<roko_cli::spec_red_on_base::RedOnBaseOptions>,
) -> Result<i32> {
    let config_path = workdir.join("roko.toml");
    let config = if config_path.is_file() {
        let config_text = std::fs::read_to_string(&config_path)
            .with_context(|| format!("read {}", config_path.display()))?;
        let config: RokoConfig = toml::from_str(&config_text)
            .map_err(|error| anyhow!(error))
            .with_context(|| format!("parse {}", config_path.display()))?;
        Some(config)
    } else {
        None
    };
    let models = config
        .as_ref()
        .map(crate::commands::config_cmd::configured_models);

    let report =
        plan_validate::validate_plans_dir_with_workdir(dir, models.as_ref(), Some(workdir))?;

    // Cross-plan crate overlap analysis (#195).
    let overlaps = match roko_cli::runner::plan_loader::load_plans(dir) {
        Ok(plans) => {
            let overlaps = roko_cli::runner::plan_loader::compute_crate_overlaps(&plans);
            roko_cli::runner::plan_loader::warn_crate_overlaps(&overlaps);
            overlaps
        }
        Err(_) => Vec::new(),
    };

    // S07.9: score every task's spec with the speclint rules. Only the flag adds output.
    let spec_files = spec_quality
        .then(|| plan_validate::collect_tasks_files(dir))
        .transpose()?;
    // 3214: with --dynamic, each implementer task's checks first run on its
    // base, so the scores count SQ06 and find HF3.
    let red_on_base = match (&spec_files, dynamic) {
        (Some(files), Some(mut options)) => {
            let spec_config = config
                .as_ref()
                .map(|config| config.spec_quality.clone())
                .unwrap_or_default();
            options.timeout = options.timeout.or(Some(std::time::Duration::from_secs(
                spec_config.red_on_base_timeout_secs,
            )));
            // gap-0ee70b: cargo checks are proven at the batch gate unless
            // `[spec_quality] red_on_base_cargo` is set.
            if !spec_config.red_on_base_cargo {
                options.cargo = roko_cli::spec_red_on_base::CargoSteps::Skip;
            }
            match roko_cli::spec_red_on_base::check_plans(files, workdir, &options) {
                Ok(report) => Some(report),
                Err(error) => {
                    use roko_cli::spec_red_on_base::Interrupted;
                    if let Some(interrupted) = error.downcast_ref::<Interrupted>() {
                        eprintln!("{interrupted}");
                        return Ok(128 + interrupted.signal);
                    }
                    return Err(error.context("plan validate --dynamic"));
                }
            }
        }
        _ => None,
    };
    let spec_report = spec_files.map(|files| {
        let results = red_on_base
            .as_ref()
            .map(|report| report.results())
            .unwrap_or_default();
        roko_gate::spec_quality::lint_files_with(&files, workdir, &results)
    });

    // The workspace rungs every plan task runs after its own verify steps.
    let rungs = config
        .as_ref()
        .map(|config| plan_validate::workspace_rungs(dir, &config.gates))
        .transpose()?
        .filter(|rungs| !rungs.rungs.is_empty());

    if json_output {
        let output = ValidateJson {
            report: &report,
            spec_quality: spec_report.as_ref(),
            red_on_base: red_on_base.as_ref(),
            workspace_rungs: rungs.as_ref(),
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        let mut text = plan_validate::render_text(&report);
        if let Some(rungs) = &rungs {
            text.push_str("\n\n");
            text.push_str(&plan_validate::render_rungs_text(rungs));
        }
        if !overlaps.is_empty() {
            text.push_str("\n\ncrate overlaps detected:\n");
            for overlap in &overlaps {
                text.push_str(&format!(
                    "  warning: plans {} and {} both touch: {}\n",
                    overlap.plan_a,
                    overlap.plan_b,
                    overlap.crates.join(", "),
                ));
            }
        }
        println!("{text}");
        if let Some(spec_quality) = &spec_report {
            println!("\n{}", roko_gate::spec_quality::render_text(spec_quality));
        }
        if let Some(red_on_base) = &red_on_base {
            println!("\n{}", red_on_base.render_text());
        }
    }
    // A hard fail fails the run only under --strict; a low score never does.
    let spec_exit = spec_report
        .as_ref()
        .map_or(0, |spec_quality| spec_quality.exit_code(strict));
    Ok(report.exit_code(strict).max(spec_exit))
}

pub(crate) fn read_executor_state(
    workdir: &std::path::Path,
) -> Option<Vec<(String, usize, usize)>> {
    let executor_path = RokoLayout::for_project(workdir).executor_snapshot();
    if !executor_path.is_file() {
        return None;
    }

    let contents = std::fs::read_to_string(&executor_path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&contents).ok()?;

    if let Some(plans) = value.get("plans").and_then(serde_json::Value::as_array) {
        let mut entries = Vec::with_capacity(plans.len());
        for plan in plans {
            let id = json_str_field(plan, &["plan_id", "id"]).unwrap_or("unknown");
            let tasks_done =
                json_usize_field(plan, &["tasks_completed", "completed_tasks"]).unwrap_or(0);
            let tasks_total =
                json_usize_field(plan, &["tasks_total", "total_tasks", "task_count"]).unwrap_or(0);
            entries.push((id.to_string(), tasks_done, tasks_total));
        }
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        return Some(entries);
    }

    if let Some(plan_states) = value
        .get("plan_states")
        .and_then(serde_json::Value::as_object)
    {
        let completed_counts = read_run_state_completed_counts(workdir);
        let discovered_totals = discovered_plan_totals(workdir);
        let mut entries = Vec::with_capacity(plan_states.len());

        for (plan_id, plan_state) in plan_states {
            let tasks_total = discovered_totals.get(plan_id).copied().unwrap_or_else(|| {
                json_usize_field(plan_state, &["tasks_total", "total_tasks", "task_count"])
                    .unwrap_or(0)
            });
            let mut tasks_done = completed_counts.get(plan_id).copied().unwrap_or(0);
            if tasks_done == 0
                && tasks_total > 0
                && json_str_field(
                    plan_state
                        .get("current_phase")
                        .unwrap_or(&serde_json::Value::Null),
                    &["kind"],
                )
                .is_some_and(|kind| {
                    kind.eq_ignore_ascii_case("complete") || kind.eq_ignore_ascii_case("completed")
                })
            {
                tasks_done = tasks_total;
            }
            entries.push((plan_id.clone(), tasks_done, tasks_total));
        }

        entries.sort_by(|a, b| a.0.cmp(&b.0));
        return Some(entries);
    }

    if let Some(tasks) = value.get("tasks").and_then(serde_json::Value::as_array) {
        let mut progress: std::collections::BTreeMap<String, (usize, usize)> =
            std::collections::BTreeMap::new();
        for task in tasks {
            let Some(plan_id) = json_str_field(task, &["plan", "plan_id"]) else {
                continue;
            };
            let entry = progress.entry(plan_id.to_string()).or_insert((0, 0));
            entry.0 += 1;

            let status = task
                .get("status")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_ascii_lowercase();
            if matches!(
                status.as_str(),
                "done" | "complete" | "completed" | "passed" | "skipped"
            ) {
                entry.1 += 1;
            }
        }

        return Some(
            progress
                .into_iter()
                .map(|(plan_id, (tasks_total, tasks_done))| (plan_id, tasks_done, tasks_total))
                .collect(),
        );
    }

    Some(Vec::new())
}

fn discovered_plan_totals(workdir: &std::path::Path) -> std::collections::HashMap<String, usize> {
    roko_cli::plan::summarize_discovered_plans(workdir)
        .ok()
        .map(|summaries| {
            summaries
                .into_iter()
                .map(|summary| (summary.id, summary.task_count))
                .collect()
        })
        .unwrap_or_default()
}

fn read_run_state_completed_counts(
    workdir: &std::path::Path,
) -> std::collections::HashMap<String, usize> {
    let run_state_path = RokoLayout::for_project(workdir).run_state_path();
    let Ok(contents) = std::fs::read_to_string(&run_state_path) else {
        return std::collections::HashMap::new();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&contents) else {
        return std::collections::HashMap::new();
    };
    let Some(completed_tasks) = value
        .get("completed_tasks")
        .and_then(serde_json::Value::as_object)
    else {
        return std::collections::HashMap::new();
    };

    completed_tasks
        .iter()
        .map(|(plan_id, tasks)| {
            (
                plan_id.clone(),
                tasks.as_array().map_or(0, std::vec::Vec::len),
            )
        })
        .collect()
}

fn json_str_field<'a>(value: &'a serde_json::Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(serde_json::Value::as_str))
}

fn json_usize_field(value: &serde_json::Value, keys: &[&str]) -> Option<usize> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(serde_json::Value::as_u64))
        .map(|count| count as usize)
}

pub(crate) fn plan_path_exists(workdir: &std::path::Path, plan_id: &str) -> bool {
    let plan_dir = workdir.join("plans").join(plan_id);
    let roko_plan_dir = RokoLayout::for_project(workdir).plan_dir(plan_id);
    plan_dir.exists() || roko_plan_dir.exists()
}

/// Format validation diagnostics for a tasks.toml file into a string context
/// block suitable for embedding in agent prompts and error messages.
fn format_pre_validation_context(
    tasks_path: &std::path::Path,
    validate_fn: &dyn Fn(
        &std::path::Path,
        Option<&indexmap::IndexMap<String, roko_core::config::ModelProfile>>,
    ) -> anyhow::Result<roko_cli::plan_validate::ValidationReport>,
) -> String {
    let parent = tasks_path.parent().unwrap_or(tasks_path);
    match validate_fn(parent, None) {
        Ok(report) => {
            let issues: Vec<String> = report
                .plans
                .iter()
                .flat_map(|p| {
                    p.diagnostics
                        .iter()
                        .map(move |d| format!("  - [{}] {}", p.plan_id, d.message))
                })
                .collect();
            if issues.is_empty() {
                String::new()
            } else {
                format!("\n\n## Validation issues\n\n{}", issues.join("\n"))
            }
        }
        Err(_) => String::new(),
    }
}

/// How a real run would schedule a plan set: its execution order, the
/// effective `max_parallel_plans`, and which plans never run side by side.
struct ParallelSchedule {
    max_parallel_plans: usize,
    order: Vec<String>,
    conflicts: roko_cli::graph_execution::plan_set::PlanConflicts,
}

/// The schedule `roko plan run` would use: the `--max-parallel-plans`
/// override, else `[conductor] max_parallel_plans`. Conflicts are only
/// computed when more than one plan could run at once.
async fn parallel_schedule(
    workdir: &Path,
    plans_dir: &Path,
    plans: &[roko_cli::runner::plan_loader::Plan],
    max_parallel_plans: Option<usize>,
) -> Result<ParallelSchedule> {
    let max_parallel_plans = match max_parallel_plans {
        Some(limit) => limit,
        None => roko_core::config::loader::load_config_validated(workdir)
            .map(|loaded| loaded.into_config().conductor.max_parallel_plans)
            .unwrap_or(1),
    }
    .max(1);
    let order = roko_cli::graph_execution::plan_set_order(workdir, plans_dir, plans)?.order;
    let conflicts = if max_parallel_plans > 1 && plans.len() > 1 {
        roko_cli::graph_execution::plan_set::plan_set_conflicts(workdir, plans).await
    } else {
        roko_cli::graph_execution::plan_set::PlanConflicts::new()
    };
    Ok(ParallelSchedule {
        max_parallel_plans,
        order,
        conflicts,
    })
}

/// Print which plans a parallel run would keep apart, and why.
fn print_parallel_schedule(schedule: &ParallelSchedule) {
    if schedule.max_parallel_plans < 2 || schedule.order.len() < 2 {
        return;
    }
    println!(
        "\nParallel schedule: up to {} plans at once, started in this order: {}",
        schedule.max_parallel_plans,
        schedule.order.join(", ")
    );
    if schedule.conflicts.is_empty() {
        println!("  No plans share part of the working tree.");
        return;
    }
    for plan_id in &schedule.order {
        let Some(others) = schedule.conflicts.get(plan_id) else {
            continue;
        };
        for (other, reason) in others {
            if schedule.order.iter().position(|id| id == other)
                > schedule.order.iter().position(|id| id == plan_id)
            {
                println!("  {plan_id} and {other} never run together: {reason}");
            }
        }
    }
}

/// Validate the exact Graph plan set before any execution-only side effects.
///
/// Graph dry-run follows the same cross-plan dependency rules as execution,
/// including prerequisites outside the selected set, which must already be
/// complete on disk. The engine validates the freshly loaded set again under
/// its workspace lock to fail closed if plan files change between preflight
/// and execution.
fn validate_graph_selected_plans_before_run(
    engine: PlanEngine,
    workdir: &Path,
    plans_dir: &Path,
) -> Result<()> {
    if matches!(engine, PlanEngine::Graph) {
        let plans = roko_cli::runner::plan_loader::load_plans(plans_dir)?;
        roko_cli::graph_execution::plan_set_order(workdir, plans_dir, &plans)?;
    }
    Ok(())
}

/// The `plan run` flags the Graph engine, the only engine, does not
/// implement (gap-d60281), each with what to use instead. `plan run` stops on
/// any of them rather than run without it. `--force` (the disk-space
/// pre-check) and `--log-file` are implemented.
fn graph_unsupported_flags(
    resume_session: Option<&str>,
    effort: Option<&Effort>,
    skip_preflight: bool,
    screenshots: bool,
    screenshot_interval: u64,
    screenshot_dir: Option<&std::path::Path>,
    batch_size: Option<usize>,
) -> Vec<&'static str> {
    let mut unsupported = Vec::new();
    if resume_session.is_some() {
        unsupported.push(
            "--resume <session> resumes an agent session; resume a plan run with \
             --resume-plan (its Graph checkpoints) or `roko resume`",
        );
    }
    if effort.is_some() {
        unsupported.push("--effort: tasks run at `[agent] default_effort` from roko.toml");
    }
    if skip_preflight {
        unsupported.push("--skip-preflight: the Graph engine always runs its provider preflight");
    }
    // 60 is `--screenshot-interval`'s default.
    if screenshots || screenshot_interval != 60 || screenshot_dir.is_some() {
        unsupported.push(
            "--screenshots, --screenshot-interval and --screenshot-dir: plan runs take no \
             screenshots; capture the TUI with `roko screenshot`",
        );
    }
    if batch_size.is_some() {
        unsupported.push(
            "--batch-size: plan runs do not pause after N plans; run the plans in smaller sets",
        );
    }
    unsupported
}

/// Execute plans via the Graph Engine path.
///
/// Loads plans using the Runner v2 plan_loader, converts each to a Graph
/// via `roko_graph::convert::plan_to_graph` (default) or
/// `roko_graph::topology::ProductionPlanTopology` (when `rich_topology` is
/// true), and runs them through the GraphEngine with the default cell registry.
async fn cmd_plan_run_engine(
    plans_dir: &std::path::Path,
    workdir: &std::path::Path,
    cli: &Cli,
    resume_plan: Option<&std::path::Path>,
    fresh: bool,
    force_resume: bool,
    max_retries: Option<u32>,
    max_tasks: usize,
    budget_override: Option<f64>,
    no_budget: bool,
    frozen_learning: bool,
    no_holdout: bool,
    cli_model_override: Option<String>,
    dangerously_skip_permissions: bool,
    log_file: Option<&std::path::Path>,
    worktree_flag: Option<bool>,
    rich_topology: bool,
    promote: Option<String>,
    no_tui: bool,
    max_parallel_plans: Option<usize>,
    fail_fast: bool,
    force: bool,
) -> Result<i32> {
    use roko_cli::graph_execution::plan_runner::{
        PlanRunInterruptHandle, install_plan_run_signal_handlers, run_graph_plan,
    };

    let worktree_per_task = resolve_worktree_per_task(worktree_flag, workdir);
    if promote.is_some() && !worktree_per_task {
        anyhow::bail!(
            "--promote needs per-task worktrees: pass --worktree-per-task or set \
             [runner] worktree_per_task = true"
        );
    }

    // SIGINT, SIGTERM and SIGHUP stop this run gracefully (cancel, finalize
    // checkpoints, restore the terminal, exit 130, 143 or 129) for as long as
    // the guard lives.
    let interrupt = PlanRunInterruptHandle::default();
    let _signals = install_plan_run_signal_handlers(interrupt.clone())?;

    // Standalone `roko plan run` opens no listener, so the bind is always
    // loopback-only. Pass `effective(true)` so a `trusted` config setting is
    // honoured here even though there is no HTTP server running.
    let live_agent_output = {
        let core = roko_core::config::loader::load_config_unified(workdir)
            .map(|cfg| cfg.serve.live_agent_output)
            .unwrap_or_default()
            .effective(true);
        match core {
            roko_core::config::serve::LiveAgentOutput::Trusted => {
                roko_cli::graph_task_dispatch::LiveAgentOutput::Trusted
            }
            roko_core::config::serve::LiveAgentOutput::ToolSteps => {
                roko_cli::graph_task_dispatch::LiveAgentOutput::ToolSteps
            }
        }
    };

    // A `roko dashboard` in another terminal follows this run live through
    // the hub socket, as it follows a server's runs (gap-6533bf). Binding is
    // best-effort: without the socket the dashboard polls files as before.
    let state_hub = roko_cli::state_hub::shared_state_hub();
    #[cfg(unix)]
    let ipc_shutdown = tokio_util::sync::CancellationToken::new();
    #[cfg(unix)]
    let ipc_server = {
        use roko_cli::state_hub_ipc::start_hub_ipc_server;
        match start_hub_ipc_server(state_hub.clone(), workdir, ipc_shutdown.clone()) {
            Ok(server) => Some(server),
            Err(error) => {
                tracing::warn!(
                    %error,
                    "StateHub IPC server failed to bind; a dashboard beside this run polls files"
                );
                None
            }
        }
    };

    let params = roko_cli::graph_execution::GraphPlanRunParams {
        plans_dir: plans_dir.to_path_buf(),
        workdir: workdir.to_path_buf(),
        quiet: cli.quiet,
        json: cli.json,
        resume_plan: resume_plan.map(|p| p.to_path_buf()),
        fresh,
        force_resume,
        max_retries,
        max_tasks,
        budget_override,
        no_budget,
        cli_model_override,
        dangerously_skip_permissions,
        log_file: log_file.map(|p| p.to_path_buf()),
        worktree_per_task,
        worktree_per_task_explicit: worktree_flag == Some(true),
        rich_topology,
        promote,
        no_tui,
        state_hub: Some(state_hub),
        interrupt: Some(interrupt),
        max_parallel_plans,
        fail_fast,
        only_plans: None,
        live_agent_output,
        force_disk_check: force,
        effort: None,
        no_cascade: false,
        // `--frozen-learning` holds learned state fixed for this run alone.
        frozen_learning,
        // `--no-holdout` runs it in maximize mode (decision 4115).
        no_holdout,
        metrics: None,
        outbound_floor: None,
    };
    let exit_code = run_graph_plan(params).await;

    // Stop serving, and wait until the socket and token files are gone.
    #[cfg(unix)]
    {
        ipc_shutdown.cancel();
        if let Some(server) = ipc_server {
            let _ = server.await;
        }
    }
    exit_code
}

/// Whether a plan run in `workdir` isolates each task in its own git
/// worktree: [`roko_cli::graph_execution::batch::resolve_worktree_per_task`],
/// the one resolver `roko run` uses too (backlog 3112).
fn resolve_worktree_per_task(flag: Option<bool>, workdir: &std::path::Path) -> bool {
    roko_cli::graph_execution::batch::resolve_worktree_per_task(flag, workdir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn read_executor_state_returns_none_without_snapshot() {
        let dir = tempdir().expect("tempdir");
        assert!(read_executor_state(dir.path()).is_none());
    }

    /// gap-20ab07: `plan status` shows an interrupted or cancelled Graph run
    /// as such, not through the task statuses in tasks.toml.
    #[test]
    fn plan_status_reports_interrupted_and_cancelled_checkpoints() {
        let dir = tempdir().expect("tempdir");
        let checkpoint_dir = dir.path().join(".roko/state/graph/demo-plan");
        std::fs::create_dir_all(&checkpoint_dir).expect("checkpoint dir");
        for (recorded, label) in [
            ("interrupted", "interrupted"),
            ("cancelled", "cancelled"),
            ("succeeded", "complete"),
            ("failed", "failed"),
        ] {
            let checkpoint = format!(r#"{{"status":"{recorded}"}}"#);
            std::fs::write(checkpoint_dir.join("checkpoint.json"), checkpoint).expect("checkpoint");
            let status =
                roko_cli::graph_checkpoint::canonical_checkpoint_status(dir.path(), "demo-plan");
            // tasks.toml says one of two tasks is done.
            assert_eq!(plan_status_label(status, 1, 2), label, "{recorded}");
        }

        assert_eq!(plan_status_label(None, 0, 2), "not started");
        assert_eq!(plan_status_label(None, 1, 2), "in progress");
        assert_eq!(plan_status_label(None, 2, 2), "complete");
    }

    #[test]
    fn read_executor_state_parses_plans_array() {
        let dir = tempdir().expect("tempdir");
        let state_dir = RokoLayout::for_project(dir.path()).state_dir();
        std::fs::create_dir_all(&state_dir).expect("state dir");
        std::fs::write(
            state_dir.join("executor.json"),
            r#"{"plans":[{"plan_id":"plan-a","tasks_completed":1,"tasks_total":3}]}"#,
        )
        .expect("write executor state");

        let state = read_executor_state(dir.path()).expect("state");
        assert_eq!(state, vec![("plan-a".to_string(), 1, 3)]);
    }

    #[test]
    fn graph_selected_plan_preflight_runs_without_creating_workspace_lock() {
        let workspace = tempdir().expect("tempdir");
        let plans_dir = workspace.path().join("plans");
        let consumer_dir = plans_dir.join("consumer");
        std::fs::create_dir_all(&consumer_dir).expect("create consumer plan directory");
        std::fs::write(
            consumer_dir.join("tasks.toml"),
            r#"
[meta]
plan = "consumer"

[[task]]
id = "T1"
title = "Consume foundation"
role = "researcher"
depends_on_plan = ["missing-foundation"]
"#,
        )
        .expect("write consumer plan");
        let lock_path = workspace.path().join(".roko/runtime/roko.lock");

        let error = validate_graph_selected_plans_before_run(
            PlanEngine::Graph,
            workspace.path(),
            &plans_dir,
        )
        .expect_err("unknown selected dependency");

        assert!(error.to_string().contains("missing-foundation"));
        assert!(
            !lock_path.exists(),
            "Graph preflight must not create a lock"
        );
    }

    /// 3209: a task with an open question keeps its plan from running: plan
    /// validation fails with PLAN_045, which lists the question, and `plan
    /// run` stops before any agent starts. Answered and deleted, it runs.
    #[test]
    fn open_questions_block_dispatch() {
        let workspace = tempdir().expect("tempdir");
        let plans = workspace.path().join("plans");
        std::fs::create_dir_all(plans.join("questions")).expect("plan dir");
        let write_plan = |questions: &str| {
            let tasks = format!(
                r#"
[meta]
plan = "questions"

[[task]]
id = "T1"
title = "Retry limit"
role = "implementer"
files = ["src/config.rs"]
depends_on = []
open_questions = [{questions}]
verify = [{{ phase = "test", command = "cargo test -p demo --lib retry" }}]
"#
            );
            std::fs::write(plans.join("questions/tasks.toml"), tasks).expect("write the plan");
        };

        write_plan(r#""Is the limit per call or per task?""#);
        let report = plan_validate::validate_plans_dir(&plans, None).expect("validate");
        let questions: Vec<_> = report
            .plans
            .iter()
            .flat_map(|plan| &plan.diagnostics)
            .filter(|diagnostic| diagnostic.rule_id == "PLAN_045")
            .collect();
        assert_eq!(questions.len(), 1, "{report:?}");
        assert_eq!(questions[0].severity, plan_validate::Severity::Error);
        assert!(
            questions[0]
                .message
                .ends_with(":\n- Is the limit per call or per task?"),
            "{}",
            questions[0].message
        );
        let text = plan_validate::render_text(&report);
        assert!(
            text.contains("\n                 - Is the limit per call or per task?\n"),
            "the question is printed under the task: {text}"
        );
        assert_eq!(
            validate_before_run(&plans, workspace.path(), false),
            Some(1)
        );

        write_plan("");
        assert_eq!(validate_before_run(&plans, workspace.path(), false), None);
    }

    /// 3211: `plan run` refuses a plan whose verify step can never fail
    /// before any agent starts. With `[spec_quality] mode = "off"` it does
    /// not, and a low-scoring task without a hard fail runs.
    #[test]
    fn plan_run_refuses_a_vacuous_verify_step() {
        let workspace = tempdir().expect("tempdir");
        let plans = workspace.path().join("plans");
        std::fs::create_dir_all(plans.join("vacuous")).expect("plan dir");
        let write_plan = |command: &str| {
            let tasks = format!(
                r#"
[meta]
plan = "vacuous"

[[task]]
id = "T1"
title = "Retry limit"
description = "Add the retry limit to `parse_config`."
role = "implementer"
files = ["src/config.rs"]
depends_on = []
verify = [{{ phase = "compile", command = "{command}" }}]
"#
            );
            std::fs::write(plans.join("vacuous/tasks.toml"), tasks).expect("write the plan");
        };

        write_plan("cargo check -p x || true");
        assert_eq!(
            validate_before_run(&plans, workspace.path(), false),
            Some(1)
        );

        let config = workspace.path().join("roko.toml");
        std::fs::write(&config, "[spec_quality]\nmode = \"off\"\n").expect("write roko.toml");
        assert_eq!(validate_before_run(&plans, workspace.path(), false), None);
        std::fs::remove_file(&config).expect("remove roko.toml");

        // Scores advise: a vague but checkable task still runs.
        write_plan("cargo test -p x --lib retry");
        assert_eq!(validate_before_run(&plans, workspace.path(), false), None);
    }

    /// gap-d60281: `plan run` stops on a flag the Graph engine does not
    /// implement and says what to use instead. `--force` and `--log-file`,
    /// which it implements, take no part in the check.
    #[test]
    fn graph_plan_run_rejects_or_honours_legacy_flags() {
        assert!(graph_unsupported_flags(None, None, false, false, 60, None, None).is_empty());

        let resume = graph_unsupported_flags(Some("s1"), None, false, false, 60, None, None);
        assert_eq!(resume.len(), 1, "{resume:?}");
        assert!(resume[0].contains("--resume-plan"), "{resume:?}");

        let high = Effort::High;
        let effort = graph_unsupported_flags(None, Some(&high), false, false, 60, None, None);
        assert!(effort[0].contains("default_effort"), "{effort:?}");

        let preflight = graph_unsupported_flags(None, None, true, false, 60, None, None);
        assert!(
            preflight[0].starts_with("--skip-preflight"),
            "{preflight:?}"
        );

        let dir = std::path::Path::new("shots");
        for shots in [
            graph_unsupported_flags(None, None, false, true, 60, None, None),
            graph_unsupported_flags(None, None, false, false, 30, None, None),
            graph_unsupported_flags(None, None, false, false, 60, Some(dir), None),
        ] {
            assert_eq!(shots.len(), 1, "{shots:?}");
            assert!(shots[0].contains("roko screenshot"), "{shots:?}");
        }

        let batch = graph_unsupported_flags(None, None, false, false, 60, None, Some(5));
        assert!(batch[0].starts_with("--batch-size"), "{batch:?}");

        let all =
            graph_unsupported_flags(Some("s1"), Some(&high), true, true, 30, Some(dir), Some(5));
        assert_eq!(all.len(), 5, "{all:?}");
    }
}
