# 05-agent/harness-engineering -- Harness Engineering

> What the Meta-Harness paper contributes, the harness-design principles Roko
> follows (its own synthesis), and what HarnessX, Harness-Bench, the Belief
> Divergence paper and a mechanism-level review contribute, each as its paper
> describes it, with Roko's own mapping marked as such.

<!-- content-checked: §3–§6 against HarnessX, Harness-Bench, Belief Divergence and the mechanism-level review (full texts on arXiv), 2026-10-01 -->

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

## 3. HarnessX (Chen et al., 2026)

HarnessX (arXiv:2606.14249) is a "foundry" for harnesses that can be composed,
adapted and evolved. It is independent work; it does not cite Meta-Harness.

- **Composition (§3).** The harness is a typed, first-class object: processors
  attached to lifecycle hooks, assembled through a substitution algebra. A
  nine-dimensional taxonomy spans its behavior (§3.3): model selection, context
  assembly, memory management, tool ecosystem, execution environment,
  evaluation and reward, control and safety, observability, and a training
  bridge.
- **Adaptation (§4).** AEGIS, a four-stage meta-agent pipeline (Digester,
  Planner, Evolver, Critic), proposes typed harness edits from execution traces
  and admits each through a deterministic acceptance gate.
- **Co-evolution (§5).** The harness and the model improve together; the model
  trains through cross-harness GRPO.
- **Results (§1, §6).** On GAIA, ALFWorld, WebShop, τ-Bench and SWE-bench
  Verified with three task-agent families, harness evolution gains +14.5%
  absolute on average over 15 model–benchmark configurations, and the weakest
  agent gains most (+44.0% on ALFWorld for Qwen3.5-9B).

Roko's own mapping onto the nine dimensions (not the paper's): model selection
is the CascadeRouter and the model ladder; context assembly the
SystemPromptBuilder; memory the pruning and the knowledge store; the tool
ecosystem `ToolDef` and the ToolDispatcher; the execution environment worktrees
and sandbox levels; evaluation the gates; control and safety the SafetyLayer;
observability the episode and efficiency records. Roko has no training bridge.

---

## 4. Harness-Bench (Yao et al., 2026)

Harness-Bench (arXiv:2605.27922) is a diagnostic benchmark that fixes the task,
sandbox, budget, timeout and evaluator and varies only the harness around the
model (§3.1). It has 106 sandboxed, offline tasks in eight workflow categories
(§3.2). A run is scored on completion, a binary security and permission gate,
and LLM-rubric process scores; the overall score multiplies them (§3.4). Across
six configurable harnesses and eight model backends (5,194 trajectories), the
best and worst harnesses differ by 23.8 points on the same tasks and models
(§4).

| Score (§3.4) | What it measures | Closest Roko mechanism (Roko's mapping) |
|--------------|------------------|-----------------------------------------|
| Completion | Task-specific output quality, by validator or rubric | Gate rungs and authored verify steps |
| Security | A permission or security violation zeroes the run | SafetyLayer and the ToolDispatcher's permission check |
| Robustness | Whether the agent handles tool or environment failures | Gate-failure replan and provider fallback |
| Tool use | Whether tools are selected and applied appropriately | `ToolDef` schemas and the Translator |
| Consistency | Whether actions, state and outputs stay consistent with the workspace and the user's constraints | No dedicated mechanism |

Roko has not been run on Harness-Bench; `roko bench swe` runs SWE-bench-style
evaluations, a different benchmark.

---

## 5. Belief Divergence (Yi & Song, 2026)

The paper (arXiv:2607.04528) starts from the observation that a harness can
change what an agent knows about a task without changing the task (§1): what it
observes, which actions it may take, which failures are repaired before it sees
them, and which states are verified. To measure this it holds the task,
environment and model fixed, varies only the harness, and elicits a multi-step
belief rollout from the model: predicted progress, constraints, risk state,
recoverability, uncertainty, likely failure mode, success probability and next
action (§1, §4). The divergence between rollouts splits into an arrival readout
(immediate interface mismatch) and a growth readout (drift over the rollout
horizon). The controlled study (HIBench-Code-v0, §5) compares a raw reference
harness with five mediated ones:

| Harness in the paper (§5) | What it changes | Closest Roko mechanism (Roko's mapping) |
|---------------------------|-----------------|-----------------------------------------|
| Structured parsing | How observations are presented | Translator and tool-result formatting |
| Risk gating | Blocked actions return a policy-violation signal | SafetyLayer and ToolDispatcher denials |
| Repair-heavy execution | Failures are repaired before the agent sees them | Auto-fix (`cargo fix`, clippy fix) before an agent retry |
| Verification-selective execution | Which states are verified | Gate rungs chosen per task |
| Cost-aware execution | Execution under a cost budget | Budget ceilings and context pruning |

The diagnostic exposes harness effects that final success rates hide (§1). The
paper's BIWM protocol (§8) is a no-training procedure that canonicalises
beliefs, keeps blocked and repaired branches, records verification masks, runs
risky branches in shadow and aligns beliefs across harness views.

---

## 6. Mechanism-Level Review (Fan & Lan, 2026)

The review (arXiv:2607.23942) connects ten historical cognitive architectures
(among them ACT-R, Soar, CLARION, LIDA, Hearsay-II and BDI), eight
language-agent runtime families and forty-two modern systems. It reconstructs
each mechanism through state, control, transition, persistence, failure,
learning and resource governance, and codes how deeply each has migrated into
modern agents (D0–D4) separately from the strength of the evidence (E1–E4)
(§II–§III). Modern agents already cover much of adaptive memory, failure
recovery, team selection, workflow search, skill induction, resource scheduling
and uncertainty-conditioned action. The remaining gaps are couplings between
mechanisms: five residual control bundles (§VI) pair activation with latency
and action utility; a typed impasse with isolated substates and compiled
resolutions; bounded content competition with broadcast and admission
learning; a persistent intention with reconsideration and live method
switching; and uncertainty with resource allocation, interruption and
stopping. A sixth candidate is closed because GraSP already implements it.
Which of these couplings Roko implements has not been assessed.

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
   LLM reasoning, cutting average task completion time by 48.5% (v1 abstract; 43.5% in the current version). Roko does not implement this.

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
2. Chen, T. et al. (2026). "HarnessX: A Composable, Adaptive, and Evolvable
   Agent Harness Foundry." arXiv:2606.14249. -- Typed harness composition
   (§3), trace-driven evolution (§4), harness-model co-evolution (§5).
3. Yao, Y. et al. (2026). "Harness-Bench: Measuring Harness Effects across
   Models in Realistic Agent Workflows." arXiv:2605.27922. -- Harness effects
   across models (§3–§4).
4. Yi, H. & Song, X. (2026). "Measuring Harness-Induced Belief Divergence in
   Multi-Step LLM Agents." arXiv:2607.04528. -- How a harness changes an agent's
   beliefs (§1, §4).
5. Fan, H. & Lan, Z. (2026). "From Cognitive Architectures to Language Agents: A
   Mechanism-Level Review of Lineage, Convergence, and Migration Gaps."
   arXiv:2607.23942. -- Survey of agent mechanisms.
6. Jimenez, C. E. et al. (2024). "SWE-bench: Can Language Models Resolve Real-World GitHub Issues?"
7. arXiv:2603.18897. "Parallelizing Tool Execution and LLM Generation for Low-Latency Agent Serving." --
   48.5% shorter task completion time via speculative execution (v1; 43.5% in the current version).
8. arXiv:2511.14650. "AutoTool: Efficient Tool Selection for Large Language Model Agents." AAAI 2026.
9. `crates/roko-agent/src/dispatcher/mod.rs` -- 7-step pipeline.
10. `crates/roko-compose/src/system_prompt_builder.rs` -- 9-layer prompts.
11. `crates/roko-agent/src/tool_loop/prune.rs` -- Context pruning.
12. `crates/roko-learn/` -- Feedback loop components.
