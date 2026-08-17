# 05: Idea → Execution Flow

## Problem

Going from "I want to implement X" to "roko is working on it" requires 5+ commands, each with failure modes, and deep knowledge of the internal pipeline. This is the #1 UX friction point.

### Current Flow (5+ steps, each can fail)

```bash
roko prd idea "implement cursor support"          # 1. capture
roko prd draft new "cursor-backend"               # 2. synthesize (needs API key, model)
roko research enhance-prd cursor-backend          # 3. optional enrichment
roko prd plan cursor-backend                      # 4. generate tasks (fails with weak models)
roko plan run .roko/prd/plans/cursor-backend/     # 5. execute
roko dashboard                                     # 6. monitor
```

Each step can fail independently. The user must diagnose each failure, know the right flags, and remember the exact directory paths.

### What It Should Be (1 command)

```bash
roko develop "I want cursor composer 2 support, like mori has. Spawn agent subprocess, JSON-RPC over stdio."
```

That's it. One command. Roko handles: PRD creation → plan generation → task breakdown → execution → TUI monitoring.

## Current State: `roko do` Already Does This

**`do_cmd.rs` is fully implemented and wired.** The `roko do` command at
`/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/do_cmd.rs` already implements
the full "idea to execution" pipeline with progressive formality routing:

```rust
pub(crate) async fn cmd_do(
    cli: &Cli,
    prompt_args: Vec<String>,
    plan: bool,
    complexity_override: Option<PlanComplexity>,
    dry_run: bool,
    yes: bool,
    ghost: bool,
    compare: bool,
    continue_work: Option<Option<String>>,
    no_cascade: bool,
    provider: Option<String>,
) -> Result<i32>
```

The four complexity paths `roko do` routes through:

| Complexity | Pipeline | Trigger |
|---|---|---|
| `Trivial` | single agent, `mechanical` template | short/simple prompts |
| `Simple` | single agent, `focused` template | medium prompts |
| `Standard` | generate plan → execute | auto-detected or `--plan` |
| `Complex` | PRD → draft → generate plan → execute | large architectural prompts |

### Standard Path Code Flow

```
roko do "implement cursor support"
  → ScopeResolver::resolve(&prompt)  →  PlanComplexity::Standard
  → build_generation_prompt(workdir, prompt, "prompt")
  → run_agent_capture_silent(strategist role, high effort)
  → plan_loader::load_plans(&plans_dir)
  → run_plan_execution(cli, workdir, &plans_dir, ...)
      → runner::event_loop::run(plans, &run_config, &state_hub, cancel)
```

### Complex Path Code Flow

```
roko do "full architectural refactor of dispatch layer"
  → ScopeResolver::resolve(&prompt)  →  PlanComplexity::Complex
  → prd::ensure_dirs(workdir)
  → prd::cmd_idea(workdir, prompt)           # Step 1/4
  → prd_agent_prompt + run_agent_capture_silent(scribe role)  # Step 2/4
  → prd::generate_plan_from_prd(&slug, &draft_path, false)    # Step 3/4
  → run_plan_execution(cli, workdir, &plans_root, ...)         # Step 4/4
```

Fallback: if PRD draft fails, falls back to the Standard path automatically.

## The `roko develop` Command Design

The docs originally proposed `roko develop` as a new command. Given that `roko do` already
implements the same pipeline, `roko develop` should be an alias or thin wrapper that:

1. Always uses `--plan` (forces at least Standard complexity)
2. Shows a plan approval screen before execution
3. Auto-launches the TUI after approval

### Command Signature

```
roko develop [OPTIONS] <PROMPT>...

Arguments:
  <PROMPT>  Natural language description of what to build (can be multiple args joined)

Options:
  --model <MODEL>     Model for planning (default: strongest available)
  --dry-run           Generate plan but don't execute
  --approve           Require TUI approval before execution (default: true)
  --effort <EFFORT>   low/medium/high — controls plan detail level
  --resume <ID>       Resume a previous develop session
  --capture           Store as a note without planning/executing (see roko note)
  --from-notes        Synthesize from accumulated notes
  --tag <TAG>         When used with --from-notes, filter notes by tag
```

### Internal Pipeline

```
roko develop "implement cursor composer 2 support"
```

Internally does:

1. **Capture** — stores the prompt as an idea + creates a PRD draft in one shot
2. **Contextualize** — scans the repo for relevant code (existing cursor references,
   provider system, etc.) via `repo_context::build_repo_context`
3. **Plan** — generates tasks.toml using the strongest available model (strategist role,
   high effort). Uses `plan_generate::build_generation_prompt`.
   - If first model fails validation, auto-escalates via `scope_resolver::ScopeResolver`
   - If TOML has minor schema errors, self-heals them
4. **Present** — shows the generated plan in a compact summary:
   ```
   ═══ cursor-composer-2 ═══════════════════════════════
   6 tasks, ~90 min, max 2 parallel

   T1: JSON-RPC types + stdio transport        [20m]
   T2: Process spawn + session lifecycle        [15m]  → depends: T1
   T3: Streaming event parser                   [15m]  → depends: T1
   T4: LlmBackend trait impl                    [20m]  → depends: T2, T3
   T5: Provider wiring                          [10m]  → depends: T4
   T6: Integration test                         [10m]  → depends: T5

   Press [Enter] to start, [e] to edit, [q] to quit:
   ```
5. **Execute** — launches the plan runner via `runner::event_loop::run`
6. **Monitor** — drops into TUI automatically, showing live progress

## Implementation Sketch

```rust
// crates/roko-cli/src/commands/develop.rs
// NOTE: roko do already handles all pipeline logic.
// develop is a thin wrapper that:
//   1. Forces at least Standard complexity (--plan)
//   2. Adds pre-execution plan approval
//   3. Auto-launches TUI

pub async fn cmd_develop(cli: &Cli, prompt_args: Vec<String>, opts: DevelopOpts) -> Result<i32> {
    // 1. Force plan-level complexity (same as `roko do --plan`)
    let complexity_override = None; // let ScopeResolver classify, but promote via --plan flag

    // 2. Dry-run to get the plan path
    let plans_dir = generate_plan_for_approval(cli, &prompt_args, &opts).await?;

    // 3. Show plan approval screen
    if opts.approve {
        let plans = plan_loader::load_plans(&plans_dir)?;
        let action = present_plan_interactive(&plans)?;
        match action {
            Action::Execute => {},
            Action::Edit => { open_editor(&plans_dir)?; return Ok(0) },
            Action::Quit => return Ok(0),
        }
    }

    // 4. Execute + auto-launch TUI
    if std::io::stdout().is_terminal() && !cli.json {
        // Wire run_plan_execution into TUI mode
        run_plan_execution_with_tui(cli, workdir, &plans_dir).await
    } else {
        run_plan_execution(cli, workdir, &plans_dir, false, None).await
    }
}
```

The key existing functions to reuse (all already in `do_cmd.rs`):
- `run_standard_path` → generates plan from prompt
- `run_complex_path` → PRD → draft → plan → execute
- `run_plan_execution` → runs a plans directory through the event loop

## Repo Context Gathering (Already Implemented)

`think.rs` and `prd.rs` already do this via `repo_context::build_repo_context`:

```rust
// Already exists in crates/roko-cli/src/repo_context.rs
// Called from prd draft new and from think command:
let repo_context_pack = roko_cli::repo_context::build_repo_context(&workdir, &keyword_refs).await?;
```

The context pack is embedded into the plan generation prompt automatically.

## TOML Self-Healing (Design Sketch)

```rust
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

## Failure Recovery

When something in the pipeline fails, `roko develop` / `roko do` doesn't dump an error and exit. It tells you what happened and what to do. The `run_complex_path` already has fallback to `run_standard_path_inner` when PRD draft generation fails.

For model escalation, the `scope_resolver::ScopeResolver` already handles classification. Model
escalation on TOML validation failure is the missing piece — it should try the next model in
the preference list when the generated plan fails to parse.

## Priority

`roko do` is already the one command. The remaining work for `roko develop`:

1. Implement `commands/develop.rs` as a thin wrapper over `do_cmd.rs` with:
   - `--plan` forced
   - Pre-execution plan approval screen
   - Auto-TUI launch
2. Register `develop` in `main.rs` Command enum
3. Add model escalation on TOML validation failure to `plan_generate.rs`
4. Wire `roko note` as the capture-only entry point (see doc 09)
