//! Canonical plugin SDK trust and capability contracts.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Trust tier assigned to plugins and plugin-declared tools.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginTier {
    /// Tier 1: no filesystem, subprocess, network, or secret access.
    Untrusted = 1,
    /// Tier 2: read-only filesystem access.
    Sandboxed = 2,
    /// Tier 3: worktree filesystem, subprocesses, and allowlisted network.
    Standard = 3,
    /// Tier 4: trusted native plugin with full resource access.
    Trusted = 4,
    /// Tier 5: in-tree kernel extension.
    Kernel = 5,
}

impl Default for PluginTier {
    fn default() -> Self {
        Self::Sandboxed
    }
}

impl PluginTier {
    /// Human-readable tier label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Untrusted => "untrusted",
            Self::Sandboxed => "sandboxed",
            Self::Standard => "standard",
            Self::Trusted => "trusted",
            Self::Kernel => "kernel",
        }
    }

    /// Convert a raw numeric level to a tier, clamping outside 1-5.
    #[must_use]
    pub const fn from_level(level: u8) -> Self {
        match level {
            0 | 1 => Self::Untrusted,
            2 => Self::Sandboxed,
            3 => Self::Standard,
            4 => Self::Trusted,
            _ => Self::Kernel,
        }
    }

    /// Numeric tier level.
    #[must_use]
    pub const fn level(self) -> u8 {
        self as u8
    }

    /// Numeric tier level, retained for existing capability-policy callers.
    #[must_use]
    pub const fn as_level(&self) -> u8 {
        *self as u8
    }

    /// Whether this tier permits filesystem reads.
    #[must_use]
    pub const fn allows_reads(self) -> bool {
        !matches!(self, Self::Untrusted)
    }

    /// Whether this tier permits filesystem writes.
    #[must_use]
    pub const fn allows_writes(self) -> bool {
        matches!(self, Self::Standard | Self::Trusted | Self::Kernel)
    }

    /// Whether this tier permits subprocess execution.
    #[must_use]
    pub const fn allows_exec(self) -> bool {
        matches!(self, Self::Standard | Self::Trusted | Self::Kernel)
    }

    /// Whether this tier permits network egress.
    #[must_use]
    pub const fn allows_network(self) -> bool {
        matches!(self, Self::Standard | Self::Trusted | Self::Kernel)
    }

    /// Whether this tier permits secret access.
    #[must_use]
    pub const fn allows_secrets(self) -> bool {
        matches!(self, Self::Trusted | Self::Kernel)
    }

    /// Concrete capabilities allowed by the legacy agent warrant policy.
    #[must_use]
    pub fn allowed_capabilities(self) -> Vec<PluginAccessCapability> {
        match self {
            Self::Untrusted => Vec::new(),
            Self::Sandboxed => vec![PluginAccessCapability::ReadPath(PathBuf::from("/"))],
            Self::Standard => vec![
                PluginAccessCapability::ReadPath(PathBuf::from("/")),
                PluginAccessCapability::WritePath(PathBuf::from("/")),
                PluginAccessCapability::Exec("*".into()),
            ],
            Self::Trusted | Self::Kernel => vec![
                PluginAccessCapability::ReadPath(PathBuf::from("/")),
                PluginAccessCapability::WritePath(PathBuf::from("/")),
                PluginAccessCapability::Exec("*".into()),
                PluginAccessCapability::Network {
                    host: "*".into(),
                    port: 0,
                },
            ],
        }
    }
}

/// Coarse capabilities declared by a plugin manifest.
///
/// These describe what a plugin requires; [`PluginTier`] describes the upper
/// bound granted by policy.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PluginCapability {
    /// Read files within sandbox-allowed paths.
    #[serde(default)]
    pub filesystem_read: bool,
    /// Write files within sandbox-allowed paths.
    #[serde(default)]
    pub filesystem_write: bool,
    /// Make outbound network requests.
    #[serde(default)]
    pub network_egress: bool,
    /// Read explicitly allowlisted secrets or environment values.
    #[serde(default)]
    pub secrets: bool,
    /// Execute a subprocess.
    #[serde(default)]
    pub exec: bool,
}

impl PluginCapability {
    /// Minimal inferred capability declaration for a manifest with tools.
    #[must_use]
    pub const fn declarative_tools() -> Self {
        Self {
            filesystem_read: false,
            filesystem_write: false,
            network_egress: false,
            secrets: false,
            exec: true,
        }
    }

    /// Capability names denied by `tier`.
    #[must_use]
    pub fn denied_by(self, tier: PluginTier) -> Vec<&'static str> {
        let mut denied = Vec::new();
        if self.filesystem_read && !tier.allows_reads() {
            denied.push("filesystem_read");
        }
        if self.filesystem_write && !tier.allows_writes() {
            denied.push("filesystem_write");
        }
        if self.network_egress && !tier.allows_network() {
            denied.push("network_egress");
        }
        if self.secrets && !tier.allows_secrets() {
            denied.push("secrets");
        }
        if self.exec && !tier.allows_exec() {
            denied.push("exec");
        }
        denied
    }
}

/// Concrete resource capability carried by agent warrants.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PluginAccessCapability {
    /// Named tool invocation.
    Tool(String),
    /// Shell execution capability.
    Exec(String),
    /// Read access to a path.
    ReadPath(PathBuf),
    /// Write access to a path.
    WritePath(PathBuf),
    /// Network capability for a host and port.
    Network {
        /// Hostname or wildcard.
        host: String,
        /// Port, or zero for any port.
        port: u16,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tier_and_capability_serde_are_stable() {
        assert_eq!(
            serde_json::to_string(&PluginTier::Standard).expect("tier serializes"),
            "\"standard\""
        );
        assert_eq!(
            serde_json::from_str::<PluginTier>("\"trusted\"").expect("tier deserializes"),
            PluginTier::Trusted
        );

        let declared: PluginCapability =
            serde_json::from_str("{}").expect("empty capability object deserializes");
        assert_eq!(declared, PluginCapability::default());
    }

    #[test]
    fn declared_capabilities_are_bounded_by_tier() {
        let required = PluginCapability {
            filesystem_read: true,
            exec: true,
            ..PluginCapability::default()
        };
        assert_eq!(required.denied_by(PluginTier::Sandboxed), vec!["exec"]);
        assert!(required.denied_by(PluginTier::Standard).is_empty());
    }
}
