//! [`CliRuntime`] implementation backed by the real CLI internals.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Context;
use async_trait::async_trait;
use roko_core::config::schema::RokoConfig;
use roko_fs::RokoLayout;
use roko_learn::playbook::PlaybookStore;
use roko_neuro::KnowledgeStore;
use roko_serve::bench::{BenchConfigOverrides, BenchStrategy};
use roko_serve::plan_types::{PlanSummaryDto, PlanTaskDto, PlanTasksDto};
use roko_serve::runtime::{
    CliRuntime, DashboardInfo, PlanExecutionResult, PlanGenerationResult, RepoInfo, RunResult,
    RunResultUsage, RuntimeGateResult, SessionStatusInfo, TriggerExecutionScope,
};

use crate::config::{Config, RepoRegistry};
use crate::prd;
use crate::runner::types::{GateCompletionKind, RunnerEvent};
use crate::state_hub::SharedStateHub;
use crate::status::collect_session_status;
use crate::tui::DashboardScaffold;
use crate::workspace_paths;

/// Concrete runtime that delegates to the real CLI functions.
pub struct RokoCliRuntime {
    config: Config,
    repo_registry: RepoRegistry,
    state_hub: SharedStateHub,
    /// Shared metric registry from AppState so serve-launched runs increment
    /// counters visible on the live `/metrics` endpoint.
    metrics: Option<Arc<roko_core::obs::metrics::MetricRegistry>>,
    // Lazily bound to the workspace used by the first bench task that needs it.
    knowledge_store: OnceLock<KnowledgeStore>,
    // Lazily bound to the workspace used by the first bench task that earns a playbook.
    playbook_store: OnceLock<PlaybookStore>,
    /// Per-workspace extension chains shared by serve status routes and plan runs.
    extension_chains: Arc<
        std::sync::RwLock<
            BTreeMap<PathBuf, Arc<tokio::sync::Mutex<roko_core::extension::ExtensionChain>>>,
        >,
    >,
}

impl RokoCliRuntime {
    #[must_use]
    pub fn new(config: Config, repo_registry: RepoRegistry) -> Self {
        Self::new_with_state_hub(config, repo_registry, SharedStateHub::new_in_process())
    }

    #[must_use]
    pub fn new_with_state_hub(
        config: Config,
        repo_registry: RepoRegistry,
        state_hub: SharedStateHub,
    ) -> Self {
        Self::new_with_state_hub_and_metrics(config, repo_registry, state_hub, Default::default())
    }

    /// Like [`Self::new_with_state_hub`] but also threads a shared
    /// [`MetricRegistry`] so serve-launched runner plans increment counters
    /// visible on the live `/metrics` endpoint.
    #[must_use]
    pub fn new_with_state_hub_and_metrics(
        config: Config,
        repo_registry: RepoRegistry,
        state_hub: SharedStateHub,
        metrics: Option<Arc<roko_core::obs::metrics::MetricRegistry>>,
    ) -> Self {
        Self {
            config,
            repo_registry,
            state_hub,
            metrics,
            knowledge_store: OnceLock::new(),
            playbook_store: OnceLock::new(),
            extension_chains: Arc::new(std::sync::RwLock::new(BTreeMap::new())),
        }
    }

    pub fn into_arc(self) -> Arc<dyn CliRuntime> {
        Arc::new(self)
    }

    /// Resolve, load, and initialize the extension chain before an HTTP
    /// server binds for this workspace.
    pub async fn prepare_workspace_extensions(&self, workdir: &Path) -> anyhow::Result<()> {
        let chain = self.extension_chain_for_workdir(workdir)?;
        crate::runner::extension_loader::initialize_extensions(Some(&chain)).await
    }
}

#[async_trait]
impl CliRuntime for RokoCliRuntime {
    /// Dispatch a single prompt via the v2 `ModelCallService` path.
    ///
    /// This is the converged entry point for HTTP/serve one-shot dispatch.
    /// Uses `dispatch_bench_prompt()` which routes through the same
    /// `ModelCallService` infrastructure as the WorkflowEngine v2 runner.
    async fn run_once(&self, workdir: &Path, prompt: &str) -> anyhow::Result<RunResult> {
        let result = dispatch_bench_prompt(workdir, &self.config, prompt, None).await?;
        Ok(RunResult {
            success: true,
            output_text: Some(result.text),
            usage: Some(RunResultUsage {
                input_tokens: result.input_tokens,
                output_tokens: result.output_tokens,
            }),
            gate_results: Vec::new(),
        })
    }

    async fn run_once_with_config(
        &self,
        workdir: &Path,
        prompt: &str,
        overrides: &BenchConfigOverrides,
    ) -> anyhow::Result<RunResult> {
        // Demo strategy returns simulated results — no LLM dispatch needed.
        if matches!(overrides.strategy, BenchStrategy::Demo) {
            return Ok(simulate_bench_result(prompt));
        }

        // Apply model override if provided by cloning the config.
        let mut config = self.config.clone();
        if let Some(ref model) = overrides.model {
            config.agent.model = Some(model.clone());
        }

        let model_override = overrides.model.clone();
        let result =
            dispatch_bench_prompt(workdir, &config, prompt, model_override.as_deref()).await;

        match result {
            Ok(dispatch) => {
                let output_text = Some(dispatch.text);

                // Extract playbook on success (skip for Minimal strategy).
                if !matches!(overrides.strategy, BenchStrategy::Minimal) {
                    match crate::run::extract_bench_playbook(
                        workdir,
                        prompt,
                        output_text.as_deref(),
                    )
                    .await
                    {
                        Ok(Some(playbook)) => {
                            let playbook_store = self.playbook_store(workdir);
                            if let Err(err) = playbook_store.save_or_merge(&playbook).await {
                                tracing::warn!(
                                    error = %err,
                                    playbook_id = %playbook.id,
                                    "failed to save extracted playbook"
                                );
                            }
                        }
                        Ok(None) => {}
                        Err(err) => {
                            tracing::warn!(error = %err, "failed to extract bench playbook");
                        }
                    }
                }

                Ok(RunResult {
                    success: true,
                    output_text,
                    usage: Some(RunResultUsage {
                        input_tokens: dispatch.input_tokens,
                        output_tokens: dispatch.output_tokens,
                    }),
                    gate_results: Vec::new(),
                })
            }
            Err(err) => {
                let error_msg = format!("{err:#}");

                // Record anti-pattern on failure (skip for Minimal strategy).
                if !matches!(overrides.strategy, BenchStrategy::Minimal) {
                    let knowledge_store = self.knowledge_store(workdir);
                    if let Err(record_err) = knowledge_store.record_anti_pattern_from_failure(
                        "bench-dispatch",
                        prompt,
                        "model_call",
                        &error_msg,
                        Some(&error_msg),
                    ) {
                        tracing::warn!(
                            error = %record_err,
                            "failed to save anti-pattern from bench dispatch failure"
                        );
                    }
                }

                Ok(RunResult {
                    success: false,
                    output_text: Some(error_msg),
                    usage: None,
                    gate_results: Vec::new(),
                })
            }
        }
    }

    async fn generate_plan_from_prd(
        &self,
        workdir: &Path,
        slug: &str,
        prd_path: &Path,
    ) -> anyhow::Result<PlanGenerationResult> {
        let plans_root = workspace_paths::plans_dir(workdir);
        let before = snapshot_plan_artifacts(&plans_root);
        let generated_root = prd::generate_plan_from_prd(slug, prd_path, false).await?;
        let after = snapshot_plan_artifacts(&generated_root);

        let mut plan_targets = changed_plan_targets(&generated_root, &before, &after);
        if plan_targets.is_empty() {
            plan_targets.push(generated_root.clone());
        }

        Ok(PlanGenerationResult {
            plans_root: generated_root,
            artifacts: plan_artifact_paths(&plan_targets),
            plan_targets,
        })
    }

    async fn run_plan(
        &self,
        workdir: &Path,
        plan_target: &Path,
    ) -> anyhow::Result<PlanExecutionResult> {
        let workdir = workdir.to_path_buf();
        let plan_target = plan_target.to_path_buf();
        let config = self.config.clone();
        let repo_registry = self.repo_registry.clone();
        let state_hub = self.state_hub.clone();
        let metrics = self.metrics.clone();
        let extension_chain = self.extension_chain_for_workdir(&workdir)?;
        tokio::task::spawn_blocking(move || {
            run_plan_on_local_runtime(
                workdir,
                plan_target,
                config,
                repo_registry,
                state_hub,
                metrics,
                extension_chain,
            )
        })
        .await
        .map_err(|err| anyhow::anyhow!("plan execution worker failed: {err}"))?
    }

    async fn run_trigger_graph(
        &self,
        _workdir: &Path,
        graph: &Path,
        event: &roko_core::trigger::TriggerEvent,
    ) -> anyhow::Result<PlanExecutionResult> {
        let output =
            crate::graph_command::execute_graph(graph, &self.state_hub, Some(event), None).await?;
        Ok(PlanExecutionResult {
            success: output.success,
            output_text: Some(output.summary()),
            gate_results: Vec::new(),
        })
    }

    async fn run_trigger_graph_scoped(
        &self,
        _workdir: &Path,
        graph: &Path,
        event: &roko_core::trigger::TriggerEvent,
        scope: &TriggerExecutionScope,
    ) -> anyhow::Result<PlanExecutionResult> {
        let output = crate::graph_command::execute_graph(
            graph,
            &self.state_hub,
            Some(event),
            scope.capabilities.as_ref(),
        )
        .await?;
        Ok(PlanExecutionResult {
            success: output.success,
            output_text: Some(output.summary()),
            gate_results: Vec::new(),
        })
    }

    fn session_status(&self, workdir: PathBuf) -> SessionStatusInfo {
        let ss = collect_session_status(&workdir);
        SessionStatusInfo {
            session_id: ss.session_id,
            workdir: ss.workdir,
            daemon_running: ss.daemon_running,
            signal_count: ss.signal_count,
            episode_count: ss.episode_count,
            last_episode_passed: ss.last_episode_passed,
        }
    }

    fn dashboard_scaffold(&self, workdir: &Path) -> DashboardInfo {
        let scaffold = DashboardScaffold::new_in(workdir);
        DashboardInfo {
            rendered: scaffold.render_overview_text(),
        }
    }

    fn extension_chain(
        &self,
        workdir: &Path,
    ) -> Option<Arc<tokio::sync::Mutex<roko_core::extension::ExtensionChain>>> {
        self.extension_chain_for_workdir(workdir)
            .map_err(|error| {
                tracing::warn!(
                    workdir = %workdir.display(),
                    %error,
                    "failed to load workspace extensions"
                );
            })
            .ok()
    }

    fn resolve_repo_workdir(&self, repo_full_name: &str) -> Option<PathBuf> {
        self.repo_registry
            .find_by_full_name(repo_full_name)
            .map(|entry| entry.root.clone())
    }

    fn repo_roko_config(&self, repo_name: &str) -> Option<RokoConfig> {
        self.repo_registry
            .get(repo_name)
            .and_then(|entry| entry.roko_config.clone())
    }

    fn list_repos(&self) -> Vec<RepoInfo> {
        self.repo_registry
            .repos()
            .iter()
            .map(|entry| RepoInfo {
                name: entry.config.name.clone(),
                path: entry.root.clone(),
                branch: entry.config.branch.clone(),
            })
            .collect()
    }

    async fn list_plans(&self, workdir: &std::path::Path) -> anyhow::Result<Vec<PlanSummaryDto>> {
        // A workspace with no plans directory is empty, not broken: report an empty
        // list rather than a 500. Every other discovery failure is a real error and
        // must stay visible to the operator.
        let mut summaries = match crate::plan::summarize_discovered_plans(workdir) {
            Ok(summaries) => summaries,
            Err(crate::orchestrator::DiscoveryError::DirMissing(_)) => Vec::new(),
            Err(error) => {
                return Err(anyhow::Error::new(error)
                    .context(format!("failed to discover plans in {}", workdir.display())));
            }
        };
        summaries.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(summaries.into_iter().map(plan_summary_to_dto).collect())
    }

    async fn load_plan_summary(
        &self,
        workdir: &std::path::Path,
        plan_id: &str,
    ) -> anyhow::Result<Option<PlanSummaryDto>> {
        let Some(plan_info) =
            crate::plan::discover_plan_by_id(workdir, plan_id).with_context(|| {
                format!(
                    "failed to discover plan '{}' in {}",
                    plan_id,
                    workdir.display()
                )
            })?
        else {
            return Ok(None);
        };
        let mut summary = crate::plan::summarize_plan_info(&plan_info);
        crate::plan::overlay_graph_checkpoint_status(workdir, std::slice::from_mut(&mut summary));
        Ok(Some(plan_summary_to_dto(summary)))
    }

    async fn load_plan_tasks(
        &self,
        workdir: &std::path::Path,
        plan_id: &str,
    ) -> anyhow::Result<Option<PlanTasksDto>> {
        let Some(plan_info) =
            crate::plan::discover_plan_by_id(workdir, plan_id).with_context(|| {
                format!(
                    "failed to discover plan '{}' in {}",
                    plan_id,
                    workdir.display()
                )
            })?
        else {
            return Ok(None);
        };

        let Some(tasks_path) = crate::plan::tasks_path(&plan_info) else {
            return Ok(None);
        };

        if !tasks_path.is_file() {
            return Ok(None);
        }

        let tasks_file = crate::task_parser::TasksFile::parse(&tasks_path)
            .with_context(|| format!("failed to parse tasks at {}", tasks_path.display()))?;

        let task_count = tasks_file.tasks.len();
        let tasks = tasks_file
            .tasks
            .iter()
            .map(|task| PlanTaskDto {
                id: task.id.clone(),
                title: task.title.clone(),
                description: task.description.clone(),
                role: task.role.clone(),
                tier: task.tier.clone(),
                status: task.status.clone(),
                depends_on: task.depends_on.clone(),
                files: task.files.clone(),
                completed: task.status == "done",
                verify_phases: task.verify.iter().map(|v| v.phase.clone()).collect(),
            })
            .collect();

        Ok(Some(PlanTasksDto {
            plan_id: plan_id.to_string(),
            task_count,
            tasks,
        }))
    }
}

impl RokoCliRuntime {
    fn extension_chain_for_workdir(
        &self,
        workdir: &Path,
    ) -> anyhow::Result<Arc<tokio::sync::Mutex<roko_core::extension::ExtensionChain>>> {
        let key = workdir
            .canonicalize()
            .unwrap_or_else(|_| workdir.to_path_buf());
        if let Some(chain) = self
            .extension_chains
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&key)
            .cloned()
        {
            return Ok(chain);
        }

        let config = load_effective_roko_config(workdir, &self.repo_registry)?;
        let chain = load_serve_extension_chain(workdir, &config)?;
        let chain = Arc::new(tokio::sync::Mutex::new(chain));
        let mut chains = self
            .extension_chains
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        Ok(chains.entry(key).or_insert_with(|| chain).clone())
    }

    fn knowledge_store(&self, workdir: &Path) -> &KnowledgeStore {
        // `run_once_with_config` is the only bench entry point, so this store can
        // be initialized on demand from the workspace passed into that call.
        self.knowledge_store
            .get_or_init(|| KnowledgeStore::for_workdir(workdir))
    }

    fn playbook_store(&self, workdir: &Path) -> &PlaybookStore {
        // `run_once_with_config` is the only bench entry point that creates
        // playbooks, so this store can be initialized on demand from the same
        // workspace.
        self.playbook_store
            .get_or_init(|| PlaybookStore::new(RokoLayout::for_project(workdir).playbooks_dir()))
    }
}

fn load_serve_extension_chain(
    workdir: &Path,
    config: &RokoConfig,
) -> anyhow::Result<roko_core::extension::ExtensionChain> {
    let registry_base =
        crate::runner::extension_registry::registry_base_url(config.relay.url.as_deref());
    let registry_failures = crate::runner::extension_registry::fetch_missing_registry_extensions(
        workdir,
        &config.agent.extensions,
        registry_base.as_deref(),
    )
    .into_iter()
    .map(
        |(extension, message)| crate::runner::extension_loader::ExtensionStartupFailure {
            extension,
            stage: "registry_fetch",
            message,
        },
    )
    .collect();
    let mut chain = roko_core::extension::ExtensionChain::new();
    let report = crate::runner::extension_loader::load_extensions_for_startup(
        workdir,
        &config.agent.extensions,
        &[],
        &mut chain,
        registry_failures,
    );
    if !report.required_failures.is_empty() {
        let failures = report
            .required_failures
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("; ");
        anyhow::bail!("workspace extension startup failed: {failures}");
    }
    Ok(chain)
}

fn run_plan_on_local_runtime(
    workdir: PathBuf,
    plan_target: PathBuf,
    _config: Config,
    repo_registry: RepoRegistry,
    _state_hub: SharedStateHub,
    _metrics: Option<Arc<roko_core::obs::metrics::MetricRegistry>>,
    _extension_chain: Arc<tokio::sync::Mutex<roko_core::extension::ExtensionChain>>,
) -> anyhow::Result<PlanExecutionResult> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let local = tokio::task::LocalSet::new();
    local.block_on(&runtime, async move {
        let execution_root = prepare_plan_execution_root(&workdir, &plan_target)?;
        ensure_git_repo_for_runner(&workdir);

        // Load plans now to capture plan IDs for post-run gate evidence collection.
        // run_graph_plan also loads plans internally from the same execution_root.
        let plans = crate::runner::plan_loader::load_plans(&execution_root)?;
        let plan_ids = plans
            .iter()
            .map(|plan| plan.id.clone())
            .collect::<BTreeSet<_>>();

        let roko_config = load_effective_roko_config(&workdir, &repo_registry)?;
        let dangerously_skip_permissions = roko_config.runner.dangerously_skip_permissions;

        let events_offset = runner_events_offset(&workdir);

        let exit_code =
            crate::graph_execution::run_graph_plan(crate::graph_execution::GraphPlanRunParams {
                plans_dir: execution_root,
                workdir: workdir.clone(),
                // Suppress interactive output: this runs inside an HTTP handler.
                quiet: true,
                json: false,
                resume_plan: None,
                fresh: false,
                force_resume: false,
                max_retries: None,
                // 0 → use each plan's meta.max_parallel default.
                max_tasks: 0,
                budget_override: None,
                no_budget: false,
                cli_model_override: None,
                dangerously_skip_permissions,
                log_file: None,
                worktree_per_task: false,
                rich_topology: false,
                // Never launch an interactive TUI from an HTTP handler.
                no_tui: true,
            })
            .await?;

        let success = exit_code == crate::exit_codes::EXIT_SUCCESS;

        let gate_results = collect_runner_gate_results(&workdir, events_offset, &plan_ids)
            .unwrap_or_else(|err| {
                tracing::warn!(error = %err, "failed to collect runner gate evidence");
                Vec::new()
            });

        let task_count: usize = plans.iter().map(|p| p.tasks.tasks.len()).sum();
        let output_text = Some(format!(
            "graph engine plan execution {}: {} plan(s), {} task(s), {} gate results",
            if success { "succeeded" } else { "failed" },
            plans.len(),
            task_count,
            gate_results.len(),
        ));

        Ok(PlanExecutionResult {
            success,
            output_text,
            gate_results,
        })
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PlanArtifactSnapshot {
    modified: Option<SystemTime>,
    len: u64,
}

fn snapshot_plan_artifacts(root: &Path) -> BTreeMap<PathBuf, PlanArtifactSnapshot> {
    let mut out = BTreeMap::new();
    collect_plan_artifact_snapshots(root, root, &mut out);
    out
}

fn collect_plan_artifact_snapshots(
    root: &Path,
    dir: &Path,
    out: &mut BTreeMap<PathBuf, PlanArtifactSnapshot>,
) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        if meta.is_dir() {
            collect_plan_artifact_snapshots(root, &path, out);
            continue;
        }
        if !meta.is_file() || !is_plan_artifact_file(&path) {
            continue;
        }
        let rel = path.strip_prefix(root).unwrap_or(&path).to_path_buf();
        out.insert(
            rel,
            PlanArtifactSnapshot {
                modified: meta.modified().ok(),
                len: meta.len(),
            },
        );
    }
}

fn is_plan_artifact_file(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    name == "tasks.toml" || name == "plan.md" || path.extension().is_some_and(|ext| ext == "md")
}

fn changed_plan_targets(
    root: &Path,
    before: &BTreeMap<PathBuf, PlanArtifactSnapshot>,
    after: &BTreeMap<PathBuf, PlanArtifactSnapshot>,
) -> Vec<PathBuf> {
    let mut targets = BTreeSet::new();
    for (rel, snapshot) in after {
        let changed = before.get(rel).is_none_or(|old| old != snapshot);
        if !changed {
            continue;
        }
        if rel
            .file_name()
            .is_some_and(|name| name == "tasks.toml" || name == "plan.md")
        {
            if let Some(parent) = rel.parent() {
                targets.insert(root.join(parent));
            }
        } else {
            targets.insert(root.join(rel));
        }
    }
    targets.into_iter().collect()
}

fn plan_artifact_paths(targets: &[PathBuf]) -> Vec<PathBuf> {
    let mut artifacts = BTreeSet::new();
    for target in targets {
        if target.extension().is_some_and(|ext| ext == "md") {
            artifacts.insert(target.clone());
            continue;
        }
        artifacts.insert(target.join("plan.md"));
        artifacts.insert(target.join("tasks.toml"));
    }
    artifacts.into_iter().collect()
}

fn prepare_plan_execution_root(workdir: &Path, plan_target: &Path) -> anyhow::Result<PathBuf> {
    let absolute_target = if plan_target.is_absolute() {
        plan_target.to_path_buf()
    } else {
        workdir.join(plan_target)
    };

    if absolute_target.is_dir() && !absolute_target.join("tasks.toml").is_file() {
        return Ok(absolute_target);
    }

    let copy_source = if absolute_target.is_file() {
        let parent = absolute_target.parent().ok_or_else(|| {
            anyhow::anyhow!(
                "plan target has no parent directory: {}",
                absolute_target.display()
            )
        })?;
        if !parent.join("tasks.toml").is_file() {
            anyhow::bail!(
                "runner v2 requires a tasks.toml plan directory; file target is not inside one: {}",
                absolute_target.display()
            );
        }
        parent.to_path_buf()
    } else if absolute_target.is_dir() {
        absolute_target.clone()
    } else {
        anyhow::bail!("plan target does not exist: {}", absolute_target.display());
    };

    let plan_base = copy_source
        .file_stem()
        .or_else(|| copy_source.file_name())
        .and_then(|name| name.to_str())
        .map(sanitize_plan_base)
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "job-plan".to_string());
    let run_root = RokoLayout::for_project(workdir)
        .plan_runs_dir()
        .join(format!("{plan_base}-{}", unique_suffix()));
    std::fs::create_dir_all(&run_root)?;

    copy_dir_recursive(&copy_source, &run_root.join(&plan_base))?;

    Ok(run_root)
}

fn ensure_git_repo_for_runner(workdir: &Path) {
    if workdir.join(".git").exists() {
        return;
    }

    tracing::info!(workdir = %workdir.display(), "initializing git repo for runner tooling");
    for args in [
        &["init"][..],
        &["add", "-A"][..],
        &[
            "commit",
            "-m",
            "init (auto-created by roko)",
            "--allow-empty",
        ][..],
    ] {
        let _ = std::process::Command::new("git")
            .args(args)
            .current_dir(workdir)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }
}

fn load_effective_roko_config(
    workdir: &Path,
    repo_registry: &RepoRegistry,
) -> anyhow::Result<RokoConfig> {
    use roko_core::config::loader::{LoadOptions, load_config_file, load_config_unified};

    // If workdir matches a configured repo entry, load that repo's roko.toml
    // directly via the unified loader (which applies global merge and env overrides).
    if let Some(repo_config) = repo_roko_config_for_workdir_path(workdir, repo_registry) {
        return load_config_file(&repo_config, &LoadOptions::default())
            .map_err(|e| anyhow::anyhow!("{e}"));
    }

    // Standard path: unified loader applies ancestor walk, ROKO_CONFIG env,
    // global merge, named env overrides, and hierarchical ROKO__* overrides.
    load_config_unified(workdir).map_err(|e| anyhow::anyhow!("{e}"))
}

/// Return the roko.toml path for the repo entry that owns `workdir`, if any.
///
/// Used by `load_effective_roko_config` to route per-repo config loading
/// through the unified loader rather than a bare file read.
fn repo_roko_config_for_workdir_path(
    workdir: &Path,
    repo_registry: &RepoRegistry,
) -> Option<PathBuf> {
    let canonical_workdir = workdir
        .canonicalize()
        .unwrap_or_else(|_| workdir.to_path_buf());
    repo_registry
        .repos()
        .iter()
        .find(|entry| canonical_workdir == entry.root || canonical_workdir.starts_with(&entry.root))
        .and_then(|entry| entry.roko_config_path.clone())
}

/// Produce a deterministic, realistic-looking simulated result for demo mode.
///
/// No LLM dispatch is performed. ~88% of tasks "pass" based on a hash of the
/// prompt, and token counts / cost are synthetic but plausible.
fn simulate_bench_result(prompt: &str) -> RunResult {
    // Deterministic "randomness" from prompt bytes.
    let seed: u64 = prompt.bytes().map(|b| b as u64).sum();
    let passed = (seed % 100) < 88;
    let input_tokens = 800 + (seed % 600);
    let output_tokens = 200 + (seed % 400);

    RunResult {
        success: passed,
        output_text: Some(if passed {
            format!(
                "[demo] Simulated successful completion for prompt ({input_tokens}+{output_tokens} tokens)"
            )
        } else {
            "[demo] Simulated failure — gate check did not pass".to_string()
        }),
        usage: Some(RunResultUsage {
            input_tokens,
            output_tokens,
        }),
        gate_results: Vec::new(),
    }
}

/// Dispatch a bench prompt via the `ModelCallService` path.
///
/// Uses the same ModelCallService that `WorkflowEngine` uses, preserving
/// routing, budget, and feedback behavior.
pub(crate) async fn dispatch_bench_prompt(
    workdir: &Path,
    config: &Config,
    prompt: &str,
    model_override: Option<&str>,
) -> anyhow::Result<BenchDispatchResult> {
    use crate::learning_helpers::{
        capture_runtime_model_slugs, provider_id_for_model, record_persisted_provider_health,
    };
    use roko_agent::model_call_service::ModelCallService;
    use roko_core::agent::resolve_model;
    use roko_core::config::schema::RokoConfig;
    use roko_core::foundation::{
        ChatMessage, FeedbackSink, MessageRole, ModelCallRequest, ModelCaller, caller,
    };
    use roko_learn::cascade_router::CascadeRouter;
    use roko_learn::feedback_service::FeedbackService;

    // Build a RokoConfig from CLI config (same pattern as dispatch_v2.rs).
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
    if let Some(ref model) = model_override
        .map(ToString::to_string)
        .or_else(|| config.agent.model.clone())
    {
        model_config.agent.default_model = model.clone();
    }

    let model_key = model_override
        .map(ToString::to_string)
        .or_else(|| config.agent.model.clone())
        .unwrap_or_else(|| model_config.agent.default_model.clone());
    let model = resolve_model(&model_config, &model_key).slug;

    // Set up cascade router for learning.
    let cascade_path = workdir
        .join(".roko")
        .join("learn")
        .join("cascade-router.json");
    let cascade_model_slugs = capture_runtime_model_slugs(&model_config, &model);
    let cascade_router = (!cascade_model_slugs.is_empty()).then(|| {
        Arc::new(CascadeRouter::load_or_new(
            &cascade_path,
            cascade_model_slugs,
        ))
    });

    // Build feedback sink.
    let feedback_service = FeedbackService::from_roko_dir(&workdir.join(".roko"));
    let feedback_sink: Arc<dyn FeedbackSink> = match &cascade_router {
        Some(router) => Arc::new(feedback_service.with_cascade_router(Arc::clone(router))),
        None => Arc::new(feedback_service),
    };

    // Build and call ModelCallService.
    let cost_table = roko_agent::CostTable::from_config_with_defaults(&model_config.models);
    let mut service = ModelCallService::new(model.clone())
        .with_config(model_config.clone())
        .with_working_dir(workdir)
        .with_immune_root(workdir)
        .with_cost_table(cost_table)
        .with_feedback_sink(feedback_sink)
        .with_inference_observer(Arc::new(
            crate::inference_observer::RuntimeEventInferenceObserver::new(),
        ))
        .with_dangerously_skip_permissions(config.runner.dangerously_skip_permissions);
    if let Some(ref mcp_path) = config.agent.mcp_config {
        service = service.with_mcp_config(mcp_path.clone());
    }

    let request = ModelCallRequest {
        model: model.clone(),
        system: None,
        messages: vec![ChatMessage {
            role: MessageRole::User,
            content: prompt.to_string(),
        }],
        max_tokens: None,
        caller: Some(caller::CLI.to_string()),
        ..Default::default()
    };

    let call_result = service.call(request).await;

    // Persist cascade router observations.
    if let Some(router) = &cascade_router
        && let Err(err) = router.save(&cascade_path)
    {
        tracing::warn!(
            path = %cascade_path.display(),
            error = %err,
            "failed to persist bench cascade observation"
        );
    }

    let response = match call_result {
        Ok(response) => {
            if let Some(provider) = provider_id_for_model(&model_config, &response.model) {
                let _ = record_persisted_provider_health(workdir, &provider, true);
            }
            response
        }
        Err(err) => {
            if let Some(provider) = provider_id_for_model(&model_config, &model) {
                let _ = record_persisted_provider_health(workdir, &provider, false);
            }
            return Err(err).context("ModelCallService bench dispatch failed");
        }
    };

    Ok(BenchDispatchResult {
        text: response.content,
        input_tokens: response.usage.input_tokens,
        output_tokens: response.usage.output_tokens,
    })
}

/// Result from dispatching a bench prompt via `ModelCallService`.
pub(crate) struct BenchDispatchResult {
    pub(crate) text: String,
    pub(crate) input_tokens: u64,
    pub(crate) output_tokens: u64,
}

fn non_empty_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn runner_events_offset(workdir: &Path) -> u64 {
    std::fs::metadata(runner_events_path(workdir))
        .map(|metadata| metadata.len())
        .unwrap_or(0)
}

fn runner_events_path(workdir: &Path) -> PathBuf {
    RokoLayout::for_project(workdir).events_jsonl_path()
}

fn collect_runner_gate_results(
    workdir: &Path,
    offset: u64,
    plan_ids: &BTreeSet<String>,
) -> anyhow::Result<Vec<RuntimeGateResult>> {
    let events = read_runner_events_since(&runner_events_path(workdir), offset)?;
    if events.trim().is_empty() {
        return Ok(Vec::new());
    }

    let parsed = events
        .lines()
        .filter_map(|line| {
            serde_json::from_str::<RunnerEvent>(line)
                .map_err(|err| {
                    tracing::debug!(error = %err, "skipping malformed runner event");
                    err
                })
                .ok()
        })
        .collect::<Vec<_>>();
    let run_ids = matching_run_ids(&parsed, plan_ids);
    let mut final_results = BTreeMap::new();

    for event in parsed {
        let RunnerEvent::GateCompleted {
            run_id,
            attempt,
            kind,
            rung,
            passed,
            duration_ms,
            output,
            verdicts,
            ..
        } = event
        else {
            continue;
        };

        if !run_ids.is_empty() && !run_ids.contains(&run_id) {
            continue;
        }
        if !plan_ids.contains(&attempt.plan_id) {
            continue;
        }

        let kind_label = gate_kind_label(kind);
        if verdicts.is_empty() {
            let gate_name = "gate".to_string();
            let key = gate_evidence_key(
                &attempt.plan_id,
                &attempt.task_id,
                kind_label,
                rung,
                &gate_name,
            );
            final_results.insert(
                key,
                RuntimeGateResult {
                    gate: gate_evidence_label(
                        &attempt.plan_id,
                        &attempt.task_id,
                        kind_label,
                        rung,
                        &gate_name,
                    ),
                    passed,
                    detail: gate_detail(
                        kind_label,
                        attempt.attempt,
                        rung,
                        duration_ms,
                        None,
                        None,
                        &output,
                    ),
                },
            );
            continue;
        }

        for verdict in verdicts {
            let key = gate_evidence_key(
                &attempt.plan_id,
                &attempt.task_id,
                kind_label,
                rung,
                &verdict.gate_name,
            );
            final_results.insert(
                key,
                RuntimeGateResult {
                    gate: gate_evidence_label(
                        &attempt.plan_id,
                        &attempt.task_id,
                        kind_label,
                        rung,
                        &verdict.gate_name,
                    ),
                    passed: verdict.passed,
                    detail: gate_detail(
                        kind_label,
                        attempt.attempt,
                        rung,
                        duration_ms,
                        Some(verdict.summary.as_str()),
                        verdict.error_digest.as_deref(),
                        &output,
                    ),
                },
            );
        }
    }

    Ok(final_results.into_values().collect())
}

fn read_runner_events_since(path: &Path, offset: u64) -> anyhow::Result<String> {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(String::new()),
        Err(err) => return Err(err).with_context(|| format!("open {}", path.display())),
    };
    let len = file
        .metadata()
        .with_context(|| format!("stat {}", path.display()))?
        .len();
    file.seek(SeekFrom::Start(offset.min(len)))
        .with_context(|| format!("seek {}", path.display()))?;

    let mut events = String::new();
    file.read_to_string(&mut events)
        .with_context(|| format!("read {}", path.display()))?;
    Ok(events)
}

fn matching_run_ids(events: &[RunnerEvent], plan_ids: &BTreeSet<String>) -> BTreeSet<String> {
    events
        .iter()
        .filter_map(|event| {
            let RunnerEvent::RunStarted {
                run_id,
                plan_ids: event_plan_ids,
                ..
            } = event
            else {
                return None;
            };
            let event_plan_ids = event_plan_ids.iter().cloned().collect::<BTreeSet<_>>();
            (event_plan_ids.len() == plan_ids.len()
                && event_plan_ids
                    .iter()
                    .all(|plan_id| plan_ids.contains(plan_id)))
            .then(|| run_id.clone())
        })
        .collect()
}

fn gate_kind_label(kind: GateCompletionKind) -> &'static str {
    match kind {
        GateCompletionKind::Preflight => "preflight",
        GateCompletionKind::Gate => "gate",
        GateCompletionKind::PlanVerify => "plan_verify",
        GateCompletionKind::Merge => "merge",
    }
}

fn gate_evidence_key(
    plan_id: &str,
    task_id: &str,
    kind: &str,
    rung: u32,
    gate_name: &str,
) -> (String, String, String, u32, String) {
    (
        plan_id.to_string(),
        task_id.to_string(),
        kind.to_string(),
        rung,
        gate_name.to_string(),
    )
}

fn gate_evidence_label(
    plan_id: &str,
    task_id: &str,
    kind: &str,
    rung: u32,
    gate_name: &str,
) -> String {
    format!("{plan_id}:{task_id}:{kind}:{rung}:{gate_name}")
}

fn gate_detail(
    kind: &str,
    attempt: u32,
    rung: u32,
    duration_ms: u64,
    summary: Option<&str>,
    error_digest: Option<&str>,
    output: &str,
) -> String {
    let evidence = summary
        .and_then(non_empty_string)
        .or_else(|| error_digest.and_then(non_empty_string))
        .or_else(|| first_non_empty_line(output))
        .unwrap_or_else(|| "gate completed".to_string());
    format!("{kind} attempt {attempt}, rung {rung}, {duration_ms}ms: {evidence}")
}

fn first_non_empty_line(output: &str) -> Option<String> {
    output
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(ToOwned::to_owned)
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        let meta = entry.metadata()?;
        if meta.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else if meta.is_file() {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

fn sanitize_plan_base(value: &str) -> String {
    let mut out = value
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_ascii_lowercase();
    if out
        .chars()
        .next()
        .is_none_or(|ch| !ch.is_ascii_alphanumeric())
    {
        out.insert_str(0, "job-");
    }
    out
}

fn unique_suffix() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    format!("{}-{millis}", std::process::id())
}

/// Map a [`crate::plan::PlanSummary`] to the wire-format [`PlanSummaryDto`].
fn plan_summary_to_dto(summary: crate::plan::PlanSummary) -> PlanSummaryDto {
    PlanSummaryDto {
        id: summary.id,
        title: summary.title,
        task_count: summary.task_count,
        tasks_done: summary.tasks_done,
        tasks_failed: summary.tasks_failed,
        completed: summary.completed,
        status: summary.status,
        superseded_by: summary.superseded_by,
        old_format: summary.old_format,
        last_error: summary.last_error,
    }
}

#[cfg(test)]
mod tests_extension_startup {
    use super::*;

    fn write_native_manifest(workdir: &Path, name: &str, optional: bool) {
        let directory = RokoLayout::for_project(workdir).extensions_dir().join(name);
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(
            directory.join("extension.toml"),
            format!(
                r#"[extension]
name = "{name}"
version = "1.0.0"
layer = "action"
tier = "native_rust"
optional = {optional}
"#
            ),
        )
        .unwrap();
    }

    fn write_failing_init_wasm(workdir: &Path, name: &str) {
        let directory = RokoLayout::for_project(workdir).extensions_dir().join(name);
        std::fs::create_dir_all(&directory).unwrap();
        let output = r#"{"error":"fixture init failure"}"#;
        let escaped = output
            .as_bytes()
            .iter()
            .map(|byte| format!("\\{byte:02x}"))
            .collect::<String>();
        let wat = format!(
            r#"(module
                (memory (export "memory") 1)
                (func (export "roko_alloc") (param i32) (result i32) i32.const 4096)
                (data (i32.const 0) "{escaped}")
                (func (export "on_init") (param i32 i32) (result i64)
                    i64.const {length}))"#,
            length = output.len(),
        );
        std::fs::write(directory.join("hook.wasm"), wat::parse_str(wat).unwrap()).unwrap();
        std::fs::write(
            directory.join("extension.toml"),
            format!(
                r#"[extension]
name = "{name}"
version = "1.0.0"
layer = "foundation"
tier = "wasm"
optional = false

[extension.config]
module = "hook.wasm"
memory_mb = 1
fuel = 100000
hooks = ["on_init"]
"#,
            ),
        )
        .unwrap();
    }

    #[test]
    fn serve_startup_rejects_configured_required_extension_load_failure() {
        let tmp = tempfile::tempdir().unwrap();
        write_native_manifest(tmp.path(), "required-native", false);
        let mut config = RokoConfig::default();
        config.agent.extensions = vec!["required-native".to_string()];

        let error = match load_serve_extension_chain(tmp.path(), &config) {
            Ok(_) => panic!("required extension load failure must reject serve startup"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("required-native"));
        assert!(error.to_string().contains("native_load"));
    }

    #[test]
    fn serve_startup_isolates_configured_optional_extension_load_failure() {
        let tmp = tempfile::tempdir().unwrap();
        write_native_manifest(tmp.path(), "optional-native", true);
        let mut config = RokoConfig::default();
        config.agent.extensions = vec!["optional-native".to_string()];

        let chain = load_serve_extension_chain(tmp.path(), &config).unwrap();
        assert!(chain.is_empty());
    }

    // Ignored: WASM extension system is dormant; fixtures not yet built for current wasmtime version
    #[ignore]
    #[tokio::test]
    async fn serve_preflight_rejects_required_init_failure_before_bind() {
        let tmp = tempfile::tempdir().unwrap();
        write_failing_init_wasm(tmp.path(), "init-fail");
        std::fs::write(
            tmp.path().join("roko.toml"),
            "[agent]\nextensions = [\"init-fail\"]\n",
        )
        .unwrap();
        let runtime = RokoCliRuntime::new(Config::default(), RepoRegistry::default());

        let error = runtime
            .prepare_workspace_extensions(tmp.path())
            .await
            .unwrap_err();
        assert!(error.to_string().contains("init-fail"));
        assert!(error.to_string().contains("fixture init failure"));
    }
}
