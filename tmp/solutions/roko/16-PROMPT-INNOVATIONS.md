# Prompt Engineering Innovations

State-of-the-art techniques (2025-2026) applicable to roko's prompt assembly
system. Each technique is evaluated for integration feasibility and mapped
to specific components in `roko-compose`.

---

## 1. Structured Prompting and Schema Enforcement

### 1.1 XML/JSON Schema in System Prompts

Research consensus (2025): embedding output schema directly in the system prompt
produces more reliable structured output than post-hoc parsing. The technique:
include the exact schema as part of the role identity, not just as instructions.

**Current state in roko:** Templates like `ReviewerTemplate` include TOML
verdict format instructions (`format_verdict_instructions()` in `templates/common.rs`).
This is correct but incomplete -- the format is specified as prose instructions
rather than as a machine-parseable schema.

**Improvement:** Add an optional `output_schema` field to `SystemPromptBuilder`
that embeds a JSON Schema or TOML schema directly. The model sees:

```
Your output MUST conform to this schema:

```json
{
  "verdict": { "overall": "approve|revise", "code": "approve|revise" },
  "issues": [{ "id": "string", "severity": "blocking|warning", "file": "string" }]
}
```

**Integration point:** `crates/roko-compose/src/system_prompt_builder.rs` --
add a `with_output_schema()` builder method that injects the schema as a new
Layer 4c (between task and gate feedback, since it specifies expected output
for the current task).

### 1.2 Constrained Decoding via Provider APIs

Claude's tool-use API and OpenAI's `response_format` parameter enforce output
structure at the token generation level (constrained decoding). This is more
reliable than prompt-based schema enforcement.

**Current state:** roko's agent backends (`roko-agent`) support tool calling
but do not use provider-level structured output modes. Verdict parsing is
post-hoc string matching.

**Improvement:** When the target model supports structured output (Claude API,
OpenAI API), use the provider's constrained decoding mode in addition to the
prompt-level schema. This gives a "belt and suspenders" approach: the prompt
tells the model what to produce, and the API enforces the structure.

**Integration point:** `crates/roko-agent/src/openai_compat_backend.rs` and
the Claude API backend -- pass `response_format` when available.

---

## 2. Chain-of-Thought and Reasoning Control

### 2.1 Adaptive CoT Depth

Research finding (2025-2026): the optimal chain-of-thought depth depends on
task complexity. Simple tasks degrade with verbose reasoning (the model
"overthinks" and introduces errors). Complex tasks require deep reasoning.

**Mapping to roko tiers:**
- **Surgical tier:** Suppress reasoning. Use "Do this. Do not explain." phrasing.
  Small models waste context on reasoning they cannot execute well.
- **Focused tier:** Brief reasoning. "Think through the approach in 2-3 sentences,
  then implement."
- **Full tier:** Deep reasoning. "Analyze the codebase structure, identify
  the relevant abstractions, explain your approach, then implement."

**Integration point:** Add a `reasoning_depth` field to `SystemPromptBuilder`
derived from `ContextTier`. Include tier-appropriate reasoning instructions
in Layer 1 (role identity).

```rust
pub enum ReasoningDepth {
    Suppress,     // "Do not explain. Just implement."
    Brief,        // "Briefly explain your approach, then implement."
    Deep,         // "Think step by step. Analyze, explain, implement."
}
```

### 2.2 Structured Reasoning Blocks

Format reasoning as labeled blocks that can be parsed and evaluated:

```
<reasoning>
APPROACH: Modify the budget system to accept a ContextTier parameter.
RISKS: Changing the budget function signature affects all callers.
DECISION: Use a new wrapper function to avoid breaking existing callers.
</reasoning>

<implementation>
[actual code changes]
</implementation>
```

Benefits:
- Reasoning blocks can be stripped before passing to downstream agents
- Reasoning quality can be evaluated as a gate criterion
- Reasoning content provides rich data for section influence learning

**Integration point:** Add labeled-block instructions to templates in
`crates/roko-compose/src/templates/`. The reviewer can parse `<reasoning>`
blocks to evaluate decision quality.

---

## 3. Hierarchical Context Organization

### 3.1 Progressive Disclosure

Instead of dumping all context at once, organize information in expanding
layers that the model reads top-down:

```
## Task (CRITICAL - read this first)
[Task description]

## Context (READ if you need more information)
[Conventions, brief, dependency outputs]

## Reference (CONSULT if unsure)
[PRD extract, research memo, episode history]

## Background (SKIM only if relevant)
[Workspace map, domain context, affect guidance]
```

This exploits the primacy effect (models pay more attention to content at the
start of the prompt) and gives the model a signal about which sections deserve
deep reading vs skimming.

**Integration point:** `crates/roko-compose/src/system_prompt_builder.rs` --
the layer ordering already approximates this (identity and task are early,
anti-patterns and affect are late). Add explicit disclosure-level headers
between stability tiers.

### 3.2 Section Gating with Conditional Expansion

An advanced technique: include section *headers* in the base prompt, with
instructions to request expansion via tool calls:

```
## Anti-Patterns
[5 known anti-patterns are available. Request with `get_anti_patterns()`
if you encounter a pattern that seems risky.]
```

This keeps the base prompt small (Surgical tier) while giving the model
access to full context on demand (via tool calls). The model "pulls" context
rather than having it "pushed."

**Feasibility:** Requires models that reliably use tools for context retrieval.
Claude Opus and Sonnet can do this. Haiku and local models cannot. Appropriate
for Focused and Full tiers only.

**Integration point:** Requires changes to the tool dispatch system in
`roko-agent` to support "context retrieval" tools, and changes to the prompt
builder to include section headers without section content.

---

## 4. Anti-Pattern Inoculation

### 4.1 Negative Examples with Corrections

Research (2025): including negative examples ("DO NOT do this") with corrected
versions ("DO this instead") is more effective than negative-only instructions.

**Current state:** roko's Layer 7 (anti-patterns) includes warnings like
"WARNING: Do not use `unwrap()` in production code" but does not include
the correction ("Use `unwrap_or_else()` or propagate with `?`").

**Improvement:** Extend anti-pattern entries in the knowledge store to include
`correction` and `example` fields:

```rust
KnowledgeEntry {
    kind: KnowledgeKind::AntiKnowledge,
    content: "Do not use `unwrap()` in production code",
    correction: Some("Use `unwrap_or_else(|| ...)` or propagate with `?`"),
    example: Some("BAD: let x = map.get(k).unwrap();\nGOOD: let x = map.get(k).ok_or(Error::Missing)?;"),
    ...
}
```

**Integration point:** `crates/roko-neuro/src/knowledge_store.rs` (add fields)
and `crates/roko-compose/src/prompt_assembly_service.rs` (format corrections
alongside anti-patterns in Layer 7).

### 4.2 Gate Failure Injection as Negative Examples

When a task fails a gate, the gate feedback (Layer 4b) should include not
just the error but the pattern that caused it, so the model recognizes the
pattern on retry:

```
## Gate Failure (attempt 2 of 3)

ERROR: cargo clippy failed:
  error: unused variable `x` (crates/foo/src/lib.rs:42)

PATTERN: This often happens when refactoring removes usages.
FIX: Either use the variable or prefix with `_x`.
```

**Current state:** Gate feedback injection exists (`with_gate_feedback_text()`
in `SystemPromptBuilder`). The feedback includes raw error text but not
the pattern/fix framing.

**Integration point:** `crates/roko-compose/src/gate_feedback.rs` -- add
pattern classification and fix suggestion to gate feedback formatting.

---

## 5. Metacognitive Prompting

### 5.1 Confidence Calibration

Include instructions for the model to self-assess confidence:

```
After completing the task, rate your confidence:
- HIGH: I am certain this is correct and will pass all gates.
- MEDIUM: This should work but I am uncertain about [specific concern].
- LOW: This is my best attempt but I expect [specific failure mode].

If LOW, explain what additional context would help.
```

Benefits:
- Low-confidence signals can trigger model escalation (upgrade to Opus)
- Confidence ratings feed into the learning loop (calibration tracking)
- The model's stated concerns can pre-populate gate expectations

**Integration point:** Add confidence calibration instructions to Layer 1
(role identity) for all roles. Parse confidence from model output and
record in episode metadata.

### 5.2 Reflective Self-Repair

For retry scenarios (gate failure), include a reflective prompt:

```
The previous attempt failed gate: [gate_name]

Before retrying, answer:
1. What went wrong? (root cause, not just the error message)
2. What assumption was incorrect?
3. What will you do differently this time?

Then implement the fix.
```

Research (2025-2026) shows that explicit reflection before retry improves
success rates by 15-30% compared to simply re-prompting with the error.

**Integration point:** `crates/roko-compose/src/gate_feedback.rs` -- add
reflective prompting to gate feedback formatting. This changes the gate
feedback from "here is the error, fix it" to "here is the error, reflect
on it, then fix it."

---

## 6. Attention Steering Techniques

### 6.1 Salience Markers

Use formatting to draw model attention to critical content:

```
>>> CRITICAL: The function signature MUST match this exactly: <<<
pub fn adjusted_budget_for(role: AgentRole, tier: ContextTier) -> AdjustedBudget
```

Triple angle brackets, CAPS, and explicit "CRITICAL" labels have been shown to
improve instruction following for Claude and GPT models.

**Integration point:** `crates/roko-compose/src/system_prompt_builder.rs` --
add salience markers around Critical-priority sections and acceptance criteria.

### 6.2 Instruction Anchoring

Repeat the most important instructions at both the beginning and end of the
prompt. This exploits both primacy and recency effects:

```
[START OF PROMPT]
TASK: Wire ContextTier into dispatch_agent_with.
[... 10K tokens of context ...]
REMINDER: Your task is to wire ContextTier into dispatch_agent_with.
[END OF PROMPT]
```

**Integration point:** When `dynamic_placement()` assigns sections, add a
compressed task reminder at the `End` position in addition to the full task
description at the `Start` position.

### 6.3 Negative Space

Explicitly state what is NOT part of the task, to prevent scope creep:

```
You are NOT:
- Refactoring the budget system
- Adding new roles
- Changing the template format

You ARE:
- Adding a ContextTier parameter to the assembly pipeline
- Scaling budgets to fit the tier's token limit
```

**Integration point:** Add an optional `out_of_scope` field to
`TaskContext` that is rendered as negative instructions in Layer 4.

---

## 7. Context Compression Techniques

### 7.1 Extractive Summarization for Documents

For PRD extracts, research memos, and other long documents: instead of
truncating at a character limit, extract the most relevant sentences:

1. Split document into sentences
2. Score each sentence by term overlap with task description
3. Select top-N sentences (within budget)
4. Preserve original order

This produces a summary that is both shorter and more relevant than
head-truncation.

**Integration point:** Add `extractive_summarize()` to
`crates/roko-compose/src/compaction.rs` alongside the existing
conversation compaction.

### 7.2 Symbol-Level Code Context

Instead of including raw file content, extract only the relevant symbols:

```
// Instead of including all 500 lines of lib.rs:
pub struct ContextTier { ... }              // line 38-45
pub fn from_task_and_model(...) -> Self     // line 54-64
pub const fn default_token_budget(self)     // line 68-74
pub fn is_local_model(slug: &str) -> bool   // line 98-115
```

This is 10x more token-efficient than raw file inclusion and provides the
model with exactly the API surface it needs.

**Integration point:** `crates/roko-compose/src/symbol_resolver.rs` already
exists. Wire it into the context assembly pipeline to produce symbol-level
summaries for file context sections.

### 7.3 Diff-Based Context for Retry

On retry (gate failure), instead of including the full file context again,
include only the diff from the previous attempt:

```
## Previous Attempt Diff
- fn budget_for(role: AgentRole) -> PromptBudget {
+ fn budget_for(role: AgentRole, tier: ContextTier) -> PromptBudget {
```

This is dramatically more efficient than re-including the entire file,
especially for large files. The model sees exactly what changed and can
focus on fixing the specific issue.

**Integration point:** Gate feedback (Layer 4b) should include the diff
when available (from the git working tree or from the agent's output).

---

## 8. Multi-Agent Prompt Coordination

### 8.1 Shared Vocabulary Injection

When multiple agents work on related tasks, inject shared vocabulary
definitions to ensure consistent naming:

```
## Shared Vocabulary (from plan coordination)
- "tier" always means ContextTier (Surgical/Focused/Full)
- "budget" always means token budget, not character budget
- "section" always means PromptSection (from prompt.rs)
```

This prevents the common failure mode where Agent A introduces a concept
that Agent B refers to by a different name.

**Integration point:** `crates/roko-compose/src/context_mesh.rs` --
`SharedContextEntry` can carry vocabulary definitions. Inject into Layer 3c
(active signals).

### 8.2 Dependency Chain Context

When a task depends on prior tasks, inject a structured summary of what
the prior tasks produced:

```
## Completed Dependencies
T-041: Added ContextTier enum to context_provider.rs (PASSED all gates)
T-042: Added is_local_model() function (PASSED all gates)
T-043: Added tier_constrained_budget() to budget.rs (FAILED clippy gate,
       retried with fix, PASSED)
```

This gives the model awareness of what is already done and what issues
were encountered.

**Integration point:** Already partially implemented via `PriorTaskOutput`
in `context_provider.rs`. Expand to include gate outcome summaries.

---

## 9. Prompt Versioning and A/B Testing

### 9.1 Prompt Experiment Framework

The `ExperimentStore` in `roko-learn` already supports A/B testing for
prompt variants. Extend to support systematic prompt engineering experiments:

```toml
# .roko/learn/experiments.json
{
  "prompt_experiments": {
    "reasoning_depth": {
      "variants": ["suppress", "brief", "deep"],
      "metric": "gate_pass_rate",
      "current_winner": "brief",
      "observations": 150
    },
    "anti_pattern_format": {
      "variants": ["warning_only", "warning_with_correction", "example_based"],
      "metric": "anti_pattern_avoidance_rate",
      "current_winner": "warning_with_correction",
      "observations": 80
    }
  }
}
```

**Integration point:** `crates/roko-learn/src/experiments.rs` -- extend
the existing experiment system with prompt-specific experiment types.

### 9.2 Section Effectiveness as A/B Test

Treat each section inclusion decision as an implicit A/B test:
- Treatment group: tasks where the section was included
- Control group: tasks where the section was excluded
  (naturally, due to budget pressure or random variation)
- Metric: gate pass rate

This is exactly what `SectionInfluence` in `budget_predictor.rs` already
computes. The gap is that the results are not fed back into allocation.

---

## 10. Ecological Prompt Design

### 10.1 Information Scent Theory

Apply information scent theory from HCI to prompt design: each section
should provide a "scent trail" that leads the model toward the correct
action. If a section does not contribute to the model finding the right
approach, it is noise.

Concrete application:
- Section headers should use task-relevant keywords (not generic "Context")
- Section content should be organized from most to least relevant
- Cross-references between sections should use consistent terminology

### 10.2 Cognitive Load Management

Models, like humans, have limited working memory. The prompt should
manage cognitive load:

- **Surgical tier:** Low cognitive load. One task, one file, one command.
  The model can focus entirely on execution.
- **Focused tier:** Medium cognitive load. Task + context + constraints.
  The model must integrate information from multiple sources but the
  scope is bounded.
- **Full tier:** High cognitive load. Complex task + rich context +
  multiple constraints + historical patterns. The model must reason
  about architecture and cross-cutting concerns.

The key insight: a model's effective capability depends not just on its
parameter count but on how well the prompt manages its cognitive load.
A well-structured Focused-tier prompt on Sonnet can outperform a
poorly-structured Full-tier prompt on Opus.

---

## Applicability Summary

| Technique | Tier | Effort | Impact | Already Built? |
|---|---|---|---|---|
| Output schema embedding | All | Small | High | Partial (verdict format) |
| Adaptive CoT depth | All | Small | High | No |
| Progressive disclosure headers | All | Small | Medium | Partial (layer ordering) |
| Anti-pattern corrections | All | Small | High | No |
| Reflective self-repair | Retry only | Small | High | No (gate feedback is raw) |
| Salience markers | All | Small | Medium | No |
| Instruction anchoring | Focused, Full | Small | Medium | No |
| Negative space | All | Small | Medium | No |
| Extractive summarization | Focused, Full | Medium | High | No |
| Symbol-level code context | All | Medium | High | Built (symbol_resolver.rs) |
| Diff-based retry context | Retry only | Medium | High | No |
| Shared vocabulary injection | Multi-agent | Medium | Medium | Partial (context_mesh.rs) |
| Confidence calibration | All | Small | Medium | No |
| Prompt A/B testing | All | Small | Medium | Built (experiments.rs) |
| Section gating (on-demand) | Focused, Full | Large | High | No |

---

## Sources

- `crates/roko-compose/src/system_prompt_builder.rs` -- layer structure, build methods
- `crates/roko-compose/src/prompt_assembly_service.rs` -- assembly pipeline
- `crates/roko-compose/src/templates/common.rs` -- format_verdict_instructions
- `crates/roko-compose/src/attention.rs` -- PositionAttentionModel, placement
- `crates/roko-compose/src/symbol_resolver.rs` -- symbol-level context extraction
- `crates/roko-compose/src/context_mesh.rs` -- SharedContextEntry
- `crates/roko-compose/src/gate_feedback.rs` -- gate feedback formatting
- `crates/roko-compose/src/compaction.rs` -- conversation compaction
- `crates/roko-compose/src/budget_predictor.rs` -- SectionInfluence
- `crates/roko-compose/src/context_provider.rs` -- ContextTier
- `crates/roko-learn/src/experiments.rs` -- ExperimentStore
