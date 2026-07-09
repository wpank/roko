# HDC Deep Integration: Where Everything Connects

## What Roko Already Has

### The HDC Primitives (roko-primitives)

10,240-bit binary vectors (`[u64; 160]`, 1,280 bytes each) with:
- **Bind** (XOR) — role-filler composition, self-inverse
- **Bundle** (majority vote) — superposition/averaging
- **Permute** (cyclic shift) — sequence/position encoding
- **Similarity** (Hamming/popcount) — [0.0, 1.0], sub-microsecond
- **DecayingBundleAccumulator** — temporal weighting, recent vectors bias higher
- **ItemMemory** — brute-force nearest-neighbor codebook
- **Codebook** — deterministic symbol allocation, domain-scoped
- **PatternStore** — labeled patterns with similarity queries
- **Cross-domain resonance** — threshold 0.526 (3σ above random)
- **CodingCodebook** — 16 pre-allocated software engineering symbols

### Where HDC Is Used Today

| Component | What it fingerprints | Persists to |
|-----------|---------------------|-------------|
| **Episode logger** | prompt + outcome per agent turn | `.roko/episodes.jsonl` (base64) |
| **Neuro knowledge store** | knowledge entries with causal links | `.roko/neuro/knowledge.jsonl` |
| **Code indexer** | symbols (function/struct/trait) + files | roko-index workspace |
| **Pattern discovery** | action trigrams across episodes | Pattern signatures |
| **Resonant patterns** | evolutionary genomes with Lotka-Volterra dynamics | Fitness-weighted populations |
| **HDC clustering** | k-medoids over episode fingerprints | CompressedEpisodeSummary |
| **Orchestrate.rs** | attaches fingerprint per episode at runtime | Episodes (lines 9609, 11327) |

### What's Built But Not Connected

| Component | Status | Gap |
|-----------|--------|-----|
| **Chain** | `HdcVector(pub [u64; 160])` stub in identity_economy | Not wired to primitives crate |
| **Cross-domain resonance** | Implemented in codebook.rs + neuro/hdc.rs | Not called from orchestrator |
| **Episode clustering** | k-medoids implemented | Not run on accumulated episodes |
| **Resonant patterns** | Lotka-Volterra dynamics implemented | Not wired to dispatch decisions |
| **Knowledge → prompt injection** | Neuro store has HDC queries | Not consulted at compose time |
| **Somatic markers** | k-d tree in daimon | Not populated from episodes |

---

## The Deep Integration Map

### Level 1: Per-Episode HDC (already working)

Every agent turn produces a fingerprint. This is the raw signal.

```
Agent turn → prompt + outcome → fingerprint_episode() → 10,240-bit vector
  → stored in episodes.jsonl
```

### Level 2: Episode Clustering (built, not wired)

Periodically, accumulated episode fingerprints get clustered:

```
N episodes → k-medoids(fingerprints, k) → clusters
  → each cluster: medoid vector + pass_rate + avg_cost
  → CompressedEpisodeSummary with hdc_superposition (bundled)
```

**What this enables**: Instead of searching all episodes, you search cluster medoids.
A new task's fingerprint gets compared to medoids → jump to the right cluster →
retrieve only relevant episodes for context injection.

### Level 3: Cross-Domain Resonance (built, not wired)

The resonance detector finds structural analogies:

```
Pattern from SWE-bench arena (similarity = 0.54 > 0.526 threshold)
  ≈ Pattern from self-hosting arena

"retry after import error" in Django ≈ "retry after borrow checker" in Rust
```

**What this enables**: Playbooks transfer between domains. A successful strategy
in one arena automatically surfaces as a candidate in structurally similar
situations in another arena. The HDC similarity is the mechanism that discovers
these analogies — no semantic understanding required, just vector geometry.

### Level 4: Knowledge-Informed Routing (not wired)

The neuro store has HDC-encoded knowledge entries. At dispatch time:

```
New task fingerprint → query neuro store by similarity
  → retrieve relevant insights, heuristics, warnings
  → inject into system prompt via SystemPromptBuilder
  → also: inform CascadeRouter model selection
```

**What this enables**: The system remembers what worked. "Last time I saw a task
fingerprint similar to this one, model X succeeded and model Y failed." This is
knowledge-informed agent routing — priority #13 in CLAUDE.md's roadmap.

### Level 5: Somatic Markers (built, not wired)

Daimon's somatic markers are a k-d tree of past strategy outcomes:

```
(strategy_vector, outcome) pairs accumulated over episodes
  → k-d tree enables sub-millisecond "gut feeling" queries
  → before analytical reasoning, check: "has this approach worked before?"
```

**What this enables**: Fast-path decision making. Before spinning up an expensive
LLM call, check the somatic k-d tree. If a near-identical situation had a clear
winner, skip the exploration and go straight to the known-good approach.

### Level 6: Distributed HDC via Chain (Phase 2)

This is where it gets genuinely novel:

```
Local episodes → fingerprints → cluster medoids → bundle into knowledge vector
  → Merkle tree of knowledge vectors
  → root hash committed on-chain (cheap: ~600 gas for XOR bind)
  → raw vectors stored in P2P layer (DHT / IPFS)
  → other roko instances query by similarity
  → pull relevant clusters → integrate into local neuro store
```

**On-chain operations** (feasible and cheap):
- Store Merkle roots of knowledge collections (~20K gas per SSTORE)
- HDC bind/similarity as precompile (~600-800 gas per 10,240-bit operation)
- Reputation as similarity to "ideal performance" reference vector
- Agent identity bound to accumulated knowledge fingerprint

**Off-chain** (P2P layer):
- Raw vectors in DHT, LSH-indexed for similarity queries
- Merkle proofs for inclusion verification
- Privacy-preserving sharing via PP-HDC hash-encoding (not raw binding, which is invertible)

---

## The Three Nested Loops With HDC

### Inner Loop: Per-Turn Fingerprinting (milliseconds)

```
Task → agent turn → fingerprint(prompt, outcome)
  → episode logged with HDC fingerprint
  → somatic marker check (k-d tree, sub-ms)
  → if similar to past failure: escalate model tier immediately
```

HDC role: **instant pattern recognition**. Before the LLM even runs, the somatic
marker system checks "have I been in this situation before?" in sub-millisecond time.

### Middle Loop: Cross-Episode Learning (per-batch)

```
Batch of episodes → k-medoids clustering
  → cross-domain resonance detection
  → playbook extraction from successful clusters
  → CascadeRouter update with cluster-level features
  → prompt experiment attribution per cluster
```

HDC role: **structural analogy discovery**. Finding that "retry after type error"
in TypeScript behaves like "retry after lifetime error" in Rust — not because the
words are similar, but because the vector geometry of the episodes is similar.
This is something word embeddings can't do because the representation is compositional
(role-filler binding preserves internal structure).

### Outer Loop: Distributed Knowledge Sharing (periodic)

```
Local knowledge vectors → Merkle tree → root hash on-chain
  → other instances discover via chain events
  → query similar knowledge from P2P store
  → integrate into local neuro store
  → local clustering incorporates external knowledge
```

HDC role: **communication-efficient knowledge sharing**. A 10,240-bit vector
encodes a full episode cluster (medoid) in 1,280 bytes. Compare to neural
embeddings at 1.6-16 KB per vector, or full episode logs at 10-100 KB each.
The compression ratio is extreme, and similarity is preserved.

---

## Where HDC Fits In The Arena Framework

Revisiting the arena architecture from 04-generalized-arenas.md:

```rust
trait Arena {
    fn sample(&self, batch_size: usize) -> Vec<TaskDef>;
    fn gates_for(&self, task: &TaskDef) -> Vec<GateConfig>;
    fn score(&self, episodes: &[Episode]) -> ArenaScore;
}
```

HDC enriches every stage:

1. **Before sampling**: Query somatic markers for "what type of task should I practice next?"
   based on cluster analysis of past failures (curriculum learning via HDC).

2. **Before dispatch**: Fingerprint the task, query neuro store for similar past tasks,
   inject relevant knowledge into the prompt.

3. **Model selection**: CascadeRouter uses HDC cluster features as context vector
   dimensions (which cluster does this task belong to? → route to the model that
   performs best for this cluster).

4. **After gate**: Fingerprint the episode, update clusters, detect resonance with
   other domains, extract playbooks from new successful clusters.

5. **Across arenas**: Cross-domain resonance detection finds structural analogies
   between arenas, enabling transfer learning without explicit domain knowledge.

6. **Across instances**: Merkle-committed knowledge vectors shared via chain,
   enabling network effects where each roko instance benefits from all others' experience.

---

## What Needs Wiring

### Tier 1: Wire existing HDC code into runtime (no new code needed)

1. **Episode clustering on schedule** — Run k-medoids every N episodes (k_medoids is
   implemented, just not called from orchestrate.rs)

2. **Cross-domain resonance at dispatch time** — Query pattern store before composing
   prompt (detect_cross_domain_resonance exists, not called)

3. **Knowledge → prompt injection** — Neuro store HDC queries at compose time
   (query_by_role exists, not called from SystemPromptBuilder)

4. **Somatic marker population** — Feed episode outcomes into daimon's k-d tree
   (somatic_ta exists, not populated at runtime)

### Tier 2: Connect CascadeRouter to HDC features (small new code)

5. **Cluster-aware context vector** — Add episode cluster ID to the 18-dim
   CascadeRouter context vector. "This task's fingerprint is in cluster 7,
   where model X has 85% pass rate."

6. **Resonant pattern fitness → routing bias** — Patterns with high Lotka-Volterra
   fitness get promoted in the system prompt. Dying patterns get pruned.

### Tier 3: Chain integration (Phase 2)

7. **On-chain knowledge commitments** — Merkle root of medoid vectors on-chain

8. **P2P knowledge exchange** — DHT-based similarity queries across instances

9. **Reputation as vector similarity** — Agent reputation = similarity to
   accumulated "ideal agent" performance vector

---

## Novel Capabilities This Enables

### 1. Curriculum Learning via HDC Clustering

Instead of random sampling from a benchmark, the arena can use HDC clustering
to identify *which tasks the system is worst at*:

```
All episodes → cluster by fingerprint → compute per-cluster pass rate
  → sample preferentially from low-pass-rate clusters
  → system practices its weaknesses, not its strengths
```

This is active learning / curriculum learning, but purely geometric — no
semantic understanding of "what kind of task" it is. The HDC fingerprint
captures the structural signature of the task, and clustering groups
structurally similar tasks.

### 2. Compositional Analogy Transfer

HDC's bind operation creates compositional representations that preserve
internal structure. This means:

```
bind(LANGUAGE, rust) XOR bind(LANGUAGE, python)
  = rust XOR python (the "difference vector" between languages)

bind(ERROR_TYPE, borrow_check) + (rust XOR python)
  ≈ bind(ERROR_TYPE, reference_error) (the Python equivalent)
```

If a playbook works for "borrow check errors in Rust", the system can
algebraically compute what the equivalent playbook would be for "reference
errors in Python" — and check if it exists in the pattern store.

This is something neural embeddings *cannot do* because they lack the
algebraic structure. HDC's bind/bundle/permute form a mathematical group
that supports exact compositional reasoning.

### 3. Privacy-Preserving Knowledge Sharing

Using PP-HDC's hash-encoding (not raw binding, which is invertible):

```
Local knowledge vector → hash-encode (distance-preserving, non-invertible)
  → publish to P2P network
  → other instances compute similarity on encoded vectors
  → retrieve relevant knowledge without seeing raw episodes
```

Agents share "what they know works" without revealing "what they were working on."
This enables competitive scenarios where multiple organizations run roko instances
and share learning while keeping their specific tasks private.

### 4. Evolutionary Pattern Competition

The Lotka-Volterra dynamics in resonant_patterns.rs create an ecological model
where patterns compete for "attention share":

```
Pattern A (fitness=0.8, population=0.3) competes with
Pattern B (fitness=0.6, population=0.5)

Competition coefficient = similarity(genome_A, genome_B)
  → similar patterns compete harder (same ecological niche)
  → diverse patterns coexist

Population dynamics: dN/dt = r*N*(1 - (N + Σ a_ij*N_j) / K)
```

Over time, the pattern ecosystem converges: high-fitness patterns survive,
low-fitness ones die out, and similar patterns merge into the same niche.
The result is a minimal set of maximally diverse, high-performing strategies.

This is a genuine selection pressure on strategies, driven purely by HDC
vector geometry. The system evolves its own playbook.

---

## Key Research References

- **HD-CB** (IEEE Jan 2025): HDC for contextual bandits — replaces LinUCB's
  ridge regression with parallel vector operations. Directly relevant to
  replacing/augmenting CascadeRouter.

- **FedHDC** (ACM 2024): Federated HDC with 66x communication reduction.
  The blueprint for multi-instance knowledge sharing.

- **Emergent Collective Memory** (2025): Stigmergic coordination via HDC
  with critical density threshold ρ_c ≈ 0.23.

- **BiHDTrans** (2025): Binary Hyperdimensional Transformer — binarizing in
  HD space loses less information than directly binarizing neural weights.
  Relevant to distillation of LLM knowledge into HDC representations.

- **PP-HDC** (2024): Privacy-preserving HDC with <1% accuracy loss.
  Required for safe cross-instance knowledge sharing.

- **HDCoin** (2022): Proof-of-useful-work where mining = HDC training.
  Precedent for on-chain HDC computation.
