# 07: `roko develop` — The One Command

## The Problem Statement (from the user)

> I want to have the process of going from "I want to implement cursor like mori" to running
> the TUI with detailed broken down task TOMLs with verification criteria to be as
> straightforward and easy as possible. I don't want friction or 5 different commands.

## Current Reality: `roko do` Already Exists

**`do_cmd.rs` is fully wired.** File:
`/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/do_cmd.rs`

```bash
roko do "implement cursor composer 2 support like mori has"
```

This already does: classify complexity → generate plan → execute plan → report results.
The `Complex` path adds the full PRD → draft → plan → execute pipeline automatically.

The main things `roko develop` adds on top:
1. Always forces plan generation (no accidental simple-path dispatch)
2. Shows the plan in a TUI approval screen before executing
3. Auto-launches the interactive TUI dashboard after approval

## The Solution

```bash
roko develop "I want cursor composer 2 support like mori has — spawn agent subprocess, JSON-RPC over stdio, plugs into the LlmBackend trait"
```

One command. No config editing. No model debugging. Roko handles everything.

## What Happens Internally

```
$ roko develop "cursor composer 2 support like mori"

  Analyzing prompt...
  Scanning codebase for relevant context...
    Found: crates/roko-agent/src/cursor_agent.rs (existing stub)
    Found: crates/roko-agent/src/tool_loop/backends/mod.rs (LlmBackend trait)
    Reference: /Users/will/dev/uniswap/bardo/apps/mori/src/agent/connection.rs (mori impl)

  Generating implementation plan...
  ✓ 6 tasks, ~90 min estimated

  ┌─────────────────────────────────────────────────────────────┐
  │ cursor-composer-2                                    6 tasks │
  ├─────────────────────────────────────────────────────────────┤
  │ T1  JSON-RPC types + stdio transport           [20m]        │
  │ T2  Process spawn + session lifecycle          [15m] ← T1   │
  │ T3  Streaming event parser                     [15m] ← T1   │
  │ T4  LlmBackend trait impl                      [20m] ← T2,3 │
  │ T5  Provider wiring                            [10m] ← T4   │
  │ T6  Integration test                           [10m] ← T5   │
  ├─────────────────────────────────────────────────────────────┤
  │ Each task has: acceptance criteria, verify gates, file list  │
  │                                                             │
  │ [Enter] Start  [e] Edit plan  [v] View details  [q] Quit   │
  └─────────────────────────────────────────────────────────────┘
```

Press Enter → drops into the TUI with the plan running.

## How `roko do` Already Works (Code Walkthrough)

### Complexity Classification

`scope_resolver::ScopeResolver::resolve(&prompt, &scope_config)` classifies every prompt
into one of four tiers:

```rust
pub enum PlanComplexity {
    Trivial,   // direct agent, "mechanical" workflow template
    Simple,    // direct agent, "focused" workflow template
    Standard,  // generate plan → execute (the "standard" path)
    Complex,   // PRD → draft → generate plan → execute
}
```

The `--plan` flag (or `--complexity medium/complex`) forces promotion to Standard or Complex.

### Standard Path (already wired)

1. `build_generation_prompt(workdir, prompt, "prompt")` — builds a system prompt from the
   workspace context for the strategist agent
2. `run_agent_capture_silent(AgentExecOpts { role: "strategist", effort: "high", ... })` —
   dispatches to Claude/OpenAI/etc to generate tasks.toml
3. `plan_loader::load_plans(&plans_dir)` — parses generated plan files
4. `run_plan_execution(cli, workdir, &plans_dir, ...)` — runs via `runner::event_loop::run`

### Complex Path (PRD workflow, already wired)

Steps already implemented in `do_cmd.rs::run_complex_path`:
1. `prd::cmd_idea(workdir, prompt)` — saves idea to `.roko/prd/ideas.jsonl`
2. `prd_agent_prompt + run_agent_capture_silent(scribe role)` — drafts PRD
3. `prd::generate_plan_from_prd(&slug, &draft_path, false)` — generates tasks from PRD
4. `run_plan_execution(...)` — executes

Fallback: if PRD draft fails → falls through to `run_standard_path_inner` automatically.

## Design Principles

1. **One prompt in, TUI out.** No intermediate commands needed.
2. **Smart defaults, zero config.** Uses the strongest available model via
   `model_selection::resolve_effective_model_key`. If model fails, escalates.
3. **Show the plan before executing.** User approves with Enter, not a separate command.
4. **Repo-aware.** Already implemented: `repo_context::build_repo_context` scans the
   workspace and embeds relevant file paths + symbols into the plan generation prompt.
5. **Resumable.** `roko do --continue` picks up where it left off via executor snapshot.
6. **Interruptible.** Ctrl-C handled via `tokio::signal::ctrl_c()` → `CancellationToken`.

## Smart Model Selection (Already Implemented)

`model_selection::resolve_effective_model_key(workdir, cli_model, Some("strategist"), context)`
reads the roko.toml `[models]` table and the `[agent]` config, applies the cascade router's
learned preferences, and returns the best available model key for the given role.

The `cascade_router` (loaded at `run_plan_execution` time) feeds learned performance data
back into future model selections.

## Repo Context Gathering (Already Implemented)

`repo_context::build_repo_context(&workdir, &keyword_refs)` extracts keywords from the
prompt, searches the workspace, and returns a `RepoContextPack` with:
- `key_files`: files matching keywords
- `matching_symbols`: code symbols matching keywords
- `related_prds`: existing PRDs covering related topics
- `related_plans`: existing plans for related features
- `workspace_members`: Cargo workspace crates

This pack is formatted as a `## Repository Context` section appended to the plan generation
prompt. The model sees the real file paths and existing code patterns.

## TOML Self-Healing (Needed)

Currently the generated tasks.toml is validated by `plan_validate.rs` but there is no
self-healing. When validation fails, the pipeline stops. To add self-healing:

```rust
// New: crates/roko-cli/src/plan_generate.rs
fn heal_and_validate(raw_toml: &str) -> Result<TaskPlan> {
    match parse_task_plan(raw_toml) {
        Ok(plan) => Ok(plan),
        Err(errors) if errors.are_all_healable() => {
            let healed = apply_schema_defaults(raw_toml, &errors);
            parse_task_plan(&healed)
        }
        Err(errors) => Err(errors.into()),
    }
}

fn apply_schema_defaults(toml: &str, errors: &[ValidationError]) -> String {
    let mut doc = toml.parse::<toml_edit::DocumentMut>().unwrap();
    for err in errors {
        match err {
            MissingField { path, field: "phase" } => {
                doc[path]["phase"] = value("post");
            }
            MissingField { path, field: "command" } => {
                doc[path]["command"] = value("cargo check --workspace");
            }
            // ... other common fixable errors
        }
    }
    doc.to_string()
}
```

## Implementation Plan

### Phase 1: Wire `roko develop` (wrapper over existing `roko do`)

1. New file: `crates/roko-cli/src/commands/develop.rs`
   - Calls `cmd_do` with `plan = true` (forces Standard/Complex path)
   - After plan generation, presents plan approval screen before execution
   - After approval, calls `run_plan_execution` + auto-launches TUI
2. Register `develop` in `crates/roko-cli/src/main.rs` Command enum
3. Register in `crates/roko-cli/src/commands/mod.rs`

```rust
// crates/roko-cli/src/main.rs — add to Command enum:
/// Capture a prompt, generate a plan, approve, and execute with TUI monitoring.
/// Equivalent to `roko do --plan` with interactive plan approval and auto-TUI.
Develop {
    /// Natural language description of what to build.
    #[arg(value_name = "PROMPT")]
    prompt: Vec<String>,
    /// Preview the generated plan without executing.
    #[arg(long)]
    dry_run: bool,
    /// Skip the approval screen and execute immediately.
    #[arg(long)]
    yes: bool,
    /// Working directory (default: cwd).
    #[arg(long)]
    workdir: Option<PathBuf>,
    /// Override the provider for plan generation and execution.
    #[arg(long)]
    provider: Option<String>,
    /// Resume previous interrupted develop session.
    #[arg(long = "continue", value_name = "WORK_ID", num_args = 0..=1)]
    r#continue: Option<Option<String>>,
},
```

### Phase 2: Plan Approval Screen

Add a pre-execution plan approval display to `develop.rs`:

```rust
async fn present_plan_for_approval(plans_dir: &Path) -> Result<ApprovalAction> {
    let plans = plan_loader::load_plans(plans_dir)?;
    let total_tasks: usize = plans.iter().map(|p| p.tasks.tasks.len()).sum();
    let est_mins: u64 = plans.iter()
        .flat_map(|p| &p.tasks.tasks)
        .map(|t| t.estimated_minutes.unwrap_or(10))
        .sum();

    println!();
    println!("  ┌─────────────────────────────────────────────────────────────┐");
    for plan in &plans {
        println!("  │ {:<52} {:>5} tasks │",
            plan.plan_id, plan.tasks.tasks.len());
        for task in &plan.tasks.tasks {
            let deps = if task.depends_on.is_empty() {
                String::new()
            } else {
                format!(" <- {}", task.depends_on.join(","))
            };
            println!("  │   {:<48} {:>4}m{} │",
                task.id,
                task.estimated_minutes.unwrap_or(10),
                deps);
        }
    }
    println!("  ├─────────────────────────────────────────────────────────────┤");
    println!("  │ Total: {total_tasks} tasks, ~{est_mins} min estimated                          │");
    println!("  │                                                              │");
    println!("  │ [Enter] Start  [e] Edit plan  [v] View tasks  [q] Quit     │");
    println!("  └─────────────────────────────────────────────────────────────┘");
    println!();

    // Read single keypress
    let key = read_single_key()?;
    Ok(match key {
        '\n' | '\r' => ApprovalAction::Execute,
        'e' => ApprovalAction::Edit,
        _ => ApprovalAction::Quit,
    })
}
```

### Phase 3: Polish

- `--resume` for interrupted sessions: already works via `executor_snapshot_path`
- `--capture` mode for braindump accumulation: delegates to `roko note`
- Model escalation on TOML validation failure in `plan_generate.rs`
- Error budget tracking per model via the cascade router

### Phase 4: Intelligence

- Reference material scanning (mori, bardo docs) — already in `repo_context`
- Knowledge store integration — already queried at dispatch time
- Multi-idea synthesis (cluster related notes → single plan) — needs `roko note` + clustering

## Files to Create/Modify

| File | Change |
|------|--------|
| `crates/roko-cli/src/commands/develop.rs` | New — wrapper over do_cmd.rs |
| `crates/roko-cli/src/commands/mod.rs` | Add `pub mod develop;` |
| `crates/roko-cli/src/main.rs` | Add `Develop` variant to Command enum + dispatch |
| `crates/roko-cli/src/plan_generate.rs` | Add TOML self-healing + model escalation |

**No new crates needed.** All logic delegates to existing infrastructure.

## Success Criteria

- [ ] `roko develop "implement X"` works with zero prior configuration beyond an API key
- [ ] Plan generation shows approval screen before execution
- [ ] TUI launches automatically after approval (if TTY)
- [ ] `Ctrl-C` at any point saves state, `roko develop --continue` resumes
- [ ] `--dry-run` shows the plan without executing
- [ ] No warnings about unused providers
- [ ] Entire flow takes <60s from prompt to TUI (excluding agent execution time)
- [ ] `roko do "implement X"` still works as before (no regression)
