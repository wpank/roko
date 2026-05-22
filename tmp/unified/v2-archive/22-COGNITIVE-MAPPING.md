# Cognitive Architecture Mapping

> Maps Roko's architecture onto established cognitive science frameworks (CoALA, 5-Layer Model, GWT, ACT-R) to clarify design decisions, validate architectural choices, and enable academic positioning.

> **Implementation**: Reference document (no implementation required)

**Topic**: Cross-cutting concern
**Prerequisites**: [00-INDEX.md](./00-INDEX.md), [01-SIGNAL.md](./01-SIGNAL.md)

---

## 1. Purpose

This document maps Roko's 3 primitives + 9 protocols + 10 specializations onto four established cognitive architecture frameworks: the 5-Layer Model, CoALA (Cognitive Architectures for Language Agents), Global Workspace Theory (GWT), and ACT-R/SOAR. The mapping serves three purposes:

1. **Validation** — confirms that Roko's design decisions are grounded in 40 years of cognitive science research rather than ad hoc engineering choices.
2. **Gap identification** — surfaces divergences from proven cognitive architectures where Roko either extends the prior art or leaves known-good mechanisms unimplemented.
3. **Citation anchors** — provides the academic scaffolding needed for papers, grant applications, and technical narratives that position Roko relative to the existing literature.

This document is descriptive, not prescriptive. It records where Roko's implemented components map to established concepts. It does not mandate new implementations — those are tracked in the gap table in Section 10.

---

## 2. Five-Layer Model Mapping

The bardo-backup research collection defines a 5-layer model for agent systems that organizes concerns by computational distance from the LLM inference step. The layers form a stack: runtime at the base, orchestration at the apex, with cross-cutting concerns flowing vertically through all layers.

| Layer | Definition | Roko Implementation |
|-------|-----------|-------------------|
| **Runtime** | Process lifecycle, event bus, supervision, structured concurrency | `roko-runtime` (ProcessSupervisor, event bus, cancellation tokens) |
| **Framework** | Core abstractions: signals, tools, memory interfaces | `roko-core` (Signal + 6 verb traits), `roko-std` (19 builtin tools), `roko-fs` (FileSubstrate, JSONL persistence) |
| **Scaffold** | Context assembly, prompt engineering, enrichment, retrieval-augmented generation | `roko-compose` (SystemPromptBuilder with 9 layers, role templates, VCG attention auction) |
| **Harness** | Gates, scoring, evaluation, graduated interventions, circuit breaking | `roko-gate` (11 gates, 7-rung pipeline, adaptive thresholds), `roko-conductor` (10 watchers, circuit breaker), native harness 6-stage pipeline |
| **Orchestration** | DAG scheduling, multi-agent coordination, task decomposition, replan | `roko-orchestrator` (plan DAG, parallel executor, merge queue), `orchestrate.rs` (universal loop) |

The 5-layer decomposition clarifies where extension points live. Adding a new memory backend is a Framework-layer concern. Adding a new gate rung is a Harness-layer concern. Adding a new scheduling algorithm is an Orchestration-layer concern. This separation prevents cross-layer coupling that would otherwise make the system brittle.

**Cross-cutting concerns** span all layers without belonging to any single one:

- **Cost tracking** — token counts and dollar costs are recorded at each layer and aggregated by the orchestrator
- **Observability** — OpenTelemetry `gen_ai.*` semantic conventions are emitted at every LLM call site
- **Security** — CaMeL information flow control (IFC) architecture enforces data handling policies from Framework through Orchestration
- **Learning** — EpisodeLogger, CascadeRouter, and ExperimentStore accumulate signal at every layer and feed it back into Scaffold and Orchestration

---

## 3. CoALA Mapping

**Source**: Sumers, Yao, Narasimhan, Griffiths (2024). "Cognitive Architectures for Language Agents." *Transactions on Machine Learning Research*. arXiv:2309.02427.

CoALA provides a systematic framework for evaluating language agent architectures across three dimensions: memory subsystems, action spaces, and decision procedures. It was derived by surveying 100+ language agent papers and extracting the structural commonalities.

### 3.1 Memory Subsystems

CoALA distinguishes four memory types by storage duration and retrieval mechanism.

| CoALA Memory Type | CoALA Description | Roko Implementation | Status |
|---|---|---|---|
| **Working memory** | Short-term, in-context, bounded by context window | In-context prompt assembly via `roko-compose` SystemPromptBuilder (9-layer construction) | Wired |
| **Episodic LTM** | Long-term records of specific past events | `.roko/episodes.jsonl` via EpisodeLogger; HDC fingerprint per episode | Wired |
| **Semantic LTM** | Long-term structured knowledge; facts, relationships | `roko-neuro` NeuroStore with Ebbinghaus-style decay; queried at dispatch time | Wired |
| **Procedural LTM** | Long-term behavioral patterns; how to accomplish tasks | Playbook store; patterns learned from successful episodes; queried at dispatch into system prompt | Wired |

Working memory management is the central concern of the Scaffold layer. The 9-layer SystemPromptBuilder assembles context in priority order (role identity, task specification, knowledge excerpts, playbook entries, episodic recall, tool descriptions, enrichment, constraints, format directives), then the VCG attention auction allocates the remaining token budget across competing context bidders. This is a direct implementation of the resource-bounded working memory that CoALA describes as the defining constraint on language agent cognition.

### 3.2 Action Spaces

CoALA partitions agent actions into internal (reasoning operations that modify memory) and external (operations that affect the world).

| CoALA Action Category | CoALA Description | Roko Implementation | Status |
|---|---|---|---|
| **Internal: reasoning** | Chain-of-thought, self-reflection, planning | REFLECT stage in native harness; `build_gate_failure_plan_revision` verbalized self-correction | Designed |
| **Internal: memory write** | Store new knowledge, update beliefs | EpisodeLogger write; NeuroStore upsert; playbook store update | Wired |
| **Internal: memory read** | Retrieve relevant knowledge for current context | NeuroStore query at dispatch; playbook query at dispatch; episode recall in SystemPromptBuilder | Wired |
| **External: tool use** | File I/O, web search, code execution | `roko-std` 19 builtin tools + MCP pass-through | Wired |
| **External: agent spawn** | Create new agents to delegate subtasks | `orchestrate.rs` dispatch path; `roko-runtime` ProcessSupervisor | Wired |
| **External: chain operations** | On-chain reads and writes | Phase 4 (chain backend required) | Planned |

The action space decomposition helps explain why `roko-std` separates file tools from web tools from code execution tools: each category has distinct risk profiles, reversibility properties, and cost structures. The Harness layer applies different gate rungs to different action categories precisely because CoALA's taxonomy predicts different failure modes.

### 3.3 Decision Procedures

CoALA identifies planning (long-horizon task decomposition) and execution (per-step action selection within a plan) as the two primary decision procedures.

| CoALA Decision Type | Roko Implementation | Status |
|---|---|---|
| **Planning** | `roko-orchestrator` DAG scheduler; PRD → plan → tasks.toml pipeline | Wired |
| **Plan generation** | `roko prd plan <slug>` agent-driven task decomposition | Wired |
| **Execution loop** | `orchestrate.rs` universal loop: query → score → route → compose → act → verify → write → react | Wired |
| **Re-planning** | `build_gate_failure_plan_revision` triggered by gate failure; controlled by `learning_config.replan_on_gate_failure` | Wired |

### 3.4 Where Roko Extends CoALA

CoALA treats each agent as an isolated cognitive unit. It has no concept of inter-agent identity, trust, shared knowledge substrates, or economic coordination mechanisms. This reflects the state of the field in 2023: multi-agent coordination was emerging but had not yet converged on durable architectural patterns.

Roko extends CoALA across three dimensions:

1. **Identity and trust** — ERC-8004 agent identities with 7-domain reputation registers. Agents are not interchangeable; their identity persists across sessions and is verifiable.
2. **Shared knowledge** — on-chain knowledge substrate with ZK-HDC verification. Semantic LTM is not local to one agent; it is a shared, verifiable commons.
3. **Economic coordination** — VCG attention auction for context assembly; marketplace for task allocation. Cognitive resources are priced, not allocated by fiat.

These extensions are architectural commitments that CoALA does not anticipate because they require infrastructure (blockchain, ZK proofs) that is orthogonal to the purely computational concerns of a single cognitive agent.

---

## 4. Global Workspace Theory Mapping

**Source**: Baars, B.J. (1988). *A Cognitive Theory of Consciousness*. Cambridge University Press.

GWT posits that consciousness arises from a central broadcast workspace where specialized unconscious processors compete for access. The processors that win coalition support broadcast their content to all other processors simultaneously. This broadcast is the functional substrate of conscious experience in the theory.

The architectural implication: a bottleneck broadcast hub surrounded by specialist modules produces global coordination without centralized control. Each module handles only what it is specialized for; the workspace handles cross-module integration.

| GWT Concept | Roko Implementation | Notes |
|---|---|---|
| **Global Workspace (broadcast hub)** | `roko-runtime` event bus | All inter-Cell communication flows through the bus; no direct Cell-to-Cell messaging |
| **Specialist processors** | Cells (10 specializations) | Each Cell subscribes to the bus events relevant to its domain; ignores others |
| **Competition for access** | VCG attention auction in `roko-compose` | Context bidders (Neuro, Task, Research) bid for the limited token budget |
| **Broadcast** | Predict-publish-correct cycle on the bus | Every Cell can publish predictions; outcomes are broadcast to all subscribers |
| **Unconscious processes** | T0 ticks in native harness | Pure Rust, no LLM, $0 cost, ~80% of invocations by volume |
| **Conscious spotlight** | T2 ticks | Full frontier model reasoning, expensive, reserved for novel or complex situations |

The predict-publish-correct cycle deserves emphasis. In GWT, the broadcast workspace does not store results — it broadcasts them and moves on. Roko's event bus implements the same discipline: predictions are published, not accumulated. The correction step (when the actual outcome differs from the prediction) is a write to EpisodeLogger, not a modification to the bus state.

### 4.1 Recent Empirical Validation

Three recent results confirm the architectural predictions of GWT in neural and computational systems:

**GW-Dreamer** (arXiv:2502.21142): A model-based RL system that adds a GWT-style global workspace to DreamerV3. Reaches training criterion in 20,000 steps vs 200,000 for the baseline (10x faster); requires 2-75x fewer FLOPs per task. This validates the hypothesis that a broadcast bottleneck accelerates learning by forcing specialists to produce globally interpretable representations.

**"Theater of Mind" for LLMs** (arXiv:2604.08206, April 2026): Introduces a Stage (short-term memory), Spotlight (attention node), and Audience (heterogeneous LLM agents) architecture. The Stage maps directly to Roko's SystemPromptBuilder-assembled context. The Spotlight maps to `orchestrate.rs` and CascadeRouter. The Audience maps to the agent pool managed by `roko-runtime`. The paper validates multi-agent GWT at the LLM scale.

**Goyal et al. "Shared Workspace Through Attention"** (ICLR 2022): Demonstrates that 8-16-slot working memory bottlenecks consistently outperform full pairwise attention on compositional generalization tasks. Roko's VCG attention auction enforces exactly this discipline: not all context bidders can win; the budget constraint forces selection. The slot count is implicit (determined by token budget) rather than explicit (a fixed integer), which is a refinement over Goyal et al. but preserves the core mechanism.

---

## 5. Dual-Process Theory Mapping

**Source**: Kahneman, D. (2011). *Thinking, Fast and Slow*. Farrar, Straus and Giroux.

Dual-process theory distinguishes System 1 (fast, automatic, associative, cheap) from System 2 (slow, deliberate, effortful, expensive). The two systems do not operate independently: System 1 generates candidate responses continuously; System 2 monitors and overrides when the stakes are high or the pattern is unfamiliar.

Roko's native harness implements a three-tier version of this distinction, which more closely matches the empirical evidence that "System 1" is itself graded rather than binary.

| Dual-Process Tier | Roko Implementation | Approximate Fraction of Invocations |
|---|---|---|
| **System 1 (T0)** | Native harness T0 ticks: pure Rust, no LLM, <1ms, $0. Pattern matching, simple routing, cache lookups, heartbeat checks. | ~60-80% by volume |
| **System 1.5 (T1)** | Native harness T1 ticks: cheap model (Haiku 4.5), moderate context, low cost. Standard tasks requiring light reasoning. | ~15-25% by volume |
| **System 2 (T2)** | Native harness T2 ticks: frontier model (Opus 4.7), full 9-layer context, high cost. Novel problems, complex reasoning, final verification. | ~5-15% by volume |

The GATE stage in the native harness is the meta-cognitive routing function that decides which tier to engage. It evaluates task complexity (from the task manifest), model confidence (from the CascadeRouter's Thompson sampling state), and cost sensitivity (from the budget controller). This exactly mirrors what cognitive neuroscience predicts: a meta-cognitive gating function that allocates expensive deliberative reasoning only when cheap heuristic processing fails.

The cost discipline is significant. A system that routes everything to T2 is not building a cognitive architecture — it is building an expensive lookup table. The economic pressure to route most decisions to T0 and T1 forces the same architectural discipline that biological evolution imposed on biological cognition: deliberation is metabolically expensive, so it must be rationed.

---

## 6. ACT-R and SOAR Connections

**ACT-R** (Anderson, Bothell, Byrne, Douglass, Lebiere, Qin, 2004. "An Integrated Theory of the Mind." *Psychological Review*): Declarative memory (facts, retrievable by associative cue) plus procedural memory (production rules, condition-action pairs) plus a conflict resolution mechanism that selects among applicable rules based on expected utility.

**SOAR** (Laird, 2012. *The Soar Cognitive Architecture*. MIT Press): Universal subgoaling — when an agent reaches an impasse (no applicable rule, or multiple equally applicable rules), it automatically creates a subgoal to resolve the impasse. Successful subgoal resolutions are "chunked" into new production rules to prevent recurring impasses.

| Classical Architecture | Roko Analog | Notes |
|---|---|---|
| ACT-R declarative memory | `roko-neuro` NeuroStore | Knowledge entries with Ebbinghaus-style activation decay; entries become less accessible over time unless reinforced |
| ACT-R procedural memory | Playbook store | Learned patterns for recurring situations; stored as structured prompts, not production rules |
| ACT-R conflict resolution | CascadeRouter model selection + VCG attention auction | CascadeRouter selects among applicable models via Thompson sampling; VCG selects among context bidders |
| ACT-R base-level activation (decay function) | NeuroStore half-lives | Entries decay toward zero activation; on-chain demurrage extends this discipline to shared knowledge |
| SOAR universal subgoaling | Gate failure replan | When a task fails gate validation, `build_gate_failure_plan_revision` creates a structured sub-plan to address the specific failure mode |
| SOAR chunking | EpisodeLogger accumulation | Successful gate passes become episodes; episodes are queried at dispatch time to inform future routing and prompt assembly |

The analogy between SOAR chunking and EpisodeLogger is the most structurally significant. SOAR's chunking mechanism converts expensive deliberative problem-solving into cheap procedural knowledge, enabling the system to handle familiar situations without re-deliberating. Roko's EpisodeLogger accumulates exactly the signal needed to replicate this effect: each successful episode records the task type, the model selected, the context assembled, the gate outcomes, and the final artifact. A future retrieval-augmented dispatch could query episodes to short-circuit deliberation for recognized task types — this is the direct computational analog of SOAR chunking.

---

## 7. Reflexion and Self-Reflection

**Source**: Shinn, Cassano, Labash, Gopalan, Narasimhan, Yao (2023). "Reflexion: Language Agents with Verbal Reinforcement Learning." *NeurIPS 2023*. arXiv:2303.11366.

Reflexion achieves reinforcement without gradient updates by verbalizing evaluation signals back into the agent's context as natural language self-reflection. The verbalized reflection is prepended to the next episode's context, allowing the agent to condition on its own failure analysis. Results: +20 points on HotPotQA (multi-hop QA), +11 points on HumanEval (code generation).

Roko's REFLECT stage in the native harness directly instantiates the Reflexion architecture:

1. Gate failure produces a structured diagnostic (which rung failed, what the artifact contained, what the expected condition was).
2. `build_gate_failure_plan_revision` verbalizes the diagnostic into a natural language plan revision prompt.
3. The revision prompt is dispatched to a frontier model, which produces a structured self-reflection and a revised task plan.
4. The revised plan is executed in the next cycle; the original failure and the reflection are written to EpisodeLogger.

The EpisodeLogger accumulation creates a persistent Reflexion memory. Unlike the original Reflexion paper (which uses only session-level episodic memory), Roko's episodes persist across sessions and are retrievable at dispatch time via playbook queries. This is a structural improvement over the original architecture: the agent's accumulated self-corrections become part of its long-term procedural knowledge.

**Multi-Agent Reflexion** (arXiv:2512.20845, December 2025) extends Reflexion to multi-agent settings by introducing a dedicated judge agent that synthesizes critiques from multiple specialist agents and produces a unified reflection. This is a natural extension of Roko's existing multi-agent orchestration: the judge agent role could be instantiated as a named Cell specialization, receiving gate failure events and producing structured reflections that are broadcast to all active agents on the relevant plan.

---

## 8. Process Reward Models and the Gate Pipeline

Process Reward Models (PRMs) assign step-level reward signals to intermediate reasoning steps, not only to final outcomes. The motivation: outcome-level rewards are sparse and delayed; step-level rewards provide denser signal that guides reasoning more effectively.

Roko's gate pipeline is a deterministic PRM operating at the artifact level rather than the reasoning-step level. Each gate rung evaluates one observable property of the artifact produced by an agent turn: compilation success, test pass rate, lint cleanliness, diff size, contract conformance, semantic similarity. The 7-rung pipeline produces a vector of Boolean pass/fail signals per artifact, which is richer than a single outcome reward.

**ThinkPRM** (arXiv:2504.16828): A learned PRM trained on 1% of PRM800K labels that nonetheless outperforms discriminative verifiers trained on the full dataset. Key finding: the reasoning traces in the evaluation process are themselves informative; a model that thinks through its evaluation outperforms one that outputs a scalar directly. Implication for Roko: the REFLECT stage's verbalized gate failure analysis is already generating the reasoning traces that ThinkPRM would exploit. The episode log is accumulating exactly the labeled data (artifact, rung vector, reflection) needed to fine-tune a ThinkPRM-style learned gate.

**ToolPRMBench** (arXiv:2601.12294): Demonstrates that step-level reward signals for tool-using agents outperform outcome-level rewards across 9 benchmark tasks. This validates Roko's per-turn somatic checks in the native harness (somatic markers from `roko-daimon` that flag distress signals before an artifact is submitted to the full gate pipeline). The somatic checks are a lightweight T0-cost step-level signal that the full gate pipeline refines.

The path from Roko's current deterministic gate pipeline to a hybrid deterministic-learned gate is straightforward: the episode log already contains the labeled examples; the ThinkPRM architecture already provides the training recipe. This is a Phase 2 gap (see Section 10) rather than a Phase 1 priority because the deterministic gates already work well and the learned gate requires sufficient episode volume to train reliably.

---

## 9. Niche Construction and Co-Evolution

**Source**: Odling-Smee, F.J., Laland, K.N., Feldman, M.W. (2003). *Niche Construction: The Neglected Process in Evolution*. Princeton University Press.

Niche construction theory holds that organisms are not passive recipients of environmental selection pressure — they actively modify their environment, and the modified environment then shapes subsequent selection. The process is cumulative: modifications persist across generations, accumulating a constructed niche that is co-inherited alongside genetic material.

For agent systems, niche construction provides a conceptual framework for understanding why an agent's outputs are not merely consumed but become part of the environment that future agents operate in. An agent that writes high-quality code to a repository changes the repository state; the changed repository state affects what future agents can accomplish. The agent has constructed part of its niche.

Roko instantiates niche construction across four mechanisms:

1. **On-chain knowledge substrate with demurrage** — knowledge is added by agents and shared publicly. Demurrage (time-based decay of registration fees) ensures the shared niche is maintained by current participants rather than dominated by historical entries. High-quality knowledge that gets reinforced by subsequent queries becomes more accessible; stale knowledge decays.

2. **Predict-publish-correct cycle** — every prediction an agent publishes, and every correction that follows, modifies the episode record. Future agents dispatched on similar tasks will query episodes and inherit the accumulated corrections. Each agent's self-corrections become environmental signal for its successors.

3. **EpisodeLogger and playbook store** — successful patterns are formalized and made retrievable. The playbook store is the structured niche: a curated collection of effective patterns that any agent can draw on. Contributing to the playbook is contributing to the constructed environment.

4. **Adaptive gate thresholds** — the EMA-based threshold adaptation in the gate pipeline modifies the difficulty of the evaluation environment based on recent performance. An agent population that consistently passes gates at the current threshold will face progressively tighter thresholds. The evaluation environment is constructed by the agents' own performance.

The niche construction lens also clarifies why the on-chain demurrage mechanism is architecturally correct rather than merely economically motivated. Demurrage prevents knowledge from accumulating without maintenance — it enforces the biological analog of selective pressure on constructed niches. Knowledge that no longer receives reinforcement (because the domain it describes has changed) decays rather than persisting as an authoritative but stale anchor.

---

## 10. Gaps and Future Work

The following gaps represent divergences between Roko's current implementation and what the cognitive science frameworks predict would improve performance. They are prioritized by expected impact and implementation feasibility.

| Gap | What Is Missing | Framework Source | Priority |
|-----|----------------|-----------------|----------|
| **Affordance scoring** | CascadeRouter selects models based on task type and past performance but does not score environmental difficulty (e.g., repository complexity, task novelty relative to episode history). Gibson's affordance theory predicts that routing quality improves significantly when the model selection accounts for what the environment permits, not only what the task requires. | Ecological psychology (Gibson 1979) | Phase 1 |
| **Learned PRM** | The gate pipeline is fully deterministic. ThinkPRM demonstrates that a reasoning-augmented learned PRM trained on a small fraction of labeled data outperforms discriminative verifiers. The episode log already accumulates the labeled examples (artifact, rung vector, reflection) needed to train such a model. | ThinkPRM (arXiv:2504.16828) | Phase 2 |
| **Judge agent** | No dedicated critique-synthesizing agent exists for multi-agent plans. Multi-Agent Reflexion (arXiv:2512.20845) demonstrates that a judge agent that synthesizes critiques from multiple specialists produces structurally richer reflections than per-agent self-critique. The orchestrator could instantiate a named judge Cell that subscribes to all gate failure events on a plan. | Multi-Agent Reflexion | Phase 2 |
| **Formal GWT bottleneck** | The VCG attention auction enforces a soft token-budget constraint but does not enforce a hard slot count per Goyal et al.'s finding that 8-16 explicit working memory slots outperform soft budget constraints on compositional tasks. A hard slot limit would require the SystemPromptBuilder to reject low-bid context even when token budget remains. | Goyal et al. ICLR 2022 | Phase 3 |
| **Causal discovery for routing** | CascadeRouter uses Thompson sampling over a historical reward distribution but does not perform causal discovery — it cannot distinguish whether a model succeeded because of its capabilities or because of the task's intrinsic ease. Friston's active inference framework provides a principled basis for structure learning in model selection. | Active inference (Friston 2010) | Phase 3 |
| **SOAR-style chunking retrieval** | EpisodeLogger accumulates successful episodes but dispatch does not yet perform structured retrieval to recognize familiar task types and route directly to the successful pattern without re-deliberating. This is the Roko analog of SOAR chunking and would reduce T2 tick frequency for recurring task types. | SOAR (Laird 2012) | Phase 2 |

The Phase 1 gap (affordance scoring) is actionable with minimal infrastructure: the CascadeRouter already receives task metadata at routing time; extending it to score repository complexity from `roko-index` statistics requires no new subsystem. The Phase 2 gaps each require a new component (trained model, named Cell specialization, retrieval module) but can be built on existing infrastructure. The Phase 3 gaps require either infrastructure changes (hard slot enforcement in SystemPromptBuilder) or research investment (causal discovery implementation).
