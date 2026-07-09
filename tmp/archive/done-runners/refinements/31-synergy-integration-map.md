# Synergy & Integration Map

> **TL;DR**: The previous 30 docs describe individual refinements. This
> doc draws the wires between them. Every primitive reinforces at least
> two others; each "moat" claim from 18 cashes out as a specific web of
> interactions documented here. The lesson: Roko's competitive edge is
> not any single feature, it's the *interaction density* of the whole.
> A competitor can copy any one node, and often a pair; copying the
> whole weave requires them to commit to the same architectural choices
> *in the same order*.

> **For first-time readers**: If you've only read one or two of docs
> 02–30, you may wonder whether they're a grab-bag of ideas. They are
> not — they compose. This doc is the cross-reference map. The
> sections below pick ten core primitives and walk, for each, what it
> gives to and takes from the others. Skim §2 for the matrix; read
> the subsections for the mechanisms.

## 1. The ten load-bearing primitives

These are the nodes of the synergy graph. Each is the subject of one
or more refinement docs:

| # | Primitive | Home doc |
|---|---|---|
| P1 | Engram (durable medium) | 02 |
| P2 | Pulse (ephemeral medium) | 02 |
| P3 | Bus (transport fabric) | 03 |
| P4 | Substrate (storage fabric) | 03, existing code |
| P5 | HDC fingerprint | 11 |
| P6 | Demurrage (attention economy) | 12 |
| P7 | Heuristics + falsifiers | 14 |
| P8 | c-factor (collective intelligence) | 13 |
| P9 | Replication ledger | 16 |
| P10 | Plugin SPI + domain profiles | 17, 25 |

The edges in the matrix below are the synergies. Each cell says "what
does the row give to the column?" Empty cells are independent
primitives — rare but possible.

## 2. The synergy matrix

A compact table. Rows are what each primitive *provides*. Columns are
what it provides *to*. Entries are short — see the subsections for
depth.

| ↓ gives → to → | P1 Engram | P2 Pulse | P3 Bus | P4 Substrate | P5 HDC | P6 Dem. | P7 Heur. | P8 c-fct | P9 Repl. | P10 Plug. |
|---|---|---|---|---|---|---|---|---|---|---|
| **P1 Engram** | — | graduation src | publish target | store target | encode target | balance owner | lineage | cohort artifacts | paper body | plugin config target |
| **P2 Pulse** | graduation dest | — | payload | sub events | — | reinforce sig | calib trial | cohort events | ledger obs | plugin event |
| **P3 Bus** | `substrate.*` | delivery | — | notify | — | reinforce ch | falsif watch | cohort floor | watchdog | lifecycle evt |
| **P4 Substrate** | home | — | bridge | — | store fp | balance home | heur store | metric src | ledger store | plugin state |
| **P5 HDC** | fp field | — | — | index key | — | novelty score | fp-matching | diversity | paper search | encoder slot |
| **P6 Dem.** | weight | — | — | tier logic | — | — | freshness | minority sub. | anti-drift | plugin aging |
| **P7 Heur.** | Engram var | — | prediction/outcome | store | cluster | rein signal | — | peer model | claim body | heuristic plugin |
| **P8 c-fct** | cohort record | — | metrics topic | metric store | diversity src | — | peer-pred | — | cohort repl | c-factor plugin |
| **P9 Repl.** | paper engram | — | watchdog topic | ledger store | paper fp | claim decay | lifted claim | ledger obs | — | claim plugin |
| **P10 Plug.** | plugin engram | plugin events | plugin topics | plugin reads | plugin encoder | plugin budget | new heur src | new metric | new claim | — |

Reading the matrix: the column "P5 HDC" shows what everybody gives to
HDC — P1 provides encoding targets, P4 provides the index key slot,
P7 provides cluster semantics. The column "P8 c-factor" shows what
c-factor needs — bus stats (P3), cohort artifacts (P1), peer models
(P7), HDC diversity (P5). Sparse columns are red flags: the primitive
is probably underdeveloped.

## 3. Ten named synergies with worked mechanisms

Ten specific, multi-primitive interactions. Each is a concrete behavior
you can point at in a running Roko instance (or a future one).

### 3.1 Demurrage × HDC → self-trimming semantic memory

Ingredients: P4, P5, P6.

- Substrate stores each Engram with fingerprint.
- Demurrage taxes balance per unit time.
- `ReinforceKind::Surprised` weight is `bonus × novelty`, where
  `novelty = 1 - max(similarity with top-K neighbors)` via HDC.
- Common/duplicate Engrams get little bonus; unique Engrams get
  large bonus.

Effect: memory self-trims toward *uniquely useful* rather than
*accumulated*. Each of the three primitives individually produces a
weaker result; the combination is specific.

### 3.2 Heuristic × Pulse × Bus → continuous calibration

Ingredients: P2, P3, P7.

- Heuristic ships with a falsifier predicate (P7).
- The falsifier subscribes to relevant `gate.*` or `agent.*` Pulses
  (P3, P2).
- On match, a `CalibrationPolicy` updates the heuristic's trials
  counter.
- Brier score + Wilson CI drift with exposure.

Effect: beliefs are scored by lived experience without manual
evaluation passes. Contrast with static prompt libraries.

### 3.3 c-factor × Bus × HDC → cognitive-diversity-aware routing

Ingredients: P3, P5, P8.

- Bus stats give turn-taking entropy.
- HDC cloud distance gives cognitive diversity.
- c-factor Policy watches both; when diversity drops, spawn an
  agent with a deliberately different role or model (§3.5 of 13).

Effect: the runtime actively guards against monoculture. Requires all
three; any one alone is descriptive, not regulatory.

### 3.4 Replication ledger × Heuristics × Paper Engram → living research

Ingredients: P1, P7, P9.

- Paper as Engram lives in Substrate.
- Claims derived from the paper become Heuristics (lifted from the
  `From<Claim>` impl in 16 §4).
- Watchdogs subscribe to outcome Pulses; ledger updates when
  falsifiers fire.
- Replication status becomes a badge on every cited claim.

Effect: engineering decisions are traceable to empirical support that
itself is continuously verified. No other agent framework has this.

### 3.5 Plugin SPI × Substrate × Bus → ecosystem growth path

Ingredients: P3, P4, P10.

- Plugin manifests declare subscribed topics and readable kinds.
- Safety layer enforces declared scope.
- Plugin adds tools, gates, roles without touching core.
- Healthy plugins graduate into the default profile; unhealthy ones
  fade (same rule as heuristic calibration).

Effect: the platform grows without core churn. Two-sided network
effect (18 §2.3) becomes live once ~50 healthy plugins exist.

### 3.6 c-factor × Heuristics → peer-model learning

Ingredients: P7, P8.

- Each agent maintains `PeerModel<OtherAgentId>` — the agent's
  expectation of what another agent believes.
- `peer_prediction_accuracy` (c-factor axis) is this model's
  calibration.
- High peer-prediction accuracy correlates with high c-factor;
  low means "rotate pairs" or "inject outsider" (13 §6).

Effect: social perceptiveness is an emergent property of calibration
on heuristic-level predictions.

### 3.7 Dreams × Substrate × Pulse → retroactive insight

Ingredients: P1, P2, P4.

- Delta-speed Dreams loop reads recent Engrams (P4, P1).
- Re-consolidates with *current* priors — retrospective learning.
- Publishes `engram.promoted` Pulses (P2) so consumers (Composer,
  StateHub) update their caches.

Effect: old episodes produce *new* heuristics as the system's
knowledge grows. Compounds with time (15 §7).

### 3.8 Demurrage × Heuristic × Calibration → graceful relearning

Ingredients: P6, P7.

- Heuristic has calibration CI.
- Confidence decays via demurrage when unchallenged (12 §5).
- A long-stable heuristic weakens enough that a modest stream of
  contradicting outcomes flips it without a manual reset.

Effect: the system never gets stuck on stale beliefs. Without
demurrage on confidence, a high-confidence-early heuristic can
dominate indefinitely.

### 3.9 HDC × Consensus × Bus → substantive agreement detection

Ingredients: P2, P3, P5.

- Agents publish `consensus.vote.cast` Pulses with HDC fingerprints
  of their responses.
- Aggregator bundles and computes similarity to a proposal
  fingerprint.
- High similarity = substantive agreement even when wording
  differs; low similarity = surface-only agreement.

Effect: can distinguish "two agents said same thing in different
words" from "two agents happened to both include the phrase X." Token-
based voting can't do this.

### 3.10 TypedContext × Domain profiles × Gate → auditable domain-specific
safety

Ingredients: P10 (domain profiles) + 25 §8.1 (TypedContext) + 25 §8.2
(Custody).

- Every action carries a `TypedContext` of the situation it happens in.
- Gates match on typed predicates rather than free-text.
- Custody records who acted, why (heuristics), how (claims), and
  what happened, signed and stored as Engrams.

Effect: every Ops/Blockchain/Compliance-sensitive action is auditable
after the fact with full provenance, without the team having to
hand-build logging. Useful when regulators care (18 §2.4).

## 4. What the matrix isn't

- **Not a claim of completeness.** More synergies exist. Good
  candidates for §3 additions: "Replay scrubber × Substrate ×
  StateHub → inspectable time-travel," "Chain witness × Replication
  ledger × commons → cross-deployment truth."
- **Not a priority queue.** The synergies with the highest value per
  engineering-day are in `35-consolidated-roadmap.md`. This doc
  describes *what* combines, not *when*.
- **Not a vendor pitch.** The matrix is for internal alignment; the
  pitch lives in 19 (catalog) and 18 (moat) which cite this for the
  mechanisms.

## 5. Designing new refinements with the matrix in mind

When adding a new feature or refinement, walk the matrix before
starting:

1. Which of P1–P10 does the feature interact with?
2. For each interaction, is the feature providing or consuming?
3. Are any interactions missing that *should* exist? (Usually the
   most interesting design work.)

Example: "live voice interface for the agent." Walking the matrix:
- P2 Pulse: voice chunks are Pulses on `agent.voice.chunk`. New topic.
- P4 Substrate: does voice produce Engrams? Only if transcribed and
  graduated. Introduce a `VoiceTurn` kind.
- P6 Demurrage: voice-turn Engrams should decay quickly (high volume,
  low per-turn value).
- P10 Plugin: voice is a tier-3 plugin (tool) and tier-4 client
  (speaker/microphone binding).
- P7 Heuristic: are there voice-specific heuristics? ("Always ask
  for confirmation before destructive action when interaction is
  voice-only.")
- P5 HDC: voice fingerprint for speaker identification? Maybe.

Walking the matrix reveals where the feature *doesn't* connect —
often a sign of a gap in the feature, not a gap in the matrix.

## 6. The moat restated

18 §2.1 says the architectural coherence is the moat. The matrix
makes that claim concrete:

- P1–P4 (kernel primitives) are table stakes; anyone can copy.
- P5, P6, P7 are advanced individually; each has prior art.
- P8, P9, P10 are integrations of the earlier primitives — they only
  exist *because* P1–P7 exist.
- The matrix cells are the real moat. A competitor has to reproduce
  every cell to reproduce Roko. They can reproduce any column in
  isolation, but not the column's interaction with its neighbors.

This is why 18 §5 says the competitor "attacks the feature list and
gives up on the architectural story." Feature-list competition
ignores the matrix; column competition ignores the rows; only a
matrix-level effort reproduces the moat. That effort is an 18-month
project at 3 engineers, minimum.

## 7. Non-synergies worth naming

For honesty: some primitives don't interact despite being in the same
system.

- **P5 HDC ⊥ P9 Replication**: papers have fingerprints, but the
  ledger's empirical rigor doesn't use them. Cosine-search over papers
  is a convenience, not a substantive synergy.
- **P10 Plugins ⊥ P8 c-factor**: plugins can contribute c-factor
  projections, but c-factor doesn't regulate plugin selection.
- **P2 Pulse ⊥ P9 Replication**: individual Pulses don't directly
  feed the ledger. The ledger consumes aggregates, not streams.

These are not weaknesses — not every pair must couple. Naming the
non-synergies keeps the matrix honest.

## 8. Emergent properties from the composition

Three properties Roko-the-whole has that Roko-any-subset doesn't:

1. **Self-improvement without bespoke training.** Every operator
   predicts, every prediction is calibrated, every calibration
   updates weights via Bus. The runtime is a distributed online
   learner with no central training script.
2. **Inspectability at every level.** Pulse lineage + Engram
   lineage + heuristic provenance + claim citation means no
   decision is black-box. Contrast: an LLM wrapper with logs.
3. **Substrate neutrality.** HDC + demurrage + heuristics produce
   behavior that is *about the work*, not about the storage or
   transport implementation. Swap Substrate from SQLite to Postgres,
   Bus from broadcast to NATS — the cognitive properties persist.

These three emerge from the matrix, not from any single cell.

## 9. How to use this doc

- Design review: "does the proposed change strengthen or weaken any
  existing synergies?"
- Onboarding: "which matrix cells is this person working on today?"
- Debugging: "the feature isn't delivering compounding. Which cells
  aren't wired correctly?"
- Hiring: "can this candidate think at matrix-level granularity, or
  only feature-level?"

The matrix is a tool for thought, not a checklist. It should get
revised as Roko evolves — candidates to add next: P11 "Dreams cycle"
once Phase 2 lands, P12 "Chain witness" once on-chain attestation
ships.

## 10. Cross-references

This is the integrator doc. Every refinement doc is cited.
Specifically:

- The mediums and fabrics (P1–P4): 02, 03.
- The learning stack (P5–P7): 10, 11, 12, 14.
- The social stack (P8): 13.
- The scientific stack (P9): 16.
- The ecosystem stack (P10): 17, 25.
- The moat claim that depends on this matrix: 18.
- The catalog that itemizes the primitives: 19.
- The modularity story that keeps the matrix implementable: 20.
- The roadmap that sequences all of the above: 35.
