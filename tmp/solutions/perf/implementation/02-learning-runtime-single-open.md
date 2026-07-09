# 02 — LearningRuntime Single-Open (B03)

> Bottleneck: `LearningRuntime::open_under` is called twice per run.
> Each open reads three JSON files, spawns a distillation task, and acquires
> several mutexes. Target savings: 70–100 ms / run.
> Effort: ≈1 h. Risk: low.

---

## Goal & success criteria

After this change, **a single `roko run` opens the `LearningRuntime`
exactly once**, then threads it through to `append_episode_log` and any
other consumer.

Done when:

- A trace-level log line `learning_runtime_opened` appears exactly once
  per `roko run --gates none` invocation.
- `cargo test -p roko-cli` and `cargo test -p roko-learn` are green.
- Macro-benchmark p50 wall-time drops by ≥50 ms vs baseline (cumulative
  with plan 01).

---

## Background

- Bottleneck source: `BOTTLENECK-ANALYSIS.md` §B03,
  `OPTIMIZATION-PLAYBOOK.md` §2.
- `LearningRuntime::open_under_with_models` lives in
  `crates/roko-learn/src/runtime_feedback.rs`. The expensive part of
  `open` is the JSON file IO + `set_episode_completion_hook` distillation
  spawn (see `runtime_feedback.rs` near line 2640 in `run.rs`).
- Today's call graph for `roko run`:

```text
run_once  ──► dispatch_agent  ──► (uses learning indirectly via cascade router)
       ╲
        ╲─► append_episode_log
              └─► LearningRuntime::open_under(workdir)   ← REDUNDANT 2nd open
```

The earlier opens happen inside `dispatch_agent`'s routing path
(`crates/roko-cli/src/orchestrate.rs` opens `LearningRuntime` for the
cascade router). The episode logger then opens its own copy.

> **Note for the next agent.** Plan 01 (config cache) is a prerequisite
> in spirit but not in code. You can do this plan independently — the
> two changes are disjoint. If you do plan 01 first, you'll have a clean
> place (`ConfigBundle`) to attach the runtime as a sibling field if you
> ever need it.

---

## Files to read first

| File | Why |
|---|---|
| `crates/roko-cli/src/run.rs` lines 2520–2665 | `append_episode_log` body — primary edit site. |
| `crates/roko-learn/src/runtime_feedback.rs` | `LearningRuntime::open_under{,_with_models}`, `record_completed_run`, `set_episode_completion_hook`. |
| `crates/roko-cli/src/orchestrate.rs` | Confirm whether the orchestrator already owns a long-lived `LearningRuntime`; the answer drives whether plan 02 only matters for one-shot `run` or for plans too. |
| `crates/roko-cli/src/learning_helpers.rs` | Helper utilities for instantiating runtime in tests. |

---

## Code-level plan

### Step 1 — Lift the open out of `append_episode_log`

Today's signature:

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
    let mut runtime = LearningRuntime::open_under_with_models(...).await?;
    // ... record_completed_run ...
}
```

New signature (and its only caller in `run_once`):

```rust
async fn append_episode_log(
    runtime: &mut LearningRuntime,
    config: &Config,                // OR ConfigBundle if plan 01 is merged
    prompt: &Engram,
    final_output: &Engram,
    verdicts: &[(String, bool)],
    agent_result: &AgentResult,
) -> Result<()> {
    // ... build episode ...
    let mut completed = CompletedRunInput::from_episode(episode);
    completed.provider = Some(infer_provider(config));
    completed.task_metric = Some(build_task_metric(config, prompt, verdicts, agent_result));
    runtime.record_completed_run(completed)
        .await
        .map_err(|e| anyhow!("record learning feedback: {e}"))?;
    Ok(())
}
```

### Step 2 — Open the runtime once at the top of `run_once`

In `run_once` (line ~1092), construct the runtime *before* dispatch so
both routing/orchestrator decisions and the episode log share it:

```rust
let learn_root = workdir.join(".roko").join("learn");
let model_keys = load_roko_config_models(workdir);
let mut learning = if model_keys.is_empty() {
    LearningRuntime::open_under(&learn_root).await
} else {
    LearningRuntime::open_under_with_models(&learn_root, model_keys.clone()).await
}.map_err(|e| anyhow!("open learning runtime: {e}"))?;

// Hook the same distillation callback in one place.
let distillation_workdir = workdir.to_path_buf();
let distillation_caller = distillation_model_caller(workdir);
learning.set_episode_completion_hook(move |episode| {
    roko_neuro::spawn_episode_distillation(
        distillation_workdir.clone(),
        episode,
        Some(Arc::clone(&distillation_caller)),
    );
});
```

Then pass `&mut learning` to `append_episode_log`. If you also wire the
runtime into the dispatch path (orchestrator), pass `&learning` (read
references) into the cascade router observer hooks already in
`dispatch_agent`.

### Step 3 — Audit the orchestrator path

`crates/roko-cli/src/orchestrate.rs` constructs its own
`LearningRuntime` for plan execution (see the `Orchestrator::new` /
`run_plan` call sites). For long-running plans this is fine — it lives
for the plan's lifetime. For `roko run` (which calls into the
orchestrator's dispatch logic), make sure we are not opening twice.

If the orchestrator currently opens its own runtime even when it is
embedded inside `run_once`, accept an optional `Option<&mut
LearningRuntime>` from the caller and use it when present:

```rust
pub async fn dispatch_with_routing(
    &mut self,
    learning: Option<&mut LearningRuntime>,
    ...
) -> Result<...>;
```

> **Reality check.** If `dispatch_agent` lives inside the orchestrator's
> service factory and constructs its own runtime for the cascade router,
> the only single-open you can guarantee from `run_once` is for the
> *episode logger*, not the router. That is still ~70 ms saved. Document
> the remaining double-open as future work; do not try to ship a
> cross-cutting refactor in this plan.

### Step 4 — Drop the `learn_root` plumbing where redundant

`load_roko_config_models(workdir)` is called twice today (once in
dispatch, once in `append_episode_log`). After step 2, only one call
remains. Delete the second one. Use `rg "load_roko_config_models"` to
verify there are no other callers.

---

## Step-by-step execution

1. `git checkout -b perf/02-learning-runtime-single-open`.
2. Change `append_episode_log` signature (Step 1).
3. Open runtime in `run_once` (Step 2). `cargo build -p roko-cli`.
4. Wire `&mut learning` into the only caller of `append_episode_log`.
5. Run `cargo test -p roko-cli --release`.
6. (Optional but recommended) Audit and update orchestrator (Step 3).
7. Add a tracing line `learning_runtime_opened` at the open site so future
   benchmarks can detect regressions:

   ```rust
   tracing::info!(target: "roko_perf", path = %learn_root.display(), "learning_runtime_opened");
   ```

8. Macro-benchmark before/after; record in PR.
9. Open PR `perf(cli): open LearningRuntime once per run (B03)`.

---

## Anti-patterns / things NOT to do

- **Do NOT make `LearningRuntime` a `static` singleton.** It is
  workdir-scoped: tests, `roko serve` (multi-tenant), and concurrent
  CLI invocations all need their own. A static would cross-contaminate
  cascade-router state between tenants.
- **Do NOT convert `record_completed_run` to take `Arc<Mutex<LearningRuntime>>`.**
  Single-owner `&mut` is fine because the call is sequential within a
  run. Adding interior mutability invites lock contention later.
- **Do NOT skip the `set_episode_completion_hook` registration** when
  passing the runtime through. The hook is what spawns distillation;
  losing it silently breaks the neuro-store learning loop with no
  visible error.
- **Do NOT pre-open the runtime before checking `--no-learn` flags.**
  Some users disable learning; opening the runtime then allocates files
  they explicitly opted out of. Gate the open behind the same flag the
  episode logger uses.
- **Do NOT couple this plan to plan 09 (warm pool).** It is tempting to
  pass the runtime through `EffectServices`. Resist; warm pool is its
  own large change. This plan is just a 1 h move.

---

## Test plan

| Level | Test | How |
|---|---|---|
| Unit | `LearningRuntime` is opened once per `run_once` | `tracing-test` capture, count occurrences of `learning_runtime_opened`. |
| Unit | `append_episode_log` writes to the same `episodes.jsonl` as before | Existing tests in `runtime_feedback.rs` + new test that creates a runtime, calls `append_episode_log`, then asserts file has 1 line. |
| Integration | `cargo run --release -p roko-cli -- run --gates none "hi"` produces a single distillation task spawn | Add `tracing::info!` in `spawn_episode_distillation`, count from test logs. |
| Manual | `.roko/learn/cascade-router.json` mtime advances exactly once per run | `stat -f %m .roko/learn/cascade-router.json` before/after. |

---

## Rollback plan

- The change is local to `run.rs` and `append_episode_log`'s signature.
  `git revert` is safe.
- If a downstream caller of `append_episode_log` (there is currently
  only one, but `cargo doc` may surface more) breaks, restore the old
  internal-open codepath behind a feature flag while you migrate.

---

## Status check (acceptance)

- [ ] `LearningRuntime::open_under{,_with_models}` is called exactly
      once in `run_once` (verifiable by grep + log capture).
- [ ] `append_episode_log` accepts `&mut LearningRuntime` and no longer
      opens its own.
- [ ] Tests in `roko-cli` and `roko-learn` pass.
- [ ] Macro-benchmark p50 improvement of ≥50 ms recorded in PR.
- [ ] Distillation hook still registered (verifiable via the manual
      check above).
