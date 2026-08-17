# AUDIT: Batch R3_A03 — Resolve tool policy from existing safety/tool contracts

Run: run-20260429-030528 | Phase: AUDIT | Model: codex-5.5

## Your Role

You are an **auditor**. A fast implementation agent just completed batch `R3_A03`.
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

Resolve tool policy from existing safety/tool contracts

## Runner Context

You are working in runner `mega-parity`, batch R3_A03.
This batch is part of Runner 3: agent-session-parity — Make interactive roko and one-shot use a real agent session via existing adapters.

## Problem

The `ChatAgentSession` constructor (R3_A01) has a placeholder empty string for `allowed_tools_csv`. Interactive chat sends no `--tools` flag at all, meaning the agent gets unrestricted tool access. Meanwhile, `orchestrate.rs` resolves tool policies from `AgentContract` safety contracts. The chat session should consult the same contracts and default to a read-oriented tool set when no contract exists.

## Architecture Contract

- Tool policy resolved from `AgentContract` in `roko-agent/src/safety/contract.rs`
- Default when no contract: `"Read,Glob,Grep,Bash,Edit,Write,NotebookEdit"`
- Uses the `allowed_tools` field of `AgentContract` (NOT `tool_allowlist` — that field does not exist)
- Formatted as comma-separated string for Claude CLI `--tools` flag
- Log a debug message if no contract is found (not an error)
- Same tool set for one-shot and interactive (no branching)

## Changes Required

### Step 1: Add import to `crates/roko-cli/src/chat_session.rs`

Add to the imports section:
```rust
use roko_agent::safety::contract::AgentContract;
```

### Step 2: Replace the allowed_tools_csv placeholder in the constructor

In `crates/roko-cli/src/chat_session.rs`, find this line inside `ChatAgentSession::new`:
```rust
        // 2. Resolve tool policy (placeholder — R3_A03 fills this in)
        let allowed_tools_csv = String::new();
```

Replace it with:
```rust
        // 2. Resolve tool policy from safety contracts
        let allowed_tools_csv = resolve_tool_policy(&workdir);
```

### Step 3: Add the helper functions

After the existing helper functions (after `gather_workspace_context`), add:

```rust
/// Default tools for interactive chat when no safety contract is found.
const DEFAULT_CHAT_TOOLS: &str = "Read,Glob,Grep,Bash,Edit,Write,NotebookEdit";

/// Resolve tool allowlist from safety contracts.
///
/// Looks for an `AgentContract` for the "chat" role at `.roko/safety/chat.yaml`.
/// If found, uses its `allowed_tools` field. If not found, falls back to a
/// read-oriented default set and logs a debug message.
fn resolve_tool_policy(workdir: &Path) -> String {
    // Try to load contract for "chat" role
    let contract_path = workdir.join(".roko/safety/chat.yaml");
    match std::fs::read_to_string(&contract_path) {
        Ok(content) => {
            match serde_yaml::from_str::<AgentContract>(&content) {
                Ok(contract) => {
                    if let Some(ref allowlist) = contract.allowed_tools {
                        if !allowlist.is_empty() {
                            tracing::debug!(
                                "chat tool policy from contract: {}",
                                allowlist.join(",")
                            );
                            return allowlist.join(",");
                        }
                    }
                    tracing::debug!(
                        "chat contract has no allowed_tools, using defaults"
                    );
                    DEFAULT_CHAT_TOOLS.to_string()
                }
                Err(e) => {
                    tracing::warn!("failed to parse chat contract at {}: {e}", contract_path.display());
                    DEFAULT_CHAT_TOOLS.to_string()
                }
            }
        }
        Err(_) => {
            tracing::debug!(
                "no chat safety contract at {}, using default tools",
                contract_path.display()
            );
            DEFAULT_CHAT_TOOLS.to_string()
        }
    }
}
```

## Write Scope (files you may modify)

- `crates/roko-cli/src/chat_session.rs`

## Read-Only Context (do not modify these)

- `crates/roko-agent/src/safety/contract.rs` — AgentContract struct and field names
- `crates/roko-cli/src/config.rs`

## Actual Code — Key Facts

### AgentContract struct (safety/contract.rs lines 47–69)

```rust
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AgentContract {
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub invariants: Vec<Invariant>,
    #[serde(default)]
    pub governance: Vec<GovernanceRule>,
    #[serde(default)]
    pub recovery: Vec<RecoveryAction>,
    /// Optional explicit allowlist of tool names this role may invoke.
    ///
    /// When `Some(_)`, the dispatcher enforces capability intersection...
    #[serde(default)]
    pub allowed_tools: Option<Vec<String>>,   // ← CORRECT field name
}
```

**Critical**: The field is `allowed_tools: Option<Vec<String>>`, NOT `tool_allowlist`. Any code using `contract.tool_allowlist` will fail to compile.

### serde_yaml dependency

`serde_yaml` is already a dependency of `roko-cli`. The import path is:
```rust
// No need to add to imports — serde_yaml is used elsewhere in the crate
// Just use it directly: serde_yaml::from_str::<AgentContract>(&content)
```

If you get a "use of undeclared crate" error, add to the top of `chat_session.rs`:
```rust
// serde_yaml is a dep of roko-cli; no import needed at file level
```
It is accessed via the crate path `serde_yaml::from_str`.

### roko-agent safety module import

The `AgentContract` type is in `roko_agent::safety::contract`. The import:
```rust
use roko_agent::safety::contract::AgentContract;
```

`roko-agent` is already a dependency of `roko-cli`.

### Path to contract file

The chat contract is expected at `<workdir>/.roko/safety/chat.yaml`. If the file does not exist, `std::fs::read_to_string` returns `Err(_)` and we fall back to `DEFAULT_CHAT_TOOLS`. This is intentional — it is not an error.

### AgentContract YAML format (for reference only)

A valid `.roko/safety/chat.yaml` looks like:
```yaml
role: chat
allowed_tools:
  - Read
  - Glob
  - Grep
  - Bash
  - Edit
  - Write
```

The `AgentContract` deserialization uses serde with `#[serde(default)]` on all fields, so partial YAML is valid.

### Why not use AgentContract::load_for_role?

`AgentContract::load_for_role` (contract.rs line 107) reads from bundled assets at `src/safety/contracts/<role>.yaml` inside the `roko-agent` crate — it uses `env!("CARGO_MANIFEST_DIR")` (line 501). There is no bundled "chat" contract. We use workspace-local YAML instead.

## Verification

```bash
cd /Users/will/dev/nunchi/roko/roko
cargo check -p roko-cli 2>&1 | head -30
```

Expected: no errors. `AgentContract` resolves from `roko_agent::safety::contract`. The constructor calls `resolve_tool_policy`.

## Do NOT

- Hardcode a tool list without checking contracts first
- Create a new safety/contract system
- Use different tool sets for one-shot vs interactive
- Error on missing contract (it is a debug log, not an error)
- Modify `AgentContract` struct or safety module
- Use `contract.tool_allowlist` — the correct field is `contract.allowed_tools`
- Call `AgentContract::load_for_role` — it reads bundled assets, not workspace contracts

---

## Current Implementation (as written by implementation agent)

### `crates/roko-cli/src/chat_session.rs`

```rust
//! Unified agent session for interactive and one-shot CLI modes.
//!
//! This module owns the session state that will later be passed to the Claude
//! CLI adapter or to API-backed provider adapters.

use std::fs;
use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;
use std::time::Duration;

use anyhow::Result;
use roko_agent::safety::contract::AgentContract;
use roko_compose::system_prompt_builder::SystemPromptBuilder;
use roko_compose::{ProjectConventions, TokenCounter, detect_conventions};
use roko_core::foundation::ChatMessage;

use crate::config::Config;
use crate::model_selection::EffectiveModelSelection;

const CHAT_SYSTEM_PROMPT_TOKEN_BUDGET: usize = 4_000;
const MAX_WORKSPACE_SAMPLE_BYTES: usize = 16_384;
const MAX_WORKSPACE_SAMPLE_FILES: usize = 8;
const MAX_WORKSPACE_SCAN_DEPTH: usize = 5;
const SKIP_DIR_NAMES: [&str; 12] = [
    ".git",
    ".next",
    ".roko",
    ".turbo",
    ".venv",
    "__pycache__",
    "build",
    "coverage",
    "dist",
    "node_modules",
    "target",
    "venv",
];

/// Unified agent session for interactive and one-shot CLI modes.
///
/// Delegates to `ClaudeCliAgent` for Claude CLI turns and to provider
/// adapters for API turns, instead of duplicating command construction.
pub struct ChatAgentSession {
    /// Working directory for the agent.
    pub workdir: PathBuf,
    /// Resolved model identity (provider + slug + source).
    pub model_selection: EffectiveModelSelection,
    /// Reasoning effort level: `"low"`, `"medium"`, `"high"`, `"max"`.
    pub effort: String,
    /// System prompt built by `SystemPromptBuilder`.
    pub system_prompt: String,
    /// Tool allowlist as comma-separated names for `--tools`.
    pub allowed_tools_csv: String,
    /// Path to MCP config file, if discovered.
    pub mcp_config: Option<PathBuf>,
    /// Session ID from previous turn, reused via `--resume`.
    pub session_id: Option<String>,
    /// API message history for non-CLI providers.
    pub api_history: Vec<ChatMessage>,
    /// Shared HTTP client for API providers.
    pub http_client: reqwest::Client,
    /// Path to Claude CLI settings JSON file.
    pub settings_json: Option<PathBuf>,
    /// Per-turn timeout.
    pub timeout: Option<Duration>,
}

impl ChatAgentSession {
    /// Create a new session from CLI config and working directory.
    ///
    /// Resolves system prompt via `SystemPromptBuilder`, tool policy from
    /// safety contracts, and MCP config from discovery paths. Creates one
    /// shared `reqwest::Client`.
    #[must_use]
    pub fn new(
        config: &Config,
        workdir: PathBuf,
        model_selection: EffectiveModelSelection,
    ) -> Result<Self> {
        let system_prompt = build_chat_system_prompt(&workdir, config);
        let allowed_tools_csv = resolve_tool_policy(&workdir);
        let mcp_config = discover_mcp_config_stub(config, &workdir);
        let effort = config.agent.effort.clone();
        let timeout = (config.agent.timeout_ms > 0)
            .then(|| Duration::from_millis(config.agent.timeout_ms));

        Ok(Self {
            workdir,
            model_selection,
            effort,
            system_prompt,
            allowed_tools_csv,
            mcp_config,
            session_id: None,
            api_history: Vec::new(),
            http_client: shared_http_client(),
            settings_json: None,
            timeout,
        })
    }
}

/// Build a system prompt for interactive and one-shot chat using the shared
/// `SystemPromptBuilder`.
///
/// Workspace context is inferred from the working directory. If the composed
/// prompt ends up empty for any reason, fall back to a minimal role identity.
fn build_chat_system_prompt(workdir: &Path, config: &Config) -> String {
    let role_identity = "You are an expert software engineer working in an interactive chat session. You help inspect, understand, and edit the current repository. Stay concise, grounded in the workspace, and prefer existing code over inventing new abstractions.";

    let mut builder = SystemPromptBuilder::new(role_identity);

    if let Some(conventions) = gather_workspace_conventions(workdir) {
        builder = builder.with_conventions(conventions);
    }

    let project_name = project_name_for(workdir);
    builder = builder.with_domain(format!(
        "Working directory: {}\nProject: {}",
        workdir.display(),
        project_name
    ));

    if let Ok(context) = gather_workspace_context(workdir) {
        if !context.trim().is_empty() {
            builder = builder.with_context(context);
        }
    }

    let token_budget = config.prompt.token_budget.clamp(1, CHAT_SYSTEM_PROMPT_TOKEN_BUDGET);
    let prompt = builder
        .with_token_budget(token_budget)
        .build_with_counter(&TokenCounter::Heuristic {
            chars_per_token: 4.0,
        });

    if prompt.trim().is_empty() {
        role_identity.to_string()
    } else {
        prompt
    }
}

/// Gather lightweight workspace context: git branch and language hints.
///
/// The result is best-effort. Missing git metadata or workspace markers are
/// treated as empty context instead of a hard error.
fn gather_workspace_context(workdir: &Path) -> Result<String> {
    let mut parts = Vec::new();

    if let Some(branch) = capture_git_branch(workdir) {
        if !branch.is_empty() {
            parts.push(format!("Git branch: {branch}"));
        }
    }

    let language_hints = language_hints_for(workdir);
    if !language_hints.is_empty() {
        parts.push(format!("Language hints: {}", language_hints.join(", ")));
    }

    Ok(parts.join("\n"))
}

/// Default tools for interactive chat when no safety contract is found.
const DEFAULT_CHAT_TOOLS: &str = "Read,Glob,Grep,Bash,Edit,Write,NotebookEdit";

/// Resolve tool allowlist from safety contracts.
///
/// Looks for an `AgentContract` at `.roko/safety/chat.yaml`. If found, uses
/// its `allowed_tools` field. If not found, falls back to a read-oriented
/// default set and logs a debug message.
fn resolve_tool_policy(workdir: &Path) -> String {
    let contract_path = workdir.join(".roko/safety/chat.yaml");
    match fs::read_to_string(&contract_path) {
        Ok(content) => match serde_yaml::from_str::<AgentContract>(&content) {
            Ok(contract) => {
                if let Some(ref allowlist) = contract.allowed_tools {
                    if !allowlist.is_empty() {
                        tracing::debug!(
                            "chat tool policy from contract: {}",
                            allowlist.join(",")
                        );
                        return allowlist.join(",");
                    }
                }
                tracing::debug!("chat contract has no allowed_tools, using defaults");
                DEFAULT_CHAT_TOOLS.to_string()
            }
            Err(e) => {
                tracing::warn!(
                    "failed to parse chat contract at {}: {e}",
                    contract_path.display()
                );
                DEFAULT_CHAT_TOOLS.to_string()
            }
        },
        Err(_) => {
            tracing::debug!(
                "no chat safety contract at {}, using default tools",
                contract_path.display()
            );
            DEFAULT_CHAT_TOOLS.to_string()
        }
    }
}

fn gather_workspace_conventions(workdir: &Path) -> Option<String> {
    let cargo_toml = read_text_snippet(&workdir.join("Cargo.toml")).unwrap_or_default();
    let (source_samples, file_listing) = collect_workspace_samples(workdir);

    if cargo_toml.is_empty() && source_samples.is_empty() && file_listing.is_empty() {
        return None;
    }

    let source_refs = source_samples
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let file_refs = file_listing.iter().map(String::as_str).collect::<Vec<_>>();
    let conventions = detect_conventions(&cargo_toml, &source_refs, &file_refs);

    if conventions == ProjectConventions::default() {
        return None;
    }

    let fragment = conventions.to_prompt_fragment();
    if fragment.trim().is_empty() {
        None
    } else {
        Some(fragment)
    }
}

fn collect_workspace_samples(workdir: &Path) -> (Vec<String>, Vec<String>) {
    let mut source_samples = Vec::new();
    let mut file_listing = Vec::new();
    collect_workspace_samples_from_dir(
        workdir,
        workdir,
        0,
        &mut source_samples,
        &mut file_listing,
    );
    (source_samples, file_listing)
}

fn collect_workspace_samples_from_dir(
    dir: &Path,
    root: &Path,
    depth: usize,
    source_samples: &mut Vec<String>,
    file_listing: &mut Vec<String>,
) {
    if depth > MAX_WORKSPACE_SCAN_DEPTH || source_samples.len() >= MAX_WORKSPACE_SAMPLE_FILES {
        return;
    }

    let mut entries = match fs::read_dir(dir) {
        Ok(entries) => entries.filter_map(|entry| entry.ok()).collect::<Vec<_>>(),
        Err(_) => return,
    };
    entries.sort_by(|left, right| left.path().cmp(&right.path()));

    for entry in entries {
        if source_samples.len() >= MAX_WORKSPACE_SAMPLE_FILES {
            break;
        }

        let path = entry.path();
        let file_name = path.file_name().and_then(|name| name.to_str());
        if path.is_dir() {
            if file_name.map_or(false, is_skipped_dir_name) {
                continue;
            }
            collect_workspace_samples_from_dir(
                &path,
                root,
                depth + 1,
                source_samples,
                file_listing,
            );
            continue;
        }

        if !path.is_file() || !is_workspace_source_file(&path) {
            continue;
        }

        let relative = path
            .strip_prefix(root)
            .ok()
            .and_then(|relative| relative.to_str())
            .map(|relative| relative.to_string())
            .unwrap_or_else(|| path.to_string_lossy().into_owned());
        file_listing.push(relative);

        if let Some(sample) = read_text_snippet(&path) {
            if !sample.trim().is_empty() {
                source_samples.push(sample);
            }
        }
    }
}

fn read_text_snippet(path: &Path) -> Option<String> {
    let file = fs::File::open(path).ok()?;
    let mut limited = file.take(MAX_WORKSPACE_SAMPLE_BYTES as u64);
    let mut bytes = Vec::new();
    limited.read_to_end(&mut bytes).ok()?;
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

fn capture_git_branch(workdir: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(workdir)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if branch.is_empty() {
        None
    } else {
        Some(branch)
    }
}

fn language_hints_for(workdir: &Path) -> Vec<String> {
    let mut hints = Vec::new();

    if workdir.join("Cargo.toml").is_file() || workdir.join("rust-toolchain.toml").is_file() {
        push_unique_hint(&mut hints, "Rust");
    }
    if workdir.join("package.json").is_file()
        || workdir.join("tsconfig.json").is_file()
        || workdir.join("deno.json").is_file()
        || workdir.join("deno.jsonc").is_file()
    {
        push_unique_hint(&mut hints, "TypeScript/JavaScript");
    }
    if workdir.join("pyproject.toml").is_file()
        || workdir.join("requirements.txt").is_file()
        || workdir.join("uv.lock").is_file()
    {
        push_unique_hint(&mut hints, "Python");
    }
    if workdir.join("go.mod").is_file() {
        push_unique_hint(&mut hints, "Go");
    }
    if workdir.join("pom.xml").is_file()
        || workdir.join("build.gradle").is_file()
        || workdir.join("build.gradle.kts").is_file()
    {
        push_unique_hint(&mut hints, "Java/Kotlin");
    }

    hints
}

fn push_unique_hint(hints: &mut Vec<String>, hint: &str) {
    if !hints.iter().any(|existing| existing == hint) {
        hints.push(hint.to_string());
    }
}

fn project_name_for(workdir: &Path) -> String {
    workdir
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or("unknown")
        .to_string()
}

fn is_workspace_source_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some(
            "rs" | "ts" | "tsx" | "js" | "jsx" | "py" | "go" | "java" | "kt" | "swift" | "rb"
                | "c" | "h" | "cpp" | "hpp" | "cs" | "lua" | "sh"
        )
    )
}

fn is_skipped_dir_name(name: &str) -> bool {
    SKIP_DIR_NAMES.contains(&name)
}

fn discover_mcp_config_stub(_config: &Config, _workdir: &Path) -> Option<PathBuf> {
    // Placeholder until R3_A04 wires MCP discovery into session creation.
    None
}

fn shared_http_client() -> reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(reqwest::Client::new).clone()
}
```

---

## Read-Only Context (do not modify)

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

### `crates/roko-cli/src/orchestrate.rs` (22076 lines — signatures only)

```rust
231:fn domain_uses_git(domain: &TaskDomain) -> bool {
235:fn workflow_enabled_gate_names(gates: &[crate::config::GateConfig]) -> Vec<String> {
248:fn workflow_shell_gate_commands(gates: &[crate::config::GateConfig]) -> Vec<CoreShellGateCommand> {
267:fn resolve_task_role(role_str: Option<&str>) -> AgentRole {
277:fn model_experiments_path(workdir: &Path) -> PathBuf {
284:fn failure_pattern_store_path(workdir: &Path) -> PathBuf {
291:fn pre_agent_remediation_log_path(workdir: &Path) -> PathBuf {
298:fn daimon_state_path(workdir: &Path) -> PathBuf {
302:fn latency_registry_path(workdir: &Path) -> PathBuf {
313:fn routing_log_path(workdir: &Path) -> PathBuf {
319:fn custody_logger_for(workdir: &Path) -> CustodyLogger {
323:fn cfactor_history_path(workdir: &Path) -> PathBuf {
330:struct HeartbeatCounts {
341:struct SectionEffectCatalystSource {
346:impl CatalystSignalSource for SectionEffectCatalystSource {
372:struct StaticCFactorSource {
376:impl CFactorSource for StaticCFactorSource {
443:fn predictive_policy_sections(
475:fn predictive_calibration_summary_section(
503:fn cfactor_policy_sections(source: Arc<dyn CFactorSource>) -> Vec<PromptSection> {
524:fn parse_count_tag(signal: &Engram, key: &str) -> usize {
531:fn top_cfactor_contributors(snapshot: &CFactor) -> (Vec<String>, Vec<String>) {
581:fn task_requirements_for_routing(
645:fn conductor_policy_path(workdir: &Path) -> PathBuf {
649:fn scrub_json_value(value: &serde_json::Value, policy: &ScrubPolicy) -> serde_json::Value {
668:fn scrub_body(body: &Body, policy: &ScrubPolicy) -> Body {
676:fn scrub_signal(signal: &Engram, policy: &ScrubPolicy) -> Engram {
688:fn scrub_agent_result(result: &AgentResult, policy: &ScrubPolicy) -> AgentResult {
702:fn state_dir(workdir: &Path) -> PathBuf {
706:fn executor_snapshot_path(workdir: &Path) -> PathBuf {
710:fn agent_invocation_ledger_path(workdir: &Path) -> PathBuf {
714:fn append_agent_invocation_record(workdir: &Path, record: &AgentInvocationSession) {
745:fn invocation_state_from_agent_result(result: &AgentResult) -> InvocationState {
763:pub fn save_snapshot_atomic(snapshot: &ExecutorSnapshot, path: &Path) -> Result<()> {
785:fn persisted_circuit_breaker_state(state: CircuitBreakerState) -> PersistedCircuitBreakerState {
805:fn restored_circuit_breaker_state(state: PersistedCircuitBreakerState) -> CircuitBreakerState {
848:fn sync_file_if_present(path: &Path) -> Result<()> {
858:fn load_roko_config(workdir: &Path) -> Result<RokoConfig> {
869:fn frequency_label(frequency: OperatingFrequency) -> &'static str {
877:fn task_runner_cost_table(resolved: &roko_core::agent::ResolvedModel) -> RunnerCostTable {
```

---

## Verification Commands

Run these and fix any failures:
```bash
cargo check -p roko-cli
cargo clippy -p roko-cli --no-deps -- -D warnings
```

## Do NOT

- Rewrite the entire implementation from scratch
- Add features not in the original prompt
- Modify files outside the write scope
- Skip running verification commands
