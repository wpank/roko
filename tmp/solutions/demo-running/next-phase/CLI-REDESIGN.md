# CLI Redesign — Unified Synthesis

**Source documents synthesized**: 14 files across tmp/subsystem-audits, tmp/mori-diffs,
tmp/workflow, tmp/solutions/roko, tmp/binary-issues, tmp/demo-req, tmp/dogfood,
tmp/learnings3.

This document supersedes all of them on matters of CLI UX direction. It describes
what the CLI should become, why, and how to get there.

---

## Part 1: The Problem — Evidence, Not Opinion

### 1.1 Structural problems

The roko CLI has accumulated three layers of dysfunction:

**Layer 1: Too many commands, wrong abstraction level.**
35+ top-level subcommands at 3–4 levels deep (`roko knowledge dream journal`,
`roko config providers health`). The taxonomy reflects the internal architecture —
PRDs, plans, agents, research, knowledge, learn — not the user's mental model, which
is simply "I want to do something." Users have to know the taxonomy before they can
start. Nobody remembers `roko prd draft new` vs `roko plan generate` vs `roko run`.

**Layer 2: Commands that lie (confirmation theater).**
From binary-issues S3: `/system` stores a message but dispatch ignores it. `/effort`
prints "set to X" but stores nothing. `/gate` prints a hint but changes nothing.
`/config set` prints a confirmation but writes nothing. `roko learn tune gates` has a
`--dry-run` flag but the command never writes. These are not minor bugs — they destroy
trust. The user believes they're configuring the system, but they're not.

**Layer 3: Two parallel engines, two chat modes, two init paths.**
From binary-issues S10: runner v2 AND legacy PlanRunner coexist. `chat_inline.rs` has
two event loops. `chat.rs` (659 LOC) and `chat_inline.rs` (4,100 LOC) both implement
chat. `roko init` and `roko config init` are separate unconnected commands. Every bug
fix must be applied twice.

### 1.2 Evidence from actual usage

From `tmp/subsystem-audits/05-01/42-workflow-redesign-suggestion.md`, examining
`.roko/`, `tmp/`, git history, and artifact state:

**PRD pipeline**: 4 ideas captured, all duplicates. 1 skeleton draft. 0 PRDs
published. 0 PRD-driven plans completed. The full PRD→plan→run pipeline has never
been executed end-to-end by a human.

**Plans**: 6 directories, 2 completed (34% task completion). Plans that shipped were
created directly, not generated from PRDs.

**Episodes**: 22 total, mostly failed dispatch initialization. Zero successful task
completions recorded in the learning system at time of audit.

**Default agent is `cat`.** `AgentConfig::default()` sets `command: "cat".into()`
(binary-issues doc 18, CF2). So `roko run "fix this"` without a config echoes the
prompt back as output and reports success. The CLI warns about this on the `roko run`
path but not on other paths.

**The real workflow**: The user opens Claude Code, describes what they want in natural
language, the agent does it. Roko's 35-subcommand taxonomy exists alongside this but
is not the primary interface.

**What actually gets used**:

| Surface | Used? | How |
|---|---|---|
| `roko` (no args, interactive chat) | Yes | Primary entry — talks to Claude |
| `roko run` | Sometimes | Quick single prompts |
| `roko dashboard` | Rarely | Occasional check, not primary |
| `roko serve` | For demo | Backend for demo-app, not daily |
| 35+ subcommands | Almost never | Too much taxonomy to internalize |

### 1.3 What Claude Code gets right

The user has `CLAUDE_CODE_DISABLE_ADAPTIVE_THINKING=1`, `CLAUDE_CODE_DISABLE_BACKGROUND_TASKS=1`,
`CLAUDE_CODE_EFFORT_LEVEL=max`. The pattern: **maximum capability, maximum transparency,
minimum ceremony.** You say what you want and it does it. There is no `claude prd idea
"fix the bug"`. There is no `claude plan generate`. There is no `--resume
.roko/state/executor.json`.

This is the standard roko must meet.

---

## Part 2: Core Principle — Progressive Formality

The workflow should be a spectrum, not a pipeline. The system escalates formality
automatically based on complexity, risk, and stakes. The user does not pick the level.

```
Instant          Light           Planned         Managed
─────────────────────────────────────────────────────────
"fix typo"    "refactor auth"  "rewrite frontend"  "migrate to v2"
│                │                │                   │
│  direct exec   │  auto-plan     │  plan review       │  multi-agent
│  no approval   │  auto-execute  │  approval gates    │  staged rollout
│  1 agent       │  1 agent       │  1-3 agents        │  N agents
│  no PRD        │  no PRD        │  optional PRD      │  PRD required
│  seconds       │  minutes       │  hours             │  days
```

The user says what they want. The system picks the level and can be overridden.

### 2.1 Intent classification

Inside `roko do`, the system classifies the intent automatically:

```
1. Trivial (< 1 file, < 10 lines, obvious fix)
   → Direct execution, no plan, no approval
   → "Fixed the typo in README.md line 42"

2. Small (1-3 files, clear scope)
   → Auto-plan (invisible), auto-execute
   → Shows diff when done, asks to commit
   → "Changed auth middleware in 2 files. Commit?"

3. Medium (3-10 files, needs decomposition)
   → Shows plan, waits for approval (unless --yes)
   → Executes with progress streaming
   → Gates validate each step
   → "Here's my plan (4 tasks). Proceed?"

4. Large (10+ files, architectural)
   → Creates a named work item with full plan
   → Multi-agent execution
   → Approval gates between phases
   → Resumable across sessions
   → "This is a big change. I've created work item 'auth-redesign'. Review?"

5. Ambiguous
   → One clarifying question max, then classifies
   → "Do you want me to fix the null check, or redesign error handling?"
```

Initial classification uses keyword heuristics (fast, no LLM call). Ambiguous cases
escalate to a lightweight LLM classification. From implementation plan 11:

```rust
impl WorkflowConfig {
    pub fn auto_select(prompt: &str) -> WorkflowConfig {
        let words = prompt.split_whitespace().count();
        let lower = prompt.to_lowercase();
        let express_kw = ["fix typo", "rename", "update", "add comment"];
        let full_kw    = ["refactor", "redesign", "architecture", "rewrite"];

        if words < 15 && express_kw.iter().any(|k| lower.contains(k)) {
            WorkflowConfig::express()
        } else if words > 50 || full_kw.iter().any(|k| lower.contains(k)) {
            WorkflowConfig::full()
        } else {
            WorkflowConfig::standard()
        }
    }
}
```

---

## Part 3: Design — 5 Primary Verbs

Everything roko does maps to 5 verbs. Every surface (CLI, chat, TUI, API) exposes
the same verbs.

### 3.1 The verbs

| Verb | What it does | Formality |
|---|---|---|
| `do` | Execute something | Adapts: instant → managed |
| `think` | Research, analyze, explain | Always lightweight, no side effects |
| `show` | Display state, progress, history | Read-only |
| `tune` | Adjust preferences, models, thresholds | Configuration |
| `undo` | Revert, cancel, roll back | Corrective |

Everything else — `prd`, `plan`, `research`, `knowledge`, `learn`, `dashboard`,
`status`, `deploy`, `daemon`, `agent`, `config`, `index`, `explain`, `inject`,
`replay`, `new`, `completions` — is either a sub-operation of these 5 or an
implementation detail users should not think about.

### 3.2 `do` — The universal entry point

```
roko do "wire auth into the login flow"
```

Internally runs: intent classify → formality select → model select → execute
(streaming output, live progress) → gate validate → record episode.

User overrides when needed:
- `roko do --plan "fix typo"` — forces a plan even for trivial work
- `roko do --yes "refactor auth"` — skips approval, auto-executes
- `roko do --review "refactor"` — forces per-step review
- `roko do --ghost "redesign auth"` — dry-run, shows full plan, cost estimate, models,
  gates without executing

### 3.3 `think` — Research without action

```
roko think "what patterns exist for rate limiting in this codebase?"
roko think "how should we handle auth tokens?"
```

No code changes. Returns analysis, citations, recommendations. Feeds into a subsequent
`do` as context (see context carry in section 5.4). Replaces: `research topic`,
`research search`, `knowledge query`, `explain`.

### 3.4 `show` — State inspection

```
roko show                — Overview: agents, plans, recent activity, learning state
roko show costs          — Cost breakdown by model, task, time period
roko show agents         — Active agents, roles, status
roko show knowledge      — What the system has learned
roko show plans          — Plans in progress, completion %
roko show history        — Recent executions with outcomes
roko show auth-redesign  — Detail on a specific work item
roko show learning       — Routing confidence, model performance, gate drift
```

Replaces: `status`, `dashboard`, `learn all`, `learn efficiency`, `plan list`,
`agent list`.

### 3.5 `tune` — Behavior adjustment

```
roko tune routing      — Adjust model routing preferences
roko tune gates        — Change validation strictness
roko tune budget       — Set cost limits
roko tune style        — Output preferences (verbose/quiet/streaming)
roko tune model sonnet — Default model for this workspace
```

Replaces: `learn tune *`, `config set`, `config experiments`. These commands must
actually write — no more confirmation theater (S3 in binary-issues).

### 3.6 `undo` — Reversibility (currently missing)

```
roko undo              — Undo last action
roko undo auth-redesign — Cancel specific work item
roko undo --list       — Show reversible actions
```

Currently no equivalent. This is critical for user confidence — users need to know
they can experiment without consequence. The absence of `undo` makes every action
feel high-stakes.

---

## Part 4: Output — Clack-Style Inline Rendering

Replace raw terminal dump with structured inline output. This is Tier 0 for
demo-readiness (from `tmp/learnings3/08-BUILD-PLAN.md`): raw debug output signals
"prototype," formatted output signals "product." The entire perception of the tool
changes.

### 4.1 Target output format

```
◆ roko do "add health check endpoint"

│ Classifying intent... small (single endpoint, 2-3 files)
│
│ ┌ Plan: 3 tasks
│ │  1. Create /health route handler
│ │  2. Wire into axum router
│ │  3. Add integration test
│ └
│
│ ● Task 1/3: Create route handler
│   └ Writing crates/roko-serve/src/routes/health.rs
│     + 24 lines
│
│ ● Task 2/3: Wire into router
│   └ Editing crates/roko-serve/src/routes/mod.rs
│     ~ 2 lines changed
│
│ ● Task 3/3: Integration test
│   └ Writing tests/health_check.rs
│     + 18 lines
│
│ ◆ Gates
│   compile (0.3s)  ok
│   clippy  (1.2s)  ok
│   test    (2.1s)  ok
│
└ Done in 11s · $0.02 · 3/3 tasks · haiku+sonnet
```

### 4.2 Welcome banner (10A from binary-issues quality-of-life doc)

```
◆ roko v0.1.0  ·  claude-sonnet-4-6  ·  auth: API key
│ workspace: /Users/will/dev/project  ·  .roko/ initialized
│ 3 plans  ·  12 tasks  ·  $0.42 lifetime spend
└ Type a message. Ctrl-D to exit. /help for commands.
```

### 4.3 Agents list (from demo-build P0-2)

```
◆ Agents (workspace)
│
│  NAME        MODEL           STATUS
│  researcher  claude-haiku    active
│  auditor     claude-sonnet   active
│  reviewer    gpt-4o-mini     idle
│
└  3 agents registered · 2 active · 1 idle
```

### 4.4 The 18 inline output primitives

From `tmp/demo-req/IMPLEMENTATION-PLAN.md`, built on ratatui `Viewport::Inline(N)`:

**For plan execution:**
- `RunBlock` — completed run summary, pushed to scrollback
- `StreamingBlock` — live agent output with token-by-token append
- `ToolCallBlock` — collapsed by default, expandable (already built in `inline/primitives/tool_call.rs`, not yet wired)
- `GateBlock` — gate pipeline progress, each rung updates in place

**For display:**
- `CostMeter` — per-session cost accumulator (already built in `inline/primitives/cost_meter.rs`)
- `KnowledgeBlock` — knowledge loaded from neuro store
- `PredictionBlock` — cost estimate + model route before execution
- `ErrorBlock` — error with pattern-matched recovery suggestions (10F from quality-of-life doc)

**For progress:**
- `SpinnerLine` — inline spinner for async operations
- `CheckmarkLine` / `WarningLine` — step completion indicators
- `ProgressBar` — task completion within a plan

**For structure:**
- `AgentCard` — agent identity, model, role, status
- `TaskCard` — task detail with acceptance criteria and gate output
- `PlanOverview` — DAG view with wave execution visualization
- `CompletionSummary` — end-of-run summary (currently missing — I-UX02)

**For code:**
- `DiffBlock` — git diff for changes made (built in `inline/primitives/diff_block.rs`, not yet wired)
- `FileWriteBlock` — file creation/modification indicator
- `RichErrorBlock` — structured rustc/clippy errors with file path, line number, suggestion

**Key insight from implementation plan 16**: The ratatui `Viewport::Inline(N)` approach
(not alternate screen) is what makes this composable. Completed output scrolls up into
terminal history via `insert_before()`. The live viewport is at the bottom. This is
how Claude Code works (with React/Ink) but in Rust with ~20MB memory vs ~300MB.

---

## Part 5: WorkflowEngine Architecture

### 5.1 One engine, all paths

All 5 verbs go through ONE engine. From the `WorkflowRequest→WorkflowResolver→WorkflowPlan`
pattern identified in `tmp/mori-diffs/36-WORKFLOW-ENTRYPOINT-ORCHESTRATION-AUDIT.md`:

```
User Intent
    ↓
WorkflowRequest (verb, prompt, context, constraints)
    ↓
WorkflowResolver (intent classification, formality selection)
    ↓
WorkflowPlan (tasks, models, gates, budget, approval gates)
    ↓
StepExecutor (streaming, gates, learning, persistence)
    ↓
WorkflowResult (outcome, costs, episodes, knowledge)
```

This replaces:
- `orchestrate.rs` PlanRunner (legacy, currently default for `plan run`)
- `runner/event_loop.rs` (v2, currently the "new" path)
- `chat.rs` event loop (659 LOC parallel REPL)
- `run.rs` single-shot dispatch
- `prd.rs` pipeline orchestration
- `agent_exec.rs` bespoke spawn helpers

From implementation plan 11 (entry point coverage matrix), the current state is:

| Entry Point | WorkflowEngine? | Notes |
|---|---|---|
| `roko` (interactive chat) | No | Highest-volume path |
| `roko "prompt"` (one-shot) | No | |
| `roko chat` (REPL) | No | Parallel to `roko` |
| `roko run "<prompt>"` | YES (v2) | |
| `roko plan run plans/` | No (`runner/event_loop.rs`) | Default path |
| `roko prd refine`, research | No (`agent_exec.rs::spawn_agent_scoped`) | |
| `roko acp` (workflow mode) | YES | |
| HTTP `POST /api/inference/complete` | No (passthrough) | |

The goal: every entry point uses `WorkflowEngine` for any operation involving more
than a single bare model call.

### 5.2 WorkflowConfig templates

```rust
WorkflowConfig::express()    // < 2 files, no review, no strategy
WorkflowConfig::standard()   // 2-10 files, auto-plan, auto-execute
WorkflowConfig::full()       // 10+ files, plan review, multi-agent, resumable
WorkflowConfig::auto_select(prompt)  // heuristic classification
```

### 5.3 Aggregate → Funnel → Execute (for large work)

When `roko do` classifies something as Large, it runs the full pipeline internally.
This is what the PRD→plan→run pipeline was trying to be, but automated:

**Aggregate**: Gather context (codebase, docs, prior knowledge, research). Replaces
the proposed `roko ingest` command — aggregation happens inside `do` when needed.

**Funnel** (5 passes):
1. Architecture — analyze system structure, what changes where
2. Gaps — identify delta from requirements, what's missing
3. Tasks — decompose into atomic work units for a single agent
4. Dependencies — order tasks, find parallelism, validate DAG
5. Gates — add acceptance criteria and validation to each task

From `tmp/solutions/roko/09-UX-WORKFLOW-VISION.md`: each pass is an agent call with
structured output and user approval. Checkpoints saved to `.roko/funnels/`. The user
can interrupt and resume at any pass.

**Execute**: Plan run with streaming, gates, learning. Same engine as Medium work,
just with more tasks and more agents.

The user never invokes `roko ingest` or `roko funnel` directly. These are internal
phases of `roko do --large` or any task classified as Large.

### 5.4 Context carry — sessions remember across invocations

Today each CLI invocation is stateless. `roko run "fix auth"` does not know about
`roko think "how does auth work?"` from 5 minutes ago.

Context carry: each invocation appends to a lightweight session persisted at
`.roko/sessions/current.json`. The `think` output becomes context for the next `do`.
Sessions auto-expire after configurable idle time (default: 1 hour).

```bash
roko think "how does the auth module work?"
# → analyzes codebase, returns explanation, records context

roko do "refactor it to use JWT"
# → "it" resolves to auth module from previous invocation
# → analysis from `think` injected as context into the plan
# → saved $0.40 in context reconstruction

roko show
# → shows both the research and the refactor, linked
```

### 5.5 Ghost mode — dry-run with full transparency

```bash
roko do --ghost "redesign the auth system"
```

Output before any execution:

```
◆ Ghost Run: redesign-auth

│ Classified: Large (architectural)
│ Would create: named work item + 8-task plan
│ Agents: 3 (implementer x2 + reviewer x1)
│ Models: opus (plan) → sonnet (implement) → opus (review)
│ Gates: compile → test → clippy → diff-review
│ Estimated cost: $3.20 – $5.40
│ Estimated time: 15-25 minutes
│
│ Plan:
│   1. Extract auth types (sonnet, ~$0.30)
│   2. Implement JWT middleware (sonnet, ~$0.60)
│   3. Wire into request pipeline (sonnet, ~$0.45)
│   ...
│
└ Run this? [y]es  [e]dit plan  [n]o
```

The user has disabled adaptive thinking in Claude Code because they want predictability.
Ghost mode gives predictability through transparency — see the full decision-making
before any work starts.

### 5.6 Batch mode — the missing middle

The most productive workflow observed is multi-batch agent runs. This has no CLI
representation today — users reach for Claude Code or Codex instead.

```bash
roko do --batch <<EOF
fix the null check in auth.rs
add error handling to the token refresh
update the tests for the new auth flow
EOF

# 3 work items created, classified independently
# trivial ones execute immediately
# complex ones plan and await approval
# all run in parallel where possible
# unified progress view

# Or pipe:
cat issues.txt | roko do --batch
```

---

## Part 6: Work Items as First-Class Objects

Today "work" is scattered across PRDs, plans, tasks, episodes, executor snapshots,
and signals — each with its own format, storage, and lifecycle. The user sees none of
this coherently.

A **work item** is the single user-facing concept:

```rust
pub struct WorkItem {
    pub id: String,           // "auth-redesign"
    pub status: WorkStatus,   // Running | Paused | Done | Failed
    pub created: DateTime<Utc>,
    pub prompt: String,       // "redesign the auth system to use JWT"
    pub formality: Formality, // auto-classified: Trivial | Small | Medium | Large

    // Internal, not user-facing
    prd:      Option<PrdRef>,
    plan:     Option<PlanRef>,
    tasks:    Vec<TaskRef>,
    episodes: Vec<EpisodeRef>,
    git_branch: Option<String>,
    cost:     CostSummary,
}
```

Users interact with work items, not the underlying machinery:

```
roko show

  auth-redesign     Running   3/7 tasks   $0.42
  fix-login-bug     Done      1/1 tasks   $0.03
  readme-typo       Done      -           $0.01
```

Work items replace `.roko/state/executor.json` as the user-facing state artifact.
`--resume .roko/state/executor.json` becomes `roko do --continue auth-redesign` or
just `roko do` with no args, which offers to resume whatever was in progress.

---

## Part 7: Ambient Learning — Make the Invisible Visible

The learning subsystem records everything but surfaces nothing. CascadeRouter has
37 model slugs and 28 role→model mappings. Gate thresholds are EMA-adjusted.
Efficiency data is logged per-turn. None of this reaches the user.

### 7.1 Surface learning as pre-execution context

```
you> refactor the auth module

roko> Using opus for this — it's handled 4 similar refactors with 100%
      gate pass rate. Sonnet was 2/3 on auth-related work. Estimated cost:
      $1.20 based on similar episodes. Last auth refactor: 12 minutes.

      Plan (3 tasks): [...]

      Proceed?
```

The system explains its choices using its own learning data. This is the
"roko is getting better" signal that is currently invisible.

### 7.2 Learning summary via `roko show learning`

```
◆ Model Performance (last 30 days)

│  opus     12 tasks   92% pass   avg $1.40   avg 8min
│  sonnet   31 tasks   87% pass   avg $0.35   avg 3min
│  haiku     8 tasks   75% pass   avg $0.08   avg 1min
│
│  Routing confidence: 0.78 (improving — was 0.62 a week ago)
│  Gate threshold drift: clippy -2%  test +5%
│
└  37 models learned · 28 role→model mappings
```

---

## Part 8: Task Metadata Architecture

From `tmp/solutions/roko/15-UX-ISSUES.md` (I-UX04) and `tmp/solutions/roko/15-UX-PLAN.md`
(AD-2): task TOMLs carry 20+ metadata fields that all flow into agent prompts.
A haiku-class agent working on a trivial task receives `reasoning_level`, `quality_profile`,
`preferred_model`, `escalate_on_retry` — none of which it can use.

### 8.1 Two-struct separation

```rust
/// Full task spec — persisted to disk and API.
pub struct TaskSpec {
    // Agent-visible: always in prompt
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub files: Vec<String>,
    pub depends_on: Vec<String>,
    pub acceptance: Vec<String>,

    // Routing-only: NEVER in agent prompt
    pub preferred_model: Option<String>,
    pub complexity_band: Option<ComplexityBand>,
    pub reasoning_level: Option<String>,
    pub escalate_on_retry: Option<bool>,
    // ...
}

/// Agent-visible input — stripped of routing metadata.
pub struct TaskAgentInput {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub files: Vec<String>,
    pub depends_on: Vec<String>,
    pub acceptance: Vec<String>,
    // Only conditional fields for standard+ tiers:
    pub category: Option<TaskCategory>,
    pub estimated_minutes: Option<u32>,
    pub example_pattern: Option<String>,
}
```

Routing-only fields are stripped when building the agent prompt. `to_agent_input()`
filters based on `complexity_band`.

### 8.2 Context budgets per complexity tier

```rust
pub struct ContextBudget {
    pub total_tokens: usize,
    pub identity_tokens: usize,
    pub role_tokens: usize,
    pub task_tokens: usize,
    pub files_tokens: usize,
    pub context_tokens: usize,
    pub memory_tokens: usize,
    // ...
}

impl ContextBudget {
    pub fn for_complexity(band: ComplexityBand) -> Self {
        match band {
            ComplexityBand::Trivial => Self { total_tokens: 4_096, ... },
            ComplexityBand::Fast    => Self { total_tokens: 16_384, ... },
            ComplexityBand::Standard => Self { total_tokens: 32_768, ... },
            ComplexityBand::Complex  => Self { total_tokens: 65_536, ... },
        }
    }
}
```

A trivial task gets a 4K token prompt with 5-6 fields. A complex task gets a 64K
token prompt with 20+ fields. The 9-layer `SystemPromptBuilder` enforces per-section
budgets.

---

## Part 9: Rendering Architecture

### 9.1 `ResponseRenderer` trait

From `tmp/workflow/implementation-plans/16-cli-tui-rendering-convergence.md`:

```rust
pub trait ResponseRenderer: Send {
    fn render_text(&mut self, text: &str);
    fn render_thinking_delta(&mut self, text: &str);
    fn render_tool_call(&mut self, call: &ToolCallEvent);
    fn render_tool_output(&mut self, output: &ToolOutputEvent);
    fn render_cost(&mut self, summary: &CostSummary);
    fn render_gate_verdict(&mut self, verdict: &GateVerdict);
    fn render_error(&mut self, error: &str, suggestions: &[String]);
    fn finalize(&mut self);
}
```

Implementations:

- `InlineRenderer` — uses `inline/primitives/` for `roko` and `roko run`
- `PlainRenderer` — no styling, for `roko -q` / non-TTY
- `TuiRenderer` — writes to `TuiState`, for dashboard Agents tab

### 9.2 Single chat loop

`chat_inline.rs` has two event loops (`run_chat_inline` HTTP-sidecar and
`run_unified_inline` direct) duplicating ~700 LOC each. Both become 30-LOC wrappers
over a shared `run_chat_loop<R: ResponseRenderer, B: ChatBackend>`.

The `ChatBackend` trait:
```rust
pub trait ChatBackend: Send {
    async fn send_turn(&mut self, prompt: String) -> Result<DispatchResult>;
    async fn cancel(&self) -> Result<()>;
}

pub struct DirectModelCallBackend { service: Arc<ModelCallService> }
pub struct HttpSidecarBackend { client: HttpClient, agent_id: String }
```

After migration: `chat_inline.rs` shrinks from 4,100 LOC to ~1,500 LOC.
`chat.rs` (659 LOC parallel REPL) is deleted or becomes a 30-LOC thin wrapper
calling `run_chat_loop` with `PlainRenderer`.

### 9.3 TUI on RuntimeProjection

Today the TUI loads disk independently (`tui/dashboard.rs` 6,382 LOC loads `.roko/`
files on every refresh). After migration, the TUI reads from `RuntimeProjection`
(the in-memory state already maintained by WorkflowEngine), eliminating disk I/O
on every render. Target: TUI startup < 500ms (was ~2s).

---

## Part 10: HTTP API — 7 Routes Behind 85

The ~85 serve routes stay as internal implementation. Externally, 7 verb-aligned
routes serve as the stable interface:

```
POST /api/do            → start work (prompt, formality override, batch)
POST /api/think         → research/analyze (no side effects)
GET  /api/show          → list work items, sessions, learning state
GET  /api/show/:id      → detail of specific work item
POST /api/tune          → update config, thresholds, routing
POST /api/undo          → revert, cancel, pause
GET  /api/stream/:id    → SSE event stream for work item (replaces polling)
```

Push-based progress via SSE replaces the current pattern of running `roko plan run`
in the foreground and watching nothing, or opening the TUI and polling. Every `do`
operation emits structured `DashboardEvent`s that SSE subscribers receive in real time.

---

## Part 11: TUI — Mission Control, Not Dashboard

The current TUI has 10 tabs (F1-F10) organized by subsystem (one tab per feature).
The proposed layout is workflow-organized (one tab per need):

```
F1: Home
    Active work items with live progress
    Quick action: [d]o  [t]hink  [s]how  [u]ndo
    Learning summary (one line: "routing confidence: 0.78, 31 episodes today")
    Cost burn since midnight

F2: Work (replaces Plans + Agents + Atelier)
    Left: work item list (filterable by status)
    Right: selected item — plan, current task, agent output, diff
    Bottom: streaming agent output for active work
    Actions: pause, resume, cancel, approve, intervene

F3: Observe (replaces Logs + Inspect + Learning)
    Live DashboardEvent stream (structured, not raw logs)
    Filter by: work item, agent, gate, model, severity
    Cost graph (burn rate over time)
    Model routing decisions (which model, why, outcome)

F4: Configure (replaces Config)
    Live config editor with validation
    Provider health
    Model routing overrides
    Gate threshold adjustments
```

4 tabs instead of 10. The existing widget infrastructure, modal system, and event bus
are reused — only the organization changes.

---

## Part 12: Command Mapping — Before and After

### 12.1 Before (today's workflow)

```bash
roko prd idea "add auth"
roko prd draft new "auth"
roko research enhance-prd auth
roko prd plan auth
roko plan run plans/auth/
roko plan run plans/auth/ --resume .roko/state/executor.json
roko dashboard
```

7 commands. The user must know the taxonomy, manage files, pass paths and flags.
The pipeline has never been run end-to-end by a human.

### 12.2 After (proposed)

```bash
roko do "add auth to the login flow"
```

One command. The system classifies complexity, generates a plan, shows it, executes,
gates, records, learns. Resume is automatic — `roko do` with no args offers to
continue what was in progress.

### 12.3 Subcommands that survive (as-is or renamed)

| Current | Becomes | Why |
|---|---|---|
| `roko` (no args) | `roko` (interactive session) | Primary entry, conversation-first |
| `roko run "prompt"` | `roko do "prompt"` | Direct intent, one-shot |
| `roko chat` | Deleted (merged into `roko`) | No distinction from `roko` |
| `roko status` | `roko show` | Read-only state, cleaner name |
| `roko dashboard` | `roko show --live` or F1 | Same state, TUI mode |
| `roko serve` | `roko serve` | Infrastructure, stays |
| `roko init` | `roko init` | Setup, stays (unified — see 12.4) |

### 12.4 Subcommands that become internal or deprecated

| Current | Absorbed into | Notes |
|---|---|---|
| `roko prd *` | `roko do` (large formality) | PRDs are internal artifacts, not user concepts |
| `roko plan *` | `roko do` (medium+ formality) | Plans generated automatically |
| `roko research *` | `roko think` | Research is thinking |
| `roko agent *` | Internal to `do` | Agents are implementation |
| `roko knowledge *` | `roko show knowledge` | Read-only inspection |
| `roko learn *` | `roko show learning` + `roko tune` | Split by read/write |
| `roko config *` | `roko tune` + `roko init` | Configuration |
| `roko deploy *` | `roko do --deploy` | Deployment is an action |

Old commands kept as deprecated aliases for one release, then removed. The user base
for these commands is the self-hosted system itself — breakage is manageable.

### 12.5 Two init paths → one (binary-issues CF1, S10.6)

`roko init` and `roko config init` are unconnected. `roko init` creates `.roko/`
and `roko.toml` non-interactively. `roko config init` runs an interactive wizard
for `~/.roko/config.toml`.

After: one `roko init` command. In a TTY: interactive wizard (project + global).
Non-TTY: writes sane defaults and prints what to configure next.

---

## Part 13: Migration Path

### Phase 0: Foundation (1 week)

Minimum viable slice that immediately improves daily experience:

1. **`roko next`** — inspect workspace state, output prioritized suggested actions.
   File: new `commands/next.rs`. ~200 LOC.

2. **`roko do`** — alias for `roko run` with auto-select workflow config. No new
   behavior yet, just the name. File: add `Do` variant to `Command` enum.

3. **End-of-run summary** — after `plan run` completes, print:
   ```
   Run complete: sprint-42
     Passed: 8/10 tasks
     Failed: T6 (gate: clippy), T9 (gate: test)
     Cost: $8.47 | Duration: 34min
     Resume: roko do --continue sprint-42
   ```
   File: `orchestrate.rs`. ~300 LOC.

4. **Welcome banner** — version, model, auth, workspace stats on startup.
   File: `run_unified_inline` entry. ~30 LOC.

5. **Error recovery suggestions** — pattern-match common errors to actionable
   commands. File: `main.rs` top-level error handler. ~30 LOC.

6. **Fix confirmation theater** — wire `/system`, `/effort`, `/gate`, `/config set`
   so they actually do what they say. File: `chat_inline.rs`.

Validates: user can discover what to do, see clear run results, configure without lying.

### Phase 1: Output (1 week)

1. **Clack-style formatter** — replace raw `println!`/`eprintln!` in `roko run`,
   `roko plan run`, `roko status` with structured `RunBlock`/`StreamingBlock` output.
   File: new `render/inline.rs`, wire into `orchestrate.rs` and `run.rs`.

2. **Wire built-but-unused primitives** — `ToolCallBlock`, `CostWaterfall`, `DiffBlock`,
   `ReplanBlock` are all built but only used in `bench_demo.rs`. Wire to live paths.
   File: `inline/primitives/` → `render/inline.rs`.

3. **Surface cost prediction** — show estimated cost + model route before execution.
   File: read from CascadeRouter data, format in `PredictionBlock`.

4. **Surface knowledge loading** — show knowledge loaded from neuro store at dispatch time.
   File: `orchestrate.rs` dispatch path.

Validates: raw debug output is gone from demo-critical paths.

### Phase 2: WorkflowEngine convergence (2 weeks)

Per implementation plan 11:

1. **Migrate `roko plan run`** to `WorkflowEngine` with `WorkflowTemplate::PlanExecution`.
   Keep `--use-event-loop` flag for one release transition.
   File: `commands/plan.rs`.

2. **Migrate `agent_exec.rs`** callers to `WorkflowEngine::run` with `WorkflowConfig::express()`.
   Delete `spawn_agent_scoped`. File: `agent_exec.rs`.

3. **Migrate ACP default mode** to `WorkflowEngine` with `AcpEventBridge` consumer.
   File: `roko-acp/src/bridge_events.rs` → extract `AcpEventBridge`.

4. **Delete `roko chat`** or reduce to 30-LOC wrapper calling `run_chat_loop` with
   `PlainRenderer`. File: `chat.rs`.

5. **`WorkflowConfig::auto_select`** moved from `roko-acp` to `roko-runtime` so
   CLI, HTTP, and ACP all use the same heuristic.

Validates: one engine, every entry point uses it, `runner/event_loop.rs` is gated
behind `--use-event-loop` only.

### Phase 3: Rendering convergence (1-2 weeks)

Per implementation plan 16:

1. **`ResponseRenderer` trait** with `InlineRenderer`, `PlainRenderer`, `TuiRenderer`.
2. **Single chat loop** in `chat_session_loop.rs` with `ChatBackend` trait.
   `chat_inline.rs` shrinks from 4,100 LOC to ~1,500 LOC.
3. **TUI on `RuntimeProjection`** — no more disk loading per render.
4. **TUI Agents tab** renders `ToolCallBlock` from shared primitives.

Validates: adding a feature to rendering touches one place, not five.

### Phase 4: 5-verb surface (2 weeks)

1. **`roko show`** — unified read-only state. Replaces `status`, `dashboard` (in text mode),
   `learn all`, `plan list`, `agent list`.

2. **`roko think`** — wraps `research topic`, `research search`, `knowledge query`, `explain`.
   Adds context-carry so output feeds into subsequent `do`.

3. **`roko tune`** — replaces `learn tune *`, `config set`, `config experiments`.
   All changes actually write (no more confirmation theater).

4. **`roko undo`** — cancel running work item, roll back file changes via git,
   list reversible actions.

5. **Work items** — create named, queryable, auto-managed work items. Replace
   `executor.json` as user-facing state artifact.

6. **Session auto-save** — persist context across invocations to `.roko/sessions/current.json`.

Validates: users can operate roko with 5 commands instead of 35.

### Phase 5: Polish (1 week)

1. **TUI reorganization** — 4-tab mission control layout (Home, Work, Observe, Configure).

2. **`roko do --ghost`** — full dry-run with plan, agents, models, gates, cost estimate.

3. **`roko do --batch`** — process multiple prompts from stdin or file.

4. **Ambient learning** — show routing confidence, model performance, cost estimates
   from episode data at pre-execution time.

5. **Command palette** — Ctrl+K universal entry from CLI, TUI, editor. Fuzzy matching
   on work items + verb completions + recent history.

6. **7-route HTTP API** — stable verb-aligned API in front of 85 internal routes.

---

## Part 14: What This Does NOT Change

- The underlying engine (agents, gates, learning, routing) stays the same.
- PRDs, plans, tasks still exist internally — they become implementation details,
  not user-facing commands.
- The 18-crate architecture does not change.
- The 85 serve routes do not change (they become the implementation behind 7 public routes).
- Nothing in the gate pipeline changes.
- Learning, episodes, cascade router — all unchanged, just surfaced better.

This is a UX layer on top of the existing engine, not a rewrite. The engine is correct.
The surface is wrong.

---

## Part 15: Open Questions

1. **How smart does the intent classifier need to be?** Simple heuristic (word count,
   keyword matching) vs an LLM call. The LLM call adds ~1-2s latency but is more
   accurate for ambiguous prompts. Proposal: heuristic-first, LLM-if-ambiguous, with
   the ambiguous threshold configurable.

2. **Named work items for trivial tasks?** Should `roko do "fix typo"` create a named
   work item or be ephemeral? Probably ephemeral for Trivial, named for Small and up.
   Threshold is configurable.

3. **Session scope?** Per-terminal? Per-day? Until explicit `--new`? Auto-expire after
   idle? Proposal: auto-expire after 1 hour idle, explicit `--new` to force fresh,
   `--resume` to list and pick a specific session.

4. **Backward compatibility?** Old subcommands as deprecated aliases for one release.
   The user base for these is mostly the self-hosted system itself — roko's own plan
   runner and agents. Update those callers, then remove the aliases.

5. **Where does `serve` fit?** Infrastructure, not a verb. Stays as-is. The API routes
   behind `serve` mirror the 5 verbs.

6. **Self-hosting model.** In the new model, roko developing itself calls `roko do
   "implement feature X"` where the prompt comes from its own PRD/plan system
   internally. The 5-verb interface becomes both the human-facing and machine-facing API.
   The PRD/plan machinery stays but is invoked by `WorkflowEngine`, not by the user
   directly.

7. **Minimum viable slice.** The weekend version: rename `roko run` → `roko do`,
   add `roko show` as alias for `roko status`, add `roko think` as alias for
   `roko research topic`, wrap executor state in a work item object, add the
   end-of-run summary. That's four changes with immediate daily-use impact.

---

## Sources

| Document | Key contribution |
|---|---|
| `tmp/subsystem-audits/05-01/42-workflow-redesign-suggestion.md` | 5-verb model, progressive formality, evidence from actual usage, ideas A–K |
| `tmp/mori-diffs/36-WORKFLOW-ENTRYPOINT-ORCHESTRATION-AUDIT.md` | WorkflowRequest→WorkflowResolver→WorkflowPlan pattern, entry point coverage matrix |
| `tmp/workflow/implementation-plans/11-entry-point-convergence.md` | WorkflowEngine migration plan, delete roko chat, agent_exec collapse |
| `tmp/workflow/implementation-plans/16-cli-tui-rendering-convergence.md` | ResponseRenderer trait, single chat loop, TUI on RuntimeProjection |
| `tmp/solutions/roko/09-UX-WORKFLOW-VISION.md` | Aggregate→funnel→execute, 5-pass funnel architecture, corpus management |
| `tmp/solutions/roko/15-UX-PLAN.md` | 6-phase plan, TaskSpec/TaskAgentInput types, ContextBudget, full effort estimates |
| `tmp/solutions/roko/15-UX-ISSUES.md` | Issues I-UX01 through I-UX18, concrete root causes and file references |
| `tmp/binary-issues/MASTER-INDEX.md` | S3 confirmation theater, S10 dual engines, 90+ issues with file:line references |
| `tmp/binary-issues/18-CONFIG-AND-ERRORS.md` | Default agent is `cat`, two init paths, error message quality, silent failures |
| `tmp/binary-issues/10-QUALITY-OF-LIFE.md` | Welcome banner, error recovery suggestions, session auto-save, smart prompts |
| `tmp/demo-req/IMPLEMENTATION-PLAN.md` | Clack-style output, 18 inline primitives, ratatui Viewport::Inline architecture |
| `tmp/dogfood/09-MAY6-DEMO-BUILD.md` | Clack-style demo output format, P0 demo-critical checklist |
| `tmp/subsystem-audits/ux/PLAN.md` | ingest/funnel/next commands, phase dependency graph, effort estimates |
| `tmp/learnings3/08-BUILD-PLAN.md` | Tier 0: formatted CLI output as prerequisite for any demo |
