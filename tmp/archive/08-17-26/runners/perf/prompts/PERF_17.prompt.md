# PERF_17: Plan executor parallel dispatch (Feature A)

## Task

Add **provider-aware concurrency caps** for multi-task plan execution so
many ready tasks hitting the *same* API provider do not exceed sensible
parallelism, while still respecting the existing executor-wide
`max_concurrent_tasks` ceiling. Implement **Feature A only** from plan
15; do **not** implement `--batch-async` / `BatchProvider` (Feature B).

## Tracker & sources

- Issue tracker row: [ISSUE-TRACKER.md#perf_17](../ISSUE-TRACKER.md#perf_17)
- Plan: `tmp/solutions/perf/implementation/15-batch-inference.md` (§Feature A)
- Performance contract: **C-16**
- Priority: P4
- Effort: ≈8 h
- Depends on: none
- Wave: 2–3

## Problem

`Orchestrator::handle_implementing_parallel` (see
`crates/roko-cli/src/orchestrate.rs`) already launches multiple
`run_prepared_agent` futures under a single **global**
`concurrency_limit = self.executor.config().max_concurrent_tasks`. That
does not distinguish providers: ten tasks on Anthropic can still launch
ten concurrent HTTP sessions if `max_concurrent_tasks ≥ 10`, which can
trigger 429s and backoff storms.

We need a second axis: **per-provider** caps (conservative defaults from
the plan) combined with the global cap (`min(global,
provider_limit)` per slot).

## Exact Changes

### Step 1 — New module `crates/roko-cli/src/dispatch/parallel.rs`

1. `pub struct ConcurrencyPolicy { pub global_max: usize, pub per_provider: HashMap<String, usize> }`
   - `global_max` defaults to **8** (logical default for grouping; the
     orchestrator still passes `executor.config().max_concurrent_tasks`
     as the hard ceiling — document that the **effective** cap per wave
     is `min(global_max, executor_limit, provider_cap)`).
2. `pub fn default_provider_limits() -> HashMap<String, usize>` with
   the exact keys and values from the plan:
   - `openai` → 10, `anthropic` → 5, `gemini` → 5, `cerebras` → 4,
     `moonshot` → 3, `ollama` → 2.
   - Keys must be **lowercase** provider ids as returned by
     `crate::dispatch::resolve_agent_runtime` (`provider_id` field on
     `ResolvedAgentRuntime::Bridge`) or a stable fallback string (e.g.
     `"cli"`) when the runtime is `Cli` without a resolved provider id.
3. `impl ConcurrencyPolicy { pub fn limit_for(&self, provider: &str) -> usize }`
   — lowercases `provider` before lookup; missing key → `self.global_max`.
4. `#[derive(Debug, Clone)] pub struct DispatchSlot { pub task_id: String, pub provider_key: String, pub model_slug: String, /* carry whatever minimal fields tests need */ }`
   — enough to group and to drive a **mock** dispatch in unit tests.
5. `pub fn group_by_provider_model(slots: &[DispatchSlot]) -> HashMap<(String, String), Vec<usize>>`
   — map `(provider_key, model_slug)` → indices (or owned ids); keep it
   deterministic (sort keys for stable tests).
6. Export a documented helper for **bounded concurrent futures** (e.g.
   `tokio::sync::Semaphore` with `acquire_owned` per task, or `JoinSet`
   plus permits) so both the unit test and `handle_implementing_parallel`
   share one correct pattern — avoid copy-pasting semaphore logic twice.

7. **Unit test** `dispatch_group_respects_concurrency_cap`: 20 mock
   tasks, cap **4**, assert `max_observed_in_flight ≤ 4` using atomics +
   small `tokio::time::sleep` inside each mock task.

### Step 2 — Config toggle `[conductor.plan.parallel]`

In `crates/roko-core/src/config/schema.rs`, extend the conductor section
so TOML can contain:

```toml
[conductor.plan.parallel]
enabled = true
```

Serde shape (adjust names to match existing style, but preserve the
**table path** `conductor.plan.parallel`):

- Add nested structs under `ConductorConfig` (e.g. `plan: ConductorPlanConfig` with `parallel: PlanParallelDispatchConfig`).
- `PlanParallelDispatchConfig { #[serde(default = "default_true")] pub enabled: bool }` — **default `true`** so behaviour improves out of the box; when `false`, restore the **previous** purely global-limited loop (single semaphore dimension only).

Wire env override if other conductor fields use env (optional; do not
break existing `ROKO_CONDUCTOR_*` patterns).

### Step 3 — Wire `handle_implementing_parallel`

In `crates/roko-cli/src/orchestrate.rs`:

1. After building `configs: Vec<(String, String, String, AgentRunConfig)>`, derive `provider_key` for each row:
   - Load or reuse `Arc<RokoConfig>` / `RokoConfig` already available on
     `Orchestrator` (if only `&RokoConfig` exists in scope, pass it in).
   - Call `crate::dispatch::resolve_agent_runtime(Some(&arc_cfg), &cfg.model)`; map to lowercase `provider_id` or `"cli"`.
2. If `self.config.conductor.plan.parallel.enabled` (path per your serde) **is false**, keep today's logic unchanged.
3. If **true**:
   - Compute `effective_global = min(executor.config().max_concurrent_tasks, policy.global_max)` (or document that `global_max` in `ConcurrencyPolicy` is replaced entirely by executor config — pick **one** coherent story in code comments).
   - For each wave of the inner loop, partition pending configs by `(provider_key, model)`; for each group, cap =
     `min(effective_global, policy.limit_for(&provider_key))`.
   - Use **one `JoinSet` per wave** (or a single `JoinSet` with permits)
     so that **no more than** `cap` tasks for that provider/model run
     concurrently, and **no more than** `effective_global` total across
     all groups in the wave.

**Critical:** preserve existing semantics:

- Per-task worktrees and `ParallelTaskResult` collection unchanged.
- Failures in one task do not cancel siblings.
- Ordering of **results** may be non-deterministic across tasks (already
  true with `JoinSet`); completion bookkeeping must still call the same
  `record_task_*` / tracker update paths as today.

### Step 4 — `dispatch/mod.rs`

Add `pub mod parallel;` and re-export the types the orchestrator needs
(`ConcurrencyPolicy`, `default_provider_limits`, …) if that matches
crate style.

## Write Scope

- `crates/roko-cli/src/dispatch/parallel.rs` (**new**)
- `crates/roko-cli/src/dispatch/mod.rs`
- `crates/roko-cli/src/orchestrate.rs` (`handle_implementing_parallel` + config read)
- `crates/roko-core/src/config/schema.rs` (nested `conductor.plan.parallel`)

## Read-Only Context

- `tmp/solutions/perf/implementation/15-batch-inference.md` (Feature B is out of scope)
- `tmp/runners/perf/context-pack/02-ANTI-PATTERNS.md` (AP-ASYNC unbounded spawn)
- `crates/roko-cli/src/dispatch/mod.rs` (`resolve_agent_runtime`)

## Acceptance Criteria

- [ ] `parallel.rs` exists with `ConcurrencyPolicy`, `default_provider_limits`, grouping helper, bounded concurrent dispatch helper.
- [ ] Default provider map covers openai, anthropic, gemini, cerebras, moonshot, ollama with the numeric caps from the plan.
- [ ] `handle_implementing_parallel` uses provider-aware caps when enabled.
- [ ] `[conductor.plan.parallel]` toggles the feature; default **on**.
- [ ] Test `dispatch_group_respects_concurrency_cap` passes.
- [ ] **No** `BatchProvider`, `--batch-async`, or `roko plan collect` in this PR.
- [ ] Commit message trailer: `tracker: PERF_17 done <sha>`.

## Verify

```bash
rg -n 'ConcurrencyPolicy|plan\.parallel|dispatch/parallel' crates/roko-cli crates/roko-core
./target/release/roko --help | rg -n conductor || true   # if CLI exposes nested config docs
```

## Do NOT

- Do NOT spawn an unbounded number of concurrent provider calls (AP-ASYNC).
- Do NOT parallelize **across DAG layers** — only within the existing
  “ready set” wave handled by `handle_implementing_parallel`.
- Do NOT implement Feature B (async batch APIs); defer to a future runner.
- Do NOT reduce `max_concurrent_tasks` semantics silently — document the
  interaction with `global_max` / per-provider caps.
- Do NOT compile or run tests during the batch (see `context-pack/00-RULES.md`).

## Tracker update

```
tracker: PERF_17 done <commit-sha>
```
