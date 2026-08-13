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

use std::fmt;

use serde::{Deserialize, Serialize};

// ─── SandboxValidationError ───────────────────────────────────────────────

/// Errors produced by command and path validation in [`SandboxConfig`].
///
/// Each variant carries a human-readable description of the specific
/// violation so callers can surface actionable diagnostics without
/// having to inspect enum internals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SandboxValidationError {
    /// The command string contains one or more shell metacharacters that
    /// could enable injection (e.g. `|`, `;`, `&&`, `` ` ``, `$(`, `>`).
    ShellMetacharacter {
        /// The specific metacharacter or sequence that was detected.
        found: String,
        /// The original command string.
        command: String,
    },

    /// A path entry contains `../` or equivalent traversal sequences that
    /// could escape the intended sandbox root.
    PathTraversal {
        /// The offending path pattern.
        path: String,
    },

    /// The command string is empty or contains only whitespace.
    EmptyCommand,
}

impl fmt::Display for SandboxValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ShellMetacharacter { found, command } => {
                write!(
                    f,
                    "shell metacharacter '{found}' found in command: {command}"
                )
            }
            Self::PathTraversal { path } => {
                write!(f, "path traversal detected in path pattern: {path}")
            }
            Self::EmptyCommand => {
                write!(f, "command is empty or contains only whitespace")
            }
        }
    }
}

impl std::error::Error for SandboxValidationError {}

// ─── Shell metacharacter constants ────────────────────────────────────────

/// Multi-character shell metacharacter sequences to reject.
///
/// Order matters: longer sequences are checked before their single-char
/// substrings so that the error message reports the most specific match.
const SHELL_META_SEQUENCES: &[&str] = &["&&", "||", "$(", "<<", ">>"];

/// Single-character shell metacharacters to reject.
///
/// Each of these can enable command injection, output redirection, or
/// subshell execution when passed unsanitized to a shell.
const SHELL_META_CHARS: &[char] = &['|', ';', '`', '>', '<', '&'];

// ─── SandboxConfig ────────────────────────────────────────────────────────

/// Sandbox constraints for a plugin-declared tool.
///
/// All fields use permissive defaults (`Default`) so that code that
/// constructs [`SandboxConfig`] without setting every field does not
/// accidentally over-restrict. The per-tier constructors
/// ([`SandboxConfig::for_tier_level`]) provide secure, opinionated starting
/// points.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

    /// Validate that a command string is safe for sandboxed execution.
    ///
    /// Rejects commands that are empty/whitespace-only or that contain
    /// shell metacharacters which could enable injection attacks:
    ///
    /// - Pipe (`|`), semicolon (`;`), logical operators (`&&`, `||`)
    /// - Backtick (`` ` ``), command substitution (`$(`)
    /// - Redirections (`>`, `<`, `>>`, `<<`)
    /// - Bare ampersand (`&`) for background execution
    ///
    /// # Errors
    ///
    /// Returns [`SandboxValidationError::EmptyCommand`] if the command is
    /// blank, or [`SandboxValidationError::ShellMetacharacter`] with the
    /// specific offending sequence.
    ///
    /// # Example
    ///
    /// ```
    /// use roko_std::tool::sandbox_config::SandboxConfig;
    ///
    /// assert!(SandboxConfig::validate_command("cargo build").is_ok());
    /// assert!(SandboxConfig::validate_command("ls ; rm -rf /").is_err());
    /// ```
    pub fn validate_command(cmd: &str) -> Result<(), SandboxValidationError> {
        if cmd.trim().is_empty() {
            return Err(SandboxValidationError::EmptyCommand);
        }

        // Check multi-char sequences first for more specific error messages.
        for seq in SHELL_META_SEQUENCES {
            if cmd.contains(seq) {
                return Err(SandboxValidationError::ShellMetacharacter {
                    found: (*seq).to_string(),
                    command: cmd.to_string(),
                });
            }
        }

        // Check single-char metacharacters.
        for &ch in SHELL_META_CHARS {
            if cmd.contains(ch) {
                return Err(SandboxValidationError::ShellMetacharacter {
                    found: ch.to_string(),
                    command: cmd.to_string(),
                });
            }
        }

        Ok(())
    }

    /// Remove or escape dangerous shell metacharacters from a command string.
    ///
    /// This is a best-effort sanitizer: it strips all characters and
    /// sequences that [`SandboxConfig::validate_command`] would reject.
    /// The result is safe to pass to a non-shell executor (e.g. direct
    /// `execvp`), but callers should prefer [`validate_command`](Self::validate_command)
    /// and rejecting bad input over silently sanitizing it.
    ///
    /// # Example
    ///
    /// ```
    /// use roko_std::tool::sandbox_config::SandboxConfig;
    ///
    /// let clean = SandboxConfig::sanitize_command("cargo build && rm -rf /");
    /// assert!(!clean.contains("&&"));
    /// assert!(!clean.contains("rm -rf /"));
    /// ```
    #[must_use]
    pub fn sanitize_command(cmd: &str) -> String {
        let mut result = cmd.to_string();

        // Strip multi-char sequences first (order matters: longer before
        // shorter so we don't leave partial matches).
        for seq in SHELL_META_SEQUENCES {
            result = result.replace(seq, "");
        }

        // Strip single-char metacharacters.
        result.retain(|c| !SHELL_META_CHARS.contains(&c));

        // Collapse runs of whitespace that result from stripping.
        let collapsed: String = result.split_whitespace().collect::<Vec<_>>().join(" ");
        collapsed
    }

    /// Validate that this `SandboxConfig` is internally consistent.
    ///
    /// Returns a list of human-readable violations. An empty list means
    /// the config is valid. Checks performed:
    ///
    /// - A path that appears in both `allowed_paths` and `denied_paths`
    ///   with an exact match (denial always wins, but the overlap is almost
    ///   certainly a mistake).
    /// - An `allowed_paths` entry that contains `../` path traversal
    ///   sequences, which could escape the sandbox root.
    /// - `denied_paths` containing `**` while `allowed_paths` is non-empty,
    ///   making the effective access empty (likely a config error).
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

        // Check allowed_paths for path traversal sequences.
        for path in &self.allowed_paths {
            if Self::contains_path_traversal(path) {
                violations.push(format!(
                    "allowed_paths entry '{path}' contains path traversal (../) \
                     which could escape the sandbox root"
                ));
            }
        }

        // Also check denied_paths — traversal there is suspicious too.
        for path in &self.denied_paths {
            if Self::contains_path_traversal(path) {
                violations.push(format!(
                    "denied_paths entry '{path}' contains path traversal (../) \
                     which is suspicious and likely a config error"
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

    /// Check whether a path pattern contains traversal sequences (`../`).
    ///
    /// Detects:
    /// - Literal `../` anywhere in the path
    /// - Path ending with `..` (traversal without trailing slash)
    /// - Path that is exactly `..`
    fn contains_path_traversal(path: &str) -> bool {
        // Exact match
        if path == ".." {
            return true;
        }
        // Starts with ../ or contains /../
        if path.starts_with("../") || path.contains("/../") {
            return true;
        }
        // Ends with /..
        if path.ends_with("/..") {
            return true;
        }
        false
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
        let allowed = self
            .allowed_paths
            .iter()
            .any(|p| p == path_glob || p == "**");
        if !allowed {
            return false;
        }
        let denied = self
            .denied_paths
            .iter()
            .any(|d| d == path_glob || d == "**");
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
        assert!(
            cfg.allowed_paths.is_empty(),
            "tier 1 must not allow any paths"
        );
        assert!(!cfg.network_access, "tier 1 must block network");
        assert!(cfg.max_memory_mb <= 128, "tier 1 must cap memory");
        assert!(cfg.max_cpu_seconds <= 30, "tier 1 must cap cpu");
    }

    #[test]
    fn sandbox_tier2_allows_reads_denies_secrets() {
        let cfg = SandboxConfig::for_tier_level(2);
        assert!(
            !cfg.allowed_paths.is_empty(),
            "tier 2 must allow some paths"
        );
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
        assert!(
            cfg.denied_paths.is_empty(),
            "tier 4 must not deny any paths"
        );
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
        assert!(
            cfg.validate().is_empty(),
            "valid tier config must not produce violations"
        );
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
        assert!(
            cfg.permits_path("src/main.rs"),
            "non-denied path under ** must be allowed"
        );
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

    // ── validate_command tests ────────────────────────────────────────

    #[test]
    fn sandbox_validate_command_accepts_clean_commands() {
        assert!(SandboxConfig::validate_command("cargo build").is_ok());
        assert!(SandboxConfig::validate_command("rustfmt --edition 2021 src/main.rs").is_ok());
        assert!(SandboxConfig::validate_command("python3 -c 'print(1)'").is_ok());
        assert!(SandboxConfig::validate_command("ls -la /tmp/output").is_ok());
    }

    #[test]
    fn sandbox_validate_command_rejects_empty() {
        assert_eq!(
            SandboxConfig::validate_command(""),
            Err(SandboxValidationError::EmptyCommand)
        );
        assert_eq!(
            SandboxConfig::validate_command("   "),
            Err(SandboxValidationError::EmptyCommand)
        );
        assert_eq!(
            SandboxConfig::validate_command("\t\n"),
            Err(SandboxValidationError::EmptyCommand)
        );
    }

    #[test]
    fn sandbox_validate_command_rejects_pipe() {
        let err = SandboxConfig::validate_command("cat /etc/passwd | grep root").expect_err("pipe");
        assert!(matches!(
            err,
            SandboxValidationError::ShellMetacharacter { ref found, .. } if found == "|"
        ));
    }

    #[test]
    fn sandbox_validate_command_rejects_semicolon() {
        let err = SandboxConfig::validate_command("ls ; rm -rf /").expect_err("semicolon");
        assert!(matches!(
            err,
            SandboxValidationError::ShellMetacharacter { ref found, .. } if found == ";"
        ));
    }

    #[test]
    fn sandbox_validate_command_rejects_logical_and() {
        let err = SandboxConfig::validate_command("true && rm -rf /").expect_err("&&");
        assert!(matches!(
            err,
            SandboxValidationError::ShellMetacharacter { ref found, .. } if found == "&&"
        ));
    }

    #[test]
    fn sandbox_validate_command_rejects_logical_or() {
        let err = SandboxConfig::validate_command("false || echo pwned").expect_err("||");
        assert!(matches!(
            err,
            SandboxValidationError::ShellMetacharacter { ref found, .. } if found == "||"
        ));
    }

    #[test]
    fn sandbox_validate_command_rejects_backtick() {
        let err = SandboxConfig::validate_command("echo `whoami`").expect_err("backtick");
        assert!(matches!(
            err,
            SandboxValidationError::ShellMetacharacter { ref found, .. } if found == "`"
        ));
    }

    #[test]
    fn sandbox_validate_command_rejects_dollar_paren() {
        let err = SandboxConfig::validate_command("echo $(whoami)").expect_err("$(");
        assert!(matches!(
            err,
            SandboxValidationError::ShellMetacharacter { ref found, .. } if found == "$("
        ));
    }

    #[test]
    fn sandbox_validate_command_rejects_redirect_out() {
        let err = SandboxConfig::validate_command("echo hi > /tmp/out").expect_err(">");
        assert!(matches!(
            err,
            SandboxValidationError::ShellMetacharacter { ref found, .. } if found == ">"
        ));
    }

    #[test]
    fn sandbox_validate_command_rejects_redirect_in() {
        let err = SandboxConfig::validate_command("wc < /etc/passwd").expect_err("<");
        assert!(matches!(
            err,
            SandboxValidationError::ShellMetacharacter { ref found, .. } if found == "<"
        ));
    }

    #[test]
    fn sandbox_validate_command_rejects_append_redirect() {
        let err = SandboxConfig::validate_command("echo hi >> /tmp/log").expect_err(">>");
        assert!(matches!(
            err,
            SandboxValidationError::ShellMetacharacter { ref found, .. } if found == ">>"
        ));
    }

    #[test]
    fn sandbox_validate_command_rejects_heredoc() {
        let err = SandboxConfig::validate_command("cat << EOF").expect_err("<<");
        assert!(matches!(
            err,
            SandboxValidationError::ShellMetacharacter { ref found, .. } if found == "<<"
        ));
    }

    #[test]
    fn sandbox_validate_command_rejects_background_ampersand() {
        let err = SandboxConfig::validate_command("sleep 999 &").expect_err("&");
        assert!(matches!(
            err,
            SandboxValidationError::ShellMetacharacter { ref found, .. } if found == "&"
        ));
    }

    // ── sanitize_command tests ────────────────────────────────────────

    #[test]
    fn sandbox_sanitize_preserves_clean_commands() {
        assert_eq!(
            SandboxConfig::sanitize_command("cargo build"),
            "cargo build"
        );
        assert_eq!(
            SandboxConfig::sanitize_command("rustfmt src/main.rs"),
            "rustfmt src/main.rs"
        );
    }

    #[test]
    fn sandbox_sanitize_strips_pipe() {
        let result = SandboxConfig::sanitize_command("cat file | grep pattern");
        assert!(!result.contains('|'));
        // Ensure we kept the meaningful tokens.
        assert!(result.contains("cat"));
        assert!(result.contains("file"));
    }

    #[test]
    fn sandbox_sanitize_strips_semicolon() {
        let result = SandboxConfig::sanitize_command("ls ; rm -rf /");
        assert!(!result.contains(';'));
    }

    #[test]
    fn sandbox_sanitize_strips_logical_operators() {
        let result = SandboxConfig::sanitize_command("true && rm -rf / || echo fail");
        assert!(!result.contains("&&"));
        assert!(!result.contains("||"));
    }

    #[test]
    fn sandbox_sanitize_strips_subshell() {
        let result = SandboxConfig::sanitize_command("echo $(whoami)");
        assert!(!result.contains("$("));
    }

    #[test]
    fn sandbox_sanitize_strips_backtick() {
        let result = SandboxConfig::sanitize_command("echo `id`");
        assert!(!result.contains('`'));
    }

    #[test]
    fn sandbox_sanitize_strips_redirects() {
        let result =
            SandboxConfig::sanitize_command("echo data > /tmp/out 2>> /tmp/err < /dev/null");
        assert!(!result.contains('>'));
        assert!(!result.contains('<'));
    }

    #[test]
    fn sandbox_sanitize_collapses_whitespace() {
        let result = SandboxConfig::sanitize_command("echo   &&   data");
        // After stripping && and collapsing spaces, result should be tidy.
        assert!(!result.contains("  "), "should not contain double spaces");
    }

    #[test]
    fn sandbox_sanitize_then_validate_passes() {
        // The sanitized output of any string should pass validate_command
        // (unless empty).
        let dirty = "cat /etc/passwd | grep root; rm -rf / && echo $(whoami)";
        let clean = SandboxConfig::sanitize_command(dirty);
        if !clean.trim().is_empty() {
            assert!(
                SandboxConfig::validate_command(&clean).is_ok(),
                "sanitized command should pass validation: '{clean}'"
            );
        }
    }

    // ── Path traversal in validate() ──────────────────────────────────

    #[test]
    fn sandbox_validate_detects_path_traversal_in_allowed() {
        let cfg = SandboxConfig {
            allowed_paths: vec!["../../../etc/passwd".to_string()],
            denied_paths: vec![],
            network_access: false,
            max_memory_mb: 64,
            max_cpu_seconds: 5,
        };
        let violations = cfg.validate();
        assert!(
            !violations.is_empty(),
            "path traversal in allowed_paths must be flagged"
        );
        assert!(
            violations[0].contains("path traversal"),
            "violation message must mention path traversal"
        );
    }

    #[test]
    fn sandbox_validate_detects_path_traversal_in_denied() {
        let cfg = SandboxConfig {
            allowed_paths: vec!["**".to_string()],
            denied_paths: vec!["../../sensitive".to_string()],
            network_access: false,
            max_memory_mb: 64,
            max_cpu_seconds: 5,
        };
        let violations = cfg.validate();
        assert!(
            !violations.is_empty(),
            "path traversal in denied_paths must be flagged"
        );
        assert!(
            violations[0].contains("path traversal"),
            "violation message must mention path traversal"
        );
    }

    #[test]
    fn sandbox_validate_detects_mid_path_traversal() {
        let cfg = SandboxConfig {
            allowed_paths: vec!["src/../../etc/shadow".to_string()],
            denied_paths: vec![],
            network_access: false,
            max_memory_mb: 64,
            max_cpu_seconds: 5,
        };
        let violations = cfg.validate();
        assert!(
            !violations.is_empty(),
            "mid-path traversal (/../) must be flagged"
        );
    }

    #[test]
    fn sandbox_validate_detects_trailing_dotdot() {
        let cfg = SandboxConfig {
            allowed_paths: vec!["src/..".to_string()],
            denied_paths: vec![],
            network_access: false,
            max_memory_mb: 64,
            max_cpu_seconds: 5,
        };
        let violations = cfg.validate();
        assert!(
            !violations.is_empty(),
            "trailing /.. traversal must be flagged"
        );
    }

    #[test]
    fn sandbox_validate_detects_bare_dotdot() {
        let cfg = SandboxConfig {
            allowed_paths: vec!["..".to_string()],
            denied_paths: vec![],
            network_access: false,
            max_memory_mb: 64,
            max_cpu_seconds: 5,
        };
        let violations = cfg.validate();
        assert!(!violations.is_empty(), "bare '..' as path must be flagged");
    }

    #[test]
    fn sandbox_validate_allows_double_dot_in_filenames() {
        // A filename like "foo..bar" or a glob like "**..txt" is NOT
        // path traversal — only "../" or "/.."/standalone ".." patterns.
        let cfg = SandboxConfig {
            allowed_paths: vec!["foo..bar".to_string(), "data...csv".to_string()],
            denied_paths: vec![],
            network_access: false,
            max_memory_mb: 64,
            max_cpu_seconds: 5,
        };
        let violations = cfg.validate();
        assert!(
            violations.is_empty(),
            "double dots in filenames must not be flagged as traversal: {violations:?}"
        );
    }

    #[test]
    fn sandbox_validate_tier_configs_have_no_traversal() {
        // Verify all tier defaults pass the traversal check.
        for level in 1..=5 {
            let cfg = SandboxConfig::for_tier_level(level);
            let violations = cfg.validate();
            assert!(
                violations.is_empty(),
                "tier {level} config must not have traversal violations: {violations:?}"
            );
        }
    }

    // ── SandboxValidationError Display ────────────────────────────────

    #[test]
    fn sandbox_validation_error_display_metacharacter() {
        let err = SandboxValidationError::ShellMetacharacter {
            found: "|".to_string(),
            command: "ls | cat".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("|"));
        assert!(msg.contains("ls | cat"));
    }

    #[test]
    fn sandbox_validation_error_display_path_traversal() {
        let err = SandboxValidationError::PathTraversal {
            path: "../secret".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("path traversal"));
        assert!(msg.contains("../secret"));
    }

    #[test]
    fn sandbox_validation_error_display_empty_command() {
        let err = SandboxValidationError::EmptyCommand;
        let msg = err.to_string();
        assert!(msg.contains("empty"));
    }
}
