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

/// Task tier: minimum model and maximum scope. The one tier enum, shared with
/// routing, budgets and turn caps.
pub use roko_core::task::TaskTier;

use crate::plan_policy::{DEFAULT_GENERATED_TASK_LIMIT, PlanExecutionPolicy};
use crate::task_parser::role_capabilities;

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
    let max_tasks = crate::plan_policy::effective_generated_task_limit(template.max_task_count());
    let _ = writeln!(out, "- max task count: {max_tasks}");
    let _ = writeln!(
        out,
        "- This is a ceiling, not a target. Prefer the fewest cohesive tasks that preserve safe ownership."
    );
    out
}

/// The system prompt for the plan generator agent.
///
/// This prompt produces tasks with surgical context, executable verification,
/// and model-adaptive tier hints. It's designed to produce tasks that even
/// the smallest models can execute successfully.
///
/// `{ROLE_TOOL_TABLE}` is replaced by [`render_role_tool_table`], and
/// `{TIER_SIZE_LIMITS}` by [`render_tier_size_limits`].
const PLAN_GENERATOR_SYSTEM_PROMPT: &str = r#"## CRITICAL: Output format

Your response MUST be a single ```toml fenced code block containing ONLY valid TOML, followed
only by one ```accept:accept/<file> block for each acceptance test the plan pins (see
"Planner-written acceptance tests" below).
Do not include prose, explanations, Rust code, or markdown outside those blocks.

MINIMUM VALID STRUCTURE (use this as your template):
```toml
[meta]
plan = "slug-matches-prd"
total = 2
done = 0
status = "ready"

[[task]]
id = "T1"
title = "First task"
description = "What this task accomplishes."
status = "ready"
tier = "focused"
max_loc = 50
files = ["crates/roko-core/src/lib.rs"]
allowed_tools = ["read_file", "grep", "edit_file"]
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
5. **Model hints**: NEVER set `model_hint`. The task's `tier` and `role` pick its model on the runtime's routing ladder; set `rung` only when a task needs more than its tier's start rung. Hardcoded model names break across providers.
6. **Executable spec (TSS v1)**: Every task states its `goal`, one observable outcome in a sentence. Its `acceptance` criteria are `AC1: …`, `AC2: …` items, each an input and its output, a state, an exit code or a message that a check can observe. Every test-class verify step names the criteria it proves in `covers = ["AC1"]` and says `expect = "fail_on_base"` (it fails on the unchanged code and passes once the task is done), or `expect = "pass_on_base"` for a regression check that must pass before and after. Every `read_files` entry has a `why`. `non_goals` lists what the task must not do or change. A compile check passes on the unchanged code, and a test filter that matches no test passes too, so a `fail_on_base` step checks what the task adds: a `grep -q` for the new item before the compile, or the test's pass count (`cargo test -p x new_test 2>&1 | grep -q 'ok. 1 passed'`).
7. **Say what you don't know**: When the source leaves a choice open that changes the outcome (a limit, a format, which caller wins), write it in the task's `open_questions` instead of guessing; such a plan will not run until it is answered. Leave `open_questions` empty when nothing is open.

## Task tiers

| Tier | Name | Max LOC | Examples |
|------|------|---------|----------|
| 0 | Mechanical | 20 | Add import, add struct field, rename function |
| 1 | Focused | 50 | Implement function body, write single test |
| 2 | Integrative | 150 | Wire module A→B, implement trait for type |
| 3 | Architectural | 300 | Design new API, decompose complex feature |

{TIER_SIZE_LIMITS}

## Output format

Create plan directories with these files:

### tasks.toml
```toml
[meta]
plan = "add-funding-rate"  # MUST match the PRD slug exactly
total = 3
done = 0
status = "ready"
# max_parallel is omitted: tasks that do not depend on each other run together

[[task]]
id = "T1"
title = "Add FundingRate struct to core types"
goal = "roko-core exports a FundingRate type that other crates can construct and read."
description = "Define the FundingRate data structure in roko-core for storing funding rate observations."
acceptance = [
    "AC1: types.rs defines `pub struct FundingRate`, and `cargo check -p roko-core` passes.",
]
non_goals = ["Do not change any existing type in types.rs."]
open_questions = []
status = "ready"
tier = "mechanical"       # mechanical | focused | integrative | architectural
# model_hint omitted — runtime picks the best model automatically
max_loc = 20              # maximum lines of change
files = ["crates/roko-core/src/types.rs"]   # REAL file paths only, never <path> or <crate>
allowed_tools = ["read_file", "grep", "edit_file"]
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
command = "grep -q 'pub struct FundingRate' crates/roko-core/src/types.rs && cargo check -p roko-core"
covers = ["AC1"]
expect = "fail_on_base"

[[task]]
id = "T2"
title = "Wire FundingRate display into CLI status output"
goal = "`roko status` prints the latest funding rate."
description = "Import FundingRate from roko-core and add it to the status command output."
acceptance = [
    "AC1: `cargo test -p roko-cli status_shows_funding_rate` passes: the status output has a `funding rate:` line.",
]
non_goals = ["Do not modify roko-core."]
open_questions = []
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
phase = "test"
command = "cargo test -p roko-cli status_shows_funding_rate 2>&1 | grep -q 'ok. 1 passed'"
covers = ["AC1"]
expect = "fail_on_base"
```

## Role selection

Every `[[task]]` MUST include a `role` field. Choose the most specific role:

| Role | Use when |
|------|----------|
| `"implementer"` | Writing code, adding fields, modifying functions, creating files |
| `"architect"` | Reviewing and designing APIs and module structure (read-only: cannot change files) |
| `"researcher"` | Gathering information, analyzing existing code, reading docs |
| `"strategist"` | Decomposing requirements, planning approach, making design decisions |
| `"scribe"` | Writing documentation, updating comments, generating markdown |
| `"quick-reviewer"` | Code review tasks, auditing for correctness |

Missing or misspelled roles will be rejected by `roko plan validate`. The `role` field is REQUIRED.

## Role-Tool Constraints

Each role has a default tool permission set. Tasks can further restrict via `allowed_tools`/`denied_tools`, never widen it.

{ROLE_TOOL_TABLE}

## Model hints

**NEVER set `model_hint`.** Setting model_hint hardcodes a provider-specific model name that breaks when users run non-Claude providers.

Always omit the `model_hint` field entirely. Set `tier` (mechanical/focused/integrative/architectural) and `role`: together they pick the task's start rung on the runtime's routing ladder, a list of models from cheapest to strongest.

Set `rung` only when a task needs more than its tier's start rung, for example a small change that is hard to get right: `rung = "strong"`. The default ladder's rungs, cheapest first, are `cheap`, `mid`, `strong` and `top`. A rung names a capability level, not a model, so the plan stays portable.

## Before generating tasks, you MUST:

1. Use the bounded repository map, exact matches, and source excerpts supplied in this prompt.
   Do not scan session history, other worktrees, unreachable Git objects, home directories, or
   the web. If one fact is still missing, run at most one repository-rooted exact-symbol query
   capped at 20 results; otherwise report that the source needs more context.

2. Read only the specific files needed for the cohesive outcome and record exact line ranges.

3. Check the supplied exact matches for an existing or partial implementation before planning.

4. For each task, verify the context files actually exist:
   `test -f crates/roko-core/src/types.rs && echo "exists" || echo "MISSING"`

## Language detection

Detect the project language and use the right commands:
- Cargo.toml → Rust: `cargo check`, `cargo test`, `cargo clippy`
- package.json → TypeScript: `npx tsc`, `npx jest`, `npx eslint`
- go.mod → Go: `go build`, `go test`, `golangci-lint`
- pyproject.toml/setup.py → Python: `python -m py_compile`, `pytest`, `ruff`

## Verify steps by role

- **implementer**: MUST have exactly one focused verify step. Use a target-aware compile for ordinary Rust edits, an exact test for behavioral logic, or one shell command that combines a structural assertion with the selected check.
- **Weak verify steps** (any role): a verify step must fail on the unchanged code and pass once the task is done. Prefer the crate's own gate (e.g. `cargo check -p roko-core`, `cargo test -p roko-core parse_config`) to a check that reads only the edited file, which lets a regression elsewhere in the crate through. Never negate a grep (`! grep -q ...`): correct code that mentions the text fails it (`roko plan validate` warns, PLAN_042). Never use a command that cannot fail (`echo ok`, `true`).
- **architect/researcher/strategist**: MUST have only structural checks on files that already exist (e.g. `grep -q ...`). These roles cannot write, so never verify an output file they would have to create, and do NOT add compile/test verify steps.
- **scribe/quick-reviewer**: structural checks only (verify docs exist, verify reviewed files haven't changed)

## Planner-written acceptance tests ([task.accept])

When a task's outcome can be checked by a test you can write now, write the test yourself and pin it in `[task.accept]`: the run copies your test into place before each check, so the implementer can neither edit nor weaken it.

1. After the tasks.toml block, emit the test as its own fenced block whose info string is `accept:` followed by its path under `accept/` in the plan directory, for example ```accept:accept/test_slug.py on the opening line. At most 8 such blocks, each under 64 KB.
2. Declare it in the task with the four keys:
   - `src`: the block's path, relative to the plan directory (`accept/...`).
   - `dest`: where the run copies the pinned test, relative to the repository root.
   - `runner`: the command that runs it from the repository root; `{dest}` expands to the copied test's path and `{count}` to `count`.
   - `count`: exactly how many tests the file holds; the check fails unless the runner reports exactly that many passing.

Rules:
- The test must fail on the unchanged code: it calls or imports what the task adds.
- It states outcomes (inputs and the outputs, files, exit codes or messages they produce), not how the code gets there.
- `count` is the number of tests in the file.
- The task's own `files` never include `dest`: the run writes it, the implementer does not.
- Keep the task's own `[[task.verify]]` step as well; the pinned test runs before it.

Rust example, an integration test the run copies into the crate's `tests/`:

```toml
[task.accept]
files = [
    { src = "accept/slug_accept.rs", dest = "crates/roko-core/tests/slug_accept.rs", runner = "cargo test -p roko-core --test slug_accept", count = 3 },
]
```

Python example:

```toml
[task.accept]
files = [
    { src = "accept/test_slug.py", dest = "tests/test_slug.py", runner = "python3 -m unittest tests.test_slug", count = 2 },
]
```

## Quality gates for YOUR output

Before finalizing, verify your tasks against:
- [ ] `meta.plan` matches the PRD slug exactly (e.g. slug "add-funding-rate" → `plan = "add-funding-rate"`)
- [ ] `meta.max_parallel` is omitted, and two tasks that share a file depend on each other, directly or through other tasks
- [ ] Every task has ≤ max_loc lines of change for its tier
- [ ] Every task has exactly one focused verify step and no semantic duplicate exists elsewhere in the plan
- [ ] No verify step negates a grep, cannot fail, or checks only the edited file where the crate's gate would catch a regression
- [ ] Architect/researcher/strategist tasks have ONLY structural verify steps (no cargo check, no cargo test)
- [ ] Every task with a non-empty `files` list uses a role that can write (see Role-Tool Constraints)
- [ ] No task requires reading more than 3 files
- [ ] Anti-patterns are specific (not generic "be careful")
- [ ] Dependencies form a DAG (no cycles)
- [ ] `model_hint` is NEVER set, and `rung` is set only where a task needs more than its tier's start rung
- [ ] Every task has a `goal` naming one observable outcome
- [ ] `acceptance` items are `AC1: …`, `AC2: …`, each an input and output, a state, an exit code or a message
- [ ] Every test-class verify step has `covers` naming its criteria and `expect = "fail_on_base"` (`"pass_on_base"` only for a regression check)
- [ ] Every `read_files` entry has a `why`, and `non_goals` says what the task must leave alone
- [ ] Every choice the source leaves open is an `open_questions` entry, not a guess
- [ ] Each test you could write now is pinned: an ```accept:accept/<file> block plus a `[task.accept]` entry whose `count` matches the file and whose `dest` is not in the task's `files`

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

A realistic cohesive plan for "Add health check endpoint to roko-serve":

```toml
[meta]
plan = "add-health-check"
total = 1
done = 0
status = "ready"

[[task]]
id = "T1"
title = "Implement and prove GET /health"
goal = "GET /health answers 200 with a JSON body that says the server is up."
description = "Add the response type and handler, register GET /health, and add one exact API integration test as one observable endpoint outcome."
acceptance = [
    "AC1: GET /health returns HTTP 200 with the JSON body {\"status\": \"ok\"}.",
    "AC2: GET /health needs no token: a request without one also gets 200.",
]
non_goals = [
    "Do not add readiness or dependency checks; /health reports only that the server answers.",
]
open_questions = []
status = "ready"
tier = "integrative"
max_loc = 150
files = [
    "crates/roko-serve/src/routes/health.rs",
    "crates/roko-serve/src/routes/mod.rs",
    "crates/roko-serve/tests/api_integration.rs",
]
allowed_tools = ["read_file", "write_file", "grep"]
denied_tools = []
depends_on = []
role = "implementer"

[task.context]
read_files = [
    { path = "crates/roko-serve/src/routes/providers.rs", lines = "1-80", why = "Follow the existing JSON response and handler conventions." },
    { path = "crates/roko-serve/src/routes/mod.rs", lines = "270-340", why = "Exact build_router registration seam." },
    { path = "crates/roko-serve/tests/api_integration.rs", lines = "60-180", why = "Reuse the bounded test_app and GET helper pattern." },
]
symbols = [
    "build_router — existing route registration function",
    "test_app — existing integration test fixture",
]
anti_patterns = ["Do NOT add new dependencies. Use only std and existing crate types."]

[[task.verify]]
phase = "test"
command = "cargo test -p roko-serve --test api_integration health_endpoint 2>&1 | grep -q 'ok. 1 passed'"
covers = ["AC1", "AC2"]
expect = "fail_on_base"
fail_msg = "The exact health endpoint integration test failed or was not found"
```
"#;

/// Roles offered to plan-generating models, in the order they are described.
const GENERATOR_ROLES: &[&str] = &[
    "implementer",
    "architect",
    "researcher",
    "strategist",
    "scribe",
    "quick-reviewer",
];

/// Render the role/tool table shown to plan-generating models.
///
/// Every cell comes from [`role_capabilities`] (the roles' safety contracts
/// plus their default task denials), so the prompt cannot promise a role a
/// capability that dispatch denies.
#[must_use]
pub fn render_role_tool_table() -> String {
    let yes_no = |allowed: bool| if allowed { "yes" } else { "no" };
    let mut table = String::from(
        "| Role | Read | Write | Execute | Notes |\n|------|------|-------|---------|-------|\n",
    );
    for role in GENERATOR_ROLES {
        let caps = role_capabilities(role);
        let notes = match (caps.read, caps.write, caps.execute) {
            (false, _, _) => "No tool access",
            (true, true, true) => "Full access to modify files and run commands",
            (true, true, false) => "Can write files but cannot run commands",
            (true, false, true) => "Read-only: may run commands but cannot change files",
            (true, false, false) => "Read-only: cannot change files or run commands",
        };
        let _ = writeln!(
            table,
            "| `\"{role}\"` | {} | {} | {} | {notes} |",
            yes_no(caps.read),
            yes_no(caps.write),
            yes_no(caps.execute)
        );
    }
    let writers: Vec<String> = GENERATOR_ROLES
        .iter()
        .filter(|role| role_capabilities(role).write)
        .map(|role| format!("`\"{role}\"`"))
        .collect();
    let _ = write!(
        table,
        "\nOnly {} can write files. A task with a non-empty `files` list MUST use one of \
         them; `roko plan validate` rejects any other role with `files` (PLAN_036).",
        writers.join(" and ")
    );
    table
}

/// One prompt line with each tier's size limits for generated plans: the
/// limits `roko plan validate` checks (`PLAN_TIER_SIZE`), capped by the
/// generated-plan lane.
#[must_use]
pub fn render_tier_size_limits() -> String {
    let policy = PlanExecutionPolicy::generated(DEFAULT_GENERATED_TASK_LIMIT);
    let limits = TaskTier::ALL
        .map(|tier| {
            let limits = policy.tier_size_limits(tier);
            format!(
                "{tier} at most {} files, max_loc {} and {} description words",
                limits.max_files, limits.max_loc, limits.max_description_words
            )
        })
        .join("; ");
    format!(
        "Size each task for its tier ({limits}). `roko plan validate` warns about a larger \
         task (PLAN_TIER_SIZE): split it, or give it a higher tier."
    )
}

/// Build the shared system prompt for plan generation and regeneration.
#[must_use]
pub fn build_generator_system_prompt(workdir: &Path) -> String {
    let mut prompt = String::new();
    let _ = writeln!(
        prompt,
        "{}",
        PLAN_GENERATOR_SYSTEM_PROMPT
            .replace("{ROLE_TOOL_TABLE}", &render_role_tool_table())
            .replace("{TIER_SIZE_LIMITS}", &render_tier_size_limits())
    );
    append_naming_glossary_prompt(&mut prompt, workdir);
    append_claude_md_prompt(&mut prompt, workdir);
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

    /// 3219: the generator prompt asks for every TSS v1 field and for open
    /// questions, with a checklist line for them, and its end-to-end example
    /// parses with all of them and has no spec hard fail in this repository.
    #[test]
    fn generator_prompt_carries_the_tss_checklist() {
        let prompt = PLAN_GENERATOR_SYSTEM_PROMPT;
        for field in [
            "`goal`",
            "`acceptance`",
            "`AC1: …`",
            "`covers = [\"AC1\"]`",
            "`expect = \"fail_on_base\"`",
            "`expect = \"pass_on_base\"`",
            "has a `why`",
            "`non_goals`",
            "`open_questions`",
            "such a plan will not run until it is answered",
            "- [ ] Every choice the source leaves open is an `open_questions` entry",
        ] {
            assert!(prompt.contains(field), "the prompt names {field}");
        }

        let example = prompt
            .rsplit("```toml\n")
            .next()
            .and_then(|tail| tail.split("```").next())
            .expect("the end-to-end example");
        let parsed = crate::task_parser::TasksFile::parse_str(example).expect("parse the example");
        let task = &parsed.tasks[0];
        assert!(
            task.spec
                .goal
                .as_deref()
                .is_some_and(|goal| !goal.is_empty())
        );
        assert!(task.acceptance.iter().all(|item| item.starts_with("AC")));
        assert!(!task.spec.non_goals.is_empty());
        assert!(task.spec.open_questions.is_empty());
        assert_eq!(task.verify[0].covers, ["AC1", "AC2"]);
        assert!(task.verify[0].expect.is_some());

        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("tasks.toml");
        std::fs::write(&path, example).expect("write the example");
        let report = roko_gate::spec_quality::lint_files(&[path], &root);
        assert_eq!(report.tasks.len(), 1, "{report:?}");
        assert!(
            report.tasks[0].hard_fail.is_empty(),
            "{:?}",
            report.tasks[0]
        );
    }

    /// 3222: the generator prompt teaches `[task.accept]`: the section, the
    /// `accept:` block syntax and the four keys, with a Rust and a Python
    /// example, and the Python example's entry passes `accept_issues`
    /// against an example test file.
    #[test]
    fn generator_prompt_teaches_task_accept() {
        let prompt = PLAN_GENERATOR_SYSTEM_PROMPT;
        for needle in [
            "## Planner-written acceptance tests ([task.accept])",
            "```accept:accept/test_slug.py",
            "`src`",
            "`dest`",
            "`runner`",
            "`count`",
            "`{dest}`",
            "`{count}`",
            "The test must fail on the unchanged code",
            "never include `dest`",
            "cargo test -p roko-core --test slug_accept",
            "python3 -m unittest tests.test_slug",
        ] {
            assert!(prompt.contains(needle), "the prompt names {needle}");
        }

        let start = prompt.find("Python example").expect("the Python example");
        let block = prompt[start..]
            .split("```toml\n")
            .nth(1)
            .and_then(|tail| tail.split("```").next())
            .expect("its TOML");
        let plan = format!(
            "[meta]\nplan = \"p\"\n\n[[task]]\nid = \"T1\"\ntitle = \"Slugs\"\n\
             role = \"implementer\"\nfiles = [\"src/slug.py\"]\n\
             verify = [{{ phase = \"test\", command = \"test -f src/slug.py\" }}]\n\n{block}"
        );
        let parsed = crate::task_parser::TasksFile::parse_str(&plan).expect("parse the example");
        let entry = &parsed.tasks[0]
            .accept
            .as_ref()
            .expect("[task.accept]")
            .files[0];
        assert!(!parsed.tasks[0].files.contains(&entry.dest));
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join("accept")).expect("accept dir");
        std::fs::write(dir.path().join(&entry.src), "import unittest\n").expect("the test");
        let issues = crate::task_accept::accept_issues(&parsed.tasks[0], dir.path());
        assert!(issues.is_empty(), "{issues:?}");
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
/// agent to fill in `tier`, `read_files`, `verify`, `context`,
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
         Do NOT set `model_hint`: the task's tier and role pick its model. Set `rung` only when a \
         task needs more than its tier's start rung.\n\n\
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
    fn tier_labels_and_loc_budgets_match_the_generator_prompt() {
        for tier in TaskTier::ALL {
            assert!(
                PLAN_GENERATOR_SYSTEM_PROMPT.contains(tier.label()),
                "{tier}"
            );
        }
        assert_eq!(TaskTier::Mechanical.max_loc(), 20);
        assert_eq!(TaskTier::Focused.max_loc(), 50);
        assert_eq!(TaskTier::Integrative.max_loc(), 150);
        assert_eq!(TaskTier::Architectural.max_loc(), 300);
    }

    #[test]
    fn build_generator_system_prompt_never_suggests_model_names() {
        let prompt = build_generator_system_prompt(std::path::Path::new("/test"));

        assert!(prompt.contains("## Model hints"));
        assert!(prompt.contains("NEVER set `model_hint`"));
        // gap-dbf2a6: a task that needs a stronger start names a ladder rung.
        assert!(
            prompt.contains("Set `rung` only when a task needs more than its tier's start rung")
        );
        // Must NOT contain hardcoded model names that break non-Claude providers.
        assert!(!prompt.contains("claude-haiku-4-5"));
        assert!(!prompt.contains("claude-sonnet-4-6"));
        assert!(!prompt.contains("claude-opus-4-6"));
        // Tier table is still present.
        assert!(prompt.contains("| 0 | Mechanical | 20 |"));
    }

    #[test]
    fn role_tool_table_is_derived_from_enforced_capabilities() {
        let yes_no = |allowed: bool| if allowed { "yes" } else { "no" };
        let table = render_role_tool_table();
        for role in GENERATOR_ROLES {
            let caps = role_capabilities(role);
            let row = format!(
                "| `\"{role}\"` | {} | {} | {} |",
                yes_no(caps.read),
                yes_no(caps.write),
                yes_no(caps.execute)
            );
            assert!(table.contains(&row), "missing {row} in\n{table}");
        }
        // The dogfood trap: architect was advertised as able to write.
        assert!(
            table.contains("| `\"architect\"` | yes | no | no |"),
            "{table}"
        );
        assert!(table.contains("Only `\"implementer\"` and `\"scribe\"` can write files"));

        let prompt = build_generator_system_prompt(std::path::Path::new("/test"));
        assert!(prompt.contains(&table));
        assert!(!prompt.contains("{ROLE_TOOL_TABLE}"));
        assert!(!prompt.contains("Same as implementer"));
    }

    /// gap-1d1fa6: the generator is told the size limits `plan validate`
    /// checks, capped by the generated-plan lane.
    #[test]
    fn generator_prompt_states_the_tier_size_limits() {
        let line = render_tier_size_limits();
        assert!(
            line.contains("mechanical at most 3 files, max_loc 20 and 300 description words"),
            "{line}"
        );
        assert!(
            line.contains("architectural at most 8 files, max_loc 300"),
            "{line}"
        );
        let prompt = build_generator_system_prompt(std::path::Path::new("/test"));
        assert!(prompt.contains(&line));
        assert!(!prompt.contains("{TIER_SIZE_LIMITS}"));
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
}
