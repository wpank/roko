//! plan command handlers.

use std::io::IsTerminal as _;

use crate::*;
use anyhow::Context as _;
use roko_cli::plan_validate;
use roko_fs::RokoLayout;

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
            let exit =
                cmd_plan_validate(&plans_dir, &workdir, strict, json || cli.json, spec_quality)?;

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
            let prepared = roko_cli::plan_brief::prepare(&plan_dir, &workdir, force)?;
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
            approval,
            no_tui,
            max_retries,
            max_tasks,
            dry_run,
            fresh,
            force_resume,
            budget_override,
            no_budget,
            force,
            dangerously_skip_permissions,
            log_file,
            skip_preflight,
            screenshots,
            screenshot_interval,
            screenshot_dir,
            batch_size,
            worktree_per_task,
            rich_topology,
            promote,
            max_parallel_plans,
            fail_fast,
        } => {
            let _t_setup = std::time::Instant::now();

            // The global `--model` flag (with `--force-model` and
            // `--force-backend` as aliases) is the single model override.
            // It populates `RunConfig.cli_model_override`, which the event
            // loop maps to `DispatchContext.force_backend`.
            let effective_model_override = cli.model.clone();

            // Auto-enable inline TUI when stdout is an interactive terminal,
            // unless the user explicitly opted out with --no-tui (item 108).
            let approval =
                approval || (!no_tui && !cli.quiet && !cli.json && std::io::stdout().is_terminal());

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
            if let Some(exit_code) = validate_before_run(&resolved_plans_dir, &wd) {
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

            validate_graph_execution_options(engine, approval)?;

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
                    worktree_per_task,
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
                    effective_model_override.clone(),
                    dangerously_skip_permissions,
                    log_file.as_deref(),
                    worktree_per_task,
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
            use roko_cli::agent_config::load_gateway_env;
            use roko_cli::agent_exec::{
                AgentExecEpisode, AgentExecOpts, run_agent_logged_with_spend,
            };
            use roko_cli::plan_authoring::AuthoringSpend;

            let workdir = std::env::current_dir().context("resolve cwd")?;
            // Plan generation is read-only on workspace state: it reads source
            // code and writes one plan to the workspace plans directory
            // (per-slug, non-overlapping).
            // No workspace lock needed (#226) — allows generating plans while
            // other plans are running.
            let gw = load_gateway_env(&workdir);

            // --from-backlog: resolve backlog specs by numeric ID and generate
            // plans with deterministic slugs written to plans/ (#227).
            if let Some(ref backlog_ids_str) = from_backlog {
                use roko_cli::plan_generate::{
                    DEFAULT_BACKLOG_DIR, build_backlog_generation_prompt,
                    build_backlog_task_prompt, parse_backlog_ids, resolve_backlog_spec,
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

                    let system = build_backlog_generation_prompt(&workdir, &spec, &slug);
                    let task_prompt = build_backlog_task_prompt(&spec, &slug);
                    let task_id = format!("plan:generate:backlog:{id}");
                    // The call's spend is recorded against the plan, as every
                    // other generate path records it (bug-ac5432).
                    let spend = AuthoringSpend::generation(&workdir, &slug, None);

                    let exit_code = run_agent_logged_with_spend(
                        AgentExecOpts {
                            prompt: &task_prompt,
                            workdir: &workdir,
                            model: Some(model_key.as_str()),
                            effort: Some("high"),
                            system_prompt: Some(&system),
                            resume_session: None,
                            env_vars: &gw.vars,
                            role: Some("strategist"),
                            allowed_tools: None,
                        },
                        AgentExecEpisode {
                            task_kind: "plan-generate",
                            task_id: &task_id,
                        },
                        &spend,
                    )
                    .await;

                    match exit_code {
                        Ok(code) if code == EXIT_SUCCESS => {
                            // Validate the generated tasks.toml.
                            let tasks_path = plan_dir.join("tasks.toml");
                            if tasks_path.is_file() {
                                match roko_cli::task_parser::TasksFile::parse(&tasks_path) {
                                    Ok(tf) => {
                                        tracing::info!(
                                            id,
                                            %slug,
                                            task_count = tf.tasks.len(),
                                            "plan generated"
                                        );
                                        results.push((*id, slug, "generated"));
                                    }
                                    Err(err) => {
                                        tracing::warn!(
                                            id,
                                            %slug,
                                            error = %err,
                                            "plan generated but validation failed"
                                        );
                                        results.push((*id, slug, "validation-failed"));
                                    }
                                }
                            } else {
                                tracing::warn!(
                                    id,
                                    %slug,
                                    "agent succeeded but no tasks.toml written"
                                );
                                results.push((*id, slug, "no-output"));
                            }
                        }
                        Ok(code) => {
                            tracing::error!(id, %slug, exit_code = code, "agent exited with non-zero code");
                            results.push((*id, slug, "failed"));
                        }
                        Err(err) => {
                            tracing::error!(id, %slug, error = %err, "agent failed");
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

                    let request = roko_cli::prd::PlanRequest {
                        model: Some(model_key.as_str()),
                        effort: Some("high"),
                        ..roko_cli::prd::PlanRequest::new(
                            roko_cli::prd::PlanSource::Text {
                                text: &combined,
                                kind: "notes",
                            },
                            &slug,
                            &workdir,
                        )
                    };
                    match roko_cli::prd::generate_plan(request).await {
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
                    || roko_cli::prd::slugify(&source_text),
                    roko_cli::prd::slugify,
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
            let request = roko_cli::prd::PlanRequest {
                context: Some(context_block.as_str()),
                model: Some(model_key.as_str()),
                effort: Some("high"),
                ..roko_cli::prd::PlanRequest::new(
                    roko_cli::prd::PlanSource::Text {
                        text: &source_text,
                        kind: source_type,
                    },
                    &slug,
                    &workdir,
                )
            };
            let (_, outcome) = roko_cli::prd::generate_plan(request).await?;
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
            let source_path = find_plan_source_document(&plan_dir)?;
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
            let slug = roko_cli::prd::plan_dir_slug(&plan_dir);
            let request = roko_cli::prd::PlanRequest {
                context: Some(pre_validation_context.as_str()),
                model: Some(model_key.as_str()),
                effort: Some("high"),
                ..roko_cli::prd::PlanRequest::new(
                    roko_cli::prd::PlanSource::Regenerate(&plan_dir),
                    &slug,
                    &workdir,
                )
            };
            let (_, outcome) = roko_cli::prd::generate_plan(request).await?;
            if outcome.artifact_valid {
                Ok(EXIT_SUCCESS)
            } else {
                tracing::error!(
                    "plan regenerate: the regenerated plan failed validation (see warnings above)"
                );
                Ok(1)
            }
        }
        PlanCmd::Queue { cmd } => cmd_plan_queue(cli, cmd).await,

        // ── Plan control commands (#146) ────────────────────────────
        PlanCmd::Pause { workdir } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            let state_dir = wd.join(".roko").join("state");
            let cmd = roko_cli::runner::types::ControlCommand {
                command: roko_cli::runner::types::ControlAction::Pause,
                plan_id: None,
                task_id: None,
            };
            cmd.write(&state_dir)
                .map_err(|e| anyhow!("failed to write control command: {e}"))?;
            if !cli.quiet {
                tracing::info!(path = %state_dir.join("control.json").display(), "pause signal written");
            }
            Ok(EXIT_SUCCESS)
        }
        PlanCmd::Resume { workdir } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            let state_dir = wd.join(".roko").join("state");
            let cmd = roko_cli::runner::types::ControlCommand {
                command: roko_cli::runner::types::ControlAction::Resume,
                plan_id: None,
                task_id: None,
            };
            cmd.write(&state_dir)
                .map_err(|e| anyhow!("failed to write control command: {e}"))?;
            if !cli.quiet {
                tracing::info!(path = %state_dir.join("control.json").display(), "resume signal written");
            }
            Ok(EXIT_SUCCESS)
        }
        PlanCmd::Cancel { plan_id, workdir } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            let state_dir = wd.join(".roko").join("state");
            let cmd = roko_cli::runner::types::ControlCommand {
                command: roko_cli::runner::types::ControlAction::Cancel,
                plan_id,
                task_id: None,
            };
            cmd.write(&state_dir)
                .map_err(|e| anyhow!("failed to write control command: {e}"))?;
            if !cli.quiet {
                tracing::info!(path = %state_dir.join("control.json").display(), "cancel signal written");
            }
            Ok(EXIT_SUCCESS)
        }
        PlanCmd::Retry {
            task_id,
            plan_id,
            workdir,
        } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            let state_dir = wd.join(".roko").join("state");
            let cmd = roko_cli::runner::types::ControlCommand {
                command: roko_cli::runner::types::ControlAction::Retry,
                plan_id,
                task_id,
            };
            cmd.write(&state_dir)
                .map_err(|e| anyhow!("failed to write control command: {e}"))?;
            if !cli.quiet {
                tracing::info!(path = %state_dir.join("control.json").display(), "retry signal written");
            }
            Ok(EXIT_SUCCESS)
        }
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
            let attempt_key = record_held_review(&wd, &plan_id, &task_id, decision, &note)?;
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

/// Show task-level status for a specific plan directory.
///
/// Reads `tasks.toml` in the plan directory and, when executor state is
/// available, overlays runtime completion counts from the snapshot.
/// Record `decision` (`approved` or `rejected`) with `note` on the attempt
/// of `task_id` that `plan_id`'s run holds for review (gap-0d64d5), in the
/// review log the run reads; `roko serve`'s review route writes the same
/// entry. Returns the attempt's key.
fn record_held_review(
    workdir: &std::path::Path,
    plan_id: &str,
    task_id: &str,
    decision: &str,
    note: &str,
) -> Result<String> {
    use std::io::Write as _;

    let layout = roko_fs::RokoLayout::for_project(workdir);
    let hold_path = layout.review_hold(plan_id, task_id);
    let hold: serde_json::Value = std::fs::read(&hold_path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .ok_or_else(|| {
            anyhow!("task `{task_id}` of plan `{plan_id}` is not waiting for a review")
        })?;
    let attempt_key = hold["attempt_key"]
        .as_str()
        .ok_or_else(|| anyhow!("the review hold {} names no attempt", hold_path.display()))?
        .to_string();
    let entry = serde_json::json!({
        "plan_id": plan_id,
        "task_id": task_id,
        "decision": decision,
        "comment": note,
        "attempt_key": attempt_key,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });
    let log = layout.reviews_log();
    std::fs::create_dir_all(layout.state_dir())?;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log)?;
    file.write_all(format!("{entry}\n").as_bytes())?;
    Ok(attempt_key)
}

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
        force: false,
        dangerously_skip_permissions: false,
        log_file: None,
        skip_preflight: false,
        screenshots: false,
        screenshot_interval: 60,
        screenshot_dir: None,
        batch_size: None,
        worktree_per_task: false,
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

/// Run plan validation before `plan run` starts any agents.
///
/// Returns `Some(exit_code)` when validation fails, or `None` when the plan
/// set is valid enough to continue.
fn validate_before_run(plans_dir: &Path, workdir: &Path) -> Option<i32> {
    // If the plans directory doesn't exist yet (e.g. before `prd plan` runs),
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
        None
    } else {
        tracing::error!(report = %plan_validate::render_text(&report), "plan validation failed — fix the errors above before running");
        Some(1)
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
    workspace_rungs: Option<&'a plan_validate::WorkspaceRungs>,
}

pub(crate) fn cmd_plan_validate(
    dir: &Path,
    workdir: &Path,
    strict: bool,
    json_output: bool,
    spec_quality: bool,
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
    let spec_report = spec_quality
        .then(|| plan_validate::collect_tasks_files(dir))
        .transpose()?
        .map(|files| roko_gate::spec_quality::lint_files(&files, workdir));

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
    }
    // A hard fail fails the run only under --strict; a low score never does.
    let spec_exit = spec_report
        .as_ref()
        .map_or(0, |spec_quality| spec_quality.exit_code(strict));
    Ok(report.exit_code(strict).max(spec_exit))
}

pub(crate) fn find_plan_source_document(plan_dir: &Path) -> Result<PathBuf> {
    for candidate in ["source-prd.md", "prd-extract.md", "plan.md"] {
        let path = plan_dir.join(candidate);
        if path.exists() {
            return Ok(path);
        }
    }

    anyhow::bail!(
        "no source PRD found in {} (looked for source-prd.md, prd-extract.md, and plan.md)",
        plan_dir.display()
    )
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

/// Reject Graph options whose promised enforcement is not implemented.
///
/// The caller invokes this before acquiring the workspace lock or constructing
/// any provider so `--approval` can never degrade into warning-and-continue.
fn validate_graph_execution_options(_engine: PlanEngine, _approval: bool) -> Result<()> {
    // Graph engine now supports approval mode via GraphExecutionControlAdapter.
    // The approval TUI thread is spawned separately and communicates through
    // the control channel. No validation needed.
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
    cli_model_override: Option<String>,
    dangerously_skip_permissions: bool,
    log_file: Option<&std::path::Path>,
    worktree_per_task: bool,
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

    // SIGINT/SIGTERM stop this run gracefully (cancel, finalize checkpoints,
    // restore the terminal, exit 130/143) for as long as the guard lives.
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

    run_graph_plan(roko_cli::graph_execution::GraphPlanRunParams {
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
        rich_topology,
        promote,
        no_tui,
        state_hub: None,
        interrupt: Some(interrupt),
        max_parallel_plans,
        fail_fast,
        only_plans: None,
        live_agent_output,
        force_disk_check: force,
        effort: None,
        no_cascade: false,
    })
    .await
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

    #[test]
    fn graph_approval_is_accepted_for_all_engines() {
        assert!(validate_graph_execution_options(PlanEngine::Graph, true).is_ok());
        assert!(validate_graph_execution_options(PlanEngine::Graph, false).is_ok());
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
