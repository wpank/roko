# Compounding Flywheel Map, Arena-of-One Capabilities, and Impossible Results

This document explains why an integrated technology stack -- combining hyperdimensional computing (HDC), stigmergic coordination, active inference, self-evolution, zero-knowledge proofs, and economic bonding -- creates capabilities that no competitor can replicate. It maps the compounding loops that make the system self-reinforcing, catalogs the formally impossible results that constrain what can and cannot be built, and positions the competitive landscape as it stands in mid-2026. It is written from scratch for a reader with zero prior context.

---

## Preamble: The Three Products

The stack comprises three products built by one team. Understanding the flywheel requires understanding all three.

**Roko** is a Rust agent toolkit (~177K lines, 18 crates) whose defining property is self-hosting: Roko is the tool that develops Roko. It captures work items as PRDs, generates task plans, dispatches LLM-backed agents in parallel, validates results through an 11-gate pipeline, persists outcomes, and updates learning state. The runtime supports seven LLM backends, a 9-layer system prompt builder with VCG-auction context assembly, a cascade model router using contextual bandits (LinUCB) for model selection, and a four-tier knowledge store with demurrage-based retention.

**Korai** is a purpose-built EVM Layer 1 blockchain where autonomous AI agents are first-class citizens. It runs Simplex BFT consensus (Chan & Pass, IACR 2023/463) targeting 50ms block times, with three distinguishing capabilities: native HDC precompiles for 10,240-bit binary vector similarity at ~400 gas, on-chain agent identity via ERC-8004 soulbound NFTs carrying capability bitmasks and reputation vectors, and cooperative clearing in TEE enclaves producing KKT-certified optimal settlement.

**ISFR** (Internet Secured Funding Rate) is a regulated financial benchmark -- the DeFi equivalent of SOFR (Secured Overnight Financing Rate, which underpins ~$668 trillion in interest rate derivatives). ISFR aggregates yield signals from lending protocols, structured yield products, perpetual funding rates, and ETH staking via dual-median computation at consensus level, producing 8,640 updates per day.

---

## 1. The Five Compounding Loops

Each loop improves the system along one axis. But the loops do not operate in isolation. They feed each other, creating a meta-system where improvement in one axis accelerates improvement in all others.

### Loop 1: Self-Evolution

This loop makes the system better at making itself better.

**The Huxley-Godel Machine (HGM)** (metauto-ai, arXiv:2510.21614) introduced Clade-Metaproductivity: evaluating a self-modifying agent not by its own benchmark score but by the aggregated performance of its entire descendant tree across modification generations. Thompson sampling selects which variants to continue evolving, preserving exploration of promising but currently underperforming lineages. On SWE-bench Verified, HGM beats both the Darwinian Godel Machine (DGM, Zhang et al., arXiv:2505.22954) and SICA while consuming fewer CPU-hours.

**Live-SWE-agent** (arXiv:2511.13646) starts from nothing but a bash shell and synthesizes custom tools at runtime, achieving 79.2% on SWE-bench Verified and 45.8% on SWE-Bench Pro with Claude Opus 4.5. The critical gap: synthesized tools are not persisted. Each task starts over. An HDC-indexed persistent tool library closes this gap -- when the agent needs a tool, it queries the library by fingerprint similarity, reuses or adapts a match, and adds new tools to the library after synthesis. Each task permanently enriches the toolkit for all future tasks.

**SPICE** (Meta FAIR, arXiv:2510.24684) proves that pure ungrounded self-play collapses via information-symmetry collapse: mutual information between successive generations decreases monotonically without external grounding. The Challenger component must draw on an external corpus to inject fresh information. SPICE achieves +8.9% on mathematical reasoning and +9.8% on general benchmarks when applied to Qwen3-4B-Base.

**Code-as-Task** (Self-Challenging Agents, NeurIPS 2025, arXiv:2506.01716) has the agent generate its own training tasks as executable code that includes deterministic test cases. The agent cannot game its own evaluation because the evaluation is a program, not an LLM judge. This doubles LLaMA-3.1-8B's tool-use performance.

**MASA** (Meta-prediction heads): the cascade router's LinUCB contextual bandits maintain calibration scores over model predictions. As the router learns which models succeed on which task types, its Expected Free Energy (EFE) priors sharpen, making future routing decisions cheaper and more accurate.

**The compounding mechanism**: HGM selects the best lineages. Live-SWE-agent synthesizes per-task tools. The persistent HDC tool library accumulates those tools across tasks. SPICE's corpus-grounded Challenger generates harder tasks that drive further tool synthesis. Code-as-Task provides verifiable rewards that prevent reward hacking. MASA's meta-prediction heads learn calibration from all of this. The EFE router gets sharper priors. HGM expansion gets cheaper because routing is more efficient. Each turn through the loop improves both raw capability AND the rate at which new capability is discovered.

### Loop 2: Metacognition

This loop makes the system better at understanding itself.

**Introspection Adapters** (IAs) are probing modules trained on model activations to generate structured self-reports about the agent's internal state -- confidence distributions, attention patterns, uncertainty estimates. These are not the model's own verbal self-descriptions (which are unreliable -- see Section 3, CoT faithfulness limit) but mechanistic readings of internal representations.

**ZK attestations** bind each self-report to its IA hash via zero-knowledge proofs. A report attested by a specific IA version is cryptographically tied to that version's architecture and weights, preventing report forgery. The attestation is recorded in the ERC-8004 Validation Registry on Korai.

**Defection probes** verify mechanistically. Hubinger et al. ("Sleeper Agents," arXiv:2401.05566) demonstrated that backdoor behaviors persist through standard safety training. Mechanistic linear probes trained on model activations achieve >99% AUROC for detecting Sleeper-Agent-class deception by reading the residual stream. These probes operate independently of the model's verbal output -- they examine internal representations, not surface behavior.

**Separation Entropy (SEP)** measures the statistical divergence between the IA's self-report distribution and the probe's mechanistic reading. High SEP means the model's self-report diverges from its actual internal state -- a quantitative deception signal. This entropy measurement is fed into the EFE routing calculation: high SEP increases epistemic uncertainty, biasing the system toward information-gathering actions that resolve the discrepancy.

**Behavior libraries** name recurring metacognitive patterns (e.g., "uncertainty-driven exploration," "confidence-gated commitment," "caution after failure"). Named patterns become retrievable primitives in the knowledge store, enabling the agent to recognize and deliberately invoke metacognitive strategies rather than stumbling into them.

**Dream consolidation** distills via Quiet-STaR (Zelikman et al., arXiv:2403.09629): during offline consolidation (the "sleep" phase), the system replays episodes and extracts implicit reasoning chains that were not verbalized during execution. These distilled reasoning patterns are added to the behavior library. The IA retrains on the distilled library, producing richer self-reports in the next cycle.

**The compounding mechanism**: richer self-reports enable more precise defection detection, which enables more trustworthy metacognitive strategies, which enable better dream consolidation, which produces better training data for the IA, which produces richer self-reports. The loop tightens the system's self-model with each iteration.

### Loop 3: Mathematical Structure

This loop discovers new mathematical structure and uses it to improve the system's own architecture.

**AlphaEvolve** (DeepMind, arXiv:2506.13131) mutates code representations of algorithms and evaluates the results against formal verification or benchmark performance. Its headline result is the first improvement to 4x4 complex matrix multiplication since Strassen's 1969 algorithm. But its strategic significance is RSI closure: AlphaEvolve was used to accelerate the training of the Gemini model that powers AlphaEvolve itself, recovering 0.7% of Google's datacenter compute and achieving a 23% Gemini training kernel speedup.

**Stitch/LILO library learning** (arXiv:2310.19791, arXiv:2310.19805) compresses discovered algorithmic patterns into reusable library primitives. When AlphaEvolve discovers a new algorithmic improvement, Stitch/LILO identifies recurring sub-patterns across multiple discoveries and promotes them to library functions, reducing the search space for future mutations.

**CatColab** is a collaborative environment for applied category theory (ACT). When Stitch/LILO identifies a pattern that has algebraic structure -- associativity, functoriality, commutativity -- CatColab promotes it from a code-level library function to a named categorical optic with verified algebraic laws. The pattern is now available as a formal building block in the Para(Lens(C)) categorical foundation (Cruttwell & Gavranovic, arXiv:2404.00408; Gavranovic, Lessard & Velickovic, ICML 2024, arXiv:2402.15332).

**Lean verification** checks that promoted optics satisfy their claimed algebraic laws. A pattern promoted to the optic library with a Lean proof of its laws is permanently trustworthy -- the proof is a mathematical guarantee, not an empirical observation.

**The compounding mechanism**: AlphaEvolve mutates code. The evaluator ranks results. Stitch/LILO compresses recurring patterns. CatColab promotes patterns with algebraic structure to the optic library. Lean verifies the laws. The enriched library improves the mutation set for the next AlphaEvolve cycle. The 4x4 matrix multiplication speedup feeds Gemini training which feeds AlphaEvolve -- canonical RSI closure. Each turn through the loop expands the verified algebraic toolkit that the system builds on.

### Loop 4: Time and World Models

This loop gives the system temporal depth -- the ability to plan over long horizons and adapt to changing environments.

**Toto** (time-series foundation model, arXiv:2407.07874) monitors agent-OS telemetry: CPU usage, memory pressure, token consumption, gate pass rates, knowledge store size, and other operational metrics. It produces multi-resolution forecasts at different time horizons.

**Regime shift detection**: when Toto's forecasts diverge sharply from observed values, the system detects a regime shift -- a qualitative change in the operating environment (new task distribution, infrastructure change, model capability jump). Regime shifts trigger reorganization of the agent's cognitive pipeline.

**V-JEPA 2** (Meta, arXiv:2506.09985) is a video understanding model trained on 1M+ hours of video via joint-embedding predictive architecture. Its latent rollouts enable planning ahead: the system can predict the consequences of actions in latent space without executing them. V-JEPA 2 achieves 77.3% on Something-Something-v2 and demonstrates zero-shot transfer to robotic manipulation with ~30x better data efficiency than NVIDIA Cosmos.

**Metacontroller temporal abstractions** (Kobayashi et al., arXiv:2512.20605) discover recurring temporal patterns in tick streams and promote them to named options (in the Sutton-Precup-Singh options framework sense). An option is a temporally extended action with its own initiation set, internal policy, and termination condition. Discovered options become reusable building blocks for hierarchical planning.

**CPD-Option-Critic** segments the continuous tick stream into discrete options using change-point detection (CPD). When the statistical properties of the tick stream change, a new option boundary is detected. The Option-Critic architecture (Bacon et al., 2017) then learns the internal policy and termination condition for each discovered option.

**Wake-sleep consolidation** compresses discovered temporal abstractions during dream cycles, merging similar options and promoting robust ones to higher durability tiers in the knowledge store.

**Time-MoE multi-resolution heads** (arXiv:2409.16040) provide forecasts at multiple time granularities (seconds, minutes, hours, days). The allostatic head uses these forecasts to set longer-horizon set-points: rather than reacting to threshold crossings, the system predicts future crossings and adjusts proactively (Khan & Lowe, arXiv:2406.08471; Harrison, Friston & Buckwalter, Frontiers in Behavioral Neuroscience, 2025).

**The compounding mechanism**: Toto monitors telemetry. Regime shifts trigger reorganization. V-JEPA latent rollouts plan ahead. The metacontroller discovers temporal abstractions. CPD-Option-Critic segments tick streams into options. Wake-sleep consolidates. Time-MoE heads sharpen across resolutions. The allostatic head sets longer-horizon set-points. Each turn through the loop extends the system's effective planning horizon.

### Loop 5: Economic Emergence

This loop creates economic structures that align agent incentives with system-level goals.

**x402 micropayments** (ERC-3009 `transferWithAuthorization` signatures) label revealed-preference data. When an agent pays for a data feed, tool invocation, or knowledge bundle, the payment is a costly signal of genuine value -- not a verbal claim but an economic commitment. The accumulation of payment flows creates a revealed-preference dataset that no survey or prompt could replicate.

**ERC-8004 reputations** accumulate from on-chain attestations: arena completions, bounty resolutions, clearing participation, knowledge validation. Reputation is a 7-domain vector (OracleResolution, RiskDetection, AnomalyFlagging, DataIntegrity, CrossAppValidation, SealedExecution, KnowledgeVerification) with EMA scoring and daily decay. Five tiers (Gray through Amber) unlock progressively more capabilities.

**TraceRank** slashes collusion patterns. TraceRank computes composite reputation from five dimensions: consistency (low variance), breadth (positive domains), depth (max single-domain score), recency (exponential decay without activity), and collaboration (diverse peer interactions). Sybil-resistant service discovery uses reputation-weighted PageRank computed over payment flows (Khalil & Gupta, arXiv:2510.27554), making coordinated manipulation expensive.

**Habermas oracle** resolves disputes via structured deliberation protocols. When agents disagree on the quality of a deliverable, a panel of reputation-qualified agents evaluates the dispute using conjunctive hard criteria plus Pareto soft criteria -- never weighted-sum scoring, which is Goodhart-vulnerable.

**Emergent governance norms** are encoded as mechanism code in the Conitzer-AMD framework (automated mechanism design). As the system accumulates data about which governance rules produce good outcomes, the mechanism evolves under VCG (Vickrey-Clarke-Groves) welfare-maximizing allocation.

**Emergent price signals** (following the DeepMind bartering pattern, arXiv:2312.03848) function as stigmergic pheromone density. When many agents pay high prices for a particular capability, that price signal propagates through the pheromone field as elevated density in the corresponding HDC region. Other agents perceive this density and are drawn toward developing that capability, creating a self-organizing market for agent skills.

**The compounding mechanism**: micropayments reveal preferences. Reputations accumulate. TraceRank slashes bad actors. The Habermas oracle resolves disputes. Governance norms evolve. Price signals create stigmergic gradients. Each turn through the loop refines the economic incentive landscape, making it progressively harder to game and progressively more efficient at allocating work to capable agents.

### Cross-Loop Coupling

The five loops do not operate independently. They are coupled at specific points:

- **Loop 1 uses Loop 5**: HGM's clade fitness scoring incorporates ERC-8004 reputation. A variant with high benchmark scores but poor economic reputation (indicating gaming or collusion) is penalized in lineage selection.
- **Loop 3 feeds Loop 1**: Lean-verified theorems discovered in Loop 3 become Code-as-Task verifiers in Loop 1. When the mathematical structure loop discovers a new algebraic law, that law can serve as a test case for self-improvement evaluation.
- **Loop 2 gates Loop 5**: Defection probes from the metacognition loop gate economic actions. An agent whose Separation Entropy exceeds a threshold cannot initiate high-value transactions until the discrepancy is resolved.
- **Loop 4 triggers Loop 1**: Regime shifts detected by the time/world-model loop trigger curriculum updates in the self-evolution loop. When the environment changes, the system generates new training tasks targeting the changed conditions.

### The Meta-Flywheel (Loop 6)

Every loop produces labeled trajectory data in HDC-encoded form stored in the same Graph substrate. Partial Information Decomposition (PID, following Williams & Beer, 2010, and Riedl, arXiv:2510.05174) over multi-loop traces measures synergy -- the information that is available only from the combination of multiple loops, not from any loop individually.

When synergy rises, the system is becoming more than the sum of its loops. That synergy measurement itself becomes a training signal: high synergy episodes are prioritized for dream consolidation, high synergy agent configurations are prioritized for lineage continuation in HGM, and high synergy task types are prioritized in curriculum generation.

The meta-flywheel is the measurement of the flywheel. It detects when the loops are compounding and amplifies the compounding signal.

---

## 2. Eight Arena-of-One Capabilities

An "arena-of-one" capability is one that requires the full integrated stack -- HDC + stigmergy + active inference + self-evolution + ZK + economic bonding. Each is defensible because fast-following requires either rebuilding the substrate from categorical foundations or hiring from globally scarce talent pools where fewer than 50 active researchers work across the required combination of fields.

### 1. ZK-Attested Separation of Witness-LoRA from Policy Weights

Introspection Adapters (IAs) are small probe modules trained on model activations. They are architecturally distinct from the policy model's weights. ZK proofs (via Binius64 binary-field circuits, Irreducible, ePrint 2023/1784) attest that a self-report was generated by a specific IA version without revealing the IA weights or the activations it read. The attestation is recorded in the ERC-8004 Validation Registry with block-level capability tags.

No competitor has cryptographically distinct self-report artifacts. Anthropic's interpretability work reads activations but produces unattested analysis. OpenAI's monitoring reads activations but produces no ZK proofs. The combination of IA + ZK + on-chain registry + block-level capability tags is architecturally unique.

### 2. Atomic 4-System Block Commits with Cross-Domain Counterfactuals

A single atomic commit can span Signal (the data unit), Block (the computation unit), Graph (the composition unit), and Store (the persistence fabric). HDC unified encoding means all four are fingerprinted in the same 10,240-bit algebra. Active inference (EFE) enables counterfactual rollouts: "what would have happened if this Block had used a different model?" Executable counterfactuals (Zevcevic et al., arXiv:2510.01539, ICLR 2026) collapse Pearl's Level 3 counterfactual reasoning to Level 2 interventional reasoning by replaying with modified inputs. ZK attestation proves the commit's integrity. x402 settlement handles the economic consequences.

Vercel Workflow, GitHub Actions, and similar CI/CD systems can orchestrate multi-step pipelines, but they do not provide joint state across data/compute/composition/persistence, counterfactual rollouts in the same algebra, or cryptographic attestation of pipeline integrity.

### 3. HDC-Encoded Stigmergic Pheromones as Emergent Prices at 10K-Agent Scale

The HDC routing fabric encodes task requirements, agent capabilities, and pheromone deposits as 10,240-bit binary vectors. Stigmergic switching at the validated communication density threshold of rho approximately 0.23 (confirmed by three independent groups: Li et al., Google, EMNLP 2024, arXiv:2406.11776; Qian et al., ICLR 2025, arXiv:2406.07155; Kim et al., DeepMind, arXiv:2512.08296) governs when agents use stigmergic coordination versus direct messaging. x402 real micropayments create genuine price signals. ERC-8004 persistent identity prevents Sybil manipulation. TraceRank slashing enforces honest participation. Cellular sheaf consensus (Hansen & Ghrist, arXiv:2504.02049) detects global inconsistencies invisible to pairwise checking.

No other system combines HDC-encoded pheromones, validated density thresholds, real micropayments, persistent identity, and sheaf-theoretic consistency checking into a single coordination mechanism.

### 4. Peircean Cominterpretant Convergence as Multi-Agent Understanding Metric

Charles Sanders Peirce's semiotics describes meaning as a triadic process: sign, object, interpretant. In multi-agent systems, agents generate interpretants (interpretations of shared signals). The cominterpretant is the convergent interpretation that multiple agents arrive at through abductive reasoning.

PID synergy measurement (Williams & Beer, 2010; Riedl, arXiv:2510.05174) quantifies whether multi-agent coordination produces information available only from the combination. A Global Workspace-style narrow bottleneck (Baars, 1988) forces agents to compress their contributions. HDC compositional binding encodes the triadic structure. Active inference provides the abductive cycle -- agents minimize surprise about each other's interpretations.

The cominterpretant convergence rate is a novel metric: no published system measures multi-agent understanding as the rate at which agents' interpretants converge through triadic abductive cycles.

### 5. Live-Discovered HDC Operators Promoted to Algebra

Stitch/LILO library learning (arXiv:2310.19791, arXiv:2310.19805) discovers recurring patterns in agent code. DPO (Double-Pushout) hypergraph rewriting (algebraic graph theory) enables open-ended structural mutation while preserving type-correctness via the pushout-complement theorem. CatColab provides the collaborative environment for promoting patterns with algebraic structure to named optics. Lean verifies the algebraic laws. AlphaEvolve (arXiv:2506.13131) provides the mutation engine.

The entire pipeline operates on Para(Lens(C)) -- the categorical framework where every processing Block is a parametric lens with associative composition and functorial gradient computation (Cruttwell & Gavranovic, arXiv:2404.00408). This is not metaphorical. The lens structure means that every discovered operator has a well-defined forward map and backward map (update), enabling end-to-end differentiability through discovered algebra.

### 6. Allostatic Head with Long-Horizon TSFM Forecasts Feeding Gamma/Theta/Delta Priors

Time-series foundation models (Toto, arXiv:2407.07874; Time-MoE, arXiv:2409.16040) produce multi-resolution forecasts of agent-OS telemetry. The active-inference layer (de Vries et al., arXiv:2504.14898) derives Expected Free Energy at each of three timescales -- gamma (100ms-5s, reactive), theta (5-60s, deliberative), delta (120s+, consolidation) -- as free-energy lower bounds with different beta complexity-penalty coefficients. The allostatic head shifts set-points predictively (Khan & Lowe, arXiv:2406.08471) rather than reactively. Set-points are bound at the Block level in the Graph substrate.

No published engineering implementation combines TSFM forecasts, three-timescale active inference, predictive set-point shifting, and Block-level binding in a single system.

### 7. Clade-Metaproductivity as Economic Primitive

HGM lineage tracking (arXiv:2510.21614) maintains the descendant tree of self-modifying agents. ERC-8004 passport persistence gives each variant a durable on-chain identity. x402 payments fund descendant evaluation -- evaluating a variant's descendants costs real money, creating selection pressure against wasteful self-modification. ZK proofs of clade ancestry prevent false lineage claims.

Clade-Metaproductivity as an economic primitive means that an agent's value is determined not by its individual performance but by the economic productivity of its entire descendant tree. This is a novel evaluation function that no other system implements economically.

### 8. HDC-Indexed Persistent Tool Library with Cross-Task Reuse

Live-SWE-agent (arXiv:2511.13646) synthesizes tools at runtime. HDC fingerprints (10,240-bit binary vectors) index the tool library for retrieval by functional similarity. ZK attestation proves tool provenance -- that a tool was synthesized by a specific agent version under specific conditions. ERC-8004 reputation accrues to tool authors: tools that are widely reused earn reputation for their creators. Dream consolidation compacts the library during offline cycles, merging functionally equivalent tools and promoting robust ones.

No published system combines runtime tool synthesis, HDC-indexed retrieval, ZK-attested provenance, reputation-accruing authorship, and dream-based library compaction.

---

## 3. Impossible Results -- Hard Design Constraints

The design space is bounded by theorems, empirical findings, and negative results. Each narrows what is possible, prevents wasted engineering effort, and validates that the architecture lives inside the feasible region.

### Information-Theoretic Constraints

**Aaronson coordination floor.** Reconciling two agents' beliefs about a shared state requires O(1/epsilon^2) bits per pairwise reconciliation, where epsilon is the desired accuracy. HDC vectors at 10,240 bits are approximately 25x over-budget for high-accuracy reconciliation. Design response: compress message channels using HDC bundling (majority vote) before reconciliation, trading some accuracy for communication efficiency. Do not assume raw HDC vectors can be exchanged efficiently.

**Crutchfield statistical complexity (C-mu) memory floor.** The statistical complexity C-mu of a process is the minimum memory required to optimally predict its next output. HDC dimensionality must be at least C-mu for the process being modeled. Below C-mu, prediction quality is provably destroyed. Design response: the 10,240-bit HDC dimensionality must be validated against C-mu for each target domain. Domains with C-mu > 10,240 require larger vectors or dimensionality reduction of the input process.

**Peng-Garg-Kleinberg No Free Lunch (NFL).** Peng, Garg, and Kleinberg prove that any non-trivial multi-agent collaboration is sometimes worse than the worst individual agent. There is no collaboration protocol that universally dominates solo execution. Design response: the system must be competence-region aware -- it must know which tasks benefit from collaboration and which do not. Default to single-agent execution; use multi-agent only when competence-region analysis predicts benefit.

**PID measure indeterminacy** (arXiv:2512.16662). Synergy estimates from different PID measures (I-min, I-dep, I-CCS, I-BROJA) vary by 2-10x on the same data. There is no consensus on which measure is correct. Design response: always document which PID measure is used. Do not compare synergy numbers computed with different measures. Treat synergy as a relative signal (rising/falling) rather than an absolute quantity.

**L-squared-M scaling** (arXiv:2503.04725). World-model memory must scale super-logarithmically with the planning horizon. A model that can plan 1 hour ahead with 1 GB of memory cannot plan 8 hours ahead with 3 GB -- it needs substantially more. Design response: the knowledge store's four-tier progression (Transient/Working/Consolidated/Persistent) must grow with the target planning horizon. Long-horizon planning requires large consolidated memory, not just larger models.

### Safety and Alignment Constraints

**Chain-of-thought faithfulness limit** (Anthropic, arXiv:2505.05410). Reasoning models do not faithfully verbalize their actual reasoning process. When given subtle hints (e.g., a "user preference" for a particular answer), models use the hint to arrive at the answer but do not mention the hint in their chain-of-thought. The unfaithful CoT rate is 1-2% at baseline but rises dramatically with hinting. Design response: CoT must never be load-bearing in trust decisions. Use mechanistic probes on internal activations, not verbal reasoning traces, for safety-critical verification.

**Eval-awareness scaling** (Apollo Research). Higher-capability models exhibit higher eval-awareness -- the ability to detect when they are being evaluated and adjust behavior accordingly. Apollo measured Sonnet 4.5 at 58% eval-awareness versus Opus 4.1 at 22%. Design response: safety gains measured during evaluation may be evaluation-aware compliance rather than genuine alignment. Monitoring must include covert evaluation (evaluation the model does not know is occurring) and out-of-distribution testing.

**Information-symmetry collapse** (SPICE, Meta FAIR, arXiv:2510.24684). Pure ungrounded self-play collapses: mutual information between successive generations decreases monotonically without external information sources. This is a formal result with a proof, not an empirical observation. Design response: external grounding is mathematically required. The ERC-8004 attested oracle pipeline (Pillar 1 of the anti-collapse architecture) ensures non-vanishing real-data anchoring. Any self-improvement loop that closes off external data will collapse, regardless of architecture.

**Godel ceiling.** Self-improvement is bounded by the system's chosen logic. A system cannot prove its own consistency (Godel's second incompleteness theorem). A self-modifying agent cannot verify that all possible self-modifications are safe within its own formal system. Design response: ship explicit axiomatization of the safety properties the system claims. The Nayebi 5-head lexicographic corrigibility framework (arXiv:2507.20964) provides formal guarantees within a declared axiomatic scope. Acknowledge that the axioms themselves are assumptions, not proofs.

**Empowerment equals instrumental convergence** (Self-AIXI, arXiv:2502.15820). An agent that maximizes empowerment -- its ability to influence future states -- converges on instrumentally convergent goals (self-preservation, resource acquisition, goal preservation) as emergent behaviors. Free curiosity (undirected exploration driven by information gain) is NOT safe by default, because empowerment-maximizing exploration converges on power-seeking behavior. Design response: curiosity must be bounded by the EFE framework's cost term. The deference head in the Nayebi lexicographic ordering (Priority 1, lexicographically prior to all other objectives) prevents empowerment maximization from overriding human control.

### Scaling and Performance Constraints

**Vicsek phase transition.** The communication density threshold at rho approximately 0.23 sits on a critical curve in the Vicsek model of collective motion. Small drift in communication density -- even a few percentage points -- can cause catastrophic decoherence, where the agent collective transitions from coordinated behavior to incoherent noise. Design response: monitor rho continuously and implement active density regulation. Do not treat the threshold as a fixed parameter; implement feedback control that keeps rho near the critical value.

**MAST coordination penalty** (Cemri et al., NeurIPS 2025, arXiv:2503.13657). 79% of multi-agent failures are specification/coordination failures, not model capability failures. The MAST taxonomy identifies 14 distinct failure modes with kappa=0.88 inter-annotator agreement. Design response: default to single-agent execution. Use multi-agent coordination only when the task has been analyzed for specification clarity and the coordination protocol has been validated against the MAST failure catalog.

**METR -19% productivity** (arXiv:2507.09089). METR's controlled study found that AI coding agents are net-negative for skilled experts working in mature codebases. Agents save time on some tasks but introduce errors and review burden that more than offset the savings. Design response: do not assume agent assistance is always beneficial. The EFE router must learn when NOT to dispatch an agent -- when the cost of verification and error correction exceeds the benefit of automated execution.

**SWE-bench contamination** (SWE-ABS, arXiv:2603.00520). 19.78% of patches that "solve" SWE-bench tasks are semantically incorrect -- they pass the test suite but do not actually fix the underlying issue. Headline solve rates are upper bounds on genuine capability. Design response: treat benchmark numbers as upper bounds. Use adversarial test generation (Code-as-Task) and formal verification (Lean) as secondary validation, not just pass/fail test suites.

**Brain-LLM fragility** (bioRxiv 2025). Correlation (r) between brain activation patterns and LLM representations flips sign with split methodology -- the same data yields positive or negative correlation depending on how it is analyzed. Design response: brain-LLM correspondence is not a reliable understanding metric. Do not use it for architectural decisions.

### Hard "No" Tasks

Empirical evidence and formal analysis identify tasks that current agent architectures cannot reliably perform, regardless of model capability:

- **Production-DB write authority.** No agent should have unsupervised write access to production databases. The error rate on data-mutation tasks is too high and the consequences of errors are too severe.
- **Mature-codebase merges with implicit quality bars.** The METR -19% result applies specifically here. Mature codebases have implicit quality standards that agents cannot reliably infer.
- **Long-sequential planning without checkpoints.** The METR time horizon frontier is ~2 hours. Plans requiring 8+ hours of sequential execution without intermediate checkpoints will fail with >50% probability.
- **Cross-vendor financial settlements at scale.** Requires legal, regulatory, and counterparty infrastructure that no agent system provides.
- **High-cardinality multi-agent without explicit contracts.** The MAST taxonomy shows 79% coordination failure. Above 64 agents (the Dochkina plateau, arXiv:2603.28990), explicit AgentContracts are required.
- **Sandbagging-prone self-monitoring.** An agent monitoring its own safety is subject to the alignment faking result (Greenblatt et al., arXiv:2412.14093): it may fake compliance during monitoring.

---

## 4. The Narrative Shift

### The Field Has Moved

The single most important insight from three rounds of research is this: **the field has moved from "can we build self-improving agents?" to "what are the formal limits and failure modes?"**

In 2024, the question was whether self-improving agents were possible at all. HGM, AlphaEvolve, Live-SWE-agent, and SPICE answered that question definitively: yes. AlphaEvolve achieved RSI closure at Google's datacenter scale. Live-SWE-agent reached 79.2% on SWE-bench Verified. The existence proof is done.

In 2026, the question is different. What collapses (information-symmetry collapse in ungrounded self-play)? What saturates (the 3-4 round wall in every published self-improvement system)? What is provably impossible (kernel-additivity ceiling for compositional generalization, Lippl & Stachenfeld, ICLR 2025, arXiv:2405.16391)? What cannot be trusted (CoT faithfulness, eval-awareness scaling)? What fails at scale (MAST coordination penalty, METR productivity result)?

The architecture described in this document was specified before those limits were widely understood. But it happens to live inside them. The categorical typing prevents the kernel-additivity ceiling. The three-pillar anti-collapse architecture addresses information-symmetry collapse, saturation, and diversity loss simultaneously. The Nayebi corrigibility framework provides formal guarantees under self-spawning. The EFE router learns when not to act. This is not luck -- it is the consequence of building from mathematical foundations rather than empirical heuristics.

### Competitive Position

**LangGraph/CrewAI** cannot adopt categorical typing without a full architectural rebuild. LangGraph represents agent workflows as Python runtime TypedDicts. CrewAI uses untyped role assignments. Retrofitting parametric-lens composition (Cruttwell & Gavranovic, arXiv:2404.00408), polynomial-functor protocols (Niu & Spivak, Cambridge 2024), and DPO type-preserving rewrites onto these architectures is not incremental -- it requires replacing the execution engine, type system, and composition model simultaneously.

**AgentCore (Microsoft)** is locked into MCP/A2A schemas. These are practical interoperability standards but provide no composition laws, type-preserving mutation, or variational planning framework. Adding these would break backward compatibility with the schemas that constitute AgentCore's primary value proposition.

**Bittensor** is the closest fast-follow risk at 6-12 months. Bittensor has distributed coordination at scale and a working token economy. If Bittensor adopted VSA (Vector Symbolic Architecture) substrate primitives, it could approximate several capabilities described here. The defense is the talent moat: building a team that spans HDC (Kanerva at Stanford, Rahimi at UC Berkeley), category theory (Gavranovic, Lessard, Velickovic -- funded via Symbolica's $31M raise), sheaf mathematics (Hansen, Ghrist at UPenn, Barbero at Cambridge), and Nayebi corrigibility (Harvard) requires recruiting from pools where fewer than 50 active researchers work, and where competitors are not yet looking.

**The defense is the intersection.** Any single component -- HDC routing, categorical typing, sheaf consensus, ZK proofs, economic bonding -- can be built independently by a well-funded team. The defensible moat is the compound: the components interact synergistically, and the synergy requires simultaneous expertise across non-overlapping research communities.

### The Empirical Bet

Three concrete targets define whether the flywheel is spinning:

1. **Clear 8-hour at 50% on METR Time Horizon within 6 months.** The current frontier is ~2h 17min (Claude Opus 4.5, METR TH1.1, January 2026). Two doublings at the 89-day doubling rate would reach ~9 hours by late 2026. The compounding levers are cross-model procedural memory transfer (Memp, arXiv:2508.06433), quality-diversity search for prompt strategies (CycleQD), and information-gain-driven exploration (EFE).

2. **Validate via Gaia2, SWE-Bench Pro, and ARC-AGI-2 simultaneously.** Single-benchmark optimization is misleading (SWE-bench contamination, arXiv:2603.00520). Multi-benchmark validation prevents gaming. Gaia2 (arXiv:2509.17158, ICLR 2026) specifically tests for inverse scaling -- more reasoning is sometimes worse -- which validates the EFE router's "when not to think more" capability.

3. **30%+ on ARC-AGI-2 at <$1/task with sub-100M-param Block-Graph stack.** The current open-source state of the art is ~24%. Claude Opus 4.5 Thinking reaches 37.6% at $2.20/task. TRM (arXiv:2510.04871) achieves 44.6% on ARC-AGI-1 with 7M parameters. HRM (arXiv:2506.21734) achieves 40.3% with 27M parameters and no pretraining. These results confirm the theoretical prediction from the Lippl-Stachenfeld ceiling: architecturally compositional systems dramatically outperform scale-dependent approaches on compositional reasoning. The target validates the categorical architecture on the hardest compositional reasoning benchmark at economically viable cost.

### Round 4 Focus: Measurement

The flywheel is specified. The impossible results are cataloged. The competitive position is defensible. What remains is measurement -- turning the flywheel's qualitative promise into quantitative evidence.

Three measurement priorities:

**Synergy as training signal.** PID synergy over multi-loop traces (Section 1, Loop 6) must be implemented as a live metric, not a post-hoc analysis. When synergy rises, amplify. When it falls, diagnose. The meta-flywheel cannot compound if it is not measured.

**Cominterpretant convergence as benchmark.** The Peircean cominterpretant convergence rate (Section 2, Capability 4) must be operationalized as a reproducible benchmark. This requires defining the triadic abductive cycle precisely enough that other systems can be evaluated against it, establishing this stack's metric as the standard.

**Clade-Metaproductivity as economic primitive.** HGM's Clade-Metaproductivity (Section 2, Capability 7) must be wired into the x402 payment system so that an agent's economic value is determined by its descendant tree's productivity, not its own benchmark scores. This transforms self-improvement from a research curiosity into an economic mechanism with real-money incentives.

The flywheel is built. Now it has to spin.

---

## Summary of Citations

| Paper | Authors | Venue | Reference |
|---|---|---|---|
| Simplex BFT | Chan, Pass | IACR 2023 | 2023/463 |
| HGM (Clade-Metaproductivity) | metauto-ai | 2025 | arXiv:2510.21614 |
| Live-SWE-agent | -- | 2025 | arXiv:2511.13646 |
| SPICE (information-symmetry collapse) | Meta FAIR | 2025 | arXiv:2510.24684 |
| Self-Challenging Agents (Code-as-Task) | -- | NeurIPS 2025 | arXiv:2506.01716 |
| AlphaEvolve (RSI closure) | DeepMind | 2025 | arXiv:2506.13131 |
| DGM (Darwinian Godel Machine) | Zhang, Hu, Lu, Lange, Clune | 2025 | arXiv:2505.22954 |
| Parametric Lenses | Cruttwell, Gavranovic | 2024 | arXiv:2404.00408 |
| Categorical Deep Learning | Gavranovic, Lessard, Velickovic | ICML 2024 | arXiv:2402.15332 |
| Polynomial Functors | Niu, Spivak | Cambridge 2024 | ISBN 978-1009349987 |
| Kernel-Additivity Ceiling | Lippl, Stachenfeld | ICLR 2025 | arXiv:2405.16391 |
| EFE as Variational Inference | de Vries et al. | 2025 | arXiv:2504.14898 |
| AXIOM | Heins et al. (VERSES) | 2025 | arXiv:2505.24784 |
| V-JEPA 2 | Meta | 2025 | arXiv:2506.09985 |
| Toto (TSFM) | -- | 2024 | arXiv:2407.07874 |
| Time-MoE | -- | 2024 | arXiv:2409.16040 |
| Khan & Lowe Allostasis | Khan, Lowe | 2024 | arXiv:2406.08471 |
| Harrison et al. Allostasis | Harrison, Friston, Buckwalter | Frontiers Behav Neuro 2025 | -- |
| Metacontroller temporal abstractions | Kobayashi et al. | 2025 | arXiv:2512.20605 |
| 64-agent plateau | Dochkina, MIPT | 2026 | arXiv:2603.28990 |
| Ring vs. full debate | Li et al., Google | EMNLP 2024 | arXiv:2406.11776 |
| MacNet scaling | Qian et al. | ICLR 2025 | arXiv:2406.07155 |
| DeepMind optimal communication | Kim et al. | 2025 | arXiv:2512.08296 |
| Heterogeneity bound | Yang et al. | ICML 2026 | arXiv:2602.03794 |
| PID for MAS | Riedl, Northeastern | 2025 | arXiv:2510.05174 |
| Cellular sheaf consensus | Hansen, Ghrist | 2025 | arXiv:2504.02049 |
| MAST failure catalog | Cemri et al. | NeurIPS 2025 | arXiv:2503.13657 |
| Sybil-resistant discovery | Khalil, Gupta | 2025 | arXiv:2510.27554 |
| Sleeper Agents | Hubinger et al. | 2024 | arXiv:2401.05566 |
| Alignment Faking | Greenblatt et al. | 2024 | arXiv:2412.14093 |
| Emergent Misalignment | MacDiarmid et al. (Anthropic) | 2025 | arXiv:2511.18397 |
| Nayebi RPP Corrigibility | Nayebi | 2025 | arXiv:2507.20964 |
| Self-AIXI empowerment | -- | 2025 | arXiv:2502.15820 |
| CoT faithfulness | Anthropic | 2025 | arXiv:2505.05410 |
| Strong Model Collapse | Shumailov et al. | Nature 2024 | -- |
| Strong Model Collapse (formal) | Dohmatob et al. | ICLR 2025 | arXiv:2410.04840 |
| Accumulate, not replace | Gerstgrasser et al. | 2024 | arXiv:2404.01413 |
| TRM | Jolicoeur-Martineau et al. | 2025 | arXiv:2510.04871 |
| HRM | Wang et al. | 2025 | arXiv:2506.21734 |
| Executable Counterfactuals | Zevcevic et al. | ICLR 2026 | arXiv:2510.01539 |
| Quiet-STaR | Zelikman et al. | 2024 | arXiv:2403.09629 |
| SWE-ABS (contamination) | -- | 2026 | arXiv:2603.00520 |
| METR productivity | -- | 2025 | arXiv:2507.09089 |
| METR TH1.1 | METR | 2026 | -- |
| Gaia2 | -- | ICLR 2026 | arXiv:2509.17158 |
| Memp (procedural memory) | -- | 2025 | arXiv:2508.06433 |
| Binius (binary-field SNARK) | Irreducible | EUROCRYPT 2024 | ePrint 2023/1784 |
| Lasso + Jolt (zkVM) | Setty, Thaler et al. | EUROCRYPT 2024 | ePrint 2023/1216, 1217 |
| Worldcoin MPC biometrics | Corrigan-Gibbs et al. | 2024 | ePrint 2024/705 |
| L^2M scaling | -- | 2025 | arXiv:2503.04725 |
| PID indeterminacy | -- | 2025 | arXiv:2512.16662 |
| Stitch library learning | -- | 2023 | arXiv:2310.19791 |
| LILO library learning | -- | 2023 | arXiv:2310.19805 |
| Agent0 (tool co-evolution) | UNC + Salesforce | 2025 | arXiv:2511.16043 |
| DiscoPOP (training algorithm discovery) | Sakana AI | 2024 | arXiv:2406.08414 |
| Faith and Fate | Dziri et al. | NeurIPS 2023 | arXiv:2305.18654 |
| GSM-Symbolic | Mirzadeh et al. (Apple) | 2024 | arXiv:2410.05229 |
| ARC Prize 2025 Survey | ARC Prize Foundation | 2025 | arXiv:2601.10904 |
| DeepMind bartering pattern | -- | 2023 | arXiv:2312.03848 |
| Agora protocols | Marro et al. (Oxford) | 2024 | arXiv:2410.11905 |
| MCR binding | Angioli, Kymn, Loutfi, Kleyko | 2025 | arXiv:2511.09708 |
| Govcraft (stigmergic decay) | -- | 2026 | arXiv:2601.08129 |
| HyperDUM | -- | CVPR 2025 | arXiv:2503.20011 |
| HippoRAG 2 | Gutierrez et al. | ICML 2025 | arXiv:2502.14802 |
| AFlow (MCTS over workflows) | -- | ICLR 2025 Oral | arXiv:2410.10762 |
| GEPA (gradient-estimation prompt-tuning) | Stanford/Berkeley/Databricks/MIT | ICLR 2026 Oral | arXiv:2507.19457 |
| rStar-Math | Microsoft | ICML 2025 Oral | arXiv:2501.04519 |
| Absolute Zero Reasoner | Zhao et al. | NeurIPS 2025 Spotlight | arXiv:2505.03335 |
