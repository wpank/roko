//! Version-1 signal payloads for the production compose/enrichment graph cells.
//!
//! Every payload carries the four identity fields (`run_id`, `plan_id`,
//! `task_id`, `role`) that scope it to a single compose invocation.
//! Enrichment payloads carry `sections: Vec<PromptSection>` and
//! `warnings: Vec<String>`. The final [`ComposedPrompt`] additionally
//! records which sections were included and which were dropped.
//!
//! These payloads are consumed and produced by the seven enrichment provider
//! Cells and the aggregate Cell. They are transported as `Signal` bodies
//! through the graph engine.

use roko_core::AgentRole;
use serde::{Deserialize, Serialize};

use crate::prompt::PromptSection;

// ---------------------------------------------------------------------------
// Identity scope
// ---------------------------------------------------------------------------

/// Shared identity scope carried by every compose signal.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComposeScope {
    /// Unique run identifier.
    pub run_id: String,
    /// Plan identifier.
    pub plan_id: String,
    /// Task identifier.
    pub task_id: String,
    /// Agent role for which the prompt is being composed.
    pub role: AgentRole,
}

impl ComposeScope {
    /// Returns `true` when `self` and `other` reference the same
    /// (run, plan, task, role) tuple.
    #[must_use]
    pub fn matches(&self, other: &Self) -> bool {
        self.run_id == other.run_id
            && self.plan_id == other.plan_id
            && self.task_id == other.task_id
            && self.role == other.role
    }
}

// ---------------------------------------------------------------------------
// Request
// ---------------------------------------------------------------------------

/// Input signal consumed by all seven enrichment provider Cells.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComposeRequest {
    /// Identity scope for this compose invocation.
    pub scope: ComposeScope,
    /// Optional overall token budget hint for the composed prompt.
    pub token_budget: Option<usize>,
    /// Optional context-window size hint (model-dependent).
    pub context_window_tokens: Option<usize>,
}

impl ComposeRequest {
    /// Convenience constructor.
    #[must_use]
    pub fn new(
        run_id: impl Into<String>,
        plan_id: impl Into<String>,
        task_id: impl Into<String>,
        role: AgentRole,
    ) -> Self {
        Self {
            scope: ComposeScope {
                run_id: run_id.into(),
                plan_id: plan_id.into(),
                task_id: task_id.into(),
                role,
            },
            token_budget: None,
            context_window_tokens: None,
        }
    }

    /// Builder: set the token budget.
    #[must_use]
    pub const fn with_token_budget(mut self, budget: usize) -> Self {
        self.token_budget = Some(budget);
        self
    }

    /// Builder: set the context-window size.
    #[must_use]
    pub const fn with_context_window(mut self, tokens: usize) -> Self {
        self.context_window_tokens = Some(tokens);
        self
    }
}

// ---------------------------------------------------------------------------
// Enrichment section payloads
// ---------------------------------------------------------------------------

/// Macro to reduce boilerplate for enrichment section payloads.
///
/// Each enrichment payload carries the identity scope, a list of prompt
/// sections produced by that provider, and any warnings generated.
macro_rules! enrichment_payload {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
        pub struct $name {
            /// Identity scope for this enrichment output.
            pub scope: ComposeScope,
            /// Prompt sections produced by this enrichment provider.
            pub sections: Vec<PromptSection>,
            /// Non-fatal warnings generated during enrichment.
            pub warnings: Vec<String>,
        }

        impl $name {
            /// Convenience constructor with no warnings.
            #[must_use]
            pub fn new(scope: ComposeScope, sections: Vec<PromptSection>) -> Self {
                Self {
                    scope,
                    sections,
                    warnings: Vec::new(),
                }
            }

            /// Constructor for a degraded (empty) result with a warning.
            #[must_use]
            pub fn degraded(scope: ComposeScope, warning: impl Into<String>) -> Self {
                Self {
                    scope,
                    sections: Vec::new(),
                    warnings: vec![warning.into()],
                }
            }
        }
    };
}

enrichment_payload!(
    /// Output from the knowledge enrichment Cell (`compose.knowledge@1`).
    ///
    /// Provides scoped memory, knowledge store, and code-index context.
    KnowledgeSections
);

enrichment_payload!(
    /// Output from the episodes enrichment Cell (`compose.episodes@1`).
    ///
    /// Provides relevant episode summaries and error-pattern context.
    EpisodeSections
);

enrichment_payload!(
    /// Output from the playbook enrichment Cell (`compose.playbook@1`).
    ///
    /// Provides playbook/skill/Dreams context.
    PlaybookSections
);

enrichment_payload!(
    /// Output from the task context enrichment Cell (`compose.task_context@1`).
    ///
    /// Provides dependency-output and task-context sections. **Required** --
    /// absence or error fails the aggregate.
    TaskContextSections
);

enrichment_payload!(
    /// Output from the modulation enrichment Cell (`compose.modulation@1`).
    ///
    /// Provides Daimon/cortical/routing modulation context.
    ModulationSections
);

enrichment_payload!(
    /// Output from the safety enrichment Cell (`compose.safety@1`).
    ///
    /// Provides safety and capability context. **Required** -- absence or
    /// error fails the aggregate.
    SafetySections
);

// ---------------------------------------------------------------------------
// Experiment assignment
// ---------------------------------------------------------------------------

/// Output from the experiment enrichment Cell (`compose.experiment@1`).
///
/// Carries prompt experiment assignments rather than raw sections,
/// because experiment sections may modify or replace existing sections.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExperimentAssignment {
    /// Identity scope for this experiment output.
    pub scope: ComposeScope,
    /// Prompt sections injected by the experiment (may replace canonical ones).
    pub sections: Vec<PromptSection>,
    /// Non-fatal warnings generated during experiment evaluation.
    pub warnings: Vec<String>,
    /// Active experiment IDs that contributed sections.
    pub active_experiment_ids: Vec<String>,
}

impl ExperimentAssignment {
    /// Convenience constructor with no warnings.
    #[must_use]
    pub fn new(
        scope: ComposeScope,
        sections: Vec<PromptSection>,
        active_experiment_ids: Vec<String>,
    ) -> Self {
        Self {
            scope,
            sections,
            warnings: Vec::new(),
            active_experiment_ids,
        }
    }

    /// Constructor for a degraded (empty) result with a warning.
    #[must_use]
    pub fn degraded(scope: ComposeScope, warning: impl Into<String>) -> Self {
        Self {
            scope,
            sections: Vec::new(),
            warnings: vec![warning.into()],
            active_experiment_ids: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Composed prompt (aggregate output)
// ---------------------------------------------------------------------------

/// Final output from the aggregate Cell (`compose.aggregate@1`).
///
/// Contains the assembled prompt text plus metadata about which sections
/// were included and which were dropped due to budget pressure or absence.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComposedPrompt {
    /// Identity scope for this composed prompt.
    pub scope: ComposeScope,
    /// The fully assembled prompt text.
    pub text: String,
    /// Estimated token count of `text`.
    pub estimated_tokens: usize,
    /// Section IDs that were included in the final prompt.
    pub included_section_ids: Vec<String>,
    /// Section IDs that were dropped (budget pressure, absence, etc.).
    pub dropped_section_ids: Vec<String>,
    /// Warnings accumulated from all enrichment providers.
    pub warnings: Vec<String>,
    /// Active experiment IDs, if any were applied.
    pub active_experiment_ids: Vec<String>,
}

impl ComposedPrompt {
    /// Create a new composed prompt.
    #[must_use]
    pub fn new(scope: ComposeScope, text: String) -> Self {
        let estimated_tokens = crate::prompt::estimate_tokens(&text);
        Self {
            scope,
            text,
            estimated_tokens,
            included_section_ids: Vec::new(),
            dropped_section_ids: Vec::new(),
            warnings: Vec::new(),
            active_experiment_ids: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Stable Cell IDs and versions
// ---------------------------------------------------------------------------

/// Stable Cell IDs for the compose graph.
pub mod cell_ids {
    /// Knowledge enrichment provider Cell.
    pub const KNOWLEDGE: &str = "compose.knowledge@1";
    /// Episodes enrichment provider Cell.
    pub const EPISODES: &str = "compose.episodes@1";
    /// Playbook enrichment provider Cell.
    pub const PLAYBOOK: &str = "compose.playbook@1";
    /// Task context enrichment provider Cell.
    pub const TASK_CONTEXT: &str = "compose.task_context@1";
    /// Modulation enrichment provider Cell.
    pub const MODULATION: &str = "compose.modulation@1";
    /// Safety enrichment provider Cell.
    pub const SAFETY: &str = "compose.safety@1";
    /// Experiment assignment provider Cell.
    pub const EXPERIMENT: &str = "compose.experiment@1";
    /// Aggregate composition Cell.
    pub const AGGREGATE: &str = "compose.aggregate@1";

    /// All provider Cell IDs (excludes aggregate).
    pub const ALL_PROVIDERS: &[&str] = &[
        KNOWLEDGE,
        EPISODES,
        PLAYBOOK,
        TASK_CONTEXT,
        MODULATION,
        SAFETY,
        EXPERIMENT,
    ];

    /// Required provider Cell IDs -- aggregate fails closed if missing.
    pub const REQUIRED_PROVIDERS: &[&str] = &[TASK_CONTEXT, SAFETY];
}

// ---------------------------------------------------------------------------
// Signal serialization helpers
// ---------------------------------------------------------------------------

/// Payload tag used to identify compose signals in the graph.
pub const COMPOSE_SIGNAL_TAG: &str = "compose.v1";

/// Deserialize a [`ComposeRequest`] from a Signal body.
///
/// # Errors
///
/// Returns an error if the signal body cannot be deserialized.
pub fn parse_compose_request(signal: &roko_core::Signal) -> Result<ComposeRequest, String> {
    signal.body.as_json::<ComposeRequest>().map_err(|e| {
        format!(
            "failed to parse ComposeRequest from signal {}: {e}",
            signal.id
        )
    })
}

/// Serialize a payload into a Signal body.
///
/// # Errors
///
/// Returns an error if serialization fails.
pub fn compose_signal_body<T: Serialize>(payload: &T) -> Result<serde_json::Value, String> {
    serde_json::to_value(payload).map_err(|e| format!("failed to serialize compose payload: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_scope() -> ComposeScope {
        ComposeScope {
            run_id: "run-1".into(),
            plan_id: "plan-1".into(),
            task_id: "task-1".into(),
            role: AgentRole::Implementer,
        }
    }

    #[test]
    fn scope_matches_identical() {
        let a = test_scope();
        let b = test_scope();
        assert!(a.matches(&b));
    }

    #[test]
    fn scope_mismatch_on_any_field() {
        let a = test_scope();
        let mut b = test_scope();
        b.run_id = "run-2".into();
        assert!(!a.matches(&b));

        let mut c = test_scope();
        c.task_id = "task-2".into();
        assert!(!a.matches(&c));
    }

    #[test]
    fn compose_request_serde_round_trip() {
        let req = ComposeRequest::new("r1", "p1", "t1", AgentRole::Implementer)
            .with_token_budget(8000)
            .with_context_window(128_000);
        let json = serde_json::to_string(&req).unwrap();
        let deser: ComposeRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(req, deser);
    }

    #[test]
    fn knowledge_sections_serde_round_trip() {
        let scope = test_scope();
        let sections = vec![PromptSection::new("knowledge", "some context")];
        let payload = KnowledgeSections::new(scope, sections);
        let json = serde_json::to_string(&payload).unwrap();
        let deser: KnowledgeSections = serde_json::from_str(&json).unwrap();
        assert_eq!(payload, deser);
    }

    #[test]
    fn degraded_payload_has_warning() {
        let scope = test_scope();
        let payload = EpisodeSections::degraded(scope, "store unavailable");
        assert!(payload.sections.is_empty());
        assert_eq!(payload.warnings.len(), 1);
        assert!(payload.warnings[0].contains("store unavailable"));
    }

    #[test]
    fn experiment_assignment_serde_round_trip() {
        let scope = test_scope();
        let payload = ExperimentAssignment::new(
            scope,
            vec![PromptSection::new("exp", "test content")],
            vec!["exp-001".into()],
        );
        let json = serde_json::to_string(&payload).unwrap();
        let deser: ExperimentAssignment = serde_json::from_str(&json).unwrap();
        assert_eq!(payload, deser);
    }

    #[test]
    fn composed_prompt_estimates_tokens() {
        let scope = test_scope();
        let text = "a".repeat(400); // ~100 tokens at 4 bytes/token
        let prompt = ComposedPrompt::new(scope, text);
        assert_eq!(prompt.estimated_tokens, 100);
    }

    #[test]
    fn cell_ids_all_providers_count() {
        assert_eq!(cell_ids::ALL_PROVIDERS.len(), 7);
    }

    #[test]
    fn cell_ids_required_providers() {
        assert!(cell_ids::REQUIRED_PROVIDERS.contains(&cell_ids::TASK_CONTEXT));
        assert!(cell_ids::REQUIRED_PROVIDERS.contains(&cell_ids::SAFETY));
        assert_eq!(cell_ids::REQUIRED_PROVIDERS.len(), 2);
    }
}
