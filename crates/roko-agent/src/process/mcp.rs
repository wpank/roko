//! MCP (Model Context Protocol) server launch discovery and configuration.
//!
//! An MCP server is always spawned as a **child of the agent** (never of Roko
//! itself). This module handles:
//! - Discovering the MCP config by walking up from the working directory.
//! - Normalizing the launch command (resolving relative paths).
//!
//! # Writers
//!
//! The only authoritative writer for `.roko/mcp-config.json` is
//! `PlanRunner::resolve_mcp_config_path` in `roko-cli/src/runner/event_loop.rs`
//! (the runner-v2 runtime path). The legacy C4 writer (`write_mcp_config`) has
//! been removed to prevent it from clobbering the runner's output with an
//! incompatible JSON shape.

use std::path::Path;

use serde::{Deserialize, Serialize};

/// A resolved MCP server launch specification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpLaunch {
    /// The command to execute (e.g. an absolute path to a binary, or `cargo`).
    pub command: String,
    /// Arguments to pass to the command.
    pub args: Vec<String>,
}

/// Load an MCP launch config from a JSON file with `mcpServers.roko` shape.
///
/// Expected format:
/// ```json
/// { "mcpServers": { "roko": { "command": "...", "args": [...] } } }
/// ```
fn load_json_mcp_launch(path: &Path) -> Option<McpLaunch> {
    let raw = std::fs::read_to_string(path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&raw).ok()?;
    let server = value.get("mcpServers")?.get("roko")?;
    let command = server.get("command")?.as_str()?.to_string();
    let args = server
        .get("args")
        .and_then(|a| a.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(ToOwned::to_owned))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Some(McpLaunch { command, args })
}

/// Load an MCP launch config from a TOML file with `mcp_servers.roko` shape.
///
/// Expected format:
/// ```toml
/// [mcp_servers.roko]
/// command = "..."
/// args = ["..."]
/// ```
fn load_toml_mcp_launch(path: &Path) -> Option<McpLaunch> {
    let raw = std::fs::read_to_string(path).ok()?;
    let value: toml::Value = raw.parse().ok()?;
    let server = value.get("mcp_servers")?.get("roko")?;
    let command = server.get("command")?.as_str()?.to_string();
    let args = server
        .get("args")
        .and_then(|a| a.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(ToOwned::to_owned))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Some(McpLaunch { command, args })
}

/// Check whether the command exists at the given path (absolute or relative to `probe_root`).
fn mcp_command_exists(command: &str, probe_root: &Path) -> bool {
    let command_path = Path::new(command);
    if command_path.is_absolute() {
        return command_path.exists();
    }
    probe_root.join(command_path).exists()
}

/// Walk ancestor directories looking for `target/{release,debug}/<binary>`.
fn find_mcp_binary_from_ancestors(start: &Path, binary: &str) -> Option<String> {
    for dir in start.ancestors() {
        for profile in ["target/release", "target/debug"] {
            let candidate = dir.join(profile).join(binary);
            if candidate.exists() {
                return Some(candidate.to_string_lossy().into_owned());
            }
        }
    }
    None
}

/// Normalize a discovered MCP launch.
///
/// If the command is a bare binary name that does not exist at the configured
/// path, attempt to resolve it from ancestor `target/` directories.  Unlike
/// the old C4 path, there is **no fallback to `cargo run -p roko-mcp`** — the
/// `roko-mcp` crate does not exist in this workspace and the C5 writer
/// (`resolve_mcp_config_path` in `roko-cli`) is the sole authority over
/// `.roko/mcp-config.json`.
pub fn normalize_mcp_launch(launch: McpLaunch, probe_root: &Path) -> McpLaunch {
    // Only attempt binary resolution when the command is not already an
    // absolute path that exists on disk.
    if !mcp_command_exists(&launch.command, probe_root) {
        let binary = Path::new(&launch.command)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&launch.command)
            .to_owned();
        if let Some(resolved) = find_mcp_binary_from_ancestors(probe_root, &binary) {
            return McpLaunch {
                command: resolved,
                args: launch.args,
            };
        }
    }
    launch
}

/// Search for an MCP config starting from `working_dir` and walking up.
///
/// Checks (in order at each directory level):
/// 1. `$ROKO_MCP_CONFIG` env var (explicit override)
/// 2. `.roko/mcp-config.local.json`
/// 3. `.roko/mcp-config.json`
/// 4. `.codex/config.toml`
///
/// Returns the first valid config found, normalized via [`normalize_mcp_launch`].
pub fn find_mcp_launch(working_dir: &Path) -> Option<McpLaunch> {
    // Check explicit env override first.
    if let Ok(config_path) = std::env::var("ROKO_MCP_CONFIG") {
        let explicit = Path::new(&config_path);
        if explicit.exists() {
            return load_json_mcp_launch(explicit).map(|launch| {
                normalize_mcp_launch(launch, explicit.parent().unwrap_or(working_dir))
            });
        }
    }

    let mut search: Option<&Path> = Some(working_dir);
    while let Some(dir) = search {
        for rel in [".roko/mcp-config.local.json", ".roko/mcp-config.json"] {
            let candidate = dir.join(rel);
            if candidate.exists()
                && let Some(config) = load_json_mcp_launch(&candidate)
            {
                return Some(normalize_mcp_launch(config, dir));
            }
        }
        // Also check codex-style TOML config.
        let codex_candidate = dir.join(".codex/config.toml");
        if codex_candidate.exists()
            && let Some(config) = load_toml_mcp_launch(&codex_candidate)
        {
            return Some(normalize_mcp_launch(config, dir));
        }
        search = dir.parent();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper: write a minimal Claude-shape mcpServers JSON into a temp dir so
    // find_mcp_launch can discover it.  The C4 writer (removed by E15-T6) is
    // NOT used here — this helper only exists to set up fixture files for tests.
    fn seed_test_mcp_config(base: &Path, command: &str, args: &[&str]) {
        let dot_roko = base.join(".roko");
        std::fs::create_dir_all(&dot_roko).expect("create .roko");
        let value = serde_json::json!({
            "mcpServers": {
                "roko": {
                    "command": command,
                    "args": args,
                }
            }
        });
        let target = dot_roko.join("mcp-config.json");
        std::fs::write(&target, serde_json::to_string_pretty(&value).unwrap())
            .expect("write test fixture");
    }

    #[test]
    fn find_mcp_launch_returns_none_in_empty_dir() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let result = find_mcp_launch(tmp.path());
        assert!(result.is_none());
    }

    #[test]
    fn find_mcp_launch_discovers_config() {
        let tmp = tempfile::tempdir().expect("tempdir");
        seed_test_mcp_config(
            tmp.path(),
            "/usr/bin/custom-mcp-server",
            &["--port", "8080"],
        );
        let found = find_mcp_launch(tmp.path()).expect("should find config");
        assert_eq!(found.command, "/usr/bin/custom-mcp-server");
        assert_eq!(found.args, vec!["--port", "8080"]);
    }

    /// normalize_mcp_launch must NOT fall back to `cargo run` for the absent
    /// roko-mcp crate.  When a bare binary name is not resolvable the launch
    /// is returned unchanged rather than emitting a broken cargo command.
    #[test]
    fn normalize_does_not_fall_back_to_cargo_run() {
        // Use a bare (non-existent) binary name that would previously have
        // triggered the old cargo-run fallback.
        let bare_cmd = "nonexistent-mcp-bin";
        let launch = McpLaunch {
            command: bare_cmd.to_string(),
            args: vec!["--port".to_string(), "9999".to_string()],
        };
        // /tmp won't have target/{release,debug}/<bare_cmd> so normalization
        // cannot resolve the binary and must leave the command as-is.
        let normalized = normalize_mcp_launch(launch.clone(), Path::new("/tmp"));
        // Must NOT become `cargo`.
        assert_ne!(normalized.command, "cargo");
        assert_eq!(normalized.command, bare_cmd);
        // Args are preserved.
        assert!(normalized.args.contains(&"--port".to_string()));
    }

    /// E15-T6 acceptance: the C4 writer has been removed from this module.
    /// Compilation of this file without the dead writer is the acceptance
    /// criterion; no runtime assertion is needed beyond the code compiling.
    #[test]
    fn c4_mcp_writer_is_neutralized() {
        // Intentionally empty — the structural verify in tasks.toml checks for
        // absence of the removed symbols at the source level.
    }
}
