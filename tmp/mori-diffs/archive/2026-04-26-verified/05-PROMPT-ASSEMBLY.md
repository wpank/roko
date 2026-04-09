# Path 5: Prompt Assembly -- 9-Layer Builder Integration

> Completed in implementation pass 2026-04-26. Verified by composed prompt and gate-feedback tests plus no-mock Codex/Claude plan runs.

## Current State (What's Broken)

The runner v2 builds prompts manually in `agent_stream.rs`, completely bypassing the 9-layer `RoleSystemPromptSpec` builder that the orchestrator already uses. This results in stripped-down, role-blind prompts that miss most of the context the orchestrator provides.

### Problem 1: Manual Prompt Construction (No RoleSystemPromptSpec)

The runner builds system prompts via `build_minimal_system_prompt()` (agent_stream.rs:310-340):

```rust
/// TODO: Replace with `RoleSystemPromptSpec` 9-layer builder (Phase 5 R028-R029).
pub fn build_minimal_system_prompt(
    task: &crate::task_parser::TaskDef,
    plan_id: &str,
) -> String {
    let role = task.role.as_deref().unwrap_or("implementer");
    let mut prompt = format!(
        "You are a {role} agent working on plan `{plan_id}`, task `{}`.\n\n",
        task.id
    );
    prompt.push_str("## Constraints\n");
    prompt.push_str("- Make minimal, targeted changes.\n");
    prompt.push_str("- Do not modify files outside the task scope.\n");
    prompt.push_str("- Ensure `cargo check` passes before finishing.\n");
    // ... acceptance + verify ...
    prompt
}
```

This is a 1-layer prompt. Meanwhile, the orchestrator uses the full 9-layer builder via `dispatch_helpers.rs` -> `prompting.rs` -> `roko-compose::RoleSystemPromptSpec`, which produces prompts with role identity, conventions, domain context, pheromone signals, task context, tool instructions, playbooks, anti-patterns, and affect guidance.

The orchestrator's `build_system_prompt_with_context_validated()` (dispatch_helpers.rs:122-161) accepts:
- Role-scoped `AgentRole`
- `TaskContext` with plan_id, workspace, domain notes
- Tool allowlist CSV
- Affect state (`PadState`)
- Complexity band
- Extra conventions (file scope, max_loc)
- Anti-patterns
- Relevant skills and playbooks
- Code-intelligence context chunks
- Pheromone signals
- Section effectiveness registry (for learned priority adjustments)

The runner uses **none** of this.

### Problem 2: No Playbook Injection

The orchestrator queries the `PlaybookStore` for relevant playbooks and injects them into the prompt (orchestrate.rs:14366-14368):

```rust
let playbook_query = playbook_query_context(role, task, &task_text, task_def.as_ref());
let relevant_playbooks = match self.playbook.query(&playbook_query).await {
    Ok(playbooks) => playbooks,
    Err(err) => { /* ... */ Vec::new() }
};
```

These playbooks contain learned strategies from successful past tasks. The runner has no playbook lookup -- agents re-discover strategies from scratch on every task.

### Problem 3: No Neuro Anti-Pattern Warnings

The orchestrator builds a "learned-context string" from skills, playbook rules, and patterns (orchestrate.rs:10121-10177). This includes anti-patterns from the neuro knowledge store -- things like "this approach caused a regression in task X" or "prefer `spawn_agent_with_layer` over direct `create_agent_for_model`". The runner injects zero learned context.

### Problem 4: Raw Gate Failure Feedback

When a gate fails and the agent retries, the runner prepends raw gate output as unstructured text (event_loop.rs:480-487):

```rust
let final_prompt = if !ctx.state.gate_output.is_empty() {
    format!(
        "## Previous Verify Failure\n\n{}\n\n---\n\n{prompt}",
        ctx.state.gate_output
    )
} else {
    prompt
};
```

This is a wall of text from `cargo clippy` or `cargo test` output. The orchestrator (via `build_gate_failure_plan_revision`, orchestrate.rs:4900) structures this into parsed errors, affected files, and actionable guidance. The raw text approach wastes agent context window on noise.

### Problem 5: No Per-Role Tool Allowlists

The runner passes no tool restrictions to agents. Every agent gets the same unrestricted tool access regardless of role. The orchestrator uses `claude_task_tool_allowlist_with()` (dispatch_helpers.rs:309-360) to scope tools per-role:

| Role | Tools |
|------|-------|
| Implementer | Full: Read, Edit, Write, Bash, Glob, Grep |
| Reviewer | Read-only: Read, Glob, Grep |
| Researcher | No-edit: Read, Glob, Grep, WebSearch, WebFetch |
| Conductor | Read + planning: Read, Glob, Grep, Edit |
| AutoFixer | Same as Implementer |

The runner gives all roles all tools.

### Problem 6: No File Scope Constraint

`TaskDef.files` declares which files the task is allowed to modify. The orchestrator injects this as a convention in the system prompt via `task_dispatch_conventions()` (dispatch_helpers.rs:52-80). The runner ignores file scope entirely.

### Problem 7: No Acceptance Criteria / Verification in System Prompt

While `build_minimal_system_prompt` does include acceptance criteria and verify commands, it does so in a minimal format without the structured sections, priority ordering, and cache alignment that `RoleSystemPromptSpec` provides.

## Design Goals

1. **Reuse existing infrastructure**: Use `RoleSystemPromptSpec`, `PromptBuildOptions`, and `build_role_system_prompt_validated()` from `prompting.rs` -- do not reimplement.
2. **Wire all 9 layers**: Every layer the orchestrator uses should be available in the runner.
3. **Structured gate feedback**: Parse gate errors into file/line/code/message structs.
4. **Per-role tool scoping**: Use `claude_task_tool_allowlist_with()` for tool restrictions.
5. **Playbook + neuro injection**: Query stores at dispatch time, inject into prompt.
6. **Context-window aware**: Use `build_role_system_prompt_validated()` with token budget, so prompts are trimmed to fit.

## Architecture

### New Module: `crates/roko-cli/src/dispatch/prompt_builder.rs`

This module wraps the existing `prompting.rs` and `dispatch_helpers.rs` infrastructure into a single struct that the runner event loop can call.

### New Types

```rust
// dispatch/prompt_builder.rs

use std::path::PathBuf;
use std::sync::Arc;

use roko_compose::{ContextChunk, PadState, TaskContext};
use roko_core::AgentRole;
use roko_learn::playbook::{Playbook, PlaybookStore};
use roko_learn::section_effect::SectionEffectivenessRegistry;
use roko_learn::skill_library::{Skill, SkillLibrary};

use crate::config::Config;
use crate::dispatch::DispatchRequest;
use crate::task_parser::TaskDef;

/// Assembles system prompts using the 9-layer RoleSystemPromptSpec builder.
///
/// Replaces `build_minimal_system_prompt()` in `agent_stream.rs` with the
/// full prompt pipeline that the orchestrator uses.
pub struct PromptAssembler {
    /// CLI config (prompt budget, conventions, etc.).
    config: Arc<Config>,
    /// Playbook store for querying relevant strategies.
    playbook_store: Arc<PlaybookStore>,
    /// Skill library for querying relevant skills.
    skill_library: Arc<SkillLibrary>,
    /// Section effectiveness registry for learned priority adjustments.
    section_effectiveness: Option<Arc<SectionEffectivenessRegistry>>,
    /// Working directory for code-intelligence index.
    workdir: PathBuf,
    /// Cached workspace code index (lazy-loaded).
    code_index: Option<Arc<roko_index::WorkspaceIndex>>,
}

/// The assembled prompt pair returned by the builder.
pub struct AssembledPrompt {
    /// The user-facing task prompt.
    pub user_prompt: String,
    /// The system prompt (9-layer assembled).
    pub system_prompt: String,
    /// Tool allowlist CSV for the agent.
    pub tools_csv: String,
    /// Metadata about what was injected (for logging/debugging).
    pub metadata: PromptMetadata,
}

/// What went into the prompt (for observability).
#[derive(Debug, Default)]
pub struct PromptMetadata {
    /// Number of playbooks injected.
    pub playbooks_injected: usize,
    /// Number of skills injected.
    pub skills_injected: usize,
    /// Number of anti-patterns injected.
    pub anti_patterns_injected: usize,
    /// Number of code context chunks injected.
    pub code_context_chunks: usize,
    /// Number of pheromone signals injected.
    pub pheromones_injected: usize,
    /// Estimated token count of the system prompt.
    pub system_prompt_tokens: usize,
    /// Whether the prompt was trimmed to fit context window.
    pub was_trimmed: bool,
    /// The complexity band used.
    pub complexity: String,
    /// Role identity used.
    pub role: String,
}

/// Structured gate failure feedback for retry prompts.
///
/// Replaces the raw `gate_output` string with parsed, actionable feedback.
pub struct GateFeedback {
    /// Which gate failed.
    pub gate_name: String,
    /// Rung number.
    pub rung: u32,
    /// Parsed individual errors.
    pub errors: Vec<GateError>,
    /// Summary of what failed (1-2 sentences).
    pub summary: String,
    /// Raw output (preserved for fallback).
    pub raw_output: String,
}

/// A single parsed error from gate output.
#[derive(Debug, Clone)]
pub struct GateError {
    /// Source file path.
    pub file: Option<String>,
    /// Line number in the file.
    pub line: Option<u32>,
    /// Column number.
    pub column: Option<u32>,
    /// Error/warning code (e.g., E0308, clippy::needless_borrow).
    pub code: Option<String>,
    /// Error severity (error, warning).
    pub severity: String,
    /// The error message.
    pub message: String,
}
```

### Implementation

```rust
// dispatch/prompt_builder.rs — impl PromptAssembler

impl PromptAssembler {
    pub fn new(
        config: Arc<Config>,
        playbook_store: Arc<PlaybookStore>,
        skill_library: Arc<SkillLibrary>,
        section_effectiveness: Option<Arc<SectionEffectivenessRegistry>>,
        workdir: PathBuf,
    ) -> Self {
        Self {
            config,
            playbook_store,
            skill_library,
            section_effectiveness,
            workdir,
            code_index: None,
        }
    }

    /// Build the full prompt pair for a task dispatch.
    ///
    /// This is the main entry point called by the event loop before
    /// dispatching an agent. It:
    ///
    /// 1. Resolves the agent role from the TaskDef
    /// 2. Builds the user prompt from the TaskDef
    /// 3. Queries playbook store for relevant strategies
    /// 4. Queries skill library for relevant skills
    /// 5. Gathers code-intelligence context
    /// 6. Computes per-role tool allowlist
    /// 7. Assembles the 9-layer system prompt via RoleSystemPromptSpec
    /// 8. Injects gate feedback if this is a retry
    pub async fn build_for_task(
        &self,
        task_def: &TaskDef,
        plan_id: &str,
        gate_feedback: Option<&GateFeedback>,
    ) -> AssembledPrompt {
        let role = task_def.role_enum();

        // 1. User prompt
        let user_prompt = self.build_user_prompt(task_def, plan_id, gate_feedback);

        // 2. Tool allowlist
        let tools_csv = self.build_tool_allowlist(role, task_def);

        // 3. Query playbooks
        let playbooks = self.query_playbooks(role, task_def).await;

        // 4. Query skills
        let skills = self.query_skills(task_def).await;

        // 5. Code context
        let code_context = self.gather_code_context(task_def);

        // 6. Anti-patterns (from task context + neuro store)
        let anti_patterns = self.gather_anti_patterns(task_def);

        // 7. Build 9-layer system prompt
        let (system_prompt, was_trimmed) = self.assemble_system_prompt(
            role,
            plan_id,
            task_def,
            &tools_csv,
            &playbooks,
            &skills,
            &code_context,
            &anti_patterns,
        );

        let metadata = PromptMetadata {
            playbooks_injected: playbooks.len(),
            skills_injected: skills.len(),
            anti_patterns_injected: anti_patterns.len(),
            code_context_chunks: code_context.len(),
            pheromones_injected: 0, // TODO: wire pheromone source
            system_prompt_tokens: roko_compose::estimate_tokens(&system_prompt),
            was_trimmed,
            complexity: format!("{:?}", self.task_complexity(task_def)),
            role: role.label().to_string(),
        };

        AssembledPrompt {
            user_prompt,
            system_prompt,
            tools_csv,
            metadata,
        }
    }

    // ─── User Prompt ─────────────────────────────────────────────────

    fn build_user_prompt(
        &self,
        task_def: &TaskDef,
        plan_id: &str,
        gate_feedback: Option<&GateFeedback>,
    ) -> String {
        let base_prompt = task_def.build_prompt(plan_id, &self.workdir);

        match gate_feedback {
            Some(feedback) => self.prepend_structured_gate_feedback(&base_prompt, feedback),
            None => base_prompt,
        }
    }

    /// Prepend structured gate failure context instead of raw text.
    fn prepend_structured_gate_feedback(
        &self,
        base_prompt: &str,
        feedback: &GateFeedback,
    ) -> String {
        let mut sections = Vec::new();
        sections.push(format!("## Previous Verification Failure\n"));
        sections.push(format!(
            "Gate `{}` (rung {}) failed: {}\n",
            feedback.gate_name, feedback.rung, feedback.summary
        ));

        if !feedback.errors.is_empty() {
            sections.push("### Errors to Fix\n".to_string());
            for (i, error) in feedback.errors.iter().enumerate().take(20) {
                let location = match (&error.file, error.line) {
                    (Some(file), Some(line)) => format!("{}:{}", file, line),
                    (Some(file), None) => file.clone(),
                    _ => "unknown".to_string(),
                };
                let code_str = error.code.as_deref().unwrap_or("");
                sections.push(format!(
                    "{}. **{}** `{}` at `{}`: {}",
                    i + 1,
                    error.severity,
                    code_str,
                    location,
                    error.message,
                ));
            }

            let remaining = feedback.errors.len().saturating_sub(20);
            if remaining > 0 {
                sections.push(format!("\n... and {remaining} more errors."));
            }
        }

        sections.push("\n### Instructions\n".to_string());
        sections.push(
            "Fix all errors listed above. Do not introduce new warnings. \
             Run `cargo check` to verify your fixes compile.\n"
                .to_string(),
        );
        sections.push(format!("---\n\n{base_prompt}"));

        sections.join("\n")
    }

    // ─── Tool Allowlist ──────────────────────────────────────────────

    fn build_tool_allowlist(&self, role: AgentRole, task_def: &TaskDef) -> String {
        use crate::dispatch_helpers::claude_task_tool_allowlist_with;

        claude_task_tool_allowlist_with(
            role,
            task_def.allowed_tools.as_deref(),
            task_def.denied_tools.as_deref(),
            None, // dynamic_registry — wire when MCP registry available
        )
    }

    // ─── Playbook Queries ────────────────────────────────────────────

    async fn query_playbooks(
        &self,
        role: AgentRole,
        task_def: &TaskDef,
    ) -> Vec<Playbook> {
        use crate::learning_helpers::playbook_query_context;

        let task_text = format!("{}: {}", task_def.title, task_def.description);
        let query = playbook_query_context(
            role.label(),
            &task_def.id,
            &task_text,
            Some(task_def),
        );

        match self.playbook_store.query(&query).await {
            Ok(playbooks) => {
                if !playbooks.is_empty() {
                    tracing::debug!(
                        count = playbooks.len(),
                        task = %task_def.id,
                        "injecting relevant playbooks"
                    );
                }
                playbooks
            }
            Err(err) => {
                tracing::warn!(err = %err, "playbook query failed");
                Vec::new()
            }
        }
    }

    // ─── Skill Queries ───────────────────────────────────────────────

    async fn query_skills(&self, task_def: &TaskDef) -> Vec<Skill> {
        let query = format!("{} {}", task_def.title, task_def.description);
        match self.skill_library.search(&query, 5).await {
            Ok(skills) => skills,
            Err(err) => {
                tracing::warn!(err = %err, "skill library query failed");
                Vec::new()
            }
        }
    }

    // ─── Code Context ────────────────────────────────────────────────

    fn gather_code_context(&self, task_def: &TaskDef) -> Vec<String> {
        use crate::dispatch_helpers::code_context_for_task;

        let description = format!("{}\n{}", task_def.title, task_def.description);
        code_context_for_task(
            &self.workdir,
            &description,
            self.code_index.as_deref(),
        )
    }

    // ─── Anti-Patterns ───────────────────────────────────────────────

    fn gather_anti_patterns(&self, task_def: &TaskDef) -> Vec<String> {
        let mut patterns = Vec::new();

        // From task context (declared in tasks.toml)
        if let Some(ref ctx) = task_def.context {
            patterns.extend(ctx.anti_patterns.iter().cloned());
        }

        // TODO: Query neuro store for anti-patterns matching task description.
        // This requires wiring roko_neuro::KnowledgeStore into PromptAssembler.
        // For now, the declared anti-patterns are the only source.

        patterns
    }

    // ─── System Prompt Assembly (9 Layers) ───────────────────────────

    fn assemble_system_prompt(
        &self,
        role: AgentRole,
        plan_id: &str,
        task_def: &TaskDef,
        tools_csv: &str,
        playbooks: &[Playbook],
        skills: &[Skill],
        code_context: &[String],
        anti_patterns: &[String],
    ) -> (String, bool) {
        use crate::dispatch_helpers::{
            effective_context_window_tokens, prompt_budget_complexity,
            task_dispatch_conventions,
        };
        use crate::prompting::{PromptBuildOptions, build_role_system_prompt_validated};

        let task_description = format!("{}: {}", task_def.title, task_def.description);
        let mut task_context = TaskContext::new(&task_description)
            .with_plan_id(plan_id)
            .with_workspace("roko-cli runner");

        // Add acceptance criteria as task context
        if !task_def.acceptance.is_empty() {
            let criteria = task_def.acceptance.join("\n- ");
            task_context = task_context.with_context(
                &format!("## Acceptance Criteria\n- {criteria}")
            );
        }

        // Add verify commands as task context
        if !task_def.verify.is_empty() {
            let verifications: Vec<String> = task_def.verify.iter().map(|v| {
                format!("`{}` ({})", v.command, v.phase)
            }).collect();
            task_context = task_context.with_context(
                &format!(
                    "## Verification Commands\nThese commands will run after your changes:\n- {}",
                    verifications.join("\n- ")
                )
            );
        }

        let options = PromptBuildOptions {
            affect_state: None,  // TODO: wire DaimonState
            complexity: Some(prompt_budget_complexity(Some(task_def))),
            extra_conventions: task_dispatch_conventions(Some(task_def)),
            extra_anti_patterns: anti_patterns.to_vec(),
            relevant_skills: skills.to_vec(),
            relevant_playbooks: playbooks.to_vec(),
            code_context: code_context.to_vec(),
            pheromones: Vec::new(),  // TODO: wire pheromone source
        };

        let context_window = effective_context_window_tokens(&self.config);

        match build_role_system_prompt_validated(
            role,
            task_context.clone(),
            tools_csv,
            options.clone(),
            context_window,
            self.section_effectiveness.as_deref(),
        ) {
            Ok(prompt) => (prompt, false),
            Err(err) => {
                tracing::warn!(err = %err, "validated prompt build failed, falling back");
                // Fallback: build without validation (no trimming)
                let prompt = crate::prompting::build_role_system_prompt(
                    role,
                    task_context,
                    tools_csv,
                    options,
                );
                (prompt, false)
            }
        }
    }

    fn task_complexity(
        &self,
        task_def: &TaskDef,
    ) -> roko_compose::Complexity {
        crate::dispatch_helpers::prompt_budget_complexity(Some(task_def))
    }
}
```

### The 9 Layers

Here is what each layer contributes, matching `RoleSystemPromptSpec` in `roko-compose/src/role_prompts.rs` and `SystemPromptBuilder` in `roko-compose/src/system_prompt_builder.rs`:

| # | Layer | Source | Cache Tier | Content |
|---|-------|--------|------------|---------|
| 1 | **Role Identity** | `role_identity_for(role)` via role templates | System (stable) | "You are an implementer agent. Your job is to write correct, minimal code changes..." |
| 2 | **Conventions** | `DEFAULT_CONVENTIONS_SUFFIX` + `task_dispatch_conventions()` | System (semi-stable) | "Keep changes minimal...", file scope constraints, max_loc budget |
| 3 | **Domain Context** | `task_context.with_context()` + code-intelligence chunks | Session (semi-stable) | Code symbols, file references, acceptance criteria, verify commands |
| 3c | **Active Signals** | `pheromones` via `ContextChunk` | Session (semi-stable) | Stigmergic pheromone signals from concurrent agents |
| 4 | **Task Context** | `TaskContext::new(task_description)` | Task (volatile) | The actual task title and description |
| 5 | **Tool Instructions** | `tools_csv` via `claude_task_tool_allowlist_with()` | System (stable) | "Available tools: Read, Edit, Write, Bash, Glob, Grep" |
| 6 | **Relevant Techniques** | `relevant_playbooks` + `relevant_skills` | Task (volatile) | Learned strategies from past successful tasks |
| 7 | **Anti-Patterns** | `DEFAULT_ANTI_PATTERNS` + task context + neuro store | Task (volatile) | "Do not reimplement existing modules..." |
| 8 | **Affect Guidance** | `PadState` from DaimonState | Dynamic | Tone/focus: "slow down, prefer caution" or "keep momentum" |

The builder orders these by cache stability: layers 1+2+5 form the prefix-cacheable system tier, layers 3+3c form the session tier, layers 4+6+7 are per-task, and layer 8 is dynamic. Cache alignment markers (`<!-- cache:system -->`, etc.) are inserted between tiers when `with_cache_markers()` is called.

### Gate Feedback Parser

```rust
// dispatch/prompt_builder.rs

impl GateFeedback {
    /// Parse gate output into structured feedback.
    ///
    /// Handles three gate output formats:
    /// - Cargo compiler errors (rustc)
    /// - Clippy warnings/errors
    /// - Test failures (cargo test output)
    pub fn parse(gate_name: &str, rung: u32, raw_output: &str) -> Self {
        let errors = match gate_name {
            "compile" | "check" => parse_rustc_errors(raw_output),
            "clippy" => parse_clippy_errors(raw_output),
            "test" => parse_test_failures(raw_output),
            _ => Vec::new(),
        };

        let summary = if errors.is_empty() {
            format!("{gate_name} gate failed (could not parse errors)")
        } else {
            let error_count = errors.iter().filter(|e| e.severity == "error").count();
            let warning_count = errors.iter().filter(|e| e.severity == "warning").count();
            let files: Vec<&str> = errors
                .iter()
                .filter_map(|e| e.file.as_deref())
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .take(5)
                .collect();
            format!(
                "{error_count} error(s), {warning_count} warning(s) in {} file(s): {}",
                files.len(),
                files.join(", "),
            )
        };

        Self {
            gate_name: gate_name.to_string(),
            rung,
            errors,
            summary,
            raw_output: raw_output.to_string(),
        }
    }
}

/// Parse rustc/cargo check error output into structured errors.
fn parse_rustc_errors(output: &str) -> Vec<GateError> {
    let mut errors = Vec::new();

    // Match lines like: "error[E0308]: mismatched types"
    // Followed by: "  --> crates/foo/src/bar.rs:42:10"
    let lines: Vec<&str> = output.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim();

        // Match "error[CODE]: message" or "warning[CODE]: message"
        if let Some(parsed) = parse_rustc_diagnostic_line(line) {
            // Look for location on next lines
            let location = (i + 1..lines.len())
                .find_map(|j| parse_location_line(lines[j]));

            errors.push(GateError {
                file: location.as_ref().map(|(f, _, _)| f.clone()),
                line: location.as_ref().and_then(|(_, l, _)| *l),
                column: location.as_ref().and_then(|(_, _, c)| *c),
                code: parsed.code,
                severity: parsed.severity,
                message: parsed.message,
            });
        }
        i += 1;
    }

    errors
}

struct DiagnosticLine {
    severity: String,
    code: Option<String>,
    message: String,
}

fn parse_rustc_diagnostic_line(line: &str) -> Option<DiagnosticLine> {
    // "error[E0308]: mismatched types"
    // "warning: unused variable"
    let (severity, rest) = if line.starts_with("error[") {
        ("error", &line[5..])
    } else if line.starts_with("error:") {
        return Some(DiagnosticLine {
            severity: "error".to_string(),
            code: None,
            message: line[6..].trim().to_string(),
        });
    } else if line.starts_with("warning[") {
        ("warning", &line[7..])
    } else if line.starts_with("warning:") {
        return Some(DiagnosticLine {
            severity: "warning".to_string(),
            code: None,
            message: line[8..].trim().to_string(),
        });
    } else {
        return None;
    };

    // Extract code from brackets
    let code_end = rest.find(']')?;
    let code = rest[1..code_end].to_string();
    let message = rest[code_end + 1..].trim_start_matches(':').trim().to_string();

    Some(DiagnosticLine {
        severity: severity.to_string(),
        code: Some(code),
        message,
    })
}

fn parse_location_line(line: &str) -> Option<(String, Option<u32>, Option<u32>)> {
    let trimmed = line.trim();
    // "  --> crates/foo/src/bar.rs:42:10"
    let arrow = trimmed.strip_prefix("-->")?;
    let path_str = arrow.trim();
    let parts: Vec<&str> = path_str.rsplitn(3, ':').collect();

    match parts.len() {
        3 => {
            let col = parts[0].parse().ok();
            let line_num = parts[1].parse().ok();
            let file = parts[2].to_string();
            Some((file, line_num, col))
        }
        2 => {
            let line_num = parts[0].parse().ok();
            let file = parts[1].to_string();
            Some((file, line_num, None))
        }
        _ => Some((path_str.to_string(), None, None)),
    }
}

/// Parse clippy diagnostic output (same format as rustc, with clippy:: codes).
fn parse_clippy_errors(output: &str) -> Vec<GateError> {
    // Clippy uses the same format as rustc
    parse_rustc_errors(output)
}

/// Parse cargo test failure output.
fn parse_test_failures(output: &str) -> Vec<GateError> {
    let mut errors = Vec::new();

    for line in output.lines() {
        let trimmed = line.trim();
        // "test crate::module::test_name ... FAILED"
        if trimmed.starts_with("test ") && trimmed.ends_with("FAILED") {
            let test_name = trimmed
                .strip_prefix("test ")
                .and_then(|s| s.strip_suffix(" ... FAILED"))
                .unwrap_or(trimmed);

            errors.push(GateError {
                file: None,
                line: None,
                column: None,
                code: None,
                severity: "error".to_string(),
                message: format!("test failed: {test_name}"),
            });
        }
    }

    errors
}
```

### Integration Points

The event loop constructs `PromptAssembler` once and passes it into `RunContext`:

```rust
// runner/event_loop.rs — in run()

// Build prompt assembler with stores.
let playbook_store = load_or_create_playbook_store(&paths.playbooks_dir).await?;
let skill_library = load_or_create_skill_library(&paths.skills_dir).await?;
let section_effectiveness = load_section_effectiveness(&paths.section_effects_json).ok();

let prompt_assembler = PromptAssembler::new(
    Arc::new(cli_config),
    Arc::new(playbook_store),
    Arc::new(skill_library),
    section_effectiveness.map(Arc::new),
    config.workdir.clone(),
);
```

In the `SpawnAgent` action handler:

```rust
// runner/event_loop.rs — SpawnAgent branch

// Parse gate feedback if this is a retry
let gate_feedback = if !ctx.state.gate_output.is_empty() {
    Some(GateFeedback::parse(
        &ctx.state.last_gate_name,
        ctx.state.last_gate_rung,
        &ctx.state.gate_output,
    ))
} else {
    None
};

// Build prompt via PromptAssembler
let assembled = ctx.prompt_assembler.build_for_task(
    &task_def,
    plan_id,
    gate_feedback.as_ref(),
).await;

tracing::info!(
    task = %task_id,
    playbooks = assembled.metadata.playbooks_injected,
    skills = assembled.metadata.skills_injected,
    anti_patterns = assembled.metadata.anti_patterns_injected,
    tokens = assembled.metadata.system_prompt_tokens,
    trimmed = assembled.metadata.was_trimmed,
    "assembled prompt"
);

// Pass to dispatcher (see 01-AGENT-DISPATCH.md)
let request = DispatchRequest {
    prompt: assembled.user_prompt,
    system_prompt: assembled.system_prompt,
    // ...
};
```

## Error Handling

| Error | Behavior |
|-------|----------|
| Playbook store query fails | Log warning, continue with empty playbooks |
| Skill library query fails | Log warning, continue with empty skills |
| Code index not available | Log debug, continue without code context |
| Validated prompt build fails | Log warning, fall back to unvalidated build |
| Gate feedback parse fails | Fall back to raw text prepend (current behavior) |
| Section effectiveness load fails | Continue without learned priority adjustments |

The prompt builder is designed to degrade gracefully. Every data source (playbooks, skills, code context, anti-patterns, pheromones) is optional. If all sources fail, the builder still produces a valid 9-layer prompt with role identity, conventions, task context, and tool instructions -- which is already a significant upgrade over the current 1-layer prompt.

## Testing Strategy

### Unit Tests

1. **Gate feedback parser**: Test `GateFeedback::parse()` with real cargo check, clippy, and test output.
   - Rustc error with code: `error[E0308]: mismatched types`
   - Rustc error without code: `error: cannot find value`
   - Clippy warning: `warning: needless_borrow`
   - Test failure: `test foo::bar ... FAILED`
   - Mixed output with both errors and warnings.
   - Malformed output (graceful degradation to empty errors list).

2. **Tool allowlist**: Test per-role allowlists:
   - Implementer gets full tools
   - Reviewer gets read-only tools
   - Researcher gets no-edit tools
   - Task-level allowed_tools/denied_tools override role defaults

3. **Prompt metadata**: Verify metadata counts match injected content.

4. **Structured gate feedback prepend**: Verify format of error list, truncation at 20 errors, instruction section.

### Integration Tests

1. **Full prompt assembly**: Given a `TaskDef` with all fields populated, verify the assembled prompt contains all 9 layers in correct order.

2. **Playbook injection**: Seed a `PlaybookStore` with a known playbook, verify it appears in the assembled prompt.

3. **Context window trimming**: Set a small context window (1000 tokens), verify the prompt is trimmed and `was_trimmed` is true.

4. **Backward compatibility**: Verify that the assembled prompt for a basic task (no playbooks, no skills, no code context) is equivalent in content to the current `build_minimal_system_prompt()` output.

### Snapshot Tests

For prompt stability, snapshot tests compare assembled prompts against golden files. This catches unintended prompt regressions when layer content or ordering changes.

## Open Questions

1. **Neuro anti-pattern query interface**: The neuro store (`roko-neuro::KnowledgeStore`) supports `query()`, but there's no dedicated anti-pattern retrieval method. Should we add a `query_anti_patterns(topic: &str)` method, or filter general query results by a `kind: "anti-pattern"` tag?

2. **Pheromone source**: The orchestrator builds pheromone `ContextChunk`s from concurrent agent signals. The runner is currently single-agent (sequential). Should pheromones be wired from the `StateHub` event stream, or deferred until multi-agent runner support?

3. **DaimonState integration**: Layer 8 (affect guidance) requires `PadState` from the `DaimonState`. The runner doesn't load daimon state. Should this be a `PromptAssembler` constructor parameter, or queried per-dispatch?

## Implementation Packet

This work replaces runner hand-written prompts with the real composition path.

### Required Context

- `crates/roko-cli/src/runner/agent_stream.rs`
- `crates/roko-cli/src/dispatch_helpers.rs`
- `crates/roko-compose/src/role_prompts.rs`
- `crates/roko-compose/src/system_prompt_builder.rs`
- `crates/roko-compose/src/prompt.rs`
- `crates/roko-compose/src/context_provider.rs`
- `crates/roko-compose/src/templates/mod.rs`
- `docs/03-composition/02-system-prompt-builder-7-layer.md`
- `docs/03-composition/08-5-stage-assembly-pipeline.md`
- `tmp/unified/14-TOOLS.md`

### Target Files

- [ ] Create `crates/roko-cli/src/dispatch/prompt_builder.rs`.
- [ ] Update `runner/event_loop.rs` to call `PromptAssembler`.
- [ ] Keep `build_minimal_system_prompt` only for tests or delete it.
- [ ] Add prompt assembly tests under `crates/roko-compose/tests/` or `crates/roko-cli/tests/`.

### Checklist

- [ ] Define `PromptAssemblyRequest` with task, plan id, role, files in scope, retry context, acceptance criteria, and verify commands.
- [ ] Define `AssembledPrompt` with `system_prompt`, `user_prompt`, `tool_allowlist`, token estimate, and section diagnostics.
- [ ] Convert task role string into `AgentRole` using the existing role parser.
- [ ] Call `RoleSystemPromptSpec` or the closest existing role policy entrypoint.
- [ ] Add structured gate feedback section for retries.
- [ ] Query playbook store for repeated failure or successful pattern hints.
- [ ] Query neuro store for anti-patterns and relevant durable knowledge.
- [ ] Enforce role-specific tool allowlists.
- [ ] Include code index context through a structured `Code Context` section, not raw concatenation.
- [ ] Enforce prompt token budget with deterministic section dropping.

### Verification

- [ ] Snapshot test for implementer prompt.
- [ ] Snapshot test for reviewer prompt with restricted tools.
- [ ] Snapshot test for retry prompt with structured gate feedback.
- [ ] Unit test: long context drops lower-priority sections before task requirements.
- [ ] Search gate: `rg "build_minimal_system_prompt" crates/roko-cli/src/runner` returns no production call.
