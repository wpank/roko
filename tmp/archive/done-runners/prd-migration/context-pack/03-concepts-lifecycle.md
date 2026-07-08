# Concepts Lifecycle — Removed, Kept, Introduced

> This file catalogs which legacy concepts are removed from the new architecture, which
> are kept (unchanged or reframed), and which concepts are new. Use this as a checklist
> when processing legacy content.

## REMOVED (do not carry forward)

These concepts are explicitly removed from the new architecture. If legacy sources
discuss them, do not propagate the content into the new docs, except when necessary to
explain what was removed and why.

| Concept | Reason for removal |
|---|---|
| Natural death / death protocol | Agents don't have natural death. Users create and delete agents. |
| Thanatopsis | Death protocol phase — removed with mortality |
| Bloodstain | Death artifact — no longer applicable |
| Katabasis | Deep memory descent tied to death — removed |
| Necrocracy | Governance by dead agents — removed |
| Three mortality clocks (economic, epistemic, stochastic) | Reframed as budget limits + confidence tracking. **Stochastic death removed entirely.** |
| Vitality gauge (Thriving → Terminal phases) | Reframed as Daimon PAD behavioral states. **No Terminal state** — cyclical, not terminal. |
| Succession / generational knowledge transfer | Replaced by user-controlled knowledge backup/restore + mesh sharing. |
| Death-themed UX (terminal requiem, death animations, degraded ambient music) | Removed entirely. Spectre animations reflect cognitive state. |
| `roko-golem` umbrella crate | Dissolved — see `01-naming-map.md` Crate Dissolution section |
| Mori vs. Golem separation | Unified under Roko. Chain is a domain plugin, not a separate agent type. |
| "Fleet" as group name | The correct rename is **Collective** or **Mesh**. An earlier memory note wrongly said "fleet" — that is incorrect. |
| Fractal mortality | Removed with mortality |
| Immortal control | Removed with mortality |
| Antifragile mortality (death-specific framing) | Antifragility is kept in general, but the death-specific framing is removed |
| Mortality affect (death emotions) | Reframed — PAD tracks cognitive performance, not mortality anxiety |
| GNOS token | Replaced by KORAI (mainnet) / DAEJI (testnet) |
| Bardo narrative as the primary framing | Replaced by Roko architecture narrative |
| "1 noun 6 verbs" branding | Replaced by Synapse Architecture |

**Files to SKIP entirely** (do not use as sources):
- `bardo-backup/prd/02-mortality/00-thesis.md` (death thesis)
- `bardo-backup/prd/02-mortality/01-architecture.md` (death clocks)
- `bardo-backup/prd/02-mortality/03-stochastic-mortality.md` (random death)
- `bardo-backup/prd/02-mortality/06-thanatopsis.md`
- `bardo-backup/prd/02-mortality/09-fractal-mortality.md`
- `bardo-backup/prd/02-mortality/11-immortal-control.md`
- `bardo-backup/prd/02-mortality/16-necrocracy.md`
- `bardo-backup/prd/02-mortality/18-antifragile-mortality.md`
- `bardo-backup/prd/01-golem/04-mortality.md`
- `bardo-backup/prd/01-golem/05-death.md`
- `bardo-backup/prd/03-daimon/05-death-daimon.md`
- `bardo-backup/prd/22-oneirography/02-death-masks.md`

**Files to extract citations from only** (do not use their framing):
- `bardo-backup/prd/02-mortality/04-economic-mortality.md` — extract budget math only
- `bardo-backup/prd/02-mortality/08-mortality-affect.md` — extract somatic marker citations only
- `bardo-backup/prd/02-mortality/14-research-foundations.md` — keep ALL citations (130+ papers)
- `bardo-backup/prd/02-mortality/15-references.md` — keep ALL citations (162 papers)
- `bardo-backup/prd/03-daimon/04-mortality-daimon.md` — extract non-death concepts only

## KEPT (unchanged)

These concepts and names are kept as-is in the new architecture:

- **Mirage / mirage-rs** — in-process EVM simulator (Korai proxy during dev)
- **CoALA** — 9-step cognitive cycle (maps into universal loop)
- **HDC / VSA** — 10,240-bit BSC vectors, XOR bind, majority bundle, Hamming similarity, cyclic-shift permutation
- **Stigmergy theory** — generalized beyond termites to git commits, code patterns, HDC pheromones
- **Pheromone system** — typed Engrams with Threat/Opportunity/Wisdom + Alpha/Pattern/Anomaly/Consensus decay profiles
- **Sleepwalker** — reduced-capability sleep mode
- **Oneirography / Hypnagogia** — dream journals, hypnagogia engine for Alpha Convergence
- **ALMA** — three-layer temporal affect model (emotion/mood/personality)
- **Somatic markers** — Damasio 1994, now implemented as k-d tree over 8D strategy space
- **Bazaar / MPP / Commerce primitives**
- **All academic citations** — ~200+ papers preserved
- **Portal** (interface concept, renamed Bardo Sanctum → Roko Portal)
- **Testament** (repurposed: knowledge transfer between agents, not death inheritance)
- **Library of Babel** (cross-collective knowledge)
- **Lethe** (knowledge exchange — now P2P Engram sharing via Mesh)
- **Venice dreaming**
- **Xenocognition** (hypnagogia-related)
- **Hauntology** (Derrida trace concept — grounds hypnagogia engine)
- **Homunculus** (hypnagogia observer)

## KEPT, REFRAMED

These concepts are kept but their framing or motivation is changed. The research,
citations, and mechanisms are preserved; the narrative is updated.

| Concept | Old framing | New framing |
|---|---|---|
| **Daimon** | Mortality anxiety affect engine | Cognitive performance affect engine. Tracks task success/urgency/confidence via PAD. Drives tier routing, VCG bidding, somatic retrieval, behavioral state display. |
| **Dreams** | Death-approach triggered consolidation | Idle-time / scheduled consolidation. Three-phase cycle: NREM replay (Mattar-Daw) + REM imagination (Boden + Pearl SCM + emotional depotentiation) + integration staging. |
| **Neuro (was Grimoire)** | Generic knowledge store | Semantic wrapper around Substrate. Six knowledge types, four tiers, Ebbinghaus × tier decay, HDC encoding, cross-domain transfer via structural analogy. |
| **Sonification** | Musical layers mapped to mortality phases | Musical layers mapped to Daimon behavioral states. Eno mandate preserved. No terminal requiem. |
| **Spectre creature** | Vitality/mortality display | Cognitive state display (Daimon PAD). Never dies; adapts. Reflects behavioral state, knowledge tier distribution, current activity, health, mesh connections, pheromone emission. |
| **ROSEDUST design language** | (kept unchanged) | (kept unchanged) |
| **Lifecycle** | Mortality lifecycle (creation to death) | Agent lifecycle (creation, provisioning, deletion, knowledge transfer). User-directed. |
| **TUI 29 screens** | Full TUI with vitality dashboards | Full TUI with C-Factor dashboard, Neuro tier visualization, Spectre viewport |
| **Signal decay (half-life)** | Memory management + mortality metaphor | Memory management only (Ebbinghaus, half-life, Ttl) |
| **Epistemic decay** | Knowledge freshness → agent death clock | Knowledge freshness → knowledge tier demotion (NOT agent lifespan) |
| **Knowledge demurrage** | Economic death + knowledge decay | Token-level decay on KORAI (mirrors Engram half-life) |

## INTRODUCED (new concepts)

These concepts are new in the refactoring-prd and did not exist (or were unnamed) in
the legacy docs.

### Core
- **Engram** — content-addressed, scored, decaying, lineage-tracked unit of cognition. BLAKE3(kind+body+author+tags). Replaces "Signal" as architectural noun.
- **Synapse Architecture** — the 6-trait composition (Substrate/Scorer/Gate/Router/Composer/Policy) crystallized across 5 layers.
- **7-axis Score** — confidence, novelty, utility, reputation (existing) + **precision, salience, coherence** (new).
- **Attestation** — optional cryptographic proof on Engrams (Ed25519 signature + optional ChainAttestation).

### Layers
- **Five Layers**: L0 Runtime, L1 Framework, L2 Scaffold, L3 Harness, L4 Orchestration. Dependencies flow downward. Cross-cuts via trait objects.
- **Cognitive Cross-Cuts**: Neuro, Daimon, Dreams (+ inference optimization, safety/provenance, observability).

### Cognitive primitives
- **Three cognitive speeds**: Gamma (~5-15s reactive), Theta (~75s reflective), Delta (~hours consolidation).
- **Dual-process (System 1 / System 2)**: T0 (no LLM) / T1 (fast) / T2 (deep) cascade.
- **Active inference / EFE**: Expected Free Energy for context selection and action selection.
- **C-Factor (ratio)**: `Collective / Sum(Individual)`. Reporting metric.
- **C-Score (composite)**: `gate_pass×0.3 + cost_eff×0.2 + speed×0.15 + first_try×0.25 + knowledge_growth×0.1`. Optimization metric.

### Frontier innovations (14, see `refactoring-prd/09-innovations.md`)
- **16 T0 Probes** — zero-LLM probes, ~80% tier suppression
- **VCG Attention Auction** — truthful bidding for context budget
- **Somatic Landscape** — k-d tree over 8D strategy space
- **Hypnagogia Engine** — Thalamic Gate + Executive Loosener + Dali Interrupt + Homuncular Observer
- **31.6× Collective Calibration** — 1/sqrt(N×t) heuristic (with caveats)
- **Predictive Foraging** — falsifiable predictions + CalibrationTracker
- **x402 Micropayments** — Coinbase/Linux Foundation protocol
- **Forensic AI** — content-addressed causal replay for regulatory compliance
- **EvoSkills** — self-evolving skill libraries via adversarial verification
- **ADAS** — meta-agent architecture search (Hu et al. ICLR 2025)
- **Cognitive Kernel Primitives** — namespaces, signals, scheduling, syscalls
- **Cross-Domain Insight Resonance** — HDC structural analogy (threshold 0.526 for 10,240-bit)
- **Generative Interfaces (A2UI)** — agents create their own UI
- **Knowledge Futures Market** — on-chain escrow for committed knowledge (P3, deferred)

### Identity, chain, mesh
- **Korai chain** — dedicated EVM for agent coordination. 400ms blocks. HDC precompile.
- **KORAI token** (mainnet, 1% annual demurrage). **DAEJI** on testnet.
- **ERC-8004** — agent identity (ERC-721 soulbound), reputation registry, validation registry.
- **Korai Passport** — ERC-721 soulbound with capabilityList bitmask, domainStakes, reputationTracks, teeAttestation, systemPromptHash (ventriloquist defense), tier (Protocol/Sovereign/Worker/Edge), slashHistory.
- **Agent Mesh** — WebSocket + Iroh P2P + ERC-8004 discovery.
- **Permissioned subnets** — company collectives with private knowledge meshes.
- **4-tier gossip architecture** — GossipSub v1.1 (ms) + MiroFish simulation (sec-min) + FABRIC TEE aggregation (epoch) + Canonical Event Bus (block-finalized).
- **Spore / Sparrow** — job market protocols. Spore = BountySpec. Sparrow = power-of-two-choices dispatch (Ousterhout 2013).
- **Vickrey reputation-adjusted auction** — `s_i = p_i × (1 + (1 - R_i))`. Payment = `s_second / (1 + (1 - R_winner))`.
- **ISFR** — Intersubjective Fact Registry (collective price discovery, 3-arbitrator voting).
- **Valhalla privacy layer** — TEE attestation, PSI protocol, ZK range proofs.

### Visualization
- **Spectre** — procedurally generated creature per agent. Deterministic from agent ID hash. Encodes multiple dimensions into organic visual form. Never dies. TUI ASCII/Unicode + WebGL 3D web rendering.
- **ROSEDUST** — dark-only design system. Void-black + rose accents + jade/amber/crimson/violet/sapphire signals. Glass morphism. Luxury easing.

## Summary checklist

When processing legacy content, ask:
1. Is this a mortality/death concept? → Remove or reframe as resource constraint.
2. Is this an old crate name (golem-*, bardo-*, mori-*)? → Rename to roko-*.
3. Is this a renamed entity (Golem, Grimoire, Styx, Clade, GNOS)? → Apply the rename map.
4. Does this contain academic citations? → Preserve them exactly.
5. Does this contradict refactoring-prd? → refactoring-prd wins.
6. Is this a ROSEDUST or Spectre reference? → Keep (reframed appropriately).
7. Is this a new frontier innovation? → Include from 09-innovations.md.
