# AUDIT: Batch R2_B01 — Document current model resolution paths

Run: run-20260429-030528 | Phase: AUDIT | Model: codex-5.5

## Your Role

You are an **auditor**. A fast implementation agent just completed batch `R2_B01`.
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

Document current model resolution paths

## Runner Context

You are working in runner `mega-parity`, batch R2_B01.
This batch is part of Runner 2: execution-contract — Make CLI execution contracts truthful enough
that demo scenarios and agent sessions can rely on them.

## Problem

Model resolution is scattered across 8+ code paths with no single source of truth. The `--model`
CLI flag is accepted but silently dropped in several paths. The cascade router has its own
resolution that may conflict with explicit overrides. Without a map, subsequent batches (B02-B07)
risk creating yet another ad-hoc resolution path.

## Architecture Contract

This is a context-only batch. No code changes. Produce a reference document that B02-B07 depend on.
Target: one `EffectiveModelSelection` module, one `resolve_effective_model()` function.

## Changes Required

Produce `tmp/runners/mega-parity/context/R2_B01_model_resolution_paths.md` documenting all 8
model resolution paths with exact file paths and line numbers as found in the codebase today.

### Path 1: `roko run` (v2/WorkflowEngine path)

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/run.rs`

1. `--model` is read at main.rs:229 into `Cli.model: Option<String>` (global flag).
2. `Cli.model` is placed into `crate::config::Config` before `run_with_workflow_engine_with_hub` is
   called. Trace into `build_workflow_effect_services` at run.rs:480.
3. `build_workflow_effect_services(workdir, cli_config)` at run.rs:480:
   - If `cli_config` is `Some`, uses `c.agent.model` (line 510-512).
   - If `cli_config` is `None`, calls `crate::config::load_layered(workdir)` — reloads config from
     disk, discarding CLI override.
   - The resolved model key is extracted at run.rs:513-517:
     ```rust
     let model_key = config
         .agent
         .model
         .clone()
         .unwrap_or_else(|| model_config.agent.default_model.clone());
     let model = resolve_model(&model_config, &model_key).slug;
     ```
4. `resolve_model` is `roko_core::agent::resolve_model` at
   `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/agent.rs:262`.
5. Display model (stderr only) is resolved separately at run.rs:619-626 by AGAIN calling
   `crate::config::load_layered(workdir)` — a second redundant disk read.

**Drop point**: If `cli_config` is `None` when `build_workflow_effect_services` is called (e.g.,
from `run_plan_with_workflow_engine` at run.rs:717), CLI override is lost because `cli_config` is
hardcoded as `None` at line 717.

### Path 2: `roko run` (legacy/v1 path — `run_once`)

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/run.rs`

1. `run_once(workdir, config, prompt_text, external_hub)` at run.rs:902 receives `config: &Config`.
2. `Config.agent.model: Option<String>` carries the CLI flag.
3. Inside `dispatch_agent` at run.rs:1291, model is read at:
   - routing path (line 1308-1313): `config.agent.model.clone().unwrap_or_else(|| routing_config.agent.default_model.clone())`
   - Claude CLI path (line 1347-1355): `config.agent.model.clone().unwrap_or_else(|| routing_config.agent.default_model.clone())`
   - ollama path (line 1476-1480): `config.agent.model.clone().unwrap_or_else(|| "llama3.1:8b".to_string())`
   - Anthropic API path (line 1606-1610): `config.agent.model.clone().unwrap_or_else(|| "claude-sonnet-4-6".to_string())`
4. `append_episode_log` at run.rs:1762 records model via `resolved_model(config)` helper — infers
   from `config.agent.model` or `config.agent.command`.

**Drop point**: The 4 dispatch sub-paths each have their own fallback logic. No unified precedence.

### Path 3: `roko prd draft new`

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/prd.rs`

1. `cmd_prd(cli, cmd)` at prd.rs:6.
2. `model` resolved at prd.rs:12-13:
   ```rust
   let model = cli.model.clone().or_else(|| model_from_config(&workdir));
   let model_ref = model.as_deref();
   ```
3. `model_from_config` is `roko_cli::agent_config::model_from_config` — reads `roko.toml` via text
   scanning (not full TOML parse).
4. `model_ref` passed directly to `AgentExecOpts { model: model_ref, ... }` at prd.rs:101.
5. CLI `--model` is honored IF present. Falls back to config file parsing.

**No cascade router consulted.** No provider validation.

### Path 4: `roko prd plan <slug>`

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/prd.rs`
Calls: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/prd.rs:771`

1. `PrdCmd::Plan { slug, dry_run }` handler at prd.rs:267-271 calls
   `roko_cli::prd::generate_plan_from_prd(&slug, &prd_path, dry_run)`.
2. `generate_plan_from_prd` calls `generate_plan_from_prd_with_failure_context` at prd.rs:777.
3. Model resolved at prd.rs:828: `model: resolved.config.agent.model.as_deref()` — from
   `crate::load_layered(workdir_ref)`.
4. **CLI `--model` flag is NOT passed into `generate_plan_from_prd`**. The function signature
   takes only `slug`, `prd_path`, `dry_run`. The `cli.model` value from prd.rs:12 is ignored for
   this sub-path.

**Drop point**: `cli.model` is computed at prd.rs:12 but `PrdCmd::Plan` at prd.rs:267 calls
`generate_plan_from_prd` which re-reads config via `load_layered`, discarding `cli.model`.

### Path 5: `roko plan run` (per-task dispatch via `dispatch_agent_with`)

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs`

1. `dispatch_agent_with` at orchestrate.rs:13945 takes `model_override: Option<String>`.
2. Model selection logic at orchestrate.rs:14020-14061 (three branches):
   - With `prompt_override`: uses `explicit_model_override.clone().unwrap_or(config.agent.model.clone()...)`
   - With `task_def`: calls `td.effective_model(config.agent.model.as_deref()..., tier_models)` — task's `model_hint` is honored.
   - Fallback: uses `config.agent.model.clone().unwrap_or("claude-opus-4-6")`.
3. Cascade router at orchestrate.rs:14113 onwards: `cascade_router.explain_route(&routing_ctx, Some(&healthy_models))` — selects via bandit.
4. `explicit_model_override` at orchestrate.rs:14015 comes from the `model_override` param.
5. CLI `--model` reaches here only if it was placed in `RunConfig.model` at commands/plan.rs:286:
   ```rust
   model: roko_config.agent.default_model.clone(),
   ```
   **This line ignores `cli.model`.** The `model` field in `RunConfig` is always
   `roko_config.agent.default_model`, not `cli.model`.

**Drop point**: `cli.model` at main.rs:229 is never passed to `RunConfig.model` in the `plan run`
path (commands/plan.rs:283-316). The cascade router selects a model unaware of the user's override.

### Path 6: `roko plan generate`

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/plan.rs`

1. `PlanCmd::Generate` handler at plan.rs:407-473.
2. Model at plan.rs:413: `let model = model_from_config(&workdir);` — reads only config, ignores `cli.model`.
3. `model_ref` passed to `AgentExecOpts { model: model_ref, ... }` at plan.rs:460.

**Drop point**: `cli.model` is not consulted at all in `plan generate`. `model_from_config` reads
`roko.toml` directly via text scan.

### Path 7: `roko plan regenerate`

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/plan.rs`

1. `PlanCmd::Regenerate` handler at plan.rs:474-600.
2. Model at plan.rs:517-518: `let model = model_from_config(&workdir);` — same as `generate`.
3. `model_ref` passed to `AgentExecOpts`.

**Drop point**: Same as path 6. `cli.model` ignored.

### Path 8a: `roko config providers test`

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/config_cmd.rs`

1. `cmd_provider_test(workdir, provider_name)` at config_cmd.rs:315.
2. `select_provider_test_model(config, provider_name)` at config_cmd.rs:798-816:
   - Prefers `config.agent.default_model` if that model's provider matches.
   - Falls back to alphabetically first model for provider.
3. No `--model` flag consulted. The CLI's `Cli.model` field is never passed into this function.
4. Test is connectivity-only (no real LLM call for ClaudeCli kind, HTTP ping for API kinds).

**Drop point**: `ConfigProviderCmd::Test` handler at commands/config_cmd.rs:114-128 passes
`provider` name but not `cli.model` to `cmd_provider_test`.

### Path 8b: `roko config models route`

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/config_cmd.rs`

1. `cmd_model_route(workdir, role_arg, requested_model, explain, complexity_arg)` at config_cmd.rs:448.
2. `requested_model` comes from `ConfigModelCmd::Route { model, ... }` — a positional argument, not
   `cli.model`.
3. Cascade router consulted at config_cmd.rs:493-506:
   ```rust
   let router = CascadeRouter::load_or_new(&cascade_router_path(workdir), model_slugs.clone());
   let explanation = router.explain_route(&context, Some(available_candidates.as_slice()));
   ```
4. Shows what the cascade router would select, but uses a fresh `RoutingContext` built from the CLI
   args — does not use `resolve_effective_model()` (which doesn't exist yet).

---

## Cascade Router API

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/cascade_router.rs`

### `CascadeRouter::select`
```rust
/// Select a model from a raw context vector.
pub fn select(&self, context_vec: Vec<f64>) -> CascadeSelection {
```
- Takes: raw `Vec<f64>` feature vector
- Returns: `CascadeSelection { model: String, observations: u64, stage: CascadeStage }`

### `CascadeRouter::route_with_cfactor`
The primary method used in orchestrate.rs dispatch. Used indirectly through `explain_route`.

### `CascadeRouter::explain_route`
```rust
pub fn explain_route(
    &self,
    ctx: &RoutingContext,
    candidates: Option<&[String]>,
) -> CascadeRouteExplanation
```
Returns `CascadeRouteExplanation { selected_slug: String, stage: CascadeStage, reason: String, ... }`.

### `CascadeRouter::load_or_new`
```rust
pub fn load_or_new(path: &Path, model_slugs: Vec<String>) -> CascadeRouter
```
Loads persisted state from `path`, falls back to `new(model_slugs)`.

---

## `roko_core::agent::resolve_model` API

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/agent.rs:262`

```rust
pub fn resolve_model(config: &RokoConfig, model_key: &str) -> ResolvedModel
```

Returns:
```rust
pub struct ResolvedModel {
    pub model_key: String,
    pub slug: String,           // API model ID sent to backend
    pub provider_kind: ProviderKind,
    pub provider_config: Option<ProviderConfig>,
    pub profile: Option<ModelProfile>,
    pub backend: AgentBackend,
}
```

Resolution logic:
1. Direct lookup by config key: `config.models.get(model_key)`
2. Fallback: search by slug: any `profile.slug == model_key`
3. Heuristic fallback: `AgentBackend::from_model(model_key)`

---

## Proposed `EffectiveModelSelection` Struct

```rust
/// The result of applying the full precedence chain for model selection.
pub struct EffectiveModelSelection {
    /// Raw value from --model flag, if provided.
    pub requested_model: Option<String>,
    /// The model key that will actually be used (config key or slug).
    pub effective_model_key: String,
    /// Provider name as configured in roko.toml [providers].
    pub provider_key: String,
    /// Provider kind string (e.g. "claude-cli", "anthropic-api").
    pub provider_kind: String,
    /// Backend slug for display/logging.
    pub backend_slug: String,
    /// Which level of the precedence chain resolved this.
    pub source: SelectionSource,
    /// Human-readable explanation of why this model was chosen.
    pub reason: String,
}
```

---

## Proposed `SelectionSource` Enum

```rust
pub enum SelectionSource {
    /// --model flag was provided and validated.
    CliOverride,
    /// task.model_hint in tasks.toml (no --model provided).
    TaskModel,
    /// [agent.role_overrides.<role>] in roko.toml.
    RoleConfig,
    /// CascadeRouter selected based on observations.
    CascadeRouter,
    /// agent.default_model in roko.toml or global config.
    ProjectDefault,
    /// Hardcoded fallback ("claude-sonnet-4-6").
    BuiltInDefault,
}
```

---

## Precedence Order

```
CLI (--model) > task model_hint > role config override > cascade router > project default > builtin default
```

This matches what callers expect but is NOT what the code currently implements uniformly.

---

## Write Scope (files you may modify)

- `tmp/runners/mega-parity/context/R2_B01_model_resolution_paths.md` (context doc only)

## Read-Only Context (do not modify these)

- `crates/roko-cli/src/run.rs`
- `crates/roko-cli/src/commands/prd.rs`
- `crates/roko-cli/src/commands/plan.rs`
- `crates/roko-cli/src/commands/config_cmd.rs`
- `crates/roko-cli/src/main.rs`
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-learn/src/cascade_router.rs`

## Acceptance Criteria

- [ ] All 8 resolution paths are mapped with file:line references
- [ ] Each path documents where --model is read, passed, and/or dropped
- [ ] Cascade router API is documented (method signature, return type)
- [ ] Proposed EffectiveModelSelection struct is specified
- [ ] Proposed SelectionSource enum covers all 6 precedence levels
- [ ] No source code was modified

## Verification

N/A (context-only batch)

## Do NOT

- Change any source code
- Propose new crates or config fields
- Document aspirational behavior — only document what the code actually does

## Evidence

E2E-DOGFOOD-AUDIT Path 7, Path 3, Path 10

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
