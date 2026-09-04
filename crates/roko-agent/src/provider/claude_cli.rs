pub mod stream;

pub use stream::{
    ClaudeAssistantEvent, ClaudeContentBlock, ClaudeMessage, ClaudeResultEvent, ClaudeStreamEvent,
    ClaudeSystemEvent, ClaudeToolEvent, ClaudeUsage, parse_stream_line,
};

use crate::Agent;
use crate::ExecAgent;
use crate::claude_cli_agent::{ClaudeCliAgent, build_settings_json};
use crate::provider::current_safety_layer;
use crate::provider::{
    AgentCreationError, AgentOptions, ProviderAdapter, ProviderError, configured_resource_limits,
};
use crate::safety::SafetyLayer;
use roko_core::agent::ProviderKind;
#[cfg(test)]
use roko_core::config::DEFAULT_TTFT_TIMEOUT_MS;
use roko_core::config::schema::{ModelProfile, ProviderConfig};
use roko_core::tool::aliases::{canonical_names, claude_of_canonical};
use serde_json::Value;
use std::path::PathBuf;

/// Adapter for the `claude` CLI subprocess protocol.
pub struct ClaudeCliAdapter;

impl ProviderAdapter for ClaudeCliAdapter {
    fn kind(&self) -> ProviderKind {
        ProviderKind::ClaudeCli
    }

    fn create_agent(
        &self,
        provider: &ProviderConfig,
        model: &ModelProfile,
        options: &AgentOptions,
    ) -> Result<Box<dyn Agent>, AgentCreationError> {
        if provider.kind != self.kind() {
            return Err(AgentCreationError::InvalidKind(provider.kind));
        }

        let command = provider
            .command
            .as_deref()
            .map(str::trim)
            .filter(|command| !command.is_empty())
            .ok_or_else(|| AgentCreationError::MissingConfig("providers.*.command".to_string()))?;

        // Verify the binary exists on PATH before attempting to spawn.
        if !crate::provider::pre_flight::binary_on_path(command) {
            return Err(AgentCreationError::BinaryNotFound(command.to_string()));
        }

        let current_dir = options
            .working_dir
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        let timeout_ms = options.effective_timeout_ms(provider.timeout_ms);

        let mut agent = ClaudeCliAgent::new(command, current_dir, model.slug.clone())
            .with_timeout_ms(timeout_ms)
            .with_settings_json(build_settings_json())
            .with_bare_mode(options.bare_mode)
            .with_dangerously_skip_permissions(options.dangerously_skip_permissions);

        if let Some(limits) = configured_resource_limits(provider)? {
            agent = agent.with_resource_limits(limits);
        }

        if let Some(args) = &provider.args {
            agent = agent.with_extra_args(args.clone());
        }
        if let Some(prompt) = &options.system_prompt {
            agent = agent.with_system_prompt(prompt.clone());
        }
        if let Some(allowed) = options
            .agent_contract
            .as_ref()
            .and_then(|contract| contract.allowed_tools.as_ref())
        {
            agent = agent.with_tools(render_claude_tool_policy(allowed));
        } else if let Some(tools) = &options.tools {
            agent = agent.with_tools(tools.clone());
        }
        if let Some(contract) = &options.agent_contract {
            let denied = render_claude_tool_policy(&contract.forbidden_tool_names());
            if !denied.is_empty() {
                agent = agent.with_disallowed_tools(denied);
            }
        }
        if let Some(mcp_config) = &options.mcp_config {
            agent = agent.with_mcp_config(mcp_config.clone());
        }
        if let Some(effort) = &options.effort {
            agent = agent.with_effort(effort.clone());
        }
        if !options.name.is_empty() {
            agent = agent.with_name(options.name.clone());
        }
        if !options.extra_args.is_empty() {
            agent = agent.with_extra_args(options.extra_args.clone());
        }
        for (key, value) in &options.env {
            agent = agent.with_env_var(key.clone(), value.clone());
        }

        Ok(Box::new(agent))
    }

    fn classify_error(&self, status: u16, body: &Value) -> ProviderError {
        super::error_classify::classify_cli_error(status, body, "CLI")
    }
}

/// Adapter for the `codex` CLI subprocess protocol (`codex exec --json`).
///
/// Previously, Codex CLI piggy-backed on [`ClaudeCliAdapter`] with
/// executable-name sniffing. This adapter gives `CodexCli` its own
/// first-class dispatch path so routing and capability logic can
/// distinguish the two protocols at the type level.
pub struct CodexCliAdapter;

impl ProviderAdapter for CodexCliAdapter {
    fn kind(&self) -> ProviderKind {
        ProviderKind::CodexCli
    }

    fn create_agent(
        &self,
        provider: &ProviderConfig,
        model: &ModelProfile,
        options: &AgentOptions,
    ) -> Result<Box<dyn Agent>, AgentCreationError> {
        if provider.kind != self.kind() {
            return Err(AgentCreationError::InvalidKind(provider.kind));
        }

        let command = provider
            .command
            .as_deref()
            .map(str::trim)
            .filter(|command| !command.is_empty())
            .unwrap_or("codex");

        // Verify the binary exists on PATH before attempting to spawn.
        if !crate::provider::pre_flight::binary_on_path(command) {
            return Err(AgentCreationError::BinaryNotFound(command.to_string()));
        }

        let current_dir = options
            .working_dir
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        let timeout_ms = options.effective_timeout_ms(provider.timeout_ms);

        let mut args = vec![
            "exec".to_string(),
            "--json".to_string(),
            "--cd".to_string(),
            current_dir.to_string_lossy().to_string(),
            "--skip-git-repo-check".to_string(),
            "--color".to_string(),
            "never".to_string(),
        ];

        if options.dangerously_skip_permissions {
            args.push("--dangerously-bypass-approvals-and-sandbox".to_string());
        } else {
            args.push("--sandbox".to_string());
            args.push("workspace-write".to_string());
        }

        // Only pass --model for non-Claude models (codex defaults to its own)
        if !model.slug.is_empty() && !model.slug.starts_with("claude") {
            args.push("--model".to_string());
            args.push(model.slug.clone());
        }

        args.push("-".to_string()); // Read prompt from stdin

        let safety = current_safety_layer().unwrap_or_else(SafetyLayer::with_defaults);

        let mut agent = ExecAgent::new(command, args, safety)
            .with_timeout_ms(timeout_ms)
            .with_current_dir(&current_dir)
            .with_extract_codex_jsonl(true);

        // Codex lacks --system-prompt; fold it into stdin prefix.
        if let Some(system_prompt) = &options.system_prompt {
            agent = agent.with_stdin_prefix(system_prompt.clone());
        }

        if !options.name.is_empty() {
            agent = agent.with_name(options.name.clone());
        } else {
            agent = agent.with_name(format!("codex-cli:{}", model.slug));
        }
        for (key, value) in &options.env {
            agent = agent.with_env_var(key.clone(), value.clone());
        }

        tracing::info!(
            command = command,
            model = %model.slug,
            "creating Codex CLI agent via ExecAgent"
        );

        Ok(Box::new(agent))
    }

    fn classify_error(&self, status: u16, body: &Value) -> ProviderError {
        // Codex CLI errors look similar to Claude CLI errors (stderr text).
        // Reuse the same classification logic.
        ClaudeCliAdapter.classify_error(status, body)
    }
}

fn render_claude_tool_policy(tools: &[String]) -> String {
    tools
        .iter()
        .filter_map(|name| {
            if let Some(alias) = claude_of_canonical(name) {
                Some(alias.to_string())
            } else if canonical_names().any(|canonical| canonical == name) {
                // Roko-only canonical tools cannot be executed by Claude CLI.
                None
            } else {
                // Preserve MCP/plugin names and already-native Claude names.
                Some(name.clone())
            }
        })
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(test)]
mod tests {
    use super::*;
    use roko_core::{Body, Context, Kind, Signal};
    use std::fs;
    use tempfile::tempdir;

    fn prompt(text: &str) -> Signal {
        Signal::builder(Kind::Prompt).body(Body::text(text)).build()
    }

    fn write_script(path: &std::path::Path, body: &str) {
        fs::write(path, body).expect("write script");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(path).expect("script metadata").permissions();
            perms.set_mode(0o755);
            fs::set_permissions(path, perms).expect("chmod script");
        }
    }

    fn claude_model() -> ModelProfile {
        ModelProfile {
            provider: "claude_cli".to_string(),
            slug: "claude-sonnet-4-6".to_string(),
            context_window: 200_000,
            max_output: Some(8_192),
            supports_tools: true,
            supports_thinking: false,
            supports_vision: false,
            supports_web_search: false,
            supports_mcp_tools: false,
            supports_partial: false,
            supports_grounding: false,
            supports_code_execution: false,
            supports_caching: false,
            provider_routing: None,
            tool_format: "anthropic_blocks".to_string(),
            cost_input_per_m: None,
            cost_output_per_m: None,
            cost_input_per_m_high: None,
            cost_output_per_m_high: None,
            cost_cache_read_per_m: None,
            cost_cache_write_per_m: None,
            thinking_level: None,
            max_tools: None,
            tokenizer_ratio: None,
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn claude_cli_adapter_creates_agent_with_all_options_applied() {
        let tmp = tempdir().expect("tempdir");
        let script = tmp.path().join("claude-fake.sh");
        let args_file = tmp.path().join("args.txt");
        let prompt_file = tmp.path().join("prompt.txt");
        let env_file = tmp.path().join("env.txt");
        let mcp_config = tmp.path().join("mcp.json");
        fs::write(&mcp_config, "{}").expect("write mcp config");
        let mcp_config_arg = mcp_config.clone();

        let script_body = format!(
            r#"#!/bin/sh
set -eu
args_file="{args_file}"
prompt_file="{prompt_file}"
env_file="{env_file}"
printf '%s\n' "$@" > "$args_file"
printf '%s\n' "${{CLAUDE_TEST_ENV-}}" > "$env_file"
cat > "$prompt_file"
printf '%s\n' '{{"type":"content_block_delta","delta":{{"text":"adapter-ok"}}}}'
"#,
            args_file = args_file.display(),
            prompt_file = prompt_file.display(),
            env_file = env_file.display(),
        );
        write_script(&script, &script_body);

        let provider = ProviderConfig {
            kind: ProviderKind::ClaudeCli,
            base_url: None,
            api_key_env: None,
            command: Some(script.display().to_string()),
            args: Some(vec![
                "--provider-flag".to_string(),
                "provider-value".to_string(),
            ]),
            timeout_ms: Some(2_500),
            ttft_timeout_ms: Some(DEFAULT_TTFT_TIMEOUT_MS),
            connect_timeout_ms: Some(5_000),
            extra_headers: None,
            max_concurrent: None,
            limits: None,
            require_confirmation: false,
        };
        let options = AgentOptions {
            safety_layer: None,
            temperament: None,
            command: None,
            timeout_ms: Some(5_000),
            system_prompt: Some("system guidance".to_string()),
            input_messages: Vec::new(),
            cached_content: None,
            tools: Some("Read,Edit".to_string()),
            agent_contract: Some(crate::safety::contract::AgentContract {
                role: "auditor".to_string(),
                allowed_tools: Some(vec![
                    "read_file".to_string(),
                    "grep".to_string(),
                    "apply_patch".to_string(),
                ]),
                governance: vec![crate::safety::contract::GovernanceRule::ForbiddenTools(
                    vec!["bash".to_string(), "web_search".to_string()],
                )],
                ..crate::safety::contract::AgentContract::default()
            }),
            mcp_config: Some(mcp_config_arg),
            immune_root: None,
            working_dir: None,
            provider_semaphores: None,
            env: vec![("CLAUDE_TEST_ENV".to_string(), "env-value".to_string())],
            extra_args: vec!["--option-flag".to_string(), "option-value".to_string()],
            effort: Some("high".to_string()),
            bare_mode: false,
            dangerously_skip_permissions: false,
            name: "claude-cli-adapter".to_string(),
            pre_discovered_mcp_tools: None,
            pre_discovered_mcp_runtime: None,
            pre_discovered_local_tools: None,
            local_tool_mcp_servers: None,
            rate_limiter: None,
            gemini_safety_settings: Vec::new(),
            cancel_token: None,
            tool_audit: None,
        };
        let model = claude_model();

        let adapter = ClaudeCliAdapter;
        assert_eq!(adapter.kind(), ProviderKind::ClaudeCli);

        let agent = adapter
            .create_agent(&provider, &model, &options)
            .expect("create agent");
        assert_eq!(agent.name(), "claude-cli-adapter");

        let result = agent.run(&prompt("hello"), &Context::now()).await;
        assert!(
            result.success,
            "{}",
            result.output.body.as_text().unwrap_or("unknown")
        );
        assert_eq!(result.output.body.as_text().unwrap_or(""), "adapter-ok");

        let args_text = fs::read_to_string(&args_file).expect("read args");
        assert!(args_text.contains("--provider-flag"));
        assert!(args_text.contains("provider-value"));
        assert!(args_text.contains("--option-flag"));
        assert!(args_text.contains("option-value"));
        assert!(args_text.contains("--model"));
        assert!(args_text.contains("claude-sonnet-4-6"));
        assert!(args_text.contains("--effort"));
        assert!(args_text.contains("high"));
        assert!(args_text.contains("--settings"));
        assert!(args_text.contains("--append-system-prompt"));
        assert!(args_text.contains("system guidance"));
        assert!(args_text.contains("--tools"));
        assert!(args_text.contains("Read,Grep"));
        assert!(!args_text.contains("Read,Edit"));
        assert!(args_text.contains("--disallowed-tools"));
        assert!(args_text.contains("Bash,WebSearch"));
        assert!(args_text.contains("--mcp-config"));
        assert!(args_text.contains(mcp_config.to_str().expect("mcp path")));
        assert!(args_text.contains("--strict-mcp-config"));
        assert!(!args_text.contains("--bare"));
        assert!(!args_text.contains("--dangerously-skip-permissions"));

        let provider_pos = args_text.find("--provider-flag").expect("provider args");
        let option_pos = args_text.find("--option-flag").expect("option args");
        assert!(provider_pos < option_pos);

        let prompt_text = fs::read_to_string(&prompt_file).expect("read prompt");
        assert_eq!(prompt_text, "hello");
        let env_text = fs::read_to_string(&env_file).expect("read env");
        assert_eq!(env_text.trim(), "env-value");
    }

    #[tokio::test]
    async fn claude_cli_adapter_uses_explicit_working_dir() {
        let tmp = tempdir().expect("tempdir");
        let worktree = tmp.path().join("worktree");
        fs::create_dir(&worktree).expect("create worktree");
        let args_file = tmp.path().join("args.txt");
        let cwd_file = tmp.path().join("cwd.txt");
        let script = tmp.path().join("claude-fake.sh");
        let script_body = format!(
            r#"#!/bin/sh
set -eu
args_file="{args_file}"
cwd_file="{cwd_file}"
printf '%s\n' "$@" > "$args_file"
pwd > "$cwd_file"
cat >/dev/null
printf '%s\n' '{{"type":"content_block_delta","delta":{{"text":"worktree-ok"}}}}'
"#,
            args_file = args_file.display(),
            cwd_file = cwd_file.display(),
        );
        write_script(&script, &script_body);

        let provider = ProviderConfig {
            kind: ProviderKind::ClaudeCli,
            base_url: None,
            api_key_env: None,
            command: Some(script.display().to_string()),
            args: None,
            timeout_ms: Some(1_000),
            ttft_timeout_ms: Some(DEFAULT_TTFT_TIMEOUT_MS),
            connect_timeout_ms: Some(5_000),
            extra_headers: None,
            max_concurrent: None,
            limits: None,
            require_confirmation: false,
        };
        let options = AgentOptions {
            timeout_ms: Some(10_000),
            working_dir: Some(worktree.clone()),
            name: "claude-cli-worktree".to_string(),
            ..Default::default()
        };
        let model = claude_model();

        let adapter = ClaudeCliAdapter;
        let agent = adapter
            .create_agent(&provider, &model, &options)
            .expect("create agent");

        let result = agent.run(&prompt("x"), &Context::now()).await;
        assert!(
            result.success,
            "{}",
            result.output.body.as_text().unwrap_or("unknown")
        );
        assert_eq!(result.output.body.as_text().unwrap_or(""), "worktree-ok");

        let cwd_text = fs::read_to_string(&cwd_file).expect("read cwd");
        let observed_cwd = fs::canonicalize(cwd_text.trim()).expect("canonicalize cwd");
        let expected_cwd = fs::canonicalize(&worktree).expect("canonicalize worktree");
        assert_eq!(observed_cwd, expected_cwd);

        let args_text = fs::read_to_string(&args_file).expect("read args");
        assert!(args_text.contains("--model"));
        assert!(args_text.contains("claude-sonnet-4-6"));
    }

    #[tokio::test]
    async fn claude_cli_adapter_timeout_comes_from_agent_options() {
        let tmp = tempdir().expect("tempdir");
        let script = tmp.path().join("claude-fake.sh");
        let script_body = r#"#!/bin/sh
set -eu
sleep 1
printf '%s\n' '{"type":"content_block_delta","delta":{"text":"late"}}'
"#;
        write_script(&script, script_body);

        let provider = ProviderConfig {
            kind: ProviderKind::ClaudeCli,
            base_url: None,
            api_key_env: None,
            command: Some(script.display().to_string()),
            args: None,
            timeout_ms: Some(1_000),
            ttft_timeout_ms: Some(DEFAULT_TTFT_TIMEOUT_MS),
            connect_timeout_ms: Some(5_000),
            extra_headers: None,
            max_concurrent: None,
            limits: None,
            require_confirmation: false,
        };
        let options = AgentOptions {
            timeout_ms: Some(100),
            name: "claude-cli-timeout".to_string(),
            ..Default::default()
        };
        let model = claude_model();

        let adapter = ClaudeCliAdapter;
        let agent = adapter
            .create_agent(&provider, &model, &options)
            .expect("create agent");

        let result = agent.run(&prompt("slow"), &Context::now()).await;
        assert!(!result.success);
        assert!(
            result
                .output
                .body
                .as_text()
                .unwrap_or("")
                .contains("timed out after 100 ms"),
            "{}",
            result.output.body.as_text().unwrap_or("unknown")
        );
    }
}
