//! `roko run`: run a prompt, or a plan directory, through the Graph engine.
//!
//! `roko run` is the one entry point for work. `roko do` and the PRD pipeline
//! (`roko prd`) were folded into it on 2026-10-02 (tmp/workflow-audit), and
//! `roko develop` is gone: a prompt either runs as one checked task or becomes
//! a plan, and a plan directory runs as `roko plan run` runs it.
//!
//! ## Routes
//!
//! Two checks on the input come first; the rest is the pure
//! [`resolve_run_route`], table-tested below.
//!
//! | Input                              | Route                                         |
//! |------------------------------------|-----------------------------------------------|
//! | a plan directory (`plans/<slug>`)  | run it as `roko plan run <dir>` does          |
//! | one word that names `plans/<word>` | print `roko run plans/<word>`; run nothing    |
//! | `--serve` / `--share`              | one task, with the HTTP control plane         |
//! | prompt sized trivial               | one task at the `mechanical` tier             |
//! | prompt sized simple                | one task at the `focused` tier                |
//! | prompt sized standard or complex   | write a plan from the prompt, then run it     |
//! | `--plan`                           | write a plan; on a terminal show it and ask   |
//! | `--plan --dry-run`                 | write the plan and stop                       |
//! | `--dry-run`                        | print the route; run nothing                  |
//!
//! `--complexity` replaces the detected size. `roko run` does not route by
//! intent: a question is a prompt like any other (`roko think` and
//! `roko research` answer questions).

use crate::*;
use roko_core::TaskDomain;
use roko_core::config::schema::RokoConfig;
use roko_gate::PlanComplexity;
use std::path::{Path, PathBuf};

use super::plan::{PlanCmd, PlanEngine};

// ─── Routing types ──────────────────────────────────────────────────────

/// Resolved route for a prompt given to `roko run`.
///
/// Each variant maps to one row of the routing table. The resolver is pure:
/// no I/O, no config loading, no filesystem probing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunRoute {
    /// One task at the `mechanical` tier (`mechanical@1`).
    Mechanical,
    /// One task at the `focused` tier (`focused@1`).
    Focused,
    /// Write a plan from the prompt, then run it.
    PromptPlan,
    /// `--dry-run` without `--plan`: print the route, run nothing.
    DryRun {
        /// The route that would run.
        inner: Box<RunRoute>,
    },
}

impl RunRoute {
    /// Canonical template name for display in previews.
    #[must_use]
    pub fn template_name(&self) -> &'static str {
        match self {
            Self::Mechanical => "mechanical@1",
            Self::Focused => "focused@1",
            Self::PromptPlan => "prompt-plan",
            Self::DryRun { inner } => inner.template_name(),
        }
    }

    /// Estimated cost band for display in previews.
    #[must_use]
    pub fn cost_band(&self) -> &'static str {
        match self {
            Self::Mechanical => "<$0.01",
            Self::Focused => "$0.01-$0.05",
            Self::PromptPlan => "$0.05-$0.25",
            Self::DryRun { inner } => inner.cost_band(),
        }
    }

    /// Human-readable pipeline description.
    #[must_use]
    pub fn pipeline_description(&self) -> &'static str {
        match self {
            Self::Mechanical => "single agent (mechanical)",
            Self::Focused => "single agent (focused)",
            Self::PromptPlan => "generate plan -> execute",
            Self::DryRun { inner } => inner.pipeline_description(),
        }
    }
}

/// Inputs to the pure routing resolver.
#[derive(Debug, Clone)]
pub struct RunRouteInput {
    /// Detected or forced size of the prompt.
    pub complexity: PlanComplexity,
    /// Whether `--plan` was passed.
    pub plan_flag: bool,
    /// Whether the route is only to be printed (`--dry-run` without `--plan`).
    pub dry_preview: bool,
}

/// Pure routing resolver: maps [`RunRouteInput`] to a [`RunRoute`].
///
/// `--plan` always plans. Otherwise trivial and simple prompts run as one
/// task, standard and complex ones get a plan first.
#[must_use]
pub fn resolve_run_route(input: &RunRouteInput) -> RunRoute {
    let base = if input.plan_flag {
        RunRoute::PromptPlan
    } else {
        match input.complexity {
            PlanComplexity::Trivial => RunRoute::Mechanical,
            PlanComplexity::Simple => RunRoute::Focused,
            PlanComplexity::Standard | PlanComplexity::Complex => RunRoute::PromptPlan,
        }
    };
    if input.dry_preview {
        return RunRoute::DryRun {
            inner: Box::new(base),
        };
    }
    base
}

/// What `roko run` was asked to do: its flags, as clap parsed them.
#[derive(Debug, Clone, Default)]
pub(crate) struct RunArgs {
    /// The prompt words, or one plan directory.
    pub(crate) input: Vec<String>,
    /// `--plan`: write a plan first, whatever the prompt's size.
    pub(crate) plan: bool,
    /// `--dry-run`: print the route, or with `--plan` write the plan and stop.
    pub(crate) dry_run: bool,
    /// `--yes`: run a `--plan` plan without asking.
    pub(crate) yes: bool,
    /// `--complexity`: the forced size.
    pub(crate) complexity: Option<PlanComplexity>,
    /// `--context`: extra context for the planner.
    pub(crate) context: Vec<PathBuf>,
    /// `--no-cascade`.
    pub(crate) no_cascade: bool,
    /// `--workdir`.
    pub(crate) workdir: Option<PathBuf>,
    /// `--serve`: one task with the HTTP control plane.
    pub(crate) serve: bool,
    /// `--share`: one task with a share URL.
    pub(crate) share: bool,
    /// `--provider`: the provider of a one-task run.
    pub(crate) provider: Option<String>,
    /// `--max-retries`.
    pub(crate) max_retries: Option<u32>,
    /// `--domain`: the work-domain label of a one-task run (9121).
    pub(crate) domain: Option<String>,
    /// `--fresh` (plan directories only).
    pub(crate) fresh: bool,
    /// `--resume-plan` (plan directories only).
    pub(crate) resume_plan: Option<PathBuf>,
}

/// Main entry point for `roko run`.
pub(crate) async fn cmd_run(cli: &Cli, args: RunArgs) -> Result<i32> {
    let workdir = args.workdir.clone().unwrap_or_else(|| resolve_workdir(cli));
    let input = args.input.join(" ").trim().to_string();
    let out = roko_cli::cli_output::CliOutput::new(cli.quiet);

    if input.is_empty() {
        out.error("the prompt is empty");
        out.step(
            "Usage",
            "roko run \"fix the bug\"   or   roko run plans/<slug>",
        );
        return Ok(EXIT_AGENT_FAILURE);
    }

    match plan_argument(&workdir, &input) {
        PlanArgument::Plan(plans_dir) => {
            return run_plan_dir(cli, &workdir, plans_dir, &args).await;
        }
        PlanArgument::NoPlans(dir) => {
            out.error(&format!(
                "{} holds no plan: no tasks.toml in it or in a directory just below it",
                dir.display()
            ));
            out.step("Usage", "roko run plans/<slug> runs a plan; quote a prompt");
            return Ok(EXIT_AGENT_FAILURE);
        }
        PlanArgument::Missing(path) => {
            out.error(&format!("no such plan directory: {}", path.display()));
            out.step("Plans", "roko plan list");
            return Ok(EXIT_AGENT_FAILURE);
        }
        PlanArgument::Prompt => {}
    }
    if args.fresh || args.resume_plan.is_some() {
        out.error("--fresh and --resume-plan apply to a plan directory (roko run plans/<slug>)");
        return Ok(EXIT_AGENT_FAILURE);
    }

    // One word that names a plan: say how to run the plan instead of running
    // the word as a prompt, so a prompt never runs a plan by accident (#278).
    if !input.contains(char::is_whitespace)
        && let Some(plan_dir) = named_plan_dir(&workdir, &input)
    {
        out.step(
            "Plan found",
            &format!("\"{input}\" names the plan at {}", plan_dir.display()),
        );
        out.step(
            "Run with",
            &format!("roko run {}", display_path(&workdir, &plan_dir)),
        );
        return Ok(EXIT_SUCCESS);
    }

    if args.serve || args.share {
        if args.plan || args.dry_run {
            out.error(
                "--serve and --share run the prompt as one task; they do not combine with \
                 --plan or --dry-run",
            );
            return Ok(EXIT_AGENT_FAILURE);
        }
        return super::util::cmd_run(
            cli,
            Some(workdir),
            input,
            args.serve,
            args.share,
            args.provider,
            args.max_retries,
            None,
            args.domain.as_deref().and_then(TaskDomain::from_label),
        )
        .await;
    }

    let preview_config = load_resolved_config(&workdir)
        .map(|resolved| resolved.config)
        .unwrap_or_default();
    let forced = args.complexity.is_some();
    // `--plan` plans whatever the size, so the prompt is not sized for it.
    let complexity = match args.complexity {
        Some(complexity) => complexity,
        None if args.plan => PlanComplexity::Standard,
        None => {
            let scope_config = scope_model_config_from_cli_config(&preview_config);
            roko_cli::scope_resolver::ScopeResolver::resolve(&input, &scope_config).await
        }
    };

    let route = resolve_run_route(&RunRouteInput {
        complexity,
        plan_flag: args.plan,
        dry_preview: args.dry_run && !args.plan,
    });

    tracing::info!(
        ?route,
        template = route.template_name(),
        cost_band = route.cost_band(),
        pipeline = route.pipeline_description(),
        complexity_forced = forced,
        plan_flag = args.plan,
        "resolved run route (#278 deterministic template routing)"
    );

    if let RunRoute::DryRun { ref inner } = route {
        print_run_preview(&input, complexity, forced, &args, &preview_config, inner);
        return Ok(EXIT_SUCCESS);
    }

    // A run owns the runner slot for its whole lifetime, plan generation
    // included. Read-only commands with shared workspace locks coexist.
    let _lock = roko_cli::workspace_lock::acquire_runner_lock(&workdir.join(".roko"))?;

    match route {
        RunRoute::Mechanical | RunRoute::Focused => {
            run_one_task(cli, &workdir, &input, complexity, forced, &args).await
        }
        RunRoute::PromptPlan => {
            run_prompt_plan(cli, &workdir, &input, complexity, forced, &args).await
        }
        RunRoute::DryRun { .. } => unreachable!("dry-run routes return above"),
    }
}

// ─── Plan directories ───────────────────────────────────────────────────

/// What the positional argument of `roko run` names.
#[derive(Debug, PartialEq, Eq)]
enum PlanArgument {
    /// A plan directory, or a directory of plans, to run.
    Plan(PathBuf),
    /// An existing directory that holds no plan.
    NoPlans(PathBuf),
    /// One word written as a path (`plans/x`, `./x`, `x/tasks.toml`) that
    /// does not exist.
    Missing(PathBuf),
    /// Prompt text.
    Prompt,
}

/// Read `input` as a path when it is one word naming an existing directory
/// (relative to `workdir`, or absolute) or a `tasks.toml` file. A directory
/// is a plan when it holds a `tasks.toml`, or when a directory just below it
/// does (`plans/`, a plan set). One word written as a path that does not
/// exist is reported as missing rather than run as a prompt.
fn plan_argument(workdir: &Path, input: &str) -> PlanArgument {
    if input.contains(char::is_whitespace) {
        return PlanArgument::Prompt;
    }
    let raw = Path::new(input);
    let path = if raw.is_absolute() {
        raw.to_path_buf()
    } else {
        workdir.join(raw)
    };
    if path.is_file() && path.file_name().is_some_and(|name| name == "tasks.toml") {
        return path.parent().map_or(PlanArgument::Prompt, |dir| {
            PlanArgument::Plan(dir.to_path_buf())
        });
    }
    if path.is_dir() {
        return if holds_plan(&path) {
            PlanArgument::Plan(path)
        } else {
            PlanArgument::NoPlans(path)
        };
    }
    let looks_like_path = input.contains('/') || input.ends_with("tasks.toml");
    if looks_like_path {
        PlanArgument::Missing(path)
    } else {
        PlanArgument::Prompt
    }
}

/// Whether `dir` holds a `tasks.toml`, or a directory just below it does.
fn holds_plan(dir: &Path) -> bool {
    if dir.join("tasks.toml").is_file() {
        return true;
    }
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .any(|entry| entry.path().join("tasks.toml").is_file())
}

/// The plan directory a single `word` names, in the workspace plans
/// directory (`plans/<word>`) or the legacy `.roko/plans/<word>`.
fn named_plan_dir(workdir: &Path, word: &str) -> Option<PathBuf> {
    [
        workdir.join("plans").join(word),
        workdir.join(".roko").join("plans").join(word),
    ]
    .into_iter()
    .find(|dir| dir.join("tasks.toml").is_file())
}

/// `path` relative to `workdir` when it lies inside it, for hints.
fn display_path(workdir: &Path, path: &Path) -> String {
    path.strip_prefix(workdir)
        .unwrap_or(path)
        .display()
        .to_string()
}

/// Run a plan directory as `roko plan run <dir>` runs it, with the plan-run
/// options `roko run` shares (`--dry-run`, `--fresh`, `--resume-plan`,
/// `--max-retries`). The options that only mean something for a prompt are
/// refused.
async fn run_plan_dir(
    cli: &Cli,
    workdir: &Path,
    plans_dir: PathBuf,
    args: &RunArgs,
) -> Result<i32> {
    let prompt_only: Vec<&str> = [
        (args.plan, "--plan"),
        (args.yes, "--yes"),
        (args.complexity.is_some(), "--complexity"),
        (!args.context.is_empty(), "--context"),
        (args.no_cascade, "--no-cascade"),
        (args.provider.is_some(), "--provider"),
        (args.serve, "--serve"),
        (args.share, "--share"),
    ]
    .into_iter()
    .filter_map(|(set, flag)| set.then_some(flag))
    .collect();
    if !prompt_only.is_empty() {
        let out = roko_cli::cli_output::CliOutput::new(cli.quiet);
        out.error(&format!(
            "{} only applies to a prompt, not to a plan directory",
            prompt_only.join(", ")
        ));
        out.step(
            "Options",
            "roko plan run --help lists every plan-run option",
        );
        return Ok(EXIT_AGENT_FAILURE);
    }

    // Neither worktree flag: the run follows `[runner] worktree_per_task`,
    // resolved as `roko plan run` resolves it (backlog 3112).
    let (worktree_flag, no_worktree_flag) = (false, false);
    let plan_cmd = PlanCmd::Run {
        plans_dir,
        engine: PlanEngine::default(),
        workdir: Some(workdir.to_path_buf()),
        resume_plan: args.resume_plan.clone(),
        approval: false,
        no_tui: false,
        max_retries: args.max_retries,
        max_tasks: 0,
        dry_run: args.dry_run,
        fresh: args.fresh,
        force_resume: false,
        budget_override: None,
        no_budget: false,
        force: false,
        dangerously_skip_permissions: false,
        log_file: None,
        skip_preflight: false,
        screenshots: false,
        screenshot_interval: 60,
        screenshot_dir: None,
        batch_size: None,
        worktree_per_task: worktree_flag,
        no_worktree_per_task: no_worktree_flag,
        rich_topology: false,
        // A one-task run learns as usual (2219: `--frozen-learning` is a
        // `plan run` flag).
        frozen_learning: false,
        no_holdout: false,
        promote: None,
        max_parallel_plans: None,
        fail_fast: false,
    };
    let result = super::plan::cmd_plan(cli, plan_cmd).await;
    // `roko plan run` rebuilds the plan indexes after a run that succeeds
    // (main.rs wraps it); this is the same run, so it does too.
    if !args.dry_run && matches!(result, Ok(EXIT_SUCCESS)) {
        roko_cli::index::rebuild_all(workdir)?;
    }
    result
}

// ─── One task: a one-task plan through the Graph engine ─────────────────

async fn run_one_task(
    cli: &Cli,
    workdir: &Path,
    prompt: &str,
    complexity: PlanComplexity,
    forced: bool,
    args: &RunArgs,
) -> Result<i32> {
    let tier = workflow_template_for_complexity(complexity);

    // `--json` keeps stdout to the one report document.
    let out = roko_cli::cli_output::CliOutput::new(cli.quiet || cli.json);
    out.step(
        "Complexity",
        &format!(
            "{} ({})",
            complexity_label(complexity),
            if forced { "forced" } else { "auto-detected" }
        ),
    );
    out.step("Running", "single agent...");

    prepare_runtime_hooks(workdir, cli.quiet);

    let overrides = roko_cli::run::CliOverrides {
        model: cli.model.clone(),
        role: cli.role.clone(),
        provider: args.provider.clone(),
        cascade_enabled: Some(!args.no_cascade),
        effort: cli.effort.map(|e| e.to_string()),
    };

    tracing::debug!(
        complexity = complexity_label(complexity),
        tier,
        cascade_enabled = !args.no_cascade,
        engine = "graph",
        "dispatching roko run as a one-task Graph plan"
    );

    let result = roko_cli::run::run_prompt(roko_cli::run::PromptRun {
        prompt,
        workdir,
        tier,
        overrides: &overrides,
        max_retries: args.max_retries,
        quiet: cli.quiet || cli.json,
        state_hub: None,
        run_id: None,
        cancel: None,
        domain: args.domain.as_deref().and_then(TaskDomain::from_label),
        max_usd: None,
        origin: roko_serve::runtime::RunOrigin::Cli,
    })
    .await;

    handle_workflow_result(cli, prompt, tier, result)
}

// ─── Plan: write a plan from the prompt, then run it ────────────────────

async fn run_prompt_plan(
    cli: &Cli,
    workdir: &Path,
    prompt: &str,
    complexity: PlanComplexity,
    forced: bool,
    args: &RunArgs,
) -> Result<i32> {
    let out = roko_cli::cli_output::CliOutput::new(cli.quiet);
    if args.plan {
        out.step("Plan", "writing a plan first (--plan)");
    } else {
        out.step(
            "Complexity",
            &format!(
                "{} ({})",
                complexity_label(complexity),
                if forced {
                    "forced"
                } else {
                    "auto-detected, override with --complexity simple"
                }
            ),
        );
    }
    let steps = if args.dry_run { "1/1" } else { "1/2" };
    out.step(&format!("Step {steps}"), "Generating plan...");

    prepare_runtime_hooks(workdir, cli.quiet);

    let model_key = roko_cli::model_selection::resolve_planner_model(
        workdir,
        cli.model.clone(),
        "roko run (plan)",
    )?;

    // Pre-flight: check provider.
    {
        let run_config: RokoConfig = std::fs::read_to_string(workdir.join("roko.toml"))
            .ok()
            .and_then(|s| match RokoConfig::from_toml(&s) {
                Ok(cfg) => Some(cfg),
                Err(e) => {
                    tracing::warn!(
                        path = %workdir.join("roko.toml").display(),
                        error = %e,
                        "roko.toml parse error; using default config. Run `roko config validate` to fix."
                    );
                    None
                }
            })
            .unwrap_or_default();
        crate::commands::util::preflight_provider_for_model(&run_config, &model_key)?;
    }

    // Generate the plan from the prompt.
    let context_block = if args.context.is_empty() {
        String::new()
    } else {
        let loaded = roko_cli::context_loader::load_context_files(
            &args.context,
            roko_cli::context_loader::DEFAULT_BUDGET,
            workdir,
        );
        if loaded.is_empty() {
            String::new()
        } else {
            format!("<context>\n{loaded}</context>")
        }
    };
    let Some(generated) =
        generate_prompt_plan(cli, workdir, prompt, &model_key, &context_block, &out).await
    else {
        return Ok(EXIT_AGENT_FAILURE);
    };

    let plans = match roko_cli::runner::plan_loader::load_plans(&generated) {
        Ok(plans) if plans.is_empty() => {
            out.error("Plan generation produced no executable plans");
            return Ok(EXIT_AGENT_FAILURE);
        }
        Ok(plans) => plans,
        Err(err) => {
            out.error(&format!("Failed to load generated plans: {err:#}"));
            return Ok(EXIT_AGENT_FAILURE);
        }
    };
    let total_tasks: usize = plans.iter().map(|p| p.tasks.tasks.len()).sum();
    let run_hint = format!("roko run {}", display_path(workdir, &generated));
    // The plans index lists the new plan, as after `roko plan generate`.
    roko_cli::index::rebuild_all(workdir)?;

    if args.dry_run {
        out.step(
            "Plan written",
            &format!("{} ({total_tasks} tasks)", generated.display()),
        );
        out.step(
            "Next",
            &format!("review or edit it, then run it with `{run_hint}`"),
        );
        return Ok(EXIT_SUCCESS);
    }

    if args.plan && !confirm_plan_run(cli, &generated, &plans, args.yes)? {
        out.step(
            "Plan kept",
            &format!("{}; run it later with `{run_hint}`", generated.display()),
        );
        return Ok(EXIT_SUCCESS);
    }

    if args.provider.is_some() {
        out.warning(
            "--provider applies to one-task runs; the plan's tasks use the providers their \
             models route to",
        );
    }

    out.step(
        "Step 2/2",
        &format!("Executing plan ({total_tasks} tasks)..."),
    );
    let code =
        run_plan_execution(cli, workdir, &generated, args.no_cascade, args.max_retries).await?;
    // As after `roko plan run`, the indexes show the run's result.
    if code == EXIT_SUCCESS {
        roko_cli::index::rebuild_all(workdir)?;
    }
    Ok(code)
}

/// On a terminal, show the plan `--plan` wrote and ask whether to run it now.
/// `--yes`, `--json`, or input that is not a terminal run it without asking;
/// end of input keeps the plan without running it.
fn confirm_plan_run(
    cli: &Cli,
    plan_dir: &Path,
    plans: &[roko_cli::runner::plan_loader::Plan],
    yes: bool,
) -> Result<bool> {
    use std::io::{BufRead, Write};

    if yes || cli.json || !roko_cli::stdin_is_tty() {
        return Ok(true);
    }
    let mut err = std::io::stderr().lock();
    writeln!(err, "\nPlan written to {}:", plan_dir.display())?;
    for plan in plans {
        writeln!(err, "  {} ({} tasks)", plan.id, plan.tasks.tasks.len())?;
        for task in &plan.tasks.tasks {
            if task.depends_on.is_empty() {
                writeln!(err, "    {}  {}", task.id, task.title)?;
            } else {
                writeln!(
                    err,
                    "    {}  {}  (after {})",
                    task.id,
                    task.title,
                    task.depends_on.join(", ")
                )?;
            }
        }
    }
    write!(err, "Run this plan now? [Y/n] ")?;
    err.flush()?;
    drop(err);

    let mut answer = String::new();
    if std::io::stdin().lock().read_line(&mut answer)? == 0 {
        return Ok(false);
    }
    Ok(plan_answer_runs(&answer))
}

/// Whether an answer to "Run this plan now? [Y/n]" means yes: empty (the
/// default), `y` or `yes`, in any case.
fn plan_answer_runs(answer: &str) -> bool {
    matches!(
        answer.trim().to_ascii_lowercase().as_str(),
        "" | "y" | "yes"
    )
}

// ─── Shared: execute a plan directory through the Graph engine ──────────

pub(crate) async fn run_plan_execution(
    cli: &Cli,
    workdir: &Path,
    plans_dir: &Path,
    no_cascade: bool,
    max_retries: Option<u32>,
) -> Result<i32> {
    use roko_cli::graph_execution::plan_runner::{
        PlanRunInterruptHandle, install_plan_run_signal_handlers, run_graph_plan,
    };

    let out = roko_cli::cli_output::CliOutput::new(cli.quiet);

    // Load and verify plans exist.
    let plans = roko_cli::runner::plan_loader::load_plans(plans_dir)?;
    if plans.is_empty() {
        out.error(&format!("No plans found at {}", plans_dir.display()));
        return Ok(EXIT_AGENT_FAILURE);
    }

    // Scaffold missing crates if needed.
    let scaffolded = roko_cli::runner::plan_loader::scaffold_missing_crates(workdir, &plans)?;
    if !scaffolded.is_empty() {
        out.step(
            "Scaffolded",
            &format!(
                "{} new crate(s): {}",
                scaffolded.len(),
                scaffolded.join(", ")
            ),
        );
    }

    // SIGINT, SIGTERM and SIGHUP stop the run gracefully (cancel, finalize
    // checkpoints, exit 130, 143 or 129) for as long as the guard lives. The
    // Graph engine prints the run summary itself: one JSON document with
    // --json, else a line.
    let interrupt = PlanRunInterruptHandle::default();
    let _signals = install_plan_run_signal_handlers(interrupt.clone())?;
    run_graph_plan(roko_cli::graph_execution::GraphPlanRunParams {
        plans_dir: plans_dir.to_path_buf(),
        workdir: workdir.to_path_buf(),
        quiet: cli.quiet,
        json: cli.json,
        resume_plan: None,
        fresh: false,
        force_resume: false,
        max_retries,
        // Each plan's own `max_parallel`, as `roko plan run` defaults to.
        max_tasks: 0,
        budget_override: None,
        no_budget: false,
        cli_model_override: cli.model.clone(),
        dangerously_skip_permissions: false,
        log_file: None,
        // `[runner] worktree_per_task`, as `roko plan run` resolves it with no
        // flag (backlog 3113); the run then ends with the merge hint.
        worktree_per_task: roko_cli::graph_execution::batch::resolve_worktree_per_task(
            None, workdir,
        ),
        worktree_per_task_explicit: false,
        rich_topology: false,
        promote: None,
        // Inline progress on stderr; `roko dashboard` is the TUI.
        no_tui: true,
        state_hub: None,
        interrupt: Some(interrupt),
        max_parallel_plans: None,
        fail_fast: false,
        only_plans: None,
        live_agent_output: roko_cli::graph_task_dispatch::LiveAgentOutput::ToolSteps,
        force_disk_check: false,
        effort: cli.effort.map(|effort| effort.to_string()),
        no_cascade,
        frozen_learning: false,
        no_holdout: false,
        metrics: None,
    })
    .await
}

/// Plan `prompt` with the one plan generator (gap-2623b2), which validates
/// the plan and writes it to the workspace plans directory; `context` follows
/// the prompt. Returns the new plan's directory, so `roko run` runs that plan
/// and no other plan beside it, or `None` after reporting a failure on `out`.
async fn generate_prompt_plan(
    cli: &Cli,
    workdir: &Path,
    prompt: &str,
    model_key: &str,
    context: &str,
    out: &roko_cli::cli_output::CliOutput,
) -> Option<PathBuf> {
    let effort = cli.effort.map(|e| e.to_string());
    let slug = roko_cli::plan_generate::slugify(prompt);
    let request = roko_cli::plan_generate::PlanRequest {
        context: Some(context),
        model: Some(model_key),
        effort: Some(effort.as_deref().unwrap_or("high")),
        ..roko_cli::plan_generate::PlanRequest::new(
            roko_cli::plan_generate::PlanSource::Text {
                text: prompt,
                kind: "prompt",
            },
            &slug,
            workdir,
        )
    };
    match roko_cli::plan_generate::generate_plan(request).await {
        Ok((plans_root, _)) => Some(plans_root.join(&slug)),
        Err(err) => {
            out.error(&format!("Plan generation failed: {err:#}"));
            None
        }
    }
}

// ─── Preview / formatting helpers ───────────────────────────────────────

fn print_run_preview(
    prompt: &str,
    complexity: PlanComplexity,
    forced: bool,
    args: &RunArgs,
    config: &Config,
    route: &RunRoute,
) {
    let gate_count = roko_cli::run::workflow_enabled_gate_names(&config.gates).len();

    println!("roko run");
    println!("prompt      : {}", truncate_for_preview(prompt, 80));
    println!(
        "complexity  : {} ({})",
        complexity_label(complexity),
        if forced {
            "forced"
        } else {
            "auto-detected, override with --complexity simple"
        }
    );
    println!("template    : {}", route.template_name());
    println!("pipeline    : {}", route.pipeline_description());
    println!("cost        : {}", route.cost_band());
    println!("gates       : {gate_count}");
    println!(
        "cascade     : {}",
        if args.no_cascade {
            "disabled"
        } else {
            "enabled"
        }
    );
    println!("execution   : skipped");
}

fn handle_workflow_result(
    cli: &Cli,
    prompt: &str,
    tier: &str,
    result: anyhow::Result<roko_runtime::workflow_contract::WorkflowRunReport>,
) -> Result<i32> {
    match result {
        Ok(report) => {
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else if !cli.quiet {
                roko_cli::run::print_workflow_run_report(prompt, tier, &report);
            }

            if report.success {
                Ok(EXIT_SUCCESS)
            } else {
                Ok(EXIT_AGENT_FAILURE)
            }
        }
        Err(error) => {
            let out = roko_cli::cli_output::CliOutput::new(cli.quiet);
            out.error(&format!("roko run failed: {error:#}"));
            Ok(EXIT_AGENT_FAILURE)
        }
    }
}

/// Task tier a one-task route runs the prompt at.
pub(crate) fn workflow_template_for_complexity(complexity: PlanComplexity) -> &'static str {
    match complexity {
        PlanComplexity::Trivial => "mechanical",
        PlanComplexity::Simple => "focused",
        PlanComplexity::Standard => "integrative",
        PlanComplexity::Complex => "architectural",
    }
}

fn complexity_label(complexity: PlanComplexity) -> &'static str {
    match complexity {
        PlanComplexity::Trivial => "trivial",
        PlanComplexity::Simple => "simple",
        PlanComplexity::Standard => "standard",
        PlanComplexity::Complex => "complex",
    }
}

fn truncate_for_preview(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let mut truncated = value
        .chars()
        .take(max_chars.saturating_sub(1))
        .collect::<String>();
    truncated.push_str("...");
    truncated
}

fn scope_model_config_from_cli_config(config: &Config) -> RokoConfig {
    let mut model_config = RokoConfig::default();
    model_config.providers.extend(config.providers.clone());
    model_config.models.extend(config.models.clone());
    model_config.agent.command = Some(config.agent.command.clone());
    model_config.agent.args = Some(config.agent.args.clone());
    model_config.agent.timeout_ms = Some(config.agent.timeout_ms);
    model_config.agent.env = Some(config.agent.env.clone());
    model_config.agent.default_effort = config.agent.effort.clone();
    model_config.agent.bare_mode = config.agent.bare_mode;
    model_config.agent.fallback_model = config.agent.fallback_model.clone();
    model_config.agent.tier_models = config.agent.tier_models.clone();
    if let Some(model) = config.agent.model.clone() {
        model_config.agent.default_model = model;
    }
    model_config
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── complexity_label ───────────────────────────────────────────

    #[test]
    fn complexity_labels_are_stable() {
        assert_eq!(complexity_label(PlanComplexity::Trivial), "trivial");
        assert_eq!(complexity_label(PlanComplexity::Simple), "simple");
        assert_eq!(complexity_label(PlanComplexity::Standard), "standard");
        assert_eq!(complexity_label(PlanComplexity::Complex), "complex");
    }

    // ── workflow_template_for_complexity ────────────────────────────

    #[test]
    fn workflow_templates_follow_the_size() {
        assert_eq!(
            workflow_template_for_complexity(PlanComplexity::Trivial),
            "mechanical"
        );
        assert_eq!(
            workflow_template_for_complexity(PlanComplexity::Simple),
            "focused"
        );
        assert_eq!(
            workflow_template_for_complexity(PlanComplexity::Standard),
            "integrative"
        );
        assert_eq!(
            workflow_template_for_complexity(PlanComplexity::Complex),
            "architectural"
        );
    }

    // ── truncate_for_preview ───────────────────────────────────────

    #[test]
    fn truncate_short_string_unchanged() {
        assert_eq!(truncate_for_preview("hello", 80), "hello");
    }

    #[test]
    fn truncate_exact_boundary() {
        assert_eq!(truncate_for_preview("abcde", 5), "abcde");
    }

    #[test]
    fn truncate_over_limit_adds_ellipsis() {
        assert_eq!(truncate_for_preview("abcdefghij", 5), "abcd...");
    }

    #[test]
    fn truncate_empty_string() {
        assert_eq!(truncate_for_preview("", 10), "");
    }

    #[test]
    fn truncate_unicode() {
        // Unicode chars count by char, not byte.
        let emoji = "\u{1f600}\u{1f600}\u{1f600}\u{1f600}\u{1f600}"; // 5 emoji
        assert_eq!(truncate_for_preview(emoji, 5), emoji);
    }

    // ── resolve_run_route table-driven tests (#278) ──────────────

    fn route(complexity: PlanComplexity, plan: bool, dry: bool) -> RunRoute {
        resolve_run_route(&RunRouteInput {
            complexity,
            plan_flag: plan,
            dry_preview: dry,
        })
    }

    #[test]
    fn trivial_prompt_runs_one_mechanical_task() {
        let r = route(PlanComplexity::Trivial, false, false);
        assert_eq!(r, RunRoute::Mechanical);
        assert_eq!(r.template_name(), "mechanical@1");
    }

    #[test]
    fn simple_prompt_runs_one_focused_task() {
        let r = route(PlanComplexity::Simple, false, false);
        assert_eq!(r, RunRoute::Focused);
        assert_eq!(r.template_name(), "focused@1");
    }

    #[test]
    fn standard_and_complex_prompts_get_a_plan() {
        assert_eq!(
            route(PlanComplexity::Standard, false, false),
            RunRoute::PromptPlan
        );
        // Complex no longer drafts a PRD first: it plans from the prompt.
        assert_eq!(
            route(PlanComplexity::Complex, false, false),
            RunRoute::PromptPlan
        );
    }

    #[test]
    fn plan_flag_plans_whatever_the_size() {
        for complexity in [
            PlanComplexity::Trivial,
            PlanComplexity::Simple,
            PlanComplexity::Standard,
            PlanComplexity::Complex,
        ] {
            assert_eq!(route(complexity, true, false), RunRoute::PromptPlan);
        }
    }

    #[test]
    fn dry_run_wraps_the_route_and_delegates_its_names() {
        let r = route(PlanComplexity::Simple, false, true);
        assert_eq!(
            r,
            RunRoute::DryRun {
                inner: Box::new(RunRoute::Focused)
            }
        );
        assert_eq!(r.template_name(), "focused@1");
        assert_eq!(r.cost_band(), "$0.01-$0.05");
        assert_eq!(r.pipeline_description(), "single agent (focused)");
    }

    #[test]
    fn route_names_are_stable() {
        assert_eq!(RunRoute::Mechanical.cost_band(), "<$0.01");
        assert_eq!(RunRoute::PromptPlan.cost_band(), "$0.05-$0.25");
        assert_eq!(RunRoute::PromptPlan.template_name(), "prompt-plan");
        assert_eq!(
            RunRoute::PromptPlan.pipeline_description(),
            "generate plan -> execute"
        );
        assert_eq!(
            RunRoute::Mechanical.pipeline_description(),
            "single agent (mechanical)"
        );
    }

    // ── plan_argument ───────────────────────────────────────────────

    fn write_plan(dir: &Path) {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join("tasks.toml"), "[meta]\nplan = \"p\"\n").unwrap();
    }

    #[test]
    fn a_plan_directory_runs_as_a_plan() {
        let tmp = tempfile::tempdir().unwrap();
        write_plan(&tmp.path().join("plans").join("add-login"));
        assert_eq!(
            plan_argument(tmp.path(), "plans/add-login"),
            PlanArgument::Plan(tmp.path().join("plans/add-login"))
        );
        // The plans root holds plans one level down.
        assert_eq!(
            plan_argument(tmp.path(), "plans"),
            PlanArgument::Plan(tmp.path().join("plans"))
        );
        // A tasks.toml path names its directory.
        assert_eq!(
            plan_argument(tmp.path(), "plans/add-login/tasks.toml"),
            PlanArgument::Plan(tmp.path().join("plans/add-login"))
        );
    }

    #[test]
    fn a_directory_without_plans_is_refused() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("src")).unwrap();
        assert_eq!(
            plan_argument(tmp.path(), "src"),
            PlanArgument::NoPlans(tmp.path().join("src"))
        );
    }

    #[test]
    fn a_missing_path_is_not_run_as_a_prompt() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(
            plan_argument(tmp.path(), "plans/typo"),
            PlanArgument::Missing(tmp.path().join("plans/typo"))
        );
    }

    #[test]
    fn prompt_text_stays_a_prompt() {
        let tmp = tempfile::tempdir().unwrap();
        write_plan(&tmp.path().join("plans").join("login"));
        assert_eq!(
            plan_argument(tmp.path(), "fix the login bug"),
            PlanArgument::Prompt
        );
        // One word is a prompt unless it is a path; a plan it names is only
        // hinted at (named_plan_dir), never run.
        assert_eq!(plan_argument(tmp.path(), "refactor"), PlanArgument::Prompt);
        assert_eq!(
            named_plan_dir(tmp.path(), "login"),
            Some(tmp.path().join("plans").join("login"))
        );
        assert_eq!(named_plan_dir(tmp.path(), "refactor"), None);
    }

    #[test]
    fn plan_answers() {
        for yes in ["", "\n", "y", "Y\n", "yes", " YES "] {
            assert!(plan_answer_runs(yes), "{yes:?}");
        }
        for no in ["n", "no\n", "later", "q"] {
            assert!(!plan_answer_runs(no), "{no:?}");
        }
    }
}
