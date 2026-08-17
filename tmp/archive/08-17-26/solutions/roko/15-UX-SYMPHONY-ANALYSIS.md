# Symphony Analysis: Integration & Synergy with Roko

## What Symphony Is

[Symphony](https://github.com/openai/symphony) (Apache-2.0, released March
2026, 15.8K stars) is an open-source spec + Elixir reference implementation
for turning an issue tracker (Linear) into a control plane for autonomous
coding agents.

Core idea: **Your kanban board IS the scheduler. Every open issue gets an
agent. Agents run continuously until the work is done. Humans review results,
not code.**

OpenAI reported a **500% increase in landed PRs** on some internal teams.

---

## How Symphony Works

### Architecture

```
WORKFLOW.md (YAML config + Liquid prompt template)
    |
Orchestrator (GenServer, poll every 5s)
    |
    +--> Issue-1 workspace --> Codex agent
    +--> Issue-2 workspace --> Codex agent
    +--> Issue-N workspace --> Codex agent
```

### State Machine

```
Linear Board:
  Backlog -> Todo -> In Progress -> Human Review -> Merging -> Done
                        |
                     Rework (loop)

Symphony Internal:
  Unclaimed -> Claimed -> Running -> Released
                            |
                       RetryQueued (exponential backoff)
```

### The Flow

1. Symphony daemon polls Linear every 5s for issues in `active_states`
2. Issue becomes Todo -> Symphony claims, creates isolated workspace
3. Agent starts -> Codex subprocess spawns, receives rendered prompt
4. Agent works autonomously -> implements, tests, creates PR
5. Agent moves issue to Human Review -> PR ready with CI passing
6. Human reviews -> approves or sends to Rework
7. Rework -> agent closes PR, creates fresh branch, restarts from scratch
8. Approved -> Merging -> agent lands PR via `land` skill
9. Done -> Symphony kills agent, cleans workspace

---

## Roko vs. Symphony: What Matters for the User's Workflow

The user's workflow is: **aggregate docs -> multiple passes to funnel into
mechanical plans with gates -> execute via agents.** How does each system
support this?

### Aggregation Phase

| Aspect | Symphony | Roko (Current) | Roko (Planned) |
|---|---|---|---|
| Input format | Linear issues (human-written) | PRDs, tasks.toml (human-written) | `roko ingest` (automated) |
| Context gathering | Issue description only | `context_files` field | Corpus management with token counting |
| Research | None | `roko research` commands | Integrated into corpus |
| Prior knowledge | None | roko-neuro knowledge store | Integrated into corpus |

**Symphony has no aggregation phase.** The human writes a Linear issue and
that is the entire context. Roko can and should do better.

### Funneling Phase

| Aspect | Symphony | Roko (Current) | Roko (Planned) |
|---|---|---|---|
| Refinement passes | None (issue -> agent) | Single-pass (PRD -> plan) | Multi-pass funnel (5 passes) |
| Task decomposition | Human writes issues | `roko prd plan` (single agent call) | Funnel task pass with user approval |
| Dependency analysis | None (independent issues) | `depends_on` in tasks.toml | Funnel deps pass with auto-analysis |
| Acceptance criteria | Human writes in issue | Flat strings in tasks.toml | Funnel gates pass with shell commands |
| Validation | None | Minimal | Full semantic validation |

**Symphony has no funneling phase.** Issues are independent -- no DAG, no
dependencies, no decomposition. Each issue is a standalone work item.

This is Symphony's biggest limitation for complex codebases. When you need
coordinated changes across 10 files with ordering constraints, Symphony
cannot express the dependency graph.

### Execution Phase

| Aspect | Symphony | Roko (Current) | Roko (Planned) |
|---|---|---|---|
| Execution model | 1 agent per issue, independent | DAG-parallel with gates | Same, + context windowing |
| Isolation | Separate workspace per issue | Worktrees per plan | Same |
| Gates | External CI only | 7-rung built-in pipeline | Same, + acceptance criteria from funnel |
| Model selection | Codex only | 8+ backends + CascadeRouter | Same, + tier-based routing |
| Cost tracking | None | Per-run (partial) | Per-task (planned) |
| Resume | Rebuild from tracker state | Executor snapshots | Same |
| Rework strategy | Close PR, fresh branch | gate_failure_plan_revision | Same |

**Roko's execution is strictly superior** for the user's use case.
DAG-parallel execution, built-in gates, multi-backend routing, and learning
from prior runs are all advantages that Symphony does not have.

---

## What Roko Should Borrow from Symphony

### 1. Single-Command Simplicity

Symphony: `./bin/symphony WORKFLOW.md` -- one command, one file, running.

Roko has 60+ commands. The user should not need to learn them all.

**Adoption:** `roko next` + workflow guidance in bare `roko` invocation.
For advanced users: `roko funnel sprint-42` -> `roko plan run sprint-42/`
is just two commands for the full workflow.

### 2. "Board as Scheduler" Mindset

Symphony: the Linear board IS the execution state. Moving a card is a
scheduling decision.

**Adoption for Roko:** The tasks.toml (and later, the board view) should
be the execution state. Editing a task's status in the TOML (or the TUI)
should be the way to control execution. This is already how Mori worked
with the "edit and re-ingest" loop.

### 3. Rework = Full Reset

Symphony: when a task fails review, close the PR, create a fresh branch,
and restart from scratch. No incremental patching.

**Roko already has this:** `build_gate_failure_plan_revision` in
`orchestrate.rs`. But it should be more visible: the run summary should
say "T6: reworking (attempt 2/3, fresh branch)."

### 4. Hot Reload

Symphony: change `WORKFLOW.md` while agents are running. New config applies
to next dispatch.

**Adoption:** Roko should detect edits to `tasks.toml` during a run and
re-ingest without stopping execution. Mori had this via filesystem watcher.
Roko's TUI filesystem watcher (`tui/fs_watch.rs`) exists but is not wired
to the executor.

### 5. Idempotent Recovery

Symphony: on restart, rebuild state entirely from the tracker + filesystem.
No persistent DB needed.

**Roko already has this** via executor snapshots. But it could be improved:
if the snapshot is missing or corrupted, Roko should be able to rebuild
state from the tasks.toml files (like Symphony rebuilds from Linear).

---

## What Roko Should NOT Borrow from Symphony

### 1. No DAG (Independent Issues)

Symphony treats every issue as independent. There is no `depends_on`, no
`parallel_group`, no execution waves. This is fine for isolated bug fixes
but fails for coordinated feature work.

The user's workflow requires dependencies. Keep Roko's DAG.

### 2. No Built-in Gates

Symphony relies entirely on external CI for verification. The agent creates
a PR, CI runs, human reviews. There is no immediate feedback to the agent.

Roko's 7-rung gate pipeline provides immediate feedback: the agent knows
within seconds whether its code compiles, tests pass, and clippy is clean.
This enables the auto-fix loop without waiting for CI.

### 3. No Learning

Symphony treats every run as independent. There is no episode logging, no
playbook recall, no cascade routing. Each run starts from zero knowledge.

Roko's learning subsystem means that the 100th task benefits from the
patterns discovered in the first 99 tasks. This is a fundamental moat.

### 4. Codex Only

Symphony dispatches only to Codex. Roko dispatches to 8+ backends with
intelligent routing based on task characteristics. Different tasks benefit
from different models.

### 5. No Context Assembly

Symphony renders a Liquid template with the issue description and nothing
else. There is no research context, no knowledge store hits, no prior task
outputs, no architecture overview.

Roko's 9-layer SystemPromptBuilder, while in need of context windowing
improvements, provides dramatically richer context than Symphony.

---

## Symphony-Compatible Features for Roko

### Priority 1: Click-to-Execute (Single Task)

The highest-value Symphony pattern: any task on any surface can be executed
with one action.

```bash
# CLI
roko task run T42

# API
POST /api/tasks/T42/execute

# TUI
Select task -> press 'x' -> confirm
```

Implementation:
1. Task transitions to `active`
2. Create workspace (git worktree from current branch)
3. Build prompt from TaskAgentInput (not full TaskSpec) + context budget
4. Dispatch to agent (CascadeRouter selects model)
5. Stream output to subscribed surfaces
6. On completion: run gate pipeline
7. On pass: create PR, status = "review"
8. On fail: status = "rework" with failure context

**File:** `crates/roko-cli/src/orchestrate.rs` (add `execute_single_task`)
**Effort:** ~300 LOC

### Priority 2: Continuous Watch Mode (Daemon)

Symphony's daemon mode: continuously watch for ready tasks and dispatch.

```bash
roko plan watch plans/sprint-42/ --max-agents 5
```

Behavior:
- Check ready frontier every 10s
- Sort by priority, dependency satisfaction
- Dispatch up to max_agents concurrently
- Reconcile on each tick
- Clean up completed workspaces

**File:** `crates/roko-cli/src/orchestrate.rs` (add `watch_mode`)
**Effort:** ~400 LOC

### Priority 3: WORKFLOW.md Compatibility

Accept Symphony-format WORKFLOW.md as a simplified config:

```bash
roko symphony WORKFLOW.md
```

Parser maps Symphony config to Roko internals:
- `tracker.kind: roko` -> use internal task store
- `agent.max_concurrent` -> ProcessSupervisor limit
- Prompt template -> feed to SystemPromptBuilder as task layer
- `hooks` section -> workspace lifecycle

**File:** new `crates/roko-cli/src/commands/symphony.rs`
**Effort:** ~500 LOC

### Priority 4: External Tracker Sync (Linear, GitHub)

Bidirectional sync between Roko tasks and external trackers:

```toml
# roko.toml
[board.sync]
tracker = "linear"
project_slug = "my-project"
api_key = "$LINEAR_API_KEY"
poll_interval_ms = 10000
state_mapping = { "Todo" = "ready", "In Progress" = "active" }
```

**File:** new `crates/roko-cli/src/tracker_sync.rs`
**Effort:** ~800 LOC

---

## Architectural Alignment Summary

| Concept | Symphony | Roko (Current) | Roko (Target) |
|---|---|---|---|
| Config | WORKFLOW.md | roko.toml + tasks.toml | Same + optional WORKFLOW.md |
| Scheduler | Linear board | PlanRunner | PlanRunner + board + watch mode |
| Agent dispatch | Codex subprocess | 8+ backends | Same + tier-based routing |
| Isolation | workspace per issue | worktree per plan | Same |
| Gates | External CI | 7-rung built-in | Same + acceptance from funnel |
| Recovery | Rebuild from tracker | Executor snapshot | Same + TOML rebuild fallback |
| Learning | None | Full (episodes, playbooks, routing) | Same |
| Tracker | Linear only | Internal only | Internal + Linear + GitHub |
| Rework | Close PR, fresh branch | gate_failure_plan_revision | Same, more visible |

The target is clear: adopt Symphony's simplicity patterns (single-command
start, board-as-scheduler, hot reload) while keeping Roko's power features
(DAG, gates, learning, multi-backend, progressive context refinement).

---

## Sources

### External (Symphony)
- [OpenAI Symphony Blog Post](https://openai.com/index/open-source-codex-orchestration-symphony/)
- [Symphony GitHub Repository](https://github.com/openai/symphony)
- [Symphony SPEC.md](https://github.com/openai/symphony/blob/main/SPEC.md)
- [Symphony WORKFLOW.md](https://github.com/openai/symphony/blob/main/elixir/WORKFLOW.md)

### Internal (Roko)
- `crates/roko-agent/src/` -- 8+ backend files
- `crates/roko-cli/src/orchestrate.rs` -- PlanRunner (comparison target)
- `crates/roko-gate/` -- 7-rung gate pipeline
- `crates/roko-serve/src/routes/` -- ~85 routes
- `crates/roko-acp/src/transport.rs` -- stdio transport
- `crates/roko-learn/` -- episodes, playbooks, cascade router
