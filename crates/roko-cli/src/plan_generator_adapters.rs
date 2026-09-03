//! Concrete [`PlanGeneratorAdapter`] implementations for every production
//! call site inventoried by backlog #280.
//!
//! Each adapter owns persistence, execution triggers, and user rendering for
//! its lane while delegating generation, repair, and validation to the shared
//! [`PlanGenerator`] service. The adapter key constants come from
//! [`crate::plan_generator::adapter_keys`].
//!
//! # Lane B (Direct / PRD / research)
//!
//! - [`PrdDefaultAdapter`] — `prd_default` / `prd_model` / `prd_replan`:
//!   persists to the workspace plans directory and optionally updates PRD
//!   frontmatter.
//! - [`PlanGenerateAdapter`] — `plan_generate`: persists to the workspace
//!   plans directory from a prompt or file source.
//! - [`DoStandardAdapter`] — `do_standard` / `do_complex`: persists and
//!   optionally auto-executes from `roko do`.
//! - [`CliServeRuntimeAdapter`] — `cli_serve_runtime`: delegates to the
//!   serve runtime's plan generation result type.
//!
//! # Lane C (Servers)
//!
//! - [`ServeRuntimeAdapter`] — `serve_runtime`: delegates through the
//!   `CliRuntime` trait for job runner callers.
//! - [`ServeHttpAdapter`] — `serve_http`: background-spawned generation
//!   from the HTTP `POST /api/plans/generate` route.
//!
//! # Shared patterns
//!
//! All adapters:
//! - Are `Send + Sync` and implement [`PlanGeneratorAdapter`].
//! - Accept [`ResolvedExecutionOverrides`] where the caller had explicit
//!   model/flag resolution from #262.
//! - Return the host-owned output path from `persist`.
//! - Never call the provider directly — that remains in the generation
//!   function until #243 provides `RuntimeServices`.
//!
//! This module is #283's exclusive edit surface for host adapter code.

use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result};
use roko_core::io::atomic_write_str;

use crate::plan_generator::{
    PlanGeneratorAdapter, PlanGeneratorOutcome, adapter_keys,
};
use crate::workspace_paths;

// ---------------------------------------------------------------------------
// Lane B: Direct / PRD / research adapters
// ---------------------------------------------------------------------------

/// PRD plan generation adapter — handles `prd_default`, `prd_model`, and
/// `prd_replan` call sites.
///
/// Persists `tasks.toml` (and optional `plan.md`) under the workspace
/// `plans/<slug>/` directory, injects `source_prd` into the meta section,
/// and writes a minimal `plan.md` when the generator does not produce one.
pub struct PrdDefaultAdapter {
    adapter_key: String,
}

impl PrdDefaultAdapter {
    /// Create an adapter for the default PRD path.
    #[must_use]
    pub fn new_default() -> Self {
        Self {
            adapter_key: adapter_keys::PRD_DEFAULT.to_string(),
        }
    }

    /// Create an adapter for the explicit-model PRD path.
    #[must_use]
    pub fn new_model() -> Self {
        Self {
            adapter_key: adapter_keys::PRD_MODEL.to_string(),
        }
    }

    /// Create an adapter for the replan PRD path.
    #[must_use]
    pub fn new_replan() -> Self {
        Self {
            adapter_key: adapter_keys::PRD_REPLAN.to_string(),
        }
    }
}

impl PlanGeneratorAdapter for PrdDefaultAdapter {
    fn adapter_key(&self) -> &str {
        &self.adapter_key
    }

    fn persist(
        &self,
        outcome: &PlanGeneratorOutcome,
        workdir: &Path,
    ) -> Result<PathBuf> {
        let plans_root = workspace_paths::plans_dir(workdir);
        let plan_dir = plans_root.join(&outcome.slug);
        std::fs::create_dir_all(&plan_dir)
            .with_context(|| format!("create plan dir {}", plan_dir.display()))?;

        // Inject source_prd into the [meta] section.
        let tasks_toml = outcome
            .tasks_toml
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("no tasks_toml in outcome"))?;

        let tasks_toml = if tasks_toml.contains("source_prd") {
            tasks_toml.to_string()
        } else {
            tasks_toml.replacen(
                "[meta]",
                &format!("[meta]\nsource_prd = \"{}\"", outcome.slug),
                1,
            )
        };

        atomic_write_str(&plan_dir.join("tasks.toml"), &tasks_toml)
            .with_context(|| format!("write tasks.toml to {}", plan_dir.display()))?;

        // Write plan.md (from outcome or minimal fallback).
        let plan_md = outcome.plan_md.clone().unwrap_or_else(|| {
            format!(
                "---\nplan: {slug}\ntitle: {slug}\n---\n\n# {slug}\n\nGenerated plan.\n",
                slug = outcome.slug
            )
        });
        atomic_write_str(&plan_dir.join("plan.md"), &plan_md)
            .with_context(|| format!("write plan.md to {}", plan_dir.display()))?;

        Ok(plans_root)
    }

    fn report(&self, message: &str) {
        eprintln!("  [prd] {message}");
    }
}

/// Plan generate adapter — handles `plan_generate` call site from
/// `commands/plan.rs`.
///
/// Persists to the workspace plans directory. The host selects, validates,
/// and executes the returned plan.
pub struct PlanGenerateAdapter;

impl PlanGeneratorAdapter for PlanGenerateAdapter {
    fn adapter_key(&self) -> &str {
        adapter_keys::PLAN_GENERATE
    }

    fn persist(
        &self,
        outcome: &PlanGeneratorOutcome,
        workdir: &Path,
    ) -> Result<PathBuf> {
        let plans_root = workspace_paths::plans_dir(workdir);
        let plan_dir = plans_root.join(&outcome.slug);
        std::fs::create_dir_all(&plan_dir)
            .with_context(|| format!("create plan dir {}", plan_dir.display()))?;

        let tasks_toml = outcome
            .tasks_toml
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("no tasks_toml in outcome"))?;
        atomic_write_str(&plan_dir.join("tasks.toml"), tasks_toml)
            .with_context(|| format!("write tasks.toml to {}", plan_dir.display()))?;

        if let Some(plan_md) = &outcome.plan_md {
            atomic_write_str(&plan_dir.join("plan.md"), plan_md)
                .with_context(|| format!("write plan.md to {}", plan_dir.display()))?;
        }

        Ok(plans_root)
    }

    fn report(&self, message: &str) {
        eprintln!("  [plan generate] {message}");
    }
}

/// Do-command adapter — handles `do_standard` and `do_complex` call sites.
///
/// Persists to the workspace plans directory. For `do_complex`, the host
/// manages the PRD-first flow and delegates only the plan generation step
/// here.
pub struct DoAdapter {
    adapter_key: String,
}

impl DoAdapter {
    /// Create an adapter for the standard `roko do` path.
    #[must_use]
    pub fn new_standard() -> Self {
        Self {
            adapter_key: adapter_keys::DO_STANDARD.to_string(),
        }
    }

    /// Create an adapter for the complex (PRD-first) `roko do` path.
    #[must_use]
    pub fn new_complex() -> Self {
        Self {
            adapter_key: adapter_keys::DO_COMPLEX.to_string(),
        }
    }
}

impl PlanGeneratorAdapter for DoAdapter {
    fn adapter_key(&self) -> &str {
        &self.adapter_key
    }

    fn persist(
        &self,
        outcome: &PlanGeneratorOutcome,
        workdir: &Path,
    ) -> Result<PathBuf> {
        let plans_root = workspace_paths::plans_dir(workdir);
        let plan_dir = plans_root.join(&outcome.slug);
        std::fs::create_dir_all(&plan_dir)
            .with_context(|| format!("create plan dir {}", plan_dir.display()))?;

        let tasks_toml = outcome
            .tasks_toml
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("no tasks_toml in outcome"))?;
        atomic_write_str(&plan_dir.join("tasks.toml"), tasks_toml)
            .with_context(|| format!("write tasks.toml to {}", plan_dir.display()))?;

        if let Some(plan_md) = &outcome.plan_md {
            atomic_write_str(&plan_dir.join("plan.md"), plan_md)
                .with_context(|| format!("write plan.md to {}", plan_dir.display()))?;
        }

        Ok(plans_root)
    }

    fn report(&self, message: &str) {
        eprintln!("  [do] {message}");
    }
}

// ---------------------------------------------------------------------------
// Lane B3: DirectLight service pattern
// ---------------------------------------------------------------------------

/// Minimal dispatch profile for direct CLI commands (`agent_exec`, `research`,
/// `serve_runtime`).
///
/// This is a thin configuration carrier that replaces per-call factory/model
/// resolution with one-time construction at the command entry point. Until
/// #243 provides `RuntimeServices`, the actual provider dispatch stays in the
/// existing call sites. This type captures the resolved model, role, and tool
/// policy so they are not re-derived per call.
#[derive(Debug, Clone)]
pub struct DirectLight {
    /// Resolved model key (from config + CLI overrides).
    pub model: Option<String>,
    /// Default agent role for this session.
    pub role: String,
    /// Allowed tools (comma-separated), if restricted.
    pub allowed_tools: Option<String>,
    /// Effort level string.
    pub effort: String,
}

impl DirectLight {
    /// Construct a `DirectLight` from resolved workspace config.
    #[must_use]
    pub fn from_config(config: &roko_core::config::schema::RokoConfig) -> Self {
        Self {
            model: config.agent.model.clone(),
            role: "implementer".to_string(),
            allowed_tools: None,
            effort: config.agent.effort.to_string(),
        }
    }

    /// Override the model for this session.
    #[must_use]
    pub fn with_model(mut self, model: Option<String>) -> Self {
        if model.is_some() {
            self.model = model;
        }
        self
    }

    /// Override the role for this session.
    #[must_use]
    pub fn with_role(mut self, role: impl Into<String>) -> Self {
        self.role = role.into();
        self
    }

    /// Override allowed tools for this session.
    #[must_use]
    pub fn with_allowed_tools(mut self, tools: Option<String>) -> Self {
        self.allowed_tools = tools;
        self
    }
}

// ---------------------------------------------------------------------------
// Lane C: Server adapters
// ---------------------------------------------------------------------------

/// CLI serve-runtime adapter — handles `cli_serve_runtime` call site.
///
/// Used by `serve_runtime.rs::RokoCliRuntime` to bridge serve-launched PRD
/// plan generation back into the CLI's plan generator service.
pub struct CliServeRuntimeAdapter;

impl PlanGeneratorAdapter for CliServeRuntimeAdapter {
    fn adapter_key(&self) -> &str {
        adapter_keys::CLI_SERVE_RUNTIME
    }

    fn persist(
        &self,
        outcome: &PlanGeneratorOutcome,
        workdir: &Path,
    ) -> Result<PathBuf> {
        // Same persistence as PrdDefaultAdapter — plans go to workspace plans dir.
        let plans_root = workspace_paths::plans_dir(workdir);
        let plan_dir = plans_root.join(&outcome.slug);
        std::fs::create_dir_all(&plan_dir)
            .with_context(|| format!("create plan dir {}", plan_dir.display()))?;

        let tasks_toml = outcome
            .tasks_toml
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("no tasks_toml in outcome"))?;
        atomic_write_str(&plan_dir.join("tasks.toml"), tasks_toml)
            .with_context(|| format!("write tasks.toml to {}", plan_dir.display()))?;

        if let Some(plan_md) = &outcome.plan_md {
            atomic_write_str(&plan_dir.join("plan.md"), plan_md)
                .with_context(|| format!("write plan.md to {}", plan_dir.display()))?;
        }

        Ok(plans_root)
    }

    fn report(&self, message: &str) {
        tracing::info!("[cli-serve-runtime] {message}");
    }
}

/// Serve runtime adapter — handles `serve_runtime` call site from
/// `roko-serve/src/runtime.rs` and `job_runner.rs`.
///
/// Used when the HTTP control plane generates plans from PRDs via the
/// `CliRuntime` trait. Persistence follows the same workspace layout as
/// direct CLI callers.
pub struct ServeRuntimeAdapter;

impl PlanGeneratorAdapter for ServeRuntimeAdapter {
    fn adapter_key(&self) -> &str {
        adapter_keys::SERVE_RUNTIME
    }

    fn persist(
        &self,
        outcome: &PlanGeneratorOutcome,
        workdir: &Path,
    ) -> Result<PathBuf> {
        let plans_root = workspace_paths::plans_dir(workdir);
        let plan_dir = plans_root.join(&outcome.slug);
        std::fs::create_dir_all(&plan_dir)
            .with_context(|| format!("create plan dir {}", plan_dir.display()))?;

        let tasks_toml = outcome
            .tasks_toml
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("no tasks_toml in outcome"))?;
        atomic_write_str(&plan_dir.join("tasks.toml"), tasks_toml)
            .with_context(|| format!("write tasks.toml to {}", plan_dir.display()))?;

        if let Some(plan_md) = &outcome.plan_md {
            atomic_write_str(&plan_dir.join("plan.md"), plan_md)
                .with_context(|| format!("write plan.md to {}", plan_dir.display()))?;
        }

        Ok(plans_root)
    }

    fn report(&self, message: &str) {
        tracing::info!("[serve-runtime] {message}");
    }
}

/// HTTP plan generation adapter — handles `serve_http` call site from
/// `roko-serve/src/routes/plans.rs`.
///
/// Background-spawned generation from `POST /api/plans/generate`. The route
/// handler retains authorization and event-bus publication; this adapter only
/// owns the filesystem persistence step.
pub struct ServeHttpAdapter;

impl PlanGeneratorAdapter for ServeHttpAdapter {
    fn adapter_key(&self) -> &str {
        adapter_keys::SERVE_HTTP
    }

    fn persist(
        &self,
        outcome: &PlanGeneratorOutcome,
        workdir: &Path,
    ) -> Result<PathBuf> {
        let plans_root = workspace_paths::plans_dir(workdir);
        let plan_dir = plans_root.join(&outcome.slug);
        std::fs::create_dir_all(&plan_dir)
            .with_context(|| format!("create plan dir {}", plan_dir.display()))?;

        let tasks_toml = outcome
            .tasks_toml
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("no tasks_toml in outcome"))?;
        atomic_write_str(&plan_dir.join("tasks.toml"), tasks_toml)
            .with_context(|| format!("write tasks.toml to {}", plan_dir.display()))?;

        if let Some(plan_md) = &outcome.plan_md {
            atomic_write_str(&plan_dir.join("plan.md"), plan_md)
                .with_context(|| format!("write plan.md to {}", plan_dir.display()))?;
        }

        Ok(plans_root)
    }

    fn report(&self, message: &str) {
        tracing::info!("[serve-http] {message}");
    }
}

// ---------------------------------------------------------------------------
// Lane C2: AgentServer dispatch profile
// ---------------------------------------------------------------------------

/// Dispatch profile for the per-agent sidecar server.
///
/// Injected into `AgentState` as a long-lived handle. Both HTTP messaging
/// and relay routes reuse this profile for model/role resolution. These paths
/// never generate plans.
///
/// Until #243 provides the full `RuntimeServices` bundle, this type captures
/// the resolved dispatch parameters that were previously built inline in
/// `AgentServerBuilder::build`.
#[derive(Debug, Clone)]
pub struct AgentServerDispatchProfile {
    /// Resolved model key for this agent's dispatch.
    pub model: Option<String>,
    /// Agent role.
    pub role: String,
    /// Agent ID for tracing.
    pub agent_id: String,
    /// Cost accounting label.
    pub cost_label: Option<String>,
}

impl AgentServerDispatchProfile {
    /// Create a new dispatch profile for an agent server.
    #[must_use]
    pub fn new(agent_id: impl Into<String>) -> Self {
        Self {
            model: None,
            role: "implementer".to_string(),
            agent_id: agent_id.into(),
            cost_label: None,
        }
    }

    /// Set the model for this dispatch profile.
    #[must_use]
    pub fn with_model(mut self, model: Option<String>) -> Self {
        self.model = model;
        self
    }

    /// Set the role for this dispatch profile.
    #[must_use]
    pub fn with_role(mut self, role: impl Into<String>) -> Self {
        self.role = role.into();
        self
    }

    /// Set the cost label for this dispatch profile.
    #[must_use]
    pub fn with_cost_label(mut self, label: Option<String>) -> Self {
        self.cost_label = label;
        self
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan_generator::{
        PlanGeneratorOutcome, ValidationEvidence, adapter_keys,
    };
    use roko_learn::runtime_feedback::GenerationOutcome;

    fn test_outcome(slug: &str, adapter_key: &str) -> PlanGeneratorOutcome {
        PlanGeneratorOutcome {
            tasks_toml: Some(format!(
                "[meta]\nplan = \"{slug}\"\ntotal = 1\nstatus = \"ready\"\n\n\
                 [[task]]\nid = \"T1\"\ntitle = \"Test\"\nstatus = \"ready\"\n\
                 tier = \"focused\"\nrole = \"implementer\"\n"
            )),
            plan_md: Some(format!("# {slug}\n\nTest plan.\n")),
            slug: slug.to_string(),
            outcome: GenerationOutcome {
                process_success: true,
                artifact_valid: true,
                validation_report: None,
            },
            evidence: ValidationEvidence {
                extraction_attempts: 1,
                model_escalated: false,
                final_model: None,
                repairs_applied: vec![],
                policy_violations: vec![],
            },
            task_count: 1,
            estimated_complexity: None,
            adapter_key: adapter_key.to_string(),
        }
    }

    #[test]
    fn plan_generator_adapters_prd_default_key() {
        let adapter = PrdDefaultAdapter::new_default();
        assert_eq!(adapter.adapter_key(), adapter_keys::PRD_DEFAULT);
    }

    #[test]
    fn plan_generator_adapters_prd_model_key() {
        let adapter = PrdDefaultAdapter::new_model();
        assert_eq!(adapter.adapter_key(), adapter_keys::PRD_MODEL);
    }

    #[test]
    fn plan_generator_adapters_prd_replan_key() {
        let adapter = PrdDefaultAdapter::new_replan();
        assert_eq!(adapter.adapter_key(), adapter_keys::PRD_REPLAN);
    }

    #[test]
    fn plan_generator_adapters_plan_generate_key() {
        let adapter = PlanGenerateAdapter;
        assert_eq!(adapter.adapter_key(), adapter_keys::PLAN_GENERATE);
    }

    #[test]
    fn plan_generator_adapters_do_standard_key() {
        let adapter = DoAdapter::new_standard();
        assert_eq!(adapter.adapter_key(), adapter_keys::DO_STANDARD);
    }

    #[test]
    fn plan_generator_adapters_do_complex_key() {
        let adapter = DoAdapter::new_complex();
        assert_eq!(adapter.adapter_key(), adapter_keys::DO_COMPLEX);
    }

    #[test]
    fn plan_generator_adapters_cli_serve_runtime_key() {
        let adapter = CliServeRuntimeAdapter;
        assert_eq!(adapter.adapter_key(), adapter_keys::CLI_SERVE_RUNTIME);
    }

    #[test]
    fn plan_generator_adapters_serve_runtime_key() {
        let adapter = ServeRuntimeAdapter;
        assert_eq!(adapter.adapter_key(), adapter_keys::SERVE_RUNTIME);
    }

    #[test]
    fn plan_generator_adapters_serve_http_key() {
        let adapter = ServeHttpAdapter;
        assert_eq!(adapter.adapter_key(), adapter_keys::SERVE_HTTP);
    }

    #[test]
    fn plan_generator_adapters_prd_persist_creates_source_prd() {
        let dir = tempfile::tempdir().unwrap();
        let workdir = dir.path();
        std::fs::create_dir_all(workdir.join(".roko/plans")).unwrap();
        let outcome = test_outcome("test-prd", adapter_keys::PRD_DEFAULT);
        let adapter = PrdDefaultAdapter::new_default();
        let result = adapter.persist(&outcome, workdir);
        assert!(result.is_ok());
        let plans_root = result.unwrap();
        let tasks = std::fs::read_to_string(plans_root.join("test-prd/tasks.toml")).unwrap();
        assert!(tasks.contains("source_prd"));
    }

    #[test]
    fn plan_generator_adapters_plan_generate_persist() {
        let dir = tempfile::tempdir().unwrap();
        let workdir = dir.path();
        std::fs::create_dir_all(workdir.join(".roko/plans")).unwrap();
        let outcome = test_outcome("gen-test", adapter_keys::PLAN_GENERATE);
        let adapter = PlanGenerateAdapter;
        let result = adapter.persist(&outcome, workdir);
        assert!(result.is_ok());
        let tasks_path = result.unwrap().join("gen-test/tasks.toml");
        assert!(tasks_path.exists());
    }

    #[test]
    fn plan_generator_adapters_do_persist() {
        let dir = tempfile::tempdir().unwrap();
        let workdir = dir.path();
        std::fs::create_dir_all(workdir.join(".roko/plans")).unwrap();
        let outcome = test_outcome("do-test", adapter_keys::DO_STANDARD);
        let adapter = DoAdapter::new_standard();
        let result = adapter.persist(&outcome, workdir);
        assert!(result.is_ok());
    }

    #[test]
    fn plan_generator_adapters_serve_persist() {
        let dir = tempfile::tempdir().unwrap();
        let workdir = dir.path();
        std::fs::create_dir_all(workdir.join(".roko/plans")).unwrap();
        let outcome = test_outcome("serve-test", adapter_keys::SERVE_RUNTIME);
        let adapter = ServeRuntimeAdapter;
        let result = adapter.persist(&outcome, workdir);
        assert!(result.is_ok());
    }

    #[test]
    fn plan_generator_adapters_all_non_replan_adapters_covered() {
        // Verify we have adapters for all non-gate_replan keys.
        let covered_keys: Vec<&str> = vec![
            PrdDefaultAdapter::new_default().adapter_key(),
            PrdDefaultAdapter::new_model().adapter_key(),
            PrdDefaultAdapter::new_replan().adapter_key(),
            PlanGenerateAdapter.adapter_key(),
            DoAdapter::new_standard().adapter_key(),
            DoAdapter::new_complex().adapter_key(),
            CliServeRuntimeAdapter.adapter_key(),
            ServeRuntimeAdapter.adapter_key(),
            ServeHttpAdapter.adapter_key(),
        ];
        // All 9 non-gate_replan keys from the manifest.
        assert_eq!(covered_keys.len(), 9);
        // No duplicates.
        let unique: std::collections::HashSet<&str> = covered_keys.iter().copied().collect();
        assert_eq!(unique.len(), 9);
    }

    // ---- DirectLight tests ----

    #[test]
    fn direct_light_from_config() {
        let config = roko_core::config::schema::RokoConfig::default();
        let dl = DirectLight::from_config(&config);
        assert_eq!(dl.role, "implementer");
        assert_eq!(dl.effort, config.agent.effort.to_string());
    }

    #[test]
    fn direct_light_with_overrides() {
        let config = roko_core::config::schema::RokoConfig::default();
        let dl = DirectLight::from_config(&config)
            .with_model(Some("claude-opus-4-6".to_string()))
            .with_role("strategist")
            .with_allowed_tools(Some("Read,Grep".to_string()));
        assert_eq!(dl.model.as_deref(), Some("claude-opus-4-6"));
        assert_eq!(dl.role, "strategist");
        assert_eq!(dl.allowed_tools.as_deref(), Some("Read,Grep"));
    }

    // ---- AgentServerDispatchProfile tests ----

    #[test]
    fn dispatch_profile_builder() {
        let profile = AgentServerDispatchProfile::new("agent-001")
            .with_model(Some("claude-sonnet-4-6".to_string()))
            .with_role("researcher")
            .with_cost_label(Some("research-pool".to_string()));
        assert_eq!(profile.agent_id, "agent-001");
        assert_eq!(profile.model.as_deref(), Some("claude-sonnet-4-6"));
        assert_eq!(profile.role, "researcher");
        assert_eq!(profile.cost_label.as_deref(), Some("research-pool"));
    }

    #[test]
    fn dispatch_profile_defaults() {
        let profile = AgentServerDispatchProfile::new("test");
        assert!(profile.model.is_none());
        assert_eq!(profile.role, "implementer");
        assert!(profile.cost_label.is_none());
    }
}
