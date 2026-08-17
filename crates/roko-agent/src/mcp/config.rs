//! `.mcp.json` walk-up config reader (SS36.61).
//!
//! Searches upward from a starting directory to find the nearest
//! `.mcp.json` config file, then parses it into [`McpConfig`].

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::safety::capabilities::PluginTier;

/// Configuration for a single MCP server.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpServerConfig {
    /// Logical name for the server (used as prefix in tool names).
    pub name: String,
    /// Transport used to reach the server.
    #[serde(default)]
    pub transport: McpTransportConfig,
    /// The command to spawn the server process.
    #[serde(default)]
    pub command: String,
    /// Arguments passed to the command.
    #[serde(default)]
    pub args: Vec<String>,
    /// Environment variables set for the server process.
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// Streamable HTTP endpoint for remote MCP servers.
    #[serde(default)]
    pub endpoint: Option<String>,
    /// Optional bearer token or environment placeholder for HTTP auth.
    #[serde(default)]
    pub auth_token: Option<String>,
    /// Trust tier for this MCP server (1-5). Defaults to `Sandboxed` (tier 2).
    ///
    /// Lower tiers are denied secrets and network egress. See
    /// [`PluginTier`](crate::safety::capabilities::PluginTier) for the
    /// full capability matrix.
    #[serde(default)]
    pub tier: PluginTier,
}

/// MCP server transport kind.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum McpTransportConfig {
    /// Local child process over newline-delimited JSON-RPC.
    #[default]
    Stdio,
    /// Remote Streamable HTTP endpoint.
    Http,
}

/// Top-level MCP configuration parsed from `.mcp.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpConfig {
    /// List of MCP server configurations.
    #[serde(default)]
    pub servers: Vec<McpServerConfig>,
}

impl McpConfig {
    /// Load and parse an MCP config from an explicit file path.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or is not valid JSON.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        load_config(path).map(|(_path, cfg)| cfg)
    }
}

/// Walk up from `start_dir` looking for a `.mcp.json` file.
///
/// Checks `start_dir`, then its parent, then grandparent, etc. until
/// the filesystem root. Returns the parsed config and its path on
/// success.
///
/// # Errors
///
/// Returns `None` if no `.mcp.json` is found. Returns `Some(Err(...))`
/// if a file is found but cannot be read or parsed.
pub fn find_mcp_config(start_dir: &Path) -> Option<Result<(PathBuf, McpConfig), ConfigError>> {
    let mut dir = start_dir.to_path_buf();
    loop {
        let candidate = dir.join(".mcp.json");
        if candidate.is_file() {
            return Some(load_config(&candidate));
        }
        if !dir.pop() {
            break;
        }
    }

    let home_candidate = std::env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join(".mcp.json"));
    if let Some(candidate) = home_candidate.filter(|candidate| candidate.is_file()) {
        return Some(load_config(&candidate));
    }

    None
}

/// Load and parse a `.mcp.json` file.
fn load_config(path: &Path) -> Result<(PathBuf, McpConfig), ConfigError> {
    let content = std::fs::read_to_string(path).map_err(|e| ConfigError::Io {
        path: path.to_path_buf(),
        detail: e.to_string(),
    })?;
    let config: McpConfig = serde_json::from_str(&content).map_err(|e| ConfigError::Parse {
        path: path.to_path_buf(),
        detail: e.to_string(),
    })?;
    Ok((path.to_path_buf(), config))
}

/// Commands considered safe to spawn as MCP server processes.
///
/// Doctor checks warn when a configured MCP server command falls outside this
/// list. Operators can extend the set via `[security.mcp_allowed_commands]` in
/// `roko.toml` (not yet wired), but the defaults cover the common case.
pub const DEFAULT_ALLOWED_COMMANDS: &[&str] = &[
    "npx",
    "node",
    "deno",
    "bun",
    "python",
    "python3",
    "uvx",
    "uv",
    "cargo",
    "docker",
    "mcp-server-fetch",
    "mcp-server-filesystem",
    "mcp-server-git",
    "mcp-server-github",
    "mcp-server-slack",
    "mcp-server-sqlite",
    "mcp-server-postgres",
    "mcp-server-memory",
    "mcp-server-brave-search",
    "mcp-server-puppeteer",
    "mcp-server-sequential-thinking",
];

/// Environment variable key patterns that likely carry secrets.
///
/// Doctor warns (but does not print values) when MCP server env maps contain
/// keys matching any of these substrings (case-insensitive).
pub const SENSITIVE_ENV_PATTERNS: &[&str] = &[
    "SECRET",
    "TOKEN",
    "KEY",
    "PASSWORD",
    "CREDENTIAL",
    "AUTH",
    "PRIVATE",
    "API_KEY",
    "APIKEY",
];

/// Check whether `command` is on the default allowlist.
///
/// Compares against the basename (last path component) so
/// `/usr/local/bin/npx` still matches `npx`.
#[must_use]
pub fn is_command_allowed(command: &str, extra_allowed: &[&str]) -> bool {
    if command.is_empty() {
        return true; // HTTP-only servers have no command.
    }
    let basename = std::path::Path::new(command)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(command);
    DEFAULT_ALLOWED_COMMANDS
        .iter()
        .chain(extra_allowed.iter())
        .any(|allowed| basename == *allowed)
}

/// Return the names of env keys that look like they carry secrets.
///
/// Values are **not** returned — only key names — to avoid leaking secrets
/// into doctor output.
#[must_use]
pub fn sensitive_env_keys(env: &std::collections::HashMap<String, String>) -> Vec<String> {
    let mut hits = Vec::new();
    for key in env.keys() {
        let upper = key.to_ascii_uppercase();
        if SENSITIVE_ENV_PATTERNS.iter().any(|pat| upper.contains(pat)) {
            hits.push(key.clone());
        }
    }
    hits.sort();
    hits
}

/// Check whether `command` resolves to an executable on the system `PATH`.
///
/// Returns `true` for empty commands (HTTP-only servers), absolute paths that
/// exist as files, and bare names that are found in a `PATH` directory.
/// Does **not** spawn the command — only inspects the filesystem.
#[must_use]
pub fn is_command_on_path(command: &str) -> bool {
    if command.is_empty() {
        return true; // HTTP-only servers have no command.
    }
    let p = std::path::Path::new(command);
    // Absolute or relative path: just check existence.
    if p.is_absolute() || command.contains('/') {
        return p.is_file();
    }
    // Bare name: search PATH.
    if let Some(path_var) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join(command);
            if candidate.is_file() {
                return true;
            }
        }
    }
    false
}

/// Return keys of env entries whose values are `${VAR}` or `$VAR` references
/// where the referenced variable is **not set** in the current process
/// environment.
///
/// Only key names are returned; values are not included in the output.
#[must_use]
pub fn unset_env_var_refs(env: &std::collections::HashMap<String, String>) -> Vec<String> {
    let mut missing = Vec::new();
    for (key, value) in env {
        // Match ${VAR} and $VAR patterns.
        let var_name = if value.starts_with("${") && value.ends_with('}') {
            Some(&value[2..value.len() - 1])
        } else if value.starts_with('$') && value.len() > 1 {
            Some(&value[1..])
        } else {
            None
        };
        if let Some(var) = var_name
            && !var.is_empty()
            && std::env::var_os(var).is_none()
        {
            missing.push(key.clone());
        }
    }
    missing.sort();
    missing
}

/// Return the names of sensitive env keys whose values look like hardcoded
/// secrets rather than environment variable references (`${VAR}` / `$VAR`).
///
/// A value is treated as hardcoded when it is non-empty AND does not start
/// with `$`. Values are **not** returned to avoid leaking them into
/// diagnostic output.
#[must_use]
pub fn hardcoded_secret_values(env: &std::collections::HashMap<String, String>) -> Vec<String> {
    let mut hits = Vec::new();
    for (key, value) in env {
        let upper = key.to_ascii_uppercase();
        let is_sensitive = SENSITIVE_ENV_PATTERNS.iter().any(|pat| upper.contains(pat));
        let is_hardcoded = !value.is_empty() && !value.starts_with('$');
        if is_sensitive && is_hardcoded {
            hits.push(key.clone());
        }
    }
    hits.sort();
    hits
}

/// Errors from MCP config loading.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// Could not read the config file.
    #[error("failed to read {path}: {detail}")]
    Io {
        /// Path that failed.
        path: PathBuf,
        /// Underlying error description.
        detail: String,
    },

    /// Could not parse the config file as JSON.
    #[error("failed to parse {path}: {detail}")]
    Parse {
        /// Path that failed.
        path: PathBuf,
        /// Underlying error description.
        detail: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn mcp_config_parse_full() {
        let json = r#"{
            "servers": [
                {
                    "name": "filesystem",
                    "transport": "stdio",
                    "command": "npx",
                    "args": ["-y", "@modelcontextprotocol/server-filesystem"],
                    "env": {"HOME": "/tmp"}
                },
                {
                    "name": "git",
                    "command": "mcp-git",
                    "args": []
                }
            ]
        }"#;
        let config: McpConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.servers.len(), 2);
        assert_eq!(config.servers[0].name, "filesystem");
        assert_eq!(config.servers[0].transport, McpTransportConfig::Stdio);
        assert_eq!(config.servers[0].command, "npx");
        assert_eq!(
            config.servers[0].args,
            vec!["-y", "@modelcontextprotocol/server-filesystem"]
        );
        assert_eq!(config.servers[0].env.get("HOME").unwrap(), "/tmp");
        assert_eq!(config.servers[1].name, "git");
        assert!(config.servers[1].env.is_empty());
        assert_eq!(config.servers[1].transport, McpTransportConfig::Stdio);
    }

    #[test]
    fn mcp_config_parse_http_transport() {
        let json = r#"{
            "servers": [{
                "name": "remote-tools",
                "transport": "http",
                "endpoint": "https://tools.example.com/mcp",
                "auth_token": "${MCP_AUTH_TOKEN}"
            }]
        }"#;
        let config: McpConfig = serde_json::from_str(json).unwrap();
        let server = &config.servers[0];
        assert_eq!(server.transport, McpTransportConfig::Http);
        assert_eq!(
            server.endpoint.as_deref(),
            Some("https://tools.example.com/mcp")
        );
        assert_eq!(server.auth_token.as_deref(), Some("${MCP_AUTH_TOKEN}"));
    }

    #[test]
    fn mcp_config_parse_empty_servers() {
        let json = r#"{"servers": []}"#;
        let config: McpConfig = serde_json::from_str(json).unwrap();
        assert!(config.servers.is_empty());
    }

    #[test]
    fn mcp_find_config_in_start_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let config = McpConfig {
            servers: vec![McpServerConfig {
                name: "test".to_string(),
                transport: McpTransportConfig::Stdio,
                command: "echo".to_string(),
                args: vec![],
                env: HashMap::new(),
                endpoint: None,
                auth_token: None,
                tier: PluginTier::default(),
            }],
        };
        let config_path = tmp.path().join(".mcp.json");
        fs::write(&config_path, serde_json::to_string(&config).unwrap()).unwrap();

        let result = find_mcp_config(tmp.path());
        assert!(result.is_some());
        let (path, parsed) = result.unwrap().unwrap();
        assert_eq!(path, config_path);
        assert_eq!(parsed.servers.len(), 1);
        assert_eq!(parsed.servers[0].name, "test");
    }

    #[test]
    fn mcp_find_config_walks_up() {
        let tmp = tempfile::tempdir().unwrap();
        let child = tmp.path().join("a").join("b").join("c");
        fs::create_dir_all(&child).unwrap();

        let config = McpConfig {
            servers: vec![McpServerConfig {
                name: "parent".to_string(),
                transport: McpTransportConfig::Stdio,
                command: "cat".to_string(),
                args: vec![],
                env: HashMap::new(),
                endpoint: None,
                auth_token: None,
                tier: PluginTier::default(),
            }],
        };
        let config_path = tmp.path().join(".mcp.json");
        fs::write(&config_path, serde_json::to_string(&config).unwrap()).unwrap();

        let result = find_mcp_config(&child);
        assert!(result.is_some());
        let (path, parsed) = result.unwrap().unwrap();
        assert_eq!(path, config_path);
        assert_eq!(parsed.servers[0].name, "parent");
    }

    #[test]
    fn mcp_find_config_not_found() {
        let tmp = tempfile::tempdir().unwrap();
        let child = tmp.path().join("no_config_here");
        fs::create_dir_all(&child).unwrap();
        let result = find_mcp_config(&child);
        assert!(result.is_none());
    }

    #[test]
    fn mcp_find_config_invalid_json() {
        let tmp = tempfile::tempdir().unwrap();
        let config_path = tmp.path().join(".mcp.json");
        fs::write(&config_path, "not valid json!!!").unwrap();

        let result = find_mcp_config(tmp.path());
        assert!(result.is_some());
        let err = result.unwrap().unwrap_err();
        assert!(matches!(err, ConfigError::Parse { .. }));
    }

    // -- is_command_on_path tests -------------------------------------------

    #[test]
    fn is_command_on_path_empty_command_returns_true() {
        // HTTP-only servers pass through.
        assert!(is_command_on_path(""));
    }

    #[test]
    fn is_command_on_path_finds_well_known_binary() {
        // `sh` is present on every POSIX system.
        assert!(
            is_command_on_path("sh"),
            "expected `sh` to be found on PATH"
        );
    }

    #[test]
    fn is_command_on_path_rejects_nonexistent_command() {
        assert!(
            !is_command_on_path("__roko_does_not_exist_xyz_9999__"),
            "should return false for a nonexistent command"
        );
    }

    #[test]
    fn is_command_on_path_accepts_existing_absolute_path() {
        // /bin/sh or /usr/bin/sh should exist.
        let candidate = if std::path::Path::new("/bin/sh").exists() {
            "/bin/sh"
        } else {
            "/usr/bin/sh"
        };
        assert!(
            is_command_on_path(candidate),
            "absolute path to sh should be accepted"
        );
    }

    #[test]
    fn is_command_on_path_rejects_nonexistent_absolute_path() {
        assert!(
            !is_command_on_path("/nonexistent/path/to/binary"),
            "should return false for a missing absolute path"
        );
    }

    // -- unset_env_var_refs tests -------------------------------------------

    #[test]
    fn unset_env_var_refs_ignores_literal_values() {
        let mut env = HashMap::new();
        env.insert("FOO".to_string(), "literal_value".to_string());
        assert!(unset_env_var_refs(&env).is_empty());
    }

    #[test]
    fn unset_env_var_refs_detects_unset_dollar_brace_ref() {
        let mut env = HashMap::new();
        env.insert(
            "GITHUB_TOKEN".to_string(),
            "${__ROKO_UNSET_VAR_UNLIKELY__}".to_string(),
        );
        let missing = unset_env_var_refs(&env);
        assert!(
            missing.contains(&"GITHUB_TOKEN".to_string()),
            "should detect unset ${{VAR}} reference"
        );
    }

    #[test]
    fn unset_env_var_refs_ignores_set_dollar_brace_ref() {
        let mut env = HashMap::new();
        // PATH is always set.
        env.insert("SOME_KEY".to_string(), "${PATH}".to_string());
        let missing = unset_env_var_refs(&env);
        assert!(
            !missing.contains(&"SOME_KEY".to_string()),
            "should not flag ${{PATH}} because PATH is set"
        );
    }

    #[test]
    fn unset_env_var_refs_detects_unset_dollar_ref() {
        let mut env = HashMap::new();
        env.insert(
            "MY_TOKEN".to_string(),
            "$__ROKO_UNSET_VAR_UNLIKELY__".to_string(),
        );
        let missing = unset_env_var_refs(&env);
        assert!(
            missing.contains(&"MY_TOKEN".to_string()),
            "should detect unset $VAR reference"
        );
    }

    // -- hardcoded_secret_values tests -------------------------------------

    #[test]
    fn hardcoded_secret_values_detects_literal_secret() {
        let mut env = HashMap::new();
        env.insert("GITHUB_TOKEN".to_string(), "ghp_abc123literal".to_string());
        let hits = hardcoded_secret_values(&env);
        assert!(
            hits.contains(&"GITHUB_TOKEN".to_string()),
            "should flag a sensitive key with a literal value"
        );
    }

    #[test]
    fn hardcoded_secret_values_ignores_env_var_reference() {
        let mut env = HashMap::new();
        env.insert("GITHUB_TOKEN".to_string(), "${GITHUB_TOKEN}".to_string());
        let hits = hardcoded_secret_values(&env);
        assert!(
            hits.is_empty(),
            "should not flag a sensitive key whose value is an env-var reference"
        );
    }

    #[test]
    fn hardcoded_secret_values_ignores_non_sensitive_key() {
        let mut env = HashMap::new();
        env.insert(
            "DATABASE_URL".to_string(),
            "postgres://localhost/db".to_string(),
        );
        let hits = hardcoded_secret_values(&env);
        assert!(
            hits.is_empty(),
            "DATABASE_URL does not match sensitive patterns so should not be flagged"
        );
    }

    #[test]
    fn hardcoded_secret_values_ignores_empty_value() {
        let mut env = HashMap::new();
        env.insert("API_KEY".to_string(), "".to_string());
        let hits = hardcoded_secret_values(&env);
        assert!(
            hits.is_empty(),
            "empty value should not be flagged as hardcoded"
        );
    }
}
