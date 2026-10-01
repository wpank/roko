# CoALA 9-Step Cognitive Pipeline

> v3 depth file for chapter 29 (Heartbeat and Cognitive Loop).
> Source: v1/16-heartbeat/00-coala-9-step-pipeline.md.
> Parent: `docs/v3/29-HEARTBEAT.md` SS1--3.

---

## 1. Why Decision Cycles, Not Conversation Turns

Most agent frameworks model cognition as a conversation: user says something, agent
thinks, agent responds. This model descends from chatbot heritage -- LLMs were first
deployed as conversational interfaces, and agent frameworks inherited that frame.

For Roko agents, this model is wrong on its face. **80% of heartbeat ticks have no
human input.** The agent fires its heartbeat, observes its environment (codebase state,
market conditions, research corpus), evaluates whether anything interesting happened,
and either acts or moves on. There is no "user message." There is no "response." There
is a continuous loop of observe-decide-act-learn running autonomously.

The distinction matters architecturally:

| Aspect | Conversation Turn | Decision Cycle |
|--------|------------------|----------------|
| Trigger | User message | Timer (heartbeat interval) |
| Input | Natural language text | Structured observation (build status, market data, research signal) |
| Output | Natural language response | Typed `DecisionCycleRecord` (structured, self-contained) |
| Storage | Append to growing session | Self-contained record per tick |
| Replay | Parse message history | Structured fields, zero parsing needed |
| Credit assignment | Parse LLM output text | Read outcome + context fields directly |
| Cost | Every turn costs money | ~80% of ticks cost $0.00 (T0 suppression) |

---

## 2. CoALA: The Organizing Framework

The organizing framework for the heartbeat is CoALA (Cognitive Architectures for
Language Agents), proposed by Sumers, Yao, Narasimhan, and Griffiths (2023,
arXiv:2309.02427). CoALA draws on decades of cognitive architecture research to
formalize what a language agent IS and how its decision cycle should be structured:

> "The agent's decision procedure executes a decision cycle in a loop with the
> external environment. During each cycle, the agent uses retrieval and reasoning
> to plan by proposing and evaluating candidate learning or grounding actions. The
> best action is then selected and executed. An observation may be made, and the
> cycle begins again."

This maps precisely to Roko's heartbeat. Each tick IS a decision cycle: observe,
retrieve from memory, reason about what was observed, decide on an action (or decide
to do nothing), execute, observe the outcome, learn.

Roko adopted the CoALA framework as an early organizing taxonomy because it provides
a rigorous, research-grounded structure for agent cognition that maps cleanly onto
the Synapse Architecture's 12 traits. The canonical seven-step universal loop
(SENSE / ASSESS / COMPOSE / ACT / VERIFY / PERSIST + BROADCAST / REACT) now
supersedes the original 9-step CoALA narration, but the theoretical lineage remains
foundational.

---

## 3. Academic Predecessors

CoALA does not arise in a vacuum. It synthesizes insights from four major cognitive
architecture traditions, each contributing specific mechanisms that appear in Roko's
implementation.

### 3.1 Soar (Laird, Newell, Rosenbloom 1987; Laird 2012)

Soar introduced the concept of a **production system** with a working memory
(analogous to Roko's context window), long-term memory (analogous to the Neuro
knowledge store), and a decision cycle that selects operators based on preferences.
Key contributions to Roko:

**Impasse-driven subgoaling.** When Soar cannot resolve a decision (impasse), it
creates a subgoal to resolve the impasse. Roko's tier escalation (T0 -> T1 -> T2)
implements this principle -- when deterministic probes cannot resolve the situation
(impasse at T0), the agent escalates to analytical reasoning (T1) or deep
deliberation (T2).

**Chunking.** Soar learns new production rules from subgoal resolution. Roko's
playbook rule extraction and skill library serve the same function -- successful
reasoning patterns are compiled into reusable artifacts that future T0 ticks can
execute without LLM involvement.

**Universal subgoaling.** Newell's "Unified Theories of Cognition" (1990) argued
that a single architecture should handle all cognitive tasks. Roko's universal loop
embodies this -- one loop, parameterized by domain, handles all agent types.

### 3.2 ACT-R (Anderson 1993, 2007)

ACT-R (Adaptive Control of Thought -- Rational) provides a modular cognitive
architecture with distinct declarative and procedural memory systems. Key
contributions to Roko:

**Activation-based retrieval.** ACT-R retrieves memories based on activation levels
that decay with time and increase with use. Roko's Ebbinghaus decay curves in Neuro
implement precisely this mechanism -- knowledge that is used successfully gains
activation (tier promotion), while unused knowledge decays.

**Rational analysis.** Anderson's rational analysis principle states that cognitive
mechanisms should be understood as optimal adaptations to the statistical structure
of the environment. Roko's active inference framework (Friston's Free Energy
Principle) formalizes this same insight -- the agent's behavior should minimize
surprise given its model of the environment.

**Declarative/procedural distinction.** ACT-R separates what you know (declarative)
from how you act (procedural). Roko separates Neuro (declarative knowledge: Insights,
Heuristics, Warnings) from playbook rules (procedural knowledge: reusable action
sequences).

### 3.3 CLARION (Sun et al. 2001, 2005)

CLARION (Connectionist Learning with Adaptive Rule Induction ON-line) provides a
dual-process architecture with explicit (top-down) and implicit (bottom-up)
processing. This is the direct ancestor of Roko's dual-process T0/T1/T2 system:

**Dual-process cognition.** CLARION's bottom-up implicit processing maps to T0
(fast, heuristic, no LLM). Its top-down explicit processing maps to T2 (slow,
deliberate, full LLM). T1 is the intermediate tier that CLARION does not explicitly
model.

**Bottom-up learning.** CLARION extracts explicit rules from implicit knowledge
through a process called "rule extraction." Roko's dream consolidation and playbook
compilation perform the same function -- implicit patterns learned through experience
are extracted into explicit heuristics.

**Motivation subsystem.** CLARION includes a motivational subsystem that modulates
cognitive processing. Roko's Daimon (PAD affect engine) serves this role -- emotional
state modulates tier routing, context bidding, and risk tolerance.

### 3.4 Global Workspace Theory (Baars 1988; Baddeley 2000)

Baars' Global Workspace Theory proposes that consciousness arises from a shared
workspace where specialized processors compete for access. Baddeley's Working Memory
Model refines this with a central executive, episodic buffer, visuospatial sketchpad,
and phonological loop. Roko's mapping:

| GWT Component | Roko Implementation |
|---|---|
| Central executive | Context Governor (allocates attention tokens across subsystems) |
| Episodic buffer | The assembled context window (integrates information from all sources) |
| Visuospatial sketchpad | Current observations (environment state, build results, market data) |
| Phonological loop | Playbook heuristics (rehearsed procedural knowledge -- the agent's "inner speech") |

The Cognitive Workspace paper (2025, arXiv:2508.13171) validated this approach
computationally, demonstrating that active memory management with deliberate
information curation achieves 58.6% memory reuse rate compared to 0% for traditional
RAG, with 17-18% net efficiency gain.

---

## 4. The Canonical Seven-Step Loop

Each heartbeat tick used to be described as a 9-step pipeline. That framing is
retained here only as historical scaffolding. The canonical loop now has seven steps,
with PERSIST and BROADCAST co-equal at step 6 and cross-cuts injected into operators
rather than sequenced as step 9.

```
1. SENSE          -- Substrate.query | Bus.subscribe | external I/O
2. ASSESS         -- Scorer.score + Router.select
3. COMPOSE        -- Assemble the prompt/context under budget
4. ACT            -- LLM, tool, or chain action
5. VERIFY         -- Gate pipeline + stream-gates
6. PERSIST        -- Store Signals in Substrate
   BROADCAST      -- Publish Pulses on the Bus
7. REACT          -- Policy.decide, emit follow-on Pulses + Signals
```

The older 9-step list was a CoALA-adjacent way to explain the same runtime, but it
hid the Bus, split scoring from routing too aggressively, and treated cross-cuts as
sequential. The seven-step version matches the canonical architecture and keeps the
transport fabric visible.

### 4.1 Step-by-step

**Step 1: SENSE.** Three inputs: durable retrieval through `Substrate.query()` for
plans, episodes, heuristics, and prior verdicts; live delivery through
`Bus.subscribe()` for tick Pulses, turn output, approvals, and cancellation; external
I/O not yet normalized into either fabric. The 16 T0 probes are part of this sensing
surface. Synapse traits: `Substrate.query()`, `Bus.subscribe()`. Layer: L0 Runtime.

**Step 2: ASSESS.** Score candidate Signals and Pulses, compute surprise/confidence,
and route toward T0/T1/T2 in one joint decision. Multi-factor scoring:

```
score = w_recency * recency(Ebbinghaus_decay)
      + w_importance * quality(confidence * validation_ratio)
      + w_relevance * similarity(query, entry)
      + w_emotional * PAD_cosine(current_mood, entry_affect)
```

The fourth factor implements Bower's (1981) mood-congruent memory. Every 100 ticks,
mandatory 15% contrarian retrieval forces mood-opposite entries to prevent rumination.
Synapse traits: `Scorer.score()`, `Router.select()`. Layer: L1/L2.

**Step 3: COMPOSE.** Assemble the prompt or action bundle under budget. Prediction
error drives the context depth. The VCG attention auction allocates tokens across
competing subsystems at T2. Synapse trait: `Composer.compose()`. Layer: L2 Scaffold.

**Step 4: ACT.** Execute the composed work -- LLM call, tool call, chain transaction.
Dual-process thresholding determines whether the tick stays at T0, escalates to T1,
or pays for T2. During ACT, live Pulses (`agent.msg.chunk`, `tool.call.started`,
`agent.turn.completed`) are emitted. Synapse trait: `Agent.execute()`. Layer: L1.

**Step 5: VERIFY.** Canonical verification boundary -- pre-flight checks, safety
checks, stream-gates, and ground-truth verification as one coherent phase. Domain-
specific: coding agents run `CompileGate -> TestGate -> ClippyGate`; chain agents
run `TxSimGate -> WalletGate -> VerifyChainGate`. Synapse trait: `Gate.verify()`.
Layer: L3.

**Step 6: PERSIST + BROADCAST.** Store the finished Signal in the Substrate with
lineage and provenance. Publish the matching Pulse on the Bus for live subscribers.
Both outputs are explicit, not side effects. Synapse traits: `Substrate.put()`,
`Bus.publish()`. Layer: L0.

**Step 7: REACT.** Build the `DecisionCycleRecord`. Fire learning hooks: episode
logging, Daimon PAD update, Neuro tier promotion/demotion, CascadeRouter feedback,
prediction calibration. Synapse trait: `Policy.decide()`. Layer: L3-L4.

---

## 5. CoALA-to-Synapse Mapping

| Historical CoALA framing | Canonical loop | Synapse trait(s) | Layer |
|---|---|---|---|
| OBSERVE + RETRIEVE | **SENSE** | `Substrate.query()`, `Bus.subscribe()` | L0 |
| ANALYZE + GATE | **ASSESS** | `Scorer.score()`, `Router.select()` | L1/L2 |
| _(implicit)_ | **COMPOSE** | `Composer.compose()` | L2 |
| EXECUTE | **ACT** | `Agent.execute()` | L1 |
| SIMULATE + VALIDATE + VERIFY | **VERIFY** | `Gate.verify()`, domain gates | L3 |
| _(folded into REFLECT)_ | **PERSIST** | `Substrate.put()` | L0 |
| _(under-described)_ | **BROADCAST** | `Bus.publish()` | L0 |
| REFLECT | **REACT** | `Policy.decide()` | L3-L4 |

Key differences: COMPOSE is explicit (context engineering is a first-class step, per
Meta-Harness, Lee et al. 2026, arXiv:2603.28052). PERSIST and BROADCAST are both
explicit (content-addressed Signal storage and topic-addressed Pulse delivery are
co-equal). Domain-specific extensions (SIMULATE, VALIDATE) are injected into VERIFY,
not added as universal steps.

---

## 6. The OODA Correspondence

Boyd's Observe-Orient-Decide-Act (OODA) loop maps directly onto the canonical
heartbeat, with one critical addition: REACT closes the learning loop that OODA
leaves open.

| OODA Phase | Pipeline Steps | What Happens |
|---|---|---|
| **Observe** | SENSE | Deterministic probes, live Pulses, external I/O |
| **Orient** | ASSESS + COMPOSE | Scoring, routing, context assembly |
| **Decide** | ACT | Commit to action or suppress |
| **Act** | VERIFY + PERSIST/BROADCAST | Effects checked, then written and delivered |
| _(missing in OODA)_ | REACT | DecisionCycleRecord + learning hooks close the loop |

The REACT step is what makes Roko agents self-improving rather than merely reactive.
Every tick that resolves -- whether the agent acted or suppressed -- produces data
that improves future ticks. This is the cybernetic feedback loop that Boyd's OODA
framework lacks.

---

## 7. Why CoALA Over Alternatives

### 7.1 ReAct (Yao et al. 2022)

ReAct interleaves reasoning and acting but has no explicit memory retrieval step,
no gating mechanism, and no separation of cognitive tiers. It is a prompting strategy,
not a cognitive architecture.

### 7.2 Reflexion (Shinn et al. 2023)

Reflexion adds verbal self-reflection to trial-and-error. It provides the REFLECT
step but lacks the structured OBSERVE-RETRIEVE-ANALYZE-GATE pipeline. It also uses
self-assessment (LLM judges its own output) rather than external verification
(compiler, tests, blockchain).

### 7.3 AutoGPT / BabyAGI

Purely loop-based architectures without cognitive tiering, memory retrieval, or
verification. Every iteration invokes the LLM, making them expensive and lacking
the T0/T1/T2 cost optimization.

### 7.4 Why CoALA Wins

CoALA provides the most complete mapping from cognitive science to agent
implementation:

1. **Explicit decision cycle** (not conversation turns) -- matches Roko's autonomous
   tick model.
2. **Memory retrieval as a first-class step** -- matches Roko's Neuro integration.
3. **Learning from action outcomes** -- matches Roko's Gate-based feedback loops.
4. **Grounding in established cognitive architectures** (Soar, ACT-R) -- provides
   theoretical rigor rather than ad hoc design.
5. **Composable with active inference** -- CoALA's prediction error mechanism maps
   directly to Friston's free energy minimization, enabling the T0/T1/T2 gating
   system that provides ~80% cost reduction.

---

## 8. Cost Model Summary

The canonical seven-step loop preserves the T0/T1/T2 gating economics:

| Tier | Model | Cost/Call | Frequency | Trigger |
|------|-------|-----------|-----------|---------|
| **T0** | None | $0.00 | ~80% of ticks | Prediction error < 0.2 |
| **T1** | Haiku-class | $0.001-0.003 | ~15% of ticks | Prediction error in [0.2, 0.6) |
| **T2** | Sonnet/Opus-class | $0.01-0.25 | ~5% of ticks | Prediction error >= 0.6 |

Without gating (every tick at T2): daily cost $100-500+. With T0 suppression at
~80%: daily inference cost drops to ~$2-50. FrugalGPT (Chen et al. 2023,
arXiv:2305.05176; published 2024 in TMLR) demonstrated that cascade architectures
can achieve up to 98% cost reduction while matching top-model quality.

---

## 9. Pipeline Instrumentation: CognitiveCycleSpan

Every heartbeat execution is instrumented for observability, cost tracking, and
dual-process validation. The `CognitiveCycleSpan` bridges per-turn
`AgentEfficiencyEvent` and per-tool `ToolTrace`, following OTel gen_ai semantic
conventions:

```
invoke_agent {roko-agent}                          [CLIENT span]
|
+-- cognitive_cycle {cycle_index=0}                 [INTERNAL span]
|   |   roko.cycle.frequency = "gamma"
|   |   roko.cycle.tier = "T2"
|   |
|   +-- sense                                       [INTERNAL]
|   +-- assess                                      [INTERNAL]
|   +-- compose                                     [INTERNAL]
|   +-- act                                         [INTERNAL]
|   +-- chat {claude-opus-4-6}                      [CLIENT]
|   |   +-- execute_tool {Bash}                     [INTERNAL]
|   +-- verify                                      [INTERNAL]
|   +-- persist                                     [INTERNAL]
|   +-- broadcast                                   [INTERNAL]
|   +-- react                                       [INTERNAL]
|
+-- cognitive_cycle {cycle_index=1, tier="T0"}      [INTERNAL span]
    +-- sense + assess (only)                       [steps 4-7 skipped]
```

---

## 10. References

### Primary

- **Sumers, Yao, Narasimhan & Griffiths 2023** -- "Cognitive Architectures for
  Language Agents" (arXiv:2309.02427). The CoALA framework.

### Cognitive architecture lineage

- **Laird, Newell & Rosenbloom 1987** -- "SOAR: An Architecture for General
  Intelligence" (Artificial Intelligence 33(1)).
- **Laird 2012** -- "The Soar Cognitive Architecture" (MIT Press).
- **Anderson 1993, 2007** -- "Rules of the Mind" (Erlbaum); "How Can the Human
  Mind Occur in the Physical Universe?" (Oxford). ACT-R.
- **Sun, Merrill & Peterson 2001** -- "From Implicit Skills to Explicit Knowledge"
  (Cognitive Science 25(2)). CLARION.
- **Sun 2005** -- "The CLARION Cognitive Architecture" (Cambridge Handbook of
  Computational Psychology).
- **Newell 1990** -- "Unified Theories of Cognition" (Harvard University Press).
- **Baars 1988** -- "A Cognitive Theory of Consciousness" (Cambridge University
  Press). Global Workspace Theory.
- **Baddeley 2000** -- "The Episodic Buffer: A New Component of Working Memory?"
  (Trends in Cognitive Sciences 4(11)).

### Agent efficiency and cost

- **Chen et al. 2023** -- FrugalGPT (arXiv:2305.05176; published 2024 TMLR).
- **Cognitive Workspace (An 2025)** -- arXiv:2508.13171.
- **Lee et al. 2026** -- Meta-Harness (arXiv:2603.28052).
- **Bower 1981** -- "Mood and Memory" (American Psychologist 36(2)).

### Other

- **Friston 2010** -- "The Free-Energy Principle" (Nature Reviews Neuroscience
  11(2)).
- **Clark 2013** -- "Whatever Next?" (Behavioral and Brain Sciences 36(3)).
- **Boyd 1986** -- "Patterns of Conflict" (unpublished briefing). OODA Loop.

---

## 11. Implementation Sources

| Surface | Authority | Status |
|---------|-----------|--------|
| InferenceTier + TierRouter | `crates/roko-primitives/src/tier.rs` | Shipped |
| CascadeRouter | `crates/roko-learn/src/cascade_router.rs` | Shipped |
| PredictiveScorer | `crates/roko-core/src/score.rs` | Shipped |
| Adaptive gate thresholds | `crates/roko-learn/src/gate_thresholds.rs` | Shipped |
| Episode logger | `crates/roko-learn/src/episode_logger.rs` | Shipped |
| Efficiency events | `crates/roko-learn/src/efficiency.rs` | Shipped |
| Graph engine | `crates/roko-graph/src/` | Sole execution engine |
| Runner event loop | `crates/roko-cli/src/runner/event_loop.rs` | Shipped |

---

## Cross-References

- `docs/v3/depth/29-heartbeat/universal-loop-mapping.md` -- CoALA-to-Synapse mapping
- `docs/v3/depth/29-heartbeat/dual-process-t0-t1-t2.md` -- Full dual-process spec
- `docs/v3/depth/29-heartbeat/16-t0-probes.md` -- 16 zero-cost probes
- `docs/v3/29-HEARTBEAT.md` -- Parent chapter
