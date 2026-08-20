# 34 — Research command cascade learning

**Priority**: P2 — Learning gap: cascade router never learns from `roko research` operations
**Size**: S (2-4 hours)
**Crates**: `crates/roko-cli/` (`src/commands/research.rs`, `src/commands/util.rs`)
**Depends on**: None

---

## Background

Roko is a Rust agent toolkit. It includes a CascadeRouter: a LinUCB contextual bandit that learns which LLM model performs best for different task types. Every time an LLM call completes — whether for plan execution, PRD drafting, or research — the router should receive an "observation" (model name, success/failure, reward) so it can improve future routing decisions.

The cascade router's state is persisted at `.roko/learn/cascade-router.json`. The `roko learn router` command reads this file and shows per-model observation counts and win rates. If a model shows 0 observations despite being used, the router has no data to make informed routing decisions and the dashboard shows models as "(unavailable)".

The `roko research` family of subcommands (`topic`, `search`, `enhance-prd`, `enhance-plan`, etc.) each dispatch LLM calls but record episodes through a code path that initializes the CascadeRouter with a hardcoded fallback model list instead of the actual runtime models. Because the cascade router silently drops observations for models not in its initialized model list, these calls produce no learning signal for the router.

**Note on PRD commands**: The original version of this backlog item included `roko prd draft` and `roko prd plan` in the scope. These commands import `persist_capture_episode` from `crates/roko-cli/src/agent_exec.rs` which correctly calls `LearningRuntime::open_for_project_with_models`. They are already fixed and are NOT in scope for this item.

## Current State

1. **`commands/research.rs` uses the wrong `persist_capture_episode`:**

   - `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/research.rs` has 11 `persist_capture_episode` call sites at approximately lines 96, 147, 282, 330, 396, 445, 485, 531, 587, 638, and 690.
   - Each call is `crate::commands::util::persist_capture_episode(...)` — the `util.rs` version.
   - Example at line 96:
     ```rust
     let _ = crate::commands::util::persist_capture_episode(
         &workdir,
         "perplexity",
         Some(&model_slug),
         "research-topic-deep",
         &format!("research:topic:{}",  topic.to_lowercase().replace(' ', "-")),
         &combined_prompt,
         &output,
         false,
         started.elapsed().as_millis() as u64,
         resume_session,
     ).await;
     ```

2. **`commands/util.rs::persist_capture_episode` opens the router with hardcoded defaults:**

   - `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/util.rs`, line 1855:
     ```rust
     let mut runtime = LearningRuntime::open_for_project(workdir)
         .await
         .map_err(|e| anyhow!("open learning runtime: {e}"))?;
     ```
   - `LearningRuntime::open_for_project` is defined in `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/runtime_feedback.rs`, line 1702. It calls the internal `open()` function which initializes the CascadeRouter at line 1529:
     ```rust
     let cascade_router = CascadeRouter::load_or_new(
         &paths.cascade_router_json,
         vec!["claude-sonnet-4-5".into(), "claude-haiku-4-5".into()],
     );
     ```
   - If the actual model used is anything other than `claude-sonnet-4-5` or `claude-haiku-4-5` (e.g., `claude-sonnet-4-6`), its index is not found in the router. The `record_observation` method at `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/cascade_router.rs`, line 1174 silently returns early when `model_index_for_slug` returns `None`:
     ```rust
     let Some(model_idx) = self.model_index_for_slug(model_slug) else {
         return;
     };
     ```

3. **`agent_exec.rs::persist_capture_episode` is the correct version:**

   - `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/agent_exec.rs`, line 295:
     ```rust
     let model_slugs = capture_runtime_model_slugs(&config, episode.model.as_str());
     let mut runtime = if model_slugs.is_empty() {
         LearningRuntime::open_for_project(workdir).await
     } else {
         LearningRuntime::open_for_project_with_models(workdir, model_slugs).await
     }
     ```
   - `capture_runtime_model_slugs` (defined in `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/learning_helpers.rs`, line 64) reads all model slugs from the runtime config and adds the episode model if not already present. This ensures the router's model list always includes the actual model being used.
   - `commands/prd.rs` already imports from `crate::agent_exec::persist_capture_episode` (line 28 of `prd.rs`), not from `commands::util`. That is why PRD commands work correctly.

4. **`roko learn router` shows the gap:**

   After running `roko research topic "some topic"`, `roko learn router` will show 0 observations for the model that was used, confirming the router was not updated.

## Implementation Plan

**Option A (recommended): Fix `commands/util.rs::persist_capture_episode`**

Change the function in `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/util.rs` at line 1830 to use `open_for_project_with_models` with the actual model slug from config:

```rust
pub(crate) async fn persist_capture_episode(
    workdir: &Path,
    agent_command: &str,
    model: Option<&str>,
    task_kind: &str,
    task_id: &str,
    prompt: &str,
    output: &str,
    success: bool,
    wall_time_ms: u64,
    resume_session: Option<&str>,
) -> Result<()> {
    let (episode, provider) = build_capture_episode(
        agent_command,
        model,
        task_kind,
        task_id,
        prompt,
        output,
        success,
        wall_time_ms,
        resume_session,
    );

    tracing::debug!(workdir = %workdir.display(), "opening project learning runtime");

    // Load config to resolve actual model slugs for the cascade router.
    let config = roko_core::config::loader::load_config_unified(workdir).unwrap_or_default();
    let model_slugs = roko_cli::learning_helpers::capture_runtime_model_slugs(
        &config,
        episode.model.as_str(),
    );

    let mut runtime = if model_slugs.is_empty() {
        LearningRuntime::open_for_project(workdir).await
    } else {
        LearningRuntime::open_for_project_with_models(workdir, model_slugs).await
    }
    .map_err(|e| anyhow!("open learning runtime: {e}"))?;

    let distillation_workdir = workdir.to_path_buf();
    let distillation_caller = roko_cli::learning_helpers::distillation_model_caller(workdir);
    runtime.set_episode_completion_hook(move |episode| {
        roko_neuro::spawn_episode_distillation(
            distillation_workdir.clone(),
            episode,
            Some(std::sync::Arc::clone(&distillation_caller)),
        );
    });

    let mut completed = CompletedRunInput::from_episode(episode);
    completed.provider = Some(provider);
    runtime
        .record_completed_run(completed)
        .await
        .map_err(|e| anyhow!("record learning feedback: {e}"))?;
    Ok(())
}
```

The only changes are the two new lines that load `config` and call `capture_runtime_model_slugs`, and replacing `LearningRuntime::open_for_project(workdir).await` with the conditional that picks `open_for_project_with_models` when models are available.

**Option B: Switch research.rs to use `roko_cli::agent_exec::persist_capture_episode`**

Change each `crate::commands::util::persist_capture_episode` call in `research.rs` to `roko_cli::agent_exec::persist_capture_episode`. This makes research match the same pattern as PRD commands and `commands/prd.rs`.

The import at the top of `research.rs` changes from (implicit via `crate::*`):
```rust
// currently calls: crate::commands::util::persist_capture_episode(...)
```
To explicit:
```rust
use roko_cli::agent_exec::persist_capture_episode;
// then call: persist_capture_episode(...).await
```

Note: `agent_exec::persist_capture_episode` is `pub` (not `pub(crate)`) so it is accessible from `commands/research.rs`.

**Option A is preferred** because it fixes the gap for any other callers of `commands/util::persist_capture_episode` (there may be more than just research.rs) without requiring 11 call-site changes.

## Acceptance Criteria

1. After running `roko research topic "test topic"`, running `roko learn router` (or `roko learn all`) shows at least 1 observation for the model that was used.
2. The file `.roko/learn/cascade-router.json` is updated with a non-zero observation count after a research command completes.
3. `cargo test -p roko-cli 2>&1 | grep -E "test result|FAILED"` shows zero failures.
4. `cargo clippy -p roko-cli -- -D warnings` is clean.

## Verification Checklist

- [ ] Note the current observation count: `cat .roko/learn/cascade-router.json | python3 -m json.tool | grep -i "total_obs\|n_obs\|count"`
- [ ] Run `roko research topic "cascade router test"` in a roko workspace with a configured model
- [ ] Run `roko learn router` and confirm the model used shows >0 observations
- [ ] Run `cat .roko/learn/cascade-router.json` and confirm the file was modified (check `mtime` or observation count)
- [ ] Run `cargo test -p roko-cli -- --test-threads=1 2>&1 | tail -5` — confirm zero failures
- [ ] Run `cargo clippy -p roko-cli -- -D warnings 2>&1 | tail -5` — confirm no warnings
- [ ] Run a second `roko research topic "test2"` and confirm observation count increased

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/util.rs` | In `persist_capture_episode` (line 1830): load config with `load_config_unified`, call `capture_runtime_model_slugs`, use `open_for_project_with_models` when models are available |
