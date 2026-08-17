# UX Feature Inventory

Status key: [W] Wired | [B] Built (unwired) | [D] Designed | [X] Not built

---

## 1. Workflow Support (Aggregate -> Funnel -> Execute)

### 1.1 Aggregation Phase

| Feature | Status | Location | Notes |
|---|---|---|---|
| `roko ingest <files>` | [X] | -- | Aggregate docs/code into corpus |
| `roko ingest --from-prd <slug>` | [X] | -- | Auto-extract PRD references |
| `roko ingest --from-plan <dir>` | [X] | -- | Gather context from existing plan |
| Corpus storage (.roko/corpora/) | [X] | -- | Indexed, token-counted JSON |
| Corpus listing/inspection | [X] | -- | `roko corpus list` / `roko corpus show` |
| Incremental corpus update | [X] | -- | Add to existing corpus |
| Research integration in corpus | [B] | roko-agent research | `roko research` exists but not integrated with corpus |
| Knowledge store integration | [B] | roko-neuro | `roko knowledge query` exists but not integrated with corpus |
| PRD-to-corpus pipeline | [D] | -- | Read PRD, extract referenced files/crates |

### 1.2 Funneling Phase (Progressive Refinement)

| Feature | Status | Location | Notes |
|---|---|---|---|
| `roko funnel <corpus>` | [X] | -- | Multi-pass refinement workflow |
| Architecture pass | [X] | -- | Agent analyzes system structure |
| Gap analysis pass | [X] | -- | Agent identifies deltas from requirements |
| Task decomposition pass | [X] | -- | Agent generates atomic tasks |
| Dependency analysis pass | [X] | -- | Agent orders tasks, finds parallelism |
| Gate specification pass | [X] | -- | Agent adds acceptance criteria |
| Per-pass checkpoints | [X] | -- | Resume from any pass |
| User approval between passes | [X] | -- | Interactive approve/reject/edit |
| Single-pass plan generation | [B] | `roko prd plan <slug>` | Exists but single-pass only |
| Plan generation from prompt | [B] | `roko plan generate` | Exists but single-pass only |

### 1.3 Execution Phase

| Feature | Status | Location | Notes |
|---|---|---|---|
| Plan execution | [W] | orchestrate.rs PlanRunner | Works end-to-end |
| Gate pipeline per task | [W] | roko-gate | 7-rung pipeline, compile/test/clippy + more |
| State persistence + resume | [W] | .roko/state/executor.json | Snapshot + `--resume` |
| End-of-run summary | [X] | -- | Pass/fail/cost/time summary after run |
| `--dry-run` mode | [X] | -- | Show execution plan without running |
| `--approve-each` mode | [X] | -- | Pause for approval between tasks |
| `--cost-cap` enforcement | [X] | -- | Stop at budget limit |
| Progress output (non-TUI) | [X] | -- | Inline progress for SSH/CI |
| Per-task cost tracking | [B] | CostMeter in chat_inline | Exists but only in inline chat |

### 1.4 Review Phase

| Feature | Status | Location | Notes |
|---|---|---|---|
| `roko status --last-run` | [X] | -- | Detailed last-run summary |
| `roko next` guidance | [X] | -- | Suggest next action based on state |
| Gate failure details | [B] | orchestrate.rs | Logged but not shown clearly |
| Decision explanations | [X] | -- | Why this model? Why this threshold? |
| Playbook recall display | [X] | -- | Show when playbook is matched |

---

## 2. Smart Context Windowing

### 2.1 Context Budget Allocation

| Feature | Status | Location | Notes |
|---|---|---|---|
| Per-task context budget | [X] | -- | Scale with complexity_band |
| Per-section token budget | [X] | -- | Allocate within total budget |
| Context weight enforcement | [B] | SystemPromptBuilder | `context_weight` field exists but not enforced |
| Slim context mode | [X] | -- | identity + role + task + constraints only |
| Standard context mode | [B] | SystemPromptBuilder | Current default, no differentiation |
| Deep context mode | [X] | -- | All sections with generous budgets |

### 2.2 Intelligent Content Selection

| Feature | Status | Location | Notes |
|---|---|---|---|
| File chunking (function-level) | [X] | -- | Include only relevant functions |
| Symbol-aware context | [B] | roko-index | Index exists but not used for context |
| Import chain analysis | [X] | -- | Only include files that task's files import |
| Prior task output feeding | [B] | orchestrate.rs | `load_prior_task_outputs` exists |
| Research context per task | [B] | roko-agent research | Research agent wired, not per-task |
| Neuro knowledge per task | [B] | roko-neuro | Knowledge queries work, not per-task |

### 2.3 Model-Appropriate Content

| Feature | Status | Location | Notes |
|---|---|---|---|
| Strip routing metadata from prompt | [X] | -- | Don't show model/provider to agent |
| Tier-appropriate field selection | [X] | -- | Fast tasks get fewer fields |
| Example pattern pruning | [X] | -- | Only include if context budget allows |
| Context file relevance scoring | [X] | -- | Rank context files by relevance to task |

---

## 3. Task TOML Validation & Splitting

### 3.1 Structural Validation

| Feature | Status | Location | Notes |
|---|---|---|---|
| TOML parse validation | [W] | plan_validate.rs | Basic structural checks |
| Required field check | [B] | plan_validate.rs | Partial |
| Task ID uniqueness | [B] | plan_validate.rs | Partial |
| Dependency reference check | [X] | -- | All depends_on targets exist |
| File path existence check | [X] | -- | All files/context_files exist |

### 3.2 Semantic Validation

| Feature | Status | Location | Notes |
|---|---|---|---|
| Circular dependency detection | [X] | -- | Build DAG, detect cycles |
| File conflict detection | [X] | -- | Same file in same parallel group |
| Acceptance criteria format | [X] | -- | Warn on prose-only (not shell commands) |
| Complexity tier consistency | [X] | -- | fast + high reasoning = warning |
| Cost/time estimate consistency | [X] | -- | Flag outliers |
| Context file relevance check | [X] | -- | Warn on unrelated context_files |

### 3.3 Task Splitting

| Feature | Status | Location | Notes |
|---|---|---|---|
| `roko task split <plan> <id>` | [X] | -- | Split large task into subtasks |
| File-count threshold (8+) | [X] | -- | Auto-suggest split when 8+ files |
| Directory-based grouping | [X] | -- | Group files by directory for subtasks |
| Import-chain grouping | [X] | -- | Group files by import relationships |
| Auto-dependency generation | [X] | -- | Subtask depends_on from file deps |
| User approval of split | [X] | -- | Show proposed split, ask for approval |
| Auto-split during funnel | [X] | -- | Integrated into task decomposition pass |

### 3.4 Task Metadata Tiering

| Feature | Status | Location | Notes |
|---|---|---|---|
| Tier 1: Trivial (5-6 fields) | [X] | -- | id, title, files, acceptance |
| Tier 2: Fast (8-10 fields) | [X] | -- | + depends_on, category, complexity_band, context_files |
| Tier 3: Standard (12-15 fields) | [X] | -- | + parallel_group, exclusive_files, estimated_minutes, tags |
| Tier 4: Complex (20+ fields) | [B] | Mori tasks.toml | Full metadata, exists in Mori format |
| Auto-tier assignment | [X] | -- | Infer tier from file count + description length |
| Tier validation | [X] | -- | Warn when tier-inappropriate fields present |
| Tier-aware prompt assembly | [X] | -- | Strip tier-4 fields from tier-1 agent prompts |

---

## 4. Task Management UX

### 4.1 Data Model

| Feature | Status | Location | Notes |
|---|---|---|---|
| Task struct (basic) | [W] | orchestrate.rs, plan types | id, title, status, files, depends_on |
| Task struct (rich metadata) | [D] | -- | Mori had 20+ fields; roko has ~8 |
| Epic/Plan struct | [W] | roko-orchestrator plan types | Maps to plan with tasks.toml |
| Board struct | [X] | -- | Workspace-level container |
| Task state machine | [B] | orchestrate.rs | pending/active/done/blocked |
| Task dependencies (intra-plan) | [W] | tasks.toml depends_on | Works |
| Task dependencies (cross-plan) | [X] | -- | Mori: "09:T3" cross-plan refs |
| GlobalTaskId | [X] | -- | plan:task composite key |
| Acceptance criteria (structured) | [D] | -- | Mori: checkpointable; roko: flat strings |

### 4.2 Task CRUD

| Feature | Status | Location | Notes |
|---|---|---|---|
| Create task (CLI) | [X] | -- | No `roko task create` |
| Create task (API) | [B] | roko-serve plans routes | POST /api/plans/{id}/tasks (partial) |
| Create task (TUI) | [X] | -- | No task creation UI |
| Create task (Agent) | [X] | -- | Agent-proposed task creation |
| Edit task | [X] | -- | No editing UI |
| View task detail | [B] | `modals/task_detail.rs` (177 LOC) | Shows gate results only |
| List tasks | [W] | TUI task_progress widget | Checklist, no metadata |
| Filter/search tasks | [X] | -- | Not built |
| Batch operations | [X] | -- | Not built |

### 4.3 Task Enrichment

| Feature | Status | Location | Notes |
|---|---|---|---|
| Auto-estimate complexity | [X] | -- | Agent estimates from files/context |
| Auto-suggest model | [B] | CascadeRouter | Router exists but not pre-task |
| Auto-add context files | [X] | -- | Scan imports, find related |
| Auto-add acceptance criteria | [X] | -- | Generate from description + PRD |
| Auto-suggest dependencies | [X] | -- | File overlap analysis |
| Research enrichment | [D] | -- | `roko research` not connected to tasks |
| Enrichment history | [X] | -- | Log of who/what enriched each task |
| Approval flow | [X] | -- | Propose-then-approve for agent changes |

---

## 5. DAG & Execution

### 5.1 DAG Engine

| Feature | Status | Location | Notes |
|---|---|---|---|
| Plan-level DAG | [B] | roko-orchestrator | Types exist, partially wired |
| Task-level DAG | [X] | -- | Mori: UnifiedTaskDag with cross-plan |
| Topological sort | [B] | roko-orchestrator | Exists for plans |
| Critical path computation | [X] | -- | Longest dependency chain |
| Ready frontier | [B] | PlanRunner | Computes next runnable tasks |
| Execution waves | [X] | -- | Group into parallel layers |
| Parallel width estimation | [X] | -- | Agents needed per wave |
| File conflict detection | [X] | -- | exclusive_files prevents overlap |
| DAG recomputation on change | [X] | -- | Event-driven recompute |

### 5.2 Execution Engine

| Feature | Status | Location | Notes |
|---|---|---|---|
| Sequential plan execution | [W] | orchestrate.rs PlanRunner | Works |
| Parallel plan execution | [B] | PlanRunner | Partial |
| Parallel task execution | [B] | PlanRunner | Within-plan, basic |
| Cross-plan task parallelism | [X] | -- | GlobalTaskId scheduling |
| Agent budget enforcement | [B] | roko-runtime | ProcessSupervisor tracks |
| Warm agent reuse | [X] | -- | Keep primary/reviewer warm |
| Task batching | [X] | -- | SpawnTaskAgentBatch |
| Express mode | [X] | -- | Skip reviews, auto-fix |

### 5.3 Queue & Batch

| Feature | Status | Location | Notes |
|---|---|---|---|
| Named queues | [X] | -- | Mori: queue.toml with milestones |
| Milestone grouping | [X] | -- | Group epics by milestone |
| Batch controller | [X] | -- | No orchestrator controller |
| Batch pause modal | [B] | `modals/batch_review.rs` (164 LOC) | Built, no trigger |
| Queue persistence | [X] | -- | Not built |
| Refactor interval | [X] | -- | Every N plans, spawn refactorer |
| Integration test interval | [X] | -- | Every N plans, run cross-crate tests |

### 5.4 Gates & Verification

| Feature | Status | Location | Notes |
|---|---|---|---|
| Compile gate | [W] | gate pipeline | Works |
| Test gate | [W] | gate pipeline | Works |
| Clippy gate | [W] | gate pipeline | Works |
| Custom verify scripts | [B] | gate pipeline | Rungs 4-6 return stubs |
| Gate result display | [W] | TUI gate_block.rs | Inline rendering |
| Verification test grid | [X] | -- | Per-test status indicators |
| Gate failure auto-fix | [B] | orchestrate.rs | build_gate_failure_plan_revision exists |
| Verify convergence detection | [X] | -- | Hash last N errors, detect loops |

---

## 6. Visualization

### 6.1 Board/Kanban View

| Feature | Status | Surface | Notes |
|---|---|---|---|
| Column layout (by status) | [X] | TUI | New tab needed |
| Epic cards with progress | [X] | TUI | New widget |
| Drag between columns | [X] | Web | Web only |
| Column filters | [X] | Both | By tag, assignee, priority |
| Board-level stats | [X] | Both | Total tasks, cost, ETA |

### 6.2 DAG View

| Feature | Status | Surface | Notes |
|---|---|---|---|
| DAG layout (auto) | [X] | Both | Sugiyama or force-directed |
| Node coloring by status | [X] | Both | done/active/ready/pending/failed |
| Critical path highlight | [X] | Both | Bold the longest path |
| Ready frontier highlight | [X] | Both | Show what can run next |
| Click/select node | [X] | Both | Open task detail |

### 6.3 Wave/Timeline View

| Feature | Status | Surface | Notes |
|---|---|---|---|
| Execution waves | [B] | TUI | plan_tree shows waves, limited |
| Gantt timeline | [B] | TUI | phase_timeline.rs (120 LOC, basic) |
| ETA per wave | [X] | Both | Based on task estimates |
| Actual vs. planned | [X] | Both | Show drift |

### 6.4 Agent Pool View

| Feature | Status | Surface | Notes |
|---|---|---|---|
| Active agent cards | [W] | TUI | parallel_pool widget |
| Agent output stream | [W] | TUI | WebSocket connected |
| Token burn sparkline | [W] | TUI | token_sparkline widget |
| Agent-task mapping | [B] | TUI | Shows in plan detail |
| Cost per agent | [B] | serve | CostMeter exists |

---

## 7. Multi-Surface

### 7.1 TUI (ratatui)

| Feature | Status | Notes |
|---|---|---|
| 10-tab layout (F1-F10) | [W] | All rendered |
| Plan tree with waves | [W] | Left panel of F1 |
| Task progress checklist | [W] | Bottom-left of F1 |
| Agent output streaming | [W] | F3 tab + F1 subtab |
| Git branch/commit view | [W] | F4 tab |
| Config inspector | [W] | F6 tab |
| Inspect/context view | [W] | F7 tab |
| Queue overview modal | [W] | Overlay via 'u' key |
| Task detail modal | [B] | Shows gate results only |
| Batch review modal | [B] | No trigger |
| Board/kanban tab | [X] | Needed |
| DAG visualization | [X] | Needed |
| Funnel progress view | [X] | Needed |

### 7.2 CLI

| Feature | Status | Notes |
|---|---|---|
| `roko plan list/show/create/run` | [W] | Exists |
| `roko next` | [X] | Needed -- workflow guidance |
| `roko ingest` | [X] | Needed -- aggregation |
| `roko funnel` | [X] | Needed -- refinement |
| `roko task create/list/show/split` | [X] | Needed |
| `roko corpus list/show` | [X] | Needed |
| `roko plan validate` (semantic) | [X] | Needs enhancement |
| `roko plan tune` | [X] | Needed -- metadata optimization |

### 7.3 HTTP API (roko-serve)

| Feature | Status | Notes |
|---|---|---|
| Plan CRUD routes | [W] | ~12 routes |
| Agent routes | [W] | ~18 routes |
| Learning routes | [W] | ~8 routes |
| Corpus management routes | [X] | Needed |
| Funnel state routes | [X] | Needed |
| Task CRUD routes | [B] | Partial |
| Task enrichment routes | [X] | Needed |
| SSE/WS for live events | [W] | Exists |

### 7.4 ACP (Editor)

| Feature | Status | Notes |
|---|---|---|
| JSON-RPC over stdio | [W] | Editor-agnostic |
| Phase FSM (10 phases) | [W] | Full lifecycle |
| Task awareness | [X] | ACP has no task context |
| Task sidebar | [X] | Show current task in editor |

---

## 8. Summary

| Category | [W] | [B] | [D] | [X] | Total |
|---|---|---|---|---|---|
| Workflow Support | 4 | 6 | 2 | 23 | 35 |
| Smart Context | 0 | 5 | 0 | 11 | 16 |
| Task Validation/Splitting | 2 | 2 | 0 | 17 | 21 |
| Task Management | 3 | 5 | 3 | 14 | 25 |
| DAG & Execution | 5 | 10 | 0 | 19 | 34 |
| Visualization | 4 | 4 | 0 | 11 | 19 |
| Multi-Surface | 16 | 4 | 0 | 16 | 36 |
| **Total** | **34** | **36** | **5** | **111** | **186** |

**Coverage: ~18% wired, ~19% built-but-unwired, ~3% designed, ~60% not built.**

The biggest gaps are in Workflow Support (23 unbuilt features) and Task
Validation/Splitting (17 unbuilt features) -- exactly the areas the user
identified as pain points.

---

## Sources

- `crates/roko-cli/src/main.rs` -- Command enum, all subcommands
- `crates/roko-cli/src/orchestrate.rs` -- PlanRunner, dispatch, gate pipeline
- `crates/roko-cli/src/commands/plan.rs` -- plan handlers
- `crates/roko-cli/src/commands/prd.rs` -- PRD lifecycle
- `crates/roko-cli/src/plan_validate.rs` -- plan validation (basic)
- `crates/roko-cli/src/chat_session.rs` -- ChatAgentSession
- `crates/roko-cli/src/dispatch_helpers.rs` -- prompt assembly helpers
- `crates/roko-compose/src/system_prompt_builder.rs` -- 9-layer builder
- `crates/roko-compose/src/enrichment.rs` -- EnrichmentPipeline
- `crates/roko-orchestrator/src/` -- DAG, executor, task types
- `crates/roko-acp/src/pipeline.rs` -- PipelinePhase (10 variants)
- `crates/roko-acp/src/session.rs` -- ACP session state
- `crates/roko-cli/src/tui/` -- 10 tabs, 21 widgets, 14 modals
- `crates/roko-serve/src/routes/` -- ~85 routes
