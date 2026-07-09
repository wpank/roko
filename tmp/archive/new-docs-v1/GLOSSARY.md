# Glossary

> Flat A-Z lookup. One term per line. Use [`ALIASES.md`](ALIASES.md) for public-facing aliases.
> This is a lookup tool, not a tour. Follow the home-doc link for depth.
>
> **Status tags**: `[shipping]` = in the codebase and wired · `[built]` = code exists, not yet wired ·
> `[planned]` = design term, no shipped code · `[retired]` = replaced, use the replacement.

---

## A

**ACT** — `[shipping]` — Step 4 of the seven-step universal loop (query → score → route → compose → **act** → verify → persist); the point where the agent calls the LLM or dispatches a tool. Home: [`reference/06-loop/`](reference/06-loop/).

**Active inference** — `[planned]` — Predict-publish-correct loop carried across operators using `prediction.*`, `outcome.*`, and `prediction.error.*` Pulses; the free-energy minimization framework underlying Roko's planned self-calibration. Home: [`research/foundations/active-inference.md`](research/foundations/active-inference.md).

**AffectBias** — `[built]` — Public alias for `Daimon`; the name used in user-facing docs, CLI output, and external communication. See `Daimon`.

**Agent** — `[shipping]` — A running process or session that executes the cognitive loop. Replaces the retired term `Golem`. Home: [`reference/README.md`](reference/README.md).

**AlphaCode** — `[—]` — DeepMind compound system (Li et al. 2022) demonstrating that harness architecture, not model size, drove competitive programming performance; cited as empirical evidence for the scaffold thesis. Home: [`status/vision.md`](status/vision.md).

**AntiKnowledge** — `[built]` — One of six `Neuro` knowledge types; encodes what the agent has learned *not* to do. Home: [`reference/README.md`](reference/README.md).

**Attestation** — `[shipping]` — Trust level assigned to an `Engram`'s origin, ranging from `LocalAgent` through `ChainWitness`. Home: [`reference/01-engram/`](reference/01-engram/).

## B

**BLAKE3** — `[shipping]` — Cryptographic hash function used to compute `ContentHash` for every `Engram`; enables content-addressed identity and deduplication. Home: [`reference/01-engram/`](reference/01-engram/).

**Body** — `[shipping]` — Enum field on `Engram` encoding the actual payload variant (Code, Text, Json, Binary, etc.). Home: [`reference/10-types/`](reference/10-types/).

**Bardo** — `[retired]` — Former project name, retired in favor of `Roko`. See [`strategy/refinements/naming-history.md`](strategy/refinements/naming-history.md).

**Bus** — `[planned]` — Target-state transport fabric abstraction; routes `Pulse` events through `Topic` handles. Current live implementation: `EventBus<E>`. Home: [`reference/04-bus/`](reference/04-bus/).

## C

**C-Factor** — `[shipping]` — Collective intelligence quotient measuring how much faster a fleet of agents solves problems together than any single agent alone. Analogous to the *c* factor in human collective intelligence research. Home: [`research/foundations/c-factor.md`](research/foundations/c-factor.md).

**Calibrator** — `[planned]` — Target-state learning component split from `Policy`; handles adaptation and threshold tuning independently of control logic. Home: [`strategy/refinements/`](strategy/refinements/).

**CascadeRouter** — `[shipping]` — Three-stage model-selection router: Static (rule-based) → Confidence (threshold-gated) → UCB (bandit-driven). Part of `roko-agent`. Home: [`reference/05-operators/`](reference/05-operators/).

**CausalLink** — `[built]` — One of six `Neuro` knowledge types; encodes a cause-effect relationship learned from execution history. Home: [`reference/README.md`](reference/README.md).

**Chain** — `[built]` — Roko's EVM-compatible layer for on-chain agent coordination; soulbound identity passports, reputation, job marketplace. See also `Korai`. Home: [`reference/README.md`](reference/README.md).

**Clade** — `[retired]` — Former term for the agent roster; replaced by `Fleet`. See [`strategy/refinements/naming-history.md`](strategy/refinements/naming-history.md).

**Composer** — `[shipping]` — One of the six Synapse traits; assembles system prompts and context windows for LLM calls. Home: [`reference/05-operators/`](reference/05-operators/).

**Compound AI Systems** — `[—]` — Berkeley AI Research thesis (Zaharia et al. 2024) that SOTA results come from multi-component systems, not individual models; foundational citation for Roko's architecture. Home: [`status/vision.md`](status/vision.md).

**ContentHash** — `[shipping]` — BLAKE3-derived identity field on `Engram`; enables content-addressed deduplication and integrity verification. Home: [`reference/10-types/`](reference/10-types/).

**Conductor** — `[built]` — Cybernetic regulator implementing Good Regulator Theorem; 10 watchers, graduated interventions, EWMA anomaly detection, Yerkes-Dodson pressure dynamics. Home: [`reference/README.md`](reference/README.md).

**Custody** — `[planned]` — Chain-of-custody audit record for auditable agent actions; distinct from `Provenance`. Home: [`reference/01-engram/`](reference/01-engram/).

## D

**Daimon** — `[built]` — Affect cross-cut implementing PAD vectors (Pleasure-Arousal-Dominance); modulates model tier selection, exploration rate, and compute allocation. Public alias: `AffectBias`. Home: [`reference/09-cross-cuts/`](reference/09-cross-cuts/).

**Datum** — `[planned]` — Target-state polymorphic input type accepting either an `Engram` or a `Pulse`; eliminates one-off sum types in operator signatures. Home: [`reference/README.md`](reference/README.md).

**Decay** — `[shipping]` — Time-variant attenuation applied to `Engram` scores; four variants: balance (demurrage), reinforcement, novelty weighting, and cold-tier freeze/thaw. Home: [`reference/10-types/`](reference/10-types/).

**Delta (δ)** — `[scaffold]` — Slowest cognitive speed; consolidation window of hours; used for offline learning, knowledge compression, and playbook construction. Home: [`reference/07-speeds/`](reference/07-speeds/).

**Demurrage** — `[shipping]` — Preferred decay model; charges a holding fee on unused `Engram` balances rather than degrading the record, keeping active knowledge fresh. Home: [`reference/10-types/`](reference/10-types/).

**Dreams** — `[scaffold]` — Delta-speed consolidation cross-cut; NREM replay, REM imagination, and slow consolidation. Transforms execution episodes into persistent `Neuro` knowledge. Home: [`reference/09-cross-cuts/`](reference/09-cross-cuts/).

**DSPy** — `[—]` — Prompt compiler framework (Khattab et al. 2024) demonstrating automated prompt pipeline optimization; cited as evidence that prompt assembly is a learnable scaffold concern. Home: [`status/vision.md`](status/vision.md).

## E

**Engram** — `[shipping]` — The durable content-addressed record medium. Content-addressed via BLAKE3, 7-axis scored, four decay models, lineage DAG, attestation level. The fundamental noun of the Roko data model. Replaces the retired `Signal` (durable usage). Home: [`reference/01-engram/`](reference/01-engram/).

**EventBus\<E\>** — `[shipping]` — Current live transport implementation; will be replaced by the `Bus` abstraction in target state. Home: [`reference/04-bus/`](reference/04-bus/).

## F

**FileSubstrate** — `[shipping]` — JSONL-backed `Substrate` implementation; the default durable storage backend. Home: [`reference/03-substrate/`](reference/03-substrate/).

**Fleet** — `[planned]` — Agent roster; replaces the retired `Clade`. Home: [`strategy/refinements/naming-history.md`](strategy/refinements/naming-history.md).

**FrugalGPT** — `[—]` — Cascade routing framework (Chen et al. 2023, arXiv:2305.05176) showing GPT-4 quality at 2% cost via cascading; the academic precursor to Roko's `CascadeRouter`. Home: [`status/vision.md`](status/vision.md).

## G

**Gamma (γ)** — `[shipping]` — Fastest cognitive speed; reactive execution window of ~5–15 seconds; handles a single LLM call or tool dispatch. Home: [`reference/07-speeds/`](reference/07-speeds/).

**Gate** — `[shipping]` — One of the six Synapse traits; binary accept/reject verdict on agent output. 11-gate pipeline in `roko-gate`. Home: [`reference/05-operators/`](reference/05-operators/).

**Golem** — `[retired]` — Former term for agent; replaced by `Agent`. See [`strategy/refinements/naming-history.md`](strategy/refinements/naming-history.md).

**Grimoire** — `[retired]` — Former term for `Neuro`; replaced. See [`strategy/refinements/naming-history.md`](strategy/refinements/naming-history.md).

## H

**HDC (Hyperdimensional Computing)** — `[built]` — Encoding scheme used by `Neuro`; 10,240-bit binary vectors enabling sub-millisecond similarity search via Hamming distance. Home: [`reference/10-types/`](reference/10-types/).

**Heuristic** — `[built]` — One of six `Neuro` knowledge types; an actionable rule extracted from execution history. Home: [`reference/README.md`](reference/README.md).

## I

**Insight** — `[built]` — One of six `Neuro` knowledge types; a general observation about agent behavior or domain structure. Home: [`reference/README.md`](reference/README.md).

## K

**Kind** — `[shipping]` — Enum field on `Engram` classifying its semantic category (Task, Observation, Knowledge, Plan, Result, etc.). Home: [`reference/10-types/`](reference/10-types/).

**Korai** — `[built]` — The EVM-compatible chain for agent coordination; soulbound passports, reputation, job marketplace, HDC precompile, KORAI/DAEJI tokens. Blocked by chain deployment. See `Chain`. Home: [`reference/README.md`](reference/README.md).

## L

**Lineage** — `[shipping]` — DAG field on `Engram` encoding the chain-of-thought / causal ancestry of a record. Enables forensic replay and causal attribution. Home: [`reference/01-engram/`](reference/01-engram/).

## M

**Mesh** — `[planned]` — Target-state agent-network layer; replaces the retired `Styx`. Home: [`strategy/refinements/naming-history.md`](strategy/refinements/naming-history.md).

**Meta-Harness** — `[—]` — Framework (Lee et al. 2026, arXiv:2603.28052) demonstrating +7.7 points classification and +4.7 points math from harness optimization alone at 4× fewer tokens; the primary empirical anchor for the scaffold thesis. Home: [`status/vision.md`](status/vision.md).

**Mori** — `[retired]` — Former project name (predecessor to Bardo/Roko). See [`strategy/refinements/naming-history.md`](strategy/refinements/naming-history.md).

## N

**Neuro** — `[built]` — Durable knowledge cross-cut; 6 knowledge types × 4 validation tiers, HDC-encoded. The agent's long-term memory. Replaces the retired `Grimoire`. Home: [`reference/09-cross-cuts/`](reference/09-cross-cuts/).

**Novelty weighting** — `[shipping]` — Decay variant that boosts new `Engram` scores and allows them to decay faster; ensures fresh information gets attention before it ages. Home: [`reference/10-types/`](reference/10-types/).

## P

**PAD vectors** — `[built]` — Pleasure-Arousal-Dominance encoding used by `Daimon` to represent affect state. Home: [`reference/09-cross-cuts/`](reference/09-cross-cuts/).

**ParallelExecutor** — `[shipping]` — Pure state machine in `roko-orchestrator`; schedules tasks from a cross-plan DAG, isolates work in git worktrees, serializes merges via conflict-aware queue. Home: [`reference/README.md`](reference/README.md).

**PERSIST** — `[shipping]` — Step 7 of the universal loop; writes the verified output `Engram` to `Substrate`. Home: [`reference/06-loop/`](reference/06-loop/).

**Plan** — `[shipping]` — A DAG of tasks generated from a PRD; executed by `ParallelExecutor`. Home: [`reference/README.md`](reference/README.md).

**Policy** — `[shipping]` — One of the six Synapse traits; encodes behavioral rules and safety constraints. Planned: `Calibrator` will split learning logic out of `Policy`. Home: [`reference/05-operators/`](reference/05-operators/).

**PRD** — `[shipping]` — Product requirements document; the input to `roko prd`; Roko reads its own PRDs to generate self-improvement plans. Home: [`reference/README.md`](reference/README.md).

**Provenance** — `[shipping]` — Durable audit context attached to an `Engram`: who created it, what tools were used, what Engrams it derived from. Home: [`reference/01-engram/`](reference/01-engram/).

**Pulse** — `[planned]` — Target-state ephemeral transport medium; the counterpart to `Engram` for short-lived events. Replaces retired wire terms: `Event`, `Envelope`, `Message`, `Signal` (ephemeral usage). Home: [`reference/02-pulse/`](reference/02-pulse/).

**PulseSource** — `[planned]` — Lightweight Pulse origin attribution; replaces overloaded provenance terms for ephemeral events. Home: [`reference/02-pulse/`](reference/02-pulse/).

## Q

**QUERY** — `[shipping]` — Step 1 of the universal loop; retrieves candidate `Engram` records from `Substrate` for a task. Home: [`reference/06-loop/`](reference/06-loop/).

## R

**REACT** — `[shipping]` — Step 8 of the universal loop; updates learning subsystems (playbook, bandit, cost tracker) after each turn. Home: [`reference/06-loop/`](reference/06-loop/).

**Roko** — `[shipping]` — The project and framework name. Replaces the retired project names `Bardo` and `Mori`. Home: [`status/vision.md`](status/vision.md).

**Router** — `[shipping]` — One of the six Synapse traits; selects model, agent, or execution path. Home: [`reference/05-operators/`](reference/05-operators/).

**runtime shape** — `[planned]` — Deployment form descriptor (laptop / server / container / cluster); replaces overloaded use of `profile`. Home: [`strategy/refinements/`](strategy/refinements/).

## S

**Scaffold** — `[—]` — (1) The harness wrapping an LLM that determines agent performance; the core subject of the scaffold thesis. (2) Implementation status tier: struct/trait stubs exist, no meaningful implementation. Context determines which sense is intended.

**Score** — `[shipping]` — 7-axis appraisal value attached to every `Engram`: four stable axes (confidence, novelty, utility, reputation) + three extended (precision, salience, coherence). Home: [`reference/10-types/`](reference/10-types/).

**Scorer** — `[shipping]` — One of the six Synapse traits; computes `Score` for any `Engram`. Home: [`reference/05-operators/`](reference/05-operators/).

**Signal** — `[retired]` — Former term used for both durable records (replaced by `Engram`) and wire events (replaced by `Pulse` in target state). Both uses retired. See [`strategy/refinements/naming-history.md`](strategy/refinements/naming-history.md).

**Situation** — `[planned]` — Public alias for `TypedContext`; the user-facing name for structured domain situation payloads. See `TypedContext`.

**StateHub** — `[built]` — Current dashboard and event hub; target-state projection layer over `Bus` and `Substrate`. Home: [`reference/README.md`](reference/README.md).

**StrategyFragment** — `[built]` — One of six `Neuro` knowledge types; a partial strategy pattern extracted from successful execution episodes. Home: [`reference/README.md`](reference/README.md).

**Substrate** — `[shipping]` — The durable storage fabric; one of the six Synapse traits. CRUD + similarity search on `Engram` records. Default implementation: `FileSubstrate`. Home: [`reference/03-substrate/`](reference/03-substrate/).

**Synapse Architecture** — `[shipping]` — Roko's internal architecture: two mediums (Engram + Pulse), two fabrics (Substrate + Bus), six operator traits. Name reflects the design philosophy that simple connectors produce complex cognition. Home: [`status/vision.md`](status/vision.md).

**SystemPromptBuilder** — `[shipping]` — 7-layer prompt assembly component in `roko-compose`; handles template selection, Liu et al. U-shape placement, and token budget management. Home: [`reference/05-operators/`](reference/05-operators/).

**SWE-bench** — `[—]` — Software engineering benchmark (Jimenez et al. 2024) demonstrating 2× performance variance from harness design with the same base model; primary empirical anchor for the scaffold thesis alongside Meta-Harness. Home: [`status/vision.md`](status/vision.md).

## T

**Taint** — `[shipping]` — One-way provenance flag on an `Engram`; marks records from untrusted or potentially compromised sources; cannot be removed. Home: [`reference/01-engram/`](reference/01-engram/).

**Theta (θ)** — `[shipping]` — Middle cognitive speed; reflective synthesis window of ~75 seconds; handles multi-step reasoning, context assembly, and plan evaluation. Home: [`reference/07-speeds/`](reference/07-speeds/).

**Topic** — `[planned]` — Target-state `Pulse` routing handle; replaces `Channel` and `Subject`. Home: [`reference/04-bus/`](reference/04-bus/).

**TopicFilter** — `[planned]` — Target-state subscription matcher for `Pulse` routing; replaces ad hoc routing filters. Home: [`reference/04-bus/`](reference/04-bus/).

**TypedContext** — `[planned]` — Structured domain situation payload for context-aware routing and composition. Public alias: `Situation`. Replaces free-text-only context matching. Home: [`strategy/refinements/`](strategy/refinements/).

## V

**VERIFY** — `[shipping]` — Step 6 of the universal loop; runs the gate pipeline on agent output before persistence. Home: [`reference/06-loop/`](reference/06-loop/).

## W

**Warning** — `[built]` — One of six `Neuro` knowledge types; encodes a known hazard or failure mode that the agent should avoid. Home: [`reference/README.md`](reference/README.md).

---

## Terms Not Yet in the Glossary

The following terms appear in source documents and have been noted as candidates for addition
in a future pass. See [`_migration/cluster-D-vision.md`](_migration/cluster-D-vision.md) for
the tracking list.

- `Experiment` (bandit experiment framework in `roko-learn`)
- `Episode` (execution record unit in the learning subsystem)
- `Playbook` (collection of extracted heuristic rules)
- `SkillLibrary` (reusable execution patterns)
- `PatternMiner` (pattern extraction subsystem)
- `CostTracker` (efficiency accounting subsystem)
- `SporePool` / `SparrowJob` (Korai chain job marketplace terms)
- `Kauri BFT` (Korai chain consensus)
- `ERC-8004` / agent passport standard
- `KORAI` / `DAEJI` tokens
- `SpecPool` (EVM parallel execution)
