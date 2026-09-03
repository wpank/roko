//! `roko init` template rendering.
#![allow(dead_code)]

use anyhow::{Context, Result};
use std::ffi::OsStr;

use roko_cli::config::command_on_path;
use roko_core::config::schema::RokoConfig;
use roko_core::config::GateRungConfig;

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

fn append_no_profile_gate_hint(out: &mut String) {
    out.push_str(
        "\n# No default gate rungs were written because no supported project profile was supplied.\n",
    );
    out.push_str("# Supported profiles: rust, typescript.\n");
    out.push_str("# Add [[gates.rungs]] entries under the [gates] table for custom validation commands.\n");
    out.push_str(
        "# Or rerun `roko init --profile rust` / `roko init --profile typescript`.\n",
    );
}
