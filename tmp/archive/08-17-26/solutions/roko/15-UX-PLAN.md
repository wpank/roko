# UX Implementation Plan

## Architecture Decisions

### AD-1: Workflow Commands as First-Class Citizens

The aggregation-funnel-execute workflow gets dedicated top-level commands:
- `roko ingest` -- Phase 1: aggregate context
- `roko funnel` -- Phase 2: progressive refinement
- `roko plan run` -- Phase 3: execute (already exists)
- `roko next` -- Cross-phase: suggest next action

These are not hidden in subcommands. They are the primary workflow commands,
listed first in help text.

### AD-2: Task Metadata Tiering

Two-struct approach: `TaskSpec` (full 20+ fields for disk/API) and
`TaskAgentInput` (agent-visible fields only, filtered by complexity tier).

```rust
/// Full task specification -- persisted to disk and API.
pub struct TaskSpec {
    // === Agent-visible (always in prompt) ===
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: TaskStatus,
    pub files: Vec<String>,
    pub depends_on: Vec<String>,
    pub acceptance: Vec<String>,
    pub context_files: Vec<String>,
    pub tags: Vec<String>,

    // === Conditional (included based on complexity tier) ===
    pub parallel_group: Option<String>,
    pub exclusive_files: Option<bool>,
    pub category: Option<TaskCategory>,
    pub estimated_minutes: Option<u32>,
    pub example_pattern: Option<String>,

    // === Routing-only (never in agent prompt) ===
    pub preferred_model: Option<String>,
    pub preferred_provider: Option<String>,
    pub reasoning_level: Option<String>,
    pub speed_priority: Option<String>,
    pub quality_profile: Option<String>,
    pub context_weight: Option<String>,
    pub complexity_band: Option<ComplexityBand>,
    pub escalate_on_retry: Option<bool>,
}

/// Agent-visible task input -- stripped of routing metadata.
/// Assembled by the dispatcher based on complexity tier.
pub struct TaskAgentInput {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub files: Vec<String>,
    pub depends_on: Vec<String>,
    pub acceptance: Vec<String>,
    pub context_files: Vec<String>,
    pub tags: Vec<String>,
    // Conditional fields included only for standard+ tiers:
    pub parallel_group: Option<String>,
    pub category: Option<TaskCategory>,
    pub estimated_minutes: Option<u32>,
    pub example_pattern: Option<String>,
}

impl TaskSpec {
    /// Convert to agent input, stripping routing metadata and
    /// conditional fields based on complexity tier.
    pub fn to_agent_input(&self) -> TaskAgentInput {
        let tier = self.complexity_band.unwrap_or(ComplexityBand::Standard);
        TaskAgentInput {
            id: self.id.clone(),
            title: self.title.clone(),
            description: self.description.clone(),
            files: self.files.clone(),
            depends_on: self.depends_on.clone(),
            acceptance: self.acceptance.clone(),
            context_files: self.context_files.clone(),
            tags: self.tags.clone(),
            // Only include conditional fields for standard+ tiers
            parallel_group: if tier >= ComplexityBand::Standard {
                self.parallel_group.clone()
            } else { None },
            category: if tier >= ComplexityBand::Fast {
                self.category
            } else { None },
            estimated_minutes: if tier >= ComplexityBand::Standard {
                self.estimated_minutes
            } else { None },
            example_pattern: if tier >= ComplexityBand::Complex {
                self.example_pattern.clone()
            } else { None },
        }
    }
}
```

**Location:** `crates/roko-orchestrator/src/task_spec.rs` (new file)
**Wiring:** `crates/roko-cli/src/dispatch_helpers.rs` calls `to_agent_input()`
when building the prompt.

### AD-3: Smart Context Windowing

The `SystemPromptBuilder` gets a `ContextBudget` parameter that constrains
per-section token allocation:

```rust
pub struct ContextBudget {
    pub total_tokens: usize,
    pub identity_tokens: usize,     // 200-500
    pub role_tokens: usize,         // 500-2000
    pub task_tokens: usize,         // 1000-4000
    pub files_tokens: usize,        // 2000-32000
    pub context_tokens: usize,      // 0-16000
    pub tools_tokens: usize,        // 500-2000
    pub constraints_tokens: usize,  // 200-1000
    pub memory_tokens: usize,       // 0-4000
    pub meta_tokens: usize,         // 0-2000
}

impl ContextBudget {
    pub fn for_complexity(band: ComplexityBand) -> Self {
        match band {
            ComplexityBand::Trivial => Self {
                total_tokens: 4_096,
                identity_tokens: 200,
                role_tokens: 500,
                task_tokens: 2_000,
                files_tokens: 1_000,
                context_tokens: 0,
                tools_tokens: 300,
                constraints_tokens: 96,
                memory_tokens: 0,
                meta_tokens: 0,
            },
            ComplexityBand::Fast => Self {
                total_tokens: 16_384,
                identity_tokens: 300,
                role_tokens: 1_000,
                task_tokens: 4_000,
                files_tokens: 8_000,
                context_tokens: 2_000,
                tools_tokens: 500,
                constraints_tokens: 384,
                memory_tokens: 0,
                meta_tokens: 200,
            },
            ComplexityBand::Standard => Self {
                total_tokens: 32_768,
                identity_tokens: 500,
                role_tokens: 2_000,
                task_tokens: 4_000,
                files_tokens: 16_000,
                context_tokens: 6_000,
                tools_tokens: 1_000,
                constraints_tokens: 768,
                memory_tokens: 2_000,
                meta_tokens: 500,
            },
            ComplexityBand::Complex => Self {
                total_tokens: 65_536,
                identity_tokens: 500,
                role_tokens: 2_000,
                task_tokens: 4_000,
                files_tokens: 32_000,
                context_tokens: 16_000,
                tools_tokens: 2_000,
                constraints_tokens: 1_000,
                memory_tokens: 4_000,
                meta_tokens: 4_036,
            },
        }
    }
}
```

**Location:** `crates/roko-compose/src/context_budget.rs` (new file)
**Wiring:** `SystemPromptBuilder::build()` accepts `ContextBudget` and
enforces per-section limits.

### AD-4: Corpus Management

Corpora are stored as indexed JSON in `.roko/corpora/`:

```rust
pub struct Corpus {
    pub label: String,
    pub created_at: DateTime<Utc>,
    pub sources: Vec<CorpusSource>,
    pub total_tokens: usize,
    pub checksum: String,
}

pub struct CorpusSource {
    pub kind: SourceKind,   // File, Url, Prd, Research, Knowledge
    pub path: String,
    pub content: String,
    pub tokens: usize,
    pub metadata: HashMap<String, String>,
}
```

**Location:** `crates/roko-compose/src/corpus.rs` (new file)
**Storage:** `.roko/corpora/<label>.json`

### AD-5: Funnel Pass Architecture

Each funnel pass is an agent call with structured output and user approval:

```rust
pub enum FunnelPass {
    Architecture,   // Analyze system structure
    Gaps,           // Identify deltas from requirements
    Tasks,          // Decompose into atomic work units
    Dependencies,   // Order tasks, find parallelism
    Gates,          // Add acceptance criteria
}

pub struct FunnelCheckpoint {
    pub corpus_label: String,
    pub completed_passes: Vec<FunnelPass>,
    pub architecture_summary: Option<String>,
    pub gap_analysis: Option<String>,
    pub draft_tasks: Option<Vec<TaskSpec>>,
    pub dependency_graph: Option<DagSnapshot>,
    pub final_tasks: Option<Vec<TaskSpec>>,
}
```

**Location:** `crates/roko-cli/src/commands/funnel.rs` (new file)
**Storage:** `.roko/funnels/<corpus-label>/checkpoint.json`

### AD-6: Task Validation Engine

Validation runs as a pipeline of checks:

```rust
pub struct ValidationReport {
    pub errors: Vec<ValidationIssue>,
    pub warnings: Vec<ValidationIssue>,
    pub info: Vec<ValidationIssue>,
    pub task_count: usize,
    pub dependency_stats: DependencyStats,
    pub cost_estimate: CostEstimate,
}

pub enum ValidationCheck {
    CircularDependency,
    FileConflict,
    AcceptanceCriteriaFormat,
    ComplexityTierConsistency,
    MissingRequiredFields,
    FileExistence,
    CostEstimateReasonableness,
}
```

**Location:** `crates/roko-orchestrator/src/validation.rs` (new file)
**CLI wiring:** `crates/roko-cli/src/plan_validate.rs` (extend)

---

## Implementation Phases

### Phase 0: Workflow Foundation

**Goal:** `roko next` + run summaries + `--dry-run`. The minimum to make the
CLI user-friendly and give the user confidence that things work.

| Task | File(s) | Depends On | Effort |
|---|---|---|---|
| Add `roko next` command | new `commands/next.rs`, `main.rs` | -- | ~200 LOC |
| Implement workspace state inspection | `commands/next.rs` | -- | ~150 LOC |
| Add end-of-run summary to PlanRunner | `orchestrate.rs` | -- | ~300 LOC |
| Add `roko status --last-run` | `commands/status.rs` | -- | ~200 LOC |
| Add `--dry-run` to `roko plan run` | `orchestrate.rs`, `main.rs` | -- | ~150 LOC |
| Change bare `roko` to show `next` output | `unified.rs`, `main.rs` | roko next | ~50 LOC |
| Update help text with workflow guidance | `main.rs` | -- | ~30 LOC |

**Total:** ~1,080 LOC new code
**Dependencies:** None -- can start immediately
**Validates:** User can discover what to do next, see clear run results,
preview execution before starting.

### Phase 1: Task Metadata Tiering + Smart Context

**Goal:** Tasks get right-sized metadata and context based on complexity.
Smaller models get cleaner, slimmer prompts.

| Task | File(s) | Depends On | Effort |
|---|---|---|---|
| Define TaskSpec + TaskAgentInput types | new `roko-orchestrator/src/task_spec.rs` | -- | ~250 LOC |
| Implement to_agent_input() with tier filtering | `task_spec.rs` | types | ~150 LOC |
| Define ContextBudget type | new `roko-compose/src/context_budget.rs` | -- | ~200 LOC |
| Wire ContextBudget into SystemPromptBuilder | `system_prompt_builder.rs` | budget type | ~300 LOC |
| Wire task_spec into dispatch_helpers | `dispatch_helpers.rs` | TaskSpec | ~200 LOC |
| Add section pruning for fast tasks | `dispatch_helpers.rs` | budget | ~100 LOC |
| Add intelligent file chunking | `roko-compose/src/enrichment.rs` | roko-index | ~400 LOC |
| Add context weight enforcement | `system_prompt_builder.rs` | budget | ~150 LOC |

**Total:** ~1,750 LOC new code
**Dependencies:** Phase 0 (workflow foundation)
**Validates:** A `complexity_band = "fast"` task dispatched to haiku gets a
4K token prompt with 5-6 task fields. A `complexity_band = "complex"` task
dispatched to opus gets a 64K token prompt with 20+ fields.

### Phase 2: Task TOML Validation

**Goal:** `roko plan validate` catches errors before execution. No more
runtime failures from malformed tasks.toml.

| Task | File(s) | Depends On | Effort |
|---|---|---|---|
| Define ValidationReport + check types | new `roko-orchestrator/src/validation.rs` | -- | ~200 LOC |
| Implement circular dependency detection | `validation.rs` | DAG engine | ~120 LOC |
| Implement file conflict detection | `validation.rs` | -- | ~100 LOC |
| Implement acceptance criteria format check | `validation.rs` | -- | ~80 LOC |
| Implement complexity tier consistency | `validation.rs` | TaskSpec | ~100 LOC |
| Implement missing required fields check | `validation.rs` | TaskSpec | ~80 LOC |
| Implement file existence check | `validation.rs` | -- | ~60 LOC |
| Implement cost/time estimate validation | `validation.rs` | -- | ~80 LOC |
| Wire validation into `roko plan validate` | `plan_validate.rs` | validation engine | ~150 LOC |
| Add validation to pre-flight in `plan run` | `orchestrate.rs` | validation engine | ~50 LOC |

**Total:** ~1,020 LOC new code
**Dependencies:** Phase 1 (TaskSpec types)
**Validates:** `roko plan validate plans/` catches circular deps, file
conflicts, and malformed acceptance criteria. `roko plan run` runs
validation as pre-flight and aborts if errors found.

### Phase 3: Task Auto-Splitting

**Goal:** Complex tasks with 8+ files get auto-split into subtasks that
smaller models can handle.

| Task | File(s) | Depends On | Effort |
|---|---|---|---|
| Define SplitProposal type | new `roko-orchestrator/src/task_split.rs` | -- | ~100 LOC |
| Implement file grouping (by directory) | `task_split.rs` | -- | ~150 LOC |
| Implement file grouping (by imports) | `task_split.rs` | roko-index | ~200 LOC |
| Implement auto-dependency generation | `task_split.rs` | DAG engine | ~150 LOC |
| Implement acceptance criteria distribution | `task_split.rs` | -- | ~100 LOC |
| Add `roko task split <plan> <id>` command | new `commands/task.rs`, `main.rs` | split engine | ~200 LOC |
| Add interactive approval for splits | `commands/task.rs` | -- | ~100 LOC |
| Wire auto-split into funnel task pass | `commands/funnel.rs` | split engine | ~100 LOC |

**Total:** ~1,100 LOC new code
**Dependencies:** Phase 2 (validation), Phase 1 (TaskSpec)
**Validates:** `roko task split sprint-42 T6` proposes splitting a 10-file
task into 3 subtasks, each touching 3-4 files. User approves and the
tasks.toml is updated.

### Phase 4: Aggregation + Funneling

**Goal:** `roko ingest` and `roko funnel` support the full workflow natively.

| Task | File(s) | Depends On | Effort |
|---|---|---|---|
| Define Corpus types | new `roko-compose/src/corpus.rs` | -- | ~200 LOC |
| Implement file ingestion | `corpus.rs` | -- | ~200 LOC |
| Implement PRD-to-corpus | `corpus.rs` | roko-cli PRD code | ~150 LOC |
| Implement token counting | `corpus.rs` | roko-compose TokenCounter | ~50 LOC |
| Add `roko ingest` command | new `commands/ingest.rs`, `main.rs` | corpus types | ~300 LOC |
| Define FunnelPass + checkpoint types | `commands/funnel.rs` | -- | ~200 LOC |
| Implement architecture pass | `commands/funnel.rs` | corpus, agent dispatch | ~250 LOC |
| Implement gap analysis pass | `commands/funnel.rs` | architecture output | ~200 LOC |
| Implement task decomposition pass | `commands/funnel.rs` | gap output | ~300 LOC |
| Implement dependency analysis pass | `commands/funnel.rs` | draft tasks, DAG | ~200 LOC |
| Implement gate specification pass | `commands/funnel.rs` | draft tasks | ~200 LOC |
| Implement per-pass checkpointing | `commands/funnel.rs` | -- | ~150 LOC |
| Implement user approval between passes | `commands/funnel.rs` | -- | ~150 LOC |
| Wire auto-split into task decomposition | `commands/funnel.rs` | Phase 3 | ~50 LOC |
| Wire validation into final output | `commands/funnel.rs` | Phase 2 | ~50 LOC |

**Total:** ~2,650 LOC new code
**Dependencies:** Phase 3 (split), Phase 2 (validation), Phase 1 (TaskSpec)
**Validates:** `roko ingest docs/ crates/roko-core/src/` builds a corpus.
`roko funnel sprint-42` runs 5 passes with user approval and produces a
validated tasks.toml.

### Phase 5: Unified Chat Session

**Goal:** Consolidate 5 chat modes into one shared session. Every mode
gets cost tracking, tool output rendering, and consistent behavior.

| Task | File(s) | Depends On | Effort |
|---|---|---|---|
| Define unified ChatSession type | `chat_session.rs` (extend) | -- | ~300 LOC |
| Extract shared event loop | `chat_session.rs` | -- | ~400 LOC |
| Migrate chat_inline.rs to ChatSession | `chat_inline.rs` | shared session | ~200 LOC refactor |
| Migrate chat.rs to ChatSession | `chat.rs` | shared session | ~100 LOC refactor |
| Migrate run.rs to ChatSession | `run.rs` | shared session | ~200 LOC refactor |
| Migrate unified.rs to ChatSession | `unified.rs` | shared session | ~50 LOC refactor |
| Migrate dispatch_direct.rs to ChatSession | `dispatch_direct.rs` | shared session | ~50 LOC refactor |
| Wire inline primitives to live paths | `chat_session.rs` | shared session | ~200 LOC |

**Total:** ~1,500 LOC (mix of new + refactor)
**Dependencies:** Phase 0
**Validates:** All 5 chat modes share one event loop. Adding cost tracking
in one place works everywhere.

### Phase 6: TUI Enhancements

**Goal:** TUI becomes an actionable workflow surface, not just observation.

| Task | File(s) | Depends On | Effort |
|---|---|---|---|
| Enrich task detail modal with full metadata | `modals/task_detail.rs` | TaskSpec | ~300 LOC |
| Add funnel progress view to Atelier tab | `views/atelier_view.rs` | funnel types | ~400 LOC |
| Wire batch review modal to orchestrator | `modals/batch_review.rs`, `orchestrate.rs` | -- | ~200 LOC |
| Add pause/resume keyboard shortcuts | `input.rs`, `orchestrate.rs` | -- | ~100 LOC |
| Add per-task approve/reject in dashboard | `views/dashboard_view.rs` | event bus | ~300 LOC |
| Add cost tracking to header bar | `widgets/header_bar.rs` | CostMeter | ~50 LOC |
| Add complexity tier indicators to plan tree | `widgets/plan_tree.rs` | TaskSpec | ~100 LOC |

**Total:** ~1,450 LOC
**Dependencies:** Phase 1 (TaskSpec), Phase 4 (funnel)
**Validates:** Task detail modal shows all agent-visible fields plus gate
output. User can pause/resume execution and approve/reject tasks from the
TUI.

---

## Dependency Graph

```
Phase 0 (Foundation: next, summary, dry-run)
  |
  +---> Phase 1 (Metadata tiering + context windowing)
  |       |
  |       +---> Phase 2 (Task TOML validation)
  |               |
  |               +---> Phase 3 (Task auto-splitting)
  |                       |
  |                       +---> Phase 4 (Aggregation + funneling)
  |                               |
  |                               +---> Phase 6 (TUI enhancements)
  |
  +---> Phase 5 (Unified chat session) [parallel with Phase 1-4]
```

**Critical path:** Phase 0 -> Phase 1 -> Phase 2 -> Phase 3 -> Phase 4

Phase 5 (unified chat session) is a parallel workstream that can proceed
independently after Phase 0.

Phase 6 (TUI enhancements) depends on Phase 1 and Phase 4 for the types
and funnel workflow.

---

## Effort Estimates

| Phase | New LOC | Refactor LOC | Total | Notes |
|---|---|---|---|---|
| Phase 0: Foundation | ~1,080 | ~0 | ~1,080 | Quick wins, immediate UX improvement |
| Phase 1: Metadata + Context | ~1,750 | ~0 | ~1,750 | Core quality improvement |
| Phase 2: Validation | ~1,020 | ~0 | ~1,020 | Pre-flight safety |
| Phase 3: Auto-splitting | ~1,100 | ~0 | ~1,100 | Unlock small-model quality |
| Phase 4: Aggregate + Funnel | ~2,650 | ~0 | ~2,650 | Full workflow native |
| Phase 5: Unified Chat | ~700 | ~800 | ~1,500 | Consolidate 5 modes |
| Phase 6: TUI | ~1,450 | ~0 | ~1,450 | Actionable observation |
| **Total** | **~9,750** | **~800** | **~10,550** | |

---

## Risk Mitigations

### R1: Another parallel universe of types

**Risk:** TaskSpec + TaskAgentInput become yet another set of task types
alongside the existing plan/task types in roko-orchestrator.
**Mitigation:** TaskSpec replaces the existing task types, not supplements
them. Migration path: update existing code to use TaskSpec, deprecate old
types.

### R2: Funnel workflow becomes too agent-heavy

**Risk:** 5 agent calls per funnel run is expensive for iteration. At ~$2
per Opus call, a full funnel is $10 before any execution.
**Mitigation:** Use sonnet for passes 1-4 (architecture, gaps, tasks, deps).
Use opus only for pass 5 (gates) where accuracy matters most. Total funnel
cost: ~$3-5. Or: allow the user to skip passes and manually provide
artifacts for any pass.

### R3: Context windowing breaks working prompts

**Risk:** Existing prompts that work with full context may break when context
is reduced for fast tasks.
**Mitigation:** Default to `standard` context weight for all existing tasks.
Only apply slim/deep context for tasks that explicitly set `context_weight`
or `complexity_band`. Existing behavior is preserved unless the user opts
into tiering.

### R4: Validation becomes a gate that blocks execution

**Risk:** Aggressive validation prevents legitimate runs because the
validator is too strict.
**Mitigation:** Validation errors block execution. Validation warnings
are shown but do not block. `--skip-validation` flag available as escape
hatch.

### R5: Five-to-one chat session migration breaks things

**Risk:** Consolidating 5 chat modes into one shared session introduces
regressions in each mode.
**Mitigation:** Implement ChatSession as a new internal module. Migrate
one mode at a time. Run existing tests after each migration. Keep the old
code as dead code until all modes are migrated and tested.

---

## Implementation Order Rationale

**Why Phase 0 first:** The user's primary complaints are "CLI isn't user-
friendly" and "can't be sure things work." Phase 0 directly addresses both
with `roko next` (guidance) and run summaries + dry-run (confidence). These
are small changes with immediate impact.

**Why Phase 1 before Phase 2:** Task metadata tiering establishes the type
system that validation checks against. You cannot validate complexity tier
consistency without first defining what the tiers are.

**Why Phase 3 before Phase 4:** Auto-splitting is needed during the funnel's
task decomposition pass. Building it first means the funnel can use it.

**Why Phase 5 in parallel:** Unifying chat modes is important for maintenance
but does not block any workflow improvement. It can proceed independently.

**Why Phase 6 last:** TUI enhancements are observation improvements. The
workflow improvements (Phases 0-4) deliver more value sooner.

---

## File Paths for Implementation

| New File | Phase | Purpose |
|---|---|---|
| `crates/roko-cli/src/commands/next.rs` | 0 | `roko next` command |
| `crates/roko-orchestrator/src/task_spec.rs` | 1 | TaskSpec + TaskAgentInput |
| `crates/roko-compose/src/context_budget.rs` | 1 | ContextBudget type |
| `crates/roko-orchestrator/src/validation.rs` | 2 | Validation engine |
| `crates/roko-orchestrator/src/task_split.rs` | 3 | Task splitting engine |
| `crates/roko-cli/src/commands/task.rs` | 3 | `roko task` commands |
| `crates/roko-compose/src/corpus.rs` | 4 | Corpus management |
| `crates/roko-cli/src/commands/ingest.rs` | 4 | `roko ingest` command |
| `crates/roko-cli/src/commands/funnel.rs` | 4 | `roko funnel` command |

| Modified File | Phase | Change |
|---|---|---|
| `crates/roko-cli/src/main.rs` | 0,3,4 | Add new Command variants |
| `crates/roko-cli/src/commands/mod.rs` | 0,3,4 | Re-export new modules |
| `crates/roko-cli/src/orchestrate.rs` | 0,2 | Run summary, dry-run, pre-flight validation |
| `crates/roko-cli/src/commands/status.rs` | 0 | --last-run flag |
| `crates/roko-cli/src/unified.rs` | 0 | Change bare invocation to show next |
| `crates/roko-cli/src/dispatch_helpers.rs` | 1 | Wire TaskAgentInput, section pruning |
| `crates/roko-compose/src/system_prompt_builder.rs` | 1 | ContextBudget enforcement |
| `crates/roko-cli/src/plan_validate.rs` | 2 | Wire validation engine |
| `crates/roko-cli/src/chat_session.rs` | 5 | Unified ChatSession |
| `crates/roko-cli/src/tui/modals/task_detail.rs` | 6 | Full metadata display |
| `crates/roko-cli/src/tui/modals/batch_review.rs` | 6 | Wire to orchestrator |

---

## Sources

Existing infrastructure this plan builds on:

- `crates/roko-cli/src/main.rs` -- CLI entry, Command enum (extend with new variants)
- `crates/roko-cli/src/orchestrate.rs` -- PlanRunner (add summary, dry-run, pre-flight)
- `crates/roko-cli/src/commands/` -- command handlers (add next, ingest, funnel, task)
- `crates/roko-cli/src/dispatch_helpers.rs` -- prompt assembly (wire TaskAgentInput)
- `crates/roko-cli/src/plan_validate.rs` -- plan validation (extend with semantic checks)
- `crates/roko-cli/src/chat_session.rs` -- ChatAgentSession (extend to unified session)
- `crates/roko-compose/src/system_prompt_builder.rs` -- 9-layer builder (add budget)
- `crates/roko-compose/src/enrichment.rs` -- EnrichmentPipeline (add file chunking)
- `crates/roko-orchestrator/src/` -- DAG computation (add validation, splitting)
- `crates/roko-cli/src/tui/modals/task_detail.rs` -- 177 LOC (extend to full metadata)
- `crates/roko-cli/src/tui/modals/batch_review.rs` -- 164 LOC (wire to orchestrator)
