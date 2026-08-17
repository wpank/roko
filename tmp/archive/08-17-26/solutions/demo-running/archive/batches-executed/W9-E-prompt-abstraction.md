# W9-E: Unified Prompt Pipeline — Generalized, Testable, Loggable Prompt Assembly

**Priority**: P1 — structural improvement enabling better debugging, testing, and extensibility across all prompt paths
**Effort**: 8-12 hours
**Files to modify**: 7 files, 2 new files
**Dependencies**: None (builds on existing `PromptAssembler` and `SystemPromptBuilder`)

## Problem

Prompt construction in roko is scattered across four independent code paths, each with different levels of structure, testability, and observability:

1. **`dispatch/prompt_builder.rs` (`PromptAssembler`)** — Used by runner v2. Has its own `PromptSection`, `PromptContext`, `PromptDiagnostics`, `PromptSectionSource` trait, and budget enforcement. Produces a minimal prompt (role, task, files, acceptance, verify, retry) plus pluggable knowledge/playbook/effectiveness sources. Does NOT use `ImplementerTemplate` or `SystemPromptBuilder`.

2. **`task_parser.rs` (`TaskDef::build_prompt`)** — Used by `orchestrate.rs` (runner v1). Hand-builds a user prompt by string concatenation from `TaskDef` fields. No section metadata, no token budget, no diagnostics. Inlines file content via `std::fs::read_to_string`.

3. **`prompting.rs` + `dispatch_helpers.rs` (`build_system_prompt*`)** — Used by `orchestrate.rs` (runner v1). Wraps `RoleSystemPromptSpec` from `roko-compose`. Has `PromptBuildOptions` with affect state, complexity, skills, playbooks, code context, pheromones. Produces a system prompt string but no structured diagnostics about what was included/dropped.

4. **`plan_generate.rs` (`build_generator_system_prompt`)** — Used by PRD plan generation. Concatenates a static `PLAN_GENERATOR_SYSTEM_PROMPT` constant with workspace-specific appendices (naming glossary, CLAUDE.md). No sections, no budget, no diagnostics.

### Consequences

- **Not queryable**: There is no way to inspect what prompt an agent actually received. The `PromptAssemblyDiagnostics` struct in `runner/types.rs` captures section names and token estimates but not the actual prompt text. An operator cannot run `roko plan show-prompt <plan> <task>` to see the full prompt.

- **Not testable end-to-end**: The `ImplementerTemplate`, `StrategistTemplate`, and `ResearcherTemplate` each have unit tests for section generation, but there are no integration tests that verify the full pipeline from `TaskDef` -> assembled prompt matches expectations. The two runtime paths (v1 orchestrate.rs, v2 runner) produce different prompts for the same task.

- **Not loggable at the right level**: When a task fails, diagnosing "was the prompt missing context?" requires reading tracing logs for `dispatch prompt assembled` debug lines (event_loop.rs:2208-2216). There is no file artifact that captures the full prompt sent per task per attempt.

- **Not extensible**: Adding a new section source (e.g., code-intelligence index results, dependency graph context) requires modifying `PromptAssembler::assemble` directly (for v2) AND `build_system_prompt_with_context_validated` (for v1). The two paths must be kept in sync manually.

- **Duplicate types**: `dispatch/prompt_builder.rs` defines its own `PromptSection` (line 195, with `name`, `body`, `drop_priority`) that shadows `roko_compose::PromptSection` (with `name`, `content`, `priority`, `cache_layer`, `placement`, `hard_cap`, `bidder`). These are structurally similar but incompatible.

### Existing infrastructure to build on

The codebase already has most of the primitives needed:

| Component | Location | What it provides |
|---|---|---|
| `PromptSection` | `roko-compose/src/prompt.rs:105` | Rich section type with priority, cache layer, placement, hard cap, bidder |
| `PromptComposer` | `roko-compose/src/prompt.rs` | Budget-aware section assembly with VCG auction, foraging, diagnostics |
| `RolePromptTemplate` trait | `roko-compose/src/templates/mod.rs:82` | `sections(&self, input) -> Vec<PromptSection>` + `role_identity()` |
| `SystemPromptBuilder` | `roko-compose/src/system_prompt_builder.rs` | 9-layer builder with cache markers, budget profiles, section effectiveness |
| `RoleSystemPromptSpec` | `roko-compose/src/system_prompt_builder.rs` | Convenience wrapper around `SystemPromptBuilder` per role |
| `PromptSectionSource` trait | `dispatch/prompt_builder.rs:226` | Pluggable section provider (knowledge, playbook, effectiveness) |
| `PromptDiagnostics` | `dispatch/prompt_builder.rs:178` | Included/dropped sections, token estimate, knowledge/playbook ids |
| `PromptAssemblyDiagnostics` | `runner/types.rs:1181` | Runner event version of diagnostics |

The gap is a **unified pipeline** that composes these primitives into a single, shared path with full logging.

## Design

### New trait: `PromptPipeline`

A pipeline is a composable, named sequence that takes a `PromptRequest` and produces a `PromptAssembly`. Every prompt construction path (plan-run dispatch, PRD generation, research, `roko run`) goes through a pipeline instance.

```
PromptRequest --[pipeline]--> PromptAssembly
                                  |
                                  +--> system_prompt: String
                                  +--> user_prompt: String
                                  +--> diagnostics: PromptDiagnostics
                                  +--> sections_snapshot: Vec<SectionSnapshot>
```

### Prompt logging

Every assembled prompt is persisted to `.roko/prompts/{plan_id}/{task_id}_attempt{N}.md` as a human-readable markdown file with section headers, token counts, and metadata. This enables:
- Post-mortem debugging (`cat .roko/prompts/my-plan/T3_attempt1.md`)
- CLI inspection (`roko plan show-prompt plans/my-plan T3`)
- Diff between attempts (`diff .roko/prompts/my-plan/T3_attempt0.md T3_attempt1.md`)

### Consolidation strategy

Rather than rewriting all four paths at once, this batch introduces the pipeline trait and a single canonical implementation (`TaskDispatchPipeline`) that replaces the v2 `PromptAssembler::assemble`. The v1 path in `orchestrate.rs` and the plan-generation path are left as-is but documented as future migration targets.

## Exact Code to Change

### File 1 (NEW): `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/pipeline.rs`

Create the `PromptPipeline` trait, `PromptRequest`, `PromptAssembly`, and `SectionSnapshot` types.

**New file content:**

```rust
//! Unified prompt pipeline: composable, loggable, testable prompt assembly.
//!
//! Every prompt construction path should go through a [`PromptPipeline`]
//! implementation. The pipeline takes a [`PromptRequest`] describing what
//! the caller needs, and produces a [`PromptAssembly`] with the rendered
//! prompt plus structured diagnostics for logging, testing, and debugging.
//!
//! ## Design invariants
//!
//! 1. **No I/O in the pipeline itself.** All context (workspace map, PRD
//!    excerpt, knowledge entries, playbooks) arrives via `PromptRequest`.
//!    Callers are responsible for loading context before calling `assemble`.
//! 2. **Deterministic.** Same `PromptRequest` always produces the same
//!    `PromptAssembly`. No randomness, no clock reads, no filesystem access.
//! 3. **Section-level granularity.** Every section that enters the pipeline
//!    is individually tracked — included or dropped, with token count and
//!    drop reason. This enables budget forensics and A/B testing.

use std::collections::HashMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::prompt::estimate_tokens;

// ─── Request ──────────────────────────────────────────────────────────

/// Everything the pipeline needs to assemble a prompt.
///
/// Callers construct this from their domain objects (TaskDef, DispatchContext,
/// etc.) and hand it to the pipeline. The pipeline never reaches outside
/// this struct for context.
#[derive(Debug, Clone, Default)]
pub struct PromptRequest {
    /// Unique identifier for logging (e.g., "my-plan/T3/attempt-1").
    pub prompt_id: String,
    /// Logical role name ("implementer", "strategist", "researcher", ...).
    pub role: String,
    /// Plan identifier.
    pub plan_id: String,
    /// Task identifier within the plan.
    pub task_id: String,
    /// Task title for display.
    pub task_title: String,
    /// Task description / body.
    pub task_description: Option<String>,
    /// Files in scope for the task.
    pub files_in_scope: Vec<String>,
    /// Acceptance criteria strings.
    pub acceptance_criteria: Vec<String>,
    /// Verification commands the agent should run.
    pub verify_commands: Vec<String>,
    /// Attempt number (0 = first try).
    pub attempt: u32,
    /// Token budget for the assembled prompt.
    pub token_budget: u32,
    /// Pre-loaded context sections from external sources.
    /// Each entry is (section_name, section_body).
    pub context_sections: Vec<(String, String)>,
    /// Structured gate feedback for retry attempts.
    pub gate_feedback: Option<GateFeedbackPayload>,
    /// Arbitrary metadata carried through to the assembly.
    pub metadata: HashMap<String, String>,
}

/// Structured gate feedback for retry prompts.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GateFeedbackPayload {
    /// Compile errors from cargo check.
    #[serde(default)]
    pub compile_errors: Vec<String>,
    /// Failing test names + summaries.
    #[serde(default)]
    pub test_failures: Vec<String>,
    /// Clippy warnings.
    #[serde(default)]
    pub clippy_warnings: Vec<String>,
    /// Original gate output (truncated).
    pub raw_output: String,
}

// ─── Assembly ─────────────────────────────────────────────────────────

/// The assembled prompt with full diagnostics.
///
/// Returned by every `PromptPipeline::assemble` call. Contains the
/// rendered prompt strings plus a complete audit trail of what was
/// included, what was dropped, and why.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptAssembly {
    /// Rendered system prompt.
    pub system_prompt: String,
    /// Rendered user prompt (task instructions).
    pub user_prompt: String,
    /// Optional tool allowlist.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_allowlist: Option<Vec<String>>,
    /// Section-level audit trail.
    pub sections: Vec<SectionSnapshot>,
    /// Estimated total token count for system + user prompts.
    pub total_tokens_est: u32,
    /// Names of sections that were dropped to fit the budget.
    pub dropped_sections: Vec<String>,
    /// Knowledge entry ids that contributed to the prompt.
    #[serde(default)]
    pub knowledge_ids: Vec<String>,
    /// Playbook ids that contributed to the prompt.
    #[serde(default)]
    pub playbook_ids: Vec<String>,
    /// Arbitrary metadata from the request, carried through for logging.
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl PromptAssembly {
    /// Convert to the simpler diagnostics struct used by runner events.
    #[must_use]
    pub fn diagnostics(&self) -> PromptDiagnosticsCompat {
        PromptDiagnosticsCompat {
            included_sections: self
                .sections
                .iter()
                .filter(|s| s.included)
                .map(|s| s.name.clone())
                .collect(),
            dropped_sections: self.dropped_sections.clone(),
            estimated_tokens: self.total_tokens_est,
            knowledge_ids: self.knowledge_ids.clone(),
            playbook_ids: self.playbook_ids.clone(),
        }
    }

    /// Render the assembly as a human-readable markdown document for logging.
    ///
    /// Format:
    /// ```text
    /// # Prompt Assembly: {prompt_id}
    /// - Role: implementer
    /// - Total tokens (est): 12345
    /// - Sections included: 8
    /// - Sections dropped: 2
    ///
    /// ## System Prompt
    /// <system prompt text>
    ///
    /// ## User Prompt
    /// <user prompt text>
    ///
    /// ## Section Audit
    /// | Section | Tokens | Priority | Included | Drop Reason |
    /// |---------|--------|----------|----------|-------------|
    /// | role    | 150    | critical | yes      |             |
    /// | ...     |        |          |          |             |
    /// ```
    #[must_use]
    pub fn render_log(&self, prompt_id: &str) -> String {
        let mut out = String::with_capacity(self.system_prompt.len() + self.user_prompt.len() + 2048);
        out.push_str(&format!("# Prompt Assembly: {prompt_id}\n\n"));
        out.push_str(&format!("- **Role**: {}\n", self.metadata.get("role").map_or("unknown", String::as_str)));
        out.push_str(&format!("- **Total tokens (est)**: {}\n", self.total_tokens_est));
        let included_count = self.sections.iter().filter(|s| s.included).count();
        out.push_str(&format!("- **Sections included**: {included_count}\n"));
        out.push_str(&format!("- **Sections dropped**: {}\n", self.dropped_sections.len()));
        if !self.knowledge_ids.is_empty() {
            out.push_str(&format!("- **Knowledge ids**: {}\n", self.knowledge_ids.join(", ")));
        }
        if !self.playbook_ids.is_empty() {
            out.push_str(&format!("- **Playbook ids**: {}\n", self.playbook_ids.join(", ")));
        }
        for (key, value) in &self.metadata {
            if key != "role" {
                out.push_str(&format!("- **{key}**: {value}\n"));
            }
        }

        out.push_str("\n---\n\n## System Prompt\n\n");
        out.push_str(&self.system_prompt);

        out.push_str("\n\n---\n\n## User Prompt\n\n");
        out.push_str(&self.user_prompt);

        out.push_str("\n\n---\n\n## Section Audit\n\n");
        out.push_str("| Section | Tokens | Priority | Included | Drop Reason |\n");
        out.push_str("|---------|--------|----------|----------|-------------|\n");
        for section in &self.sections {
            out.push_str(&format!(
                "| {} | {} | {} | {} | {} |\n",
                section.name,
                section.token_count,
                section.priority,
                if section.included { "yes" } else { "no" },
                section.drop_reason.as_deref().unwrap_or(""),
            ));
        }

        out
    }
}

/// Backward-compatible diagnostics struct matching `PromptDiagnostics`
/// in `dispatch/prompt_builder.rs` and `PromptAssemblyDiagnostics` in
/// `runner/types.rs`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PromptDiagnosticsCompat {
    pub included_sections: Vec<String>,
    pub dropped_sections: Vec<String>,
    pub estimated_tokens: u32,
    pub knowledge_ids: Vec<String>,
    pub playbook_ids: Vec<String>,
}

/// Snapshot of a single section's fate in the assembly pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionSnapshot {
    /// Section name (e.g., "role", "task", "knowledge", "playbooks").
    pub name: String,
    /// Estimated token count for this section's content.
    pub token_count: u32,
    /// Priority level.
    pub priority: String,
    /// Whether this section was included in the final prompt.
    pub included: bool,
    /// If dropped, why (e.g., "budget_exceeded", "empty_content").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drop_reason: Option<String>,
    /// Which source provided this section.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

// ─── Pipeline trait ───────────────────────────────────────────────────

/// A composable, named prompt assembly pipeline.
///
/// Implementations take a `PromptRequest` and produce a `PromptAssembly`.
/// The trait is object-safe so pipelines can be stored in dispatch
/// configuration and swapped at runtime.
pub trait PromptPipeline: Send + Sync + fmt::Debug {
    /// Assemble the full prompt from a request.
    ///
    /// Implementations must be deterministic: same request -> same assembly.
    fn assemble(&self, request: &PromptRequest) -> PromptAssembly;

    /// Human-readable pipeline name for logging and diagnostics.
    fn name(&self) -> &str;
}

// ─── Default implementation ───────────────────────────────────────────

/// Standard task-dispatch pipeline.
///
/// Assembles prompts using the section-based approach: each piece of
/// context becomes a named section with a priority. Sections are sorted
/// by priority and dropped from the bottom when the token budget is
/// exceeded.
#[derive(Debug)]
pub struct StandardPipeline;

impl PromptPipeline for StandardPipeline {
    fn name(&self) -> &str {
        "standard"
    }

    fn assemble(&self, request: &PromptRequest) -> PromptAssembly {
        let mut sections: Vec<(SectionSnapshot, String)> = Vec::new();

        // 1. Role section (critical, never dropped).
        let role_body = format!("# Role\nYou are the **{}** for this task.", request.role);
        sections.push((
            SectionSnapshot {
                name: "role".into(),
                token_count: estimate_tokens(&role_body) as u32,
                priority: "critical".into(),
                included: true,
                drop_reason: None,
                source: Some("builtin".into()),
            },
            role_body,
        ));

        // 2. Task section (critical).
        let task_body = format!(
            "# Task\n**{}**: {}",
            request.task_id,
            request
                .task_description
                .as_deref()
                .unwrap_or(&request.task_title),
        );
        sections.push((
            SectionSnapshot {
                name: "task".into(),
                token_count: estimate_tokens(&task_body) as u32,
                priority: "critical".into(),
                included: true,
                drop_reason: None,
                source: Some("builtin".into()),
            },
            task_body,
        ));

        // 3. Files section (high).
        if !request.files_in_scope.is_empty() {
            let body = format!(
                "# Files in scope\n{}",
                request
                    .files_in_scope
                    .iter()
                    .map(|f| format!("- `{f}`"))
                    .collect::<Vec<_>>()
                    .join("\n"),
            );
            sections.push((
                SectionSnapshot {
                    name: "files".into(),
                    token_count: estimate_tokens(&body) as u32,
                    priority: "high".into(),
                    included: true,
                    drop_reason: None,
                    source: Some("builtin".into()),
                },
                body,
            ));
        }

        // 4. Acceptance section (high).
        if !request.acceptance_criteria.is_empty() {
            let body = format!(
                "# Acceptance criteria\n{}",
                request
                    .acceptance_criteria
                    .iter()
                    .map(|c| format!("- {c}"))
                    .collect::<Vec<_>>()
                    .join("\n"),
            );
            sections.push((
                SectionSnapshot {
                    name: "acceptance".into(),
                    token_count: estimate_tokens(&body) as u32,
                    priority: "high".into(),
                    included: true,
                    drop_reason: None,
                    source: Some("builtin".into()),
                },
                body,
            ));
        }

        // 5. Verify section (high).
        if !request.verify_commands.is_empty() {
            let body = format!(
                "# Verify\nAfter editing, run:\n{}",
                request
                    .verify_commands
                    .iter()
                    .map(|v| format!("- `{v}`"))
                    .collect::<Vec<_>>()
                    .join("\n"),
            );
            sections.push((
                SectionSnapshot {
                    name: "verify".into(),
                    token_count: estimate_tokens(&body) as u32,
                    priority: "high".into(),
                    included: true,
                    drop_reason: None,
                    source: Some("builtin".into()),
                },
                body,
            ));
        }

        // 6. Context sections from external sources (normal priority).
        for (name, body) in &request.context_sections {
            if body.trim().is_empty() {
                continue;
            }
            sections.push((
                SectionSnapshot {
                    name: name.clone(),
                    token_count: estimate_tokens(body) as u32,
                    priority: "normal".into(),
                    included: true,
                    drop_reason: None,
                    source: Some("context".into()),
                },
                body.clone(),
            ));
        }

        // 7. Gate feedback section (normal, only on retry).
        if request.attempt > 0 {
            if let Some(ref feedback) = request.gate_feedback {
                let body = render_gate_feedback_payload(feedback);
                if !body.is_empty() {
                    sections.push((
                        SectionSnapshot {
                            name: "retry_feedback".into(),
                            token_count: estimate_tokens(&body) as u32,
                            priority: "normal".into(),
                            included: true,
                            drop_reason: None,
                            source: Some("gate".into()),
                        },
                        body,
                    ));
                }
            }
        }

        // ── Budget enforcement ────────────────────────────────────────
        // Drop lowest-priority sections first until we fit the budget.
        let priority_order = |p: &str| -> u32 {
            match p {
                "critical" => 0,
                "high" => 1,
                "normal" => 2,
                "low" => 3,
                _ => 4,
            }
        };

        let budget = request.token_budget.max(1) as usize;
        let total: usize = sections.iter().map(|(snap, _)| snap.token_count as usize).sum();
        if total > budget {
            // Sort indices by priority (lowest priority = highest index = drop first).
            let mut indices: Vec<usize> = (0..sections.len()).collect();
            indices.sort_by(|&a, &b| {
                let pa = priority_order(&sections[b].0.priority);
                let pb = priority_order(&sections[a].0.priority);
                pa.cmp(&pb)
            });

            let mut current = total;
            for idx in indices {
                if current <= budget {
                    break;
                }
                // Never drop critical sections.
                if sections[idx].0.priority == "critical" {
                    continue;
                }
                sections[idx].0.included = false;
                sections[idx].0.drop_reason = Some("budget_exceeded".into());
                current -= sections[idx].0.token_count as usize;
            }
        }

        // ── Render ────────────────────────────────────────────────────
        let system_parts: Vec<&str> = sections
            .iter()
            .filter(|(snap, _)| snap.included)
            .map(|(_, body)| body.as_str())
            .collect();
        let system_prompt = system_parts.join("\n\n");

        let mut user_prompt = format!("# Task Request\n{}\n", request.task_title);
        if let Some(description) = &request.task_description {
            user_prompt.push_str("\n## Details\n");
            user_prompt.push_str(description);
            user_prompt.push('\n');
        }
        if !request.acceptance_criteria.is_empty() {
            user_prompt.push_str("\n## Acceptance\n");
            for item in &request.acceptance_criteria {
                user_prompt.push_str("- ");
                user_prompt.push_str(item);
                user_prompt.push('\n');
            }
        }
        if !request.verify_commands.is_empty() {
            user_prompt.push_str("\n## Verification Commands\n");
            for cmd in &request.verify_commands {
                user_prompt.push_str("- ");
                user_prompt.push_str(cmd);
                user_prompt.push('\n');
            }
        }

        let snapshots: Vec<SectionSnapshot> = sections.iter().map(|(snap, _)| snap.clone()).collect();
        let dropped: Vec<String> = snapshots
            .iter()
            .filter(|s| !s.included)
            .map(|s| s.name.clone())
            .collect();
        let total_tokens = estimate_tokens(&system_prompt) as u32
            + estimate_tokens(&user_prompt) as u32;

        PromptAssembly {
            system_prompt,
            user_prompt,
            tool_allowlist: None,
            sections: snapshots,
            total_tokens_est: total_tokens,
            dropped_sections: dropped,
            knowledge_ids: Vec::new(),
            playbook_ids: Vec::new(),
            metadata: request.metadata.clone(),
        }
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────

fn render_gate_feedback_payload(feedback: &GateFeedbackPayload) -> String {
    let mut body = String::new();
    if !feedback.compile_errors.is_empty() {
        body.push_str("# Retry: Compile Errors\n");
        for err in &feedback.compile_errors {
            body.push_str("- ");
            body.push_str(err);
            body.push('\n');
        }
        body.push('\n');
    }
    if !feedback.test_failures.is_empty() {
        body.push_str("# Retry: Test Failures\n");
        for fail in &feedback.test_failures {
            body.push_str("- ");
            body.push_str(fail);
            body.push('\n');
        }
        body.push('\n');
    }
    if !feedback.clippy_warnings.is_empty() {
        body.push_str("# Retry: Clippy Warnings\n");
        for warn in &feedback.clippy_warnings {
            body.push_str("- ");
            body.push_str(warn);
            body.push('\n');
        }
        body.push('\n');
    }
    body
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_request() -> PromptRequest {
        PromptRequest {
            prompt_id: "test-plan/T1/attempt-0".into(),
            role: "implementer".into(),
            plan_id: "test-plan".into(),
            task_id: "T1".into(),
            task_title: "Add error handling".into(),
            task_description: Some("Add proper error handling to the parser module.".into()),
            files_in_scope: vec!["crates/parser/src/lib.rs".into()],
            acceptance_criteria: vec!["No unwrap() in library code".into()],
            verify_commands: vec!["cargo test -p parser".into()],
            attempt: 0,
            token_budget: 100_000,
            context_sections: vec![
                ("workspace_map".into(), "crates/parser/src/lib.rs\ncrates/parser/src/error.rs".into()),
            ],
            gate_feedback: None,
            metadata: [("role".into(), "implementer".into())].into_iter().collect(),
        }
    }

    #[test]
    fn standard_pipeline_produces_all_sections() {
        let pipeline = StandardPipeline;
        let assembly = pipeline.assemble(&sample_request());
        let names: Vec<&str> = assembly.sections.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"role"));
        assert!(names.contains(&"task"));
        assert!(names.contains(&"files"));
        assert!(names.contains(&"acceptance"));
        assert!(names.contains(&"verify"));
        assert!(names.contains(&"workspace_map"));
    }

    #[test]
    fn standard_pipeline_deterministic() {
        let pipeline = StandardPipeline;
        let req = sample_request();
        let a1 = pipeline.assemble(&req);
        let a2 = pipeline.assemble(&req);
        assert_eq!(a1.system_prompt, a2.system_prompt);
        assert_eq!(a1.user_prompt, a2.user_prompt);
        assert_eq!(a1.sections.len(), a2.sections.len());
    }

    #[test]
    fn standard_pipeline_drops_under_budget() {
        let pipeline = StandardPipeline;
        let mut req = sample_request();
        req.token_budget = 50; // Very small budget.
        req.context_sections = vec![
            ("big_context".into(), "x".repeat(2000)),
        ];
        let assembly = pipeline.assemble(&req);
        // Critical sections (role, task) should survive.
        assert!(assembly.sections.iter().any(|s| s.name == "role" && s.included));
        assert!(assembly.sections.iter().any(|s| s.name == "task" && s.included));
        // At least one section should be dropped.
        assert!(!assembly.dropped_sections.is_empty());
    }

    #[test]
    fn retry_includes_gate_feedback() {
        let pipeline = StandardPipeline;
        let mut req = sample_request();
        req.attempt = 1;
        req.gate_feedback = Some(GateFeedbackPayload {
            compile_errors: vec!["error[E0308]: mismatched types".into()],
            test_failures: Vec::new(),
            clippy_warnings: Vec::new(),
            raw_output: "error[E0308]: mismatched types".into(),
        });
        let assembly = pipeline.assemble(&req);
        assert!(assembly.sections.iter().any(|s| s.name == "retry_feedback"));
    }

    #[test]
    fn render_log_contains_all_metadata() {
        let pipeline = StandardPipeline;
        let assembly = pipeline.assemble(&sample_request());
        let log = assembly.render_log("test-plan/T1/attempt-0");
        assert!(log.contains("# Prompt Assembly: test-plan/T1/attempt-0"));
        assert!(log.contains("implementer"));
        assert!(log.contains("## System Prompt"));
        assert!(log.contains("## User Prompt"));
        assert!(log.contains("## Section Audit"));
        assert!(log.contains("| role"));
        assert!(log.contains("| task"));
    }

    #[test]
    fn empty_context_sections_skipped() {
        let pipeline = StandardPipeline;
        let mut req = sample_request();
        req.context_sections = vec![
            ("empty".into(), "   ".into()),
            ("nonempty".into(), "real content".into()),
        ];
        let assembly = pipeline.assemble(&req);
        let names: Vec<&str> = assembly.sections.iter().map(|s| s.name.as_str()).collect();
        assert!(!names.contains(&"empty"));
        assert!(names.contains(&"nonempty"));
    }

    #[test]
    fn name_returns_standard() {
        let pipeline = StandardPipeline;
        assert_eq!(pipeline.name(), "standard");
    }
}
```

### File 2: `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/lib.rs`

#### Change 1: Add `pipeline` module declaration and re-exports

**Find:**
```rust
pub mod prompt;
```

**Replace with:**
```rust
pub mod pipeline;
pub mod prompt;
```

**Find the re-export block** (will be near other `pub use` statements). Add after the last `pub use` line in the re-export block:

```rust
pub use pipeline::{
    GateFeedbackPayload, PromptAssembly, PromptDiagnosticsCompat, PromptPipeline, PromptRequest,
    SectionSnapshot, StandardPipeline,
};
```

### File 3 (NEW): `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/prompt_log.rs`

Prompt logging utility that writes assembled prompts to `.roko/prompts/`.

**New file content:**

```rust
//! Prompt logging — persist every assembled prompt for debugging.
//!
//! Writes assembled prompts to `.roko/prompts/{plan_id}/{task_id}_attempt{N}.md`
//! so operators can inspect exactly what each agent saw.

use std::path::Path;

use crate::pipeline::PromptAssembly;

/// Write an assembled prompt to the logging directory.
///
/// Creates the directory tree if it does not exist. Errors are logged
/// but never propagated — prompt logging must not break dispatch.
pub fn log_prompt(
    roko_dir: &Path,
    plan_id: &str,
    task_id: &str,
    attempt: u32,
    assembly: &PromptAssembly,
) {
    let prompt_id = format!("{plan_id}/{task_id}/attempt-{attempt}");
    let dir = roko_dir.join("prompts").join(plan_id);
    if let Err(err) = std::fs::create_dir_all(&dir) {
        tracing::warn!(
            error = %err,
            plan_id,
            task_id,
            "failed to create prompt log directory"
        );
        return;
    }

    let filename = format!("{task_id}_attempt{attempt}.md");
    let path = dir.join(&filename);
    let content = assembly.render_log(&prompt_id);

    if let Err(err) = std::fs::write(&path, &content) {
        tracing::warn!(
            error = %err,
            path = %path.display(),
            "failed to write prompt log"
        );
    } else {
        tracing::debug!(
            path = %path.display(),
            tokens = assembly.total_tokens_est,
            sections = assembly.sections.len(),
            "prompt logged"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::{PromptRequest, StandardPipeline, PromptPipeline};

    #[test]
    fn log_prompt_creates_file() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let roko_dir = tmp.path().join(".roko");
        std::fs::create_dir_all(&roko_dir).expect("create .roko");

        let pipeline = StandardPipeline;
        let assembly = pipeline.assemble(&PromptRequest {
            prompt_id: "test/T1/attempt-0".into(),
            role: "implementer".into(),
            plan_id: "test".into(),
            task_id: "T1".into(),
            task_title: "Test task".into(),
            token_budget: 100_000,
            ..Default::default()
        });

        log_prompt(&roko_dir, "test", "T1", 0, &assembly);

        let expected_path = roko_dir.join("prompts").join("test").join("T1_attempt0.md");
        assert!(expected_path.exists(), "prompt log file should exist");
        let content = std::fs::read_to_string(&expected_path).expect("read log");
        assert!(content.contains("# Prompt Assembly"));
        assert!(content.contains("## System Prompt"));
        assert!(content.contains("## Section Audit"));
    }

    #[test]
    fn log_prompt_handles_missing_dir_gracefully() {
        // Point to a non-writable path — should not panic.
        let roko_dir = Path::new("/nonexistent/path/.roko");
        let pipeline = StandardPipeline;
        let assembly = pipeline.assemble(&PromptRequest {
            prompt_id: "test/T1/attempt-0".into(),
            role: "implementer".into(),
            plan_id: "test".into(),
            task_id: "T1".into(),
            task_title: "Test task".into(),
            token_budget: 100_000,
            ..Default::default()
        });
        // Should not panic.
        log_prompt(roko_dir, "test", "T1", 0, &assembly);
    }
}
```

### File 4: `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/lib.rs`

#### Change 2: Add `prompt_log` module declaration

**Find (after the `pipeline` module added in Change 1):**
```rust
pub mod pipeline;
pub mod prompt;
```

**Replace with:**
```rust
pub mod pipeline;
pub mod prompt;
pub mod prompt_log;
```

### File 5: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch/prompt_builder.rs`

#### Change 1: Add pipeline bridge to `AssembledPrompt`

This connects the existing `AssembledPrompt` to the new `PromptAssembly` type so callers can migrate incrementally.

**Find (line ~164):**
```rust
/// Assembled prompt, allowlist, and diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssembledPrompt {
    /// Rendered system prompt.
    pub system_prompt: String,
    /// Rendered user prompt.
    pub user_prompt: String,
    /// Optional tool allowlist (intersected with safety contract).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_allowlist: Option<Vec<String>>,
    /// Per-assembly diagnostics for experiments + projection.
    pub diagnostics: PromptDiagnostics,
}
```

**Replace with:**
```rust
/// Assembled prompt, allowlist, and diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssembledPrompt {
    /// Rendered system prompt.
    pub system_prompt: String,
    /// Rendered user prompt.
    pub user_prompt: String,
    /// Optional tool allowlist (intersected with safety contract).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_allowlist: Option<Vec<String>>,
    /// Per-assembly diagnostics for experiments + projection.
    pub diagnostics: PromptDiagnostics,
}

impl AssembledPrompt {
    /// Convert from a `PromptAssembly` (the unified pipeline output).
    ///
    /// This bridge allows callers to migrate to the pipeline incrementally:
    /// construct via `PromptPipeline::assemble`, then convert to
    /// `AssembledPrompt` for the existing dispatch machinery.
    #[must_use]
    pub fn from_pipeline_assembly(assembly: roko_compose::PromptAssembly) -> Self {
        let compat = assembly.diagnostics();
        Self {
            system_prompt: assembly.system_prompt,
            user_prompt: assembly.user_prompt,
            tool_allowlist: assembly.tool_allowlist,
            diagnostics: PromptDiagnostics {
                included_sections: compat.included_sections,
                dropped_sections: compat.dropped_sections,
                estimated_tokens: compat.estimated_tokens,
                playbook_ids: compat.playbook_ids,
                knowledge_ids: compat.knowledge_ids,
            },
        }
    }
}
```

### File 6: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch/prompt_builder.rs`

#### Change 2: Add prompt logging to `PromptAssembler::assemble`

**Find (line ~488, at the end of the `assemble` method):**
```rust
        Ok(AssembledPrompt {
            system_prompt,
            user_prompt,
            tool_allowlist: allowlist,
            diagnostics,
        })
    }
```

**Replace with:**
```rust
        let assembled = AssembledPrompt {
            system_prompt,
            user_prompt,
            tool_allowlist: allowlist,
            diagnostics,
        };

        // Log the assembled prompt for debugging.
        let roko_dir = ctx.workdir.join(".roko");
        if roko_dir.exists() {
            let pipeline_assembly = roko_compose::PromptAssembly {
                system_prompt: assembled.system_prompt.clone(),
                user_prompt: assembled.user_prompt.clone(),
                tool_allowlist: assembled.tool_allowlist.clone(),
                sections: Vec::new(), // Detailed sections not available from legacy path.
                total_tokens_est: assembled.diagnostics.estimated_tokens,
                dropped_sections: assembled.diagnostics.dropped_sections.clone(),
                knowledge_ids: assembled.diagnostics.knowledge_ids.clone(),
                playbook_ids: assembled.diagnostics.playbook_ids.clone(),
                metadata: [("role".into(), ctx.role.clone())].into_iter().collect(),
            };
            roko_compose::prompt_log::log_prompt(
                &roko_dir,
                &ctx.plan_id,
                &task.id,
                ctx.attempt,
                &pipeline_assembly,
            );
        }

        Ok(assembled)
    }
```

### File 7a: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/main.rs`

**Important**: The `PlanCmd` enum is in `main.rs` (line 1100), NOT in `commands/plan.rs`. The handler is in `commands/plan.rs`.

#### Change 7a.1: Add `ShowPrompt` variant to the `PlanCmd` enum

**Find the `PlanCmd` enum** in `main.rs` (search for `enum PlanCmd`). Add a new variant anywhere within the enum (suggested: after the existing `Validate` variant or at the end):

```rust
    /// Show the assembled prompt for a specific task (dry-run prompt inspection).
    ShowPrompt {
        /// Path to the plan directory.
        plan_dir: PathBuf,
        /// Task ID to show the prompt for (e.g., "T1").
        task_id: String,
        /// Attempt number to show (default: 0).
        #[arg(long, default_value = "0")]
        attempt: u32,
    },
```

### File 7b: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/plan.rs`

#### Change 7b.1: Add `ShowPrompt` handler

**Add the handler** in the `cmd_plan` match arm (which matches on `PlanCmd`):

```rust
            PlanCmd::ShowPrompt {
                plan_dir,
                task_id,
                attempt,
            } => {
                let workdir = std::env::current_dir().context("current dir")?;
                let roko_dir = workdir.join(".roko");
                let plan_id = plan_dir
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown");
                let log_path = roko_dir
                    .join("prompts")
                    .join(plan_id)
                    .join(format!("{task_id}_attempt{attempt}.md"));

                if log_path.exists() {
                    let content = std::fs::read_to_string(&log_path)
                        .with_context(|| format!("reading {}", log_path.display()))?;
                    println!("{content}");
                } else {
                    // Fallback: assemble a fresh prompt from the plan's tasks.toml.
                    let tasks_path = plan_dir.join("tasks.toml");
                    if !tasks_path.exists() {
                        anyhow::bail!(
                            "no prompt log at {} and no tasks.toml at {}",
                            log_path.display(),
                            tasks_path.display()
                        );
                    }
                    let tasks_file = crate::task_parser::TasksFile::parse(&tasks_path)
                        .context("parsing tasks.toml")?;
                    let task_def = tasks_file
                        .tasks
                        .iter()
                        .find(|t| t.id == task_id)
                        .with_context(|| format!("task {task_id} not found in {}", tasks_path.display()))?;

                    let pipeline = roko_compose::StandardPipeline;
                    let request = roko_compose::PromptRequest {
                        prompt_id: format!("{plan_id}/{task_id}/attempt-{attempt}"),
                        role: task_def
                            .role
                            .clone()
                            .unwrap_or_else(|| "implementer".into()),
                        plan_id: plan_id.into(),
                        task_id: task_id.clone(),
                        task_title: task_def.title.clone(),
                        task_description: task_def.description.clone(),
                        files_in_scope: task_def.files.clone(),
                        acceptance_criteria: task_def.acceptance.clone(),
                        verify_commands: task_def
                            .verify
                            .iter()
                            .map(|v| v.command.clone())
                            .collect(),
                        attempt,
                        token_budget: 200_000,
                        context_sections: Vec::new(),
                        gate_feedback: None,
                        metadata: [
                            ("role".into(), task_def.role.clone().unwrap_or_else(|| "implementer".into())),
                            ("tier".into(), task_def.tier.clone()),
                        ]
                        .into_iter()
                        .collect(),
                    };
                    let assembly = roko_compose::PromptPipeline::assemble(&pipeline, &request);
                    println!("{}", assembly.render_log(&request.prompt_id));
                }
                Ok(())
            }
```

## Testing

### Unit tests (already included in File 1)

The `pipeline.rs` module includes 6 tests:
- `standard_pipeline_produces_all_sections` — verifies all expected sections are present
- `standard_pipeline_deterministic` — same input produces same output
- `standard_pipeline_drops_under_budget` — budget enforcement preserves critical sections
- `retry_includes_gate_feedback` — gate feedback injected on retry
- `render_log_contains_all_metadata` — log format includes all audit fields
- `empty_context_sections_skipped` — blank sections excluded

### Unit tests (already included in File 3)

The `prompt_log.rs` module includes 2 tests:
- `log_prompt_creates_file` — verifies file creation and content
- `log_prompt_handles_missing_dir_gracefully` — no panic on bad paths

### Integration test: full pipeline round-trip

Run after applying all changes:

```bash
# Verify roko-compose compiles with new modules
cargo check -p roko-compose

# Run pipeline tests
cargo test -p roko-compose -- pipeline

# Run prompt_log tests
cargo test -p roko-compose -- prompt_log

# Verify roko-cli compiles (prompt_builder changes + show-prompt command)
cargo check -p roko-cli

# Run existing prompt_builder tests (ensure no regressions)
cargo test -p roko-cli -- prompt_builder

# Full workspace check
cargo clippy --workspace --no-deps -- -D warnings
```

### Manual verification of show-prompt command

```bash
# If a plan exists with logged prompts:
cargo run -p roko-cli -- plan show-prompt plans/some-plan T1

# If no log exists, dry-run assembly from tasks.toml:
cargo run -p roko-cli -- plan show-prompt plans/some-plan T1 --attempt 0
```

## Migration Path (future batches, not this one)

This batch establishes the pipeline trait and wires logging into the v2 dispatch path. The remaining work to fully unify prompting:

1. **Migrate `PromptAssembler::assemble` to delegate to `StandardPipeline`** — Replace the inline section construction in `assemble()` with `StandardPipeline::assemble()` + `AssembledPrompt::from_pipeline_assembly()`. The existing `PromptSectionSource` plugins (knowledge, playbook, effectiveness) would become `context_sections` entries on the `PromptRequest`.

2. **Migrate v1 orchestrate.rs** — Replace `build_system_prompt_with_context_validated` + `TaskDef::build_prompt` with a pipeline call. The `RoleSystemPromptSpec` builder becomes the system-prompt layer of a richer pipeline implementation.

3. **Migrate PRD generation** — Replace `build_generator_system_prompt` with a `PlanGeneratorPipeline` implementation that uses the same logging and diagnostics.

4. **Wire `ImplementerTemplate` sections into `PromptRequest::context_sections`** — Load workspace_map, tasks, brief, preflight, registry at the dispatch site and pass them as context sections so the rich template content reaches the agent.

5. **Add prompt experiments** — Use the pipeline's deterministic output to A/B test prompt variations (e.g., with/without workspace map) and measure gate pass rates.

## What This Does NOT Change

- Does not modify `orchestrate.rs` (runner v1) prompt paths — those continue using `RoleSystemPromptSpec` and `TaskDef::build_prompt` unchanged.
- Does not modify `SystemPromptBuilder` or `RolePromptTemplate` — the compose-crate builders remain as-is. The pipeline is a layer above them.
- Does not modify `plan_generate.rs` — PRD/plan generation prompts remain hand-built for now.
- Does not change agent dispatch behavior — the assembled prompt content is identical; only logging and the pipeline abstraction are new.
- Does not add runtime overhead to the hot path — `log_prompt` writes happen after assembly and are fire-and-forget (errors logged, not propagated).

## Audit Status

Audited: 2026-05-05. 2 issues fixed -- (1) Removed unused `SectionPriority` import from pipeline.rs that would trigger clippy warning; (2) Fixed File 7 to correctly reference `PlanCmd` enum in `main.rs` instead of non-existent `PlanSubcommand` enum, and split into File 7a (main.rs enum variant) and File 7b (commands/plan.rs handler)
