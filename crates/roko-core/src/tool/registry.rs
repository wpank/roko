//! The [`ToolRegistry`] trait.
//!
//! A registry maps canonical snake_case tool names to [`ToolDef`]s and
//! answers the per-role filtering question (§36.10). Concrete
//! implementations:
//!
//! - `StaticToolRegistry` in `roko-std` (§36.9) — compile-time baked
//!   registry of the 16 built-in tools (§36.b).
//! - [`VecToolRegistry`] (this file) — a trivial `Vec`-backed registry
//!   used in tests and as a fallback for ad-hoc dispatcher setup.

use super::def::ToolDef;
use super::role_allowlist::role_allowlist;
use crate::error::{Result, RokoError};
use crate::AgentRole;

// ─── ToolRegistry trait ──────────────────────────────────────────────────

/// Name → [`ToolDef`] lookup plus per-role filtering.
///
/// Every implementor must answer [`Self::get`] and [`Self::all`] — the
/// rest can rely on default impls that delegate through those.
pub trait ToolRegistry: Send + Sync {
    /// Look up a tool definition by canonical name.
    fn get(&self, name: &str) -> Option<&ToolDef>;

    /// Iterate over every registered [`ToolDef`].
    fn all(&self) -> &[ToolDef];

    /// Validate arguments against the tool's JSON schema.
    ///
    /// The default implementation only checks that the tool exists;
    /// downstream registries (notably `StaticToolRegistry`) override to
    /// plug in real JSON-schema validation (see §36.42 for the
    /// dispatcher-side wiring).
    fn validate_args(&self, name: &str, _args: &serde_json::Value) -> Result<()> {
        if self.get(name).is_some() {
            Ok(())
        } else {
            Err(RokoError::invalid(format!("unknown tool: {name}")))
        }
    }

    /// Return the subset of tools this `role` is allowed to call.
    ///
    /// The default implementation filters by
    /// [`crate::tool::ToolPermission::satisfied_by`] against the role's
    /// [`ToolPermissions`](crate::ToolPermissions); a richer registry can
    /// override to honor per-role overrides from config.
    fn for_role(&self, role: AgentRole) -> Vec<&ToolDef> {
        role_allowlist(role, self.all())
    }
}

// ─── VecToolRegistry ─────────────────────────────────────────────────────

/// A trivial registry backed by a `Vec<ToolDef>`. Used in tests and as
/// a dynamic fallback when the static registry isn't convenient.
#[derive(Debug, Clone, Default)]
pub struct VecToolRegistry {
    tools: Vec<ToolDef>,
}

impl VecToolRegistry {
    /// Construct an empty registry.
    #[must_use]
    pub const fn new() -> Self {
        Self { tools: Vec::new() }
    }

    /// Construct from a vector of tool definitions.
    #[must_use]
    pub const fn from_tools(tools: Vec<ToolDef>) -> Self {
        Self { tools }
    }

    /// Push a single tool definition. Returns the new length.
    pub fn push(&mut self, def: ToolDef) -> usize {
        self.tools.push(def);
        self.tools.len()
    }
}

impl ToolRegistry for VecToolRegistry {
    fn get(&self, name: &str) -> Option<&ToolDef> {
        self.tools.iter().find(|t| t.name == name)
    }

    fn all(&self) -> &[ToolDef] {
        &self.tools
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool::{ToolCategory, ToolPermission};

    fn read_file() -> ToolDef {
        ToolDef::new("read_file", "read a file", ToolCategory::Read, ToolPermission::read_only())
    }

    fn write_file() -> ToolDef {
        ToolDef::new("write_file", "write a file", ToolCategory::Write, ToolPermission::writes())
    }

    fn bash() -> ToolDef {
        ToolDef::new("bash", "run a command", ToolCategory::Exec, ToolPermission::executes())
    }

    #[test]
    fn vec_registry_get_roundtrip() {
        let r = VecToolRegistry::from_tools(vec![read_file(), bash()]);
        assert!(r.get("read_file").is_some());
        assert!(r.get("bash").is_some());
        assert!(r.get("nope").is_none());
    }

    #[test]
    fn vec_registry_all_preserves_order() {
        let r = VecToolRegistry::from_tools(vec![read_file(), bash(), write_file()]);
        let names: Vec<&str> = r.all().iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, vec!["read_file", "bash", "write_file"]);
    }

    #[test]
    fn validate_args_rejects_unknown_tool() {
        let r = VecToolRegistry::from_tools(vec![read_file()]);
        let result = r.validate_args("does_not_exist", &serde_json::json!({}));
        assert!(result.is_err());
    }

    #[test]
    fn validate_args_accepts_known_tool_default_impl() {
        let r = VecToolRegistry::from_tools(vec![read_file()]);
        assert!(r.validate_args("read_file", &serde_json::json!({})).is_ok());
    }

    #[test]
    fn for_role_filters_by_tool_permissions() {
        let r = VecToolRegistry::from_tools(vec![read_file(), write_file(), bash()]);

        // Implementer has full perms → sees all 3.
        let impl_tools = r.for_role(AgentRole::Implementer);
        assert_eq!(impl_tools.len(), 3);

        // Auditor is read-only → sees only read_file.
        let auditor_tools = r.for_role(AgentRole::Auditor);
        let names: Vec<&str> = auditor_tools.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, vec!["read_file"]);

        // IntegrationTester is read+exec → sees read_file + bash, not write_file.
        let tester_tools = r.for_role(AgentRole::IntegrationTester);
        let names: Vec<&str> = tester_tools.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, vec!["read_file", "bash"]);
    }

    #[test]
    fn push_adds_and_returns_length() {
        let mut r = VecToolRegistry::new();
        assert_eq!(r.push(read_file()), 1);
        assert_eq!(r.push(bash()), 2);
        assert_eq!(r.all().len(), 2);
    }
}
