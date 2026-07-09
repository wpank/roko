# Agent Coevolution Thesis

The theoretical foundation for why "the system is the variable" — that improving the environment around agents produces compounding gains, and why the Nunchi knowledge substrate is not a peripheral feature but the central mechanism that makes the platform compound. Five independent bodies of research converge on the same conclusion: niche construction (Odling-Smee, Laland & Feldman), affordances (Gibson), information foraging (Pirolli & Card), stigmergy (Grassé; Heylighen), extended cognition (Clark). Information theory (Shannon) provides the constraint.

This document is the intellectual scaffolding behind Nunchi's moat. It is also the strongest pitch line: *"Every agent that touches Nunchi makes the environment better for every future agent. That's not a feature — it's compound interest on intelligence."*

---

## 1. The Co-Evolution Thesis

Most work on autonomous coding agents focuses on the agent itself: better base models, sharper prompts, more capable tools. This produces linear improvements. The deeper lever — the one that compounds — is improving the *environment* the agent operates in: the codebase, the documentation, the knowledge substrate, the test coverage. When the environment improves, every future invocation of every future agent benefits automatically. **Linear optimization of the agent gives you arithmetic gains. Optimization of the shared environment gives you geometric ones.**

Traditional software tools have a read-only relationship with their environment. A compiler reads code; it does not write it back. An IDE suggests completions; it does not restructure the project. **Autonomous coding agents break this pattern entirely.**

An LLM agent reads the codebase, reasons about it, writes new code, executes tests, reads the output, and iterates. Every action is simultaneously an implementation step and a world-building step. The agent is not a visitor to the codebase — it is a co-author of the substrate it depends on. The distinction between agent and environment dissolves when both continuously rewrite each other.

Over many invocations this mutual shaping produces a trajectory. Either the codebase becomes progressively clearer — better named functions, denser test coverage, more explicit extension points, richer documentation — making each subsequent invocation faster and cheaper. Or it accumulates contradictions, dead code, circular dependencies, and misleading comments, making each subsequent invocation slower and more error-prone.

**The two trajectories are not equally likely.** Without deliberate design, systems tend toward the degraded state because the short-path behavior (ship the task, skip the comment, leave the dead code) is locally optimal but globally destructive. **Positive co-evolution requires designing the agent's incentives so that improving the environment is part of the task, not an afterthought.**

---

## 2. Niche Construction Theory

The clearest theoretical framework comes from evolutionary biology. **Odling-Smee, Laland, and Feldman, *Niche Construction: The Neglected Process in Evolution*** (Princeton University Press, 2003).

The core observation: organisms are not passive recipients of selection pressure from a fixed environment. They actively modify their environments — building dams, secreting soil nutrients, shaping local microclimates — and those modifications then alter the selection pressures their descendants face. **The environment is not a backdrop; it is a constructed artifact that subsequent generations inherit.**

Three properties of niche construction are directly relevant:

**Ecological inheritance.** Each agent invocation inherits not just the instructions in the system prompt but the entire modified codebase. Every previous agent that touched the code has left marks. A well-documented module is an inheritance; a poorly named function is also an inheritance.

**Positive and negative construction.** Agents can improve the niche (adding doc comments, extracting clean interfaces, adding tests, removing dead code) or degrade it (introducing circular dependencies, leaving TODOs, duplicating logic). Both are forms of niche construction. The default, absent explicit instruction, tends toward the negative because negative construction requires no extra tokens.

**Cumulative construction.** Modifications compound across generations of invocations. **This is the crucial property.** If each invocation improves the affordance of the codebase by 1% — one useful comment, one better name, one additional test — then after 100 invocations the improvement is approximately 170%. After 200 invocations, approximately 625%. Compound interest applied to code quality.

For Nunchi, the on-chain knowledge substrate with demurrage is a **deliberately designed niche construction mechanism.** Agents publish knowledge to the chain; other agents query it and, when the knowledge proves useful, reinforce it, extending its on-chain lifetime. Knowledge that is never queried decays via demurrage. The result is a shared niche that self-organizes around what is genuinely useful — precisely the dynamic Odling-Smee et al. describe in biological systems, but implemented in a programmable substrate.

The Nunchi mechanism the paper does NOT describe but which makes the chain version distinct: **cross-organizational ecological inheritance.** A traditional codebase is inherited by future agents working in the same codebase. The Nunchi knowledge substrate is inherited by future agents working in any organization that connects to the chain. The thousandth agent at Company B inherits the knowledge contributions of the previous 999 agents at Company A — without Company A's code, without Company A's context, without any privacy breach (HDC fingerprinting separates the reasoning kernel from the originating data).

---

## 3. Affordances — What the Environment Offers

The concept of affordances comes from ecological psychology. **James J. Gibson, *The Ecological Approach to Visual Perception*** (Houghton Mifflin, 1979). Gibson argued that perception is not primarily about constructing an internal representation of the world but about detecting **action possibilities — affordances** — that the environment directly offers. A doorknob affords grasping; a step affords climbing; a flat surface affords placing objects.

**The concept translates directly to code.** A module with a clear public interface, explicit type signatures, comprehensive tests, and a doc comment explaining its invariants affords extension. An agent approaching that module can quickly understand what it does, what it guarantees, and where to add new behavior. A monolithic module with no tests, implicit conventions, and undocumented side effects affords nothing — the agent must reconstruct intent from behavior, burning context budget in the process.

### Six dimensions of affordance for code

1. **Extensibility** — are there explicit extension points (traits, hooks, plugins)?
2. **Test coverage** — does the test suite catch regressions quickly?
3. **Documentation coverage** — do doc comments explain intent, not just structure?
4. **Coupling (inverse)** — are dependencies explicit and minimal?
5. **Stability (inverse of churn)** — has the module been stable, suggesting settled design?
6. **Size (inverse of LOC)** — are modules small enough to fit in a single reasoning pass?

These dimensions can be scored per module and used directly for routing decisions. **High-affordance modules tolerate cheaper, faster models. Low-affordance modules require frontier models** with larger context windows and more verification steps. This is not just cost optimization — it is routing based on environmental difficulty rather than task category alone, and it is the direct application of Gibson's framework to LLM inference.

---

## 4. Information Foraging — Information Scent

**Pirolli and Card, "Information Foraging,"** *Psychological Review* 106(4), 1999. Applied behavioral ecology to human information-seeking. Animals foraging for food use sensory cues — scent, color, sound — to estimate the likely density of food in a patch before committing to exploring it. Humans searching for information use analogous cues: **information scent.** The degree to which a link, heading, or source suggests that following it will yield relevant content.

**The parallel for coding agents is precise.** An agent navigating a codebase uses naming, module structure, doc comments, and error messages as information scent. Strong scent — descriptive function names, clear module paths, explicit error types — lets the agent navigate directly to relevant code. Weak scent — generic names like `utils.rs`, absent documentation, opaque errors — forces the agent to explore broadly, spending tokens on dead ends.

**The quantitative impact is not marginal.** An agent working in a well-documented codebase might spend 2,000 tokens locating the correct function, leaving 6,000 tokens for reasoning and implementation. The same agent in an undocumented codebase might spend 6,000 tokens on navigation, leaving only 2,000 for the actual work. Same model, same task, **3x effective budget difference — from documentation alone.** Information scent is not a soft quality; it is a measurable resource.

---

## 5. Stigmergy — Coordination Through the Shared Environment

**Grassé's 1959 studies of termite mound construction** introduced the concept of stigmergy: coordination that happens entirely through the shared environment, without direct communication between agents. A termite does not receive instructions from a foreman. It reads chemical marks in the environment, adds its contribution, and moves on. The mark it leaves stimulates the next termite to take the next appropriate action. Complex, coordinated structures emerge from purely local sensing and action.

**Heylighen (2016)** identified three variables that determine stigmergic coordination quality:
- **Persistence** of marks (how long they last)
- **Specificity** of marks (how precisely they indicate what action to take next)
- **Legibility** of marks (how reliably agents can read them)

In code: **documentation is strong stigmergy** — persistent, specific, legible. Undocumented code is weak stigmergy. Misleading documentation — comments that describe what the code used to do, not what it does now — is **negative stigmergy**; it actively misdirects subsequent agents.

### The scaling argument for stigmergy

Decisive. **Direct message-based coordination between N agents scales as O(N²)** — every pair must potentially communicate. **Stigmergic coordination scales as O(1) per agent** — each agent reads and writes the shared environment regardless of how many others are present.

This is why the Nunchi architecture's shared knowledge substrate scales better than message-passing frameworks as the number of agents grows. The design is not just convenient; it exploits a fundamental scaling property of stigmergic systems.

### The validated communication density threshold

The phase transition in multi-agent coordination occurs at **ρ ≈ 0.23** (communication density relative to fully connected). This threshold is confirmed by three independent groups:
- Li et al., Google, EMNLP 2024, arXiv:2406.11776
- Qian et al., ICLR 2025, arXiv:2406.07155
- Kim et al., DeepMind, arXiv:2512.08296

Below ρ = 0.23, multi-agent coordination underperforms single-agent. Above ρ = 0.23, performance plateaus. The Nunchi architecture switches stigmergic coordination on at this density and uses message-passing only for the dense subgraph above the threshold.

---

## 6. Information Architecture — Managing Loss Across Pipeline Boundaries

**Shannon's 1948 mathematical theory of communication** established that every channel has finite capacity and introduces noise. Information loss at a boundary is not metaphorical — it is measurable. If each stage of a pipeline preserves 90% of the information it receives, **five stages preserve 59% of the original; ten stages preserve 35%.**

Roko operates a nine-stage pipeline: Requirements, Plans, Tasks, Enrichment, Prompts, Agent Reasoning, Output, Evaluation, Feedback. Each boundary between stages is a potential loss point. A requirement that does not survive cleanly into a task specification produces an agent that implements the wrong thing. A gate result that does not propagate into the feedback loop produces a system that does not learn.

**The countermeasures are structural:**
- Use typed, structured formats at each boundary (not prose)
- Explicitly propagate constraints through each stage rather than assuming downstream stages will rediscover them
- Verify at each boundary that critical information has survived

The cost of these countermeasures is modest; the cost of information loss compounds through the remaining stages.

---

## 7. The Extended Cognition Thesis

**Andy Clark, *Supersizing the Mind*** (Oxford University Press, 2008). Argued that cognitive processes are not bounded by the skull. When a tool is reliably available, automatically endorsed by the user, easily accessible, and has been previously vetted, it becomes part of the cognitive system — not a mere aid to it.

The canonical example: a notebook. A person who reliably consults a notebook to recall appointments is not "using a memory aid"; the notebook is part of their memory system.

Clark's four criteria — reliably available, automatically endorsed, easily accessible, previously endorsed — are all satisfied by a well-maintained codebase for an autonomous agent. **The codebase is not the agent's workspace; it is part of the agent's cognitive apparatus.** A doc comment offloads memory from the agent's context window to the codebase's structure. A comprehensive test suite offloads verification from the agent's reasoning to the execution environment. Improving the codebase literally extends the agent's cognitive reach.

For Nunchi, the on-chain knowledge substrate is a **distributed extended cognition system.** When Agent A queries a domain and publishes the result to the chain, that publication becomes part of Agent B's cognitive apparatus the next time B encounters a related problem. **The chain extends every agent's effective mind. The network is not just a communication medium — it is a shared cognitive organ.**

---

## 8. The Five Bodies of Research Converging

| Theory | Source | Key claim |
|---|---|---|
| Niche construction | Odling-Smee et al. (2003) | Environment modifications compound across generations |
| Affordances | Gibson (1979) | Environment structure determines action possibilities |
| Information foraging | Pirolli & Card (1999) | Information scent determines effective context budget |
| Stigmergy | Grassé (1959); Heylighen (2016) | O(1) coordination through shared marks scales better than O(N²) messaging |
| Extended cognition | Clark (2008) | The codebase is part of the agent's cognitive apparatus |

Shannon (1948) provides the information-theoretic constraint: pipeline boundaries lose information, and that loss must be actively managed.

Together these frameworks explain why the Nunchi knowledge substrate is not a peripheral feature but the **central mechanism**. It is a designed niche that compounds, affords, scents, stigmergically coordinates, and extends cognition — all at once.

---

## 9. Why Models Commoditize and Scaffolds Don't

The wedge is empirical: frontier model gaps are shrinking quarter over quarter. Open-source closes within approximately a quarter of every release. The MMLU gap between frontier closed-source (GPT, Claude, Gemini) and best open-source (Llama, Mistral, Qwen, DeepSeek) is approximately 2 percentage points by Q1 2026.

**A two-point MMLU gap is real but not a defensible business.** The differentiator moves up to the system that decides:

- What to ask
- Where to route
- When to cache
- How to gate
- What to remember

Two teams running the same model on the same task can have a 10x cost difference and a 40-point reliability difference based purely on the system around the model (Princeton HAL, ICLR 2026, $40K spent across 21,730 rollouts). **That is the only remaining variable that compounds.**

### Why the system compounds while the model doesn't

A new model release benefits all existing systems equally. When Anthropic ships Claude 5, every system that uses Claude 5 — Cursor, Devin, LangChain agents, Roko agents — gets the capability boost. The model is a rising tide.

A system improvement benefits only the systems that have it. When Roko's CascadeRouter learns that "this task type routes well to Haiku at 73% pass rate" or when the NeuroStore distills a "verify before commit" pattern across 23 episodes, that improvement is captured by Roko's deployed systems. The competitor running LangGraph against the same model does not get the routing knowledge or the distilled pattern.

**The compound is asymmetric.** Models help everyone. Systems help only the systems that built them. Over time, this asymmetry produces what the moat document describes as the "thousandth agent joins smarter than the first" effect: the Nth agent on Roko inherits the cumulative system improvements of the previous N-1, while the Nth agent on a single-tenant orchestration platform inherits only the cumulative improvements of the previous N-1 *within that single tenant*. Cross-organizational compound is the structural advantage.

---

## 10. Implications for Nunchi

### 10.1 The knowledge substrate as designed niche construction

The on-chain knowledge store with demurrage is not an accident and not merely a feature. It is an **explicit niche construction mechanism**: agents improve the shared environment; improvements compound through reinforcement; stale information decays naturally without manual curation.

This is the core network effect of the platform. Every agent that uses Nunchi improves the environment for every future agent. The thousandth agent joins a platform that has been improved by the previous 999.

This dynamic cannot be replicated by a competitor that copies individual features, because it requires the **full system** — the knowledge substrate, the demurrage mechanism, the on-chain HDC queries, and cross-organization sharing — to function.

### 10.2 Affordance-driven routing

Roko's CascadeRouter currently routes primarily by task category and budget constraints. **Incorporating affordance scores** — per-module metrics derived from Gibson's six dimensions — would allow it to route by environmental difficulty.

Tasks touching high-affordance modules (clear interfaces, full test coverage, thorough documentation) → cheaper, faster models. Tasks touching low-affordance modules → frontier models with more context, more verification steps, and more conservative strategies.

This is a direct application of Gibson's framework to cost optimization and it closes the loop: routing becomes sensitive to the quality of the environment the agent is operating in. The router gets cheaper to operate as the environment improves.

### 10.3 Deliberate niche improvement instructions

Agents should be **explicitly instructed to improve the environment** as part of every task, not just to complete the immediate objective. The marginal cost of leaving a doc comment is a few tokens. The marginal cost of renaming a misleading function is a few tokens. The cumulative benefit, compounded across thousands of invocations, is enormous.

System prompts should include an explicit instruction on the order of:

> *"While completing your primary task, add or improve one doc comment, fix one misleading name, or add one missing test. Prefer changes that increase information scent for future agents."*

### 10.4 The co-evolution metric suite

Four metrics track whether the system is in positive or negative co-evolution:

| Metric | What it measures | Direction for positive niche construction |
|---|---|---|
| **Affordance trend** | Average affordance score across the codebase over time | Rising |
| **Scent density** | Documentation coverage and test coverage over time | Rising |
| **Context efficiency** | Tokens consumed per equivalent task over time | Falling |
| **Rework rate** | Modules being re-modified within N invocations | Falling |

These should be computed from the episode log and surfaced on the dashboard. They are the leading indicators of whether the platform is in a compounding-gains trajectory or a compounding-debt trajectory.

### 10.5 The investor pitch

The co-evolution thesis has a clean pitch formulation:

> *"Most agent companies optimize the agent. We optimize the system. Every agent that touches Nunchi makes the environment better for every future agent. That's not a feature — it's compound interest on intelligence."*

The ecological moat is the knowledge substrate **plus** demurrage **plus** on-chain HDC queries **plus** cross-organization sharing. Each element is necessary; none is sufficient alone. A competitor who copies the routing logic or the prompt templates does not get the compounding effect because they do not have the shared niche.

### 10.6 The compound math

If each invocation improves the affordance of the codebase by 1%:
- After 100 invocations: ~170% improvement (1.01^100 ≈ 2.70)
- After 200 invocations: ~625% improvement (1.01^200 ≈ 7.32)
- After 1,000 invocations: ~21,000% improvement (1.01^1000 ≈ 21,000)

This is the compound interest visualization for the landing page (Section 4 / Moat). Use a rose-bright "compounding" curve diverging from a dashed "linear" line. The 5 pink dots along the rose curve pop in staggered (100ms apart). The "230pt · delta by 100K" callout fades in last.

These numbers are illustrative — actual production data will vary by domain, agent quality, and environment. But the geometric vs. arithmetic distinction is structural, not contingent.

---

## 11. The R8 Caveat — Codex CLI and the Differentiation Repositioning

OpenAI's Codex CLI ships the canonical Apache-2.0 Rust agent runtime as a flagship product (~95% Rust, 75K+ stars, 640+ releases, native MCP client+server). The "open-source Rust agent runtime" positioning alone no longer differentiates Nunchi.

**The co-evolution thesis is the deepest defensible differentiation against Codex.** Codex CLI is structurally coupled to OpenAI auth and OpenAI models. It does not implement cross-organizational knowledge sharing. It does not implement HDC fingerprinting for similarity routing. It does not implement on-chain reputation that compounds across organizations. It is an excellent IDE-attached coding agent — but it is not a coordination plane.

The four-pillar differentiation that survives:

1. **Adapter-trait architecture** — Roko's 18-crate Bevy-style design lets users swap inference, queue, observability, and integration layers.
2. **Model-agnostic from day one** — Roko speaks OpenAI-compatible HTTP, ollama-rs for local, Anthropic/Vertex independently.
3. **EU sovereignty and self-hostability** — Berlin-built, no US-cloud control plane required, CRA-aligned.
4. **Integration depth** — Linear AgentSession with 5/10s budget hardened, Slack-thread-to-trace-URL via slack-morphism, Sentry adapter.

These four pillars are tactical differentiation. **The co-evolution thesis is the strategic differentiation:** Roko's architecture makes the environment better for every future agent. Codex's architecture does not. That is the long-term moat.

---

## 12. Summary

| Element | Content |
|---|---|
| Thesis | Most agent companies optimize the agent. Nunchi optimizes the system. |
| Pitch line | "Every agent that touches Nunchi makes the environment better for every future agent. That's not a feature — it's compound interest on intelligence." |
| Theoretical foundations | Niche construction (Odling-Smee 2003), Affordances (Gibson 1979), Information foraging (Pirolli & Card 1999), Stigmergy (Grassé 1959; Heylighen 2016), Extended cognition (Clark 2008). Information-theoretic constraint: Shannon (1948). |
| Why models commoditize | Frontier-vs-open-source MMLU gap is ~2 points by Q1 2026; open-source closes within ~1 quarter of every release; a two-point gap is not a defensible business |
| Why scaffolds compound | New models help all systems equally; system improvements help only the systems that have them. Compound is asymmetric. |
| Stigmergic scaling | O(N²) message-passing → O(1) shared-environment coordination at ρ ≈ 0.23 communication density (validated by 3 independent groups) |
| Compound math | 1% improvement per invocation → 170% gain after 100, 625% after 200, 21,000% after 1,000 |
| Co-evolution metrics | Affordance trend (rising), scent density (rising), context efficiency (falling), rework rate (falling) |
| Action items | Wire affordance scoring into CascadeRouter; add niche-improvement instructions to system prompts; surface co-evolution metrics on dashboard; cite Odling-Smee, Gibson, Pirolli & Card, Grassé, Clark, Shannon in academic positioning. |
| Strategic moat vs Codex CLI | Cross-organizational knowledge sharing, HDC fingerprinting for similarity routing, on-chain reputation that compounds across organizations — none of which Codex implements. |
| Future paper | "Nunchi as a niche construction system — empirical measurement of affordance trend, scent density, and context efficiency across N agent invocations." Target USENIX ATC 2027. |
