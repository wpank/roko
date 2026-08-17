//! Dynamic tool registry for the plugin ecosystem.
//!
//! [`DynamicToolRegistry`] merges statically-registered tools with tools
//! declared by plugins at runtime. It supports:
//!
//! - Registration and unregistration of [`RegisteredTool`] entries.
//! - Lookup by name (latest version) or by `name@version` selector.
//! - Multiple versions of the same tool tracked per-entry.
//!
//! # Tier
//!
//! [`PluginTier`] is the canonical dependency-safe SDK type from `roko-core`.
//!
//! # Version resolution
//!
//! Tools are stored in a `Vec` (matching [`StaticToolRegistry`]'s linear-scan
//! pattern — the set is small enough that a hashmap would add overhead without
//! measurable benefit). Multiple versions of a tool with the same name are
//! allowed; [`DynamicToolRegistry::resolve_tool`] picks the latest semver when
//! no version is specified.
//!
//! # Example
//!
//! ```rust
//! use roko_plugin::tool_registry::{DynamicToolRegistry, RegisteredTool, PluginTier};
//! use serde_json::json;
//!
//! let mut registry = DynamicToolRegistry::new();
//! let tool = RegisteredTool {
//!     name: "my-plugin.lint".to_string(),
//!     version: "1.0.0".to_string(),
//!     description: "Run linter".to_string(),
//!     tier: PluginTier::Standard,
//!     schema: json!({"type": "object", "properties": {}}),
//!     source_plugin: Some("my-plugin".to_string()),
//! };
//! registry.register_tool("my-plugin.lint@1.0.0", tool).unwrap();
//! assert!(registry.get_tool("my-plugin.lint").is_some());
//! ```

use std::collections::HashMap;

use roko_core::error::{Result, RokoError};
pub use roko_core::plugin::PluginTier;
use serde::{Deserialize, Serialize};

// ─── RegisteredTool ───────────────────────────────────────────────────────

/// A tool entry held in [`DynamicToolRegistry`].
///
/// Carries everything needed to validate, display, and route calls to a
/// plugin-declared tool: its name, version, description, trust tier, JSON
/// Schema for inputs, and the plugin that declared it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegisteredTool {
    /// Canonical tool name (snake_case or `plugin.tool` namespaced form).
    pub name: String,
    /// Semantic version string (e.g. `"1.0.0"`).
    pub version: String,
    /// Human-readable description sent to the LLM.
    pub description: String,
    /// Trust tier of the plugin that declared this tool.
    pub tier: PluginTier,
    /// JSON Schema for the tool's input arguments.
    pub schema: serde_json::Value,
    /// Name of the plugin that registered this tool, if any.
    pub source_plugin: Option<String>,
}

impl RegisteredTool {
    /// Return the storage key used by [`DynamicToolRegistry`] for this tool.
    ///
    /// The key is `name@version`, e.g. `"my-plugin.lint@1.0.0"`.
    #[must_use]
    pub fn storage_key(&self) -> String {
        format!("{}@{}", self.name, self.version)
    }
}

// ─── Version helpers ──────────────────────────────────────────────────────

/// Parse a semver string `"major.minor.patch"` into a comparable tuple.
///
/// Non-conforming strings fall back to `(0, 0, 0)` so they sort last.
fn parse_version(v: &str) -> (u64, u64, u64) {
    let mut parts = v.splitn(3, '.');
    let major = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let minor = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let patch = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    (major, minor, patch)
}

// ─── DynamicToolRegistry ──────────────────────────────────────────────────

/// Runtime registry that merges static builtins with plugin-declared tools.
///
/// Tools are stored as `key → RegisteredTool` where `key` is
/// `"name@version"`. Multiple versions of the same tool name can coexist;
/// [`resolve_tool`](Self::resolve_tool) picks the latest semver when no
/// version constraint is specified.
///
/// Deduplication policy: if a tool with the same `name@version` key is
/// registered twice, the second registration is rejected with an error.
/// To replace a tool, [`unregister_tool`](Self::unregister_tool) it first.
#[derive(Debug, Clone, Default)]
pub struct DynamicToolRegistry {
    /// `"name@version"` → [`RegisteredTool`].
    tools: HashMap<String, RegisteredTool>,
}

impl DynamicToolRegistry {
    /// Create an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Register a tool under the given key.
    ///
    /// The `key` must be either `"name"` (version inferred from `tool.version`)
    /// or `"name@version"`. Using a bare name is equivalent to
    /// `"name@{tool.version}"`.
    ///
    /// # Errors
    ///
    /// Returns [`RokoError::invalid`] if a tool with the same `name@version`
    /// key is already present.
    pub fn register_tool(&mut self, key: &str, tool: RegisteredTool) -> Result<()> {
        let storage_key = if key.contains('@') {
            key.to_string()
        } else {
            // Bare name — append the tool's own version.
            format!("{}@{}", key, tool.version)
        };

        if self.tools.contains_key(&storage_key) {
            return Err(RokoError::invalid(format!(
                "tool '{storage_key}' is already registered; unregister it first"
            )));
        }

        self.tools.insert(storage_key, tool);
        Ok(())
    }

    /// Remove all versions of a tool that match the given key.
    ///
    /// If `key` contains `@` it removes exactly that `name@version` entry.
    /// Otherwise it removes *all* versions of tools with that name.
    ///
    /// # Errors
    ///
    /// Returns [`RokoError::invalid`] if no matching tool is found.
    pub fn unregister_tool(&mut self, key: &str) -> Result<()> {
        if key.contains('@') {
            // Exact name@version removal.
            if self.tools.remove(key).is_none() {
                return Err(RokoError::invalid(format!(
                    "no tool registered under key '{key}'"
                )));
            }
        } else {
            // Remove all versions of this tool name.
            let prefix = format!("{key}@");
            let before = self.tools.len();
            self.tools.retain(|k, _| !k.starts_with(&prefix));
            if self.tools.len() == before {
                return Err(RokoError::invalid(format!(
                    "no tool named '{key}' found in registry"
                )));
            }
        }
        Ok(())
    }

    /// Look up a tool by name, returning the latest registered version.
    ///
    /// Returns `None` if no tool with that name is registered.
    #[must_use]
    pub fn get_tool(&self, name: &str) -> Option<&RegisteredTool> {
        self.resolve_tool(name, None)
    }

    /// Iterate over all registered tools (all versions).
    ///
    /// Returns `(key, tool)` pairs in unspecified order.
    #[must_use]
    pub fn list_tools(&self) -> Vec<(&str, &RegisteredTool)> {
        self.tools.iter().map(|(k, v)| (k.as_str(), v)).collect()
    }

    // ── T02: version resolution ──────────────────────────────────────────

    /// Resolve a tool by name and optional version constraint.
    ///
    /// - `name` only → returns the highest semver version of tools with that name.
    /// - `name@version` in `name` or `version = Some("1.2.3")` → exact lookup.
    ///
    /// Returns `None` if no matching tool exists.
    #[must_use]
    pub fn resolve_tool(&self, name: &str, version: Option<&str>) -> Option<&RegisteredTool> {
        // Split name@version selector if embedded in `name`.
        let (tool_name, pinned_version) = if let Some(at) = name.find('@') {
            let (n, v) = name.split_at(at);
            (n, Some(&v[1..]))
        } else {
            (name, version)
        };

        if let Some(ver) = pinned_version {
            // Exact version lookup.
            let key = format!("{tool_name}@{ver}");
            return self.tools.get(&key);
        }

        // Latest-version lookup: find all entries for `tool_name`, pick max.
        let prefix = format!("{tool_name}@");
        self.tools
            .iter()
            .filter(|(k, _)| k.starts_with(&prefix))
            .max_by_key(|(_, tool)| parse_version(&tool.version))
            .map(|(_, tool)| tool)
    }

    /// Number of registered tool entries (counting each version separately).
    #[must_use]
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Returns `true` if no tools are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    /// Return all unique tool names (without version suffixes).
    #[must_use]
    pub fn tool_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .tools
            .values()
            .map(|t| t.name.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        names.sort();
        names
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_tool(name: &str, version: &str, tier: PluginTier) -> RegisteredTool {
        RegisteredTool {
            name: name.to_string(),
            version: version.to_string(),
            description: format!("{name} v{version}"),
            tier,
            schema: json!({"type": "object", "properties": {}}),
            source_plugin: Some("test-plugin".to_string()),
        }
    }

    // ── PluginTier tests ─────────────────────────────────────────────────

    #[test]
    fn plugin_tier_default_is_sandboxed() {
        assert_eq!(PluginTier::default(), PluginTier::Sandboxed);
    }

    #[test]
    fn plugin_tier_ordering_ascending() {
        assert!(PluginTier::Untrusted < PluginTier::Sandboxed);
        assert!(PluginTier::Sandboxed < PluginTier::Standard);
        assert!(PluginTier::Standard < PluginTier::Trusted);
        assert!(PluginTier::Trusted < PluginTier::Kernel);
    }

    #[test]
    fn plugin_tier_labels() {
        assert_eq!(PluginTier::Untrusted.label(), "untrusted");
        assert_eq!(PluginTier::Sandboxed.label(), "sandboxed");
        assert_eq!(PluginTier::Standard.label(), "standard");
        assert_eq!(PluginTier::Trusted.label(), "trusted");
        assert_eq!(PluginTier::Kernel.label(), "kernel");
    }

    #[test]
    fn plugin_tier_serde_roundtrip() {
        for tier in [
            PluginTier::Untrusted,
            PluginTier::Sandboxed,
            PluginTier::Standard,
            PluginTier::Trusted,
            PluginTier::Kernel,
        ] {
            let json = serde_json::to_string(&tier).unwrap();
            let decoded: PluginTier = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded, tier);
        }
    }

    // ── RegisteredTool tests ──────────────────────────────────────────────

    #[test]
    fn registered_tool_storage_key() {
        let tool = make_tool("my-plugin.lint", "1.2.3", PluginTier::Standard);
        assert_eq!(tool.storage_key(), "my-plugin.lint@1.2.3");
    }

    #[test]
    fn registered_tool_serde_roundtrip() {
        let tool = make_tool("my-plugin.lint", "1.0.0", PluginTier::Trusted);
        let json = serde_json::to_string(&tool).unwrap();
        let decoded: RegisteredTool = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, tool);
    }

    // ── DynamicToolRegistry: register / unregister / get ─────────────────

    #[test]
    fn register_and_get_tool() {
        let mut reg = DynamicToolRegistry::new();
        let tool = make_tool("my-plugin.lint", "1.0.0", PluginTier::Standard);
        reg.register_tool("my-plugin.lint@1.0.0", tool.clone())
            .unwrap();

        let found = reg.get_tool("my-plugin.lint").unwrap();
        assert_eq!(found.name, "my-plugin.lint");
        assert_eq!(found.version, "1.0.0");
    }

    #[test]
    fn register_bare_name_uses_tool_version() {
        let mut reg = DynamicToolRegistry::new();
        let tool = make_tool("check", "2.1.0", PluginTier::Sandboxed);
        reg.register_tool("check", tool).unwrap();

        assert!(reg.get_tool("check").is_some());
        // Exact key must also work.
        assert!(reg.resolve_tool("check", Some("2.1.0")).is_some());
    }

    #[test]
    fn duplicate_registration_is_error() {
        let mut reg = DynamicToolRegistry::new();
        let tool = make_tool("dup", "1.0.0", PluginTier::Standard);
        reg.register_tool("dup@1.0.0", tool.clone()).unwrap();
        let err = reg.register_tool("dup@1.0.0", tool).unwrap_err();
        assert!(err.to_string().contains("already registered"));
    }

    #[test]
    fn unregister_exact_version() {
        let mut reg = DynamicToolRegistry::new();
        reg.register_tool("t@1.0.0", make_tool("t", "1.0.0", PluginTier::Standard))
            .unwrap();
        reg.register_tool("t@2.0.0", make_tool("t", "2.0.0", PluginTier::Standard))
            .unwrap();

        reg.unregister_tool("t@1.0.0").unwrap();
        assert!(reg.resolve_tool("t", Some("1.0.0")).is_none());
        assert!(reg.get_tool("t").is_some()); // 2.0.0 still present
    }

    #[test]
    fn unregister_all_versions_by_name() {
        let mut reg = DynamicToolRegistry::new();
        reg.register_tool("x@1.0.0", make_tool("x", "1.0.0", PluginTier::Standard))
            .unwrap();
        reg.register_tool("x@1.1.0", make_tool("x", "1.1.0", PluginTier::Standard))
            .unwrap();

        reg.unregister_tool("x").unwrap();
        assert!(reg.get_tool("x").is_none());
        assert!(reg.is_empty());
    }

    #[test]
    fn unregister_missing_returns_error() {
        let mut reg = DynamicToolRegistry::new();
        assert!(reg.unregister_tool("does-not-exist").is_err());
        assert!(reg.unregister_tool("does-not-exist@1.0.0").is_err());
    }

    #[test]
    fn get_missing_returns_none() {
        let reg = DynamicToolRegistry::new();
        assert!(reg.get_tool("nonexistent").is_none());
    }

    #[test]
    fn list_tools_returns_all_versions() {
        let mut reg = DynamicToolRegistry::new();
        reg.register_tool("a@1.0.0", make_tool("a", "1.0.0", PluginTier::Standard))
            .unwrap();
        reg.register_tool("a@2.0.0", make_tool("a", "2.0.0", PluginTier::Standard))
            .unwrap();
        reg.register_tool("b@1.0.0", make_tool("b", "1.0.0", PluginTier::Trusted))
            .unwrap();

        let all = reg.list_tools();
        assert_eq!(all.len(), 3);
    }

    // ── T02: version resolution ───────────────────────────────────────────

    #[test]
    fn resolve_tool_latest_version() {
        let mut reg = DynamicToolRegistry::new();
        reg.register_tool(
            "tool@1.0.0",
            make_tool("tool", "1.0.0", PluginTier::Standard),
        )
        .unwrap();
        reg.register_tool(
            "tool@1.2.0",
            make_tool("tool", "1.2.0", PluginTier::Standard),
        )
        .unwrap();
        reg.register_tool(
            "tool@1.10.0",
            make_tool("tool", "1.10.0", PluginTier::Standard),
        )
        .unwrap();
        reg.register_tool(
            "tool@2.0.0",
            make_tool("tool", "2.0.0", PluginTier::Standard),
        )
        .unwrap();

        // No version → should return 2.0.0 (the latest).
        let latest = reg.resolve_tool("tool", None).unwrap();
        assert_eq!(latest.version, "2.0.0");
    }

    #[test]
    fn resolve_tool_exact_version_via_option() {
        let mut reg = DynamicToolRegistry::new();
        reg.register_tool(
            "tool@1.0.0",
            make_tool("tool", "1.0.0", PluginTier::Standard),
        )
        .unwrap();
        reg.register_tool(
            "tool@2.0.0",
            make_tool("tool", "2.0.0", PluginTier::Standard),
        )
        .unwrap();

        let v1 = reg.resolve_tool("tool", Some("1.0.0")).unwrap();
        assert_eq!(v1.version, "1.0.0");
    }

    #[test]
    fn resolve_tool_name_at_version_selector() {
        let mut reg = DynamicToolRegistry::new();
        reg.register_tool(
            "tool@1.0.0",
            make_tool("tool", "1.0.0", PluginTier::Standard),
        )
        .unwrap();
        reg.register_tool(
            "tool@2.0.0",
            make_tool("tool", "2.0.0", PluginTier::Standard),
        )
        .unwrap();

        // "name@version" syntax embedded in name argument.
        let v1 = reg.resolve_tool("tool@1.0.0", None).unwrap();
        assert_eq!(v1.version, "1.0.0");
    }

    #[test]
    fn resolve_tool_missing_version_returns_none() {
        let mut reg = DynamicToolRegistry::new();
        reg.register_tool(
            "tool@1.0.0",
            make_tool("tool", "1.0.0", PluginTier::Standard),
        )
        .unwrap();

        assert!(reg.resolve_tool("tool", Some("9.9.9")).is_none());
    }

    #[test]
    fn resolve_tool_semver_numeric_ordering() {
        // 1.10.0 > 1.9.0 — numeric comparison, not lexicographic.
        let mut reg = DynamicToolRegistry::new();
        reg.register_tool("v@1.9.0", make_tool("v", "1.9.0", PluginTier::Standard))
            .unwrap();
        reg.register_tool("v@1.10.0", make_tool("v", "1.10.0", PluginTier::Standard))
            .unwrap();

        let latest = reg.resolve_tool("v", None).unwrap();
        assert_eq!(latest.version, "1.10.0");
    }

    #[test]
    fn tool_names_returns_deduplicated_sorted_names() {
        let mut reg = DynamicToolRegistry::new();
        reg.register_tool(
            "beta@1.0.0",
            make_tool("beta", "1.0.0", PluginTier::Standard),
        )
        .unwrap();
        reg.register_tool(
            "alpha@1.0.0",
            make_tool("alpha", "1.0.0", PluginTier::Standard),
        )
        .unwrap();
        reg.register_tool(
            "beta@2.0.0",
            make_tool("beta", "2.0.0", PluginTier::Standard),
        )
        .unwrap();

        let names = reg.tool_names();
        assert_eq!(names, vec!["alpha", "beta"]);
    }

    #[test]
    fn empty_registry_len_and_is_empty() {
        let reg = DynamicToolRegistry::new();
        assert_eq!(reg.len(), 0);
        assert!(reg.is_empty());
    }

    #[test]
    fn len_counts_each_version_separately() {
        let mut reg = DynamicToolRegistry::new();
        reg.register_tool("t@1.0.0", make_tool("t", "1.0.0", PluginTier::Standard))
            .unwrap();
        reg.register_tool("t@2.0.0", make_tool("t", "2.0.0", PluginTier::Standard))
            .unwrap();
        assert_eq!(reg.len(), 2);
        assert!(!reg.is_empty());
    }
}
