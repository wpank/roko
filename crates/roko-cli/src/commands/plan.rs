//! plan command handlers.

use std::io::IsTerminal as _;

use crate::*;
use anyhow::Context as _;
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
            let summaries =
                roko_cli::plan::summarize_discovered_plans(&wd).map_err(|e| anyhow!("{e}"))?;
            let executor_state = read_executor_state(&wd);
            let has_run_state = executor_state.is_some();
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
                        println!("no plans found in discovery path");
                    } else {
                        println!("no run state found");
                    }
                } else {
                    println!(
                        "{:<16} {:<40} {:<12} {}",
                        "ID", "TITLE", "PROGRESS", "STATUS"
                    );
                    for summary in &summaries {
                        println!(
                            "{:<16} {:<40} {:<12} {}",
                            summary.id.as_str(),
                            summary.title.as_str(),
                            format!("{}/{}", summary.tasks_done, summary.task_count),
                            summary.status_label()
                        );
                    }
                    if !has_run_state {
                        println!("(no run state found — counts from tasks.toml files)");
                    }
                }

                for (plan_id, _, _) in &state_entries {
                    if !plan_path_exists(&wd, plan_id) {
                        println!(
                            "warning: state references missing plan: {plan_id} (not found in plans/ or .roko/plans/)"
                        );
                    }
                }
            }
            Ok(EXIT_SUCCESS)
        }
        PlanCmd::Show { plan_id, workdir } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
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
                "[meta]\nplan = {:?}\nmax_parallel = 1\n\n# Add [[task]] entries below.\n",
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
        } => {
            let workdir = resolve_workdir(cli);
            let plans_dir = if dir.is_absolute() {
                dir.clone()
            } else {
                workdir.join(&dir)
            };
            let exit = cmd_plan_validate(&plans_dir, &workdir, strict, json || cli.json)?;

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
                roko_cli::index::check_plans_index(&workdir)?;
            } else {
                roko_cli::index::rebuild_plans_index(&workdir)?;
            }
            if !cli.quiet {
                let status = if check { "current" } else { "rebuilt" };
                println!("plans index {status}");
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
            screenshot_interval: _,
            screenshot_dir: _,
            batch_size,
            worktree_per_task,
            rich_topology,
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
            // Skipped when the user passes --skip-validate (e.g. freshly-generated plans).
            if !cli.skip_validate {
                if let Some(exit_code) = validate_before_run(&resolved_plans_dir, &wd) {
                    return Ok(exit_code);
                }

                // Cross-plan Graph semantics belong to the exact set selected by
                // `plan_loader` (one root plan, or the root's immediate plans),
                // not to the generic validator's recursive file discovery. Run
                // this preflight before both dry-run and workspace-lock mutation.
                validate_graph_selected_plans_before_run(engine, &resolved_plans_dir)?;
            }

            // ── Dry-run mode: parse plans + show summary without executing ──
            if dry_run {
                return cmd_plan_dry_run(&resolved_plans_dir, cli).await;
            }

            validate_graph_execution_options(engine, approval)?;

            // Both execution engines mutate shared workspace/runtime state.
            // Hold one guard across the complete selected engine lifetime.
            let _lock = roko_cli::workspace_lock::acquire_workspace_lock(layout.root())?;

            // ── Graph Engine path (explicit opt-in) ──
            if matches!(engine, PlanEngine::Graph) {
                // Warn about flags that are parsed at the top level but cannot
                // be forwarded to the Graph Engine. Without these warnings the
                // user would have no indication the flags were silently dropped.
                warn_graph_unsupported_flags(
                    cli.resume.as_deref(),
                    cli.effort.as_ref(),
                    log_file.as_deref(),
                    skip_preflight,
                    force,
                    screenshots,
                    batch_size,
                    cli.no_replan,
                    cli.skip_validate,
                    cli.quiet,
                );

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
                    no_tui,
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
            use roko_cli::agent_exec::{AgentExecEpisode, AgentExecOpts, run_agent_logged};

            let workdir = std::env::current_dir().context("resolve cwd")?;
            // Plan generation is read-only on workspace state: it reads source
            // code and writes to .roko/plans/ (per-slug, non-overlapping).
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
                let model_key = roko_cli::model_selection::resolve_effective_model_key(
                    &workdir,
                    cli.model.clone(),
                    Some("strategist"),
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

                    let exit_code = run_agent_logged(
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
                                tracing::info!(id, slug = slug.as_str(), status, "batch plan generate result");
                            }
                            _ => {
                                tracing::warn!(id, slug = slug.as_str(), status, "batch plan generate result");
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

                let model_key = roko_cli::model_selection::resolve_effective_model_key(
                    &workdir,
                    cli.model.clone(),
                    Some("strategist"),
                    "plan generate",
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

                    let system = roko_cli::plan_generate::build_generation_prompt(
                        &workdir, &combined, "notes",
                    );
                    let task_id = format!("plan:generate:notes:{slug}");
                    let task_prompt = format!(
                        "Read the notes below and generate an implementation plan directory \
                         under .roko/plans/{slug}/. \
                         Use the supplied bounded context; allow at most one repository-rooted \
                         exact-symbol query capped at 20 matches when a fact is missing. \
                         Create plan.md and tasks.toml files with tier and context \
                         (read_files with line ranges), mcp_servers (per-task MCP server names), \
                         and verify steps (executable shell commands). \
                         Use the cheapest model tier for each task.\n\n{combined}"
                    );

                    let exit_code = run_agent_logged(
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
                    )
                    .await;

                    match exit_code {
                        Ok(code) if code == EXIT_SUCCESS => {
                            tracing::info!(%slug, "plan generated from notes cluster");
                        }
                        Ok(code) => {
                            tracing::warn!(%slug, exit_code = code, "plan generate for cluster exited with non-zero code");
                        }
                        Err(err) => {
                            tracing::warn!(%slug, error = %err, "plan generate for cluster failed");
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
            let task_id = from_file
                .as_ref()
                .and_then(|path| path.file_stem())
                .and_then(|stem| stem.to_str())
                .map(|stem| format!("plan:generate:{stem}"))
                .unwrap_or_else(|| "plan:generate:prompt".to_string());
            let system = roko_cli::plan_generate::build_generation_prompt(
                &workdir,
                &source_text,
                source_type,
            );
            let model_key = roko_cli::model_selection::resolve_effective_model_key(
                &workdir,
                cli.model.clone(),
                Some("strategist"),
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
                    format!("\n\n<context>\n{loaded}</context>\n")
                } else {
                    String::new()
                }
            };

            let task_prompt = format!(
                "Read the source below and generate implementation plan directories under .roko/plans/. \
                 Use the supplied bounded context; allow at most one repository-rooted exact-symbol query \
                 capped at 20 matches when a fact is missing. \
                 Create plan.md and tasks.toml files with tier and context (read_files with line ranges), \
                 mcp_servers (per-task MCP server names), and verify steps (executable shell commands). \
                 Use the cheapest model tier for each task.\n\n{source_text}{context_block}"
            );

            let exit_code = run_agent_logged(
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
            )
            .await?;

            if exit_code != EXIT_SUCCESS {
                tracing::error!(
                    exit_code,
                    "plan generate: agent exited with non-zero code; \
                     check the latest episode in .roko/episodes.jsonl for details"
                );
            }

            // Validate all tasks.toml files written by the agent under .roko/plans/.
            // Check all files and collect all errors before reporting.
            let mut final_exit_code = exit_code;
            if exit_code == EXIT_SUCCESS {
                let plans_output_dir = workdir.join(".roko").join("plans");
                if plans_output_dir.is_dir() {
                    let mut validation_failed = false;
                    let entries = std::fs::read_dir(&plans_output_dir)
                        .with_context(|| format!("read {}", plans_output_dir.display()))?;
                    for entry in entries.flatten() {
                        let tasks_path = entry.path().join("tasks.toml");
                        if !tasks_path.is_file() {
                            continue;
                        }
                        match roko_cli::task_parser::TasksFile::parse(&tasks_path) {
                            Ok(tasks) => {
                                let policy = roko_cli::plan_policy::PlanExecutionPolicy::generated_for_environment(
                                    roko_cli::plan_policy::DEFAULT_GENERATED_TASK_LIMIT,
                                );
                                let issues = roko_cli::plan_policy::validate_plan_context(
                                    &tasks,
                                    &workdir,
                                    &entry.path(),
                                    policy,
                                );
                                if !issues.is_empty() {
                                    tracing::warn!(
                                        path = %tasks_path.display(),
                                        issues = %issues.iter().map(|i| format!("  - {i}")).collect::<Vec<_>>().join("\n"),
                                        "generated plan violates its execution contract"
                                    );
                                    validation_failed = true;
                                }
                            }
                            Err(err) => {
                                tracing::warn!(
                                    path = %tasks_path.display(),
                                    error = %err,
                                    "invalid tasks.toml"
                                );
                                validation_failed = true;
                            }
                        }
                    }
                    if validation_failed {
                        tracing::error!(
                            "plan generate: one or more generated tasks.toml files failed \
                             TOML validation (see warnings above)"
                        );
                        final_exit_code = 1;
                    }
                }
            }

            Ok(final_exit_code)
        }
        PlanCmd::Regenerate { plan_dir, dry_run } => {
            use roko_cli::agent_config::load_gateway_env;
            use roko_cli::agent_exec::{AgentExecEpisode, AgentExecOpts, run_agent_logged};

            let workdir = std::env::current_dir().context("resolve cwd")?;
            // Plan regeneration writes only to the target plan directory,
            // which is per-slug and non-overlapping with active plan runs.
            // No workspace lock needed (#226).
            let tasks_path = plan_dir.join("tasks.toml");
            if !tasks_path.exists() {
                anyhow::bail!("No tasks.toml found in {}", plan_dir.display());
            }

            let existing = std::fs::read_to_string(&tasks_path)
                .with_context(|| format!("read {}", tasks_path.display()))?;
            let existing_tasks = roko_cli::task_parser::TasksFile::parse(&tasks_path).ok();
            let source_path = find_plan_source_document(&plan_dir)?;
            let source_content = std::fs::read_to_string(&source_path)
                .with_context(|| format!("read {}", source_path.display()))?;
            let model_key = roko_cli::model_selection::resolve_effective_model_key(
                &workdir,
                cli.model.clone(),
                Some("strategist"),
                "plan regenerate",
            )?;

            // Collect pre-existing validation diagnostics so the agent knows what was wrong.
            let pre_validation_context =
                format_pre_validation_context(&tasks_path, &plan_validate::validate_plans_dir);

            if dry_run {
                let system = roko_cli::plan_generate::build_generation_prompt(
                    &workdir,
                    &source_content,
                    "prd",
                );
                let task_prompt = format!(
                    "Regenerate the plan at {} from the source PRD above. \
                     Rewrite tasks.toml in place with full modern metadata: tier, \
                     max_loc, files, allowed_tools, denied_tools, mcp_servers, depends_on, \
                     [task.context], and exactly one focused [[task.verify]] per task. Never set \
                     model_hint. Preserve the status of any task that \
                     is already marked done in the existing file. Do not create new plan \
                     directories.\n\n## Existing tasks.toml\n\n```toml\n{existing}\n```\
                     {pre_validation_context}",
                    tasks_path.display(),
                    existing = existing,
                );
                tracing::info!(
                    tasks = %tasks_path.display(),
                    source = %source_path.display(),
                    prompt_len = system.len() + task_prompt.len(),
                    "[dry-run] would regenerate plan"
                );
                return Ok(EXIT_SUCCESS);
            }

            let gw = load_gateway_env(&workdir);

            let system =
                roko_cli::plan_generate::build_generation_prompt(&workdir, &source_content, "prd");
            let task_prompt = format!(
                "Regenerate the plan at {} from the source PRD above. \
                 Rewrite tasks.toml in place with full modern metadata: tier, \
                 max_loc, files, allowed_tools, denied_tools, mcp_servers, depends_on, \
                 [task.context], and exactly one focused [[task.verify]] per task. Never set \
                 model_hint. Preserve the status of any task that \
                 is already marked done in the existing file. Do not create new plan \
                 directories.\n\n## Existing tasks.toml\n\n```toml\n{existing}\n```\
                 {pre_validation_context}",
                tasks_path.display(),
                existing = existing,
            );
            let plan_name = plan_dir
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("unknown");
            let task_id = format!("plan:regenerate:{plan_name}");

            let exit_code = match run_agent_logged(
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
                    task_kind: "plan-regenerate",
                    task_id: &task_id,
                },
            )
            .await
            {
                Ok(code) => code,
                Err(err) => {
                    std::fs::write(&tasks_path, &existing)
                        .with_context(|| format!("restore {}", tasks_path.display()))?;
                    return Err(err);
                }
            };

            if exit_code != 0 {
                std::fs::write(&tasks_path, &existing)
                    .with_context(|| format!("restore {}", tasks_path.display()))?;
                anyhow::bail!("plan regeneration agent failed with exit code {exit_code}");
            }

            let regenerated = match roko_cli::task_parser::TasksFile::parse(&tasks_path) {
                Ok(tasks) => tasks,
                Err(err) => {
                    std::fs::write(&tasks_path, &existing)
                        .with_context(|| format!("restore {}", tasks_path.display()))?;
                    return Err(err);
                }
            };

            let merged =
                preserve_completed_task_status(existing_tasks.as_ref(), regenerated, &plan_dir);
            let policy = roko_cli::plan_policy::PlanExecutionPolicy::generated_for_environment(
                roko_cli::plan_policy::DEFAULT_GENERATED_TASK_LIMIT,
            );
            let policy_issues =
                roko_cli::plan_policy::validate_plan_context(&merged, &workdir, &plan_dir, policy);
            if !policy_issues.is_empty() {
                std::fs::write(&tasks_path, &existing)
                    .with_context(|| format!("restore {}", tasks_path.display()))?;
                anyhow::bail!(
                    "regenerated tasks.toml violates the bounded execution contract:\n{}",
                    policy_issues
                        .iter()
                        .map(|issue| format!("  - {issue}"))
                        .collect::<Vec<_>>()
                        .join("\n")
                );
            }
            let rendered =
                toml::to_string_pretty(&merged).context("serialize regenerated tasks.toml")?;
            if let Err(err) = std::fs::write(&tasks_path, rendered) {
                std::fs::write(&tasks_path, &existing)
                    .with_context(|| format!("restore {}", tasks_path.display()))?;
                return Err(err.into());
            }

            match roko_cli::task_parser::TasksFile::validate_modern_fields(&tasks_path) {
                Ok(issues) if !issues.is_empty() => {
                    // Collect post-regeneration diagnostics for richer error output.
                    let post_context = format_pre_validation_context(
                        &tasks_path,
                        &plan_validate::validate_plans_dir,
                    );
                    std::fs::write(&tasks_path, &existing)
                        .with_context(|| format!("restore {}", tasks_path.display()))?;
                    anyhow::bail!(
                        "regenerated tasks.toml is still missing modern fields after regeneration.\n\
                         Missing fields: {missing}\n\
                         Pre-regeneration issues:{pre}\n\
                         Post-regeneration issues:{post}",
                        missing = issues
                            .into_iter()
                            .map(|issue| format!("{}: {:?}", issue.task_id, issue.missing_fields))
                            .collect::<Vec<_>>()
                            .join("; "),
                        pre = pre_validation_context,
                        post = post_context,
                    );
                }
                Ok(_) => {}
                Err(err) => {
                    std::fs::write(&tasks_path, &existing)
                        .with_context(|| format!("restore {}", tasks_path.display()))?;
                    return Err(err);
                }
            }

            Ok(EXIT_SUCCESS)
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

        PlanCmd::Status { workdir } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
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

            // Collect completed plan IDs from executor state.
            let completed: std::collections::HashSet<String> = read_executor_state(&wd)
                .unwrap_or_default()
                .into_iter()
                .filter(|(_, done, total)| *total > 0 && done == total)
                .map(|(id, _, _)| id)
                .collect();

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
/// to `cmd_plan` with a synthesized `PlanCmd::Run`.
pub(crate) async fn cmd_resume(
    cli: &Cli,
    run_id: Option<String>,
    workdir: Option<std::path::PathBuf>,
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
        if unified.exists() {
            unified
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
        max_tasks: 0,
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
    };
    cmd_plan(cli, plan_cmd).await
}

/// Parse and display a plan directory without executing anything.
pub(crate) async fn cmd_plan_dry_run(plans_dir: &Path, cli: &Cli) -> Result<i32> {
    let plans = roko_cli::orchestrator::discover_plans(plans_dir)
        .map_err(|e| anyhow!("plan discovery failed: {e}"))?;

    if plans.is_empty() {
        if cli.json {
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "dry_run": true,
                    "plans": [],
                    "total_plans": 0,
                    "total_tasks": 0,
                }))?
            );
        } else {
            println!("No plans found in {}", plans_dir.display());
        }
        return Ok(EXIT_SUCCESS);
    }

    // For each plan, try to load and count tasks.
    let mut plan_summaries: Vec<serde_json::Value> = Vec::new();
    let mut total_tasks: usize = 0;
    let mut total_estimated_minutes: u32 = 0;

    for plan in &plans {
        // Try loading the tasks.toml adjacent to the plan file.
        let tasks_path = plan
            .path
            .parent()
            .map(|p| p.join("tasks.toml"))
            .filter(|p| p.exists());

        let (task_count, task_details) = if let Some(ref tp) = tasks_path {
            match roko_cli::task_parser::TasksFile::parse(tp) {
                Ok(tf) => {
                    let details: Vec<serde_json::Value> = tf
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
                    (tf.tasks.len(), details)
                }
                Err(_) => (0, vec![]),
            }
        } else {
            // New-layout plans might have tasks.toml at plans_dir/plan_name/tasks.toml
            let dir_tasks = plans_dir.join(&plan.base).join("tasks.toml");
            if dir_tasks.exists() {
                match roko_cli::task_parser::TasksFile::parse(&dir_tasks) {
                    Ok(tf) => {
                        let details: Vec<serde_json::Value> = tf
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
                        (tf.tasks.len(), details)
                    }
                    Err(_) => (0, vec![]),
                }
            } else {
                (0, vec![])
            }
        };

        total_tasks += task_count;
        if let Some(ref fm) = plan.frontmatter
            && let Some(mins) = fm.estimated_minutes
        {
            total_estimated_minutes += mins;
        }

        plan_summaries.push(json!({
            "plan": plan.base,
            "num": plan.num,
            "task_count": task_count,
            "estimated_minutes": plan.frontmatter.as_ref().and_then(|f| f.estimated_minutes),
            "parallel_width": plan.frontmatter.as_ref().and_then(|f| f.estimated_parallel_width),
            "priority": plan.frontmatter.as_ref().and_then(|f| f.priority),
            "tags": plan.frontmatter.as_ref().map(|f| &f.tags),
            "tasks": task_details,
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
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!(
            "Dry run: {} plan(s), {} task(s) in {}\n",
            plans.len(),
            total_tasks,
            plans_dir.display()
        );

        for (i, plan) in plans.iter().enumerate() {
            let est = plan
                .frontmatter
                .as_ref()
                .and_then(|f| f.estimated_minutes)
                .map(|m| format!(" (~{m} min)"))
                .unwrap_or_default();
            let priority = plan
                .frontmatter
                .as_ref()
                .and_then(|f| f.priority)
                .map(|p| format!(" [priority={p}]"))
                .unwrap_or_default();
            println!("  {}. {}{}{}", i + 1, plan.base, est, priority);

            // Print task list if available.
            if let Some(tasks) = plan_summaries[i].get("tasks").and_then(|v| v.as_array()) {
                for t in tasks {
                    let tid = t.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                    let title = t.get("title").and_then(|v| v.as_str()).unwrap_or("");
                    let status = t
                        .get("status")
                        .and_then(|v| v.as_str())
                        .unwrap_or("pending");
                    let tier = t.get("tier").and_then(|v| v.as_str()).unwrap_or("?");
                    let deps = t
                        .get("depends_on")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            let ids: Vec<&str> = arr.iter().filter_map(|v| v.as_str()).collect();
                            if ids.is_empty() {
                                String::new()
                            } else {
                                format!(" (after {})", ids.join(", "))
                            }
                        })
                        .unwrap_or_default();
                    println!("     {tid}: {title} [{tier}, {status}]{deps}");
                }
            }
        }

        if total_estimated_minutes > 0 {
            println!("\nEstimated total: ~{total_estimated_minutes} min");
        }
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

    let code = report.exit_code(false);
    if code != 0 {
        tracing::error!(report = %plan_validate::render_text(&report), "plan validation failed — fix the errors above before running");
        Some(1)
    } else {
        None
    }
}

pub(crate) fn cmd_plan_validate(
    dir: &Path,
    workdir: &Path,
    strict: bool,
    json_output: bool,
) -> Result<i32> {
    let config_path = workdir.join("roko.toml");
    let models = if config_path.is_file() {
        let config_text = std::fs::read_to_string(&config_path)
            .with_context(|| format!("read {}", config_path.display()))?;
        let config: RokoConfig = toml::from_str(&config_text)
            .map_err(|error| anyhow!(error))
            .with_context(|| format!("parse {}", config_path.display()))?;
        Some(crate::commands::config_cmd::configured_models(&config))
    } else {
        None
    };

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

    if json_output {
        println!("{}", plan_validate::render_json(&report)?);
    } else {
        let mut text = plan_validate::render_text(&report);
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
    }
    Ok(report.exit_code(strict))
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

pub(crate) fn normalize_task_title(title: &str) -> String {
    title
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

pub(crate) fn preserve_completed_task_status(
    old_tasks: Option<&roko_cli::task_parser::TasksFile>,
    mut regenerated: roko_cli::task_parser::TasksFile,
    plan_dir: &Path,
) -> roko_cli::task_parser::TasksFile {
    if let Some(old_tasks) = old_tasks {
        let completed: Vec<&roko_cli::task_parser::TaskDef> = old_tasks
            .tasks
            .iter()
            .filter(|task| task.status.eq_ignore_ascii_case("done"))
            .collect();

        for task in &mut regenerated.tasks {
            let normalized = normalize_task_title(&task.title);
            if completed.iter().any(|old| {
                old.id == task.id
                    || normalize_task_title(&old.title) == normalized
                    || normalize_task_title(&old.title).contains(&normalized)
                    || normalized.contains(&normalize_task_title(&old.title))
            }) {
                task.status = "done".to_string();
            }
        }

        regenerated.meta.iteration = old_tasks.meta.iteration.saturating_add(1);
        if regenerated.meta.plan.trim().is_empty() {
            regenerated.meta.plan = old_tasks.meta.plan.clone();
        }
    }

    if regenerated.meta.plan.trim().is_empty() {
        regenerated.meta.plan = plan_dir
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown-plan".to_string());
    }

    regenerated.meta.total = regenerated.tasks.len() as u32;
    regenerated.meta.done = regenerated
        .tasks
        .iter()
        .filter(|task| task.status.eq_ignore_ascii_case("done"))
        .count() as u32;
    regenerated.meta.status =
        if regenerated.meta.total > 0 && regenerated.meta.done == regenerated.meta.total {
            "complete".to_string()
        } else {
            "ready".to_string()
        };

    regenerated
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
    ) -> anyhow::Result<crate::plan_validate::ValidationReport>,
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

/// Collect and validate the plan-level dependency graph used by the Graph
/// Engine host. A single-plan graph cannot represent `depends_on_plan`, so the
/// host enforces those dependencies before constructing or dispatching one.
fn graph_plan_execution_order(
    plans: &[roko_cli::runner::plan_loader::Plan],
) -> Result<(
    Vec<String>,
    std::collections::BTreeMap<String, std::collections::BTreeSet<String>>,
)> {
    use std::collections::{BTreeMap, BTreeSet};

    let mut plan_ids = BTreeSet::new();
    for plan in plans {
        if !plan_ids.insert(plan.id.as_str()) {
            anyhow::bail!(
                "Graph selected plan set contains duplicate plan ID '{}'",
                plan.id
            );
        }
    }

    let dependencies = plans
        .iter()
        .map(|plan| {
            let dependencies = plan
                .tasks
                .tasks
                .iter()
                .flat_map(|task| task.depends_on_plan.iter().cloned())
                .collect::<BTreeSet<_>>();
            (plan.id.clone(), dependencies)
        })
        .collect::<BTreeMap<_, _>>();

    let order = graph_plan_topological_order(&dependencies)?;
    Ok((order, dependencies))
}

/// Validate the exact Graph plan set before any execution-only side effects.
///
/// Graph dry-run follows the same cross-plan dependency rules as execution.
/// The engine validates the freshly loaded set again under its workspace lock
/// to fail closed if plan files change between preflight and execution.
fn validate_graph_selected_plans_before_run(engine: PlanEngine, plans_dir: &Path) -> Result<()> {
    if matches!(engine, PlanEngine::Graph) {
        let plans = roko_cli::runner::plan_loader::load_plans(plans_dir)?;
        graph_plan_execution_order(&plans)?;
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

/// Emit explicit warnings for CLI flags that are silently ignored by the
/// Graph Engine. Called just before entering the graph execution path so
/// operators are never surprised by dropped configuration.
///
/// Flags that ARE forwarded to the graph engine (and thus do NOT warn):
///   `--model`, `--dangerously-skip-permissions`,
///   `--resume-plan`, `--fresh`, `--force-resume`, `--max-retries`,
///   `--max-tasks`, `--budget-override`, `--no-budget`, `--no-tui`
///
/// `--approval` / `--tui` is rejected as an error by
/// `validate_graph_execution_options` above, not warned here.
#[allow(clippy::fn_params_excessive_bools)]
fn warn_graph_unsupported_flags(
    resume_session: Option<&str>,
    effort: Option<&Effort>,
    log_file: Option<&std::path::Path>,
    skip_preflight: bool,
    force: bool,
    screenshots: bool,
    batch_size: Option<usize>,
    no_replan: bool,
    skip_validate: bool,
    quiet: bool,
) {
    if quiet {
        return;
    }

    if let Some(session) = resume_session {
        tracing::warn!(
            session,
            "--resume is not supported with --engine graph and will be ignored"
        );
    }
    if effort.is_some() {
        tracing::warn!(
            "--effort is not supported with --engine graph and will be ignored; \
             the graph engine uses the configured default_effort"
        );
    }
    // --log-file is now wired for Graph Engine (#115) -- no warning needed.
    let _ = log_file;
    if skip_preflight {
        tracing::warn!(
            "--skip-preflight is not supported with --engine graph and will be ignored; \
             the graph engine runs its own provider preflight"
        );
    }
    if force {
        tracing::warn!(
            "--force is not supported with --engine graph and will be ignored; \
             the graph engine does not perform a disk-space pre-check"
        );
    }
    if screenshots {
        tracing::warn!("--screenshots is not supported with --engine graph and will be ignored");
    }
    if batch_size.is_some() {
        tracing::warn!("--batch-size is not supported with --engine graph and will be ignored");
    }
    if no_replan {
        tracing::warn!(
            "--no-replan has no effect with --engine graph; \
             the Graph engine does not replan on gate failure \
             (use max_retries in tasks.toml to control retries)"
        );
    }
    if skip_validate {
        tracing::warn!(
            "--skip-validate skips tasks.toml structure checks but the Graph engine \
             always runs its own internal graph validation before execution"
        );
    }
}

fn graph_plan_topological_order(
    dependencies: &std::collections::BTreeMap<String, std::collections::BTreeSet<String>>,
) -> Result<Vec<String>> {
    use std::collections::{BTreeMap, BTreeSet};

    let mut indegree = BTreeMap::new();
    let mut dependents = BTreeMap::<String, BTreeSet<String>>::new();
    for (plan_id, plan_dependencies) in dependencies {
        for dependency in plan_dependencies {
            if dependency == plan_id {
                anyhow::bail!("Graph plan '{plan_id}' cannot depend on itself");
            }
            if !dependencies.contains_key(dependency) {
                anyhow::bail!(
                    "Graph plan '{plan_id}' depends on unknown plan '{dependency}' in the selected plan set"
                );
            }
            dependents
                .entry(dependency.clone())
                .or_default()
                .insert(plan_id.clone());
        }
        indegree.insert(plan_id.clone(), plan_dependencies.len());
    }

    let mut ready = indegree
        .iter()
        .filter_map(|(plan_id, degree)| (*degree == 0).then_some(plan_id.clone()))
        .collect::<BTreeSet<_>>();
    let mut order = Vec::with_capacity(dependencies.len());
    while let Some(plan_id) = ready.iter().next().cloned() {
        ready.remove(&plan_id);
        order.push(plan_id.clone());

        if let Some(plan_dependents) = dependents.get(&plan_id) {
            for dependent in plan_dependents {
                let Some(degree) = indegree.get_mut(dependent) else {
                    anyhow::bail!("Graph plan dependency index is inconsistent for '{dependent}'");
                };
                *degree = degree.saturating_sub(1);
                if *degree == 0 {
                    ready.insert(dependent.clone());
                }
            }
        }
    }

    if order.len() != dependencies.len() {
        let cycle = indegree
            .into_iter()
            .filter_map(|(plan_id, degree)| (degree > 0).then_some(plan_id))
            .collect::<Vec<_>>();
        anyhow::bail!(
            "Graph plan dependency cycle involving: {}",
            cycle.join(", ")
        );
    }

    Ok(order)
}

fn unsatisfied_graph_plan_dependencies(
    plan_id: &str,
    dependencies: &std::collections::BTreeMap<String, std::collections::BTreeSet<String>>,
    outcomes: &std::collections::BTreeMap<String, bool>,
) -> Vec<String> {
    dependencies
        .get(plan_id)
        .into_iter()
        .flatten()
        .filter(|dependency| outcomes.get(*dependency) != Some(&true))
        .cloned()
        .collect()
}

/// Inline progress telemetry sink that prints per-node lifecycle events to
/// stderr and delegates to the inner (StateHub) sink. This provides real-time
/// feedback during Graph engine execution without requiring the full TUI.
struct InlineProgressTelemetrySink {
    inner: std::sync::Arc<dyn roko_core::TelemetryEventSink>,
    show_progress: bool,
}

#[async_trait::async_trait]
impl roko_core::TelemetryEventSink for InlineProgressTelemetrySink {
    async fn emit(
        &self,
        event: &roko_core::ObservableEvent,
        ancestry: &[roko_core::LensScope],
    ) -> roko_core::error::Result<Vec<roko_core::Signal>> {
        if self.show_progress {
            match event {
                roko_core::ObservableEvent::CellStarted { block, .. } => {
                    // User-facing progress output (no TUI active)
                    eprintln!("    \u{25b8} executing node '{block}'...");
                }
                roko_core::ObservableEvent::CellCompleted {
                    block,
                    duration_ms,
                    cost_usd,
                    ..
                } => {
                    let secs = *duration_ms as f64 / 1000.0;
                    // User-facing progress output (no TUI active)
                    if *cost_usd > 0.0 {
                        eprintln!(
                            "    \u{2713} node '{block}' completed ({secs:.1}s, ${cost_usd:.4})"
                        );
                    } else {
                        eprintln!("    \u{2713} node '{block}' completed ({secs:.1}s)");
                    }
                }
                roko_core::ObservableEvent::CellFailed { block, error, .. } => {
                    // User-facing progress output (no TUI active)
                    eprintln!("    \u{2717} node '{block}' failed: {error}");
                }
                _ => {}
            }
        }
        self.inner.emit(event, ancestry).await
    }
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
    no_tui: bool,
) -> Result<i32> {
    use std::sync::Arc;

    use roko_graph::cell::CellContext;
    use roko_graph::cells::{TaskDispatcher, TaskExecutorCell};
    use roko_graph::convert::{PlanTaskInfo, plan_to_graph};
    use roko_graph::engine::GraphEngine;

    let run_start = std::time::Instant::now();
    let plans = roko_cli::runner::plan_loader::load_plans(plans_dir)?;
    // Validate the complete selected set before initializing extensions or
    // launching a provider. This makes missing and cyclic cross-plan
    // dependencies fail closed without partially executing the batch.
    let (plan_execution_order, plan_dependencies) = graph_plan_execution_order(&plans)?;

    // Build the same provider/config/extension foundation used by runner-v2.
    // Graph tasks are Activities, but they must still share rate limits,
    // health state, prompt context, MCP/plugin handlers, and safety contracts.
    let mut roko_config = roko_core::config::loader::load_config_validated(workdir)
        .map_err(|error| anyhow!("load Graph runtime config: {error}"))?
        .into_config();
    roko_core::config::loader::normalize_and_validate_dispatch_models(&mut roko_config)
        .context("validate model configuration before Graph dispatch")?;

    // Note: --no-replan has no meaningful effect in the Graph engine.
    // The Graph engine uses max_retries (from tasks.toml) for retry control
    // and does not perform plan-level replanning on gate failure. A warning
    // is emitted by warn_graph_unsupported_flags before reaching this path.

    // Merge CLI flag with config (same logic as runner-v2).
    let dangerously_skip_permissions =
        dangerously_skip_permissions || roko_config.runner.dangerously_skip_permissions;

    let (plan_budget_ceiling, budget_override_active) = resolve_budget_ceiling(
        budget_override,
        no_budget,
        f64::from(roko_config.budget.max_plan_usd),
    );
    if !roko_config.agent.default_model.trim().is_empty() {
        crate::commands::util::preflight_provider_for_model(
            &roko_config,
            &roko_config.agent.default_model,
        )?;
    }
    let graph_run_config = roko_cli::runner::RunConfig::from_roko_config(
        workdir.to_path_buf(),
        plans_dir.to_path_buf(),
        roko_config.clone(),
    );
    roko_cli::runner::extension_loader::initialize_extensions(graph_run_config.extension_chain.as_ref())
        .await?;

    let roko_config = Arc::new(roko_config);
    let prompt_cache = Arc::new(roko_cli::dispatch::PromptCache::load(workdir));
    let mut shared_factory = roko_cli::dispatch::SharedAgentFactory::new(
        Arc::clone(&roko_config),
        roko_config.agent.mcp_config.as_ref(),
        graph_run_config.cascade_router.clone(),
        Some(prompt_cache),
    )
    .await
    .with_health_registry(Arc::new(
        roko_learn::provider_health::ProviderHealthRegistry::load_or_new(
            &RokoLayout::for_project(workdir)
                .learn_dir()
                .join("provider-health.json"),
        ),
    ))
    .with_error_patterns_from_disk(workdir);
    let plugin_catalog = roko_cli::runner::extension_loader::resolve_plugin_tool_catalog(
        workdir,
        &roko_config.agent.extensions,
        &[],
    )?;
    if !plugin_catalog.plugin_tools().is_empty() {
        shared_factory = shared_factory.with_local_tool_runtime(plugin_catalog.local_runtime());
    }
    let shared_factory = Arc::new(shared_factory);
    if dangerously_skip_permissions {
        tracing::warn!(
            "running Graph Engine with --dangerously-skip-permissions: agents will execute tools without approval"
        );
    }

    // ── Learning/feedback subsystem wiring ─────────────────────────────
    //
    // Build the same feedback infrastructure that Runner-v2 uses, so Graph
    // engine runs produce episodes, efficiency events, playbook outcomes,
    // routing observations, experiment settlements, and daimon feedback.
    let graph_layout = RokoLayout::for_project(workdir);
    let graph_learn_dir = graph_layout.learn_dir();
    let _ = std::fs::create_dir_all(&graph_learn_dir);

    // ── #144: Construct daimon state early so it can be shared between
    //    the feedback facade (for plan-completion persistence) and the
    //    GraphFeedbackContext (for dispatch-time affect modulation). ───────
    let affect_path = workdir.join(".roko").join("daimon").join("affect.json");
    let shared_daimon_state: Option<std::sync::Arc<std::sync::Mutex<roko_daimon::DaimonState>>> = {
        let dims_vec = &roko_config.daimon.strategy_space.dimensions;
        if dims_vec.len() == 8 {
            let dims: [String; 8] = dims_vec.clone().try_into().unwrap();
            let def = roko_daimon::StrategySpaceDefinition {
                domain: roko_config.daimon.strategy_space.domain.clone(),
                dimensions: dims,
            };
            let mut s = roko_daimon::DaimonState::load_or_new(&affect_path);
            let _ = s.configure_strategy_space(def);
            Some(std::sync::Arc::new(std::sync::Mutex::new(s)))
        } else {
            tracing::warn!(
                dims = dims_vec.len(),
                "daimon strategy_space.dimensions must have exactly 8 entries; skipping"
            );
            None
        }
    };

    let graph_episodes_path = graph_layout.root_episodes_path();
    let graph_feedback_facade = {
        let mut facade = roko_cli::runtime_feedback::FeedbackFacade::new()
            .with_sink(std::sync::Arc::new(
                roko_cli::runtime_feedback::EpisodeSink::at(&graph_episodes_path),
            ));
        if let Some(cascade) = &graph_run_config.cascade_router {
            facade = facade.with_sink(std::sync::Arc::new(
                roko_cli::runtime_feedback::RoutingObservationSink::new(
                    cascade.clone(),
                ),
            ));
        }

        // ── #143: Dream consolidation trigger on plan completion ────────
        facade = facade.with_sink(std::sync::Arc::new(
            roko_cli::runtime_feedback::DreamConsolidationSink::new(
                workdir.to_path_buf(),
                roko_config.learning.dream_on_completion,
                roko_config.learning.dreams.trigger_on_plan_complete,
            ),
        ));

        // ── #144: Daimon affect persistence on plan completion ──────────
        if let Some(ref daimon) = shared_daimon_state {
            facade = facade.with_sink(std::sync::Arc::new(
                roko_cli::runtime_feedback::DaimonPersistenceSink::new(
                    affect_path.clone(),
                    std::sync::Arc::clone(daimon),
                ),
            ));
        }

        // ── Theta reflection on plan completion ─────────────────────────
        //
        // Runs a five-phase reflective cycle (gamma summary, affect update,
        // calibration check, progress assessment, meta-cognition) after each
        // plan completes. Lightweight and synchronous (no LLM calls).
        let shared_cortical = std::sync::Arc::new(
            roko_runtime::heartbeat::CorticalState::default(),
        );
        let shared_theta = std::sync::Arc::new(std::sync::Mutex::new(
            roko_runtime::theta_consumer::ThetaConsumer::default(),
        ));
        facade = facade.with_sink(std::sync::Arc::new(
            roko_cli::runtime_feedback::ThetaReflectionSink::new(
                std::sync::Arc::clone(&shared_theta),
                std::sync::Arc::clone(&shared_cortical),
            ),
        ));

        // ── Delta consolidation on plan completion ──────────────────────
        //
        // Tracks episode counts and checks trigger conditions for a dream
        // consolidation cycle (NREM replay, REM imagination, integration).
        // Bridges roko_runtime::delta_consumer into the feedback pipeline.
        let shared_delta = std::sync::Arc::new(std::sync::Mutex::new(
            roko_runtime::delta_consumer::DeltaConsumer::default(),
        ));
        facade = facade.with_sink(std::sync::Arc::new(
            roko_cli::runtime_feedback::DeltaConsolidationSink::new(
                std::sync::Arc::clone(&shared_delta),
                std::sync::Arc::clone(&shared_cortical),
            ),
        ));

        std::sync::Arc::new(facade)
    };

    // ── P0-04: CodingOracle ─────────────────────────────────────────────
    //
    // Persists across the plan run, accumulating build/test observations
    // for predictive gate feedback. Mirrors Runner-v2's CodingOracle.
    let coding_oracle = std::sync::Arc::new(
        roko_learn::oracles::coding::CodingOracle::new(),
    );

    // ── P1-01: GateGamingDetector ────────────────────────────────────
    //
    // Flags when agents game the gate system by passing gates at an
    // increasing rate while delivering lower-quality outputs. Alerts are
    // appended to a JSONL file on disk.
    let gate_gaming_detector = std::sync::Arc::new(tokio::sync::Mutex::new(
        roko_learn::GateGamingDetector::new(
            graph_learn_dir.join("gate-gaming-alerts.jsonl"),
        ),
    ));

    // ── P1-04: HoldoutExperiment ─────────────────────────────────────
    //
    // Deterministic 80/20 train/holdout split for detecting overfitting
    // in learned routing. Learning updates are gated behind the holdout
    // partition check.
    let holdout_experiment = std::sync::Arc::new(tokio::sync::Mutex::new(
        roko_learn::HoldoutExperiment::load_or_new(
            graph_learn_dir.join("holdout-state.json"),
        )
        .unwrap_or_else(|err| {
            tracing::warn!(error = %err, "failed to load holdout experiment state; starting fresh");
            roko_learn::HoldoutExperiment::new(
                graph_learn_dir.join("holdout-state.json"),
            )
        }),
    ));

    // ── P2-01: ShadowRunner ─────────────────────────────────────────
    //
    // Records shadow dispatch decisions (infrastructure-only; no actual
    // shadow task spawn). Uses the configured default model as the
    // shadow alternative.
    let shadow_runner = std::sync::Arc::new(roko_learn::shadow::ShadowRunner::new(
        roko_learn::shadow::ShadowConfig {
            model_slug: roko_config.agent.default_model.clone(),
            prompt_variant: None,
            label: "graph-shadow".to_string(),
        },
        graph_learn_dir.join("shadow-results.jsonl"),
    ));

    let graph_feedback = roko_cli::graph_task_dispatch::GraphFeedbackContext {
        feedback_facade: Some(graph_feedback_facade),
        efficiency_path: Some(graph_learn_dir.join("efficiency.jsonl")),
        costs_path: Some(graph_learn_dir.join("costs.jsonl")),
        playbook_dir: Some(graph_learn_dir.join("playbooks")),
        // Reuse the daimon state constructed above so the feedback facade
        // persistence sink and dispatch-time modulation share the same
        // mutable state (#144).
        daimon_state: shared_daimon_state,
        experiment_store_path: Some(graph_learn_dir.join("experiments.json")),
        gate_failures_path: Some(graph_layout.gate_failures_path()),
        replan_on_gate_failure: roko_config.learning.replan_on_gate_failure,
        coding_oracle: Some(coding_oracle),
        gate_gaming_detector: Some(gate_gaming_detector),
        holdout_experiment: Some(holdout_experiment.clone()),
        shadow_runner: Some(shadow_runner),
        eval_generation_enabled: true,
    };

    // ── TUI vs inline progress decision ──────────────────────────────
    //
    // Auto-enable the interactive TUI dashboard when stdout is an
    // interactive terminal, unless the user explicitly opted out with
    // --no-tui, --quiet, or --json. This mirrors the runner-v2 approval
    // TUI logic (line ~470).
    let launch_tui =
        !no_tui && !cli.quiet && !cli.json && std::io::stdout().is_terminal();

    // Keep the full SharedStateHub alive so the TUI can subscribe to the
    // live event stream. Previously this path only extracted sender().
    let state_hub = roko_cli::state_hub::shared_state_hub();
    let state_hub_sender = state_hub.sender();
    let state_hub_sink: Arc<dyn roko_core::TelemetryEventSink> = Arc::new(
        roko_cli::runner::graph_tui_bridge::StateHubTelemetrySink::new(state_hub_sender.clone()),
    );

    // Inline progress display: print per-node start/complete/fail to stderr
    // so the user can see what the Graph engine is doing in real time.
    // Disabled when the TUI is active — events flow through the dashboard
    // instead of being printed inline.
    let show_progress = !cli.quiet && !cli.json && !launch_tui;
    let graph_telemetry: Arc<dyn roko_core::TelemetryEventSink> = Arc::new(
        InlineProgressTelemetrySink {
            inner: state_hub_sink,
            show_progress,
        },
    );

    // Wire graph engine execution into the TUI dashboard event stream.
    // Create separate TUI bridges for the task dispatcher (agent output
    // streaming) and the graph lifecycle bridge (plan/node events).
    let dispatcher_tui_bridge = roko_cli::runner::tui_bridge::TuiBridge::new(state_hub_sender.clone());
    let graph_tui_bridge = roko_cli::runner::graph_tui_bridge::GraphTuiBridge::new(
        roko_cli::runner::tui_bridge::TuiBridge::new(state_hub_sender),
    );

    let mut dispatcher_builder = roko_cli::graph_task_dispatch::GraphTaskDispatcher::new(
        Arc::clone(&shared_factory),
        Arc::clone(&roko_config),
        workdir.to_path_buf(),
    )
    .with_plan_budget(
        plan_budget_ceiling,
        f64::from(roko_config.budget.max_turn_usd),
        budget_override_active,
    )
    .with_cli_model_override(cli_model_override)
    .with_dangerously_skip_permissions(dangerously_skip_permissions)
    .with_feedback(graph_feedback)
    .with_tui_bridge(dispatcher_tui_bridge);

    // ── Per-task worktree isolation (opt-in via --worktree-per-task) ──
    if worktree_per_task {
        use roko_cli::orchestrator::worktree::{WorktreeConfig, WorktreeManager};
        let worktree_manager = WorktreeManager::new(WorktreeConfig {
            repo_root: workdir.to_path_buf(),
            base_branch: "HEAD".to_string(),
            worktrees_root: workdir.join(".roko").join("worktrees"),
            max_live: None,
            idle_ttl: std::time::Duration::from_hours(1),
        });
        let workspace_provider = Arc::new(
            roko_cli::graph_execution::WorktreeExecutionWorkspaceProvider::new(worktree_manager),
        );
        if !cli.quiet && !cli.json {
            tracing::info!("per-task worktree isolation enabled (--worktree-per-task)");
        }
        dispatcher_builder = dispatcher_builder.with_workspace_provider(workspace_provider);
    }

    let graph_task_dispatcher = Arc::new(dispatcher_builder);
    let task_dispatcher: Arc<dyn TaskDispatcher> = graph_task_dispatcher.clone();

    // ── Spawn interactive TUI thread ─────────────────────────────────
    //
    // Replicates the runner-v2 approval TUI pattern: spawn the App on a
    // dedicated OS thread so it owns the terminal while the async engine
    // drives execution on the current task.
    let mut tui_handle: Option<std::thread::JoinHandle<anyhow::Result<()>>> = None;
    if launch_tui {
        // Redirect stderr to a log file so tracing output does not
        // corrupt the TUI's raw terminal display.
        let layout = RokoLayout::for_project(workdir);
        let stderr_log_path = layout.runner_stderr_log();
        let _ = std::fs::create_dir_all(stderr_log_path.parent().unwrap_or(workdir));
        #[cfg(unix)]
        if let Ok(log_file) = std::fs::File::create(&stderr_log_path) {
            use std::os::unix::io::AsRawFd;
            #[allow(unsafe_code)]
            unsafe {
                libc::dup2(log_file.as_raw_fd(), 2);
            }
        }

        let state_hub_for_tui = state_hub.clone();
        let workdir_for_tui = workdir.to_path_buf();
        let handle = std::thread::Builder::new()
            .name("roko-graph-engine-tui".to_string())
            .spawn(move || {
                let app = App::new_connected_with_page(
                    &workdir_for_tui,
                    None, // Default page = Tab::Dashboard
                    &state_hub_for_tui,
                )
                .without_mouse_capture()
                .with_exit_on_plan_completion();
                app.run()
            })
            .context("spawn Graph Engine TUI thread")?;
        tui_handle = Some(handle);
    }

    // ── Canonical --log-file recorder for Graph Engine (#115) ──
    let graph_event_logger: Option<Arc<dyn roko_graph::events::GraphEventSink>> = match log_file {
        Some(path) => {
            let resolved = if path.is_absolute() {
                path.to_path_buf()
            } else {
                workdir.join(path)
            };
            let logger = roko_cli::runner::structured_log::GraphEventLogger::open(&resolved)
                .map_err(|e| anyhow!("open --log-file {}: {e}", resolved.display()))?;
            Some(Arc::new(logger))
        }
        None => None,
    };

    let total_tasks: usize = plans.iter().map(|p| p.tasks.tasks.len()).sum();
    let plan_count = plans.len();

    if !cli.quiet && !cli.json && !launch_tui {
        let plan_names: Vec<&str> = plan_execution_order.iter().map(String::as_str).collect();
        tracing::info!(
            plan_count,
            total_tasks,
            plans = plan_names.join(", "),
            "running plans via Graph Engine"
        );
    }

    let mut all_succeeded = true;
    let mut total_output_count = 0usize;
    let mut plan_outcomes = std::collections::BTreeMap::<String, bool>::new();

    for plan_id in &plan_execution_order {
        let plan = plans
            .iter()
            .find(|plan| &plan.id == plan_id)
            .ok_or_else(|| anyhow!("Graph execution order references unloaded plan '{plan_id}'"))?;
        let unsatisfied =
            unsatisfied_graph_plan_dependencies(&plan.id, &plan_dependencies, &plan_outcomes);
        if !unsatisfied.is_empty() {
            tracing::warn!(
                plan_id = %plan.id,
                prerequisites = unsatisfied.join(", "),
                "plan blocked: prerequisite plan(s) did not succeed"
            );
            graph_tui_bridge.log_event(
                "graph.plan_blocked",
                &format!(
                    "plan '{}' blocked: prerequisites {}",
                    plan.id,
                    unsatisfied.join(", ")
                ),
            );
            plan_outcomes.insert(plan.id.clone(), false);
            all_succeeded = false;
            continue;
        }

        if !cli.quiet && !cli.json && !launch_tui {
            tracing::info!(
                plan_id = %plan.id,
                task_count = plan.tasks.tasks.len(),
                "running plan via Graph Engine"
            );
        }

        // Convert Runner v2 tasks into PlanTaskInfo for the converter.
        let tasks: Vec<(String, PlanTaskInfo)> = plan
            .tasks
            .tasks
            .iter()
            .map(|t| {
                let info = PlanTaskInfo {
                    title: t.title.clone(),
                    description: t.description.clone(),
                    role: t.role.clone(),
                    tier: t.tier.clone(),
                    model_hint: t.model_hint.clone(),
                    files: t.files.clone(),
                    depends_on: t.depends_on.clone(),
                    depends_on_plan: t.depends_on_plan.clone(),
                    timeout_secs: t.timeout_secs,
                    max_retries: max_retries.unwrap_or(t.max_retries),
                    domain: t.domain.as_ref().map(|d| format!("{d:?}")),
                    sequence: t.sequence,
                    full_config_json: serde_json::to_value(t).unwrap_or_default(),
                };
                (t.id.clone(), info)
            })
            .collect();

        let max_parallel = if max_tasks > 0 {
            u32::try_from(max_tasks).unwrap_or(u32::MAX)
        } else {
            plan.tasks.meta.max_parallel
        };
        let max_parallel_usize =
            usize::try_from(max_parallel.max(1)).unwrap_or(usize::MAX);
        let plan_dir_str = plan.dir.display().to_string();

        let (graph, registry) = if rich_topology {
            // ── Rich 11-node-per-task production topology ──────────────────
            // Warn: enricher cells are currently PassthroughCell stubs and do
            // not yet add runtime value. The richer topology is available for
            // incremental implementation of each enricher cell type.
            if !cli.quiet && !cli.json {
                tracing::info!(
                    "--rich-topology is active; enricher cells (knowledge, \
                     episodes, playbook, modulation, safety, experiment) are \
                     currently passthrough stubs"
                );
            }
            let topo = roko_graph::ProductionPlanTopology::new(
                &plan.id,
                &plan_dir_str,
                max_parallel_usize,
            );
            // Convert PlanTaskInfo -> TopologyTaskInfo.
            // Note: info.max_retries is already resolved (max_retries.unwrap_or(t.max_retries))
            // during the PlanTaskInfo construction above, so we use it directly.
            let topo_tasks: Vec<roko_graph::TopologyTaskInfo> = tasks
                .iter()
                .map(|(id, info)| roko_graph::TopologyTaskInfo {
                    task_id: id.clone(),
                    title: info.title.clone(),
                    description: info.description.clone(),
                    role: info.role.clone(),
                    tier: info.tier.clone(),
                    model_hint: info.model_hint.clone(),
                    files: info.files.clone(),
                    depends_on: info.depends_on.clone(),
                    timeout_secs: info.timeout_secs,
                    max_retries: info.max_retries,
                    domain: info.domain.clone(),
                    sequence: info.sequence,
                    full_config_json: info.full_config_json.clone(),
                })
                .collect();
            match topo.build(&topo_tasks) {
                Ok((g, _report)) => {
                    let mut reg = roko_graph::default_registry();
                    // Register stub passthrough cells for all topology node types.
                    roko_graph::register_topology_cells(&mut reg);
                    let plan_dispatcher = Arc::clone(&task_dispatcher);
                    reg.register("task-executor", move |config| {
                        Box::new(TaskExecutorCell::live(config, Arc::clone(&plan_dispatcher)))
                    });
                    (g, reg)
                }
                Err(e) => {
                    graph_tui_bridge.error(&format!(
                        "failed to build rich topology for plan '{}': {e}",
                        plan.id
                    ));
                    tracing::error!(plan_id = %plan.id, error = %e, "failed to build rich topology for plan");
                    plan_outcomes.insert(plan.id.clone(), false);
                    all_succeeded = false;
                    continue;
                }
            }
        } else {
            // ── Simple single-Activity-per-task converter (default) ─────────
            match plan_to_graph(&plan.id, &plan_dir_str, &tasks, max_parallel) {
                Ok(g) => {
                    let mut reg = roko_graph::default_registry();
                    let plan_dispatcher = Arc::clone(&task_dispatcher);
                    reg.register("task-executor", move |config| {
                        Box::new(TaskExecutorCell::live(config, Arc::clone(&plan_dispatcher)))
                    });
                    (g, reg)
                }
                Err(e) => {
                    graph_tui_bridge.error(&format!(
                        "failed to convert plan '{}' to graph: {e}",
                        plan.id
                    ));
                    tracing::error!(plan_id = %plan.id, error = %e, "failed to convert plan to graph");
                    plan_outcomes.insert(plan.id.clone(), false);
                    all_succeeded = false;
                    continue;
                }
            }
        };
        let mut checkpoint = roko_cli::graph_checkpoint::prepare_graph_checkpoint(
            workdir,
            resume_plan,
            &plan.id,
            plan_count,
            &graph,
            fresh,
            force_resume,
        )?;
        let run_id = checkpoint.run_id().to_string();
        let replayed_entries = checkpoint.replayed_entries();
        graph_task_dispatcher
            .attach_plan_budget_checkpoint(&plan.id, checkpoint.take_cost_ledger())?;
        let mut engine = GraphEngine::new(graph, registry)
            .with_recorder(checkpoint.take_recorder())
            .with_telemetry(Arc::clone(&graph_telemetry))
            // Allow stub cells when using the rich topology. Enricher cells are
            // PassthroughCell stubs; without this the engine rejects the graph
            // at validate_for_start time.
            .with_allow_test_stubs(rich_topology);
        // Wire canonical --log-file recorder (#115): attach the event sink
        // so every GraphExecutionEvent is written to JSONL.
        if let Some(ref sink) = graph_event_logger {
            engine = engine.with_event_sink(Arc::clone(sink));
        }
        if let Some(replayer) = checkpoint.take_replayer() {
            engine = engine.with_replayer(replayer);
        }
        let ctx = CellContext::new().with_run_id(run_id);

        // Validate before running.
        let issues = engine.validate();
        if !issues.is_empty() {
            graph_tui_bridge.error(&format!(
                "plan '{}' has {} validation error{}",
                plan.id,
                issues.len(),
                if issues.len() == 1 { "" } else { "s" },
            ));
            tracing::error!(plan_id = %plan.id, error_count = issues.len(), "plan has validation errors");
            for issue in &issues {
                tracing::error!(plan_id = %plan.id, issue, "validation error");
            }
            plan_outcomes.insert(plan.id.clone(), false);
            all_succeeded = false;
            checkpoint.finish(false)?;
            continue;
        }

        if replayed_entries > 0 && !cli.quiet && !cli.json {
            tracing::info!(
                plan_id = %plan.id,
                replayed_entries,
                manifest = %checkpoint.paths().manifest.display(),
                "resuming plan with replayed task outputs"
            );
        }

        // ── Graph TUI bridge: emit PlanStarted + per-node TaskStarted ──
        let plan_task_count = tasks.len();
        graph_tui_bridge.plan_started(&plan.id, plan_task_count);
        graph_tui_bridge.log_event(
            "graph.plan_executing",
            &format!(
                "plan '{}': {} task{}, engine=graph",
                plan.id,
                plan_task_count,
                if plan_task_count == 1 { "" } else { "s" },
            ),
        );
        // Pre-populate the TUI plan tree with all nodes.
        for (task_id, info) in &tasks {
            graph_tui_bridge.node_started(&plan.id, task_id, &info.title);
        }

        match engine.execute(&ctx).await {
            Ok(output) => {
                let output_count = output
                    .node_results
                    .iter()
                    .map(|r| r.output_count)
                    .sum::<usize>();
                total_output_count += output_count;
                let budget = graph_task_dispatcher.plan_budget_snapshot(&plan.id);
                let execution_succeeded = output.success && !budget.dispatch_blocked;

                // ── Graph TUI bridge: emit per-node completions + PlanCompleted ──
                roko_cli::runner::graph_tui_bridge::emit_plan_lifecycle(
                    &graph_tui_bridge,
                    &plan.id,
                    plan_task_count,
                    &output,
                    execution_succeeded,
                );

                if !cli.quiet && !cli.json {
                    if execution_succeeded {
                        tracing::info!(
                            plan_id = %plan.id,
                            node_count = output.node_results.len(),
                            output_count,
                            "plan completed: SUCCESS"
                        );
                    } else {
                        tracing::warn!(
                            plan_id = %plan.id,
                            node_count = output.node_results.len(),
                            output_count,
                            "plan completed: FAILED"
                        );
                    }
                    // Log per-node error details so failures are not silent.
                    if !execution_succeeded {
                        for result in &output.node_results {
                            if let Some(error) = &result.error {
                                tracing::warn!(
                                    node_id = %result.node_id,
                                    status = ?result.status,
                                    %error,
                                    "node failed"
                                );
                            }
                        }
                    }
                    if budget.exhausted {
                        let ceiling = budget.ceiling_usd.unwrap_or_default();
                        if budget.dispatch_blocked {
                            tracing::warn!(
                                plan_id = %plan.id,
                                spent_usd = budget.spent_usd,
                                ceiling_usd = ceiling,
                                "plan budget exhausted"
                            );
                        } else {
                            tracing::warn!(
                                plan_id = %plan.id,
                                spent_usd = budget.spent_usd,
                                ceiling_usd = ceiling,
                                "plan budget exhausted; explicit override active, continuing"
                            );
                        }
                    }
                }
                if !execution_succeeded {
                    all_succeeded = false;
                }
                plan_outcomes.insert(plan.id.clone(), execution_succeeded);
                checkpoint.finish(execution_succeeded)?;
            }
            Err(e) => {
                // ── Graph TUI bridge: emit error + PlanCompleted(false) ──
                graph_tui_bridge.error(&format!("plan '{}' execution failed: {e}", plan.id));
                graph_tui_bridge.plan_completed(&plan.id, false);

                tracing::error!(plan_id = %plan.id, error = %e, "plan execution failed");
                plan_outcomes.insert(plan.id.clone(), false);
                all_succeeded = false;
                checkpoint.finish(false)?;
            }
        }
    }

    let plan_budgets = plans
        .iter()
        .map(|plan| {
            let budget = graph_task_dispatcher.plan_budget_snapshot(&plan.id);
            serde_json::json!({
                "plan_id": plan.id,
                "spent_usd": budget.spent_usd,
                "reserved_usd": budget.reserved_usd,
                "ceiling_usd": budget.ceiling_usd,
                "exhausted": budget.exhausted,
                "dispatch_blocked": budget.dispatch_blocked,
            })
        })
        .collect::<Vec<_>>();
    let total_cost_usd = plans
        .iter()
        .map(|plan| {
            graph_task_dispatcher
                .plan_budget_snapshot(&plan.id)
                .spent_usd
        })
        .sum::<f64>();

    if let Some(extension_chain) = graph_run_config.extension_chain.as_ref() {
        let mut chain = extension_chain.lock().await;
        for (name, error) in chain.shutdown_all().await {
            tracing::warn!(extension = %name, %error, "extension shutdown failed");
            all_succeeded = false;
        }
    }

    // ── Wait for TUI to exit ───────────────────────────────────────
    // The TUI is operator-owned: keep the final state visible until the
    // operator explicitly quits (same semantics as the runner-v2 path).
    if !all_succeeded {
        state_hub
            .sender()
            .publish(roko_core::DashboardEvent::Error {
                message: format!(
                    "Graph Engine: {plan_count} plan(s), {} succeeded, {} failed",
                    plan_outcomes.values().filter(|v| **v).count(),
                    plan_outcomes.values().filter(|v| !**v).count(),
                ),
            });
    }
    // ── Persist holdout experiment state ────────────────────────────
    //
    // Save holdout state so overfitting detection survives across runs
    // and partition assignments remain stable. Mirrors Runner-v2 cleanup.
    if let Ok(exp) = holdout_experiment.try_lock() {
        if let Err(err) = exp.save() {
            tracing::warn!(error = %err, "failed to persist holdout experiment state (non-fatal)");
        }
    }

    // ── Persist run metrics (backlog #169) ──────────────────────────
    //
    // Collect task counts and cost from the just-completed plan loop and
    // append a structured RunMetricsRecord to `.roko/learn/run-metrics.jsonl`.
    // The write is fire-and-forget on a background task so it never blocks
    // the TUI exit path.
    {
        let duration_ms = run_start.elapsed().as_millis() as u64;
        let per_plan: Vec<roko_learn::run_metrics::PlanMetrics> = plan_outcomes
            .iter()
            .map(|(id, succeeded)| {
                let plan_tasks = plans
                    .iter()
                    .find(|p| &p.id == id)
                    .map_or(0, |p| p.tasks.tasks.len());
                let completed = if *succeeded { plan_tasks } else { 0 };
                roko_learn::run_metrics::PlanMetrics {
                    plan_id: id.clone(),
                    completed: *succeeded,
                    tasks_completed: completed,
                    tasks_failed: plan_tasks.saturating_sub(completed),
                }
            })
            .collect();
        let tasks_completed: usize = per_plan.iter().map(|p| p.tasks_completed).sum();
        let tasks_failed: usize = per_plan.iter().map(|p| p.tasks_failed).sum();
        let any_budget_exhausted = plans.iter().any(|p| {
            graph_task_dispatcher
                .plan_budget_snapshot(&p.id)
                .exhausted
        });
        let (agg_tokens_in, agg_tokens_out, agg_dispatch_count) =
            graph_task_dispatcher.run_aggregate_stats();
        let record = roko_learn::run_metrics::RunMetricsRecord {
            run_id: format!("graph-run-{}", chrono::Utc::now().timestamp_millis().max(0)),
            timestamp: chrono::Utc::now().to_rfc3339(),
            duration_ms,
            total_tasks,
            tasks_completed,
            tasks_failed,
            total_cost_usd,
            total_tokens_in: agg_tokens_in,
            total_tokens_out: agg_tokens_out,
            total_agent_calls: agg_dispatch_count as usize,
            budget_exhausted: any_budget_exhausted,
            plans: per_plan,
        };
        let metrics_path = graph_learn_dir.join("run-metrics.jsonl");
        tokio::spawn(async move {
            if let Err(err) = roko_learn::run_metrics::append_run_metrics(&metrics_path, &record) {
                tracing::warn!(error = %err, "failed to persist run metrics (non-fatal)");
            }
        });
    }

    join_approval_tui_thread(tui_handle.take());

    if cli.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "engine": "graph",
                "dry_run": false,
                "succeeded": all_succeeded,
                "plan_count": plan_count,
                "total_tasks": total_tasks,
                "total_outputs": total_output_count,
                "total_cost_usd": total_cost_usd,
                "plan_budgets": plan_budgets,
            }))
            .unwrap_or_default()
        );
    } else if !cli.quiet && !launch_tui {
        // Only print the summary line when no TUI was shown — otherwise
        // the TUI already rendered all progress information interactively.
        tracing::info!(
            plan_count,
            total_tasks,
            total_output_count,
            total_cost_usd,
            "Graph Engine complete"
        );
    }

    Ok(if all_succeeded {
        EXIT_SUCCESS
    } else {
        EXIT_FAILURE
    })
}

/// Resolve the effective per-plan USD ceiling from CLI flags and config.
///
/// Priority order (highest to lowest):
/// 1. `no_budget = true` → `0.0` (unlimited — no enforcement)
/// 2. `budget_override = Some(amount)` → `amount.max(0.0)` (explicit CLI ceiling)
/// 3. `config_max_plan_usd` (from `roko.toml [budget].max_plan_usd`)
///
/// Returns `(effective_ceiling, bypass_block)` where `bypass_block` is `true`
/// when the caller explicitly provided a ceiling via the CLI (so the runner
/// warns on overage instead of hard-blocking).
pub(crate) fn resolve_budget_ceiling(
    budget_override: Option<f64>,
    no_budget: bool,
    config_max_plan_usd: f64,
) -> (f64, bool) {
    if no_budget {
        // Disable enforcement: ceiling 0.0 means unlimited.
        (0.0, true)
    } else if let Some(ceiling) = budget_override {
        // Explicit CLI ceiling — clamp negatives to 0.0 (unlimited).
        (ceiling.max(0.0), true)
    } else {
        // Use the config value unchanged; no CLI override active.
        (config_max_plan_usd, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn graph_test_plan(id: &str, dependencies: &[&str]) -> roko_cli::runner::plan_loader::Plan {
        let mut tasks = roko_cli::task_parser::TasksFile::parse_str(
            r#"
[meta]
plan = "test-plan"

[[task]]
id = "T1"
title = "Test task"
role = "researcher"
"#,
        )
        .expect("parse graph test plan");
        tasks.meta.plan = id.to_string();
        tasks.tasks[0].depends_on_plan = dependencies
            .iter()
            .map(|dependency| (*dependency).to_string())
            .collect();
        roko_cli::runner::plan_loader::Plan {
            id: id.to_string(),
            dir: PathBuf::from(id),
            tasks,
            prd_excerpt: String::new(),
        }
    }

    #[test]
    fn read_executor_state_returns_none_without_snapshot() {
        let dir = tempdir().expect("tempdir");
        assert!(read_executor_state(dir.path()).is_none());
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

    // ── resolve_budget_ceiling unit tests ────────────────────────────────

    /// No CLI flags: config value is used, bypass is false.
    #[test]
    fn budget_ceiling_no_override_uses_config() {
        let (ceiling, bypass) = resolve_budget_ceiling(None, false, 25.0);
        assert!((ceiling - 25.0).abs() < 1e-12);
        assert!(!bypass, "should not bypass block when using config value");
    }

    /// `--budget-override 50.0`: ceiling is set to 50.0, bypass is true.
    #[test]
    fn budget_ceiling_override_amount_sets_ceiling_and_bypass() {
        let (ceiling, bypass) = resolve_budget_ceiling(Some(50.0), false, 25.0);
        assert!((ceiling - 50.0).abs() < 1e-12);
        assert!(bypass, "CLI override should enable bypass");
    }

    /// `--budget-override 0`: ceiling is 0.0 (unlimited), bypass is true.
    #[test]
    fn budget_ceiling_override_zero_means_unlimited() {
        let (ceiling, bypass) = resolve_budget_ceiling(Some(0.0), false, 25.0);
        assert!((ceiling - 0.0).abs() < 1e-12);
        assert!(bypass, "zero ceiling should enable bypass");
    }

    /// `--budget-override <negative>`: clamped to 0.0 (unlimited), bypass is true.
    #[test]
    fn budget_ceiling_override_negative_clamped_to_zero() {
        let (ceiling, bypass) = resolve_budget_ceiling(Some(-10.0), false, 25.0);
        assert!(
            (ceiling - 0.0).abs() < 1e-12,
            "negative should be clamped to 0"
        );
        assert!(bypass, "negative ceiling should enable bypass");
    }

    /// `--no-budget`: ceiling is 0.0, bypass is true regardless of config.
    #[test]
    fn budget_ceiling_no_budget_flag_disables_enforcement() {
        let (ceiling, bypass) = resolve_budget_ceiling(None, true, 100.0);
        assert!(
            (ceiling - 0.0).abs() < 1e-12,
            "--no-budget should set ceiling to 0.0"
        );
        assert!(bypass, "--no-budget should enable bypass");
    }

    /// `--no-budget` takes precedence over config even when config is unlimited.
    #[test]
    fn budget_ceiling_no_budget_with_zero_config() {
        let (ceiling, bypass) = resolve_budget_ceiling(None, true, 0.0);
        assert!((ceiling - 0.0).abs() < 1e-12);
        assert!(bypass);
    }

    /// When `budget_override` is Some, the config value is ignored entirely.
    #[test]
    fn budget_ceiling_override_ignores_config() {
        let config_val = 999.0;
        let (ceiling, bypass) = resolve_budget_ceiling(Some(5.0), false, config_val);
        assert!(
            (ceiling - 5.0).abs() < 1e-12,
            "override should ignore config"
        );
        assert!(bypass);
    }

    #[test]
    fn graph_plan_order_honors_dependencies_before_lexical_order() {
        use std::collections::{BTreeMap, BTreeSet};

        let dependencies = BTreeMap::from([
            (
                "a-consumer".to_string(),
                BTreeSet::from(["z-foundation".to_string()]),
            ),
            ("m-independent".to_string(), BTreeSet::new()),
            ("z-foundation".to_string(), BTreeSet::new()),
        ]);

        let order = graph_plan_topological_order(&dependencies).expect("valid plan graph");
        assert_eq!(order, ["m-independent", "z-foundation", "a-consumer"]);
    }

    #[test]
    fn graph_plan_order_rejects_unknown_dependency_before_dispatch() {
        use std::collections::{BTreeMap, BTreeSet};

        let dependencies = BTreeMap::from([(
            "consumer".to_string(),
            BTreeSet::from(["missing-foundation".to_string()]),
        )]);

        let error = graph_plan_topological_order(&dependencies).expect_err("unknown plan");
        assert!(
            error
                .to_string()
                .contains("unknown plan 'missing-foundation'")
        );
    }

    #[test]
    fn graph_plan_order_rejects_self_dependency_before_dispatch() {
        use std::collections::{BTreeMap, BTreeSet};

        let dependencies = BTreeMap::from([(
            "self-dependent".to_string(),
            BTreeSet::from(["self-dependent".to_string()]),
        )]);

        let error = graph_plan_topological_order(&dependencies).expect_err("self dependency");
        assert!(
            error
                .to_string()
                .contains("plan 'self-dependent' cannot depend on itself")
        );
    }

    #[test]
    fn graph_plan_order_rejects_duplicate_plan_ids_before_collection() {
        let plans = [
            graph_test_plan("duplicate", &[]),
            graph_test_plan("duplicate", &[]),
        ];

        let error = graph_plan_execution_order(&plans).expect_err("duplicate plan ID");
        assert!(error.to_string().contains("duplicate plan ID 'duplicate'"));
    }

    #[test]
    fn graph_plan_order_rejects_cross_plan_cycle_before_dispatch() {
        use std::collections::{BTreeMap, BTreeSet};

        let dependencies = BTreeMap::from([
            ("plan-a".to_string(), BTreeSet::from(["plan-b".to_string()])),
            ("plan-b".to_string(), BTreeSet::from(["plan-a".to_string()])),
        ]);

        let error = graph_plan_topological_order(&dependencies).expect_err("dependency cycle");
        assert!(error.to_string().contains("dependency cycle"));
        assert!(error.to_string().contains("plan-a, plan-b"));
    }

    #[test]
    fn graph_plan_failure_blocks_downstream_plan() {
        use std::collections::{BTreeMap, BTreeSet};

        let dependencies = BTreeMap::from([
            ("foundation".to_string(), BTreeSet::new()),
            (
                "consumer".to_string(),
                BTreeSet::from(["foundation".to_string()]),
            ),
        ]);
        let failed = BTreeMap::from([("foundation".to_string(), false)]);
        assert_eq!(
            unsatisfied_graph_plan_dependencies("consumer", &dependencies, &failed),
            ["foundation"]
        );

        let succeeded = BTreeMap::from([("foundation".to_string(), true)]);
        assert!(
            unsatisfied_graph_plan_dependencies("consumer", &dependencies, &succeeded).is_empty()
        );
    }

    #[test]
    fn graph_plan_failure_blocks_transitive_downstream_plans() {
        use std::collections::{BTreeMap, BTreeSet};

        let dependencies = BTreeMap::from([
            ("foundation".to_string(), BTreeSet::new()),
            (
                "middle".to_string(),
                BTreeSet::from(["foundation".to_string()]),
            ),
            (
                "consumer".to_string(),
                BTreeSet::from(["middle".to_string()]),
            ),
        ]);
        let mut outcomes = BTreeMap::from([("foundation".to_string(), false)]);

        assert_eq!(
            unsatisfied_graph_plan_dependencies("middle", &dependencies, &outcomes),
            ["foundation"]
        );
        outcomes.insert("middle".to_string(), false);
        assert_eq!(
            unsatisfied_graph_plan_dependencies("consumer", &dependencies, &outcomes),
            ["middle"]
        );
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

        let error = validate_graph_selected_plans_before_run(PlanEngine::Graph, &plans_dir)
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

    /// Smoke-test: `warn_graph_unsupported_flags` must not panic regardless
    /// of the flag combination. The actual warning output goes to stderr and
    /// is validated manually or via integration tests.
    #[test]
    fn warn_graph_unsupported_flags_does_not_panic() {
        // All flags off (quiet = true suppresses output).
        warn_graph_unsupported_flags(None, None, None, false, false, false, None, false, false, true);
        // All flags on (quiet = true still suppresses).
        warn_graph_unsupported_flags(
            Some("session-id"),
            Some(&Effort::High),
            Some(std::path::Path::new("/tmp/log.jsonl")),
            true,
            true,
            true,
            Some(5),
            true,
            true,
            true,
        );
        // All flags on, quiet = false (will write to stderr but must not panic).
        warn_graph_unsupported_flags(
            Some("session-id"),
            Some(&Effort::High),
            Some(std::path::Path::new("/tmp/log.jsonl")),
            true,
            true,
            true,
            Some(5),
            true,
            true,
            false,
        );
    }
}
