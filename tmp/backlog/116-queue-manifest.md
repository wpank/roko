# 116 — Queue Manifest (`.roko/queue.toml` / Milestone System)

**Priority**: P2 — Without milestones, operators running `roko plan run plans/` cannot distinguish the MVP batch from the polish batch or control sequential vs parallel plan ordering beyond raw DAG edges.
**Size**: L (5-7 days)
**Crates**: `crates/roko-cli/src/runner/event_loop.rs`, `crates/roko-cli/src/commands/do_cmd.rs`, `crates/roko-core/src/config/`
**Depends on**: #117 (plan-level wave computation provides the ordering foundation)
**Sources**: `tmp/backlog/_checklist-gaps.md` §1.1, `tmp/backlog/_mori-old-gaps.md` MO-05

---

## Background

When a project has 30+ plans, raw DAG order is not enough to express intent. Mori introduced milestones: named groups of plans that progress sequentially (complete milestone 0 before starting milestone 1) while allowing plans within a milestone to run in parallel within their wave. This lets an operator declare "these 5 plans are the MVP, these 3 are the release polish, these 2 are the docs sprint" without encoding that structure in individual plan dependencies.

The existing `plans/INDEX.md` has narrative grouping but it is not machine-readable. The `tasks.toml` schema has `depends_on_plan` edges for plan-level ordering but no grouping concept. The queue manifest formalizes both grouping and per-run session settings (max agents, mode, express flag) in a single TOML file that operators edit and commit.

## Current State

- `crates/roko-cli/src/runner/` — runner-v2 loads plans via `plan_loader::load_plans()` and executes the DAG, but has no milestone concept.
- `tasks.toml` schema has `depends_on_plan: Vec<String>` for cross-plan edges.
- `plans/INDEX.md` — narrative grouping only; not consumed by the runner.
- No `queue.toml` or equivalent file exists anywhere in the workspace.
- Mori had a `QueueConfig` struct in `orchestrator/queue.rs` (reference: `/Users/will/dev/uniswap/bardo/apps/mori/`).

## Implementation Plan

1. **Define `QueueConfig` struct** in `crates/roko-core/src/config/` (or in `roko-cli` if scope is CLI-only):
   ```toml
   # .roko/queue.toml or plans/queue.toml

   [run]
   max_agents = 4
   mode = "balanced"  # quality | balanced | cost | speed
   express = false

   [[milestone]]
   name = "mvp"
   description = "Core execution loop"
   plans = ["01-task-dag", "02-event-loop", "03-gate-pipeline"]
   tags = ["core"]

   [[milestone]]
   name = "polish"
   description = "TUI and UX improvements"
   plans = ["10-tui-tabs", "11-header-bar"]
   depends_on = ["mvp"]  # milestone-level dependency
   ```

2. **Parse queue manifest**: Implement `QueueConfig::from_file(path)` using `toml` crate. Validate that all plan IDs in milestones exist in `plans/`. Validate that milestone dependencies form a DAG.

3. **Wire into `plan_loader::load_plans()`**: Accept an optional `QueueConfig`. When present, filter and order plans according to milestone progression. Active milestone's plans are eligible to run; subsequent milestone's plans are blocked until the current milestone completes.

4. **Milestone sequencing in event loop**: Track completed plans per milestone. When all plans in milestone N complete, unlock milestone N+1's plans. Emit a `RunnerEvent::MilestoneCompleted` event.

5. **Add `roko plan queue` subcommands**:
   - `roko plan queue show` — display milestone status in table format.
   - `roko plan queue validate` — validate queue.toml structure and plan references.
   - `roko plan queue init` — generate a starter `queue.toml` from `plans/INDEX.md`.

6. **`--queue <path>` flag on `plan run`**: Accept `roko plan run plans/ --queue .roko/queue.toml`. When omitted, use default DAG ordering (current behaviour unchanged).

7. **Per-run session overrides**: `[run]` section in `queue.toml` overrides `roko.toml` runner config for just this run (max agents, mode, express). Document precedence: CLI flags > queue.toml > roko.toml.

## Acceptance Criteria

1. `roko plan run plans/ --queue .roko/queue.toml` loads and respects the milestone ordering.
2. Plans in milestone 1 do not start until all plans in milestone 0 complete.
3. `roko plan queue show` displays milestone names, plan counts, and completion status.
4. `roko plan queue validate` exits non-zero if a plan ID in `queue.toml` does not exist in `plans/`.
5. `roko plan queue init` generates a valid `queue.toml` from `plans/INDEX.md`.
6. When `--queue` is omitted, behaviour is identical to today (no regression).
7. `[run]` overrides in `queue.toml` take effect (e.g., `max_agents = 2` limits concurrency for that run).

## Verification Checklist

- [ ] Create a `queue.toml` with two milestones; verify milestone 1 plans do not start until milestone 0 completes.
- [ ] `roko plan queue validate` returns an error when a plan ID is misspelled.
- [ ] `roko plan queue show` output includes milestone names and per-milestone progress.
- [ ] Run without `--queue` and verify existing behaviour is unchanged.
- [ ] Unit test: `QueueConfig::from_file` with a valid and invalid TOML.

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-core/src/config/` or `crates/roko-cli/src/config.rs` | Add `QueueConfig` struct and parser |
| `crates/roko-cli/src/runner/event_loop.rs` | Add milestone tracking and plan unlock logic |
| `crates/roko-cli/src/commands/do_cmd.rs` | Add `--queue` flag to `plan run` |
| `crates/roko-cli/src/commands/plan.rs` | Add `roko plan queue show/validate/init` subcommands |
| `crates/roko-cli/src/commands/mod.rs` | Register new queue subcommands |
