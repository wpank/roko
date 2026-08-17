# TEST_22: HAL harness integration stub

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#test-22`](../ISSUE-TRACKER.md#test-22)
- Source: `tmp/solutions/roko/tasks/15-TESTING-VERIFICATION.md` — Task 15.22
- Priority: **P2**
- Effort: 5 hours
- Depends on: `TEST_01` (source 15.1), `TEST_05` (source 15.5), `TEST_06` (source 15.6)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: TEST_22 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

HAL (Holistic Agent Leaderboard) at `hal.cs.princeton.edu` evaluates agents on accuracy, cost, and reliability across 9+ benchmarks. Roko's integration wraps the Rust CLI binary in a format that HAL's Python harness can invoke. This task builds the Rust-side adapter and tests it without requiring the Python harness.

Research from `13-PERF-HAL-AND-AGENT-BENCHMARKS.md` shows:
- HAL evaluates across 4 dimensions: accuracy, cost, reliability, safety
- Key benchmarks: SWE-bench Verified/Pro, USACO, CORE-Bench, GAIA
- HAL's reliability dashboard decomposes into consistency, robustness, predictability, safety
- Internal task replay is the most predictive benchmark for deployment

Research from `13-PERF-HAL-BENCHMARK-INTEGRATION.md` details the Python wrapper pattern: `hal/roko_agent/main.py` with `run(task, **kwargs)` signature, calling `roko run` as a subprocess.

## Exact Changes

1. Define `HalAgentAdapter` trait in `hal_adapter.rs`:
   ```rust
   pub trait HalAgentAdapter {
       fn initialize(&mut self, config: HalConfig) -> Result<()>;
       fn step(&mut self, task: HalTask) -> Result<HalResult>;
       fn cleanup(&mut self) -> Result<HalMetrics>;
   }
   ```
2. Define data types:
   - `HalConfig`: model, workflow, gates, timeout
   - `HalTask`: instance_id, prompt, repo (optional), base_commit (optional)
   - `HalResult`: model_patch (diff), cost_usd, tokens, duration_s, exit_code
   - `HalMetrics`: total_cost, total_tokens, total_duration, tasks_passed, tasks_failed
3. Implement `RokoHalAdapter` struct that wraps the CLI binary:
   - `initialize()` -- verifies `roko` binary exists, sets up workspace
   - `step()` -- runs `roko run` with the task prompt, captures diff and metrics
   - `cleanup()` -- aggregates metrics from all steps
4. Test `HalAgentAdapter::initialize()` creates workspace and config
5. Test `HalAgentAdapter::step(task)` dispatches to mock agent and returns structured result
6. Test `HalAgentAdapter::cleanup()` collects cost/usage metrics
7. Test HAL-compatible result format: all required fields present
8. Test multi-dimensional scoring structure: correctness, cost, latency fields populated

## Design Guidance

The `HalAgentAdapter` trait is the extensibility point. Different adapter implementations can wrap:
- The CLI binary (for external HAL harness integration)
- The Rust API directly (for in-process benchmarking)
- A mock (for testing)

The Python wrapper (`hal/roko_agent/main.py` from the research doc) will call the CLI adapter. This task only builds the Rust types and mock-backed tests.

## Write Scope

- `crates/roko-cli/src/lib.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/15-TESTING-VERIFICATION.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `HalAgentAdapter` trait and types compile
- [ ] `RokoHalAdapter` passes mock-backed tests
- [ ] Result format matches HAL's expected schema (instance_id, model_patch, cost)
- [ ] Cost and latency are populated even in mock mode

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: TEST_22 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `HalAgentAdapter` trait and types compile
- `RokoHalAdapter` passes mock-backed tests
- Result format matches HAL's expected schema (instance_id, model_patch, cost)
- Cost and latency are populated even in mock mode
- No files outside the Write Scope are modified.
- Commit message contains `tracker: TEST_22 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
