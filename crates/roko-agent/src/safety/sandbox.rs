//! Runtime sandbox policy used by tool and agent dispatch gates.

use roko_core::config::schema::RunnerSandboxLevel;
use roko_core::plugin::PluginTier;
use roko_core::tool::{ToolCall, ToolContext, ToolDef, ToolError, ToolPermission};
use roko_std::tool::SandboxConfig;
use serde::{Deserialize, Serialize};

use super::path::{PathPolicy, canonicalize_with_policy};

/// Increasing sandbox enforcement levels.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SandboxLevel {
    /// No sandbox. Reserved for trusted in-tree execution.
    None,
    /// Audit policy decisions without blocking.
    Observe,
    /// Enforce worktree paths, secret-path denials, and network policy.
    #[default]
    Restrict,
    /// Deny network and constrain filesystem access to the worktree.
    Isolate,
    /// Memory-only execution: no filesystem, network, subprocess, or git.
    Quarantine,
}

/// Effective policy for a sandbox level.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxPolicy {
    /// Enforcement level that produced the policy.
    pub level: SandboxLevel,
    /// Declarative path, network, and resource bounds.
    pub config: SandboxConfig,
    /// Whether findings are audit-only.
    pub audit_only: bool,
    /// Whether the child/tool may inherit environment variables.
    pub allow_environment: bool,
}

impl SandboxLevel {
    /// Return the conservative effective policy for this level.
    #[must_use]
    pub fn effective_policy(self) -> SandboxPolicy {
        match self {
            Self::None => SandboxPolicy {
                level: self,
                config: SandboxConfig::unrestricted(),
                audit_only: false,
                allow_environment: true,
            },
            Self::Observe => SandboxPolicy {
                level: self,
                config: SandboxConfig::unrestricted(),
                audit_only: true,
                allow_environment: true,
            },
            Self::Restrict => SandboxPolicy {
                level: self,
                config: SandboxConfig::for_tier_level(3),
                audit_only: false,
                allow_environment: true,
            },
            Self::Isolate => SandboxPolicy {
                level: self,
                config: SandboxConfig::for_tier_level(2),
                audit_only: false,
                allow_environment: false,
            },
            Self::Quarantine => SandboxPolicy {
                level: self,
                config: SandboxConfig::most_restricted(),
                audit_only: false,
                allow_environment: false,
            },
        }
    }

    /// Whether provider permission/sandbox bypass flags are acceptable.
    #[must_use]
    pub const fn allows_permission_bypass(self) -> bool {
        matches!(self, Self::None | Self::Observe)
    }
}

impl From<PluginTier> for SandboxLevel {
    fn from(tier: PluginTier) -> Self {
        match tier {
            PluginTier::Kernel => Self::None,
            PluginTier::Trusted => Self::Observe,
            PluginTier::Standard => Self::Restrict,
            PluginTier::Sandboxed => Self::Isolate,
            PluginTier::Untrusted => Self::Quarantine,
        }
    }
}

impl From<RunnerSandboxLevel> for SandboxLevel {
    fn from(level: RunnerSandboxLevel) -> Self {
        match level {
            RunnerSandboxLevel::None => Self::None,
            RunnerSandboxLevel::Observe => Self::Observe,
            RunnerSandboxLevel::Restrict => Self::Restrict,
            RunnerSandboxLevel::Isolate => Self::Isolate,
            RunnerSandboxLevel::Quarantine => Self::Quarantine,
        }
    }
}

impl SandboxPolicy {
    /// Enforce this policy when only the canonical tool name is available.
    /// The dispatcher additionally calls [`Self::check_tool`] with the full
    /// definition, covering custom and dynamically discovered tools.
    pub fn check_call(
        &self,
        call: &ToolCall,
        ctx: &ToolContext,
        path_policy: &PathPolicy,
    ) -> Result<(), ToolError> {
        let permission = match call.name.as_str() {
            "web_fetch" | "web_search" => ToolPermission {
                network: true,
                ..ToolPermission::default()
            },
            "bash" | "run_tests" => ToolPermission::executes(),
            "write_file" | "edit_file" | "multi_edit" | "apply_patch" | "notebook_edit" => {
                ToolPermission::writes()
            }
            "read_file" | "ls" | "glob" | "grep" => ToolPermission::read_only(),
            _ => ToolPermission::default(),
        };
        self.check_permissions(&call.name, permission, &call.arguments, ctx, path_policy)
    }

    /// Enforce this policy against a structured tool call.
    pub fn check_tool(
        &self,
        tool: &ToolDef,
        params: &serde_json::Value,
        ctx: &ToolContext,
        path_policy: &PathPolicy,
    ) -> Result<(), ToolError> {
        self.check_permissions(&tool.name, tool.permission, params, ctx, path_policy)
    }

    fn check_permissions(
        &self,
        tool_name: &str,
        permission: ToolPermission,
        params: &serde_json::Value,
        ctx: &ToolContext,
        path_policy: &PathPolicy,
    ) -> Result<(), ToolError> {
        if matches!(self.level, SandboxLevel::None | SandboxLevel::Observe) {
            return Ok(());
        }

        if self.level == SandboxLevel::Quarantine
            && (permission.read
                || permission.write
                || permission.exec
                || permission.git
                || permission.network)
        {
            return Err(ToolError::PermissionDenied(format!(
                "sandbox quarantine blocks capability-bearing tool `{}`",
                tool_name
            )));
        }
        if permission.network && !self.config.network_access {
            return Err(ToolError::PermissionDenied(format!(
                "sandbox {:?} blocks network tool `{}`",
                self.level, tool_name
            )));
        }

        if permission.read || permission.write {
            let path_arg = params
                .get("file_path")
                .or_else(|| params.get("path"))
                .or_else(|| params.get("pattern"))
                .and_then(serde_json::Value::as_str);
            if let Some(path_arg) = path_arg {
                let canonical =
                    canonicalize_with_policy(&ctx.worktree_path, path_arg, path_policy)?;
                let relative = canonical.relative.to_string_lossy().replace('\\', "/");
                if !sandbox_path_allowed(&self.config, &relative) {
                    return Err(ToolError::PermissionDenied(format!(
                        "sandbox {:?} blocks path `{relative}`",
                        self.level
                    )));
                }
            } else if self.config.allowed_paths.is_empty() {
                return Err(ToolError::PermissionDenied(format!(
                    "sandbox {:?} blocks filesystem tool `{}` without a bounded path",
                    self.level, tool_name
                )));
            }
        }

        Ok(())
    }

    /// Enforce sandbox restrictions for an opaque subprocess launch.
    pub fn check_exec(&self, program: &str) -> Result<(), ToolError> {
        if self.level == SandboxLevel::Quarantine {
            return Err(ToolError::PermissionDenied(format!(
                "sandbox quarantine blocks subprocess `{program}`"
            )));
        }
        Ok(())
    }
}

fn sandbox_path_allowed(config: &SandboxConfig, relative: &str) -> bool {
    let allowed = config
        .allowed_paths
        .iter()
        .any(|pattern| path_pattern_matches(pattern, relative));
    allowed
        && !config
            .denied_paths
            .iter()
            .any(|pattern| path_pattern_matches(pattern, relative))
}

fn path_pattern_matches(pattern: &str, path: &str) -> bool {
    let pattern = pattern.trim_start_matches("./");
    let path = path.trim_start_matches("./");
    if pattern == "**" || pattern == path {
        return true;
    }
    if let Some(segment) = pattern
        .strip_prefix("**/")
        .and_then(|value| value.strip_suffix("/**"))
    {
        return path.split('/').any(|part| part == segment);
    }
    if let Some(suffix) = pattern.strip_prefix("**/") {
        return path == suffix || path.ends_with(&format!("/{suffix}"));
    }
    if let Some(prefix) = pattern.strip_suffix("/**") {
        return path == prefix || path.starts_with(&format!("{prefix}/"));
    }
    if let Some(prefix) = pattern.strip_suffix('*') {
        return path.starts_with(prefix)
            || path
                .rsplit_once('/')
                .is_some_and(|(_, name)| name.starts_with(prefix));
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use roko_core::tool::{ToolCategory, ToolPermission};

    #[test]
    fn plugin_tiers_map_monotonically_to_sandbox_levels() {
        assert_eq!(SandboxLevel::from(PluginTier::Kernel), SandboxLevel::None);
        assert_eq!(
            SandboxLevel::from(PluginTier::Untrusted),
            SandboxLevel::Quarantine
        );
    }

    #[test]
    fn quarantine_blocks_filesystem_and_subprocesses() {
        let policy = SandboxLevel::Quarantine.effective_policy();
        let tool = ToolDef::new(
            "read_file",
            "read",
            ToolCategory::Read,
            ToolPermission::read_only(),
        );
        let ctx = ToolContext::testing("/tmp/worktree");
        assert!(
            policy
                .check_tool(
                    &tool,
                    &serde_json::json!({"path": "README.md"}),
                    &ctx,
                    &PathPolicy::default(),
                )
                .is_err()
        );
        assert!(policy.check_exec("bash").is_err());
    }

    #[test]
    fn restrict_denies_secret_paths() {
        let policy = SandboxLevel::Restrict.effective_policy();
        let tool = ToolDef::new(
            "read_file",
            "read",
            ToolCategory::Read,
            ToolPermission::read_only(),
        );
        let ctx = ToolContext::testing("/tmp/worktree");
        let result = policy.check_tool(
            &tool,
            &serde_json::json!({"path": ".env.local"}),
            &ctx,
            &PathPolicy::default(),
        );
        assert!(result.is_err());
    }
}
