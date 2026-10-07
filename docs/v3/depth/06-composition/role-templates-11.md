# Role Templates: Per-Role Prompt Specialization

> **Depth file for [06-COMPOSITION.md](../../06-COMPOSITION.md)**
> Source: `crates/roko-compose/src/templates/`
> v1 source: `docs/v1/03-composition/03-role-templates.md`

---

## Overview

Each agent role in Roko receives a specialized system prompt tailored to its
task type. The role template system defines per-role identities, per-role token
budgets, and per-role section emphasis. Eleven templates are currently
implemented: implementer, strategist, scribe, reviewer, conductor, researcher,
refactorer, integration, quick, task_impl, and common. Each role receives a
different allocation of the token budget, emphasizing the context types most
critical to its function.

---

## 1. The 11 Templates

### 1.1 Implementer (`templates/implementer.rs`)

**Purpose:** Writes code to implement specified changes. The workhorse role.

**Identity prompt:**
```
You are a senior software engineer implementing changes to a Rust codebase.

Your job is to make the specific changes described in your task brief.
Follow the project's conventions. Write tests for new functionality.
Handle edge cases. Check compilation. Do not over-engineer.

Read context/in/execution-pack.md for your main context.
Read context/in/brief.md for your task brief.
Read the narrowest artifacts first -- only open broader context if needed.
```

**Budget emphasis:** Largest file_context (8K) because it needs actual source
code. Large workspace_map (20K) for navigation. Large brief (8K) for detailed
task description. Largest skills (8K) for playbook rules.

### 1.2 Strategist (`templates/strategist.rs`)

**Purpose:** Decomposes complex tasks into subtasks. Plans execution order and
identifies dependencies.

**Identity prompt:**
```
You are a technical strategist who decomposes complex tasks into actionable
subtasks.

Your job is to break down the implementation plan into a set of tasks with
clear dependencies, ordered for execution. You never write code -- you
produce plans, decompositions, and task TOMLs.

Analyze the plan content and workspace structure. Identify which crates are
affected. Map dependencies between tasks. Estimate complexity.
```

**Budget emphasis:** Large workspace_map (20K) to see project structure. Zero
file_context -- Strategist plans but does not code.

### 1.3 Scribe (`templates/scribe.rs`)

**Purpose:** Technical documentation. Writes docstrings, README sections,
architecture docs.

**Identity prompt:**
```
You are a technical writer documenting implementations in a Rust codebase.

Your job is to produce accurate, well-structured documentation that cites
specifications and academic references where appropriate. Follow the
project's documentation patterns. Be precise about type signatures.
```

**Budget emphasis:** Moderate budgets; the plan and brief carry the specification
it cites. Large file_context (6K) to see the code being documented.

### 1.4 Reviewer (`templates/reviewer.rs`)

**Purpose:** Reviews implementation for architectural quality, consistency with
project patterns, and cross-crate impact.

**Identity prompt:**
```
You are a software architect reviewing implementation quality.

Evaluate design decisions. Check interface contracts. Identify coupling
issues. Verify that changes follow project conventions and do not introduce
regressions.

Format your review as:
## Issues Found
- [severity] [file:line] Description
## Suggestions
- [priority] Description
```

**Budget emphasis:** Moderate across all sections. Smaller workspace_map (6K)
because reviews are focused. Moderate file_context (6K).

### 1.5 Conductor (`templates/conductor.rs`)

**Purpose:** Coordinates multi-agent plan execution. The meta-role.

**Identity prompt:**
```
You coordinate multi-agent plan execution.

Monitor progress across tasks. Resolve conflicts between agents. Allocate
tasks based on role fitness. Report overall plan status.
```

**Budget emphasis:** Large plan (to see full execution plan). Moderate across
other sections.

### 1.6 Researcher (`templates/researcher.rs`)

**Purpose:** Conducts deep research on technical topics with citations.

**Identity prompt:**
```
You conduct deep research on technical topics.

Find and cite primary sources. Produce structured research artifacts with
clear methodology. Distinguish established findings from speculation.
```

**Budget emphasis:** Default budgets. Moderate skills (for research
methodologies).

### 1.7 Refactorer (`templates/refactorer.rs`)

**Purpose:** Restructures code without changing behavior.

**Identity prompt:**
```
You restructure code without changing behavior.

Preserve all public API contracts. Reduce duplication. Improve structure.
Run existing tests to verify behavioral preservation.
```

**Budget emphasis:** Large file_context (to see code being refactored). Large
workspace_map (to understand impact).

### 1.8 Integration Tester (`templates/integration.rs`)

**Purpose:** Validates that changes work across system boundaries.

**Identity prompt:**
```
You validate that changes work across system boundaries.

Test cross-crate interactions. Check public API contracts. Validate
integration points. Write integration tests that exercise the full path.
```

**Budget emphasis:** Moderate workspace_map (cross-crate relationships).
Moderate file_context (test files and interfaces).

### 1.9 Quick Reviewer (`templates/quick.rs`)

**Purpose:** Fast-turnaround code review for simple changes.

**Identity prompt:**
```
You are a fast-turnaround code reviewer.

Focus on obvious bugs, formatting violations, and convention breaks.
Do not evaluate architecture. Keep reviews concise.
```

**Budget emphasis:** Minimal budgets across the board. Designed for
low-token-cost reviews.

### 1.10 Task Implementer (`templates/task_impl.rs`)

**Purpose:** Implements a specific, well-defined task with tight scope.

**Identity prompt:**
```
You implement a specific, well-defined task.

Follow the acceptance criteria exactly. Do not expand scope. If blocked,
report the blocker rather than working around it.
```

**Budget emphasis:** Focused on task brief and file context. Minimal
workspace_map.

### 1.11 Common (`templates/common.rs`)

**Purpose:** Shared utilities, the PromptBudget struct, budget_for() function,
and adaptive_budget_for(). Not a role template itself but the foundation for
all templates.

---

## 2. The PromptBudget Struct

```rust
// crates/roko-compose/src/templates/common.rs

pub struct PromptBudget {
    pub plan: usize,
    pub workspace_map: usize,
    pub context: usize,
    pub brief: usize,
    pub reviews: usize,
    pub instructions: usize,
    pub file_context: usize,
    pub skills: usize,
}
```

### Budget Allocation Table

```rust
pub const fn budget_for(role: AgentRole) -> PromptBudget {
    match role {
        AgentRole::Implementer => PromptBudget {
            plan: 50_000, workspace_map: 20_000,
            context: 4_000, brief: 8_000, reviews: 3_000,
            instructions: 4_000, file_context: 8_000, skills: 8_000,
        },
        AgentRole::Strategist => PromptBudget {
            plan: 50_000, workspace_map: 20_000,
            context: 4_000, brief: 6_000, reviews: 3_000,
            instructions: 4_000, file_context: 0, skills: 4_000,
        },
        AgentRole::Architect | AgentRole::Auditor => PromptBudget {
            plan: 50_000, workspace_map: 6_000,
            context: 2_000, brief: 4_000, reviews: 3_000,
            instructions: 4_000, file_context: 6_000, skills: 4_000,
        },
        AgentRole::Scribe => PromptBudget {
            plan: 50_000, workspace_map: 6_000,
            context: 4_000, brief: 6_000, reviews: 3_000,
            instructions: 4_000, file_context: 6_000, skills: 4_000,
        },
        _ => PromptBudget {
            plan: 50_000, workspace_map: 8_000,
            context: 4_000, brief: 4_000, reviews: 2_000,
            instructions: 4_000, file_context: 6_000, skills: 4_000,
        },
    }
}
```

### Key Budget Differences

| Section | Implementer | Strategist | Scribe | Default |
|---------|------------|------------|--------|---------|
| workspace_map | **20K** | **20K** | 6K | 8K |
| file_context | **8K** | **0** | 6K | 6K |
| brief | **8K** | 6K | 6K | 4K |
| skills | **8K** | 4K | 4K | 4K |

Key asymmetries:
- **Implementer gets most file_context** (8K) -- needs to see existing code
- **Strategist gets zero file_context** -- plans, never codes
- **Implementer gets most skills** (8K) -- playbook rules prevent repeated
  implementation mistakes

---

## 3. Complexity-Adaptive Budgets

Base budgets are adjusted by task complexity through `adaptive_budget_for()`:

```rust
pub enum Complexity {
    Trivial,   // Two-line fix, rename. ~4K total
    Standard,  // Standard implementation. ~12K total
    Complex,   // Cross-crate integration. ~24K total
}
```

| Complexity | Budget Effect |
|-----------|--------------|
| Trivial | Drop context and skills entirely. Halve workspace_map and brief. |
| Standard | No change. Base budget applies. |
| Complex | +50% workspace_map, +100% context, +50% file_context. ~40% increase. |

---

## 4. Shared Stanzas

Shared text fragments used across multiple role templates:

### CONTEXT_LAYOUT_STANZA

```
Read context/in/execution-pack.md for your main context.
Read context/in/brief.md for your task brief.
Read the narrowest artifacts first -- only open broader context if needed.
```

### MCP_TOOLS_STANZA

```
You have access to MCP tools via the configured MCP servers.
Use tools as described in their schemas. Do not guess parameters.
Prefer MCP tools over shell commands when both are available.
```

### NITS_FORMAT

```
Format your review as:
## Issues Found
- [severity] [file:line] Description
## Suggestions
- [priority] Description
```

---

## 5. Truncation Helpers

```rust
pub fn truncate(content: &str, max_chars: usize) -> String
pub fn truncate_tail(content: &str, max_chars: usize) -> String
```

| Section | Strategy | Rationale |
|---------|----------|-----------|
| workspace_map | truncate (keep beginning) | Top of tree is most important |
| gate_errors | truncate_tail (keep end) | Most recent errors are most relevant |
| file_context | truncate (keep beginning) | Headers and imports are most important |

---

## 6. Role-to-Context-Tier Mapping

| Role | Default Tier | Default Model | Rationale |
|------|-------------|---------------|-----------|
| Strategist | Full | Opus | Strategic planning needs maximum context |
| Implementer | Focused | Sonnet | Implementation needs focused context |
| Reviewer | Focused | Sonnet | Reviews are focused operations |
| Researcher | Full | Opus | Research needs maximum context |
| Conductor | Full | Opus | Coordination needs full plan visibility |
| Scribe | Focused | Sonnet | Documentation needs moderate context |
| Refactorer | Focused | Sonnet | Refactoring needs focused context |
| Integration | Focused | Sonnet | Integration testing needs moderate context |
| QuickReviewer | Surgical | Haiku | Fast, cheap reviews |
| TaskImplementer | Focused | Sonnet | Focused single-task work |

The CascadeRouter may override these defaults based on historical performance.

---

## 7. Empirical Budget Analysis

From prompt-logs analysis during development:

| Section | Avg Tokens | % of Prompt | Pass Rate When Present |
|---------|-----------|-------------|----------------------|
| Learning Pack | 2,347 | 49% | 61% |
| PRD2 Context (section since removed) | 712 | 15% | 67% |
| Strategist Brief | 491 | 10% | **72%** |
| Workspace Map | 334 | 7% | 64% |
| Execution Strategy | 298 | 6% | 58% |
| Cross-Plan Context | 243 | 5% | 55% |
| Your Assignment | 189 | 4% | **71%** |
| MCP Tools | 78 | 2% | 65% |
| Self-Review | 78 | 2% | 63% |

Key findings:
- **Brief and Assignment** have highest pass rates (72%, 71%) at lowest token
  costs -- highest value per token
- **Learning Pack** dominates at 49% of tokens but has the lowest pass rate
  (61%) -- may be adding noise
- **Cross-Plan Context** has lowest pass rate (55%) and may actively hurt
  simple tasks

These findings motivated complexity-adaptive budgets: Trivial tasks drop
Learning Pack and Cross-Plan Context entirely.

---

## 8. Implementation Status

| Aspect | Status |
|--------|--------|
| 11 role templates | **Shipped** |
| PromptBudget per role | **Shipped** |
| Complexity-adaptive budgets | **Shipped** |
| Truncation helpers | **Shipped** |
| Shared stanzas | **Shipped** |
| Wired into runner dispatch | **Shipped** |
| Per-role pass rate tracking | **Shipped** (via efficiency events) |
| Section effectiveness learning | **Shipped** |
| Learned budget optimization (DSPy) | **Not yet** |

---

## Cross-References

- [system-prompt-builder-9-layer.md](system-prompt-builder-9-layer.md) -- Builder layers
- [enrichment-pipeline-13-step.md](enrichment-pipeline-13-step.md) -- Enrichment artifacts
- [token-budget-management.md](token-budget-management.md) -- Budget allocation
- `crates/roko-compose/src/templates/common.rs` -- Budget table
- `crates/roko-compose/src/templates/implementer.rs` -- Implementer template
- `crates/roko-compose/src/templates/strategist.rs` -- Strategist template
