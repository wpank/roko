# Spec Layer Rewrite Prompt

This file instructs Claude on how to rewrite the 22 spec documents in this directory **from scratch**. The current docs were written with assumptions from prior versions and need a clean rewrite from the perspective of someone encountering them with zero prior context. Copy everything below the `---` into a new session.

## What this directory is

`tmp/unified/` is the **spec layer** — 22 documents defining the vocabulary, protocols, contracts, and composition rules for **Nunchi** (the project) and **Roko** (the agent runtime). "Nunchi" is the canonical name for both the overall project and the Nunchi blockchain. "Roko" is the agent runtime crate ecosystem.

These docs are:
- **Authoritative**: everything else references these
- **Self-contained**: a reader with no prior context understands the full system
- **Concise**: protocols and types, not algorithms or implementation detail
- **Elegant**: the minimum number of primitives that compose to express everything
- **Protocol-grade**: reads like an RFC that third parties adopt because not adopting is more expensive

The **depth layer** lives in `tmp/unified-depth/` — algorithms, research backing, domain-specific patterns, and implementation detail. Don't put depth content here. If it has an arXiv citation, a specific benchmark number, or a code implementation — it goes in depth.

The **learnings** at `tmp/learnings/` provide self-contained briefings (architecture, research synthesis, strategy, implementation, risks) that summarize everything for new sessions or team members.

---

## Phase 1: Read all 22 spec documents

Read in order:

1. `/Users/will/dev/nunchi/roko/roko/tmp/unified/00-INDEX.md`
2. `/Users/will/dev/nunchi/roko/roko/tmp/unified/01-SIGNAL.md`
3. `/Users/will/dev/nunchi/roko/roko/tmp/unified/02-BLOCK.md`
4. `/Users/will/dev/nunchi/roko/roko/tmp/unified/03-GRAPH.md`
5. `/Users/will/dev/nunchi/roko/roko/tmp/unified/04-SPECIALIZATIONS.md`
6. `/Users/will/dev/nunchi/roko/roko/tmp/unified/05-EXECUTION-ENGINE.md`
7. `/Users/will/dev/nunchi/roko/roko/tmp/unified/06-TRIGGER-SYSTEM.md`
8. `/Users/will/dev/nunchi/roko/roko/tmp/unified/07-AGENT-RUNTIME.md`
9. `/Users/will/dev/nunchi/roko/roko/tmp/unified/08-EXTENSION-SYSTEM.md`
10. `/Users/will/dev/nunchi/roko/roko/tmp/unified/09-TELEMETRY.md`
11. `/Users/will/dev/nunchi/roko/roko/tmp/unified/10-LEARNING-LOOPS.md`
12. `/Users/will/dev/nunchi/roko/roko/tmp/unified/11-MEMORY-AND-KNOWLEDGE.md`
13. `/Users/will/dev/nunchi/roko/roko/tmp/unified/12-CONNECTIVITY.md`
14. `/Users/will/dev/nunchi/roko/roko/tmp/unified/13-BUILTIN-BLOCK-CATALOG.md`
15. `/Users/will/dev/nunchi/roko/roko/tmp/unified/14-CONFIG-AND-AUTHORING.md`
16. `/Users/will/dev/nunchi/roko/roko/tmp/unified/15-MARKETPLACE-AND-SHARING.md`
17. `/Users/will/dev/nunchi/roko/roko/tmp/unified/16-SURFACES.md`
18. `/Users/will/dev/nunchi/roko/roko/tmp/unified/17-SECURITY-MODEL.md`
19. `/Users/will/dev/nunchi/roko/roko/tmp/unified/18-ON-CHAIN-REGISTRIES.md`
20. `/Users/will/dev/nunchi/roko/roko/tmp/unified/19-ARENAS-EVALS-BOUNTIES.md`
21. `/Users/will/dev/nunchi/roko/roko/tmp/unified/20-DEPLOYMENT.md`
22. `/Users/will/dev/nunchi/roko/roko/tmp/unified/21-ROADMAP.md`

## Phase 2: Read all source material that drives these updates

### Research (formal grounding + algorithms + frontier integrations + strategic framing)
- `/Users/will/dev/nunchi/roko/roko/tmp/unified/research.md` — substrates: field calculus, parametric optics, CRDTs, event sourcing, QD/active inference, competitive landscape
- `/Users/will/dev/nunchi/roko/roko/tmp/unified/research2.md` — algorithms: HGM CMP, AXIOM BMR, CaMeL IFC, HDC routing, scaling laws, self-play, safety
- `/Users/will/dev/nunchi/roko/roko/tmp/unified/research3.md` — frontier: ZK-HDC passports, topology-breaking 64-agent plateau, hindsight relabeling, Verify-as-reward, PID collective observability, visual self-programming
- `/Users/will/dev/nunchi/roko/roko/tmp/unified/research4.md` — strategic: category-creation framing (Sequoia/a16z/NFX), protocol-not-framework positioning, cost-control as killer wedge, MCP+A2A+ERC-8004+x402 as fixed exoskeleton, five compounding mechanisms, five named UX surfaces, marketplace economics, failure patterns to avoid, autopoiesis (spec-as-runtime-artifact)

### Refinements (architectural redesign proposals — the most important source for elegance)
- `/Users/will/dev/nunchi/roko/roko/tmp/refinements/01-critique-one-noun.md` — why 1 noun is reductive: system has 2 mediums (Engram/Pulse)
- `/Users/will/dev/nunchi/roko/roko/tmp/refinements/02-engram-vs-pulse.md` — Engram (durable) vs Pulse (ephemeral), graduation law
- `/Users/will/dev/nunchi/roko/roko/tmp/refinements/03-bus-as-first-class.md` — Bus as L0 kernel trait alongside Substrate
- `/Users/will/dev/nunchi/roko/roko/tmp/refinements/04-operators-generalized.md` — 6 operators generalized to work on both mediums
- `/Users/will/dev/nunchi/roko/roko/tmp/refinements/05-loop-retold.md` — universal loop with two mediums
- `/Users/will/dev/nunchi/roko/roko/tmp/refinements/10-self-learning-cybernetic-loops.md` — predict-publish-correct: every operator is a learner
- `/Users/will/dev/nunchi/roko/roko/tmp/refinements/11-hyperdimensional-substrate.md` — HDC as first-class, 6 capabilities
- `/Users/will/dev/nunchi/roko/roko/tmp/refinements/12-knowledge-demurrage.md` — demurrage replacing pure Ebbinghaus decay
- `/Users/will/dev/nunchi/roko/roko/tmp/refinements/13-collective-intelligence-c-factor.md` — c-factor as runtime observable
- `/Users/will/dev/nunchi/roko/roko/tmp/refinements/14-worldview-validation.md` — heuristics with mandatory falsifiers
- `/Users/will/dev/nunchi/roko/roko/tmp/refinements/15-exponential-scaling.md` — 7 compounding feedback loops
- `/Users/will/dev/nunchi/roko/roko/tmp/refinements/17-plugin-extension-architecture.md` — 5-tier plugin SPI
- `/Users/will/dev/nunchi/roko/roko/tmp/refinements/18-competitive-moat.md` — moat from architectural coherence
- `/Users/will/dev/nunchi/roko/roko/tmp/refinements/19-net-new-innovations.md` — novel innovations
- `/Users/will/dev/nunchi/roko/roko/tmp/refinements/21-from-scratch-redesigns.md` — rewrite candidates
- `/Users/will/dev/nunchi/roko/roko/tmp/refinements/26-statehub-rearchitecture.md` — StateHub as projection layer
- `/Users/will/dev/nunchi/roko/roko/tmp/refinements/31-synergy-integration-map.md` — 10 named synergies

### Visual-gate2 (verification redesign)
- `/Users/will/dev/nunchi/roko/roko/tmp/visual-gate2/PRD-01-Core-Abstractions.md` — EvidenceCollector, Criterion, Profile type system
- `/Users/will/dev/nunchi/roko/roko/tmp/visual-gate2/PRD-04-Judge-Methodology.md` — pairwise Bradley-Terry, disjoint-family panels, 6 anti-Goodhart safeguards
- `/Users/will/dev/nunchi/roko/roko/tmp/visual-gate2/PRD-05-Self-Improvement-Flywheel.md` — 7-step flywheel: trace → auto-grade → preference-mining → failure-clustering → curriculum-gen → pattern-extract → preference-bootstrap
- `/Users/will/dev/nunchi/roko/roko/tmp/visual-gate2/PRD-06-Community-Marketplace.md` — criteria as plugins, profiles as presets, fork as fundamental, DAW composability

### Run-anywhere (deployment ubiquity)
- `/Users/will/dev/nunchi/roko/roko/tmp/run-anywhere/01-agent-architecture.md` — core architecture
- `/Users/will/dev/nunchi/roko/roko/tmp/run-anywhere/02-whats-novel.md` — novel contributions
- `/Users/will/dev/nunchi/roko/roko/tmp/run-anywhere/07-cognitive-engine.md` — cognitive engine detail
- `/Users/will/dev/nunchi/roko/roko/tmp/run-anywhere/09-skills-and-evolution.md` — skill evolution
- `/Users/will/dev/nunchi/roko/roko/tmp/run-anywhere/19-self-improvement-systems.md` — self-improvement
- `/Users/will/dev/nunchi/roko/roko/tmp/run-anywhere/wasm-and-vision.md` — WASM deployment, Merkle-CRDT, progressive enhancement, brain export

### DeFi gap analysis (real-time domain generalization)
- `/Users/will/dev/nunchi/roko/roko/tmp/defi/gap/00-INDEX.md` — gap heatmap
- `/Users/will/dev/nunchi/roko/roko/tmp/defi/gap/04-GAP-SAFETY.md` — pre-action verification, multi-dimensional risk
- `/Users/will/dev/nunchi/roko/roko/tmp/defi/gap/06-GAP-HEARTBEAT.md` — tick-driven heartbeat for real-time
- `/Users/will/dev/nunchi/roko/roko/tmp/defi/gap/07-GAP-LEARNING-LOOPS.md` — continuous rewards, P&L attribution
- `/Users/will/dev/nunchi/roko/roko/tmp/defi/gap/12-OFFCHAIN-AGENT-MAPPING.md` — how production DeFi bot maps to roko primitives

### 04-21-26 docs (operational specs)
- `/Users/will/dev/nunchi/roko/roko/tmp/04-21-26/generalizations/02-golem-vision.md` — mortality, timescales, heartbeat
- `/Users/will/dev/nunchi/roko/roko/tmp/04-21-26/generalizations/04-agent-runtime-design.md` — type-state lifecycle, CorticalState, CognitiveWorkspace
- `/Users/will/dev/nunchi/roko/roko/tmp/04-21-26/generalizations/05-domain-specialization.md` — domain profiles, somatic markers, behavioral phases
- `/Users/will/dev/nunchi/roko/roko/tmp/04-21-26/generalizations/06-extension-model.md` — 22 hooks with dependencies, actor model
- `/Users/will/dev/nunchi/roko/roko/tmp/04-21-26/generalizations/07-native-harness-design.md` — inference gateway, intent-based routing
- `/Users/will/dev/nunchi/roko/roko/tmp/04-21-26/PRDs/PRD-03-COGNITIVE-ENGINE.md` — PE computation, adaptive threshold ensemble, novelty attenuation
- `/Users/will/dev/nunchi/roko/roko/tmp/04-21-26/PRDs/PRD-04-CONTEXT-ENGINEERING.md` — VCG auction, section effect tracking
- `/Users/will/dev/nunchi/roko/roko/tmp/04-21-26/PRDs/PRD-06-DOMAINS-AND-ARENAS.md` — arenas, Gittins foraging
- `/Users/will/dev/nunchi/roko/roko/tmp/04-21-26/PRDs/PRD-09-EXTENSIBILITY-AND-MULTICHAIN.md` — package ecosystem, multi-chain
- `/Users/will/dev/nunchi/roko/roko/tmp/04-21-26/04-generalized-arenas.md` — Arena trait, meta-arena
- `/Users/will/dev/nunchi/roko/roko/tmp/04-21-26/06-hdc-deep-integration.md` — 6 HDC integration levels
- `/Users/will/dev/nunchi/roko/roko/tmp/04-21-26/09-unified-narrative.md` — full 4-phase agent loop
- `/Users/will/dev/nunchi/roko/roko/tmp/04-21-26/10-knowledge-publishing-and-privacy.md` — 7-layer publishing defense
- `/Users/will/dev/nunchi/roko/roko/tmp/04-21-26/11-geometric-knowledge-sharing.md` — algebraic privacy, O(domains²) network effects

### Workflow docs (Graph authoring patterns)
- `/Users/will/dev/nunchi/roko/roko/tmp/workflow/01-workspace-subsystem.md` — workspace isolation, multi-workspace
- `/Users/will/dev/nunchi/roko/roko/tmp/workflow/07-doc-ingest-worked-example.md` — real Graph composition patterns
- `/Users/will/dev/nunchi/roko/roko/tmp/workflow/11-visual-config-wizard.md` — visual Graph authoring, macro promotion

### Nunchi blockchain docs (purpose-built chain for AI agents)
- `/Users/will/dev/nunchi/roko/roko/docs/08-chain/00-vision-and-framing.md` — chain vision: shared self-curating knowledge ledger
- `/Users/will/dev/nunchi/roko/roko/docs/08-chain/01-korai-chain-spec.md` — 50ms blocks, Simplex consensus, EVM-compatible
- `/Users/will/dev/nunchi/roko/roko/docs/08-chain/02-korai-token-economics.md` — NUNCHI token demurrage (1% annual), earning/spending
- `/Users/will/dev/nunchi/roko/roko/docs/08-chain/03-hdc-on-chain-precompile.md` — native HDC similarity at ~400 gas (20-100× cheaper)
- `/Users/will/dev/nunchi/roko/roko/docs/08-chain/04-korai-passport-erc-721-soulbound.md` — soulbound identity, ventriloquist defense
- `/Users/will/dev/nunchi/roko/roko/docs/08-chain/06-erc-8004-registries.md` — three registries (Identity, Reputation, Validation)
- `/Users/will/dev/nunchi/roko/roko/docs/08-chain/07-4-tier-gossip-architecture.md` — T0 millisecond → T3 canonical
- `/Users/will/dev/nunchi/roko/roko/docs/08-chain/09-peer-scoring-3-layer.md` — protocol + application + economic scoring
- `/Users/will/dev/nunchi/roko/roko/docs/08-chain/10-spore-job-market.md` — marketplace with three hiring models
- `/Users/will/dev/nunchi/roko/roko/docs/08-chain/14-reputation-system-7-domain.md` — EMA reputation with adaptive learning rates
- `/Users/will/dev/nunchi/roko/roko/docs/08-chain/18-mirage-rs-evm-simulator.md` — in-process EVM for development
- `/Users/will/dev/nunchi/roko/roko/docs/08-chain/20-x402-micropayments.md` — HTTP 402 + ERC-3009 gasless transfers
- `/Users/will/dev/nunchi/roko/roko/docs/08-chain/22-valhalla-privacy-layer.md` — four privacy tiers (P0 public → P3 ZK-sealed)
- `/Users/will/dev/nunchi/roko/roko/docs/08-chain/23-knowledge-futures-market.md` — ISFR, clearing, KKT certificates

### Series A intelligence
- `/Users/will/dev/nunchi/roko/roko/tmp/unified/research6.md` — a16z partner map, demo script, 13-slide deck, bear cases, comps, last-14-days intel

### Learnings (self-contained briefings — use for quick context)
- `/Users/will/dev/nunchi/roko/roko/tmp/learnings/01-ARCHITECTURE.md` — full architecture from scratch including Nunchi chain
- `/Users/will/dev/nunchi/roko/roko/tmp/learnings/03-STRATEGY-AND-PITCH.md` — pitch narrative, positioning, fundraise

## Phase 3: Your task — rewrite all 22 docs from scratch

**Rewrite every document from the perspective of someone encountering it for the first time with zero prior context.** The current docs have assumptions baked in from v1 that confuse new readers. Start fresh for each doc, using the source material to write the best possible version.

"Nunchi" is the canonical project name. "Roko" is the agent runtime. The Nunchi blockchain is a core component, not an appendix. Use "Nunchi" for the project/blockchain, "Roko" for the runtime/crates, "Signal/Block/Graph" for the primitives.

Goals:
- **Self-contained**: each doc readable without any other doc (though cross-references help)
- **Protocol-grade**: reads like an RFC — precise, implementable, adoptable by third parties
- **Elegant**: minimum primitives that compose to express everything
- **Chain-integrated**: the Nunchi blockchain is woven throughout (especially docs 01, 12, 15, 17, 18, 19, 21), not siloed

Rules:
- **Keep the 22-file structure** (don't split or merge docs)
- **Keep the fundamental vocabulary** (Signal, Pulse, Bus, Block, Graph, 9 protocols, 10 specializations)
- **Don't add implementation detail** — algorithms, benchmarks, citations go in depth
- **Every concept should feel native** — nothing bolted on, everything composed from primitives
- **The Nunchi chain is part of the core narrative** — doc-18 covers it in detail, but docs 01, 02, 07, 11, 12, 15, 17, 19, 21 all reference chain capabilities naturally

### The Nunchi blockchain in the spec

The chain should appear in these docs (in addition to the dedicated doc-18):

- **Doc-01 (Signal)**: ChainSubstrate as a Store backend. On-chain Signals. ChainBus for chain event Pulses.
- **Doc-02 (Block)**: HDC precompile as a built-in Block. Verify gates that attest on-chain. ZK-HDC proofs.
- **Doc-07 (Agent)**: Soulbound passport as Agent identity. Ventriloquist defense. Agent heartbeat on-chain.
- **Doc-11 (Memory)**: On-chain InsightStore. Knowledge publishing pipeline (7-layer defense). Geometric knowledge sharing (algebraic privacy).
- **Doc-12 (Connectivity)**: Nunchi chain as part of the exoskeleton (alongside MCP, ACP, A2A, x402). Four-tier gossip. Multi-chain temporal resolution.
- **Doc-15 (Marketplace)**: Spore job marketplace. Three hiring models. Demurrage economics for marketplace.
- **Doc-17 (Security)**: Ventriloquist defense. TEE attestation. ZK-HDC capability proofs. Sybil resistance via staking.
- **Doc-18 (On-Chain)**: The full Nunchi chain spec — this is the dedicated chain doc. Use "Nunchi" not "Korai."
- **Doc-19 (Arenas)**: On-chain arena competition. Reputation-weighted bounties. ISFR clearing.
- **Doc-21 (Roadmap)**: Chain deployment phases. Testnet → mainnet. mirage-rs as dev environment.

### The 7 fundamental upgrades (from refinements + research)

These are the deepest changes. They affect the kernel, not just individual docs.

**A. Two mediums, not one noun** (refinements 01-02)

The spec currently says Signal = universal datum. The code already has two shapes:
- **Signal (durable)**: content-addressed, lineage-bearing, scored, decayed, persisted in Store
- **Pulse (ephemeral)**: sequence-numbered, ring-buffered, broadcast via Bus, lives ~seconds

Formalize this. Signal and Pulse are siblings, not parent-child. The graduation law converts Pulse → Signal (the ONLY path from transport to audit DAG). The projection law converts Signal → Pulse (lossy).

Update: Doc-01 (split Signal into two mediums), Doc-00 (vocabulary).

**B. Bus as first-class kernel fabric** (refinements 03)

The spec currently treats Bus as implementation detail. It's actually L0 kernel — the transport fabric alongside the storage fabric (Store/Substrate). Every real-time behavior (heartbeat, event streaming, learning feedback, pheromone sensing) runs through Bus.

```rust
pub trait Bus: Send + Sync {
    async fn publish(&self, pulse: Pulse) -> Result<u64>;
    fn subscribe(&self, filter: TopicFilter) -> PulseStream;
    async fn replay_since(&self, since: u64, filter: &TopicFilter) -> Result<Vec<Pulse>>;
}
```

Update: Doc-01 (add Bus as peer of Store), Doc-02 (protocols work on both mediums), Doc-00 (vocabulary).

**C. Every operator is a learner** (refinements 10, research active inference)

The spec treats learning as a separate subsystem (Learning Loops doc). The refinements show every protocol should predict-publish-correct:

```
1. Block O publishes Pulse("O.prediction", y_hat)
2. Reality publishes Pulse("O.outcome", y_true)
3. CalibrationPolicy joins by lineage, computes error
4. Block O subscribes to error and updates
```

This is active inference made structural. The spec should note this as a design principle — not a separate subsystem, but intrinsic to every protocol.

Update: Doc-02 (note predict-publish-correct on each protocol), Doc-10 (reframe loops as emergent from operator learning).

**D. Demurrage replaces pure Ebbinghaus decay** (refinements 12)

The spec's decay model is time-only (Ebbinghaus). Demurrage adds attention-weighting:
- Every Signal has a balance (inversely taxed over time)
- Balance restored by: retrieval, citation, surprise, gate-pass
- Cold threshold → freeze and archive
- Effect: self-trimming knowledge (unique insights stay warm; duplicates fade)

This is strictly more expressive than Ebbinghaus — you can recover Ebbinghaus as the special case where no interactions occur.

Update: Doc-01 (decay model), Doc-11 (Memory specialization).

**E. Heuristics with mandatory falsifiers** (refinements 14)

The spec mentions playbooks. Heuristics are a richer primitive:
- First-class `Kind::Heuristic` Signal with when/then clause
- **Mandatory falsifier**: "what would prove this wrong?"
- Live calibration from Bus events (gate verdicts, agent outcomes)
- Confidence CI decays via demurrage if unchallenged
- Worldviews emerge as coherent clusters of co-citing heuristics

Update: Doc-11 (add Heuristic as knowledge kind), Doc-01 (Kind enum).

**F. c-factor as runtime observable** (refinements 13, research Riedl PID)

Collective intelligence is measurable — turn-taking entropy, peer prediction accuracy, citation reciprocity, HDC diversity. The spec should note c-factor as a Lens output that gates L4 evolution: only evolve configurations that increase genuine collective intelligence.

Update: Doc-09 (add CollectiveIntelligenceLens), Doc-10 (L4 gates on c-factor).

**G. Verify redesigned per visual-gate2** (visual-gate2 PRDs)

The spec's Verify protocol is too simple. Visual-gate2 shows:
- **Evidence as first-class**: EvidenceCollector separate from Criterion, typed evidence kinds, declarative requirements
- **Conjunctive hard + Pareto soft**: never weighted-sum (Goodhart-resistant). Hard criteria are AND; soft criteria are multi-objective Pareto
- **Pre-action and post-action**: `verify_pre()` + `verify_post()`
- **Continuous reward**: `Verdict.reward: f64` alongside binary pass/fail
- **Pairwise BT judges**: fixed-anchor pairwise comparison aggregated via Bradley-Terry MLE, disjoint-family panels
- **7-step flywheel**: trace → auto-grade → preference-mining → failure-clustering → curriculum-gen → pattern-extract → preference-bootstrap

Update: Doc-02 (Verify protocol), Doc-19 (Arena as flywheel).

### The 12 major additions (from 04-21-26 + DeFi + run-anywhere)

**1. Mortality/Vitality** → Doc-07: Vitality scalar creates behavioral phases (Thriving→Terminal). Economic pressure drives efficient resource use. Mortality is a feature.

**2. Type-state lifecycle** → Doc-07: `Agent<Provisioning>` → `Agent<Active>` ↔ `Agent<Dreaming>` → `Agent<Terminal>`. Compile-time enforced.

**3. CorticalState as lock-free atomics** → Doc-07: Sub-microsecond concurrent perception surface. Prediction error, regime, PAD affect, vitality, attention — all atomic.

**4. Learnable context (CognitiveWorkspace)** → Doc-04/07: VCG auction with 8+ bidders, section effect tracking via beta-distribution posteriors, affect modulation. Context quality compounds.

**5. Hot Graph / tick-driven execution** → Doc-05: Flow stays resident, re-fires per tick. Binds to Agent clock. Workflow/Activity split for deterministic replay.

**6. EFE replacing LinUCB** → Doc-07/10: Expected Free Energy for T0/T1/T2 gating and L2 routing. Each timescale = different free-energy lower bound. 84.4% accuracy at 82% lower compute.

**7. Regime conditioning** → Doc-07/10: Route receives `regime: Signal`. Calm/Normal/Volatile/Crisis affects strategy selection, model tiers, resource allocation.

**8. Hindsight relabeling in L3** → Doc-10: Failed trajectories relabeled for sub-goals they achieved. Recovers ≥45% of discarded episodes. Phase order: NREM → Hindsight → REM → Integration.

**9. L4 self-evolution** → Doc-10: HGM Clade-Metaproductivity, CycleQD with HDC BCs, Verify-as-reward. Variance Inequality: verifier spectrally cleaner than generator.

**10. CaMeL + corrigibility** → Doc-17: Capability-tagged IFC on Extensions. Nayebi 5-head lexicographic corrigibility. Verify gates outside modifiable surface.

**11. Arena as measurement surface** → Doc-19: 8 concrete arenas with cross-arena transfer. Meta-arena = roko developing itself.

**12. Domain profiles as cognitive postures** → Doc-14: Clock + extensions + wakeup events + context weights + gates + infrastructure per profile.

### The 8 structural additions (from run-anywhere + DeFi + workflow)

**13. Multi-slot Agent state** → Doc-07: N named slots with per-slot state/guards and shared global limits. Generalizes DeFi positions, parallel edits, concurrent builds.

**14. Multi-chain temporal resolution** → Doc-12: Actor-per-chain, finality oracle (Final/QuasiFinalized/Reversible), reorg handling.

**15. Package ecosystem** → Doc-14: 5-tier SPI (prompts → JS sandbox → WASM → Rust native → compiled). Progressive capability with progressive isolation.

**16. Workspace scoping** → Doc-14: Multi-workspace daemon, per-workspace capability grants, knowledge scope with cross-workspace sharing.

**17. StateHub as projection layer** → Doc-09/16: Universal typed projections (cohort_health, active_tasks, gate_pipeline, cost_meter, etc.) consumed by TUI/web/Slack/audit. Not TUI-specific.

**18. WASM compilation + brain export** → Doc-20: Same core compiles to native + WASM. Merkle-CRDT syncs learning state across instances. Brain export/import (~100KB-1MB portable knowledge).

**19. DAW marketplace composability** → Doc-15: Criteria as plugins, profiles as presets, fork as fundamental. Fork chains with attribution. Slots + macros on marketplace artifacts.

**20. Somatic markers + affect** → Doc-07: PAD model, prospect theory for continuous outcomes (Kahneman-Tversky λ=1.6), somatic k-d tree queries (<100μs), 15% mandatory contrarian retrieval. 6 behavioral states modulate risk tolerance and context allocation.

### The 6 strategic upgrades (from research4 — framing + positioning + economics)

These change the *tone and framing* of the spec, not just the content. The spec should read as a **protocol specification for the agent economy** — peers are Stripe/Ethereum/ERC-20, not LangGraph/CrewAI.

**H. Protocol-first framing** → Doc-00 (Index)

Reframe the entire spec as defining **standards for the agent economy**, not "how Roko works." The vocabulary table, design principles, and reading order should feel like protocol RFCs that third parties adopt because not adopting is more expensive. The peer set is Stripe-the-protocol, Ethereum-the-protocol, ERC-20-the-standard.

Signal/Block/Graph + HDC + ERC-8004 addresses what Sequoia calls "the three technical bottlenecks for the agent economy":
1. **Persistent identity** → ERC-8004 passports + HDC fingerprints + ZK attestation
2. **TCP/IP-equivalent agent communication** → MCP (tools) + A2A (agent discovery) + Bus (ephemeral transport) + stigmergic coordination
3. **Trust without face-to-face** → ZK proofs over HDC vectors + TraceRank reputation + demurrage-weighted knowledge with on-chain provenance

The spec should state this explicitly as its purpose.

**I. Cost as a design principle** → Doc-00 (Principles)

Add a design principle: **"Cost falls mechanically with volume."** Cost-per-decision decreases through Wright's-law-on-rails — not as an optimization, but as a structural property of the primitives:

- **Route protocol** = cost-aware model selection (EFE naturally balances quality vs cost)
- **Compose protocol** = budget-constrained context assembly (VCG auction)
- **Verify protocol** = cost attribution per Block per Graph per Agent
- **Observe protocol (CostLens)** = real-time cost attribution as first-class telemetry
- **Demurrage** = even memory has cost pressure
- **Semantic caching** = content-addressed Signals maximize cache reuse across Flows
- **T0 gating** = 80% of ticks cost $0 (pure Rust pattern matching, no LLM)

Stacked: semantic caching (5×) × model routing (3×) × structured handoffs (2×) = 10-30× cost reduction vs naïve baseline. This is not optional polish — it's the immediate developer-facing wedge.

**J. Five compounding mechanisms** → Doc-00 (new section)

Name these explicitly as the five mechanisms that take the system from linear to exponential:

1. **Protocol composability** (ERC-20 precedent: $11.4T DEX volume from a few standards). Any conforming Block composes with every existing Block, every Graph, every Signal channel. Each new Block multiplies combinations, not adds them.

2. **Reed's-law group formation** (2^N from ad-hoc coalitions). Stigmergic coordination lets agent coalitions form without central permission. The correction (Briscoe-Odlyzko: real value ∝ N·log(N) due to Dunbar limits) still outpaces Metcalfe (N²) once groups form.

3. **Wright's-law cost curve** (LLM inference prices fell 9-900× per year by task). The primitives ensure cost-per-decision falls mechanically with volume and converts savings into more usage (Jevons paradox), not status-quo savings.

4. **Knowledge compounding with attribution** (each interaction adds to corpus all future agents can query). HDC fingerprinting + ERC-8004 identity + demurrage = compounding semantic memory with cryptographic provenance. Avoids Stack Overflow's failure mode (contributor incentives degraded → 76% drop).

5. **Recursive self-improvement on the OS itself** (DGM + AlphaEvolve precedent). L4 makes the OS itself an agent in the evolutionary archive. The spec evolves through use.

**K. MCP + A2A + ERC-8004 + x402 as fixed exoskeleton** → Doc-12 (Connectivity)

Name these four protocols as the settled exoskeleton the spec builds on:

| Protocol | Role | Status |
|---|---|---|
| **MCP** | Tool/resource discovery | 97M monthly SDK downloads, Linux Foundation |
| **A2A** | Agent-card discovery (`/.well-known/agent-card.json`) | 150+ org support, Linux Foundation |
| **ERC-8004** | On-chain identity (NFT), reputation, validation registries | Ethereum mainnet since Jan 29, 2026 |
| **x402** | Stablecoin agent-to-agent payments | 75M+ transactions, Foundation payment layer |

The spec defines what flows through them:
- Signal/Pulse format over MCP tool calls
- Agent Card format over A2A with HDC capability fingerprints
- ZK-attested HDC fingerprints into ERC-8004 passports
- Budget-bounded payment intents over x402

**L. Five named UX surfaces as protocol-level primitives** → Doc-16 (Surfaces)

Elevate five surfaces from implementation detail to spec-level data contracts that third parties build on. Each surface defines what data it consumes, what interactions it supports, and what Signals it emits:

1. **Workbench** — Structured task surfaces (Linear/Notion pattern). Replaces blank-chat UX. Consumes: active Flows, agent slots, Graph topology. Emits: task assignments, slot fillings, macro adjustments. Design principle: the primary interaction is delegating structured work, not chatting.

2. **Agent Inbox** — Ambient notification surface (LangChain three-mode: notify, question, review). Consumes: Pulses tagged for human attention at three urgency levels. Emits: approvals, decisions, reviews. Design principle: calm technology — peripheral attention, not stream-every-reasoning-step.

3. **Generative Canvas** — Visual Graph editor (workflow doc-11 wizard pattern). Consumes: Graph TOML, Block catalog, TypeSchema. Emits: authored Graphs, promoted Macros, Slot fillings. Design principle: nodes-as-cards, typed cables, drag-and-drop composition.

4. **Stigmergy Minimap** — Coordination visualization (RTS game pattern: fog-of-war, group selection, micro/macro). Consumes: pheromone field state, agent positions, density metrics, c-factor scores. Emits: spawn/cull commands, topology adjustments. Design principle: the "StarCraft UI for AI agents."

5. **Autonomy Slider** — Progressive trust control (Karpathy/Cloudflare pattern). Consumes: agent capability declarations, CaMeL capability tags, Nayebi corrigibility state. Emits: autonomy-level changes, capability grants/revocations. Design principle: five named levels from observe-only to full autonomy, with per-capability granularity.

Each surface is a **projection** (from the StateHub projection layer) + an **interaction contract**. Third parties can build entirely new surfaces that consume the same projections.

**M. Marketplace economics** → Doc-15 (Marketplace)

Add concrete economic model:

- **0% take-rate on first $1M lifetime creator revenue** (Shopify pattern — removes friction for creators)
- **12-15% above $1M** (Unreal Engine 88/12 model — most generous established precedent)
- **All metrics published**: installs, active runs, fork count, gate pass rates, cost averages, revenue — no opaque algorithms
- **Creator owns customer relationship**: direct access to install data, usage patterns, feedback
- **On-chain attribution via ERC-8004**: fork chains, contribution provenance, and reputation all publicly auditable
- **Anti-GPT-Store lessons**: median GPT Store creator earned <$100/quarter because of opaque discovery, no analytics, and invite-only revenue share. The spec's marketplace must not repeat this.

Revenue model comparison:
| Platform | Take rate | Creator visibility | Result |
|---|---|---|---|
| GPT Store | Opaque | None | <$100/quarter median |
| npm / VS Code | 0% | Full | Strongest strategic moats |
| Unreal | 12% (retroactive) | Full | Healthy creator economy |
| Unity | 30% flat | Partial | No asset-store unicorn |

**N. Spec-as-runtime-artifact (autopoiesis)** → Doc-00 (Principles) + Doc-10 (L4)

Add a design principle: **"The spec is a first-class citizen of its own runtime."**

The specification documents are not just human-readable prose — they are:
- **Readable by agents at startup** as context for self-improvement (injected into system prompts during L4 evolution)
- **Queryable as MCP tools** (an MCP server serves spec sections, protocol definitions, and vocabulary)
- **Evolvable through L4** (the L4 archive can propose spec amendments, which are human-reviewed before adoption)
- **Signed under ERC-8004** (each spec version has verifiable provenance via the agent passport system)

This means the spec's formal structure (protocols, type definitions, invariants) should be machine-parseable — not just markdown, but structured enough that an agent can query "what does the Verify protocol guarantee?" and get a precise answer.

In Doc-10 (Learning Loops), note that L4's evolutionary archive includes the spec itself as a mutable artifact — the system's self-model. Structural changes to the spec go through the same CMP scoring + human approval gate as any other L4 mutation.

### Vocabulary update for Doc-00

After all updates, the vocabulary table in Doc-00 should include (at minimum, add others as needed):

| Concept | What It Is | Where |
|---|---|---|
| Signal (durable) | Content-addressed, lineage-bearing, scored, decayed, persisted | Doc-01 |
| Pulse (ephemeral) | Sequence-numbered, ring-buffered, broadcast via Bus | Doc-01 |
| Graduation | Pulse → Signal (the only path from transport to audit DAG) | Doc-01 |
| Bus | Transport fabric — ephemeral pub/sub alongside Store | Doc-01 |
| Pre-action Verify | `verify_pre()` — check before execution, can veto | Doc-02 |
| Continuous reward | `Verdict.reward: f64` — domain-specific learning signal | Doc-02 |
| Evidence typing | EvidenceCollector separate from Criterion, typed kinds | Doc-02 |
| Conjunctive/Pareto | Hard criteria (AND) vs soft criteria (multi-objective Pareto) | Doc-02 |
| Predict-publish-correct | Every operator predicts → publishes → receives corrections via Bus | Doc-02 |
| Hot Graph | Tick-driven Flow that stays resident between firings | Doc-05 |
| Workflow/Activity split | Deterministic orchestration vs non-deterministic execution | Doc-05 |
| Vitality | remaining_budget / initial_budget — economic pressure scalar | Doc-07 |
| Behavioral phases | Thriving/Stable/Conservation/Declining/Terminal | Doc-07 |
| Type-state lifecycle | Compile-time enforced Agent state transitions | Doc-07 |
| CorticalState | Lock-free atomic shared perception surface | Doc-07 |
| Multi-slot state | Agent manages N concurrent slots with shared limits | Doc-07 |
| EFE gating | Expected Free Energy for T0/T1/T2 and L2 routing | Doc-07, Doc-10 |
| Regime conditioning | Route receives regime Signal for context-aware selection | Doc-07, Doc-10 |
| Somatic markers | PAD affect + prospect theory + k-d tree queries (<100μs) | Doc-07 |
| CognitiveWorkspace | Learnable context assembly via VCG + section effect tracking | Doc-07, Doc-04 |
| Section effect | Beta-distribution tracking context → gate success correlation | Doc-07 |
| Novelty attenuation | `1/(1+ln(freq))` — habituation that never reaches zero | Doc-07 |
| Demurrage | Attention-weighted retention replacing pure time decay | Doc-11 |
| Heuristic | First-class Signal kind with when/then + mandatory falsifier | Doc-11 |
| Resonator Networks | HDC factorization — recover constituents from bundles | Doc-11 |
| Hindsight relabeling | Failed trajectories relabeled for achieved sub-goals | Doc-10 |
| Clade-Metaproductivity | Score variants by descendant performance | Doc-10 |
| Verify-as-reward | Verify protocol as reward function for self-play | Doc-10 |
| Variance Inequality | Verifier spectrally cleaner than generator | Doc-10 |
| c-factor | Collective intelligence as runtime observable via PID | Doc-09 |
| CaMeL IFC | Capability-tagged information flow control on Extensions | Doc-17 |
| 5-head corrigibility | Lexicographic safety: deference > switch > truth > impact > task | Doc-17 |
| Domain profile (full) | Complete cognitive posture: clock + extensions + events + gates + infra | Doc-14 |
| Package tiers | 5-tier extensibility with progressive capability/isolation | Doc-14 |
| Arena | Universal measurement surface + 7-step flywheel | Doc-19 |
| Meta-arena | Roko developing itself as an Arena | Doc-19 |
| Finality oracle | Per-transaction confidence for multi-chain operation | Doc-12 |
| Workspace scope | Multi-workspace isolation with cross-workspace knowledge sharing | Doc-14 |
| Brain export | Portable agent knowledge via Merkle-CRDT merge (~100KB-1MB) | Doc-20 |
| StateHub projections | Universal typed projections for all surfaces (TUI/web/audit) | Doc-09 |
| Exoskeleton protocols | MCP + A2A + ERC-8004 + x402 as the fixed external protocol layer | Doc-12 |
| Workbench surface | Structured task delegation (not blank chat) | Doc-16 |
| Agent Inbox surface | Ambient notify/question/review (calm technology) | Doc-16 |
| Generative Canvas | Visual Graph editor with typed cables | Doc-16 |
| Stigmergy Minimap | RTS-style coordination visualization | Doc-16 |
| Autonomy Slider | Progressive trust control with per-capability granularity | Doc-16 |
| Spec-as-artifact | The specification is readable, queryable, and evolvable by agents | Doc-00, Doc-10 |
| Protocol composability | Each new Block multiplies combinations (ERC-20 precedent) | Doc-00 |
| Wright's-law cost curve | Cost-per-decision falls mechanically with volume | Doc-00 |
| Knowledge compounding | Each interaction adds to corpus all future agents query | Doc-00 |

### Design principles update for Doc-00

The 10 principles should be updated to reflect the full vision. Consider adding or revising:

- **Two mediums, two fabrics**: Durable Signals in Store, ephemeral Pulses on Bus. Both are kernel-level.
- **Every operator is a learner**: Predict-publish-correct via Bus. Learning is structural, not a separate subsystem.
- **Demurrage is default**: Signals decay unless actively used. Retrieval, citation, and surprise restore balance.
- **Mortality is a feature**: Agents have finite lifetimes that create efficient resource use and knowledge transfer.
- **Verify is load-bearing**: It's the reward function, relabeling oracle, safety boundary, and economic attestation — all four loops depend on it.
- **Collective intelligence is measurable**: c-factor as a runtime observable that gates evolutionary decisions.
- **Elegance through composition**: Everything composes from 3 fundamentals + 9 protocols. No special machinery.
- **Seven loops compound**: Demurrage-retrieval, heuristic calibration, HDC cleanup, c-factor feedback, playbook meta-learning, cross-deployment commons, plugin ecosystem — each superlinear.
- **Cost falls mechanically with volume**: Wright's-law on rails — caching × routing × gating × handoffs = 10-30× reduction. Savings convert to more usage (Jevons), not status-quo savings.
- **Protocol, not framework**: Signal/Block/Graph are standards for the agent economy. Peers are Stripe and ERC-20, not LangGraph. Third parties build on these because not adopting is more expensive.
- **The spec is a runtime artifact**: Readable by agents at startup, queryable as tools, evolvable through L4, signed under ERC-8004.

### Anti-principles (patterns that have failed — do not repeat)

The spec should also encode what NOT to do, based on documented failures:

- **No standalone destination app** — embed in existing surfaces. Sora D30 retention <8%. Humane/Rabbit failed against smartphones.
- **No naïve multi-agent debate** — requires heterogeneity + structured indirection. Homogeneous debate = majority vote in expectation.
- **No opaque marketplace economics** — GPT Store median creator earned <$100/quarter. Publish all metrics, transparent take-rates, creators own customers.
- **No "we have the most data" moat claims** — pure data network effects are ~98% mythical (Towson). Lean on protocol + workflow embedding + cross-side marketplace.
- **No token speculation narrative** — ERC-8004 identity and utility, not token price.
- **No weighted-sum verification** — Goodhart's Law. Use conjunctive hard + Pareto soft (visual-gate2).
- **No LLM-judging-itself** — Variance Inequality + preference leakage. Verify must be external and heterogeneous.

## Rules

- **Redesign where needed**: If the new understanding fundamentally changes a concept, rewrite the section. Don't bolt new ideas onto old framing.
- **No implementation detail**: Algorithms, benchmarks, and citations go in `tmp/unified-depth/`. The spec says *what* and *why*, not *how*.
- **Preserve elegance**: The unified spec's power is that 36 concepts became 12. Don't inflate it. If a new concept can be expressed as a composition of existing primitives, express it that way.
- **Unified vocabulary**: Signal/Pulse/Bus/Store, Block, Graph, 9 protocols, 10 specializations. Be consistent.
- **Cross-reference**: When a concept spans docs, add a brief note + link in each relevant doc pointing to the primary definition.
- **Keep the 22-file structure**: Don't split or merge docs. The numbering and organization are stable.
