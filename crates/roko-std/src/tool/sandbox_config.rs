//! [`SandboxConfig`] — resource and access limits for plugin-declared tools.
//!
//! Each plugin tier maps to a default [`SandboxConfig`] that restricts what
//! tool execution may access. The config is declarative: it describes the
//! *intent* of sandboxing; OS-level enforcement (seccomp/landlock) is future
//! work.
//!
//! # Tier defaults
//!
//! | Tier | allowed_paths | network_access | max_memory_mb | max_cpu_seconds |
//! |------|---------------|----------------|---------------|-----------------|
//! | 1 Untrusted | none | false | 64 | 5 |
//! | 2 Sandboxed | read-only worktree | false | 128 | 10 |
//! | 3 Standard | worktree r/w | allowlisted | 256 | 30 |
//! | 4 Trusted | full | full | 512 | 120 |
//! | 5 Kernel | unrestricted | unrestricted | u64::MAX | u64::MAX |

use serde::{Deserialize, Serialize};

// ─── SandboxConfig ────────────────────────────────────────────────────────

/// Sandbox constraints for a plugin-declared tool.
///
/// All fields use permissive defaults (`Default`) so that code that
/// constructs [`SandboxConfig`] without setting every field does not
/// accidentally over-restrict. The per-tier constructors
/// ([`SandboxConfig::for_tier_level`]) provide secure, opinionated starting
/// points.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Glob patterns (relative to the worktree) the tool may *read or write*.
    ///
    /// An empty list means **no path access is allowed**. Use `["**"]` to
    /// allow all worktree paths.
    pub allowed_paths: Vec<String>,

    /// Glob patterns that are explicitly *denied*, even if they would otherwise
    /// match an entry in `allowed_paths`.
    ///
    /// Denial takes priority over allowance. Use to carve out secrets,
    /// credentials, or OS paths from a broad `allowed_paths`.
    pub denied_paths: Vec<String>,

    /// Whether the tool may make outbound network connections.
    ///
    /// When `false` the tool must not use any network-backed built-ins
    /// (`web_fetch`, `web_search`). Enforcement is advisory at the config
    /// layer; the dispatcher gate enforces it at runtime.
    pub network_access: bool,

    /// Maximum resident-set-size (RSS) in megabytes.
    ///
    /// `0` means no limit. Non-zero values are advisory at the config layer;
    /// OS-level enforcement via `setrlimit` is future work.
    pub max_memory_mb: u64,

    /// Maximum wall-clock CPU seconds the tool may consume.
    ///
    /// `0` means no limit. Non-zero values are advisory at the config layer.
    pub max_cpu_seconds: u64,
}

impl Default for SandboxConfig {
    /// Unrestricted defaults — equivalent to [`SandboxConfig::unrestricted`].
    ///
    /// Code that builds a `SandboxConfig` for an *unknown* tier should prefer
    /// an explicit tier constructor ([`SandboxConfig::for_tier_level`]) or the
    /// named constructors instead of relying on `Default`.
    fn default() -> Self {
        Self::unrestricted()
    }
}

impl SandboxConfig {
    /// Completely unrestricted sandbox (tier 5 / Kernel equivalent).
    ///
    /// Use only for fully-trusted in-tree extensions.
    #[must_use]
    pub fn unrestricted() -> Self {
        Self {
            allowed_paths: vec!["**".to_string()],
            denied_paths: vec![],
            network_access: true,
            max_memory_mb: 0,
            max_cpu_seconds: 0,
        }
    }

    /// Most restrictive sandbox (tier 1 / Untrusted equivalent).
    ///
    /// No filesystem access, no network, tight CPU and memory caps.
    #[must_use]
    pub fn most_restricted() -> Self {
        Self {
            allowed_paths: vec![],
            denied_paths: vec![],
            network_access: false,
            max_memory_mb: 64,
            max_cpu_seconds: 5,
        }
    }

    /// Construct the default [`SandboxConfig`] for a numeric plugin tier.
    ///
    /// Accepts a raw tier level (1–5) so this module does not need to
    /// depend on `roko-agent`'s `PluginTier` enum. Callers in `roko-agent`
    /// can pass `tier as u8` directly.
    ///
    /// | Level | Tier name | Description |
    /// |-------|-----------|-------------|
    /// | 1 | Untrusted | No FS, no network, 64 MB / 5 s |
    /// | 2 | Sandboxed | Worktree read-only, no network, 128 MB / 10 s |
    /// | 3 | Standard | Worktree r/w, allowlisted network, 256 MB / 30 s |
    /// | 4 | Trusted | Full FS, full network, 512 MB / 120 s |
    /// | 5+ | Kernel | Unrestricted |
    #[must_use]
    pub fn for_tier_level(level: u8) -> Self {
        match level {
            1 => Self {
                allowed_paths: vec![],
                denied_paths: vec![],
                network_access: false,
                max_memory_mb: 64,
                max_cpu_seconds: 5,
            },
            2 => Self {
                // Sandboxed: read-only worktree paths; deny hidden/secret dirs.
                allowed_paths: vec!["**".to_string()],
                denied_paths: vec![
                    ".env".to_string(),
                    ".env.*".to_string(),
                    "**/.git/config".to_string(),
                    "**/secrets/**".to_string(),
                    "**/credentials/**".to_string(),
                ],
                network_access: false,
                max_memory_mb: 128,
                max_cpu_seconds: 10,
            },
            3 => Self {
                // Standard: full worktree r/w; deny secret artifacts; network allowed.
                allowed_paths: vec!["**".to_string()],
                denied_paths: vec![
                    ".env".to_string(),
                    ".env.*".to_string(),
                    "**/secrets/**".to_string(),
                ],
                network_access: true,
                max_memory_mb: 256,
                max_cpu_seconds: 30,
            },
            4 => Self {
                // Trusted: full filesystem, full network, generous caps.
                allowed_paths: vec!["**".to_string()],
                denied_paths: vec![],
                network_access: true,
                max_memory_mb: 512,
                max_cpu_seconds: 120,
            },
            _ => Self::unrestricted(),
        }
    }

    /// Validate that this `SandboxConfig` is internally consistent.
    ///
    /// Returns a list of human-readable violations. An empty list means
    /// the config is valid. Contradictions checked:
    ///
    /// - A path that appears in both `allowed_paths` and `denied_paths`
    ///   with an exact match (denial always wins, but the overlap is almost
    ///   certainly a mistake).
    /// - `max_cpu_seconds > 0` while `max_memory_mb == 0` when a memory
    ///   limit is clearly expected (i.e. tier < Trusted) is **not** flagged
    ///   here — that would require tier context. Callers may perform that
    ///   check separately.
    ///
    /// # Example
    ///
    /// ```
    /// use roko_std::tool::sandbox_config::SandboxConfig;
    ///
    /// let cfg = SandboxConfig {
    ///     allowed_paths: vec!["src/**".to_string()],
    ///     denied_paths:  vec!["src/**".to_string()], // exact duplicate
    ///     network_access: false,
    ///     max_memory_mb: 128,
    ///     max_cpu_seconds: 10,
    /// };
    /// let violations = cfg.validate();
    /// assert!(!violations.is_empty());
    /// ```
    #[must_use]
    pub fn validate(&self) -> Vec<String> {
        let mut violations = Vec::new();

        // Check for exact-match overlaps between allowed and denied paths.
        for allowed in &self.allowed_paths {
            if self.denied_paths.iter().any(|d| d == allowed) {
                violations.push(format!(
                    "path pattern '{allowed}' appears in both allowed_paths and denied_paths; \
                     denial wins but the overlap is likely a mistake"
                ));
            }
        }

        // Catch obviously impossible configs: allowed_paths is non-empty but
        // denied_paths covers ALL allowed entries (wildcard "**" in denied).
        let deny_all = self.denied_paths.iter().any(|d| d == "**");
        if deny_all && !self.allowed_paths.is_empty() {
            violations.push(
                "denied_paths contains '**' which blocks all allowed paths; \
                 the effective access is empty"
                    .to_string(),
            );
        }

        violations
    }

    /// Return `true` if `path_glob` (relative to worktree) is permitted
    /// by this config — i.e., it matches an entry in `allowed_paths` AND
    /// does *not* match any entry in `denied_paths`.
    ///
    /// This is a **name-equality** check, not a full glob expansion; callers
    /// that need real glob matching should use the `glob` crate themselves.
    /// The method exists so unit tests can verify the allow/deny logic
    /// without touching the filesystem.
    #[must_use]
    pub fn permits_path(&self, path_glob: &str) -> bool {
        let allowed = self.allowed_paths.iter().any(|p| p == path_glob || p == "**");
        if !allowed {
            return false;
        }
        let denied = self.denied_paths.iter().any(|d| d == path_glob || d == "**");
        !denied
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Tier mapping tests ────────────────────────────────────────────

    #[test]
    fn sandbox_tier1_is_most_restricted() {
        let cfg = SandboxConfig::for_tier_level(1);
        assert!(cfg.allowed_paths.is_empty(), "tier 1 must not allow any paths");
        assert!(!cfg.network_access, "tier 1 must block network");
        assert!(cfg.max_memory_mb <= 128, "tier 1 must cap memory");
        assert!(cfg.max_cpu_seconds <= 30, "tier 1 must cap cpu");
    }

    #[test]
    fn sandbox_tier2_allows_reads_denies_secrets() {
        let cfg = SandboxConfig::for_tier_level(2);
        assert!(!cfg.allowed_paths.is_empty(), "tier 2 must allow some paths");
        assert!(!cfg.network_access, "tier 2 must block network");
        assert!(
            cfg.denied_paths.iter().any(|d| d.contains(".env")),
            "tier 2 must deny .env files"
        );
    }

    #[test]
    fn sandbox_tier3_allows_network() {
        let cfg = SandboxConfig::for_tier_level(3);
        assert!(cfg.network_access, "tier 3 must allow network");
        assert!(
            cfg.max_memory_mb <= 512,
            "tier 3 must have a memory cap below tier 4/5"
        );
    }

    #[test]
    fn sandbox_tier4_is_generous() {
        let cfg = SandboxConfig::for_tier_level(4);
        assert!(cfg.network_access, "tier 4 must allow network");
        assert!(cfg.denied_paths.is_empty(), "tier 4 must not deny any paths");
    }

    #[test]
    fn sandbox_tier5_is_unrestricted() {
        let cfg = SandboxConfig::for_tier_level(5);
        assert!(cfg.network_access, "tier 5 must allow network");
        assert_eq!(cfg.max_memory_mb, 0, "tier 5 must have no memory cap");
        assert_eq!(cfg.max_cpu_seconds, 0, "tier 5 must have no cpu cap");
    }

    #[test]
    fn sandbox_tier_level_above_5_is_unrestricted() {
        let cfg = SandboxConfig::for_tier_level(99);
        assert_eq!(cfg, SandboxConfig::unrestricted());
    }

    #[test]
    fn sandbox_tiers_form_ascending_permissiveness() {
        // Higher tiers should be at least as permissive as lower tiers in
        // terms of memory and cpu caps (0 = unlimited).
        let t1 = SandboxConfig::for_tier_level(1);
        let t2 = SandboxConfig::for_tier_level(2);
        let t3 = SandboxConfig::for_tier_level(3);
        let t4 = SandboxConfig::for_tier_level(4);
        let t5 = SandboxConfig::for_tier_level(5);

        // Memory caps: 0 means unlimited, so higher tier should have higher or 0.
        fn mem_ge(higher: &SandboxConfig, lower: &SandboxConfig) -> bool {
            higher.max_memory_mb == 0 || higher.max_memory_mb >= lower.max_memory_mb
        }
        assert!(mem_ge(&t2, &t1));
        assert!(mem_ge(&t3, &t2));
        assert!(mem_ge(&t4, &t3));
        assert!(mem_ge(&t5, &t4));

        // Network: each tier should be at least as permissive as the previous.
        assert!(!t1.network_access);
        assert!(!t2.network_access);
        assert!(t3.network_access);
        assert!(t4.network_access);
        assert!(t5.network_access);
    }

    // ── Validation tests ──────────────────────────────────────────────

    #[test]
    fn validate_clean_config_returns_empty() {
        let cfg = SandboxConfig::for_tier_level(2);
        assert!(cfg.validate().is_empty(), "valid tier config must not produce violations");
    }

    #[test]
    fn validate_detects_exact_allow_deny_overlap() {
        let cfg = SandboxConfig {
            allowed_paths: vec!["src/**".to_string()],
            denied_paths: vec!["src/**".to_string()],
            network_access: false,
            max_memory_mb: 128,
            max_cpu_seconds: 10,
        };
        let violations = cfg.validate();
        assert!(
            !violations.is_empty(),
            "exact allow/deny overlap must produce a violation"
        );
        assert!(
            violations[0].contains("src/**"),
            "violation must mention the conflicting pattern"
        );
    }

    #[test]
    fn validate_detects_deny_all_with_nonempty_allowed() {
        let cfg = SandboxConfig {
            allowed_paths: vec!["src/**".to_string()],
            denied_paths: vec!["**".to_string()],
            network_access: false,
            max_memory_mb: 64,
            max_cpu_seconds: 5,
        };
        let violations = cfg.validate();
        assert!(
            !violations.is_empty(),
            "deny '**' with non-empty allowed must be flagged"
        );
    }

    #[test]
    fn validate_unrestricted_is_clean() {
        assert!(SandboxConfig::unrestricted().validate().is_empty());
    }

    #[test]
    fn validate_most_restricted_is_clean() {
        assert!(SandboxConfig::most_restricted().validate().is_empty());
    }

    // ── permits_path tests ────────────────────────────────────────────

    #[test]
    fn permits_path_denied_takes_priority() {
        let cfg = SandboxConfig {
            allowed_paths: vec!["**".to_string()],
            denied_paths: vec!["secrets/**".to_string()],
            network_access: false,
            max_memory_mb: 128,
            max_cpu_seconds: 10,
        };
        assert!(cfg.permits_path("src/main.rs"), "non-denied path under ** must be allowed");
        assert!(
            !cfg.permits_path("secrets/**"),
            "explicitly denied path must be blocked"
        );
    }

    #[test]
    fn permits_path_empty_allowed_blocks_everything() {
        let cfg = SandboxConfig::most_restricted();
        assert!(
            !cfg.permits_path("anything.rs"),
            "empty allowed_paths must block all access"
        );
    }

    #[test]
    fn permits_path_unrestricted_allows_everything() {
        let cfg = SandboxConfig::unrestricted();
        assert!(cfg.permits_path("src/lib.rs"));
        assert!(cfg.permits_path("very/deeply/nested/path.rs"));
    }

    // ── Serde round-trip ──────────────────────────────────────────────

    #[test]
    fn sandbox_config_serde_roundtrip() {
        let cfg = SandboxConfig::for_tier_level(3);
        let json = serde_json::to_string(&cfg).expect("serialize");
        let decoded: SandboxConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(cfg, decoded);
    }
}
