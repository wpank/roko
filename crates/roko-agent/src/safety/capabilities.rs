//! OCaps-style capability tokens for agent execution.

use std::path::Path;

use rand::random;
pub use roko_core::plugin::{PluginAccessCapability as Capability, PluginTier};
use roko_std::tool::SandboxConfig;
use serde::{Deserialize, Serialize};
use url::Url;

// ─── Plugin trust tiers ──────────────────────────────────────────────

/// Check whether a plugin at the given `tier` is allowed to invoke the
/// requested `capability`. Returns `Ok(())` on success; returns a
/// human-readable error on denial.
pub fn check_plugin_tier(tier: PluginTier, capability: &Capability) -> Result<(), String> {
    match capability {
        Capability::Network { .. } if !tier.allows_network() => Err(format!(
            "plugin tier {:?} does not permit network access",
            tier
        )),
        Capability::WritePath(_) if !tier.allows_writes() => Err(format!(
            "plugin tier {:?} does not permit filesystem writes",
            tier
        )),
        Capability::ReadPath(_) if matches!(tier, PluginTier::Untrusted) => Err(format!(
            "plugin tier {:?} does not permit filesystem reads",
            tier
        )),
        Capability::Exec(_) if matches!(tier, PluginTier::Untrusted | PluginTier::Sandboxed) => {
            Err(format!(
                "plugin tier {:?} does not permit subprocess execution",
                tier
            ))
        }
        _ => Ok(()),
    }
}

/// Unforgeable warrant token carrying a reduced capability set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentWarrant {
    /// Token identifier.
    pub id: [u8; 32],
    /// Granted capabilities.
    pub capabilities: Vec<Capability>,
    /// Authority that issued the warrant.
    pub issuer: String,
    /// Optional expiry timestamp in unix seconds.
    pub expires_at: Option<u64>,
    /// Remaining delegation depth.
    pub delegate_depth: u8,
}

impl AgentWarrant {
    /// Create a warrant with a random identifier.
    #[must_use]
    pub fn new(
        issuer: impl Into<String>,
        capabilities: Vec<Capability>,
        delegate_depth: u8,
    ) -> Self {
        Self {
            id: random(),
            capabilities,
            issuer: issuer.into(),
            expires_at: None,
            delegate_depth,
        }
    }

    /// Attach an expiry timestamp.
    #[must_use]
    pub fn with_expiry(mut self, expires_at: Option<u64>) -> Self {
        self.expires_at = expires_at;
        self
    }
}

/// Errors raised when a warrant cannot be delegated.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CapabilityError {
    /// The requested capability is not covered by the parent warrant.
    #[error("subset capability is not covered by parent warrant")]
    NotCovered,
    /// The warrant cannot be delegated further.
    #[error("delegation depth exhausted")]
    DepthExhausted,
    /// The parent warrant has reached or passed its expiry.
    #[error("warrant expired")]
    Expired,
}

/// Check whether `warrant` covers `required`.
#[must_use]
pub fn check_capability(warrant: &AgentWarrant, required: &Capability) -> bool {
    check_capability_at(warrant, required, unix_now())
}

/// Check a warrant at a trusted unix-second observation.
#[must_use]
pub fn check_capability_at(
    warrant: &AgentWarrant,
    required: &Capability,
    observed_at: u64,
) -> bool {
    if warrant
        .expires_at
        .is_some_and(|expiry| expiry <= observed_at)
    {
        return false;
    }
    warrant
        .capabilities
        .iter()
        .any(|granted| capability_covers(granted, required))
}

/// Delegate a warrant to a strict subset of its capabilities.
pub fn delegate(
    warrant: &AgentWarrant,
    subset: &[Capability],
) -> Result<AgentWarrant, CapabilityError> {
    delegate_at(warrant, subset, unix_now())
}

/// Delegate a warrant using a trusted unix-second observation.
pub fn delegate_at(
    warrant: &AgentWarrant,
    subset: &[Capability],
    observed_at: u64,
) -> Result<AgentWarrant, CapabilityError> {
    if warrant
        .expires_at
        .is_some_and(|expiry| expiry <= observed_at)
    {
        return Err(CapabilityError::Expired);
    }
    if warrant.delegate_depth == 0 {
        return Err(CapabilityError::DepthExhausted);
    }

    if subset
        .iter()
        .any(|required| !check_capability_at(warrant, required, observed_at))
    {
        return Err(CapabilityError::NotCovered);
    }

    Ok(AgentWarrant {
        id: random(),
        capabilities: subset.to_vec(),
        issuer: warrant.issuer.clone(),
        expires_at: warrant.expires_at,
        delegate_depth: warrant.delegate_depth.saturating_sub(1),
    })
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn capability_covers(granted: &Capability, required: &Capability) -> bool {
    match (granted, required) {
        (Capability::Tool(a), Capability::Tool(b)) => a == b,
        (Capability::Exec(a), Capability::Exec(b)) => a == b,
        (
            Capability::Network { host: ah, port: ap },
            Capability::Network { host: bh, port: bp },
        ) => ah == bh && ap == bp,
        (Capability::ReadPath(granted), Capability::ReadPath(required))
        | (Capability::WritePath(granted), Capability::WritePath(required)) => {
            path_contains(granted, required)
        }
        _ => false,
    }
}

fn path_contains(granted: &Path, required: &Path) -> bool {
    required.starts_with(granted)
}

// ─── Tool permission policy (fail-closed default) ───────────────────────

/// Policy governing what happens when a tool is **not** explicitly listed
/// in the configured permission set.
///
/// The two variants encode opposite defaults:
///
/// - [`AllowExplicit`](Self::AllowExplicit) (fail-closed): only tools that
///   appear in the explicit allow list are permitted. Everything else is
///   denied. This is the recommended production default.
/// - [`DenyExplicit`](Self::DenyExplicit) (fail-open): all tools are
///   permitted unless they appear in the explicit deny list. Useful for
///   development or fully-trusted agent profiles where enumerating every
///   tool is impractical.
///
/// # Examples
///
/// ```
/// use roko_agent::safety::capabilities::{ToolPermissionPolicy, check_tool_permission};
///
/// // Fail-closed: only "read_file" and "grep" are allowed.
/// let allowed = vec!["read_file".to_string(), "grep".to_string()];
/// assert!(check_tool_permission("read_file", &ToolPermissionPolicy::AllowExplicit, &allowed));
/// assert!(!check_tool_permission("bash", &ToolPermissionPolicy::AllowExplicit, &allowed));
///
/// // Fail-open: everything except items in the list is allowed.
/// let denied = vec!["bash".to_string()];
/// assert!(check_tool_permission("read_file", &ToolPermissionPolicy::DenyExplicit, &denied));
/// assert!(!check_tool_permission("bash", &ToolPermissionPolicy::DenyExplicit, &denied));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolPermissionPolicy {
    /// Fail-closed: only tools in the explicit list are permitted.
    /// This is the default and recommended policy for production use.
    AllowExplicit,
    /// Fail-open: all tools are permitted unless explicitly listed.
    /// Suitable for development or fully-trusted agent profiles.
    DenyExplicit,
}

impl Default for ToolPermissionPolicy {
    /// The default policy is fail-closed ([`AllowExplicit`](Self::AllowExplicit)).
    fn default() -> Self {
        Self::AllowExplicit
    }
}

/// Check whether `tool_name` is permitted under the given `policy` and
/// configured tool list.
///
/// The semantics of `configured_tools` depend on the policy:
///
/// - [`AllowExplicit`](ToolPermissionPolicy::AllowExplicit): `configured_tools`
///   is the set of **allowed** tools. A tool must appear in the list (or the
///   list must contain the wildcard `"*"`) to be permitted.
/// - [`DenyExplicit`](ToolPermissionPolicy::DenyExplicit): `configured_tools`
///   is the set of **denied** tools. A tool is permitted unless it appears in
///   the list.
///
/// Special cases:
/// - An empty `configured_tools` list under `AllowExplicit` denies everything.
/// - An empty `configured_tools` list under `DenyExplicit` allows everything.
/// - The wildcard entry `"*"` in `configured_tools` under `AllowExplicit`
///   allows all tools.
#[must_use]
pub fn check_tool_permission(
    tool_name: &str,
    policy: &ToolPermissionPolicy,
    configured_tools: &[String],
) -> bool {
    match policy {
        ToolPermissionPolicy::AllowExplicit => {
            // Fail-closed: tool must be explicitly listed or wildcard present.
            configured_tools.iter().any(|t| t == "*" || t == tool_name)
        }
        ToolPermissionPolicy::DenyExplicit => {
            // Fail-open: tool is allowed unless explicitly denied.
            !configured_tools.iter().any(|t| t == tool_name)
        }
    }
}

/// Build a capability requirement from a network URL.
#[must_use]
pub fn network_capability_from_url(url: &str) -> Option<Capability> {
    let parsed = Url::parse(url).ok()?;
    let host = parsed
        .host_str()?
        .trim_matches(|c| c == '[' || c == ']')
        .to_string();
    Some(Capability::Network {
        host,
        port: parsed.port_or_known_default().unwrap_or(0),
    })
}

/// Extract the first shell token for an exec capability requirement.
#[must_use]
pub fn exec_capability_from_command(command: &str) -> Option<Capability> {
    command
        .split_whitespace()
        .next()
        .filter(|token| !token.is_empty())
        .map(|token| Capability::Exec(token.to_string()))
}

// ─── Plugin capability checks (cross-referenced with SandboxConfig) ─────

/// Well-known capability names accepted by [`check_plugin_capability`].
///
/// These map to the three enforcement dimensions shared by [`PluginTier`]
/// and [`SandboxConfig`]:
///
/// | Name | PluginTier gate | SandboxConfig field |
/// |-------------|----------------------|---------------------|
/// | `network` | `allows_network()` | `network_access` |
/// | `filesystem`| `allows_writes()` | `!allowed_paths.is_empty()` |
/// | `subprocess`| tier >= Standard (3) | (not modeled — always gated by tier) |
const KNOWN_CAPABILITIES: &[&str] = &["network", "filesystem", "subprocess"];

/// Check whether a plugin at numeric tier `tier` (1-5) is allowed to use the
/// named `capability`.
///
/// This is the string-based entry point for capability enforcement. It
/// cross-references the [`PluginTier`] permission model with the
/// [`SandboxConfig`] for the same tier level, ensuring both agree.
///
/// # Recognised capabilities
///
/// | `capability` | Meaning |
/// |--------------|---------|
/// | `"network"` | Outbound network access (HTTP, DNS, etc.) |
/// | `"filesystem"` | Filesystem write access (reads follow separate rules) |
/// | `"subprocess"` | Spawning child processes / shell execution |
///
/// Unknown capability names always return `false` (deny-by-default).
///
/// # Examples
///
/// ```
/// use roko_agent::safety::capabilities::check_plugin_capability;
///
/// // Tier 2 (Sandboxed) cannot use network or subprocess
/// assert!(!check_plugin_capability(2, "network"));
/// assert!(!check_plugin_capability(2, "subprocess"));
///
/// // Tier 4 (Trusted) can use everything
/// assert!(check_plugin_capability(4, "network"));
/// assert!(check_plugin_capability(4, "filesystem"));
/// assert!(check_plugin_capability(4, "subprocess"));
///
/// // Unknown capabilities are always denied
/// assert!(!check_plugin_capability(5, "teleport"));
/// ```
#[must_use]
pub fn check_plugin_capability(tier: u8, capability: &str) -> bool {
    // Reject unknown capabilities immediately (deny-by-default).
    if !KNOWN_CAPABILITIES.contains(&capability) {
        return false;
    }

    let plugin_tier = PluginTier::from_level(tier);
    let sandbox = SandboxConfig::for_tier_level(tier);

    match capability {
        "network" => plugin_tier.allows_network() && sandbox.network_access,
        "filesystem" => plugin_tier.allows_writes() && !sandbox.allowed_paths.is_empty(),
        "subprocess" => {
            // Subprocess execution requires tier >= Standard (3).
            // SandboxConfig does not model subprocess separately, so
            // the tier enum is the sole authority.
            !matches!(plugin_tier, PluginTier::Untrusted | PluginTier::Sandboxed)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn check_capability_matches_exact_tool() {
        let warrant = AgentWarrant::new("issuer", vec![Capability::Tool("bash".into())], 1);
        assert!(check_capability(&warrant, &Capability::Tool("bash".into())));
        assert!(!check_capability(
            &warrant,
            &Capability::Tool("grep".into())
        ));
    }

    #[test]
    fn delegate_reduces_scope() {
        let warrant = AgentWarrant::new(
            "issuer",
            vec![
                Capability::Tool("bash".into()),
                Capability::Exec("bash".into()),
            ],
            1,
        );
        let child = delegate(&warrant, &[Capability::Tool("bash".into())]).unwrap();
        assert_eq!(child.delegate_depth, 0);
        assert_eq!(child.capabilities.len(), 1);
    }

    #[test]
    fn warrant_expiry_is_fail_closed_at_exact_boundary() {
        let warrant = AgentWarrant::new("issuer", vec![Capability::Tool("bash".into())], 1)
            .with_expiry(Some(100));
        let required = Capability::Tool("bash".into());
        assert!(check_capability_at(&warrant, &required, 99));
        assert!(!check_capability_at(&warrant, &required, 100));
        assert_eq!(
            delegate_at(&warrant, std::slice::from_ref(&required), 100),
            Err(CapabilityError::Expired)
        );
    }

    #[test]
    fn network_capability_parses_host_and_port() {
        let cap = network_capability_from_url("https://api.example.com:443/path").unwrap();
        assert!(matches!(cap, Capability::Network { .. }));
    }

    // ─── PluginTier tests ────────────────────────────────────────────

    #[test]
    fn plugin_tier_default_is_sandboxed() {
        assert_eq!(PluginTier::default(), PluginTier::Sandboxed);
    }

    #[test]
    fn plugin_tier_ordering_is_ascending() {
        assert!(PluginTier::Untrusted < PluginTier::Sandboxed);
        assert!(PluginTier::Sandboxed < PluginTier::Standard);
        assert!(PluginTier::Standard < PluginTier::Trusted);
        assert!(PluginTier::Trusted < PluginTier::Kernel);
    }

    #[test]
    fn untrusted_tier_blocks_everything() {
        let tier = PluginTier::Untrusted;
        assert!(!tier.allows_network());
        assert!(!tier.allows_secrets());
        assert!(!tier.allows_writes());
        assert!(check_plugin_tier(tier, &Capability::ReadPath(PathBuf::from("/tmp"))).is_err());
        assert!(check_plugin_tier(tier, &Capability::WritePath(PathBuf::from("/tmp"))).is_err());
        assert!(check_plugin_tier(tier, &Capability::Exec("ls".into())).is_err());
        assert!(
            check_plugin_tier(
                tier,
                &Capability::Network {
                    host: "example.com".into(),
                    port: 443
                }
            )
            .is_err()
        );
    }

    #[test]
    fn sandboxed_tier_allows_reads_only() {
        let tier = PluginTier::Sandboxed;
        assert!(!tier.allows_network());
        assert!(!tier.allows_secrets());
        assert!(!tier.allows_writes());
        assert!(check_plugin_tier(tier, &Capability::ReadPath(PathBuf::from("/tmp"))).is_ok());
        assert!(check_plugin_tier(tier, &Capability::WritePath(PathBuf::from("/tmp"))).is_err());
        assert!(check_plugin_tier(tier, &Capability::Exec("ls".into())).is_err());
    }

    #[test]
    fn standard_tier_allows_reads_writes_exec_no_network() {
        let tier = PluginTier::Standard;
        assert!(tier.allows_network());
        assert!(!tier.allows_secrets());
        assert!(tier.allows_writes());
        assert!(check_plugin_tier(tier, &Capability::ReadPath(PathBuf::from("/tmp"))).is_ok());
        assert!(check_plugin_tier(tier, &Capability::WritePath(PathBuf::from("/tmp"))).is_ok());
        assert!(check_plugin_tier(tier, &Capability::Exec("ls".into())).is_ok());
    }

    #[test]
    fn trusted_tier_allows_everything() {
        let tier = PluginTier::Trusted;
        assert!(tier.allows_network());
        assert!(tier.allows_secrets());
        assert!(tier.allows_writes());
        assert!(
            check_plugin_tier(
                tier,
                &Capability::Network {
                    host: "example.com".into(),
                    port: 443
                }
            )
            .is_ok()
        );
    }

    #[test]
    fn plugin_tier_round_trips_through_serde() {
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

    // ─── PluginTier::from_level / as_level tests ─────────────────────

    #[test]
    fn from_level_round_trips_all_tiers() {
        for level in 1..=5u8 {
            let tier = PluginTier::from_level(level);
            assert_eq!(tier.as_level(), level);
        }
    }

    #[test]
    fn from_level_clamps_zero_to_untrusted() {
        assert_eq!(PluginTier::from_level(0), PluginTier::Untrusted);
    }

    #[test]
    fn from_level_clamps_high_to_kernel() {
        assert_eq!(PluginTier::from_level(99), PluginTier::Kernel);
    }

    // ─── check_plugin_capability tests ───────────────────────────────

    #[test]
    fn check_plugin_capability_unknown_is_always_denied() {
        for tier in 1..=5u8 {
            assert!(
                !check_plugin_capability(tier, "teleport"),
                "unknown capability must be denied at tier {tier}"
            );
            assert!(
                !check_plugin_capability(tier, ""),
                "empty capability must be denied at tier {tier}"
            );
            assert!(
                !check_plugin_capability(tier, "NETWORK"),
                "capability check must be case-sensitive (tier {tier})"
            );
        }
    }

    #[test]
    fn check_plugin_capability_tier1_denies_all() {
        assert!(!check_plugin_capability(1, "network"));
        assert!(!check_plugin_capability(1, "filesystem"));
        assert!(!check_plugin_capability(1, "subprocess"));
    }

    #[test]
    fn check_plugin_capability_tier2_denies_all() {
        // Sandboxed: read-only FS, no network, no subprocess.
        // filesystem capability means write-access, so denied.
        assert!(!check_plugin_capability(2, "network"));
        assert!(!check_plugin_capability(2, "filesystem"));
        assert!(!check_plugin_capability(2, "subprocess"));
    }

    #[test]
    fn check_plugin_capability_tier3_allows_all() {
        // Standard: worktree r/w, allowlisted network, exec allowed.
        assert!(check_plugin_capability(3, "network"));
        assert!(check_plugin_capability(3, "filesystem"));
        assert!(check_plugin_capability(3, "subprocess"));
    }

    #[test]
    fn check_plugin_capability_tier4_allows_all() {
        assert!(check_plugin_capability(4, "network"));
        assert!(check_plugin_capability(4, "filesystem"));
        assert!(check_plugin_capability(4, "subprocess"));
    }

    #[test]
    fn check_plugin_capability_tier5_allows_all() {
        assert!(check_plugin_capability(5, "network"));
        assert!(check_plugin_capability(5, "filesystem"));
        assert!(check_plugin_capability(5, "subprocess"));
    }

    #[test]
    fn check_plugin_capability_agrees_with_sandbox_config() {
        // Verify that the PluginTier and SandboxConfig agree for every tier.
        for level in 1..=5u8 {
            let tier = PluginTier::from_level(level);
            let sandbox = SandboxConfig::for_tier_level(level);

            // Network: both must agree.
            assert_eq!(
                tier.allows_network(),
                sandbox.network_access,
                "network mismatch at tier {level}"
            );

            // The check_plugin_capability result for "network" must match
            // both individual checks.
            assert_eq!(
                check_plugin_capability(level, "network"),
                tier.allows_network() && sandbox.network_access,
                "check_plugin_capability(network) mismatch at tier {level}"
            );

            // Filesystem (write): PluginTier.allows_writes() and SandboxConfig
            // having non-empty allowed_paths must agree in result.
            let fs_allowed = tier.allows_writes() && !sandbox.allowed_paths.is_empty();
            assert_eq!(
                check_plugin_capability(level, "filesystem"),
                fs_allowed,
                "check_plugin_capability(filesystem) mismatch at tier {level}"
            );
        }
    }

    #[test]
    fn check_plugin_capability_clamped_tiers() {
        // Tier 0 -> Untrusted, tier 99 -> Kernel.
        assert!(!check_plugin_capability(0, "network"));
        assert!(check_plugin_capability(99, "network"));
    }

    // ─── ToolPermissionPolicy tests ─────────────────────────────────

    #[test]
    fn permission_policy_default_is_allow_explicit() {
        assert_eq!(
            ToolPermissionPolicy::default(),
            ToolPermissionPolicy::AllowExplicit
        );
    }

    #[test]
    fn permission_allow_explicit_permits_listed_tools() {
        let allowed = vec!["read_file".to_string(), "grep".to_string()];
        assert!(check_tool_permission(
            "read_file",
            &ToolPermissionPolicy::AllowExplicit,
            &allowed
        ));
        assert!(check_tool_permission(
            "grep",
            &ToolPermissionPolicy::AllowExplicit,
            &allowed
        ));
    }

    #[test]
    fn permission_allow_explicit_denies_unlisted_tools() {
        let allowed = vec!["read_file".to_string()];
        assert!(!check_tool_permission(
            "bash",
            &ToolPermissionPolicy::AllowExplicit,
            &allowed
        ));
        assert!(!check_tool_permission(
            "write_file",
            &ToolPermissionPolicy::AllowExplicit,
            &allowed
        ));
    }

    #[test]
    fn permission_allow_explicit_empty_list_denies_all() {
        let allowed: Vec<String> = vec![];
        assert!(!check_tool_permission(
            "read_file",
            &ToolPermissionPolicy::AllowExplicit,
            &allowed
        ));
        assert!(!check_tool_permission(
            "bash",
            &ToolPermissionPolicy::AllowExplicit,
            &allowed
        ));
    }

    #[test]
    fn permission_allow_explicit_wildcard_allows_all() {
        let allowed = vec!["*".to_string()];
        assert!(check_tool_permission(
            "read_file",
            &ToolPermissionPolicy::AllowExplicit,
            &allowed
        ));
        assert!(check_tool_permission(
            "bash",
            &ToolPermissionPolicy::AllowExplicit,
            &allowed
        ));
        assert!(check_tool_permission(
            "anything_at_all",
            &ToolPermissionPolicy::AllowExplicit,
            &allowed
        ));
    }

    #[test]
    fn permission_deny_explicit_denies_listed_tools() {
        let denied = vec!["bash".to_string(), "write_file".to_string()];
        assert!(!check_tool_permission(
            "bash",
            &ToolPermissionPolicy::DenyExplicit,
            &denied
        ));
        assert!(!check_tool_permission(
            "write_file",
            &ToolPermissionPolicy::DenyExplicit,
            &denied
        ));
    }

    #[test]
    fn permission_deny_explicit_allows_unlisted_tools() {
        let denied = vec!["bash".to_string()];
        assert!(check_tool_permission(
            "read_file",
            &ToolPermissionPolicy::DenyExplicit,
            &denied
        ));
        assert!(check_tool_permission(
            "grep",
            &ToolPermissionPolicy::DenyExplicit,
            &denied
        ));
    }

    #[test]
    fn permission_deny_explicit_empty_list_allows_all() {
        let denied: Vec<String> = vec![];
        assert!(check_tool_permission(
            "bash",
            &ToolPermissionPolicy::DenyExplicit,
            &denied
        ));
        assert!(check_tool_permission(
            "read_file",
            &ToolPermissionPolicy::DenyExplicit,
            &denied
        ));
    }

    #[test]
    fn permission_policy_serde_round_trip() {
        for policy in [
            ToolPermissionPolicy::AllowExplicit,
            ToolPermissionPolicy::DenyExplicit,
        ] {
            let json = serde_json::to_string(&policy).unwrap();
            let decoded: ToolPermissionPolicy = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded, policy);
        }
    }

    #[test]
    fn permission_allow_explicit_case_sensitive() {
        let allowed = vec!["Read_File".to_string()];
        assert!(check_tool_permission(
            "Read_File",
            &ToolPermissionPolicy::AllowExplicit,
            &allowed
        ));
        assert!(!check_tool_permission(
            "read_file",
            &ToolPermissionPolicy::AllowExplicit,
            &allowed
        ));
    }

    #[test]
    fn permission_deny_explicit_case_sensitive() {
        let denied = vec!["Bash".to_string()];
        assert!(!check_tool_permission(
            "Bash",
            &ToolPermissionPolicy::DenyExplicit,
            &denied
        ));
        // Lowercase "bash" is not denied — case matters.
        assert!(check_tool_permission(
            "bash",
            &ToolPermissionPolicy::DenyExplicit,
            &denied
        ));
    }

    #[test]
    fn permission_allow_explicit_wildcard_among_others() {
        // Wildcard alongside named entries still grants universal access.
        let allowed = vec!["read_file".to_string(), "*".to_string(), "grep".to_string()];
        assert!(check_tool_permission(
            "bash",
            &ToolPermissionPolicy::AllowExplicit,
            &allowed
        ));
    }
}
