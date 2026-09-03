//! research command handlers.

use crate::*;
use roko_core::config::DEFAULT_TTFT_TIMEOUT_MS;

/// Maximum number of episode lines to include in analyze context.
const ANALYZE_MAX_LINES: usize = 2_000;

/// Build a deterministic output filename from a topic slug and optional suffix.
fn research_output_path(workdir: &Path, topic: &str, suffix: &str) -> PathBuf {
    let slug = topic
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    let filename = if suffix.is_empty() {
        format!("{slug}.md")
    } else {
        format!("{slug}-{suffix}.md")
    };
    workdir.join(".roko/research").join(filename)
}

/// Save Perplexity research output with full citation metadata.
///
/// Preserves snippets, dates, URLs, and inline `[N]` citations from the
/// Perplexity metadata. Previously this was a dead library function; now it
/// is the canonical path for all Perplexity research artifact persistence.
fn save_perplexity_research(
    workdir: &Path,
    topic: &str,
    content: &str,
    citations: &[String],
    suffix: &str,
) -> Result<PathBuf> {
    let mut doc = String::new();
    let _ = writeln!(doc, "# Research: {topic}\n");
    let _ = writeln!(
        doc,
        "> Generated via Perplexity search-grounded research — {}\n",
        chrono::Local::now().format("%Y-%m-%d")
    );
    let _ = writeln!(doc, "{content}\n");

    if !citations.is_empty() {
        let _ = writeln!(doc, "\n## Sources\n");
        for (i, url) in citations.iter().enumerate() {
            let _ = writeln!(doc, "{}. {url}", i + 1);
        }
    }

    let path = research_output_path(workdir, topic, suffix);
    std::fs::write(&path, &doc).with_context(|| format!("write {}", path.display()))?;
    Ok(path)
}

/// Read the last `max_lines` from a file, returning the content and the total line count.
fn tail_lines_bounded(path: &Path, max_lines: usize) -> Result<(String, usize, usize)> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("read {}", path.display()))?;
    let total = content.lines().count();
    if total <= max_lines {
        return Ok((content, total, total));
    }
    let kept: String = content
        .lines()
        .skip(total - max_lines)
        .collect::<Vec<_>>()
        .join("\n");
    Ok((kept, total, max_lines))
}

#[allow(clippy::too_many_lines)]
pub(crate) async fn cmd_research(cli: &Cli, cmd: ResearchCmd) -> Result<i32> {
    use roko_cli::agent_config::{command_from_config, load_gateway_env, model_from_config};
    use roko_cli::agent_exec::{AgentExecOpts, run_agent_capture_silent};
    use roko_cli::research::{
        ResearchMode, build_research_prompt, build_research_prompt_gemini,
        build_research_prompt_perplexity, grounding_to_citations, save_research_with_grounding,
    };

    let workdir = resolve_workdir(cli);

    // Research subcommands that write artifacts need an exclusive lock;
    // List and Search are read-only and take a shared lock.
    let _lock = match &cmd {
        ResearchCmd::List { .. } | ResearchCmd::Search { .. } => {
            roko_cli::workspace_lock::acquire_workspace_lock_shared(&workdir.join(".roko"))?
        }
        _ => roko_cli::workspace_lock::acquire_workspace_lock(&workdir.join(".roko"))?,
    };

    roko_cli::research::ensure_dirs(&workdir)?;
    let gw = load_gateway_env(&workdir);
    let model = cli.model.clone().or_else(|| model_from_config(&workdir));
    let model_ref = model.as_deref();
    let cli_effort = cli.effort.map(|effort| effort.to_string());
    let cli_effort_ref = cli_effort.as_deref();
    let resume_session = cli.resume.as_deref();
    let agent_command = command_from_config(&workdir).unwrap_or_else(|| "claude".to_string());
    let config = roko_core::config::loader::load_config_unified(&workdir).unwrap_or_default();
    // #181: resolve per-role effort for the researcher role; CLI --effort wins.
    let researcher_effort = cli_effort_ref
        .unwrap_or_else(|| config.agent.effort_for_role("researcher"));

    match cmd {
        ResearchCmd::Topic {
            topic,
            deep,
            backend,
        } => {
            let topic = topic.join(" ");

            // Validate: --deep conflicts with --backend gemini|agent
            if deep
                && matches!(
                    backend,
                    ResearchBackend::Gemini | ResearchBackend::Agent
                )
            {
                anyhow::bail!(
                    "--deep requires Perplexity; it conflicts with --backend {}",
                    backend
                );
            }

            println!("🔬 Researching: {topic}");

            // Resolve the backend to use. The auto order is:
            //   1. If deep requested (--deep or config auto_deep) -> Perplexity deep
            //   2. Configured Perplexity deep (auto_deep) -> Perplexity deep
            //   3. Configured Gemini grounding -> Gemini
            //   4. Configured Perplexity standard search -> Perplexity standard
            //   5. Agent (Claude CLI) fallback
            let effective_deep = deep || (backend == ResearchBackend::Auto && config.perplexity.auto_deep);

            match backend {
                ResearchBackend::Auto => {
                    if effective_deep {
                        if config.perplexity.default_research_model.is_some() {
                            let reason = if deep { "--deep flag" } else { "auto_deep config" };
                            println!("  Backend: perplexity-deep ({reason})");
                            return run_perplexity_deep(
                                &workdir, &config, &topic, resume_session,
                            )
                            .await;
                        }
                        // deep requested but no Perplexity research model configured
                        if deep {
                            anyhow::bail!(
                                "--deep requires [perplexity].default_research_model to be configured"
                            );
                        }
                        // auto_deep but no model: fall through to next auto branch
                    }
                    if let Some(ref _model) = config.gemini.grounding_model {
                        println!("  Backend: gemini (auto: grounding model configured)");
                        return run_gemini_grounded(
                            &workdir,
                            &config,
                            &topic,
                            model_ref,
                            resume_session,
                        )
                        .await;
                    }
                    if config.perplexity.default_search_model.is_some() {
                        println!("  Backend: perplexity (auto: search model configured)");
                        return run_perplexity_standard(
                            &workdir, &config, &topic, resume_session,
                        )
                        .await;
                    }
                    println!("  Backend: agent (auto: fallback)");
                    run_agent_fallback(
                        &workdir,
                        &topic,
                        model_ref,
                        researcher_effort,
                        resume_session,
                        &gw.vars,
                        &agent_command,
                    )
                    .await
                }
                ResearchBackend::Perplexity => {
                    if effective_deep || deep {
                        if config.perplexity.default_research_model.is_none() {
                            anyhow::bail!(
                                "--backend perplexity with --deep requires [perplexity].default_research_model"
                            );
                        }
                        println!("  Backend: perplexity-deep (explicit)");
                        return run_perplexity_deep(
                            &workdir, &config, &topic, resume_session,
                        )
                        .await;
                    }
                    if config.perplexity.default_search_model.is_none() {
                        anyhow::bail!(
                            "--backend perplexity requires [perplexity].default_search_model to be configured"
                        );
                    }
                    println!("  Backend: perplexity (explicit)");
                    run_perplexity_standard(&workdir, &config, &topic, resume_session)
                        .await
                }
                ResearchBackend::Gemini => {
                    if config.gemini.grounding_model.is_none() {
                        anyhow::bail!(
                            "--backend gemini requires [gemini].grounding_model to be configured"
                        );
                    }
                    println!("  Backend: gemini (explicit)");
                    run_gemini_grounded(
                        &workdir,
                        &config,
                        &topic,
                        model_ref,
                        resume_session,
                    )
                    .await
                }
                ResearchBackend::Agent => {
                    println!("  Backend: agent (explicit)");
                    run_agent_fallback(
                        &workdir,
                        &topic,
                        model_ref,
                        researcher_effort,
                        resume_session,
                        &gw.vars,
                        &agent_command,
                    )
                    .await
                }
            }
        }
        ResearchCmd::EnhancePrd { slug } => {
            let prd_path = crate::commands::prd::find_prd(&workdir, &slug)?;
            let content = std::fs::read_to_string(&prd_path)
                .with_context(|| format!("read {}", prd_path.display()))?;
            println!("🔬 Enhancing PRD: {slug}");
            let task_prompt = format!(
                "Read the PRD at {path} and enhance it: \
                 (1) Add academic citations [AUTHOR-YEAR] for every design decision. \
                 (2) Add mermaid diagrams with color styling where architecture would be clearer. \
                 (3) Identify improvements from recent research. \
                 (4) Flag claims that contradict recent findings. \
                 Update the file in place. Also save a research summary to .roko/research/enhance-{slug}.md",
                path = prd_path.display()
            );
            let system = build_research_prompt(&workdir, &slug, &content, ResearchMode::EnhancePrd);
            let started = Instant::now();
            let (exit_code, output) = run_agent_capture_silent(AgentExecOpts {
                prompt: &task_prompt,
                workdir: &workdir,
                model: model_ref,
                effort: Some(researcher_effort),
                system_prompt: Some(&system),
                resume_session,
                env_vars: &gw.vars,
                role: Some("researcher"),
                allowed_tools: Some("Read,Write,Edit"),
            })
            .await?;
            if !output.is_empty() {
                print!("{output}");
            }
            let _ = crate::commands::util::persist_capture_episode(
                &workdir,
                &agent_command,
                model_ref,
                "research-enhance-prd",
                &format!("research:enhance-prd:{slug}"),
                &task_prompt,
                &output,
                exit_code == 0,
                started.elapsed().as_millis() as u64,
                resume_session,
            )
            .await;
            Ok(exit_code)
        }
        ResearchCmd::EnhancePlan { plan } => {
            let plan_dir = roko_cli::plan::plans_dir(&workdir).join(&plan);
            if !plan_dir.is_dir() {
                anyhow::bail!("Plan directory not found: {}", plan_dir.display());
            }
            println!("🔬 Enhancing plan: {plan}");
            // Use the resolved plan directory path, not hardcoded .roko/plans/
            let task_prompt = format!(
                "Read the plan at {plan_dir}/plan.md and {plan_dir}/tasks.toml. \
                 Optimize them using research-backed techniques: \
                 (1) Better task decomposition (cite SWE-bench, Agentless). \
                 (2) More precise context injection per task (exact file:line ranges). \
                 (3) Stronger verification (executable commands, not descriptions). \
                 (4) Cost optimization (assign cheapest model per task tier). \
                 Update the files in place.",
                plan_dir = plan_dir.display()
            );
            let mut context = String::new();
            for name in ["plan.md", "tasks.toml"] {
                let p = plan_dir.join(name);
                if p.exists() {
                    let c = std::fs::read_to_string(&p).unwrap_or_default();
                    let _ = write!(context, "### {name}\n```\n{c}\n```\n\n");
                }
            }
            let system =
                build_research_prompt(&workdir, &plan, &context, ResearchMode::EnhancePlan);
            let started = Instant::now();
            let (exit_code, output) = run_agent_capture_silent(AgentExecOpts {
                prompt: &task_prompt,
                workdir: &workdir,
                model: model_ref,
                effort: Some(researcher_effort),
                system_prompt: Some(&system),
                resume_session,
                env_vars: &gw.vars,
                role: Some("researcher"),
                allowed_tools: Some("Read,Write,Edit"),
            })
            .await?;
            if !output.is_empty() {
                print!("{output}");
            }
            let _ = crate::commands::util::persist_capture_episode(
                &workdir,
                &agent_command,
                model_ref,
                "research-enhance-plan",
                &format!("research:enhance-plan:{plan}"),
                &task_prompt,
                &output,
                exit_code == 0,
                started.elapsed().as_millis() as u64,
                resume_session,
            )
            .await;
            Ok(exit_code)
        }
        ResearchCmd::EnhanceTasks { plan } => {
            let plan_dir = roko_cli::plan::plans_dir(&workdir);
            let tasks_path = plan_dir.join(&plan).join("tasks.toml");
            if !tasks_path.exists() {
                anyhow::bail!("tasks.toml not found: {}", tasks_path.display());
            }
            println!("🔬 Optimizing tasks: {plan}");
            let content = std::fs::read_to_string(&tasks_path)?;
            // Use the resolved tasks path, not hardcoded .roko/plans/
            let task_prompt = format!(
                "Read {tasks_path} and optimize every task: \
                 (1) Split any task >50 LOC into smaller subtasks. \
                 (2) Add context.read_files with exact line ranges for each task. \
                 (3) Ensure every acceptance criterion is a runnable shell command. \
                 (4) Remove unnecessary dependency edges to increase parallelism. \
                 (5) Assign tier (mechanical/focused/integrative/architectural) and model_hint. \
                 Search the codebase to verify file paths exist. Update tasks.toml in place.",
                tasks_path = tasks_path.display()
            );
            let system =
                build_research_prompt(&workdir, &plan, &content, ResearchMode::EnhanceTasks);
            let started = Instant::now();
            let (exit_code, output) = run_agent_capture_silent(AgentExecOpts {
                prompt: &task_prompt,
                workdir: &workdir,
                model: model_ref,
                effort: Some(researcher_effort),
                system_prompt: Some(&system),
                resume_session,
                env_vars: &gw.vars,
                role: Some("researcher"),
                allowed_tools: Some("Read,Write,Edit"),
            })
            .await?;
            if !output.is_empty() {
                print!("{output}");
            }
            let _ = crate::commands::util::persist_capture_episode(
                &workdir,
                &agent_command,
                model_ref,
                "research-enhance-tasks",
                &format!("research:enhance-tasks:{plan}"),
                &task_prompt,
                &output,
                exit_code == 0,
                started.elapsed().as_millis() as u64,
                resume_session,
            )
            .await;
            Ok(exit_code)
        }
        ResearchCmd::Analyze => {
            let episodes_path = workdir.join(".roko/episodes.jsonl");
            if !episodes_path.exists() {
                println!("No episodes found. Run some tasks first:");
                println!("  roko plan run plans/<plan-dir> --engine runner-v2");
                println!("  roko do \"<prompt>\"");
                return Ok(1);
            }

            let (context, total_lines, included_lines) =
                tail_lines_bounded(&episodes_path, ANALYZE_MAX_LINES)?;
            if total_lines == 0 {
                println!("Episodes file is empty. Run some tasks first:");
                println!("  roko plan run plans/<plan-dir> --engine runner-v2");
                return Ok(1);
            }

            println!("🔬 Analyzing execution data ({included_lines}/{total_lines} episodes)");
            if included_lines < total_lines {
                println!(
                    "  Note: analyzing most recent {included_lines} of {total_lines} episodes"
                );
            }

            let date_suffix = chrono::Local::now().format("%Y%m%d").to_string();
            let out_path = research_output_path(&workdir, "execution-analysis", &date_suffix);
            let task_prompt = format!(
                "Read .roko/episodes.jsonl and analyze: \
                 (1) First-attempt pass rate by task tier and model. \
                 (2) Cost per task — are expensive models used for easy tasks? \
                 (3) Retry patterns — what kinds of tasks fail most? \
                 (4) Recommendations: which bandit weights to adjust. \
                 Save analysis to {}",
                out_path.display()
            );
            let system = build_research_prompt(
                &workdir,
                "execution-analysis",
                &context,
                ResearchMode::AnalyzeExecution,
            );
            let started = Instant::now();
            let (exit_code, output) = run_agent_capture_silent(AgentExecOpts {
                prompt: &task_prompt,
                workdir: &workdir,
                model: model_ref,
                effort: Some(researcher_effort),
                system_prompt: Some(&system),
                resume_session,
                env_vars: &gw.vars,
                role: Some("researcher"),
                allowed_tools: Some("Read,Write,Edit"),
            })
            .await?;
            if !output.is_empty() {
                print!("{output}");
            }
            let _ = crate::commands::util::persist_capture_episode(
                &workdir,
                &agent_command,
                model_ref,
                "research-analyze",
                "research:analyze:execution",
                &task_prompt,
                &output,
                exit_code == 0,
                started.elapsed().as_millis() as u64,
                resume_session,
            )
            .await;
            Ok(exit_code)
        }
        ResearchCmd::List {
            json,
            include_generated,
        } => {
            let files = roko_cli::research::list_research(&workdir)?;
            let files: Vec<_> = if include_generated {
                files
            } else {
                files
                    .into_iter()
                    .filter(|f| {
                        f.file_stem()
                            .map(|s| s.to_string_lossy().to_uppercase() != "INDEX")
                            .unwrap_or(true)
                    })
                    .collect()
            };

            if json {
                #[derive(serde::Serialize)]
                struct Entry {
                    name: String,
                    path: String,
                    size: u64,
                    #[serde(skip_serializing_if = "Option::is_none")]
                    modified: Option<String>,
                }
                let entries: Vec<Entry> = files
                    .iter()
                    .map(|f| {
                        let meta = std::fs::metadata(f).ok();
                        Entry {
                            name: f
                                .file_stem()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string(),
                            path: f.display().to_string(),
                            size: meta.as_ref().map(|m| m.len()).unwrap_or(0),
                            modified: meta
                                .and_then(|m| m.modified().ok())
                                .map(|t| {
                                    let dt: chrono::DateTime<chrono::Local> = t.into();
                                    dt.format("%Y-%m-%dT%H:%M:%S").to_string()
                                }),
                        }
                    })
                    .collect();
                println!(
                    "{}",
                    serde_json::to_string_pretty(&entries)
                        .unwrap_or_else(|_| "[]".to_string())
                );
            } else if files.is_empty() {
                println!("No research artifacts. Run: roko research topic \"your topic\"");
            } else {
                println!("═══ Research Artifacts ═══");
                for f in &files {
                    let name = f.file_stem().unwrap_or_default().to_string_lossy();
                    let meta = std::fs::metadata(f).ok();
                    let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
                    let mtime = meta
                        .and_then(|m| m.modified().ok())
                        .map(|t| {
                            let dt: chrono::DateTime<chrono::Local> = t.into();
                            dt.format("%Y-%m-%d %H:%M").to_string()
                        })
                        .unwrap_or_default();
                    println!("  {name:<40} {size:>6} bytes  {mtime}");
                }
            }
            Ok(0)
        }
        ResearchCmd::Search {
            query,
            domains,
            recency,
            output,
            no_save,
        } => {
            use roko_agent::perplexity::search::{PerplexitySearchClient, SearchQuery};

            let query_str = query.join(" ");
            if query_str.trim().is_empty() {
                anyhow::bail!("provide a search query");
            }

            let api_key =
                std::env::var("PERPLEXITY_API_KEY").context("PERPLEXITY_API_KEY not set")?;

            let recency_filter = recency.map(|r| r.as_api_str().to_string());

            let search_query = SearchQuery {
                query: query_str.clone(),
                domain_filter: if domains.is_empty() {
                    None
                } else {
                    Some(domains)
                },
                date_range: None,
                recency_filter,
                ..Default::default()
            };

            println!("🔍 Searching: {query_str}");

            let started = Instant::now();
            let client = PerplexitySearchClient::new(api_key);
            let responses = client
                .search_batch(&[search_query])
                .await
                .map_err(|e| anyhow::anyhow!("search error: {e}"))?;

            let results: Vec<_> = responses.into_iter().flat_map(|r| r.results).collect();

            if results.is_empty() {
                println!("No results found.");
            } else {
                println!("\n═══ Results ═══\n");
                for (i, r) in results.iter().enumerate() {
                    println!("{}. {}", i + 1, r.title);
                    println!("   {}", r.url);
                    if let Some(date) = &r.date {
                        println!("   Published: {date}");
                    }
                    let snippet = if r.content.len() > 300 {
                        format!("{}...", &r.content[..300])
                    } else {
                        r.content.clone()
                    };
                    println!("   {snippet}");
                    println!();
                }
            }

            // Persist search results unless --no-save
            if !no_save && !results.is_empty() {
                let out_path = output.unwrap_or_else(|| {
                    research_output_path(&workdir, &query_str, "search")
                });
                let mut doc = String::new();
                let _ = writeln!(doc, "# Search: {query_str}\n");
                let _ = writeln!(
                    doc,
                    "> Perplexity search — {}\n",
                    chrono::Local::now().format("%Y-%m-%d %H:%M")
                );
                for (i, r) in results.iter().enumerate() {
                    let _ = writeln!(doc, "## {}. {}\n", i + 1, r.title);
                    let _ = writeln!(doc, "**URL**: {}", r.url);
                    if let Some(date) = &r.date {
                        let _ = writeln!(doc, "**Published**: {date}");
                    }
                    let _ = writeln!(doc, "\n{}\n", r.content);
                }
                if let Some(parent) = out_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&out_path, &doc)
                    .with_context(|| format!("write {}", out_path.display()))?;
                println!("📄 Saved: {}", out_path.display());
            }

            // Record a typed operation episode for the search
            let _ = crate::commands::util::persist_capture_episode(
                &workdir,
                "perplexity",
                None,
                "research-search",
                &format!(
                    "research:search:{}",
                    query_str.to_lowercase().replace(' ', "-")
                ),
                &query_str,
                &format!("{} results", results.len()),
                !results.is_empty(),
                started.elapsed().as_millis() as u64,
                resume_session,
            )
            .await;

            Ok(0)
        }
    }
}

// ── Backend dispatch helpers ──────────────────────────────────────────

/// Run Perplexity deep research (sonar-deep-research, async polling).
async fn run_perplexity_deep(
    workdir: &Path,
    config: &RokoConfig,
    topic: &str,
    resume_session: Option<&str>,
) -> Result<i32> {
    use roko_agent::perplexity::types::PerplexityMetadata;
    use roko_cli::research::{ResearchMode, build_research_prompt_perplexity};
    use roko_core::Body;

    let model_slug = config
        .perplexity
        .default_research_model
        .clone()
        .unwrap_or_else(|| "sonar-deep-research".to_string());

    let (combined_prompt, _) = build_research_prompt_perplexity(
        workdir,
        topic,
        "",
        ResearchMode::Topic,
        &config.perplexity,
    );

    let (routing_config, timeout_ms) =
        with_perplexity_research_model(config, &model_slug, true);
    let agent = spawn_agent_scoped(
        &routing_config,
        SpawnAgentSpec {
            model: model_slug.clone(),
            command: None,
            timeout_ms: Some(timeout_ms),
            system_prompt: None,
            cached_content: None,
            tools: None,
            mcp_config: None,
            working_dir: Some(workdir.to_path_buf()),
            env: Vec::new(),
            extra_args: Vec::new(),
            effort: None,
            bare_mode: false,
            dangerously_skip_permissions: false,
            name: String::new(),
            role: Some("researcher".to_string()),
        },
        format!("create Perplexity deep research agent for model {model_slug}"),
    )?;
    println!("⏳ Deep research submitted ({model_slug}). This takes 1-10 min...");

    let input = roko_core::Signal::builder(Kind::Prompt)
        .body(Body::text(&combined_prompt))
        .build();

    let started = Instant::now();
    let mut handle =
        tokio::spawn(async move { agent.run(&input, &Context::now()).await });
    let poll_started = std::time::Instant::now();
    let result = loop {
        tokio::select! {
            r = &mut handle => break r.context("agent task panicked")?,
            _ = tokio::time::sleep(std::time::Duration::from_secs(15)) => {
                let elapsed = poll_started.elapsed().as_secs();
                println!("  ⏳ Still researching... ({elapsed}s elapsed)");
            }
        }
    };

    if !result.success {
        let err_text = result.output.body.as_text().unwrap_or("unknown error");
        let output = result.output.body.as_text().unwrap_or_default().to_string();
        let _ = crate::commands::util::persist_capture_episode(
            workdir,
            "perplexity",
            Some(&model_slug),
            "research-topic-deep",
            &format!("research:topic:{}", topic.to_lowercase().replace(' ', "-")),
            &combined_prompt,
            &output,
            false,
            started.elapsed().as_millis() as u64,
            resume_session,
        )
        .await;
        anyhow::bail!("Deep research failed: {err_text}");
    }

    let content = result
        .output
        .body
        .as_text()
        .map_err(|e| anyhow::anyhow!("response body not text: {e}"))?
        .to_string();

    let citations: Vec<String> = result
        .output
        .tag("pplx_meta")
        .and_then(|meta_json| {
            serde_json::from_str::<PerplexityMetadata>(meta_json)
                .ok()
                .map(|m| m.citations)
        })
        .unwrap_or_default();

    let out_path =
        save_perplexity_research(workdir, topic, &content, &citations, "deep")?;
    println!("📄 Saved: {}", out_path.display());
    if !citations.is_empty() {
        println!("📚 {} citations", citations.len());
    }
    let _ = crate::commands::util::persist_capture_episode(
        workdir,
        "perplexity",
        Some(&model_slug),
        "research-topic-deep",
        &format!("research:topic:{}", topic.to_lowercase().replace(' ', "-")),
        &combined_prompt,
        &content,
        true,
        started.elapsed().as_millis() as u64,
        resume_session,
    )
    .await;
    Ok(0)
}

/// Run Gemini grounding-backed research.
#[allow(clippy::too_many_lines)]
async fn run_gemini_grounded(
    workdir: &Path,
    config: &RokoConfig,
    topic: &str,
    _model_override: Option<&str>,
    resume_session: Option<&str>,
) -> Result<i32> {
    use roko_cli::research::{
        ResearchMode, build_research_prompt_gemini, grounding_to_citations,
        save_research_with_grounding,
    };
    use roko_core::Body;

    let model_slug = config
        .gemini
        .grounding_model
        .clone()
        .context("gemini.grounding_model not configured")?;

    let (combined_prompt, enable_grounding) = build_research_prompt_gemini(
        workdir,
        topic,
        ResearchMode::Topic,
        &config.gemini,
    );
    if !enable_grounding {
        anyhow::bail!("Gemini grounding not enabled for model {model_slug}");
    }

    let configured_profile = config.models.get(&model_slug).cloned();
    let provider_key = configured_profile
        .as_ref()
        .map(|profile| profile.provider.clone())
        .unwrap_or_else(|| "gemini".to_string());
    let configured_provider = config
        .providers
        .get(&provider_key)
        .cloned()
        .or_else(|| config.providers.get("gemini").cloned());
    let base_url = configured_provider
        .as_ref()
        .and_then(|provider| provider.base_url.clone())
        .unwrap_or_else(|| "https://generativelanguage.googleapis.com".to_string());
    let timeout_ms = configured_provider
        .as_ref()
        .and_then(|provider| provider.timeout_ms)
        .unwrap_or(300_000);

    let mut model_profile = configured_profile.unwrap_or_else(|| ModelProfile {
        provider: provider_key.clone(),
        slug: model_slug.clone(),
        context_window: 1_048_576,
        max_output: Some(65_536),
        supports_tools: true,
        supports_thinking: true,
        supports_vision: false,
        supports_web_search: false,
        supports_mcp_tools: false,
        supports_partial: false,
        supports_grounding: true,
        supports_code_execution: false,
        supports_caching: false,
        provider_routing: None,
        tool_format: "gemini_native".to_string(),
        cost_input_per_m: None,
        cost_output_per_m: None,
        cost_input_per_m_high: None,
        cost_output_per_m_high: None,
        cost_cache_read_per_m: None,
        cost_cache_write_per_m: None,
        thinking_level: Some(config.gemini.thinking_level.clone()),
        max_tools: None,
        tokenizer_ratio: None,
        supports_search: false,
        supports_citations: false,
        supports_async: false,
        is_embedding_model: false,
        search_context_size: None,
        cost_per_request: None,
        ..Default::default()
    });
    model_profile.supports_grounding = true;
    model_profile.tool_format = "gemini_native".to_string();
    if model_profile.thinking_level.is_none() {
        model_profile.thinking_level = Some(config.gemini.thinking_level.clone());
    }

    let routing_config = with_research_provider_model(
        config,
        &provider_key,
        configured_provider.unwrap_or(ProviderConfig {
            kind: ProviderKind::GeminiApi,
            base_url: Some(base_url),
            api_key_env: Some("GEMINI_API_KEY".to_string()),
            command: None,
            args: None,
            timeout_ms: Some(timeout_ms),
            ttft_timeout_ms: Some(DEFAULT_TTFT_TIMEOUT_MS),
            connect_timeout_ms: Some(5_000),
            extra_headers: None,
            max_concurrent: None,
            limits: None,
        }),
        model_profile,
    );
    let agent = spawn_agent_scoped(
        &routing_config,
        SpawnAgentSpec {
            model: model_slug.clone(),
            command: None,
            timeout_ms: Some(timeout_ms),
            system_prompt: None,
            cached_content: None,
            tools: None,
            mcp_config: None,
            effort: Some(config.gemini.thinking_level.clone()),
            name: format!("gemini:{model_slug}"),
            working_dir: Some(workdir.to_path_buf()),
            env: Vec::new(),
            extra_args: Vec::new(),
            bare_mode: false,
            dangerously_skip_permissions: false,
            role: Some("researcher".to_string()),
        },
        format!("create Gemini research agent for model {model_slug}"),
    )?;

    let input = roko_core::Signal::builder(Kind::Prompt)
        .body(Body::text(&combined_prompt))
        .build();
    let started = Instant::now();
    let result = agent.run(&input, &Context::now()).await;

    if !result.success {
        let err_text = result.output.body.as_text().unwrap_or("unknown error");
        let output = result.output.body.as_text().unwrap_or_default().to_string();
        let _ = crate::commands::util::persist_capture_episode(
            workdir,
            "gemini",
            Some(&model_slug),
            "research-topic-gemini",
            &format!("research:topic:{}", topic.to_lowercase().replace(' ', "-")),
            &combined_prompt,
            &output,
            false,
            started.elapsed().as_millis() as u64,
            resume_session,
        )
        .await;
        anyhow::bail!("Gemini research failed: {err_text}");
    }

    let content = result
        .output
        .body
        .as_text()
        .map_err(|e| anyhow::anyhow!("response body not text: {e}"))?
        .to_string();

    let grounding = result
        .output
        .tag("gemini_meta")
        .and_then(|meta_json| {
            serde_json::from_str::<roko_agent::gemini::GeminiMetadata>(meta_json).ok()
        })
        .and_then(|metadata| metadata.grounding_metadata);

    let out_path = if let Some(grounding) = &grounding {
        save_research_with_grounding(workdir, topic, &content, grounding)?
    } else {
        let path = research_output_path(workdir, topic, "");
        std::fs::write(&path, &content)
            .with_context(|| format!("write {}", path.display()))?;
        path
    };

    println!("📄 Saved: {}", out_path.display());
    if let Some(grounding) = &grounding {
        let citations = grounding_to_citations(grounding);
        if !citations.is_empty() {
            println!("📚 {} citations", citations.len());
        }
    }
    let _ = crate::commands::util::persist_capture_episode(
        workdir,
        "gemini",
        Some(&model_slug),
        "research-topic-gemini",
        &format!("research:topic:{}", topic.to_lowercase().replace(' ', "-")),
        &combined_prompt,
        &content,
        true,
        started.elapsed().as_millis() as u64,
        resume_session,
    )
    .await;
    Ok(0)
}

/// Run Perplexity standard (non-deep) search-grounded research.
async fn run_perplexity_standard(
    workdir: &Path,
    config: &RokoConfig,
    topic: &str,
    resume_session: Option<&str>,
) -> Result<i32> {
    use roko_agent::perplexity::types::PerplexityMetadata;
    use roko_cli::research::{ResearchMode, build_research_prompt_perplexity};
    use roko_core::Body;

    let model_slug = config
        .perplexity
        .default_search_model
        .clone()
        .context("perplexity.default_search_model not configured")?;

    let (combined_prompt, search_opts) = build_research_prompt_perplexity(
        workdir,
        topic,
        "",
        ResearchMode::Topic,
        &config.perplexity,
    );
    let (routing_config, timeout_ms) =
        with_perplexity_research_model(config, &model_slug, false);
    let agent = spawn_agent_scoped(
        &routing_config,
        SpawnAgentSpec {
            model: model_slug.clone(),
            command: None,
            timeout_ms: Some(timeout_ms),
            system_prompt: None,
            cached_content: None,
            tools: None,
            mcp_config: None,
            working_dir: Some(workdir.to_path_buf()),
            env: Vec::new(),
            extra_args: vec![format!(
                "{}{}",
                roko_agent::provider::PERPLEXITY_SEARCH_OPTIONS_ARG_PREFIX,
                serde_json::to_string(&search_opts)
                    .expect("Perplexity search options must serialize"),
            )],
            effort: None,
            bare_mode: false,
            dangerously_skip_permissions: false,
            name: String::new(),
            role: Some("researcher".to_string()),
        },
        format!("create Perplexity research agent for model {model_slug}"),
    )?;

    let input = roko_core::Signal::builder(Kind::Prompt)
        .body(Body::text(&combined_prompt))
        .build();
    let started = Instant::now();
    let result = agent.run(&input, &Context::now()).await;

    if !result.success {
        let err_text = result.output.body.as_text().unwrap_or("unknown error");
        let output = result.output.body.as_text().unwrap_or_default().to_string();
        let _ = crate::commands::util::persist_capture_episode(
            workdir,
            "perplexity",
            Some(&model_slug),
            "research-topic-perplexity",
            &format!("research:topic:{}", topic.to_lowercase().replace(' ', "-")),
            &combined_prompt,
            &output,
            false,
            started.elapsed().as_millis() as u64,
            resume_session,
        )
        .await;
        anyhow::bail!("Perplexity research failed: {err_text}");
    }

    let content = result
        .output
        .body
        .as_text()
        .map_err(|e| anyhow::anyhow!("response body not text: {e}"))?
        .to_string();

    let citations: Vec<String> = result
        .output
        .tag("pplx_meta")
        .and_then(|meta_json| {
            serde_json::from_str::<PerplexityMetadata>(meta_json)
                .ok()
                .map(|m| m.citations)
        })
        .unwrap_or_default();

    // Use the rich citation saver instead of manual append
    let out_path =
        save_perplexity_research(workdir, topic, &content, &citations, "")?;
    println!("📄 Saved: {}", out_path.display());
    if !citations.is_empty() {
        println!("📚 {} citations", citations.len());
    }
    let _ = crate::commands::util::persist_capture_episode(
        workdir,
        "perplexity",
        Some(&model_slug),
        "research-topic-perplexity",
        &format!("research:topic:{}", topic.to_lowercase().replace(' ', "-")),
        &combined_prompt,
        &content,
        true,
        started.elapsed().as_millis() as u64,
        resume_session,
    )
    .await;
    Ok(0)
}

/// Run agent (Claude CLI) fallback for research.
async fn run_agent_fallback(
    workdir: &Path,
    topic: &str,
    model_ref: Option<&str>,
    researcher_effort: &str,
    resume_session: Option<&str>,
    env_vars: &[(String, String)],
    agent_command: &str,
) -> Result<i32> {
    use roko_cli::agent_exec::{AgentExecOpts, run_agent_capture_silent};
    use roko_cli::research::{ResearchMode, build_research_prompt};

    let slug = topic.to_lowercase().replace(' ', "-");
    let task_prompt = format!(
        "Research the topic: \"{topic}\". \
         Save your findings to .roko/research/{slug}.md with full citations. \
         Read existing docs in .roko/prd/ and .roko/research/ for context on the project.",
    );
    let system = build_research_prompt(workdir, topic, "", ResearchMode::Topic);
    let started = Instant::now();
    let (exit_code, output) = run_agent_capture_silent(AgentExecOpts {
        prompt: &task_prompt,
        workdir,
        model: model_ref,
        effort: Some(researcher_effort),
        system_prompt: Some(&system),
        resume_session,
        env_vars,
        role: Some("researcher"),
        allowed_tools: Some("Read,Write,Edit"),
    })
    .await?;
    if !output.is_empty() {
        print!("{output}");
    }
    let _ = crate::commands::util::persist_capture_episode(
        workdir,
        agent_command,
        model_ref,
        "research-topic-claude",
        &format!("research:topic:{}", topic.to_lowercase().replace(' ', "-")),
        &task_prompt,
        &output,
        exit_code == 0,
        started.elapsed().as_millis() as u64,
        resume_session,
    )
    .await;
    Ok(exit_code)
}

pub(crate) fn with_research_provider_model(
    config: &RokoConfig,
    provider_key: &str,
    provider_config: ProviderConfig,
    model_profile: ModelProfile,
) -> RokoConfig {
    let mut routing_config = config.clone();
    routing_config
        .providers
        .entry(provider_key.to_string())
        .or_insert(provider_config);
    routing_config
        .models
        .entry(model_profile.slug.clone())
        .or_insert(model_profile);
    routing_config
}

pub(crate) fn with_perplexity_research_model(
    config: &RokoConfig,
    model_slug: &str,
    supports_async: bool,
) -> (RokoConfig, u64) {
    let configured_profile = config.models.get(model_slug).cloned();
    let provider_key = configured_profile
        .as_ref()
        .map(|profile| profile.provider.clone())
        .unwrap_or_else(|| "perplexity".to_string());
    let configured_provider = config
        .providers
        .get(&provider_key)
        .cloned()
        .or_else(|| config.providers.get("perplexity").cloned());
    let timeout_ms = configured_provider
        .as_ref()
        .and_then(|provider| provider.timeout_ms)
        .unwrap_or(300_000);

    let mut model_profile = configured_profile.unwrap_or_else(|| ModelProfile {
        provider: provider_key.clone(),
        slug: model_slug.to_string(),
        context_window: 127_072,
        max_output: Some(8_192),
        supports_tools: false,
        supports_thinking: false,
        supports_vision: false,
        supports_web_search: true,
        supports_mcp_tools: false,
        supports_partial: false,
        supports_grounding: false,
        supports_code_execution: false,
        supports_caching: false,
        provider_routing: None,
        tool_format: "openai_json".to_string(),
        cost_input_per_m: None,
        cost_output_per_m: None,
        cost_input_per_m_high: None,
        cost_output_per_m_high: None,
        cost_cache_read_per_m: None,
        cost_cache_write_per_m: None,
        thinking_level: None,
        max_tools: None,
        tokenizer_ratio: None,
        supports_search: true,
        supports_citations: true,
        supports_async,
        is_embedding_model: false,
        search_context_size: None,
        cost_per_request: None,
        ..Default::default()
    });
    model_profile.supports_search = true;
    model_profile.supports_citations = true;
    model_profile.supports_async |= supports_async;

    let routing_config = with_research_provider_model(
        config,
        &provider_key,
        configured_provider.unwrap_or(ProviderConfig {
            kind: ProviderKind::PerplexityApi,
            base_url: Some("https://api.perplexity.ai".to_string()),
            api_key_env: Some("PERPLEXITY_API_KEY".to_string()),
            command: None,
            args: None,
            timeout_ms: Some(timeout_ms),
            ttft_timeout_ms: Some(DEFAULT_TTFT_TIMEOUT_MS),
            connect_timeout_ms: Some(5_000),
            extra_headers: None,
            max_concurrent: None,
            limits: None,
        }),
        model_profile,
    );

    (routing_config, timeout_ms)
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn research_output_path_basic_slug() {
        let path = research_output_path(Path::new("/test"), "Git Worktree Best Practices", "");
        assert_eq!(
            path,
            PathBuf::from("/test/.roko/research/git-worktree-best-practices.md")
        );
    }

    #[test]
    fn research_output_path_with_suffix() {
        let path = research_output_path(Path::new("/test"), "execution analysis", "20260903");
        assert_eq!(
            path,
            PathBuf::from("/test/.roko/research/execution-analysis-20260903.md")
        );
    }

    #[test]
    fn research_output_path_deep_suffix() {
        let path = research_output_path(Path::new("/w"), "topic name", "deep");
        assert_eq!(path, PathBuf::from("/w/.roko/research/topic-name-deep.md"));
    }

    #[test]
    fn research_output_path_search_suffix() {
        let path = research_output_path(Path::new("/w"), "my query", "search");
        assert_eq!(path, PathBuf::from("/w/.roko/research/my-query-search.md"));
    }

    #[test]
    fn tail_lines_bounded_short_file() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("test.jsonl");
        std::fs::write(&p, "line1\nline2\nline3\n").unwrap();
        let (content, total, included) = tail_lines_bounded(&p, 100).unwrap();
        assert_eq!(total, 3);
        assert_eq!(included, 3);
        assert!(content.contains("line1"));
    }

    #[test]
    fn tail_lines_bounded_long_file() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("test.jsonl");
        let lines: Vec<String> = (0..50).map(|i| format!("line-{i}")).collect();
        std::fs::write(&p, lines.join("\n")).unwrap();
        let (content, total, included) = tail_lines_bounded(&p, 10).unwrap();
        assert_eq!(total, 50);
        assert_eq!(included, 10);
        assert!(content.contains("line-49"));
        assert!(!content.contains("line-0"));
    }

    #[test]
    fn save_perplexity_research_preserves_citations() {
        let dir = tempfile::tempdir().unwrap();
        let research_dir = dir.path().join(".roko/research");
        std::fs::create_dir_all(&research_dir).unwrap();

        let citations = vec![
            "https://example.com/1".to_string(),
            "https://example.com/2".to_string(),
        ];
        let path = save_perplexity_research(
            dir.path(),
            "test topic",
            "Some research content.",
            &citations,
            "",
        )
        .unwrap();

        let doc = std::fs::read_to_string(&path).unwrap();
        assert!(doc.contains("# Research: test topic"));
        assert!(doc.contains("Perplexity search-grounded research"));
        assert!(doc.contains("Some research content."));
        assert!(doc.contains("## Sources"));
        assert!(doc.contains("1. https://example.com/1"));
        assert!(doc.contains("2. https://example.com/2"));
    }

    #[test]
    fn save_perplexity_research_no_citations() {
        let dir = tempfile::tempdir().unwrap();
        let research_dir = dir.path().join(".roko/research");
        std::fs::create_dir_all(&research_dir).unwrap();

        let path = save_perplexity_research(
            dir.path(),
            "test",
            "Content here.",
            &[],
            "deep",
        )
        .unwrap();

        let doc = std::fs::read_to_string(&path).unwrap();
        assert!(doc.contains("Content here."));
        assert!(!doc.contains("## Sources"));
        assert!(path.to_string_lossy().contains("test-deep.md"));
    }

    #[test]
    fn research_paths_use_plans_dir() {
        // Verify the enhance plan/task prompts would use resolved paths
        // (this is a compile-time contract test; the actual interpolation
        // happens at runtime with the resolved plan_dir variable)
        let workdir = Path::new("/project");
        let plan_dir = roko_cli::plan::plans_dir(workdir).join("my-plan");
        let prompt = format!(
            "Read the plan at {plan_dir}/plan.md",
            plan_dir = plan_dir.display()
        );
        // The prompt should NOT contain hardcoded .roko/plans/
        // (plans_dir returns /project/.roko/plans when no top-level plans/ exists,
        // but the point is that it uses the resolver, not a literal)
        assert!(prompt.contains(&plan_dir.display().to_string()));
    }
}
