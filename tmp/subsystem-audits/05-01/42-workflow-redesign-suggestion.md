# 42 — Workflow Redesign Suggestion

Not an implementation plan. Thinking about what's wrong with the current
workflow, what a better one looks like, and what ideas emerge from studying
how the system is actually used.

---

## Part 1: What's Wrong Now

The current workflow is a fixed pipeline:

```
roko prd idea → prd draft new → research enhance-prd → prd plan → plan run → dashboard
```

Problems:

1. **Forced formality.** A typo fix and a major rewrite go through the same
   6-step pipeline. There's no lightweight path. Most things don't need a PRD.

2. **Too many subcommands.** 35+ subcommands, 3-4 levels deep (`roko knowledge
   dream journal`, `roko config providers health`). Users have to know the
   taxonomy before they can do anything.

3. **Pipeline is sequential and rigid.** You can't skip steps. You can't start
   from the middle. You can't go back. You can't do two things at once in
   different stages.

4. **No intent detection.** The system doesn't adapt. You choose the formality
   level, not the system. "Fix the login bug" should be quick. "Redesign the
   auth system" should be careful. The user shouldn't have to make that call
   by picking different commands.

5. **Disconnected surfaces.** CLI, TUI, chat, serve, ACP are separate entry
   points with different capabilities. Work started in one can't continue in
   another.

6. **State is manual.** `--resume .roko/state/executor.json` shouldn't be a
   flag. The system should know what's in progress.

7. **Learning is invisible.** The cascade router, episodes, efficiency logs,
   adaptive thresholds — all running but invisible to the user. No way to see
   "roko is getting better at X" or "last time you did this, it took Y."

---

## Part 2: Evidence From Actual Usage

### What the user actually does vs what the system offers

From examining `.roko/`, `tmp/`, git history, and artifact state:

**PRD pipeline**: 4 ideas captured (all duplicates). 1 skeleton draft. 0 published.
0 PRD-driven plans completed. The full pipeline has never been run end-to-end
by a human. Instead, the user reaches for multi-batch agent runs (Claude/Codex)
directly, bypassing the entire PRD/plan interface.

**Plans**: 6 plan directories, 2 completed (15/44 tasks done = 34%). Plans
that worked were created directly, not from PRDs.

**Episodes**: 22 total, mostly failed dispatch initialization. Zero successful
task completions recorded in the learning system.

**Learning**: CascadeRouter has learned 37 model slugs and 28 role→model
mappings, but from partial/failed runs. The routing data has never been
validated against successful outcomes.

**What works**: Direct `roko run "prompt"`, the unified chat REPL (`roko` with
no args), and agent-driven batch refactoring. Everything else is ceremony.

**The real workflow**: The user opens Claude Code, says what they want in natural
language, and the agent does it. Roko's 35-subcommand taxonomy exists alongside
this but isn't the primary interface. The user's actual loop is:

```
think about what to do → tell Claude → review output → iterate
```

Not:

```
prd idea → prd draft → research enhance → prd plan → plan run → dashboard
```

### What surfaces get used

| Surface | Used? | How |
|---------|-------|-----|
| CLI (`roko` no args) | Yes | Interactive chat, primary entry |
| CLI (`roko run`) | Sometimes | Quick single prompts |
| TUI (`roko dashboard`) | Rarely | Checked occasionally, not primary |
| Serve (`roko serve`) | For demo | Backend for demo-app, not daily |
| ACP | Indirectly | Editor integration path |
| 35+ subcommands | Almost never | Too much taxonomy to remember |

### What Claude Code gets right that roko doesn't

The user configures Claude Code with:
- `CLAUDE_CODE_DISABLE_ADAPTIVE_THINKING=1` — predictable behavior
- `CLAUDE_CODE_DISABLE_BACKGROUND_TASKS=1` — nothing invisible
- `CLAUDE_CODE_EFFORT_LEVEL=max` — full capability always
- Permissive bash execution, but asks before git operations

The pattern: **maximum capability, maximum transparency, minimum ceremony.**
Claude Code is effective because you say what you want and it does it. There's
no `claude prd idea "fix the bug"`. There's no `claude plan generate`. You
just say "fix the bug" and it fixes the bug.

---

## Part 3: Core Principle — Progressive Formality

The workflow should be a spectrum, not a pipeline. The system escalates
formality automatically based on complexity, risk, and user preference:

```
Instant          Light           Planned          Managed
────────────────────────────────────────────────────────────
"fix typo"    "refactor auth"  "rewrite frontend"  "migrate to v2"
│                │                │                   │
│  direct exec   │  auto-plan     │  plan review      │  multi-agent
│  no approval   │  auto-execute  │  approval gates   │  staged rollout
│  1 agent       │  1 agent       │  1-3 agents       │  N agents
│  no PRD        │  no PRD        │  optional PRD     │  PRD required
│  seconds       │  minutes       │  hours            │  days
```

The user doesn't pick the level. They say what they want. The system picks
the level and can be overridden.

---

## Part 4: Design — 5 Verbs + Implicit State

### The Verbs

Everything roko does maps to 5 verbs. Every surface (CLI, chat, TUI, API,
ACP) exposes the same verbs:

| Verb | What it does | Formality |
|------|-------------|-----------|
| **do** | Execute something | Adapts: instant → managed |
| **think** | Research, analyze, explain | Always lightweight |
| **show** | Display state, progress, history | Read-only |
| **tune** | Adjust preferences, models, thresholds | Configuration |
| **undo** | Revert, cancel, roll back | Corrective |

That's it. Everything else (`prd`, `plan`, `research`, `knowledge`, `learn`,
`dashboard`, `status`, `deploy`, `daemon`, `agent`, `config`, `index`,
`explain`, `inject`, `replay`, `new`, `completions`) is either a sub-operation
of these 5 or an implementation detail that users shouldn't think about.

### CLI Mapping

```bash
# Today (rigid, taxonomic)
roko prd idea "wire auth"
roko prd draft new "auth-wiring"
roko research enhance-prd auth-wiring
roko prd plan auth-wiring
roko plan run plans/
roko dashboard

# Proposed (intent-driven)
roko do "wire auth into the login flow"
# → system detects: medium complexity, existing code, 1 agent sufficient
# → auto-plans, shows plan, auto-executes unless --review flag
# → streams progress inline
# → records episode, updates router, done

roko think "how should we handle auth tokens?"
# → researches codebase + web, returns analysis
# → no execution, no side effects

roko show
# → what's running, what's pending, recent history, learning state

roko show auth-wiring
# → details of a specific piece of work

roko tune model sonnet
# → changes default model

roko undo
# → reverts last change or cancels running work
```

### Progressive Escalation Inside `do`

When you say `roko do "..."`, the system classifies the intent and picks a
strategy:

```
Intent Classification
─────────────────────
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
   → "This is a big change. I've created a plan: 'auth-redesign'. Review it?"

5. Ambiguous
   → Asks one clarifying question, then classifies
   → "Do you want me to just fix the null check, or redesign the error handling?"
```

The user can always override:
- `roko do --plan "fix typo"` → forces a plan even for trivial work
- `roko do --just-do-it "redesign auth"` → skips approval, YOLO
- `roko do --review "refactor"` → forces review for each step

### Implicit State

The system tracks state without the user managing files:

```bash
# Today
roko plan run plans/ --resume .roko/state/executor.json

# Proposed
roko do  # with no args: resumes whatever was in progress
# or
roko show  # shows what's pending/running
roko do --continue auth-redesign  # explicit resume by name
```

State is automatic:
- Starting work creates a work item with a name (auto-generated or user-provided)
- Interrupting (Ctrl-C) saves state
- Next `roko do` offers to resume
- `roko show` lists everything: running, paused, completed, failed

### Unified Across Surfaces

The same 5 verbs work everywhere:

```
CLI:     roko do "fix auth"
Chat:    /do fix auth
TUI:     F2 → type prompt → enter
API:     POST /api/do { "prompt": "fix auth" }
ACP:     Same protocol, same verbs
```

Work started on one surface is visible and continuable on all others.
Start `roko do "refactor"` in CLI, watch progress in TUI, adjust in chat.

---

## Part 5: Additional Ideas Beyond the 5 Verbs

### Idea A: Conversation-First, Not Command-First

The current system is command-first: you invoke a subcommand, it does a thing.
But the actual usage pattern is conversation-first: you open the chat, describe
what you want, iterate.

**What if there were no subcommands at all?**

```bash
roko
# Opens interactive session. You just talk.

you> wire auth into the login flow
# roko classifies, plans, executes (progressive formality)

you> actually wait, how does the current auth work?
# roko switches from "do" to "think" mode seamlessly, mid-conversation

you> ok do it, but use JWT not sessions
# roko updates the plan, continues execution with new constraint

you> show me what changed
# roko shows the diff, inline

you> undo the middleware change, keep the rest
# roko reverts selectively
```

No commands. No `/do`. No `roko think`. Just natural language with implicit
verb detection. The verbs exist as the internal classification, not as user-facing
syntax. You only need explicit verbs (`roko do "..."`) for non-interactive
one-shot usage.

The chat slash commands (59 of them today: `/help`, `/model`, `/run`, `/plan list`...)
collapse too. Instead of `/model sonnet`, you say "use sonnet." Instead of
`/plan list`, you say "what plans exist?" The slash commands become accelerators
for power users, not the primary interface.

**Why this matters**: The system already has `chat_inline.rs` with a 5-phase
state machine (Input → Thinking → Streaming → Error → Done). The state machine
just needs richer intent detection in the Input→Thinking transition instead
of always dispatching to a single agent turn.

### Idea B: Work Items as First-Class Objects

Today, "work" is scattered across PRDs, plans, tasks, episodes, executor
snapshots, and signals — each with its own format, storage, and lifecycle.
The user sees none of this coherently.

A **work item** is a single concept that wraps everything:

```
WorkItem {
  id: "auth-redesign"
  status: Running | Paused | Done | Failed
  created: 2026-05-01T10:30:00Z
  prompt: "redesign the auth system to use JWT"
  formality: Large  // auto-classified

  // Internal, not user-facing
  prd: Option<PrdRef>
  plan: Option<PlanRef>
  tasks: Vec<TaskRef>
  episodes: Vec<EpisodeRef>
  git_branch: Option<String>
  cost: CostSummary
}
```

Users interact with work items, not with the underlying machinery:

```bash
roko show
# ┌─────────────────────────────────────────────────┐
# │ auth-redesign     Running   3/7 tasks   $0.42   │
# │ fix-login-bug     Done      1/1 tasks   $0.03   │
# │ readme-typo       Done      —           $0.01   │
# └─────────────────────────────────────────────────┘

roko show auth-redesign
# Shows: prompt, plan, current task, recent output, cost, branch

roko do --continue auth-redesign
# Resumes from where it left off
```

**Why this matters**: Today, `.roko/state/executor.json` is 2.1 MB of nested
plan state. The user has to know the file path and pass `--resume` with it.
Work items replace this with a named, queryable, auto-managed entity.

### Idea C: Ambient Learning Surfaced as Context

The learning subsystem records everything but surfaces nothing. The cascade
router has 37 models and 28 role mappings. Gate thresholds are EMA-adjusted.
Efficiency data is logged per-turn. None of this is visible.

**Surface learning as conversational context:**

```
you> refactor the auth module

roko> I'll use opus for this — it's handled 4 similar refactors with 100% gate
      pass rate. Sonnet was 2/3 on auth-related work. Estimated cost: $1.20
      based on similar episodes. Last auth refactor took 12 minutes.

      Plan (3 tasks): [...]

      Proceed?
```

The system doesn't just execute — it explains *why* it's making the choices
it's making, using its own learning data. This is the "roko is getting better"
signal that's currently invisible.

```bash
roko show learning
# ┌─────────────────────────────────────────────────────────┐
# │ Model Performance (last 30 days)                        │
# │                                                         │
# │ opus     12 tasks   92% pass   avg $1.40   avg 8min     │
# │ sonnet   31 tasks   87% pass   avg $0.35   avg 3min     │
# │ haiku     8 tasks   75% pass   avg $0.08   avg 1min     │
# │                                                         │
# │ Routing confidence: 0.78 (improving — was 0.62 a week ago)│
# │ Gate threshold drift: clippy ↓2%, test ↑5%              │
# └─────────────────────────────────────────────────────────┘
```

**Why this matters**: The user has disabled adaptive thinking in Claude Code
because they want predictability. Surfacing learning data gives predictability
*through transparency* — the system adapts, but you can see exactly how and why.

### Idea D: The TUI as Mission Control, Not a Dashboard

The TUI has 10 tabs (F1-F10) modeled after a traditional dashboard. But
the actual need is mission control — a place to observe, intervene, and steer
ongoing work.

**Current TUI tabs**: Dashboard, Plans, Agents, Git, Logs, Config, Inspect,
Marketplace, Atelier, Learning.

**Proposed TUI layout** (fewer tabs, more purpose):

```
F1: Home
    Active work items with live progress
    Quick action bar: [d]o  [t]hink  [s]how  [u]ndo
    Recent cost summary
    Learning summary (one line: "routing confidence: 0.78, 31 episodes today")

F2: Work (replaces Plans + Agents + Atelier)
    Left: work item list (filterable)
    Right: selected item detail — plan, current task, agent output, diff
    Bottom: streaming agent output for active work
    Actions: pause, resume, cancel, approve, intervene

F3: Observe (replaces Logs + Inspect + Learning)
    Live event stream (structured DashboardEvents, not raw logs)
    Filter by: work item, agent, gate, model, severity
    Cost graph (burn rate over time)
    Model routing decisions (which model, why, outcome)

F4: Configure (replaces Config)
    Live config editor with validation
    Provider health at a glance
    Model routing overrides
    Gate threshold adjustments
```

**Why this matters**: The current 10-tab layout is feature-driven (one tab per
subsystem). The proposed 4-tab layout is workflow-driven (one tab per need:
act, watch, steer, configure). The TUI already has the state machine, modals,
and event infrastructure — it just needs reorganization.

### Idea E: Inline Mode in the Editor (ACP as Invisible Backbone)

Today ACP is a protocol that editors connect to. The session model
(`AcpSession`) manages conversation history, config state, approvals, and
workflow runs. But it's treated as a separate surface.

**What if ACP were the universal backend for all surfaces?**

```
CLI  ──→ AcpSession (local, stdio or socket)
TUI  ──→ AcpSession (embedded, in-process)
Chat ──→ AcpSession (embedded, in-process)
Serve ──→ AcpSession (per-connection, WebSocket)
Editor ──→ AcpSession (stdio, JSON-RPC — as today)
```

One session type. One state machine. One approval flow. One conversation
history format. Today there are three separate session implementations
(`ChatAgentSession`, `AcpSession`, TUI's `App` state) that each manage
their own history, model selection, and tool policy.

The ACP session already has the richest feature set:
- Conversation history with FIFO trimming (40 turns, 64K chars)
- Config state with per-session overrides
- Approval flow (request → grant/deny)
- Workflow integration (`active_run`, `shared_run`)
- Trust state (workspace-level permissions)
- Cancel token (Ctrl-C propagation)

Making ACP the universal session backend means:
- Work started in the editor continues in the CLI (same session)
- The TUI can attach to any session and show its state
- Serve exposes sessions as API resources
- All surfaces share the same approval, history, and state semantics

### Idea F: Batch Mode — The Missing Middle

The current system has two modes: single prompt (`roko run "..."`) and full
plan execution (`roko plan run`). There's nothing between them.

But the most productive workflow observed is multi-batch agent runs — giving
a list of related tasks to agents that run in parallel. This is what actually
ships code.

```bash
roko do --batch <<EOF
fix the null check in auth.rs
add error handling to the token refresh
update the tests for the new auth flow
EOF

# → 3 work items created, classified independently
# → trivial ones execute immediately
# → complex ones plan and await approval
# → all run in parallel where possible
# → unified progress view

# Or from a file:
roko do --batch tasks.txt

# Or pipe:
cat issues.txt | roko do --batch
```

**Why this matters**: The user's most productive mode (multi-batch agent runs)
has no CLI representation today. They use external tools (Claude Code, Codex)
to achieve what roko should do natively. Batch mode fills this gap.

### Idea G: Ghost Mode — Observe Without Executing

Sometimes you want to see what roko *would* do without doing it. A dry-run
that shows the full plan, agent selection, model routing, cost estimate, and
gate configuration — without executing.

```bash
roko do --ghost "redesign the auth system"
# ┌─────────────────────────────────────────────────────────┐
# │ Ghost Run: redesign-auth                                │
# │                                                         │
# │ Classified: Large (architectural)                       │
# │ Would create: named work item + 8-task plan             │
# │ Agents: 3 (implementer×2 + reviewer×1)                 │
# │ Models: opus (plan) → sonnet (implement) → opus (review)│
# │ Gates: compile → test → clippy → diff-review            │
# │ Estimated cost: $3.20 – $5.40                          │
# │ Estimated time: 15-25 minutes                          │
# │                                                         │
# │ Plan:                                                   │
# │  1. Extract auth types (sonnet, ~$0.30)                │
# │  2. Implement JWT middleware (sonnet, ~$0.60)           │
# │  3. Wire into request pipeline (sonnet, ~$0.45)        │
# │  ...                                                   │
# │                                                         │
# │ Run this? [y]es  [e]dit plan  [n]o                     │
# └─────────────────────────────────────────────────────────┘
```

**Why this matters**: The user disabled adaptive thinking in Claude Code
because they want to see what's happening. Ghost mode gives full transparency
into roko's decision-making before any work starts. It also serves as a
learning tool — "this is how roko thinks about your prompt."

### Idea H: Context Carry — Sessions Remember Everything

Today, each CLI invocation is stateless. `roko run "fix auth"` doesn't know
about the `roko think "how does auth work?"` you ran 5 minutes ago. The
chat REPL has conversation history within a session, but it's lost when
you exit.

**Context carry** means roko remembers across invocations:

```bash
roko think "how does the auth module work?"
# → analyzes, returns explanation, records context

roko do "refactor it to use JWT"
# → "it" resolves to the auth module from the previous invocation
# → the analysis from `think` is injected as context into the plan

roko show
# → shows both: the research and the refactor, linked
```

Implementation: A lightweight session that persists to
`.roko/sessions/current.json`. Each invocation appends. The `think` output
becomes context for the next `do`. Sessions auto-expire after configurable
idle time (default: 1 hour). `roko do --new` starts a fresh session.

**Why this matters**: The user's workflow is iterative — think, then do, then
adjust. Each step should inform the next without the user manually piping
context between commands.

### Idea I: Signals as Conversation — Push-Based Progress

The StateHub pattern (watch::Sender → broadcast DashboardEvent) already
exists. But it's only consumed by the TUI. What if every surface got
push-based progress?

```bash
roko do "refactor auth"
# ↓ streams inline, no polling
# ⟳ Planning... (opus, 2.1s)
# ✓ Plan: 4 tasks
#   1. Extract auth types
#   2. Implement JWT middleware  ← running (sonnet)
#   3. Wire into pipeline
#   4. Update tests
# ⟳ Task 2: editing auth/middleware.rs...
# ✓ Task 2: done (gate: compile ✓ test ✓ clippy ✓)
# ⟳ Task 3: editing auth/pipeline.rs...
```

Not a log dump. Structured events rendered inline. The same events that
the TUI renders in its agent output panel, but formatted for the terminal.

The serve endpoint becomes SSE:

```
GET /api/work/{id}/stream
→ event: task_started
→ event: agent_output (streaming tokens)
→ event: gate_result
→ event: task_completed
→ event: work_completed
```

**Why this matters**: Today, `roko plan run` runs in the foreground but
progress is opaque. The user has to open the TUI or poll `roko status`. With
push-based progress, the CLI becomes its own dashboard.

### Idea J: Composable Pipelines as Config, Not Code

The `roko.toml` already has 4 pipeline types (mechanical, focused, integrative,
architectural). But they're hardcoded in the config format and can't be
composed by the user.

```toml
# Today: fixed pipeline types
[pipeline.mechanical]
max_agents = 1
gates = ["compile", "clippy"]

# Proposed: user-defined pipelines
[[pipeline]]
name = "quick"
when = "formality < 2"
agents = 1
gates = ["compile"]
approval = false

[[pipeline]]
name = "careful"
when = "formality >= 3 or touches('auth', 'crypto', 'deploy')"
agents = { max = 4 }
gates = ["compile", "test", "clippy", "diff-review"]
approval = "per-phase"
branch = true
```

Pipelines become composable rules, not fixed slots. The `when` clause
is the intent classifier's output (formality level) plus domain heuristics
(file paths, module names). Users can create their own pipelines for their
specific workflows.

### Idea K: The Command Palette — TUI + Chat Unified

The inline chat already has a command palette (Ctrl+K, 59 commands, fuzzy
matching). What if this were the universal entry point for all interactions?

```
Ctrl+K opens the palette from anywhere:
  - CLI: opens in terminal
  - TUI: opens as overlay (already has modal system)
  - Editor: opens via ACP

Palette items:
  [recent] auth-redesign (3/7 tasks, running)
  [recent] fix-login-bug (done, 2 min ago)
  [do]     Type a prompt to start new work...
  [think]  Research a topic...
  [show]   View work items, learning, config...
  [tune]   Change model, thresholds, pipelines...
  [undo]   Revert last change, cancel work...
```

The palette replaces both the subcommand taxonomy AND the slash commands.
Fuzzy matching means you don't need to remember exact commands. Recent
work items appear at the top for quick resume.

---

## Part 6: What This Means For The Codebase

### Subcommands That Survive

| Current | Becomes | Why |
|---------|---------|-----|
| `roko` (no args) | `roko` (interactive session) | Primary entry point, conversation-first |
| `roko run "prompt"` | `roko do "prompt"` | Direct intent, one-shot |
| `roko chat` | `roko` (same as no args) | No distinction needed |
| `roko status` | `roko show` | Read-only state |
| `roko dashboard` | `roko show --live` | Same thing, different format |
| `roko serve` | `roko serve` | Infrastructure, stays |
| `roko init` | `roko init` | Setup, stays |

### Subcommands That Become Internal

| Current | Absorbed into | Why |
|---------|--------------|-----|
| `roko prd *` | `roko do` (large formality) | PRDs are an internal artifact, not a user concept |
| `roko plan *` | `roko do` (medium+ formality) | Plans are generated automatically |
| `roko research *` | `roko think` | Research is thinking |
| `roko agent *` | Internal to `do` | Agents are implementation |
| `roko knowledge *` | `roko show knowledge` | Read-only inspection |
| `roko learn *` | `roko show learning` | Read-only inspection |
| `roko config *` | `roko tune` | Configuration |
| `roko deploy *` | `roko do --deploy` | Deployment is an action |

### What's New (Priority Order)

1. **Conversation-first session** (Idea A): `roko` with no args becomes an
   intelligent REPL that detects verbs from natural language. This is the
   primary interface. Everything else is secondary.

2. **Work items** (Idea B): Replace PRD/plan/task hierarchy with a single
   concept. Named, queryable, auto-managed. This is the state model.

3. **Intent classifier**: Takes a prompt, returns a formality level + plan.
   This is the core new thing. It replaces the user choosing between 35
   subcommands.

4. **Context carry** (Idea H): Sessions persist across invocations. `think`
   informs `do`. Research context flows into execution.

5. **Push-based progress** (Idea I): Streaming structured events inline in
   CLI, SSE from serve, DashboardEvents in TUI. Already have StateHub — just
   need to surface it everywhere.

6. **Ambient learning** (Idea C): Surface routing confidence, model performance,
   cost estimates from actual episode data. Make the invisible visible.

7. **Ghost mode** (Idea G): Dry-run that shows full plan, agents, models,
   gates, cost estimate before executing. Maximum transparency.

8. **Batch mode** (Idea F): Process multiple prompts in parallel. Fill the
   gap between single-prompt and full plan execution.

9. **ACP as universal backend** (Idea E): One session type for all surfaces.
   Long-term architectural alignment.

10. **TUI as mission control** (Idea D): 4 tabs instead of 10. Workflow-driven
    instead of feature-driven.

11. **Command palette** (Idea K): Universal Ctrl+K entry point.

12. **Composable pipelines** (Idea J): User-defined pipeline rules.

---

## Part 7: HTTP API Mapping

The ~85 serve routes collapse to:

```
POST /api/do            → start work (with prompt, formality override, batch)
POST /api/think         → research/analyze (no side effects)
GET  /api/show          → list work items, sessions, learning state
GET  /api/show/:id      → detail of specific work item
POST /api/tune          → update config, thresholds, routing
POST /api/undo          → revert, cancel, pause
GET  /api/stream/:id    → SSE event stream for work item (replaces polling)
```

The existing 85 routes stay as implementation detail behind these 7.
`POST /api/do` internally calls plan creation, agent dispatch, etc.
`GET /api/show` internally reads episodes, signals, executor state.

---

## Part 8: What This Does NOT Change

- The underlying engine (agents, gates, learning, routing) stays the same.
- PRDs, plans, tasks still exist internally — they're just not user-facing
  commands.
- The 18-crate architecture doesn't change.
- The 85 serve routes don't change (they become the API behind the 5 verbs).
- Nothing in the audit backlog (doc 41) changes — those are engine fixes.

This is a UX layer on top of the existing engine, not a rewrite.

---

## Part 9: Open Questions

1. **How smart does the intent classifier need to be?** A simple heuristic
   (file count, keyword matching) vs an LLM call to classify. The LLM call
   adds latency but is more accurate. Could do heuristic-first, LLM-if-ambiguous.

2. **Named work items vs anonymous?** Should `roko do "fix typo"` create a
   named work item? Probably not for trivial work. Threshold?

3. **How much approval is too much?** The progressive model means medium
   tasks show a plan and wait. But "roko said it would do 4 things, do I
   care?" Users may want `--yes` by default and `--review` as the exception.

4. **Backward compatibility?** The old subcommands could stay as aliases
   that map to the new verbs. `roko prd idea "X"` → `roko do --prd "X"`.
   Or just remove them and break the workflow. The old workflow has few
   users (it's mostly self-hosted).

5. **Where does `serve` fit?** It's infrastructure, not a verb. Stays as-is.
   But the API routes should mirror the 5 verbs.

6. **Session scope?** How long does a session last? Per-terminal? Per-day?
   Until explicit `--new`? Auto-expire after idle? The answer affects context
   carry and work item lifecycle.

7. **How does self-hosting work?** Roko developing itself is the primary use
   case. In the new model, roko would `roko do "implement feature X"` where
   the prompt comes from its own PRD/plan system internally. The 5-verb
   interface becomes both the human-facing and machine-facing API.

8. **What's the minimum viable slice?** Probably: rename `roko run` to
   `roko do`, add `roko show` as alias for `roko status`, add `roko think`
   as alias for `roko research topic`, add work item wrapper around executor
   state. That's a weekend of work and immediately improves the experience.
