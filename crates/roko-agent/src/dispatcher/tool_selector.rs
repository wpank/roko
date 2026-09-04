//! Role-based tool selection for the ToolDispatcher (AGT-09).
//!
//! Narrows the tool set exposed to an agent based on its role, reducing noise
//! and improving accuracy. Research shows agents perform worse when exposed to
//! >50 tools (Qwen3-coder format switching, WildToolBench <15% session accuracy).
//!
//! The selector applies a role -> tool-set mapping and optionally consults
//! learned [`ToolUsageProfile`] data to further refine the set.

use roko_core::AgentRole;
use std::collections::HashSet;

/// A tool selector that filters available tools based on role and task metadata.
///
/// # Usage
///
/// ```ignore
/// let selector = ToolSelector::for_role(AgentRole::Implementer);
/// let allowed = selector.filter_tools(&all_tool_names);
/// ```
#[derive(Debug, Clone)]
pub struct ToolSelector {
    /// Base tool set for the role.
    base_tools: HashSet<String>,
    /// Additional tools explicitly granted (e.g., from task config).
    extra_tools: HashSet<String>,
    /// Tools explicitly blocked (takes priority over everything).
    blocked_tools: HashSet<String>,
}

impl ToolSelector {
    /// Create a selector for the given agent role.
    ///
    /// Uses built-in role -> tool-set mappings. All roles get the
    /// read-only baseline; write/exec tools are added by role.
    #[must_use]
    pub fn for_role(role: AgentRole) -> Self {
        Self {
            base_tools: tools_for_role(role),
            extra_tools: HashSet::new(),
            blocked_tools: HashSet::new(),
        }
    }

    /// Create a selector that allows all tools (no filtering).
    #[must_use]
    pub fn allow_all() -> Self {
        Self {
            base_tools: HashSet::new(),
            extra_tools: HashSet::new(),
            blocked_tools: HashSet::new(),
        }
    }

    /// Grant additional tools beyond the role baseline.
    #[must_use]
    pub fn with_extra_tools(mut self, tools: impl IntoIterator<Item = String>) -> Self {
        self.extra_tools.extend(tools);
        self
    }

    /// Block specific tools (overrides base and extra).
    #[must_use]
    pub fn with_blocked_tools(mut self, tools: impl IntoIterator<Item = String>) -> Self {
        self.blocked_tools.extend(tools);
        self
    }

    /// Check whether a specific tool is allowed by this selector.
    ///
    /// The incoming `tool_name` is first resolved through
    /// [`canonicalize_tool_name`] so that common aliases (e.g. `read` →
    /// `read_file`, `list_dir` → `ls`) are accepted transparently.
    ///
    /// Returns `true` if:
    /// - The base set is empty (allow-all mode), OR
    /// - The tool is in `base_tools` or `extra_tools`
    /// AND the tool is NOT in `blocked_tools`.
    #[must_use]
    pub fn is_allowed(&self, tool_name: &str) -> bool {
        let canonical = canonicalize_tool_name(tool_name);
        if self.blocked_tools.contains(canonical) {
            return false;
        }
        if self.base_tools.is_empty() && self.extra_tools.is_empty() {
            return true; // allow-all mode
        }
        self.base_tools.contains(canonical) || self.extra_tools.contains(canonical)
    }

    /// Filter a list of tool names, returning only allowed tools.
    pub fn filter_tools<'a>(&self, tools: &[&'a str]) -> Vec<&'a str> {
        tools
            .iter()
            .copied()
            .filter(|t| self.is_allowed(t))
            .collect()
    }

    /// Number of tools in the base set (0 means allow-all).
    #[must_use]
    pub fn base_tool_count(&self) -> usize {
        self.base_tools.len()
    }
}

// ── Alias resolution ────────────────────────────────────────────────────

/// Map common agent tool-name variants to their canonical roko-std names.
///
/// Providers emit varying tool names (Claude's `Read` becomes `read`,
/// OpenAI-compatible backends use `list_dir`, etc.). Resolving here means
/// the selector and the downstream dispatcher both see the canonical name.
///
/// Returns the canonical name if `name` is a known alias, otherwise returns
/// the input unchanged.
pub fn canonicalize_tool_name(name: &str) -> &str {
    match name {
        // Read-only aliases
        "read" => "read_file",
        "list_directory" | "list_dir" => "ls",
        "search_files" => "grep",
        "filesystem__read_file" => "read_file",

        // Write aliases
        "write" => "write_file",
        "edit" => "edit_file",
        "create_file" => "write_file",

        // Exec aliases
        "run_command" | "execute_command" | "shell" => "bash",

        _ => name,
    }
}

// ── Read-only tools (available to ALL roles) ────────────────────────────

fn read_only_tools() -> HashSet<String> {
    ["read_file", "glob", "grep", "ls"]
        .iter()
        .map(|s| (*s).to_string())
        .collect()
}

// ── Write tools (coding agents) ─────────────────────────────────────────

fn write_tools() -> HashSet<String> {
    ["write_file", "edit_file", "multi_edit"]
        .iter()
        .map(|s| (*s).to_string())
        .collect()
}

// ── Exec tools (agents that run commands) ───────────────────────────────

fn exec_tools() -> HashSet<String> {
    ["bash"]
        .iter()
        .map(|s| (*s).to_string())
        .collect()
}

// ── Git tools ───────────────────────────────────────────────────────────

fn git_tools() -> HashSet<String> {
    ["git_status", "git_diff", "git_log", "git_commit"]
        .iter()
        .map(|s| (*s).to_string())
        .collect()
}

/// Map a role to its base tool set.
///
/// Follows the principle of least privilege: each role gets only the tools
/// it needs. Read-only tools are universal; write/exec are added per role.
fn tools_for_role(role: AgentRole) -> HashSet<String> {
    let mut tools = read_only_tools();

    match role {
        // Coding roles: full read + write + exec.
        AgentRole::Implementer | AgentRole::AutoFixer | AgentRole::Refactorer => {
            tools.extend(write_tools());
            tools.extend(exec_tools());
            tools.extend(git_tools());
        }

        // Strategy/planning roles: read-only + limited exec for cargo check.
        AgentRole::Strategist | AgentRole::PrePlanner => {
            tools.extend(exec_tools());
        }

        // Review roles: read-only + exec for running tests.
        AgentRole::Architect
        | AgentRole::Auditor
        | AgentRole::QuickReviewer
        | AgentRole::Critic => {
            tools.extend(exec_tools());
        }

        // Research role: read-only (no writes, no exec).
        AgentRole::Researcher => {
            // read_only_tools only
        }

        // Documentation roles: read + write (for docs), limited exec.
        AgentRole::Scribe | AgentRole::DocVerifier => {
            tools.extend(write_tools());
            tools.extend(exec_tools());
        }

        // Test roles: read + exec (need to run tests, not write code).
        AgentRole::IntegrationTester
        | AgentRole::TerminalValidator
        | AgentRole::GolemLifecycleTester
        | AgentRole::CrossSystemTester
        | AgentRole::FullLoopValidator => {
            tools.extend(exec_tools());
        }

        // Conductor: read-only (orchestration, not implementation).
        AgentRole::Conductor => {
            // read_only_tools only
        }

        // Merge resolver: full access needed for conflict resolution.
        AgentRole::MergeResolver => {
            tools.extend(write_tools());
            tools.extend(exec_tools());
            tools.extend(git_tools());
        }

        // Read-only analysis roles: detect drift, regressions, patterns.
        // These roles only observe and report — no writes, no exec.
        AgentRole::SpecDriftDetector
        | AgentRole::RegressionDetector
        | AgentRole::PerformanceSentinel
        | AgentRole::CoverageTracker
        | AgentRole::PatternExtractor
        | AgentRole::SnapshotComparator => {
            // read_only_tools only
        }

        // Plan lifecycle manager: read + exec for plan state transitions.
        AgentRole::PlanLifecycleManager => {
            tools.extend(exec_tools());
        }

        // Diagnostic roles: read + limited exec for investigation.
        AgentRole::ErrorDiagnoser | AgentRole::DependencyValidator => {
            tools.extend(exec_tools());
        }

        // Future roles: read-only by default (deny-by-default, NOT allow-all).
        // AgentRole is #[non_exhaustive], so this arm is required.
        // If a new AgentRole variant needs more than read-only access,
        // add an explicit arm above.
        _ => {
            // read_only_tools already in `tools` — no escalation.
        }
    }

    tools
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn implementer_has_write_and_exec() {
        let selector = ToolSelector::for_role(AgentRole::Implementer);
        assert!(selector.is_allowed("read_file"));
        assert!(selector.is_allowed("write_file"));
        assert!(selector.is_allowed("bash"));
        assert!(selector.is_allowed("git_status"));
    }

    #[test]
    fn researcher_has_read_only() {
        let selector = ToolSelector::for_role(AgentRole::Researcher);
        assert!(selector.is_allowed("read_file"));
        assert!(selector.is_allowed("grep"));
        assert!(!selector.is_allowed("write_file"));
        assert!(!selector.is_allowed("bash"));
        assert!(!selector.is_allowed("git_commit"));
    }

    #[test]
    fn conductor_has_read_only() {
        let selector = ToolSelector::for_role(AgentRole::Conductor);
        assert!(selector.is_allowed("read_file"));
        assert!(!selector.is_allowed("write_file"));
        assert!(!selector.is_allowed("bash"));
    }

    #[test]
    fn reviewer_has_read_and_exec() {
        let selector = ToolSelector::for_role(AgentRole::Architect);
        assert!(selector.is_allowed("read_file"));
        assert!(selector.is_allowed("bash"));
        assert!(!selector.is_allowed("write_file"));
    }

    #[test]
    fn blocked_tools_override_base() {
        let selector = ToolSelector::for_role(AgentRole::Implementer)
            .with_blocked_tools(vec!["bash".to_string()]);
        assert!(selector.is_allowed("read_file"));
        assert!(selector.is_allowed("write_file"));
        assert!(!selector.is_allowed("bash"));
    }

    #[test]
    fn extra_tools_extend_base() {
        let selector = ToolSelector::for_role(AgentRole::Researcher)
            .with_extra_tools(vec!["custom_search".to_string()]);
        assert!(selector.is_allowed("read_file"));
        assert!(selector.is_allowed("custom_search"));
        assert!(!selector.is_allowed("write_file"));
    }

    #[test]
    fn allow_all_allows_everything() {
        let selector = ToolSelector::allow_all();
        assert!(selector.is_allowed("read_file"));
        assert!(selector.is_allowed("write_file"));
        assert!(selector.is_allowed("bash"));
        assert!(selector.is_allowed("any_random_tool"));
    }

    #[test]
    fn filter_tools_returns_subset() {
        let selector = ToolSelector::for_role(AgentRole::Researcher);
        let all = &["read_file", "write_file", "grep", "bash", "glob"];
        let filtered = selector.filter_tools(all);
        assert_eq!(filtered, vec!["read_file", "grep", "glob"]);
    }

    #[test]
    fn all_roles_have_read_access() {
        let roles = [
            AgentRole::Conductor,
            AgentRole::Strategist,
            AgentRole::Implementer,
            AgentRole::Architect,
            AgentRole::Researcher,
            AgentRole::Auditor,
            AgentRole::Scribe,
            AgentRole::Refactorer,
            AgentRole::AutoFixer,
            AgentRole::PrePlanner,
            AgentRole::DocVerifier,
            AgentRole::IntegrationTester,
            AgentRole::MergeResolver,
            AgentRole::QuickReviewer,
            AgentRole::Critic,
            AgentRole::TerminalValidator,
            AgentRole::GolemLifecycleTester,
            AgentRole::SpecDriftDetector,
            AgentRole::RegressionDetector,
            AgentRole::PerformanceSentinel,
            AgentRole::CoverageTracker,
            AgentRole::PlanLifecycleManager,
            AgentRole::CrossSystemTester,
            AgentRole::ErrorDiagnoser,
            AgentRole::DependencyValidator,
            AgentRole::PatternExtractor,
            AgentRole::SnapshotComparator,
            AgentRole::FullLoopValidator,
        ];
        for role in roles {
            let selector = ToolSelector::for_role(role);
            assert!(
                selector.is_allowed("read_file"),
                "{role:?} should have read_file access"
            );
        }
    }

    #[test]
    fn read_only_roles_cannot_write_or_exec() {
        let read_only_roles = [
            AgentRole::SpecDriftDetector,
            AgentRole::RegressionDetector,
            AgentRole::PerformanceSentinel,
            AgentRole::CoverageTracker,
            AgentRole::PatternExtractor,
            AgentRole::SnapshotComparator,
        ];
        for role in read_only_roles {
            let selector = ToolSelector::for_role(role);
            assert!(
                selector.is_allowed("read_file"),
                "{role:?} should have read_file access"
            );
            assert!(
                !selector.is_allowed("write_file"),
                "{role:?} should NOT have write_file access"
            );
            assert!(
                !selector.is_allowed("bash"),
                "{role:?} should NOT have bash access"
            );
        }
    }

    #[test]
    fn test_roles_have_exec_but_not_write() {
        let test_roles = [
            AgentRole::TerminalValidator,
            AgentRole::GolemLifecycleTester,
            AgentRole::CrossSystemTester,
            AgentRole::FullLoopValidator,
        ];
        for role in test_roles {
            let selector = ToolSelector::for_role(role);
            assert!(
                selector.is_allowed("read_file"),
                "{role:?} should have read_file access"
            );
            assert!(
                selector.is_allowed("bash"),
                "{role:?} should have bash access"
            );
            assert!(
                !selector.is_allowed("write_file"),
                "{role:?} should NOT have write_file access"
            );
        }
    }

    #[test]
    fn diagnostic_roles_have_exec_but_not_write() {
        let diag_roles = [AgentRole::ErrorDiagnoser, AgentRole::DependencyValidator];
        for role in diag_roles {
            let selector = ToolSelector::for_role(role);
            assert!(
                selector.is_allowed("read_file"),
                "{role:?} should have read_file access"
            );
            assert!(
                selector.is_allowed("bash"),
                "{role:?} should have bash access"
            );
            assert!(
                !selector.is_allowed("write_file"),
                "{role:?} should NOT have write_file access"
            );
        }
    }

    #[test]
    fn no_role_returns_empty_set() {
        // Verify that no role falls through to an allow-all empty set.
        // Every variant of AgentRole must return a non-empty base tool set.
        let all_roles = [
            AgentRole::Conductor,
            AgentRole::Strategist,
            AgentRole::Implementer,
            AgentRole::Architect,
            AgentRole::Researcher,
            AgentRole::Auditor,
            AgentRole::QuickReviewer,
            AgentRole::Scribe,
            AgentRole::Critic,
            AgentRole::AutoFixer,
            AgentRole::Refactorer,
            AgentRole::PrePlanner,
            AgentRole::DocVerifier,
            AgentRole::IntegrationTester,
            AgentRole::MergeResolver,
            AgentRole::TerminalValidator,
            AgentRole::GolemLifecycleTester,
            AgentRole::SpecDriftDetector,
            AgentRole::RegressionDetector,
            AgentRole::PerformanceSentinel,
            AgentRole::CoverageTracker,
            AgentRole::PlanLifecycleManager,
            AgentRole::CrossSystemTester,
            AgentRole::ErrorDiagnoser,
            AgentRole::DependencyValidator,
            AgentRole::PatternExtractor,
            AgentRole::SnapshotComparator,
            AgentRole::FullLoopValidator,
        ];
        for role in all_roles {
            let selector = ToolSelector::for_role(role);
            assert!(
                selector.base_tool_count() > 0,
                "{role:?} must have a non-empty base tool set (empty = allow-all = security hole)"
            );
        }
    }

    #[test]
    fn base_tool_count_reflects_role() {
        let researcher = ToolSelector::for_role(AgentRole::Researcher);
        let implementer = ToolSelector::for_role(AgentRole::Implementer);
        // Implementer should have more tools than researcher.
        assert!(implementer.base_tool_count() > researcher.base_tool_count());
    }

    // ── Alias / canonicalization tests ──────────────────────────────────

    #[test]
    fn canonicalize_maps_known_aliases() {
        assert_eq!(canonicalize_tool_name("read"), "read_file");
        assert_eq!(canonicalize_tool_name("list_directory"), "ls");
        assert_eq!(canonicalize_tool_name("list_dir"), "ls");
        assert_eq!(canonicalize_tool_name("search_files"), "grep");
        assert_eq!(canonicalize_tool_name("write"), "write_file");
        assert_eq!(canonicalize_tool_name("edit"), "edit_file");
        assert_eq!(canonicalize_tool_name("create_file"), "write_file");
        assert_eq!(canonicalize_tool_name("run_command"), "bash");
        assert_eq!(canonicalize_tool_name("execute_command"), "bash");
        assert_eq!(canonicalize_tool_name("shell"), "bash");
        assert_eq!(
            canonicalize_tool_name("filesystem__read_file"),
            "read_file"
        );
    }

    #[test]
    fn canonicalize_passes_through_canonical_names() {
        for name in ["read_file", "write_file", "edit_file", "grep", "ls", "bash", "glob"] {
            assert_eq!(canonicalize_tool_name(name), name);
        }
    }

    #[test]
    fn canonicalize_passes_through_unknown_names() {
        assert_eq!(canonicalize_tool_name("my_custom_tool"), "my_custom_tool");
        assert_eq!(canonicalize_tool_name("web_fetch"), "web_fetch");
    }

    #[test]
    fn is_allowed_resolves_aliases() {
        let selector = ToolSelector::for_role(AgentRole::Researcher);
        // "read" should resolve to "read_file" and be allowed.
        assert!(selector.is_allowed("read"));
        // "list_directory" should resolve to "ls" and be allowed.
        assert!(selector.is_allowed("list_directory"));
        // "list_dir" should resolve to "ls" and be allowed.
        assert!(selector.is_allowed("list_dir"));
        // "search_files" should resolve to "grep" and be allowed.
        assert!(selector.is_allowed("search_files"));
        // "filesystem__read_file" should resolve to "read_file" and be allowed.
        assert!(selector.is_allowed("filesystem__read_file"));
    }

    #[test]
    fn is_allowed_denies_write_aliases_for_read_only_role() {
        let selector = ToolSelector::for_role(AgentRole::Researcher);
        // "write" resolves to "write_file" — still denied for researchers.
        assert!(!selector.is_allowed("write"));
        // "edit" resolves to "edit_file" — still denied.
        assert!(!selector.is_allowed("edit"));
        // "run_command" resolves to "bash" — still denied.
        assert!(!selector.is_allowed("run_command"));
    }

    #[test]
    fn is_allowed_permits_write_aliases_for_implementer() {
        let selector = ToolSelector::for_role(AgentRole::Implementer);
        assert!(selector.is_allowed("write"));
        assert!(selector.is_allowed("edit"));
        assert!(selector.is_allowed("create_file"));
        assert!(selector.is_allowed("run_command"));
        assert!(selector.is_allowed("execute_command"));
        assert!(selector.is_allowed("shell"));
    }

    #[test]
    fn blocked_tools_apply_to_aliases() {
        let selector = ToolSelector::for_role(AgentRole::Implementer)
            .with_blocked_tools(vec!["bash".to_string()]);
        // "run_command" canonicalizes to "bash" which is blocked.
        assert!(!selector.is_allowed("run_command"));
        assert!(!selector.is_allowed("shell"));
        assert!(!selector.is_allowed("execute_command"));
    }
}
