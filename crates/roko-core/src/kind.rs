//! Engram kinds — what a signal represents.
//!
//! A signal's [`Kind`] tells consumers how to interpret its body. Kinds are
//! grouped by architectural concern (agent runtime, verification, context,
//! memory, chain). The enum is `#[non_exhaustive]` and has a `Custom(String)`
//! escape hatch so extensions can define their own kinds without modifying
//! this crate.

use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt;

/// The category of a signal. Determines how its body should be interpreted.
///
/// Kinds are the switchyard for dispatch: a [`Verify`](crate::Verify) might only
/// verify signals of kind `GateVerdict`, a [`Compose`](crate::Compose) might
/// only combine `PromptSection` signals, etc.
///
/// # Extensibility
///
/// The enum is `#[non_exhaustive]`. To add a new kind without modifying this
/// file, use `Kind::Custom("my.custom.kind".into())`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    // ─── Agent runtime ───────────────────────────────────────────────────
    /// A process was spawned (LLM subprocess, sandbox, etc.).
    ProcessSpawn,
    /// A process exited.
    ProcessExit,
    /// A message chunk from an agent's stream.
    AgentMessage,
    /// Raw stdout/stderr output from an agent.
    AgentOutput,
    /// Token usage report from an LLM call.
    TokenUsage,
    /// An agent requested approval for a destructive operation.
    ApprovalRequested,

    // ─── Verification ────────────────────────────────────────────────────
    /// A gate passed or failed a check.
    GateVerdict,
    /// A test suite run result.
    TestResult,
    /// A compile diagnostic (error or warning).
    CompileDiagnostic,

    // ─── Tasks & plans ───────────────────────────────────────────────────
    /// A task description (input to an agent).
    Task,
    /// A plan (collection of tasks with dependencies).
    Plan,
    /// A plan transitioned phases.
    PlanPhase,

    // ─── Context assembly ────────────────────────────────────────────────
    /// A single section within an assembled prompt.
    PromptSection,
    /// A curated bundle of context for an agent.
    ContextPack,
    /// A fully-assembled prompt ready for an LLM.
    Prompt,

    // ─── Routing & learning ──────────────────────────────────────────────
    /// A router's decision (e.g. "use Claude for this task").
    RouterChoice,
    /// Feedback about a prior router choice (reward signal).
    RouterFeedback,

    // ─── Memory ──────────────────────────────────────────────────────────
    /// A logged episode of an agent run.
    Episode,
    /// A playbook rule extracted from patterns.
    PlaybookRule,
    /// A learned skill (reusable procedure).
    Skill,
    /// A structural grouping of several kinds under one compound label.
    Compound(Vec<Kind>),

    // ─── Observability ───────────────────────────────────────────────────
    /// A metric reading (scalar measurement).
    Metric,
    /// An experiment result (A/B test outcome).
    ExperimentResult,
    /// A tool invocation record (per-call metrics).
    ToolInvocation,
    /// A tool's health has degraded past an alert threshold.
    ToolHealthDegraded,

    // ─── Chain participation (Phase 8+) ──────────────────────────────────
    /// A chain insight (shared knowledge).
    Insight,
    /// A stigmergic pheromone (threat/opportunity/wisdom).
    Pheromone,
    /// A bounty available for claiming.
    Bounty,
    /// An on-chain transaction.
    Transaction,
    /// A service offering (`OaaS` marketplace).
    Service,
    /// A prediction claim (for predictive foraging).
    Prediction,

    // ─── Extension ───────────────────────────────────────────────────────
    /// Custom kind identified by a dotted string. Extensions should use
    /// a reverse-DNS prefix (e.g. `"com.example.my_kind"`) to avoid collisions.
    Custom(String),
}

impl Kind {
    /// String identifier for this kind, suitable for logs and keys.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::ProcessSpawn => "process_spawn",
            Self::ProcessExit => "process_exit",
            Self::AgentMessage => "agent_message",
            Self::AgentOutput => "agent_output",
            Self::TokenUsage => "token_usage",
            Self::ApprovalRequested => "approval_requested",
            Self::GateVerdict => "gate_verdict",
            Self::TestResult => "test_result",
            Self::CompileDiagnostic => "compile_diagnostic",
            Self::Task => "task",
            Self::Plan => "plan",
            Self::PlanPhase => "plan_phase",
            Self::PromptSection => "prompt_section",
            Self::ContextPack => "context_pack",
            Self::Prompt => "prompt",
            Self::RouterChoice => "router_choice",
            Self::RouterFeedback => "router_feedback",
            Self::Episode => "episode",
            Self::PlaybookRule => "playbook_rule",
            Self::Skill => "skill",
            Self::Compound(_) => "compound",
            Self::Metric => "metric",
            Self::ExperimentResult => "experiment_result",
            Self::ToolInvocation => "tool_invocation",
            Self::ToolHealthDegraded => "tool_health_degraded",
            Self::Insight => "insight",
            Self::Pheromone => "pheromone",
            Self::Bounty => "bounty",
            Self::Transaction => "transaction",
            Self::Service => "service",
            Self::Prediction => "prediction",
            Self::Custom(s) => s.as_str(),
        }
    }

    /// Create a compound kind from multiple constituent kinds.
    ///
    /// Compound kinds represent signals that carry multiple semantics
    /// simultaneously (e.g., a gate verdict that is also a metric reading).
    ///
    /// ```rust
    /// use roko_core::Kind;
    /// let k = Kind::compound(&[Kind::GateVerdict, Kind::Metric]);
    /// assert!(k.contains(&Kind::GateVerdict));
    /// assert!(k.contains(&Kind::Metric));
    /// ```
    #[must_use]
    pub fn compound(parts: &[Kind]) -> Self {
        Self::Compound(parts.to_vec())
    }

    /// Check if this kind matches another, including compound containment.
    ///
    /// - For non-compound kinds: exact equality.
    /// - For compound kinds: returns true if `other` is any constituent.
    /// - Compound-to-compound: true if all parts of `other` are in `self`.
    ///
    /// ```rust
    /// use roko_core::Kind;
    /// let compound = Kind::compound(&[Kind::GateVerdict, Kind::Metric]);
    /// assert!(compound.matches(&Kind::GateVerdict));  // constituent match
    /// assert!(!compound.matches(&Kind::Task));         // not a constituent
    /// assert!(Kind::Task.matches(&Kind::Task));        // exact match
    /// ```
    #[must_use]
    pub fn matches(&self, other: &Kind) -> bool {
        if self == other {
            return true;
        }
        match (self, other) {
            (Self::Compound(parts), Self::Compound(other_parts)) => {
                // All parts of `other` must be in `self`.
                other_parts.iter().all(|op| parts.contains(op))
            }
            (Self::Compound(parts), _) => parts.contains(other),
            (_, Self::Compound(other_parts)) => other_parts.contains(self),
            _ => false,
        }
    }

    /// Check if a compound kind contains a specific constituent.
    ///
    /// Returns false for non-compound kinds (use `matches` for general matching).
    #[must_use]
    pub fn contains(&self, part: &Kind) -> bool {
        match self {
            Self::Compound(parts) => parts.contains(part),
            _ => self == part,
        }
    }

    /// Number of constituents in a compound kind (1 for non-compound).
    #[must_use]
    pub fn arity(&self) -> usize {
        match self {
            Self::Compound(parts) => parts.len(),
            _ => 1,
        }
    }

    /// Iterator over constituent kinds (yields self for non-compound).
    pub fn constituents(&self) -> impl Iterator<Item = &Kind> {
        match self {
            Self::Compound(parts) => parts.iter(),
            _ => std::slice::from_ref(self).iter(),
        }
    }

    /// Whether this is a compound kind.
    #[must_use]
    pub fn is_compound(&self) -> bool {
        matches!(self, Self::Compound(_))
    }

    /// Canonical identity key used when hashing or serializing nested kinds.
    #[must_use]
    pub fn identity_key(&self) -> Cow<'_, str> {
        match self {
            Self::Compound(parts) => {
                let mut identity = String::from("compound(");
                for (index, part) in parts.iter().enumerate() {
                    if index > 0 {
                        identity.push('+');
                    }
                    identity.push_str(part.identity_key().as_ref());
                }
                identity.push(')');
                Cow::Owned(identity)
            }
            _ => Cow::Borrowed(self.as_str()),
        }
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// KindRegistry — runtime validation and hierarchy for Custom kinds
// ─────────────────────────────────────────────────────────────────────────────

/// Entry stored in [`KindRegistry`] for each registered custom kind.
///
/// Holds the optional parent kind and a reserved `schema` slot for future
/// JSON-Schema-based body validation. The schema field is `None` for now;
/// it is intentionally left here so downstream code can extend it without
/// a breaking struct change.
#[derive(Debug, Clone)]
pub struct KindEntry {
    /// Optional parent kind. Enables hierarchy queries (e.g. `is_a` checks).
    pub parent: Option<Kind>,
    /// Reserved for future JSON-Schema body validation. Always `None` today.
    pub schema: Option<String>,
}

/// Runtime registry for custom [`Kind`] validation and parent-child hierarchy.
///
/// Built-in variants are **always** valid without registration. Only
/// [`Kind::Custom`] variants need to be registered via [`register_custom`].
///
/// # Example
///
/// ```rust
/// use roko_core::{Kind, KindRegistry};
///
/// let mut registry = KindRegistry::default();
/// registry.register_custom("com.example.widget".into(), Some(Kind::Task));
///
/// assert!(registry.is_valid(&Kind::GateVerdict));  // built-in — always valid
/// assert!(registry.is_valid(&Kind::Custom("com.example.widget".into())));
/// assert!(!registry.is_valid(&Kind::Custom("com.example.unknown".into())));
///
/// let parent = registry.parent(&Kind::Custom("com.example.widget".into()));
/// assert_eq!(parent, Some(&Kind::Task));
/// ```
///
/// # No globals
///
/// The registry is a plain struct — pass it around rather than using a
/// `once_cell` or `lazy_static`. Callers own the instance lifetime.
#[derive(Debug, Default, Clone)]
pub struct KindRegistry {
    entries: HashMap<String, KindEntry>,
}

impl KindRegistry {
    /// Create an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a custom kind by name with an optional parent.
    ///
    /// Re-registering an existing name overwrites the previous entry.
    /// The `name` should match the string inside `Kind::Custom(name)`.
    pub fn register_custom(&mut self, name: String, parent: Option<Kind>) {
        self.entries.insert(
            name,
            KindEntry {
                parent,
                schema: None,
            },
        );
    }

    /// Returns `true` if the kind is considered valid.
    ///
    /// - Built-in variants (everything except `Custom`) are **always** valid.
    /// - [`Kind::Custom`] is valid only if registered via [`register_custom`].
    /// - [`Kind::Compound`] is valid if all constituent kinds are valid.
    #[must_use]
    pub fn is_valid(&self, kind: &Kind) -> bool {
        match kind {
            Kind::Custom(name) => self.entries.contains_key(name.as_str()),
            Kind::Compound(parts) => parts.iter().all(|k| self.is_valid(k)),
            // All named built-in variants are valid by definition.
            _ => true,
        }
    }

    /// Returns the registered parent of a kind, if any.
    ///
    /// Built-in kinds and unregistered Custom kinds return `None`.
    #[must_use]
    pub fn parent(&self, kind: &Kind) -> Option<&Kind> {
        match kind {
            Kind::Custom(name) => self.entries.get(name.as_str())?.parent.as_ref(),
            _ => None,
        }
    }

    /// List the names of all registered custom kinds.
    #[must_use]
    pub fn list_custom(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.entries.keys().map(String::as_str).collect();
        names.sort_unstable();
        names
    }

    /// Returns a reference to the full [`KindEntry`] for a registered custom kind.
    #[must_use]
    pub fn entry(&self, name: &str) -> Option<&KindEntry> {
        self.entries.get(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_str_is_stable() {
        assert_eq!(Kind::GateVerdict.as_str(), "gate_verdict");
        assert_eq!(Kind::ProcessSpawn.as_str(), "process_spawn");
    }

    #[test]
    fn custom_kinds_preserve_string() {
        let k = Kind::Custom("com.example.widget".into());
        assert_eq!(k.as_str(), "com.example.widget");
    }

    #[test]
    fn compound_kind_has_stable_identity_key() {
        let kind = Kind::Compound(vec![Kind::Task, Kind::PromptSection]);
        assert_eq!(kind.as_str(), "compound");
        assert_eq!(kind.identity_key(), "compound(task+prompt_section)");
    }

    #[test]
    fn display_matches_as_str() {
        assert_eq!(format!("{}", Kind::Task), "task");
    }

    #[test]
    fn compound_factory() {
        let k = Kind::compound(&[Kind::GateVerdict, Kind::Metric]);
        assert!(k.is_compound());
        assert_eq!(k.arity(), 2);
    }

    #[test]
    fn compound_contains() {
        let k = Kind::compound(&[Kind::Task, Kind::PromptSection, Kind::Metric]);
        assert!(k.contains(&Kind::Task));
        assert!(k.contains(&Kind::Metric));
        assert!(!k.contains(&Kind::Episode));
    }

    #[test]
    fn compound_matches_constituent() {
        let k = Kind::compound(&[Kind::GateVerdict, Kind::Metric]);
        assert!(k.matches(&Kind::GateVerdict));
        assert!(k.matches(&Kind::Metric));
        assert!(!k.matches(&Kind::Task));
    }

    #[test]
    fn non_compound_matches_exact() {
        assert!(Kind::Task.matches(&Kind::Task));
        assert!(!Kind::Task.matches(&Kind::Plan));
    }

    #[test]
    fn compound_matches_subset() {
        let big = Kind::compound(&[Kind::Task, Kind::Metric, Kind::Episode]);
        let small = Kind::compound(&[Kind::Task, Kind::Metric]);
        assert!(big.matches(&small)); // big contains all of small
        assert!(!small.matches(&big)); // small doesn't contain Episode
    }

    #[test]
    fn constituents_iterator() {
        let k = Kind::compound(&[Kind::Task, Kind::Plan]);
        let parts: Vec<_> = k.constituents().collect();
        assert_eq!(parts, vec![&Kind::Task, &Kind::Plan]);

        // Non-compound yields self.
        let simple: Vec<_> = Kind::Task.constituents().collect();
        assert_eq!(simple, vec![&Kind::Task]);
    }

    #[test]
    fn serde_roundtrip() {
        for k in [
            Kind::ProcessSpawn,
            Kind::GateVerdict,
            Kind::ToolInvocation,
            Kind::ToolHealthDegraded,
            Kind::Compound(vec![Kind::Task, Kind::Prompt]),
            Kind::Custom("x.y".into()),
        ] {
            let json = serde_json::to_string(&k).unwrap();
            let parsed: Kind = serde_json::from_str(&json).unwrap();
            assert_eq!(k, parsed);
        }
    }
}
