//! Authoritative environment-variable registry for the roko workspace.
//!
//! Every hardcoded `env::var` / `env::var_os` / `env!()` / clap `env = "…"` read
//! across the workspace is catalogued here. The registry powers:
//!
//! - `roko config env list [--json]` — operator reference
//! - Secret redaction (values of `Sensitivity::Secret` are never printed)
//! - Deprecation warnings when legacy alias names are used
//! - Precedence documentation
//!
//! ## Adding a new env var
//!
//! 1. Add an `EnvVarSpec` entry to the appropriate category function below.
//! 2. Run the source-comparison check to confirm the literal is covered.

use serde::Serialize;
use std::fmt;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// How the variable's value should be treated in output.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Sensitivity {
    /// Safe to display.
    Public,
    /// Must be redacted in output — show set/unset only.
    Secret,
}

/// Lifecycle stability.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Stability {
    /// Supported, documented, semver-protected.
    Stable,
    /// Works but may change or be removed.
    Unstable,
    /// Superseded by a canonical name; emits a warning when read.
    Deprecated,
    /// Only meaningful during `cargo build` / `build.rs`.
    BuildTime,
    /// Only read inside `#[cfg(test)]` or integration-test binaries.
    TestOnly,
    /// Demo/example binaries only.
    DemoOnly,
    /// Standard system / third-party convention (HOME, PATH, CI, NO_COLOR, ...).
    System,
}

/// What kind of value the variable expects.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ValueType {
    /// Strict boolean: `1/true/yes/on` or `0/false/no/off`.
    Bool,
    /// Presence-only (any value activates).
    Presence,
    /// Unsigned integer.
    Uint,
    /// Floating-point number.
    Float,
    /// Free-form string.
    String,
    /// URL.
    Url,
    /// Duration in seconds.
    DurationSecs,
    /// Enum with a known set of values.
    Enum,
    /// File-system path.
    Path,
    /// Comma-separated list.
    List,
    /// Base64-encoded blob.
    Base64,
    /// Hex-encoded bytes.
    Hex,
}

/// Functional scope of the variable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    /// Affects the CLI process.
    Cli,
    /// Affects the HTTP server (`roko serve`).
    Server,
    /// Affects agent dispatch / LLM providers.
    Agent,
    /// Affects the gate pipeline (compile, test, clippy).
    Gate,
    /// Affects the runner / plan execution.
    Runner,
    /// Affects the TUI.
    Tui,
    /// Affects ACP sessions.
    Acp,
    /// Affects MCP servers.
    Mcp,
    /// Affects deployment / worker.
    Deploy,
    /// Build-time only.
    Build,
    /// Standard system convention.
    System,
    /// Global / cross-cutting.
    Global,
}

/// One registered environment variable.
#[derive(Clone, Debug, Serialize)]
pub struct EnvVarSpec {
    /// Canonical variable name (e.g. `ROKO_MODEL`).
    pub name: &'static str,
    /// Owning subsystem (for grouping in output).
    pub owner: &'static str,
    /// Short description.
    pub purpose: &'static str,
    /// Expected value shape.
    pub value_type: ValueType,
    /// Human-readable default (empty string if required / no default).
    pub default: &'static str,
    /// Precedence note (e.g. "CLI --model flag > env > config").
    pub precedence: &'static str,
    /// Functional scope.
    pub scope: Scope,
    /// Whether the value is secret.
    pub sensitivity: Sensitivity,
    /// Lifecycle stability.
    pub stability: Stability,
    /// If deprecated, the canonical replacement name.
    pub replacement: Option<&'static str>,
}

impl fmt::Display for EnvVarSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:<42} ", self.name)?;
        match self.sensitivity {
            Sensitivity::Secret => write!(f, "[SECRET] ")?,
            Sensitivity::Public => {}
        }
        if self.stability == Stability::Deprecated {
            if let Some(repl) = self.replacement {
                write!(f, "[DEPRECATED -> {}] ", repl)?;
            } else {
                write!(f, "[DEPRECATED] ")?;
            }
        }
        write!(f, "{}", self.purpose)
    }
}

// ---------------------------------------------------------------------------
// Registry construction
// ---------------------------------------------------------------------------

/// Return the full ordered registry of all known environment variables.
///
/// The order is deterministic: grouped by owner/category, alphabetical within.
#[must_use]
pub fn env_registry() -> Vec<EnvVarSpec> {
    let mut specs = Vec::with_capacity(128);
    specs.extend(cli_overrides());
    specs.extend(logging_diagnostics());
    specs.extend(terminal_color());
    specs.extend(tui_accessibility());
    specs.extend(config_system());
    specs.extend(config_schema_env());
    specs.extend(provider_keys());
    specs.extend(github_integration());
    specs.extend(slack_integration());
    specs.extend(server_deploy());
    specs.extend(runner_gate());
    specs.extend(agent_dispatch());
    specs.extend(acp_vars());
    specs.extend(extension_registry());
    specs.extend(mcp_scripts());
    specs.extend(fast_mode());
    specs.extend(build_time());
    specs.extend(system_standard());
    specs.extend(demo_only());
    specs.extend(test_only());
    specs
}

/// Print the registry as a formatted table to stdout.
pub fn print_env_list(json: bool) {
    let registry = env_registry();
    if json {
        // serde_json is already a dependency of roko-core.
        println!(
            "{}",
            serde_json::to_string_pretty(&registry).unwrap_or_else(|_| "[]".into())
        );
        return;
    }

    let mut current_owner = "";
    for spec in &registry {
        if spec.owner != current_owner {
            if !current_owner.is_empty() {
                println!();
            }
            println!("── {} ──", spec.owner);
            current_owner = spec.owner;
        }
        // For secrets: show set/unset status, never values.
        let status = match spec.sensitivity {
            Sensitivity::Secret => {
                if std::env::var(spec.name).is_ok() {
                    " (set)"
                } else {
                    " (unset)"
                }
            }
            Sensitivity::Public => "",
        };
        let stability_tag = match spec.stability {
            Stability::Deprecated => {
                if let Some(repl) = spec.replacement {
                    format!(" [deprecated -> {repl}]")
                } else {
                    " [deprecated]".to_string()
                }
            }
            Stability::Unstable => " [unstable]".to_string(),
            Stability::TestOnly => " [test-only]".to_string(),
            Stability::DemoOnly => " [demo-only]".to_string(),
            Stability::BuildTime => " [build-time]".to_string(),
            Stability::System => " [system]".to_string(),
            Stability::Stable => String::new(),
        };
        println!(
            "  {:<42} {}{status}{stability_tag}",
            spec.name, spec.purpose,
        );
        if !spec.default.is_empty() {
            println!("    default: {}", spec.default);
        }
        if !spec.precedence.is_empty() {
            println!("    precedence: {}", spec.precedence);
        }
    }
    println!();
    // Summary line.
    let total = registry.len();
    let secret_count = registry
        .iter()
        .filter(|s| s.sensitivity == Sensitivity::Secret)
        .count();
    let deprecated_count = registry
        .iter()
        .filter(|s| s.stability == Stability::Deprecated)
        .count();
    println!(
        "{total} registered variables ({secret_count} secret, {deprecated_count} deprecated)"
    );
}

// ---------------------------------------------------------------------------
// Category builders (private)
// ---------------------------------------------------------------------------

fn cli_overrides() -> Vec<EnvVarSpec> {
    vec![
        EnvVarSpec {
            name: "ROKO_MODEL",
            owner: "CLI overrides",
            purpose: "Override default model name",
            value_type: ValueType::String,
            default: "",
            precedence: "CLI --model > env > config agent.default_model",
            scope: Scope::Cli,
            sensitivity: Sensitivity::Public,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "ROKO_EFFORT",
            owner: "CLI overrides",
            purpose: "Override default effort level",
            value_type: ValueType::String,
            default: "",
            precedence: "CLI --effort > env > config agent.default_effort",
            scope: Scope::Cli,
            sensitivity: Sensitivity::Public,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "ROKO_ROLE",
            owner: "CLI overrides",
            purpose: "Override default agent role",
            value_type: ValueType::String,
            default: "",
            precedence: "CLI --role > env > config",
            scope: Scope::Cli,
            sensitivity: Sensitivity::Public,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "ROKO_QUIET",
            owner: "CLI overrides",
            purpose: "Suppress non-essential output",
            value_type: ValueType::Bool,
            default: "false",
            precedence: "CLI --quiet > env",
            scope: Scope::Cli,
            sensitivity: Sensitivity::Public,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "ROKO_LOG_FORMAT",
            owner: "CLI overrides",
            purpose: "Log output format: json or text",
            value_type: ValueType::Enum,
            default: "text",
            precedence: "CLI --log-format > env",
            scope: Scope::Cli,
            sensitivity: Sensitivity::Public,
            stability: Stability::Stable,
            replacement: None,
        },
    ]
}

fn logging_diagnostics() -> Vec<EnvVarSpec> {
    vec![
        EnvVarSpec {
            name: "ROKO_LOG",
            owner: "Logging",
            purpose: "Application log verbosity / tracing filter",
            value_type: ValueType::String,
            default: "roko=info",
            precedence: "ROKO_LOG > RUST_LOG > default",
            scope: Scope::Global,
            sensitivity: Sensitivity::Public,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "RUST_LOG",
            owner: "Logging",
            purpose: "Compatibility fallback for log verbosity",
            value_type: ValueType::String,
            default: "roko=info",
            precedence: "ROKO_LOG > RUST_LOG > default",
            scope: Scope::Global,
            sensitivity: Sensitivity::Public,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "ROKO_TIMING",
            owner: "Logging",
            purpose: "Enable per-operation timing output",
            value_type: ValueType::Bool,
            default: "false",
            precedence: "CLI --timing > env",
            scope: Scope::Cli,
            sensitivity: Sensitivity::Public,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "ROKO_LOG_RAW",
            owner: "Logging",
            purpose: "Disable secret redaction in log output",
            value_type: ValueType::Bool,
            default: "false",
            precedence: "env only",
            scope: Scope::Cli,
            sensitivity: Sensitivity::Public,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "ROKO_VERBOSE",
            owner: "Logging",
            purpose: "Print model selection details to stderr",
            value_type: ValueType::Bool,
            default: "false",
            precedence: "env only",
            scope: Scope::Cli,
            sensitivity: Sensitivity::Public,
            stability: Stability::Deprecated,
            replacement: Some("ROKO_LOG=debug"),
        },
        EnvVarSpec {
            name: "ROKO_DEBUG",
            owner: "Logging",
            purpose: "Extra debug logging for Claude CLI agent",
            value_type: ValueType::Bool,
            default: "false",
            precedence: "env only",
            scope: Scope::Agent,
            sensitivity: Sensitivity::Public,
            stability: Stability::Deprecated,
            replacement: Some("ROKO_LOG=debug"),
        },
    ]
}

fn terminal_color() -> Vec<EnvVarSpec> {
    vec![
        EnvVarSpec {
            name: "NO_COLOR",
            owner: "Terminal",
            purpose: "Disable ANSI color output (standard convention)",
            value_type: ValueType::Presence,
            default: "",
            precedence: "NO_COLOR > CLICOLOR_FORCE > CLICOLOR",
            scope: Scope::Cli,
            sensitivity: Sensitivity::Public,
            stability: Stability::System,
            replacement: None,
        },
        EnvVarSpec {
            name: "CLICOLOR_FORCE",
            owner: "Terminal",
            purpose: "Force color output even when not a TTY",
            value_type: ValueType::String,
            default: "",
            precedence: "NO_COLOR > CLICOLOR_FORCE > CLICOLOR",
            scope: Scope::Cli,
            sensitivity: Sensitivity::Public,
            stability: Stability::System,
            replacement: None,
        },
        EnvVarSpec {
            name: "CLICOLOR",
            owner: "Terminal",
            purpose: "Disable color when set to 0",
            value_type: ValueType::String,
            default: "",
            precedence: "NO_COLOR > CLICOLOR_FORCE > CLICOLOR",
            scope: Scope::Cli,
            sensitivity: Sensitivity::Public,
            stability: Stability::System,
            replacement: None,
        },
    ]
}

fn tui_accessibility() -> Vec<EnvVarSpec> {
    vec![
        EnvVarSpec {
            name: "ROKO_REDUCED_MOTION",
            owner: "TUI accessibility",
            purpose: "Disable all TUI animations",
            value_type: ValueType::Presence,
            default: "",
            precedence: "env only",
            scope: Scope::Tui,
            sensitivity: Sensitivity::Public,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "ROKO_HIGH_CONTRAST",
            owner: "TUI accessibility",
            purpose: "Enable high-contrast TUI palette",
            value_type: ValueType::Presence,
            default: "",
            precedence: "env only",
            scope: Scope::Tui,
            sensitivity: Sensitivity::Public,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "ROKO_VIEWPORT_HEIGHT",
            owner: "TUI accessibility",
            purpose: "Override terminal viewport height",
            value_type: ValueType::Uint,
            default: "auto-detected",
            precedence: "env > terminal detection",
            scope: Scope::Tui,
            sensitivity: Sensitivity::Public,
            stability: Stability::Stable,
            replacement: None,
        },
    ]
}

fn config_system() -> Vec<EnvVarSpec> {
    vec![
        EnvVarSpec {
            name: "ROKO_CONFIG",
            owner: "Configuration",
            purpose: "Explicit path to roko.toml config file",
            value_type: ValueType::Path,
            default: "ancestor-walk discovery",
            precedence: "CLI --config > env > ancestor-walk",
            scope: Scope::Global,
            sensitivity: Sensitivity::Public,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "ROKO__*",
            owner: "Configuration",
            purpose: "Hierarchical config override (ROKO__section__key = value)",
            value_type: ValueType::String,
            default: "",
            precedence: "CLI > named env > ROKO__* > config file > default",
            scope: Scope::Global,
            sensitivity: Sensitivity::Public,
            stability: Stability::Stable,
            replacement: None,
        },
    ]
}

fn config_schema_env() -> Vec<EnvVarSpec> {
    vec![
        EnvVarSpec {
            name: "ROKO_PROVIDER",
            owner: "Config schema overrides",
            purpose: "Override default provider name",
            value_type: ValueType::String,
            default: "",
            precedence: "env > config",
            scope: Scope::Agent,
            sensitivity: Sensitivity::Public,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "ROKO_MODEL_SLUG",
            owner: "Config schema overrides",
            purpose: "Override default model slug",
            value_type: ValueType::String,
            default: "",
            precedence: "env > config",
            scope: Scope::Agent,
            sensitivity: Sensitivity::Public,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "ROKO_BACKEND",
            owner: "Config schema overrides",
            purpose: "Override default agent backend",
            value_type: ValueType::String,
            default: "",
            precedence: "env > config agent.default_backend",
            scope: Scope::Agent,
            sensitivity: Sensitivity::Public,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "ROKO_CONTEXT_LIMIT_K",
            owner: "Config schema overrides",
            purpose: "Override agent context window limit (thousands)",
            value_type: ValueType::Uint,
            default: "200",
            precedence: "CLI --context-limit-k > env > config",
            scope: Scope::Agent,
            sensitivity: Sensitivity::Public,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "ROKO_MAX_AGENTS",
            owner: "Config schema overrides",
            purpose: "Maximum concurrent agents",
            value_type: ValueType::Uint,
            default: "5",
            precedence: "CLI --max-agents > env > config conductor.max_agents",
            scope: Scope::Runner,
            sensitivity: Sensitivity::Public,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "ROKO_BUDGET_USD",
            owner: "Config schema overrides",
            purpose: "Maximum plan budget in USD",
            value_type: ValueType::Float,
            default: "50.0",
            precedence: "CLI --budget > env > config budget.max_plan_usd",
            scope: Scope::Runner,
            sensitivity: Sensitivity::Public,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "ROKO_PARALLEL",
            owner: "Config schema overrides",
            purpose: "Enable parallel task execution",
            value_type: ValueType::Bool,
            default: "true",
            precedence: "env > config conductor.parallel_enabled",
            scope: Scope::Runner,
            sensitivity: Sensitivity::Public,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "ROKO_EXPRESS",
            owner: "Config schema overrides",
            purpose: "Enable express mode (skip non-essential steps)",
            value_type: ValueType::Bool,
            default: "false",
            precedence: "env > config conductor.express_mode",
            scope: Scope::Runner,
            sensitivity: Sensitivity::Public,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "ROKO_SKIP_TESTS",
            owner: "Config schema overrides",
            purpose: "Skip test gates",
            value_type: ValueType::Bool,
            default: "false",
            precedence: "env > config gates.skip_tests",
            scope: Scope::Gate,
            sensitivity: Sensitivity::Public,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "ROKO_CLIPPY",
            owner: "Config schema overrides",
            purpose: "Enable clippy gate",
            value_type: ValueType::Bool,
            default: "true",
            precedence: "env > config gates.clippy_enabled",
            scope: Scope::Gate,
            sensitivity: Sensitivity::Public,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "ROKO_GATE_MODE",
            owner: "Config schema overrides",
            purpose: "Gate breadth: none, structural, focused, full",
            value_type: ValueType::Enum,
            default: "full",
            precedence: "env > config gates.mode",
            scope: Scope::Gate,
            sensitivity: Sensitivity::Public,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "ROKO_COMPILE_CONCURRENCY",
            owner: "Config schema overrides",
            purpose: "Cargo compile concurrency (positive integer)",
            value_type: ValueType::Uint,
            default: "system default",
            precedence: "env > config gates.compile_concurrency",
            scope: Scope::Gate,
            sensitivity: Sensitivity::Public,
            stability: Stability::Stable,
            replacement: None,
        },
    ]
}

fn provider_keys() -> Vec<EnvVarSpec> {
    vec![
        EnvVarSpec {
            name: "ANTHROPIC_API_KEY",
            owner: "Provider API keys",
            purpose: "Anthropic API key",
            value_type: ValueType::String,
            default: "",
            precedence: "env > config providers.*.api_key_env",
            scope: Scope::Agent,
            sensitivity: Sensitivity::Secret,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "OPENAI_API_KEY",
            owner: "Provider API keys",
            purpose: "OpenAI API key",
            value_type: ValueType::String,
            default: "",
            precedence: "env > config providers.*.api_key_env",
            scope: Scope::Agent,
            sensitivity: Sensitivity::Secret,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "OPENAI_API_BASE",
            owner: "Provider API keys",
            purpose: "Custom OpenAI-compatible API base URL",
            value_type: ValueType::Url,
            default: "https://api.openai.com/v1",
            precedence: "OPENAI_API_BASE > OPENAI_BASE_URL > default",
            scope: Scope::Agent,
            sensitivity: Sensitivity::Public,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "OPENAI_BASE_URL",
            owner: "Provider API keys",
            purpose: "Fallback for OPENAI_API_BASE",
            value_type: ValueType::Url,
            default: "https://api.openai.com/v1",
            precedence: "OPENAI_API_BASE > OPENAI_BASE_URL > default",
            scope: Scope::Agent,
            sensitivity: Sensitivity::Public,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "GEMINI_API_KEY",
            owner: "Provider API keys",
            purpose: "Google Gemini API key",
            value_type: ValueType::String,
            default: "",
            precedence: "env > config",
            scope: Scope::Agent,
            sensitivity: Sensitivity::Secret,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "PERPLEXITY_API_KEY",
            owner: "Provider API keys",
            purpose: "Perplexity API key (research/search)",
            value_type: ValueType::String,
            default: "",
            precedence: "env > config",
            scope: Scope::Agent,
            sensitivity: Sensitivity::Secret,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "DEEPSEEK_API_KEY",
            owner: "Provider API keys",
            purpose: "DeepSeek API key",
            value_type: ValueType::String,
            default: "",
            precedence: "env > config",
            scope: Scope::Agent,
            sensitivity: Sensitivity::Secret,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "XAI_API_KEY",
            owner: "Provider API keys",
            purpose: "xAI API key",
            value_type: ValueType::String,
            default: "",
            precedence: "env > config",
            scope: Scope::Agent,
            sensitivity: Sensitivity::Secret,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "CEREBRAS_API_KEY",
            owner: "Provider API keys",
            purpose: "Cerebras API key",
            value_type: ValueType::String,
            default: "",
            precedence: "env > config",
            scope: Scope::Agent,
            sensitivity: Sensitivity::Secret,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "OPENROUTER_API_KEY",
            owner: "Provider API keys",
            purpose: "OpenRouter API key",
            value_type: ValueType::String,
            default: "",
            precedence: "env > config",
            scope: Scope::Agent,
            sensitivity: Sensitivity::Secret,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "FIREWORKS_API_KEY",
            owner: "Provider API keys",
            purpose: "Fireworks AI API key",
            value_type: ValueType::String,
            default: "",
            precedence: "env > config",
            scope: Scope::Agent,
            sensitivity: Sensitivity::Secret,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "TOGETHER_API_KEY",
            owner: "Provider API keys",
            purpose: "Together AI API key",
            value_type: ValueType::String,
            default: "",
            precedence: "env > config",
            scope: Scope::Agent,
            sensitivity: Sensitivity::Secret,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "MOONSHOT_API_KEY",
            owner: "Provider API keys",
            purpose: "Moonshot API key",
            value_type: ValueType::String,
            default: "",
            precedence: "env > config",
            scope: Scope::Agent,
            sensitivity: Sensitivity::Secret,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "ZAI_API_KEY",
            owner: "Provider API keys",
            purpose: "Zhipu/GLM API key",
            value_type: ValueType::String,
            default: "",
            precedence: "env > config",
            scope: Scope::Agent,
            sensitivity: Sensitivity::Secret,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "ZAI_MODEL",
            owner: "Provider API keys",
            purpose: "Zhipu/GLM model name override",
            value_type: ValueType::String,
            default: "glm-5.1",
            precedence: "env > default",
            scope: Scope::Agent,
            sensitivity: Sensitivity::Public,
            stability: Stability::Stable,
            replacement: None,
        },
    ]
}

fn github_integration() -> Vec<EnvVarSpec> {
    vec![
        EnvVarSpec {
            name: "GITHUB_TOKEN",
            owner: "GitHub",
            purpose: "GitHub personal access token",
            value_type: ValueType::String,
            default: "",
            precedence: "GITHUB_TOKEN > GH_TOKEN",
            scope: Scope::Global,
            sensitivity: Sensitivity::Secret,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "GH_TOKEN",
            owner: "GitHub",
            purpose: "Fallback GitHub token (gh CLI convention)",
            value_type: ValueType::String,
            default: "",
            precedence: "GITHUB_TOKEN > GH_TOKEN",
            scope: Scope::Global,
            sensitivity: Sensitivity::Secret,
            stability: Stability::Stable,
            replacement: None,
        },
    ]
}

fn slack_integration() -> Vec<EnvVarSpec> {
    vec![
        EnvVarSpec {
            name: "SLACK_BOT_TOKEN",
            owner: "Slack",
            purpose: "Slack bot OAuth token",
            value_type: ValueType::String,
            default: "",
            precedence: "SLACK_BOT_TOKEN > SLACK_TOKEN",
            scope: Scope::Server,
            sensitivity: Sensitivity::Secret,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "SLACK_TOKEN",
            owner: "Slack",
            purpose: "Fallback Slack token",
            value_type: ValueType::String,
            default: "",
            precedence: "SLACK_BOT_TOKEN > SLACK_TOKEN",
            scope: Scope::Server,
            sensitivity: Sensitivity::Secret,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "SLACK_SIGNING_SECRET",
            owner: "Slack",
            purpose: "Slack request signing secret (webhooks)",
            value_type: ValueType::String,
            default: "",
            precedence: "env only; required for webhook verification",
            scope: Scope::Server,
            sensitivity: Sensitivity::Secret,
            stability: Stability::Stable,
            replacement: None,
        },
    ]
}

fn server_deploy() -> Vec<EnvVarSpec> {
    vec![
        EnvVarSpec {
            name: "PORT",
            owner: "Server / deploy",
            purpose: "HTTP server listen port",
            value_type: ValueType::Uint,
            default: "6677",
            precedence: "CLI --port > env > config serve.port > 6677",
            scope: Scope::Server,
            sensitivity: Sensitivity::Public,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "ROKO_SERVER_AUTH_TOKEN",
            owner: "Server / deploy",
            purpose: "Bearer token for serve auth middleware",
            value_type: ValueType::String,
            default: "",
            precedence: "env > config serve.auth.auth_token_env",
            scope: Scope::Server,
            sensitivity: Sensitivity::Secret,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "ROKO_SERVE_URL",
            owner: "Server / deploy",
            purpose: "URL of the roko serve instance",
            value_type: ValueType::Url,
            default: "http://localhost:6677",
            precedence: "ROKO_SERVE_URL > ROKO_SERVER_URL > default",
            scope: Scope::Global,
            sensitivity: Sensitivity::Public,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "ROKO_SERVER_URL",
            owner: "Server / deploy",
            purpose: "Fallback serve URL (use ROKO_SERVE_URL instead)",
            value_type: ValueType::Url,
            default: "http://localhost:6677",
            precedence: "ROKO_SERVE_URL > ROKO_SERVER_URL > default",
            scope: Scope::Global,
            sensitivity: Sensitivity::Public,
            stability: Stability::Deprecated,
            replacement: Some("ROKO_SERVE_URL"),
        },
        EnvVarSpec {
            name: "ROKO_API_KEY",
            owner: "Server / deploy",
            purpose: "API key for authenticated serve requests",
            value_type: ValueType::String,
            default: "",
            precedence: "CLI flag > env > config serve.auth.api_key > stored credential",
            scope: Scope::Server,
            sensitivity: Sensitivity::Secret,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "ROKO_SPA_DIR",
            owner: "Server / deploy",
            purpose: "Path to SPA frontend assets",
            value_type: ValueType::Path,
            default: "embedded assets",
            precedence: "env > compile-time path",
            scope: Scope::Server,
            sensitivity: Sensitivity::Public,
            stability: Stability::Unstable,
            replacement: None,
        },
        EnvVarSpec {
            name: "ROKO_TEMPLATE_JSON",
            owner: "Server / deploy",
            purpose: "Base64-encoded template for worker execution",
            value_type: ValueType::Base64,
            default: "",
            precedence: "env only; required for worker mode",
            scope: Scope::Deploy,
            sensitivity: Sensitivity::Public,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "ROKO_CONTROL_PLANE_URL",
            owner: "Server / deploy",
            purpose: "Control plane URL for worker callback",
            value_type: ValueType::Url,
            default: "",
            precedence: "env only",
            scope: Scope::Deploy,
            sensitivity: Sensitivity::Public,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "ROKO_DEPLOYMENT_ID",
            owner: "Server / deploy",
            purpose: "Deployment identifier for worker registration",
            value_type: ValueType::String,
            default: "",
            precedence: "env only",
            scope: Scope::Deploy,
            sensitivity: Sensitivity::Public,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "ROKO_WORKER_CALLBACK_TOKEN",
            owner: "Server / deploy",
            purpose: "Hashed token verifier for worker callbacks",
            value_type: ValueType::String,
            default: "",
            precedence: "env only",
            scope: Scope::Deploy,
            sensitivity: Sensitivity::Secret,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "RAILWAY_PUBLIC_DOMAIN",
            owner: "Server / deploy",
            purpose: "Railway platform public domain",
            value_type: ValueType::String,
            default: "",
            precedence: "RAILWAY_PUBLIC_DOMAIN > FLY_APP_NAME > localhost",
            scope: Scope::Deploy,
            sensitivity: Sensitivity::Public,
            stability: Stability::System,
            replacement: None,
        },
        EnvVarSpec {
            name: "FLY_APP_NAME",
            owner: "Server / deploy",
            purpose: "Fly.io application name",
            value_type: ValueType::String,
            default: "",
            precedence: "RAILWAY_PUBLIC_DOMAIN > FLY_APP_NAME > localhost",
            scope: Scope::Deploy,
            sensitivity: Sensitivity::Public,
            stability: Stability::System,
            replacement: None,
        },
        EnvVarSpec {
            name: "ROKO_MIRAGE_URL",
            owner: "Server / deploy",
            purpose: "Mirage devnet RPC URL",
            value_type: ValueType::Url,
            default: "http://127.0.0.1:8545",
            precedence: "env > config > default",
            scope: Scope::Server,
            sensitivity: Sensitivity::Public,
            stability: Stability::Unstable,
            replacement: None,
        },
        EnvVarSpec {
            name: "ROKO_AGENT_RELAY_URL",
            owner: "Server / deploy",
            purpose: "Agent relay service URL",
            value_type: ValueType::Url,
            default: "",
            precedence: "env > config relay.url",
            scope: Scope::Server,
            sensitivity: Sensitivity::Public,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "NUNCHI_DASHBOARD_URL",
            owner: "Server / deploy",
            purpose: "Dashboard URL for roko login browser auth",
            value_type: ValueType::Url,
            default: "http://localhost:5173",
            precedence: "clap env > default",
            scope: Scope::Cli,
            sensitivity: Sensitivity::Public,
            stability: Stability::Stable,
            replacement: None,
        },
    ]
}

fn runner_gate() -> Vec<EnvVarSpec> {
    vec![
        EnvVarSpec {
            name: "ROKO_SKIP_PREFLIGHT",
            owner: "Runner / gate",
            purpose: "Skip preflight checks before task execution",
            value_type: ValueType::Bool,
            default: "false",
            precedence: "env only",
            scope: Scope::Runner,
            sensitivity: Sensitivity::Public,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "ROKO_TASK_VERIFY_ONLY",
            owner: "Runner / gate",
            purpose: "Run only the authored verify command, skip pipeline gates",
            value_type: ValueType::Bool,
            default: "false",
            precedence: "env only",
            scope: Scope::Gate,
            sensitivity: Sensitivity::Public,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "ROKO_ACP_PROGRESS",
            owner: "Runner / gate",
            purpose: "ACP progress reporting sink URL",
            value_type: ValueType::String,
            default: "",
            precedence: "env only",
            scope: Scope::Acp,
            sensitivity: Sensitivity::Public,
            stability: Stability::Unstable,
            replacement: None,
        },
        EnvVarSpec {
            name: "ROKO_EXPLICIT_CARGO_CLEAN",
            owner: "Runner / gate",
            purpose: "Enable explicit cargo clean of task build artifacts",
            value_type: ValueType::Bool,
            default: "false",
            precedence: "env only",
            scope: Scope::Runner,
            sensitivity: Sensitivity::Public,
            stability: Stability::Stable,
            replacement: None,
        },
    ]
}

fn agent_dispatch() -> Vec<EnvVarSpec> {
    vec![
        EnvVarSpec {
            name: "ROKO_MCP_CONFIG",
            owner: "Agent dispatch",
            purpose: "Explicit path to .mcp.json configuration",
            value_type: ValueType::Path,
            default: "walk-up .mcp.json discovery",
            precedence: "env > walk-up discovery",
            scope: Scope::Agent,
            sensitivity: Sensitivity::Public,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "ROKO_WORKSPACE_ROOT",
            owner: "Agent dispatch",
            purpose: "Override workspace root for code intelligence MCP",
            value_type: ValueType::Path,
            default: "current directory",
            precedence: "env > cwd",
            scope: Scope::Mcp,
            sensitivity: Sensitivity::Public,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "ROKO_ATTEST_SIGNING_KEY_HEX",
            owner: "Agent dispatch",
            purpose: "Hex-encoded signing key for output attestations",
            value_type: ValueType::Hex,
            default: "",
            precedence: "env only; absent = no attestation",
            scope: Scope::Runner,
            sensitivity: Sensitivity::Secret,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "ROKO_DISPATCHER",
            owner: "Agent dispatch",
            purpose: "Override agent dispatcher (mock-<fixture> for testing)",
            value_type: ValueType::String,
            default: "",
            precedence: "env only",
            scope: Scope::Agent,
            sensitivity: Sensitivity::Public,
            stability: Stability::Unstable,
            replacement: None,
        },
        EnvVarSpec {
            name: "ROKO_MOCK_STATE_PATH",
            owner: "Agent dispatch",
            purpose: "Path for mock agent state file",
            value_type: ValueType::Path,
            default: "",
            precedence: "env only",
            scope: Scope::Agent,
            sensitivity: Sensitivity::Public,
            stability: Stability::Unstable,
            replacement: None,
        },
        EnvVarSpec {
            name: "ROKO_DEMO_CACHE",
            owner: "Agent dispatch",
            purpose: "Enable file caching for demo scenarios",
            value_type: ValueType::Bool,
            default: "false",
            precedence: "env only",
            scope: Scope::Agent,
            sensitivity: Sensitivity::Public,
            stability: Stability::Unstable,
            replacement: None,
        },
    ]
}

fn acp_vars() -> Vec<EnvVarSpec> {
    vec![
        EnvVarSpec {
            name: "ROKO_ACP_CASCADE_SELECT",
            owner: "ACP",
            purpose: "Enable cascade model selection in ACP sessions",
            value_type: ValueType::Bool,
            default: "false",
            precedence: "env only",
            scope: Scope::Acp,
            sensitivity: Sensitivity::Public,
            stability: Stability::Unstable,
            replacement: None,
        },
        EnvVarSpec {
            name: "ROKO_ACP_LEGACY",
            owner: "ACP",
            purpose: "Activate legacy ACP behavior paths",
            value_type: ValueType::Presence,
            default: "",
            precedence: "env only; removal planned",
            scope: Scope::Acp,
            sensitivity: Sensitivity::Public,
            stability: Stability::Deprecated,
            replacement: None,
        },
    ]
}

fn extension_registry() -> Vec<EnvVarSpec> {
    vec![
        EnvVarSpec {
            name: "ROKO_EXTENSION_REGISTRY_URL",
            owner: "Extensions",
            purpose: "Extension registry URL",
            value_type: ValueType::Url,
            default: "",
            precedence: "env > config relay.url",
            scope: Scope::Server,
            sensitivity: Sensitivity::Public,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "ROKO_EXTENSION_REGISTRY_PUBLISH_TOKEN",
            owner: "Extensions",
            purpose: "Auth token for extension publishing",
            value_type: ValueType::String,
            default: "",
            precedence: "env only; required for publish",
            scope: Scope::Cli,
            sensitivity: Sensitivity::Secret,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "ROKO_EXTENSION_REGISTRY_SIGNING_KEY",
            owner: "Extensions",
            purpose: "Signing key for extension packages",
            value_type: ValueType::String,
            default: "",
            precedence: "env only; required for publish",
            scope: Scope::Cli,
            sensitivity: Sensitivity::Secret,
            stability: Stability::Stable,
            replacement: None,
        },
    ]
}

fn mcp_scripts() -> Vec<EnvVarSpec> {
    vec![
        EnvVarSpec {
            name: "ROKO_MCP_SCRIPTS_TIMEOUT_SECS",
            owner: "MCP scripts",
            purpose: "Script execution timeout in seconds",
            value_type: ValueType::DurationSecs,
            default: "60",
            precedence: "env > default",
            scope: Scope::Mcp,
            sensitivity: Sensitivity::Public,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "ROKO_MCP_SCRIPTS_ENV_ALLOWLIST",
            owner: "MCP scripts",
            purpose: "Comma-separated list of env vars passed to scripts",
            value_type: ValueType::List,
            default: "PATH, HOME, etc.",
            precedence: "env > default allowlist",
            scope: Scope::Mcp,
            sensitivity: Sensitivity::Public,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "ROKO_MCP_SCRIPTS_DIR",
            owner: "MCP scripts",
            purpose: "Directory containing MCP scripts",
            value_type: ValueType::Path,
            default: "default scripts paths",
            precedence: "ROKO_SCRIPTS_DIR (deprecated) > ROKO_MCP_SCRIPTS_DIR > default",
            scope: Scope::Mcp,
            sensitivity: Sensitivity::Public,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "ROKO_SCRIPTS_DIR",
            owner: "MCP scripts",
            purpose: "Deprecated alias for ROKO_MCP_SCRIPTS_DIR",
            value_type: ValueType::Path,
            default: "",
            precedence: "ROKO_SCRIPTS_DIR > ROKO_MCP_SCRIPTS_DIR > default",
            scope: Scope::Mcp,
            sensitivity: Sensitivity::Public,
            stability: Stability::Deprecated,
            replacement: Some("ROKO_MCP_SCRIPTS_DIR"),
        },
    ]
}

fn fast_mode() -> Vec<EnvVarSpec> {
    vec![
        EnvVarSpec {
            name: "ROKO_FAST_MODE",
            owner: "FAST mode",
            purpose: "Enable FAST self-development mode",
            value_type: ValueType::Bool,
            default: "false",
            precedence: "env only (set by dev.sh fast)",
            scope: Scope::Runner,
            sensitivity: Sensitivity::Public,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "ROKO_FAST_MAX_AGENT_TURNS",
            owner: "FAST mode",
            purpose: "Maximum agent turns per task in FAST mode",
            value_type: ValueType::Uint,
            default: "25",
            precedence: "env > default",
            scope: Scope::Runner,
            sensitivity: Sensitivity::Public,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "ROKO_FAST_PLAN_DEADLINE_SECS",
            owner: "FAST mode",
            purpose: "Plan-level time budget in FAST mode (seconds)",
            value_type: ValueType::DurationSecs,
            default: "900",
            precedence: "env > default",
            scope: Scope::Runner,
            sensitivity: Sensitivity::Public,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "ROKO_FAST_SETTLEMENT_HEADROOM_SECS",
            owner: "FAST mode",
            purpose: "Settlement headroom before deadline in FAST mode",
            value_type: ValueType::DurationSecs,
            default: "30",
            precedence: "env > default",
            scope: Scope::Runner,
            sensitivity: Sensitivity::Public,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "ROKO_FAST_STARTUP_DEADLINE_SECS",
            owner: "FAST mode",
            purpose: "Startup phase timeout in FAST mode",
            value_type: ValueType::DurationSecs,
            default: "120",
            precedence: "env > default",
            scope: Scope::Runner,
            sensitivity: Sensitivity::Public,
            stability: Stability::Stable,
            replacement: None,
        },
        EnvVarSpec {
            name: "ROKO_EVIDENCE_BUNDLE",
            owner: "FAST mode",
            purpose: "Path for evidence bundle output in FAST mode",
            value_type: ValueType::Path,
            default: "",
            precedence: "env only; absent = no evidence bundle",
            scope: Scope::Runner,
            sensitivity: Sensitivity::Public,
            stability: Stability::Stable,
            replacement: None,
        },
    ]
}

fn build_time() -> Vec<EnvVarSpec> {
    vec![
        EnvVarSpec {
            name: "ROKO_BUILD_FRONTEND",
            owner: "Build",
            purpose: "Force frontend build during cargo build of roko-serve",
            value_type: ValueType::Bool,
            default: "false",
            precedence: "env only (build.rs)",
            scope: Scope::Build,
            sensitivity: Sensitivity::Public,
            stability: Stability::BuildTime,
            replacement: None,
        },
        EnvVarSpec {
            name: "SKIP_FRONTEND_BUILD",
            owner: "Build",
            purpose: "Skip frontend build during cargo build of roko-serve",
            value_type: ValueType::Presence,
            default: "",
            precedence: "env only (build.rs)",
            scope: Scope::Build,
            sensitivity: Sensitivity::Public,
            stability: Stability::BuildTime,
            replacement: None,
        },
    ]
}

fn system_standard() -> Vec<EnvVarSpec> {
    vec![
        EnvVarSpec {
            name: "HOME",
            owner: "System",
            purpose: "User home directory",
            value_type: ValueType::Path,
            default: "",
            precedence: "HOME > USERPROFILE (Windows)",
            scope: Scope::System,
            sensitivity: Sensitivity::Public,
            stability: Stability::System,
            replacement: None,
        },
        EnvVarSpec {
            name: "PATH",
            owner: "System",
            purpose: "Binary lookup path",
            value_type: ValueType::String,
            default: "",
            precedence: "system",
            scope: Scope::System,
            sensitivity: Sensitivity::Public,
            stability: Stability::System,
            replacement: None,
        },
        EnvVarSpec {
            name: "EDITOR",
            owner: "System",
            purpose: "Default text editor for roko config edit",
            value_type: ValueType::String,
            default: "vi",
            precedence: "env > vi",
            scope: Scope::System,
            sensitivity: Sensitivity::Public,
            stability: Stability::System,
            replacement: None,
        },
        EnvVarSpec {
            name: "CI",
            owner: "System",
            purpose: "CI environment detection (timeout scaling, test skipping)",
            value_type: ValueType::Presence,
            default: "",
            precedence: "env only",
            scope: Scope::System,
            sensitivity: Sensitivity::Public,
            stability: Stability::System,
            replacement: None,
        },
        EnvVarSpec {
            name: "SHELL",
            owner: "System",
            purpose: "Shell for terminal sessions",
            value_type: ValueType::Path,
            default: "/bin/zsh",
            precedence: "env > /bin/zsh",
            scope: Scope::System,
            sensitivity: Sensitivity::Public,
            stability: Stability::System,
            replacement: None,
        },
        EnvVarSpec {
            name: "XDG_CONFIG_HOME",
            owner: "System",
            purpose: "XDG config directory (legacy config path fallback)",
            value_type: ValueType::Path,
            default: "~/.config/",
            precedence: "env > ~/.config/",
            scope: Scope::System,
            sensitivity: Sensitivity::Public,
            stability: Stability::System,
            replacement: None,
        },
        EnvVarSpec {
            name: "HF_HUB_CACHE",
            owner: "System",
            purpose: "HuggingFace Hub cache directory (tokenizer lookup)",
            value_type: ValueType::Path,
            default: "~/.cache/huggingface/hub",
            precedence: "HF_HUB_CACHE > HUGGINGFACE_HUB_CACHE > HF_HOME/hub > default",
            scope: Scope::System,
            sensitivity: Sensitivity::Public,
            stability: Stability::System,
            replacement: None,
        },
        EnvVarSpec {
            name: "HUGGINGFACE_HUB_CACHE",
            owner: "System",
            purpose: "Fallback HuggingFace cache directory",
            value_type: ValueType::Path,
            default: "",
            precedence: "HF_HUB_CACHE > HUGGINGFACE_HUB_CACHE > HF_HOME/hub > default",
            scope: Scope::System,
            sensitivity: Sensitivity::Public,
            stability: Stability::System,
            replacement: None,
        },
        EnvVarSpec {
            name: "HF_HOME",
            owner: "System",
            purpose: "HuggingFace home directory",
            value_type: ValueType::Path,
            default: "~/.cache/huggingface",
            precedence: "HF_HUB_CACHE > HUGGINGFACE_HUB_CACHE > HF_HOME/hub > default",
            scope: Scope::System,
            sensitivity: Sensitivity::Public,
            stability: Stability::System,
            replacement: None,
        },
    ]
}

fn demo_only() -> Vec<EnvVarSpec> {
    vec![
        EnvVarSpec {
            name: "ANTHROPIC_MODEL",
            owner: "Demo",
            purpose: "Override Anthropic model name in demo scenarios",
            value_type: ValueType::String,
            default: "claude-sonnet-4-20250514",
            precedence: "env > default",
            scope: Scope::Agent,
            sensitivity: Sensitivity::Public,
            stability: Stability::DemoOnly,
            replacement: None,
        },
        EnvVarSpec {
            name: "OLLAMA_MODEL",
            owner: "Demo",
            purpose: "Override Ollama model name in demo scenarios",
            value_type: ValueType::String,
            default: "gemma3:7b",
            precedence: "env > default",
            scope: Scope::Agent,
            sensitivity: Sensitivity::Public,
            stability: Stability::DemoOnly,
            replacement: None,
        },
        EnvVarSpec {
            name: "OLLAMA_URL",
            owner: "Demo",
            purpose: "Override Ollama URL in demo scenarios",
            value_type: ValueType::Url,
            default: "http://localhost:11434",
            precedence: "env > default",
            scope: Scope::Agent,
            sensitivity: Sensitivity::Public,
            stability: Stability::DemoOnly,
            replacement: None,
        },
    ]
}

fn test_only() -> Vec<EnvVarSpec> {
    vec![
        EnvVarSpec {
            name: "ROKO_TEST_REPO_ROOT",
            owner: "Test",
            purpose: "Worktree lock integration test repo root",
            value_type: ValueType::Path,
            default: "",
            precedence: "test-only injection",
            scope: Scope::System,
            sensitivity: Sensitivity::Public,
            stability: Stability::TestOnly,
            replacement: None,
        },
        EnvVarSpec {
            name: "ROKO_TEST_OLLAMA",
            owner: "Test",
            purpose: "Gate ollama integration tests",
            value_type: ValueType::Presence,
            default: "",
            precedence: "test-only injection",
            scope: Scope::System,
            sensitivity: Sensitivity::Public,
            stability: Stability::TestOnly,
            replacement: None,
        },
        EnvVarSpec {
            name: "ROKO_TEST_RPC_URL",
            owner: "Test",
            purpose: "Chain integration test RPC URL",
            value_type: ValueType::Url,
            default: "",
            precedence: "test-only injection",
            scope: Scope::System,
            sensitivity: Sensitivity::Public,
            stability: Stability::TestOnly,
            replacement: None,
        },
    ]
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn registry_has_no_duplicate_names() {
        let registry = env_registry();
        let mut seen = HashSet::new();
        for spec in &registry {
            assert!(
                seen.insert(spec.name),
                "duplicate env var name: {}",
                spec.name
            );
        }
    }

    #[test]
    fn registry_is_nonempty() {
        assert!(
            env_registry().len() >= 80,
            "expected at least 80 registered env vars"
        );
    }

    #[test]
    fn secret_vars_are_marked() {
        let registry = env_registry();
        let secret_names: Vec<&str> = registry
            .iter()
            .filter(|s| s.sensitivity == Sensitivity::Secret)
            .map(|s| s.name)
            .collect();
        // At minimum these must be secret.
        for required in &[
            "ANTHROPIC_API_KEY",
            "OPENAI_API_KEY",
            "GITHUB_TOKEN",
            "ROKO_API_KEY",
            "ROKO_SERVER_AUTH_TOKEN",
            "SLACK_SIGNING_SECRET",
            "ROKO_ATTEST_SIGNING_KEY_HEX",
        ] {
            assert!(
                secret_names.contains(required),
                "{required} must be marked as secret"
            );
        }
    }

    #[test]
    fn deprecated_vars_have_documentation() {
        let registry = env_registry();
        for spec in &registry {
            if spec.stability == Stability::Deprecated {
                // Deprecated vars should document what replaces them OR be
                // scheduled for removal (replacement = None is ok for ACP_LEGACY).
                assert!(
                    !spec.purpose.is_empty(),
                    "deprecated var {} needs a purpose",
                    spec.name
                );
            }
        }
    }

    #[test]
    fn all_specs_have_owner_and_purpose() {
        for spec in env_registry() {
            assert!(!spec.owner.is_empty(), "{} missing owner", spec.name);
            assert!(!spec.purpose.is_empty(), "{} missing purpose", spec.name);
        }
    }

    #[test]
    fn print_env_list_does_not_panic() {
        // Just verify the formatting path doesn't panic.
        print_env_list(false);
        print_env_list(true);
    }

    #[test]
    fn json_output_is_valid() {
        let registry = env_registry();
        let json = serde_json::to_string_pretty(&registry).expect("serialize to JSON");
        let parsed: Vec<serde_json::Value> =
            serde_json::from_str(&json).expect("parse JSON back");
        assert_eq!(parsed.len(), registry.len());
    }

    #[test]
    fn bool_env_consistency() {
        // Verify that parse_bool_env in schema.rs and the env_flag_enabled pattern
        // both accept the same canonical vocabulary.
        use crate::config::schema::parse_bool_env;
        for truthy in &["1", "true", "yes", "on", "TRUE", "Yes", "ON"] {
            assert!(parse_bool_env(truthy), "{truthy} should be truthy");
        }
        for falsy in &["0", "false", "no", "off", "FALSE", "No", "OFF", ""] {
            assert!(!parse_bool_env(falsy), "{falsy} should be falsy");
        }
    }

    #[test]
    fn precedence_documented_for_stable_vars() {
        let registry = env_registry();
        for spec in &registry {
            if spec.stability == Stability::Stable {
                assert!(
                    !spec.precedence.is_empty(),
                    "stable var {} must document precedence",
                    spec.name
                );
            }
        }
    }
}
