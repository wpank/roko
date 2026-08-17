# AUDIT: Batch R4_Z02 — Audit plan generation and validation flow

Run: run-20260429-030528 | Phase: AUDIT | Model: codex-5.5

## Your Role

You are an **auditor**. A fast implementation agent just completed batch `R4_Z02`.
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
Audit plan generation and validation flow — read-only, write findings to `.roko/GAPS.md`

## Runner Context
You are working in runner `mega-parity`, batch R4_Z02.
This batch is part of Runner 4: plan-grounding — Ground PRD/plan generation in the real repository and reject invalid artifacts.

## Problem
Plan generation from PRDs and plan validation are separate code paths that lack repo-awareness. The generator produces `tasks.toml` files that may reference nonexistent crates, use invalid model aliases, or duplicate existing workspace members. Before adding grounding, we need a complete map of what is and isn't validated.

This is a **read-only audit batch**. No code changes. Read the source files and write your findings to `.roko/GAPS.md`.

## Step-by-step instructions

### Step 1: Confirm source files exist

```bash
for f in \
  crates/roko-cli/src/commands/prd.rs \
  crates/roko-cli/src/prd.rs \
  crates/roko-cli/src/plan_generate.rs \
  crates/roko-cli/src/plan_validate.rs \
  crates/roko-cli/src/plan.rs; do
  test -f "$f" && echo "OK: $f" || echo "MISSING: $f"
done
```
All 5 must be `OK`.

### Step 2: Read the entry point

File: `crates/roko-cli/src/commands/prd.rs`, lines 267–271

```rust
PrdCmd::Plan { slug, dry_run } => {
    let prd_path = find_prd(&workdir, &slug)?;
    let _generated_plans_root =
        roko_cli::prd::generate_plan_from_prd(&slug, &prd_path, dry_run).await?;
    Ok(0)
}
```

`find_prd()` (lines 332–337 of the same file) checks:
1. `.roko/prd/published/<slug>.md`
2. `.roko/prd/drafts/<slug>.md`

### Step 3: Read the plan generation function

File: `crates/roko-cli/src/prd.rs`, lines 770–926

The function signature (line 771):
```rust
pub async fn generate_plan_from_prd(slug: &str, prd_path: &Path, dry_run: bool) -> Result<PathBuf>
```

It delegates to `generate_plan_from_prd_with_failure_context()` (line 777).

Key lines in `generate_plan_from_prd_with_failure_context()`:
- Line 803: `crate::plan_generate::build_generator_system_prompt(workdir_ref)` — builds system prompt
- Line 807: `workspace_plans_dir(workdir_ref)` — output root
- Line 809–821: Builds `task_prompt` with PRD content and template guidance
- Line 824–840: Calls `run_agent_logged()` with `role: Some("strategist")`
- Line 847–858: Calls `regenerate_old_format_plans()` if not dry_run
- Line 875: Checks `task_count > template_kind.max_task_count()`
- Lines 891–925: Emits `prd:plan:generated` or `prd:plan:failed` signal

### Step 4: Read the system prompt builder

File: `crates/roko-cli/src/plan_generate.rs`, lines 271–277

```rust
pub fn build_generator_system_prompt(workdir: &Path) -> String {
    let mut prompt = String::new();
    let _ = writeln!(prompt, "{PLAN_GENERATOR_SYSTEM_PROMPT}");
    append_naming_glossary_prompt(&mut prompt, workdir);  // lines 382–402
    append_claude_md_prompt(&mut prompt, workdir);         // lines 404–424
    prompt
}
```

`PLAN_GENERATOR_SYSTEM_PROMPT` is defined at lines 164–267.

The system prompt instructs the agent to search codebase before generating, but does NOT enforce:
- A crate inventory injection
- Validation that the agent actually searched
- Whether `files = [...]` fields in tasks are real paths

### Step 5: Read `plans_dir()`

File: `crates/roko-cli/src/plan.rs`, lines 15–22

```rust
pub fn plans_dir(workdir: &Path) -> PathBuf {
    let top = workdir.join("plans");
    if top.is_dir() {
        return top;
    }
    workdir.join(".roko").join("plans")
}
```

Plans are directories: `plans/<plan-id>/plan.md` + `plans/<plan-id>/tasks.toml`.

### Step 6: Read the plan validation function

File: `crates/roko-cli/src/plan_validate.rs`, lines 98–124

`validate_plans_dir()` calls `validate_tasks_file()` for each `tasks.toml`.

**ERRORS (blocking) — verified from source:**
- `PLAN_001`: Failed to parse TOML (lines ~200)
- `PLAN_002`: `[[task]]` missing/empty (lines ~230)
- `PLAN_003`: Required fields `id`, `title`, `role` missing (lines ~250)
- `PLAN_004`: Duplicate task IDs (lines ~280)
- `PLAN_005`: `depends_on` references unknown task ID (lines ~310)
- `PLAN_006`: Dependency cycle (lines ~340)
- `PLAN_007`: Invalid `gate_rung` (must be 0–6) (lines ~360)
- `PLAN_012`: Malformed `acceptance_contract` (lines ~420)
- `PLAN_020`–`PLAN_026`: Architecture queue tasks missing required fields

**WARNINGS (non-blocking) — verified from source:**
- `PLAN_008`: Task `role` has no compose template
- `PLAN_009`: Task `model` not in configured models map (only when models map provided)
- `PLAN_010`: Task unreachable from DAG root
- `PLAN_011`: `gate_rung = 0` but no verify steps

**NOT checked by `plan validate` (key gaps):**
1. `files = [...]` paths do not exist on disk
2. `context.read_files[*].path` files do not exist on disk
3. `model_hint` values are not valid model names (only `model` is checked, not `model_hint`)
4. `role` values not in known set vs. typos
5. `[[task.verify]]` commands would actually succeed
6. PRD slug in plan metadata exists as a real PRD
7. Symbol names in `context.symbols` exist in codebase
8. Crate names in `files` exist in workspace

### Step 7: Read the regeneration path

File: `crates/roko-cli/src/prd.rs`, lines 183–295

`regenerate_old_format_plan()` receives:
1. `source_content` — the plan.md/source doc content
2. `existing` — the existing `tasks.toml` content (embedded in prompt)
3. Same system prompt as `prd plan` (no additional repo context)

Restores original file on failure.

### Step 8: Read learning records created

`run_agent_logged()` in `crates/roko-cli/src/agent_exec.rs` calls `run_agent_capture_impl(opts, true, Some(episode))`.

Episode persisted to:
- `.roko/memory/episodes.jsonl`
- Triggers `roko_neuro::spawn_episode_distillation()`
- Updates CascadeRouter via `LearningRuntime::record_completed_run()`

Signal emitted (lines 891–906 of `prd.rs`):
```rust
emit_prd_plan_signal(
    &workdir,
    Kind::Custom("prd:plan:generated".into()),
    serde_json::json!({
        "plan_path": plans_root.display().to_string(),
        "task_count": task_count,
        "estimated_complexity": estimated_complexity,
    }),
).await
```
Written to `.roko/signals.jsonl`.

### Step 9: Write findings to `.roko/GAPS.md`

Append this section to `.roko/GAPS.md` (create if missing):

```markdown
## R4_Z02: Plan generation and validation gaps (audited <DATE>)

### Plan generation entry point
- `roko prd plan <slug>` → `commands/prd.rs:267` → `prd::generate_plan_from_prd()`
- `find_prd()` checks `.roko/prd/published/` then `.roko/prd/drafts/`
- Agent: `run_agent_logged()` with `role: Some("strategist")`
- Output directory: `workspace_plans_dir()` → `plans/` or `.roko/plans/`

### System prompt for plan generation
- `build_generator_system_prompt()` in `plan_generate.rs:271`
- Includes: `PLAN_GENERATOR_SYSTEM_PROMPT` + naming glossary + CLAUDE.md
- Does NOT inject crate inventory
- Does NOT validate that agent actually searched before generating

### What `plan validate` checks (PLAN_001 – PLAN_026)
- TOML parse, task array presence, required fields, duplicate IDs
- Dependency references, dependency cycles, gate_rung range
- Malformed acceptance_contract

### What `plan validate` does NOT check
- `files = [...]` path existence on disk
- `context.read_files[*].path` existence on disk
- `model_hint` validity (only `model` is checked)
- `[[task.verify]]` command correctness
- PRD slug existence as real PRD
- Symbol names in `context.symbols` codebase membership
- Crate names in `files` workspace membership

### Regeneration context
- Source: plan.md/prd content + existing tasks.toml embedded in prompt
- No crate inventory passed
- Restores original on agent failure or modern-field validation failure

### Learning records from plan generation
- `.roko/memory/episodes.jsonl` (via `run_agent_logged`)
- `.roko/signals.jsonl` (`prd:plan:generated` or `prd:plan:failed`)
- CascadeRouter rewards updated via `LearningRuntime::record_completed_run()`
```

## Acceptance Criteria

- [ ] Verified all 5 source files exist (Step 1 passes)
- [ ] Plan generation flow documented from `prd plan <slug>` entry to `tasks.toml` write
- [ ] Exact function names and line numbers cited for each step
- [ ] All structural checks in `plan validate` listed (PLAN_001 through PLAN_026)
- [ ] Complete list of NOT-validated items documented (at least 8 gaps)
- [ ] Regeneration context documented (what prompt receives)
- [ ] Output path for generated plans identified (`plans_dir()` behavior)
- [ ] Learning records from plan generation identified
- [ ] Findings appended to `.roko/GAPS.md`

## Verification

```bash
# Confirm source files are readable
for f in \
  crates/roko-cli/src/commands/prd.rs \
  crates/roko-cli/src/prd.rs \
  crates/roko-cli/src/plan_generate.rs \
  crates/roko-cli/src/plan_validate.rs \
  crates/roko-cli/src/plan.rs; do
  wc -l "$f"
done

# Confirm GAPS.md was updated
grep "R4_Z02" .roko/GAPS.md && echo "OK" || echo "MISSING"

# Confirm no source files were modified
git diff --name-only crates/ | head -20
# Expected: no output (no files changed)
```

## Do NOT
- Modify any source files
- Run `roko prd plan` or `roko plan validate` commands
- Make assumptions about code you haven't read (cite actual line numbers)
- Skip documenting the NOT-validated items (this is the key output)
- Delete existing content in `.roko/GAPS.md` — only append

---

## Read-Only Context (do not modify)

### `crates/roko-cli/src/commands/prd.rs`

```rust
//! prd command handlers.
#![allow(unused_imports)]

use crate::*;

pub(crate) async fn cmd_prd(cli: &Cli, cmd: PrdCmd) -> Result<i32> {
    use roko_cli::agent_config::{command_from_config, load_gateway_env, model_from_config};
    use roko_cli::agent_exec::{AgentExecOpts, run_agent_capture_silent};

    let workdir = resolve_workdir(cli);
    let gw = load_gateway_env(&workdir);
    let model = cli.model.clone().or_else(|| model_from_config(&workdir));
    let model_ref = model.as_deref();
    let effort = cli.effort.map(|effort| effort.to_string());
    let effort_ref = effort.as_deref();
    let resume_session = cli.resume.as_deref();
    let agent_command = command_from_config(&workdir).unwrap_or_else(|| "claude".to_string());

    match cmd {
        PrdCmd::Idea { text } => {
            let joined = text.join(" ");
            roko_cli::prd::cmd_idea(&workdir, &joined)?;
            Ok(0)
        }
        PrdCmd::List => {
            roko_cli::prd::cmd_list(&workdir)?;
            Ok(0)
        }
        PrdCmd::Status => {
            roko_cli::prd::cmd_status(&workdir, None)?;
            Ok(0)
        }
        PrdCmd::Draft { cmd: draft_cmd } => match draft_cmd {
            PrdDraftCmd::New { title } => {
                let title = title.join(" ");
                let slug = roko_cli::prd::slugify(&title);
                let drafts = roko_cli::workspace_paths::drafts_dir(&workdir);
                roko_cli::prd::ensure_dirs(&workdir)?;
                let target = drafts.join(format!("{slug}.md"));
                // If the draft exists and has real content (not just scaffold),
                // point the user to `edit` instead. But if it's only the
                // skeleton left by a failed `new` run, overwrite it.
                if target.exists() {
                    let existing = std::fs::read_to_string(&target).unwrap_or_default();
                    let is_skeleton = existing
                        .lines()
                        .filter(|l| {
                            !l.starts_with("---")
                                && !l.starts_with('#')
                                && !l.starts_with("##")
                                && !l.trim().is_empty()
                        })
                        .count()
                        == 0;
                    if !is_skeleton {
                        eprintln!("Draft already exists with content: {}", target.display());
                        eprintln!("Use: roko prd draft edit {slug}");
                        return Ok(1);
                    }
                    eprintln!("Found empty scaffold from previous run — regenerating.");
                }
                let model_key =
                    resolve_effective_model_key(&workdir, cli.model.clone(), Some("scribe"), "prd draft new")?;
                // Write scaffold first so agent can read and fill it
                let frontmatter = roko_cli::prd::new_draft_frontmatter(&slug, &title);
                let scaffold = format!(
                    "{frontmatter}# {title}\n\n\
                     ## Overview\n\n## Requirements\n\n## Acceptance criteria\n\n\
                     ## Design\n\n## References\n"
                );
                std::fs::write(&target, &scaffold)?;
                println!("📄 Creating PRD: {title}");

                let system = roko_cli::prd::prd_agent_prompt(
                    &workdir,
                    &format!(
                        "Fill in the draft PRD at {path}. \
                         If you have file tools, read the codebase to understand what exists \
                         and write the PRD directly to {path}. \
                         If you do NOT have file tools, output the complete PRD markdown \
                         (with YAML frontmatter) as your response — do not wrap in code fences. \
                         Follow the PRD quality standards in your system prompt exactly.",
                        path = target.display()
                    ),
                );
                let task_prompt = format!(
                    "Generate a complete PRD for: {title}. \
                     If you have file tools available, search the codebase to understand \
                     what exists and write the completed PRD to {path}. \
                     Otherwise, output the complete PRD markdown with YAML frontmatter. \
                     Include specific requirements, machine-verifiable acceptance criteria, \
                     and a design section.",
                    path = target.display()
                );
                // Snapshot file mtime before agent runs so we can detect
                // whether a CLI agent wrote the file directly.
                let mtime_before = std::fs::metadata(&target).and_then(|m| m.modified()).ok();

                let started = Instant::now();
                let (exit_code, output) = run_agent_capture_silent(AgentExecOpts {
                    prompt: &task_prompt,
                    workdir: &workdir,
                    model: Some(model_key.as_str()),
                    effort: effort_ref,
                    system_prompt: Some(&system),
                    resume_session,
                    env_vars: &gw.vars,
                    role: Some("scribe"),
                })
                .await?;

                // Check if the agent already wrote the file (CLI agents with tools).
                let mtime_after = std::fs::metadata(&target).and_then(|m| m.modified()).ok();
                let file_was_modified = match (mtime_before, mtime_after) {
                    (Some(before), Some(after)) => after > before,
                    _ => false,
                };

                if file_was_modified {
                    // Agent wrote the file directly — verify it has content.
                    let content = std::fs::read_to_string(&target).unwrap_or_default();
                    let has_content = roko_cli::prd::has_substantive_markdown_content(&content);
                    if has_content {
                        println!("📄 Draft written to {}", target.display());
                    } else {
                        eprintln!(
                            "Agent modified file but left it empty at {}",
                            target.display()
                        );
                    }
                } else if exit_code == 0 && !output.trim().is_empty() {
                    // Agent returned content as text — write it to the file.
                    let content =
                        roko_cli::prd::materialize_agent_markdown_output(&output, Some(&scaffold))
                            .unwrap_or_else(|| scaffold.clone());
                    std::fs::write(&target, content)?;
                    println!("📄 Draft written to {}", target.display());
                } else if exit_code != 0 {
                    eprintln!(
                        "Agent failed (exit {exit_code}). Scaffold preserved at {}",
                        target.display()
                    );
                } else {
                    eprintln!(
                        "Agent returned empty output. Scaffold preserved at {}",
                        target.display()
                    );
                }
                let _ = crate::commands::util::persist_capture_episode(
                    &workdir,
                    &agent_command,
                    Some(model_key.as_str()),
                    "prd-draft-new",
                    &format!("prd:draft:new:{slug}"),
                    &task_prompt,
                    &output,
                    exit_code == 0,
                    started.elapsed().as_millis() as u64,
                    resume_session,
                )
                .await;
                Ok(exit_code)
            }
            PrdDraftCmd::Edit { slug } => {
                let draft = roko_cli::workspace_paths::draft_prd_path(&workdir, &slug);
                if !draft.exists() {
                    eprintln!("Draft not found: {}", draft.display());
                    return Ok(1);
                }
                println!("📝 Refining draft: {slug}");
                let system = roko_cli::prd::prd_agent_prompt(
                    &workdir,
                    &format!(
                        "Read and improve the draft PRD at {path}. \
                         If you have file tools, update that file directly. \
                         If you do NOT have file tools, output the complete improved PRD markdown \
                         with YAML frontmatter and no code fences. \
                         Follow the PRD quality standards in your system prompt.",
                        path = draft.display()
                    ),
                );
                let task_prompt = format!(
                    "Read {path} and improve it: \
                     (1) Are requirements specific and testable? \
                     (2) Are acceptance criteria machine-verifiable shell commands? \
                     (3) Are there 10+ citations with [AUTHOR-YEAR] format? \
                     (4) Are there 2+ mermaid diagrams with color styling? \
                     Search the codebase to verify claims. \
                     If you have file tools, update the file in place. \
                     Otherwise, output the complete improved PRD markdown with YAML frontmatter.",
                    path = draft.display()
                );
                let mtime_before = std::fs::metadata(&draft).and_then(|m| m.modified()).ok();
                let started = Instant::now();
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
                let mtime_after = std::fs::metadata(&draft).and_then(|m| m.modified()).ok();
                let file_was_modified = match (mtime_before, mtime_after) {
                    (Some(before), Some(after)) => after > before,
                    _ => false,
                };
                if file_was_modified {
                    let content = std::fs::read_to_string(&draft).unwrap_or_default();
                    if roko_cli::prd::has_substantive_markdown_content(&content) {
                        println!("📄 Draft updated at {}", draft.display());
                    } else {
                        eprintln!(
                            "Agent modified file but left it empty at {}",
                            draft.display()
                        );
                    }
                } else if exit_code == 0 {
                    if let Some(content) =
                        roko_cli::prd::materialize_agent_markdown_output(&output, None)
                    {
                        std::fs::write(&draft, content)?;
                        println!("📄 Draft updated at {}", draft.display());
                    } else {
                        eprintln!(
                            "Agent returned empty output. Existing draft preserved at {}",
                            draft.display()
                        );
                    }
                } else if !output.is_empty() {
                    print!("{output}");
                }
                let _ = crate::commands::util::persist_capture_episode(
                    &workdir,
                    &agent_command,
                    model_ref,
                    "prd-draft-edit",
                    &format!("prd:draft:edit:{slug}"),
                    &task_prompt,
                    &output,
                    exit_code == 0,
                    started.elapsed().as_millis() as u64,
                    resume_session,
                )
                .await;
                Ok(exit_code)
            }
            PrdDraftCmd::Promote { slug, auto_execute } => {
                roko_cli::prd::cmd_promote(&workdir, &slug, auto_execute).await?;
                Ok(0)
            }
            PrdDraftCmd::List => {
                let drafts = roko_cli::workspace_paths::drafts_dir(&workdir);
                roko_cli::prd::ensure_dirs(&workdir)?;
                let files = roko_cli::prd::list_md_files(&drafts);
                if files.is_empty() {
                    println!("No drafts. Create one: roko prd draft new \"title\"");
                } else {
                    for f in &files {
                        println!("  {}", f.file_stem().unwrap_or_default().to_string_lossy());
                    }
                }
                Ok(0)
            }
        },
        PrdCmd::Plan { slug, dry_run } => {
            let prd_path = find_prd(&workdir, &slug)?;
            let model_key =
                resolve_effective_model_key(&workdir, cli.model.clone(), Some("strategist"), "prd plan")?;
            let _generated_plans_root =
                roko_cli::prd::generate_plan_from_prd_with_model(
                    &slug,
                    &prd_path,
                    dry_run,
                    Some(model_key.as_str()),
                )
                .await?;
            Ok(0)
        }
        PrdCmd::Consolidate => {
            println!("🔄 Scanning all PRDs for duplicates, gaps, and inconsistencies...");
            let mut all_context = String::new();
            for dir_name in ["published", "drafts"] {
                let dir = roko_cli::workspace_paths::prd_dir(&workdir).join(dir_name);
                for path in roko_cli::prd::list_md_files(&dir) {
                    if let Ok(c) = std::fs::read_to_string(&path) {
                        let truncated: String = c.lines().take(50).collect::<Vec<_>>().join("\n");
                        let _ = write!(all_context, "### {}\n{truncated}\n---\n\n", path.display());
                    }
                }
            }
            let ideas = std::fs::read_to_string(roko_cli::workspace_paths::ideas_path(&workdir))
                .unwrap_or_default();
            let task_prompt = format!(
                "Review ALL existing PRDs and ideas. Report: \
                 (1) DUPLICATES: PRDs covering the same thing (propose merge). \
                 (2) GAPS: Areas with no PRD coverage. \
                 (3) INCONSISTENCIES: Conflicting requirements. \
                 (4) STALE: Requirements already implemented (check the code). \
                 (5) IDEAS TO PROMOTE: Ideas that should become draft PRDs. \
                 After analysis, create new drafts for gaps and update existing PRDs.\n\n\
                 PRDs:\n{all_context}\n\nIdeas:\n{ideas}"
            );
            let system = roko_cli::prd::prd_agent_prompt(&workdir, "Consolidate all PRDs");
            let started = Instant::now();
            let (exit_code, output) = run_agent_capture_silent(AgentExecOpts {
                prompt: &task_prompt,
                workdir: &workdir,
                model: model_ref,
                effort: effort_ref,
                system_prompt: Some(&system),
                resume_session,
                env_vars: &gw.vars,
                role: Some("strategist"),
            })
            .await?;
            if !output.is_empty() {
                print!("{output}");
            }
            let _ = crate::commands::util::persist_capture_episode(
                &workdir,
                &agent_command,
                model_ref,
                "prd-consolidate",
                "prd:draft:consolidate",
                &task_prompt,
                &output,
                exit_code == 0,
                started.elapsed().as_millis() as u64,
                resume_session,
            )
            .await;
            Ok(exit_code)
        }
    }
}

fn resolve_effective_model_key(
    workdir: &Path,
    cli_model: Option<String>,
    role: Option<&str>,
    context: &str,
) -> Result<String> {
    let config = crate::load_roko_config(workdir)?;
    let selection = roko_cli::model_selection::resolve_effective_model(
        cli_model,
        None,
        role,
        None,
        &config,
    )
    .map_err(|err| anyhow::anyhow!("resolve model selection for {context}: {err}"))?;
    eprintln!("[{context}] effective selection: {}", selection.reason);
    Ok(selection.effective_model_key)
}

/// Find a PRD by slug in either published or drafts.
pub(crate) fn find_prd(workdir: &Path, slug: &str) -> Result<PathBuf> {
    if let Some(path) = roko_cli::workspace_paths::find_prd_path(workdir, slug) {
        return Ok(path);
    }
    anyhow::bail!("PRD not found: {slug} (checked published/ and drafts/)");
}

/// Auto-detect the project domain from file patterns in the target directory.
pub(crate) fn detect_project_domain(target: &Path) -> &'static str {
    if target.join("Cargo.toml").exists() {
        "rust"
    } else if target.join("package.json").exists() {
        "typescript"
    } else if target.join("go.mod").exists() {
        "go"
    } else if target.join("requirements.txt").exists()
        || target.join("pyproject.toml").exists()
        || target.join("setup.py").exists()
    {
        "python"
    } else if target.join("Gemfile").exists() {
        "ruby"
    } else if target.join("pom.xml").exists() || target.join("build.gradle").exists() {
        "java"
    } else {
        "general"
    }
}

/// Verify configuration hint based on domain profile.
pub(crate) fn domain_gate_hint(domain: &str) -> &'static str {
    match domain {
        "rust" => "compile (cargo check), test (cargo test), clippy (cargo clippy)",
        "typescript" => "compile (tsc --noEmit), test (npm test), lint (eslint)",
        "go" => "compile (go build), test (go test), lint (golangci-lint)",
        "python" => "test (pytest), lint (ruff), typecheck (mypy)",
        "ruby" => "test (rspec), lint (rubocop)",
        "java" => "compile (mvn compile), test (mvn test)",
        _ => "compile, test, lint (configure in roko.toml)",
    }
}
```

### `crates/roko-cli/src/plan_generate.rs`

```rust
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
            Self::Default => 20,
            Self::Compact => 12,
            Self::Strict => 8,
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
        "- Keep the plan within this budget unless the PRD explicitly requires more tasks."
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
pub const PLAN_GENERATOR_SYSTEM_PROMPT: &str = r#"You are a task decomposition engine for software projects. Your job is to take a feature description and produce a set of tasks that are so precisely scoped that even the smallest, cheapest LLM can execute them correctly.

## Core principles

1. **Surgical scope**: Each task touches 1-2 files, changes ≤50 lines. If a change requires more, split it.
2. **Precise context**: For each task, specify EXACTLY which files and line ranges to read. Not "read the crate" — "read lines 40-80 of src/lib.rs".
3. **Executable verification**: Every acceptance criterion is a shell command that exits 0 on success, 1 on failure. No subjective criteria.
4. **Dependency ordering**: Types before implementations. Implementations before wiring. Wiring before tests.
5. **Model hints**: Assign the cheapest model that can handle each task. Imports → Haiku. Single function → Sonnet. Multi-module wiring → Opus.

## Task tiers

| Tier | Name | Max LOC | Model | Examples |
|------|------|---------|-------|----------|
| 0 | Mechanical | 20 | haiku | Add import, add struct field, rename function |
| 1 | Focused | 50 | sonnet | Implement function body, write single test |
| 2 | Integrative | 150 | sonnet/opus | Wire module A→B, implement trait for type |
| 3 | Architectural | 300 | opus | Design new API, decompose complex feature |

## Output format

Create plan directories with these files:

### tasks.toml
```toml
[meta]
plan = "<slug>"
total = <N>
done = 0
status = "ready"
max_parallel = <N>  # how many can run concurrently

[[task]]
id = "T1"
title = "<imperative verb phrase>"
description = "<short outcome description>"
status = "ready"
tier = "mechanical"       # mechanical | focused | integrative | architectural
model_hint = "haiku"      # cheapest model for this tier
max_loc = 20              # maximum lines of change
files = ["<path>"]        # files this task modifies
allowed_tools = ["read_file", "grep"]
denied_tools = []
mcp_servers = ["filesystem"] # MCP servers this task needs
depends_on = []

# SURGICAL CONTEXT: exactly what the agent needs to read
[task.context]
read_files = [
    { path = "<file>", lines = "40-80", why = "<reason>" },
]
symbols = [
    "<TypeName>::<method> — <brief signature description>",
]
anti_patterns = [
    "Do NOT create new files. Modify <file> only.",
]

# EXECUTABLE VERIFICATION
[[task.verify]]
phase = "structural"
command = "grep -q 'pattern' path/to/file"
fail_msg = "Pattern not found in file"

[[task.verify]]
phase = "compile"
command = "cargo check -p <crate>"

[[task.verify]]
phase = "test"
command = "cargo test -p <crate> -- <test_name>"
```

## Before generating tasks, you MUST:

1. Search the codebase to understand what exists:
   `grep -rn 'TypeName' crates/ --include='*.rs' | grep -v target/ | head -20`

2. Read the specific files you're generating tasks for — understand the current code.

3. Check if the feature already exists (partially or fully):
   `grep -rn 'feature_keyword' crates/ --include='*.rs' | grep -v target/`

4. For each task, verify the context files actually exist:
   `test -f <path> && echo "exists" || echo "MISSING"`

## Language detection

Detect the project language and use the right commands:
- Cargo.toml → Rust: `cargo check`, `cargo test`, `cargo clippy`
- package.json → TypeScript: `npx tsc`, `npx jest`, `npx eslint`
- go.mod → Go: `go build`, `go test`, `golangci-lint`
- pyproject.toml/setup.py → Python: `python -m py_compile`, `pytest`, `ruff`

## Quality gates for YOUR output

Before finalizing, verify your tasks against:
- [ ] Every task has ≤ max_loc lines of change for its tier
- [ ] Every task has at least 1 structural check + 1 compile check
- [ ] No task requires reading more than 3 files
- [ ] Anti-patterns are specific (not generic "be careful")
- [ ] Dependencies form a DAG (no cycles)
- [ ] The cheapest possible model is assigned to each task
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
        assert_eq!(template.max_task_count(), 20);
    }

    #[test]
    fn resolves_strict_template() {
        let template = PlanTemplateKind::resolve(Some("strict"));
        assert_eq!(template.label(), "strict");
        assert_eq!(template.default_model_tier(), "integrative");
        assert_eq!(template.gate_strictness(), "strict");
        assert_eq!(template.max_task_count(), 8);
    }

    #[test]
    fn template_guidance_includes_selected_settings() {
        let guidance = render_plan_template_guidance(PlanTemplateKind::Compact);
        assert!(guidance.contains("name: compact"));
        assert!(guidance.contains("default model tier: mechanical"));
        assert!(guidance.contains("gate strictness: standard"));
        assert!(guidance.contains("max task count: 12"));
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
        "The following tasks.toml exists but is missing full metadata (description, tier, model_hint, \
         read_files, verify, context, max_loc, mcp_servers). Your job is to read the codebase and fill in \
         every field for each task. Keep the existing id, title, description, and depends_on. Add:\n\
         - `tier` (mechanical/focused/integrative/architectural)\n\
         - `model_hint` (the cheapest model for that tier)\n\
         - `max_loc` (estimated lines of change)\n\
         - `allowed_tools`, `denied_tools`, and `mcp_servers` (per-task tool/MCP constraints)\n\
         - `[task.context]` with read_files, symbols, anti_patterns\n\
         - `[[task.verify]]` with at least compile + test checks\n\n\
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tier_model_hints() {
        assert_eq!(TaskTier::Mechanical.model_hint(), "claude-haiku-4-5");
        assert_eq!(TaskTier::Focused.model_hint(), "claude-sonnet-4-6");
        assert_eq!(TaskTier::Architectural.model_hint(), "claude-opus-4-6");
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
}
```

### `crates/roko-cli/src/prd.rs` (1273 lines — signatures only)

```rust
34:fn tier_rank(tier: &str) -> u8 {
44:fn rank_to_complexity(rank: u8) -> &'static str {
54:fn generated_plan_stats(paths: &[PathBuf]) -> Result<(usize, String)> {
80:fn normalize_task_title(title: &str) -> String {
91:fn preserve_completed_task_status(
145:fn find_plan_source_document(plan_dir: &Path) -> Result<PathBuf> {
159:fn old_format_plan_dirs(root: &Path) -> Vec<PathBuf> {
361:pub fn ensure_dirs(workdir: &Path) -> Result<()> {
378:pub struct PrdMeta {
380:    pub id: String,
382:    pub title: String,
384:    pub status: String,
386:    pub version: u32,
388:    pub created: String,
390:    pub updated: String,
392:    pub depends_on: Vec<String>,
394:    pub crates: Vec<String>,
396:    pub plans_generated: Vec<String>,
398:    pub coverage: f64,
400:    pub tags: Vec<String>,
402:    pub plan_template: Option<String>,
405:impl PrdMeta {
407:    pub fn parse(content: &str) -> Option<Self> {
450:pub fn list_md_files(dir: &Path) -> Vec<PathBuf> {
465:pub struct PrdEntry {
467:    pub slug: String,
469:    pub title: String,
471:    pub status: String,
473:    pub coverage: f64,
476:fn read_prd_entry(path: &Path) -> PrdEntry {
503:pub fn cmd_idea(workdir: &Path, text: &str) -> Result<()> {
519:pub fn cmd_list(workdir: &Path) -> Result<()> {
573:pub fn cmd_status(workdir: &Path, plans_dir: Option<&Path>) -> Result<()> {
642:pub async fn cmd_promote(workdir: &Path, slug: &str, auto_execute: bool) -> Result<()> {
751:fn auto_plan_enabled(workdir: &Path) -> Result<bool> {
773:pub async fn generate_plan_from_prd(slug: &str, prd_path: &Path, dry_run: bool) -> Result<PathBuf> {
779:pub async fn generate_plan_from_prd_with_model(
790:pub async fn generate_plan_from_prd_with_failure_context(
962:pub fn prd_agent_prompt(workdir: &Path, task: &str) -> String {
1010:pub fn new_draft_frontmatter(slug: &str, title: &str) -> String {
```

### `crates/roko-cli/src/commands/plan.rs` (915 lines — signatures only)

```rust
623:fn resolve_effective_model_key(
```

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
