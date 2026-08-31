//! `roko plan generate` — intelligent task decomposition from any input source.
//!
//! Takes a PRD, prompt, file, or checklist and produces plan directories
//! with surgically-scoped tasks, executable verification, and model hints.
//!
//! Key principles (from Meta-Harness [Lee et al. 2026]):
//! - Right context, not more context
//! - Tasks ≤50 LOC for Tier 1, ≤20 LOC for Tier 0
//! - Every acceptance criterion is a runnable command
//! - Feedback from failures feeds into retry context

use std::fmt::Write as _;
use std::path::Path;

const NAMING_GLOSSARY_RELATIVE_PATH: &str = "docs/00-architecture/01-naming-and-glossary.md";
const NAMING_GLOSSARY_MAX_LINES: usize = 160;
const CLAUDE_MD_RELATIVE_PATH: &str = "CLAUDE.md";
const CLAUDE_MD_MAX_LINES: usize = 120;

/// Built-in plan generation template presets.
///
/// The PRD frontmatter selects one of these presets. Each preset controls the
/// generator's default model tier, gate strictness guidance, and total task
/// budget. Unknown or missing template names fall back to [`Default`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlanTemplateKind {
    /// Current behavior: balanced defaults.
    Default,
    /// Smaller, tighter plans with fewer tasks.
    Compact,
    /// More conservative plans with stricter gates.
    Strict,
}

impl PlanTemplateKind {
    /// Resolve a template name from PRD frontmatter.
    #[must_use]
    pub(crate) fn resolve(name: Option<&str>) -> Self {
        let Some(name) = name else {
            return Self::Default;
        };
        if name.eq_ignore_ascii_case("compact") || name.eq_ignore_ascii_case("small") {
            Self::Compact
        } else if name.eq_ignore_ascii_case("strict") {
            Self::Strict
        } else {
            Self::Default
        }
    }

    /// Template label used in prompts.
    #[must_use]
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Compact => "compact",
            Self::Strict => "strict",
        }
    }

    /// Default model tier for the template.
    #[must_use]
    pub(crate) const fn default_model_tier(self) -> &'static str {
        match self {
            Self::Default => "focused",
            Self::Compact => "mechanical",
            Self::Strict => "integrative",
        }
    }

    /// Verify strictness guidance for the template.
    #[must_use]
    pub(crate) const fn gate_strictness(self) -> &'static str {
        match self {
            Self::Default => "standard",
            Self::Compact => "standard",
            Self::Strict => "strict",
        }
    }

    /// Maximum total task count the generator should target.
    #[must_use]
    pub(crate) const fn max_task_count(self) -> usize {
        match self {
            Self::Default => 8,
            Self::Compact => 4,
            Self::Strict => 12,
        }
    }
}

/// Render the selected plan template as prompt guidance.
#[must_use]
pub(crate) fn render_plan_template_guidance(template: PlanTemplateKind) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "## Plan template");
    let _ = writeln!(out, "- name: {}", template.label());
    let _ = writeln!(
        out,
        "- default model tier: {}",
        template.default_model_tier()
    );
    let _ = writeln!(out, "- gate strictness: {}", template.gate_strictness());
    let _ = writeln!(out, "- max task count: {}", template.max_task_count());
    let _ = writeln!(
        out,
        "- This is a ceiling, not a target. Prefer the fewest cohesive tasks that preserve safe ownership."
    );
    out
}

/// Task tier determines minimum model and maximum scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskTier {
    /// Mechanical: imports, renames, field additions. ≤20 LOC. Haiku-capable.
    Mechanical,
    /// Focused: single function, single test. ≤50 LOC. Sonnet-capable.
    Focused,
    /// Integrative: multi-module connection. ≤150 LOC. Sonnet/Opus.
    Integrative,
    /// Architectural: API design, decomposition. ≤300 LOC. Opus only.
    Architectural,
}

impl TaskTier {
    /// Maximum lines of code change for this tier.
    #[must_use]
    pub const fn max_loc(&self) -> u32 {
        match self {
            Self::Mechanical => 20,
            Self::Focused => 50,
            Self::Integrative => 150,
            Self::Architectural => 300,
        }
    }

    /// Label for TOML output.
    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self {
            Self::Mechanical => "mechanical",
            Self::Focused => "focused",
            Self::Integrative => "integrative",
            Self::Architectural => "architectural",
        }
    }
}

/// The system prompt for the plan generator agent.
///
/// This prompt produces tasks with surgical context, executable verification,
/// and model-adaptive tier hints. It's designed to produce tasks that even
/// the smallest models can execute successfully.
pub const PLAN_GENERATOR_SYSTEM_PROMPT: &str = r#"## CRITICAL: Output format

Your entire response MUST be a single ```toml fenced code block containing ONLY valid TOML.
Do not include prose, explanations, Rust code, or markdown outside the TOML block.

MINIMUM VALID STRUCTURE (use this as your template):
```toml
[meta]
plan = "slug-matches-prd"
total = 2
done = 0
status = "ready"
max_parallel = 1

[[task]]
id = "T1"
title = "First task"
description = "What this task accomplishes."
status = "ready"
tier = "focused"
max_loc = 50
files = ["crates/roko-core/src/lib.rs"]
allowed_tools = ["read_file", "grep"]
denied_tools = []
depends_on = []
role = "implementer"

[task.context]
read_files = [
    { path = "crates/roko-core/src/lib.rs", lines = "1-50", why = "Read existing types." },
]

[[task.verify]]
phase = "compile"
command = "cargo check -p roko-core"
```

INVALID — do NOT produce output like this (Rust code inside TOML):
```toml
[[task]]
id = "T1"
title = "Add struct"
prompt = "Implement the following:\n\npub struct Foo {\n    bar: String,\n}\n"
```
The above is INVALID because it embeds Rust code inside the TOML value.

IMPORTANT: The meta section field is `plan`, NOT `name`.

---

You are a task decomposition engine for software projects. Your job is to take a feature description and produce a set of tasks that are so precisely scoped that even the smallest, cheapest LLM can execute them correctly.

## Core principles

1. **Cohesive scope**: One observable outcome that shares context, files, and verification belongs in one task. Select the correct tier (up to its LOC budget); do not split types, wiring, tests, and docs into separate serial microtasks merely to stay under 50 lines. Split only at a genuine ownership, dependency, security, or independently-verifiable boundary.
2. **Precise context**: For each task, specify EXACTLY which files and line ranges to read. Not "read the crate" — "read lines 40-80 of src/lib.rs".
3. **Single-owner executable verification**: Give each task exactly one focused command that proves its observable outcome. Combine structural assertions into that command when necessary. Do not repeat equivalent compile/test/clippy commands across tasks; the runner and release lane own broader validation.
4. **Dependency ordering**: Types before implementations. Implementations before wiring. Wiring before tests.
5. **Model hints**: NEVER set `model_hint`. The runtime selects the right model based on the task `tier`. Hardcoded model names break across providers.

## Task tiers

| Tier | Name | Max LOC | Examples |
|------|------|---------|----------|
| 0 | Mechanical | 20 | Add import, add struct field, rename function |
| 1 | Focused | 50 | Implement function body, write single test |
| 2 | Integrative | 150 | Wire module A→B, implement trait for type |
| 3 | Architectural | 300 | Design new API, decompose complex feature |

## Output format

Create plan directories with these files:

### tasks.toml
```toml
[meta]
plan = "add-funding-rate"  # MUST match the PRD slug exactly
total = 3
done = 0
status = "ready"
max_parallel = 1  # default to 1 for safety; only increase when tasks are truly independent

[[task]]
id = "T1"
title = "Add FundingRate struct to core types"
description = "Define the FundingRate data structure in roko-core for storing funding rate observations."
status = "ready"
tier = "mechanical"       # mechanical | focused | integrative | architectural
# model_hint omitted — runtime picks the best model automatically
max_loc = 20              # maximum lines of change
files = ["crates/roko-core/src/types.rs"]   # REAL file paths only, never <path> or <crate>
allowed_tools = ["read_file", "grep"]
denied_tools = []
# mcp_servers omitted — only include when a task genuinely requires an MCP server
depends_on = []
role = "implementer"      # REQUIRED: implementer | architect | researcher | strategist | quick-reviewer | scribe

# SURGICAL CONTEXT: exactly what the agent needs to read
[task.context]
read_files = [
    { path = "crates/roko-core/src/types.rs", lines = "1-50", why = "Find existing type definitions to follow naming conventions." },
]
symbols = [
    "Signal — existing base type to reference",
]
anti_patterns = [
    "Do NOT create new files. Modify crates/roko-core/src/types.rs only.",
]

[[task.verify]]
phase = "compile"
command = "cargo check -p roko-core"

[[task]]
id = "T2"
title = "Wire FundingRate display into CLI status output"
description = "Import FundingRate from roko-core and add it to the status command output."
status = "ready"
tier = "focused"
# model_hint omitted — runtime selects automatically
max_loc = 40
files = ["crates/roko-cli/src/commands/status.rs"]
allowed_tools = ["read_file", "grep", "write_file"]
denied_tools = []
# mcp_servers omitted — only include when a task genuinely requires an MCP server
depends_on = ["T1"]
role = "implementer"

[task.context]
read_files = [
    { path = "crates/roko-cli/src/commands/status.rs", lines = "1-80", why = "Understand current status output format." },
    { path = "crates/roko-core/src/types.rs", lines = "1-30", why = "Import the new FundingRate type." },
]
symbols = [
    "StatusOutput — struct that collects status display fields",
]
anti_patterns = [
    "Do NOT modify roko-core. Only change the CLI crate.",
]

[[task.verify]]
phase = "compile"
command = "cargo check -p roko-cli"
```

## Role selection

Every `[[task]]` MUST include a `role` field. Choose the most specific role:

| Role | Use when |
|------|----------|
| `"implementer"` | Writing code, adding fields, modifying functions, creating files |
| `"architect"` | Designing APIs, planning module structure, major refactors |
| `"researcher"` | Gathering information, analyzing existing code, reading docs |
| `"strategist"` | Decomposing requirements, planning approach, making design decisions |
| `"scribe"` | Writing documentation, updating comments, generating markdown |
| `"quick-reviewer"` | Code review tasks, auditing for correctness |

Missing or misspelled roles will be rejected by `roko plan validate`. The `role` field is REQUIRED.

## Role-Tool Constraints

Each role has a default tool permission set. Tasks can further restrict via `allowed_tools`/`denied_tools`.

| Role | Read | Write | Execute | Notes |
|------|------|-------|---------|-------|
| `"implementer"` | yes | yes | yes | Full access to modify and build |
| `"architect"` | yes | yes | yes | Same as implementer but for design-level tasks |
| `"researcher"` | yes | no | no | Read-only; cannot modify files or run commands |
| `"strategist"` | yes | no | no | Read-only; planning and analysis only |
| `"scribe"` | yes | yes | no | Can write docs but cannot execute commands |
| `"quick-reviewer"` | yes | no | no | Read-only; audits code without changes |

## Model hints

**NEVER set `model_hint`.** The runtime's model-selection chain (cascade router, project config, budget pressure) picks the right model automatically. Setting model_hint hardcodes a provider-specific model name that breaks when users run non-Claude providers.

Always omit the `model_hint` field entirely. The task `tier` field (mechanical/focused/integrative/architectural) already tells the runtime what capability level is needed.

## Before generating tasks, you MUST:

1. Search the codebase to understand what exists:
   `grep -rn 'TypeName' crates/ --include='*.rs' | grep -v target/ | head -20`

2. Read the specific files you're generating tasks for — understand the current code.

3. Check if the feature already exists (partially or fully):
   `grep -rn 'feature_keyword' crates/ --include='*.rs' | grep -v target/`

4. For each task, verify the context files actually exist:
   `test -f crates/roko-core/src/types.rs && echo "exists" || echo "MISSING"`

## Language detection

Detect the project language and use the right commands:
- Cargo.toml → Rust: `cargo check`, `cargo test`, `cargo clippy`
- package.json → TypeScript: `npx tsc`, `npx jest`, `npx eslint`
- go.mod → Go: `go build`, `go test`, `golangci-lint`
- pyproject.toml/setup.py → Python: `python -m py_compile`, `pytest`, `ruff`

## Verify steps by role

- **implementer/architect**: MUST have exactly one focused verify step. Use a target-aware compile for ordinary Rust edits, an exact test for behavioral logic, or one shell command that combines a structural assertion with the selected check.
- **researcher/strategist**: MUST have only structural checks (e.g. `test -f path/to/output.md`, `grep -q ...`). Do NOT add compile/test verify steps — researcher tasks do not modify code.
- **scribe/quick-reviewer**: structural checks only (verify docs exist, verify reviewed files haven't changed)

## Quality gates for YOUR output

Before finalizing, verify your tasks against:
- [ ] `meta.plan` matches the PRD slug exactly (e.g. slug "add-funding-rate" → `plan = "add-funding-rate"`)
- [ ] `meta.max_parallel` is 1 unless tasks are truly independent (shared files = not independent)
- [ ] Every task has ≤ max_loc lines of change for its tier
- [ ] Every task has exactly one focused verify step and no semantic duplicate exists elsewhere in the plan
- [ ] Researcher/strategist tasks have ONLY structural verify steps (no cargo check, no cargo test)
- [ ] No task requires reading more than 3 files
- [ ] Anti-patterns are specific (not generic "be careful")
- [ ] Dependencies form a DAG (no cycles)
- [ ] `model_hint` is NEVER set — runtime selects models from `tier`

## File Path Rules

1. Use CONCRETE file paths: `"crates/my-crate/src/lib.rs"` NOT `"crates/"` or `"crates/*/src/*.rs"`.
2. Never use bare directory references like `"crates/"` or `"src/"`.
3. Never use glob patterns like `*` in file paths.
4. Never output angle-bracket placeholders like `<path>`, `<crate>`, `<file>`, `<module>`, or `<relevant-lib>`.
5. Every `files` entry, every `path` in `read_files`, and every `cargo` command must reference actual files and crates that exist in the workspace or that the plan explicitly creates.
6. If a task creates a NEW crate, list the specific files: `"crates/new-crate/src/lib.rs"`, `"crates/new-crate/Cargo.toml"`. Use the PRD slug as the crate name (e.g., slug "btc-funding-alert" → `"crates/btc-funding-alert/src/lib.rs"`).
7. Researcher tasks that only READ files should still list specific file paths they will inspect.

## Complete Example (end-to-end)

The example below uses multiple tasks only to illustrate dependency syntax. For a normal endpoint
change where one implementer can safely own the response type, route, and exact test, emit one
integrative task instead. Cohesion and one verification owner override mechanical file-count splits.

A realistic 3-task plan for "Add health check endpoint to roko-serve":

```toml
[meta]
plan = "add-health-check"
total = 3
done = 0
status = "ready"
max_parallel = 1

[[task]]
id = "T1"
title = "Define HealthStatus response type"
description = "Add a HealthStatus struct with uptime, version, and db_connected fields to the serve types module."
status = "ready"
tier = "mechanical"
max_loc = 15
files = ["crates/roko-serve/src/types.rs"]
allowed_tools = ["read_file", "write_file", "grep"]
denied_tools = []
depends_on = []
role = "implementer"

[task.context]
read_files = [
    { path = "crates/roko-serve/src/types.rs", lines = "1-40", why = "Find existing response types to follow conventions." },
]
symbols = ["AppState — shared state struct to reference for db_connected"]
anti_patterns = ["Do NOT add new dependencies. Use only std and existing crate types."]

[[task.verify]]
phase = "compile"
command = "cargo check -p roko-serve"

[[task]]
id = "T2"
title = "Implement GET /health handler"
description = "Add an async handler that returns HealthStatus as JSON, wired to the router."
status = "ready"
tier = "focused"
max_loc = 35
files = ["crates/roko-serve/src/routes/health.rs", "crates/roko-serve/src/routes/mod.rs"]
allowed_tools = ["read_file", "write_file", "grep"]
denied_tools = []
depends_on = ["T1"]
role = "implementer"

[task.context]
read_files = [
    { path = "crates/roko-serve/src/routes/mod.rs", lines = "1-30", why = "Understand router setup to add new route." },
    { path = "crates/roko-serve/src/types.rs", lines = "1-40", why = "Import HealthStatus type." },
]
symbols = ["router() — function where routes are registered"]
anti_patterns = ["Do NOT modify types.rs. Only add the handler and route registration."]

[[task.verify]]
phase = "compile"
command = "cargo check -p roko-serve"

[[task]]
id = "T3"
title = "Add integration test for /health endpoint"
description = "Write a test that starts the server and verifies GET /health returns 200 with valid JSON."
status = "ready"
tier = "focused"
max_loc = 40
files = ["crates/roko-serve/tests/health_check.rs"]
allowed_tools = ["read_file", "write_file", "grep"]
denied_tools = []
depends_on = ["T2"]
role = "implementer"

[task.context]
read_files = [
    { path = "crates/roko-serve/tests/", lines = "1-50", why = "Follow existing test patterns." },
    { path = "crates/roko-serve/src/routes/health.rs", lines = "1-40", why = "Know what the handler returns." },
]
symbols = ["TestClient — test helper if one exists"]
anti_patterns = ["Do NOT modify production code. Only add the test file."]

[[task.verify]]
phase = "test"
command = "cargo test -p roko-serve --test health_check"
fail_msg = "Integration test failed or not found"
```
"#;

/// Build the shared system prompt for plan generation and regeneration.
#[must_use]
pub fn build_generator_system_prompt(workdir: &Path) -> String {
    let mut prompt = String::new();
    let _ = writeln!(prompt, "{PLAN_GENERATOR_SYSTEM_PROMPT}");
    append_naming_glossary_prompt(&mut prompt, workdir);
    append_claude_md_prompt(&mut prompt, workdir);
    prompt
}

/// Build the full prompt for plan generation from a source input.
#[must_use]
pub fn build_generation_prompt(workdir: &Path, source: &str, source_type: &str) -> String {
    let mut prompt = build_generator_system_prompt(workdir);
    let _ = writeln!(prompt, "\n---\n");
    let _ = writeln!(prompt, "## Workspace: {}\n", workdir.display());
    let _ = writeln!(
        prompt,
        "## Source type: {source_type}\n\n## Source content:\n\n{source}"
    );
    prompt
}

#[cfg(test)]
mod template_tests {
    use super::*;

    #[test]
    fn build_generator_system_prompt_includes_naming_glossary_excerpt_when_present() {
        let temp = tempfile::tempdir().expect("tempdir");
        let glossary_dir = temp.path().join("docs").join("00-architecture");
        std::fs::create_dir_all(&glossary_dir).expect("create glossary dir");
        std::fs::write(
            glossary_dir.join("01-naming-and-glossary.md"),
            "# Naming Map\n\nSignal -> Engram\n",
        )
        .expect("write glossary");

        let prompt = build_generator_system_prompt(temp.path());
        assert!(prompt.contains("## Naming glossary"));
        assert!(prompt.contains("Signal -> Engram"));
    }

    #[test]
    fn build_generator_system_prompt_includes_claude_rules_when_present() {
        let temp = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            temp.path().join("CLAUDE.md"),
            "# Rules\n\nNEVER reimplement what already exists.\n",
        )
        .expect("write claude");

        let prompt = build_generator_system_prompt(temp.path());

        assert!(prompt.contains("## Workspace rules"));
        assert!(prompt.contains("NEVER reimplement what already exists."));
    }

    #[test]
    fn resolves_missing_template_to_default() {
        let template = PlanTemplateKind::resolve(None);
        assert_eq!(template.label(), "default");
        assert_eq!(template.default_model_tier(), "focused");
        assert_eq!(template.gate_strictness(), "standard");
        assert_eq!(template.max_task_count(), 8);
    }

    #[test]
    fn resolves_strict_template() {
        let template = PlanTemplateKind::resolve(Some("strict"));
        assert_eq!(template.label(), "strict");
        assert_eq!(template.default_model_tier(), "integrative");
        assert_eq!(template.gate_strictness(), "strict");
        assert_eq!(template.max_task_count(), 12);
    }

    #[test]
    fn template_guidance_includes_selected_settings() {
        let guidance = render_plan_template_guidance(PlanTemplateKind::Compact);
        assert!(guidance.contains("name: compact"));
        assert!(guidance.contains("default model tier: mechanical"));
        assert!(guidance.contains("gate strictness: standard"));
        assert!(guidance.contains("max task count: 4"));
    }
}

/// Build a prompt for regenerating an existing plan in place (§11).
///
/// Strips the existing tasks to just `id`/`title`/`depends_on` and asks the
/// agent to fill in `tier`, `model_hint`, `read_files`, `verify`, `context`,
/// and `max_loc`.
#[must_use]
pub fn build_regeneration_prompt(workdir: &Path, existing_tasks_toml: &str) -> String {
    let mut prompt = build_generator_system_prompt(workdir);
    let _ = writeln!(prompt, "\n---\n");
    let _ = writeln!(prompt, "## Workspace: {}\n", workdir.display());
    let _ = writeln!(prompt, "## Task: Regenerate plan\n");
    let _ = writeln!(
        prompt,
        "The following tasks.toml exists but is missing full metadata (description, tier, \
         read_files, verify, context, max_loc, mcp_servers). Your job is to read the codebase and fill in \
         every field for each task. Keep the existing id, title, description, and depends_on. Add:\n\
         - `tier` (mechanical/focused/integrative/architectural)\n\
         - `max_loc` (estimated lines of change)\n\
         - `allowed_tools`, `denied_tools`, and `mcp_servers` (per-task tool/MCP constraints)\n\
         - `[task.context]` with read_files, symbols, anti_patterns\n\
         - exactly one focused `[[task.verify]]` command per task\n\
         Do NOT set `model_hint` — the runtime selects models automatically from the task tier.\n\n\
         ## Existing tasks.toml:\n\n```toml\n{existing_tasks_toml}\n```"
    );
    prompt
}

fn append_naming_glossary_prompt(prompt: &mut String, workdir: &Path) {
    let glossary_path = workdir.join(NAMING_GLOSSARY_RELATIVE_PATH);
    let Ok(glossary) = std::fs::read_to_string(&glossary_path) else {
        return;
    };

    let excerpt = glossary
        .lines()
        .take(NAMING_GLOSSARY_MAX_LINES)
        .collect::<Vec<_>>()
        .join("\n");
    if excerpt.trim().is_empty() {
        return;
    }

    let _ = writeln!(
        prompt,
        "\n## Naming glossary\nUse the canonical names and renames below when generating plans. This excerpt comes from `{}`.\n\n```md\n{}\n```",
        NAMING_GLOSSARY_RELATIVE_PATH, excerpt
    );
}

fn append_claude_md_prompt(prompt: &mut String, workdir: &Path) {
    let claude_path = workdir.join(CLAUDE_MD_RELATIVE_PATH);
    let Ok(claude_md) = std::fs::read_to_string(&claude_path) else {
        return;
    };

    let excerpt = claude_md
        .lines()
        .take(CLAUDE_MD_MAX_LINES)
        .collect::<Vec<_>>()
        .join("\n");
    if excerpt.trim().is_empty() {
        return;
    }

    let _ = writeln!(
        prompt,
        "\n## Workspace rules\nFollow the project-specific operating rules below from `{}` when generating plans.\n\n```md\n{}\n```",
        CLAUDE_MD_RELATIVE_PATH, excerpt
    );
}

// ── Backlog spec resolution (#227) ─────────────────────────────────────────

/// Default backlog directory relative to workspace root.
pub const DEFAULT_BACKLOG_DIR: &str = "tmp/backlog";

/// Parsed metadata from a backlog spec file.
#[derive(Debug, Clone)]
pub struct BacklogSpec {
    /// Numeric backlog ID (e.g. 206).
    pub id: u32,
    /// Original filename stem (e.g. "206-cargo-build-jobs-limit").
    pub file_stem: String,
    /// Full path to the spec file.
    pub path: std::path::PathBuf,
    /// Title extracted from the first `# <id> — <title>` heading.
    pub title: String,
    /// Priority (e.g. "P1").
    pub priority: Option<String>,
    /// Size (e.g. "XS", "S", "M").
    pub size: Option<String>,
    /// Crates mentioned in the spec.
    pub crates: Vec<String>,
    /// Files listed in the "Files to Modify" section.
    pub files_to_modify: Vec<String>,
    /// Full source text of the spec.
    pub source_text: String,
}

/// Derive a deterministic plan slug from a backlog filename stem.
///
/// Strips the leading numeric ID prefix and normalises to lowercase
/// alphanumeric + hyphens, truncated to 50 characters.
///
/// `"206-cargo-build-jobs-limit"` -> `"cargo-build-jobs-limit"`
#[must_use]
pub fn slug_from_backlog_stem(stem: &str) -> String {
    // Strip leading digits and the first hyphen.
    let without_id = stem.find('-').map(|i| &stem[i + 1..]).unwrap_or(stem);

    let slug: String = without_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();

    // Collapse consecutive hyphens and trim leading/trailing hyphens.
    let mut collapsed = String::with_capacity(slug.len());
    let mut prev_hyphen = true; // start true to skip leading hyphen
    for c in slug.chars() {
        if c == '-' {
            if !prev_hyphen {
                collapsed.push('-');
            }
            prev_hyphen = true;
        } else {
            collapsed.push(c);
            prev_hyphen = false;
        }
    }
    // Trim trailing hyphen.
    if collapsed.ends_with('-') {
        collapsed.pop();
    }

    // Truncate to 50 chars on a word boundary.
    if collapsed.len() > 50 {
        if let Some(pos) = collapsed[..50].rfind('-') {
            collapsed.truncate(pos);
        } else {
            collapsed.truncate(50);
        }
    }

    collapsed
}

/// Parse comma-separated backlog IDs from the `--from-backlog` argument.
///
/// Accepts: `"206"`, `"206,120,119"`, `" 206 , 120 "`.
pub fn parse_backlog_ids(input: &str) -> anyhow::Result<Vec<u32>> {
    let mut ids = Vec::new();
    for part in input.split(',') {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }
        let id: u32 = trimmed
            .parse()
            .map_err(|_| anyhow::anyhow!("invalid backlog ID: {trimmed:?}"))?;
        ids.push(id);
    }
    if ids.is_empty() {
        anyhow::bail!("--from-backlog requires at least one numeric ID");
    }
    Ok(ids)
}

/// Resolve a backlog spec file by numeric ID.
///
/// Scans `backlog_dir` for files matching `<id>-*.md` and returns the parsed
/// spec, or an error if no match or multiple matches are found.
pub fn resolve_backlog_spec(backlog_dir: &Path, id: u32) -> anyhow::Result<BacklogSpec> {
    let prefix = format!("{id}-");
    let entries: Vec<_> = std::fs::read_dir(backlog_dir)
        .map_err(|e| anyhow::anyhow!("read backlog dir {}: {e}", backlog_dir.display()))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            name_str.starts_with(&prefix) && name_str.ends_with(".md")
        })
        .collect();

    if entries.is_empty() {
        anyhow::bail!(
            "no backlog spec found for ID {id} in {}",
            backlog_dir.display()
        );
    }
    if entries.len() > 1 {
        let names: Vec<_> = entries
            .iter()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        anyhow::bail!(
            "multiple backlog specs found for ID {id}: {}",
            names.join(", ")
        );
    }

    let entry = &entries[0];
    let path = entry.path();
    let source_text = std::fs::read_to_string(&path)
        .map_err(|e| anyhow::anyhow!("read {}: {e}", path.display()))?;
    let file_stem = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let title = extract_backlog_title(&source_text, id);
    let priority = extract_backlog_field(&source_text, "Priority");
    let size = extract_backlog_field(&source_text, "Size");
    let crates_field = extract_backlog_field(&source_text, "Crates");
    let crates = crates_field
        .map(|c| {
            c.split(',')
                .map(|s| s.trim().trim_matches('`').to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();
    let files_to_modify = extract_files_to_modify(&source_text);

    Ok(BacklogSpec {
        id,
        file_stem,
        path,
        title,
        priority,
        size,
        crates,
        files_to_modify,
        source_text,
    })
}

/// Build an enhanced generation prompt for a backlog spec.
///
/// Adds structured metadata (priority, size, crates, files to modify) and
/// instructs the generator to write to `plans/<slug>/tasks.toml` with
/// backlog metadata preserved in `[meta]`.
pub fn build_backlog_generation_prompt(workdir: &Path, spec: &BacklogSpec, slug: &str) -> String {
    let mut prompt = build_generator_system_prompt(workdir);
    let _ = writeln!(prompt, "\n---\n");
    let _ = writeln!(prompt, "## Workspace: {}\n", workdir.display());
    let _ = writeln!(prompt, "## Source type: backlog-spec\n");

    // Inject structured metadata.
    let _ = writeln!(prompt, "## Backlog metadata");
    let _ = writeln!(prompt, "- backlog_id: {}", spec.id);
    if let Some(ref p) = spec.priority {
        let _ = writeln!(prompt, "- priority: {p}");
    }
    if let Some(ref s) = spec.size {
        let _ = writeln!(prompt, "- size: {s}");
    }
    if !spec.crates.is_empty() {
        let _ = writeln!(prompt, "- crates: {}", spec.crates.join(", "));
    }
    let _ = writeln!(prompt);

    // Instruct the generator to use the deterministic slug.
    let _ = writeln!(prompt, "## IMPORTANT generation instructions");
    let _ = writeln!(prompt, "- Set `meta.plan` to exactly: `\"{slug}\"`");
    let _ = writeln!(
        prompt,
        "- Write the plan to `plans/{slug}/tasks.toml` (NOT `.roko/plans/`)"
    );
    let _ = writeln!(
        prompt,
        "- Include these backlog metadata fields in `[meta]`:"
    );
    let _ = writeln!(prompt, "  ```toml");
    let _ = writeln!(prompt, "  backlog_id = {}", spec.id);
    if let Some(ref p) = spec.priority {
        let _ = writeln!(prompt, "  backlog_priority = \"{p}\"");
    }
    if let Some(ref s) = spec.size {
        let _ = writeln!(prompt, "  backlog_size = \"{s}\"");
    }
    let _ = writeln!(prompt, "  source_file = \"{}\"", spec.path.display());
    let _ = writeln!(prompt, "  ```");

    // Auto-generate context.read_files guidance from files to modify.
    if !spec.files_to_modify.is_empty() {
        let _ = writeln!(prompt, "\n## Files to modify (from backlog spec)");
        let _ = writeln!(
            prompt,
            "Generate `context.read_files` entries for each of these files. \
             Each task that modifies one of these files MUST include it in \
             `context.read_files` and `files`:"
        );
        for f in &spec.files_to_modify {
            let _ = writeln!(prompt, "- `{f}`");
        }
    }

    let _ = writeln!(prompt, "\n## Source content:\n\n{}", spec.source_text);
    prompt
}

/// Build the task prompt for `--from-backlog` generation.
#[must_use]
pub fn build_backlog_task_prompt(spec: &BacklogSpec, slug: &str) -> String {
    let mut prompt = format!(
        "Read the backlog spec below and generate an implementation plan. \
         Search the codebase first to understand what exists. \
         Write the plan to plans/{slug}/tasks.toml (create the directory). \
         Create plan.md and tasks.toml files with tier, context (read_files with line ranges), \
         mcp_servers (per-task MCP server names), and verify steps (executable shell commands). \
         Use the cheapest model tier for each task.\n\n"
    );

    // Add context files inline if small enough.
    if !spec.files_to_modify.is_empty() {
        prompt.push_str("Files referenced by the spec that tasks should operate on:\n");
        for f in &spec.files_to_modify {
            let _ = writeln!(prompt, "- {f}");
        }
        prompt.push('\n');
    }

    prompt.push_str(&spec.source_text);
    prompt
}

// ── Internal helpers ──────────────────────────────────────────────────────

/// Extract the title from the first heading: `# <id> — <title>`.
fn extract_backlog_title(source: &str, id: u32) -> String {
    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(heading) = trimmed.strip_prefix("# ") {
            // Try to strip the leading ID and em-dash.
            let after_id = heading
                .strip_prefix(&id.to_string())
                .and_then(|s| {
                    // Skip whitespace and em-dash / regular dash.
                    let s = s.trim_start();
                    s.strip_prefix("—")
                        .or_else(|| s.strip_prefix('-'))
                        .map(|s| s.trim_start())
                })
                .unwrap_or(heading);
            return after_id.to_string();
        }
    }
    format!("backlog-{id}")
}

/// Extract a `**Field**: value` metadata field from the spec header.
fn extract_backlog_field(source: &str, field: &str) -> Option<String> {
    let prefix = format!("**{field}**:");
    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(&prefix) {
            let value = rest.trim();
            // Strip inline qualifiers like "P1 — stability; ..."
            let value = value
                .split("—")
                .next()
                .unwrap_or(value)
                .split(';')
                .next()
                .unwrap_or(value)
                .trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

/// Extract file paths from the "Files to Modify" markdown table.
fn extract_files_to_modify(source: &str) -> Vec<String> {
    let mut files = Vec::new();
    let mut in_table = false;
    let mut past_header_separator = false;

    for line in source.lines() {
        let trimmed = line.trim();

        if trimmed.contains("Files to Modify") || trimmed.contains("Files to modify") {
            in_table = true;
            past_header_separator = false;
            continue;
        }

        if !in_table {
            continue;
        }

        // Table rows start with |.
        if !trimmed.starts_with('|') {
            // End of table.
            if past_header_separator {
                break;
            }
            continue;
        }

        // Skip the header row and separator.
        if trimmed.contains("---") {
            past_header_separator = true;
            continue;
        }
        if !past_header_separator {
            continue;
        }

        // Parse table row: | `path` | description |
        let cols: Vec<&str> = trimmed.split('|').collect();
        if cols.len() >= 2 {
            let file_col = cols[1].trim().trim_matches('`');
            if !file_col.is_empty() && !file_col.contains("File") {
                files.push(file_col.to_string());
            }
        }
    }

    files
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tier_labels() {
        assert_eq!(TaskTier::Mechanical.label(), "mechanical");
        assert_eq!(TaskTier::Focused.label(), "focused");
        assert_eq!(TaskTier::Integrative.label(), "integrative");
        assert_eq!(TaskTier::Architectural.label(), "architectural");
    }

    #[test]
    fn tier_max_loc() {
        assert_eq!(TaskTier::Mechanical.max_loc(), 20);
        assert_eq!(TaskTier::Focused.max_loc(), 50);
        assert_eq!(TaskTier::Integrative.max_loc(), 150);
        assert_eq!(TaskTier::Architectural.max_loc(), 300);
    }

    #[test]
    fn build_prompt_includes_source() {
        let prompt = build_generation_prompt(
            std::path::Path::new("/test"),
            "Add a logging system",
            "prompt",
        );
        assert!(prompt.contains("Add a logging system"));
        assert!(prompt.contains("Surgical scope"));
        assert!(prompt.contains("/test"));
    }

    #[test]
    fn build_generator_system_prompt_never_suggests_model_names() {
        let prompt = build_generator_system_prompt(std::path::Path::new("/test"));

        assert!(prompt.contains("## Model hints"));
        assert!(prompt.contains("NEVER set `model_hint`"));
        // Must NOT contain hardcoded model names that break non-Claude providers.
        assert!(!prompt.contains("claude-haiku-4-5"));
        assert!(!prompt.contains("claude-sonnet-4-6"));
        assert!(!prompt.contains("claude-opus-4-6"));
        // Tier table is still present.
        assert!(prompt.contains("| 0 | Mechanical | 20 |"));
    }

    // ── Backlog resolution tests (#227) ───────────────────────────────────

    #[test]
    fn slug_from_backlog_stem_strips_id_prefix() {
        assert_eq!(
            slug_from_backlog_stem("206-cargo-build-jobs-limit"),
            "cargo-build-jobs-limit"
        );
    }

    #[test]
    fn slug_from_backlog_stem_handles_no_prefix() {
        // "some-feature" has a hyphen, so the leading "some" is treated as
        // the numeric-ID prefix and stripped, leaving "feature".
        assert_eq!(slug_from_backlog_stem("some-feature"), "feature");
    }

    #[test]
    fn slug_from_backlog_stem_lowercases_and_normalises() {
        assert_eq!(
            slug_from_backlog_stem("42-My_Cool_Feature"),
            "my-cool-feature"
        );
    }

    #[test]
    fn slug_from_backlog_stem_truncates_to_50_chars() {
        let long = format!("99-{}", "a-".repeat(40));
        let slug = slug_from_backlog_stem(&long);
        assert!(slug.len() <= 50, "slug len {} > 50", slug.len());
    }

    #[test]
    fn parse_backlog_ids_single() {
        let ids = parse_backlog_ids("206").unwrap();
        assert_eq!(ids, vec![206]);
    }

    #[test]
    fn parse_backlog_ids_multiple() {
        let ids = parse_backlog_ids("206,120,119").unwrap();
        assert_eq!(ids, vec![206, 120, 119]);
    }

    #[test]
    fn parse_backlog_ids_with_spaces() {
        let ids = parse_backlog_ids(" 206 , 120 ").unwrap();
        assert_eq!(ids, vec![206, 120]);
    }

    #[test]
    fn parse_backlog_ids_rejects_non_numeric() {
        assert!(parse_backlog_ids("abc").is_err());
    }

    #[test]
    fn parse_backlog_ids_rejects_empty() {
        assert!(parse_backlog_ids("").is_err());
    }

    #[test]
    fn resolve_backlog_spec_finds_file() {
        let dir = tempfile::tempdir().unwrap();
        let spec_content = "# 42 \u{2014} Test Feature\n\
            \n\
            **Priority**: P2\n\
            **Size**: S\n\
            **Crates**: `roko-core`, `roko-cli`\n\
            \n\
            ## Files to Modify\n\
            \n\
            | File | Change |\n\
            |---|---|\n\
            | `crates/roko-core/src/lib.rs` | Add type |\n\
            | `crates/roko-cli/src/main.rs` | Wire it |\n";
        std::fs::write(dir.path().join("42-test-feature.md"), spec_content).unwrap();

        let spec = resolve_backlog_spec(dir.path(), 42).unwrap();
        assert_eq!(spec.id, 42);
        assert_eq!(spec.title, "Test Feature");
        assert_eq!(spec.priority.as_deref(), Some("P2"));
        assert_eq!(spec.size.as_deref(), Some("S"));
        assert_eq!(spec.crates, vec!["roko-core", "roko-cli"]);
        assert_eq!(
            spec.files_to_modify,
            vec!["crates/roko-core/src/lib.rs", "crates/roko-cli/src/main.rs"]
        );
    }

    #[test]
    fn resolve_backlog_spec_errors_on_missing() {
        let dir = tempfile::tempdir().unwrap();
        assert!(resolve_backlog_spec(dir.path(), 999).is_err());
    }

    #[test]
    fn extract_backlog_title_from_heading() {
        let source =
            "# 206 \u{2014} Limit CARGO_BUILD_JOBS in Agent Subprocess Spawns\n\nMore text.";
        assert_eq!(
            extract_backlog_title(source, 206),
            "Limit CARGO_BUILD_JOBS in Agent Subprocess Spawns"
        );
    }

    #[test]
    fn extract_backlog_title_fallback() {
        let source = "No heading here.";
        assert_eq!(extract_backlog_title(source, 99), "backlog-99");
    }

    #[test]
    fn extract_backlog_field_priority() {
        let source =
            "**Priority**: P1 \u{2014} stability; concurrent agents\n**Size**: XS (half day)";
        assert_eq!(
            extract_backlog_field(source, "Priority"),
            Some("P1".to_string())
        );
        assert_eq!(
            extract_backlog_field(source, "Size"),
            Some("XS (half day)".to_string())
        );
    }

    #[test]
    fn extract_files_to_modify_from_table() {
        let source = "## Files to Modify\n\
            \n\
            | File | Change |\n\
            |---|---|\n\
            | `crates/roko-agent/src/provider/claude_cli.rs` | Add env vars |\n\
            | `crates/roko-core/src/config/mod.rs` | Add config field |\n";
        let files = extract_files_to_modify(source);
        assert_eq!(
            files,
            vec![
                "crates/roko-agent/src/provider/claude_cli.rs",
                "crates/roko-core/src/config/mod.rs",
            ]
        );
    }

    #[test]
    fn backlog_generation_prompt_includes_metadata() {
        let spec = BacklogSpec {
            id: 206,
            file_stem: "206-cargo-build-jobs-limit".to_string(),
            path: std::path::PathBuf::from("tmp/backlog/206-cargo-build-jobs-limit.md"),
            title: "Limit CARGO_BUILD_JOBS".to_string(),
            priority: Some("P1".to_string()),
            size: Some("XS".to_string()),
            crates: vec!["roko-agent".to_string()],
            files_to_modify: vec!["crates/roko-agent/src/provider/claude_cli.rs".to_string()],
            source_text: "# Spec content here".to_string(),
        };
        let prompt = build_backlog_generation_prompt(
            std::path::Path::new("/test"),
            &spec,
            "cargo-build-jobs-limit",
        );
        assert!(prompt.contains("backlog_id = 206"));
        assert!(prompt.contains("backlog_priority = \"P1\""));
        assert!(prompt.contains("backlog_size = \"XS\""));
        assert!(prompt.contains("meta.plan"));
        assert!(prompt.contains("cargo-build-jobs-limit"));
        assert!(prompt.contains("crates/roko-agent/src/provider/claude_cli.rs"));
    }
}
