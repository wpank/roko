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

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Component, Path};

pub use roko_core::plugin::{PluginCapability, PluginTier};
use roko_core::{Result, RokoError};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};

// ─── TOML schema ────────────────────────────────────────────────────────

/// Top-level TOML manifest that a plugin author writes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PluginManifestFile {
    /// Explicit SDK tier. When absent, [`PluginManifestFile::tier`] infers it
    /// from the manifest contents.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tier: Option<PluginTier>,
    /// Capabilities required by this plugin. When absent, the minimum required
    /// by the manifest contents is inferred.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<PluginCapability>,
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
    /// Default sandbox configuration for declarative tools.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sandbox: Option<SandboxConfig>,
}

/// Plugin metadata section.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
    /// Legacy location for the SDK tier. New manifests should declare `tier`
    /// at the top level. Retained to deserialize manifests produced by the
    /// earlier schema without duplicating the tier type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tier: Option<PluginTier>,
    /// Whether the plugin is enabled (wired into the runtime).
    /// Defaults to `true`.
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

/// Sandbox constraints for a plugin-declared tool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SandboxConfig {
    /// Worktree-relative glob patterns the tool may access.
    #[serde(default)]
    pub allowed_paths: Vec<String>,
    /// Environment variable names the tool may read.
    #[serde(default)]
    pub env_allowlist: Vec<String>,
    /// Maximum combined stdout/stderr bytes retained from the tool.
    #[serde(default = "default_max_output_bytes")]
    pub max_output_bytes: u64,
    /// Whether shell metacharacters are explicitly permitted in `command`.
    #[serde(default)]
    pub allow_shell_metacharacters: bool,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            allowed_paths: Vec::new(),
            env_allowlist: Vec::new(),
            max_output_bytes: default_max_output_bytes(),
            allow_shell_metacharacters: false,
        }
    }
}

const fn default_max_output_bytes() -> u64 {
    1024 * 1024
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
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
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
    /// Tool-specific sandbox override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sandbox: Option<SandboxConfig>,
}

fn default_timeout() -> u64 {
    30_000
}

fn default_webhook_scope() -> String {
    "write".to_string()
}

/// Event source trigger definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
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

    validate_privilege_declarations(manifest)?;

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
        validate_declarative_tool(manifest, tool)?;
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

fn validate_privilege_declarations(manifest: &PluginManifestFile) -> Result<()> {
    if let (Some(tier), Some(legacy_tier)) = (manifest.tier, manifest.plugin.tier)
        && tier != legacy_tier
    {
        return Err(RokoError::config(format!(
            "plugin declares conflicting tiers `{}` and `{}`",
            tier.label(),
            legacy_tier.label()
        )));
    }

    let tier = manifest.tier();
    let capabilities = manifest.capabilities();
    let denied = capabilities.denied_by(tier);
    if !denied.is_empty() {
        return Err(RokoError::config(format!(
            "plugin tier `{}` does not permit declared capabilities: {}",
            tier.label(),
            denied.join(", ")
        )));
    }
    if !manifest.tools.is_empty() && !capabilities.exec {
        return Err(RokoError::config(
            "plugins with declarative tools must declare the `exec` capability",
        ));
    }
    if let Some(sandbox) = &manifest.sandbox {
        validate_sandbox("plugin", sandbox)?;
    }
    Ok(())
}

fn validate_declarative_tool(manifest: &PluginManifestFile, tool: &DeclarativeTool) -> Result<()> {
    let sandbox = tool.sandbox.as_ref().or(manifest.sandbox.as_ref());
    if let Some(sandbox) = &tool.sandbox {
        validate_sandbox(&format!("tool `{}`", tool.name), sandbox)?;
    }
    if let Some(working_dir) = tool.working_dir.as_deref() {
        validate_relative_path(&format!("tool `{}` working_dir", tool.name), working_dir)?;
        if sandbox.is_none_or(|config| config.allowed_paths.is_empty()) {
            return Err(RokoError::config(format!(
                "tool `{}` sets working_dir but its sandbox allowed_paths is empty",
                tool.name
            )));
        }
    }
    if !sandbox.is_some_and(|config| config.allow_shell_metacharacters)
        && let Some(found) = shell_metacharacter(&tool.command)
    {
        return Err(RokoError::config(format!(
            "tool `{}` command contains shell metacharacter `{found}`; set \
             allow_shell_metacharacters = true in its sandbox to permit it",
            tool.name
        )));
    }
    Ok(())
}

fn validate_sandbox(owner: &str, sandbox: &SandboxConfig) -> Result<()> {
    let mut paths = std::collections::HashSet::new();
    for allowed_path in &sandbox.allowed_paths {
        validate_relative_path(&format!("{owner} sandbox allowed_paths"), allowed_path)?;
        if !paths.insert(allowed_path) {
            return Err(RokoError::config(format!(
                "{owner} sandbox contains duplicate allowed path `{allowed_path}`"
            )));
        }
    }

    let mut env_names = std::collections::HashSet::new();
    for name in &sandbox.env_allowlist {
        if !valid_env_name(name) {
            return Err(RokoError::config(format!(
                "{owner} sandbox env_allowlist contains invalid variable name `{name}`"
            )));
        }
        if !env_names.insert(name) {
            return Err(RokoError::config(format!(
                "{owner} sandbox contains duplicate env variable `{name}`"
            )));
        }
    }
    Ok(())
}

fn validate_relative_path(owner: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(RokoError::config(format!("{owner} must not be empty")));
    }
    let path = Path::new(value);
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(RokoError::config(format!(
            "{owner} `{value}` must be worktree-relative and must not contain parent traversal"
        )));
    }
    Ok(())
}

fn valid_env_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|first| first == '_' || first.is_ascii_alphabetic())
        && chars.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

fn shell_metacharacter(command: &str) -> Option<&'static str> {
    ["$(", "`", "|", ";"]
        .into_iter()
        .find(|candidate| command.contains(candidate))
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

/// Compatibility summary used by plugin status surfaces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginSandboxSummary {
    /// Whether the effective tier and declaration permit network egress.
    pub network: bool,
    /// Effective worktree-relative path patterns.
    pub allowed_paths: Vec<String>,
    /// Reserved compatibility field; manifest sandboxes are allowlist-only.
    pub denied_paths: Vec<String>,
    /// Whether the effective tier and declaration permit secret access.
    pub secrets: bool,
}

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

    /// Compute the effective sandbox summary for display.
    #[must_use]
    pub fn effective_sandbox(&self) -> PluginSandboxSummary {
        let tier = self.tier();
        let capabilities = self.capabilities();
        PluginSandboxSummary {
            network: tier.allows_network() && capabilities.network_egress,
            allowed_paths: self
                .sandbox
                .as_ref()
                .map_or_else(Vec::new, |sandbox| sandbox.allowed_paths.clone()),
            denied_paths: Vec::new(),
            secrets: tier.allows_secrets() && capabilities.secrets,
        }
    }

    /// Effective tier, using an explicit declaration or content inference.
    #[must_use]
    pub fn tier(&self) -> PluginTier {
        self.tier.or(self.plugin.tier).unwrap_or({
            if !self.tools.is_empty() {
                PluginTier::Standard
            } else if !self.profiles.is_empty() {
                PluginTier::Sandboxed
            } else if !self.prompts.is_empty() {
                PluginTier::Untrusted
            } else {
                PluginTier::Sandboxed
            }
        })
    }

    /// Effective capability requirement, inferred for legacy manifests.
    #[must_use]
    pub fn capabilities(&self) -> PluginCapability {
        self.capabilities.unwrap_or_else(|| {
            if self.tools.is_empty() {
                PluginCapability::default()
            } else {
                PluginCapability::declarative_tools()
            }
        })
    }

    /// Sandbox effective for one tool: its override, then the plugin default,
    /// then the restrictive schema default.
    #[must_use]
    pub fn sandbox_for_tool(&self, tool: &DeclarativeTool) -> SandboxConfig {
        tool.sandbox
            .as_ref()
            .or(self.sandbox.as_ref())
            .cloned()
            .unwrap_or_default()
    }

    /// Whether this plugin is wired (enabled) at runtime.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.plugin.enabled
    }

    /// Wire status label: `"active"` or `"disabled"`.
    #[must_use]
    pub fn wire_status(&self) -> &'static str {
        if self.plugin.enabled {
            "active"
        } else {
            "disabled"
        }
    }

    /// Total number of tools declared by this plugin.
    #[must_use]
    pub fn tool_count(&self) -> usize {
        self.tools.len()
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

/// Resolve discovered plugins into one deterministic runtime load order.
///
/// Duplicate names are collapsed to the highest semantic version. Equal
/// versions retain the first discovered path, so callers control same-version
/// root precedence through input order. Required dependencies must exist;
/// dependency constraints are audited and warned when unsatisfied, matching
/// the initial E32 compatibility policy. Dependencies always precede their
/// dependents in the returned order.
///
/// # Errors
///
/// Returns an error for invalid plugin versions, missing required
/// dependencies, or dependency cycles.
#[allow(clippy::too_many_lines)] // Selection, validation, and ordering form one graph transaction.
pub fn resolve_plugins(discovered: Vec<LoadedPlugin>) -> Result<Vec<LoadedPlugin>> {
    let mut selected = BTreeMap::<String, LoadedPlugin>::new();

    for candidate in discovered {
        let name = candidate.manifest.plugin.name.clone();
        let candidate_version = parse_plugin_version(&candidate)?;
        match selected.get(&name) {
            None => {
                selected.insert(name, candidate);
            }
            Some(current) => {
                let current_version = parse_plugin_version(current)?;
                if candidate_version > current_version {
                    tracing::warn!(
                        plugin = %name,
                        selected_version = %candidate.manifest.plugin.version,
                        selected_path = %candidate.base_dir.display(),
                        skipped_version = %current.manifest.plugin.version,
                        skipped_path = %current.base_dir.display(),
                        "plugin version conflict resolved to highest version"
                    );
                    selected.insert(name, candidate);
                } else {
                    tracing::warn!(
                        plugin = %name,
                        selected_version = %current.manifest.plugin.version,
                        selected_path = %current.base_dir.display(),
                        skipped_version = %candidate.manifest.plugin.version,
                        skipped_path = %candidate.base_dir.display(),
                        "plugin version conflict resolved to highest version"
                    );
                }
            }
        }
    }

    let mut in_degree = selected
        .keys()
        .map(|name| (name.clone(), 0_usize))
        .collect::<HashMap<_, _>>();
    let mut dependents = selected
        .keys()
        .map(|name| (name.clone(), Vec::<String>::new()))
        .collect::<HashMap<_, _>>();

    for (name, plugin) in &selected {
        for dependency in &plugin.manifest.dependencies {
            let Some(resolved_dependency) = selected.get(&dependency.name) else {
                return Err(RokoError::invalid(format!(
                    "plugin '{name}' depends on missing plugin '{}'",
                    dependency.name
                )));
            };

            if let Some(constraint) = dependency.version.as_deref() {
                let actual = parse_plugin_version(resolved_dependency)?;
                if !plugin_version_satisfies(&actual, constraint) {
                    tracing::warn!(
                        plugin = %name,
                        dependency = %dependency.name,
                        required = %constraint,
                        resolved = %resolved_dependency.manifest.plugin.version,
                        "resolved plugin dependency does not satisfy requested version"
                    );
                }
            }

            let Some(degree) = in_degree.get_mut(name) else {
                return Err(RokoError::invalid(format!(
                    "internal plugin resolution error: missing in-degree for '{name}'"
                )));
            };
            *degree += 1;

            let Some(downstream) = dependents.get_mut(&dependency.name) else {
                return Err(RokoError::invalid(format!(
                    "internal plugin resolution error: missing dependency node '{}'",
                    dependency.name
                )));
            };
            downstream.push(name.clone());
        }
    }

    for names in dependents.values_mut() {
        names.sort();
    }

    let mut ready = in_degree
        .iter()
        .filter_map(|(name, degree)| (*degree == 0).then_some(name.clone()))
        .collect::<BTreeSet<_>>();
    let mut ordered_names = Vec::with_capacity(selected.len());

    while let Some(name) = ready.pop_first() {
        ordered_names.push(name.clone());
        if let Some(downstream) = dependents.get(&name) {
            for dependent in downstream {
                let Some(degree) = in_degree.get_mut(dependent) else {
                    return Err(RokoError::invalid(format!(
                        "internal plugin resolution error: missing dependent node '{dependent}'"
                    )));
                };
                *degree = degree.saturating_sub(1);
                if *degree == 0 {
                    ready.insert(dependent.clone());
                }
            }
        }
    }

    if ordered_names.len() != selected.len() {
        let cyclic = in_degree
            .into_iter()
            .filter_map(|(name, degree)| (degree > 0).then_some(name))
            .collect::<BTreeSet<_>>();
        return Err(RokoError::invalid(format!(
            "plugin dependency cycle detected among: {}",
            cyclic.into_iter().collect::<Vec<_>>().join(", ")
        )));
    }

    let mut resolved = Vec::with_capacity(ordered_names.len());
    for name in ordered_names {
        let Some(plugin) = selected.remove(&name) else {
            return Err(RokoError::invalid(format!(
                "internal plugin resolution error: selected plugin '{name}' disappeared"
            )));
        };
        resolved.push(plugin);
    }
    Ok(resolved)
}

fn parse_plugin_version(plugin: &LoadedPlugin) -> Result<Version> {
    Version::parse(&plugin.manifest.plugin.version).map_err(|error| {
        RokoError::invalid(format!(
            "plugin '{}' at {} has invalid semantic version '{}': {error}",
            plugin.manifest.plugin.name,
            plugin.base_dir.display(),
            plugin.manifest.plugin.version
        ))
    })
}

fn plugin_version_satisfies(actual: &Version, constraint: &str) -> bool {
    let constraint = constraint.trim();
    if constraint.is_empty() {
        return true;
    }

    if constraint
        .chars()
        .any(|character| matches!(character, '<' | '>' | '=' | '^' | '~' | '*' | ','))
    {
        return VersionReq::parse(constraint).is_ok_and(|requirement| requirement.matches(actual));
    }

    Version::parse(constraint).is_ok_and(|minimum| actual >= &minimum)
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

    fn loaded_plugin(
        name: &str,
        version: &str,
        dependencies: &[(&str, Option<&str>)],
        base_dir: &str,
    ) -> LoadedPlugin {
        let mut manifest = parse_manifest(&format!(
            "[plugin]\nname = \"{name}\"\nversion = \"{version}\"\n"
        ))
        .unwrap();
        manifest.dependencies = dependencies
            .iter()
            .map(|(dependency, version)| PluginDependency {
                name: (*dependency).to_string(),
                version: version.map(str::to_string),
            })
            .collect();
        LoadedPlugin {
            manifest,
            base_dir: base_dir.into(),
        }
    }

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
    fn infers_tiers_and_tool_capability_for_legacy_manifests() {
        let prompt = parse_manifest(
            r#"
[plugin]
name = "prompt-plugin"
version = "1.0.0"

[[prompts]]
name = "review"
template = "Review this"
"#,
        )
        .expect("prompt manifest parses");
        assert_eq!(prompt.tier(), PluginTier::Untrusted);

        let profile = parse_manifest(
            r#"
[plugin]
name = "profile-plugin"
version = "1.0.0"

[[profiles]]
name = "readonly"
"#,
        )
        .expect("profile manifest parses");
        assert_eq!(profile.tier(), PluginTier::Sandboxed);

        let tools = parse_manifest(
            r#"
[plugin]
name = "tool-plugin"
version = "1.0.0"

[[tools]]
name = "check"
description = "Run a check"
command = "cargo check"
"#,
        )
        .expect("tool manifest parses");
        assert_eq!(tools.tier(), PluginTier::Standard);
        assert!(tools.capabilities().exec);

        let legacy = parse_manifest(
            r#"
[plugin]
name = "legacy-plugin"
version = "1.0.0"
tier = "trusted"
"#,
        )
        .expect("legacy nested tier parses");
        assert_eq!(legacy.tier(), PluginTier::Trusted);
    }

    #[test]
    fn explicit_tier_and_capabilities_roundtrip() {
        let manifest = parse_manifest(
            r#"
tier = "trusted"

[capabilities]
filesystem_read = true
filesystem_write = true
network_egress = true
secrets = true
exec = true

[plugin]
name = "trusted-plugin"
version = "1.0.0"
"#,
        )
        .expect("explicit declarations parse");
        assert_eq!(manifest.tier(), PluginTier::Trusted);
        assert_eq!(
            manifest.capabilities(),
            PluginCapability {
                filesystem_read: true,
                filesystem_write: true,
                network_egress: true,
                secrets: true,
                exec: true,
            }
        );

        let serialized = toml::to_string_pretty(&manifest).expect("manifest serializes");
        let reparsed = parse_manifest(&serialized).expect("serialized manifest reparses");
        assert_eq!(manifest, reparsed);
    }

    #[test]
    fn rejects_capabilities_outside_effective_tier() {
        let error = parse_manifest(
            r#"
tier = "sandboxed"

[capabilities]
network_egress = true

[plugin]
name = "overprivileged"
version = "1.0.0"
"#,
        )
        .expect_err("sandboxed network capability must be rejected");
        assert!(error.to_string().contains("network_egress"));

        let missing_exec = parse_manifest(
            r#"
tier = "standard"

[capabilities]
filesystem_read = true

[plugin]
name = "missing-exec"
version = "1.0.0"

[[tools]]
name = "check"
description = "Run a check"
command = "cargo check"
"#,
        )
        .expect_err("declarative tools must require exec");
        assert!(missing_exec.to_string().contains("`exec` capability"));
    }

    #[test]
    fn sandbox_defaults_and_tool_override_deserialize() {
        let manifest = parse_manifest(
            r#"
tier = "standard"

[plugin]
name = "sandboxed-tool"
version = "1.0.0"

[[tools]]
name = "build"
description = "Build the project"
command = "cargo build | tee build.log"
working_dir = "crates/roko-plugin"

[tools.sandbox]
allowed_paths = ["crates/roko-plugin/**"]
env_allowlist = ["RUST_LOG"]
allow_shell_metacharacters = true
"#,
        )
        .expect("valid tool sandbox parses");
        let sandbox = manifest.sandbox_for_tool(&manifest.tools[0]);
        assert_eq!(sandbox.max_output_bytes, 1024 * 1024);
        assert_eq!(sandbox.allowed_paths, vec!["crates/roko-plugin/**"]);
        assert_eq!(sandbox.env_allowlist, vec!["RUST_LOG"]);
        assert!(sandbox.allow_shell_metacharacters);
    }

    #[test]
    fn sandbox_validation_fails_closed() {
        let invalid_manifests = [
            (
                "absolute working directory",
                r#"
[plugin]
name = "absolute"
version = "1.0.0"

[[tools]]
name = "check"
description = "Run a check"
command = "cargo check"
working_dir = "/tmp"

[tools.sandbox]
allowed_paths = ["tmp/**"]
"#,
            ),
            (
                "working directory without an allowlist",
                r#"
[plugin]
name = "no-allowlist"
version = "1.0.0"

[[tools]]
name = "check"
description = "Run a check"
command = "cargo check"
working_dir = "crates/roko-plugin"
"#,
            ),
            (
                "shell metacharacter without opt-in",
                r#"
[plugin]
name = "shell-meta"
version = "1.0.0"

[[tools]]
name = "check"
description = "Run checks"
command = "cargo check; cargo test"
"#,
            ),
            (
                "path traversal",
                r#"
[plugin]
name = "traversal"
version = "1.0.0"

[sandbox]
allowed_paths = ["../secrets/**"]
"#,
            ),
            (
                "invalid environment name",
                r#"
[plugin]
name = "bad-env"
version = "1.0.0"

[sandbox]
env_allowlist = ["BAD-NAME"]
"#,
            ),
        ];

        for (case, content) in invalid_manifests {
            assert!(parse_manifest(content).is_err(), "accepted {case}");
        }
    }

    #[test]
    fn security_sensitive_manifest_sections_reject_unknown_fields() {
        let invalid_manifests = [
            (
                "misspelled top-level capabilities section",
                r#"
capabilties = { exec = true }

[plugin]
name = "bad-top-level"
version = "1.0.0"
"#,
            ),
            (
                "unknown capability",
                r#"
[capabilities]
execution = true

[plugin]
name = "bad-capability"
version = "1.0.0"
"#,
            ),
            (
                "unknown sandbox control",
                r#"
[plugin]
name = "bad-sandbox"
version = "1.0.0"

[sandbox]
allow_shell = true
"#,
            ),
            (
                "misspelled enabled flag",
                r#"
[plugin]
name = "bad-enabled"
version = "1.0.0"
enable = false
"#,
            ),
            (
                "misplaced tool output limit",
                r#"
[plugin]
name = "bad-tool"
version = "1.0.0"

[[tools]]
name = "check"
description = "Run check"
command = "printf ok"
max_output_bytes = 1
"#,
            ),
            (
                "misspelled profile denylist",
                r#"
[plugin]
name = "bad-profile"
version = "1.0.0"

[[profiles]]
name = "restricted"
denied_tool = ["bash"]
"#,
            ),
            (
                "misspelled webhook scope",
                r#"
[plugin]
name = "bad-webhook"
version = "1.0.0"

[[triggers]]
kind = "webhook"
path = "/hook"
scop = "read"
"#,
            ),
            (
                "misspelled dependency version",
                r#"
[plugin]
name = "bad-dependency"
version = "1.0.0"

[[dependencies]]
name = "required-plugin"
verison = ">=2.0.0"
"#,
            ),
        ];

        for (case, content) in invalid_manifests {
            let error = parse_manifest(content).expect_err(case);
            assert!(
                error.to_string().contains("unknown field"),
                "unexpected error for {case}: {error}"
            );
        }
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

    #[test]
    fn resolve_plugins_selects_highest_version_and_dependency_order() {
        let resolved = resolve_plugins(vec![
            loaded_plugin(
                "consumer",
                "1.0.0",
                &[("shared", Some(">=2.0.0"))],
                "/consumer",
            ),
            loaded_plugin("shared", "1.9.0", &[], "/workspace/shared"),
            loaded_plugin("shared", "2.1.0", &[], "/installed/shared"),
            loaded_plugin("independent", "1.0.0", &[], "/independent"),
        ])
        .unwrap();

        let names = resolved
            .iter()
            .map(|plugin| plugin.manifest.plugin.name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["independent", "shared", "consumer"]);
        assert_eq!(resolved[1].manifest.plugin.version, "2.1.0");
        assert_eq!(
            resolved[1].base_dir,
            std::path::PathBuf::from("/installed/shared")
        );
    }

    #[test]
    fn resolve_plugins_preserves_first_root_for_equal_versions() {
        let resolved = resolve_plugins(vec![
            loaded_plugin("same", "1.0.0", &[], "/preferred"),
            loaded_plugin("same", "1.0.0", &[], "/later"),
        ])
        .unwrap();

        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].base_dir, std::path::PathBuf::from("/preferred"));
    }

    #[test]
    fn resolve_plugins_allows_version_mismatch_but_preserves_dependency_order() {
        let resolved = resolve_plugins(vec![
            loaded_plugin(
                "consumer",
                "1.0.0",
                &[("shared", Some(">=3.0.0"))],
                "/consumer",
            ),
            loaded_plugin("shared", "2.1.0", &[], "/shared"),
        ])
        .unwrap();

        let names = resolved
            .iter()
            .map(|plugin| plugin.manifest.plugin.name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["shared", "consumer"]);
    }

    #[test]
    fn resolve_plugins_rejects_missing_dependencies_and_cycles() {
        let missing = resolve_plugins(vec![loaded_plugin(
            "consumer",
            "1.0.0",
            &[("missing", None)],
            "/consumer",
        )])
        .unwrap_err();
        assert!(missing.to_string().contains("missing plugin 'missing'"));

        let cycle = resolve_plugins(vec![
            loaded_plugin("a", "1.0.0", &[("b", None)], "/a"),
            loaded_plugin("b", "1.0.0", &[("a", None)], "/b"),
        ])
        .unwrap_err();
        assert!(cycle.to_string().contains("dependency cycle"));
    }

    #[test]
    fn resolve_plugins_rejects_invalid_semantic_versions() {
        let error = resolve_plugins(vec![loaded_plugin("bad", "latest", &[], "/bad")]).unwrap_err();
        assert!(error.to_string().contains("invalid semantic version"));
    }
}
