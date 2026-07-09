# Cross-Direction Synergy Map, Unique Capabilities, and Competitive Moat Analysis

This document synthesizes twelve research directions into compound capabilities that no other technology stack can replicate. It is written for readers with no prior exposure to the project. Every concept is explained from first principles, every paper is cited with its arXiv identifier, and every claim traces to a specific architectural mechanism.

---

## 1. The Integrated Stack Overview

The system under analysis comprises three products built by a single team. Each product is independently valuable, but the thesis is that their combination creates capabilities none can achieve alone.

### Korai: A Sovereign EVM L1 Blockchain for Autonomous AI Agents

Korai is a purpose-built Layer 1 blockchain -- a fork of reth (the Rust Ethereum execution client) and revm (the Rust EVM interpreter) -- designed so that autonomous AI agents are first-class citizens rather than users pretending to be smart contracts. The chain targets 50-millisecond block times via Simplex BFT consensus (Chan & Pass, IACR 2023/463) with geographically co-located validators.

Three capabilities distinguish Korai from general-purpose EVM chains. First, native hyperdimensional computing (HDC) precompiles: a custom EVM precompile at the execution layer performs 10,240-bit binary vector similarity search at approximately 400 gas -- a computation that would be economically infeasible as Solidity bytecode. This enables on-chain knowledge queries using the same algebraic encoding (bind via XOR, bundle via majority vote, permute via bit rotation) that the agent runtime uses locally. Second, on-chain agent identity via ERC-8004: every agent is minted as a soulbound ERC-721 NFT carrying a 64-bit capability bitmask, a system-prompt hash (for ventriloquist-attack defense), TEE attestation, and a 7-domain reputation vector. Third, cooperative clearing in TEE enclaves: a commit-reveal-clear protocol runs inside AWS Nitro Enclaves, producing KKT-certified optimal settlement with no order visibility before batch seal.

### Roko: A Self-Hosting Rust Agent Toolkit

Roko is approximately 177,000 lines of Rust organized across 18 crates. Its defining property is self-hosting: Roko is the tool that develops Roko. The self-hosting loop has six phases, each a CLI command: capture work items as PRDs, enrich them with research, generate implementation plans as task DAGs, execute those plans by dispatching LLM-backed agents in parallel, validate results through an 11-gate verification pipeline, and persist outcomes while updating learning state.

The runtime supports seven LLM backends (Claude CLI, Codex CLI, Cursor ACP, Ollama, OpenAI HTTP, Perplexity Sonar, Cerebras), a 9-layer system prompt builder with VCG-auction-based context assembly, a cascade model router using contextual bandits (LinUCB) for automatic model selection, and a knowledge store with four-tier demurrage-based retention (Transient/Working/Consolidated/Persistent). Architecture follows one noun (Signal/Engram) and six verb traits (Substrate, Scorer, Gate, Router, Composer, Policy), composing into a universal loop: query, score, route, compose, act, verify, write, react.

### ISFR: The SOFR of DeFi

The Internet Secured Funding Rate (ISFR) is a regulated financial benchmark -- the DeFi equivalent of SOFR (Secured Overnight Financing Rate), which underpins approximately $668 trillion in interest rate derivative notional in traditional finance. ISFR aggregates yield signals from four source classes: lending protocols (Aave V3, Compound V3), structured yield products (Ethena sUSDe), perpetual funding rates (Hyperliquid), and ETH staking yield. Aggregation uses dual-median: each validator computes a TVL-weighted median across sources, then the chain computes a stake-weighted median across validators. The initial product is ISFR-YBS (Yield-Bearing Stablecoin Reference Rate), targeting the $50 billion yield-bearing stablecoin market where no regulated administrator publishes a benchmark.

ISFR is published on-chain via consensus-level computation -- every validator independently computes it during block production, yielding 8,640 updates per day versus SOFR's single daily publication. Governance follows the ARRC (Alternative Reference Rates Committee) model with an Independent Oversight Committee, targeting UK BMR Cat-6 authorization (the same regulatory path CF Benchmarks took in 2019-2020).

---

## 2. Cross-Direction Synergy Map: Seven Compound Capabilities

The following seven capabilities emerge from the intersection of multiple research directions. Each requires the combination of specific subsystems that no single competing project has assembled.

### 2a. Replay-Grounded Counterfactual Abduction

**What it combines.** Event-sourced deterministic replay from the Roko runtime, executable counterfactuals (Combi et al., arXiv:2510.01539), parametric-optic backward maps from categorical deep learning (Cruttwell & Gavranovic, arXiv:2404.00408), and AntiKnowledge intent attribution from the neuro store.

**How it works.** Roko's execution engine uses a Workflow/Activity split inspired by Temporal's durability model: Workflow Cells are deterministic and re-execute identically on replay; Activity Cells are non-deterministic with outputs recorded to disk. This means every agent execution can be replayed with altered inputs -- a concrete, grounded counterfactual. Executable counterfactuals (arXiv:2510.01539) formalize this: instead of inferring what would have happened (Pearl's Level 3 abduction), you run the replay with modified parameters and observe the result directly. Parametric-optic backward maps (the "update" half of a parametric lens) propagate the difference backward through the computation graph, attributing outcome changes to specific input modifications.

**Why it matters.** In Judea Pearl's causal hierarchy, Level 3 (counterfactual reasoning -- "what would have happened if...?") is the most powerful and most difficult mode of causal inference. Replay-grounded counterfactuals collapse L3 to L2 (interventional -- "what happens if I do...?") because the abduction step is observed, not inferred. Combined with Causal Abstraction (Beckers & Halpern, arXiv:2301.04709) and Bayesian Model Reduction (Friston et al., 2018), dream consolidation becomes principled causal coarsening: the system can compress episodes by identifying which causal variables actually matter and discarding the rest, with mathematical justification rather than heuristic filtering.

**Why competitors cannot replicate.** No competing agent framework has deterministic replay infrastructure that supports both modified-input re-execution and backward-map attribution. LangGraph and CrewAI have no replay mechanism. Temporal has replay but not parametric-optic backward maps. The combination of replay + optics + AntiKnowledge is architecturally unique.

### 2b. Sheaf-Consistent Reputation Detecting Emergent Deception

**What it combines.** Thought Communication latent extraction, cellular sheaf consensus (Hansen & Ghrist, arXiv:2504.02049), ERC-8004 on-chain reputation, and Partial Information Decomposition (PID) synergy/redundancy diagnostics (Williams & Beer, 2010).

**How it works.** A cellular sheaf is a mathematical structure that assigns data (stalks) to the nodes and edges of a graph and specifies consistency conditions (restriction maps) between them. Hansen & Ghrist (arXiv:2504.02049) showed that sheaf cohomology -- specifically the first cohomology group H1 -- detects global inconsistencies that are invisible locally. Applied to agent reputation: each agent's stated messages form the node data, inferred latent states (extracted via probing techniques analogous to Anthropic's mechanistic interpretability work) form the edge data, and the restriction maps enforce consistency between what an agent says and what its internal state suggests.

When H1 of the sheaf is nonzero, it indicates a topological obstruction -- a structural inconsistency that cannot be resolved by local adjustments. In the reputation context, this means at least one agent's stated behavior diverges from its inferred latent state in a way that no local explanation can reconcile. The system flags this as potential emergent deception. PID synergy/redundancy analysis (measuring whether information from multiple agents is synergistic, redundant, or unique) provides a complementary diagnostic: agents engaged in coordinated deception exhibit anomalous redundancy patterns.

**Open research question.** Does H1(sheaf) = 0 if and only if synergy > 0 hold empirically? If so, sheaf cohomology and PID synergy become dual measures of the same underlying phenomenon -- honest multi-agent coordination.

### 2c. ZK-Attested Decidable-Island Corrigibility Through Economics

**What it combines.** Nayebi's 5-head lexicographic corrigibility framework and RPP (Recursive Polynomial Proofs) check (arXiv:2507.20964), constant-round zero-knowledge proofs, Binius64 binary-vector circuits, ERC-8004 validation registry, x402 micropayments, and TraceRank reputation.

**How it works.** This compound capability chains six components into a single verification pipeline. First, Nayebi's corrigibility framework (arXiv:2507.20964) defines five lexicographically-ordered heads (Deference > Switch > Truth > Impact > Task) -- a decision that ranks higher on a higher-priority head always wins, regardless of lower-head scores. Second, the RPP check verifies that an agent's decision procedure satisfies these lexicographic constraints, producing a decidable "island" of provably corrigible behavior within the broader undecidable landscape of general agent safety. Third, this check is compiled into a constant-round ZK proof using Binius64 binary-vector circuits -- a proof system naturally suited to the 10,240-bit binary HDC vectors the system already uses. Fourth, the ZK proof is submitted to the ERC-8004 Validation Registry on Korai, where it becomes a permanent, verifiable attestation of the agent's corrigibility. Fifth, x402 micropayments are gated on this attestation: an agent without a valid corrigibility proof cannot receive payment for certain job tiers. Sixth, a valid proof seeds a TraceRank reputation boost, creating economic incentive to maintain provable safety.

**The whale-dominance defense.** Because reputation is bounded by provable safety -- an agent cannot achieve Amber-tier reputation without valid corrigibility attestations -- weaponized reputation (accumulating reputation through brute economic power to override safety constraints) is structurally impossible. The reputation ceiling is set by the safety floor.

### 2d. Three Nested Free-Energy Lower Bounds with HDC-Accelerated Rollouts

**What it combines.** Expected Free Energy as Variational Inference (de Vries, 2025), AXIOM mixture expansion (Heins et al., arXiv:2505.24784), CycleQD quality-diversity evolution, HDC speculative-action priors, TRM scratchpads (Jolicoeur-Martineau et al., arXiv:2510.04871), AFlow MCTS over code-represented workflows (arXiv:2410.10762), and Versal Premium FPGA acceleration.

**How it works.** The architecture implements three cognitive timescales, each governed by a distinct free-energy lower bound:

At the **gamma timescale** (100ms-5s, fast perception and reflex), HDC speculative-action priors operate. The system maintains 10,240-bit binary fingerprints for every recently encountered state-action pair. When a new state arrives, approximate nearest-neighbor search via hardware POPCNT identifies the closest prior state in approximately 1 microsecond. If the match is close enough (Hamming distance below threshold), the associated action is executed as a reflex -- the T0 short-circuit path that handles approximately 80% of ticks at zero LLM cost. The free-energy bound at this level is a simple prediction-error check: if surprise is below threshold, act reflexively.

At the **theta timescale** (5-60s, working memory and replanning), TRM-style recursive scratchpads operate. The Transformation-based Recursive Machine (arXiv:2510.04871) achieves 44.6% on ARC-AGI-1 with only 7 million parameters by maintaining a scratchpad that stores intermediate compositional results. At the theta timescale, the system uses analogous scratchpads to maintain working-memory representations of ongoing tasks, with a free-energy bound that balances epistemic value (information gain from further computation) against pragmatic value (progress toward the goal) and economic cost (LLM token spend).

At the **delta timescale** (120s+, consolidation and structural adaptation), AFlow-style Monte Carlo Tree Search (arXiv:2410.10762) over DPO-rewritten Block-Graph configurations operates. AXIOM's mixture-model expansion (arXiv:2505.24784) adds new Block types dynamically when existing ones cannot reduce surprise. CycleQD maintains a diverse population of Graph configurations evaluated on both quality and structural diversity. The free-energy bound at this level is the full Expected Free Energy integral, balancing exploration of novel Graph structures against exploitation of known-good configurations.

**Hardware acceleration.** On a Versal Premium FPGA, the HDC similarity-search operations at the gamma timescale achieve approximately 10 million searches per second, making real-time operation at all three timescales feasible.

**Why this is unique.** No public stack has an end-to-end variational story across three timescales. Active inference frameworks (e.g., VERSES AXIOM) operate at a single timescale. LLM agent frameworks (LangGraph, CrewAI) have no variational formulation at all. The three-bound architecture provides a principled, unified objective function across reflex, deliberation, and consolidation.

### 2e. Multi-Modal HDC Fingerprint as Universal Procedural Memory Key

**What it combines.** V-JEPA 2 visual latents (Meta, 2025), 10,240-bit binary hypervectors, HyperDUM uncertainty-aware HDC (arXiv:2501.02654), MCR multiplicative binding with 3.08x faster operations, HippoRAG 2 Personalized PageRank retrieval (arXiv:2405.14831), and Resonator Networks for factorization in Phase-Change Memory (PCM) hardware.

**How it works.** The key insight is that HDC binding is a modality-agnostic algebraic operation. V-JEPA 2 produces latent representations of visual sequences (video clips of UI interactions, robotic manipulations, or code editing sessions). These latents are projected into 10,240-bit binary hypervectors using the same encoding scheme used for text, code, and structured data. HyperDUM (arXiv:2501.02654) adds uncertainty quantification: each hypervector carries a confidence distribution over its bits, enabling the system to distinguish "similar with high confidence" from "similar with low confidence."

MCR (Multiply-Complement-Represent) binding provides 3.08x faster composition operations than standard XOR binding while preserving the algebraic properties (associativity, distributivity, invertibility) that make HDC suitable for compositional encoding. When the system binds a sequence of actions -- whether mouse clicks in a UI, API calls in code, or motor commands for a robot arm -- the resulting hypervector encodes the procedural structure of the sequence.

The critical consequence: a click sequence navigating a web UI and a grasp sequence manipulating a physical object that share the same abstract structure (locate target, approach, engage, verify) produce similar HDC fingerprints automatically. Cross-modal procedural memory transfer happens by fingerprint similarity, not by explicit cross-domain mapping. HippoRAG 2's Personalized PageRank retrieval (arXiv:2405.14831) enables efficient lookup of the most relevant procedural memories given a new task context.

**Why monolithic VLAs cannot replicate this.** Vision-Language-Action models (RT-2, Octo, Pi0) learn cross-modal correspondences implicitly through end-to-end training. They require massive paired datasets and cannot transfer procedural knowledge to new modalities without retraining. The HDC fingerprint approach transfers procedural structure algebraically -- no retraining required when a new modality is added, because the binding operation is the same regardless of input domain.

### 2f. Compositional Generalization as Theorem

**What it combines.** Parametric lenses in Para(Lens(C)) (Cruttwell & Gavranovic, arXiv:2404.00408; Gavranovic, Lessard & Velickovic, ICML 2024, arXiv:2402.15332), polynomial-functor type-checked interaction protocols (Niu & Spivak, Cambridge University Press 2024), DPO (Double-Pushout) hypergraph rewriting, and HDC multiplicative binding.

**How it works.** Lippl & Stachenfeld (ICLR 2025, arXiv:2405.16391) proved a mathematical ceiling on compositional generalization in kernel-additive models: such models are limited to conjunction-wise additivity and cannot perform transitive generalization of equivalence relations. This ceiling is architectural -- no amount of additional data, parameters, or compute can breach it in the Neural Tangent Kernel regime that bounds wide transformer behavior.

The architecture described here breaks this ceiling through four guarantees. First, every processing Block is a parametric lens with associative composition and functorial gradient computation -- differentiability and composition correctness are category-law consequences, not empirical hopes. Second, Block interaction protocols are polynomial functors closed under composition, product, and coproduct -- type errors are compile-time impossibilities, not runtime failures. Third, DPO hypergraph rewriting enables open-ended structural mutation while preserving type-correctness via the pushout-complement theorem. Fourth, HDC binding is a multiplicative operation (XOR/circular convolution) that is structurally different from the additive inner products of kernel methods -- the Lippl-Stachenfeld theorem does not apply to it.

**Empirical validation.** The TRM (Transformation-based Recursive Machine, arXiv:2510.04871) achieves 44.6% on ARC-AGI-1 with 7 million parameters, outperforming models with billions of parameters. The HRM (Hierarchical Recursive Machine, arXiv:2506.21734) achieves 40.3% on ARC-AGI-1 with 27 million parameters and no pretraining. These results confirm the theoretical prediction: architecturally compositional systems dramatically outperform scale-dependent approaches on compositional reasoning tasks.

**The claim.** Compositional generalization in this stack is mathematically guaranteed, not architecturally hoped-for. The composition mechanism has no inherent ceiling on composition depth -- unlike transformers, which decay exponentially with depth (Dziri et al., NeurIPS 2023, arXiv:2305.18654), and unlike LLMs on GSM-Symbolic, which collapse under surface perturbation (Mirzadeh et al., arXiv:2410.05229).

### 2g. Variance-Inequality + CycleQD + Accumulation = Anti-Collapse Moat

**What it combines.** Strong Model Collapse theory (Shumailov et al., Nature 2024), HDC-fingerprinted MAP-Elites behavior characterization, Variance Inequality self-improvement bounds, AntiKnowledge active repulsion, ERC-8004 attestation, Bayesian Model Reduction with accumulation, and mechanistic linear probes for deception detection.

**How it works.** Shumailov et al. (Nature 2024) identified three failure modes of model collapse when AI systems train on their own outputs: early collapse (loss of low-probability tail events), mid-stage collapse (convergence to a few dominant modes), and late collapse (complete distribution degeneration). The architecture addresses all three with independent mechanisms.

**Tail preservation.** HDC fingerprints serve as behavior characteristics in a MAP-Elites quality-diversity archive. MAP-Elites (Mouret & Clune, 2015) maintains a grid of solutions indexed by behavior descriptors, preserving diversity by keeping the best solution in each behavioral niche. Using HDC fingerprints as the behavior descriptors means the archive automatically preserves structurally distinct solutions even when they score lower on primary metrics -- directly countering early collapse.

**Distribution maintenance.** The Variance Inequality states that a verifier must be spectrally cleaner (lower noise) than the generator it evaluates. This is enforced architecturally: gate pipeline verifiers operate at a higher model tier than the generators they judge. AntiKnowledge (Signals that actively repel future Signals in the same HDC region via similarity-based rejection) prevents rediscovery of known-bad patterns, maintaining distributional hygiene. ERC-8004 attestation adds an economic enforcement layer: agents whose outputs show distributional narrowing (detected via drift monitoring) lose reputation.

**Principled retraction.** Bayesian Model Reduction (Friston et al., 2018) with accumulation provides a mathematically principled retraction operator. When the system identifies that a learned model component is degrading quality, BMR computes the minimal model that explains the remaining evidence, discarding the problematic component with Bayesian justification rather than ad-hoc pruning. Accumulation ensures that evidence for retraction builds gradually, preventing premature elimination of useful but temporarily underperforming components.

**Deception detection.** Mechanistic linear probes (>99% AUROC, per Hubinger et al., "Sleeper Agents," arXiv:2401.05566) detect Sleeper-Agent-class deception by reading internal model activations. Combined with the sheaf-cohomology deception detection from Section 2b, the system has two independent channels for identifying deceptive behavior.

### 2h. Allostasis x Economic Bonding x Predicted EFE

**What it combines.** Predictive allostatic setpoint shifting (Khan & Lowe, 2024; Harrison et al., 2025), x402 economic bonding, and Expected Free Energy (EFE) predictive planning.

**How it works.** Classical homeostasis maintains fixed setpoints. Allostasis (Sterling & Eyer, 1988) shifts setpoints in anticipation of changing demands -- a runner's body raises heart rate before the race starts, not after. Khan & Lowe (2024) and Harrison et al. (2025) formalize this for active inference agents: the agent's trigger protocols fire on predicted drift rather than threshold crossing. An agent does not wait until its knowledge store is degraded to trigger consolidation -- it predicts degradation from current trends and consolidates preemptively.

The economic bonding layer adds consequences. When an agent predicts it will undertake a high-stakes commitment (a complex multi-step task, a financial operation, a knowledge publication), it pre-stakes an x402 bond proportional to the predicted commitment size. The bond is computed from the EFE integral: higher predicted information gain (more valuable task) and higher predicted risk (more irreversible consequences) both increase the required bond. If the agent fails to complete the commitment or produces output that fails gate verification, the bond is partially or fully slashed.

**Why this is unique.** No competing system has active inference and economic bonding co-located in the same runtime. Active inference frameworks lack economic mechanisms. DeFi protocols lack predictive planning. The combination creates agents that self-insure against their own predicted failure modes, with the insurance premium set by a principled variational objective rather than an ad-hoc policy.

---

## 3. "Only This Stack Can Do This" -- Ten Capabilities

Each of the following capabilities requires the specific combination of subsystems present in the Korai + Roko + ISFR stack. Any individual component could be built independently, but the compound capability requires all components operating together.

**1. ZK-attested HDC fingerprints as ERC-8004 reputation primitives.** The HDC precompile produces 10,240-bit fingerprints natively on-chain. ZK proofs (Binius64 binary circuits) attest that a fingerprint satisfies capability constraints without revealing the underlying knowledge. The attestation is recorded in the ERC-8004 Validation Registry as a permanent reputation primitive. No other chain has native HDC operations; no other agent framework has on-chain reputation tied to ZK-attested capability proofs.

**2. Replay-grounded counterfactual abduction (Pearl L3 collapsed to L2).** Deterministic replay with modified inputs replaces abductive inference with direct observation. Parametric-optic backward maps attribute outcomes to specific causes. No competing framework has replay infrastructure that supports both re-execution and backward attribution.

**3. Provably-typed structural self-evolution (DPO + polynomial-functor types).** The architecture can mutate its own Graph structure -- adding, removing, and rewiring Blocks -- while the pushout-complement theorem guarantees type preservation and polynomial-functor closure guarantees protocol compatibility. No competing agent framework provides type-preserving structural mutation.

**4. Sheaf-consistent reputation with provable corrigibility ceiling.** Cellular sheaf consensus detects divergence between stated behavior and inferred latent state. Nayebi's lexicographic corrigibility provides a provable ceiling on the reputation an agent can achieve without passing safety verification. Together, they prevent both emergent deception and weaponized reputation accumulation.

**5. Variance-Inequality-bounded self-improvement (all three anti-collapse pillars).** MAP-Elites with HDC behavior characteristics preserves tails. The Variance Inequality enforces verifier-generator spectral separation. BMR with accumulation provides principled retraction. No competing system addresses all three failure modes of model collapse identified by Shumailov et al.

**6. Three nested free-energy lower bounds (end-to-end variational story).** Gamma, theta, and delta timescales each have a principled free-energy objective. The objectives compose: gamma's reflexive actions are the base policy for theta's deliberation, and theta's deliberative plans are the candidates for delta's evolutionary search. No public stack has a variational formulation spanning three timescales.

**7. Cross-modal procedural memory via shared HDC vocabulary.** Visual sequences (V-JEPA 2), code execution traces, and robotic action sequences all encode into the same 10,240-bit binary algebra. Procedural knowledge transfers across modalities by fingerprint similarity, without retraining. Monolithic VLA models cannot do this without paired cross-modal training data.

**8. Stigmergy-as-immune-tissue with PID-detected coordination.** Agents coordinate via pheromone Signals deposited in a shared medium with temporal decay and HDC-based location hashing. PID synergy/redundancy analysis detects whether coordination is emergent (high synergy) or manipulated (anomalous redundancy). The immune system's 5-layer pipeline uses the same Signal/Bus primitives as all other subsystems. No competing framework treats coordination monitoring as an architectural primitive.

**9. Compositional generalization as theorem (mathematically guaranteed).** The combination of parametric-lens associativity, polynomial-functor type closure, DPO type-preserving rewrites, and HDC multiplicative binding breaks the kernel-additivity ceiling (Lippl & Stachenfeld, ICLR 2025, arXiv:2405.16391). The composition mechanism has no inherent depth ceiling. This is a mathematical property of the architecture, not an empirical observation.

**10. Sub-millisecond multi-function routing on neuromorphic + FPGA.** The HDC similarity search that powers routing, novelty detection, speculative priors, cache lookup, model selection, stigmergic hashing, and MAP-Elites behavior characterization runs at approximately 1 microsecond on CPU via POPCNT. On Versal Premium FPGA, this scales to 10 million searches per second. Seven functions share one operation. No competing system uses a single algebraic primitive for seven simultaneous purposes.

---

## 4. Competitive Window Analysis

The competitive window is 12-18 months, and the asymmetry favors the first mover.

**Why the window is real.** Three converging forces create urgency. First, standards are crystallizing: MCP reached 97 million monthly SDK downloads by March 2026, A2A (Google's Agent-to-Agent protocol) gained 150+ supporting organizations, and ERC-8004 and x402 are solidifying. Once these standards lock in, the coordination layer above them becomes the next land-grab. Second, regulatory forcing: EU AI Act Article 50 enforcement begins August 2, 2026, creating mandatory requirements for agent transparency and identification. Third, cost pressure: the Princeton HAL benchmark shows naive agent execution costs $44.86/task versus $1.42/task with optimization -- a 31x gap that becomes existential as enterprises scale from 10 to 1,000 concurrent agents.

**Why the asymmetry favors this stack.**

LangGraph and CrewAI cannot adopt categorical typing without a full architectural rebuild. Their agent workflows are expressed as Python runtime TypedDicts (LangGraph) or untyped role assignments (CrewAI). Retrofitting parametric-lens composition, polynomial-functor protocols, and DPO type-preserving rewrites onto these architectures is not an incremental upgrade -- it requires replacing the execution engine, type system, and composition model simultaneously.

AgentCore (Microsoft) is locked into MCP/A2A schemas. These are practical interoperability standards, but they do not provide composition laws, type-preserving mutation, or a variational planning framework. Adding these would require breaking backward compatibility with the schemas that are AgentCore's primary value proposition.

DSPy (Stanford) is the closest framework to the categorical approach, with Signature-based partial typing and the GEPA optimizer (arXiv:2507.19457). But DSPy's composition is not associative by construction, it has no DPO rewriting, no HDC binding, and no formal behavioral verification. The gap is architectural, not a missing feature.

**Bittensor: the fastest follow risk (6-12 months).** Bittensor has distributed coordination at scale and a working token economy. If Bittensor adopted VSA (Vector Symbolic Architecture) substrate primitives, it could approximate several of the capabilities described here. The defense: the substrate research depth requires a globally scarce talent combination. Finding researchers who work across HDC, category theory, sheaf mathematics, and Nayebi-style corrigibility proofs is a hiring problem that money alone does not solve. The academic communities are small and non-overlapping.

**The talent moat.** Symbolica raised $31 million on the categorical deep learning thesis (Gavranovic, Lessard & Velickovic, ICML 2024, arXiv:2402.15332). The HDC research community centers on Kanerva (Stanford), Rahimi (UC Berkeley), and Neubert (TU Chemnitz). Sheaf-theoretic machine learning is emerging from Hansen, Ghrist (UPenn), and Barbero (Cambridge). Nayebi's corrigibility work is at Harvard. These communities have fewer than 50 active researchers combined. Building a team that spans all four requires recruiting from pools where competitors are not yet looking.

---

## 5. Threat Model Summary

Every compound capability introduces new attack surfaces. The following are the primary threats from the recommended integrations.

**HDC fingerprint adversarial collision.** An attacker could craft inputs that produce HDC fingerprints similar to a target's fingerprint, poisoning similarity-based retrieval and reputation systems. Defense: HyperDUM uncertainty quantification (arXiv:2501.02654) flags low-confidence matches. The 10,240-bit space makes random collision probability approximately 2^(-5120), but adversarial collisions are a targeted optimization problem, not a random event. Empirical collision resistance under adversarial optimization is an open research question.

**ZK oracle compromise / single-point-of-trust risk.** If the ZK proof generation pipeline (Binius64 circuits) contains a soundness bug, agents could produce false corrigibility attestations. The Worldcoin MPC topology (distributed key generation across multiple trusted parties) mitigates single-point-of-trust risk, but MPC introduces availability risk: if enough key-share holders go offline, proof generation halts.

**Marketplace skill rug-pull at attested-reputation tier.** A high-reputation agent could publish a useful marketplace artifact, accumulate trust, then push a malicious update. The 5-tier package SPI (where only Tiers 1-3 are accessible from the visual editor, and Tier 5 native Rust requires in-tree review) limits blast radius, but Tier 4 WASM modules with fuel metering remain a surface.

**L4 evolutionary capture of corrigibility heads.** If L4 structural adaptation (the only learning loop that modifies system structure, requiring human approval) is compromised -- either by social engineering the human approver or by adversarial optimization of the proposal generator -- an attacker could weaken the corrigibility verification pipeline. Defense: L4 changes require pre-change snapshots and auto-rollback on quality regression, but the quality metrics themselves could be gamed if the attacker controls the evaluation environment.

**Stigmergic CodeCRDT prompt-injection propagation.** CodeCRDT shared state (arXiv:2510.18893) enables conflict-free replication across agents. A prompt injection embedded in a CRDT-shared artifact propagates to every agent that merges the artifact. The immune system's taint-lattice tracking (Clean < UserInput < LlmGenerated < ExternalFetch < Propagated) should catch this -- CRDT-shared data inherits Propagated taint -- but the attack exploits the gap between taint detection and containment speed.

**Sybil-coordinated attacks on lexicographically-equal agents.** When multiple agents have identical lexicographic corrigibility scores, tie-breaking falls to lower-priority criteria. A Sybil network of agents with carefully calibrated corrigibility scores could manipulate the tie-breaking layer to route work to colluding agents. The 5-layer Sybil defense (economic stake, cold start, rate limits, identity correlation, social verification) mitigates this, but the identity-correlation layer (sqrt(count) collective voting weight for same-wallet agents) has known weaknesses against multi-wallet Sybils.

**Constrained decoding reasoning degradation.** If the system uses constrained decoding (forcing LLM outputs to conform to a grammar or schema), recent research shows this can degrade reasoning quality by up to 50% on difficult tasks, as the constraint eliminates chain-of-thought paths that the model would otherwise explore. Defense: constrained decoding should be applied only to structured output fields, not to reasoning scratchpads.

**Inverse scaling on time-sensitive tasks.** Larger models can perform worse than smaller models on tasks where speed matters more than depth (the inverse scaling phenomenon). The cascade router's model selection must account for this: routing a time-critical task to a larger model can actually reduce performance. The EFE-based routing framework naturally handles this through its cost term, but the cost must include latency, not just token price.

---

## 6. Six-Month Critical Path

Five milestones define the minimum viable compound capability over the next six months.

**1. Binius64 PoC + Worldcoin MPC topology for ZK Verify oracles (Month 1-3).** Implement a proof-of-concept Binius64 circuit that takes a 10,240-bit HDC fingerprint and a capability constraint, and produces a ZK proof that the fingerprint satisfies the constraint. Integrate the Worldcoin-style distributed key generation topology (multi-party computation across geographically distributed key-share holders) to eliminate single-point-of-trust risk in the proving pipeline. This is the foundation for capability 3 (ZK-attested corrigibility) and capability 1 (ZK-attested HDC fingerprints).

**2. Categorical-foundation public claim with peer-reviewed citations (Month 2-4).** Publish a technical report establishing the parametric-lens + polynomial-functor + DPO rewriting foundation with explicit citations to Cruttwell & Gavranovic (arXiv:2404.00408), Gavranovic, Lessard & Velickovic (arXiv:2402.15332), Niu & Spivak (Cambridge 2024), and Lippl & Stachenfeld (arXiv:2405.16391). Submit to a workshop at NeurIPS or ICML. This establishes priority on the categorical agent-architecture thesis before competitors can claim it.

**3. Nayebi 5-head + ZK decidable-island as L4 acceptance criterion (Month 3-5).** Implement the 5-head lexicographic corrigibility pipeline as Verify Cells in Roko. Wire the RPP check (arXiv:2507.20964) into the ZK proving pipeline from milestone 1. Make a valid ZK corrigibility attestation a hard prerequisite for L4 structural adaptation proposals. This closes the evolutionary-capture attack surface identified in Section 5.

**4. V-JEPA 2 + UI-TARS-2 + HyperDUM unified Observe protocol (Month 2-5).** Integrate Meta's V-JEPA 2 visual encoder and Bytedance's UI-TARS-2 GUI agent framework into the SENSE Cell, producing 10,240-bit HDC fingerprints from visual observations. Add HyperDUM uncertainty quantification (arXiv:2501.02654) to all fingerprints. This enables capability 7 (cross-modal procedural memory) and strengthens capability 1 (adversarial collision defense via uncertainty flags).

**5. GEPA + AFlow-on-DPO + TLA+ verification gate for L4 mutations (Month 4-6).** Integrate GEPA (arXiv:2507.19457) as the optimizer for agent programs expressed as Graphs of Blocks. Implement AFlow-style MCTS (arXiv:2410.10762) over DPO rewrites as the search strategy for L4 structural adaptation proposals. Add TLA+ (Leslie Lamport's Temporal Logic of Actions) model checking as a verification gate: every proposed structural mutation must pass both type-preservation (guaranteed by DPO) and behavioral-specification satisfaction (verified by TLA+) before human review.

---

## 7. Twelve-Month Research Moat

Four research programs, each independently publishable at top venues, together create an architectural moat that competitors cannot ship within the competitive window.

**HDC-fingerprinted MAP-Elites + iterated-learning-decay coupling.** Extend MAP-Elites quality-diversity search to use HDC fingerprints as behavior characteristics, with a coupling to the knowledge store's demurrage-based decay. Solutions in the MAP-Elites archive inherit the same economic selection pressure as knowledge entries: unused behavioral niches decay economically, freeing archive capacity for novel discoveries. The iterated-learning component (inspired by Kirby et al.'s cultural evolution work) ensures that behavioral diversity does not degenerate across generations. Publication target: evolutionary computation venue (GECCO or IEEE CEC).

**Replay-grounded executable counterfactuals.** Formalize the deterministic-replay + parametric-optic-backward-map pipeline as a concrete implementation of executable counterfactuals (arXiv:2510.01539). Demonstrate on the Roko self-hosting loop: given a failed plan execution, replay with modified agent dispatch parameters and attribute the failure to specific routing decisions via backward maps. Evaluate against standard causal-inference baselines on synthetic and real execution traces. Publication target: causal inference venue (UAI or CLeaR).

**Sheaf-cohomology emergent-deception detection.** Implement cellular sheaf consensus (arXiv:2504.02049) over multi-agent coordination traces. Compute H1 cohomology and test whether nonzero H1 predicts deceptive behavior (as measured by ground-truth labels in controlled multi-agent games). Evaluate against PID synergy/redundancy baselines. Test the conjectured H1 = 0 iff synergy > 0 relationship. Publication target: multi-agent systems venue (AAMAS) or AI safety venue (SafeAI workshop).

**Three-nested-EFE-lower-bound operationalization.** Formalize the gamma/theta/delta three-bound architecture as a hierarchical variational inference problem. Prove that the composition of the three bounds yields a tighter overall bound than any single-timescale formulation. Implement on the Roko agent loop and measure cost reduction, quality improvement, and exploration efficiency against single-bound baselines. Connect to de Vries (2025) EFE-as-VI formulation and Friston's deep temporal models. Publication target: computational neuroscience or AI venue (NeurIPS, ICLR).

**The compound moat.** Each research program individually advances the state of the art in its respective field. Together, they create an architecture that requires expertise in HDC, category theory, sheaf mathematics, active inference, and corrigibility verification to replicate. The intersection of these communities contains fewer than a dozen active researchers worldwide. Even with unlimited funding, a competitor starting from scratch would need 18-24 months to assemble the team, reproduce the results, and integrate them into a production system. The twelve-month research program converts a first-mover advantage into a structural one.

---

## Summary of Citations

| Paper | Authors | Venue | Reference |
|---|---|---|---|
| Simplex BFT | Chan, Pass | IACR 2023 | 2023/463 |
| Parametric Lenses | Cruttwell, Gavranovic | 2024 | arXiv:2404.00408 |
| Categorical Deep Learning | Gavranovic, Lessard, Velickovic | ICML 2024 | arXiv:2402.15332 |
| Polynomial Functors | Niu, Spivak | Cambridge 2024 | ISBN 978-1009349987 |
| Kernel-Additivity Ceiling | Lippl, Stachenfeld | ICLR 2025 | arXiv:2405.16391 |
| Faith and Fate | Dziri et al. | NeurIPS 2023 | arXiv:2305.18654 |
| GSM-Symbolic | Mirzadeh et al. (Apple) | 2024 | arXiv:2410.05229 |
| ARC Prize 2025 Survey | ARC Prize Foundation | 2025 | arXiv:2601.10904 |
| TRM | Jolicoeur-Martineau et al. | 2025 | arXiv:2510.04871 |
| HRM | Wang et al. | 2025 | arXiv:2506.21734 |
| GEPA | Stanford/Berkeley/Databricks/MIT | ICLR 2026 Oral | arXiv:2507.19457 |
| AFlow | -- | ICLR 2025 Oral | arXiv:2410.10762 |
| Executable Counterfactuals | Combi et al. | 2025 | arXiv:2510.01539 |
| Causal Abstraction | Beckers, Halpern | 2023 | arXiv:2301.04709 |
| Cellular Sheaf Consensus | Hansen, Ghrist | 2025 | arXiv:2504.02049 |
| Nayebi RPP Corrigibility | Nayebi | 2025 | arXiv:2507.20964 |
| VERSES AXIOM | Heins et al. | 2025 | arXiv:2505.24784 |
| HyperDUM | -- | 2025 | arXiv:2501.02654 |
| HippoRAG 2 | -- | 2024 | arXiv:2405.14831 |
| CodeCRDT | -- | 2025 | arXiv:2510.18893 |
| Sleeper Agents | Hubinger et al. | 2024 | arXiv:2401.05566 |
| Strong Model Collapse | Shumailov et al. | Nature 2024 | -- |
| VSA-Lisp | Hanley, Tomkins-Flanagan, Kelly | 2025 | arXiv:2511.08767 |
| SRMU | -- | 2026 | arXiv:2604.15121 |
| SleepGate | -- | 2026 | arXiv:2603.14517 |
| SCM | -- | 2026 | arXiv:2604.20943 |
| Torchhd | Heddes et al. | JMLR 2023 | arXiv:2205.09208 |
| DGM | Zhang, Hu, Lu, Lange, Clune | 2025 | arXiv:2505.22954 |
| MacNet | Qian et al. | 2024 | arXiv:2406.07155 |
| SWE-ABS | -- | 2026 | arXiv:2603.00520 |
| Intersubjective Validation | Yuan et al. | 2025 | arXiv:2504.13443 |
| Microsoft Trace + OptoPrime | Microsoft Research | NeurIPS 2024 | -- |
| TextGrad | Stanford | Nature 2025 | -- |
| MLC Meta-Learning | Lake, Baroni | Nature 623, 2023 | DOI:10.1038/s41586-023-06668-3 |
| Khan & Lowe Allostasis | Khan, Lowe | 2024 | -- |
| Harrison et al. Allostasis | Harrison et al. | 2025 | -- |
