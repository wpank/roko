//! `roko do` command — deterministic template routing (#278).
//!
//! Replaces silent engine/workflow delegation with explicit named Graph
//! template selection. The frozen routing table is implemented as the
//! pure [`resolve_do_route`] function and table-tested against every row.
//!
//! ## Routing table (frozen)
//!
//! | Input                          | Route                                    |
//! |-------------------------------|------------------------------------------|
//! | `--complexity trivial`        | `mechanical@1`                           |
//! | `--complexity simple`         | `focused@1`                              |
//! | `--complexity standard`       | prompt -> plan -> execute                |
//! | `--complexity complex`        | PRD -> plan -> execute                   |
//! | `--plan` with trivial/simple  | `integrative@1`                          |
//! | research intent               | `roko research topic`                    |
//! | plan-generate intent          | `roko plan generate`                     |
//! | unqualified TTY prompt        | preview + confirm before dispatch        |
//! | unqualified non-TTY prompt    | reject; require `--complexity`/`--plan`  |
//! | non-TTY `roko run` prompt     | auto-detected route (as on a TTY)        |
//! | dry-run/ghost/compare         | print route; never execute               |
//! | single word matching plan dir | instruct `roko plan run <path>`          |

use crate::*;
use roko_core::config::schema::RokoConfig;
use roko_gate::PlanComplexity;
use std::path::{Path, PathBuf};

// ─── Routing types ──────────────────────────────────────────────────────

/// Resolved execution route for `roko do`.
///
/// Each variant maps to exactly one row in the frozen routing table.
/// The resolver is pure: no I/O, no config loading, no filesystem probing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DoRoute {
    /// `--complexity trivial` -> `mechanical@1` single-agent template.
    Mechanical,
    /// `--complexity simple` -> `focused@1` single-agent template.
    Focused,
    /// `--complexity standard` -> generate plan from prompt, then execute.
    PromptPlan,
    /// `--complexity complex` -> create/validate PRD, generate plan, execute.
    PrdPlan,
    /// `--plan` with trivial/simple classification -> `integrative@1`.
    Integrative,
    /// Research intent auto-detected from prompt.
    Research,
    /// Plan-generation intent auto-detected from prompt.
    PlanGenerate,
    /// Dry-run / ghost / compare: print route without executing.
    DryRun {
        /// The inner route that *would* execute.
        inner: Box<DoRoute>,
    },
    /// Non-TTY prompt without explicit complexity/plan: reject before side effects.
    RejectNonTty,
    /// Single word matching a known plan directory: instruct `roko plan run`.
    PlanHint {
        /// The slug the user typed.
        slug: String,
    },
}

impl DoRoute {
    /// Canonical template name for display in previews.
    #[must_use]
    pub fn template_name(&self) -> &'static str {
        match self {
            Self::Mechanical => "mechanical@1",
            Self::Focused => "focused@1",
            Self::PromptPlan => "prompt-plan",
            Self::PrdPlan => "prd-plan",
            Self::Integrative => "integrative@1",
            Self::Research => "research",
            Self::PlanGenerate => "plan-generate",
            Self::DryRun { inner } => inner.template_name(),
            Self::RejectNonTty => "reject",
            Self::PlanHint { .. } => "plan-hint",
        }
    }

    /// Estimated cost band for display in previews.
    #[must_use]
    pub fn cost_band(&self) -> &'static str {
        match self {
            Self::Mechanical => "<$0.01",
            Self::Focused => "$0.01-$0.05",
            Self::PromptPlan => "$0.05-$0.25",
            Self::PrdPlan => "$0.25+",
            Self::Integrative => "$0.05-$0.25",
            Self::Research => "$0.01-$0.10",
            Self::PlanGenerate => "$0.05-$0.25",
            Self::DryRun { inner } => inner.cost_band(),
            Self::RejectNonTty => "$0.00",
            Self::PlanHint { .. } => "$0.00",
        }
    }

    /// Human-readable pipeline description.
    #[must_use]
    pub fn pipeline_description(&self) -> &'static str {
        match self {
            Self::Mechanical => "single agent (mechanical)",
            Self::Focused => "single agent (focused)",
            Self::PromptPlan => "generate plan -> execute",
            Self::PrdPlan => "PRD -> draft -> plan -> execute",
            Self::Integrative => "single agent (integrative, plan-forced)",
            Self::Research => "research command",
            Self::PlanGenerate => "plan generation command",
            Self::DryRun { inner } => inner.pipeline_description(),
            Self::RejectNonTty => "rejected (non-TTY without explicit complexity)",
            Self::PlanHint { .. } => "plan hint (use roko plan run)",
        }
    }
}

/// Inputs to the pure routing resolver.
#[derive(Debug, Clone)]
pub struct DoRouteInput {
    /// Classified or overridden complexity.
    pub complexity: PlanComplexity,
    /// Whether `--complexity` was explicitly provided.
    pub complexity_forced: bool,
    /// Whether `--plan` was passed.
    pub plan_flag: bool,
    /// Whether `--dry-run` or `--ghost` was passed.
    pub dry_preview: bool,
    /// Whether `--compare` was passed.
    pub compare: bool,
    /// Whether stdin is a TTY.
    pub is_tty: bool,
    /// Whether the prompt came from `roko run`, an explicit single-prompt
    /// invocation that scripts and CI use without a TTY.
    pub explicit_run: bool,
}

/// Pure routing resolver: maps [`DoRouteInput`] to a [`DoRoute`].
///
/// This function performs no I/O, no config loading, and no filesystem
/// access. Intent classification and single-word plan detection happen
/// *before* this function is called.
#[must_use]
pub fn resolve_do_route(input: &DoRouteInput) -> DoRoute {
    let base = if input.plan_flag {
        match input.complexity {
            PlanComplexity::Trivial | PlanComplexity::Simple => DoRoute::Integrative,
            PlanComplexity::Standard => DoRoute::PromptPlan,
            PlanComplexity::Complex => DoRoute::PrdPlan,
        }
    } else {
        match input.complexity {
            PlanComplexity::Trivial => DoRoute::Mechanical,
            PlanComplexity::Simple => DoRoute::Focused,
            PlanComplexity::Standard => DoRoute::PromptPlan,
            PlanComplexity::Complex => DoRoute::PrdPlan,
        }
    };

    // Dry-run / ghost / compare: wrap the base route.
    if input.dry_preview || input.compare {
        return DoRoute::DryRun {
            inner: Box::new(base),
        };
    }

    // Non-TTY without explicit complexity: reject before side effects.
    // `roko run` is itself the explicit request, so it keeps the auto-detected
    // route a TTY caller would get.
    if !input.is_tty && !input.complexity_forced && !input.plan_flag && !input.explicit_run {
        return DoRoute::RejectNonTty;
    }

    base
}

/// Main entry point for `roko do`.
pub(crate) async fn cmd_do(
    cli: &Cli,
    workdir: Option<PathBuf>,
    prompt_args: Vec<String>,
    plan: bool,
    complexity_override: Option<PlanComplexity>,
    dry_run: bool,
    yes: bool,
    ghost: bool,
    compare: bool,
    continue_work: Option<Option<String>>,
    no_cascade: bool,
    provider: Option<String>,
    context: Vec<PathBuf>,
    explicit_run: bool,
) -> Result<i32> {
    let workdir = workdir.unwrap_or_else(|| resolve_workdir(cli));
    let prompt = prompt_args.join(" ").trim().to_string();

    if let Some(work_id) = continue_work {
        let _lock = roko_cli::workspace_lock::acquire_workspace_lock(&workdir.join(".roko"))?;
        return cmd_do_continue(&workdir, work_id).await;
    }

    if prompt.is_empty() {
        return cmd_do_resume_hint(&workdir);
    }

    // If the prompt is a single word matching an existing plan directory,
    // instruct the user to use `roko plan run` instead of silently executing.
    // This prevents filesystem-sensitive routing (#278 frozen routing table).
    if !prompt.contains(' ') {
        let plan_slug = &prompt;
        let candidates = [
            workdir
                .join(".roko")
                .join("plans")
                .join(plan_slug)
                .join("tasks.toml"),
            workdir.join("plans").join(plan_slug).join("tasks.toml"),
        ];
        if let Some(found) = candidates.iter().find(|p| p.is_file()) {
            let plan_dir = found.parent().expect("tasks.toml has parent dir");
            let out = roko_cli::cli_output::CliOutput::new(cli.quiet);
            out.step(
                "Plan found",
                &format!("\"{}\" matches plan at {}", plan_slug, plan_dir.display()),
            );
            out.step("Run with", &format!("roko plan run {}", plan_dir.display()));
            return Ok(EXIT_SUCCESS);
        }
    }

    // ── Intent classification: route research and plan-generate intents ──
    // This implements progressive formality: `roko do "how does auth work?"`
    // routes to research, while `roko do "create a plan for auth"` routes to
    // plan generation. Only explicit --complexity or --plan overrides skip this.
    if complexity_override.is_none() && !plan {
        use roko_cli::scope_resolver::{PromptIntent, ScopeResolver};
        let intent = ScopeResolver::classify_intent(&prompt);
        match intent {
            PromptIntent::Research => {
                let out = roko_cli::cli_output::CliOutput::new(cli.quiet);
                out.step("Intent", "research (auto-detected from prompt)");
                out.step("Routing", "roko research topic ...");
                let topic_words: Vec<String> =
                    prompt.split_whitespace().map(String::from).collect();
                // Respect configured auto_deep preference for auto-routed research.
                let auto_deep = roko_core::config::loader::load_config_unified(&workdir)
                    .map(|c| c.perplexity.auto_deep)
                    .unwrap_or(false);
                let research_cmd = crate::ResearchCmd::Topic {
                    topic: topic_words,
                    deep: auto_deep,
                    backend: crate::ResearchBackend::Auto,
                };
                let research_input = roko_cli::resolved_overrides::ResearchInput {
                    backend: Some("auto".to_string()),
                    deep: auto_deep,
                };
                let resolved =
                    roko_cli::resolved_overrides::ResolvedExecutionOverrides::for_research(
                        &crate::global_cli_flags(cli),
                        &research_input,
                    );
                return crate::commands::research::cmd_research(cli, research_cmd, &resolved).await;
            }
            PromptIntent::PlanGenerate => {
                let out = roko_cli::cli_output::CliOutput::new(cli.quiet);
                out.step("Intent", "plan generation (auto-detected from prompt)");
                out.step("Routing", "roko plan generate ...");
                let plan_cmd = crate::PlanCmd::Generate {
                    source: prompt.split_whitespace().map(String::from).collect(),
                    from_file: None,
                    context: context.clone(),
                    from_notes: false,
                    tag: None,
                    from_backlog: None,
                };
                return crate::commands::plan::cmd_plan(cli, plan_cmd).await;
            }
            PromptIntent::Task => {
                // Fall through to complexity classification below
            }
        }
    }

    let preview_config = load_resolved_config(&workdir)
        .map(|resolved| resolved.config)
        .unwrap_or_default();
    let scope_config = scope_model_config_from_cli_config(&preview_config);
    let classified_complexity = match complexity_override {
        Some(complexity) => complexity,
        None => roko_cli::scope_resolver::ScopeResolver::resolve(&prompt, &scope_config).await,
    };
    let complexity = if plan {
        promote_to_planned_complexity(classified_complexity)
    } else {
        classified_complexity
    };
    let forced = complexity_override.is_some();
    let dry_preview = dry_run || ghost;

    // ── Deterministic route resolution (#278) ───────────────────────
    let route_input = DoRouteInput {
        complexity,
        complexity_forced: forced,
        plan_flag: plan,
        dry_preview,
        compare,
        is_tty: roko_cli::stdin_is_tty(),
        explicit_run,
    };
    let route = resolve_do_route(&route_input);

    tracing::info!(
        ?route,
        template = route.template_name(),
        cost_band = route.cost_band(),
        pipeline = route.pipeline_description(),
        complexity_forced = forced,
        "resolved do route (#278 deterministic template routing)"
    );

    // #278 + #343: emit structured trace when complexity was auto-detected
    // rather than user-specified. This makes silent routing observable.
    if !forced {
        tracing::info!(
            complexity = ?complexity,
            template = route.template_name(),
            "auto-detected complexity -> template (no --complexity override)"
        );
    }

    match route {
        DoRoute::DryRun { ref inner } => {
            print_do_preview(
                &prompt,
                complexity,
                forced,
                yes,
                no_cascade,
                &preview_config,
                inner,
            );
            if compare {
                println!("compare     : cascade-enabled vs --no-cascade");
                println!("execution   : skipped; compare mode is a dry preview in this worktree");
            }
            return Ok(EXIT_SUCCESS);
        }
        DoRoute::RejectNonTty => {
            roko_cli::output_format::error(
                "non-TTY input requires --complexity <level> or --plan to avoid silent dispatch",
            );
            roko_cli::output_format::step(
                "Hint",
                "roko do --complexity simple \"your prompt\" or roko do --plan \"your prompt\"",
            );
            return Ok(EXIT_AGENT_FAILURE);
        }
        DoRoute::PlanHint { ref slug } => {
            // This path is unreachable in normal flow (handled earlier), but
            // included for completeness.
            roko_cli::output_format::step(
                "Plan found",
                &format!("use `roko plan run <path>` to execute plan \"{slug}\""),
            );
            return Ok(EXIT_SUCCESS);
        }
        DoRoute::Research | DoRoute::PlanGenerate => {
            // These are handled before complexity classification.
            unreachable!("research/plan-generate routes are resolved before resolve_do_route");
        }
        _ => {}
    }

    // A `do` execution owns the runner slot for its complete lifetime
    // (including plan generation and runner dispatch).  Use the runner lock
    // so that read-only commands with shared workspace locks can coexist.
    let _lock = roko_cli::workspace_lock::acquire_runner_lock(&workdir.join(".roko"))?;

    // Route based on resolved route.
    match route {
        DoRoute::Mechanical | DoRoute::Focused => {
            run_simple_path(cli, &workdir, &prompt, complexity, no_cascade, provider).await
        }
        DoRoute::Integrative => {
            // --plan with trivial/simple: run through integrative template.
            run_simple_path(
                cli,
                &workdir,
                &prompt,
                PlanComplexity::Standard,
                no_cascade,
                provider,
            )
            .await
        }
        DoRoute::PromptPlan => {
            run_standard_path(cli, &workdir, &prompt, no_cascade, provider, &context).await
        }
        DoRoute::PrdPlan => {
            run_complex_path(cli, &workdir, &prompt, no_cascade, provider, &context).await
        }
        // Already handled above.
        DoRoute::DryRun { .. }
        | DoRoute::RejectNonTty
        | DoRoute::PlanHint { .. }
        | DoRoute::Research
        | DoRoute::PlanGenerate => unreachable!(),
    }
}

// ─── Simple path: one-task plan through the Graph engine ────────────

async fn run_simple_path(
    cli: &Cli,
    workdir: &Path,
    prompt: &str,
    complexity: PlanComplexity,
    no_cascade: bool,
    provider: Option<String>,
) -> Result<i32> {
    let tier = workflow_template_for_complexity(complexity);

    // `--json` keeps stdout to the one report document.
    let out = roko_cli::cli_output::CliOutput::new(cli.quiet || cli.json);
    out.step(
        "Complexity",
        &format!("{} (auto-detected)", complexity_label(complexity)),
    );
    out.step("Running", "single agent...");

    prepare_runtime_hooks(workdir, cli.quiet);

    let overrides = roko_cli::run::CliOverrides {
        model: cli.model.clone(),
        role: cli.role.clone(),
        provider,
        cascade_enabled: Some(!no_cascade),
        effort: cli.effort.map(|e| e.to_string()),
    };

    tracing::debug!(
        complexity = complexity_label(complexity),
        tier,
        cascade_enabled = !no_cascade,
        engine = "graph",
        "dispatching roko do (simple) as a one-task Graph plan"
    );

    let result = roko_cli::run::run_prompt(roko_cli::run::PromptRun {
        prompt,
        workdir,
        tier,
        overrides: &overrides,
        max_retries: None,
        quiet: cli.quiet || cli.json,
        state_hub: None,
    })
    .await;

    handle_workflow_result(cli, prompt, tier, result)
}

// ─── Standard path: generate plan from prompt, then execute ─────────

async fn run_standard_path(
    cli: &Cli,
    workdir: &Path,
    prompt: &str,
    no_cascade: bool,
    provider: Option<String>,
    context: &[PathBuf],
) -> Result<i32> {
    let out = roko_cli::cli_output::CliOutput::new(cli.quiet);
    out.step(
        "Complexity",
        "standard (auto-detected, override with --complexity simple)",
    );
    out.step("Step 1/2", "Generating plan...");

    prepare_runtime_hooks(workdir, cli.quiet);

    let model_key = roko_cli::model_selection::resolve_planner_model(
        workdir,
        cli.model.clone(),
        "roko do (standard)",
    )?;

    // Pre-flight: check provider.
    {
        let do_config: RokoConfig = std::fs::read_to_string(workdir.join("roko.toml"))
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
        crate::commands::util::preflight_provider_for_model(&do_config, &model_key)?;
    }

    // Generate the plan from the prompt.
    let context_block = if context.is_empty() {
        String::new()
    } else {
        let loaded = roko_cli::context_loader::load_context_files(
            context,
            roko_cli::context_loader::DEFAULT_BUDGET,
            workdir,
        );
        if !loaded.is_empty() {
            format!("<context>\n{loaded}</context>")
        } else {
            String::new()
        }
    };
    let Some(plans_dir) =
        generate_prompt_plan(cli, workdir, prompt, &model_key, &context_block, &out).await
    else {
        return Ok(EXIT_AGENT_FAILURE);
    };

    let plans = match roko_cli::runner::plan_loader::load_plans(&plans_dir) {
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
    out.step(
        "Step 2/2",
        &format!("Executing plan ({total_tasks} tasks)..."),
    );

    // Execute the plans through the Graph engine.
    run_plan_execution(cli, workdir, &plans_dir, no_cascade, provider).await
}

// ─── Complex path: PRD -> draft -> plan -> execute ──────────────────

async fn run_complex_path(
    cli: &Cli,
    workdir: &Path,
    prompt: &str,
    no_cascade: bool,
    provider: Option<String>,
    _context: &[PathBuf],
) -> Result<i32> {
    use roko_cli::agent_config::{command_from_config, load_gateway_env};
    use roko_cli::agent_exec::{AgentExecOpts, run_agent_capture_silent_recorded};
    use roko_cli::plan_authoring::AuthoringSpend;

    let out = roko_cli::cli_output::CliOutput::new(cli.quiet);
    out.step(
        "Complexity",
        "complex (auto-detected, override with --complexity simple)",
    );

    prepare_runtime_hooks(workdir, cli.quiet);

    let gw = load_gateway_env(workdir);
    let effort = cli.effort.map(|e| e.to_string());
    let effort_ref = effort.as_deref();
    let resume_session = cli.resume.as_deref();
    let agent_command = command_from_config(workdir).unwrap_or_else(|| "claude".to_string());

    // ── Step 1: Create PRD idea ──────────────────────────────────────
    out.step("Step 1/4", "Creating PRD...");
    roko_cli::prd::ensure_dirs(workdir)?;
    roko_cli::prd::cmd_idea(workdir, prompt, false)?;

    // ── Step 2: Draft the PRD ────────────────────────────────────────
    out.step("Step 2/4", "Drafting PRD...");
    let slug = roko_cli::prd::slugify(prompt);
    let drafts = roko_cli::workspace_paths::drafts_dir(workdir);
    let draft_path = drafts.join(format!("{slug}.md"));

    let model_key = roko_cli::model_selection::resolve_effective_model_key(
        workdir,
        cli.model.clone(),
        Some("scribe"),
        "roko do (complex) prd draft",
    )?;

    // Pre-flight: check provider.
    {
        let do_config: RokoConfig = std::fs::read_to_string(workdir.join("roko.toml"))
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
        crate::commands::util::preflight_provider_for_model(&do_config, &model_key)?;
    }
    // Resolve the planner now, so a bad `[authoring] planner_model` fails
    // before the PRD draft is paid for.
    let planner_model = roko_cli::model_selection::resolve_planner_model(
        workdir,
        cli.model.clone(),
        "roko do (complex) plan",
    )?;

    let title = prompt;
    let frontmatter = roko_cli::prd::new_draft_frontmatter(&slug, title);
    let scaffold = format!(
        "{frontmatter}# {title}\n\n\
         ## Overview\n\n## Requirements\n\n## Acceptance criteria\n\n\
         ## Design\n\n## References\n"
    );
    std::fs::write(&draft_path, &scaffold)?;

    let system = roko_cli::prd::prd_agent_prompt(
        workdir,
        &format!(
            "Fill in the draft PRD at {path}. \
             Output the complete PRD markdown (with YAML frontmatter) as your response. \
             Do NOT use file tools \u{2014} they are not available. \
             Do NOT wrap in code fences. \
             Follow the PRD quality standards in your system prompt exactly.",
            path = draft_path.display()
        ),
    );
    let task_prompt = format!(
        "Generate a complete PRD for: {title}. \
         Output the complete PRD markdown with YAML frontmatter. \
         Include specific requirements, machine-verifiable acceptance criteria, \
         and a design section."
    );

    let content_before = std::fs::read(&draft_path).ok();
    let started = Instant::now();
    let task_id = format!("do:prd:draft:{slug}");
    let spend = AuthoringSpend::operation(workdir, &task_id, "scribe");
    let (exit_code, output) = run_agent_capture_silent_recorded(
        AgentExecOpts {
            prompt: &task_prompt,
            workdir,
            model: Some(model_key.as_str()),
            effort: effort_ref,
            system_prompt: Some(&system),
            resume_session,
            env_vars: &gw.vars,
            role: Some("scribe"),
            allowed_tools: Some("none"),
        },
        &spend,
    )
    .await?;

    // Handle agent output: check if it wrote the file or returned text.
    let content_after = std::fs::read(&draft_path).ok();
    let file_was_modified = match (&content_before, &content_after) {
        (Some(before), Some(after)) => before != after,
        (None, Some(_)) => true,
        _ => false,
    };

    let mut draft_written = false;
    if file_was_modified {
        let content = std::fs::read_to_string(&draft_path).unwrap_or_default();
        if roko_cli::prd::has_substantive_markdown_content(&content) {
            draft_written = true;
        }
    } else if !output.trim().is_empty() {
        let content = roko_cli::prd::materialize_agent_markdown_output(&output, Some(&scaffold))
            .unwrap_or_else(|| scaffold.clone());
        if roko_cli::prd::has_substantive_markdown_content(&content) {
            std::fs::write(&draft_path, content)?;
            draft_written = true;
        }
    }

    let _ = crate::commands::util::persist_capture_episode(
        workdir,
        &agent_command,
        Some(model_key.as_str()),
        "do-prd-draft",
        &task_id,
        &task_prompt,
        &output,
        draft_written,
        started.elapsed().as_millis() as u64,
        resume_session,
    )
    .await;

    if !draft_written {
        out.warning(&format!(
            "PRD draft generation failed (exit {exit_code}); falling back to plan-from-prompt path"
        ));
        // Fall back to standard path without the PRD step.
        return run_standard_path_inner(cli, workdir, prompt, no_cascade, provider).await;
    }

    // ── Step 3: Generate plan from the PRD ───────────────────────────
    out.step("Step 3/4", "Generating plan...");
    let plans_root = match roko_cli::prd::generate_plan_from_prd_with_model(
        &slug,
        &draft_path,
        false,
        Some(planner_model.as_str()),
    )
    .await
    {
        Ok(root) => root,
        Err(err) => {
            out.error(&format!("Plan generation from PRD failed: {err:#}"));
            out.warning("Falling back to plan-from-prompt path");
            return run_standard_path_inner(cli, workdir, prompt, no_cascade, provider).await;
        }
    };

    let plans = match roko_cli::runner::plan_loader::load_plans(&plans_root) {
        Ok(plans) if plans.is_empty() => {
            out.error("Plan generation from PRD produced no executable plans");
            return Ok(EXIT_AGENT_FAILURE);
        }
        Ok(plans) => plans,
        Err(err) => {
            out.error(&format!("Failed to load generated plans: {err:#}"));
            return Ok(EXIT_AGENT_FAILURE);
        }
    };

    let total_tasks: usize = plans.iter().map(|p| p.tasks.tasks.len()).sum();
    out.step(
        "Step 4/4",
        &format!("Executing plan ({total_tasks} tasks)..."),
    );

    // ── Step 4: Execute the plan ─────────────────────────────────────
    run_plan_execution(cli, workdir, &plans_root, no_cascade, provider).await
}

// ─── Shared: execute a plan directory through the Graph engine ──────

pub(crate) async fn run_plan_execution(
    cli: &Cli,
    workdir: &Path,
    plans_dir: &Path,
    no_cascade: bool,
    _provider: Option<String>,
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
        max_retries: None,
        // Each plan's own `max_parallel`, as `roko plan run` defaults to.
        max_tasks: 0,
        budget_override: None,
        no_budget: false,
        cli_model_override: cli.model.clone(),
        dangerously_skip_permissions: false,
        log_file: None,
        worktree_per_task: false,
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
        metrics: None,
    })
    .await
}

/// Inner standard path used as fallback when the complex path's PRD step fails.
async fn run_standard_path_inner(
    cli: &Cli,
    workdir: &Path,
    prompt: &str,
    no_cascade: bool,
    provider: Option<String>,
) -> Result<i32> {
    let model_key = roko_cli::model_selection::resolve_planner_model(
        workdir,
        cli.model.clone(),
        "roko do (fallback plan)",
    )?;
    let out = roko_cli::cli_output::CliOutput::new(cli.quiet);
    let Some(plans_dir) = generate_prompt_plan(cli, workdir, prompt, &model_key, "", &out).await
    else {
        return Ok(EXIT_AGENT_FAILURE);
    };

    run_plan_execution(cli, workdir, &plans_dir, no_cascade, provider).await
}

/// Plan `prompt` with the one plan generator (gap-2623b2), which validates
/// the plan and writes it to the workspace plans directory; `context` follows
/// the prompt. Returns that directory, or `None` after reporting a failure on
/// `out`.
async fn generate_prompt_plan(
    cli: &Cli,
    workdir: &Path,
    prompt: &str,
    model_key: &str,
    context: &str,
    out: &roko_cli::cli_output::CliOutput,
) -> Option<PathBuf> {
    let effort = cli.effort.map(|e| e.to_string());
    let slug = roko_cli::prd::slugify(prompt);
    let request = roko_cli::prd::PlanRequest {
        context: Some(context),
        model: Some(model_key),
        effort: Some(effort.as_deref().unwrap_or("high")),
        ..roko_cli::prd::PlanRequest::new(
            roko_cli::prd::PlanSource::Text {
                text: prompt,
                kind: "prompt",
            },
            &slug,
            workdir,
        )
    };
    match roko_cli::prd::generate_plan(request).await {
        Ok((plans_dir, _)) => Some(plans_dir),
        Err(err) => {
            out.error(&format!("Plan generation failed: {err:#}"));
            None
        }
    }
}

// ─── Continue / resume helpers ──────────────────────────────────────

async fn cmd_do_continue(workdir: &Path, work_id: Option<String>) -> Result<i32> {
    let snapshot = match work_id {
        Some(id) => match roko_core::Workspace::open(workdir) {
            Ok(workspace) => workspace.state_dir().join(format!("{id}.json")),
            Err(_) => workdir
                .join(".roko")
                .join("state")
                .join(format!("{id}.json")),
        },
        None => executor_snapshot_path(workdir),
    };

    if snapshot.exists() {
        roko_cli::output_format::step(
            "Found snapshot",
            &format!(
                "{}; use `roko resume` until first-class work items land",
                snapshot.display()
            ),
        );
        Ok(EXIT_SUCCESS)
    } else {
        roko_cli::output_format::error(&format!(
            "no resumable work found at {}",
            snapshot.display()
        ));
        Ok(EXIT_AGENT_FAILURE)
    }
}

fn cmd_do_resume_hint(workdir: &Path) -> Result<i32> {
    let snapshot = executor_snapshot_path(workdir);
    if snapshot.exists() {
        roko_cli::output_format::step(
            "Interrupted work",
            &format!("found at {}", snapshot.display()),
        );
        roko_cli::output_format::step("Resume with", "roko do --continue");
        Ok(EXIT_AGENT_FAILURE)
    } else {
        roko_cli::output_format::error("no prompt supplied");
        roko_cli::output_format::step("Usage", "roko do \"fix the bug\"");
        Ok(EXIT_AGENT_FAILURE)
    }
}

fn executor_snapshot_path(workdir: &Path) -> PathBuf {
    roko_core::Workspace::open(workdir)
        .map(|workspace| workspace.executor_snapshot_path())
        .unwrap_or_else(|_| workdir.join(".roko").join("state").join("executor.json"))
}

// ─── Preview / formatting helpers ───────────────────────────────────

fn print_do_preview(
    prompt: &str,
    complexity: PlanComplexity,
    forced: bool,
    yes: bool,
    no_cascade: bool,
    config: &Config,
    route: &DoRoute,
) {
    let gate_count = roko_cli::run::workflow_enabled_gate_names(&config.gates).len();

    println!("roko do");
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
    println!("approval    : {}", if yes { "auto" } else { "workflow" });
    println!(
        "cascade     : {}",
        if no_cascade { "disabled" } else { "enabled" }
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

fn promote_to_planned_complexity(complexity: PlanComplexity) -> PlanComplexity {
    match complexity {
        PlanComplexity::Trivial | PlanComplexity::Simple => PlanComplexity::Standard,
        PlanComplexity::Standard | PlanComplexity::Complex => complexity,
    }
}

/// Task tier a single-agent route runs the prompt at.
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
    fn complexity_label_trivial() {
        assert_eq!(complexity_label(PlanComplexity::Trivial), "trivial");
    }

    #[test]
    fn complexity_label_simple() {
        assert_eq!(complexity_label(PlanComplexity::Simple), "simple");
    }

    #[test]
    fn complexity_label_standard() {
        assert_eq!(complexity_label(PlanComplexity::Standard), "standard");
    }

    #[test]
    fn complexity_label_complex() {
        assert_eq!(complexity_label(PlanComplexity::Complex), "complex");
    }

    // ── workflow_template_for_complexity ────────────────────────────

    #[test]
    fn workflow_template_trivial() {
        assert_eq!(
            workflow_template_for_complexity(PlanComplexity::Trivial),
            "mechanical"
        );
    }

    #[test]
    fn workflow_template_simple() {
        assert_eq!(
            workflow_template_for_complexity(PlanComplexity::Simple),
            "focused"
        );
    }

    #[test]
    fn workflow_template_standard() {
        assert_eq!(
            workflow_template_for_complexity(PlanComplexity::Standard),
            "integrative"
        );
    }

    #[test]
    fn workflow_template_complex() {
        assert_eq!(
            workflow_template_for_complexity(PlanComplexity::Complex),
            "architectural"
        );
    }

    // ── promote_to_planned_complexity ──────────────────────────────

    #[test]
    fn promote_trivial_to_standard() {
        assert_eq!(
            promote_to_planned_complexity(PlanComplexity::Trivial),
            PlanComplexity::Standard
        );
    }

    #[test]
    fn promote_simple_to_standard() {
        assert_eq!(
            promote_to_planned_complexity(PlanComplexity::Simple),
            PlanComplexity::Standard
        );
    }

    #[test]
    fn promote_standard_stays_standard() {
        assert_eq!(
            promote_to_planned_complexity(PlanComplexity::Standard),
            PlanComplexity::Standard
        );
    }

    #[test]
    fn promote_complex_stays_complex() {
        assert_eq!(
            promote_to_planned_complexity(PlanComplexity::Complex),
            PlanComplexity::Complex
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
        let result = truncate_for_preview("abcdefghij", 5);
        assert!(result.ends_with("..."));
        // Should be 4 original chars + "..."
        assert_eq!(result, "abcd...");
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

    // ── resolve_do_route table-driven tests (#278) ───────────────

    fn route_input(
        complexity: PlanComplexity,
        forced: bool,
        plan: bool,
        dry: bool,
        compare: bool,
        tty: bool,
    ) -> DoRouteInput {
        DoRouteInput {
            complexity,
            complexity_forced: forced,
            plan_flag: plan,
            dry_preview: dry,
            compare,
            is_tty: tty,
            explicit_run: false,
        }
    }

    // Row 1: explicit --complexity trivial -> mechanical@1
    #[test]
    fn do_route_explicit_trivial() {
        let r = resolve_do_route(&route_input(
            PlanComplexity::Trivial,
            true,
            false,
            false,
            false,
            true,
        ));
        assert_eq!(r, DoRoute::Mechanical);
        assert_eq!(r.template_name(), "mechanical@1");
    }

    // Row 2: explicit --complexity simple -> focused@1
    #[test]
    fn do_route_explicit_simple() {
        let r = resolve_do_route(&route_input(
            PlanComplexity::Simple,
            true,
            false,
            false,
            false,
            true,
        ));
        assert_eq!(r, DoRoute::Focused);
        assert_eq!(r.template_name(), "focused@1");
    }

    // Row 3: explicit --complexity standard -> prompt-plan
    #[test]
    fn do_route_explicit_standard() {
        let r = resolve_do_route(&route_input(
            PlanComplexity::Standard,
            true,
            false,
            false,
            false,
            true,
        ));
        assert_eq!(r, DoRoute::PromptPlan);
        assert_eq!(r.template_name(), "prompt-plan");
    }

    // Row 4: explicit --complexity complex -> prd-plan
    #[test]
    fn do_route_explicit_complex() {
        let r = resolve_do_route(&route_input(
            PlanComplexity::Complex,
            true,
            false,
            false,
            false,
            true,
        ));
        assert_eq!(r, DoRoute::PrdPlan);
        assert_eq!(r.template_name(), "prd-plan");
    }

    // Row 5: --plan with trivial classification -> integrative@1
    #[test]
    fn do_route_plan_flag_with_trivial() {
        let r = resolve_do_route(&route_input(
            PlanComplexity::Trivial,
            false,
            true,
            false,
            false,
            true,
        ));
        assert_eq!(r, DoRoute::Integrative);
        assert_eq!(r.template_name(), "integrative@1");
    }

    // Row 5b: --plan with simple classification -> integrative@1
    #[test]
    fn do_route_plan_flag_with_simple() {
        let r = resolve_do_route(&route_input(
            PlanComplexity::Simple,
            false,
            true,
            false,
            false,
            true,
        ));
        assert_eq!(r, DoRoute::Integrative);
    }

    // Row 5c: --plan with standard classification -> prompt-plan (not integrative)
    #[test]
    fn do_route_plan_flag_with_standard() {
        let r = resolve_do_route(&route_input(
            PlanComplexity::Standard,
            false,
            true,
            false,
            false,
            true,
        ));
        assert_eq!(r, DoRoute::PromptPlan);
    }

    // Row 5d: --plan with complex classification -> prd-plan
    #[test]
    fn do_route_plan_flag_with_complex() {
        let r = resolve_do_route(&route_input(
            PlanComplexity::Complex,
            false,
            true,
            false,
            false,
            true,
        ));
        assert_eq!(r, DoRoute::PrdPlan);
    }

    // Row 8: dry-run wraps the base route
    #[test]
    fn do_route_dry_run_wraps_mechanical() {
        let r = resolve_do_route(&route_input(
            PlanComplexity::Trivial,
            true,
            false,
            true,
            false,
            true,
        ));
        assert_eq!(
            r,
            DoRoute::DryRun {
                inner: Box::new(DoRoute::Mechanical),
            }
        );
        assert_eq!(r.template_name(), "mechanical@1");
        assert_eq!(r.cost_band(), "<$0.01");
    }

    // Row 8b: ghost wraps the base route (same as dry-run)
    #[test]
    fn do_route_ghost_wraps_focused() {
        let r = resolve_do_route(&route_input(
            PlanComplexity::Simple,
            true,
            false,
            true,
            false,
            true,
        ));
        assert_eq!(
            r,
            DoRoute::DryRun {
                inner: Box::new(DoRoute::Focused),
            }
        );
    }

    // Row 9: compare wraps the base route
    #[test]
    fn do_route_compare_wraps_route() {
        let r = resolve_do_route(&route_input(
            PlanComplexity::Standard,
            true,
            false,
            false,
            true,
            true,
        ));
        assert_eq!(
            r,
            DoRoute::DryRun {
                inner: Box::new(DoRoute::PromptPlan),
            }
        );
    }

    // Row 10: non-TTY without forced complexity -> RejectNonTty
    #[test]
    fn do_route_non_tty_unqualified_rejects() {
        let r = resolve_do_route(&route_input(
            PlanComplexity::Simple,
            false,
            false,
            false,
            false,
            false,
        ));
        assert_eq!(r, DoRoute::RejectNonTty);
    }

    // Row 10a: non-TTY `roko run` without forced complexity -> auto-detected route
    #[test]
    fn do_route_non_tty_explicit_run_uses_detected_route() {
        for (complexity, expected) in [
            (PlanComplexity::Trivial, DoRoute::Mechanical),
            (PlanComplexity::Simple, DoRoute::Focused),
            (PlanComplexity::Standard, DoRoute::PromptPlan),
            (PlanComplexity::Complex, DoRoute::PrdPlan),
        ] {
            let tty = resolve_do_route(&route_input(complexity, false, false, false, false, true));
            let r = resolve_do_route(&DoRouteInput {
                explicit_run: true,
                ..route_input(complexity, false, false, false, false, false)
            });
            assert_eq!(r, expected);
            assert_eq!(r, tty, "non-TTY `roko run` must match the TTY route");
        }
    }

    // Row 10b: non-TTY WITH forced complexity -> dispatches normally
    #[test]
    fn do_route_non_tty_forced_complexity_passes() {
        let r = resolve_do_route(&route_input(
            PlanComplexity::Simple,
            true,
            false,
            false,
            false,
            false,
        ));
        assert_eq!(r, DoRoute::Focused);
    }

    // Row 10c: non-TTY WITH --plan flag -> dispatches normally
    #[test]
    fn do_route_non_tty_plan_flag_passes() {
        let r = resolve_do_route(&route_input(
            PlanComplexity::Simple,
            false,
            true,
            false,
            false,
            false,
        ));
        assert_eq!(r, DoRoute::Integrative);
    }

    // Row 10d: non-TTY dry-run does NOT reject (dry-run takes precedence)
    #[test]
    fn do_route_non_tty_dry_run_does_not_reject() {
        let r = resolve_do_route(&route_input(
            PlanComplexity::Simple,
            false,
            false,
            true,
            false,
            false,
        ));
        assert_eq!(
            r,
            DoRoute::DryRun {
                inner: Box::new(DoRoute::Focused),
            }
        );
    }

    // ── DoRoute method coverage ──────────────────────────────────

    #[test]
    fn do_route_template_names_are_stable() {
        assert_eq!(DoRoute::Mechanical.template_name(), "mechanical@1");
        assert_eq!(DoRoute::Focused.template_name(), "focused@1");
        assert_eq!(DoRoute::PromptPlan.template_name(), "prompt-plan");
        assert_eq!(DoRoute::PrdPlan.template_name(), "prd-plan");
        assert_eq!(DoRoute::Integrative.template_name(), "integrative@1");
        assert_eq!(DoRoute::Research.template_name(), "research");
        assert_eq!(DoRoute::PlanGenerate.template_name(), "plan-generate");
        assert_eq!(DoRoute::RejectNonTty.template_name(), "reject");
        assert_eq!(
            DoRoute::PlanHint {
                slug: "my-plan".into()
            }
            .template_name(),
            "plan-hint"
        );
    }

    #[test]
    fn do_route_cost_bands_are_stable() {
        assert_eq!(DoRoute::Mechanical.cost_band(), "<$0.01");
        assert_eq!(DoRoute::Focused.cost_band(), "$0.01-$0.05");
        assert_eq!(DoRoute::PromptPlan.cost_band(), "$0.05-$0.25");
        assert_eq!(DoRoute::PrdPlan.cost_band(), "$0.25+");
        assert_eq!(DoRoute::Integrative.cost_band(), "$0.05-$0.25");
        assert_eq!(DoRoute::RejectNonTty.cost_band(), "$0.00");
    }

    #[test]
    fn do_route_pipeline_descriptions_are_stable() {
        assert_eq!(
            DoRoute::Mechanical.pipeline_description(),
            "single agent (mechanical)"
        );
        assert_eq!(
            DoRoute::Focused.pipeline_description(),
            "single agent (focused)"
        );
        assert_eq!(
            DoRoute::PromptPlan.pipeline_description(),
            "generate plan -> execute"
        );
        assert_eq!(
            DoRoute::PrdPlan.pipeline_description(),
            "PRD -> draft -> plan -> execute"
        );
        assert_eq!(
            DoRoute::Integrative.pipeline_description(),
            "single agent (integrative, plan-forced)"
        );
    }

    #[test]
    fn do_route_dry_run_delegates_to_inner() {
        let inner = DoRoute::PrdPlan;
        let dry = DoRoute::DryRun {
            inner: Box::new(inner.clone()),
        };
        assert_eq!(dry.template_name(), inner.template_name());
        assert_eq!(dry.cost_band(), inner.cost_band());
        assert_eq!(dry.pipeline_description(), inner.pipeline_description());
    }

    // ── Equivalent alias coverage ────────────────────────────────

    #[test]
    fn do_route_forced_trivial_and_auto_trivial_same_template() {
        let forced = resolve_do_route(&route_input(
            PlanComplexity::Trivial,
            true,
            false,
            false,
            false,
            true,
        ));
        let auto = resolve_do_route(&route_input(
            PlanComplexity::Trivial,
            false,
            false,
            false,
            false,
            true,
        ));
        // Both should resolve to Mechanical when TTY
        assert_eq!(forced.template_name(), auto.template_name());
    }
}
