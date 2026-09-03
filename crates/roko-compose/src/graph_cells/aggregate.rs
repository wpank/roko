//! Aggregate composition Cell (`compose.aggregate@1`).
//!
//! Consumes one signal from every enrichment provider Cell and produces the
//! final [`ComposedPrompt`]. The aggregate enforces:
//!
//! - **Fixed section ordering**: safety, role identity, conventions, tool
//!   instructions, knowledge/code context, episodes/error patterns,
//!   playbook/skills, dependency/task context, modulation/routing, experiment
//!   annotations, gate feedback, and final task instruction.
//! - **Scope validation**: rejects any provider output whose scope differs
//!   from the request scope.
//! - **Required providers**: `task_context` and `safety` must succeed; all
//!   others degrade to a warning.
//! - **Budget enforcement**: sections that exceed the token budget are dropped
//!   by priority, with included/dropped IDs recorded.
//! - **Deduplication**: sections with identical `section_id` are deduplicated,
//!   keeping the first occurrence in aggregate order.

use std::collections::HashSet;

use async_trait::async_trait;
use roko_core::error::Result;
use roko_core::{Body, Kind, Signal};
use tracing::warn;

use crate::prompt::{PromptSection, SectionPriority, estimate_tokens};

use super::signals::{
    ComposeRequest, ComposeScope, ComposedPrompt, EpisodeSections, ExperimentAssignment,
    KnowledgeSections, ModulationSections, PlaybookSections, SafetySections,
    TaskContextSections, cell_ids,
};

// ---------------------------------------------------------------------------
// Aggregate ordering groups
// ---------------------------------------------------------------------------

/// Fixed ordering groups for the aggregate. Within a group, sections
/// retain their original provider order and U-shaped placement.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
enum AggregateGroup {
    Safety = 0,
    RoleIdentity = 1,
    Conventions = 2,
    ToolInstructions = 3,
    KnowledgeCodeContext = 4,
    EpisodesErrorPatterns = 5,
    PlaybookSkills = 6,
    DependencyTaskContext = 7,
    ModulationRouting = 8,
    ExperimentAnnotations = 9,
    GateFeedback = 10,
    FinalTaskInstruction = 11,
}

/// Map a section to its aggregate ordering group based on its name and
/// source metadata.
fn classify_section(section: &PromptSection) -> AggregateGroup {
    let name = section.name.as_str();
    match name {
        // Safety sections
        "safety_notice" | "capability_declaration" | "corrigibility" => AggregateGroup::Safety,
        // Role identity
        "role_identity" => AggregateGroup::RoleIdentity,
        // Conventions
        "conventions" => AggregateGroup::Conventions,
        // Tool instructions
        "tool_instructions" | "tool_hints" => AggregateGroup::ToolInstructions,
        // Knowledge/code context
        "knowledge_fact" | "domain_context" | "context_layer" | "code_context"
        | "pheromone_signals" => AggregateGroup::KnowledgeCodeContext,
        // Episodes/error patterns
        "error_pattern" | "episode_summary" | "recent_failures" => {
            AggregateGroup::EpisodesErrorPatterns
        }
        // Playbook/skills
        "relevant_skills" | "playbook_match" | "dream_insight" => AggregateGroup::PlaybookSkills,
        // Task context / dependency
        "task_context" | "task_brief" | "dependency_output" | "plan_brief" => {
            AggregateGroup::DependencyTaskContext
        }
        // Modulation/routing
        "affect_guidance" | "affect" | "cortical_state" | "routing_hint" => {
            AggregateGroup::ModulationRouting
        }
        // Experiment annotations
        _ if section
            .experiment_id
            .as_ref()
            .is_some_and(|id| !id.is_empty()) =>
        {
            AggregateGroup::ExperimentAnnotations
        }
        // Gate feedback
        "gate_feedback" => AggregateGroup::GateFeedback,
        // Anti-patterns often go near the task instruction
        "anti_patterns" => AggregateGroup::FinalTaskInstruction,
        // Default: use source type or fall to task context
        _ => {
            if let Some(ref src) = section.source_type {
                match src.as_str() {
                    "safety" => AggregateGroup::Safety,
                    "knowledge" | "neuro" => AggregateGroup::KnowledgeCodeContext,
                    "episode" => AggregateGroup::EpisodesErrorPatterns,
                    "playbook" | "skill" | "dream" => AggregateGroup::PlaybookSkills,
                    "modulation" | "daimon" => AggregateGroup::ModulationRouting,
                    "experiment" => AggregateGroup::ExperimentAnnotations,
                    _ => AggregateGroup::DependencyTaskContext,
                }
            } else {
                AggregateGroup::DependencyTaskContext
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Cell implementation
// ---------------------------------------------------------------------------

/// Aggregate composition Cell for the compose graph.
///
/// Consumes enrichment signals from all seven provider Cells and produces
/// a single [`ComposedPrompt`] signal.
pub struct AggregateCell;

impl AggregateCell {
    /// Create a new aggregate cell.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for AggregateCell {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl roko_graph::Cell for AggregateCell {
    fn cell_id(&self) -> &str {
        cell_ids::AGGREGATE
    }

    fn cell_name(&self) -> &str {
        "Compose Aggregate"
    }

    fn cell_version(&self) -> roko_graph::CellVersion {
        (1, 0, 0)
    }

    async fn execute(
        &self,
        input: Vec<Signal>,
        _ctx: &roko_graph::CellContext,
    ) -> Result<Vec<Signal>> {
        // 1. Extract the compose request (it should be in the inputs or we
        //    derive the scope from the first provider output).
        let (scope, token_budget) = extract_scope_and_budget(&input)?;

        // 2. Collect enrichment outputs, validating scopes.
        let collected = collect_enrichment_outputs(&input, &scope)?;

        // 3. Validate that required providers succeeded.
        if !collected.has_safety {
            return Err(roko_core::error::RokoError::Internal(
                "AggregateCell: required safety provider missing or errored".into(),
            ));
        }
        if !collected.has_task_context {
            return Err(roko_core::error::RokoError::Internal(
                "AggregateCell: required task_context provider missing or errored".into(),
            ));
        }

        // 4. Merge all sections, deduplicate, and sort by aggregate group.
        let mut all_sections = collected.sections;
        let mut warnings = collected.warnings;
        let experiment_ids = collected.experiment_ids;

        // Deduplicate by section_id (keep first occurrence).
        let mut seen_ids = HashSet::new();
        all_sections.retain(|s| {
            if s.section_id.is_empty() {
                true
            } else {
                seen_ids.insert(s.section_id.clone())
            }
        });

        // Sort by aggregate ordering group, preserving intra-group order.
        // Use a stable sort so provider-order within groups is preserved.
        all_sections.sort_by_key(|s| classify_section(s) as u8);

        // 5. Apply token budget if set.
        let budget = token_budget.unwrap_or(usize::MAX);
        let mut included_ids = Vec::new();
        let mut dropped_ids = Vec::new();
        let mut included_sections = Vec::new();
        let mut total_tokens = 0;

        // First pass: include all Critical sections regardless of budget.
        let mut pending = Vec::new();
        for section in &all_sections {
            if section.priority == SectionPriority::Critical {
                let tokens = estimate_tokens(&section.content);
                included_ids.push(section.section_id.clone());
                included_sections.push(section.clone());
                total_tokens += tokens;
            } else {
                pending.push(section);
            }
        }

        // Second pass: include non-Critical sections in priority order
        // until the budget is exceeded.
        // Sort pending by priority descending so higher-priority sections
        // are included first when budget is tight.
        let mut pending_sorted: Vec<_> = pending.into_iter().collect();
        pending_sorted.sort_by(|a, b| b.priority.cmp(&a.priority));

        for section in pending_sorted {
            let tokens = estimate_tokens(&section.content);
            if total_tokens + tokens <= budget {
                included_ids.push(section.section_id.clone());
                included_sections.push(section.clone());
                total_tokens += tokens;
            } else {
                dropped_ids.push(section.section_id.clone());
                warnings.push(format!(
                    "dropped section '{}' ({} tokens) due to budget pressure",
                    section.name, tokens
                ));
            }
        }

        // Re-sort included sections back to aggregate order for rendering.
        included_sections.sort_by_key(|s| classify_section(s) as u8);

        // 6. Assemble the final prompt text.
        let text = assemble_prompt_text(&included_sections);

        let prompt = ComposedPrompt {
            scope,
            text,
            estimated_tokens: total_tokens,
            included_section_ids: included_ids,
            dropped_section_ids: dropped_ids,
            warnings,
            active_experiment_ids: experiment_ids,
        };

        let body = Body::from_json(&prompt).map_err(|e| {
            roko_core::error::RokoError::Internal(format!(
                "aggregate cell serialization: {e}"
            ))
        })?;
        let signal = Signal::builder(Kind::Context).body(body).build();
        Ok(vec![signal])
    }
}

// ---------------------------------------------------------------------------
// Enrichment collection
// ---------------------------------------------------------------------------

struct CollectedEnrichment {
    sections: Vec<PromptSection>,
    warnings: Vec<String>,
    experiment_ids: Vec<String>,
    has_safety: bool,
    has_task_context: bool,
}

fn collect_enrichment_outputs(
    input: &[Signal],
    request_scope: &ComposeScope,
) -> Result<CollectedEnrichment> {
    let mut sections = Vec::new();
    let mut warnings = Vec::new();
    let mut experiment_ids = Vec::new();
    let mut has_safety = false;
    let mut has_task_context = false;

    for signal in input {
        // Try each enrichment payload type in turn.
        if let Ok(payload) = signal.body.as_json::<SafetySections>() {
            if !payload.scope.matches(request_scope) {
                warn!(
                    "AggregateCell: rejecting safety output with mismatched scope"
                );
                continue;
            }
            has_safety = true;
            sections.extend(payload.sections);
            warnings.extend(payload.warnings);
            continue;
        }

        if let Ok(payload) = signal.body.as_json::<TaskContextSections>() {
            if !payload.scope.matches(request_scope) {
                warn!(
                    "AggregateCell: rejecting task_context output with mismatched scope"
                );
                continue;
            }
            has_task_context = true;
            sections.extend(payload.sections);
            warnings.extend(payload.warnings);
            continue;
        }

        if let Ok(payload) = signal.body.as_json::<KnowledgeSections>() {
            if !payload.scope.matches(request_scope) {
                warnings.push(
                    "knowledge provider scope mismatch; degrading to empty".into(),
                );
                continue;
            }
            sections.extend(payload.sections);
            warnings.extend(payload.warnings);
            continue;
        }

        if let Ok(payload) = signal.body.as_json::<EpisodeSections>() {
            if !payload.scope.matches(request_scope) {
                warnings.push(
                    "episodes provider scope mismatch; degrading to empty".into(),
                );
                continue;
            }
            sections.extend(payload.sections);
            warnings.extend(payload.warnings);
            continue;
        }

        if let Ok(payload) = signal.body.as_json::<PlaybookSections>() {
            if !payload.scope.matches(request_scope) {
                warnings.push(
                    "playbook provider scope mismatch; degrading to empty".into(),
                );
                continue;
            }
            sections.extend(payload.sections);
            warnings.extend(payload.warnings);
            continue;
        }

        if let Ok(payload) = signal.body.as_json::<ModulationSections>() {
            if !payload.scope.matches(request_scope) {
                warnings.push(
                    "modulation provider scope mismatch; degrading to empty".into(),
                );
                continue;
            }
            sections.extend(payload.sections);
            warnings.extend(payload.warnings);
            continue;
        }

        if let Ok(payload) = signal.body.as_json::<ExperimentAssignment>() {
            if !payload.scope.matches(request_scope) {
                warnings.push(
                    "experiment provider scope mismatch; degrading to empty".into(),
                );
                continue;
            }
            sections.extend(payload.sections);
            warnings.extend(payload.warnings);
            experiment_ids.extend(payload.active_experiment_ids);
            continue;
        }

        // Try to extract a ComposeRequest (passed through from upstream).
        // This is expected and should be silently ignored.
        if signal.body.as_json::<ComposeRequest>().is_ok() {
            continue;
        }

        // Unknown signal type -- log and skip.
        warn!(
            signal_id = %signal.id,
            "AggregateCell: ignoring unrecognized input signal"
        );
    }

    Ok(CollectedEnrichment {
        sections,
        warnings,
        experiment_ids,
        has_safety,
        has_task_context,
    })
}

/// Extract the compose scope and optional budget from input signals.
fn extract_scope_and_budget(
    input: &[Signal],
) -> Result<(ComposeScope, Option<usize>)> {
    // First try to find a ComposeRequest directly.
    for signal in input {
        if let Ok(req) = signal.body.as_json::<ComposeRequest>() {
            return Ok((req.scope, req.token_budget));
        }
    }

    // Fall back to extracting scope from the first provider output.
    for signal in input {
        if let Ok(payload) = signal.body.as_json::<SafetySections>() {
            return Ok((payload.scope, None));
        }
        if let Ok(payload) = signal.body.as_json::<TaskContextSections>() {
            return Ok((payload.scope, None));
        }
        if let Ok(payload) = signal.body.as_json::<KnowledgeSections>() {
            return Ok((payload.scope, None));
        }
        if let Ok(payload) = signal.body.as_json::<EpisodeSections>() {
            return Ok((payload.scope, None));
        }
        if let Ok(payload) = signal.body.as_json::<PlaybookSections>() {
            return Ok((payload.scope, None));
        }
        if let Ok(payload) = signal.body.as_json::<ModulationSections>() {
            return Ok((payload.scope, None));
        }
        if let Ok(payload) = signal.body.as_json::<ExperimentAssignment>() {
            return Ok((payload.scope, None));
        }
    }

    Err(roko_core::error::RokoError::Internal(
        "AggregateCell: could not determine compose scope from input signals".into(),
    ))
}

/// Assemble final prompt text from ordered sections.
fn assemble_prompt_text(sections: &[PromptSection]) -> String {
    let mut parts = Vec::with_capacity(sections.len());
    for section in sections {
        if section.content.is_empty() {
            continue;
        }
        parts.push(section.content.as_str());
    }
    parts.join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prompt::Placement;
    use roko_core::AgentRole;

    fn test_scope() -> ComposeScope {
        ComposeScope {
            run_id: "run-1".into(),
            plan_id: "plan-1".into(),
            task_id: "task-1".into(),
            role: AgentRole::Implementer,
        }
    }

    fn make_safety_signal(scope: &ComposeScope) -> Signal {
        let payload = SafetySections::new(
            scope.clone(),
            vec![PromptSection::new("safety_notice", "Do not modify safety files")
                .with_priority(SectionPriority::Critical)
                .with_placement(Placement::Start)],
        );
        let body = Body::from_json(&payload).unwrap();
        Signal::builder(Kind::Context).body(body).build()
    }

    fn make_task_context_signal(scope: &ComposeScope) -> Signal {
        let payload = TaskContextSections::new(
            scope.clone(),
            vec![PromptSection::new("task_brief", "Implement the widget")
                .with_priority(SectionPriority::Critical)
                .with_placement(Placement::End)],
        );
        let body = Body::from_json(&payload).unwrap();
        Signal::builder(Kind::Task).body(body).build()
    }

    fn make_knowledge_signal(scope: &ComposeScope) -> Signal {
        let payload = KnowledgeSections::new(
            scope.clone(),
            vec![PromptSection::new("knowledge_fact", "Rust uses ownership")],
        );
        let body = Body::from_json(&payload).unwrap();
        Signal::builder(Kind::Context).body(body).build()
    }

    fn make_request_signal(scope: &ComposeScope) -> Signal {
        let req = ComposeRequest {
            scope: scope.clone(),
            token_budget: None,
            context_window_tokens: None,
        };
        let body = Body::from_json(&req).unwrap();
        Signal::builder(Kind::Context).body(body).build()
    }

    #[tokio::test]
    async fn aggregate_with_all_required_providers() {
        let scope = test_scope();
        let cell = AggregateCell::new();

        let input = vec![
            make_request_signal(&scope),
            make_safety_signal(&scope),
            make_task_context_signal(&scope),
        ];

        let result = cell
            .execute(input, &roko_graph::CellContext::new())
            .await
            .unwrap();
        assert_eq!(result.len(), 1);

        let prompt: ComposedPrompt = result[0].body.as_json().unwrap();
        assert!(prompt.text.contains("Do not modify safety files"));
        assert!(prompt.text.contains("Implement the widget"));
        assert_eq!(prompt.included_section_ids.len(), 2);
        assert!(prompt.dropped_section_ids.is_empty());
    }

    #[tokio::test]
    async fn aggregate_fails_without_safety() {
        let scope = test_scope();
        let cell = AggregateCell::new();

        let input = vec![
            make_request_signal(&scope),
            make_task_context_signal(&scope),
        ];

        let result = cell
            .execute(input, &roko_graph::CellContext::new())
            .await;
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("safety"));
    }

    #[tokio::test]
    async fn aggregate_fails_without_task_context() {
        let scope = test_scope();
        let cell = AggregateCell::new();

        let input = vec![make_request_signal(&scope), make_safety_signal(&scope)];

        let result = cell
            .execute(input, &roko_graph::CellContext::new())
            .await;
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("task_context"));
    }

    #[tokio::test]
    async fn aggregate_ordering_safety_before_knowledge_before_task() {
        let scope = test_scope();
        let cell = AggregateCell::new();

        let input = vec![
            make_request_signal(&scope),
            // Deliberately put knowledge first to test ordering.
            make_knowledge_signal(&scope),
            make_task_context_signal(&scope),
            make_safety_signal(&scope),
        ];

        let result = cell
            .execute(input, &roko_graph::CellContext::new())
            .await
            .unwrap();
        let prompt: ComposedPrompt = result[0].body.as_json().unwrap();

        // Safety should come before knowledge which should come before task.
        let safety_pos = prompt.text.find("Do not modify safety files").unwrap();
        let knowledge_pos = prompt.text.find("Rust uses ownership").unwrap();
        let task_pos = prompt.text.find("Implement the widget").unwrap();
        assert!(safety_pos < knowledge_pos);
        assert!(knowledge_pos < task_pos);
    }

    #[tokio::test]
    async fn aggregate_rejects_scope_mismatch() {
        let scope = test_scope();
        let mut wrong_scope = test_scope();
        wrong_scope.task_id = "wrong-task".into();

        let cell = AggregateCell::new();

        let mismatched_knowledge = {
            let payload = KnowledgeSections::new(
                wrong_scope,
                vec![PromptSection::new("knowledge_fact", "should be rejected")],
            );
            let body = Body::from_json(&payload).unwrap();
            Signal::builder(Kind::Context).body(body).build()
        };

        let input = vec![
            make_request_signal(&scope),
            make_safety_signal(&scope),
            make_task_context_signal(&scope),
            mismatched_knowledge,
        ];

        let result = cell
            .execute(input, &roko_graph::CellContext::new())
            .await
            .unwrap();
        let prompt: ComposedPrompt = result[0].body.as_json().unwrap();

        // The mismatched knowledge should NOT be in the prompt.
        assert!(!prompt.text.contains("should be rejected"));
        // But there should be a warning about it.
        assert!(prompt.warnings.iter().any(|w| w.contains("scope mismatch")));
    }

    #[tokio::test]
    async fn aggregate_deduplicates_sections() {
        let scope = test_scope();
        let cell = AggregateCell::new();

        // Create two knowledge signals with the same section_id.
        let dup1 = {
            let payload = KnowledgeSections::new(
                scope.clone(),
                vec![PromptSection::new("knowledge_fact", "first")
                    .with_section_id("dup-id")],
            );
            let body = Body::from_json(&payload).unwrap();
            Signal::builder(Kind::Context).body(body).build()
        };
        let dup2 = {
            let payload = KnowledgeSections::new(
                scope.clone(),
                vec![PromptSection::new("knowledge_fact", "second")
                    .with_section_id("dup-id")],
            );
            let body = Body::from_json(&payload).unwrap();
            Signal::builder(Kind::Context).body(body).build()
        };

        let input = vec![
            make_request_signal(&scope),
            make_safety_signal(&scope),
            make_task_context_signal(&scope),
            dup1,
            dup2,
        ];

        let result = cell
            .execute(input, &roko_graph::CellContext::new())
            .await
            .unwrap();
        let prompt: ComposedPrompt = result[0].body.as_json().unwrap();

        // Only one "dup-id" section should be included.
        let dup_count = prompt
            .included_section_ids
            .iter()
            .filter(|id| *id == "dup-id")
            .count();
        assert_eq!(dup_count, 1);
    }

    #[tokio::test]
    async fn aggregate_budget_drops_low_priority() {
        let scope = test_scope();
        let cell = AggregateCell::new();

        // Set a very tight budget through a request signal.
        let req = ComposeRequest {
            scope: scope.clone(),
            token_budget: Some(50), // Very tight budget.
            context_window_tokens: None,
        };
        let req_signal = {
            let body = Body::from_json(&req).unwrap();
            Signal::builder(Kind::Context).body(body).build()
        };

        // Knowledge with low priority and long content.
        let knowledge = {
            let long_content = "x".repeat(1000); // ~250 tokens
            let payload = KnowledgeSections::new(
                scope.clone(),
                vec![PromptSection::new("knowledge_fact", long_content)
                    .with_priority(SectionPriority::Low)],
            );
            let body = Body::from_json(&payload).unwrap();
            Signal::builder(Kind::Context).body(body).build()
        };

        let input = vec![
            req_signal,
            make_safety_signal(&scope),
            make_task_context_signal(&scope),
            knowledge,
        ];

        let result = cell
            .execute(input, &roko_graph::CellContext::new())
            .await
            .unwrap();
        let prompt: ComposedPrompt = result[0].body.as_json().unwrap();

        // Critical sections (safety + task) should be included.
        // The large low-priority knowledge section should be dropped.
        assert!(!prompt.dropped_section_ids.is_empty());
        assert!(prompt
            .warnings
            .iter()
            .any(|w| w.contains("budget pressure")));
    }

    #[test]
    fn classify_section_ordering() {
        let safety = PromptSection::new("safety_notice", "x");
        let role = PromptSection::new("role_identity", "x");
        let task = PromptSection::new("task_brief", "x");
        let knowledge = PromptSection::new("knowledge_fact", "x");
        let gate = PromptSection::new("gate_feedback", "x");

        assert!(
            (classify_section(&safety) as u8) < (classify_section(&role) as u8)
        );
        assert!(
            (classify_section(&role) as u8)
                < (classify_section(&knowledge) as u8)
        );
        assert!(
            (classify_section(&knowledge) as u8)
                < (classify_section(&task) as u8)
        );
        assert!(
            (classify_section(&task) as u8) < (classify_section(&gate) as u8)
        );
    }

    #[test]
    fn classify_experiment_section() {
        let exp = PromptSection::new("custom_exp", "x")
            .with_section_id("experiment:001:custom_exp");
        let mut exp_with_id = exp.clone();
        exp_with_id.experiment_id = Some("001".into());
        assert_eq!(
            classify_section(&exp_with_id) as u8,
            AggregateGroup::ExperimentAnnotations as u8
        );
    }
}
