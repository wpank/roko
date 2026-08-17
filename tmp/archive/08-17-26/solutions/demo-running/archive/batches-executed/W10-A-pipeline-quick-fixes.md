# W10-A: Pipeline Quick Fixes

**Priority**: P1 -- data integrity and prompt quality bugs found in real pipeline run
**Effort**: 1-2 hours (8 small, independent fixes)
**Files to modify**: 7
**Dependencies**: None

## Problem

Eight small but real bugs found during the first end-to-end pipeline run. Each causes either data corruption, display junk, or incorrect behavior:

1. **14.6**: Raw JSON `{"tool_uses":[...]}` leaks to stdout during `prd plan` because the agent capture function echoes output before TOML extraction.
2. **14.7**: `model_hint` survives validation even though the prompt says "NEVER set model_hint" -- the validator only removes hints for unknown models, keeping known ones like `claude-sonnet-4-6`.
3. **14.9**: Dream consolidation path double-nests `.roko/.roko/` because `DreamRunner::new` receives `.roko/` but internally joins `.roko/` again, producing wrong episode path and 0 episodes processed.
4. **14.10**: Gate rung IDs use `u32::MAX` (4294967295) and `u32::MAX - 1` sentinels for plan-verify and merge, which leak into `gate-thresholds.json` and display as garbage.
5. **14.17**: INDEX.md episode count reads `.roko/memory/episodes.jsonl` but episodes are written to `.roko/episodes.jsonl` -- path mismatch produces count = 0.
6. **14.21**: Plan generation prompt doesn't include the slug, so the model guesses truncated slugs like `"btc-fundincli"` which the validator auto-corrects but TOML quality is poor.
7. **14.22**: Both example tasks in the plan generator system prompt include `mcp_servers = ["filesystem"]`, which the model cargo-cults onto every generated task even when the MCP server doesn't exist.
8. **14.25**: `rebuild_all` after `plan run` uses `std::env::current_dir()` instead of the resolved workdir, so indexes rebuild for the wrong directory when `--workdir` is used.

## Root Cause

Each is a small oversight -- wrong function variant, conditional that should be unconditional, path join error, missing template data, or wrong variable passed.

## Exact Code to Change

### File 1: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/prd.rs`

#### Change 1 (14.6): Use silent capture to suppress JSON tool_uses leak

**Find this code** (lines 1054-1071):
```rust
        let (exit_code, output) = run_agent_capture_logged(
            AgentExecOpts {
                prompt: &task_prompt,
                workdir: workdir_ref,
                model: model.or_else(|| resolved.config.agent.model.as_deref()),
                effort: Some(resolved.config.agent.effort.as_str()),
                system_prompt: Some(&system),
                resume_session: None,
                env_vars: &resolved.config.agent.env,
                role: Some("strategist"),
                allowed_tools: Some("Read,Grep,Glob"),
            },
            AgentExecEpisode {
                task_kind: "prd-plan-generate",
                task_id: &task_id,
            },
        )
        .await?;
```

**Replace with:**
```rust
        let (exit_code, output) = run_agent_capture_silent(AgentExecOpts {
            prompt: &task_prompt,
            workdir: workdir_ref,
            model: model.or_else(|| resolved.config.agent.model.as_deref()),
            effort: Some(resolved.config.agent.effort.as_str()),
            system_prompt: Some(&system),
            resume_session: None,
            env_vars: &resolved.config.agent.env,
            role: Some("strategist"),
            allowed_tools: Some("Read,Grep,Glob"),
        })
        .await?;
        tracing::debug!(
            exit_code,
            output_len = output.len(),
            "prd plan: agent capture completed (silent mode)"
        );
```

Note: `run_agent_capture_silent` takes only `AgentExecOpts`, no `AgentExecEpisode` argument. The retry path at line 1148 already uses this function correctly as a reference. Make sure `run_agent_capture_silent` is imported (check existing imports at top of file -- the retry path already uses it so it should be imported).

#### Change 2 (14.7): Unconditionally strip model_hint from generated plans

**Find this code** (lines 2040-2058):
```rust
                    // Validate model_hint: remove if not in config so runtime picks the default.
                    if let Some(hint_val) = task.get("model_hint").cloned() {
                        if let Some(hint) = hint_val.as_str() {
                            let normalized = crate::task_parser::normalize_model_alias(hint);
                            if !model_in_config(normalized, models) {
                                eprintln!(
                                    "warning: {task_id_label}: model_hint '{hint}' \
                                     not in config, removing (runtime will select)"
                                );
                                task.remove("model_hint");
                            } else if normalized != hint {
                                // Replace short alias with canonical name.
                                task.insert(
                                    "model_hint".to_string(),
                                    toml::Value::String(normalized.to_string()),
                                );
                            }
                        }
                    }
```

**Replace with:**
```rust
                    // Unconditionally strip model_hint -- runtime selects the model.
                    // The plan generator prompt says "NEVER set model_hint" but models
                    // cargo-cult it from examples. Always remove it.
                    if task.contains_key("model_hint") {
                        let hint = task
                            .get("model_hint")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        tracing::info!(
                            task_id = %task_id_label,
                            hint = %hint,
                            "stripping model_hint from generated plan (runtime selects model)"
                        );
                        eprintln!(
                            "info: {task_id_label}: removing model_hint '{hint}' \
                             (runtime selects model)"
                        );
                        task.remove("model_hint");
                    }
```

#### Change 3 (14.21): Inject slug into plan generation prompt

**Find this code** (lines 1030-1048):
```rust
        let task_prompt = format!(
            "Generate an implementation plan from the PRD below.\n\n\
             IMPORTANT: The PRD content is included inline — do NOT read {path} \
             again. You may read up to 5 codebase files to understand existing \
             structure, but then you MUST produce your output.\n\n\
             Each REQ-XXX requirement becomes one or more tasks. \
             Each acceptance criterion becomes a task verification command.\n\n\
             Do NOT create files directly. Instead, output the plan content \
             as follows:\n\n\
             1. Output a fenced block tagged `toml` containing the tasks.toml content.\n\
             2. Optionally output a fenced block tagged `plan.md` containing the plan narrative.\n\n\
             Include per-task mcp_servers when a task needs a specific MCP server.\n\n\
             {template_guidance}\n\
             PRD content:\n{trimmed_content}{prd_context_suffix}",
            path = prd_path.display(),
            template_guidance = template_guidance,
            trimmed_content = trimmed_content,
            prd_context_suffix = prd_context_suffix,
        );
```

**Replace with:**
```rust
        let task_prompt = format!(
            "Generate an implementation plan from the PRD below.\n\n\
             Plan slug (use exactly in meta.plan): {slug}\n\n\
             IMPORTANT: The PRD content is included inline — do NOT read {path} \
             again. You may read up to 5 codebase files to understand existing \
             structure, but then you MUST produce your output.\n\n\
             Each REQ-XXX requirement becomes one or more tasks. \
             Each acceptance criterion becomes a task verification command.\n\n\
             Do NOT create files directly. Instead, output the plan content \
             as follows:\n\n\
             1. Output a fenced block tagged `toml` containing the tasks.toml content.\n\
             2. Output a fenced block tagged `plan.md` containing:\n\
                - A 2-3 sentence plan summary\n\
                - Key architectural decisions\n\
                - Risk areas to watch\n\
                - Dependency graph (which tasks depend on which)\n\n\
             TOML quality checklist:\n\
             - meta.plan MUST be exactly: \"{slug}\"\n\
             - Do NOT set model_hint on any task (runtime selects automatically)\n\
             - Only set mcp_servers if the task needs a specific MCP server not available via default tooling\n\
             - When PRD defines types/structs, embed exact signatures in task descriptions\n\n\
             {template_guidance}\n\
             PRD content:\n{trimmed_content}{prd_context_suffix}",
            slug = slug,
            path = prd_path.display(),
            template_guidance = template_guidance,
            trimmed_content = trimmed_content,
            prd_context_suffix = prd_context_suffix,
        );
        tracing::debug!(slug = %slug, prompt_len = task_prompt.len(), "prd plan: prompt assembled");
```

Note: This change also subsumes W10-E Change 2 (14.20 plan.md) and Change 3 (14.23 type specs) since they modify the same prompt. The "Optionally" is replaced with a direct instruction, and the TOML quality checklist includes the type-spec embedding instruction.

### File 2: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs`

#### Change 4 (14.9): Fix dream path double-nesting

**Find this code** (line 3111):
```rust
        let mut dream_runner = roko_dreams::DreamRunner::new(workdir.join(".roko"), dream_config);
```

**Replace with:**
```rust
        tracing::debug!(workdir = %workdir.display(), "dream consolidation: using workdir (not .roko subdir)");
        let mut dream_runner = roko_dreams::DreamRunner::new(workdir.clone(), dream_config);
```

### File 3: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/gate_dispatch.rs`

#### Change 5a (14.10): Define sentinel constants

**Find this code** (lines 20-21):
```rust
static GATE_SEMAPHORE: OnceLock<Arc<Semaphore>> = OnceLock::new();
```

**Replace with:**
```rust
/// Named sentinel rung IDs for non-task gate completions.
/// Using readable constants instead of u32::MAX avoids garbage in displays and thresholds.
pub const RUNG_PLAN_VERIFY: u32 = 1000;
pub const RUNG_MERGE: u32 = 1001;

static GATE_SEMAPHORE: OnceLock<Arc<Semaphore>> = OnceLock::new();
```

#### Change 5b (14.10): Replace u32::MAX for plan-verify gate_signal call

**Find this code** (line 161):
```rust
                let signal = gate_signal(&plan_id_for_run, &task_id, u32::MAX, &workdir_for_run);
```

**Replace with:**
```rust
                let signal = gate_signal(&plan_id_for_run, &task_id, RUNG_PLAN_VERIFY, &workdir_for_run);
```

#### Change 5c (14.10): Replace u32::MAX for plan-verify GateCompletion rung

**Find this code** (line 204):
```rust
            rung: u32::MAX,
```

**Replace with:**
```rust
            rung: RUNG_PLAN_VERIFY,
```

### File 4: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/merge.rs`

#### Change 6 (14.10): Use RUNG_MERGE constant

**Find this code** (line 25):
```rust
use super::types::{GateCompletion, GateCompletionKind, GateVerdictSummary, RunnerFailureKind};
```

**Replace with:**
```rust
use super::gate_dispatch::RUNG_MERGE;
use super::types::{GateCompletion, GateCompletionKind, GateVerdictSummary, RunnerFailureKind};
```

**Find this code** (line 516):
```rust
                rung: u32::MAX - 1,
```

**Replace with:**
```rust
                rung: RUNG_MERGE,
```

### File 5: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/index.rs`

#### Change 7 (14.17): Fix episode path in INDEX.md

**Find this code** (line 328):
```rust
    let episodes_path = workdir.join(".roko/memory/episodes.jsonl");
```

**Replace with:**
```rust
    let episodes_path = workdir.join(".roko/episodes.jsonl");
```

**Find this code** (line 338):
```rust
    let _ = writeln!(out, "→ `.roko/memory/episodes.jsonl`\n");
```

**Replace with:**
```rust
    let _ = writeln!(out, "→ `.roko/episodes.jsonl`\n");
```

### File 6: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/plan_generate.rs`

#### Change 8 (14.22): Remove mcp_servers from example tasks

**Find this code** (line 207):
```rust
mcp_servers = ["filesystem"] # MCP servers this task needs
```

**Replace with:**
```rust
# mcp_servers omitted -- only set when a task needs a specific MCP server not available via default tooling
```

**Find this code** (line 248):
```rust
mcp_servers = ["filesystem"]
```

**Replace with:**
```rust
# mcp_servers omitted -- only set when a task needs a specific MCP server not available via default tooling
```

### File 7: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/main.rs`

#### Change 9 (14.25): Use resolve_workdir instead of current_dir for rebuild_all

**Find this code** (lines 2098-2101):
```rust
        Command::Plan { cmd } => {
            let result = commands::plan::cmd_plan(cli, cmd).await;
            let _ = roko_cli::index::rebuild_all(&std::env::current_dir().unwrap_or_default());
            result
        }
```

**Replace with:**
```rust
        Command::Plan { cmd } => {
            let wd = resolve_workdir(cli);
            let result = commands::plan::cmd_plan(cli, cmd).await;
            tracing::debug!(workdir = %wd.display(), "rebuilding indexes after plan command");
            let _ = roko_cli::index::rebuild_all(&wd);
            result
        }
```

**Find this code** (lines 2103-2106):
```rust
        Command::Prd { cmd } => {
            let result = commands::prd::cmd_prd(cli, cmd).await;
            let _ = roko_cli::index::rebuild_all(&std::env::current_dir().unwrap_or_default());
            result
        }
```

**Replace with:**
```rust
        Command::Prd { cmd } => {
            let wd = resolve_workdir(cli);
            let result = commands::prd::cmd_prd(cli, cmd).await;
            tracing::debug!(workdir = %wd.display(), "rebuilding indexes after prd command");
            let _ = roko_cli::index::rebuild_all(&wd);
            result
        }
```

**Find this code** (lines 2109-2112):
```rust
        Command::Research { cmd } => {
            let result = commands::research::cmd_research(cli, cmd).await;
            let _ = roko_cli::index::rebuild_all(&std::env::current_dir().unwrap_or_default());
            result
        }
```

**Replace with:**
```rust
        Command::Research { cmd } => {
            let wd = resolve_workdir(cli);
            let result = commands::research::cmd_research(cli, cmd).await;
            tracing::debug!(workdir = %wd.display(), "rebuilding indexes after research command");
            let _ = roko_cli::index::rebuild_all(&wd);
            result
        }
```

Note: `resolve_workdir(cli)` works here because `cli` is `&Cli` (borrowed) in `dispatch_subcommand`, and `resolve_workdir` also takes `&Cli`. The `cmd` is moved into the handler function, but `cli` remains available.

## Verification

```bash
# Build check -- all 7 files must compile
cd /Users/will/dev/nunchi/roko/roko
cargo check -p roko-cli 2>&1 | tail -5

# Verify sentinel constants are used (no u32::MAX in gate_dispatch or merge)
grep -n 'u32::MAX' crates/roko-cli/src/runner/gate_dispatch.rs crates/roko-cli/src/runner/merge.rs
# Should return no results

# Verify episode path is fixed
grep -n 'memory/episodes' crates/roko-cli/src/index.rs
# Should return no results

# Verify model_hint stripping is unconditional
grep -n 'model_in_config' crates/roko-cli/src/prd.rs
# Should NOT appear in the model_hint validation section (may still exist elsewhere)

# Verify mcp_servers removed from examples
grep -n 'mcp_servers = \["filesystem"\]' crates/roko-cli/src/plan_generate.rs
# Should return no results

# Verify dream path fix
grep -n 'workdir.join(".roko")' crates/roko-cli/src/runner/event_loop.rs | grep -i dream
# Should return no results

# Verify rebuild_all uses resolve_workdir
grep -n 'current_dir' crates/roko-cli/src/main.rs | grep rebuild
# Should return no results

# Verify slug is in the plan prompt
grep -n 'Plan slug' crates/roko-cli/src/prd.rs
# Should show the new prompt line
```

## Agent Prompt

```
You are fixing 8 small pipeline bugs found during a real end-to-end run of the roko self-hosting workflow. Each fix is independent and small. This batch file contains exact find/replace pairs for every change.

IMPORTANT CONTEXT: This is a Rust project. The workspace root is /Users/will/dev/nunchi/roko/roko. The CLI crate is at crates/roko-cli/.

Apply all changes in order. For each change, read the source file first to confirm the exact code matches, then apply the replacement.

### Change 1 (14.6): Suppress JSON tool_uses leak during prd plan
File: /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/prd.rs (lines 1054-1071)
- Change `run_agent_capture_logged(...)` to `run_agent_capture_silent(...)`.
- `run_agent_capture_silent` takes only `AgentExecOpts`, NOT `AgentExecEpisode`. Remove the `AgentExecEpisode { ... }` argument entirely.
- The retry path at line 1148 already uses `run_agent_capture_silent` correctly -- use it as reference for the call signature.
- Add a `tracing::debug!` after the call.

### Change 2 (14.7): Unconditionally strip model_hint from generated plans
File: /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/prd.rs (lines 2040-2058)
- Replace the conditional `model_in_config` check with unconditional removal.
- Always remove model_hint, logging the removed value via `tracing::info!` and `eprintln!`.

### Change 3 (14.21 + 14.20 + 14.23): Improve plan generation prompt
File: /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/prd.rs (lines 1030-1048)
- Add `Plan slug (use exactly in meta.plan): {slug}` near the top of the prompt.
- Add `slug = slug` to the format args.
- Replace "Optionally output a fenced block tagged `plan.md`" with direct instruction specifying content requirements.
- Add TOML quality checklist section with slug, model_hint, mcp_servers, and type-spec rules.

### Change 4 (14.9): Fix dream path double-nesting
File: /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs (line 3111)
- Change `workdir.join(".roko")` to `workdir.clone()` in the `DreamRunner::new` call.
- `DreamRunner::new` internally joins `.roko/` -- passing `.roko/` causes double-nesting to `.roko/.roko/`.

### Change 5 (14.10): Replace u32::MAX gate rung sentinels
File: /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/gate_dispatch.rs
- Add two constants after the imports (before line 20): `pub const RUNG_PLAN_VERIFY: u32 = 1000;` and `pub const RUNG_MERGE: u32 = 1001;`
- Replace `u32::MAX` at line 161 with `RUNG_PLAN_VERIFY`
- Replace `u32::MAX` at line 204 with `RUNG_PLAN_VERIFY`

File: /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/merge.rs
- Add import: `use super::gate_dispatch::RUNG_MERGE;`
- Replace `u32::MAX - 1` at line 516 with `RUNG_MERGE`

### Change 6 (14.17): Fix episode path in INDEX.md
File: /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/index.rs
- Line 328: Change `.roko/memory/episodes.jsonl` to `.roko/episodes.jsonl`
- Line 338: Change display path from `.roko/memory/episodes.jsonl` to `.roko/episodes.jsonl`

### Change 7 (14.22): Remove mcp_servers from example tasks in plan_generate
File: /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/plan_generate.rs
- Line 207: Replace `mcp_servers = ["filesystem"] # MCP servers this task needs` with a comment
- Line 248: Replace `mcp_servers = ["filesystem"]` with same comment

### Change 8 (14.25): Use resolve_workdir for rebuild_all
File: /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/main.rs (lines 2098-2112)
- In the Plan, Prd, and Research match arms, replace `std::env::current_dir().unwrap_or_default()` with `resolve_workdir(cli)`.
- Compute `let wd = resolve_workdir(cli);` BEFORE calling the handler (since resolve_workdir borrows cli).
- `resolve_workdir` is defined at line 2481 of main.rs and takes `&Cli`.

After all changes, run:
```bash
cargo check -p roko-cli 2>&1 | tail -20
```
Then run the verification grep commands from the batch file to confirm all fixes are in place.
```

## Commit

This batch is committed with Wave 10. Do not commit individually.

## Checklist

- [ ] 14.6: `run_agent_capture_logged` changed to `run_agent_capture_silent` in prd plan path
- [ ] 14.7: `model_hint` unconditionally stripped from generated plans
- [ ] 14.9: Dream path uses `workdir.clone()` not `workdir.join(".roko")`
- [ ] 14.10: Gate rung sentinels use named constants `RUNG_PLAN_VERIFY` and `RUNG_MERGE`
- [ ] 14.17: INDEX.md reads `.roko/episodes.jsonl` not `.roko/memory/episodes.jsonl`
- [ ] 14.20: plan.md prompt changed from "Optionally" to direct instruction
- [ ] 14.21: Plan generation prompt includes explicit slug and TOML quality checklist
- [ ] 14.22: `mcp_servers = ["filesystem"]` removed from both example tasks
- [ ] 14.23: Type-spec embedding instruction added to plan prompt
- [ ] 14.25: `rebuild_all` uses `resolve_workdir(cli)` not `current_dir()`
- [ ] `cargo check -p roko-cli` passes
- [ ] All verification grep checks pass

## Audit Status

Audited: 2026-05-05. PASS no changes needed
