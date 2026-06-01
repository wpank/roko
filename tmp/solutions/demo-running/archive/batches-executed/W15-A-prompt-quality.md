# W15-A: Prompt Quality Improvements (IMPROVEMENTS 4.1-4.6)

**Priority**: P2 -- extensibility and plan quality improvements
**Effort**: 3-4 hours
**Files to modify**: 3 files
**Dependencies**: None

## Problem

Six prompt quality issues degrade agent effectiveness during plan generation and task execution:

1. **No workspace context** -- `dispatch_agent_with()` sends agents into tasks blind; they don't know what crates exist or what the workspace looks like.
2. **model_hint contradiction** -- `TaskTier::model_hint()` returns hardcoded model names (`claude-haiku-4-5`, `claude-sonnet-4-6`, etc.) despite the prompt explicitly saying "NEVER set model_hint." The method exists but should not.
3. **No failure recovery guidance** -- the implementer template has no instructions for what to do when `cargo check` or tests fail. Agents waste turns on wrong recovery strategies.
4. **No few-shot TOML example** -- the plan generator prompt has a long inline example but no complete real-world example. The existing inline example is tied to a fictional "add-funding-rate" plan. A separate, complete few-shot example improves structured output quality.
5. **No role-tool mapping** -- the prompt lists roles but doesn't explain tool constraints. Agents assigned as "researcher" sometimes try to write files, which fails silently.
6. **Duplicate file path rules** -- file path rules appear in two places in the generator prompt (lines 337-342 and 344-349), with slightly different wording. Consolidate to one authoritative section.

## Exact Code to Change

### File 1: `crates/roko-cli/src/orchestrate.rs` (23,181 lines)

#### Change 1: Add workspace context helper for `dispatch_agent_with()` (4.1)

The `dispatch_agent_with` method is at line 14882. Before it, add a standalone function that gathers workspace crate info for agent prompts.

**Find this code (line ~14880):**

```rust
    async fn dispatch_agent_with(
```

**Add BEFORE this line:**

```rust
/// Generate a concise workspace context string for agent prompts.
///
/// Lists crate directories (with kind: binary/library/empty) and workspace
/// members from Cargo.toml. Capped at ~2000 chars to avoid prompt bloat.
fn workspace_context(workdir: &Path) -> String {
    let mut ctx = String::from("## Current Workspace State\n\n");
    // List crate directories
    if let Ok(entries) = std::fs::read_dir(workdir.join("crates")) {
        ctx.push_str("### Crates in workspace:\n");
        let mut crate_entries: Vec<_> = entries.flatten().collect();
        crate_entries.sort_by_key(|e| e.file_name());
        for entry in &crate_entries {
            if entry.path().is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                let has_lib = entry.path().join("src/lib.rs").exists();
                let has_main = entry.path().join("src/main.rs").exists();
                let kind = if has_main {
                    "binary"
                } else if has_lib {
                    "library"
                } else {
                    "empty"
                };
                ctx.push_str(&format!("- `crates/{name}/` ({kind})\n"));
            }
            // Cap output to avoid prompt bloat
            if ctx.len() > 2000 {
                ctx.push_str("- ... (truncated)\n");
                break;
            }
        }
    }
    // List workspace Cargo.toml members
    if let Ok(content) = std::fs::read_to_string(workdir.join("Cargo.toml")) {
        if let Ok(doc) = content.parse::<toml::Value>() {
            if let Some(members) = doc
                .get("workspace")
                .and_then(|w| w.get("members"))
                .and_then(|m| m.as_array())
            {
                ctx.push_str("\n### Workspace members (from Cargo.toml):\n");
                for m in members {
                    if let Some(s) = m.as_str() {
                        ctx.push_str(&format!("- `{s}`\n"));
                    }
                }
            }
        }
    }
    ctx
}

```

Then inside `dispatch_agent_with()`, find where the task prompt is assembled (around line 15004 where `task_model_hint` is set) and inject workspace context:

**Find this code (line ~15004):**

```rust
        let task_model_hint = task_def.as_ref().and_then(|td| td.model_hint.clone());
```

**Add AFTER this line:**

```rust
        // Inject workspace context so agents see crate layout
        let ws_ctx = workspace_context(&self.workdir);
        let task = format!("{ws_ctx}\n\n{task}");
```

**Instrumentation**: Add `tracing::debug!(workspace_ctx_len = ws_ctx.len(), "injected workspace context");` after the `workspace_context()` call to track prompt size impact. (This file uses fully-qualified `tracing::debug!`, not bare `debug!`.)

---

### File 2: `crates/roko-cli/src/plan_generate.rs` (553 lines)

#### Change 2: Remove `model_hint()` method from `TaskTier` (4.2)

The prompt says "NEVER set model_hint" (line 172, 293, 335) but `TaskTier::model_hint()` exists at lines 126-134 and returns hardcoded provider-specific model names. The runtime uses `roko_core::defaults::MODEL_FAST/MODEL_FOCUSED/MODEL_DEEP` for tier-based routing. Remove the contradicting method.

**Find this code (lines 125-134):**

```rust
impl TaskTier {
    /// Suggested model for this tier.
    #[must_use]
    pub const fn model_hint(&self) -> &'static str {
        match self {
            Self::Mechanical => "claude-haiku-4-5",
            Self::Focused | Self::Integrative => "claude-sonnet-4-6",
            Self::Architectural => "claude-opus-4-6",
        }
    }

    /// Maximum lines of code change for this tier.
```

**Replace with:**

```rust
impl TaskTier {
    /// Maximum lines of code change for this tier.
```

Then update the test at lines 513-518 that asserts on `model_hint()`:

**Find this code (lines 513-518):**

```rust
    #[test]
    fn tier_model_hints() {
        assert_eq!(TaskTier::Mechanical.model_hint(), "claude-haiku-4-5");
        assert_eq!(TaskTier::Focused.model_hint(), "claude-sonnet-4-6");
        assert_eq!(TaskTier::Architectural.model_hint(), "claude-opus-4-6");
    }
```

**Replace with:**

```rust
    #[test]
    fn tier_labels() {
        // model_hint() intentionally removed -- model selection is config-driven
        // via roko_core::defaults::MODEL_FAST/MODEL_FOCUSED/MODEL_DEEP
        assert_eq!(TaskTier::Mechanical.label(), "mechanical");
        assert_eq!(TaskTier::Focused.label(), "focused");
        assert_eq!(TaskTier::Architectural.label(), "architectural");
    }
```

**Note**: The `model_hint` field on `ParsedTask` (in `task_parser.rs` line 62) and `model_hint` references in `prd.rs` validation should remain -- those handle user-provided hints from TOML, which is different from the code-level method on the enum. The method was the contradiction; the TOML field is fine.

#### Change 3: Add role-tool mapping to plan generator prompt (4.5)

The role selection table at lines 276-289 tells the generator what each role does but not what tools each role has. Agents assigned as "researcher" sometimes try to write files.

**Find this code (lines 276-289):**

```rust
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
```

**Replace with:**

```rust
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

Each role has specific tool access. The runtime enforces these -- do not assign tasks
that require tools the role doesn't have:

| Role | Can Read | Can Write | Can Execute | Notes |
|------|----------|-----------|-------------|-------|
| researcher | Yes | No | No | Gathers information only |
| strategist | Yes | No | No | Plans and analyzes only |
| implementer | Yes | Yes | Yes | Full toolkit |
| architect | Yes | Yes | No | Designs, may write specs |
| quick-reviewer | Yes | No | No | Reviews code, no changes |
| scribe | Yes | Yes | No | Documentation only |

A researcher task that says "update the file" will FAIL because researchers
cannot write files. Use an implementer for any task that modifies files.
```

#### Change 4: Consolidate file path rules (4.6)

File path rules appear twice. Lines 337-342 have a "CRITICAL RULES" block and lines 344-349 repeat similar guidance. Consolidate.

**Find this code (lines 337-349):**

```rust
CRITICAL RULES for `files` field:
- Use CONCRETE file paths: `"crates/my-crate/src/lib.rs"` NOT `"crates/"` or `"crates/*/src/*.rs"`
- Never use bare directory references like `"crates/"` or `"src/"`
- Never use glob patterns like `*` in file paths
- If a task creates a NEW crate, list the specific files: `"crates/new-crate/src/lib.rs"`, `"crates/new-crate/Cargo.toml"`
- Researcher tasks that only READ files should still list specific file paths they will inspect

Use CONCRETE file paths and crate names from the repository context below.
Never output angle-bracket placeholders like <path>, <crate>, <file>, <module>, or <relevant-lib>.
Every `files` entry, every `path` in `read_files`, and every `cargo` command must reference
actual files and crates that exist in the workspace or that the plan explicitly creates.
If the PRD describes a new crate to create, use the PRD's slug as the crate name
(e.g., for slug "btc-funding-alert", use "crates/btc-funding-alert/src/lib.rs").
```

**Replace with:**

```rust
## File Path Rules (ALL fields that reference files)

1. CONCRETE paths only: `crates/roko-foo/src/lib.rs` -- never `crates/*/src/*.rs` or `crates/`
2. Paths are relative to workspace root (no leading `/`)
3. For new crates: `crates/{slug}/src/lib.rs` (library) or `crates/{slug}/src/main.rs` (binary)
4. `files` = the COMPLETE list of files this task will CREATE or MODIFY
5. `context.read_files` = files the agent should READ for context (may overlap with `files`)
6. Do NOT include directory paths -- always specify the exact file
7. If a task creates a new crate, include BOTH `Cargo.toml` and the source file in `files`
8. Never output angle-bracket placeholders like <path>, <crate>, <file>, <module>, or <relevant-lib>
9. Every `files` entry, every `path` in `read_files`, and every `cargo` command must reference
   actual files and crates that exist in the workspace or that the plan explicitly creates
10. Researcher tasks that only READ files should still list specific file paths they will inspect
11. Use the PRD's slug as the crate name (e.g., slug "btc-funding-alert" -> `crates/btc-funding-alert/src/lib.rs`)
```

#### Change 5: Add few-shot TOML example to plan generator prompt (4.4)

The prompt has a long inline example within the `## Output format` section (lines 183-273) using a fictional "add-funding-rate" plan. Add a separate, complete few-shot example focused on a realistic small plan. Insert before the closing `"#;` at line 350.

**Find this code (line 349-350):**

```rust
(e.g., for slug "btc-funding-alert", use "crates/btc-funding-alert/src/lib.rs").
"#;
```

**Replace with** (after the file path rules consolidation above, the text before `"#;` will end differently; adjust accordingly -- the key addition is the complete example block):

```rust
11. Use the PRD's slug as the crate name (e.g., slug "btc-funding-alert" -> `crates/btc-funding-alert/src/lib.rs`)

## Complete Example (end-to-end)

For a PRD titled "Add health check endpoint to roko-serve":

```toml
[meta]
plan = "health-check-endpoint"
total = 3
status = "pending"
max_parallel = 1

[[task]]
id = "T1"
title = "Research existing health patterns"
description = "Read the current serve routes to understand the pattern for adding new endpoints."
role = "researcher"
tier = "mechanical"
status = "ready"
files = []
depends_on = []
[task.context]
read_files = ["crates/roko-serve/src/routes/mod.rs", "crates/roko-serve/src/routes/status/mod.rs"]

[[task]]
id = "T2"
title = "Implement /health endpoint"
description = "Add a GET /health endpoint that returns 200 OK with JSON body containing version and uptime."
role = "implementer"
tier = "focused"
status = "ready"
timeout_secs = 600
max_retries = 2
max_loc = 50
files = ["crates/roko-serve/src/routes/status/health.rs", "crates/roko-serve/src/routes/status/mod.rs"]
depends_on = ["T1"]
[task.context]
read_files = ["crates/roko-serve/src/routes/mod.rs"]
anti_patterns = ["Do NOT modify existing routes", "Do NOT add external dependencies"]
[[task.verify]]
command = "cargo check -p roko-serve"
description = "Code compiles"
[[task.verify]]
command = "cargo test -p roko-serve -- health"
description = "Health endpoint tests pass"

[[task]]
id = "T3"
title = "Add integration test"
description = "Write a test that starts the server and hits GET /health."
role = "implementer"
tier = "focused"
status = "ready"
timeout_secs = 600
max_loc = 40
files = ["crates/roko-serve/tests/health_test.rs"]
depends_on = ["T2"]
[task.context]
read_files = ["crates/roko-serve/src/routes/status/health.rs"]
[[task.verify]]
command = "cargo test -p roko-serve -- health_test"
description = "Integration test passes"
```
"#;
```

---

### File 3: `crates/roko-compose/src/templates/implementer.rs` (359 lines)

#### Change 6: Add failure recovery guidance to implementer template (4.3)

The `IMPLEMENTER_ROLE_IDENTITY` static ends at line 70 with rule 12. Add failure recovery guidance after rule 12.

**Find this code (lines 69-70):**

```rust
11. Self-validate before signaling done: cargo check, cargo test on affected crates.\n\
12. Operate autonomously. Do not ask questions. Complete all work and end your turn.";
```

**Replace with:**

```rust
11. Self-validate before signaling done: cargo check, cargo test on affected crates.\n\
12. Operate autonomously. Do not ask questions. Complete all work and end your turn.\n\
\n\
## When Things Go Wrong\n\
\n\
If `cargo check` fails:\n\
1. Read the FIRST compiler error only -- fix it, then recheck. Later errors often cascade from the first.\n\
2. Common causes: missing `use` import, wrong type, missing trait implementation.\n\
3. Do NOT add `#[allow(...)]` to suppress real warnings. Only suppress unused-import warnings \
for imports you intentionally changed.\n\
\n\
If tests fail after your change:\n\
1. Run the specific failing test: `cargo test -p <crate> -- <test_name> --nocapture`\n\
2. If the test was asserting OLD behavior that your change intentionally updated, update the test's expected values.\n\
3. If the test failure reveals a bug in YOUR change, fix your code -- not the test.\n\
4. Never delete or skip a test to make your change pass.\n\
\n\
If your change breaks a file NOT in your task's `files` list:\n\
1. If the fix is a simple import or type annotation change (1-3 lines), fix it.\n\
2. If the fix requires significant changes, note it in your output as a dependency issue and continue with your assigned files.";
```

**Instrumentation**: The `role_identity_is_substantial` test (line 352) asserts the identity is 500-2000 chars. The new content adds ~900 chars, so update the upper bound:

**Find this code (line 356):**

```rust
        assert!(id.len() <= 2000);
```

**Replace with:**

```rust
        assert!(id.len() <= 3000);
```

## Agent Prompt

This change is fully self-contained. The agent should:

1. Open `crates/roko-cli/src/plan_generate.rs`
2. Delete the `model_hint()` method (lines 126-134) from the `TaskTier` impl
3. Update the test at lines 513-518 to use `label()` instead
4. In `PLAN_GENERATOR_SYSTEM_PROMPT`, add the role-tool constraints table after line 289
5. Consolidate the file path rules at lines 337-349 into a single numbered list
6. Add the complete TOML example before the closing `"#;` at the end of the prompt
7. Open `crates/roko-compose/src/templates/implementer.rs`
8. Append failure recovery guidance to `IMPLEMENTER_ROLE_IDENTITY` after rule 12
9. Update the `role_identity_is_substantial` test's upper bound to 3000
10. Open `crates/roko-cli/src/orchestrate.rs`
11. Add `workspace_context()` function before `dispatch_agent_with()` at line 14882
12. Wire it into the task prompt assembly around line 15004

**Imports needed in `orchestrate.rs`**: `std::fs` is NOT imported at module scope (only in a `#[cfg(test)]` block). `toml::Value` is NOT imported either, though `toml` is a crate dependency. `Path` IS imported at line 14 (`use std::path::{Path, PathBuf};`). The `workspace_context()` function uses `std::fs::read_dir` and `std::fs::read_to_string` (which can be fully qualified as `std::fs::read_dir(...)` and `std::fs::read_to_string(...)` without a `use` statement) and `toml::Value` (which can be fully qualified as `content.parse::<toml::Value>()`). Since the function already uses fully-qualified paths in the provided code, no new imports are strictly needed. However, for the `debug!()` instrumentation log, use `tracing::debug!()` (fully qualified) -- this file does NOT import `debug!` directly. The `plan_generate.rs` changes are purely string edits within the const prompt.

## Verification

```bash
# 1. Check model_hint method is removed
grep -n 'fn model_hint' crates/roko-cli/src/plan_generate.rs
# Should return 0 matches

# 2. Verify PLAN_GENERATOR_SYSTEM_PROMPT has the new sections
grep -c 'Role-Tool Constraints' crates/roko-cli/src/plan_generate.rs
# Should return 1

# 3. Verify few-shot example is included
grep -c 'Complete Example' crates/roko-cli/src/plan_generate.rs
# Should return 1

# 4. Verify implementer has failure recovery
grep -c 'When Things Go Wrong' crates/roko-compose/src/templates/implementer.rs
# Should return 1

# 5. Verify consolidated file path rules
grep -c 'File Path Rules' crates/roko-cli/src/plan_generate.rs
# Should return 1 (consolidated)

# 6. Build
cargo check -p roko-cli -p roko-compose

# 7. Run tests
cargo test -p roko-cli -p roko-compose
```

## Why This Matters

- Workspace context prevents agents from guessing crate names and paths
- Removing `model_hint()` eliminates the method that contradicts the "never set model_hint" rule
- Failure recovery guidance reduces wasted agent turns on known failure patterns
- Few-shot examples dramatically improve LLM output quality for structured formats
- Role-tool mapping prevents plan generators from assigning impossible tasks
- Consolidated file path rules eliminate contradictory wording

## Audit Status

Audited: 2026-05-05. PASS no changes needed
