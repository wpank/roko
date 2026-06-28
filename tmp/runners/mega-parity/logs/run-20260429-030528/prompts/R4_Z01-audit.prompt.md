# AUDIT: Batch R4_Z01 — Audit PRD generation data flow

Run: run-20260429-030528 | Phase: AUDIT | Model: codex-5.5

## Your Role

You are an **auditor**. A fast implementation agent just completed batch `R4_Z01`.
Your job is to verify correctness and fix any issues — do NOT rewrite from scratch.

## Audit Checklist

1. **Compiles:** `cargo check -p <crate>` for each crate touched by this batch
2. **Clippy clean:** `cargo clippy -p <crate> --no-deps -- -D warnings`
3. **Prompt compliance:** Compare the implementation against the original prompt below
4. **No regressions:** Changed files don't break existing functionality
5. **Anti-patterns:** No stubs that silently pass, no inline prompts, no raw CLI spawns
6. **Correct types:** Field names, method signatures, and imports match the actual codebase
7. **Tests pass:** If the prompt required tests, verify they pass

## If You Find Issues

Fix them directly in the files. Then run the verification commands from the prompt.
If you cannot fix an issue, leave a comment in the file explaining why.

## Scope

Only touch files in the batch's write scope. Do NOT refactor unrelated code.

---

## Original Implementation Prompt

## Task
Audit PRD generation data flow

## Runner Context
You are working in runner `mega-parity`, batch R4_Z01.
This batch is part of Runner 4: plan-grounding — Ground PRD/plan generation in the real repository and reject invalid artifacts.

## Problem
PRD generation currently operates without awareness of the repository it targets. The prompt chain from `prd draft new` through agent execution to signal emission has no injection point for repo context, no validation of generated references, and no separation between "the agent ran successfully" and "the PRD it produced is accurate." Before adding grounding, we need a complete map of the existing data flow.

## Architecture Contract
This is a read-only audit batch. No code changes. The output is a written analysis document that identifies:
1. The complete prompt chain from CLI command to agent output
2. How much (if any) repository context currently reaches the generation prompt
3. Where generated PRDs are accepted or rejected (acceptance gate)
4. What signals are emitted on PRD creation
5. What learning records (episodes, efficiency, cascade router) are written
6. Exact insertion points for `RepoContextPack` injection and `ArtifactValidationReport` checks

## Changes Required
None. This is a context-gathering audit.

Write your findings as a structured analysis to stdout. Focus on each of the six analysis points listed above.

---

## Actual Code Paths (verified from source)

### Entry point: `roko prd draft new <title>`

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/prd.rs`

Match arm at line 34:
```rust
PrdCmd::Draft { cmd: draft_cmd } => match draft_cmd {
    PrdDraftCmd::New { title } => {
        let title = title.join(" ");
        let slug = roko_cli::prd::slugify(&title);
        let drafts = roko_cli::workspace_paths::drafts_dir(&workdir);
        roko_cli::prd::ensure_dirs(&workdir)?;
        let target = drafts.join(format!("{slug}.md"));
        // ... scaffold write ...
        let system = roko_cli::prd::prd_agent_prompt(
            &workdir,
            &format!("Fill in the draft PRD at {path}. ..."),
        );
        let task_prompt = format!(
            "Generate a complete PRD for: {title}. ...",
        );
        let (exit_code, output) = run_agent_capture_silent(AgentExecOpts {
            prompt: &task_prompt,
            workdir: &workdir,
            model: model_ref,
            effort: effort_ref,
            system_prompt: Some(&system),
            resume_session,
            env_vars: &gw.vars,
            role: Some("scribe"),
        })
        .await?;
```

(lines 34–160 of `crates/roko-cli/src/commands/prd.rs`)

### Prompt assembly: `prd_agent_prompt()`

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/prd.rs`, lines 948–993

```rust
pub fn prd_agent_prompt(workdir: &Path, task: &str) -> String {
    let mut prompt = String::new();
    let _ = writeln!(prompt, "{}", crate::prd_prompt::PRD_SYSTEM_PROMPT);
    let _ = writeln!(prompt, "\n---\n");
    let _ = writeln!(prompt, "## Project workspace: {}\n", workdir.display());
    // Appends master index
    crate::index::append_master_index_prompt(
        &mut prompt,
        workdir,
        "## Master Index (what already exists — do NOT duplicate)",
    );
    // Appends PRD INDEX.md if it exists
    let prd_index = std::fs::read_to_string(prd_dir(workdir).join("INDEX.md")).unwrap_or_default();
    if !prd_index.is_empty() {
        let _ = writeln!(prompt, "## PRD Index\n{prd_index}\n---\n");
    }
    // Appends first 30 lines of each existing PRD
    for dir in [&published_dir(workdir), &drafts_dir(workdir)] {
        for path in list_md_files(dir) {
            if let Ok(content) = std::fs::read_to_string(&path) {
                let truncated: String = content.lines().take(30).collect::<Vec<_>>().join("\n");
                let _ = writeln!(prompt, "### {}\n{truncated}\n---\n", path.display());
            }
        }
    }
    // Appends ideas.md
    let ideas = std::fs::read_to_string(ideas_path(workdir)).unwrap_or_default();
    if !ideas.is_empty() {
        let _ = writeln!(prompt, "## Recent ideas\n{ideas}\n");
    }
    let _ = writeln!(prompt, "## Your task\n{task}");
    let _ = writeln!(prompt, "\n{}", crate::prd_prompt::PRD_QUALITY_CHECKLIST);
    prompt
}
```

**Key finding**: `prd_agent_prompt()` includes:
- PRD quality standards (from `prd_prompt::PRD_SYSTEM_PROMPT`)
- Workspace path (just the path string, no content)
- Master index (if exists at `docs/00-architecture/INDEX.md` or similar)
- PRD INDEX.md (if exists at `.roko/prd/INDEX.md`)
- First 30 lines of each existing PRD
- ideas.md content

**What is NOT included** in the prompt:
- Workspace member names (no crate inventory)
- Key source files related to the feature
- Symbol/function names that already exist
- Related plan directories
- Whether the target crate/feature already exists

### Agent execution: `run_agent_capture_silent()`

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/agent_exec.rs`, lines 83–85 and 87–179

```rust
pub async fn run_agent_capture_silent(opts: AgentExecOpts<'_>) -> Result<(i32, String)> {
    run_agent_capture_impl(opts, false, None).await
}
```

The implementation (`run_agent_capture_impl`, lines 87–179):
1. Loads routing config from `opts.workdir`
2. Resolves model from config or default (`claude-opus-4-6`)
3. Calls `spawn_agent_scoped()` with `bare_mode: true, dangerously_skip_permissions: true`
4. Sends prompt via `agent.run(&prompt, &Context::now()).await`
5. Returns `(exit_code, rendered_text)`
6. **No episode is persisted** (episode=None in `run_agent_capture_silent`)

### Learning episode persistence

After `run_agent_capture_silent` returns, the caller (`commands/prd.rs`, line 147) persists an episode:

```rust
let _ = crate::commands::util::persist_capture_episode(
    &workdir,
    &agent_command,
    model_ref,
    "prd-draft-new",
    &format!("prd:draft:new:{slug}"),
    &task_prompt,
    &output,
    exit_code == 0,
    started.elapsed().as_millis() as u64,
    resume_session,
)
.await;
```

Episode is written to: `{workdir}/.roko/memory/episodes.jsonl`

Via: `LearningRuntime::open_under(workdir.join(".roko").join("memory"))` → `record_completed_run()`

### Post-generation acceptance gate

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/prd.rs`, lines 117–146

The acceptance gate is:
1. **File modification check**: Did agent modify the file? (`mtime_after > mtime_before`)
2. **Content check**: `has_substantive_markdown_content()` — checks that body has non-heading, non-empty lines after frontmatter
3. **Exit code check**: `exit_code == 0`

**What is NOT checked**:
- Whether crate names referenced in `crates:` frontmatter field actually exist
- Whether dependencies in `depends_on:` frontmatter field refer to real PRDs
- Whether design sections reference real files or functions
- Whether acceptance criteria use valid cargo commands

### Signals emitted

For `prd draft new`: **No signals emitted**. The `emit_prd_plan_signal()` function is only called in `generate_plan_from_prd_with_failure_context()` (lines 894–924):
- `Kind::Custom("prd:plan:generated".into())` — only on plan generation success
- `Kind::Custom("prd:plan:failed".into())` — only on plan generation failure

For `prd draft promote`: The `global_event_bus().emit(RokoEvent::PrdPublished {...})` is called at line 670.

### Entry point: `roko prd plan <slug>`

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/prd.rs`, lines 267–271

```rust
PrdCmd::Plan { slug, dry_run } => {
    let prd_path = find_prd(&workdir, &slug)?;
    let _generated_plans_root =
        roko_cli::prd::generate_plan_from_prd(&slug, &prd_path, dry_run).await?;
    Ok(0)
}
```

### Plan generation prompt assembly

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/prd.rs`, lines 802–821

```rust
let system = augment_generator_system_prompt(
    crate::plan_generate::build_generator_system_prompt(workdir_ref),
    failure_context,
);
let task_prompt = format!(
    "Read the PRD at {path} and generate implementation plan directories \
     under .roko/plans/. Each REQ-XXX requirement becomes one or more tasks. \
     Each acceptance criterion becomes a task verification command. \
     Search the codebase first to understand what already exists. \
     Create plan.md and tasks.toml files directly, including per-task mcp_servers \
     when a task needs a specific MCP server.\n\n\
     {template_guidance}\n\
     PRD content:\n{content}",
    path = prd_path.display(),
    ...
);
```

`build_generator_system_prompt()` (in `plan_generate.rs`, lines 271–277) adds:
- `PLAN_GENERATOR_SYSTEM_PROMPT` (the task decomposition instructions)
- First 160 lines of `docs/00-architecture/01-naming-and-glossary.md` if it exists
- First 120 lines of `CLAUDE.md` if it exists

**What is NOT included in plan generation**:
- Workspace member names (no crate inventory)
- Whether crates referenced in tasks exist
- Whether file paths in `context.read_files` are real

---

## Insertion Points for Grounding

1. **`prd_agent_prompt()` in `prd.rs` line ~991** — insert `RepoContextPack::to_prompt_section()` here, after the workspace line and before existing PRDs

2. **`PrdDraftCmd::New` in `commands/prd.rs` line ~117** — insert `ArtifactValidationReport` check after `has_substantive_markdown_content()` check

3. **`generate_plan_from_prd_with_failure_context()` in `prd.rs` line ~803** — insert `RepoContextPack::to_prompt_section()` into the system prompt

4. **`generate_plan_from_prd_with_failure_context()` in `prd.rs` line ~847** — after `exit_code != 0` check, run `ArtifactValidationReport` validation on generated `tasks.toml` files

5. **Learning gate in `commands/prd.rs` line ~147** — pass `artifact_valid` flag to `persist_capture_episode()` to suppress positive learning signals for invalid artifacts

---

## Write Scope (files you may modify)
- None (read-only audit)

## Read-Only Context (do not modify these)
- `crates/roko-cli/src/prd.rs`
- `crates/roko-cli/src/prd_prompt.rs`
- `crates/roko-cli/src/agent_exec.rs`
- `crates/roko-cli/src/commands/prd.rs`
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-learn/src/runtime_feedback.rs`

## Acceptance Criteria
- [ ] Complete prompt chain mapped from `prd draft new <slug>` to file write
- [ ] Identified exactly what repo context (if any) reaches the generation prompt
- [ ] Identified post-generation acceptance gate (or confirmed none exists)
- [ ] Listed all signals emitted during PRD creation
- [ ] Listed all learning records written during PRD creation
- [ ] Identified 3+ insertion points for context pack and validation

## Verification
```bash
# Verify the files exist and are readable
for f in crates/roko-cli/src/prd.rs crates/roko-cli/src/prd_prompt.rs crates/roko-cli/src/commands/prd.rs crates/roko-cli/src/agent_exec.rs; do
  test -f "$f" && echo "OK: $f" || echo "MISSING: $f"
done
```

## Do NOT
- Modify any source files
- Run the PRD generation command
- Make assumptions about code you haven't read
- Skip any of the six analysis points listed above

---

## Read-Only Context (do not modify)

---

## Verification Commands

Run these and fix any failures:
```bash
cargo check --workspace
cargo clippy --workspace --no-deps -- -D warnings
```

## Do NOT

- Rewrite the entire implementation from scratch
- Add features not in the original prompt
- Modify files outside the write scope
- Skip running verification commands
