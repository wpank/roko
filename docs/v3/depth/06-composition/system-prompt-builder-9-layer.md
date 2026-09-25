# SystemPromptBuilder: 9-Layer Prompt Assembly

> **Depth file for [06-COMPOSITION.md](../../06-COMPOSITION.md)**
> Source: `crates/roko-compose/src/system_prompt_builder.rs`
> v1 source: `docs/v1/03-composition/02-system-prompt-builder-7-layer.md`

---

## Overview

The SystemPromptBuilder constructs agent system prompts through a 9-layer
architecture that separates stable identity (role, conventions) from volatile
context (task, affect). Each layer has a defined purpose, cache tier, and
injection point. The builder produces both a flat string (`build()`) and
structured sections (`build_sections()`) for use by the PromptComposer's
budget-fitting algorithm. Cache alignment markers between tiers enable the
inference gateway to place KV-cache breakpoints for maximum prefix reuse.

---

## 1. The 9 Layers

| Layer | Name | Cache Tier | Content Source | Purpose |
|-------|------|-----------|----------------|---------|
| 1 | Role Identity | System | `role_prompts.rs`, templates | Who the agent is, what it specializes in |
| 2 | Conventions | System | CLAUDE.md / project config | Project patterns, style rules, safety constraints |
| 3a | Domain Context | Session | PRD extracts, workspace map | Domain-specific knowledge for this project |
| 3b | Assembled Context | Session | Knowledge store, enrichment | Task-relevant retrieved context |
| 3c | Pheromone Signals | Session | Stigmergic signals | Active environmental signals guiding behavior |
| 4 | Task Context | Task | Task TOML, brief | What the agent should do right now |
| 4b | Gate Feedback | Dynamic | Prior verification failures | Structured retry digest from failed attempts |
| 5 | Tool Instructions | System | Tool definitions, MCP config | Available tools and how to use them |
| 6 | Relevant Techniques | Task | Playbook rules, learned skills, tool hints | Learned techniques to prefer for this task |
| 7 | Anti-Patterns | Task | Failure history, anti-knowledge | What mistakes to avoid |
| 8 | Affect Guidance | Dynamic | Daimon PAD state + temperament | Emotional/motivational modulation |

### Layer 1: Role Identity

The foundation layer. Defines the agent's role, expertise, and behavioral style.
Each of the 11 role templates provides a distinct identity:

```
Strategist:        "You are a technical strategist who decomposes complex tasks..."
Implementer:       "You are a senior software engineer implementing changes..."
Reviewer:          "You are a software architect reviewing implementation quality..."
Scribe:            "You are a technical writer documenting implementations..."
Conductor:         "You coordinate multi-agent plan execution..."
Researcher:        "You conduct deep research on technical topics..."
Refactorer:        "You restructure code without changing behavior..."
IntegrationTester: "You validate that changes work across system boundaries..."
QuickReviewer:     "You are a fast-turnaround code reviewer..."
TaskImplementer:   "You implement a specific, well-defined task..."
```

Role identity is placed in the System cache tier because it is identical across
all tasks for the same role. A 20-plan run with 40 Implementer spawns hits the
cache on 39 of them.

### Layer 2: Conventions

Project-level rules and constraints loaded from `CLAUDE.md`, `roko.toml`, and
project configuration:

- Coding style rules (naming conventions, error handling patterns)
- Safety constraints (never push to main, never delete without confirmation)
- Project-specific patterns (import organization, test structure)
- Architecture rules (crate dependencies, public API surface)

Conventions are System-tier because they do not change between tasks.

### Layer 3a: Domain Context

Project-specific knowledge that changes across sessions but not across tasks
within a session:

- PRD extracts relevant to the current plan
- Workspace map showing project structure
- Cross-plan context (what other plans have done, shared type registries)

### Layer 3b: Assembled Context

Task-relevant retrieved context from the knowledge store and enrichment
pipeline:

- Knowledge entries matching the task description (via HDC similarity or keyword)
- Episode summaries from similar past tasks
- Enrichment artifacts (research memos, dependency manifests)

Separate from 3a because its content is task-specific while 3a is session-level.

### Layer 3c: Pheromone Signals

Active environmental signals that guide agent behavior through stigmergy:

- Recent signals from the current plan indicating progress or blockers
- Inter-agent coordination signals (e.g., "crate X was just modified")
- Environment state indicators (build status, test results, resource usage)

Each pheromone `ContextChunk` is rendered with its metadata and formatted as a
section within the Session cache tier.

### Layer 4: Task Context

The specific task the agent should perform:

- Task TOML (description, files to modify, acceptance criteria)
- Task brief (What/Why/How summary from enrichment)
- Iteration memory (what was tried before and why it failed)

### Layer 4b: Gate Feedback

Structured retry feedback from prior gate failures. Rendered via
`GateFeedback::render_prompt_section()`, providing the agent with specific
information about what went wrong in previous attempts. Placed in the Dynamic
tier because it changes per retry attempt.

### Layer 5: Tool Instructions

Available tools and how to use them:

- Tool definitions (sorted alphabetically via `canonical_tool_order()` for
  cache stability)
- MCP server configuration
- Tool-specific instructions and restrictions

### Layer 6: Relevant Techniques

Learned skills, playbook sequences, and tool usage hints:

- Playbook rules matching the current task's file paths and crates
- Skill library entries relevant to the task type
- Tool usage hints from learned profiles (LEARN-12)

### Layer 7: Anti-Patterns

Known failure modes and explicit prohibitions:

- Common mistakes from episode history
- Anti-knowledge entries (things explicitly wrong or dangerous)
- Gate failure patterns from similar tasks

### Layer 8: Affect Guidance

Motivational modulation based on the Daimon's PAD
(Pleasure-Arousal-Dominance) state and optional `Temperament`:

```
High arousal (>= 0.35):
    "You are under time pressure. Focus on the most impactful changes first.
    Avoid over-engineering. Prefer simple, correct solutions over elegant ones."

Low arousal (<= -0.35):
    "You have time to explore. Consider multiple approaches before committing.
    Read surrounding code carefully. Look for patterns you can reuse."

Low pleasure (<= -0.35):
    "Recent attempts have had issues. Be extra careful with your changes.
    Double-check your work against the acceptance criteria before finishing."
```

---

## 2. The 10th Layer Target: PEEK Orientation Cache

The target design includes a 10th layer -- PEEK (Pre-Execution Environment
Knowledge) -- that would cache orientation information about the workspace
(file tree, recently modified files, build status) as a pre-computed prefix.
This layer would sit at the System cache tier and provide stable workspace
awareness without per-task computation.

Status: **Target design. Not yet implemented.**

---

## 3. Builder API

The SystemPromptBuilder uses a fluent builder pattern:

```rust
pub struct SystemPromptBuilder {
    role_identity: String,
    conventions: Option<String>,
    domain: Option<String>,
    context: Option<String>,
    pheromones: Vec<ContextChunk>,
    task: Option<String>,
    gate_feedback: Vec<String>,
    tools: Option<String>,
    relevant_skills: Vec<Skill>,
    relevant_playbooks: Vec<Playbook>,
    tool_hints: Option<String>,
    anti_patterns: Vec<String>,
    affect_state: Option<PadState>,
    temperament: Option<Temperament>,
    cache_markers: bool,
    token_budget: Option<usize>,
    budget_profile: Option<PromptBudget>,
    section_effectiveness: Option<SectionEffectivenessConfig>,
    model_hint: Option<String>,
}
```

Three build methods:

| Method | Purpose |
|--------|---------|
| `build()` | Flat string with optional cache markers |
| `build_with_counter(&TokenCounter)` | Budget-fitted string with token counting |
| `build_sections()` | Structured `Vec<PromptSection>` for PromptComposer |

### Section Effectiveness Learning

The builder accepts a `SectionEffectivenessRegistry` scoped to one role. When
present, sections whose learned effectiveness data indicates negative value can
be demoted or promoted according to `PriorityChange` directives. This enables
the system to learn which sections help or hurt for specific roles and task
types.

### Model Hint

An optional `model_hint` allows model-specific prompt formatting. Different
models may benefit from different instruction styles or emphasis patterns.

---

## 4. Cache Alignment Strategy

Cache alignment is the highest-leverage cost optimization. The goal: maximize
the byte-identical prefix across requests.

### 4.1 Prefix Tiers

```
Tier 1 (System): Role Identity + Conventions + Tools
  -> Identical across ALL tasks for this role
  -> Cache hit on every request after the first
  -> 90% discount (Anthropic), 50% (OpenAI)

Tier 2 (Session): Domain Context + Pheromones
  -> Identical across all tasks in the same plan
  -> Cache hit on all tasks within a plan run

Tier 3 (Task): Task Context + Techniques + Anti-Patterns
  -> Identical across iterations of the same task
  -> Cache hit on retry attempts

Tier 4 (Dynamic): Gate Feedback + Affect
  -> Unique per turn
  -> No cache benefit
```

### 4.2 Rules for Cache Stability

1. **Never randomize section ordering.** Deterministic priority sort only.
2. **Normalize whitespace.** `normalize_for_caching()` strips trailing spaces,
   normalizes newlines to `\n`, converts tabs to spaces.
3. **Sort tool definitions alphabetically.** `canonical_tool_order()` uses name
   comparison, not HashMap iteration.
4. **Freeze workspace map within a plan execution.** Generate once, reuse.
5. **Emit explicit layer markers.** The inference gateway places
   `cache_control` breakpoints at `<!-- roko:layer:N -->` markers.

### 4.3 Cost Impact

For a typical 20-plan run with 80 agent spawns:

| Without cache alignment | With cache alignment |
|------------------------|---------------------|
| ~$100 on Opus (20M tokens) | ~$19 on Opus |
| Every request pays full price | 90% discount on prefix layers |
| Tool definition order varies | Deterministic ordering |

---

## 5. Wiring into Orchestration

The SystemPromptBuilder is wired through `RoleSystemPromptSpec`:

```rust
pub struct RoleSystemPromptSpec {
    pub role: AgentRole,
    pub builder: SystemPromptBuilder,
}

impl RoleSystemPromptSpec {
    pub fn build_with_context_window(&self, context_window: usize) -> String
    pub fn compose_with_budget(&self, budget: &PromptBudget) -> String
}
```

In the runner event loop, the orchestrator builds the system prompt for each
agent spawn:

```rust
let spec = RoleSystemPromptSpec::for_role(task.role)
    .with_conventions(&conventions)
    .with_domain_context(&workspace_map, &prd_extract)
    .with_task_context(&task_toml, &brief, &gate_errors)
    .with_tools(&tool_defs)
    .with_anti_patterns(&playbook_rules)
    .with_affect(&daimon_state);

let system_prompt = spec.build_with_context_window(model_context_window);
```

---

## 6. Budget Profiles

Each layer has a default budget share adjustable by role via `PromptBudget`
and the `adaptive_budget_for()` function. See
[role-templates-11.md](role-templates-11.md) for the per-role allocation table.

---

## 7. The `--bare` Flag Experiment

Empirical evidence for the value of system prompts:

| Condition | Task Success Rate |
|-----------|------------------|
| `claude --bare` (no system prompt) | 15-25% |
| `claude` (with system prompt) | 60-75% |

A 3-4x quality gap from the system prompt alone. Combined with the ETH Zurich
finding that unnecessary instructions decrease success by ~3% and increase
token costs by 20%+, this motivates the builder's core design: construct
minimal, maximally effective prompts per specific task through layered
architecture.

---

## 8. Academic Foundations

**Anthropic Context Engineering** [2025]. Reframed "prompt engineering" as
"context engineering." Place grounding knowledge before task directives. The
SystemPromptBuilder's layered architecture implements this directly.

**ReAct: Reasoning + Acting** [Yao et al. 2022]. Interleaving reasoning traces
with task-specific actions. The 9-layer structure supports ReAct by placing
reasoning instructions alongside action instructions.

**Reflexion** [Shinn et al. 2023]. Verbal reinforcement learning: agents
reflect on failures using structured reflections. Gate errors and iteration
memory in Layers 4/4b are the Reflexion mechanism.

**Plan-and-Solve Prompting** [Wang et al. 2023]. Improved zero-shot reasoning
by splitting into plan then execute phases. The Strategist role's Layer 4
embodies this.

**Chain of Draft** [Zoom Research, arXiv:2502.18600]. 5-word-max intermediate
reasoning steps match CoT accuracy at 7.6% of token cost. Applicable to role
identity instructions for token-constrained contexts.

---

## 9. Implementation Status

| Aspect | Status |
|--------|--------|
| 9-layer builder | **Shipped** |
| Cache alignment markers | **Shipped** |
| Affect guidance (arousal, pleasure) | **Shipped** |
| Role-specific budget profiles | **Shipped** |
| Gate feedback layer (4b) | **Shipped** |
| Pheromone signals layer (3c) | **Shipped** |
| Section effectiveness learning | **Shipped** |
| Tool hints layer (6b) | **Shipped** |
| Temperament support | **Shipped** |
| Model hint formatting | **Shipped** |
| TokenCounter integration | **Shipped** |
| 12+ unit tests | **Passing** |
| 10th PEEK layer | **Target design** |
| Dominance affect guidance | **Not yet** |
| Dynamic layer ordering | **Designed** |
| Prompt compression integration | **Designed** |

---

## Cross-References

- [composer-trait.md](composer-trait.md) -- Compose trait definition
- [role-templates-11.md](role-templates-11.md) -- Role template details
- [token-budget-management.md](token-budget-management.md) -- Budget allocation
- [lost-in-the-middle-u-shape.md](lost-in-the-middle-u-shape.md) -- Attention curve
- [affect-modulated-retrieval.md](affect-modulated-retrieval.md) -- PAD modulation
- `crates/roko-compose/src/system_prompt_builder.rs` -- Implementation source
- `crates/roko-compose/src/templates/` -- Role template directory
