# Context Windowing Strategies

How to scale prompt assembly to match model capability. The core insight:
a 4K-context local model and a 200K-context Opus need fundamentally different
prompts, not just truncated versions of the same prompt.

---

## 1. The Problem

Task TOMLs in roko accumulate context across 9+ layers:

| Layer | Typical Size | Total |
|---|---|---|
| Role identity | 200-500 tokens | 500 |
| Conventions | 100-300 tokens | 300 |
| Domain context | 500-2000 tokens | 2000 |
| Workspace map | 200-500 tokens | 500 |
| Plan context | 1000-5000 tokens | 5000 |
| Task description | 200-1000 tokens | 1000 |
| Gate feedback | 0-500 tokens | 500 |
| Tool instructions | 200-500 tokens | 500 |
| Playbooks/skills | 200-1000 tokens | 1000 |
| Anti-patterns | 100-500 tokens | 500 |
| Affect guidance | 50-200 tokens | 200 |
| **Total** | | **~12K tokens** |

This is the *compressed* version. With full PRD extract (3K tokens), research
memo (2K tokens), prior episode history (1K tokens), and file context (2K tokens),
a fully-loaded Implementer prompt can reach 20K-30K tokens.

For Opus (200K context), this leaves 170K+ tokens for the conversation. Fine.
For Sonnet (200K context), this is manageable but the model works best with
focused prompts under 16K system. Acceptable but suboptimal.
For Haiku (200K context), the model's instruction-following degrades sharply
past 8K tokens of system prompt. Problematic.
For Ollama/Gemma (4K-32K context), the prompt *exceeds the entire context window*.
Catastrophic.

---

## 2. Tier Architecture

The solution is not truncation -- it is tier-specific prompt *composition*.
Different tiers do not just include less content; they include *different*
content organized *differently*.

### 2.1 Surgical Tier (4K token budget)

**Models:** Ollama, Gemma, Llama, Qwen, Mistral, CodeLlama, DeepSeek, Phi, StarCoder.
**Task types:** Mechanical, single-file, well-scoped changes.

**What to include:**
- Role identity (1 sentence, not 5 paragraphs)
- Task description (exact change required)
- Inline file content (the specific file to modify)
- Verification command (the exact command to run)
- Anti-patterns (top 1-2 most relevant)

**What to exclude:**
- Plan context (the model cannot reason about plan structure)
- PRD extract (too abstract for mechanical tasks)
- Workspace map (the model should not be exploring)
- Research memos (unnecessary for mechanical work)
- Playbooks (the task is too simple for multi-step playbooks)
- Conventions (inline in the file context instead)
- Episode history (irrelevant for focused tasks)

**Format:**
```
You are a Rust developer. Make the following change:

TASK: Add a `pub fn foo()` method to `crates/roko-core/src/lib.rs`

FILE CONTENT (crates/roko-core/src/lib.rs):
[exact file content, truncated to 2K tokens]

VERIFY: cargo check -p roko-core

DO NOT: [top 1-2 anti-patterns]
```

The key insight: surgical prompts are *imperative* not *collaborative*. The model
is told exactly what to do, not asked to figure it out.

### 2.2 Focused Tier (12K token budget)

**Models:** Sonnet, GPT-4o-mini.
**Task types:** Focused, integrative, multi-file but scoped to one subsystem.

**What to include:**
- Role identity (full paragraph)
- Task description with acceptance criteria
- Conventions (project-specific)
- Brief (strategist's guidance for this task)
- Dependency outputs (what prior tasks produced)
- File context (up to 3 relevant files, summarized)
- Anti-patterns (top 3-5)
- Playbook steps (most relevant playbook)
- Gate feedback (if retry)

**What to exclude:**
- Full PRD extract (summarize to 2 sentences)
- Full workspace map (show only relevant crate)
- Research memos (unless directly relevant)
- Cross-plan context (scope to current plan)
- Episode history (last 2-3 only)

**Format:** Standard 9-layer builder output, but with per-section caps scaled
to fit 12K total. Sections are prioritized by task relevance rather than
included unconditionally.

### 2.3 Full Tier (24K token budget)

**Models:** Opus, GPT-4, Gemini Pro.
**Task types:** Architectural, cross-crate, design-level.

**What to include:** Everything. All 9 layers at full fidelity:
- Full role identity with responsibilities and constraints
- Project conventions with examples
- Domain context with knowledge store entries
- Full workspace map
- Plan context with PRD extract
- Task description with acceptance criteria and examples
- Gate feedback with full error output
- Tool instructions with MCP tools
- All relevant playbooks
- Anti-patterns with full descriptions
- Affect guidance
- Episode history (last 5)
- Research memo excerpts

**Format:** Full 9-layer builder output. Cache-aligned with stability tier
markers. `dynamic_placement()` assigns high-value sections to prompt edges.

### 2.4 Extended Tier (100K+ token budget, future)

**Models:** Future 500K+ context models, or current models with explicit
long-context prompting.

**What to include:** Everything in Full, plus:
- Raw source files (not summaries)
- Multiple playbook matches (not just top 1)
- Full episode transcripts (not just metadata)
- Dream consolidation insights
- Cross-plan context with full dependency chain
- Research memos in full
- Prior review feedback in full

This tier is speculative but architecturally important: the system should be
able to scale *up* as well as down.

---

## 3. Tier Selection Logic

### 3.1 Model-Based Selection

The existing `is_local_model()` function in `context_provider.rs` correctly
identifies local models. Extend with explicit model-to-tier mapping:

```
Claude Opus      -> Full
Claude Sonnet    -> Focused
Claude Haiku     -> Surgical (conservative) or Focused (aggressive)
GPT-4            -> Full
GPT-4o-mini      -> Focused
GPT-4o           -> Full
Gemini Pro       -> Full
Gemini Flash     -> Focused
Ollama/*         -> Surgical
DeepSeek         -> Surgical (local) or Focused (API)
```

### 3.2 Task-Based Override

Task complexity can override the model-based default:
- A trivial task on Opus still gets Focused tier (no point sending 24K tokens
  for "add a semicolon")
- A complex task on Sonnet might get Full tier if the model has enough context
  window headroom

```
ContextTier::from_task_and_model(task_tier, model_slug)
```

This function already exists in `context_provider.rs`:
- "mechanical" tasks -> Surgical
- "focused"/"integrative" tasks -> Focused (or Surgical for local models)
- "architectural" tasks -> Full (or Surgical for local models)

### 3.3 Available Context Window Override

When the model's actual context window is known (from provider metadata or
config), use it to set a hard ceiling:

```
effective_budget = min(tier_default_budget, model_context_window * 0.15)
```

The 15% factor reserves 85% of the context window for the conversation
(user messages, tool results, assistant responses). This is aggressive for
short tasks (could use 25%) and conservative for long tasks (could use 10%).
A learning-based factor would be better:

```
effective_budget = min(
    tier_default_budget,
    model_context_window * budget_predictor.system_prompt_ratio(model)
)
```

Where `system_prompt_ratio()` converges on the actual fraction of context
window that system prompts consume for successful tasks.

---

## 4. Section Priority by Tier

Not all sections are equally important. Within each tier, sections are ranked
and the budget is allocated greedily by rank:

### Surgical Tier Priority Order:
1. Task description (Critical -- the model must know what to do)
2. Inline file content (Critical -- the model must see the code)
3. Verification command (High -- the model must know how to check)
4. Role identity (Normal -- single sentence)
5. Anti-patterns (Normal -- top 1-2 only)

### Focused Tier Priority Order:
1. Task description (Critical)
2. Role identity (High)
3. Gate feedback (High -- if retry, this is the most important context)
4. Anti-patterns (High -- prevent known failures)
5. Playbook steps (Normal)
6. Conventions (Normal)
7. File context (Normal)
8. Brief (Normal)
9. Dependency outputs (Low)
10. Episode history (Low -- 2-3 entries only)

### Full Tier Priority Order:
1. Task description (Critical)
2. Role identity (Critical)
3. Gate feedback (High -- if retry)
4. Anti-patterns (High)
5. Plan context (High)
6. File context (High)
7. Conventions (Normal)
8. Playbook steps (Normal)
9. Brief (Normal)
10. Domain context (Normal)
11. Workspace map (Normal)
12. Episode history (Normal)
13. Research memo (Low)
14. Affect guidance (Low)

---

## 5. Dynamic Budget Allocation

### 5.1 Initial Allocation

Given a tier budget (e.g., 12K tokens for Focused), allocate proportionally
to section priority:

```
Critical sections: each gets up to 25% of budget
High sections: each gets up to 15% of budget
Normal sections: share remaining budget equally
Low sections: share remaining budget (after Normal) equally
```

If Critical + High sections consume 70% of budget, Normal sections split the
remaining 30%. If Critical sections alone exceed 50%, start dropping Low
sections, then Normal sections (lowest-scoring first).

### 5.2 Learned Adjustment

After the initial allocation, apply `SectionInfluence.weights()` multipliers:

```
adjusted_cap = initial_cap * influence_weight  // [0.5, 1.5]
```

This shifts budget toward sections that historically improve success and away
from sections that do not.

### 5.3 Overflow Handling

When total section content exceeds the tier budget after allocation:

1. **Truncate Low-priority sections first** (reverse priority order)
2. **Summarize rather than truncate when possible** (use compaction for
   conversation history, extractive summarization for documents)
3. **Never truncate Critical sections** -- if a Critical section exceeds its
   cap, steal budget from Normal/Low sections

---

## 6. Per-Section Windowing Strategies

### 6.1 Workspace Map

**Surgical:** Omit entirely. The task description should name the exact file.
**Focused:** Show only the crate containing the task's target files. Filter
to files modified in the last 7 days or referenced by the task.
**Full:** Show entire workspace, capped at 300 lines. Highlight files
referenced by the task description.

### 6.2 Episode History

**Surgical:** Omit entirely.
**Focused:** Last 2-3 episodes with same role. Show only status, duration,
and failed gate (if any). One line per episode.
**Full:** Last 5 episodes. Show status, model, duration, tokens, failed gate.
Include episodes from related tasks (same plan or same crate).

### 6.3 Knowledge Store Entries

**Surgical:** Only anti-patterns with confidence >= 0.8. Maximum 2 entries.
**Focused:** Facts with confidence >= 0.5, techniques >= 0.3, anti-patterns
>= 0.2. Maximum 5 entries.
**Full:** Facts >= 0.3, techniques >= 0.2, anti-patterns >= 0.1. Maximum 10
entries. Include knowledge from related domains.

### 6.4 Playbooks

**Surgical:** Omit entirely.
**Focused:** Top 1 matching playbook. Show only step names, not full descriptions.
**Full:** Top 3 matching playbooks. Show step names and descriptions.

### 6.5 File Context

**Surgical:** Inline the exact file to modify. Truncate to 2K tokens. If
multiple files, inline only the primary target; name the others.
**Focused:** Up to 3 relevant files, each summarized to key functions/structs.
Total ~3K tokens.
**Full:** Up to 5 relevant files. Include full function bodies for referenced
symbols. Total ~8K tokens.

### 6.6 PRD Extract

**Surgical:** Omit entirely.
**Focused:** 2-sentence summary of the relevant PRD section.
**Full:** Full PRD extract, up to 3K tokens.

---

## 7. Cache Alignment by Tier

### System Prefix (stable across tasks, cacheable)

| Tier | System Prefix Content | Approx Tokens |
|---|---|---|
| Surgical | Role identity (1 sentence) | 50 |
| Focused | Role identity + conventions | 300-500 |
| Full | Role identity + conventions + tool instructions | 500-1000 |

### Session Prefix (stable within a plan run, cacheable)

| Tier | Session Prefix Content | Approx Tokens |
|---|---|---|
| Surgical | (none) | 0 |
| Focused | Workspace map (filtered) | 200-400 |
| Full | Workspace map + plan context + domain context | 1000-3000 |

### Task Content (per-task, not cacheable)

Everything else: task description, gate feedback, playbooks, anti-patterns,
file context, episode history, affect guidance.

**Cache break markers** are inserted between tiers. The LLM provider can
reuse the System and Session prefixes across multiple task dispatches within
the same plan run, reducing input token costs by 30-60%.

---

## 8. Integration with Existing Code

### Where ContextTier Lives

`crates/roko-compose/src/context_provider.rs` lines 38-115:
- `ContextTier` enum (Surgical/Focused/Full)
- `from_task_and_model()` -- derives tier from task type and model slug
- `default_token_budget()` -- 4K/12K/24K
- `is_local_model()` -- detects local models

### Where Budget Lives

`crates/roko-compose/src/budget.rs`:
- `adjusted_budget_for()` -- complexity-scaled per-role budgets
- Need to add: `tier_constrained_budget()` that caps adjusted budgets to tier limit

`crates/roko-compose/src/templates/common.rs`:
- `budget_for()` -- static per-role character budgets
- These become the *base* budgets that `tier_constrained_budget()` scales down

### Where Assembly Lives

`crates/roko-compose/src/prompt_assembly_service.rs`:
- `assemble()` -- needs `model_slug` parameter to determine tier
- Per-section inclusion logic needs tier-aware filtering
- Knowledge confidence thresholds need tier-dependent values

### Where Dispatch Lives

`crates/roko-cli/src/orchestrate.rs` -- `dispatch_agent_with()`:
- Model slug is available from CascadeRouter selection
- Thread model slug into prompt assembly
- Thread tier into context retrieval (scope queries to tier-appropriate sources)

---

## Sources

- `crates/roko-compose/src/context_provider.rs` -- ContextTier, is_local_model, tier budgets
- `crates/roko-compose/src/budget.rs` -- adjusted_budget_for, Complexity
- `crates/roko-compose/src/templates/common.rs` -- budget_for, PromptBudget
- `crates/roko-compose/src/prompt_assembly_service.rs` -- assemble, thresholds
- `crates/roko-compose/src/attention.rs` -- PositionAttentionModel, dynamic_placement
- `crates/roko-compose/src/foraging.rs` -- MultiPatchForager, sufficiency estimation
- `crates/roko-compose/src/compaction.rs` -- compact_history, truncation
- `crates/roko-cli/src/orchestrate.rs` -- dispatch_agent_with, model routing
