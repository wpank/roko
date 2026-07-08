# Glossary

> **TL;DR**: Every term introduced or reclaimed across the 33 earlier
> refinement docs, defined in one place. Entries are terse; each cites
> its home doc for depth. Use this as the canonical reference when
> writing new docs, code comments, or external content. If a term is
> missing here, it should be added before it ships to a wider
> audience.

> **For first-time readers**: This is an A–Z lookup. If you're reading
> other refinement docs and hit an unfamiliar term, search this one.
> The glossary is deliberately factual: definitions, not pitches.
> Cross-references point to the docs where each term earns its
> detailed treatment.

## Conventions

- Terms in **bold** at the start of each entry are the canonical form.
- `Code terms` use backticks.
- Cross-references use the convention from `00-INDEX.md`: bare
  filename for refinement docs, full path for existing architecture
  docs.
- A `(historical)` tag means the term appears in old code/docs and is
  being retired. `(new)` means the term is introduced in the
  refinements folder.
- Every entry cites a home doc. Where the term is split across
  multiple docs, the home is the one with the fullest treatment.

---

## A

**Active inference** — Free-Energy-Principle (Friston 2006) loop
implemented as predict-publish-correct Pulses across every operator.
See `10-self-learning-cybernetic-loops.md` §2.

**ACT** — Step 4 of the seven-step universal loop: execute the
composed Engram as an LLM call, tool call, or chain call. Produces a
stream of Pulses and typically a final AgentOutput Engram.
See `05-loop-retold.md` §3.

**Agent** — A running process or session that drives the universal
loop end-to-end. Historically Golem. See `01-naming-and-glossary.md`
in existing docs.

**Agent mesh** — Peer-to-peer layer for inter-agent coordination via
NATS/libp2p. Formerly Styx. Phase 2+. See `09-phase-2-implications.md`
§5, `24-deployment-ux.md` §1.4.

**Algedonic signal** — Cross-layer alarm that bypasses normal
hierarchy when a lower layer is failing (Beer's VSM term). Implemented
as a Bus Pulse on a priority topic.
See `10-self-learning-cybernetic-loops.md` §4.

**Annotation** — A typed human-authored Engram attached to a target
(episode, heuristic, plan, diff). Kinds: Note, Correction, Confirmation,
Question, Followup. See `30-rich-ux-primitives.md` §3.

**ASSESS** — Step 2 of the seven-step loop: joint Scorer + Router pass
that picks the next action and records confidence.
See `05-loop-retold.md` §3.

**Attestation** — Cryptographic signature over an Engram's content
hash. Levels: LocalAgent, OrgRole, ChainWitness.
See `32-safety-sandbox-provenance.md` §8.

**Authorization** — The `authorize(principal, action, target, ctx)`
function in `roko-agent/src/safety/`. Returns
Allow/Confirm/Once/Deny/Escalate. See `32-safety-sandbox-provenance.md` §2.

---

## B

**Balance** *(new)* — An Engram's demurrage-taxed attention credit.
Starts at 1.0; decays per time; restored by reinforcement.
See `12-knowledge-demurrage.md` §2.

**BROADCAST** — Step 6b of the seven-step loop (co-equal with
PERSIST): publish Pulses to the Bus. See `05-loop-retold.md` §3.

**Body** — Engram (or Pulse) payload. Variants: Text, Json, Bytes.
Reused between Engram and Pulse so graduation is identity.
See `02-engram-vs-pulse.md` §2.2.

**Bus** *(promoted)* — Kernel trait for transport of Pulses; sibling
to Substrate. Today a struct (`EventBus<E>`) in `roko-runtime`;
proposed to become a trait in `roko-core`.
See `03-bus-as-first-class.md`.

**`BusReceiver`** *(new)* — Handle returned by `Bus::subscribe`.
Yields Pulses in publish order. See `03-bus-as-first-class.md` §2.

**BroadcastBus** *(new)* — Default in-process Bus implementation
wrapping `tokio::sync::broadcast`. See `08-code-sketches.md` §4.

---

## C

**c-factor** — Woolley's collective intelligence factor, computed
continuously from Bus statistics for agent cohorts.
See `13-collective-intelligence-c-factor.md`.

**Calibrator** — Policy that updates a Heuristic's `Calibration` after
observing its predictions vs outcomes.
See `14-worldview-validation.md` §3.3, `10-self-learning-cybernetic-loops.md` §10.

**Calibration** — Per-heuristic (or per-claim, per-operator) record
of trials, confirmations, violations, Brier score, Wilson CI.
See `14-worldview-validation.md` §2.

**CascadeRouter** — Existing bandit-based model router in
`roko-learn/src/cascade_router.rs`. Picks model per turn.
See existing code; learning generalization in
`10-self-learning-cybernetic-loops.md` §7.1.

**ChainBus** *(Phase 2+)* — Bus backend that maps on-chain event logs
to Bus topics. See `09-phase-2-implications.md` §1.

**ChainSubstrate** — Substrate backend that persists attestations +
insights on-chain. Stubbed today; Phase 2+.

**Chain witness** — An Ed25519 (or similar) signature attesting to an
Engram's content, committed to a blockchain for cross-deployment
trust. See `32-safety-sandbox-provenance.md` §8,
`09-phase-2-implications.md` §1.

**Claim** *(new)* — Structured hypothesis derived from a Paper
Engram; includes falsifier, context, effect size, calibration.
See `16-research-to-runtime.md` §3.

**`claim!` macro** — Build-time macro resolving a ClaimId; produces
a runtime parameter that self-audits against the replication ledger.
See `16-research-to-runtime.md` §6.

**Cohort** — A set of agents working on a related task, sharing a
plan/PRD/parent episode. Unit of c-factor measurement.
See `13-collective-intelligence-c-factor.md` §2.1.

**Cold tier** — Substrate region for Engrams whose balance hit zero
under demurrage. Content retained but on slower storage; hash still
resolvable. See `12-knowledge-demurrage.md` §7.

**Commons** — Cross-deployment shared library of empirically-
validated heuristics. See `14-worldview-validation.md` §10,
`18-competitive-moat.md` §2.2.

**COMPOSE** — Step 3 of the seven-step loop; the Composer assembles
a prompt Engram under a budget. See `05-loop-retold.md` §3.

**Composer** — One of the six operators. Takes a slice of Datum,
produces an Engram (typically a Prompt). See `04-operators-generalized.md` §7.

**Consistency gate** — Stream-gate that detects semantic drift
between an agent output and its cited Engram support via HDC.
See `11-hyperdimensional-substrate.md` §8.

**ContentHash** — BLAKE3(kind + body + author + tags). Unique
identifier for Engrams. See `docs/00-architecture/02-engram-data-type.md`.

**Context** — Today: a struct passed to operator methods carrying
sidecar state (ctx id, run env, etc.). See existing code.

**Custody** *(new)* — Chain-of-custody record for an auditable
action: who, why, how, simulation, result, witness.
See `25-domain-specific-agents.md` §8.2, `32-safety-sandbox-provenance.md` §5.

---

## D

**Daimon** — Cross-cut handling affect (PAD vector); biases Scorer
and gates Actions. See `docs/00-architecture/13-cognitive-cross-cuts.md`,
`09-phase-2-implications.md` §9.

**`Datum`** *(new)* — Enum `Datum<'a> { Engram(&'a Engram), Pulse(&'a Pulse) }`
used by polymorphic operators. See `04-operators-generalized.md` §1,
`08-code-sketches.md` §3.

**Decay** — Engram decay curves: None, HalfLife, Ttl, Ebbinghaus.
Being superseded by Demurrage for attention weighting.
See `docs/00-architecture/05-decay.md`, `12-knowledge-demurrage.md` §10.

**Delta (1)** — Slowest cognitive speed (hours). Used by Dreams.
See `docs/00-architecture/10-heartbeat.md`.

**Delta (2)** — Incremental update to a StateHub projection's State.
See `26-statehub-rearchitecture.md` §3.

**Demurrage** *(new)* — Economic memory model: balance taxed per time,
restored by reinforcement. See `12-knowledge-demurrage.md`.

**Dissonance** — When two heuristics applicable to a situation would
predict different outcomes. A learning signal.
See `14-worldview-validation.md` §8.

**Domain** — One of Coding, Research, Blockchain, Data, Ops, Writing.
Each has a profile bundle of roles, tools, gates, heuristics.
See `25-domain-specific-agents.md`.

**Dreams** — Cross-cut; offline consolidation loop at Delta speed.
See `docs/00-architecture/10-dreams.md`, `09-phase-2-implications.md` §2.

---

## E

**Ebbinghaus** — A decay curve variant modeling forgetting curves.
See `docs/00-architecture/05-decay.md`.

**Engram** — Durable medium of Roko. Content-addressed, decayed,
scored, lineage-bearing record. Home type in `roko-core/src/engram.rs`.
See `02-engram-vs-pulse.md`.

**EngramBuilder** — Builder for constructing Engrams. Adds
fingerprint, lineage, score at build time. See existing code.

**Envelope** *(historical, `roko-runtime`)* — Wrapper around generic
event `E` carrying seq + timestamp. Being retired for `Pulse`.
See `02-engram-vs-pulse.md` §2, `07-naming.md` §9.

**Episode** — An Engram kind recording a full agent turn (inputs,
tool calls, output, verdicts). See existing code.

**Event** *(historical, avoided as a type name)* — Too generic;
retired in favor of `Pulse`. Still used colloquially in prose to mean
"something that happened."

**EventBus** *(historical, `roko-runtime`)* — Generic
typed broadcast channel. Being replaced by the `Bus` trait with
`Pulse` payload. See `03-bus-as-first-class.md` §6.

---

## F

**Fabric** — A kernel data-movement primitive. Roko has two:
Substrate (storage) and Bus (transport). See `03-bus-as-first-class.md` §1.

**Falsifier** — A Predicate attached to a Claim or Heuristic
specifying what observable would refute it.
See `14-worldview-validation.md` §2, `16-research-to-runtime.md` §13.

**Fingerprint** *(new)* — 10,240-bit HDC vector attached to every
Engram at put time; indexes similarity queries.
See `11-hyperdimensional-substrate.md` §3.

**Fleet** — Roster of agents (deployment-scoped). Formerly Clade.
See historical naming notes.

---

## G

**Gamma** — Fastest cognitive speed (5–15 s). Used by per-turn loops.
See `docs/00-architecture/10-heartbeat.md`.

**Gate** — One of six operators; verifies an Engram or Pulse window
against external truth. See `04-operators-generalized.md` §5.

**GateVerdict** — Engram kind produced by a Gate. Body includes pass/fail,
reason, evidence. See existing code.

**Golem** *(historical)* — Old name for Agent. Retired.

**Graduation** *(new)* — Converting a Pulse into an Engram for
durable persistence. Canonical path from transport to audit DAG.
See `02-engram-vs-pulse.md` §3, `08-code-sketches.md` §1.

**Grimoire** *(historical)* — Old name for Neuro. Retired.

---

## H

**Harness** — One of three deliverable surfaces (runtime, harness,
scaffold). Roughly: the scaffolding that runs agents with gates +
supervision. See `docs/00-architecture/12-five-layer-taxonomy.md` L3.

**HDC** — Hyperdimensional Computing (Kanerva 2009). 10,240-bit
binary-or-bipolar vectors; bind/bundle/permute algebra.
See `11-hyperdimensional-substrate.md`.

**`HdcVector`** — Rust type in `roko-primitives` / future
`roko-hdc` crate. See `11-hyperdimensional-substrate.md` §11.1.

**Heartbeat** — Cognitive clock publishing `heartbeat.gamma.tick`,
`heartbeat.theta.tick`, `heartbeat.delta.tick` Pulses.
See `09-phase-2-implications.md` §7.

**Heuristic** *(new)* — First-class Engram variant with
preconditions, prediction, calibration, lineage, receipts.
See `14-worldview-validation.md`.

**Holographic** — Property of HDC: partial information still
retrieves the whole; damage degrades gracefully.
See `11-hyperdimensional-substrate.md` §1.

---

## I

**Identity fingerprint** — HDC vector characterizing an agent's
recent Engrams; used for team discovery and diversity tracking.
See `11-hyperdimensional-substrate.md` §10.

**Intrinsic motivation** — Policy biasing attention toward high
prediction-error regions. See `10-self-learning-cybernetic-loops.md` §11.

---

## K

**Kernel** *(Roko-specific)* — The set of types and traits in
`roko-core` that every other crate depends on. Includes Engram,
Pulse, Substrate, Bus, Scorer, Gate, Router, Composer, Policy.
See `04-operators-generalized.md` §10.

**Kind** — Enum in `roko-core/src/kind.rs` enumerating semantic
categories of Engrams/Pulses (Plan, Task, GateVerdict, Episode,
Heuristic, Paper, etc.). ~28 variants today.

**Korai** — Agent-chain integration layer (blockchain). Formerly part
of Styx. Phase 2+. See `09-phase-2-implications.md` §1.

---

## L

**Layer (L0–L4)** — Five-layer taxonomy: Runtime, Framework, Scaffold,
Harness, Orchestration. Strictly downward dependencies.
See `docs/00-architecture/12-five-layer-taxonomy.md`.

**Lineage** — `Vec<ContentHash>` on an Engram pointing at its parents
in the audit DAG. See existing code.

**`loop_tick`** — The universal cognitive loop function in
`roko-core/src/loop_tick.rs`. Revised to 7 steps.
See `05-loop-retold.md` §8.

---

## M

**MCP** — Model Context Protocol. Standard for tool integration via
stdio/HTTP. Roko ships MCP integrations in `roko-mcp-*`.
See existing code; plugin-level story in `17-plugin-extension-architecture.md` §4.

**MetaGate** — Gate that runs on the agent's own self-model.
See `10-self-learning-cybernetic-loops.md` §6.3.

**Mesh** — See Agent mesh.

**MultiBus** *(new)* — Bus backend composing several Bus backends
behind one interface. See `03-bus-as-first-class.md` §3.2.

---

## N

**Neuro** — Cross-cut; durable knowledge store, distillation, tier
progression. Formerly Grimoire. See `crates/roko-neuro/`,
`docs/00-architecture/13-cognitive-cross-cuts.md`.

**Novelty** — `1 - max(similarity)` over top-K HDC neighbors; used by
demurrage reinforcement to weight uniqueness.
See `12-knowledge-demurrage.md` §3.

---

## O

**Operator** — One of the six kernel verb traits: Scorer, Gate,
Router, Composer, Policy (plus Substrate, Bus as fabric operators).
See `04-operators-generalized.md`.

**Orchestrator** — Layer-4 subsystem that runs plans, dispatches
tasks, enforces merge queues. See `crates/roko-orchestrator/`.

**Outcome Pulse** *(new)* — Pulse on `outcome.*` topic that closes
the loop on a previously-published prediction Pulse.
See `10-self-learning-cybernetic-loops.md` §2.2.

---

## P

**PAD vector** — Pleasure-Arousal-Dominance affective state;
maintained by Daimon. See `docs/00-architecture/09-daimon.md`.

**Paper** *(new)* — Engram kind representing an academic paper,
with DOI, authors, abstract, fingerprint, claims.
See `16-research-to-runtime.md` §2.

**PERSIST** — Step 6a of the seven-step loop; write an Engram to
Substrate. See `05-loop-retold.md` §3.

**Pheromone** — Engram kind used for stigmergic coordination between
agents. See `09-phase-2-implications.md` §3.

**Plan** — Engram kind representing a structured multi-task plan
with DAG edges. See existing code.

**Playbook** — Engram kind storing a distilled reusable action
sequence. See existing code; relationship to heuristics in
`14-worldview-validation.md` §1.

**Plugin** — Third-party extension. Five tiers of power/risk:
prompts, profiles, manifests, native, WASM.
See `17-plugin-extension-architecture.md`.

**Policy** — One of six operators; reacts to streams of Pulses,
emits new Pulses and Engrams. See `04-operators-generalized.md` §8.

**`PolicyOutputs`** *(new)* — Return type of `Policy::decide`;
contains `{ pulses, engrams }`. See `04-operators-generalized.md` §8.

**Prediction Pulse** *(new)* — Pulse on `prediction.*` topic
emitted by an operator when it makes a decision; matched to a
later `outcome.*` Pulse via lineage_hint.
See `10-self-learning-cybernetic-loops.md` §2.2.

**PRD** — Product Requirements Document. A directory in `.roko/prd/`
representing a work item's lifecycle (idea, draft, plan).

**Principal** — User, agent, or plugin; the subject of an
authorization decision. See `32-safety-sandbox-provenance.md` §2.

**Projection** *(new, StateHub)* — Named, typed, live-updating
view on the Bus + Substrate; has `State` and `Delta` types plus
a folding function. See `26-statehub-rearchitecture.md` §3.

**Profile** — A bundle of defaults: either a deployment profile
(laptop / single-server / container / ...) or a domain profile
(coding / research / blockchain / ...). Context disambiguates.
See `24-deployment-ux.md` §2, `25-domain-specific-agents.md` §9.

**Provenance** — Full author/trust/taint/attestation record on an
Engram. See existing code.

**Pulse** *(new)* — Ephemeral medium of Roko. Typed,
sequence-numbered, topic-addressed, ring-buffered message.
Lives on a Bus. Can graduate to Engram. See `02-engram-vs-pulse.md`.

**`PulseSource`** *(new)* — Light origin attribution struct on
every Pulse: `{ component, agent_id }`.
See `08-code-sketches.md` §1.

---

## Q

**`query_similar`** *(new)* — Substrate method returning Engrams
whose HDC fingerprint is within `radius` of a query fingerprint.
See `11-hyperdimensional-substrate.md` §4.

---

## R

**REACT** — Step 7 of the seven-step loop: Policy.decide produces
new Pulses + Engrams. See `05-loop-retold.md` §3.

**Reinforcement** *(new, demurrage)* — Bonus to an Engram's balance
when it's cited, retrieved, gated, surprises, or agent-quoted.
See `12-knowledge-demurrage.md` §2, §3.

**`ReinforceKind`** *(new)* — Enum: Cited, Retrieved, Gated,
Surprised, AgentQuoted. See `12-knowledge-demurrage.md` §2.

**Replication ledger** *(new)* — Per-claim record of
paper-reported-effect vs our-observed-effect, with CI and status.
See `16-research-to-runtime.md` §5.

**Role** — A composition template + tool allow-list + gate defaults.
Examples: researcher, planner, implementer, reviewer, compliance.
See `crates/roko-compose/src/templates/`.

**Router** — One of six operators; picks among candidates.
See `04-operators-generalized.md` §6.

**Runtime** — Layer-0 subsystem. Process supervisor, cancellation,
Bus, Substrate. See `crates/roko-runtime/`.

---

## S

**Scaffold** — One of three deliverable surfaces. Roughly: the
structure that agents compose within. See `docs/00-architecture/12-five-layer-taxonomy.md` L2.

**Score** — 7-axis appraisal attached to an Engram by the Scorer.
See `docs/00-architecture/04-seven-axis-score.md`.

**Scorer** — One of six operators; computes Score for any Datum.
See `04-operators-generalized.md` §4.

**SENSE** — Step 1 of the seven-step loop: perceive from three
sources (Substrate, Bus, external I/O). See `05-loop-retold.md` §3.

**Session** — A bounded run of agent interaction, typically
ephemeral unless graduated. Transcripts exportable.
See `23-user-ux-running-agents.md` §12.

**Signal** *(historical)* — Old name for Engram. Retired in 877:5
rename. See `07-naming.md` §2.2.

**SPI** — Service Provider Interface; the extension-point surface
for plugins. See `17-plugin-extension-architecture.md`.

**Stigmergy** — Indirect coordination via shared environment
(Grassé 1959). Implemented as Pheromone Engrams + `mesh.pheromone.*`
Pulses. See `09-phase-2-implications.md` §3.

**StateHub** *(promoted)* — Kernel projection layer. Today
TUI-specific; proposed to become a first-class kernel subsystem.
See `26-statehub-rearchitecture.md`.

**Styx** *(historical)* — Old name for Agent mesh. Split into
Mesh and Korai.

**Substrate** — Kernel trait for storage of Engrams.
See `docs/00-architecture/07-substrate-trait.md`, `03-bus-as-first-class.md` §1.

**Swarm** — Collective of agents subscribed to the same topic set;
outputs union across agents. See `09-phase-2-implications.md` §6.

---

## T

**Taint** — Metadata indicating untrusted input origin; propagates
through derived Engrams. See `32-safety-sandbox-provenance.md` §7.

**Theta** — Middle cognitive speed (~75 s). Used by the main plan-
execute loop. See `docs/00-architecture/10-heartbeat.md`.

**Topic** *(new)* — Routing handle for Bus publish/subscribe.
Dot-separated lowercase, e.g. `gate.verdict.emitted`. Type
`Topic(String)`. See `03-bus-as-first-class.md` §2.1,
`07-naming.md` §7.

**`TopicFilter`** *(new)* — Declarative filter for subscriptions.
Variants: Exact, Glob, AnyOf, All, And, Or, Not.
See `03-bus-as-first-class.md` §2.1.

**Trust score** — Per-agent-pair, per-topic accumulated reputation.
See `13-collective-intelligence-c-factor.md` §3.3.

**TypedContext** *(new)* — Structured domain situation data.
`{ domain, fields: BTreeMap<Key, Value> }`. Gates and heuristics
match typed predicates rather than free text.
See `25-domain-specific-agents.md` §8.1.

---

## U

**Undo** — Three-level mechanism: ephemeral (chat edit), short-term
(`roko undo last`), long-term (`roko replay ... --revert`).
See `23-user-ux-running-agents.md` §11.

**Universal loop** — Seven-step cognitive loop: SENSE, ASSESS,
COMPOSE, ACT, VERIFY, PERSIST/BROADCAST, REACT.
See `05-loop-retold.md`.

---

## V

**Verdict** — Output of a Gate. Always materialized as a
`GateVerdict` Engram so the audit DAG is preserved.
See `04-operators-generalized.md` §5.

**VERIFY** — Step 5 of the seven-step loop: Gate (or stream-gate)
verifies an Engram (or Pulse window) and emits a Verdict.
See `05-loop-retold.md` §3.

---

## W

**Watchdog** — Policy subscribed to a Claim's falsifier predicate
across all episodes; updates the replication ledger automatically.
See `16-research-to-runtime.md` §7.3.

**Wilson CI** — Wilson score interval for a binomial confidence
bound; used by Calibration. See `14-worldview-validation.md` §2.

**WisdomGate** — Gate enforcing Surowiecki's four conditions
(diversity, independence, decentralization, aggregation) before
a consensus Engram is finalized.
See `13-collective-intelligence-c-factor.md` §4.

**Worldview** *(new)* — Co-citation cluster of mutually-supporting
heuristics that dominate a domain-fingerprinted region of
situations. See `14-worldview-validation.md` §4.

**Witness** — See Chain witness.

---

## Retired / deprecated terms

These appear in historical code or docs; do not use in new work.

| Old | Replaced by | Reason |
|---|---|---|
| `Signal` (durable) | `Engram` | 877:5 rename already landed |
| `Signal` (ephemeral) | `Pulse` | Naming-cleanup cost too high to reclaim |
| `EventBus<E>` | `Bus` trait + `Pulse` | Ad-hoc generic; not canonical |
| `Envelope<E>` | `Pulse` | Implementation name leaked out |
| `Message` | `Pulse` (wire) / `ChatMessage` (LLM) | Ambiguous with LLM chat |
| `Event` | `Pulse` | Collides with every other framework |
| `Bardo`, `Golem`, `Mori` | `Roko` + `Agent` | Previous codename heritage |
| `Styx` | `Mesh` + `Korai` | Split into two clearer concepts |
| `Grimoire` | `Neuro` | Less mystical |
| `Clade` | `Fleet` | More conventional |
| `decay` (field on Engram) | `balance` + `demurrage` | Demurrage supersedes time-only decay |

## Terms deliberately not defined here

A few terms are used colloquially in the docs but aren't formal Roko
terms. Left to standard engineering usage:

- "session" (in casual sense, distinct from the formal Session
  above — colloquial uses always clarify context)
- "session" in the OIDC/HTTP sense (authentication session)
- "task" (agent's unit of work; vs. a background task)
- "model" (LLM, distinct from runtime model)
- "cost" (USD)

If any of these starts behaving as a technical term in a new doc,
promote them to this glossary.

## Maintenance

This doc is the canonical vocabulary. Rules:

- Every new technical term introduced in a refinement doc adds a
  glossary entry in the same PR.
- Retiring a term moves it to the "Retired" table with a reason.
- Cross-references in refinement docs use the glossary spellings.
- Annual review prunes entries that never took.

## Cross-references

- Historical naming decisions: `docs/00-architecture/01-naming-and-glossary.md`.
- Naming rationale for the new terms: `07-naming.md`.
- The synergy map that stitches primitives together: `31-synergy-integration-map.md`.
