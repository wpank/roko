//! TOML-based plugin manifest loading (TOOL-04).
//!
//! Defines the on-disk format for plugin manifests and provides a loader
//! that reads TOML files and constructs plugin metadata.
//!
//! # Plugin manifest format
//!
//! ```toml
//! [plugin]
//! name = "my-plugin"
//! version = "1.0.0"
//! description = "A description of the plugin"
//! author = "Author Name"
//!
//! # Tier 1: Prompt templates
//! [[prompts]]
//! name = "pr-review"
//! role = "reviewer"
//! template = """
//! You are a code reviewer. Review the following PR...
//! """
//!
//! [[prompts]]
//! name = "implementation"
//! role = "implementer"
//! template = "Implement the feature described below..."
//!
//! # Tier 2: Tool profile bundles
//! [[profiles]]
//! name = "read-only"
//! allowed_tools = ["read_file", "grep", "glob", "web_search"]
//! denied_tools = ["bash", "write_file", "edit_file"]
//!
//! # Tier 3: Declarative tool definitions
//! [[tools]]
//! name = "lint-check"
//! description = "Run linter on the current file"
//! command = "cargo clippy -- -D warnings"
//! timeout_ms = 30000
//!
//! # Event source triggers
//! [[triggers]]
//! kind = "cron"
//! expression = "0 */5 * * * *"
//! description = "Run every 5 minutes"
//!
//! [[triggers]]
//! kind = "file_watch"
//! paths = ["src/", "tests/"]
//! include = ["*.rs"]
//! ```

use std::path::Path;

use roko_core::{Result, RokoError};
use serde::{Deserialize, Serialize};

// ─── TOML schema ────────────────────────────────────────────────────────

/// Top-level TOML manifest that a plugin author writes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginManifestFile {
    /// Plugin metadata.
    pub plugin: PluginMeta,
    /// Tier 1: Prompt templates.
    #[serde(default)]
    pub prompts: Vec<PromptTemplate>,
    /// Tier 2: Tool profile bundles.
    #[serde(default)]
    pub profiles: Vec<ToolProfileBundle>,
    /// Tier 3: Declarative tool definitions (shell commands).
    #[serde(default)]
    pub tools: Vec<DeclarativeTool>,
    /// Event source triggers.
    #[serde(default)]
    pub triggers: Vec<TriggerDef>,
    /// Plugin dependencies (other plugins required).
    #[serde(default)]
    pub dependencies: Vec<PluginDependency>,
}

/// Plugin metadata section.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginMeta {
    /// Human-readable plugin name.
    pub name: String,
    /// Semantic version string.
    pub version: String,
    /// Optional description.
    #[serde(default)]
    pub description: Option<String>,
    /// Optional author.
    #[serde(default)]
    pub author: Option<String>,
    /// Optional license.
    #[serde(default)]
    pub license: Option<String>,
}

/// Tier 1: A prompt template that can be registered with the prompt system.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PromptTemplate {
    /// Template identifier (e.g., "pr-review", "implementation").
    pub name: String,
    /// Role this template is designed for (e.g., "implementer", "reviewer").
    #[serde(default)]
    pub role: Option<String>,
    /// The prompt template text. May contain `{{variable}}` placeholders.
    pub template: String,
    /// Optional description of when to use this template.
    #[serde(default)]
    pub description: Option<String>,
}

/// Tier 2: A bundle of tool allow/deny lists forming a profile.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolProfileBundle {
    /// Profile identifier (e.g., "read-only", "full-access").
    pub name: String,
    /// Tools explicitly allowed. Empty means "allow all not denied".
    #[serde(default)]
    pub allowed_tools: Vec<String>,
    /// Tools explicitly denied.
    #[serde(default)]
    pub denied_tools: Vec<String>,
    /// Optional description of the profile's purpose.
    #[serde(default)]
    pub description: Option<String>,
}

/// Tier 3: A declarative tool backed by a shell command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeclarativeTool {
    /// Tool name exposed to agents.
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// Shell command to execute.
    pub command: String,
    /// Timeout in milliseconds (default: 30000).
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    /// Working directory (relative to project root).
    #[serde(default)]
    pub working_dir: Option<String>,
    /// Environment variables to set.
    #[serde(default)]
    pub env: std::collections::HashMap<String, String>,
}

fn default_timeout() -> u64 {
    30_000
}

fn default_webhook_scope() -> String {
    "write".to_string()
}

/// Event source trigger definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TriggerDef {
    /// Cron-scheduled trigger.
    Cron {
        /// Cron expression (6-field with seconds).
        expression: String,
        /// Optional description.
        #[serde(default)]
        description: Option<String>,
    },
    /// File-watch trigger.
    FileWatch {
        /// Paths to watch.
        paths: Vec<String>,
        /// Include glob patterns (e.g. `["*.rs"]`).
        #[serde(default)]
        include: Vec<String>,
        /// Exclude glob patterns.
        #[serde(default)]
        exclude: Vec<String>,
        /// Optional description.
        #[serde(default)]
        description: Option<String>,
    },
    /// Webhook trigger.
    Webhook {
        /// Webhook endpoint path (e.g. "/hooks/my-plugin").
        path: String,
        /// Required API scope for mutating requests to this webhook endpoint.
        ///
        /// When `roko serve` mounts the route it registers this scope with the
        /// middleware whitelist via [`register_extension_route_scopes`] so the
        /// endpoint is never classified as `"write:unclassified"`. Recognised
        /// values: `"read"`, `"write"`, `"admin"`, `"agent:write"`,
        /// `"plan:write"`, `"terminal:write"`.
        ///
        /// Defaults to `"write"` if not specified — fail-closed, never open.
        #[serde(default = "default_webhook_scope")]
        scope: String,
        /// Optional secret for HMAC verification.
        #[serde(default)]
        secret: Option<String>,
        /// Optional description.
        #[serde(default)]
        description: Option<String>,
    },
    /// Signal-match trigger — fires when a signal with the given kind is observed.
    SignalMatch {
        /// Signal kind string to match (e.g. `"github:push"`, `"prd.plan_approved"`).
        signal_kind: String,
        /// Optional JSON-pointer expression applied to the signal body for additional filtering.
        ///
        /// If set, the trigger only fires when the signal body contains the given value at the
        /// given path (e.g. `"/branch" = "main"`).
        #[serde(default)]
        filter_path: Option<String>,
        /// Value that must be present at `filter_path` (as a JSON string).
        #[serde(default)]
        filter_value: Option<String>,
        /// Optional description.
        #[serde(default)]
        description: Option<String>,
    },
    /// Manual trigger — fired explicitly by a user or operator, never automatically.
    ///
    /// Declares intent that the trigger can be fired via `roko trigger fire <id>` or the
    /// HTTP control plane. No automatic evaluation is performed.
    Manual {
        /// Human-readable label shown in `roko trigger list`.
        #[serde(default)]
        label: Option<String>,
        /// Optional description.
        #[serde(default)]
        description: Option<String>,
    },
}

/// Plugin dependency.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginDependency {
    /// Name of the required plugin.
    pub name: String,
    /// Minimum version required.
    #[serde(default)]
    pub version: Option<String>,
}

// ─── Loader ─────────────────────────────────────────────────────────────

/// Load a plugin manifest from a TOML file.
///
/// # Errors
///
/// Returns an error if the file cannot be read or parsed.
pub fn load_manifest(path: &Path) -> Result<PluginManifestFile> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        RokoError::config(format!(
            "failed to read plugin manifest at {}: {e}",
            path.display()
        ))
    })?;
    parse_manifest(&content)
}

/// Parse a plugin manifest from a TOML string.
///
/// # Errors
///
/// Returns an error if the TOML is invalid or doesn't match the schema.
pub fn parse_manifest(content: &str) -> Result<PluginManifestFile> {
    let manifest: PluginManifestFile = toml::from_str(content)
        .map_err(|e| RokoError::config(format!("failed to parse plugin manifest: {e}")))?;
    validate_manifest(&manifest)?;
    Ok(manifest)
}

/// Validate a parsed manifest for consistency.
fn validate_manifest(manifest: &PluginManifestFile) -> Result<()> {
    if manifest.plugin.name.is_empty() {
        return Err(RokoError::config("plugin name must not be empty"));
    }
    if manifest.plugin.version.is_empty() {
        return Err(RokoError::config("plugin version must not be empty"));
    }

    // Validate prompt template names are unique.
    let mut prompt_names = std::collections::HashSet::new();
    for prompt in &manifest.prompts {
        if !prompt_names.insert(&prompt.name) {
            return Err(RokoError::config(format!(
                "duplicate prompt template name: `{}`",
                prompt.name
            )));
        }
    }

    // Validate profile names are unique.
    let mut profile_names = std::collections::HashSet::new();
    for profile in &manifest.profiles {
        if !profile_names.insert(&profile.name) {
            return Err(RokoError::config(format!(
                "duplicate tool profile name: `{}`",
                profile.name
            )));
        }
    }

    // Validate tool names are unique.
    let mut tool_names = std::collections::HashSet::new();
    for tool in &manifest.tools {
        if !tool_names.insert(&tool.name) {
            return Err(RokoError::config(format!(
                "duplicate tool name: `{}`",
                tool.name
            )));
        }
        if tool.command.is_empty() {
            return Err(RokoError::config(format!(
                "tool `{}` has an empty command",
                tool.name
            )));
        }
    }

    // Validate webhook trigger paths, scopes, and signal-match trigger kinds.
    for trigger in &manifest.triggers {
        match trigger {
            TriggerDef::Webhook { path, scope, .. } => {
                if path.is_empty() {
                    return Err(RokoError::config("webhook trigger path must not be empty"));
                }
                if !path.starts_with('/') {
                    return Err(RokoError::config(format!(
                        "webhook trigger path `{path}` must start with '/'"
                    )));
                }
                if !VALID_WEBHOOK_SCOPES.contains(&scope.as_str()) {
                    return Err(RokoError::config(format!(
                        "webhook trigger path `{path}` declares unknown scope `{scope}`; \
                         valid scopes are: {}",
                        VALID_WEBHOOK_SCOPES.join(", ")
                    )));
                }
            }
            TriggerDef::SignalMatch { signal_kind, .. } => {
                if signal_kind.is_empty() {
                    return Err(RokoError::config(
                        "signal_match trigger signal_kind must not be empty",
                    ));
                }
            }
            TriggerDef::Cron { .. } | TriggerDef::FileWatch { .. } | TriggerDef::Manual { .. } => {}
        }
    }

    Ok(())
}

/// Valid scope values for webhook trigger route requirements.
///
/// Must stay in sync with the `known_static_scope` function in
/// `roko-serve/src/routes/middleware.rs`.
const VALID_WEBHOOK_SCOPES: &[&str] = &[
    "read",
    "write",
    "admin",
    "agent:write",
    "plan:write",
    "terminal:write",
];

impl PluginManifestFile {
    /// Extract the route-scope pairs declared by this plugin's webhook triggers.
    ///
    /// Returns a `Vec<(path_prefix, required_scope)>` suitable for passing to
    /// `roko_serve::routes::middleware::register_extension_route_scopes`. Only
    /// [`TriggerDef::Webhook`] entries are included; cron and file-watch
    /// triggers do not register HTTP routes.
    pub fn webhook_route_scopes(&self) -> Vec<(String, String)> {
        self.triggers
            .iter()
            .filter_map(|t| match t {
                TriggerDef::Webhook { path, scope, .. } => Some((path.clone(), scope.clone())),
                _ => None,
            })
            .collect()
    }
}

/// Discovered plugin loaded from a manifest file.
#[derive(Debug, Clone)]
pub struct LoadedPlugin {
    /// The parsed manifest.
    pub manifest: PluginManifestFile,
    /// The directory containing the manifest file.
    pub base_dir: std::path::PathBuf,
}

/// Discover and load all plugin manifests in a directory.
///
/// Scans `dir` for files named `plugin.toml` (non-recursive) and files
/// matching `*.plugin.toml` in subdirectories.
pub fn discover_plugins(dir: &Path) -> Result<Vec<LoadedPlugin>> {
    let mut plugins = Vec::new();

    if !dir.exists() {
        return Ok(plugins);
    }

    // Check for plugin.toml directly in the directory.
    let direct = dir.join("plugin.toml");
    if direct.exists() {
        let manifest = load_manifest(&direct)?;
        plugins.push(LoadedPlugin {
            manifest,
            base_dir: dir.to_path_buf(),
        });
    }

    // Scan subdirectories for plugin.toml files.
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let sub_manifest = path.join("plugin.toml");
                if sub_manifest.exists() {
                    match load_manifest(&sub_manifest) {
                        Ok(manifest) => {
                            plugins.push(LoadedPlugin {
                                manifest,
                                base_dir: path,
                            });
                        }
                        Err(e) => {
                            tracing::warn!(
                                path = %sub_manifest.display(),
                                error = %e,
                                "skipping invalid plugin manifest"
                            );
                        }
                    }
                }
            }
        }
    }

    Ok(plugins)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL_MANIFEST: &str = r#"
[plugin]
name = "test-plugin"
version = "0.1.0"
"#;

    const FULL_MANIFEST: &str = r#"
[plugin]
name = "code-review"
version = "1.0.0"
description = "Automated code review plugin"
author = "Test Author"
license = "MIT"

[[prompts]]
name = "pr-review"
role = "reviewer"
template = "Review the following PR for correctness and style."
description = "Standard PR review prompt"

[[prompts]]
name = "security-review"
role = "reviewer"
template = "Focus on security vulnerabilities in this code."

[[profiles]]
name = "read-only"
allowed_tools = ["read_file", "grep", "glob"]
denied_tools = ["bash", "write_file"]
description = "Read-only access profile"

[[profiles]]
name = "full-access"
allowed_tools = []
denied_tools = []

[[tools]]
name = "lint-check"
description = "Run clippy on the workspace"
command = "cargo clippy --workspace -- -D warnings"
timeout_ms = 60000

[[tools]]
name = "test-run"
description = "Run the test suite"
command = "cargo test --workspace"

[[triggers]]
kind = "cron"
expression = "0 */5 * * * *"
description = "Every 5 minutes"

[[triggers]]
kind = "file_watch"
paths = ["src/", "tests/"]
include = ["*.rs"]

[[triggers]]
kind = "webhook"
path = "/hooks/code-review"

[[dependencies]]
name = "base-tools"
version = "0.1.0"
"#;

    #[test]
    fn parse_minimal_manifest() {
        let manifest = parse_manifest(MINIMAL_MANIFEST).unwrap();
        assert_eq!(manifest.plugin.name, "test-plugin");
        assert_eq!(manifest.plugin.version, "0.1.0");
        assert!(manifest.prompts.is_empty());
        assert!(manifest.profiles.is_empty());
        assert!(manifest.tools.is_empty());
        assert!(manifest.triggers.is_empty());
    }

    #[test]
    fn parse_full_manifest() {
        let manifest = parse_manifest(FULL_MANIFEST).unwrap();
        assert_eq!(manifest.plugin.name, "code-review");
        assert_eq!(manifest.plugin.version, "1.0.0");
        assert_eq!(
            manifest.plugin.description.as_deref(),
            Some("Automated code review plugin")
        );
        assert_eq!(manifest.plugin.author.as_deref(), Some("Test Author"));

        // Prompts
        assert_eq!(manifest.prompts.len(), 2);
        assert_eq!(manifest.prompts[0].name, "pr-review");
        assert_eq!(manifest.prompts[0].role.as_deref(), Some("reviewer"));
        assert!(
            manifest.prompts[0]
                .template
                .contains("Review the following PR")
        );

        // Profiles
        assert_eq!(manifest.profiles.len(), 2);
        assert_eq!(manifest.profiles[0].name, "read-only");
        assert_eq!(
            manifest.profiles[0].allowed_tools,
            vec!["read_file", "grep", "glob"]
        );
        assert_eq!(
            manifest.profiles[0].denied_tools,
            vec!["bash", "write_file"]
        );

        // Tools
        assert_eq!(manifest.tools.len(), 2);
        assert_eq!(manifest.tools[0].name, "lint-check");
        assert_eq!(manifest.tools[0].timeout_ms, 60000);

        // Triggers
        assert_eq!(manifest.triggers.len(), 3);
        assert!(
            matches!(&manifest.triggers[0], TriggerDef::Cron { expression, .. } if expression == "0 */5 * * * *")
        );
        assert!(
            matches!(&manifest.triggers[1], TriggerDef::FileWatch { paths, .. } if paths.len() == 2)
        );
        assert!(
            matches!(&manifest.triggers[2], TriggerDef::Webhook { path, .. } if path == "/hooks/code-review")
        );

        // Dependencies
        assert_eq!(manifest.dependencies.len(), 1);
        assert_eq!(manifest.dependencies[0].name, "base-tools");
    }

    #[test]
    fn rejects_empty_name() {
        let toml_str = r#"
[plugin]
name = ""
version = "0.1.0"
"#;
        assert!(parse_manifest(toml_str).is_err());
    }

    #[test]
    fn rejects_empty_version() {
        let toml_str = r#"
[plugin]
name = "test"
version = ""
"#;
        assert!(parse_manifest(toml_str).is_err());
    }

    #[test]
    fn rejects_duplicate_prompt_names() {
        let toml_str = r#"
[plugin]
name = "test"
version = "0.1.0"

[[prompts]]
name = "dup"
template = "first"

[[prompts]]
name = "dup"
template = "second"
"#;
        assert!(parse_manifest(toml_str).is_err());
    }

    #[test]
    fn rejects_duplicate_tool_names() {
        let toml_str = r#"
[plugin]
name = "test"
version = "0.1.0"

[[tools]]
name = "dup"
description = "first"
command = "echo 1"

[[tools]]
name = "dup"
description = "second"
command = "echo 2"
"#;
        assert!(parse_manifest(toml_str).is_err());
    }

    #[test]
    fn rejects_tool_with_empty_command() {
        let toml_str = r#"
[plugin]
name = "test"
version = "0.1.0"

[[tools]]
name = "broken"
description = "no command"
command = ""
"#;
        assert!(parse_manifest(toml_str).is_err());
    }

    #[test]
    fn default_timeout_is_30s() {
        let toml_str = r#"
[plugin]
name = "test"
version = "0.1.0"

[[tools]]
name = "quick"
description = "quick tool"
command = "echo hello"
"#;
        let manifest = parse_manifest(toml_str).unwrap();
        assert_eq!(manifest.tools[0].timeout_ms, 30_000);
    }

    #[test]
    fn discover_plugins_returns_empty_for_missing_dir() {
        let dir = std::path::Path::new("/nonexistent/path/that/does/not/exist");
        let plugins = discover_plugins(dir).unwrap();
        assert!(plugins.is_empty());
    }

    #[test]
    fn roundtrip_serialization() {
        let manifest = parse_manifest(FULL_MANIFEST).unwrap();
        let serialized = toml::to_string_pretty(&manifest).unwrap();
        let reparsed = parse_manifest(&serialized).unwrap();
        assert_eq!(manifest, reparsed);
    }

    #[test]
    fn parse_signal_match_trigger() {
        let toml_str = r#"
[plugin]
name = "test"
version = "0.1.0"

[[triggers]]
kind = "signal_match"
signal_kind = "github:push"
filter_path = "/branch"
filter_value = "main"
description = "Fire on main pushes"
"#;
        let manifest = parse_manifest(toml_str).unwrap();
        assert_eq!(manifest.triggers.len(), 1);
        assert!(matches!(
            &manifest.triggers[0],
            TriggerDef::SignalMatch {
                signal_kind,
                filter_path,
                filter_value,
                ..
            }
            if signal_kind == "github:push"
                && filter_path.as_deref() == Some("/branch")
                && filter_value.as_deref() == Some("main")
        ));
    }

    #[test]
    fn parse_manual_trigger() {
        let toml_str = r#"
[plugin]
name = "test"
version = "0.1.0"

[[triggers]]
kind = "manual"
label = "Deploy production"
description = "Only fire this manually"
"#;
        let manifest = parse_manifest(toml_str).unwrap();
        assert_eq!(manifest.triggers.len(), 1);
        assert!(matches!(
            &manifest.triggers[0],
            TriggerDef::Manual { label, .. }
            if label.as_deref() == Some("Deploy production")
        ));
    }

    #[test]
    fn rejects_signal_match_with_empty_signal_kind() {
        let toml_str = r#"
[plugin]
name = "test"
version = "0.1.0"

[[triggers]]
kind = "signal_match"
signal_kind = ""
"#;
        assert!(parse_manifest(toml_str).is_err());
    }

    #[test]
    fn signal_match_trigger_roundtrips() {
        let toml_str = r#"
[plugin]
name = "test"
version = "0.1.0"

[[triggers]]
kind = "signal_match"
signal_kind = "prd.plan_approved"
"#;
        let manifest = parse_manifest(toml_str).unwrap();
        let serialized = toml::to_string_pretty(&manifest).unwrap();
        let reparsed = parse_manifest(&serialized).unwrap();
        assert_eq!(manifest, reparsed);
    }

    #[test]
    fn manual_trigger_roundtrips() {
        let toml_str = r#"
[plugin]
name = "test"
version = "0.1.0"

[[triggers]]
kind = "manual"
label = "Run backfill"
"#;
        let manifest = parse_manifest(toml_str).unwrap();
        let serialized = toml::to_string_pretty(&manifest).unwrap();
        let reparsed = parse_manifest(&serialized).unwrap();
        assert_eq!(manifest, reparsed);
    }
}
