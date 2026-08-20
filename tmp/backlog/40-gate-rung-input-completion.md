# 40 — Gate Rung Input Completion

**Priority**: P2 — three gaps cause rungs 5 and 6 to evaluate descriptions rather than implementations; custom rung configurations are silently bypassed
**Size**: M (1-2 days)
**Crates**: `crates/roko-cli/src/runner/gate_dispatch.rs` (primary), `crates/roko-gate/src/`
**Depends on**: None

---

## Background

The gate pipeline has 7 rungs that validate each completed task. Rungs 1-4 (compile, lint, test, symbol) run unconditionally with real inputs. Rungs 5 (FactCheck) and 6 (LlmJudge) have complete implementations in `roko-gate` but receive degraded inputs from the runner, causing them to produce weaker verdicts than they are capable of.

There are three concrete gaps, all in `crates/roko-cli/src/runner/gate_dispatch.rs`.

**Gap 1**: The `LlmJudgeGate` (rung 6) receives a `JudgePayload` with an empty `diff` field. The runner constructs the payload at line 961-964 with `diff: String::new()`. The comment at line 950 says: "The diff is left empty here because we cannot run `git diff` synchronously in this context." This comment is outdated: the same file has a `gate_input_snapshot_blocking` function starting at line 107 that already runs `git diff --binary HEAD --` synchronously (line 126) and returns the result. The async wrapper at line 205 uses `tokio::task::spawn_blocking` for this function. Reusing this pattern before building the `JudgePayload` is straightforward. Without a real diff, the LlmJudgeGate evaluates whether the task description sounds correct, rather than whether the implementation matches the description.

**Gap 2**: The `FactCheckGate` (rung 5) always returns `Skipped` because `RungExecutionConfig.fact_check_oracle` is always `None`. The field is defined at `crates/roko-gate/src/rung_dispatch.rs` line 70 as `pub fact_check_oracle: Option<Arc<dyn SearchOracle>>`. The `SearchOracle` trait (defined in `crates/roko-gate/src/fact_check.rs` line 57) has a single async method `search(query: &str) -> Result<Vec<SearchHit>>`. The Perplexity HTTP client in `crates/roko-agent/src/perplexity/` provides exactly this capability. No adapter struct implementing `SearchOracle` via `PerplexitySearchClient` exists; one needs to be written and wired into `build_rung_execution_config`.

**Gap 3**: When operators configure custom `[[gates.rungs]]` in `roko.toml`, the runner calls `GatePipelineBuilder::from_config(&gates_config, complexity)` at line 509, which internally passes empty `RungExecutionInputs::default()` and `RungExecutionConfig::default()`. The populated `inputs` (built at line 501) and `config` (built at lines 502-507) are discarded. The correct call is `GatePipelineBuilder::from_config_with_execution(&gates_config, complexity, inputs, config)`, which handles both custom and default rung paths (it routes to `from_custom_config_with_execution` when custom rungs are present, passing the inputs and config). The same bug is in the retry path at lines 564-573.

## Current State

1. `crates/roko-cli/src/runner/gate_dispatch.rs` line 897 — `build_rung_execution_inputs(target_crates, task_ctx)` constructs `RungExecutionInputs`. At line 961-964: `JudgePayload { task_description: ..., diff: String::new() }`. The comment at line 950 explains the gap.

2. `gate_dispatch.rs` line 107 — `gate_input_snapshot_blocking(workdir)` runs `git diff --binary HEAD --` at line 126 via `std::process::Command`. The async wrapper at line 205 is `tokio::task::spawn_blocking(move || gate_input_snapshot_blocking(&workdir))`. This same subprocess capability is not reused by `build_rung_execution_inputs`.

3. `gate_dispatch.rs` line 989 — `build_rung_execution_config(workdir, timeout_secs, verify_steps, verdict_publisher)` returns `RungExecutionConfig { source_roots, timeout_ms, integration_test_pattern, integration_build_system, generated_test_artifacts, verdict_publisher, ..Default::default() }`. The `..Default::default()` leaves `fact_check_oracle`, `fact_check_min_confidence`, `llm_judge_oracle`, and `llm_judge_min_score` as `None`.

4. `gate_dispatch.rs` lines 508-517 — pipeline builder selection:
   ```rust
   let pipeline = if gates_config.has_custom_rungs() {
       GatePipelineBuilder::from_config(&gates_config, complexity)  // discards inputs/config
   } else {
       GatePipelineBuilder::from_config_with_execution(&gates_config, complexity, inputs, config)
   };
   ```
   The same pattern is repeated in the retry path at lines 564-573.

5. `crates/roko-gate/src/rung_dispatch.rs` line 94 — `GatePipelineBuilder::from_config` calls `from_config_with_execution` with `RungExecutionInputs::default()` and `RungExecutionConfig::default()`, discarding any inputs. `from_config_with_execution` at line 105 handles both custom and default rung paths correctly.

6. `crates/roko-gate/src/fact_check.rs` line 57 — `SearchOracle` trait with `async fn search(&self, query: &str) -> Result<Vec<SearchHit>>`. `SearchHit` has `title: String`, `url: String`, `snippet: String`, `score: f64`.

7. `crates/roko-agent/src/perplexity/` — `PerplexitySearchClient` exists (exported from `crates/roko-agent/src/lib.rs` line 160). No struct in `roko-gate` or `roko-cli` adapts it to the `SearchOracle` trait.

8. `crates/roko-gate/src/llm_judge_gate.rs` — `JudgePayload { task_description: String, diff: String }`. When `diff` is empty, the gate falls back to evaluating the description only.

## Implementation Plan

### Change A: Populate `JudgePayload.diff` with async git diff

Change `build_rung_execution_inputs` to accept a `diff_text: Option<String>` parameter. Populate `JudgePayload.diff` from it:

```rust
fn build_rung_execution_inputs(
    target_crates: &[String],
    task_ctx: Option<&GateTaskContext>,
    diff_text: Option<&str>,  // new parameter
) -> RungExecutionInputs {
    // ... existing symbol and fact_check signal construction ...

    let llm_judge_signal = {
        let task_description = ...;
        if task_description.is_empty() {
            None
        } else {
            let payload = JudgePayload {
                task_description: task_description.to_string(),
                diff: diff_text.unwrap_or("").to_string(),  // use real diff
            };
            // ... build signal ...
        }
    };
    // ...
}
```

In the caller (line 501), before calling `build_rung_execution_inputs`, fetch the diff using the existing `gate_input_snapshot_blocking` pattern with a bounded timeout:

```rust
let diff_text: Option<String> = tokio::time::timeout(
    Duration::from_secs(5),
    tokio::task::spawn_blocking({
        let workdir = workdir_for_run.clone();
        move || {
            std::process::Command::new("git")
                .args(["diff", "HEAD", "--", "."])
                .current_dir(&workdir)
                .env("GIT_TERMINAL_PROMPT", "0")
                .output()
                .ok()
                .and_then(|o| if o.status.success() {
                    String::from_utf8(o.stdout).ok()
                } else {
                    None
                })
        }
    }),
)
.await
.ok()
.flatten()
.flatten();

let inputs = build_rung_execution_inputs(&target_crates, task_context.as_ref(), diff_text.as_deref());
```

Apply the same change to the retry path at lines 556-558.

Estimated: ~40 lines. Risk: low (bounded timeout, `None` fallback preserves current behavior).

### Change B: `PerplexitySearchOracle` adapter

Add a new struct in `crates/roko-gate/src/fact_check.rs` (or a new file `crates/roko-gate/src/perplexity_oracle.rs`):

```rust
/// SearchOracle adapter backed by roko-agent's PerplexitySearchClient.
pub struct PerplexitySearchOracle {
    client: Arc<roko_agent::PerplexitySearchClient>,
}

#[async_trait::async_trait]
impl SearchOracle for PerplexitySearchOracle {
    async fn search(&self, query: &str) -> anyhow::Result<Vec<SearchHit>> {
        let results = self.client.search(query, ...).await?;
        Ok(results.into_iter().map(|r| SearchHit {
            title: r.title,
            url: r.url,
            snippet: r.snippet.unwrap_or_default(),
            score: r.score.unwrap_or(0.5),
        }).collect())
    }
}
```

Wire it into `build_rung_execution_config` by accepting an `oracle: Option<Arc<dyn SearchOracle>>` parameter and setting `fact_check_oracle: oracle`.

In the caller (gate_dispatch.rs), construct the oracle when the workspace config has a Perplexity key:

```rust
let fact_check_oracle: Option<Arc<dyn SearchOracle>> = {
    let config = load_roko_config(workdir).ok();
    config
        .as_ref()
        .and_then(|c| c.providers.get("perplexity"))
        .and_then(|p| p.resolve_api_key())
        .map(|key| Arc::new(PerplexitySearchOracle::new(key)) as Arc<dyn SearchOracle>)
};
let config = build_rung_execution_config(
    &workdir_for_run,
    timeout_secs,
    &verify_steps,
    verdict_publisher.clone(),
    fact_check_oracle,
);
```

Add `roko-agent` as a dependency of `roko-gate` if not already present, or place the adapter in `roko-cli` and thread it into `build_rung_execution_config`.

Estimated: ~80 lines. Risk: medium (new cross-crate dependency, requires workspace config at gate-dispatch time).

### Change C: Fix custom-rung pipeline builder call

Replace the `if gates_config.has_custom_rungs()` branch in both the primary path (lines 508-517) and the retry path (lines 564-573) with a single call to `from_config_with_execution`:

```rust
// Before (line 508-517):
let pipeline = if gates_config.has_custom_rungs() {
    GatePipelineBuilder::from_config(&gates_config, complexity)
} else {
    GatePipelineBuilder::from_config_with_execution(&gates_config, complexity, inputs, config)
};

// After:
let pipeline = GatePipelineBuilder::from_config_with_execution(
    &gates_config,
    complexity,
    inputs,
    config,
);
```

`from_config_with_execution` already handles both cases internally (routes to `from_custom_config_with_execution` when `has_custom_rungs()` is true). No behavior change occurs for the non-custom path. Custom-rung plans will now receive populated `inputs` and `config`.

Estimated: ~10 lines. Risk: low.

## Acceptance Criteria

1. After a task attempt completes, the gate log for rung 6 (LlmJudge) contains a non-empty `diff` field in its `JudgePayload` (verified by inspecting gate logs or adding a tracing statement).
2. When `PERPLEXITY_API_KEY` is set in the environment, `build_rung_execution_config` populates `fact_check_oracle` and FactCheck returns a non-`Skipped` verdict.
3. When `PERPLEXITY_API_KEY` is absent, FactCheck continues to return `Skipped` (oracle is `None`).
4. When `[[gates.rungs]]` is configured in `roko.toml`, the runner calls `GatePipelineBuilder::from_config_with_execution` (not `from_config`), confirmed by `grep -n 'GatePipelineBuilder::from_config\b' crates/roko-cli/src/runner/gate_dispatch.rs` returning zero results.
5. `cargo test --workspace` passes with zero failures.
6. `cargo clippy --workspace --no-deps -- -D warnings` is clean.

## Verification Checklist

- [ ] `grep -n 'diff: String::new()' crates/roko-cli/src/runner/gate_dispatch.rs` returns zero results
- [ ] `grep -n 'GatePipelineBuilder::from_config\b' crates/roko-cli/src/runner/gate_dispatch.rs` returns zero results (all calls are `from_config_with_execution`)
- [ ] Run a plan task; inspect `.roko/state/` or add `tracing::info!` to confirm `JudgePayload.diff` is non-empty
- [ ] `PERPLEXITY_API_KEY=<key> cargo run -p roko-cli -- plan run plans/ --engine runner-v2` — FactCheck rung shows `pass` or `fail` instead of `skipped`
- [ ] `cargo test --workspace 2>&1 | tail -5` shows all tests passed
- [ ] `cargo clippy --workspace --no-deps -- -D warnings 2>&1 | grep error` is empty

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/gate_dispatch.rs` | Add diff_text parameter to `build_rung_execution_inputs`; fetch diff via `spawn_blocking` before calling it; add oracle parameter to `build_rung_execution_config`; replace `from_config` calls with `from_config_with_execution` (lines 508-517 and 564-573) |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/fact_check.rs` (or new `perplexity_oracle.rs`) | Add `PerplexitySearchOracle` struct implementing `SearchOracle` |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/Cargo.toml` | Add `roko-agent` dependency (optional, or place adapter in roko-cli) |
