# PRD Cascade Learning

**Priority**: P2 — learning gap, model routing does not learn from PRD operations
**Size**: S (half day)
**Crate**: `crates/roko-cli/src/commands/prd.rs`

---

## Problem

PRD commands (`roko prd draft`, `roko prd plan`, `roko research`) call
`persist_capture_episode()` which writes to `.roko/learn/efficiency.jsonl` and
`provider-health.json`, but do NOT load or update `.roko/learn/cascade-router.json`.
This means the cascade router (which does learned model routing via LinUCB) never learns
from PRD operations.

The visible symptom: users see models as "(unavailable)" in the dashboard after
successful PRD runs because the router has 0 observations for those models. The runner
event loop (`crates/roko-cli/src/runner/event_loop.rs`) correctly loads `LearningSubsystem`
and updates the cascade router on every agent call, but the PRD command paths skip this
entirely.

---

## Section A: Current State

**A1.** `crates/roko-cli/src/commands/prd.rs` has three `persist_capture_episode()` call
sites (approximately lines 626, 734, and 845). Each records the episode and updates
provider health, but none touch the cascade router.

**A2.** The runner event loop loads `LearningSubsystem` which includes the cascade router.
After each agent dispatch, it records an observation (model name, success/failure,
latency) and saves the updated router state to `.roko/learn/cascade-router.json`.

**A3.** The cascade router uses LinUCB (a contextual bandit algorithm) to learn which
models perform best for different task types. Without observations from PRD operations,
it has a blind spot covering all research and drafting workloads.

---

## Section B: What To Do

**B1.** At each `persist_capture_episode()` call site in `prd.rs`, add logic to also
load the cascade router from `.roko/learn/cascade-router.json`, record the observation
(model used, success/failure, latency from the episode), and save it back.

**B2.** Look at how the runner event loop does this — search for `cascade_router` or
`CascadeRouter` in `crates/roko-cli/src/runner/` to find the exact API. The goal is to
call the same recording method.

**B3.** The observation should include:
- The model identifier (from the provider response or config)
- Whether the call succeeded or failed
- The latency (wall-clock duration of the LLM call)

**B4.** Consider extracting a small helper function (e.g.,
`record_cascade_observation(workspace, model, outcome, latency)`) that both the runner
and PRD commands can call, to avoid duplicating the load/record/save logic.

---

## Acceptance criteria

- [ ] `roko prd draft new "test-topic"` records an observation in the cascade router
- [ ] `roko prd plan <slug>` records an observation in the cascade router
- [ ] `roko research` subcommands record observations in the cascade router
- [ ] After a successful PRD run, `roko learn all` shows >0 observations for the model used
- [ ] Cascade router file (`.roko/learn/cascade-router.json`) is updated with correct model, outcome, and latency
- [ ] Existing `cargo test -p roko-cli` passes with no regressions

### Not in scope
- Changing the cascade router algorithm (LinUCB) itself
- Adding cascade learning to other non-runner command paths (e.g., `roko chat`)
- Changing how the runner event loop records observations
