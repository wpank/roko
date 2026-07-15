# 15 -- Plan Audit: E43-E48

> Quality and schema compliance audit for the 6 newest epic plans (E43-E48).
> Includes the 3 newly authored plans (E46, E47, E48) and 3 existing plans (E43, E44, E45).
> Audited: 2026-07-10

## Summary

- **Plans audited**: 6
- **Total tasks**: 61 (E43: 8, E44: 8, E45: 10, E46: 12, E47: 11, E48: 12)
- **Schema compliance**: 61/61 tasks pass
- **File path validity**: 60/61 pass (1 minor issue)
- **Verify command quality**: 61/61 pass
- **Dependency graph integrity**: 61/61 pass (no cycles, all refs resolve)
- **Issues found**: 2 minor, 0 critical

## Per-Plan Results

### E43-deployment-portability (8 tasks)

| Check | Result |
|---|---|
| `[meta]` section | PASS -- plan, total, done, status, max_parallel |
| `[[task]]` format | PASS -- all 8 use `[[task]]` (not `[[tasks]]`) |
| Required fields | PASS -- id, title, description, role, status, tier, files, depends_on |
| Extra fields | max_loc on all tasks |
| `[task.context]` blocks | PASS -- read_files with path/lines/why, symbols, anti_patterns |
| File paths (existing) | PASS -- knowledge_store.rs, daemon.rs, commands/server.rs, status.rs, commands/util.rs all exist |
| File paths (new files) | Dockerfile, railway.toml -- correctly listed as new-file targets |
| Verify commands | PASS -- structural (grep), compile (cargo check), test (cargo test) |
| Dependency graph | PASS -- T01->T02, T03->T04, T03->T05, no cycles |
| Duplicate IDs | PASS -- E43-T01 through E43-T08, all unique |
| Tier values | PASS -- all `focused` |
| Role values | PASS -- all `implementer` |

**Issues**: None

---

### E44-cross-cut-functors (8 tasks)

| Check | Result |
|---|---|
| `[meta]` section | PASS |
| `[[task]]` format | PASS |
| Required fields | PASS |
| Extra fields | max_loc on all tasks |
| `[task.context]` blocks | PASS -- comprehensive read_files with precise line ranges |
| File paths (existing) | PASS -- compose/lib.rs, compose/strategy.rs, compose/prompt.rs, compose/auction.rs, neuro/knowledge_store.rs, daimon/lib.rs, daimon/somatic_ta.rs, daimon/phase2_stubs.rs, dreams/lib.rs, dreams/cycle.rs, dreams/routing_advice.rs, dreams/runner.rs, runner/event_loop.rs, runner/types.rs all exist |
| File paths (new files) | cross_cut.rs, memory_functor.rs, daimon_functor.rs, dreams_functor.rs, natural_transforms.rs, safety_functor.rs -- correctly listed as to-be-created |
| Verify commands | PASS -- structural (grep new files), compile (cargo check) |
| Dependency graph | PASS -- T01 is root, T02/T03/T04 depend on T01, T05 depends on T02/T03/T04, T06 on T02/T03, T07 on T01/T06, T08 on T05/T06 |
| Spec doc refs | PASS -- docs/v2/26-CROSS-CUTS.md exists, line ranges are plausible |
| Duplicate IDs | PASS |
| Tier values | PASS -- `focused` and `integrative` |
| Role values | PASS -- all `implementer` |

**Issues**: None

---

### E45-orchestrator-mori-parity (10 tasks)

| Check | Result |
|---|---|
| `[meta]` section | PASS |
| `[[task]]` format | PASS |
| Required fields | PASS |
| Extra fields | max_loc on all tasks |
| `[task.context]` blocks | PASS -- all tasks have read_files, symbols, anti_patterns |
| File paths (existing) | PASS -- runner/agent_events.rs, runner/gate_dispatch.rs, runner/event_loop.rs, gate/review_verdict.rs, gate/compile_errors.rs, gate/error_patterns.rs, learn/cascade_router.rs, learn/post_gate_reflection.rs, learn/playbook_rules.rs, neuro/admission.rs, neuro/knowledge_store.rs, neuro/hdc.rs, core/config/learning.rs, dispatch/warm_pool.rs all exist |
| Verify commands | PASS -- structural + compile on all tasks |
| Dependency graph | PASS -- T02->T04, T04->T09, T07->T08, no cycles |
| Spec doc refs | PASS -- docs/v2/27-ORCHESTRATOR.md exists |
| Duplicate IDs | PASS |
| Tier values | PASS -- `focused` and `integrative` |
| Role values | PASS -- all `implementer` |

**Issues**: None

---

### E46-github-workflow-integration (12 tasks)

| Check | Result |
|---|---|
| `[meta]` section | PASS -- includes extra valid fields: phase, depends_on_plan |
| `[[task]]` format | PASS |
| Required fields | PASS |
| Extra fields | max_loc, domain, acceptance on all tasks |
| `[task.context]` blocks | PASS -- all tasks have read_files, symbols, anti_patterns |
| File paths (existing) | PASS -- core/config/schema.rs, core/config/serve.rs, core/signal_kinds.rs, serve/routes/webhooks.rs, serve/events.rs, mcp-github/main.rs, runner/event_loop.rs, runner/types.rs all exist |
| File paths (new files) | mcp-github/lib.rs, orchestrator/github_ops.rs, cli/github_ops_impl.rs, commands/github.rs -- correctly listed as to-be-created |
| Verify commands | PASS -- structural (grep, test -f), compile (cargo check), test (cargo test -- webhooks, github) |
| Dependency graph | PASS -- T01 config root, T03 API client root, T04->T05->T06 chain, T06->T07/T08/T09, T12 depends on T06/T07/T08 |
| Duplicate IDs | PASS |
| Tier values | PASS -- `focused` and `integrative` |
| Role values | PASS -- all `implementer` |
| `max_parallel = 2` | PASS -- appropriate for tasks with independent dependency chains |

**Issues**: None

---

### E47-resource-disk-management (11 tasks)

| Check | Result |
|---|---|
| `[meta]` section | PASS -- includes extra valid fields: title, priority, depends_on_plan |
| `[[task]]` format | PASS |
| Required fields | PASS |
| Extra fields | model_hint, acceptance on all tasks |
| `[task.context]` blocks | PASS -- all tasks have read_files with precise line ranges, symbols, anti_patterns |
| File paths (existing) | PASS -- core/config/schema.rs, fs/gc.rs, fs/lib.rs, fs/layout.rs, orchestrate.rs, conductor/watchers/cost_overrun.rs, conductor/watchers/mod.rs, orchestrator/worktree.rs, doctor.rs all exist |
| File paths (new files) | fs/disk.rs, fs/target_cleanup.rs, fs/log_rotation.rs, conductor/watchers/disk_pressure.rs -- correctly listed as to-be-created |
| Verify commands | PASS -- structural (test -f, rg -q), compile (cargo build), test (cargo test) |
| Dependency graph | PASS -- T01 wide fan-out root, T02->T04/T08/T09/T10, convergence at T11 |
| Duplicate IDs | PASS |
| Tier values | PASS -- all `focused` |
| Role values | PASS -- all `implementer` |
| `max_parallel = 2` | PASS |

**Issue #1 (minor)**: E47-T09 context `read_files` references `crates/roko-orchestrator/src/executor.rs`, but this file does NOT exist. The roko-orchestrator crate has `lib.rs`, `dag.rs`, `replan.rs`, etc., but no `executor.rs`. The DAG executor config is likely in `crates/roko-orchestrator/src/dag.rs` or `crates/roko-orchestrator/src/lib.rs`. This is a context hint only and will not block execution, but the agent will need to find the correct file.

---

### E48-rate-limit-budgeting (12 tasks)

| Check | Result |
|---|---|
| `[meta]` section | PASS -- includes depends_on_plan |
| `[[task]]` format | PASS |
| Required fields | PASS |
| Extra fields | model_hint, domain on all tasks |
| `[task.context]` blocks | PASS -- all tasks have read_files, symbols, anti_patterns |
| File paths (existing) | PASS -- agent/model_call_service.rs, agent/retry.rs, agent/rate_limit.rs, agent/provider/mod.rs, agent/error.rs, agent/safety/rate_limit.rs, learn/cascade_router.rs, learn/provider_health.rs, learn/budget.rs, learn/cost_table.rs, learn/cascade/helpers.rs, learn/model_router.rs, conductor/watchers/cost_overrun.rs, cli/orchestrate.rs, cli/config.rs, cli/inline/primitives/cost_meter.rs, cli/main.rs, serve/routes/mod.rs, cli/tui/mod.rs all exist |
| File paths (new files) | None -- all tasks modify existing files |
| Verify commands | PASS -- structural (grep -n with head), compile (cargo check), test (cargo test) |
| Dependency graph | PASS -- T01 independent root, T02->T03, T04 independent root, T01->T05->T06, T04->T08/T09/T10, T05->T10/T11, T09+T03->T12 |
| Duplicate IDs | PASS |

**Issue #2 (minor)**: E48 uses tier values `standard`, `complex`, and `mechanical` instead of the canonical set documented in 06-EXECUTABLE-TASK-FILE-COVERAGE.md (`mechanical`, `focused`, `integrative`, `architectural`). `mechanical` is valid; `standard` and `complex` are non-canonical. The runtime may reject these if it does strict tier validation. Recommended remapping: `standard` -> `focused`, `complex` -> `integrative`.

---

## Issues Summary

| # | Epic | Task | Severity | Description | Recommendation |
|---|---|---|---|---|---|
| 1 | E47 | T09 | Minor | Context `read_files` references non-existent `crates/roko-orchestrator/src/executor.rs` | Change to `crates/roko-orchestrator/src/dag.rs` or `crates/roko-orchestrator/src/lib.rs` |
| 2 | E48 | T01-T12 | Minor | Tier values `standard` and `complex` are non-canonical (expected: mechanical/focused/integrative/architectural) | Remap: `standard` -> `focused`, `complex` -> `integrative` |

## Schema Compliance Summary

All 61 tasks across 6 plans comply with the executable plan schema:
- `[meta]` section present with plan, total, done, status fields
- `[[task]]` arrays (not `[[tasks]]`)
- Required fields: id, title, description, role, status, files, depends_on
- `[task.context]` with read_files, symbols, anti_patterns
- `[[task.verify]]` with phase, command, fail_msg

## Verify Command Quality Assessment

| Pattern | Count | Assessment |
|---|---|---|
| Structural: `grep -q` / `rg -q` / `test -f` | 61 | Good -- checks for expected code artifacts |
| Compile: `cargo check -p` / `cargo build -p` | 56 | Good -- per-crate compilation validation |
| Test: `cargo test -p ... -- <filter>` | 21 | Good -- targeted test execution with filters |
| Multi-phase (structural + compile + test) | 18 | Best -- comprehensive 3-phase validation |
| Structural with head piping: `grep -n ... \| head -5` | 8 | Acceptable -- confirms presence without flooding |

E47 uses `rg -q` (ripgrep) instead of `grep -q` in some verify commands. Acceptable since rg is available, but `grep -q` is more portable.

## Dependency Graph Highlights

- **E46** has the most complex dependency graph: T01 (config) and T03 (API client) are two independent roots feeding into T04 (trait) -> T05 (impl) -> T06 (wiring) -> T07/T08/T09 (features) -> T12 (integration test).
- **E47** has T01 (config) as a wide fan-out root with 6 direct dependents, then convergence at T11 (lifecycle integration).
- **E48** has two independent roots (T01 retry, T04 budget halt) with convergence in the routing path (T05, T10).
- **E44** has a clean diamond: T01 -> T02/T03/T04 -> T05, with a parallel path T01 -> T06 -> T07.
- No circular dependencies detected in any plan.

## Cross-Epic Dependencies

| Field | Value | Valid |
|---|---|---|
| E46 `depends_on_plan` | `["E01-execution-engine"]` | YES -- E01 plan exists |
| E47 `depends_on_plan` | `["E01-execution-engine"]` | YES -- E01 plan exists |
| E48 `depends_on_plan` | `["E01-execution-engine"]` | YES -- E01 plan exists |
| E43 `depends_on_plan` | (none) | OK |
| E44 `depends_on_plan` | (none) | OK |
| E45 `depends_on_plan` | (none) | OK |

## New Meta Fields Observed

E46-E48 (the newly authored plans) introduce additional meta/task fields not seen in E01-E45:

| Field | Plans | Valid |
|---|---|---|
| `phase` (in meta) | E46 | Informational -- not consumed by runtime |
| `priority` (in meta) | E47 | Informational -- not consumed by runtime |
| `domain` (in task) | E46, E48 | Informational -- aids agent context |
| `acceptance` (in task) | E46, E47, E48 | Informational -- aids agent verification |
| `model_hint` (in task) | E47, E48 | Consumed by runtime for CascadeRouter |

All additional fields are either consumed by the runtime (`model_hint`) or informational extras that the TOML parser will ignore if not handled. No risk of schema rejection.
