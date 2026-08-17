# 20 — Orchestrate.rs Extraction (T5-35 expanded)

`crates/roko-cli/src/orchestrate.rs` is **22,756 lines** as of 2026-05-01.
This file owns most of the runner v2 dispatch surface. The biggest
function in it, `dispatch_agent_with`, is ~2,059 lines.

This plan describes how to extract `dispatch_agent_with` into focused
modules without changing behavior. The mechanical move comes first;
behavior changes (if any) come in separate PRs.

This is T5-35. It's the largest single architectural item in the audit.

---

## Why This Matters

Every recent runner batch added 50-200 lines to `dispatch_agent_with`.
Each addition is locally reasonable; together they make the function
unreviewable. Symptoms:

- Adding a new provider branch requires reading >2K lines to find the
  right insertion point.
- Bug fixes near one branch silently break neighboring branches.
- Tests of `dispatch_agent_with` are integration tests by necessity —
  there's no smaller unit to test.
- `cargo build -p roko-cli` recompiles the file on every change.

The fix is mechanical: split the function along its natural seams (model
selection, prompt assembly, agent launch, outcome recording) into focused
modules, each with its own request/response struct and unit tests.

---

## Target Module Layout

```
crates/roko-cli/src/orchestrate/
├── mod.rs                          // re-exports, the dispatch entry point
├── dispatch/
│   ├── mod.rs                      // pub use of the four units
│   ├── select_model.rs             // ~335 lines, slice 1
│   ├── build_prompt.rs             // ~350 lines, slice 2
│   ├── launch_agent.rs             // ~330 lines, slice 3
│   └── record_outcome.rs           // ~295 lines, slice 4
└── (future: gates/, telemetry/, etc. as more extraction lands)
```

After all four slices, `crates/roko-cli/src/orchestrate.rs` retains:

- The `dispatch_agent_with` skeleton (~80 lines): a sequence of typed
  calls.
- The other ~20K lines covering plan selection, runner event loop, retry
  policy, etc. (Out of scope for this plan; future extractions go in
  separate plans.)

---

## Anti-Patterns Specific To This Extraction

1. **Don't add `pub` to internal helpers during the move.** If a helper
   is pulled into the new module and not needed externally, keep it
   `fn` (private) or `pub(super)`.
2. **Don't change function signatures of helpers called by moved code.**
   If a helper at `orchestrate.rs:8430` is called from inside the moved
   block, the new module imports it; the helper itself stays put.
3. **Don't introduce `async fn` where the original was sync, or vice
   versa.** Mechanical move, not refactor.
4. **Don't merge two slices** even if they share a helper. The helper
   either stays in `orchestrate.rs` (if used by both slices) or gets
   extracted into `orchestrate/dispatch/common.rs` in a fifth commit.
5. **Don't add new error variants in this PR.** If `dispatch_agent_with`
   returns `anyhow::Error`, the new module returns `anyhow::Error`. A
   future PR can introduce a typed `DispatchError`.
6. **Don't drop logs.** Every `tracing::info!` / `warn!` / `error!` in
   the moved block stays in the new module verbatim.
7. **Don't change variable names** in the moved code. `let cfg =` stays
   `let cfg =`. Changes go in a follow-up "rename for clarity" commit.

---

## Pre-Work: Recompute Line Ranges

Line ranges below are accurate as of 2026-05-01 but earlier landed work
shifted offsets a few times. Before starting, recompute:

```bash
rg -n 'fn dispatch_agent_with' crates/roko-cli/src/orchestrate.rs
# Note the line number; e.g. 14575

awk 'NR > 14575 && /^    }$/ { print NR; exit }' crates/roko-cli/src/orchestrate.rs
# Find the closing brace of the function
```

Subtract to get the function's line span. Then identify the four natural
seams within the function by reading.

---

## Slice 1: `select_model`

### Scope

Lines covering model + provider resolution. Roughly:

- Reading `cfg.agent.default_model`, CLI override, runner override.
- Calling `CascadeRouter::pick(...)` if eligible.
- Resolving the picked slug into a `ResolvedModel` (provider, transport,
  capabilities).
- Computing `ModelChoiceSource::{Default, Override, Router}`.

### New module: `crates/roko-cli/src/orchestrate/dispatch/select_model.rs`

```rust
//! Model + provider resolution for dispatch_agent_with.
//!
//! Extracted from `orchestrate.rs` in T5-35a. Behavior is identical to
//! the inline version; only structure changed.

use std::sync::Arc;

use roko_core::config::{RokoConfig, ResolvedModel, ResolvedProvider};
use roko_learn::cascade_router::CascadeRouter;

use crate::dispatch::ModelChoiceSource;
use crate::runner::Task;

#[derive(Debug)]
pub(crate) struct SelectModelReq<'a> {
    pub task: &'a Task,
    pub config: &'a RokoConfig,
    pub router: &'a Arc<CascadeRouter>,
    pub cli_model: Option<&'a str>,
    pub force_backend: Option<&'a str>,
    pub budget_pressure: f32,
    pub task_complexity: TaskComplexity, // existing enum
}

#[derive(Debug)]
pub(crate) struct SelectModelRes {
    pub model: ResolvedModel,
    pub provider: ResolvedProvider,
    pub source: ModelChoiceSource,
}

pub(crate) async fn select_model(
    req: SelectModelReq<'_>,
) -> anyhow::Result<SelectModelRes> {
    // ~335 lines moved verbatim from dispatch_agent_with.
    // Replace local references with req.<field>.
}
```

### Move sequence (3 commits per slice)

**Commit 1 — Add module:**

```bash
git checkout -b t5-35a-select-model
```

1. Create `crates/roko-cli/src/orchestrate/` if it doesn't exist (likely
   `mod.rs` already exists; if not, add `pub mod orchestrate;` to lib).
2. Create `crates/roko-cli/src/orchestrate/dispatch/mod.rs`:
   ```rust
   pub(crate) mod select_model;
   ```
3. Create `crates/roko-cli/src/orchestrate/dispatch/select_model.rs` with
   the module skeleton above. Copy the relevant ~335 lines of body.
4. Convert `let var = ...` references to `req.var = ...` style for
   inputs.
5. Wrap outputs in `SelectModelRes { ... }`.
6. Add a unit test:
   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;
       #[tokio::test]
       async fn picks_cli_override_first() {
           let task = make_task();
           let cfg = make_config();
           let router = Arc::new(CascadeRouter::new(vec!["claude-sonnet-4-6".into()]));
           let res = select_model(SelectModelReq {
               task: &task,
               config: &cfg,
               router: &router,
               cli_model: Some("opus"),
               force_backend: None,
               budget_pressure: 0.0,
               task_complexity: TaskComplexity::Small,
           }).await.unwrap();
           assert_eq!(res.source, ModelChoiceSource::Override);
           assert!(res.model.slug.starts_with("opus"));
       }
   }
   ```
7. **The original `dispatch_agent_with` is untouched at this point.** The
   new module compiles but is dead.
8. `cargo test --workspace`. Must pass.
9. Commit: `T5-35a-step1: Add select_model module (unused)`.

**Commit 2 — Switch to delegation:**

1. In `dispatch_agent_with`, replace the ~335-line block with:
   ```rust
   let select = orchestrate::dispatch::select_model::select_model(
       SelectModelReq {
           task: &task,
           config: &cfg,
           router: &router,
           cli_model: cli_model.as_deref(),
           force_backend: force_backend.as_deref(),
           budget_pressure,
           task_complexity,
       },
   ).await?;
   let model = select.model;
   let provider = select.provider;
   let model_source = select.source;
   ```
2. The block lines are now duplicated (in `dispatch_agent_with` and in
   the new module). That's intentional — confidence-building.

   Actually, **don't** duplicate. Replace the block in
   `dispatch_agent_with` with the delegation. Step 1 is the move; step 2
   is the call site update. Step 3 is verification.

   Combine: in this commit, **delete** the inline block and replace it
   with the delegation. The new module is now in the product path.
3. `cargo test --workspace`. Must pass.
4. Commit: `T5-35a-step2: Delegate model selection in dispatch_agent_with`.

**Commit 3 — (no-op; reserved for follow-up cleanup):**

This step exists for symmetry across all four slices but has no work for
slice 1. Skip if no cleanup is needed.

### Verify slice 1

```bash
wc -l crates/roko-cli/src/orchestrate.rs               # ~22,420 (down ~335)
wc -l crates/roko-cli/src/orchestrate/dispatch/select_model.rs   # ~370 (body + tests)
cargo test --workspace
cargo clippy --workspace --no-deps -- -D warnings

# Confirm the only remaining reference to the old block's helpers in the
# old location is the one inside the new module.
rg 'pick_model_slug|resolve_provider_for_slug' crates/roko-cli/src/
```

---

## Slice 2: `build_prompt`

### Scope

Lines after model selection, before agent launch:

- Constructing the `SystemPromptBuilder` with the 9 layers.
- Loading playbook entries (after T4-32) and HDC similarity (after plan
  30).
- Applying safety contract restrictions to the prompt.
- Producing the final `String` prompt and the `MessageHistory`.

### New module: `crates/roko-cli/src/orchestrate/dispatch/build_prompt.rs`

Same skeleton as slice 1. Inputs include the result from
`select_model` (the resolved model + provider). Outputs:

```rust
pub(crate) struct BuildPromptRes {
    pub system_prompt: String,
    pub user_message: String,
    pub history: Vec<Message>,
    pub safety_overlay: SafetyOverlay,
}
```

### Move sequence

Same three-commit pattern. Commit names:

- `T5-35b-step1: Add build_prompt module (unused)`
- `T5-35b-step2: Delegate prompt assembly in dispatch_agent_with`

### Special considerations

- **Safety contract**: the prompt builder may inject role-specific
  restrictions ("You may not use bash"). Confirm the inline code does
  this; preserve it in the new module.
- **Playbook layer**: T4-32 wires playbooks into the prompt. If T5-35b
  lands before T4-32, leave the playbook layer empty and add a TODO; if
  after, the layer is already populated.
- **HDC similarity**: plan 30 re-enables this. Same logic: leave hooks
  in place; let the future work fill them.

---

## Slice 3: `launch_agent`

### Scope

Lines covering:

- Constructing the agent runtime (`AgentDispatcher` /
  `OllamaToolLoop` / etc.).
- Spawning the model call (`ModelCallService::call` /
  `ModelCallService::stream`).
- Streaming events through the runner's event channel.
- Collecting the final response.

This is the most "external" of the slices — it's where actual provider
interaction happens. After T5-36, all dispatch goes through
`ModelCallService` and this slice doesn't see provider HTTP directly.

### New module: `crates/roko-cli/src/orchestrate/dispatch/launch_agent.rs`

```rust
pub(crate) struct LaunchAgentReq<'a> {
    pub model: &'a ResolvedModel,
    pub provider: &'a ResolvedProvider,
    pub system_prompt: &'a str,
    pub user_message: &'a str,
    pub history: &'a [Message],
    pub safety_overlay: &'a SafetyOverlay,
    pub model_call_service: Arc<dyn ModelCallService>,
    pub event_tx: tokio::sync::mpsc::Sender<RunnerEvent>,
    pub cancel: CancellationToken,
}

pub(crate) struct LaunchAgentRes {
    pub response_text: String,
    pub usage: Option<UsageObservation>,
    pub finish_reason: FinishReason,
    pub tool_calls: Vec<ToolCall>,
}
```

### Move sequence

Same three-commit pattern. Commit names:

- `T5-35c-step1: Add launch_agent module (unused)`
- `T5-35c-step2: Delegate agent launch in dispatch_agent_with`

### Special considerations

- **Streaming**: this code path uses `ModelCallService::stream`. The
  stream events flow through `event_tx` to the runner. Preserve the
  exact channel semantics; don't introduce a new buffering layer.
- **Cancellation**: `CancellationToken` propagates from the runner. The
  new module must respect it; check `cancel.is_cancelled()` at every
  await point.
- **Ollama**: T5-39 adds budget guardrail. If T5-35c lands first, the
  guardrail goes around the call inside `launch_agent`; if T5-39 first,
  it's already in place when this slice extracts.

---

## Slice 4: `record_outcome`

### Scope

Lines covering:

- Constructing `AgentOutcome` from the launch result.
- Emitting `RunnerEvent::TaskAttemptCompleted`.
- Recording an episode via `EpisodeSink`.
- Updating the cascade router via `RoutingObservationSink`.
- Writing a knowledge candidate via `KnowledgeIngestionSink`.
- Adding a `RunLedger::Entry::Dispatch` (after plan 24).

### New module: `crates/roko-cli/src/orchestrate/dispatch/record_outcome.rs`

```rust
pub(crate) struct RecordOutcomeReq<'a> {
    pub task: &'a Task,
    pub model: &'a ResolvedModel,
    pub provider: &'a ResolvedProvider,
    pub model_source: ModelChoiceSource,
    pub launch_res: &'a LaunchAgentRes,
    pub routing_context: Option<&'a RoutingContext>,
    pub event_tx: tokio::sync::mpsc::Sender<RunnerEvent>,
    pub run_ledger: &'a Arc<RunLedger>,   // when plan 24 lands
}

pub(crate) struct RecordOutcomeRes {
    pub outcome: AgentOutcome,
    pub succeeded: bool,
}
```

### Move sequence

Same three-commit pattern.

### Special considerations

- **Routing context**: depends on T4-30. Pass `Option<&RoutingContext>`;
  the sink will fall back to confidence-only if absent.
- **Run ledger entry**: plan 24 may have already added a typed
  `RunLedger::Entry::Dispatch` write. Preserve it.
- **Don't fan out via FeedbackFacade in this module**. The
  `RunnerEvent::TaskAttemptCompleted` is the seam; the facade picks
  it up downstream.

---

## After All Four Slices

`dispatch_agent_with` becomes:

```rust
async fn dispatch_agent_with(
    &self,
    /* parameters */
) -> anyhow::Result<AgentOutcome> {
    use orchestrate::dispatch::{select_model, build_prompt, launch_agent, record_outcome};

    let select = select_model::select_model(SelectModelReq { /* ... */ }).await?;

    let prompt = build_prompt::build_prompt(BuildPromptReq {
        model: &select.model,
        /* ... */
    }).await?;

    let launch = launch_agent::launch_agent(LaunchAgentReq {
        model: &select.model,
        provider: &select.provider,
        system_prompt: &prompt.system_prompt,
        user_message: &prompt.user_message,
        history: &prompt.history,
        safety_overlay: &prompt.safety_overlay,
        model_call_service: self.model_call_service.clone(),
        event_tx: self.event_tx.clone(),
        cancel: self.cancel.clone(),
    }).await?;

    let outcome = record_outcome::record_outcome(RecordOutcomeReq {
        task: &task,
        model: &select.model,
        provider: &select.provider,
        model_source: select.source,
        launch_res: &launch,
        routing_context: routing_context.as_ref(),
        event_tx: self.event_tx.clone(),
        run_ledger: &self.run_ledger,
    }).await?;

    Ok(outcome.outcome)
}
```

~50-80 lines instead of ~2,059.

### Final verification

```bash
wc -l crates/roko-cli/src/orchestrate.rs
# ~21,000 (down from 22,756)

ls crates/roko-cli/src/orchestrate/dispatch/
# mod.rs, select_model.rs, build_prompt.rs, launch_agent.rs, record_outcome.rs

cargo test --workspace
cargo clippy --workspace --no-deps -- -D warnings

# Each new module has unit tests
for module in select_model build_prompt launch_agent record_outcome; do
    cargo test -p roko-cli orchestrate::dispatch::$module --lib
done
```

---

## Future Extraction (Out of Scope for T5-35)

After this plan lands, the same pattern can be applied to other large
sections of `orchestrate.rs`:

- Plan selection logic (~1500 lines)
- Runner event loop dispatch (~800 lines)
- Retry policy + backoff (~400 lines)
- Adaptive threshold gate observation (~300 lines)
- Workflow report construction (folded into plan 24)

Each becomes a future T5-35-style task.

---

## Status

- [ ] T5-35a — Extract `select_model` (3 commits)
- [ ] T5-35b — Extract `build_prompt` (3 commits)
- [ ] T5-35c — Extract `launch_agent` (3 commits)
- [ ] T5-35d — Extract `record_outcome` (3 commits)

**Each slice is independent**: 35a doesn't have to finish before 35b
starts (different agents can do different slices in parallel). They
share only the `orchestrate/dispatch/mod.rs` file; merge conflicts
there are trivial (just adding `pub(crate) mod <name>;` lines).
