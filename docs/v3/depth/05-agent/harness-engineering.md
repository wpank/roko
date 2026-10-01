# 05-agent/harness-engineering -- Harness Engineering

> What the Meta-Harness paper contributes, the harness-design principles Roko
> follows (its own synthesis), HarnessX extensions, Harness-Bench evaluation,
> Belief Divergence diagnostics, and how each maps to Roko's implementation.

**Parent:** [05-AGENT](../../05-AGENT.md)

**Source:** Lee, Y. et al. (2026). "Meta-Harness: End-to-End Optimization of
Model Harnesses." arXiv:2603.28052.

---

## 1. What the Meta-Harness Paper Shows

The **harness** -- the code that decides what to store, retrieve and show to the
model -- "often matters as much as the model itself" (Lee et al. 2026, §1).
Harness engineering is still mostly manual, so the paper automates it (§3):
Meta-Harness is an outer-loop search over harness code. A coding-agent proposer
(Claude Code, in the paper) reads a filesystem holding every earlier candidate's
source code, scores and execution traces, and proposes a new harness; the loop
evaluates it, logs the result and repeats.

The key paper:

> Lee, Y. et al. (2026). "Meta-Harness: End-to-End Optimization of Model Harnesses."
> arXiv:2603.28052.

### Results (§4)

| Task | Discovered harness vs. baseline | Section |
|------|---------------------------------|---------|
| Online text classification | +7.7 points over Agentic Context Engineering (ACE), with 4x fewer context tokens | §4.1 |
| Retrieval-augmented math, 200 IMO-level problems | +4.7 points on average across five held-out models | §4.2 |
| Agentic coding, TerminalBench-2 | Ranks #1 among Claude Haiku 4.5 agents | §4.3 |

### The 6x figure

The paper's introduction opens with a "6x performance gap" from changing the
harness around a fixed model on one benchmark. It is a result the paper cites
(SWE-bench Mobile), not one it measures.

The practical takeaway: the harness can matter as much as the model, and in the
paper's three domains, searching over harness code with full access to earlier
traces found better harnesses than hand-designed ones (§4, §5).

---

## 2. Roko's Harness-Design Principles

These six principles are Roko's own synthesis of harness-engineering practice.
The Meta-Harness paper does not state them: its Appendix D lists procedural tips
for running its search instead, such as writing a good skill for the proposer,
logging every run in a navigable form, and validating candidates cheaply before
evaluating them.

### Principle 1: Design Tools for the Model, Not for Humans

**Thesis:** LLMs use tools differently than humans. Tool interfaces should be
optimized for how models reason -- structured JSON schemas, unambiguous parameter
names, clear error messages.

**Roko implementation:**
- `ToolDef` carries JSON schema validated by the ToolDispatcher (step 1)
- The `Translator` layer ensures each model gets tools in its preferred wire
  format (`openai_json`, `anthropic_blocks`)
- `RenderedTools` allows different representations per backend
- Tool descriptions are written for model comprehension, not human convenience

### Principle 2: Provide the Right Context, Not More Context

**Thesis:** More context does not always help. Models perform better with
focused, relevant context than with entire files dumped into the prompt.

**Roko implementation:**
- 9-layer `SystemPromptBuilder` provides targeted per-layer context
- `prune` submodule drops oldest tool results, preserves system + first user +
  recent tail
- `AttentionBidder` variants (Neuro/Task/Research) compete for context slots
- Context is proportional to task complexity, not maximized

### Principle 3: Validate Before Executing

**Thesis:** Check tool call arguments before running them. Invalid arguments
waste tokens and risk side effects.

**Roko implementation:**
- 7-step ToolDispatcher validates schema/permissions/safety before execution
- JSON schema validation at step 1 catches structural errors
- Permission check at step 2 catches authorization failures
- SafetyLayer at step 3 catches dangerous operations
- All rejections produce clear error messages for the model to self-correct

### Principle 4: Compress History Intelligently

**Thesis:** Long conversations degrade model performance. Compress old context
while preserving recent and important messages.

**Roko implementation:**
- `prune` submodule estimates tokens from byte length
- Preserves: system prompt (always), first user message, recent tail, error
  results
- Drops: oldest tool results in the middle
- Checkpoint captures full state for later resume despite pruning

### Principle 5: Graduate Autonomy Based on Confidence

**Thesis:** Do not give agents full autonomy from the start. Start constrained,
escalate as confidence grows.

**Roko implementation:**
- Role-based permission system (28 roles, 5 permission categories)
- Read-only roles (reviewers, detectors) cannot modify
- Read-write roles (implementers) get full file operations within worktree
- SafetyLayer provides a floor that even high-autonomy roles cannot breach
- CascadeRouter escalates model tier only when confidence is low
- `AutonomyLevel` enum (Observe -> Suggest -> ActReview -> Guardrails -> Full)

### Principle 6: Close the Feedback Loop

**Thesis:** Agent performance improves when results feed back into future
decisions. Record what worked, what failed, and why.

**Roko implementation:**
- EpisodeLogger records every turn + gate result to `.roko/episodes.jsonl`
- Efficiency events per-turn to `.roko/learn/efficiency.jsonl`
- CascadeRouter persistence to `.roko/learn/cascade-router.json`
- Adaptive gate thresholds (EMA per rung) to `.roko/learn/gate-thresholds.json`
- Gate failure replan: `build_gate_failure_plan_revision` feeds gate results
  back into agent dispatch
- Playbook store: top when/then matches queried at dispatch time and injected
  into system prompts

---

## 3. HarnessX (Lee et al., 2026)

The extended framework (arXiv:2606.14249) builds on Meta-Harness with:

- **Harness taxonomy:** Classifies harness components into structural (tools,
  context) and behavioral (retry, escalation, feedback) categories
- **Composition rules:** How harness components interact and compose. Some
  combinations are synergistic (validation + feedback), others antagonistic
  (aggressive pruning + long-horizon tasks)
- **Transfer analysis:** Harness improvements that transfer across models vs.
  those that are model-specific

Roko's architecture aligns with HarnessX's structural/behavioral separation:
the crate layers (roko-core, roko-agent, roko-gate, roko-compose, roko-learn)
map to harness components, and the runner event loop provides the behavioral
orchestration.

---

## 4. Harness-Bench (arXiv:2605.27922)

Harness-Bench provides standardized evaluation of harness quality across agent
systems. Key dimensions:

| Dimension | What it measures | Roko coverage |
|-----------|-----------------|---------------|
| Tool fidelity | Do tools work correctly? | roko-std tests (35 tools) |
| Context efficiency | Tokens per successful output | Efficiency events |
| Safety compliance | Does the harness prevent harm? | SafetyLayer + E34 |
| Recovery capability | Can the harness recover from errors? | Gate replan + fallback |
| Feedback integration | Does performance improve over time? | CascadeRouter + thresholds |

Harness-Bench evaluation can be triggered via `roko bench swe` which runs
SWE-bench style evaluations and writes learning telemetry.

---

## 5. Belief Divergence (arXiv:2607.04528)

Belief Divergence quantifies the gap between an agent's internal model and its
expressed behavior. When an LLM "knows" the right answer but the harness causes
it to produce the wrong output, this is a harness-model misalignment.

Sources of divergence:

| Source | Example | Roko mitigation |
|--------|---------|-----------------|
| Tool format mismatch | Model prefers OpenAI format, given Anthropic blocks | Translator per-model wire format |
| Context overload | Too much context degrades reasoning | Prune + targeted 9-layer prompts |
| Permission denial confusion | Model retries denied tools endlessly | Clear error messages with alternatives |
| Instruction conflict | System prompt contradicts task prompt | Layer priority in SystemPromptBuilder |

The diagnostic is useful for debugging cases where the model produces poor
output despite being capable: the issue is likely in the harness, not the model.

---

## 6. Mechanism-Level Review (arXiv:2607.23942)

The most comprehensive survey of agent architectures catalogues the specific
mechanisms that distinguish high-performing harnesses:

| Mechanism | Description | Roko component |
|-----------|-------------|----------------|
| Structured observation | Parsing env feedback into structured forms | Translator layer |
| Action grounding | Binding abstract intents to concrete tool calls | ToolDef schemas |
| Memory management | Selecting what to remember and forget | Prune + knowledge tiers |
| Plan repair | Recovering from failed sub-plans | Gate failure replan |
| Self-monitoring | Detecting loops, stalls, regression | roko-conductor watchers |
| Model selection | Choosing the right model per-task | CascadeRouter + LinUCB |

---

## 7. Where Roko Follows These Principles Well

1. **Tool validation pipeline** -- The 7-step ToolDispatcher is exactly the
   "validate before executing" principle, with audit signals for observability.

2. **Format-aware translation** -- The Translator layer ensures each model gets
   tools in its preferred wire format.

3. **9-layer prompt construction** -- The SystemPromptBuilder provides targeted,
   role-appropriate context rather than dumping everything.

4. **Feedback loop wiring** -- Episode logging, efficiency tracking, playbook
   enrichment, and adaptive thresholds form a complete feedback loop.

5. **Graduated autonomy** -- 28 roles with 5 permission categories plus
   AutonomyLevel and SafetyLayer floor.

---

## 8. Where Gaps Remain

1. **CLI providers bypass the dispatcher** -- ClaudeCli, CodexCli, and GeminiCli
   drive their own internal tool loops. Roko's ToolDispatcher/SafetyLayer are
   not applied (though these CLIs have their own safety mechanisms).

2. **Context pruning is byte-based** -- The current strategy uses byte-length
   estimation rather than semantic importance. A smarter approach would preserve
   messages referenced by recent tool calls.

3. **No speculative execution** -- The PASTE pattern (Microsoft Research,
   arXiv:2603.18897) speculatively executes predicted tools in parallel with
   LLM reasoning, reducing latency by 48.5%. Roko does not implement this.

4. **No tool transition graph** -- AutoTool (arXiv:2511.14650) builds tool
   transition graphs from historical data. Roko's episode data could support
   this but the graph is not yet built.

---

## 9. Benchmarks in Context

The Meta-Harness paper evaluates online text classification, retrieval-augmented
math and TerminalBench-2 (§4). SWE-bench appears in it only through the SWE-bench
Mobile result behind the 6x figure in its introduction. SWE-bench itself
(Jimenez et al., 2024) is the usual agentic-coding benchmark, and
`roko bench swe` runs SWE-bench-style evaluations.

Roko treats the harness as the lever: the crate layers provide the harness, and
the model is a pluggable component chosen at runtime, so a harness improvement
applies to every model.

---

## 10. Citations

1. Lee, Y. et al. (2026). "Meta-Harness: End-to-End Optimization of Model
   Harnesses." arXiv:2603.28052. -- Automated harness search (§3) and its
   results (§4).
2. Chen et al. (2026). "HarnessX." arXiv:2606.14249. -- Extended
   framework, composition rules, transfer analysis.
3. Yao, Y. et al. (2026). "Harness-Bench: Measuring Harness Effects across
   Models in Realistic Agent Workflows." arXiv:2605.27922. -- Harness evaluation
   across models.
4. Yi, H. et al. (2026). "Measuring Harness-Induced Belief Divergence in
   Multi-Step LLM Agents." arXiv:2607.04528. -- Harness-model misalignment
   diagnostic.
5. Fan, H. et al. (2026). "From Cognitive Architectures to Language Agents: A
   Mechanism-Level Review of Lineage, Convergence, and Migration Gaps."
   arXiv:2607.23942. -- Survey of agent mechanisms.
6. Jimenez, C. E. et al. (2024). "SWE-bench: Can Language Models Resolve
   Real-World GitHub Issues?" -- Benchmark showing harness variance.
7. arXiv:2603.18897. "Parallelizing Tool Execution and LLM Generation for Low-Latency Agent Serving." --
   48.5% latency reduction via speculative execution.
8. arXiv:2511.14650. "AutoTool: Efficient Tool Selection for Large Language Model Agents."
   AAAI 2026. -- Graph-based tool prediction.
9. `crates/roko-agent/src/dispatcher/mod.rs` -- 7-step pipeline.
10. `crates/roko-compose/src/system_prompt_builder.rs` -- 9-layer prompts.
11. `crates/roko-agent/src/tool_loop/prune.rs` -- Context pruning.
12. `crates/roko-learn/` -- Feedback loop components.
