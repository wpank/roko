# PERF_02: LearningRuntime single-open (B03)

## Task

`LearningRuntime::open_under` is called twice per `roko run` (once in the
dispatch path, once inside `append_episode_log`). Each open reads three
JSON files and spawns a distillation task. Open it once and thread it
through.

## Tracker & sources

- Issue tracker row: [ISSUE-TRACKER.md#perf_02](../ISSUE-TRACKER.md#perf_02)
- Plan: `tmp/solutions/perf/implementation/02-learning-runtime-single-open.md`
- Bottleneck: B03 (BOTTLENECK-ANALYSIS.md §B03)
- Performance contract: **C-2** (single LearningRuntime open per CLI invocation)
- Priority: P1
- Effort: ≈1 h
- Depends on: none (independent of PERF_01; can land in either order)
- Wave: 1

## Problem

`crates/roko-cli/src/run.rs::append_episode_log` (≈line 2520) opens its
own `LearningRuntime` even though the calling `run_once` (or the
orchestrator dispatch path) typically already has one. Each open:

1. Reads `.roko/learn/cascade-router.json` (~5 KB, JSON)
2. Reads `.roko/learn/experiments.json` (~2 KB)
3. Reads `.roko/learn/gate-thresholds.json` (~1 KB)
4. Sets up mutexes and the distillation completion hook

Cost: ~70-100 ms per redundant open.

The fix: open the runtime once at the top of `run_once`, register the
distillation hook there, and pass `&mut LearningRuntime` into
`append_episode_log`.

## Exact Changes

### Step 1 — Lift the open out of `append_episode_log`

`crates/roko-cli/src/run.rs:2520..2664`. Today's body roughly:

```rust
async fn append_episode_log(
    workdir: &Path,
    config: &Config,
    prompt: &Engram,
    final_output: &Engram,
    verdicts: &[(String, bool)],
    agent_result: &AgentResult,
) -> Result<()> {
    // ... build episode ...
    let learn_root = workdir.join(".roko").join("learn");
    let mut model_keys: Vec<String> = load_roko_config_models(workdir);
    let current_model = resolved_model(config);
    if !model_keys.iter().any(|k| k == &current_model) {
        model_keys.push(current_model);
    }
    let mut runtime = if model_keys.is_empty() {
        LearningRuntime::open_under(learn_root).await?
    } else {
        LearningRuntime::open_under_with_models(learn_root, model_keys).await?
    };
    let distillation_workdir = workdir.to_path_buf();
    let distillation_caller = distillation_model_caller(workdir);
    runtime.set_episode_completion_hook(move |episode| {
        roko_neuro::spawn_episode_distillation(
            distillation_workdir.clone(),
            episode,
            Some(Arc::clone(&distillation_caller)),
        );
    });
    let mut completed = CompletedRunInput::from_episode(episode);
    completed.provider = Some(infer_provider(config));
    completed.task_metric = Some(build_task_metric(config, prompt, verdicts, agent_result));
    runtime.record_completed_run(completed).await?;
    Ok(())
}
```

New shape (note: signature change):

```rust
async fn append_episode_log(
    runtime: &mut LearningRuntime,
    config: &Config,
    prompt: &Engram,
    final_output: &Engram,
    verdicts: &[(String, bool)],
    agent_result: &AgentResult,
) -> Result<()> {
    // ... build episode (unchanged) ...

    let mut completed = CompletedRunInput::from_episode(episode);
    completed.provider = Some(infer_provider(config));
    completed.task_metric = Some(build_task_metric(config, prompt, verdicts, agent_result));
    runtime
        .record_completed_run(completed)
        .await
        .map_err(|e| anyhow!("record learning feedback: {e}"))?;
    Ok(())
}
```

Delete the `learn_root`, `model_keys`, `LearningRuntime::open_*`, and
`set_episode_completion_hook` blocks from this function entirely.

### Step 2 — Open the runtime once at the top of `run_once`

`crates/roko-cli/src/run.rs::run_once` (≈line 1092). After the substrate
is opened and *before* `dispatch_agent` runs, add:

```rust
let learn_root = workdir.join(".roko").join("learn");
let mut model_keys: Vec<String> = load_roko_config_models(workdir);
let current_model = resolved_model(config);
if !model_keys.iter().any(|k| k == &current_model) {
    model_keys.push(current_model);
}

let mut learning = if model_keys.is_empty() {
    LearningRuntime::open_under(&learn_root).await
} else {
    LearningRuntime::open_under_with_models(&learn_root, model_keys.clone()).await
}
.map_err(|e| anyhow!("open learning runtime: {e}"))?;

// Distillation hook — same code as previously inside append_episode_log.
let distillation_workdir = workdir.to_path_buf();
let distillation_caller = distillation_model_caller(workdir);
learning.set_episode_completion_hook(move |episode| {
    roko_neuro::spawn_episode_distillation(
        distillation_workdir.clone(),
        episode,
        Some(Arc::clone(&distillation_caller)),
    );
});

tracing::info!(
    target: "roko_perf",
    path = %learn_root.display(),
    "learning_runtime_opened"
);
```

### Step 3 — Pass `&mut learning` to `append_episode_log`

Find the call site in `run_once` (≈line 1273). It currently looks like:

```rust
if let Err(err) = append_episode_log(
    workdir,
    config,
    &prompt,
    &final_output_sig,
    &verdict_summary,
    &agent_result,
).await {
    // ...
}
```

Change to:

```rust
if let Err(err) = append_episode_log(
    &mut learning,
    config,
    &prompt,
    &final_output_sig,
    &verdict_summary,
    &agent_result,
).await {
    // ...
}
```

### Step 4 — Audit `load_roko_config_models` for double calls

```bash
rg -n 'load_roko_config_models' crates/roko-cli/src/
```

If it appears twice in `run_once` (once for the open we added in Step 2,
once inside what used to be `append_episode_log`), the second call is
now dead — confirm and remove.

### Step 5 — Audit the orchestrator path (read-only check)

```bash
rg -n 'LearningRuntime::open_under' crates/roko-cli/src/
```

If `crates/roko-cli/src/orchestrate.rs` opens its own `LearningRuntime`
inside the dispatch path that `run_once` invokes via `dispatch_agent`,
that is a SEPARATE double-open we are NOT fixing here. Document it in
your commit body as a follow-up:

```
followup: Orchestrator opens its own LearningRuntime for the cascade
router at orchestrate.rs:NNNN; reusing the run_once-owned instance
would save another ~70 ms but requires a larger refactor.
```

## Write Scope

- `crates/roko-cli/src/run.rs`

## Read-Only Context

- `crates/roko-learn/src/runtime_feedback.rs`
- `crates/roko-cli/src/learning_helpers.rs`
- `tmp/solutions/perf/implementation/02-learning-runtime-single-open.md`
- `tmp/runners/perf/context-pack/00-RULES.md`
- `tmp/runners/perf/context-pack/01-FILE-INVENTORY.md`

## Acceptance Criteria

- [ ] `LearningRuntime::open_under{,_with_models}` is called exactly once inside `run_once`.
- [ ] `append_episode_log` accepts `&mut LearningRuntime` instead of opening its own.
- [ ] `set_episode_completion_hook` registration preserved at the single open site.
- [ ] `tracing::info!(target = "roko_perf", ..., "learning_runtime_opened")` emitted at the open site.
- [ ] `load_roko_config_models(workdir)` no longer called twice in the run path.

## Verify

```bash
# Should show exactly ONE open in run.rs (the one in run_once):
rg -n 'LearningRuntime::open_under' crates/roko-cli/src/run.rs

# Macro-benchmark (post-merge):
RUST_LOG=roko_perf=info ./target/release/roko run --gates none "hi" 2>&1 \
  | rg -c 'learning_runtime_opened'
# Expected: 1
```

## Do NOT

- Do NOT make `LearningRuntime` a `static` singleton. It is workdir-scoped;
  a static would cross-contaminate cascade-router state between tenants in
  `roko serve`.
- Do NOT convert `record_completed_run` to take `Arc<Mutex<LearningRuntime>>`.
  Single-owner `&mut` is fine because the call is sequential within a run.
- Do NOT skip the `set_episode_completion_hook` registration when moving
  the open. The hook is what spawns distillation; losing it silently
  breaks the neuro-store learning loop with no visible error.
- Do NOT pre-open the runtime before checking `--no-learn` flags. Some
  users disable learning; opening then writes files they opted out of.
- Do NOT compile or run tests during the batch (see `00-RULES.md`).
- Do NOT couple this plan to PERF_09 (warm pool). Resist passing the
  runtime through `EffectServices` here; that's plan-09's territory.
- Do NOT attempt to fix the orchestrator's separate open in this batch.
  File a follow-up note as described in Step 5.

## Tracker update

```
tracker: PERF_02 done <commit-sha>
```
