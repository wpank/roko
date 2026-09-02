//! util command handlers.
#![allow(unused_imports)]

use crate::*;
use roko_core::config::schema::RokoConfig;
use roko_fs::RokoLayout;
use std::io::IsTerminal;

/// Print a dim next-step hint to stderr, only when stdout is a TTY.
pub(crate) fn print_next_step_hint(msg: &str) {
    if std::io::stdout().is_terminal() {
        // \x1b[2m = dim, \x1b[0m = reset
        eprintln!("\x1b[2m{msg}\x1b[0m");
    }
}

/// Returns `true` on success, `false` when the topic is not recognised.
pub(crate) fn cmd_explain(topic: &str, depth: u8) -> bool {
    use roko_cli::explain;
    let depth = depth.clamp(1, 3);
    if topic == "topics" || topic == "list" {
        println!("available topics:");
        for name in explain::topic_names() {
            let Some(entry) = explain::find_topic(name) else {
                continue;
            };
            println!("  {:<12} {}", name, entry.title);
        }
        return true;
    }
    match explain::find_topic(topic) {
        Some(entry) => {
            print!("{}", explain::render_topic(entry, depth));
            true
        }
        None => {
            eprintln!("unknown topic: {topic}");
            eprintln!("available topics: {}", explain::topic_names().join(", "));
            eprintln!("run `roko explain topics` to see all topics with descriptions");
            false
        }
    }
}

/// Read piped stdin and dispatch as a one-shot prompt.
///
/// Routes through the v2 inline path, not the legacy `run_once()` stub.
pub(crate) async fn cmd_pipe(cli: &Cli) -> Result<i32> {
    let pipe = PipeMode::new().with_json(cli.json).with_quiet(cli.quiet);

    let input = pipe
        .read_input(&mut std::io::stdin().lock())
        .map_err(|e| anyhow!("read stdin: {e}"))?;

    if input.text.is_empty() {
        if !cli.quiet {
            eprintln!("no input received on stdin");
        }
        return Ok(EXIT_SYSTEM_ERROR);
    }

    if input.truncated && !cli.quiet {
        eprintln!(
            "warning: stdin input truncated at {} bytes",
            input.bytes_read
        );
    }

    // Dispatch the piped text via the v2 inline chat path.
    roko_cli::unified::cmd_oneshot_inline(&input.text, cli.quiet).await
}

pub(crate) async fn cmd_headless(cli: &Cli) -> Result<i32> {
    let workdir = resolve_workdir(cli);
    prepare_runtime_hooks(&workdir, cli.quiet);
    roko_cli::daemon::daemon_start(&workdir, false, roko_cli::DEFAULT_SERVE_PORT).await?;
    Ok(EXIT_SUCCESS)
}

pub(crate) async fn cmd_init(
    path: Option<PathBuf>,
    cloud: bool,
    profile: Option<String>,
    demo: bool,
) -> Result<()> {
    let target = path.unwrap_or_else(|| PathBuf::from("."));
    tokio::fs::create_dir_all(&target)
        .await
        .with_context(|| format!("create {}", target.display()))?;
    let roko_dir = target.join(".roko");
    tokio::fs::create_dir_all(&roko_dir)
        .await
        .with_context(|| format!("create {}", roko_dir.display()))?;

    // Create versioned filesystem layout directories for subsystems that
    // still read roko-fs layout metadata directly.
    let layout = RokoLayout::for_project(&target);
    layout
        .ensure_dirs()
        .await
        .with_context(|| "create .roko layout directories")?;

    // Create additional directories used by CLI subsystems but not in
    // RokoLayout::top_level_dirs() (jobs, prd, task-outputs, etc.).
    for extra in &[
        roko_dir.join("jobs"),
        roko_dir.join("prd"),
        roko_dir.join("prd").join("published"),
        roko_dir.join("prd").join("drafts"),
        roko_dir.join("task-outputs"),
        roko_dir.join("research"),
        roko_dir.join("subscriptions"),
        roko_dir.join("templates"),
    ] {
        tokio::fs::create_dir_all(extra)
            .await
            .with_context(|| format!("create {}", extra.display()))?;
    }

    let engrams_path = roko_dir.join("engrams.jsonl");
    if !engrams_path.exists() {
        // Migrate from legacy name if present, but only rows that parse as
        // Engram.  Non-Engram rows (e.g. GateVerdict) stay in signals.jsonl
        // to avoid schema-mixing in the engram store.
        let legacy = roko_dir.join("signals.jsonl");
        if legacy.exists() {
            let content = tokio::fs::read_to_string(&legacy)
                .await
                .with_context(|| format!("read legacy {}", legacy.display()))?;

            let mut engram_lines = Vec::new();
            let mut kept_lines = Vec::new();
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if serde_json::from_str::<roko_core::Signal>(trimmed).is_ok() {
                    engram_lines.push(line.to_string());
                } else {
                    kept_lines.push(line.to_string());
                }
            }

            // Write valid engram rows to engrams.jsonl.
            if engram_lines.is_empty() {
                tokio::fs::write(&engrams_path, b"")
                    .await
                    .with_context(|| format!("create {}", engrams_path.display()))?;
            } else {
                let mut out = engram_lines.join("\n");
                out.push('\n');
                tokio::fs::write(&engrams_path, out.as_bytes())
                    .await
                    .with_context(|| {
                        format!("write migrated engrams to {}", engrams_path.display())
                    })?;
            }

            // If all rows migrated, remove the legacy file; otherwise leave
            // the non-Engram rows in place for the gate-verdicts migration.
            if kept_lines.is_empty() {
                let _ = tokio::fs::remove_file(&legacy).await;
            } else {
                let mut remainder = kept_lines.join("\n");
                remainder.push('\n');
                tokio::fs::write(&legacy, remainder.as_bytes())
                    .await
                    .with_context(|| format!("rewrite non-Engram rows to {}", legacy.display()))?;
            }
        } else {
            tokio::fs::write(&engrams_path, b"")
                .await
                .with_context(|| format!("create {}", engrams_path.display()))?;
        }
    }

    // Domain detection: use --profile if given, otherwise auto-detect.
    let domain = if let Some(ref p) = profile {
        p.as_str()
    } else {
        crate::commands::prd::detect_project_domain(&target)
    };

    let config_path = target.join("roko.toml");
    let demo_config = if config_path.exists() {
        println!(
            "{} already exists; leaving untouched.",
            config_path.display()
        );
        match tokio::fs::read_to_string(&config_path).await {
            Ok(text) => RokoConfig::from_toml(&text).ok(),
            Err(_) => None,
        }
    } else {
        let default = Config::default_toml_template(cloud)?;
        tokio::fs::write(&config_path, &default)
            .await
            .with_context(|| format!("write {}", config_path.display()))?;
        println!("wrote {}", config_path.display());
        RokoConfig::from_toml(&default).ok()
    };

    println!("initialized roko workspace at {}", target.display());
    println!("detected project domain: {domain}");
    println!(
        "suggested gates: {}",
        crate::commands::prd::domain_gate_hint(domain)
    );
    println!(
        "default provider command set to \"claude\". \
         Edit roko.toml [providers.claude_cli] to use a different command."
    );

    if demo {
        let report = roko_cli::demo_seed::seed_demo_workspace(&target, demo_config.as_ref())?;
        if report.any_seeded() {
            println!("{}", report.summary());
        } else {
            println!("demo data already present; leaving existing files untouched.");
        }
    }

    // Check for interrupted session from a previous run.
    let snapshot = roko_dir.join("state").join("executor.json");
    if snapshot.is_file() {
        println!();
        println!("interrupted session found: {}", snapshot.display());
        println!(
            "resume with: roko plan run plans/ --resume {}",
            snapshot.display()
        );
    }

    // Validate provider readiness (informational only — never fail init).
    let auth = roko_cli::auth_detect::detect_auth_from_config(&target);
    println!();
    match &auth {
        roko_cli::auth_detect::AuthMethod::NeedsSetup => {
            println!("warning: no provider credentials found.");
            println!();
            println!("The workspace is initialized but roko cannot dispatch agents yet.");
            println!("Next step:");
            println!("  roko config providers available   # see what providers are supported");
            println!("  export ANTHROPIC_API_KEY=sk-ant-...");
            println!("Or add a [providers.*] block to roko.toml with an API key.");
        }
        other => {
            println!("provider: {} \u{2014} ready", other.label());
        }
    }

    print_next_step_hint(
        "Next: roko doctor (verify setup) · roko setup (configure providers) · roko develop \"your task\"",
    );

    Ok(())
}

pub(crate) async fn cmd_run(
    cli: &Cli,
    workdir: Option<PathBuf>,
    prompt: String,
    serve: bool,
    share: bool,
    provider: Option<String>,
    max_retries: Option<u32>,
) -> Result<i32> {
    // Build CLI overrides from clap-parsed args instead of re-parsing
    // process args or laundering through env vars.
    let overrides = roko_cli::run::CliOverrides {
        model: cli.model.clone(),
        role: cli.role.clone(),
        provider,
        cascade_enabled: None,
    };

    let workdir = workdir.unwrap_or_else(|| resolve_workdir(cli));
    prepare_runtime_hooks(&workdir, cli.quiet);
    let _lock = roko_cli::workspace_lock::acquire_workspace_lock(&workdir.join(".roko"))?;
    let mut config = resolve_config_for_workdir(cli, &workdir)?;
    apply_resume_session_override(&mut config, cli.resume.clone());

    // Apply --max-retries to the learning config.
    if let Some(retries) = max_retries {
        config.learning.replan_max_per_plan = Some(retries);
    }

    // Optionally start the HTTP control plane for external observability.
    let server_guard: Option<(
        std::sync::Arc<roko_serve::state::AppState>,
        tokio::task::JoinHandle<anyhow::Result<()>>,
    )> = if serve || share {
        let repo_registry = RepoRegistry::load(&config, &workdir).unwrap_or_default();
        let state_hub = roko_serve::state::AppState::state_hub_for_workdir(&workdir);
        // Create a shared MetricRegistry so the runtime and the HTTP server
        // expose the same counters on /metrics (E09-T03).
        let metrics = std::sync::Arc::new(roko_core::obs::metrics::MetricRegistry::new());
        let runtime = roko_cli::serve_runtime::RokoCliRuntime::new_with_state_hub_and_metrics(
            config.clone(),
            repo_registry,
            state_hub.clone(),
            Some(std::sync::Arc::clone(&metrics)),
        );
        runtime.prepare_workspace_extensions(&workdir).await?;
        let runtime = runtime.into_arc();
        let roko_config = roko_core::config::loader::load_config_unified(&workdir)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        let server_config =
            roko_serve::ServerBuildConfig::new(workdir.clone(), runtime, roko_config, None, None)
                .with_state_hub(state_hub)
                .with_metrics(metrics);
        let (state, handle) = roko_serve::ServerBuilder::new(server_config)
            .start_background()
            .await?;
        if !cli.quiet {
            eprintln!("▸ HTTP server started on :6677");
        }
        Some((state, handle))
    } else {
        None
    };

    // TODO(R2_G01): Read workflow template from roko.toml once a [pipeline]
    // config section is added to the Config struct. For now, fall back to "standard".
    let template = "standard";

    // Build enabled gates list and typed shell commands from declared gate configs.
    let enabled_gates = roko_cli::run::workflow_enabled_gate_names(&config.gates);
    let shell_gates = roko_cli::run::workflow_shell_gate_commands(&config.gates);

    let result = roko_cli::run::run_workflow_engine_report_with_hub(
        &prompt,
        &workdir,
        template,
        enabled_gates,
        shell_gates,
        None,
        &overrides,
    )
    .await;

    // Shut down the HTTP server if it was started.
    if let Some((state, handle)) = server_guard {
        state.cancel.cancel();
        let _ = handle.await;
    }

    match result {
        Ok(report) => {
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else if !cli.quiet {
                roko_cli::run::print_workflow_run_report(&prompt, template, &report);
            }

            if !report.success {
                match roko_cli::run::workflow_report_outcome(&report) {
                    Some(roko_core::WorkflowOutcome::Halted { reason }) => {
                        eprintln!("error: workflow halted: {reason}");
                        eprintln!("  -> Check logs: .roko/roko.log");
                        eprintln!(
                            "  -> Resume:     roko plan run <dir> --engine runner-v2 --resume-plan"
                        );
                        eprintln!("  -> Diagnose:   roko doctor");
                    }
                    Some(roko_core::WorkflowOutcome::Cancelled) => {
                        eprintln!("error: workflow cancelled");
                        eprintln!(
                            "  -> Resume:     roko plan run <dir> --engine runner-v2 --resume-plan"
                        );
                    }
                    Some(roko_core::WorkflowOutcome::Success { .. }) | None => {
                        eprintln!("error: workflow failed");
                        eprintln!("  -> Check logs: .roko/roko.log");
                        eprintln!("  -> Diagnose:   roko doctor");
                    }
                }
            }

            if share
                && let Err(err) = roko_cli::run::write_shared_workflow_run(
                    &workdir,
                    &prompt,
                    &config.agent.command,
                    &config.prompt.role,
                    &report,
                )
                && !cli.quiet
            {
                eprintln!("share failed: {err}");
            }

            if report.success {
                Ok(EXIT_SUCCESS)
            } else {
                Ok(EXIT_AGENT_FAILURE)
            }
        }
        Err(e) => {
            if !cli.quiet {
                eprintln!("workflow engine error: {e:#}");
            }
            Ok(EXIT_AGENT_FAILURE)
        }
    }
}

pub(crate) async fn cmd_status(
    cli: &Cli,
    workdir: Option<PathBuf>,
    quick: bool,
    cfactor: bool,
    surfaces: bool,
) -> Result<i32> {
    let workdir = workdir.unwrap_or_else(|| resolve_workdir(cli));

    // --quick: compact 3-line health summary, no substrate I/O.
    if quick {
        let auth = roko_cli::auth_detect::detect_auth_from_config(&workdir);
        let provider_line = match &auth {
            roko_cli::auth_detect::AuthMethod::NeedsSetup => {
                "provider:   NONE — run `roko config providers available`".to_string()
            }
            other => format!("provider:   {}", other.label()),
        };
        let learn_line = if workdir.join(".roko/learn/cascade-router.json").exists() {
            "learning:   active (cascade router data present)"
        } else {
            "learning:   no data yet"
        };
        let workspace_line = if workdir.join("roko.toml").exists() {
            format!("workspace:  {}", workdir.display())
        } else {
            "workspace:  no roko.toml — run `roko init`".to_string()
        };
        println!("{provider_line}");
        println!("{learn_line}");
        println!("{workspace_line}");

        let healthy = !matches!(auth, roko_cli::auth_detect::AuthMethod::NeedsSetup)
            && workdir.join("roko.toml").exists();
        return Ok(if healthy { EXIT_SUCCESS } else { EXIT_FAILURE });
    }

    if surfaces {
        let inventory = roko_cli::surface_inventory::full_inventory();
        roko_cli::surface_inventory::print_table(&inventory, cli.json);
        return Ok(EXIT_SUCCESS);
    }

    if !cli.quiet {
        tracing::info!(
            workdir = %workdir.display(),
            json = cli.json,
            cfactor,
            "collecting status snapshot"
        );
    }
    let substrate = FileSubstrate::open(workdir.join(".roko"))
        .await
        .map_err(|e| anyhow!("open substrate: {e}"))?;
    let ctx = Context::now();

    let all = substrate
        .query(&Query::all(), &ctx)
        .await
        .map_err(|e| anyhow!("query: {e}"))?;

    let cfactor_snapshot = if cfactor {
        Some(
            refresh_cfactor_snapshot(workdir.join(".roko").join("learn"))
                .await
                .map_err(|e| anyhow!("refresh c-factor snapshot: {e}"))?,
        )
    } else {
        None
    };
    let cfactor_history = if cfactor_snapshot.is_some() {
        crate::commands::dashboard::load_cfactor_history(
            workdir.join(".roko").join("learn").join("c-factor.jsonl"),
        )
        .await
    } else {
        Vec::new()
    };
    let cfactor_trend = if cfactor_snapshot.is_some() {
        cfactor_trend_arrow(&cfactor_history, Duration::from_hours(168))
    } else {
        "→"
    };
    let learn_dir = workdir.join(".roko").join("learn");
    let costs_path = learn_dir.join("costs.jsonl");
    let costs_log = CostsLog::at(&costs_path);
    let mut cost_diagnostics: Vec<StatusDiagnostic> = Vec::new();
    let total_cost_usd = match costs_log.total_cost().await {
        Ok(v) => Some(v),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => {
            cost_diagnostics.push(StatusDiagnostic {
                source: "costs".into(),
                message: format!("could not read {}: {e}", costs_path.display()),
            });
            None
        }
    };
    let today_cost_usd = costs_log
        .daily_cost(1)
        .await
        .ok()
        .and_then(|days| days.last().map(|(_, cost)| *cost));
    let cost_by_model = costs_log.cost_by_model().await.unwrap_or_default();
    let cost_by_plan = costs_log.cost_by_plan().await.unwrap_or_default();

    if cli.json {
        let mut counts: BTreeMap<String, usize> = BTreeMap::new();
        for sig in &all {
            *counts.entry(sig.kind.to_string()).or_default() += 1;
        }
        let runner_event_status = read_runner_event_status(&workdir);
        let substrate_episode_count = counts.get("episode").copied().unwrap_or(0);
        let file_episode_status = read_file_episode_status(&workdir);
        let episode_count = if substrate_episode_count == 0 {
            file_episode_status
                .as_ref()
                .map_or(0, |status| status.count)
        } else {
            substrate_episode_count
        };

        // Verify verdicts from substrate.
        let verdicts_json = substrate
            .query(&Query::of_kind(Kind::GateVerdict), &ctx)
            .await
            .map_err(|e| anyhow!("query verdicts: {e}"))?;
        let mut gate_pass = verdicts_json
            .iter()
            .filter(|v| v.tag("passed") == Some("true"))
            .count();
        let mut gate_fail = verdicts_json
            .iter()
            .filter(|v| v.tag("passed") == Some("false"))
            .count();
        // Most recent episode.
        let mut episodes_json = substrate
            .query(&Query::of_kind(Kind::Episode), &ctx)
            .await
            .map_err(|e| anyhow!("query episodes: {e}"))?;
        episodes_json.sort_by_key(|s| std::cmp::Reverse(s.created_at_ms));
        let runner_is_latest = runner_event_status.as_ref().is_some_and(|runner| {
            runner.newer_than(latest_episode_timestamp(
                &episodes_json,
                file_episode_status.as_ref(),
            ))
        });
        if runner_is_latest
            && runner_event_status
                .as_ref()
                .is_some_and(|runner| runner.has_gates())
        {
            if let Some(runner) = runner_event_status.as_ref() {
                gate_pass = runner.gate_pass;
                gate_fail = runner.gate_fail;
            }
        } else if gate_pass == 0 && gate_fail == 0 {
            let fallback = read_gate_result_counts(&learn_dir);
            gate_pass = fallback.0;
            gate_fail = fallback.1;
            if gate_pass == 0
                && gate_fail == 0
                && let Some(runner) = runner_event_status
                    .as_ref()
                    .filter(|runner| runner.has_gates())
            {
                gate_pass = runner.gate_pass;
                gate_fail = runner.gate_fail;
            }
        }

        // Running agents from runtime directory.
        let runtime_dir_json = workdir.join(".roko").join("runtime");
        let mut running_agents_json: usize = 0;
        if let Ok(mut entries) = tokio::fs::read_dir(&runtime_dir_json).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str.ends_with(".pid") {
                    running_agents_json += 1;
                }
            }
        }

        let executor_state = crate::commands::plan::read_executor_state(&workdir);
        let state_entries = executor_state.clone().unwrap_or_default();
        let active_plans_json: usize = state_entries.len();
        let run_state_found = executor_state.is_some();

        let last_passed = episodes_json
            .first()
            .and_then(|ep| ep.tag("passed").map(|v| v == "true"))
            .or_else(|| {
                file_episode_status
                    .as_ref()
                    .and_then(|status| status.passed)
            });
        let last_passed = if runner_is_latest {
            runner_event_status
                .as_ref()
                .and_then(|status| status.passed)
                .or(last_passed)
        } else {
            last_passed
        };

        // Use the canonical collector for daemon/process/runner fields.
        let mut status = collect_session_status(&workdir);
        if let Some(resume_id) = &cli.resume {
            status.session_id = Some(resume_id.clone());
        }
        status.signal_count = Some(all.len());
        status.episode_count = Some(episode_count);
        status.last_episode_passed = last_passed;
        status.cfactor = cfactor_snapshot;
        status.total_cost_usd = total_cost_usd;
        status.today_cost_usd = today_cost_usd;
        status.diagnostics.extend(cost_diagnostics.iter().cloned());

        // Build enriched JSON with gate verdicts, workspace info, and signal counts.
        let counts_json = serde_json::to_string(&counts).unwrap_or_else(|_| "{}".to_string());
        let cost_by_model_json =
            serde_json::to_string(&cost_by_model).unwrap_or_else(|_| "{}".to_string());
        let cost_by_plan_json =
            serde_json::to_string(&cost_by_plan).unwrap_or_else(|_| "{}".to_string());
        let runner_json =
            serde_json::to_string(&runner_event_status.as_ref().map(RunnerEventStatus::json))
                .unwrap_or_else(|_| "null".to_string());
        let base = status.display_json();
        // Splice additional fields before the closing brace.
        let enriched = format!(
            "{},\"gates\":{{\"pass\":{gate_pass},\"fail\":{gate_fail}}},\"workspace\":{{\"agents\":{running_agents_json},\"plans\":{active_plans_json},\"run_state_found\":{run_state_found}}},\"signal_counts\":{counts_json},\"cost_by_model\":{cost_by_model_json},\"cost_by_plan\":{cost_by_plan_json},\"runner\":{runner_json},\"health\":\"ready\"}}",
            &base[..base.len() - 1],
        );
        println!("{enriched}");

        for (plan_id, _, _) in &state_entries {
            if !crate::commands::plan::plan_path_exists(&workdir, plan_id) {
                eprintln!(
                    "warning: state references missing plan: {plan_id} (not found in plans/ or .roko/plans/)"
                );
            }
        }
        return Ok(EXIT_SUCCESS);
    }

    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for sig in &all {
        *counts.entry(sig.kind.to_string()).or_default() += 1;
    }

    println!("signal counts ({} total):", all.len());
    if counts.is_empty() {
        println!("  (empty)");
    } else {
        for (kind, n) in &counts {
            println!("  {kind:<24} {n}");
        }
    }

    // Running agents from runtime directory.
    let runtime_dir = workdir.join(".roko").join("runtime");
    let mut running_agents: usize = 0;
    if let Ok(mut entries) = tokio::fs::read_dir(&runtime_dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.ends_with(".pid") {
                running_agents += 1;
            }
        }
    }

    let executor_state = crate::commands::plan::read_executor_state(&workdir);
    let state_entries = executor_state.clone().unwrap_or_default();
    let active_plans: usize = state_entries.len();

    println!();
    if executor_state.is_some() {
        println!(
            "workspace: {} agent pid(s), {} plan(s) in executor snapshot",
            running_agents, active_plans
        );
        for (plan_id, tasks_done, tasks_total) in &state_entries {
            println!("  plan {plan_id}: {tasks_done}/{tasks_total} tasks completed");
        }
        for (plan_id, _, _) in &state_entries {
            if !crate::commands::plan::plan_path_exists(&workdir, plan_id) {
                println!(
                    "  warning: state references missing plan: {plan_id} (not found in plans/ or .roko/plans/)"
                );
            }
        }
    } else {
        println!(
            "workspace: {} agent pid(s), no run state found",
            running_agents
        );
    }

    let mut episodes = substrate
        .query(&Query::of_kind(Kind::Episode), &ctx)
        .await
        .map_err(|e| anyhow!("query episodes: {e}"))?;
    episodes.sort_by_key(|s| std::cmp::Reverse(s.created_at_ms));
    let runner_event_status = read_runner_event_status(&workdir);
    let file_episode_status = read_file_episode_status(&workdir);
    let runner_is_latest = runner_event_status.as_ref().is_some_and(|runner| {
        runner.newer_than(latest_episode_timestamp(
            &episodes,
            file_episode_status.as_ref(),
        ))
    });
    println!();
    if runner_is_latest {
        if let Some(runner) = runner_event_status.as_ref() {
            println!(
                "most recent run: {} (passed={})",
                runner.display_id(),
                runner
                    .passed
                    .map_or_else(|| "?".to_string(), |passed| passed.to_string())
            );
            if let (Some(done), Some(total)) = (runner.tasks_completed, runner.total_tasks) {
                println!(
                    "  tasks completed={} total={} failed={}",
                    done,
                    total,
                    runner.tasks_failed.unwrap_or(0)
                );
            }
        }
    } else {
        match episodes.first() {
            Some(ep) => {
                println!(
                    "most recent episode: {} (passed={})",
                    ep.id,
                    ep.tag("passed").unwrap_or("?")
                );
                println!(
                    "  gates passed={} failed={}",
                    ep.tag("gates_passed").unwrap_or("0"),
                    ep.tag("gates_failed").unwrap_or("0")
                );
            }
            None => match &file_episode_status {
                Some(status) => println!(
                    "most recent episode: {} (passed={})",
                    status.id.as_deref().unwrap_or("?"),
                    status
                        .passed
                        .map_or_else(|| "?".to_string(), |passed| passed.to_string())
                ),
                None => println!("most recent episode: (none)"),
            },
        }
    }

    let verdicts = substrate
        .query(&Query::of_kind(Kind::GateVerdict), &ctx)
        .await
        .map_err(|e| anyhow!("query verdicts: {e}"))?;
    let mut passed = verdicts
        .iter()
        .filter(|v| v.tag("passed") == Some("true"))
        .count();
    let mut failed = verdicts
        .iter()
        .filter(|v| v.tag("passed") == Some("false"))
        .count();
    if runner_is_latest
        && runner_event_status
            .as_ref()
            .is_some_and(|runner| runner.has_gates())
    {
        if let Some(runner) = runner_event_status.as_ref() {
            passed = runner.gate_pass;
            failed = runner.gate_fail;
        }
    } else if passed == 0 && failed == 0 {
        let fallback = read_gate_result_counts(&learn_dir);
        passed = fallback.0;
        failed = fallback.1;
        if passed == 0
            && failed == 0
            && let Some(runner) = runner_event_status
                .as_ref()
                .filter(|runner| runner.has_gates())
        {
            passed = runner.gate_pass;
            failed = runner.gate_fail;
        }
    }
    println!("gate verdicts: {passed} pass / {failed} fail");

    // Learning subsystem stats.
    let efficiency_path = learn_dir.join("efficiency.jsonl");
    match read_efficiency_events(&efficiency_path).await {
        Ok(events) if !events.is_empty() => {
            println!();
            println!("efficiency events: {} total", events.len());
            let profiles = compute_role_profiles(&events);
            for p in &profiles {
                println!(
                    "  {:<16} avg_cost=${:.4}  p95_cost=${:.4}  pass_rate={:.0}%  n={}",
                    p.role,
                    p.avg_cost_usd.max(0.0),
                    p.p95_cost_usd.max(0.0),
                    p.pass_rate * 100.0,
                    p.observations,
                );
            }
        }
        _ => {}
    }

    // Experiment store summary.
    let experiments_path = learn_dir.join("experiments.json");
    let exp_store = ExperimentStore::load_or_new(&experiments_path);
    let running = exp_store.running_count();
    let concluded = exp_store.concluded_count();
    if running > 0 || concluded > 0 {
        println!();
        println!("prompt experiments: {running} running, {concluded} concluded");
    }

    // Adaptive threshold summary.
    let thresholds_path = learn_dir.join("gate-thresholds.json");
    let thresholds =
        roko_gate::adaptive_threshold::AdaptiveThresholds::load_or_new(&thresholds_path);
    let rung_count: usize = thresholds.all_rungs().count();
    if rung_count > 0 {
        println!();
        println!("adaptive gate thresholds: {rung_count} rungs tracked");
        for (rung, stats) in thresholds.all_rungs() {
            println!(
                "  rung {rung}: pass_rate={:.0}% retries={} obs={} skip={}",
                stats.ema_pass_rate * 100.0,
                thresholds.suggested_max_retries(*rung),
                stats.total_observations,
                if thresholds.should_skip_rung(*rung) {
                    "yes"
                } else {
                    "no"
                },
            );
        }
    }

    if total_cost_usd.is_some() || !cost_by_model.is_empty() || !cost_by_plan.is_empty() {
        println!();
        println!("Cost Summary:");
        if let Some(total_cost_usd) = total_cost_usd {
            println!("  Total:    ${:.4}", total_cost_usd.max(0.0));
        }
        if let Some(today_cost_usd) = today_cost_usd {
            println!("  Today:    ${:.4}", today_cost_usd.max(0.0));
        }
        if !cost_by_model.is_empty() {
            println!("  By model: {}", format_cost_breakdown(&cost_by_model, 5));
        }
        if !cost_by_plan.is_empty() {
            println!("  By plan:  {}", format_cost_breakdown(&cost_by_plan, 5));
        }
    }

    // Health probes — quick snapshot of orchestrator readiness.
    let health_probes = roko_core::obs::health::ProbeRegistry::new();
    health_probes.register(std::sync::Arc::new(
        roko_core::obs::health::AlwaysUpProbe::new("orchestrator"),
    ));
    let (readiness_status, degraded_reasons) = health_probes.readiness();
    println!();
    println!("health: {readiness_status}");
    if !degraded_reasons.is_empty() {
        for reason in &degraded_reasons {
            println!("  {} — {}", reason.component, reason.message);
        }
    }

    if let Some(cfactor) = cfactor_snapshot {
        println!();
        println!(
            "c-factor: {:.3} | trend={} | episodes={} | computed={}",
            cfactor.overall, cfactor_trend, cfactor.episode_count, cfactor.computed_at
        );
        println!(
            "  gate={:.3} cost={:.3} speed={:.3} flow={:.3} first_try={:.3} knowledge={:.3} integration={:.3} convergence={:.3} turn={:.3} social={:.3}",
            cfactor.components.gate_pass_rate,
            cfactor.components.cost_efficiency,
            cfactor.components.speed,
            cfactor.components.information_flow_rate,
            cfactor.components.first_try_rate,
            cfactor.components.knowledge_growth,
            cfactor.components.knowledge_integration_rate,
            cfactor.components.convergence_velocity,
            cfactor.components.turn_taking_equality,
            cfactor.components.social_perceptiveness
        );
        if !cfactor.agent_contributions.is_empty() {
            println!(
                "  agent contributions: {}",
                cfactor.top_agent_contribution_lines(3).join(", ")
            );
        }
    }

    Ok(EXIT_SUCCESS)
}

struct FileEpisodeStatus {
    count: usize,
    id: Option<String>,
    passed: Option<bool>,
    timestamp_ms: Option<u64>,
}

fn read_file_episode_status(workdir: &Path) -> Option<FileEpisodeStatus> {
    let roko_dir = workdir.join(".roko");
    for path in [
        roko_dir.join("episodes.jsonl"),
        roko_dir.join("learn").join("episodes.jsonl"),
        roko_dir.join("memory").join("episodes.jsonl"),
    ] {
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        let mut count = 0;
        let mut id = None;
        let mut passed = None;
        let mut timestamp_ms = None;
        for line in text.lines().filter(|line| !line.trim().is_empty()) {
            count += 1;
            let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
                continue;
            };
            id = value
                .get("id")
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned)
                .or(id);
            passed = value
                .get("success")
                .and_then(serde_json::Value::as_bool)
                .or_else(|| value.get("passed").and_then(serde_json::Value::as_bool))
                .or(passed);
            timestamp_ms = json_timestamp_ms(&value).or(timestamp_ms);
        }
        if count > 0 {
            return Some(FileEpisodeStatus {
                count,
                id,
                passed,
                timestamp_ms,
            });
        }
    }
    None
}

struct RunnerEventStatus {
    run_id: Option<String>,
    timestamp_ms: u64,
    passed: Option<bool>,
    total_tasks: Option<usize>,
    tasks_completed: Option<usize>,
    tasks_failed: Option<usize>,
    gate_pass: usize,
    gate_fail: usize,
}

impl RunnerEventStatus {
    fn display_id(&self) -> &str {
        self.run_id.as_deref().unwrap_or("?")
    }

    const fn has_gates(&self) -> bool {
        self.gate_pass > 0 || self.gate_fail > 0
    }

    fn newer_than(&self, timestamp_ms: Option<u64>) -> bool {
        timestamp_ms.is_none_or(|timestamp_ms| self.timestamp_ms >= timestamp_ms)
    }

    fn json(&self) -> serde_json::Value {
        serde_json::json!({
            "run_id": self.run_id,
            "timestamp_ms": self.timestamp_ms,
            "passed": self.passed,
            "total_tasks": self.total_tasks,
            "tasks_completed": self.tasks_completed,
            "tasks_failed": self.tasks_failed,
            "gates": {
                "pass": self.gate_pass,
                "fail": self.gate_fail,
            },
        })
    }
}

fn latest_episode_timestamp(
    substrate_episodes: &[roko_core::Signal],
    file_episode_status: Option<&FileEpisodeStatus>,
) -> Option<u64> {
    substrate_episodes
        .first()
        .and_then(|episode| u64::try_from(episode.created_at_ms).ok())
        .or_else(|| file_episode_status.and_then(|status| status.timestamp_ms))
}

fn read_runner_event_status(workdir: &Path) -> Option<RunnerEventStatus> {
    let text = std::fs::read_to_string(workdir.join(".roko").join("events.jsonl")).ok()?;
    let events: Vec<serde_json::Value> = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .collect();

    let latest = events
        .iter()
        .filter(|value| {
            matches!(
                value.get("type").and_then(serde_json::Value::as_str),
                Some("run.completed" | "run.started")
            )
        })
        .filter_map(|value| json_timestamp_ms(value).map(|timestamp_ms| (timestamp_ms, value)))
        .max_by_key(|(timestamp_ms, _)| *timestamp_ms)?;

    let latest_value = latest.1;
    let run_id = latest_value
        .get("run_id")
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned);

    let mut gate_pass = 0;
    let mut gate_fail = 0;
    for event in &events {
        if event.get("type").and_then(serde_json::Value::as_str) != Some("gate.completed") {
            continue;
        }
        if event.get("run_id").and_then(serde_json::Value::as_str) != run_id.as_deref() {
            continue;
        }
        if let Some(verdicts) = event.get("verdicts").and_then(serde_json::Value::as_array) {
            for verdict in verdicts {
                match verdict.get("passed").and_then(serde_json::Value::as_bool) {
                    Some(true) => gate_pass += 1,
                    Some(false) => gate_fail += 1,
                    None => {}
                }
            }
        } else {
            match event.get("passed").and_then(serde_json::Value::as_bool) {
                Some(true) => gate_pass += 1,
                Some(false) => gate_fail += 1,
                None => {}
            }
        }
    }

    Some(RunnerEventStatus {
        run_id,
        timestamp_ms: latest.0,
        passed: latest_value
            .get("outcome")
            .and_then(serde_json::Value::as_str)
            .map(|outcome| outcome == "succeeded"),
        total_tasks: json_usize(latest_value, "total_tasks"),
        tasks_completed: json_usize(latest_value, "tasks_completed"),
        tasks_failed: json_usize(latest_value, "tasks_failed"),
        gate_pass,
        gate_fail,
    })
}

fn json_usize(value: &serde_json::Value, key: &str) -> Option<usize> {
    value
        .get(key)
        .and_then(serde_json::Value::as_u64)
        .and_then(|n| usize::try_from(n).ok())
}

fn json_timestamp_ms(value: &serde_json::Value) -> Option<u64> {
    value
        .get("timestamp_ms")
        .and_then(serde_json::Value::as_u64)
        .or_else(|| {
            value
                .get("created_at_ms")
                .and_then(serde_json::Value::as_u64)
        })
}

fn read_gate_result_counts(learn_dir: &Path) -> (usize, usize) {
    let Ok(text) = std::fs::read_to_string(learn_dir.join("efficiency.jsonl")) else {
        return (0, 0);
    };
    let mut passed = 0;
    let mut failed = 0;
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if value.get("kind").and_then(serde_json::Value::as_str) != Some("gate_result") {
            continue;
        }
        match value.get("passed").and_then(serde_json::Value::as_bool) {
            Some(true) => passed += 1,
            Some(false) => failed += 1,
            None => {}
        }
    }
    (passed, failed)
}

pub(crate) async fn cmd_doctor(
    cli: &Cli,
    subject: Option<DoctorSubject>,
    workdir: Option<PathBuf>,
    serve_url: Option<String>,
) -> Result<i32> {
    let workdir = workdir.unwrap_or_else(|| resolve_workdir(cli));
    if matches!(subject, Some(DoctorSubject::Disk)) {
        let report = roko_cli::doctor::run_disk_doctor(&workdir, cli.config.as_deref()).await;
        if cli.json {
            println!("{}", serde_json::to_string_pretty(&report)?);
        } else {
            print!("{}", report.render_human());
        }
        return Ok(report.exit_code());
    }
    if matches!(subject, Some(DoctorSubject::Network)) {
        let report = roko_cli::doctor::run_network_doctor(roko_cli::doctor::NetworkDoctorOptions {
            workdir,
            config_override: cli.config.clone(),
            probe_timeout: roko_cli::doctor::DEFAULT_NETWORK_PROBE_TIMEOUT,
        })
        .await;
        if cli.json {
            println!("{}", serde_json::to_string_pretty(&report)?);
        } else {
            print!("{}", report.render_human());
        }
        return Ok(report.exit_code());
    }

    let report = roko_cli::doctor::run_doctor(&roko_cli::doctor::DoctorOptions {
        workdir,
        config_override: cli.config.clone(),
        serve_url,
    })
    .await?;

    if cli.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print!("{}", report.render_human());
    }

    Ok(report.exit_code())
}

pub(crate) fn format_cost_breakdown(costs: &HashMap<String, f64>, limit: usize) -> String {
    let mut entries = costs
        .iter()
        .map(|(name, cost)| (name.as_str(), *cost))
        .collect::<Vec<_>>();
    entries.sort_by(|(left_name, left_cost), (right_name, right_cost)| {
        right_cost
            .total_cmp(left_cost)
            .then_with(|| left_name.cmp(right_name))
    });
    entries.truncate(limit);
    if entries.is_empty() {
        return "none".to_string();
    }

    entries
        .into_iter()
        .map(|(name, cost)| format!("{name}=${:.4}", cost.max(0.0)))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) async fn cmd_replay(
    workdir: Option<PathBuf>,
    hash: String,
    forensic: bool,
    as_of: Option<String>,
    format: String,
) -> Result<i32> {
    let workdir = workdir.unwrap_or_else(|| PathBuf::from("."));
    let substrate = FileSubstrate::open(workdir.join(".roko"))
        .await
        .map_err(|e| anyhow!("open substrate: {e}"))?;
    let start = ContentHash::from_hex(&hash)
        .ok_or_else(|| anyhow!("invalid hash (expected 64 hex chars): {hash}"))?;

    // Parse --as-of filter: skip signals until this depth/index.
    let skip_until: usize = as_of
        .as_deref()
        .and_then(|s| {
            // Accept "step 5", "step05", "5", "#5"
            let stripped = s.trim_start_matches("step").trim_start_matches('#').trim();
            stripped.parse().ok()
        })
        .unwrap_or(0);

    let is_json = format == "json";

    let mut visited = std::collections::HashSet::new();
    let mut queue = vec![(start, 0usize)];
    let mut printed = 0usize;
    let mut index = 0usize;

    while let Some((id, depth)) = queue.pop() {
        if !visited.insert(id) {
            continue;
        }
        if let Some(sig) = substrate.get(&id).await.map_err(|e| anyhow!("get: {e}"))? {
            index += 1;

            // Apply --as-of filter: skip events before the target index.
            if index < skip_until {
                for parent in &sig.lineage {
                    queue.push((*parent, depth + 1));
                }
                continue;
            }

            if is_json {
                // JSON output: one JSON object per line.
                let mut obj = serde_json::Map::new();
                obj.insert("event".into(), serde_json::json!(index));
                obj.insert("hash".into(), serde_json::json!(sig.id.to_string()));
                obj.insert("kind".into(), serde_json::json!(sig.kind.to_string()));
                obj.insert("author".into(), serde_json::json!(sig.provenance.author));
                obj.insert("created_at_ms".into(), serde_json::json!(sig.created_at_ms));
                if !sig.tags.is_empty() {
                    obj.insert("tags".into(), serde_json::json!(sig.tags));
                }
                if let Ok(text) = sig.body.as_text() {
                    let preview: String = text.chars().take(500).collect();
                    obj.insert("body".into(), serde_json::json!(preview));
                }
                println!("{}", serde_json::Value::Object(obj));
            } else if forensic {
                let indent = "  ".repeat(depth);
                println!("{indent}{} {}", sig.kind, sig.id);
                println!("{indent}  event:     {index}");
                println!("{indent}  hash:      {}", sig.id);
                println!("{indent}  author:    {}", sig.provenance.author);
                println!("{indent}  created:   {}", sig.created_at_ms);
                println!(
                    "{indent}  lineage:   [{}]",
                    sig.lineage
                        .iter()
                        .map(|h| h.to_string())
                        .collect::<Vec<_>>()
                        .join(", "),
                );
                if !sig.tags.is_empty() {
                    println!("{indent}  tags:      {:?}", sig.tags);
                }
                if let Ok(text) = sig.body.as_text() {
                    let body_preview: String = text.chars().take(120).collect();
                    println!("{indent}  body:      {body_preview}");
                }
                println!();
            } else {
                let indent = "  ".repeat(depth);
                println!(
                    "{indent}{} {}  (event={index}, author={})",
                    sig.kind, sig.id, sig.provenance.author
                );
            }
            for parent in &sig.lineage {
                queue.push((*parent, depth + 1));
            }
            printed += 1;
        } else if !is_json {
            let indent = "  ".repeat(depth);
            println!("{indent}<missing {id}>");
        }
    }
    if printed == 0 {
        if !is_json {
            println!("signal {hash} not found in substrate");
        }
        return Ok(EXIT_AGENT_FAILURE);
    }
    Ok(EXIT_SUCCESS)
}

pub(crate) fn cmd_inject(
    cli: &Cli,
    session: String,
    kind_str: &str,
    payload: String,
    workdir: Option<PathBuf>,
) -> Result<i32> {
    let kind = InjectKind::parse(kind_str).map_err(|e| anyhow!("{e}"))?;
    let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
    let request = InjectRequest::new(session, kind, payload, wd);

    // Validation errors (empty session, empty payload for directive/context) remain
    // more specific than the transport-unavailable error below.
    request.validate().map_err(|e| anyhow!("{e}"))?;

    // No delivery backend exists yet (#361 owns the transport).
    // Fail closed: never report success when nothing was delivered.
    let code = "inject_transport_unavailable";
    let message = "inject transport unavailable — no delivery backend is configured";
    let hint =
        "No live command transport is installed; use plan pause/cancel controls where applicable.";

    if cli.json {
        println!(
            r#"{{"code":"{code}","message":"{message}","hint":"{hint}"}}"#,
        );
    } else {
        eprintln!("Error: {message}");
        eprintln!("Hint: {hint}");
    }

    Ok(EXIT_FAILURE)
}

pub(crate) fn cmd_index(cli: &Cli, cmd: IndexCmd) -> Result<i32> {
    let workdir = resolve_workdir(cli);
    match cmd {
        IndexCmd::Build { path } => {
            let target = path.unwrap_or_else(|| workdir.clone());
            let start = Instant::now();
            let idx = roko_index::WorkspaceIndex::load(&target)
                .with_context(|| format!("build index for {}", target.display()))?;
            let elapsed = start.elapsed();
            let stats = idx.stats();
            println!("Index built in {:.2}s", elapsed.as_secs_f64());
            println!("  Files:   {}", stats.indexed_files);
            println!("  Symbols: {}", stats.total_symbols);
            println!("  Edges:   {}", stats.total_edges);
            for (lang, count) in &stats.languages {
                println!("  {lang}: {count} files");
            }
            Ok(EXIT_SUCCESS)
        }
        IndexCmd::Rebuild { path } => {
            let target = path.unwrap_or_else(|| workdir.clone());
            // Remove the existing index database if present.
            let db_path = target.join(".roko").join("index.db");
            if db_path.exists() {
                std::fs::remove_file(&db_path)
                    .with_context(|| format!("remove old index at {}", db_path.display()))?;
                println!("Removed old index: {}", db_path.display());
            }
            // Rebuild from scratch.
            let start = Instant::now();
            let idx = roko_index::WorkspaceIndex::load(&target)
                .with_context(|| format!("rebuild index for {}", target.display()))?;
            let elapsed = start.elapsed();
            let stats = idx.stats();
            println!("Index rebuilt in {:.2}s", elapsed.as_secs_f64());
            println!("  Files:   {}", stats.indexed_files);
            println!("  Symbols: {}", stats.total_symbols);
            println!("  Edges:   {}", stats.total_edges);
            for (lang, count) in &stats.languages {
                println!("  {lang}: {count} files");
            }
            Ok(EXIT_SUCCESS)
        }
        IndexCmd::Search {
            query,
            kind,
            strategy,
            file_pattern,
            limit,
            path,
        } => {
            let target = path.unwrap_or_else(|| workdir.clone());
            let idx = roko_index::WorkspaceIndex::load(&target)
                .with_context(|| format!("build index for {}", target.display()))?;

            let sym_kind = if let Some(ref k) = kind {
                Some(parse_symbol_kind(k)?)
            } else {
                None
            };

            let index_query = roko_index::IndexQuery {
                strategy,
                query: query.clone(),
                kind: sym_kind,
                file_pattern,
                limit,
            };

            let results = index_query.execute(&idx)?;
            if results.is_empty() {
                println!("No results found for \"{query}\"");
            } else {
                println!("{:<50} {:<10} {:<6} {:<8}", "NAME", "KIND", "LINE", "SCORE");
                println!("{}", "-".repeat(76));
                for r in &results {
                    println!(
                        "{:<50} {:<10} {:<6} {:.4}",
                        r.symbol.id.symbol_name,
                        format!("{:?}", r.symbol.id.kind),
                        r.symbol.line,
                        r.score,
                    );
                }
                println!("\n{} result(s)", results.len());
            }
            Ok(EXIT_SUCCESS)
        }
        IndexCmd::Stats { path } => {
            let target = path.unwrap_or_else(|| workdir.clone());
            let idx = roko_index::WorkspaceIndex::load(&target)
                .with_context(|| format!("build index for {}", target.display()))?;
            let stats = idx.stats();

            println!("=== Index Statistics ===\n");
            println!("Files indexed:  {}", stats.indexed_files);
            println!("Total symbols:  {}", stats.total_symbols);
            println!("Total edges:    {}", stats.total_edges);

            println!("\nEdge breakdown:");
            for (kind, count) in &stats.edge_breakdown {
                println!("  {kind}: {count}");
            }

            println!("\nLanguages:");
            for (lang, count) in &stats.languages {
                println!("  {lang}: {count} files");
            }

            if !stats.top_symbols_by_pagerank.is_empty() {
                println!("\nTop-10 symbols by PageRank:");
                println!("{:<50} {:<10} {:<8}", "NAME", "KIND", "SCORE");
                println!("{}", "-".repeat(70));
                for r in &stats.top_symbols_by_pagerank {
                    println!(
                        "{:<50} {:<10} {:.6}",
                        r.symbol.id.symbol_name,
                        format!("{:?}", r.symbol.id.kind),
                        r.score,
                    );
                }
            }
            Ok(EXIT_SUCCESS)
        }
    }
}

pub(crate) fn parse_symbol_kind(s: &str) -> Result<roko_core::language::SymbolKind> {
    use roko_core::language::SymbolKind;
    match s.to_lowercase().as_str() {
        "function" | "fn" => Ok(SymbolKind::Function),
        "struct" => Ok(SymbolKind::Struct),
        "enum" => Ok(SymbolKind::Enum),
        "trait" => Ok(SymbolKind::Trait),
        "const" => Ok(SymbolKind::Const),
        "type" => Ok(SymbolKind::Type),
        "module" | "mod" => Ok(SymbolKind::Module),
        "impl" => Ok(SymbolKind::Impl),
        other => bail!(
            "unknown symbol kind: {other} (expected function, struct, enum, trait, const, type, module, impl)"
        ),
    }
}

// ---------------------------------------------------------------------------
// Recursive shell completion engine (#332)
// ---------------------------------------------------------------------------

/// A node in the recursive command tree used by completion generators.
#[derive(Debug, Clone)]
pub(crate) struct CompletionNode {
    pub name: String,
    pub children: Vec<CompletionNode>,
    /// Long flags (e.g. `--workdir`).
    pub long_flags: Vec<String>,
    /// Short flags (e.g. `-q`).
    pub short_flags: Vec<String>,
    /// Value-enum candidates keyed by flag name.
    pub value_enums: Vec<(String, Vec<String>)>,
}

/// Build the full recursive completion tree from clap metadata.
pub(crate) fn build_completion_tree() -> CompletionNode {
    let mut command = Cli::command();
    command.build();
    build_node(&command)
}

fn build_node(cmd: &clap::Command) -> CompletionNode {
    let children: Vec<CompletionNode> = cmd
        .get_subcommands()
        .filter(|s| !s.is_hide_set())
        .map(build_node)
        .collect();

    let mut long_flags = Vec::new();
    let mut short_flags = Vec::new();
    let mut value_enums = Vec::new();

    for arg in cmd.get_arguments() {
        if let Some(l) = arg.get_long() {
            long_flags.push(format!("--{l}"));
        }
        if let Some(s) = arg.get_short() {
            short_flags.push(format!("-{s}"));
        }
        let possible: Vec<String> = arg
            .get_possible_values()
            .iter()
            .filter(|v| !v.is_hide_set())
            .map(|v| v.get_name().to_string())
            .collect();
        if !possible.is_empty() {
            let flag_name = arg
                .get_long()
                .map(|l| format!("--{l}"))
                .unwrap_or_else(|| arg.get_id().to_string());
            value_enums.push((flag_name, possible));
        }
    }

    long_flags.sort();
    long_flags.dedup();
    short_flags.sort();
    short_flags.dedup();

    CompletionNode {
        name: cmd.get_name().to_string(),
        children,
        long_flags,
        short_flags,
        value_enums,
    }
}

/// Walk the tree to find the node matching a command path.
fn resolve_node<'a>(root: &'a CompletionNode, path: &[&str]) -> Option<&'a CompletionNode> {
    let mut node = root;
    for segment in path {
        node = node.children.iter().find(|c| c.name == *segment)?;
    }
    Some(node)
}

/// Scan the filesystem for dynamic completion candidates. Read-only, no network.
pub(crate) fn dynamic_completion_candidates(path: &[&str]) -> Vec<String> {
    match path {
        ["plan", "run" | "show" | "validate"] | ["plan"] => scan_dir_names("plans"),
        ["prd", "plan" | "status"]
        | ["prd", "draft", "edit" | "promote"]
        | ["prd"] => scan_dir_names(".roko/prd"),
        ["agent", ..] => scan_dir_names(".roko/agents"),
        _ => Vec::new(),
    }
}

fn scan_dir_names(dir: &str) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .filter_map(Result::ok)
        .filter_map(|e| {
            let p = e.path();
            if p.is_dir() {
                p.file_name().map(|n| n.to_string_lossy().into_owned())
            } else {
                p.file_stem().map(|n| n.to_string_lossy().into_owned())
            }
        })
        .collect();
    names.sort();
    names.dedup();
    names
}

/// Shell-escape a candidate word for safe embedding in shell scripts.
fn shell_escape(s: &str) -> String {
    if s.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '/')
    {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', "'\\''"))
    }
}

// ---------------------------------------------------------------------------
// Hidden __complete handler — emits newline-delimited candidates
// ---------------------------------------------------------------------------

/// Handle `roko __complete --shell <shell> --path <path> --current <word>`.
pub(crate) fn cmd_complete(path_str: &str, current: &str) {
    let tree = build_completion_tree();
    let path_segments: Vec<&str> = if path_str.is_empty() {
        Vec::new()
    } else {
        path_str.split_whitespace().collect()
    };

    let node = resolve_node(&tree, &path_segments).unwrap_or(&tree);

    let candidates: Vec<String> = if current.starts_with('-') {
        // Offer matching flags from the resolved node.
        node.long_flags
            .iter()
            .chain(node.short_flags.iter())
            .filter(|f| f.starts_with(current))
            .cloned()
            .collect()
    } else {
        // Check if the last path segment is a flag with enum values.
        let flag_values = path_segments
            .last()
            .and_then(|last_seg| {
                if last_seg.starts_with('-') {
                    node.value_enums
                        .iter()
                        .find(|(flag, _)| flag == *last_seg)
                        .map(|(_, vals)| vals.clone())
                } else {
                    None
                }
            })
            .unwrap_or_default();

        if !flag_values.is_empty() {
            flag_values
                .into_iter()
                .filter(|v| v.starts_with(current))
                .collect()
        } else {
            let mut cands: Vec<String> = node
                .children
                .iter()
                .map(|c| c.name.clone())
                .filter(|n| n.starts_with(current))
                .collect();
            let dynamic = dynamic_completion_candidates(&path_segments);
            cands.extend(dynamic.into_iter().filter(|d| d.starts_with(current)));
            cands.sort();
            cands.dedup();
            cands
        }
    };

    for c in &candidates {
        println!("{c}");
    }
}

// ---------------------------------------------------------------------------
// Static completion script generators (bash / zsh / fish)
// ---------------------------------------------------------------------------

pub(crate) fn print_completions(shell: CompletionShell) {
    match shell {
        CompletionShell::Bash => print_bash_completions(),
        CompletionShell::Zsh => print_zsh_completions(),
        CompletionShell::Fish => print_fish_completions(),
    }
}

/// Global flag names for flag completion.
pub(crate) fn completion_flag_words() -> Vec<String> {
    let mut command = Cli::command();
    command.build();
    let mut flags: Vec<String> = command
        .get_arguments()
        .filter_map(|arg| arg.get_long().map(|l| format!("--{l}")))
        .collect();
    flags.sort();
    flags.dedup();
    flags
}

/// Collect all recursive command paths for shell case statements.
fn collect_all_subcommand_paths(
    node: &CompletionNode,
    prefix: &[String],
) -> Vec<(Vec<String>, Vec<String>)> {
    let mut result = Vec::new();
    let child_names: Vec<String> = node.children.iter().map(|c| c.name.clone()).collect();
    if !child_names.is_empty() {
        result.push((prefix.to_vec(), child_names));
    }
    for child in &node.children {
        let mut child_prefix = prefix.to_vec();
        child_prefix.push(child.name.clone());
        result.extend(collect_all_subcommand_paths(child, &child_prefix));
    }
    result
}

fn print_bash_completions() {
    let tree = build_completion_tree();
    let top_names: Vec<String> = tree.children.iter().map(|c| c.name.clone()).collect();
    let all_paths = collect_all_subcommand_paths(&tree, &[]);

    println!("# roko bash completions — recursive + dynamic (#332)");
    println!("_roko()");
    println!("{{");
    println!(r#"    local cur="${{COMP_WORDS[COMP_CWORD]}}""#);
    println!(r#"    local prev="${{COMP_WORDS[COMP_CWORD-1]}}""#);
    println!();
    // Build the command path from COMP_WORDS.
    println!("    # Build command path from COMP_WORDS, skipping roko and flags.");
    println!(r#"    local cmd_path="""#);
    println!("    local i");
    println!("    for (( i=1; i<COMP_CWORD; i++ )); do");
    println!(r#"        case "${{COMP_WORDS[i]}}" in"#);
    println!(r#"            -*) ;;"#);
    println!(r#"            *) cmd_path="$cmd_path ${{COMP_WORDS[i]}}" ;;"#);
    println!("        esac");
    println!("    done");
    println!(r#"    cmd_path="${{cmd_path## }}""#);
    println!();
    // Try dynamic completion via __complete (handles workspace names + arbitrary depth).
    println!(r#"    local candidates"#);
    println!(r#"    candidates="$(roko __complete --shell bash --path "$cmd_path" --current "$cur" 2>/dev/null)""#);
    println!(r#"    if [[ -n "$candidates" ]]; then"#);
    println!(r#"        COMPREPLY=( $(compgen -W "$candidates" -- "$cur") )"#);
    println!("        return 0");
    println!("    fi");
    println!();
    // Static fallback: case tree for offline use.
    println!(r#"    case "$prev" in"#);
    let mut seen_keys = std::collections::HashSet::new();
    for (path, children) in &all_paths {
        let key = path.last().map_or("roko", |s| s.as_str());
        if !seen_keys.insert(key.to_string()) {
            continue;
        }
        let child_str = children
            .iter()
            .map(|c| shell_escape(c))
            .collect::<Vec<_>>()
            .join(" ");
        println!(r#"        {key})"#);
        println!(r#"            COMPREPLY=( $(compgen -W "{child_str}" -- "$cur") )"#);
        println!("            return 0");
        println!("            ;;");
    }
    println!("    esac");
    println!();
    // Top-level fallback.
    let top_words = top_names
        .iter()
        .map(|c| shell_escape(c))
        .collect::<Vec<_>>()
        .join(" ");
    println!(r#"    COMPREPLY=( $(compgen -W "{top_words}" -- "$cur") )"#);
    println!("}}");
    println!("complete -F _roko roko");
}

fn print_zsh_completions() {
    let tree = build_completion_tree();
    let all_paths = collect_all_subcommand_paths(&tree, &[]);
    let flags = completion_flag_words();
    let flag_words = flags.join(" ");

    println!("#compdef roko");
    println!("# roko zsh completions — recursive + dynamic (#332)");
    println!("_roko() {{");
    println!("  local -a candidates");
    println!();
    // Build command path from words, skipping flags.
    println!("  local cmd_path=()");
    println!("  local i");
    println!("  for (( i=2; i<CURRENT; i++ )); do");
    println!(r#"    case "$words[i]" in"#);
    println!("      -*) ;;");
    println!("      *) cmd_path+=(\"$words[i]\") ;;");
    println!("    esac");
    println!("  done");
    println!();
    // Flag completion.
    println!(r#"  if [[ "$words[CURRENT]" == -* ]]; then"#);
    println!("    local -a flags");
    println!("    flags=({flag_words})");
    println!(r#"    _describe 'roko flag' flags"#);
    println!("    return");
    println!("  fi");
    println!();
    // Dynamic completion via __complete.
    println!("  candidates=(${{(f)\"$(roko __complete --shell zsh --path \"${{(j: :)cmd_path}}\" --current \"$words[CURRENT]\" 2>/dev/null)\"}})");
    println!("  if (( $#candidates )); then");
    println!(r#"    _describe 'roko' candidates"#);
    println!("    return");
    println!("  fi");
    println!();
    // Static fallback by joined path.
    println!(r#"  local joined="${{(j: :)cmd_path}}""#);
    println!("  case \"$joined\" in");
    for (path, children) in &all_paths {
        let key = path.join(" ");
        let child_str = children
            .iter()
            .map(|c| shell_escape(c))
            .collect::<Vec<_>>()
            .join(" ");
        println!("    \"{key}\")");
        println!("      local -a subcmds");
        println!("      subcmds=({child_str})");
        println!(r#"      _describe 'roko subcommand' subcmds"#);
        println!("      ;;");
    }
    // Empty path = top-level.
    let top_names: Vec<String> = tree
        .children
        .iter()
        .map(|c| shell_escape(&c.name))
        .collect();
    let top_words = top_names.join(" ");
    println!("    \"\")");
    println!("      local -a commands");
    println!("      commands=({top_words})");
    println!(r#"      _describe 'roko command' commands"#);
    println!("      ;;");
    println!("  esac");
    println!("}}");
    println!(r#"_roko "$@""#);
}

fn print_fish_completions() {
    let tree = build_completion_tree();

    println!("# roko fish completions — recursive + dynamic (#332)");
    println!();
    // Dynamic completion helper function.
    println!("function __roko_dynamic_complete");
    println!("    set -l tokens (commandline -opc)");
    println!("    set -e tokens[1]  # remove 'roko'");
    println!("    set -l current (commandline -ct)");
    println!("    set -l cmd_path (string join ' ' -- $tokens)");
    println!(
        "    roko __complete --shell fish --path \"$cmd_path\" --current \"$current\" 2>/dev/null"
    );
    println!("end");
    println!();
    // Top-level subcommands.
    for child in &tree.children {
        let name = shell_escape(&child.name);
        println!("complete -c roko -f -n '__fish_use_subcommand' -a '{name}'");
    }
    println!();
    // Global flags with both long and short variants.
    let mut cli_cmd = Cli::command();
    cli_cmd.build();
    for arg in cli_cmd.get_arguments() {
        if let Some(long) = arg.get_long() {
            let escaped = shell_escape(long);
            if let Some(short) = arg.get_short() {
                println!("complete -c roko -l '{escaped}' -s '{short}'");
            } else {
                println!("complete -c roko -l '{escaped}'");
            }
        }
    }
    println!();
    // Recursive static subcommands at all depths.
    emit_fish_children(&tree);
    println!();
    // Dynamic completions for workspace items (plan/prd/agent names).
    println!("# Dynamic completions for workspace values.");
    println!(
        "complete -c roko -f -n '__fish_seen_subcommand_from plan' -a '(__roko_dynamic_complete)'"
    );
    println!(
        "complete -c roko -f -n '__fish_seen_subcommand_from prd' -a '(__roko_dynamic_complete)'"
    );
    println!(
        "complete -c roko -f -n '__fish_seen_subcommand_from agent' -a '(__roko_dynamic_complete)'"
    );
}

fn emit_fish_children(node: &CompletionNode) {
    for child in &node.children {
        let parent_name = shell_escape(&node.name);
        let child_name = shell_escape(&child.name);
        // Emit subcommand entry visible when parent is active.
        if node.name != "roko" && !node.name.is_empty() {
            println!(
                "complete -c roko -f -n '__fish_seen_subcommand_from {parent_name}' -a '{child_name}'"
            );
        }
        // Emit per-command value-enum flags.
        for (flag, values) in &child.value_enums {
            let flag_clean = flag.trim_start_matches('-');
            let vals = values
                .iter()
                .map(|v| shell_escape(v))
                .collect::<Vec<_>>()
                .join(" ");
            println!(
                "complete -c roko -f -n '__fish_seen_subcommand_from {}' -l '{}' -xa '{}'",
                shell_escape(&child.name),
                shell_escape(flag_clean),
                vals,
            );
        }
        emit_fish_children(child);
    }
}

pub(crate) fn resolved_capture_model(agent_command: &str, model: Option<&str>) -> String {
    if let Some(model) = model.filter(|value| !value.trim().is_empty()) {
        return model.to_string();
    }
    if agent_command.eq_ignore_ascii_case("claude") {
        "claude-opus-4-6".to_string()
    } else {
        String::new()
    }
}

pub(crate) fn capture_provider(agent_command: &str, resolved_model: &str) -> String {
    let command = agent_command.trim();
    let model = resolved_model.to_ascii_lowercase();
    if command.eq_ignore_ascii_case("claude") || model.starts_with("claude") {
        "anthropic".to_string()
    } else if command.eq_ignore_ascii_case("codex")
        || command.eq_ignore_ascii_case("openai")
        || model.starts_with("gpt-")
        || model.starts_with("o1")
        || model.starts_with("o3")
        || model.starts_with("o4")
    {
        "openai".to_string()
    } else if command.eq_ignore_ascii_case("ollama") || model.starts_with("ollama/") {
        "ollama".to_string()
    } else {
        command.to_string()
    }
}

pub(crate) fn capture_role(task_kind: &str) -> &'static str {
    if task_kind.starts_with("research-") {
        "Researcher"
    } else {
        "Strategist"
    }
}

pub(crate) fn capture_task_category(task_kind: &str) -> &'static str {
    if task_kind.starts_with("research-") {
        "research"
    } else if task_kind.starts_with("prd-plan") {
        "scaffolding"
    } else {
        "docs"
    }
}

pub(crate) fn capture_complexity_band(task_kind: &str) -> &'static str {
    if task_kind == "research-analyze" {
        "standard"
    } else if task_kind.starts_with("research-") {
        "deep"
    } else {
        "standard"
    }
}

pub(crate) fn capture_plan_id(task_id: &str) -> Option<&str> {
    task_id
        .rsplit(':')
        .next()
        .filter(|segment| !segment.is_empty())
}

pub(crate) fn build_capture_episode(
    agent_command: &str,
    model: Option<&str>,
    task_kind: &str,
    task_id: &str,
    prompt: &str,
    output: &str,
    success: bool,
    wall_time_ms: u64,
    resume_session: Option<&str>,
) -> (Episode, String) {
    let resolved_model = resolved_capture_model(agent_command, model);
    let provider = capture_provider(agent_command, &resolved_model);
    let role = capture_role(task_kind);
    let task_category = capture_task_category(task_kind);
    let complexity_band = capture_complexity_band(task_kind);
    let mut episode = Episode::new(agent_command.to_string(), task_id.to_string());
    episode.kind = "agent_turn".to_string();
    episode.trigger_kind = task_kind.to_string();
    episode.agent_template = role.to_string();
    episode.episode_id = episode.id.clone();
    episode.model = resolved_model.clone();
    episode.input_signal_hash = ContentHash::of(prompt.as_bytes()).to_hex();
    episode.output_signal_hash = ContentHash::of(output.as_bytes()).to_hex();
    episode.duration_secs = wall_time_ms as f64 / 1000.0;
    episode.usage.wall_ms = wall_time_ms;
    episode.success = success;
    episode.turns = 1;
    if !success {
        episode.failure_reason = Some("agent returned non-zero exit code".to_string());
    }
    episode
        .extra
        .insert("role".to_string(), serde_json::json!(role));
    episode
        .extra
        .insert("command".to_string(), serde_json::json!(agent_command));
    episode
        .extra
        .insert("backend".to_string(), serde_json::json!(agent_command));
    episode
        .extra
        .insert("task_kind".to_string(), serde_json::json!(task_kind));
    episode
        .extra
        .insert("task_id".to_string(), serde_json::json!(task_id));
    episode
        .extra
        .insert("model".to_string(), serde_json::json!(resolved_model));
    episode
        .extra
        .insert("provider".to_string(), serde_json::json!(provider.clone()));
    episode.extra.insert(
        "task_category".to_string(),
        serde_json::json!(task_category),
    );
    episode.extra.insert(
        "complexity_band".to_string(),
        serde_json::json!(complexity_band),
    );
    if let Some(plan_id) = capture_plan_id(task_id) {
        episode
            .extra
            .insert("plan_id".to_string(), serde_json::json!(plan_id));
    }
    if let Some(session_id) = resume_session.filter(|value| !value.trim().is_empty()) {
        episode
            .extra
            .insert("session_id".to_string(), serde_json::json!(session_id));
    }
    episode.extra.insert(
        "prompt_chars".to_string(),
        serde_json::json!(prompt.chars().count()),
    );
    episode.extra.insert(
        "output_chars".to_string(),
        serde_json::json!(output.chars().count()),
    );
    episode
        .extra
        .insert("success".to_string(), serde_json::json!(success));
    (episode, provider)
}

pub(crate) async fn persist_capture_episode(
    workdir: &Path,
    agent_command: &str,
    model: Option<&str>,
    task_kind: &str,
    task_id: &str,
    prompt: &str,
    output: &str,
    success: bool,
    wall_time_ms: u64,
    resume_session: Option<&str>,
) -> Result<()> {
    let (episode, provider) = build_capture_episode(
        agent_command,
        model,
        task_kind,
        task_id,
        prompt,
        output,
        success,
        wall_time_ms,
        resume_session,
    );

    // Load config and build the cascade model universe so the router learns from
    // this episode. Mirrors capture_runtime_model_slugs from learning_helpers.
    let config = roko_core::config::loader::load_config_unified(workdir).unwrap_or_default();
    let mut model_slugs = config.model_slugs_for_cascade();
    let episode_model = episode.model.as_str();
    if !episode_model.trim().is_empty() && !model_slugs.iter().any(|slug| slug == episode_model) {
        model_slugs.push(episode_model.to_string());
    }
    model_slugs.sort();
    model_slugs.dedup();
    tracing::debug!(workdir = %workdir.display(), "opening project learning runtime");
    let mut runtime = if model_slugs.is_empty() {
        LearningRuntime::open_for_project(workdir).await
    } else {
        LearningRuntime::open_for_project_with_models(workdir, model_slugs).await
    }
    .map_err(|e| anyhow!("open learning runtime: {e}"))?;
    let distillation_workdir = workdir.to_path_buf();
    let distillation_caller = roko_cli::learning_helpers::distillation_model_caller(workdir);
    runtime.set_episode_completion_hook(move |episode| {
        roko_neuro::spawn_episode_distillation(
            distillation_workdir.clone(),
            episode,
            Some(std::sync::Arc::clone(&distillation_caller)),
        );
    });

    let mut completed = CompletedRunInput::from_episode(episode);
    completed.provider = Some(provider);
    runtime
        .record_completed_run(completed)
        .await
        .map_err(|e| anyhow!("record learning feedback: {e}"))?;
    Ok(())
}

/// Check that configured providers have their API keys set and CLI binaries on
/// PATH. Returns an error with an actionable message on the first failure.
///
/// Only providers that have at least one of `api_key_env` or `command` set are
/// checked — providers with neither are silently skipped (local/mock providers).
/// Check that the provider for a specific model has its API key set and CLI
/// binary on PATH. Skips credential checks for local providers (empty
/// `api_key_env`).
pub(crate) fn preflight_provider_for_model(
    config: &RokoConfig,
    model_key: &str,
) -> anyhow::Result<()> {
    let model = config.models.get(model_key);
    if model.is_none() {
        // Builtin registry fallback: the model isn't in roko.toml but may be a
        // well-known model that works with an available API key.
        if let Some(builtin) = roko_core::config::model_registry::builtin_model(model_key) {
            // If the env var is set, dispatch will succeed.
            if std::env::var(builtin.api_key_env).is_ok() {
                return Ok(());
            }
            // The key isn't set — report an actionable error.
            anyhow::bail!(
                "model '{}' requires {} but it is not set.\n  hint: export {}=<your-key>",
                model_key,
                builtin.api_key_env,
                builtin.api_key_env
            );
        }
        anyhow::bail!("model '{}' not found in config", model_key);
    }
    let model = model.expect("model should be Some after config lookup loop");
    let provider_name = &model.provider;
    let provider = config.providers.get(provider_name).ok_or_else(|| {
        anyhow!(
            "provider '{}' (for model '{}') not found in config",
            provider_name,
            model_key
        )
    })?;

    if let Some(ref env_var) = provider.api_key_env
        && !env_var.trim().is_empty()
    {
        match std::env::var(env_var) {
            Ok(val) if val.is_empty() => {
                anyhow::bail!(
                    "provider '{}' requires {} but it is empty.\n  hint: export {}=<your-key>",
                    provider_name,
                    env_var,
                    env_var
                );
            }
            Err(_) => {
                anyhow::bail!(
                    "provider '{}' requires {} but it is not set.\n  hint: export {}=<your-key>",
                    provider_name,
                    env_var,
                    env_var
                );
            }
            Ok(_) => {}
        }
    }

    if let Some(ref binary) = provider.command
        && !binary_on_path(binary)
    {
        anyhow::bail!(
            "provider '{}' requires '{}' on PATH but it was not found.\n  hint: install {} or change provider in roko.toml",
            provider_name,
            binary,
            binary
        );
    }

    Ok(())
}

/// Check that the gate pipeline tools (cargo, git, clippy) are available.
/// Returns the names of any missing tools. The caller should warn but not
/// necessarily abort — some gate rungs may not need all tools.
pub(crate) fn preflight_gate_deps() -> Vec<String> {
    let mut missing = Vec::new();
    for tool in &["cargo", "git"] {
        if !binary_on_path(tool) {
            missing.push((*tool).to_string());
        }
    }
    // clippy is a cargo component, not a standalone binary
    let clippy_ok = std::process::Command::new("cargo")
        .args(["clippy", "--version"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !clippy_ok {
        missing.push("clippy".to_string());
    }
    missing
}

/// Return `true` if `name` is found on `PATH`.
fn binary_on_path(name: &str) -> bool {
    std::process::Command::new("which")
        .arg(name)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

// NOTE: `preflight_providers_aggregate` was removed — it emitted warnings for
// ALL configured providers, even those not used by the current command.
// Commands now call `preflight_provider_for_model` for only the selected model.
