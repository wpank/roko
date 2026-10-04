pub mod stream;

pub use stream::{
    ClaudeAssistantEvent, ClaudeContentBlock, ClaudeMessage, ClaudeResultEvent, ClaudeStreamEvent,
    ClaudeSystemEvent, ClaudeToolEvent, ClaudeUsage, parse_stream_line,
};

use crate::Agent;
use crate::ExecAgent;
use crate::claude_cli_agent::{ClaudeCliAgent, build_settings_json};
use crate::exec::CodexOperationPolicy;
use crate::provider::current_safety_layer;
use crate::provider::{
    AgentCreationError, AgentOptions, ProviderAdapter, ProviderError, TurnCapEnforcement,
    configured_resource_limits, provider_credential_scrub,
};
use crate::safety::contract::AgentContract;
use crate::safety::{SafetyLayer, SandboxLevel};
use roko_core::agent::ProviderKind;
#[cfg(test)]
use roko_core::config::DEFAULT_TTFT_TIMEOUT_MS;
use roko_core::config::schema::{ModelProfile, ProviderConfig};
use roko_core::tool::aliases::{canonical_names, claude_of_canonical};
use roko_std::roles::CHAIN_TOOL_PREFIX;
use serde_json::Value;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

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
            .with_dangerously_skip_permissions(options.dangerously_skip_permissions)
            .with_credential_scrub(provider_credential_scrub(provider, options))
            .with_pricing(options.pricing.clone());

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
        if let Some(max_turns) = options.max_turns {
            agent = agent.with_max_turns(max_turns);
        }
        for (key, value) in &options.env {
            agent = agent.with_env_var(key.clone(), value.clone());
        }
        if let Some(provider_semaphores) = options.provider_semaphores.clone() {
            agent = agent.with_provider_semaphores(model.provider.clone(), provider_semaphores);
        }
        if let Some(live_output) = options.live_output.clone() {
            agent = agent.with_live_output(live_output);
        }

        Ok(Box::new(agent))
    }

    fn classify_error(&self, status: u16, body: &Value) -> ProviderError {
        super::error_classify::classify_cli_error(status, body, "CLI")
    }

    fn turn_cap_enforcement(&self, _provider: &ProviderConfig) -> TurnCapEnforcement {
        TurnCapEnforcement::Native
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
        // Codex's built-in tools have no binding allowlist: it reads, searches
        // and edits through its shell, so a contract naming the only tools a
        // role may use cannot be honoured. Refuse it, so that failover picks a
        // provider that can (gap-baab0a).
        if options
            .agent_contract
            .as_ref()
            .is_some_and(|contract| contract.allowed_tools.is_some())
        {
            return Err(AgentCreationError::ToolAllowlistUnsupported(self.kind()));
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

        let safety = options
            .safety_layer
            .clone()
            .or_else(current_safety_layer)
            .unwrap_or_else(|| {
                // CodexCliAdapter requires an explicit SafetyLayer at agent construction time.
                // No scoped layer was found in AgentOptions or the thread-local — this
                // usually means the adapter is being called outside a `with_safety_layer`
                // scope. The conservative `with_defaults` posture is applied. Production
                // callers should pass a role-specific layer via `AgentOptions::safety_layer`.
                tracing::warn!(
                    command = command,
                    "CodexCliAdapter: no safety layer in options or scope; \
                 applying SafetyLayer::with_defaults — attach a role-scoped \
                 layer via AgentOptions::safety_layer for explicit enforcement"
                );
                SafetyLayer::with_defaults()
            });

        let mut args = vec![
            "exec".to_string(),
            "--json".to_string(),
            "--cd".to_string(),
            current_dir.to_string_lossy().to_string(),
            "--skip-git-repo-check".to_string(),
            "--color".to_string(),
            "never".to_string(),
        ];

        // Codex keeps its own OS sandbox unless the sandbox level lets
        // skip-permissions switch it off; builds may also write Cargo's
        // directories (1213).
        let inherited = |name: &str| {
            options
                .env
                .iter()
                .rev()
                .find(|(key, _)| key == name)
                .map(|(_, value)| OsString::from(value))
                .or_else(|| std::env::var_os(name))
        };
        args.extend(codex_sandbox_args(
            options.dangerously_skip_permissions,
            safety.sandbox_level,
            options.agent_contract.as_ref(),
            &codex_writable_roots(inherited, &current_dir),
        ));

        // Only pass --model for non-Claude models (codex defaults to its own)
        if !model.slug.is_empty() && !model.slug.starts_with("claude") {
            args.push("--model".to_string());
            args.push(model.slug.clone());
        }

        args.push("-".to_string()); // Read prompt from stdin

        // ── Operation policy broker (RG-2) ──────────────────────────────────
        // Derive a CodexOperationPolicy from the AgentContract so that Codex
        // built-in operations (command_execution, file_change, web_search,
        // mcp_tool_call) are screened against the configured deny/allow list,
        // and file changes against the worktree.  The broker reads the JSONL
        // output stream as Codex writes it and stops the process at the first
        // denied operation; a subprocess provider offers no earlier boundary.
        let operation_policy = options
            .agent_contract
            .as_ref()
            .map(|contract| {
                let policy = CodexOperationPolicy::from_contract(contract);
                tracing::debug!(
                    role = %contract.role,
                    has_constraints = policy.has_constraints(),
                    "CodexCliAdapter: derived operation policy from contract"
                );
                policy
            })
            .unwrap_or_else(|| {
                // No contract — apply a permissive policy so the broker is
                // present but does not block anything.  A future hardening pass
                // could make this deny-all for untrusted callers.
                tracing::debug!(
                    "CodexCliAdapter: no agent contract; using permissive operation policy"
                );
                CodexOperationPolicy::allow_all()
            });

        let mut agent = ExecAgent::new(command, args, safety)
            .with_timeout_ms(timeout_ms)
            .with_current_dir(&current_dir)
            .with_extract_codex_jsonl(true)
            .with_codex_operation_policy(operation_policy)
            .with_credential_scrub(provider_credential_scrub(provider, options));

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

    /// `codex exec` has no turn-count flag or setting, and roko does not count
    /// its turns while it runs, so nothing stops it at the cap:
    /// the cap is advisory, and only the attempt timeout bounds a long run.
    fn turn_cap_enforcement(&self, _provider: &ProviderConfig) -> TurnCapEnforcement {
        TurnCapEnforcement::Advisory
    }
}

/// The `codex exec` arguments that choose its sandbox, the directories it may
/// write besides its workspace, and its network access.
///
/// Codex keeps its OS sandbox (`--sandbox workspace-write`) unless the run
/// skips permissions and its sandbox level lets a CLI's own sandbox be
/// switched off ([`SandboxLevel::allows_sandbox_bypass`]: `None` or
/// `Observe`). `codex exec` asks for no approvals, so skipping permissions
/// never needed the sandbox off. In the sandbox, under `Isolate` and
/// `Quarantine` Codex writes only its workspace and has no network. Under the
/// other levels it may also write `writable_roots` ([`codex_writable_roots`]),
/// and its network follows the contract: off when the contract keeps the role
/// off the network ([`codex_network_pins`]), on when it lets the role reach
/// the network, and as the user's Codex configuration says without one.
///
/// Without the sandbox the network pins switch off only web search, so a run
/// whose contract keeps it off the network is logged as unconfined.
fn codex_sandbox_args(
    skip_permissions: bool,
    level: SandboxLevel,
    contract: Option<&AgentContract>,
    writable_roots: &[PathBuf],
) -> Vec<String> {
    let network_pins = codex_network_pins(contract);
    if skip_permissions && level.allows_sandbox_bypass() {
        if !network_pins.is_empty() {
            tracing::warn!(
                ?level,
                "Codex runs without its sandbox, so its network is not confined: the contract \
                 keeps the role off the network, but only web search is switched off"
            );
        }
        let mut args = vec!["--dangerously-bypass-approvals-and-sandbox".to_string()];
        args.extend(network_pins);
        return args;
    }

    let mut args = vec!["--sandbox".to_string(), "workspace-write".to_string()];
    if matches!(level, SandboxLevel::Isolate | SandboxLevel::Quarantine) {
        args.extend(CODEX_NETWORK_OFF.map(str::to_string));
        return args;
    }
    for root in writable_roots {
        args.push("--add-dir".to_string());
        args.push(root.to_string_lossy().into_owned());
    }
    if contract.is_some_and(AgentContract::permits_network) {
        args.push("-c".to_string());
        args.push("sandbox_workspace_write.network_access=true".to_string());
    }
    args.extend(network_pins);
    args
}

/// Directories outside its workspace that a sandboxed Codex run may also
/// write, so that its builds keep working: Cargo's target directory
/// (`CARGO_TARGET_DIR`) and Cargo's home (`CARGO_HOME`, else `~/.cargo`),
/// which holds the registry and git caches. `var` reads the environment Codex
/// inherits. A directory inside `workspace` is left out, since Codex may write
/// it anyway, and so is one that does not exist yet, which a sandbox may
/// refuse to grant.
///
/// They are passed as `--add-dir`, which adds to the user's
/// `sandbox_workspace_write.writable_roots` instead of replacing them, as a
/// `-c` override would. Checked against codex-cli 0.152.0.
fn codex_writable_roots(var: impl Fn(&str) -> Option<OsString>, workspace: &Path) -> Vec<PathBuf> {
    let target_dir = var("CARGO_TARGET_DIR").map(PathBuf::from);
    let cargo_home = var("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| var("HOME").map(|home| PathBuf::from(home).join(".cargo")));
    let mut roots: Vec<PathBuf> = Vec::new();
    for root in target_dir.into_iter().chain(cargo_home) {
        let root = if root.is_absolute() {
            root
        } else {
            workspace.join(root)
        };
        if root.is_dir() && !root.starts_with(workspace) && !roots.contains(&root) {
            roots.push(root);
        }
    }
    roots
}

/// `codex exec` overrides that switch off web search and the workspace
/// sandbox's network access. Checked against codex-cli 0.152.0, whose
/// `web_search` takes `disabled`, `cached`, `indexed` or `live`.
const CODEX_NETWORK_OFF: [&str; 4] = [
    "-c",
    "web_search=\"disabled\"",
    "-c",
    "sandbox_workspace_write.network_access=false",
];

/// [`CODEX_NETWORK_OFF`] for a run whose contract keeps the role off the
/// network; nothing for other runs, whose sandbox network
/// [`codex_sandbox_args`] decides. The network pin confines the network only
/// while Codex's sandbox is on.
fn codex_network_pins(contract: Option<&AgentContract>) -> Vec<String> {
    match contract {
        Some(contract) if !contract.permits_network() => {
            CODEX_NETWORK_OFF.map(str::to_string).into()
        }
        _ => Vec::new(),
    }
}

fn render_claude_tool_policy(tools: &[String]) -> String {
    tools
        .iter()
        .filter_map(|name| {
            if let Some(alias) = claude_of_canonical(name) {
                Some(alias.to_string())
            } else if canonical_names().any(|canonical| canonical == name)
                || name.starts_with(CHAIN_TOOL_PREFIX)
            {
                // Roko-only tools, the canonical builtins and the chain
                // tools, cannot be executed by Claude CLI.
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

    fn codex_provider(command: &str) -> ProviderConfig {
        ProviderConfig {
            kind: ProviderKind::CodexCli,
            base_url: None,
            api_key_env: None,
            command: Some(command.to_string()),
            args: None,
            timeout_ms: None,
            ttft_timeout_ms: None,
            connect_timeout_ms: None,
            extra_headers: None,
            max_concurrent: None,
            limits: None,
            require_confirmation: false,
            stream_usage: None,
            billing: None,
        }
    }

    /// gap-baab0a: Codex cannot honour a tool allowlist, so the adapter
    /// refuses a contract with one, before it looks for the binary. A role's
    /// forbidden tools alone are fine: the operation broker enforces them.
    #[test]
    fn codex_adapter_refuses_a_tool_allowlist() {
        use crate::safety::contract::{AgentContract, GovernanceRule};

        let model = ModelProfile {
            provider: "codex_cli".to_string(),
            slug: "gpt-5-codex".to_string(),
            ..ModelProfile::default()
        };
        let allowlist = AgentContract {
            allowed_tools: Some(vec!["read_file".to_string(), "grep".to_string()]),
            ..AgentContract::default()
        };
        let options = AgentOptions {
            agent_contract: Some(allowlist),
            ..Default::default()
        };
        let missing_binary = codex_provider("not-on-path");
        let refused = CodexCliAdapter.create_agent(&missing_binary, &model, &options);
        assert!(
            matches!(
                refused,
                Err(AgentCreationError::ToolAllowlistUnsupported(
                    ProviderKind::CodexCli
                ))
            ),
            "{:?}",
            refused.err()
        );

        let forbids_bash = AgentContract {
            governance: vec![GovernanceRule::ForbiddenTools(vec!["bash".to_string()])],
            ..AgentContract::default()
        };
        let options = AgentOptions {
            agent_contract: Some(forbids_bash),
            ..Default::default()
        };
        let created = CodexCliAdapter.create_agent(&codex_provider("sh"), &model, &options);
        assert!(created.is_ok(), "{:?}", created.err());
    }

    /// gap-baab0a: a Codex run whose contract keeps the role off the network
    /// gets web search and sandbox networking switched off; other runs keep
    /// the user's Codex configuration.
    #[test]
    fn codex_network_is_pinned_off_unless_the_contract_permits_it() {
        use crate::safety::contract::{AgentContract, GovernanceRule};

        let implementer = AgentContract {
            governance: vec![GovernanceRule::ForbiddenTools(vec![
                "web_fetch".into(),
                "web_search".into(),
            ])],
            ..AgentContract::default()
        };
        let expected = [
            "-c",
            "web_search=\"disabled\"",
            "-c",
            "sandbox_workspace_write.network_access=false",
        ];
        assert_eq!(codex_network_pins(Some(&implementer)), expected);
        assert!(codex_network_pins(Some(&AgentContract::default())).is_empty());
        assert!(codex_network_pins(None).is_empty());
    }

    /// 1213: skipping permissions switches Codex's own sandbox off only under
    /// `None` and `Observe`. Under `Restrict`, the default, Codex keeps
    /// `--sandbox workspace-write`, so the network pin of a role kept off the
    /// network now confines it: with the sandbox bypassed it only switched
    /// web search off.
    #[test]
    fn codex_keeps_workspace_sandbox_with_skip_permissions() {
        use crate::safety::contract::GovernanceRule;

        let bypass = "--dangerously-bypass-approvals-and-sandbox";
        let sandbox = ["--sandbox", "workspace-write"];
        let restrict = codex_sandbox_args(true, SandboxLevel::Restrict, None, &[]);
        assert_eq!(restrict, sandbox);
        assert!(!restrict.iter().any(|arg| arg == bypass));
        for level in [SandboxLevel::Isolate, SandboxLevel::Quarantine] {
            assert_eq!(codex_sandbox_args(true, level, None, &[])[..2], sandbox);
        }
        for level in [SandboxLevel::None, SandboxLevel::Observe] {
            assert_eq!(codex_sandbox_args(true, level, None, &[]), [bypass]);
            assert_eq!(codex_sandbox_args(false, level, None, &[]), sandbox);
        }

        // In the sandbox Cargo's directories stay writable, and the network
        // follows the contract.
        let implementer = AgentContract {
            governance: vec![GovernanceRule::ForbiddenTools(vec![
                "web_fetch".into(),
                "web_search".into(),
            ])],
            ..AgentContract::default()
        };
        let roots = [
            PathBuf::from("/shared/target"),
            PathBuf::from("/home/dev/.cargo"),
        ];
        assert_eq!(
            codex_sandbox_args(true, SandboxLevel::Restrict, Some(&implementer), &roots),
            [
                "--sandbox",
                "workspace-write",
                "--add-dir",
                "/shared/target",
                "--add-dir",
                "/home/dev/.cargo",
                "-c",
                "web_search=\"disabled\"",
                "-c",
                "sandbox_workspace_write.network_access=false",
            ]
        );
        let networked = AgentContract::default();
        assert_eq!(
            codex_sandbox_args(true, SandboxLevel::Restrict, Some(&networked), &[]),
            [
                "--sandbox",
                "workspace-write",
                "-c",
                "sandbox_workspace_write.network_access=true",
            ]
        );
        // Isolate keeps the run in its workspace and off the network.
        assert_eq!(
            codex_sandbox_args(true, SandboxLevel::Isolate, Some(&networked), &roots),
            [
                "--sandbox",
                "workspace-write",
                "-c",
                "web_search=\"disabled\"",
                "-c",
                "sandbox_workspace_write.network_access=false",
            ]
        );
        // Without the sandbox the pins still switch web search off.
        assert_eq!(
            codex_sandbox_args(true, SandboxLevel::None, Some(&implementer), &roots),
            [
                bypass,
                "-c",
                "web_search=\"disabled\"",
                "-c",
                "sandbox_workspace_write.network_access=false",
            ]
        );
    }

    /// The environment Codex would inherit, as `codex_writable_roots` reads it.
    fn env_of<P: AsRef<std::ffi::OsStr>>(vars: &[(&str, P)]) -> impl Fn(&str) -> Option<OsString> {
        let vars: Vec<(String, OsString)> = vars
            .iter()
            .map(|(name, value)| ((*name).to_string(), value.as_ref().to_os_string()))
            .collect();
        move |name: &str| {
            vars.iter()
                .find(|(key, _)| key == name)
                .map(|(_, value)| value.clone())
        }
    }

    /// 1213: a sandboxed Codex run in a per-task worktree can still build
    /// when `CARGO_TARGET_DIR` points outside it.
    #[test]
    fn codex_writable_roots_cover_cargo_dirs_outside_the_workspace() {
        let dir = tempdir().expect("tempdir");
        let workspace = dir.path().join("worktree");
        let target = dir.path().join("shared-target");
        let home = dir.path().join("home");
        let cargo_home = home.join(".cargo");
        let custom_home = dir.path().join("cargo-home");
        for path in [
            &workspace.join("target"),
            &target,
            &cargo_home,
            &custom_home,
        ] {
            fs::create_dir_all(path).expect("create dir");
        }

        let roots = codex_writable_roots(
            env_of(&[("CARGO_TARGET_DIR", &target), ("HOME", &home)]),
            &workspace,
        );
        assert_eq!(roots, [target, cargo_home]);

        // CARGO_HOME wins over ~/.cargo; a target inside the worktree, and
        // one that does not exist yet, are left out.
        let inside = workspace.join("target");
        let roots = codex_writable_roots(
            env_of(&[
                ("CARGO_TARGET_DIR", &inside),
                ("CARGO_HOME", &custom_home),
                ("HOME", &home),
            ]),
            &workspace,
        );
        assert_eq!(roots, [custom_home]);
        let missing = dir.path().join("missing-target");
        let roots = codex_writable_roots(env_of(&[("CARGO_TARGET_DIR", &missing)]), &workspace);
        assert!(roots.is_empty(), "{roots:?}");
    }

    #[test]
    fn chain_tools_never_reach_the_claude_tool_flags() {
        let tools = [
            "bash".to_string(),
            "chain.transfer".to_string(),
            "mcp__github__create_pr".to_string(),
        ];
        assert_eq!(
            render_claude_tool_policy(&tools),
            "Bash,mcp__github__create_pr"
        );
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
printf '%s\n' '{{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0}}'
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
            stream_usage: None,
            billing: None,
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
            env_passthrough: Vec::new(),
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
            trace_sink: None,
            metrics_sink: None,
            tool_correlation: None,
            provenance_sink: None,
            max_turns: None,
            live_output: None,
            thinking: None,
            data_llm: None,
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
        // Match whole arguments: the guard script inside --settings names git's `--bare` option.
        assert!(!args_text.lines().any(|arg| arg == "--bare"));
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
printf '%s\n' '{{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0}}'
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
            stream_usage: None,
            billing: None,
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
            stream_usage: None,
            billing: None,
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
