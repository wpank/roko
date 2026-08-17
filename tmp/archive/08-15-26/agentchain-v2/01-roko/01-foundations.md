# 01 — Foundations

> The vocabulary and shape of the system. The five kernel primitives, the nine protocols, the four universal patterns, the thirteen specializations, and the categorical foundations that make composition work.

---

## 1. The One Rule

**Everything is a Graph of Cells processing Signals through Bus and Store.**

Every system, subsystem, and feature in Roko is expressed as a composition of the same five primitives. There are no special cases. If something seems to need special machinery, it means a new Cell specialization is needed, not a new concept.

This rule eliminates god files, ad-hoc state management, and one-off infrastructure. Every subsystem composes with every other subsystem by construction, because they all speak the same protocol.

Roko's vocabulary is small enough that a developer learns 14 concepts and gets the rest as discoverable patterns:

- **5 Primitives**: Signal, Pulse, Cell, Graph, Protocol.
- **9 Protocols**: Store, Score, Verify, Route, Compose, React, Observe, Connect, Trigger.
- **4 Universal Patterns**: Pipeline, Loop, Functor, Space.
- **13 Specializations**: Flow, Hot Flow, Rack, Lens, Loop, Memory, Extension, Agent, Connector, Feed, Recipe, Group, Pipeline.

---

## 2. The Five Primitives

| Primitive | What it is | Where it lives |
|---|---|---|
| **Signal** | Durable datum. Content-addressed (BLAKE3 of payload), typed, scored on five axes, decayed via demurrage, lineage-tracked, HDC-fingerprinted. | Store |
| **Pulse** | Ephemeral event. Sequence-numbered, ring-buffered, broadcast. Signal's transient sibling. | Bus |
| **Cell** | Atomic computation: Signals in, Signals out. Declares typed I/O, capabilities, protocol conformance. | Registry |
| **Graph** | Typed DAG (or cyclic graph) of Cells with edges, conditions, and data mappings. A Graph implements Cell, so Graphs nest. | Loaded from TOML |
| **Protocol** | Behavioral contract a Cell conforms to. Async trait interface with typed signatures. | Trait declarations |

Two fabrics carry data: **Store** for durable Signals, **Bus** for ephemeral Pulses. Nothing else. No ad-hoc state channels, no hidden caches, no side-band communication. This constraint is what makes subsystem composition work by construction — anything that reads from Store or subscribes to Bus automatically interoperates with everything else.

### Signal — the durable medium

A Signal carries a typed payload, a 5-axis quality score, an economic balance that decays via demurrage, a full lineage DAG, and a 10,240-bit HDC (Hyperdimensional Computing) fingerprint for similarity search. Every piece of durable state in the system — knowledge entries, episode logs, gate verdicts, configuration snapshots — is a Signal.

The Signal struct is an algebraic object. Three fields each participate in a separate algebraic structure:

- `content_hash` participates in the **lineage monoid** (append-only DAG).
- `hdc_fingerprint` participates in the **vector semiring** (bind, bundle, permute).
- `kind` participates in the **kind lattice** (flat kinds join into compound kinds).

Identity is algebraically exact (hash). Similarity is algebraically approximate (vector).

### Pulse — the ephemeral medium

A Pulse is a sequence-numbered, ring-buffered event broadcast via Bus. Pulses carry lifecycle events, streaming output, predictions, and coordination signals. Unlike Signals they have no lineage, no scoring, and no HDC fingerprint — they are intentionally transient.

Topics are hierarchical, OpenTelemetry-style: `agent:{id}.heartbeat`, `gate.verdict.emitted`, `prediction.{operator}`, `pheromone.{location_hash}`, and so on.

### The two bridges: graduation and projection

Signals and Pulses are siblings, not parent and child. Two explicit bridges connect them.

- **Graduation**: `Pulse → Signal`. The only path from transport into the audit DAG. Idempotent: graduating the same Pulse twice produces the same SignalRef because the Signal is content-addressed.
- **Projection**: `Signal → Pulse`. A lossy broadcast of stored Signals. Forgets the content hash, fingerprint, score, balance, tier, full lineage; retains kind, body, timestamp, and a `lineage_hint` back to the Signal.

Categorically, graduation is a structure-preserving functor `F : Pulse → Signal` and projection is a forgetful functor `G : Signal → Pulse`. They form an adjunction `F ⊣ G`: subscribing to store-write notifications on Bus is equivalent to querying Store for recently graduated Signals.

A graduation policy decides which transient events deserve durable preservation. Gate verdicts, agent turn completions, safety approvals, and cost charges always graduate; heartbeat ticks and UI refreshes never do.

---

## 3. The Kind System

Every Signal and Pulse has a `Kind` determining schema, demurrage behavior, and Cell interaction. Kinds cover: core data (Text, Markdown, Json, Code, Diff, Binary, Image), artifacts (File, Artifact), knowledge (Insight, Heuristic, Warning, CausalLink, StrategyFragment, AntiKnowledge), coordination (Pheromone, Heartbeat, Presence), execution (Evidence, Finding, Verdict, Episode, CostReport), and observation (Observation, Alert, Trend, Anomaly), plus user-defined Custom kinds.

The Kind system forms a **join-semilattice**. A `Compound([Kind])` kind is the lattice join. Filter matching uses lattice ordering: a `Verdict` filter matches `Compound([Verdict, TestResult])` because `Verdict ≤ Compound([Verdict, TestResult])`.

### Heuristic — a first-class learned rule

A Heuristic is the system's actionable knowledge: a testable prediction with a mandatory falsifier and a live calibration record grounded in episode outcomes (not LLM self-report). Every Heuristic carries `when` predicates, a `then` action, a `falsifier` (Popper's falsificationism applied to learned rules), and a Calibration with Brier score and Wilson confidence interval. A heuristic without a falsifier cannot be created. When a falsifier fires above its retirement threshold, the heuristic spawns refined children with narrower preconditions.

---

## 4. Score — Five Axes

Every Signal carries a five-dimensional Score: `relevance`, `quality`, `confidence`, `novelty`, `utility`. Score Cells produce these; Route Cells consume them; Compose uses them for budget-constrained assembly.

The five axes collapse to a scalar when total ordering is needed:

```
effective(score) = confidence
                 * max(relevance, 0.1)
                 * max(quality, 0.1)
                 * (1 + novelty)
                 * (1 + utility)
```

Multiplicative composition gives three properties: zero confidence kills (data known to be wrong cannot be prioritized by gaming other axes), bonus stacking is superlinear, and the `max(0.1)` floor prevents zeroing relevant Signals while still letting confidence-zero kill the score.

Novelty uses **attenuation**: `novelty = 1 / (1 + ln(1 + freq))` — habituation that never reaches zero, so even highly familiar Signals retain a nonzero floor.

Scores from different Score Cells need not be on the same scale. **Temperature scaling** (Guo et al. 2017) corrects this; the temperature is learned by minimizing Expected Calibration Error (Naeini et al. 2015). Per-axis confidence uses **Beta-Binomial conjugate updates** with weakly informative prior `Beta(2, 2)`.

When no single aggregation is appropriate, the **Pareto front** maintains the set of non-dominated Signals.

---

## 5. Demurrage — Knowledge Pays a Holding Cost

Signals decay via **demurrage** (Gesell 1916) — an attention-weighted holding cost that replaces pure time-based forgetting. Every Signal has a `balance` that starts at 1.0 and decreases unless reinforced.

The balance evolves by `dB/dt = -r - β * B(t) + reinforcement(t)` — a flat tax, an exponential decay, and a reinforcement bonus. The flat tax prevents zombie Signals; the exponential term prevents knowledge hoarding.

| Reinforcement | Trigger |
|---|---|
| Retrieved | Signal returned in a query |
| Cited | Signal included in a context that passed a gate |
| GatePassed | Signal in the context pack of a successful gate evaluation |
| Surprised | Signal relevant to a high-prediction-error observation |
| AgentQuoted | Signal directly referenced in agent output |

Reinforcement is novelty-weighted: citing a common Signal gives a small bump; citing a rare one gives a large bump. The combined freshness score blends balance with an Ebbinghaus-style age weight per Kind.

### Tiers

Signals progress through Transient → Working → Consolidated → Persistent, with multipliers (0.1×, 0.5×, 1.0×, 5.0×) on the decay rate. Progression criteria (gate-pass counts, distinct contexts, consortium approval) and demotion criteria are explicit. Tier transitions form a Markov chain that is **ergodic** — every tier reachable from every other via cold storage and thaw.

Below a cold threshold, Signals enter cold storage: body moves to slower storage; content hash stays valid; HDC fingerprint stays in the warm index for thaw discovery; lineage preserved. Frozen Signals (consensus + tier + calibration met) skip demurrage entirely. Demurrage is continuous and economically grounded; genuinely useful knowledge persists indefinitely while noise self-eliminates.

---

## 6. HDC Fingerprints — Algebraic Similarity

Every Signal carries a 10,240-bit binary HDC vector for similarity search and cross-domain pattern discovery (Kanerva 2009). HDC vectors form a **semiring** under three operations:

- **Bind (XOR)** — multiplicative. Self-inverse: `bind(a, bind(a, b)) = b`. Used for role-filler binding and causal association.
- **Bundle (majority vote)** — additive. Produces a composite similar to all inputs. Bundle noise scales as `√N`. Practical limit ≈ 200 vectors before re-encoding via a hierarchical bundle tree.
- **Permute (cyclic rotation)** — temporal ordering. `permute(v, i+j) = permute(permute(v, i), j)`.

Hamming distance via hardware POPCNT computes similarity in roughly a microsecond per pair. At 10,240 bits, 800K fingerprints fit in 1 GB of RAM and full SIMD similarity search runs in under 1 ms. **No external vector store is needed.**

| Property | HDC (10,240-bit binary) | Float (1536-d float32) |
|---|---|---|
| Size per vector | 1,280 bytes | 6,144 bytes |
| Similarity cost | XOR + POPCNT | Dot product (hundreds of FLOPs) |
| Compositionality | Native (bind, bundle, permute) | Requires learned operations |
| Privacy | Non-invertible after privacy-preserving HDC | Invertible via decoder |
| Determinism | Identical seeds produce identical vectors | Depends on model version |

When Signals from different domains have similar HDC fingerprints, they share structural properties despite surface differences. Retrieval gives cross-domain matches a bonus. Resonator networks (Frady et al. 2020) factor bundles back into role-filler pairs, enabling decomposition of composite knowledge entries and partial-information queries.

The chain-side HDC similarity-search precompile is the chain's responsibility. The agent runtime describes only the agent-side HDC engine and how it consumes the precompile when the chain is available.

---

## 7. Cell — The Universal Computation Unit

A **Cell** is atomic computation with an identity, typed inputs and outputs, declared capabilities, protocol conformance, and cost estimation.

```rust
pub trait Cell: Send + Sync + 'static {
    fn id(&self) -> CellId;
    fn name(&self) -> &str;
    fn version(&self) -> Version;
    fn input_schema(&self) -> Option<&TypeSchema>;
    fn output_schema(&self) -> Option<&TypeSchema>;
    fn capabilities(&self) -> &Capabilities;
    fn protocols(&self) -> &[ProtocolId];
    fn estimated_cost(&self) -> Option<Cost>;
    fn estimated_duration(&self) -> Option<Duration>;
    async fn execute(&self, input: Vec<Signal>, ctx: &CellContext)
        -> Result<Vec<Signal>, CellError>;
}
```

`CellId` is content-addressed: deterministic from `(name, version, author)`. The runtime uses Cell declarations for type-checking edges, capability intersection, cost budgeting, and protocol dispatch.

### Capabilities — three-layer intersection

Capabilities are fail-closed. A Cell only runs if the intersection of three capability sets is non-empty:

| Layer | Source | Authority |
|---|---|---|
| Declared | Cell's TOML manifest | Author of the Cell |
| Granted | Agent's role configuration | Operator |
| Permitted | Space's `workspace.toml` | User |

The narrowest constraint at any layer wins. Capabilities can be narrowed through delegation but never widened. Categorically, the intersection is a **pullback** in the category of capability sets. Capability escalation through composition is impossible.

Eleven capability types cover the resource surface: file read, file write, execute (with sandbox level), network (with allow-list), LLM call (with model list), Store read, Store write, Bus publish/subscribe (with topic filters), human escalation, spawn agent, modify plan, custom. Sandbox levels: None, Readonly, Sandboxed (default), Full.

### Five implementation tiers

Cells range from zero-code configuration to native Rust, forming a Spectral Package Interface (SPI):

| Tier | Cell defined by | Capabilities | Audience |
|---|---|---|---|
| T0 Prompts | System prompt text + role config | LLM call only | Domain experts |
| T1 Config | TOML parameters on existing Cells | Varies by base Cell | Power users |
| T2 Declarative Tools | JSON/TOML tool manifests + MCP | Tool execution | Developers |
| T3 WASM | Compiled WASM module | Sandboxed compute | Plugin developers |
| T4 Native Rust | `impl Cell for MyCell` | Full capability set | Core developers |

The visual editor only writes Tiers 1–3, so anything built visually is sandbox-safe by construction.

---

## 8. Graph — Universal Composition

A **Graph** is a typed DAG (or cyclic graph) of Cells connected by edges with optional conditions and data mappings. Graphs are defined in TOML and interpreted by a single execution engine.

The critical property is **fractal composition**: a Graph implements the Cell trait. Any Graph can be embedded as a node inside another Graph. A Pipeline of Pipelines is just a Pipeline. A Loop containing a Graph is just a Loop. This eliminates special glue code between subsystems.

```toml
[graph]
name = "inference-gateway"
version = "1.0.0"

[[graph.nodes]]
id = "loop-detect"
cell = "roko.gateway.loop_detect"

[[graph.nodes]]
id = "cache-lookup"
cell = "roko.gateway.cache_lookup"

[[graph.edges]]
from = "loop-detect"
to = "cache-lookup"
```

A Graph edge `A → B` is legal iff three conditions hold: type compatibility (A's output schema ≤ B's input schema in the TypeSchema preorder), protocol adjacency (at least one of A's protocols composes with at least one of B's per the adjacency matrix), and capability pullback (the intersection of effective capabilities is permitted by the surrounding Space). All three checks happen at Graph load time. A poorly-typed Graph never reaches the executor.

---

## 9. The Nine Protocols

Roko separates **what a Cell can do** (its protocols) from **how it does it** (its implementation). Each of nine protocols captures one role in the universal `query → score → route → compose → act → verify → write → react` loop.

```rust
pub enum ProtocolId {
    Store,    // persist and retrieve
    Score,    // rate quality
    Verify,   // check correctness
    Route,    // select among candidates
    Compose,  // assemble under budget
    React,    // respond to ephemeral events
    Observe,  // read-only telemetry
    Connect,  // external system I/O
    Trigger,  // event ingress
}
```

A single Cell can implement multiple protocols. A `CodeReviewCell` implementing both Score and Verify lives in the product category `Score × Verify`.

### Store — persistence

The primary durable fabric. `put`, `get`, `query`, `query_similar` (native HDC similarity, no external vector store), `prune` (enforces demurrage). Implementations include a JSONL-on-disk substrate, in-memory backends for testing, and chain-backed stores for on-chain attested knowledge.

### Score — five-axis rating

Score Cells rate a Signal along five dimensions given a context (recent neighbors, the prompting query, the current attention focus). They are learners: they predict quality, publish predictions, and receive corrections from gate verdicts via calibration.

### Verify — the load-bearing protocol

The Verify protocol is the single most consequential abstraction in Roko. It serves four roles simultaneously:

1. **Reward function** — continuous `Verdict.reward: f64` for learning.
2. **Relabeling oracle** — hindsight relabeling of failed trajectories (Andrychowicz et al. 2017).
3. **Safety boundary** — `verify_pre()` can veto execution before it happens.
4. **Economic attestation** — passing Verdicts feed reputation flows.

The Verdict has structural separation: **conjunctive hard criteria** (all must pass — there is no scalar to game) and **Pareto soft criteria** (multi-objective, never collapsed to a scalar; lateral moves only). This resists Goodhart's Law by construction. Criterion kinds cover correctness, quality, safety, economic, and semantic dimensions. Evidence is typed separately from Criterion so it can be reused across criteria and aggregated across passes.

The central safety property is the **Variance Inequality**:

```
Var[verifier(x) - truth(x)] < Var[generator(x) - truth(x)]
```

The verifier ensemble must have lower variance on ground-truth benchmarks than the generator. A noisier verifier adds uncertainty rather than resolving it. Three structural mechanisms enforce this: disjoint-family panels (judges from disjoint model families; correlated errors cancel), no self-judgment (a Cell never verifies its own output), and calibration benchmarks (Verify Cells are periodically tested against known ground truth).

For subjective criteria, Verify uses **pairwise comparison** aggregated via Bradley-Terry MLE (Bradley & Terry 1952). This avoids the well-known instability of absolute Likert-scale LLM judgments. Panel design: at least three judges from disjoint model families; no judge from the generator's family; the comparison graph must be connected.

The Verify pipeline sits **outside the modifiable surface**. The agent can choose which Cells to run and how to allocate budget, but cannot add, remove, or reorder Verify heads, modify Verify implementations, or bypass pre-action verification — the execution engine calls it, not the agent. Structural changes require explicit human approval. This is enforced by architecture, not policy.

### Route — selecting among candidates

Route Cells select among candidate Cells, models, or paths. Roko uses **Expected Free Energy** (Friston 2006) rather than LinUCB, decomposing each candidate's value into pragmatic value (expected goal advancement), epistemic value (information gain from uncertain candidates), cost, and a regime-conditioned penalty:

```
EFE(candidate) ≈ pragmatic + explore_weight * epistemic - cost - regime_penalty
```

`explore_weight` shifts with regime (Calm > Normal > Volatile > Crisis). EFE subsumes LinUCB by also modelling the value of reducing uncertainty: an agent that has never tried a cheap model on simple tasks has high epistemic value for that pairing even when the pragmatic expectation is uncertain. EFE naturally produces the progressive cascade (T0 reflex → T1 cheap model → T2 capable model) without hand-coded tier thresholds.

The cascade learns itself through predict-publish-correct on Bus: `prediction.route.{cell_id}` published before selection, `outcome.route.{cell_id}` after the Verdict. A CalibrationReact joins them by lineage and updates per-candidate priors.

### Compose — budget-constrained assembly

Compose Cells assemble multiple Signals into a single output Signal under a budget constraint, using a **VCG auction** (Vickrey 1961, Clarke 1971, Groves 1973) with eight or more context bidders. Eight built-in bidders cover task description, code references, research findings, prior episodes, calibrated heuristics, tool documentation, safety constraints, and distilled knowledge.

Every context section is tracked by a Beta-distribution posterior. After each gate evaluation, the workspace updates posteriors for all included sections. Sections with high posterior mean are boosted in future auctions; sections with low mean are penalized. This is prompt A/B testing at the section level. Novelty attenuation (`1 / (1 + ln(freq))`) keeps habituation from reaching zero, making room for novel context.

VCG makes truthful bidding the dominant strategy: a bidder that lies pays externality costs that exceed its gain.

### React — pulse-driven response

React Cells watch the Pulse stream and emit Signals or Pulses in response. **React operates on Pulses**, not Signals — it is the protocol for ephemeral event response. The CalibrationPolicy that drives predict-publish-correct learning is itself a React Cell. The graduation policy is itself a React Cell. The supervisor that tracks circuit-breaker state is itself a React Cell.

### Observe — read-only telemetry

Observe Cells produce observation Signals without mutating state. Lenses (the runtime's observation primitive) implement Observe. Four principles govern observation: it is passive (removing all Lenses changes nothing about behaviour), compositional (Lenses stack, chain, and scope from Cell up to Global), uses the same primitives as everything else (output is Signal; configuration is TOML), and exposes typed projections as the data contracts to display surfaces.

### Connect — external system I/O

Connect Cells manage lifecycle-bound connections to external systems (RPC endpoints, databases, MCP servers, webhooks, APIs). Five built-in Connect Cells handle the common cases: chain-rpc, mcp, database, webhook, api.

### Trigger — event ingress

Trigger Cells listen for events and fire Graphs in response. Seven built-in trigger kinds — Cron, Webhook, FileWatch, Bus, ChainEvent, Manual, SignalPattern — cover the common ingress paths. The system is push-based and event-driven end to end. There is no polling.

---

## 10. Predict-Publish-Correct

Every Cell is a learner. The structural pattern:

1. Before acting, the Cell publishes a **prediction** Pulse on `prediction.{cell_id}`.
2. The Cell executes and produces output.
3. Reality (gate verdicts, downstream results) publishes an **outcome** Pulse on `outcome.{cell_id}`.
4. A `CalibrationPolicy` (a React Cell) subscribes to both, joins by `lineage_hint`, computes error, publishes `calibration.{cell_id}.updated`.
5. The Cell subscribes to its own calibration topic and adjusts parameters.

Every protocol participates. Score Cells calibrate score predictions against gate verdicts. Verify Cells calibrate verdict predictions against meta-verification. Route Cells calibrate EFE estimates against actual outcomes. Compose Cells calibrate Beta posteriors against gate pass rates.

Why predict-publish-correct rather than a learning subsystem: learning is not a separate module — it emerges from the same pub/sub fabric that carries heartbeats and gate verdicts. Every Cell improves by construction, using the system's own primitives.

---

## 11. Protocol Adjacency and the Free Monad

Not all protocol-to-protocol compositions are meaningful. The runtime enforces an adjacency matrix at Graph load time. Examples: Store → Score is legal (retrieve then rate), Store → React is illegal (Store is pull, React is push), Verify → React is legal (verdict-as-reward signal), React → Store is the only legal Pulse-to-Signal path (graduation).

The 9 protocols define an algebra. The **free monad** over this algebra is the type of all possible Cell programs before any interpretation. This separates *description* from *execution* and gives three benefits:

- **Static analysis**: inspect a `CellProgram` structure before executing to determine resource needs and cost.
- **Interpretation swapping**: same program interpreted by real executor, mock executor, or cost estimator.
- **Optimization**: a compiler pass can fuse adjacent Store operations, batch Score calls, eliminate redundant Verify checks.

### Natural transformations

Several protocol-to-protocol relationships are natural transformations — structure-preserving maps that commute with composition:

| Transformation | What it does |
|---|---|
| `Score ⇒ Verify` | Geometric-mean reward; thresholds become criteria. |
| `Verify ⇒ React` | Every Verdict becomes a Pulse, enabling predict-publish-correct. |
| `React ⇒ Store` | Graduation (the only Pulse-to-Signal path). |
| `Store ⇒ React` | Projection (lossy broadcast of stored data). |

The pair (graduation, projection) is the Pulse–Signal adjunction.

---

## 12. The Four Universal Patterns

Every subsystem in Roko is one of four patterns, not bespoke infrastructure:

| Pattern | Topology | Key property |
|---|---|---|
| **Pipeline** | Linear chain with reject/transform/redirect edges | Sequential processing with early exit |
| **Loop** | Graph with feedback edge | Self-improving via predict-publish-correct |
| **Functor** | Cross-cut enriching Signals pre/post a Cell | Composable orthogonal concerns (no topology change) |
| **Space** | Graph owning Bus + Store partitions | Isolation + collaboration boundary |

These four patterns eliminate architectural proliferation. A new verification pipeline is "just a Pipeline." A new learning mechanism is "just a Loop." A new memory enrichment is "just a Functor." A new tenancy boundary is "just a Space." See the runtime doc for how each pattern is realised in execution.

---

## 13. The Thirteen Specializations

Roko's full vocabulary of pre-built compositions:

| Specialization | Underlying pattern | What it is |
|---|---|---|
| **Flow** | Pipeline | A standard Graph instance running once to completion |
| **Hot Flow** | Pipeline | A Graph that stays resident and re-fires on a clock |
| **Rack** | Graph | A parameterized Graph with explicit Macros (knobs) and Slots (jacks) |
| **Lens** | Observe Cell | Read-only telemetry observer |
| **Loop** | Pattern | A Graph with a feedback edge |
| **Memory** | Store Cell | Store with demurrage, tier progression, and dream consolidation |
| **Extension** | Functor | Interceptor Cell hooked into another Cell's pipeline |
| **Agent** | Composite | Space + Extensions + Memory + adaptive clock + vitality |
| **Connector** | Connect Cell | Lifecycle-bound external connection |
| **Feed** | Connect + Trigger + Store | Continuous data stream as a Cell |
| **Recipe** | Graph | Pure data Graph (no LLM, no agent) |
| **Group** | Space | Persistent agent collective with membership and coordination mode |
| **Pipeline** | Pattern | Linear Graph with conditional edges |

Every pre-built thing is one of these. Every new thing should be one of these.

---

## 14. Provenance and Taint

Every Signal carries a taint classification forming a security lattice:

```
                    Propagated
                   /          \
        LlmGenerated      ExternalFetch
                   \          /
                  UserInput
                      |
                    Clean
```

Taint propagation is **join** (least upper bound): output taint = `Taint::join_all(inputs.taint)`. Compose preserves taint. Derivation preserves taint. Verify does not clear taint — validation is not provenance. Taint can decrease only through explicit human-initiated declassification recorded in the audit trail.

Tainted data is allowed *into* the system (blocking at intake would be censorship). The taint gate operates on **actions**: the riskier the action and the more tainted the context, the stronger the authorization required. Privileged Signal kinds (Declassification, Deployment, ExternalWrite, NetworkEgress, FileDelete) require a valid Custody witness for `Store.put()`.

**Attestation** is orthogonal to taint. Taint tracks **trust** (where data came from); attestation tracks **integrity** (who signed). Three levels: LocalAgent (ephemeral session key), OrgRole (human-held org key), ChainWitness (on-chain independent verifier).

---

## 15. Lineage as a Free Category

The `source: Vec<SignalRef>` field defines edges in a DAG forming a **free category**:

- Objects: Signals (identified by content hash).
- Morphisms: lineage edges (`A` in `B`'s `source` means "A contributed to B").
- Composition: transitive closure.

Acyclicity is enforced structurally — content hashes are computed before storage. Hash stability requires references to be resolvable. Growth is monotonic (Signals are append-only). Fan-in is bounded (max 32 parents) so deeper derivations use intermediate consolidation. Practical traversals are depth-limited (default 100 ancestors) with optional kind filtering.

The `autocatalytic_score` of a Signal is its out-degree in the reversed DAG — how many descendants used it. When the average across the store exceeds about 1.5, the knowledge network is **autocatalytic**: it sustains its own growth.

---

## 16. The Algebra in One Page

| Object | Algebra | Operation |
|---|---|---|
| Signal `content_hash` | Lineage monoid | Append (provenance DAG) |
| Signal `hdc_fingerprint` | Vector semiring | Bind (XOR), Bundle (majority), Permute (rotation) |
| Signal `kind` | Join-semilattice | Compound (lattice join) |
| Signal `taint` | Information flow lattice | Join (least upper bound) |
| Capability sets | Pullback in capability category | Intersection |
| Cell composition | Category Cell | Sequential composition with type compatibility |
| Graduation, Projection | Adjunction | `F ⊣ G` between Pulse and Signal |

These structures are how the runtime achieves composability. Every conforming Cell automatically interoperates with every existing Cell because they all live in the same algebraic universe.

---

## 17. Anti-Patterns Roko Refuses

| Anti-pattern | Roko's response |
|---|---|
| Standalone destination app | Embed in existing surfaces |
| Naive multi-agent debate | Require heterogeneity + structured indirection |
| Opaque marketplace economics | Publish all metrics; transparent take-rates |
| "We have the most data" moat | Protocol composition + workflow embedding |
| Weighted-sum verification | Conjunctive hard + Pareto soft |
| LLM judging itself | Variance Inequality: verifier spectrally cleaner than generator |
| Token speculation | Identity and utility, not token price |
| God files | Composition of small Cells |
| Ad-hoc state | Everything through Bus or Store |
| One-off infrastructure | Express as Cell specialization |
| Naive agent self-modification | Verify outside the modifiable surface |

---

## 18. Compositional Foundations

The kernel above is small and orthogonal because it rests on a body of category-theoretic and algebraic results. The foundations matter because they explain *why* arbitrary Cell compositions remain type-correct, why composition does not leak gradients, why the kernel cannot be replicated by adding features to a framework.

### The composition ceiling

Large language models are the most capable AI systems built. They are also, provably, the wrong architecture for tasks requiring composing learned primitives into novel combinations. Dziri et al. ("Faith and Fate," NeurIPS 2023, arXiv:2305.18654) tested GPT-4 on multi-digit multiplication: 2×2 ≈90% accuracy, 3×3 ≈50%, 4×4 ≈4%, 5×5 ≈0%. The decay is exponential, not gradual. Their explanation: transformers perform "linearized subgraph matching against memorized fragments." When composition depth exceeds memorized examples, the model has no mechanism for chaining computation steps. GSM-Symbolic (Mirzadeh et al., Apple, arXiv:2410.05229) confirmed the diagnosis: minor symbolic perturbations (renaming, swapping numbers, altering surface details while preserving structure) drop LLM accuracy by 10–20 percentage points.

Lippl and Stachenfeld (ICLR 2025, arXiv:2405.16391) moved this from empirical observation to mathematical certainty. Compositionally-structured kernel models — the regime that bounds wide neural networks — are limited to **conjunction-wise additivity**. They can compute sums of values over training-seen feature combinations but cannot perform **transitive generalization of equivalence relations**, the fundamental operation behind open-ended composition. A kernel model that learns "A ≡ B" and "B ≡ C" cannot infer "A ≡ C" unless that combination appeared in training. The ceiling is architectural; more parameters cannot breach a mathematical barrier.

### Category theory as architecture

Category theory studies composition itself. Where set theory asks "what are things?", category theory asks "how do things compose?" That makes it the natural foundation for an architecture designed to compose learned primitives. The claim is not speculative: it rests on peer-reviewed work at top venues, and Symbolica raised $31M USD on the categorical deep learning thesis.

**Parametric lenses** (Cruttwell and Gavranovic, arXiv:2404.00408) formalize a Block as `(P, f, f*)` — a parameter space plus forward and backward maps. Composition is associative by category law: `(f ∘ g) ∘ h = f ∘ (g ∘ h)` is a theorem, not a design goal. Reverse-mode automatic differentiation is a **functor** from `Para(Smooth)` to `Para(Lens(Smooth))` — gradient computation for any composed Block is automatically correct by functoriality. Gavranovic, Lessard, and Velickovic ("Categorical Deep Learning," ICML 2024, arXiv:2402.15332) prove that standard architectures (CNNs, RNNs, transformers, GNNs) are all instances of `Para(Lens(C))`.

**Polynomial functors** (Niu and Spivak, *Polynomial Functors*, Cambridge University Press, 2024) provide the established framework for interaction protocols. Every Cell's input/output protocol is a polynomial functor; type-checking happens at compose time. If two Cells have incompatible protocols, their composition is undefined in the polynomial category — it does not exist as a mathematical object. This eliminates an entire class of runtime errors by making them compile-time impossibilities. Polynomial functors are closed under composition, product, and coproduct: composed protocols are themselves polynomial functors. The type system is fractal.

**DPO hypergraph rewriting** (Double-Pushout) takes a pattern (left), a replacement (right), and an interface (shared boundary), and transforms a host graph by replacing the pattern, gluing along the interface. The **pushout-complement theorem** guarantees type-correctness preservation. Applied to a Graph-of-Cells, any rewrite — adding, removing, rewiring, replacing — automatically preserves type-correctness of the entire Graph. New combinations never seen during training can be constructed by DPO rewriting; their type-correctness is theorem, not test.

### HDC binding breaks the kernel ceiling

HDC binding is **multiplicative** (XOR for binary, circular convolution for real-valued). Three properties: distributivity (`A * (B + C) = A*B + A*C`), invertibility (given `A * B` and `A`, recover `B`), dimensionality preservation. Kernel methods compute additive dot products. The Lippl–Stachenfeld theorem proves conjunction-wise additivity cannot achieve transitive generalization. HDC binding, being multiplicative, is **not subject to the theorem**.

The combination of DPO rewriting and HDC binding is the open-ended composition mechanism. DPO rewrites can construct combinations never seen in training with type-correctness guaranteed. HDC binding produces fingerprints for these novel compositions that preserve algebraic structure (associativity, distributivity, invertibility). Together, they operate in a fundamentally different algebraic regime from kernel methods.

### Empirical proof points

The theoretical argument predicts that architecturally compositional systems should outperform LLMs on compositional reasoning at tiny parameter counts. TRM (Jolicoeur-Martineau et al., Samsung SAIT, arXiv:2510.04871, October 2025) is a 7-million-parameter recursive model: 44.6% on ARC-AGI-1 (outperforming DeepSeek R1, o3-mini, Gemini 2.5 Pro — all with billions of parameters) and 7.8% on ARC-AGI-2. The parameter count comparison is staggering — a factor of roughly 1000×. HRM (Wang et al., arXiv:2506.21734, June 2025) is a 27M-parameter two-timescale architecture: 40.3% on ARC-AGI-1 with only 1,000 training examples and *no pretraining*. "No pretraining" is crucial: HRM achieves these results without internet-scale pretraining because its architecture is structured to compose, not to memorize.

### Five guarantees

The compositional generalization of the Cell-Graph stack is mathematically guaranteed, not architecturally hoped-for:

1. **Differentiability composes by category law.** Every Cell is a parametric lens in `Para(Lens(C))`; composition inherits associativity; reverse-mode AD is a functor.
2. **Polynomial types are closed under composition.** Cell interaction protocols are polynomial functors; the category is closed under composition, product, and coproduct.
3. **DPO rewriting preserves type-correctness.** The pushout-complement theorem makes architectural mutation safe by theorem.
4. **HDC binding exceeds the kernel-additivity ceiling.** Multiplicative composition is structurally different from additive kernel inner products.
5. **Search over compositional space is systematic.** Quality-diversity search combined with active inference biases search toward configurations that minimize surprise.

The claim is not that the system solves every compositional problem; it is that the composition mechanism does not have an inherent ceiling on composition depth. The specific failure modes identified in the empirical work — exponential decay with depth, collapse under symbolic perturbation, kernel-additivity ceiling — are eliminated by mathematical structure, not ameliorated by engineering effort.

### What this means competitively

| Framework | Compile-time types | Composition law | Type-preserving mutation | Formal verification | Compositional encoding |
|---|---|---|---|---|---|
| LangGraph | Runtime TypedDict | None | None | None | None |
| CrewAI | None | None | None | None | None |
| DSPy | Partial Signatures | None | None | None | None |
| **Roko Cell-Graph** | **Polynomial functors** | **Yes (Para(Lens(C)))** | **Yes (DPO + pushout-complement)** | **Yes (TLA+ + deterministic engine)** | **Yes (HDC binding)** |

The advantage is **mathematical**, not engineering. A competitor with twice the engineering team cannot replicate the guarantees by writing more code; they follow from architectural choices made at the foundation. Replicating them requires rebuilding from those foundations.

---

## 19. External Citations Used Throughout

The principles above draw on a deliberately narrow body of theory. Each is cited once where it first appears, then referenced by short tag thereafter.

- **Active inference / Free Energy Principle**: Friston, K. (2006). A free energy principle for the brain. *Journal of Physiology — Paris*, 100(1–3).
- **Hyperdimensional computing**: Kanerva, P. (2009). Hyperdimensional computing. *Cognitive Computation*, 1(2). Heddes et al. (2023). Torchhd. *JMLR* (arXiv:2205.09208). Frady et al. (2020). Resonator networks. *Neural Computation*, 32. Plate (2003). *Holographic Reduced Representation*. CSLI. Levy & Gayler (2008). VSA. *Artificial Intelligence*.
- **Demurrage**: Gesell, S. (1916). *The Natural Economic Order*.
- **Stigmergy**: Grassé (1959). Dorigo, M. (1992). *Optimization, learning and natural algorithms* (PhD thesis).
- **Calibration**: Brier, G. W. (1950). *Monthly Weather Review*, 78(1). Guo et al. (2017). On calibration of modern neural networks. *ICML*. Naeini et al. (2015). Bayesian binning calibration. *AAAI*.
- **VCG mechanism**: Vickrey (1961), Clarke (1971), Groves (1973).
- **Bradley–Terry**: Bradley, R. A., & Terry, M. E. (1952). *Biometrika*, 39(3/4).
- **Hindsight relabeling**: Andrychowicz et al. (2017). Hindsight Experience Replay. *NeurIPS*.
- **Information flow lattice**: Denning, D. E. (1976). *CACM*, 19(5).
- **Falsificationism**: Popper, K. (1934). *The Logic of Scientific Discovery*.
- **Category theory**: Mac Lane, S. (1971). *Categories for the Working Mathematician*. Springer. Awodey (2010). *Category Theory*. Oxford.
- **Parametric lenses**: Cruttwell, Gavranovic (arXiv:2404.00408, March 2024). Gavranovic, Lessard, Velickovic (ICML 2024, arXiv:2402.15332).
- **Polynomial functors**: Niu, Spivak (Cambridge University Press 2024). *Polynomial Functors*.
- **Kernel-additivity ceiling**: Lippl, Stachenfeld (ICLR 2025, arXiv:2405.16391).
- **Composition empirics**: Dziri et al. (NeurIPS 2023, arXiv:2305.18654). Mirzadeh et al. (Apple, arXiv:2410.05229). TRM (Samsung SAIT, arXiv:2510.04871, October 2025). HRM (arXiv:2506.21734, June 2025).

The next document, [Runtime](./02-runtime.md), describes how Graphs of these protocol-conforming Cells actually run, how Agents work on top of the engine, and how learning operates as predict-publish-correct.
