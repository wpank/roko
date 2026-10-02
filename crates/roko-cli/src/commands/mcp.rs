//! MCP configuration dispatch and auto-discovery helpers.

use anyhow::{Context as _, Result, anyhow};
use clap::Subcommand;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Subcommand)]
pub(crate) enum ConfigMcpCmd {
    /// List configured MCP servers.
    List {
        /// Directory containing `roko.toml` (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Test an MCP server by performing a real initialize + tools/list handshake.
    Test {
        /// MCP server name from the config.
        name: String,
        /// Per-stage timeout in seconds (applies to initialize, tools/list, and shutdown).
        #[arg(long, default_value_t = roko_core::defaults::DEFAULT_MCP_DISCOVERY_TIMEOUT_SECS, value_parser = clap::value_parser!(u64).range(1..=60))]
        timeout_secs: u64,
        /// Directory containing `.roko/mcp-config.json` (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Add an MCP server entry to `.roko/mcp-config.json`.
    Add {
        /// Server name (e.g. "roko").
        name: String,
        /// Launch command (e.g. "/usr/local/bin/roko-mcp").
        command: String,
        /// Optional arguments.
        #[arg(last = true)]
        args: Vec<String>,
        /// Directory containing `.roko/` (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
}

/// Dispatch `roko config mcp` subcommands.
pub(crate) async fn dispatch_mcp_cmd(cmd: &ConfigMcpCmd, workdir: &Path) -> Result<()> {
    match cmd {
        ConfigMcpCmd::List {
            workdir: wd_override,
        } => {
            let wd = wd_override.as_deref().unwrap_or(workdir);
            let resolved = resolve_mcp_config_path(None, wd);
            let path = resolved.ok_or_else(|| {
                anyhow!("no MCP config found; set agent.mcp_config in roko.toml or create .roko/mcp.json")
            })?;
            let cfg = roko_agent::mcp::McpConfig::load(&path)
                .map_err(|e| anyhow!("load MCP config from {}: {}", path.display(), e))?;
            println!("MCP config: {}", path.display());
            if cfg.servers.is_empty() {
                println!("  (no servers configured)");
            } else {
                println!("{} server(s):", cfg.servers.len());
                for server in &cfg.servers {
                    println!("  [{:?}] {} ({})", server.tier, server.name, server.command);
                }
            }
            Ok(())
        }
        ConfigMcpCmd::Test {
            name,
            workdir: wd_override,
            timeout_secs,
        } => {
            let wd = wd_override.as_deref().unwrap_or(workdir);
            let resolved = resolve_mcp_config_path(None, wd);
            let path = resolved.ok_or_else(|| {
                anyhow!("no MCP config found; set agent.mcp_config in roko.toml or create .roko/mcp.json")
            })?;
            if !path.is_file() {
                return Err(anyhow!("MCP config file not found: {}", path.display()));
            }
            let cfg = roko_agent::mcp::McpConfig::load(&path)
                .map_err(|e| anyhow!("parse MCP config at {}: {}", path.display(), e))?;
            let server = cfg
                .servers
                .iter()
                .find(|s| s.name == *name)
                .ok_or_else(|| anyhow!("server '{}' not found in {}", name, path.display()))?;
            let custom_timeout = Some(Duration::from_secs(*timeout_secs));
            let report =
                roko_agent::mcp::test_mcp_server(server, path.clone(), custom_timeout).await;
            println!("MCP test: {} ({})", server.name, path.display());
            println!("  status: {:?}", report.status);
            println!(
                "  protocol: {}",
                report.protocol_version.as_deref().unwrap_or("n/a")
            );
            println!("  tools: {}", report.tool_count);
            if report.status == roko_agent::mcp::McpTestStatus::Failed {
                return Err(anyhow!("MCP test failed for server '{}'", name));
            }
            Ok(())
        }
        ConfigMcpCmd::Add {
            name,
            command,
            args,
            workdir: wd_override,
        } => {
            let wd = wd_override.as_deref().unwrap_or(workdir);
            let path = {
                let roko_dir = wd.join(".roko");
                std::fs::create_dir_all(&roko_dir)
                    .with_context(|| format!("create {}", roko_dir.display()))?;
                roko_dir.join("mcp.json")
            };
            let mut cfg = if path.is_file() {
                roko_agent::mcp::McpConfig::load(&path).map_err(|e| {
                    anyhow!("load existing MCP config from {}: {}", path.display(), e)
                })?
            } else {
                roko_agent::mcp::McpConfig {
                    servers: Vec::new(),
                }
            };
            if cfg.servers.iter().any(|s| s.name == *name) {
                return Err(anyhow!(
                    "server '{}' already exists in {}",
                    name,
                    path.display()
                ));
            }
            cfg.servers.push(roko_agent::mcp::McpServerConfig {
                name: name.clone(),
                transport: roko_agent::mcp::McpTransportConfig::Stdio,
                command: command.clone(),
                args: args.clone(),
                env: Default::default(),
                endpoint: None,
                auth_token: None,
                tier: Default::default(),
            });
            let json = serde_json::to_string_pretty(&cfg).context("serialize MCP config")?;
            std::fs::write(&path, json).with_context(|| format!("write {}", path.display()))?;
            println!("added server '{}' to {}", name, path.display());
            Ok(())
        }
    }
}

/// Resolve the MCP config path using the following chain:
/// 1. Explicit path (if provided)
/// 2. `.roko/mcp.json` relative to workdir
/// 3. `~/.claude/mcp-config.json`
/// 4. Walk-up `.mcp.json` discovery from workdir
pub(crate) fn resolve_mcp_config_path(explicit: Option<&Path>, workdir: &Path) -> Option<PathBuf> {
    if let Some(p) = explicit {
        return Some(p.to_path_buf());
    }
    let roko_local = workdir.join(".roko").join("mcp.json");
    if roko_local.is_file() {
        return Some(roko_local);
    }
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        let claude_default = home.join(".claude").join("mcp-config.json");
        if claude_default.is_file() {
            return Some(claude_default);
        }
    }
    roko_agent::mcp::find_mcp_config(workdir)
        .and_then(|r| r.ok())
        .map(|(p, _)| p)
}

/// Locate a named binary via `$PATH` scan.
///
/// Returns the full path to the first matching executable, or `None` if the
/// binary is not found on the PATH.
pub(crate) fn find_binary_on_path(name: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(name);
        if let Ok(meta) = std::fs::metadata(&candidate)
            && meta.is_file()
        {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if meta.permissions().mode() & 0o111 != 0 {
                    return Some(candidate);
                }
            }
            #[cfg(not(unix))]
            {
                return Some(candidate);
            }
        }
    }
    None
}

/// Walk ancestor directories looking for `target/{release,debug}/<binary>`.
///
/// This covers the common developer workflow where the binary is built locally
/// but not installed to `$PATH`.
pub(crate) fn find_binary_in_target_dirs(start: &Path, name: &str) -> Option<PathBuf> {
    for dir in start.ancestors() {
        for profile in ["target/release", "target/debug"] {
            let candidate = dir.join(profile).join(name);
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }
    None
}

/// Discover the `roko-mcp-github` binary.
///
/// Search order:
/// 1. `$PATH` scan -- covers installed binaries
/// 2. Ancestor `target/{release,debug}/` -- covers local dev builds
///
/// Returns the binary's absolute path as a string, or `None` if not found.
pub(crate) fn discover_roko_github_binary(workdir: &Path) -> Option<String> {
    const BINARY: &str = "roko-mcp-github";

    if let Some(path) = find_binary_on_path(BINARY) {
        tracing::debug!(path = %path.display(), "discovered roko-mcp-github on PATH");
        return Some(path.to_string_lossy().into_owned());
    }

    if let Some(path) = find_binary_in_target_dirs(workdir, BINARY) {
        tracing::debug!(path = %path.display(), "discovered roko-mcp-github in target/");
        return Some(path.to_string_lossy().into_owned());
    }

    tracing::debug!("roko-mcp-github not found on PATH or in target/ dirs");
    None
}

/// Add a `github` MCP server entry using the given command path.
///
/// Internal helper used by both production code and tests.  The caller is
/// responsible for ensuring `command` points to a valid `roko-mcp-github`
/// binary.  `GITHUB_TOKEN` is forwarded from the current environment when set.
pub(crate) fn add_github_mcp_server(config: &mut roko_agent::mcp::McpConfig, command: String) {
    let mut env = std::collections::HashMap::new();
    if let Ok(token) = std::env::var("GITHUB_TOKEN")
        && !token.is_empty()
    {
        env.insert("GITHUB_TOKEN".to_string(), token);
    }

    config.servers.push(roko_agent::mcp::McpServerConfig {
        name: "github".to_string(),
        transport: roko_agent::mcp::McpTransportConfig::Stdio,
        command,
        args: vec![],
        env,
        endpoint: None,
        auth_token: None,
        tier: Default::default(),
    });
}

/// Augment an [`McpConfig`](roko_agent::mcp::McpConfig) with an auto-discovered
/// `roko-mcp-github` server entry.
///
/// The entry is only added when:
/// - The binary is discoverable (PATH or target/)
/// - No server named `"github"` already exists in `config.servers`
///
/// The auto-discovered entry includes `GITHUB_TOKEN` from the current
/// environment when the variable is set.
pub(crate) fn augment_mcp_config_with_github(
    config: &mut roko_agent::mcp::McpConfig,
    workdir: &Path,
) {
    // User-configured 'github' server takes precedence -- never override it.
    if config.servers.iter().any(|s| s.name == "github") {
        tracing::debug!("user-configured 'github' MCP server present; skipping auto-discovery");
        return;
    }

    let Some(command) = discover_roko_github_binary(workdir) else {
        tracing::debug!("roko-mcp-github not found; skipping auto-discovery");
        return;
    };

    add_github_mcp_server(config, command);
    tracing::info!("auto-discovered roko-mcp-github; added 'github' MCP server entry");
}

/// Resolve the MCP config path, then auto-augment it with `roko-mcp-github`
/// when the binary is available and no user-configured `github` server exists.
///
/// The augmented config is written to `<roko_dir>/mcp-auto.json`.  When
/// augmentation adds no new entries the path to the original config is
/// returned unchanged so callers that already have a fully-configured MCP
/// file do not pay the extra write cost.
///
/// Returns `None` when no MCP config is found **and** `roko-mcp-github` is
/// not available.
pub fn resolve_mcp_config_with_autodiscovery(workdir: &Path, roko_dir: &Path) -> Option<PathBuf> {
    // Load base config (may be None when no file exists yet).
    let base_path = resolve_mcp_config_path(None, workdir);
    let mut config: roko_agent::mcp::McpConfig = match &base_path {
        Some(p) => match roko_agent::mcp::McpConfig::load(p) {
            Ok(c) => c,
            Err(err) => {
                tracing::warn!(
                    path = %p.display(),
                    error = %err,
                    "MCP config load failed; using empty config for github augmentation"
                );
                roko_agent::mcp::McpConfig { servers: vec![] }
            }
        },
        None => roko_agent::mcp::McpConfig { servers: vec![] },
    };

    let servers_before = config.servers.len();
    augment_mcp_config_with_github(&mut config, workdir);

    // If augmentation added entries, write the merged config to mcp-auto.json.
    if config.servers.len() > servers_before {
        let auto_path = roko_dir.join("mcp-auto.json");
        match serde_json::to_string_pretty(&config) {
            Ok(json) => match std::fs::write(&auto_path, json) {
                Ok(()) => {
                    tracing::debug!(
                        path = %auto_path.display(),
                        "wrote augmented MCP config"
                    );
                    return Some(auto_path);
                }
                Err(err) => {
                    tracing::warn!(
                        path = %auto_path.display(),
                        error = %err,
                        "failed to write augmented MCP config; falling back to base"
                    );
                }
            },
            Err(err) => {
                tracing::warn!(
                    error = %err,
                    "failed to serialize augmented MCP config; falling back to base"
                );
            }
        }
    }

    // Return original base path (or None if nothing was found/discovered).
    if config.servers.is_empty() {
        None
    } else {
        base_path
    }
}
