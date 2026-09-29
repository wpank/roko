//! `roko init` template rendering.

use anyhow::{Context, Result};
use std::ffi::OsStr;
use std::path::Path;

use roko_cli::config::command_on_path;
use roko_core::config::GateRungConfig;
use roko_core::config::schema::RokoConfig;

/// Environment variable that holds the Anthropic API key.
const ANTHROPIC_API_KEY_ENV: &str = "ANTHROPIC_API_KEY";

/// The provider that `roko init` points the default model at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InitProvider {
    /// `claude` is on `PATH`, so the default model runs through Claude CLI.
    ClaudeCli,
    /// `claude` is missing but `ANTHROPIC_API_KEY` is set, so the default
    /// model calls the Anthropic API.
    AnthropicApi,
    /// Neither is available. The provider and model blocks are written
    /// commented out, because a model entry must name a configured provider.
    Unconfigured,
}

impl InitProvider {
    /// Pick the provider from what is installed and exported now.
    #[must_use]
    pub fn detect() -> Self {
        if command_on_path("claude") {
            Self::ClaudeCli
        } else if std::env::var(ANTHROPIC_API_KEY_ENV).is_ok_and(|key| !key.trim().is_empty()) {
            Self::AnthropicApi
        } else {
            Self::Unconfigured
        }
    }

    /// What `roko init` prints about the provider it configured.
    #[must_use]
    pub const fn summary(self) -> &'static str {
        match self {
            Self::ClaudeCli => {
                "default provider command set to \"claude\". \
                 Edit roko.toml [providers.claude_cli] to use a different command."
            }
            Self::AnthropicApi => {
                "claude was not found on PATH, so the default model uses the Anthropic API \
                 (ANTHROPIC_API_KEY). Edit roko.toml [providers.anthropic] to change it."
            }
            Self::Unconfigured => {
                "no provider configured: claude is not on PATH and ANTHROPIC_API_KEY is not set. \
                 The comments at the end of roko.toml say how to add one."
            }
        }
    }
}

/// Whether `workdir` still needs `roko init`, that is, has no `roko.toml`.
///
/// `.roko/` cannot decide this: roko's own log creates it before any
/// command runs, so even an empty directory has one.
#[must_use]
pub fn needs_init(workdir: &Path) -> bool {
    !workdir.join("roko.toml").is_file()
}

/// Write the default `roko.toml` into `target`, with the default model on
/// `provider`, and return the text written.
///
/// The text must pass the offline checks of `roko config validate`
/// (`roko_cli::config_cmd::write_checked_config`) before it is written, so
/// `roko init` never leaves a config that commands cannot load.
pub fn write_init_config(target: &Path, cloud: bool, provider: InitProvider) -> Result<String> {
    let text = render_init_template_for(cloud, provider)?;
    roko_cli::config_cmd::write_checked_config(&target.join("roko.toml"), &text)?;
    Ok(text)
}

/// Render the default `roko.toml` template used by `roko init`.
///
/// The base document comes from the v2 schema serializer so the generated
/// workspace starts in the provider/model world rather than the legacy
/// v1 `[agent]` command world.
pub(crate) fn render_init_template(cloud: bool) -> Result<String> {
    render_init_template_for(cloud, InitProvider::detect())
}

/// Render the default `roko.toml` template with the default model on `provider`.
pub fn render_init_template_for(cloud: bool, provider: InitProvider) -> Result<String> {
    let profile = detect_init_profile().map(|profile| profile.trim().to_ascii_lowercase());

    let mut config = RokoConfig::default();
    config.agent.default_backend = "claude".to_string();
    config.agent.default_model = "claude-sonnet-4-6".to_string();
    if cloud {
        config.server.bind = "0.0.0.0".to_string();
    }
    config.gates.custom_rungs = profile_gate_rungs(profile.as_deref());

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

    // Every `[models.*]` entry must name a configured provider (config
    // invariant 3), so the model block is only live when its provider is.
    match provider {
        InitProvider::ClaudeCli => {
            push_claude_cli_provider(&mut out, "");
            push_default_model(&mut out, "claude_cli", "");
        }
        InitProvider::AnthropicApi => {
            out.push_str("\n# Claude CLI was not found on PATH when this workspace was\n");
            out.push_str("# initialized, so the default model uses the Anthropic API through\n");
            out.push_str("# ANTHROPIC_API_KEY. To use Claude CLI instead, install it,\n");
            out.push_str("# uncomment the provider block below and set\n");
            out.push_str("# provider = \"claude_cli\" in [models.claude-sonnet-4-6].\n");
            push_claude_cli_provider(&mut out, "# ");
            out.push_str("\n[providers.anthropic]\n");
            out.push_str("kind = \"anthropic_api\"\n");
            out.push_str("base_url = \"https://api.anthropic.com\"\n");
            out.push_str("api_key_env = \"ANTHROPIC_API_KEY\"\n");
            push_default_model(&mut out, "anthropic", "");
        }
        InitProvider::Unconfigured => {
            out.push_str("\n# Claude CLI was not found on PATH and ANTHROPIC_API_KEY was not\n");
            out.push_str("# set when this workspace was initialized, so no provider is\n");
            out.push_str("# configured. Either export ANTHROPIC_API_KEY (the built-in\n");
            out.push_str("# claude-sonnet-4-6 model then uses the Anthropic API), or install\n");
            out.push_str("# Claude CLI and uncomment both blocks below.\n");
            push_claude_cli_provider(&mut out, "# ");
            push_default_model(&mut out, "claude_cli", "# ");
        }
    }

    if profile.is_none() {
        append_no_profile_gate_hint(&mut out);
    }

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
            return args
                .next()
                .map(|value| value.to_string_lossy().into_owned());
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

/// Build custom gate rungs for a detected project profile.
///
/// These are set on `config.gates.custom_rungs` before serialization so the
/// `[gates]` table and its `[[gates.rungs]]` entries appear as one canonical
/// block in the generated `roko.toml`.
fn profile_gate_rungs(profile: Option<&str>) -> Vec<GateRungConfig> {
    match profile {
        Some("rust") => vec![
            GateRungConfig {
                name: "compile".to_string(),
                command: "cargo check --workspace".to_string(),
                timeout_secs: 120,
                required: true,
                parallel_with: Vec::new(),
            },
            GateRungConfig {
                name: "test".to_string(),
                command: "cargo test --workspace".to_string(),
                timeout_secs: 300,
                required: true,
                parallel_with: Vec::new(),
            },
            GateRungConfig {
                name: "lint".to_string(),
                command: "cargo clippy --workspace --no-deps -- -D warnings".to_string(),
                timeout_secs: 120,
                required: true,
                parallel_with: Vec::new(),
            },
        ],
        Some("typescript") => vec![
            GateRungConfig {
                name: "compile".to_string(),
                command: "npx tsc --noEmit".to_string(),
                timeout_secs: 120,
                required: true,
                parallel_with: Vec::new(),
            },
            GateRungConfig {
                name: "test".to_string(),
                command: "npm test".to_string(),
                timeout_secs: 300,
                required: true,
                parallel_with: Vec::new(),
            },
        ],
        _ => Vec::new(),
    }
}

/// Append the Claude CLI provider block, each line prefixed with `prefix`
/// (`"# "` writes it commented out).
fn push_claude_cli_provider(out: &mut String, prefix: &str) {
    out.push('\n');
    for line in [
        "[providers.claude_cli]",
        "kind = \"claude_cli\"",
        "command = \"claude\"",
    ] {
        out.push_str(prefix);
        out.push_str(line);
        out.push('\n');
    }
}

/// Append the default model block on `provider`, each line prefixed with
/// `prefix` (`"# "` writes it commented out).
fn push_default_model(out: &mut String, provider: &str, prefix: &str) {
    let provider_line = format!("provider = \"{provider}\"");
    out.push('\n');
    for line in [
        "[models.claude-sonnet-4-6]",
        provider_line.as_str(),
        "slug = \"claude-sonnet-4-6\"",
        "context_window = 200000",
        "tool_format = \"anthropic_blocks\"",
        "max_tools = 32",
    ] {
        out.push_str(prefix);
        out.push_str(line);
        out.push('\n');
    }
}

fn append_no_profile_gate_hint(out: &mut String) {
    out.push_str(
        "\n# No default gate rungs were written because no supported project profile was supplied.\n",
    );
    out.push_str("# Supported profiles: rust, typescript.\n");
    out.push_str(
        "# Add [[gates.rungs]] entries under the [gates] table for custom validation commands.\n",
    );
    out.push_str("# Or rerun `roko init --profile rust` / `roko init --profile typescript`.\n");
}
