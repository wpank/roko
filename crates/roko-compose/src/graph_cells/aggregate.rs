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
use serde::Deserialize;
use tracing::warn;

use crate::prompt::{PromptSection, SectionPriority, estimate_tokens};

use super::signals::{
    ComposeRequest, ComposeScope, ComposedPrompt, EpisodeSections, ExperimentAssignment,
    KnowledgeSections, ModulationSections, PlaybookSections, SafetySections, TaskContextSections,
    cell_ids,
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
            return Err(roko_core::error::RokoError::Store(
                "AggregateCell: required safety provider missing or errored".into(),
            ));
        }
        if !collected.has_task_context {
            return Err(roko_core::error::RokoError::Store(
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
        pending_sorted.sort_by_key(|s| std::cmp::Reverse(s.priority));

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
            roko_core::error::RokoError::Store(format!("aggregate cell serialization: {e}"))
        })?;
        let signal = Signal::builder(Kind::ContextPack).body(body).build();
        Ok(vec![signal])
    }
}

// ---------------------------------------------------------------------------
// Enrichment collection
// ---------------------------------------------------------------------------

/// Generic enrichment shape for provider-tag dispatch.
///
/// All enrichment payloads share `provider`, `scope`, `sections`, and
/// `warnings`. We deserialize into this common shape once and dispatch
/// on the `provider` value to avoid serde matching the wrong concrete type.
#[derive(Deserialize)]
struct GenericEnrichment {
    provider: String,
    scope: ComposeScope,
    #[serde(default)]
    sections: Vec<PromptSection>,
    #[serde(default)]
    warnings: Vec<String>,
}

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
        // All enrichment payloads share the same structure (scope, sections,
        // warnings) plus a `provider` discriminator tag. We deserialize once
        // into a generic shape and dispatch on the provider value to avoid
        // serde matching the wrong type.
        if let Ok(generic) = signal.body.as_json::<GenericEnrichment>() {
            match generic.provider.as_str() {
                SafetySections::PROVIDER_TAG => {
                    if !generic.scope.matches(request_scope) {
                        warn!("AggregateCell: rejecting safety output with mismatched scope");
                        continue;
                    }
                    has_safety = true;
                    sections.extend(generic.sections);
                    warnings.extend(generic.warnings);
                }
                TaskContextSections::PROVIDER_TAG => {
                    if !generic.scope.matches(request_scope) {
                        warn!(
                            "AggregateCell: rejecting task_context output with mismatched scope"
                        );
                        continue;
                    }
                    has_task_context = true;
                    sections.extend(generic.sections);
                    warnings.extend(generic.warnings);
                }
                KnowledgeSections::PROVIDER_TAG => {
                    if !generic.scope.matches(request_scope) {
                        warnings
                            .push("knowledge provider scope mismatch; degrading to empty".into());
                        continue;
                    }
                    sections.extend(generic.sections);
                    warnings.extend(generic.warnings);
                }
                EpisodeSections::PROVIDER_TAG => {
                    if !generic.scope.matches(request_scope) {
                        warnings
                            .push("episodes provider scope mismatch; degrading to empty".into());
                        continue;
                    }
                    sections.extend(generic.sections);
                    warnings.extend(generic.warnings);
                }
                PlaybookSections::PROVIDER_TAG => {
                    if !generic.scope.matches(request_scope) {
                        warnings
                            .push("playbook provider scope mismatch; degrading to empty".into());
                        continue;
                    }
                    sections.extend(generic.sections);
                    warnings.extend(generic.warnings);
                }
                ModulationSections::PROVIDER_TAG => {
                    if !generic.scope.matches(request_scope) {
                        warnings
                            .push("modulation provider scope mismatch; degrading to empty".into());
                        continue;
                    }
                    sections.extend(generic.sections);
                    warnings.extend(generic.warnings);
                }
                ExperimentAssignment::PROVIDER_TAG => {
                    if !generic.scope.matches(request_scope) {
                        warnings
                            .push("experiment provider scope mismatch; degrading to empty".into());
                        continue;
                    }
                    sections.extend(generic.sections);
                    warnings.extend(generic.warnings);
                    // Extract experiment IDs from the full payload.
                    if let Ok(exp) = signal.body.as_json::<ExperimentAssignment>() {
                        experiment_ids.extend(exp.active_experiment_ids);
                    }
                }
                other => {
                    warn!(
                        provider = other,
                        "AggregateCell: ignoring enrichment signal with unknown provider tag"
                    );
                }
            }
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
fn extract_scope_and_budget(input: &[Signal]) -> Result<(ComposeScope, Option<usize>)> {
    // First try to find a ComposeRequest directly.
    for signal in input {
        if let Ok(req) = signal.body.as_json::<ComposeRequest>() {
            return Ok((req.scope, req.token_budget));
        }
    }

    // Fall back to extracting scope from the first enrichment signal.
    for signal in input {
        if let Ok(generic) = signal.body.as_json::<GenericEnrichment>() {
            return Ok((generic.scope, None));
        }
    }

    Err(roko_core::error::RokoError::Store(
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
    use roko_graph::Cell;

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
            vec![
                PromptSection::new("safety_notice", "Do not modify safety files")
                    .with_priority(SectionPriority::Critical)
                    .with_placement(Placement::Start),
            ],
        );
        let body = Body::from_json(&payload).unwrap();
        Signal::builder(Kind::ContextPack).body(body).build()
    }

    fn make_task_context_signal(scope: &ComposeScope) -> Signal {
        let payload = TaskContextSections::new(
            scope.clone(),
            vec![
                PromptSection::new("task_brief", "Implement the widget")
                    .with_priority(SectionPriority::Critical)
                    .with_placement(Placement::End),
            ],
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
        Signal::builder(Kind::ContextPack).body(body).build()
    }

    fn make_request_signal(scope: &ComposeScope) -> Signal {
        let req = ComposeRequest {
            scope: scope.clone(),
            token_budget: None,
            context_window_tokens: None,
        };
        let body = Body::from_json(&req).unwrap();
        Signal::builder(Kind::ContextPack).body(body).build()
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

        let result = cell.execute(input, &roko_graph::CellContext::new()).await;
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("safety"));
    }

    #[tokio::test]
    async fn aggregate_fails_without_task_context() {
        let scope = test_scope();
        let cell = AggregateCell::new();

        let input = vec![make_request_signal(&scope), make_safety_signal(&scope)];

        let result = cell.execute(input, &roko_graph::CellContext::new()).await;
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
            Signal::builder(Kind::ContextPack).body(body).build()
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
                vec![PromptSection::new("knowledge_fact", "first").with_section_id("dup-id")],
            );
            let body = Body::from_json(&payload).unwrap();
            Signal::builder(Kind::ContextPack).body(body).build()
        };
        let dup2 = {
            let payload = KnowledgeSections::new(
                scope.clone(),
                vec![PromptSection::new("knowledge_fact", "second").with_section_id("dup-id")],
            );
            let body = Body::from_json(&payload).unwrap();
            Signal::builder(Kind::ContextPack).body(body).build()
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
            Signal::builder(Kind::ContextPack).body(body).build()
        };

        // Knowledge with low priority and long content.
        let knowledge = {
            let long_content = "x".repeat(1000); // ~250 tokens
            let payload = KnowledgeSections::new(
                scope.clone(),
                vec![
                    PromptSection::new("knowledge_fact", long_content)
                        .with_priority(SectionPriority::Low),
                ],
            );
            let body = Body::from_json(&payload).unwrap();
            Signal::builder(Kind::ContextPack).body(body).build()
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
        assert!(
            prompt
                .warnings
                .iter()
                .any(|w| w.contains("budget pressure"))
        );
    }

    #[test]
    fn classify_section_ordering() {
        let safety = PromptSection::new("safety_notice", "x");
        let role = PromptSection::new("role_identity", "x");
        let task = PromptSection::new("task_brief", "x");
        let knowledge = PromptSection::new("knowledge_fact", "x");
        let gate = PromptSection::new("gate_feedback", "x");

        assert!((classify_section(&safety) as u8) < (classify_section(&role) as u8));
        assert!((classify_section(&role) as u8) < (classify_section(&knowledge) as u8));
        assert!((classify_section(&knowledge) as u8) < (classify_section(&task) as u8));
        assert!((classify_section(&task) as u8) < (classify_section(&gate) as u8));
    }

    #[test]
    fn classify_experiment_section() {
        let exp =
            PromptSection::new("custom_exp", "x").with_section_id("experiment:001:custom_exp");
        let mut exp_with_id = exp.clone();
        exp_with_id.experiment_id = Some("001".into());
        assert_eq!(
            classify_section(&exp_with_id) as u8,
            AggregateGroup::ExperimentAnnotations as u8
        );
    }

    /// Full-pipeline test: all 7 providers feed into the aggregate.
    /// Verifies non-duplication, correct ordering across all groups, and
    /// that each provider's sections appear exactly once.
    #[tokio::test]
    async fn full_pipeline_all_seven_providers() {
        let scope = test_scope();
        let cell = AggregateCell::new();

        // Build one signal per provider, each with a uniquely-named section.
        let safety_sig = make_safety_signal(&scope);
        let task_ctx_sig = make_task_context_signal(&scope);
        let knowledge_sig = make_knowledge_signal(&scope);

        let episodes_sig = {
            let payload = EpisodeSections::new(
                scope.clone(),
                vec![PromptSection::new("error_pattern", "E0277: add trait bound")],
            );
            let body = Body::from_json(&payload).unwrap();
            Signal::builder(Kind::ContextPack).body(body).build()
        };
        let playbook_sig = {
            let payload = PlaybookSections::new(
                scope.clone(),
                vec![PromptSection::new("playbook_match", "Use builder pattern")],
            );
            let body = Body::from_json(&payload).unwrap();
            Signal::builder(Kind::ContextPack).body(body).build()
        };
        let modulation_sig = {
            let payload = ModulationSections::new(
                scope.clone(),
                vec![PromptSection::new("affect", "Focus on correctness")],
            );
            let body = Body::from_json(&payload).unwrap();
            Signal::builder(Kind::ContextPack).body(body).build()
        };
        let experiment_sig = {
            let mut exp_section = PromptSection::new("exp_hint", "Try approach B");
            exp_section.experiment_id = Some("exp-42".into());
            let payload = ExperimentAssignment::new(
                scope.clone(),
                vec![exp_section],
                vec!["exp-42".into()],
            );
            let body = Body::from_json(&payload).unwrap();
            Signal::builder(Kind::ContextPack).body(body).build()
        };

        let input = vec![
            make_request_signal(&scope),
            // Deliberately shuffled order to verify aggregate sorts correctly.
            experiment_sig,
            modulation_sig,
            episodes_sig,
            playbook_sig,
            knowledge_sig,
            task_ctx_sig,
            safety_sig,
        ];

        let result = cell
            .execute(input, &roko_graph::CellContext::new())
            .await
            .unwrap();
        assert_eq!(result.len(), 1);

        let prompt: ComposedPrompt = result[0].body.as_json().unwrap();

        // All 7 sections should be present.
        assert_eq!(prompt.included_section_ids.len(), 7);
        assert!(prompt.dropped_section_ids.is_empty());

        // Verify ordering: safety < knowledge < episodes < playbook < task < modulation < experiment.
        let text = &prompt.text;
        let safety_pos = text.find("Do not modify safety files").unwrap();
        let knowledge_pos = text.find("Rust uses ownership").unwrap();
        let episodes_pos = text.find("E0277: add trait bound").unwrap();
        let playbook_pos = text.find("Use builder pattern").unwrap();
        let task_pos = text.find("Implement the widget").unwrap();
        let modulation_pos = text.find("Focus on correctness").unwrap();
        let experiment_pos = text.find("Try approach B").unwrap();

        assert!(safety_pos < knowledge_pos, "safety before knowledge");
        assert!(knowledge_pos < episodes_pos, "knowledge before episodes");
        assert!(episodes_pos < playbook_pos, "episodes before playbook");
        assert!(playbook_pos < task_pos, "playbook before task");
        assert!(task_pos < modulation_pos, "task before modulation");
        assert!(modulation_pos < experiment_pos, "modulation before experiment");

        // Experiment IDs propagated.
        assert_eq!(prompt.active_experiment_ids, vec!["exp-42"]);
    }

    /// Duplicate sections from multiple providers are deduplicated.
    /// The same section ID appearing in knowledge and episodes should
    /// only appear once in the output.
    #[tokio::test]
    async fn cross_provider_deduplication() {
        let scope = test_scope();
        let cell = AggregateCell::new();

        let shared_id = "shared-section-001";

        let knowledge_sig = {
            let payload = KnowledgeSections::new(
                scope.clone(),
                vec![PromptSection::new("knowledge_fact", "first copy")
                    .with_section_id(shared_id)],
            );
            let body = Body::from_json(&payload).unwrap();
            Signal::builder(Kind::ContextPack).body(body).build()
        };
        let episodes_sig = {
            let payload = EpisodeSections::new(
                scope.clone(),
                vec![PromptSection::new("error_pattern", "second copy")
                    .with_section_id(shared_id)],
            );
            let body = Body::from_json(&payload).unwrap();
            Signal::builder(Kind::ContextPack).body(body).build()
        };

        let input = vec![
            make_request_signal(&scope),
            make_safety_signal(&scope),
            make_task_context_signal(&scope),
            knowledge_sig,
            episodes_sig,
        ];

        let result = cell
            .execute(input, &roko_graph::CellContext::new())
            .await
            .unwrap();
        let prompt: ComposedPrompt = result[0].body.as_json().unwrap();

        // The shared section ID should appear exactly once.
        let dup_count = prompt
            .included_section_ids
            .iter()
            .filter(|id| id.as_str() == shared_id)
            .count();
        assert_eq!(dup_count, 1, "cross-provider duplicate not deduplicated");

        // Only the first copy (knowledge) should be in the text.
        assert!(prompt.text.contains("first copy"));
        assert!(!prompt.text.contains("second copy"));
    }

    /// Deterministic output: same inputs always produce the same prompt text.
    #[tokio::test]
    async fn deterministic_prompt_output() {
        let scope = test_scope();
        let cell = AggregateCell::new();

        let build_input = || {
            vec![
                make_request_signal(&scope),
                make_safety_signal(&scope),
                make_task_context_signal(&scope),
                make_knowledge_signal(&scope),
            ]
        };

        let result1 = cell
            .execute(build_input(), &roko_graph::CellContext::new())
            .await
            .unwrap();
        let prompt1: ComposedPrompt = result1[0].body.as_json().unwrap();

        let result2 = cell
            .execute(build_input(), &roko_graph::CellContext::new())
            .await
            .unwrap();
        let prompt2: ComposedPrompt = result2[0].body.as_json().unwrap();

        assert_eq!(prompt1.text, prompt2.text, "prompt text not deterministic");
        assert_eq!(
            prompt1.included_section_ids, prompt2.included_section_ids,
            "included IDs not deterministic"
        );
        assert_eq!(
            prompt1.estimated_tokens, prompt2.estimated_tokens,
            "token count not deterministic"
        );
    }
}
