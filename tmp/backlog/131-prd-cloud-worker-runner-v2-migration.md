# 131 — PRD/Cloud-Worker Migration to Runner-v2

**Priority**: P1 — `prd.rs::run_generated_plans()` and `worker/cloud.rs` both use the old `PlanRunner` which has an acknowledged memory leak and bypasses runner-v2 safety, learning, and gate wiring; these are frequently-exercised paths.
**Size**: S (1-2 days)
**Crates**: `crates/roko-cli/src/prd.rs`, `crates/roko-cli/src/serve_runtime.rs`
**Depends on**: None (runner-v2 API already exists)
**Sources**: `tmp/backlog/_mori-diffs-gaps.md` §A-1 (suggested 111)

---

## Background

Runner-v2 is the canonical execution engine, wiring safety, learning (efficiency events, episodes, gate thresholds), and the full gate pipeline through `event_loop.rs`. The old `PlanRunner` (the legacy runtime) has known problems: an unbounded `efficiency_events: Vec<...>` that grows without bound (memory leak on long runs) and missing hooks for safety checks, learning, and adaptive gate thresholds.

Two code paths still call `PlanRunner::from_plans_dir` instead of runner-v2:
1. `prd.rs::run_generated_plans()` — called when `prd.auto_plan = true` and a PRD is published; this triggers automatic plan execution from a freshly generated plan.
2. `worker/cloud.rs` — called when roko runs as a cloud worker (deployed mode); cloud execution should have the same reliability and learning wiring as local execution.

Both paths are frequently exercised in the self-hosting workflow: every `roko prd plan <slug>` followed by a publication triggers `run_generated_plans`. Fixing these two call sites eliminates the legacy runtime from all production-facing paths.

## Current State

- `crates/roko-cli/src/prd.rs` — calls `PlanRunner::from_plans_dir(...)`. Exact line needs inspection.
- `crates/roko-cli/src/serve_runtime.rs` — contains cloud worker logic; calls `PlanRunner`.
- `crates/roko-cli/src/runner/event_loop.rs` — runner-v2 entry point. API: `RunnerV2::new(config).run(plans_dir)`.
- `PlanRunner` — old runtime; present in `crates/roko-cli/src/runner/` (or `crates/roko-cli/src/`).
- No CI guard exists blocking new `PlanRunner::from_plans_dir` call sites.

## Implementation Plan

1. **Audit `prd.rs` call site**: Read `prd.rs` and identify `PlanRunner::from_plans_dir` usage. Note the config parameters passed (plans directory, max agents, etc.).

2. **Replace in `prd.rs`**: Construct a `RunConfig` from the same parameters used by the legacy call and invoke `runner_v2::run_plans(run_config, plans_dir).await`. Ensure the auto-plan path properly awaits runner-v2 completion and propagates errors.

3. **Audit `serve_runtime.rs` / cloud worker call site**: Identify the `PlanRunner` call and its configuration.

4. **Replace in cloud worker**: Same substitution as step 2. The cloud worker must use runner-v2 for safety guarantees (the cloud environment is untrusted and needs the full safety layer).

5. **Add CI grep guard**: In the CI pipeline or as a pre-commit hook, add:
   ```bash
   # block new call sites for the legacy PlanRunner::from_plans_dir
   if grep -rn 'PlanRunner::from_plans_dir' crates/ --include='*.rs' | grep -v 'target/'; then
       echo "ERROR: new PlanRunner::from_plans_dir call site detected. Use runner-v2."
       exit 1
   fi
   ```
   Add this as a `cargo test` integration test or a shell script in `tests/`.

6. **Verify memory behaviour**: After the migration, run a plan and confirm that efficiency events do not accumulate without bound (runner-v2 flushes them periodically, unlike the old `Vec`).

## Acceptance Criteria

1. `prd.rs::run_generated_plans()` invokes runner-v2, not `PlanRunner`.
2. Cloud worker execution invokes runner-v2.
3. A CI check fails if any new `PlanRunner::from_plans_dir` call site is added.
4. A plan triggered by `roko prd plan <slug>` followed by publication runs through runner-v2 and produces episodes in `.roko/episodes.jsonl`.
5. Memory usage during a PRD-triggered run does not grow unboundedly.

## Verification Checklist

- [ ] After migration, `grep -rn 'PlanRunner::from_plans_dir' crates/` returns only the legacy `PlanRunner` definition itself (no call sites).
- [ ] Run `roko prd plan <slug>` + publish; verify `.roko/episodes.jsonl` has new entries from runner-v2 (not the legacy runner).
- [ ] Run the CI grep guard script; verify it passes after migration.
- [ ] Run the CI guard with a deliberate regression (add a fake call site); verify it fails.

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-cli/src/prd.rs` | Replace `PlanRunner::from_plans_dir` with runner-v2 invocation |
| `crates/roko-cli/src/serve_runtime.rs` | Replace `PlanRunner::from_plans_dir` in cloud worker path |
| `tests/` or `.github/workflows/` | Add CI guard script for `PlanRunner::from_plans_dir` |
