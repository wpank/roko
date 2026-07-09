# Vision: Dynamic, Composable Agent Workflows

This document captures the desired end state for how agent roles, workflows, and pipelines should work. It applies across multiple subsystems and drives the goals in each folder.

## Core Principles

1. **Everything is configurable, nothing is hardcoded** — Roles, prompts, workflows, gates, recovery logic are all data-driven (TOML/YAML config files), not Rust enums and match statements.

2. **Composable building blocks** — Roles, gates, and pipeline steps are Lego pieces that snap together. You build a workflow by chaining them, not by modifying a state machine.

3. **Dynamic prompt assembly with cybernetic injection** — Prompts aren't static strings. They're assembled from templates + context injected from episodes, insights, knowledge learned from other agents, playbook successes, and anti-patterns discovered at runtime.

4. **Self-learning loops everywhere** — Every component has the possibility to self-adjust based on evals, benchmarks, gate results, and experience. Agents can modify their own behavior based on what the numbers say works.

5. **Visual, intuitive authoring** — Workflows are built visually in a UI that feels like a video game or Lego set — drag nodes, connect cables, see live previews. Three views: Recipe (linear), Graph (DAG), Timeline (Gantt).

6. **Community ecosystem** — People create and share workflows, role definitions, gate configurations, and prompt templates. Network effects: battle-tested configurations rise to the top based on real eval data.

7. **Adapter-first extensibility** — Every subsystem boundary is an adapter surface. Where roko connects to anything external (LLM providers, project trackers, CI pipelines, storage backends, observability, payments, communication), that connection is defined by a trait — not hardcoded to a single implementation. Each adapter trait has ≤5 required methods, is implementable in-process (Rust) or out-of-process (stdio/gRPC/WASM), and is configurable via TOML without code changes. Adapters should also be dual-exposed as MCP servers (so external agents can consume them) and implementable via MCP (so external MCP servers can serve as adapter implementations). Declarative `connector.toml` manifests handle the 80% case (REST API wrappers) without writing Rust. See [gtm/ADAPTER-PHILOSOPHY.md](gtm/ADAPTER-PHILOSOPHY.md) for the full design principle, [gtm/ADAPTER-MAP.md](gtm/ADAPTER-MAP.md) for per-subsystem adapter opportunities, and [gtm/ECOSYSTEM-PATTERNS.md](gtm/ECOSYSTEM-PATTERNS.md) for the research behind these patterns.

---

## Current State: What's Hardcoded Today

### 28 Roles = Rust Enum Variants (Not Configurable)

```rust
pub enum AgentRole {
    Conductor, Strategist, Implementer, AutoFixer, Architect, Auditor,
    Scribe, Critic, QuickReviewer, Refactorer, Researcher, PrePlanner,
    // + 16 specialist validation roles
}
```

Adding a new role requires: adding enum variant, backend assignment, label/short methods, prompt template struct, budget allocation, tool permission profile. **5+ code changes across 3+ files.**

### Role Prompts = Static `&'static str` Constants

Each role has a hardcoded identity string in `roko-compose/src/templates/`:
- Strategist: "You are the Strategist. Your job is to analyze the plan..."
- Implementer: "You are the Implementer. Your job is to write production-quality code..."
- Reviewer (3 variants): Architect (code quality), Auditor (spec compliance), Combined

Behavior rules embedded in the identity string itself ("Do not ask questions", "No unwrap()"). **Not customizable without code changes.**

### Workflow = 3 Hardcoded Templates

| Template | Phases | Selection |
|---|---|---|
| Express | Implement → Gate → Commit | <15 words, simple keywords |
| Standard | + Review | Default |
| Full | + Strategist | >50 words, complex keywords |

Phases are hardcoded in `pipeline.rs`. **Cannot reorder, add, skip, or branch.** Cannot insert a custom reviewer step between gate and commit.

### Budgets = `match role {` Function

Per-role character budgets for each prompt section:
```rust
fn budget_for(role: AgentRole) -> PromptBudget {
    match role {
        AgentRole::Implementer => PromptBudget { plan: 50_000, workspace: 20_000, ... },
        AgentRole::Strategist => PromptBudget { plan: 50_000, workspace: 20_000, ... },
        // ... 28 match arms
    }
}
```

### Failure Recovery = Hardcoded Match Arms

- Compile fail (simple) → AutoFix → re-gate
- Compile fail (complex) → back to Implementer
- Review: REVISE → back to Implementer
- Max 3 gate failures → halt

**Cannot customize recovery logic** (e.g., "on security scan fail, escalate to security reviewer").

### Mori Was the Same

Mori had the same 28-role enum, fixed pipeline, hardcoded prompts, and match-based budgets. Roko inherited this architecture.

---

## Implementation Progress (2026-04-28)

The architecture runner completed Phases 0-3 of the [MASTER-IMPLEMENTATION-PLAN.md](MASTER-IMPLEMENTATION-PLAN.md), creating the trait-based service layer that makes this vision achievable:

| Vision Goal | Foundation Delivered | Remaining |
|---|---|---|
| **Everything configurable** | Foundation traits define extension points (6 traits in `roko-core/src/foundation.rs`) | Phase 4: role/workflow/gate TOML config system |
| **Composable building blocks** | `WorkflowEngine` composes `PipelineStateV2` + `EffectDriver` + `TaskScheduler` + foundation services | Phase 4: workflow TOML composition |
| **Cybernetic prompt assembly** | `PromptAssemblyService` (P1B) wraps `SystemPromptBuilder` with pluggable `ContextSource`s | Wire neuro/episode/playbook as live `ContextSource` impls |
| **Self-learning loops** | `FeedbackService` (P1C) fans out to all sinks; `RuntimeEvent` bus captures all outcomes | Wire CascadeRouter learning, section effectiveness tuning |
| **Visual authoring** | `RuntimeProjection` (P3C) + `SseAdapter` (P3B) provide real-time state for any UI | Phase 5: Generative Canvas |
| **Community ecosystem** | `EventConsumer` trait enables arbitrary adapters | Phase 5: marketplace |
| **Adapter-first extensibility** | All 6 traits (`ModelCaller`, `PromptAssembler`, `FeedbackSink`, `GateRunner`, `EventConsumer`, `EffectExecutor`) are the adapter surfaces | Phase 4: TOML connector manifests |

Branch: `codex/arch-run-20260428-012508` (16 batches, all cargo-check verified).

---

## Desired End State

### 1. Role Definitions as Config Files

```toml
# roles/implementer.toml
[role]
name = "implementer"
label = "impl"
backend = "claude"
model_hint = "claude-sonnet-4-6"

[identity]
description = "Production code writer. Follows plan spec exactly."
rules = [
    "Operate autonomously. Do not ask questions.",
    "No unwrap() in libraries.",
    "Doc comments on all pub items.",
]

[tools]
allowed = ["read_file", "write_file", "edit_file", "bash", "grep", "glob"]
denied = ["web_search"]

[budget]
plan = 50_000
workspace = 20_000
prd = 12_000
file_context = 8_000

[prompt.sections]
# Which prompt layers this role uses and their priority
plan_spec = { priority = "critical", max_tokens = 50_000 }
workspace_map = { priority = "high", max_tokens = 20_000 }
file_context = { priority = "high", max_tokens = 8_000 }
skills = { priority = "normal", max_tokens = 8_000 }
```

**Anyone can create a new role** by dropping a `.toml` file in a roles directory. No code changes needed. Share roles via marketplace.

### 2. Workflows as Composable Pipelines

```toml
# workflows/security-review.toml
[workflow]
name = "security-review"
description = "Implementation with security-focused review"

[[steps]]
name = "implement"
role = "implementer"
on_success = "gate"
on_failure = "auto-fix"

[[steps]]
name = "gate"
gates = ["compile", "test", "clippy"]
on_success = "security-review"
on_failure = "auto-fix"

[[steps]]
name = "security-review"
role = "security-auditor"           # Custom role
gates = ["security-scan"]           # Custom gate
on_success = "code-review"
on_failure = "implement"            # Loop back with findings

[[steps]]
name = "code-review"
role = "architect"
on_success = "commit"
on_failure = "implement"

[[steps]]
name = "auto-fix"
role = "auto-fixer"
max_retries = 3
on_success = "gate"
on_failure = "halt"

[[steps]]
name = "commit"
action = "git-commit"
```

**Insert any step anywhere.** Chain roles + gates in any order. Define custom failure recovery per step. Share workflows via marketplace.

### 3. Dynamic Prompt Assembly with Cybernetic Context

The 9-layer prompt builder becomes data-driven. Context is injected from:

- **Episodes** — What worked/failed in past runs (from this agent and others)
- **Knowledge store** — Durable insights extracted from dream consolidation
- **Playbooks** — Proven action sequences with success scores
- **Anti-patterns** — Failure patterns to avoid (learned from gate failures)
- **Cross-agent insights** — Knowledge from other agents' experiences
- **Gate feedback** — Structured errors from prior verification attempts

Each section's budget and priority adapts based on **section effectiveness scoring** — sections that historically correlate with success get more tokens, sections that don't help get trimmed.

### 4. Self-Adjusting Agents

Agents observe their own performance and adjust:

- **Model routing** — CascadeRouter learns which model works best for each task type
- **Gate thresholds** — Adaptive EMA: gates that always pass get skipped
- **Prompt sections** — Section effectiveness: remove sections that don't help
- **Tool selection** — Track which tools correlate with success per role
- **Recovery strategy** — Conductor bandit learns: retry vs escalate vs decompose
- **Workflow selection** — Learn which workflow template works best for which prompt type

Agents can also **propose structural changes** (L4 loop): "I've noticed security scans fail 40% of the time on chain code — suggest adding a security-auditor step to chain workflows."

### 5. Visual Workflow Builder (Generative Canvas)

Three views of the same workflow:
- **Recipe view** — Linear, Apple Shortcuts-style card list
- **Graph view** — DAG with nodes and typed cables
- **Timeline view** — Gantt-style waterfall showing parallel execution

Interactions:
- Drag role nodes from a palette
- Connect with typed cables (output of one → input of next)
- Configure each node (role, gates, tools, budget)
- Right-click → "Test in isolation" runs just that node
- Live preview: hover shows last output
- Red cables = type mismatch
- Promote parameter to "macro knob" for easy tuning

### 6. Community Marketplace

5-tier package system:
1. **Prompts** — Markdown with TOML front-matter (no execution)
2. **Config profiles** — TOML bundles layering onto roko.toml
3. **Declarative tools** — TOML manifests for subprocess/HTTP/MCP
4. **WASM** — Sandboxed, fuel-metered computation
5. **Native Rust** — Full performance (compiled in-tree only)

Trust by evidence: Verified Run badges, gate pass rates, community validation. Fork as fundamental mechanism. Transparent take-rates.

---

## Gap: Current → Desired

| Aspect | Current | Desired | Gap |
|---|---|---|---|
| **Role definition** | 28-variant Rust enum | TOML config files, drop-in | Large — need role loader, schema, registry |
| **Role prompts** | Static `&'static str` | Template + dynamic context injection | Medium — builder exists, needs data-driven identity |
| **Workflow composition** | 3 hardcoded templates | TOML pipeline definitions, any topology | Large — need workflow DSL, step executor |
| **Failure recovery** | Hardcoded match arms | Per-step configurable on_success/on_failure | Medium — state machine exists, needs config |
| **Prompt context injection** | Built but only from dead code | Live injection from episodes/knowledge/playbooks | Medium — all components exist, need wiring |
| **Section effectiveness** | Built, dead code only | Live learning, budget adjustment | Small — wire FeedbackService |
| **Model routing learning** | Built, dead code only | Live CascadeRouter updates | Small — wire from live paths |
| **Visual UI** | No visual workflow builder | Generative Canvas with 3 views | Very large — new frontend system |
| **Community sharing** | Nothing | 5-tier marketplace with trust scoring | Very large — new platform |
| **Self-adjusting agents** | Conductor bandit built, dead | Agents observe + propose changes | Medium — wire existing, add L4 proposals |

---

## Subsystems Affected

This vision touches nearly every subsystem:

| Subsystem | What Changes |
|---|---|
| **orchestration/** | WorkflowEngine reads workflow TOML, executes configurable pipelines |
| **acp-protocol/** | ACP pipeline uses same configurable workflows |
| **prompt-assembly/** | PromptAssemblyService becomes fully data-driven, dynamic context injection |
| **learning-feedback/** | FeedbackService wired to live paths, section effectiveness, routing |
| **cognitive-layer/** | Knowledge/episodes/playbooks actively injected into prompts |
| **gate-pipeline/** | Gates configurable per-step, adaptive thresholds per workflow |
| **safety-agent/** | Contracts defined per-role config, tool permissions from role TOML |
| **config-tools-events/** | Role/workflow/gate config loading, tool registry from config |
| **inference-dispatch/** | ModelCallService routes based on role config model_hint |
| **cli-chat-tui/** | Visual workflow builder (Generative Canvas) |
| **http-persistence/** | Workflow/role configs served via API, marketplace routes |
| **code-intelligence/** | Context injection into prompts from code index |
