# Hyperdimensional Substrate

> **TL;DR**: Every Engram should carry a 10,240-bit HDC fingerprint as
> a first-class field, not as an optional side-table. Doing so turns
> similarity, consensus, stigmergy, analogy, and compositional memory
> into O(1) vector ops over a fabric that already exists. The `roko-primitives`
> crate is scaffolded for this; the Bus makes it the standard currency
> for *agreement* between agents.

> **For first-time readers**: Hyperdimensional Computing (HDC) represents
> concepts as very long (10,240-bit) binary vectors. Two random vectors of
> that length are nearly orthogonal; meaningful structure is composed by
> XOR (bind), majority-vote (bundle), and cyclic shift (permute). Cosine /
> Hamming similarity between two such vectors is ~1 ns with SIMD popcount.
> `roko-primitives` already has `HdcVector`; this doc proposes making it a
> field on every Engram (not just a side-table) and building the core
> operations around it.

## 1. Why HDC is the right choice for Roko

Hyperdimensional Computing (Kanerva 2009, "Hyperdimensional Computing:
An Introduction to Computing in Distributed Representation") uses very
high-dimensional binary or bipolar vectors (10,000+ bits) to represent
symbols, concepts, structures, and sequences in a uniform space. The
properties we need:

1. **Near-orthogonality**: two random HD vectors are nearly orthogonal.
   Noise tolerance is exponential in dimension.
2. **Compositionality**: bind (⊗, XOR for binary), bundle (+, majority
   vote), permute (ρ, cyclic shift) compose into structured meaning.
3. **Fast similarity**: cosine / Hamming over 10,240 bits is a cache
   line plus SIMD popcount — ≈1 ns per comparison.
4. **Holographic**: partial information still retrieves. Damage one
   fraction; the rest still works.
5. **Content-addressable**: similar inputs produce similar vectors
   without an index.

These properties are what the Substrate *pretends* to have today via
BLAKE3 content addressing (unique retrieval) and what it *needs* to
have to support collective intelligence (similarity-based consensus).
HDC is the bridge.

## 2. What `roko-primitives` already provides

`roko-primitives` has:
- `HdcVector` type (10,240 bits)
- Tier routing that uses it (partial wiring)
- Similarity computation

What it doesn't have:
- A first-class Engram field for the fingerprint
- A canonical encoder from Engram body → HDC vector
- HDC-based Substrate query primitives
- Integration with the Bus as a consensus channel

## 3. The missing field on Engram

Proposed addition to `roko-core::engram::Engram`:

```rust
pub struct Engram {
    // ... existing fields ...
    /// Hyperdimensional fingerprint of this Engram's content.
    /// Populated by the Substrate at `put()` time. None only if
    /// encoder is unavailable or disabled.
    pub fingerprint: Option<HdcVector>,
}
```

The fingerprint is computed deterministically from (kind, body) via an
encoder registered in `roko-primitives`. This makes it:

- Deterministic (same input → same fingerprint)
- Reproducible across nodes (deterministic encoder)
- Cheap to compute (microseconds)
- Cheap to compare (nanoseconds)

## 4. HDC-based Substrate queries

Today `Substrate::query` is filter-based: "give me all Engrams of Kind
X in tier Y from time T1 to T2". Add:

```rust
pub trait Substrate {
    // existing query(...) method stays

    /// Find Engrams whose fingerprint is within `radius` of `fp`.
    /// Returns up to `limit` results sorted by similarity descending.
    fn query_similar(
        &self,
        fp: &HdcVector,
        radius: f32,
        limit: usize,
    ) -> Vec<(EngramHash, f32)>;
}
```

This is *not* semantic search over an external embedding index. It's
native similarity over content we've already stored. Every Engram is
queryable by similarity the moment it lands.

### 4.1 Scale

At 10,240 bits per fingerprint, a single 1 GB RAM buffer holds about
800,000 fingerprints. Brute-force cosine (SIMD) comparison is
~10^9/second on a modern CPU. So `query_similar` against 800k Engrams is
< 1 ms. For larger scales, existing LSH techniques over HDC give
sub-ms retrieval at tens of millions.

This is the "competitive moat" at the tier: an agent runtime with
native, millisecond-latency similarity over its entire memory. Compare
to external vector stores (Pinecone, Weaviate): round-trip alone is
usually 10–100 ms, before query.

## 5. HDC as consensus currency

Consensus among agents today is a human or an orchestrator arbitrating
between outputs. With HDC:

### 5.1 Bundle-based consensus

Two agents produce outputs A and B on the same task. The Substrate
computes `fingerprint(A)` and `fingerprint(B)`. Consensus vector =
`bundle(fp_A, fp_B)`. If `similarity(fp_A, bundle) > threshold` and
`similarity(fp_B, bundle) > threshold`, the two agents *substantively
agree* (in the holographic sense) despite surface differences. This
catches "both said correct thing with different words".

### 5.2 Stigmergic pheromones as HDC vectors

Doc 09 §3 introduced stigmergy as Engrams in a shared Substrate. With
HDC, *pheromone strength along a direction* is represented as the HDC
vector's similarity to a "reward direction" vector. Deposits that point
in the same direction reinforce via bundling; deposits that point
elsewhere stay orthogonal. Natural, fast, and emergent.

This is the computational realization of ant colony optimization (Dorigo
1992) but with *semantic* pheromones rather than scalar counts.

### 5.3 Consensus Bus topic

`consensus.proposal.made` carries an HDC vector plus a claim. Each
subscribing agent publishes `consensus.vote.cast` with its own HDC
vector relative to the proposal. The orchestrator bundles the votes
and publishes `consensus.achieved` or `consensus.failed` based on
bundle similarity to the original proposal.

Three advantages over token-based voting:
- Disagreements about wording vs. substance become visible.
- Partial agreements can be detected ("agents agree on X, differ on Y").
- Swarm scales: bundling N vectors is still one vector.

## 6. Compositional memory via bind/bundle

The magic of HDC is that you can represent *structures* by composition:

```text
fp(turn_5) = bind(role_vector, agent_A) + bind(task_vector, T123) +
             bind(output_vector, output_hash) + bind(time_vector, t5)
```

Every Engram's fingerprint encodes its role, task, author, time, and
content in *one vector*. Queries can then decompose:

- "what was agent A doing at time t5?" → query_similar to
  `bind(agent_A, time_t5)`.
- "which Engrams relate to task T123?" → query_similar to
  `bind(task_vector, T123)`.

This replaces N indexes with one vector space. The Substrate becomes
holographic: every fingerprint carries its own context.

## 7. HDC as the decay mechanism

The existing decay model (`None`, `HalfLife`, `Ttl`, `Ebbinghaus`)
operates on Engram weights. HDC gives us a subtler decay: *vector noise
accumulation*. An Engram's effective fingerprint can be a weighted
blend of its original fingerprint and a noise vector, with the noise
weight growing over time. Old Engrams become *fuzzier* rather than
gone:

- A 1-year-old Engram matches broad categories but not specific ones.
- A 1-hour-old Engram matches both.

This is biologically faithful — human memory gets more categorical and
less episodic over time — and it's what enables *generalization* in a
Substrate. The Neuro tier-progression loop (Phase 4) uses exactly this
to promote specific episodes to semantic knowledge: as fingerprints
drift, similar ones cluster, and a cluster-center becomes a new
category Engram.

## 8. Anti-hallucination via HDC consistency

A hallucination is a claim whose fingerprint doesn't match the
fingerprints of its claimed supporting evidence. Concretely:

1. Agent produces output O claiming to be about X.
2. Substrate computes fp(O).
3. Substrate queries for Engrams with lineage tagged as supporting X.
   These have their own fingerprints.
4. If fp(O) is far from the bundle of supporting fingerprints, the
   output is *semantically disconnected* from its claimed support.
5. A `ConsistencyGate` subscribes to `agent.turn.completed`, runs this
   check, and publishes `gate.hallucination.detected` if threshold
   exceeded.

This isn't absolute hallucination detection — only semantic-drift
detection — but combined with provenance chains it's the highest-value
signal available without external truth. (The Gate pipeline then
cascades to expensive verifiers only on flagged cases, saving 90%+ of
verifier cost.)

## 9. Analogy and few-shot via HDC

Classical HDC result (Kanerva 1994, "The Binary Spatter Code"): analogy
solves via vector arithmetic. "Paris is to France as Tokyo is to ___"
becomes `fp(Tokyo) + (fp(France) - fp(Paris))`; query_similar returns
Japan. In Roko:

- Analogy-driven playbook retrieval: given a new task whose fingerprint
  is `fp(new)`, find the playbook whose fingerprint is
  `fp(playbook) ≈ fp(new) + (fp(old_task) - fp(old_playbook))`.
- Few-shot prompt construction: the Composer picks examples whose
  fingerprint-difference to the current input matches the successful
  pattern of a prior winning prompt.

These are not research projects; they're three-line queries against
the Substrate.

## 10. HDC as meta-state

Agents can carry an *identity fingerprint* — the bundle of all Engrams
authored by them over the last K turns. An agent's identity drifts as
they work. Observable properties:

- Two agents with similar identity fingerprints are doing similar work
  (automatic team discovery).
- An agent whose identity fingerprint changed sharply has changed
  domains (algedonic signal to orchestrator).
- Agent identity vectors can be *composed* via bind: `identity(A) +
  identity(B) - identity(C)` = "A and B, but not doing what C does".
  This is the algebraic foundation for team-building policies.

## 11. What to implement (concrete tasks)

### 11.1 Phase B.5 (between kernel landing and subsystem migration)

1. Add `fingerprint: Option<HdcVector>` to `Engram`. Default None.
2. Register a default encoder in `roko-primitives`: hash each bytestring
   word into HD space, bundle words with position-bind.
3. `FileSubstrate::put` populates the fingerprint at insert time if not
   already set. `FileSubstrate::query_similar` implemented via
   brute-force scan (fine for <1M Engrams).
4. `fingerprint` exposed on all HTTP/REST routes that return Engrams.
5. TUI F7 Substrate tab gains a "Similar to…" search box.

### 11.2 Phase C.5 (HDC consensus)

1. Bus topics: `consensus.proposal.made`, `consensus.vote.cast`,
   `consensus.achieved`, `consensus.failed`.
2. A `ConsensusPolicy` in `roko-learn` that accumulates votes and
   publishes outcomes.
3. `query_similar` used by Router when selecting among candidate
   Engrams (not only by score, but also by similarity to prior
   winners).

### 11.3 Phase D (HDC-native operations)

1. HDC-based `Kind::Playbook` retrieval: analogy-driven.
2. ConsistencyGate deployed as stream-gate in `roko-gate`.
3. HDC-powered Dreams consolidation: fingerprint clustering picks
   Engrams for promotion.

## 12. Why this is a competitive moat

Three converging facts:

1. No agent framework today has HDC as a core primitive. LangChain,
   LlamaIndex, CrewAI, AutoGen all rely on external vector stores for
   similarity. That's a 10–100 ms tax on every retrieval.
2. HDC has a 20-year research literature with concrete algorithms for
   binding, unbinding, cleanup, analogy, and sequential representation.
   None of this requires model training.
3. HDC is fundamentally compatible with the `Engram` concept —
   content-addressed, deterministic, compositional. Roko's data model
   was already HDC-shaped before anyone noticed.

An HDC-native Substrate is thus a moat Roko can build in weeks that
would take competitors months to replicate, because it requires
changing their core data model rather than bolting on a library.

## 13. Academic lineage

- **Kanerva 2009**: hyperdimensional computing foundational paper.
- **Plate 2003**: Holographic Reduced Representations — the real-valued
  analog.
- **Rachkovskij 2001**: binary spatter codes.
- **Levy & Gayler 2008**: vector-symbolic architectures survey.
- **Rahimi & Recht 2007**: random feature maps, theoretical tie to HD
  computing.
- **Olshausen & Field 1996**: sparse distributed representations in
  cortex — biological precedent.

This is a mature field, not speculative. The engineering path is
clear. Each citation becomes a Paper Engram once
`16-research-to-runtime.md` lands; the capacity and
near-orthogonality claims become testable hypotheses in Roko's own
replication ledger.

## 14. Canonical encoder — the default implementation

For Phase B.5 (per §11.1) the default Engram encoder needs to be
simple, deterministic, and fast. A rough sketch:

```rust
// roko-hdc/src/encoder.rs (new crate per 20-modularity-composability.md §2.2)
pub struct DefaultEncoder {
    word_memory: Arc<WordMemory>,   // hash -> HdcVector for reproducibility
    dim: usize,                     // 10,240 by default
}

impl DefaultEncoder {
    /// Encode an Engram's kind + body into a deterministic fingerprint.
    ///
    /// For textual bodies: tokenize, look up per-word vectors from
    /// word_memory (created on first sight, cached thereafter),
    /// permute by position, bundle. For structured bodies (JSON),
    /// bind each (key, value) pair and bundle the results.
    pub fn encode(&self, e: &Engram) -> HdcVector {
        let mut acc = HdcVector::zero(self.dim);
        acc = acc.bundle(&self.word_memory.for_kind(&e.kind));
        match &e.body {
            Body::Text(s) => acc = acc.bundle(&self.encode_text(s)),
            Body::Json(v) => acc = acc.bundle(&self.encode_json(v)),
            Body::Bytes(b) => acc = acc.bundle(&self.encode_bytes(b)),
        }
        for (k, v) in &e.tags {
            let kv = self.word_memory.for_key(k)
                .bind(&self.word_memory.for_value(v));
            acc = acc.bundle(&kv);
        }
        acc
    }

    fn encode_text(&self, text: &str) -> HdcVector {
        let mut acc = HdcVector::zero(self.dim);
        for (pos, word) in text.split_whitespace().enumerate() {
            let wv = self.word_memory.for_word(word);
            acc = acc.bundle(&wv.permute(pos as u32));
        }
        acc
    }

    fn encode_json(&self, v: &serde_json::Value) -> HdcVector {
        use serde_json::Value::*;
        match v {
            Object(o) => {
                let mut acc = HdcVector::zero(self.dim);
                for (k, vv) in o {
                    let kv = self.word_memory.for_key(k)
                        .bind(&self.encode_json(vv));
                    acc = acc.bundle(&kv);
                }
                acc
            }
            Array(a) => {
                let mut acc = HdcVector::zero(self.dim);
                for (i, vv) in a.iter().enumerate() {
                    acc = acc.bundle(&self.encode_json(vv).permute(i as u32));
                }
                acc
            }
            String(s) => self.encode_text(s),
            Number(n) => self.word_memory.for_number(n.as_f64().unwrap_or(0.0)),
            Bool(b) => self.word_memory.for_bool(*b),
            Null => HdcVector::zero(self.dim),
        }
    }

    fn encode_bytes(&self, b: &[u8]) -> HdcVector {
        // Hash bytes into a vector; for binary bodies this loses
        // semantic content but gives uniqueness. Specialized binary
        // encoders can be registered per-kind.
        HdcVector::from_hash(blake3::hash(b).as_bytes(), self.dim)
    }
}
```

`word_memory` is a cache — same word always produces the same vector
within a deployment. Cross-deployment determinism is achieved by
seeding `word_memory` from a BLAKE3 hash of the word (or key), so any
two deployments with the same seed produce the same vectors.

## 15. Encoder plurality

The default encoder above is generic. Specific Kinds benefit from
specialized encoders:

- `Kind::Plan` — encode tasks in order via position-permute, bind
  task IDs with dependency edges.
- `Kind::GateVerdict` — bind gate-name with pass/fail vector; bundle.
- `Kind::Transaction` (Phase 2+) — bind from/to addresses, amount,
  chain id.

Specialized encoders register via `roko_hdc::register_encoder::<K>(...)`.
The default catches everything else. This is the HDC analog of
trait-based dispatch: the kernel knows one function; domains
specialize it.

## 16. Cross-synergies with other refinements

HDC is one of the most load-bearing refinements because it multiplies
the value of everything else:

- **Demurrage (12)** uses HDC neighbor similarity to weight
  reinforcement: citing a rare Engram bumps balance more than citing a
  common one.
- **c-factor (13)** §2.2 — cognitive diversity is the pairwise
  distance between agents' HDC clouds. Without HDC, this metric has no
  efficient implementation.
- **Heuristics (14)** §4 — worldview clustering is community
  detection on heuristic fingerprints.
- **Research-to-runtime (16)** §2 — paper fingerprints let the
  Composer pull in the "most similar paper" for a situation without a
  separate embedding service.
- **Compose (beyond 21 §2.5)** — prompt construction via HDC cleanup
  picks templates whose fingerprint is closest to the current
  situation.
- **StateHub projection hashing (26)** — deduplicating identical
  deltas across consumers uses fingerprint-equality.

## 17. What can go wrong

HDC isn't magic. Three failure modes to watch:

1. **Encoder drift** — two encoders producing different vectors for
   the same input. Mitigation: encoder version in the fingerprint
   metadata; Substrate refuses to mix. A `fingerprint.encoder_version`
   field goes alongside the vector.
2. **Capacity exhaustion** — in theory 10,240 bits hold enormous
   structure, but bundling too many items crowds the space. The
   cleanup-to-codebook primitive helps, and per-Engram encoders
   shouldn't bundle more than ~1000 atomic items in a single vector.
   For bigger structures, compose a small set of sub-fingerprints
   lazily rather than one giant vector.
3. **Near-duplicates confusing retrieval** — if two Engrams are
   fingerprint-similar but semantically different (e.g. two
   error stacks that share keywords but are unrelated), retrieval
   returns both. Fix with tag-binding: `fp(stack) · fp(error_code)`
   separates them even when the text overlaps.

These aren't theoretical; they're what operators will encounter and
should be documented in the eventual `docs/00-architecture/XX-hdc.md`
chapter.
