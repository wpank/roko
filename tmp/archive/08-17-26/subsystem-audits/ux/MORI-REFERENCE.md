# Mori UX Reference

Analysis from 19 screenshots (`tmp/screenshots/mori/`) + full code exploration
of `/Users/will/dev/uniswap/bardo/apps/mori/src/`.

---

## What Mori Did Right (The "Okay-ish" Way)

Mori's core UX pattern was: **task TOMLs are the source of truth. Ingest them,
build the DAG, execute, update the TOML, repeat.** This worked because:

1. The user (or Claude) wrote a `tasks.toml` file with rich metadata
2. `mori plan ingest <dir>` read all task TOMLs, built the DAG, computed
   execution waves, and set up the executor state
3. The user pressed 'p' in the TUI to start/pause execution
4. As tasks completed, Mori updated the `tasks.toml` (status field)
5. If the user edited the `tasks.toml` mid-run, Mori detected the change
   (filesystem watcher), re-ingested, rebuilt the DAG, and continued
6. The entire state was visible: plan tree, task progress, agent output,
   gate results, git branches, verification tests

The user reports this was "okay-ish" -- it worked, but the user still had
to write the tasks.toml manually, there was no validation, and the context
assembly was not intelligent about task complexity.

### How Mori's Ingestion Worked

When `mori plan ingest <dir>` was invoked:

1. **Discover plans:** Scan directory recursively for `tasks.toml` files.
   Each directory containing a `tasks.toml` became a plan.

2. **Parse frontmatter:** Each plan's `tasks.toml` had a `[meta]` section
   with plan-level metadata:
   ```toml
   [meta]
   plan = "71-mvp-gate"
   iteration = 1
   total = 22
   done = 21
   status = "implementing"
   verify_passed = false
   last_gate = "compile-fail"
   max_parallel = 8
   estimated_total_minutes = 50
   ```

3. **Parse tasks:** Each `[[task]]` entry was parsed into a task struct
   with 20+ fields. All fields were optional except `id` and `title`.

4. **Build plan DAG:** Plan-level `depends_on` from frontmatter:
   ```yaml
   depends_on: ["70-terminal-soma"]
   parallel_with: ["71a-extended", "72-ui-components"]
   ```
   Created a DAG of plans. Plans in `parallel_with` could run simultaneously.
   Plans in `depends_on` had to complete first.

5. **Build task DAG (within plan):** Each task's `depends_on` field
   referenced other task IDs within the same plan. Mori built a DAG per
   plan and computed topological order.

6. **Build unified task DAG (cross-plan):** When `depends_on` contained
   cross-plan references like `"09:T3"` (plan 09, task T3), Mori built a
   global DAG across all plans using `GlobalTaskId` (plan_id:task_id).

7. **Compute execution waves:** Topological layers of the unified DAG
   became execution waves. Wave 0 = all tasks with no dependencies.
   Wave 1 = all tasks whose dependencies are all in wave 0. Etc.

8. **Set up parallel groups:** Tasks within the same wave that shared a
   `parallel_group` letter could run simultaneously. Tasks with
   `exclusive_files = true` in the same group were serialized to prevent
   file conflicts.

9. **Persist state:** The executor wrote state to `.mori/state/executor.json`
   as a snapshot. On crash, `--resume` restored from this snapshot.

### How Mori's "Edit and Re-ingest" Loop Worked

The filesystem watcher (`notify::RecommendedWatcher`) watched the plans
directory. When a `tasks.toml` file changed:

1. Pause the executor (if running)
2. Re-read the changed file
3. Diff against the in-memory task list:
   - New tasks: add to DAG, compute new dependencies
   - Removed tasks: remove from DAG, unblock dependents
   - Changed tasks: update metadata, recompute if status changed
4. Rebuild the DAG (only the affected portion)
5. Recompute execution waves
6. Resume the executor

This allowed mid-flight plan editing. The user could:
- Add new tasks to a running plan
- Change task metadata (model, priority, acceptance criteria)
- Reorder tasks by changing `depends_on`
- Mark tasks as `done` manually to skip them
- Remove tasks entirely

The re-ingest was incremental -- it did not rebuild the entire DAG from
scratch, only the affected subgraph.

### How Mori's Agent Dispatch Used Task Metadata

When dispatching an agent for a task, Mori used the task metadata to:

1. **Select model:** `preferred_model` + `preferred_provider` specified
   the exact model. If not specified, the CascadeRouter chose based on
   `reasoning_level`, `speed_priority`, and `complexity_band`.

2. **Assemble context:** `context_files` were read and included in the
   prompt. `context_weight` determined how much additional context (prior
   task outputs, research, knowledge store hits) was included.

3. **Set acceptance criteria:** `acceptance` strings were injected into
   the prompt as "You must satisfy these conditions." and into the gate
   pipeline as shell commands to verify.

4. **Configure parallelism:** `parallel_group` and `exclusive_files`
   determined whether the task could run alongside other tasks.

5. **Set budget:** `estimated_minutes` translated to a token budget and
   timeout.

6. **Handle retries:** `escalate_on_retry = true` meant that on gate
   failure, the retry would use a more capable model.

### What Did NOT Work Well in Mori

1. **Manual task authoring:** The user (or Claude) had to write every
   tasks.toml by hand. There was no aggregation or funneling phase. You
   started with implementation tasks, not with requirements analysis.

2. **All-or-nothing metadata:** Every task got the full 20+ field treatment
   regardless of complexity. A trivial rename task had the same metadata
   structure as a complex architectural redesign. This was wasteful and
   sometimes confusing for smaller models.

3. **No validation:** Malformed tasks.toml caused runtime errors with
   unhelpful messages. Circular dependencies caused deadlocks. File
   conflicts in parallel groups caused race conditions. There was no
   pre-flight check.

4. **Monolithic context:** The prompt assembly included all context layers
   for every task. No differentiation by task complexity. A `complexity_band
   = "fast"` task got research context, knowledge hits, and architecture
   overview that it did not need.

5. **No cost tracking per task:** Cost was tracked per-run and per-plan
   but not per-task. The user could not see "T6 cost $3.47 because it
   used Opus for 3 iterations."

6. **No multi-pass refinement:** There was no mechanism for progressive
   context narrowing. The user jumped from "here is a PRD" to "here are
   implementation tasks" in one agent call.

---

## Screenshot Index

| # | Screenshot | Tab | What It Shows |
|---|---|---|---|
| 1 | 5:45:00 | F1:dash / a:Agents | Plan tree (36/48), agents subtab, phase bar |
| 2 | 5:45:04 | F1:dash / o:Output | Plan tree, output subtab |
| 3 | 5:45:14 | F1:dash / o:Output | Same, time progression |
| 4 | 5:45:18 | F1:dash / d:Diff | Diff subtab: "No changes detected" |
| 5 | 5:45:22 | F1:dash / e:Errors | Preflight warnings + runtime log |
| 6 | 5:45:27 | F1:dash / g:Git | Branch tree + commit graph + worktrees |
| 7 | 5:45:33 | F1:dash / m:MCP | MCP runtime, AST index, Tool/Learning stats |
| 8 | 5:45:40 | F1:dash / P:Procs | Processes, fixtures, conductor output |
| 9 | 5:45:46 | F1:dash / v:verify | Verification tests, conductor, system metrics |
| 10 | 5:46:42 | F1:dash (detail) | Plan detail: summary, "What Was Built", files |
| 11 | 5:46:50 | F1:dash (task) | Task T1 detail: all metadata fields |
| 12 | 5:46:57 | F2:plans | Waves/Plans: W0-W6, plan detail |
| 13 | 5:47:07 | F2:plans (W2) | Plan 14b: 13 tasks, 58 verification tests |
| 14 | 5:47:16 | F3:agents | Agent detail: model, route, tokens |
| 15 | 5:47:23 | F4:git | Branch tree, commit graph, worktrees |
| 16 | 5:47:28 | F5:logs | 9 log entries: queue, executor, express mode |
| 17 | 5:47:33 | F6:cfg | Backend defaults, 30+ role overrides, agents |
| 18 | 5:47:38 | F7:inspect | MCP + Tool/Learning: 6.6K episodes, 92% routing |
| 19 | 5:47:43 | F8:queue | Queue overview: milestones, plan completion |

---

## Key UX Patterns to Replicate (With Improvements)

### 1. Header Bar (Always Visible)

```
roko  Wave 1/7  Plan: sprint-42  ||||....  8/10  80%  ETA:2m  F1-F10
```

Mori had this. Roko's TUI has a header bar but it does not show wave
progress, ETA, or active plan name prominently.

**Improvement:** Add cost tracking to the header:
```
roko  W1/3  sprint-42  ||||..  8/10  80%  $6.42  ETA:5m  F1-F10
```

### 2. Plan Tree (Left Panel)

Mori's plan tree showed collapsible waves with per-plan progress. This is
replicated in Roko's TUI (F1 left panel, `widgets/plan_tree.rs`).

**Improvement:** Add complexity tier indicators:
```
Wave 0 (5/7)
  T1 [trivial]  Fix typo in README.md       done  haiku  $0.01
  T2 [fast]     Scaffold widget module       done  sonnet $0.23
  T3 [standard] Implement auth flow          impl  opus   $1.47
```

### 3. Task Detail Modal (Enter on a Task)

Mori's task detail showed: title, status, parallel group, exclusive files,
blocked-by, files, acceptance criteria (6+ criteria with shell checkpoints).

Roko's `modals/task_detail.rs` (177 LOC) shows: task name, id, status,
elapsed time, assigned agents, gate results. Missing: all the rich metadata.

**Improvement:** Show the agent-visible fields only (not routing metadata).
Add the gate failure output verbatim so the user can diagnose issues.

### 4. Verification Test Grid

Mori showed individual tests: CG1-CG7 (compile gates), LS1-LS16 (lifecycle),
SV1-SV8 (scaffold verification), UT1-UT27 (unit tests), INV-001-020
(invariants). Each test had a pass/fail/skip indicator.

Roko shows gate verdicts as pass/fail blocks but does not itemize individual
tests.

**Improvement:** Keep Roko's simpler gate display for most tasks. Add
itemized test output as an expandable section in the task detail modal.

### 5. Queue Overview

Mori's F8 showed milestones with plan counts and a completion checklist.

Roko's queue overview modal (`modals/queue_overview.rs`) exists and is wired
via the 'u' key. It needs to show: pending plans, next up, completion state,
and total cost.

**Improvement:** Add cost-per-milestone and ETA-per-milestone.

---

## Mori Task TOML Structure (Canonical Reference)

### Plan Frontmatter

```toml
[meta]
plan = "71-mvp-gate"
iteration = 1
total = 22
done = 21
status = "implementing"
verify_passed = false
last_gate = "compile-fail"
completed_at = ""
max_parallel = 8
estimated_total_minutes = 50
```

### Task Entry (Full 20+ Fields)

```toml
[[task]]
id = "T1"
title = "Scaffold golem-runtime crate stubs for all 15 modules"
status = "done"
files = ["crates/golem-runtime/src/lib.rs", "crates/golem-runtime/src/mind.rs"]
depends_on = []
parallel_group = "A"
exclusive_files = true
category = "scaffolding"
reasoning_level = "low"
speed_priority = "latency"
quality_profile = "pragmatic"
context_weight = "slim"
preferred_model = "claude-opus-4-6"
preferred_provider = "codex"
estimated_minutes = 5
complexity_band = "fast"
escalate_on_retry = false
tags = ["runtime", "scaffold"]
acceptance = ["All 15 module files exist with correct pub mod declarations"]
context_files = ["crates/golem-runtime/Cargo.toml"]
example_pattern = "crates/roko-core/Cargo.toml"
```

### Field Classification (For Roko's Tiered Approach)

**Agent-visible fields (always include in prompt):**
- `id`, `title`, `status`, `files`, `depends_on`, `acceptance`,
  `context_files`, `tags`

**Dispatcher-only fields (never include in agent prompt):**
- `preferred_model`, `preferred_provider`, `reasoning_level`,
  `speed_priority`, `quality_profile`, `context_weight`,
  `complexity_band`, `escalate_on_retry`

**Conditional fields (include based on complexity tier):**
- `parallel_group`, `exclusive_files`, `category`,
  `estimated_minutes`, `example_pattern`

### Queue TOML (Execution Configuration)

```toml
[run]
mode = "express"
max_agents = 8
max_parallel_plans = 3
preset = "balanced"

[[milestone]]
name = "Minimal MVP"
description = "Core workspace, agent spawn, CLI"
tags = ["core", "priority"]
plans = ["01-workspace-scaffold", "02-agent-spawn", ...]
```

---

## Executor State Machine (Mori)

```
PlanPhase:
  Implementing -> Gating -> Verifying -> Reviewing -> Done -> Merging -> Complete
                                                         \-> Failed { reason }
                                 \-> AutoFixing -> (back to Implementing)
                                 \-> RegeneratingVerify -> (back to Verifying)
```

Roko's PlanRunner implements a similar state machine in `orchestrate.rs`.
The key difference: Mori had explicit `AutoFixing` and `RegeneratingVerify`
phases that looped back. Roko's `build_gate_failure_plan_revision` handles
the auto-fix case but does not expose it as a distinct phase in the UX.

---

## Key Metrics From Mori's Last Session (Screenshots)

| Metric | Value | Source |
|---|---|---|
| Total plans in queue | 48 | Queue: Audit Remediation |
| Completed plans | 36 (75%) | Plan tree |
| Waves | 7 (W0-W6) | Wave view |
| Plans in W0 | 29/35 | Wave header |
| Max tasks per plan | 13 | Plan 14b-cognitive-mechanisms |
| Max verification tests | 58 | Plan 14b-cognitive-mechanisms |
| Episodes logged | 6.6K | MCP/inspect view |
| Playbooks | 98 total, 98 learned | Learning stats |
| Routing accuracy | 92% (1.6K/1.7K) | Cascade router |
| Model pass rate | 100% (claude-opus-4-6) | Learning stats |
| Avg task time | 129s | Learning stats |
| Avg cost per run | $1.433 | Learning stats |
| Total runs | 1,905 model / 3,918 provider | Learning stats |
| AST index | 6.1K files, 153.6K symbols | AST index panel |
| Resolution rate | 285.3K/634.9K (45%) | AST index panel |
| Routing via index | 92% | AST index panel |
| MCP servers | 3 (codex, claude, cursor) | MCP panel |
| Agent roles | 30+ | Config view |
| Worktrees | 15+ active | Git view |

---

## What Roko Should Carry Forward vs. Improve

### Carry Forward (Proven Patterns)

1. **Task TOML as canonical artifact:** The TOML format is human-readable,
   git-friendly, and machine-parseable. Keep it.

2. **DAG-driven execution:** Topological ordering with parallel waves is
   correct. Keep the plan/task DAG.

3. **Gate pipeline per task:** Compile, test, clippy verification after
   each task is essential. Keep and extend.

4. **State persistence + resume:** Executor snapshots for crash recovery
   are essential. Keep the `.roko/state/` pattern.

5. **Rich task metadata:** The 20+ fields enable intelligent routing and
   scheduling. Keep the fields -- but tier them.

6. **Live TUI with subtabs:** The F1-F10 layout with subtabs is proven.
   Keep and extend.

### Improve (Known Weaknesses)

1. **Add aggregation and funneling phases.** Mori jumped from PRD to tasks.
   Roko should support the full workflow.

2. **Tier task metadata by complexity.** Not every task needs 20+ fields.
   Fast tasks get 5-6 fields.

3. **Separate routing metadata from agent prompt.** The agent should not
   see `preferred_model` or `reasoning_level`.

4. **Add validation before execution.** Catch circular deps, file conflicts,
   and malformed acceptance criteria before dispatching agents.

5. **Scale context with task complexity.** Slim context for trivial tasks,
   deep context for complex tasks.

6. **Add per-task cost tracking and clear run summaries.** The user should
   not have to read JSONL logs to determine run outcome.

7. **Add workflow guidance.** `roko next` should tell you what to do.

---

## Sources

- `tmp/screenshots/mori/` -- 19 screenshots from 5:45:00-5:47:43
- `/Users/will/dev/uniswap/bardo/apps/mori/src/` -- Mori source (reference only)
- `/Users/will/dev/uniswap/bardo/.mori/plans/` -- 171 plan TOMLs with task metadata
- `crates/roko-cli/src/orchestrate.rs` -- Roko's PlanRunner (comparison target)
- `crates/roko-cli/src/tui/` -- Roko's TUI (comparison target)
- `crates/roko-orchestrator/src/` -- DAG computation (comparison target)
