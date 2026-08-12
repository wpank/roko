//! [`StaticToolRegistry`] — compile-time registry of shipped built-ins (§36.9).
//!
//! This unit struct implements [`roko_core::tool::ToolRegistry`] over
//! the shared [`super::builtin::ROKO_BUILTIN_TOOLS`] slice. Every Roko
//! deployment uses this registry as its base tool set; config-driven
//! extensions (MCP tools, role overrides) layer on top through a
//! separate compound registry (shipped in a later phase).

use std::collections::HashMap;

use roko_core::error::{Result, RokoError};
use roko_core::tool::{ToolDef, ToolRegistry};
#[allow(unused_imports)]
use tracing;

use super::builtin::ROKO_BUILTIN_TOOLS;
use super::sandbox_config::SandboxConfig;

/// Registry of the built-in Roko tools (§36.b).
///
/// Zero-sized: the definitions live in the
/// [`ROKO_BUILTIN_TOOLS`] static. Lookups are a linear scan —
/// The built-in set is small enough that a hashmap would be slower after
/// allocation overhead.
#[derive(Debug, Clone, Copy, Default)]
pub struct StaticToolRegistry;

impl StaticToolRegistry {
    /// Construct the registry. Cheap (zero-sized).
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Number of built-in tools — always [`super::builtin::TOOL_COUNT`].
    #[must_use]
    pub fn len(&self) -> usize {
        ROKO_BUILTIN_TOOLS.len()
    }

    /// Is the registry empty? Always `false` for the static registry.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        ROKO_BUILTIN_TOOLS.is_empty()
    }
}

impl ToolRegistry for StaticToolRegistry {
    fn get(&self, name: &str) -> Option<&ToolDef> {
        ROKO_BUILTIN_TOOLS.iter().find(|t| t.name == name)
    }

    fn all(&self) -> &[ToolDef] {
        ROKO_BUILTIN_TOOLS.as_slice()
    }

    fn validate_args(&self, name: &str, args: &serde_json::Value) -> Result<()> {
        let def = self
            .get(name)
            .ok_or_else(|| RokoError::invalid(format!("unknown tool: {name}")))?;
        validate_against_schema(&def.parameters, args)
    }
}

// ─── DynamicToolRegistry ─────────────────────────────────────────────────

/// A runtime-extensible registry that merges built-in tools with plugin-declared
/// tools (§E32-T02).
///
/// The registry starts with a snapshot of [`ROKO_BUILTIN_TOOLS`] and allows
/// additional [`ToolDef`]s to be registered at runtime via [`Self::register`].
/// All tools (static + dynamic) are stored in a single owned `Vec<ToolDef>` so
/// that [`ToolRegistry::get`] and [`ToolRegistry::all`] can return references
/// with the struct's own lifetime.
///
/// # Sandbox inheritance
///
/// Plugin tools registered via [`Self::register_plugin`] automatically inherit
/// the plugin's [`SandboxConfig`]. The config is stored in a side-map keyed by
/// plugin name and is accessible via [`Self::sandbox_for`] and
/// [`Self::sandbox_for_tool`].
///
/// # Thread-safety
///
/// `DynamicToolRegistry` itself is `Send + Sync`. For shared mutable access
/// across threads, wrap it in `Arc<parking_lot::RwLock<_>>`.
///
/// # Deduplication
///
/// When a plugin tool shares a name with an existing entry, the new entry
/// **replaces** the old one and a `tracing::warn` is emitted.
#[derive(Debug, Clone)]
pub struct DynamicToolRegistry {
    /// All tools: built-in snapshot + plugin-registered, deduplicated by name.
    tools: Vec<ToolDef>,
    /// Sandbox configuration per plugin name.
    ///
    /// Built-in tools have no entry here; lookups for built-in tools via
    /// [`Self::sandbox_for_tool`] return `None`.
    sandbox_configs: HashMap<String, SandboxConfig>,
}

impl DynamicToolRegistry {
    /// Construct a new `DynamicToolRegistry` pre-populated with every built-in
    /// tool from [`ROKO_BUILTIN_TOOLS`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            tools: ROKO_BUILTIN_TOOLS.to_vec(),
            sandbox_configs: HashMap::new(),
        }
    }

    /// Construct a `DynamicToolRegistry` from an explicit seed list of tools.
    /// Does **not** include the built-ins automatically. Useful for tests.
    #[must_use]
    pub fn from_tools(tools: Vec<ToolDef>) -> Self {
        Self {
            tools,
            sandbox_configs: HashMap::new(),
        }
    }

    /// Register a new tool at runtime.
    ///
    /// If a tool with the same name already exists, the existing entry is
    /// replaced and a warning is logged.
    pub fn register(&mut self, tool: ToolDef) {
        const SAFETY_CRITICAL: &[&str] = &["bash", "read_file", "write_file"];

        if let Some(pos) = self.tools.iter().position(|t| t.name == tool.name) {
            let old_source = &self.tools[pos].source;
            if SAFETY_CRITICAL.contains(&tool.name.as_str()) {
                tracing::warn!(
                    tool = %tool.name,
                    old_source = ?old_source,
                    new_source = ?tool.source,
                    "DynamicToolRegistry: overriding safety-critical built-in tool"
                );
            } else {
                tracing::warn!(
                    tool = %tool.name,
                    old_source = ?old_source,
                    new_source = ?tool.source,
                    "DynamicToolRegistry: plugin tool overrides existing entry"
                );
            }
            self.tools[pos] = tool;
        } else {
            self.tools.push(tool);
        }
    }

    /// Register multiple tools at once.
    pub fn register_all(&mut self, tools: impl IntoIterator<Item = ToolDef>) {
        for tool in tools {
            self.register(tool);
        }
    }

    /// Register a batch of plugin-declared tools with an associated
    /// [`SandboxConfig`] that all tools in the batch will inherit.
    ///
    /// The `plugin_name` is stored in the sandbox map so that
    /// [`Self::sandbox_for`] and [`Self::sandbox_for_tool`] can retrieve it
    /// later. Tools are registered individually via [`Self::register`] so the
    /// usual deduplication and safety-critical warnings apply.
    pub fn register_plugin(
        &mut self,
        plugin_name: &str,
        tools: Vec<ToolDef>,
        sandbox: SandboxConfig,
    ) {
        self.sandbox_configs.insert(plugin_name.to_string(), sandbox);
        for tool in tools {
            self.register(tool);
        }
    }

    /// Remove a tool by name. Returns the removed [`ToolDef`] if it existed.
    pub fn unregister(&mut self, name: &str) -> Option<ToolDef> {
        if let Some(pos) = self.tools.iter().position(|t| t.name == name) {
            Some(self.tools.swap_remove(pos))
        } else {
            None
        }
    }

    /// Total number of registered tools (built-ins + plugin-declared).
    #[must_use]
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Returns `true` when no tools are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    /// Return all tools whose source is a `Plugin { name }` matching `extension`.
    #[must_use]
    pub fn by_extension<'a>(&'a self, extension: &str) -> Vec<&'a ToolDef> {
        use roko_core::tool::ToolSource;
        self.tools
            .iter()
            .filter(|t| matches!(&t.source, ToolSource::Plugin { name } if name == extension))
            .collect()
    }

    /// Return all tools in the given [`roko_core::tool::ToolCategory`].
    #[must_use]
    pub fn by_category(&self, category: roko_core::tool::ToolCategory) -> Vec<&ToolDef> {
        self.tools
            .iter()
            .filter(|t| t.category == category)
            .collect()
    }

    /// Returns `true` if a tool with `name` is registered.
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.tools.iter().any(|t| t.name == name)
    }

    // ── Sandbox config accessors ──────────────────────────────────────

    /// Return the [`SandboxConfig`] registered for `plugin_name`, if any.
    ///
    /// Returns `None` for built-in tools or plugins that were registered
    /// without an explicit sandbox config via [`Self::register`] /
    /// [`Self::register_all`] (i.e., not via [`Self::register_plugin`]).
    #[must_use]
    pub fn sandbox_for(&self, plugin_name: &str) -> Option<&SandboxConfig> {
        self.sandbox_configs.get(plugin_name)
    }

    /// Return the [`SandboxConfig`] that applies to a tool identified by
    /// `tool_name`.
    ///
    /// If the tool's source is `ToolSource::Plugin { name }` and that plugin
    /// has a registered sandbox config, that config is returned. Otherwise
    /// `None` is returned (built-ins, MCP tools, and plugin tools without
    /// an explicit sandbox are all unconstrained at the registry layer).
    #[must_use]
    pub fn sandbox_for_tool(&self, tool_name: &str) -> Option<&SandboxConfig> {
        use roko_core::tool::ToolSource;
        let tool = self.tools.iter().find(|t| t.name == tool_name)?;
        if let ToolSource::Plugin { name } = &tool.source {
            self.sandbox_configs.get(name)
        } else {
            None
        }
    }

    /// Return the number of plugins that have a registered [`SandboxConfig`].
    #[must_use]
    pub fn sandbox_count(&self) -> usize {
        self.sandbox_configs.len()
    }
}

impl Default for DynamicToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolRegistry for DynamicToolRegistry {
    fn get(&self, name: &str) -> Option<&ToolDef> {
        self.tools.iter().find(|t| t.name == name)
    }

    fn all(&self) -> &[ToolDef] {
        &self.tools
    }

    fn validate_args(&self, name: &str, args: &serde_json::Value) -> Result<()> {
        let def = self
            .get(name)
            .ok_or_else(|| RokoError::invalid(format!("unknown tool: {name}")))?;
        validate_against_schema(&def.parameters, args)
    }
}

// ─── Internal helpers ─────────────────────────────────────────────────────

/// Validate `args` against a tool's JSON schema.
///
/// Day-one implementation: checks the top-level `type` constraint (the
/// only constraint the §36.9 stub schemas set). Real JSON-schema
/// validation — required-property enforcement, nested types, enums —
/// ships with the concrete handler schemas in §36.b via the
/// `jsonschema` crate.
fn validate_against_schema(
    schema: &roko_core::tool::ToolSchema,
    args: &serde_json::Value,
) -> Result<()> {
    let schema_value = schema.as_value();
    let expected_type = schema_value.get("type").and_then(serde_json::Value::as_str);
    match expected_type {
        Some("object") => {
            if args.is_object() {
                Ok(())
            } else {
                Err(RokoError::invalid(format!(
                    "schema validation failed: expected object, got {}",
                    json_kind(args)
                )))
            }
        }
        Some("array") => {
            if args.is_array() {
                Ok(())
            } else {
                Err(RokoError::invalid(format!(
                    "schema validation failed: expected array, got {}",
                    json_kind(args)
                )))
            }
        }
        // Unknown or missing "type" → permissive until §36.b ships real schemas.
        _ => Ok(()),
    }
}

const fn json_kind(v: &serde_json::Value) -> &'static str {
    match v {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "bool",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use super::super::builtin::{BUILTIN_TOOL_NAMES, TOOL_COUNT};
    use super::*;
    use roko_core::AgentRole;
    use std::collections::HashSet;

    #[test]
    fn builtin_count_matches() {
        // TOOL_COUNT is a compile-time constant that reflects the active
        // feature set (chain tools add 17 when `chain` is enabled).
        // Assert the invariant between the constant and the runtime slices.
        assert_eq!(ROKO_BUILTIN_TOOLS.len(), TOOL_COUNT);
        assert_eq!(BUILTIN_TOOL_NAMES.len(), TOOL_COUNT);
        // Sanity: minimum is 16 std + 4 ISFR = 20.
        assert!(TOOL_COUNT >= 20, "TOOL_COUNT must be at least 20");
    }

    #[test]
    fn all_len_matches_tool_count() {
        let reg = StaticToolRegistry::new();
        assert_eq!(reg.all().len(), TOOL_COUNT);
        assert_eq!(reg.len(), TOOL_COUNT);
        assert!(!reg.is_empty());
    }

    #[test]
    fn no_duplicate_names() {
        let mut seen = HashSet::new();
        for t in ROKO_BUILTIN_TOOLS.iter() {
            assert!(
                seen.insert(t.name.clone()),
                "duplicate tool name: {}",
                t.name
            );
        }
        assert_eq!(seen.len(), TOOL_COUNT);
    }

    #[test]
    fn builtin_tool_names_match_definitions() {
        let reg = StaticToolRegistry::new();
        for (i, name) in BUILTIN_TOOL_NAMES.iter().enumerate() {
            let def = reg.get(name).expect("name must be present");
            assert_eq!(def.name, *name, "index {i}");
        }
    }

    #[test]
    fn get_missing_returns_none() {
        let reg = StaticToolRegistry::new();
        assert!(reg.get("definitely_not_a_real_tool").is_none());
    }

    #[test]
    fn get_existing_returns_some() {
        let reg = StaticToolRegistry::new();
        assert!(reg.get("read_file").is_some());
        assert!(reg.get("bash").is_some());
        assert!(reg.get("task").is_some());
    }

    #[test]
    fn validate_unknown_tool_is_err() {
        let reg = StaticToolRegistry::new();
        let result = reg.validate_args("zzz_no_such_tool", &serde_json::json!({}));
        assert!(result.is_err());
        let err = result.expect_err("validation should fail");
        let msg = format!("{err}");
        assert!(msg.contains("unknown tool"));
    }

    #[test]
    fn validate_known_tool_with_object_args_is_ok() {
        let reg = StaticToolRegistry::new();
        assert!(
            reg.validate_args("read_file", &serde_json::json!({}))
                .is_ok()
        );
        assert!(
            reg.validate_args("bash", &serde_json::json!({"command": "ls"}))
                .is_ok()
        );
    }

    #[test]
    fn validate_known_tool_with_non_object_args_is_err() {
        let reg = StaticToolRegistry::new();
        // Schemas declare `type: "object"` via `ToolSchema::any_object`,
        // so non-object args must fail.
        let result = reg.validate_args("read_file", &serde_json::json!("not an object"));
        assert!(result.is_err());
        let err = result.expect_err("validation should fail");
        let msg = format!("{err}");
        assert!(msg.contains("schema validation failed"));
    }

    #[test]
    fn for_role_implementer_is_nonempty() {
        let reg = StaticToolRegistry::new();
        let tools = reg.for_role(AgentRole::Implementer);
        assert!(
            !tools.is_empty(),
            "Implementer should see at least one tool"
        );
        // Implementer has read + write + exec → should see write_file and bash.
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"read_file"));
        assert!(names.contains(&"write_file"));
        assert!(names.contains(&"bash"));
    }

    #[test]
    fn for_role_auditor_is_read_only_subset() {
        let reg = StaticToolRegistry::new();
        let tools = reg.for_role(AgentRole::Auditor);
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        // Auditor is read-only → read_file/grep/glob/ls visible; write_file/bash not.
        assert!(names.contains(&"read_file"));
        assert!(names.contains(&"grep"));
        assert!(names.contains(&"glob"));
        assert!(names.contains(&"ls"));
        assert!(!names.contains(&"write_file"));
        assert!(!names.contains(&"bash"));
        assert!(!names.contains(&"web_fetch"));
    }

    #[test]
    fn for_role_preserves_allowlist_invariants() {
        let reg = StaticToolRegistry::new();
        // Every role enumerates without panic and returns a subset of all().
        let all_roles: Vec<AgentRole> = std::iter::once(AgentRole::Conductor)
            .chain(AgentRole::ALL_AGENTS.iter().copied())
            .collect();
        for role in all_roles {
            let tools = reg.for_role(role);
            assert!(tools.len() <= TOOL_COUNT);
        }
    }

    /// Verify that all 19 GitHub MCP tools appear in the catalog with correct
    /// source, category, and write/read permission classification.
    #[test]
    fn github_tool_catalog_entries() {
        use super::super::builtin::github::{GITHUB_TOOL_COUNT, GITHUB_TOOL_NAMES};
        use roko_core::tool::{ToolCategory, ToolSource};

        let reg = StaticToolRegistry::new();

        // All GitHub tools must be present.
        let mut found = 0usize;
        for name in &GITHUB_TOOL_NAMES {
            let def = reg
                .get(name)
                .unwrap_or_else(|| panic!("GitHub tool '{name}' missing from catalog"));
            assert_eq!(
                def.category,
                ToolCategory::Mcp,
                "GitHub tool '{name}' must have category Mcp"
            );
            assert!(
                matches!(&def.source, ToolSource::Mcp { server } if server == "roko-mcp-github"),
                "GitHub tool '{name}' must have source Mcp {{ server: \"roko-mcp-github\" }}"
            );
            found += 1;
        }
        assert_eq!(
            found, GITHUB_TOOL_COUNT,
            "expected exactly {GITHUB_TOOL_COUNT} GitHub tools"
        );

        // Write tools must carry write permission.
        let write_tools = [
            "github.create_pr",
            "github.comment_pr",
            "github.review_pr",
            "github.merge_pr",
            "github.create_issue",
            "github.comment_issue",
            "github.close_issue",
            "github.add_labels",
            "github.create_label",
            "github.create_branch",
        ];
        for name in write_tools {
            let def = reg.get(name).unwrap_or_else(|| panic!("missing '{name}'"));
            assert!(
                def.permission.write,
                "write tool '{name}' must have permission.write = true"
            );
        }

        // Read-only tools must NOT carry write permission.
        let read_tools = [
            "github.list_prs",
            "github.get_pr",
            "github.list_issues",
            "github.get_file",
            "github.search_code",
            "github.list_commits",
            "github.get_branch",
            "github.compare_branches",
            "github.get_actions_status",
        ];
        for name in read_tools {
            let def = reg.get(name).unwrap_or_else(|| panic!("missing '{name}'"));
            assert!(
                !def.permission.write,
                "read tool '{name}' must have permission.write = false"
            );
        }
    }

    // ── DynamicToolRegistry sandbox inheritance tests ─────────────────

    use roko_core::tool::{ToolCategory, ToolPermission, ToolSource};

    fn plugin_tool(name: &str, plugin: &str) -> ToolDef {
        let mut t =
            ToolDef::new(name, "a plugin tool", ToolCategory::Exec, ToolPermission::executes());
        t.source = ToolSource::Plugin { name: plugin.to_string() };
        t
    }

    #[test]
    fn sandbox_register_plugin_stores_config() {
        let mut reg = DynamicToolRegistry::from_tools(vec![]);
        let sandbox = SandboxConfig::for_tier_level(2);
        reg.register_plugin(
            "myplugin",
            vec![plugin_tool("myplugin.lint", "myplugin")],
            sandbox.clone(),
        );
        assert_eq!(reg.sandbox_count(), 1);
        let stored = reg.sandbox_for("myplugin").expect("must have sandbox");
        assert_eq!(*stored, sandbox);
    }

    #[test]
    fn sandbox_for_tool_resolves_via_source() {
        let mut reg = DynamicToolRegistry::from_tools(vec![]);
        let sandbox = SandboxConfig::for_tier_level(3);
        reg.register_plugin(
            "myplugin",
            vec![plugin_tool("myplugin.format", "myplugin")],
            sandbox.clone(),
        );
        let resolved = reg.sandbox_for_tool("myplugin.format").expect("must resolve");
        assert_eq!(*resolved, sandbox);
    }

    #[test]
    fn sandbox_for_tool_returns_none_for_builtins() {
        let reg = DynamicToolRegistry::new();
        // Built-in tools have no plugin sandbox config.
        assert!(reg.sandbox_for_tool("bash").is_none());
        assert!(reg.sandbox_for_tool("read_file").is_none());
    }

    #[test]
    fn sandbox_for_unknown_plugin_returns_none() {
        let reg = DynamicToolRegistry::new();
        assert!(reg.sandbox_for("nonexistent_plugin").is_none());
    }

    #[test]
    fn sandbox_for_tool_returns_none_for_unknown_tool() {
        let reg = DynamicToolRegistry::new();
        assert!(reg.sandbox_for_tool("no_such_tool").is_none());
    }

    #[test]
    fn sandbox_register_plugin_registers_tools() {
        let mut reg = DynamicToolRegistry::from_tools(vec![]);
        let sandbox = SandboxConfig::for_tier_level(2);
        reg.register_plugin(
            "myplugin",
            vec![
                plugin_tool("myplugin.lint", "myplugin"),
                plugin_tool("myplugin.format", "myplugin"),
            ],
            sandbox,
        );
        assert!(reg.get("myplugin.lint").is_some());
        assert!(reg.get("myplugin.format").is_some());
        assert_eq!(reg.len(), 2);
    }

    #[test]
    fn sandbox_multiple_plugins_independent_configs() {
        let mut reg = DynamicToolRegistry::from_tools(vec![]);
        let sandbox_a = SandboxConfig::for_tier_level(2);
        let sandbox_b = SandboxConfig::for_tier_level(4);

        reg.register_plugin(
            "plugin_a",
            vec![plugin_tool("plugin_a.run", "plugin_a")],
            sandbox_a.clone(),
        );
        reg.register_plugin(
            "plugin_b",
            vec![plugin_tool("plugin_b.run", "plugin_b")],
            sandbox_b.clone(),
        );

        assert_eq!(reg.sandbox_count(), 2);
        assert_eq!(*reg.sandbox_for("plugin_a").unwrap(), sandbox_a);
        assert_eq!(*reg.sandbox_for("plugin_b").unwrap(), sandbox_b);
        // Cross-check that tool lookup resolves to the right sandbox.
        assert!(!reg.sandbox_for_tool("plugin_a.run").unwrap().network_access);
        assert!(reg.sandbox_for_tool("plugin_b.run").unwrap().network_access);
    }

    #[test]
    fn sandbox_tier1_is_most_restricted_in_registry() {
        let mut reg = DynamicToolRegistry::from_tools(vec![]);
        let sandbox = SandboxConfig::for_tier_level(1);
        reg.register_plugin(
            "untrusted",
            vec![plugin_tool("untrusted.op", "untrusted")],
            sandbox,
        );
        let cfg = reg.sandbox_for("untrusted").unwrap();
        assert!(cfg.allowed_paths.is_empty(), "tier 1 must allow no paths");
        assert!(!cfg.network_access, "tier 1 must block network");
    }

    #[test]
    fn sandbox_tier5_is_unrestricted_in_registry() {
        let mut reg = DynamicToolRegistry::from_tools(vec![]);
        let sandbox = SandboxConfig::for_tier_level(5);
        reg.register_plugin(
            "kernel_ext",
            vec![plugin_tool("kernel_ext.op", "kernel_ext")],
            sandbox,
        );
        let cfg = reg.sandbox_for("kernel_ext").unwrap();
        assert!(cfg.network_access, "tier 5 must allow network");
        assert_eq!(cfg.max_memory_mb, 0, "tier 5 must have no memory cap");
    }
}
