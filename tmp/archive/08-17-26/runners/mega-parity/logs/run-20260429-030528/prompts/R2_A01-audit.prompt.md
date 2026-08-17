# AUDIT: Batch R2_A01 — Document current init template and config data flow

Run: run-20260429-030528 | Phase: AUDIT | Model: codex-5.5

## Your Role

You are an **auditor**. A fast implementation agent just completed batch `R2_A01`.
Your job is to verify correctness and fix any issues — do NOT rewrite from scratch.

## Audit Checklist

1. **Compiles:** `cargo check -p <crate>` for each crate touched by this batch
2. **Clippy clean:** `cargo clippy -p <crate> --no-deps -- -D warnings`
3. **Prompt compliance:** Compare the implementation against the original prompt below
4. **No regressions:** Changed files don't break existing functionality
5. **Anti-patterns:** No stubs that silently pass, no inline prompts, no raw CLI spawns
6. **Correct types:** Field names, method signatures, and imports match the actual codebase
7. **Tests pass:** If the prompt required tests, verify they pass

## If You Find Issues

Fix them directly in the files. Then run the verification commands from the prompt.
If you cannot fix an issue, leave a comment in the file explaining why.

## Scope

Only touch files in the batch's write scope. Do NOT refactor unrelated code.

---

## Original Implementation Prompt

## Task

Document current init template and config data flow

## Runner Context

You are working in runner `mega-parity`, batch R2_A01.
This batch is part of Runner 2: execution-contract — Make CLI execution contracts truthful enough that demo scenarios and agent sessions can rely on them.

## Problem

The init template, config schema versions (v1 vs v2), migration behavior, and downstream config consumers (WorkflowEngine, one-shot path) are undocumented. Without a clear map of the data flow, subsequent batches risk breaking existing paths or duplicating logic.

## Architecture Contract

This is a context-only batch. No code changes. Produce a reference document that subsequent R2_A* batches depend on.

## Changes Required

Read the following four files in their entirety, then produce a context document at:

```
tmp/runners/mega-parity/context/R2_A01_init_config_dataflow.md
```

Document the following facts from the actual code:

### 1. Init template entry point

File: `crates/roko-cli/src/commands/util.rs`
Function: `pub(crate) async fn cmd_init` (line 98)

The function:
1. Creates the target directory and `.roko/` subdirectory
2. Calls `RokoLayout::for_project(&target).ensure_dirs()` to create layout dirs
3. Creates extra dirs: `jobs/`, `prd/`, `prd/published/`, `prd/drafts/`, `task-outputs/`, `research/`, `subscriptions/`, `templates/`
4. Creates `engrams.jsonl` if missing (migrates from `signals.jsonl` if found)
5. Detects domain via `detect_project_domain()` (line 155-159)
6. Calls `Config::default_toml_template(cloud)?` (line 168) to get the TOML string
7. Writes it to `target/roko.toml`

The `--profile` arg is accepted (line 313-315 in main.rs `Command::Init`) and passed to `cmd_init` but is **not yet passed into** `Config::default_toml_template`. Profile is only used for `detect_project_domain` display output.

### 2. The template string

File: `crates/roko-cli/src/config.rs`
Function: `pub fn default_toml_template(cloud: bool) -> Result<String>` (line 119)

Steps:
1. Creates `Config::default()` which sets `gates: vec![GateConfig::default_shell_true()]`
2. Sets `config.agent.command = "claude"`
3. If `cloud`: sets `log_format`, `bind`, `data_dir`
4. Calls `config.to_toml()` (serializes to TOML via `toml::to_string_pretty`)
5. Appends cloud webhook stanza if `cloud`
6. Wraps in a format string prepending `# REQUIRED_ENV` comment block and appending `[prd]` section

`GateConfig::default_shell_true()` (line 903) produces:
```rust
GateConfig::Shell {
    program: "true".into(),
    args: Vec::new(),
    timeout_ms: 60_000,
}
```
Which serializes to TOML as:
```toml
[[gate]]
kind = "shell"
program = "true"
```

The `Config::default()` does NOT emit `schema_version`, `[providers.*]`, or `[models.*]` — those are `RokoConfig` fields, not `Config` fields. `Config` is the layered-config struct (lives in `config.rs`); `RokoConfig` is the core schema struct (lives in `roko-core/src/config/schema.rs`).

### 3. v1 vs v2 schema

File: `crates/roko-core/src/config/schema.rs`

Key constants:
```rust
pub const CURRENT_SCHEMA_VERSION: u32 = 2;
pub const CURRENT_CONFIG_VERSION: u32 = 2;
```

`RokoConfig` struct fields relevant to v1/v2 distinction:
```rust
pub config_version: u32,    // default fn returns 1 (!)
pub schema_version: u32,    // default fn returns CURRENT_SCHEMA_VERSION (2)
pub providers: HashMap<String, ProviderConfig>,  // empty in v1
pub models: HashMap<String, ModelProfile>,       // empty in v1
```

`RokoConfig::is_stale()` (line 196): returns `self.schema_version < CURRENT_SCHEMA_VERSION`

v1 config: no `[providers.*]` table, no `[models.*]` table; `config_version = 1` (or missing)
v2 config: has `[providers.<name>]` and `[models.<name>]` tables; `schema_version = 2`

`RokoConfig::from_toml()` (line 171): emits a one-time `tracing::warn!` when `config_version == 1`.

### 4. Config migrate behavior

File: `crates/roko-cli/src/config_cmd.rs`
Function: `pub fn cmd_migrate(workdir: &Path, dry_run: bool)` (line 371)

Flow:
1. Resolves config path via `validate_config_path` (project config or `workdir/roko.toml`)
2. Calls `build_config_migration_plan(&text)` (line 376)
3. If `ConfigMigrationPlan::AlreadyCurrent`: prints "nothing to migrate", returns Ok
4. If `ConfigMigrationPlan::Legacy(plan)`: prints proposed changes via `render_migration_preview`
5. If `dry_run`: prints "[dry-run] no changes written", returns
6. If NOT `dry_run`: calls `prompt_bool("Apply changes?", false)` — **interactive prompt**
7. If confirmed: writes `plan.rendered` to the config path

`ConfigCmd::Migrate` variant in `main.rs` (line 1505-1513):
```rust
Migrate {
    workdir: Option<PathBuf>,
    dry_run: bool,
}
```

There is NO `--yes`/`-y` flag today. The `prompt_bool` call at line 398 is the **only** confirmation gate.

### 5. WorkflowEngine provider/model reading path

File: `crates/roko-cli/src/run.rs`
Function: `pub async fn run_with_workflow_engine` (line 572)

Takes a `cli_config: Option<&crate::config::Config>` parameter (the **layered** `Config` struct, not `RokoConfig`). This `Config` struct has `providers: HashMap<String, ModelProfile>` and `models: HashMap<String, ModelProfile>` fields but they are populated from the layered config resolution path, not directly from `RokoConfig`.

### 6. One-shot path

File: `crates/roko-cli/src/run.rs`
Function: `pub async fn run_once` (line 902)

Takes `config: &Config` (the layered CLI config struct). It reads:
- `config.agent.command` — the agent program
- `config.prompt.role` — system prompt role
- `config.gates` — the list of `GateConfig` entries

It does NOT read `RokoConfig` directly. Provider routing in `run_once` goes through the agent config.

## Write Scope (files you may modify)

- `tmp/runners/mega-parity/context/R2_A01_init_config_dataflow.md` (context doc only)

## Read-Only Context (do not modify these)

- `crates/roko-cli/src/commands/util.rs` (lines 98-198: `cmd_init`)
- `crates/roko-cli/src/config.rs` (lines 119-159: `default_toml_template`; lines 858-910: `GateConfig`)
- `crates/roko-core/src/config/schema.rs` (lines 39-124: `RokoConfig`, `CURRENT_SCHEMA_VERSION`)
- `crates/roko-cli/src/config_cmd.rs` (lines 371-408: `cmd_migrate`; lines 718-774: `ConfigMigrationPlan`)
- `crates/roko-cli/src/run.rs` (lines 572-588: `run_with_workflow_engine`; lines 902-912: `run_once`)
- `crates/roko-cli/src/main.rs` (lines 307-316: `Command::Init`; lines 1505-1513: `ConfigCmd::Migrate`)

## Acceptance Criteria

- [ ] Document identifies that the init template uses `Config::default_toml_template` in `config.rs` (not a string constant)
- [ ] Document maps v1 vs v2 schema differences: `config_version`, `schema_version`, absence/presence of `[providers.*]` and `[models.*]`
- [ ] Document describes config migrate behavior: interactive `prompt_bool` at line 398, no `--yes` flag today
- [ ] Document traces WorkflowEngine provider/model reading: takes `Option<&Config>` (layered), not `RokoConfig`
- [ ] Document traces one-shot path reading: `config.agent.command`, `config.prompt.role`, `config.gates`
- [ ] Document notes that `--profile` arg to `cmd_init` is NOT wired into `default_toml_template`
- [ ] Document notes that `GateConfig::default_shell_true()` produces `program = "true"`
- [ ] No source code was modified

## Verification

N/A (context-only batch)

## Do NOT

- Change any source code
- Guess at implementation details — read the actual code
- Document aspirational behavior — only document what the code actually does
- Create the context directory if it already exists

## Evidence

E2E-DOGFOOD-AUDIT Path 1, E2E-TEST-RESULTS S1

---

## Read-Only Context (do not modify)

### `crates/roko-cli/src/commands/init.rs`

```rust
//! `roko init` template rendering.

use anyhow::{Context, Result};
use std::ffi::OsStr;

use roko_cli::config::command_on_path;
use roko_core::config::schema::RokoConfig;

/// Render the default `roko.toml` template used by `roko init`.
///
/// The base document comes from the v2 schema serializer so the generated
/// workspace starts in the provider/model world rather than the legacy
/// v1 `[agent]` command world.
pub(crate) fn render_init_template(cloud: bool) -> Result<String> {
    let profile = detect_init_profile().map(|profile| profile.trim().to_ascii_lowercase());

    let mut config = RokoConfig::default();
    config.agent.default_backend = "claude".to_string();
    config.agent.default_model = "claude-sonnet-4-6".to_string();
    if cloud {
        config.server.bind = "0.0.0.0".to_string();
    }

    let mut rendered = config
        .to_toml_pretty()
        .context("serialize default v2 roko.toml")?;
    if !rendered.ends_with('\n') {
        rendered.push('\n');
    }

    let mut out = String::with_capacity(rendered.len() + 512);
    out.push_str("# REQUIRED_ENV\n");
    out.push_str("# Required environment variables (set in .env or shell):\n");
    out.push_str("# GITHUB_TOKEN       - GitHub personal access token (for MCP GitHub server)\n");
    out.push_str("# GITHUB_WEBHOOK_SECRET - GitHub webhook secret for deploy registration\n");
    out.push_str("# SLACK_BOT_TOKEN    - Slack bot token (for MCP Slack server)\n");
    out.push_str("# SLACK_SIGNING_SECRET - Slack webhook signing secret\n");
    out.push_str("# ANTHROPIC_API_KEY  - Claude API key (for direct API agents, not needed for CLI agents)\n\n");
    out.push_str(&rendered);

    if command_on_path("claude") {
        out.push_str("\n[providers.claude_cli]\n");
        out.push_str("kind = \"claude_cli\"\n");
        out.push_str("command = \"claude\"\n");
    } else {
        out.push_str("\n# Claude CLI was not found on PATH when this workspace was initialized.\n");
        out.push_str("# Install Claude CLI and uncomment the provider block below to use the default setup.\n");
        out.push_str("# [providers.claude_cli]\n");
        out.push_str("# kind = \"claude_cli\"\n");
        out.push_str("# command = \"claude\"\n");
    }

    out.push_str("\n[models.claude-sonnet-4-6]\n");
    out.push_str("provider = \"claude_cli\"\n");
    out.push_str("slug = \"claude-sonnet-4-6\"\n");
    out.push_str("context_window = 200000\n");
    out.push_str("tool_format = \"anthropic_blocks\"\n");
    out.push_str("max_tools = 32\n");

    append_verification_gates(&mut out, profile.as_deref());

    if cloud {
        out.push_str("\n# Auto-register webhooks after deploy\n");
        out.push_str("[[serve.deploy.webhooks]]\n");
        out.push_str("provider = \"github\"\n");
        out.push_str("owner = \"nunchi\"\n");
        out.push_str("repo = \"roko\"\n\n");
        out.push_str("[[serve.deploy.webhooks]]\n");
        out.push_str("provider = \"github\"\n");
        out.push_str("owner = \"nunchi\"\n");
        out.push_str("repo = \"collaboration\"\n");
    }

    Ok(out)
}

fn detect_init_profile() -> Option<String> {
    // `cmd_init` does not currently thread the parsed profile through this helper.
    let mut args = std::env::args_os();
    let _ = args.next();

    while let Some(arg) = args.next() {
        if arg.as_os_str() == OsStr::new("--profile") {
            return args.next().map(|value| value.to_string_lossy().into_owned());
        }

        let arg = arg.to_string_lossy();
        if let Some(profile) = arg.strip_prefix("--profile=") {
            if profile.is_empty() {
                return None;
            }
            return Some(profile.to_owned());
        }
    }

    None
}

fn append_verification_gates(out: &mut String, profile: Option<&str>) {
    out.push_str("\n# -- Verification gates --\n");
    match profile {
        Some("rust") => {
            out.push_str("# Rust projects use cargo for compile, test, and lint checks.\n");
            append_shell_gate(out, "cargo", &["check"], 600_000);
            append_shell_gate(out, "cargo", &["test"], 600_000);
            append_shell_gate(out, "cargo", &["clippy"], 600_000);
        }
        Some("typescript") => {
            out.push_str("# TypeScript projects use npx tsc and npm test.\n");
            append_shell_gate(out, "npx", &["tsc", "--noEmit"], 600_000);
            append_shell_gate(out, "npm", &["test"], 600_000);
        }
        _ => {
            out.push_str(
                "# No default gates were written because no supported project profile was supplied.\n",
            );
            out.push_str("# Supported profiles: rust, typescript.\n");
            out.push_str("# Add [[gate]] entries manually to run your own validation commands.\n");
            out.push_str("# Or rerun `roko init --profile rust` / `roko init --profile typescript`.\n");
        }
    }
}

fn append_shell_gate(out: &mut String, program: &str, args: &[&str], timeout_ms: u64) {
    out.push_str("\n[[gate]]\n");
    out.push_str("kind = \"shell\"\n");
    out.push_str("program = \"");
    out.push_str(program);
    out.push_str("\"\n");
    out.push_str("args = [");
    for (index, arg) in args.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        out.push('"');
        out.push_str(arg);
        out.push('"');
    }
    out.push_str("]\n");
    out.push_str("timeout_ms = ");
    out.push_str(&timeout_ms.to_string());
    out.push('\n');
}
```

### `crates/roko-core/src/config/mod.rs`

```rust
//! Roko runtime configuration.
//!
//! # Modules
//!
//! - [`schema`] -- The unified `RokoConfig` type with hierarchical sections.
//! - [`compat`] -- Reader for legacy Mori `config.toml` format.
//! - [`presets`] -- Named presets (minimal / balanced / thorough).

use std::path::Path;

use thiserror::Error;

pub mod agent;
pub mod budget;
pub mod chain;
pub mod compat;
pub mod gates;
pub mod hot_reload;
pub mod learning;
pub mod presets;
pub mod project;
pub mod provider;
pub mod routing;
pub mod schema;
pub mod serve;
pub mod subscriptions;
pub mod tools;
pub mod tui_cfg;

// Re-exports for ergonomic use.
pub use crate::temperament::Temperament;
pub use compat::from_mori_toml;
pub use presets::Preset;
// All section structs are re-exported from schema (which re-exports from submodules).
pub use schema::{
    AgentBudget, AgentConfig, AgentDefinition, AgentMode, AgentRoleToggles, AgentThresholds,
    ApiKeyEntry, AttentionConfig, BudgetConfig, CURRENT_SCHEMA_VERSION, ChainConfig,
    CompileFailRepeatConfig, ConductorConfig, ContextWindowPressureConfig, CostOverrunConfig,
    DataLlmConfig, DemurrageConfig, DeployConfig, EnergyConfig, GatesConfig, GeminiConfig,
    GhostTurnConfig, GithubWebhookConfig, GoalsConfig, ImmuneConfig, IterationLoopConfig,
    LearningConfig, ModelProfile, OneirographyConfig, PerplexityConfig, PipelineBandConfig,
    PipelineConfig, PipelineReviewerMode, PrdConfig, ProjectConfig, ProviderConfig,
    ProviderRouting, RelayConfig, ReviewLoopConfig, RewardWeights, RokoConfig, RoleOverride,
    RoutingAlgorithm, RoutingConfig, RoutingOverrides, RoutingRewardWeightsConfig, SafetySetting,
    SchedulerConfig, SchedulerCronConfig, ServeAuthConfig, ServeConfig, ServeDeployConfig,
    ServeDeployWebhookConfig, ServerConfig, SpecDriftConfig, StuckPatternConfig,
    SubscriptionConfig, SubscriptionFilterConfig, SubscriptionTrigger, TemporalConfig,
    TestFailureBudgetConfig, TimeOverrunConfig, ToolProfileConfig, ToolsConfig, TuiConfig,
    WatcherConfig, WatcherPathConfig, WatcherThresholds, WebhooksConfig,
};

/// Error returned when loading a `roko.toml` file from disk.
#[derive(Debug, Error)]
pub enum LoadConfigError {
    /// Reading the config file failed.
    #[error("read {path}: {source}")]
    Read {
        /// Config file path.
        path: std::path::PathBuf,
        /// Underlying I/O error.
        source: std::io::Error,
    },
    /// Parsing the config file failed.
    #[error("parse {path}: {source}")]
    Parse {
        /// Config file path.
        path: std::path::PathBuf,
        /// Underlying parse error.
        source: toml::de::Error,
    },
}

/// Load the workspace configuration from `workdir/roko.toml`.
///
/// Missing files fall back to `RokoConfig::default()` so callers can start a
/// daemon in an uninitialized workspace.
///
/// After parsing, two secret-resolution passes run automatically:
///   1. `${VAR}` interpolation — expands environment variable references in
///      provider config strings.
///   2. `*_file` resolution — reads secrets from file paths in `extra_headers`
///      whose keys end with `_file`.
pub fn load_config(workdir: &Path) -> Result<RokoConfig, LoadConfigError> {
    let path = workdir.join("roko.toml");
    if !path.exists() {
        return Ok(RokoConfig::default());
    }

    let text = std::fs::read_to_string(&path).map_err(|source| LoadConfigError::Read {
        path: path.clone(),
        source,
    })?;
    let mut config: RokoConfig =
        toml::from_str(&text).map_err(|source| LoadConfigError::Parse {
            path: path.clone(),
            source,
        })?;

    // Secret resolution passes.
    config.interpolate_env_vars();
    config.resolve_file_secrets();

    Ok(config)
}
```

### `crates/roko-cli/src/commands/config_cmd.rs` (2590 lines — signatures only)

```rust
235:impl ProviderLatencySummary {
590:fn format_effective_model_selection_summary(
618:fn model_selection_recommendation(error: &crate::model_selection::Error) -> String {
1083:fn model_profile_for_effective_selection(
1096:fn build_provider_test_report(
```

### `crates/roko-cli/src/run.rs` (3413 lines — signatures only)

```rust
65:pub struct RunReport {
67:    pub episode_id: String,
69:    pub prompt_id: String,
71:    pub agent_output_id: String,
73:    pub agent_success: bool,
75:    pub gate_verdicts: Vec<(String, bool)>,
77:    pub total_signals: usize,
79:    pub output_text: Option<String>,
82:    pub usage: Option<RunUsage>,
87:pub struct RunUsage {
89:    pub input_tokens: u64,
91:    pub output_tokens: u64,
94:impl RunReport {
97:    pub fn overall_success(&self) -> bool {
110:struct StrategyPromptAugmentation {
115:struct ContextEnrichmentOverlay {
120:struct PlaybookSection {
125:struct DispatchOutcome {
133:pub fn write_shared_run(workdir: &std::path::Path, report: &RunReport) -> anyhow::Result<String> {
155:pub fn write_shared_workflow_run(
202:fn write_shared_transcript(
226:pub struct PlanWorkflowReport {
228:    pub total: usize,
230:    pub passed: usize,
232:    pub failed: usize,
234:    pub outcomes: Vec<(String, bool, String)>,
235:    pub task_reports: Vec<PlanTaskWorkflowReport>,
236:    pub task_errors: Vec<PlanTaskWorkflowError>,
240:pub struct PlanWorkflowTask {
241:    pub plan_id: String,
242:    pub task: crate::task_parser::TaskDef,
246:pub struct PlanTaskWorkflowReport {
247:    pub plan_id: String,
248:    pub task_id: String,
249:    pub report: WorkflowRunReport,
253:pub struct PlanTaskWorkflowError {
254:    pub plan_id: String,
255:    pub task_id: String,
256:    pub error: String,
261:struct StateHubBridge {
```

---

## Verification Commands

Run these and fix any failures:
```bash
cargo check --workspace
cargo clippy --workspace --no-deps -- -D warnings
```

## Do NOT

- Rewrite the entire implementation from scratch
- Add features not in the original prompt
- Modify files outside the write scope
- Skip running verification commands
