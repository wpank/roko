# W9-A: Wire Workspace Map, Tasks TOML, and PRD Excerpt into Runtime Dispatch

**Priority**: P0 -- THE systemic root cause: implementer agents never see workspace state, task DAG, or PRD requirements
**Effort**: 3-4 hours
**Files to modify**: 4 files + fixup all constructor sites
**Dependencies**: None

## Problem

The `ImplementerTemplate` in `roko-compose` has 11 rich prompt sections (workspace_map, tasks, brief, preflight, registry, prev_reviews, verify_chain, invariants, enhanced_sections) designed to give implementer agents complete context about the workspace. But the runtime dispatch chain in `dispatch/prompt_builder.rs` uses a `PromptAssembler` that produces a minimal prompt with only role, task, files, acceptance, verify, retry, and allowlist sections. The implementer agent never sees what crates exist, what other tasks will run, or what the PRD requires.

This is why agents duplicate types instead of importing them, why generated code does not match PRD type specs, and why cross-task dependencies break silently.

Additionally, the PRD that motivated the plan is never passed to implementer agents. The `build_prompt` method in `task_parser.rs` only uses `TaskDef` fields (title, description, files, context, verify). The PRD specifying exact CLI flags, API requirements, and type signatures is invisible to the agent.

**Note**: This batch does NOT replace PromptAssembler with ImplementerTemplate wholesale. That is a separate, larger effort. This batch adds the three most impactful sections (workspace_map, tasks_toml, prd_excerpt) to the existing PromptAssembler pipeline using the same `PromptSection` mechanism it already uses.

## Root Cause Chain

### 1. PromptAssembler does not inject workspace/PRD context
**File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch/prompt_builder.rs` (lines 320-494)

`PromptAssembler::assemble()` (line 321) builds prompts with inline section construction. It pushes sections for `role`, `task`, `files`, `acceptance`, `verify`, `retry`, and `allowlist`, then queries source plugins for knowledge/playbooks/effectiveness. It never injects workspace_map, tasks_toml, or prd_excerpt.

### 2. PromptContext has no workspace/PRD fields
**File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch/prompt_builder.rs` (lines 60-77)

`PromptContext` carries `plan_id`, `role`, `workdir`, `files_in_scope`, `acceptance_criteria`, `verify_commands`, `gate_feedback`, and `attempt`. No `prd_excerpt`, `workspace_map`, or `tasks_toml` field exists.

### 3. Plan struct has no PRD excerpt
**File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/plan_loader.rs` (lines 17-25)

`Plan` has `id`, `dir`, and `tasks`. The loader never reads the corresponding PRD from `.roko/prd/published/`.

## Exact Code to Change

### File 1: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch/prompt_builder.rs`

#### Change 1.1: Add workspace/PRD fields to PromptContext (line 60)

**Find this code:**
```rust
#[derive(Debug, Clone)]
pub struct PromptContext {
    /// Plan id.
    pub plan_id: String,
    /// Role label.
    pub role: String,
    /// Workspace root used to resolve `.roko` learning stores.
    pub workdir: PathBuf,
    /// Files in scope for this task (from `task.files`).
    pub files_in_scope: Vec<String>,
    /// Acceptance criteria (from `task.acceptance`).
    pub acceptance_criteria: Vec<String>,
    /// `task.verify` shell commands.
    pub verify_commands: Vec<String>,
    /// Optional structured gate feedback for retry prompts.
    pub gate_feedback: Option<GateFeedback>,
    /// Attempt number (0 = first, > 0 = retry).
    pub attempt: u32,
}
```

**Replace with:**
```rust
#[derive(Debug, Clone)]
pub struct PromptContext {
    /// Plan id.
    pub plan_id: String,
    /// Role label.
    pub role: String,
    /// Workspace root used to resolve `.roko` learning stores.
    pub workdir: PathBuf,
    /// Files in scope for this task (from `task.files`).
    pub files_in_scope: Vec<String>,
    /// Acceptance criteria (from `task.acceptance`).
    pub acceptance_criteria: Vec<String>,
    /// `task.verify` shell commands.
    pub verify_commands: Vec<String>,
    /// Optional structured gate feedback for retry prompts.
    pub gate_feedback: Option<GateFeedback>,
    /// Attempt number (0 = first, > 0 = retry).
    pub attempt: u32,
    /// Workspace map (crate tree), generated at dispatch time.
    pub workspace_map: String,
    /// Tasks TOML content for sibling task awareness.
    pub tasks_toml: String,
    /// PRD excerpt (first 2000 chars of the published PRD).
    pub prd_excerpt: String,
}
```

#### Change 1.2: Populate new fields in `from_task` (line 82)

**Find this code:**
```rust
    pub fn from_task(task: &TaskDef, ctx: &DispatchContext) -> Self {
        Self {
            plan_id: ctx.plan_id.clone(),
            role: ctx.role.clone(),
            workdir: ctx.workdir.clone(),
            files_in_scope: task.files.clone(),
            acceptance_criteria: task.acceptance.clone(),
            verify_commands: task
                .verify
                .iter()
                .map(|step| step.command.clone())
                .collect(),
            gate_feedback: ctx.gate_feedback.clone(),
            attempt: ctx.attempt,
        }
    }
```

**Replace with:**
```rust
    pub fn from_task(task: &TaskDef, ctx: &DispatchContext) -> Self {
        let workspace_map = generate_workspace_map(&ctx.workdir);
        let tasks_toml = load_tasks_toml(&ctx.workdir, &ctx.plan_id);
        let prd_excerpt = load_prd_excerpt(&ctx.workdir, &ctx.plan_id);
        tracing::debug!(
            plan_id = %ctx.plan_id,
            workspace_map_len = workspace_map.len(),
            tasks_toml_len = tasks_toml.len(),
            prd_excerpt_len = prd_excerpt.len(),
            "PromptContext enrichment loaded"
        );
        Self {
            plan_id: ctx.plan_id.clone(),
            role: ctx.role.clone(),
            workdir: ctx.workdir.clone(),
            files_in_scope: task.files.clone(),
            acceptance_criteria: task.acceptance.clone(),
            verify_commands: task
                .verify
                .iter()
                .map(|step| step.command.clone())
                .collect(),
            gate_feedback: ctx.gate_feedback.clone(),
            attempt: ctx.attempt,
            workspace_map,
            tasks_toml,
            prd_excerpt,
        }
    }
```

#### Change 1.3: Add workspace map, tasks TOML, and PRD loading helpers

Add these free functions AFTER the closing `}` of `impl PromptContext` (after line 98) and BEFORE the `impl GateFeedback` block (line 119):

```rust
/// Generate a workspace map by walking the crate directory tree.
///
/// Produces a compact tree of `crates/*/src/` showing modules and key files.
/// Truncated to 20,000 chars to fit within prompt budgets.
fn generate_workspace_map(workdir: &Path) -> String {
    let crates_dir = workdir.join("crates");
    if !crates_dir.is_dir() {
        // Fallback: try workspace root src/
        let src_dir = workdir.join("src");
        if src_dir.is_dir() {
            return walk_src_tree(&src_dir, "src", 0);
        }
        return String::new();
    }

    let mut entries: Vec<_> = match std::fs::read_dir(&crates_dir) {
        Ok(rd) => rd.filter_map(|e| e.ok()).collect(),
        Err(_) => return String::new(),
    };
    entries.sort_by_key(|e| e.file_name());

    let mut map = String::from("## Workspace Map\n\n");
    for entry in entries {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        let src_path = entry.path().join("src");
        if src_path.is_dir() {
            map.push_str(&format!("### crates/{name_str}/\n"));
            map.push_str(&walk_src_tree(&src_path, "src", 1));
            map.push('\n');
        }
        if map.len() > 20_000 {
            map.push_str("\n[... truncated]\n");
            break;
        }
    }

    map
}

/// Recursively walk a src tree and produce an indented file listing.
fn walk_src_tree(dir: &Path, prefix: &str, depth: usize) -> String {
    const MAX_DEPTH: usize = 3;
    if depth > MAX_DEPTH {
        return String::new();
    }
    let indent = "  ".repeat(depth);
    let mut out = String::new();

    let mut entries: Vec<_> = match std::fs::read_dir(dir) {
        Ok(rd) => rd.filter_map(|e| e.ok()).collect(),
        Err(_) => return out,
    };
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        let path = entry.path();
        if path.is_dir() {
            out.push_str(&format!("{indent}- {prefix}/{name_str}/\n"));
            out.push_str(&walk_src_tree(
                &path,
                &format!("{prefix}/{name_str}"),
                depth + 1,
            ));
        } else if name_str.ends_with(".rs") {
            out.push_str(&format!("{indent}- {prefix}/{name_str}\n"));
        }
    }
    out
}

/// Load the tasks.toml content for the current plan.
fn load_tasks_toml(workdir: &Path, plan_id: &str) -> String {
    // Try .roko/plans/{plan_id}/tasks.toml first, then plans/{plan_id}/tasks.toml
    let candidates = [
        workdir
            .join(".roko")
            .join("plans")
            .join(plan_id)
            .join("tasks.toml"),
        workdir.join("plans").join(plan_id).join("tasks.toml"),
    ];
    for path in &candidates {
        if let Ok(content) = std::fs::read_to_string(path) {
            // Truncate to 10,000 chars to fit budget
            if content.len() > 10_000 {
                return format!("{}\n\n[... truncated]", &content[..10_000]);
            }
            return content;
        }
    }
    String::new()
}

/// Load and truncate the published PRD for context injection.
///
/// Looks up the PRD slug from the plan_id (which typically matches the
/// PRD slug), reads from `.roko/prd/published/{slug}.md`, and returns
/// the first 2000 chars.
fn load_prd_excerpt(workdir: &Path, plan_id: &str) -> String {
    let candidates = [
        workdir
            .join(".roko")
            .join("prd")
            .join("published")
            .join(format!("{plan_id}.md")),
        workdir
            .join(".roko")
            .join("prd")
            .join("drafts")
            .join(format!("{plan_id}.md")),
    ];
    for path in &candidates {
        if let Ok(content) = std::fs::read_to_string(path) {
            if content.trim().is_empty() {
                continue;
            }
            let max = 2000;
            if content.len() > max {
                return format!(
                    "{}\n\n[... PRD truncated at {max} chars]",
                    &content[..max]
                );
            }
            return content;
        }
    }
    String::new()
}
```

#### Change 1.4: Inject workspace_map, tasks_toml, and prd_excerpt into the assembled prompt

In `PromptAssembler::assemble()`, find the section assembly block. This is lines 401-420.

**Find this code:**
```rust
        // ── Assemble + budget ─────────────────────────────────────────
        let mut sections: Vec<PromptSection> = Vec::new();
        sections.push(PromptSection::new("role", role_section, 1));
        sections.push(PromptSection::new("task", task_section, 1));
        if let Some(s) = files_section {
            sections.push(PromptSection::new("files", s, 4));
        }
        if let Some(s) = acceptance_section {
            sections.push(PromptSection::new("acceptance", s, 2));
        }
        if let Some(s) = verify_section {
            sections.push(PromptSection::new("verify", s, 3));
        }
        if let Some(s) = retry_section {
            sections.push(PromptSection::new("retry", s, 5));
        }
        if let Some(s) = allowlist_section {
            sections.push(PromptSection::new("allowlist", s, 6));
        }

        let mut diagnostics = PromptDiagnostics::default();
```

**Replace with:**
```rust
        // ── Assemble + budget ─────────────────────────────────────────
        let mut sections: Vec<PromptSection> = Vec::new();
        sections.push(PromptSection::new("role", role_section, 1));
        sections.push(PromptSection::new("task", task_section, 1));
        if let Some(s) = files_section {
            sections.push(PromptSection::new("files", s, 4));
        }
        if let Some(s) = acceptance_section {
            sections.push(PromptSection::new("acceptance", s, 2));
        }
        if let Some(s) = verify_section {
            sections.push(PromptSection::new("verify", s, 3));
        }
        if let Some(s) = retry_section {
            sections.push(PromptSection::new("retry", s, 5));
        }
        if let Some(s) = allowlist_section {
            sections.push(PromptSection::new("allowlist", s, 6));
        }

        // PRD excerpt — the requirements document that motivated this plan
        if !ctx.prd_excerpt.is_empty() {
            sections.push(PromptSection::new(
                "prd_excerpt",
                format!(
                    "# PRD Requirements (source document)\n\n\
                     This is the Product Requirements Document that this plan implements. \
                     Your code MUST satisfy these requirements.\n\n{}",
                    ctx.prd_excerpt
                ),
                7, // higher priority than workspace_map — requirements are critical
            ));
        }

        // Workspace map — gives the agent awareness of crate structure
        if !ctx.workspace_map.is_empty() {
            sections.push(PromptSection::new(
                "workspace_map",
                ctx.workspace_map.clone(),
                8, // drop before knowledge/playbooks, keep before retry
            ));
        }

        // Tasks TOML — sibling task awareness
        if !ctx.tasks_toml.is_empty() {
            sections.push(PromptSection::new(
                "tasks_toml",
                format!(
                    "# Sibling Tasks\n\n\
                     These are the other tasks in this plan. Use them to understand \
                     cross-task dependencies and avoid duplicating work.\n\n\
                     ```toml\n{}\n```",
                    ctx.tasks_toml
                ),
                9, // lower priority than workspace_map
            ));
        }

        let mut diagnostics = PromptDiagnostics::default();
```

#### Change 1.5: Update enforce_budget canonical ordering

The `enforce_budget` method (line 509) has a canonical ordering array that determines final section order. Add the new section names.

**Find this code:**
```rust
        let canonical: &[&str] = &[
            "role",
            "task",
            "files",
            "acceptance",
            "verify",
            "retry",
            "allowlist",
            "knowledge",
            "episode_knowledge",
            "playbooks",
            "section_effectiveness",
        ];
```

**Replace with:**
```rust
        let canonical: &[&str] = &[
            "role",
            "task",
            "prd_excerpt",
            "files",
            "acceptance",
            "verify",
            "retry",
            "allowlist",
            "workspace_map",
            "tasks_toml",
            "knowledge",
            "episode_knowledge",
            "playbooks",
            "section_effectiveness",
        ];
```

### File 2: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/task_parser.rs`

#### Change 2.1: Add PRD excerpt injection to build_prompt (line 455)

**Find this code:**
```rust
    pub fn build_prompt(&self, plan_id: &str, workdir: &Path) -> String {
        let mut prompt = String::new();
        prompt.push_str(&format!("# Task: {}\n\n", self.title));
        prompt.push_str(&format!("Plan: {plan_id}\nTask ID: {}\n", self.id));

        if let Some(max) = self.max_loc {
            prompt.push_str(&format!("Maximum lines of change: {max}\n"));
        }

        if !self.files.is_empty() {
```

**Replace with:**
```rust
    pub fn build_prompt(&self, plan_id: &str, workdir: &Path) -> String {
        let mut prompt = String::new();
        prompt.push_str(&format!("# Task: {}\n\n", self.title));
        prompt.push_str(&format!("Plan: {plan_id}\nTask ID: {}\n", self.id));

        if let Some(max) = self.max_loc {
            prompt.push_str(&format!("Maximum lines of change: {max}\n"));
        }

        // Inject PRD excerpt so the agent knows the actual requirements
        let prd_path = workdir
            .join(".roko")
            .join("prd")
            .join("published")
            .join(format!("{plan_id}.md"));
        if let Ok(prd_content) = std::fs::read_to_string(&prd_path) {
            if !prd_content.trim().is_empty() {
                let max_chars = 2000;
                let excerpt = if prd_content.len() > max_chars {
                    format!(
                        "{}\n\n[... PRD truncated at {max_chars} chars]",
                        &prd_content[..max_chars]
                    )
                } else {
                    prd_content
                };
                prompt.push_str("\n## PRD Requirements (source document)\n\n");
                prompt.push_str(&excerpt);
                prompt.push('\n');
                tracing::debug!(plan_id, prd_len = excerpt.len(), "injected PRD excerpt into build_prompt");
            }
        }

        if !self.files.is_empty() {
```

**Note**: The `tracing::debug!` macro call works without an explicit `use tracing;` import because `tracing` is in `roko-cli`'s `Cargo.toml` dependencies (line 66). You can use `tracing::debug!()` with the full path. If the linter complains, add `use tracing::debug;` to the imports at the top of the file. Do NOT write `use tracing;` -- that is not valid Rust syntax for bringing a crate into scope. `task_parser.rs` currently has no tracing imports at all.

### File 3: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/plan_loader.rs`

#### Change 3.1: Add PRD excerpt to Plan struct (line 17)

**Find this code:**
```rust
/// A loaded plan ready for execution.
#[derive(Debug, Clone)]
pub struct Plan {
    /// Plan identifier (directory name).
    pub id: String,
    /// Directory containing this plan's `tasks.toml`.
    pub dir: PathBuf,
    /// Parsed task definitions.
    pub tasks: TasksFile,
}
```

**Replace with:**
```rust
/// A loaded plan ready for execution.
#[derive(Debug, Clone)]
pub struct Plan {
    /// Plan identifier (directory name).
    pub id: String,
    /// Directory containing this plan's `tasks.toml`.
    pub dir: PathBuf,
    /// Parsed task definitions.
    pub tasks: TasksFile,
    /// PRD excerpt loaded from `.roko/prd/published/{slug}.md` (truncated to 2000 chars).
    /// Empty if no PRD found.
    pub prd_excerpt: String,
}
```

#### Change 3.2: Load PRD in load_plan (line 28)

**Find this code:**
```rust
pub fn load_plan(dir: &Path) -> Result<Plan> {
    let tasks_path = dir.join("tasks.toml");
    if !tasks_path.exists() {
        bail!("No tasks.toml found in {}", dir.display());
    }

    let id = dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "unnamed".to_string());

    let tasks = TasksFile::parse(&tasks_path)
        .with_context(|| format!("failed to parse {}", tasks_path.display()))?;

    info!(plan_id = %id, task_count = tasks.tasks.len(), "loaded plan");
    Ok(Plan {
        id,
        dir: dir.to_path_buf(),
        tasks,
    })
}
```

**Replace with:**
```rust
pub fn load_plan(dir: &Path) -> Result<Plan> {
    let tasks_path = dir.join("tasks.toml");
    if !tasks_path.exists() {
        bail!("No tasks.toml found in {}", dir.display());
    }

    let id = dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "unnamed".to_string());

    let tasks = TasksFile::parse(&tasks_path)
        .with_context(|| format!("failed to parse {}", tasks_path.display()))?;

    // Try to load the PRD excerpt from the published PRD directory.
    // The plan slug (from meta.plan or directory name) typically matches the PRD slug.
    let prd_slug = &tasks.meta.plan;
    let prd_excerpt = load_prd_excerpt_for_plan(dir, prd_slug, &id);

    info!(
        plan_id = %id,
        task_count = tasks.tasks.len(),
        has_prd = !prd_excerpt.is_empty(),
        "loaded plan"
    );
    Ok(Plan {
        id,
        dir: dir.to_path_buf(),
        tasks,
        prd_excerpt,
    })
}

/// Load and truncate the PRD for a plan.
///
/// Searches in order:
/// 1. `.roko/prd/published/{prd_slug}.md` (relative to workspace root)
/// 2. `.roko/prd/published/{plan_id}.md` (fallback if slug differs)
/// 3. `.roko/prd/drafts/{prd_slug}.md` (draft fallback)
///
/// The workspace root is inferred by walking up from the plan directory
/// until we find a `.roko/` directory.
fn load_prd_excerpt_for_plan(plan_dir: &Path, prd_slug: &str, plan_id: &str) -> String {
    const MAX_EXCERPT_CHARS: usize = 2000;

    // Walk up from plan_dir to find the workspace root (contains .roko/)
    let workdir = find_workspace_root(plan_dir);

    let candidates = [
        workdir
            .join(".roko/prd/published")
            .join(format!("{prd_slug}.md")),
        workdir
            .join(".roko/prd/published")
            .join(format!("{plan_id}.md")),
        workdir
            .join(".roko/prd/drafts")
            .join(format!("{prd_slug}.md")),
    ];

    for path in &candidates {
        if let Ok(content) = std::fs::read_to_string(path) {
            if content.trim().is_empty() {
                continue;
            }
            info!(path = %path.display(), len = content.len(), "loaded PRD excerpt");
            if content.len() > MAX_EXCERPT_CHARS {
                return format!(
                    "{}\n\n[... PRD truncated at {} chars]",
                    &content[..MAX_EXCERPT_CHARS],
                    MAX_EXCERPT_CHARS
                );
            }
            return content;
        }
    }
    String::new()
}

/// Walk up from a directory to find the workspace root (directory containing `.roko/`).
fn find_workspace_root(start: &Path) -> PathBuf {
    let mut current = start.to_path_buf();
    for _ in 0..10 {
        if current.join(".roko").is_dir() {
            return current;
        }
        if !current.pop() {
            break;
        }
    }
    // Fallback: assume plan is in .roko/plans/{id}/ or plans/{id}/
    // so workspace root is 2-3 levels up
    start
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| {
            if p.join(".roko").is_dir() {
                Some(p.to_path_buf())
            } else {
                p.parent().map(|pp| pp.to_path_buf())
            }
        })
        .unwrap_or_else(|| start.to_path_buf())
}
```

### File 4: Fix all constructor sites

These are every place in the codebase that constructs `PromptContext` or `Plan` directly with struct literal syntax. Each one MUST add the new fields.

#### 4a. Test constructor in prompt_builder.rs (line 1192)

**Find this code:**
```rust
    fn ctx() -> DispatchContext {
        DispatchContext {
            plan_id: "p".into(),
            role: "implementer".into(),
            workdir: PathBuf::from("/tmp"),
            model_hint: None,
            force_backend: None,
            budget_remaining_usd: 5.0,
            attempt: 0,
            gate_feedback: None,
            routing_context: None,
        }
    }
```

No change needed here -- `DispatchContext` is NOT modified by this batch. `PromptContext` is constructed via `from_task()` which handles the new fields internally.

#### 4b. Test constructor in dispatch/mod.rs (line 342)

**Find this code:**
```rust
    fn make_ctx() -> DispatchContext {
        DispatchContext {
            plan_id: "p1".into(),
            role: "implementer".into(),
            workdir: PathBuf::from("/tmp"),
            model_hint: None,
            force_backend: None,
            budget_remaining_usd: 5.0,
            attempt: 0,
            gate_feedback: None,
            routing_context: None,
        }
    }
```

No change needed here either -- DispatchContext is unchanged.

#### 4c. Test constructor in model_routing.rs (line 250)

No change needed -- same reason.

#### 4d. Plan struct constructor in plan_loader.rs tests

The test function `load_single_plan` calls `load_plan()` which internally creates the struct. No direct `Plan { ... }` literal in test code. But `load_plan` now returns a Plan with the new `prd_excerpt` field, so all test paths through `load_plan()` automatically get `prd_excerpt: String::new()` (since test temp dirs won't have PRD files).

**Confirm**: Search for all `Plan {` struct literals in `crates/roko-cli/src/runner/plan_loader.rs`. The ONLY one is inside `load_plan()` at line 43, which we already updated above.

#### 4e. Important: check event_loop.rs for Plan usage

`event_loop.rs` uses `plans: Vec<Plan>` passed in as an argument (line 109). It never constructs Plan directly. No change needed.

## Verification

```bash
# 1. Build to verify compilation
cd /Users/will/dev/nunchi/roko/roko
cargo check -p roko-cli 2>&1 | head -30

# 2. Run prompt_builder tests
cargo test -p roko-cli --lib dispatch::prompt_builder 2>&1 | tail -20

# 3. Run plan_loader tests
cargo test -p roko-cli --lib runner::plan_loader 2>&1 | tail -20

# 4. Run the compose template tests (should still pass -- we did not change compose)
cargo test -p roko-compose --lib templates 2>&1 | tail -20

# 5. Search for any remaining PromptContext or Plan struct literal constructors
grep -rn 'PromptContext {' crates/roko-cli/src/ --include='*.rs' | grep -v target/
grep -rn 'plan_loader::Plan {' crates/roko-cli/src/ --include='*.rs' | grep -v target/

# 6. Clippy
cargo clippy -p roko-cli --no-deps -- -D warnings 2>&1 | tail -20
```

## Agent Prompt

```
You are implementing enrichment of the Roko agent prompt pipeline: adding workspace map, tasks TOML, and PRD excerpt to the system prompt so implementer agents see real context.

## Context

Roko is a Rust agent toolkit with 18 crates. The `PromptAssembler` in
`crates/roko-cli/src/dispatch/prompt_builder.rs` produces the system prompt for
implementer agents. Currently it only includes: role, task, files, acceptance,
verify, retry, allowlist, plus knowledge/playbook/effectiveness plugins. Agents
never see:

- The workspace structure (what crates exist, what modules they have)
- The tasks.toml (what sibling tasks exist, what the DAG looks like)
- The PRD requirements document that motivated the plan

This causes agents to duplicate types, invent incompatible APIs, and miss
requirements.

## Architecture

The dispatch chain is:

1. `event_loop.rs` line 2144: constructs `DispatchContext` with plan/task/role info
2. `event_loop.rs` line 2162: calls `dispatcher.plan(task_def, &dispatch_ctx)`
3. `dispatch/mod.rs` line 153: `Dispatcher::plan()` calls `PromptContext::from_task(task, ctx)`
4. `dispatch/mod.rs` line 154: calls `self.prompt_assembler.assemble(task, &prompt_ctx)`
5. `dispatch/prompt_builder.rs` line 321: `PromptAssembler::assemble()` builds sections
6. The assembled system_prompt + user_prompt are sent to the agent

`PromptContext::from_task()` is the ONLY constructor used in production code
(line 82 of prompt_builder.rs). All test code uses `from_task()` too. So adding
fields there and populating them in `from_task()` covers all paths.

## What to do

Follow the "Exact Code to Change" section in this batch document precisely.

### Step 1: Add 3 new fields to PromptContext
In `prompt_builder.rs` line 60, add to the struct:
- `pub workspace_map: String`
- `pub tasks_toml: String`
- `pub prd_excerpt: String`

### Step 2: Populate them in PromptContext::from_task()
In `prompt_builder.rs` line 82, call the three new helper functions before
constructing Self. Add `tracing::debug!` to log the loaded sizes.

### Step 3: Add helper functions
After the `impl PromptContext` block, add four functions:
- `generate_workspace_map(workdir)` -- walks `crates/*/src/` producing indented tree
- `walk_src_tree(dir, prefix, depth)` -- recursive helper
- `load_tasks_toml(workdir, plan_id)` -- reads the plan's tasks.toml
- `load_prd_excerpt(workdir, plan_id)` -- reads `.roko/prd/published/{plan_id}.md`

### Step 4: Inject sections into PromptAssembler::assemble()
After the `allowlist_section` push (line 418), add three conditional section pushes:
- `prd_excerpt` with drop_priority 7
- `workspace_map` with drop_priority 8
- `tasks_toml` with drop_priority 9

### Step 5: Update canonical order in enforce_budget
Add `"prd_excerpt"`, `"workspace_map"`, `"tasks_toml"` to the canonical array.

### Step 6: Add PRD injection to task_parser.rs build_prompt()
In `task_parser.rs` line 455, after the `max_loc` block, add code to read
`.roko/prd/published/{plan_id}.md` and prepend it as a PRD section.

### Step 7: Add prd_excerpt field to Plan struct in plan_loader.rs
Add `pub prd_excerpt: String` to the Plan struct at line 18, and populate it
in `load_plan()` by reading from `.roko/prd/published/`.

### Step 8: Verify no remaining constructor sites need updating
Run: `grep -rn 'PromptContext {' crates/roko-cli/src/ --include='*.rs'`
All PromptContext construction goes through `from_task()` so no manual fixups
needed beyond what we changed.

### Verification
Run: `cargo check -p roko-cli && cargo test -p roko-cli --lib dispatch::prompt_builder && cargo test -p roko-cli --lib runner::plan_loader && cargo clippy -p roko-cli --no-deps -- -D warnings`
```

## Commit

This batch is committed with Wave 9 (Systemic Pipeline Quality). Do not commit individually.

## Checklist

- [ ] `PromptContext` struct has `workspace_map`, `tasks_toml`, `prd_excerpt` fields
- [ ] `PromptContext::from_task()` calls the three loader helpers with `tracing::debug!`
- [ ] `generate_workspace_map()` walks `crates/*/src/` and produces indented tree
- [ ] `walk_src_tree()` recursive helper with MAX_DEPTH=3
- [ ] `load_tasks_toml()` reads the plan's tasks.toml content, truncates to 10k
- [ ] `load_prd_excerpt()` reads `.roko/prd/published/{plan_id}.md`, truncates to 2000 chars
- [ ] `PromptAssembler::assemble()` pushes prd_excerpt (priority 7), workspace_map (priority 8), tasks_toml (priority 9)
- [ ] `enforce_budget` canonical ordering includes new section names
- [ ] `TaskDef::build_prompt()` in task_parser.rs injects PRD excerpt with tracing
- [ ] `Plan` struct in plan_loader.rs has `prd_excerpt` field
- [ ] `load_plan()` populates `prd_excerpt` via `load_prd_excerpt_for_plan()`
- [ ] `find_workspace_root()` helper added to plan_loader.rs
- [ ] No remaining `PromptContext {}` or `Plan {}` struct literal constructors broken
- [ ] `cargo check -p roko-cli` passes
- [ ] `cargo test -p roko-cli --lib dispatch::prompt_builder` passes
- [ ] `cargo test -p roko-cli --lib runner::plan_loader` passes
- [ ] `cargo clippy -p roko-cli --no-deps -- -D warnings` passes

## Audit Status

Audited: 2026-05-05. PASS no changes needed
