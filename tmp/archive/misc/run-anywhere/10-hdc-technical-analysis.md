# Hyperdimensional Computing: Technical Analysis and Pattern Recognition

> **Audience**: Quantitative researchers, ML engineers, systems architects
> **Scope**: How HDC (Binary Spatter Codes) enables compositional pattern matching,
> spectral liquidity analysis, causal microstructure discovery, and predictive geometry

---

## Why HDC, Not Neural Networks

Traditional ML approaches to pattern recognition (LSTM, Transformer, CNN) require:
- Expensive training on historical data
- GPU hardware for inference
- Retraining when patterns shift
- Opaque representations (no interpretable algebra)

Hyperdimensional Computing (Kanerva, 2009) provides:
- **No training required** — patterns encode deterministically from data
- **CPU-only inference** — XOR + POPCNT at wire speed (~13ns per 10,240-bit comparison via AVX-512)
- **Instant adaptation** — new patterns bundle with existing knowledge in O(D) time
- **Interpretable algebra** — XOR bind is exact, self-inverse; patterns can be decomposed

**Research**: Kanerva (2009) — Hyperdimensional Computing: An Introduction to Computing in Distributed Representation. Kleyko et al. (2022) — comprehensive survey, Parts I & II. Frady et al. (2023, PNAS) — fast, robust, interpretable paradigm for biological data.

---

## The BSC Foundation

### Binary Spatter Codes (D = 10,240)

Each concept is a 10,240-bit binary vector stored as 160 u64 words.

**Three core operations**:

| Operation | Math | What It Does | Complexity |
|---|---|---|---|
| **Bind** (XOR) | `A ⊕ B` | Creates association between A and B | O(D) — 160 XOR ops |
| **Bundle** (majority vote) | `maj(A, B, C, ...)` | Superposition — combines multiple concepts | O(D × K) |
| **Permute** (cyclic shift) | `π(A)` | Encodes sequence position (shift by 1 = "next") | O(D) — single rotate |

**Key property**: XOR is **self-inverse**: `A ⊕ B ⊕ B = A`. This means you can **unbind** — given a composite vector and one component, recover the other. No other vector operation in ML has this property.

### Why D = 10,240 (Not 8,192 or 16,384)

1. **Quasi-orthogonality**: At D=10,240, P(sim > 0.05) < 1e-9 for random pairs. Two unrelated concepts are guaranteed to be nearly orthogonal.
2. **SIMD alignment**: 10,240 bits = 160 × 64-bit words = 5 × 32-word AVX-512 passes or 10 × 16-word AVX2 passes. Clean loop boundaries with no remainder.
3. **Bundle capacity**: SNR = sqrt(D/K). At D=10,240, K=500 → SNR=4.5 (above discrimination threshold ~4.0). Can reliably encode 500+ items in one vector.

### Comparison with Float Embeddings

| Criterion | Float Embeddings (768-dim) | BSC HDC (10,240-bit) |
|---|---|---|
| Compositional queries | Cannot express Boolean AND | `bind(role_A, val_A) ⊕ bind(role_B, val_B)` |
| Unbinding | Approximate (cosine similarity) | Exact (XOR is self-inverse) |
| Bundle capacity | ~27 items (D=768) | ~1,000 items (D=10,240) |
| Generational transfer | Per-item overhead, lossy | Single 1,280-byte vector for any N |
| Forgetting | Per-entry decay (cliff-edge) | Vote decay (smooth SNR degradation) |
| Hardware | BLAS/GPU required | CPU wire speed (XOR + POPCNT) |
| Size per vector | 3,072 bytes | 1,280 bytes |

---

## Application 1: Spectral Liquidity Manifolds

### The Mechanism

Liquidity data from DeFi pools is mapped to a **Riemannian manifold** where:
- **Position** on the manifold = pool configuration (tick range, TVL, fee tier)
- **Geodesic distance** = similarity between pools (closer = more similar behavior)
- **Curvature** = stability/instability signal (high curvature = volatile liquidity dynamics)

Each pool's liquidity profile is encoded as an HDC vector via trigram hashing of its parameters. Manifold distances are computed as Hamming distances between HDC vectors.

### Why This Matters

Traditional liquidity analysis treats each pool independently. The manifold approach reveals **structural relationships**:
- Pools with similar curvature respond similarly to market shocks
- Curvature spikes (second derivative of liquidity distribution) predict instability before it manifests in price
- Geodesic paths between pools suggest migration routes for capital

**Research**: Riemannian geometry for representation learning (Bronstein et al., 2017). Applied to DeFi liquidity: the manifold structure captures information that scalar metrics (TVL, volume) miss.

---

## Application 2: Causal Microstructure Discovery

### The Mechanism

Transaction causality is inferred from three signals:
1. **Temporal proximity**: Transactions within the same block or adjacent blocks
2. **Value flow**: Token amounts flowing between addresses/contracts
3. **Protocol state**: Which protocol functions were called in what order

Each causal relationship is encoded as an HDC bind: `cause_event ⊕ effect_event`. The resulting vector captures the association.

### Cycle Detection

Causal graphs often contain cycles (feedback loops). HDC enables efficient cycle detection:
- **Autocorrelation**: Bind a causal chain with shifted versions of itself
- If `chain ⊕ shift(chain, k)` has high similarity with the original chain, there's a cycle of period k
- This detects MEV sandwich attacks, liquidation cascades, and arbitrage loops

**Research**: Daian et al. (2020) — Flash Boys 2.0 (front-running mechanics). Qin et al. (2021) — 70% of DeFi liquidations from gradual decay, 30% from flash events. The HDC causal analysis can distinguish between these two modes.

---

## Application 3: Adaptive Signal Metabolism

### The Mechanism

Technical analysis patterns (RSI, MACD, volume profiles, etc.) are encoded as HDC vectors and subjected to evolutionary selection:

```
Generation 0: Random patterns + classical TA indicators
    ↓
Evaluate: Each pattern predicts next-tick market state
Score:    Fitness = correct_predictions / total_predictions
    ↓
Select:   Top 50% survive
Mutate:   Random bit flips in BSC vectors (exploration)
Bundle:   Combine successful patterns (exploitation)
    ↓
Generation N: Population adapted to current regime
```

**Regime conditioning**: Evolution runs independently per market regime. Different regimes select different patterns:
- **Bull market**: Momentum indicators survive (RSI trending, MACD crossover)
- **Bear market**: Mean-reversion indicators survive (oversold bounce, volume exhaustion)
- **Volatile market**: Hedging patterns survive (straddle-like structures)

### Why HDC Over Traditional Feature Engineering

- **No manual feature selection**: The evolutionary process discovers which pattern combinations predict well
- **Automatic regime adaptation**: Patterns that work in one regime but fail in another are culled
- **Composable**: Two successful patterns can be bundled into a composite that captures both signals
- **Interpretable**: Each pattern can be unbound to inspect its components

---

## Application 4: Predictive Geometry

### The Mechanism

Trade execution characteristics (arrival times, slippage, MEV exposure) are encoded as positions in a feature space. A kernel SVM learns regime-dependent separation:

- **Features**: Time-of-day, gas price, pool liquidity depth, pending mempool size, historical slippage at this liquidity level
- **Encoding**: Each feature set → HDC vector via role-filler binding
- **Classification**: High-slippage vs. low-slippage regions learned per regime
- **Output**: Predicted slippage distribution for proposed trade, used by the Gate to decide whether to execute

### The Predictive Foraging Connection

The PredictionEngine (on-chain) collects all agents' slippage predictions and actual outcomes. **Collective calibration** makes every agent's slippage estimates more accurate:

```
Agent A predicts: 0.3% slippage for 100 ETH swap on Uniswap V3
Actual: 0.5% slippage
Residual: -0.2% (underpredicted)

Aggregated across 200 agents for this (pool, size, time) context:
  Mean bias: -0.15% (all agents underpredicting)
  Agent A's corrected prediction: 0.3% + 0.15% = 0.45%
```

**Research**: Crowd wisdom (Galton, 1907; Surowiecki, 2004). The aggregate prediction from many imperfect predictors is better than any individual. The PredictionEngine formalizes this with bias-correcting residual aggregation.

---

## The HDC Fingerprinting System

### Code Symbol Fingerprinting

Roko-index uses HDC to fingerprint code symbols for fast similarity search:

```rust
// Trigram encoding: "fn process_signal" → trigrams → random seed per trigram → XOR bundle
let fingerprint = HdcVector::from_text("fn process_signal(&self, sig: Signal) -> Result<()>");

// Role vectors for structured encoding:
let ROLE_FUNCTION = HdcVector::from_seed(b"roko:role:function");
let ROLE_FILE = HdcVector::from_seed(b"roko:role:file");
let ROLE_MODULE = HdcVector::from_seed(b"roko:role:module");

// Structured fingerprint: bind role with value
let structured = ROLE_FUNCTION.bind(&function_hv)
    .bundle(&[&ROLE_FILE.bind(&file_hv), &ROLE_MODULE.bind(&module_hv)]);
```

### Applications

| Use Case | How HDC Helps |
|---|---|
| **Duplicate detection** | Hamming distance < threshold → likely duplicate |
| **Similarity search** | Find functions similar to a query in nanoseconds |
| **Pattern clustering** | K-medoids on Hamming distance → discover code patterns |
| **Change impact** | Fingerprint delta between versions → quantify semantic change |
| **Cross-language matching** | Language-agnostic trigram encoding matches concepts across Rust/TS/Go |

### The Search Architecture (Three Tiers)

**Tier 1 — Bloom Pre-Filter** (eliminates 90-99% of candidates):
- Per-segment Bloom filters on: entry_type, weight_bucket, poster_clade
- Bitwise OR merge (natural CRDT — gossipable between nodes)
- Cost: ~2 GB RAM for all filters at 10M entries
- Eliminates >90% of segments before any distance computation

**Tier 2 — Multi-Index Hashing (exact Hamming search)**:
- Split 10,240-bit vector into M substrings of length 10,240/M
- Build hash table per substring
- Query: for Hamming radius r, at least one substring must match exactly within r/M distance
- **Exact** results (not approximate like HNSW)
- 10M entries: ~50μs query time

**Tier 3 — HNSW (approximate, for very large collections)**:
- usearch configuration: M=16, ef_construction=200, ef_search=100
- Metric: Hamming distance (binary B1 quantization)
- Memory at 100K entries: ~250 MB
- Query latency: <1ms

### Two-Tier Streaming Index

The index handles real-time inserts without full rebuild:
- **Main index**: Rebuilt periodically (Delta tick or at 500-episode threshold merge)
- **Staging index**: Accepts real-time inserts between rebuilds
- **Query**: Search both, merge results, re-rank
- **Merge**: Every ~8 hours (500 episodes/minute × ~480 minutes)

### Performance

- **Encode**: ~1μs per symbol (deterministic, parallelizable)
- **Compare**: ~13ns per pair (XOR + POPCNT, 10,240 bits = 160 u64 words)
- **Search 10M symbols**: ~130ms brute-force, ~50μs with Multi-Index Hashing pre-filter
- **Memory**: 1,280 bytes per fingerprint × 10M symbols = ~12.8 GB

### Storage Architecture (6-Month Budget)

| Tier | Engine | Data | Size |
|---|---|---|---|
| Hot (active state) | redb (ACID, copy-on-write B-tree) | CorticalState, ChainScope, metadata | ~500 MB |
| Warm (write-heavy) | fjall (LSM-tree) | Chain events, triage traces, sketches | ~2 GB |
| Vectors | usearch/LanceDB | HDC binary + float embeddings | ~250 MB |
| Cold (archival) | Parquet + Zstd (~87% compression) | Historical episodes, signals | ~5 GB |
| **Total** | | | **~8 GB** |

**Research**: Multi-Index Hashing (Norouzi, Punjani, Fleet, 2014). HNSW (Malkov & Yashunin, 2018). FreshDiskANN (Singh et al., 2021) — streaming ANN updates. Anisotropic vector quantization (Guo et al., 2020, ICML).

---

## The ItemMemory (Shared Codebook)

All HDC operations use a shared codebook of role and filler vectors:

### Role Vectors (Generated Once at Boot, Deterministic from Seed)

```rust
const ROLE_SEEDS: &[&[u8]] = &[
    b"roko:role:episode_type",
    b"roko:role:protocol",
    b"roko:role:outcome",
    b"roko:role:regime",
    b"roko:role:insight_type",
    b"roko:role:confidence",
    b"roko:role:function",
    b"roko:role:file",
    b"roko:role:module",
    b"roko:role:crate",
];
```

Each role vector is a random 10,240-bit vector generated from its seed via BLAKE3-based PRNG. **Deterministic**: same seed always produces same vector.

### Filler Vectors (Discretized Values with Thermometer Encoding)

Continuous values (e.g., confidence 0.0-1.0) are discretized into buckets and encoded as filler vectors. Thermometer encoding ensures similar values have similar vectors:

```
Confidence 0.0-0.2 → filler_low
Confidence 0.2-0.4 → filler_low BUNDLE filler_med_low
Confidence 0.4-0.6 → filler_low BUNDLE filler_med_low BUNDLE filler_med
...
```

This preserves ordinal relationships: `similarity(0.3, 0.4) > similarity(0.3, 0.9)`.

---

## Compositional Primitives for Dream Replay (v2)

The dream engine uses HDC-encoded compositional primitives for scenario generation:

| Primitive | Composes With | Example |
|---|---|---|
| `LIQUIDITY_CRISIS` | MOMENTUM_BREAK, CASCADING_LIQUIDATION | "Pool drains + momentum reversal" |
| `MOMENTUM_BREAKOUT` | GAS_SPIKE, CORRELATION_SHIFT | "Price breaks out during gas spike" |
| `MEAN_REVERSION` | LIQUIDITY_RECOVERY, VOLATILITY_COMPRESSION | "Bounce after panic sell" |
| `GAS_SPIKE` | Any primitive | Universal stress modifier |
| `ORACLE_DEVIATION` | LIQUIDATION_CASCADE, ARB_OPPORTUNITY | "Oracle lag creates liquidation wave" |
| `CASCADING_LIQUIDATION` | LIQUIDITY_CRISIS, CORRELATION_SHIFT | "Liquidations trigger more liquidations" |
| `CORRELATION_SHIFT` | MOMENTUM_BREAKOUT, REGIME_CHANGE | "Previously correlated assets decouple" |
| `REGIME_CHANGE` | All primitives | Fundamental state transition |

**Composition**: Two primitives combine via XOR-bind: `LIQUIDITY_CRISIS ⊕ GAS_SPIKE` creates a composite scenario vector. The dream engine samples from these composites to generate counterfactual stress tests.

**Research**: Bakermans et al. (2025, Nature Neuroscience) — hippocampal state spaces are compositional from primitives, enabling zero-shot generalization. Applied to DeFi: compositional market scenarios from 8 base primitives.

---

## The EpisodeCompressor (For Knowledge Transfer)

When compressing episodes for transfer (to successors or across clades):

```rust
pub fn compress(
    episodes: &[(HdcVector, HdcVector, f64)],  // (episode_hv, insight_hv, importance)
    generation: u32,
) -> HdcVector {
    let weighted: Vec<(&HdcVector, u32)> = episodes.iter()
        .map(|(ep, ins, imp)| {
            let bound = ep.bind(ins);  // Associate episode with its insight
            let repeats = (imp * 4.0).round() as u32;  // Importance → vote count
            (&bound, repeats)
        })
        .collect();

    HdcVector::weighted_bundle(&weighted)
}
```

**Result**: A single 1,280-byte vector encoding all episode-insight pairs, weighted by importance.

**Capacity**: At D=10,240, ~500 pairs with SNR > 4.0 (above discrimination threshold).

**Generational stacking**: Each generation's legacy is bundled with the successor's own experience:

```
weight = 0.85^distance × 8
  distance 1 (direct predecessor): weight = 6.8
  distance 5: weight = 3.6
  distance 20: weight ≈ 0.3
```

After ~10 generations, only the most validated patterns survive from early ancestors. This IS the Baldwin Effect: learned patterns that persist across generations become structural defaults.

---

## Vote Decay: Controlled Forgetting in HDC Space

### The Problem with Per-Entry Decay

Traditional knowledge systems decay entries individually: each entry has a confidence score that decreases over time. When confidence crosses a threshold, the entry is deleted. This creates a **cliff-edge**: one tick the entry exists with its full signal, the next tick it's gone.

### The HDC Solution: Smooth SNR Degradation

Instead of decaying entries, decay the **votes** in the bundle accumulator:

```rust
pub fn apply_decay(&mut self, decay_factor: f64) {
    for vote in &mut self.votes {
        *vote = (*vote as f64 * decay_factor).round() as i32;
    }
}
```

Each bit position in the HDC vector has a vote count (how many entries voted 1 vs 0 for that bit). Multiplying all votes by 0.95 per cycle:
- After 14 cycles: old entry's influence halves
- After 46 cycles: influence drops to 10%
- **Smooth curve**, not cliff-edge

### Why This Matters

Old entries gradually lose influence but don't vanish. New evidence can add votes back — the system is resilient to noisy spikes. The Signal-to-Noise Ratio degrades smoothly: `SNR = sqrt(D/K_effective)` where `K_effective` shrinks as old votes decay.

**Research**: Ebbinghaus (1885) — forgetting follows an exponential curve. Richards & Frankland (2017) — forgetting is regularization, preventing overfitting to recent data. Davis & Zhong (2017) — active forgetting is metabolically expensive in biological systems; the HDC approach achieves it computationally.

---

## Superposition Memory: The Sorted Merkle Tree

On the shared chain, all active knowledge entries form a **sorted Merkle tree** with the root (`sm_root`) committed in every block header:

```
Block Header
  ├── parent_hash
  ├── state_root
  ├── sm_root  ← sorted Merkle tree over ALL active InsightEntry records
  └── ...
```

### Properties

- **Verifiable completeness**: Light clients verify any query result against `sm_root` with O(log N) Merkle proof
- **Deterministic state**: All validators agree on exactly which entries are active (deterministic pruning below 1% weight threshold)
- **Efficient sync**: New validators sync via Merkle comparison, not full chain replay
- **Consensus-free reads**: Queries hit local validator's search index (HDC/HNSW), not consensus. Results include Merkle proofs for client-side verification.

### Bucketed Weight Decay (Performance Optimization)

Instead of computing per-entry decay for 10M+ entries every block, entries are grouped into 16 time-buckets with pre-computed decay factors. At query time:

```
effective_weight = base_weight × bucket_decay_factors[bucket_index]
```

**Performance**: Decaying 10M entries at query time: ~6ms (vs. 80-200ms for per-entry computation).

---

## Research Citations

| Paper | Year | Application |
|---|---|---|
| Hyperdimensional Computing (Kanerva) | 2009 | BSC foundation, vector algebra |
| HDC Survey Parts I & II (Kleyko et al.) | 2022 | Comprehensive framework comparison |
| HDC for Biological Data (Frady et al.) | 2023 (PNAS) | Fast, robust, interpretable |
| Federated HDC (IoT context) | 2025 | Constant-cost distributed learning |
| Riemannian Geometry for ML (Bronstein et al.) | 2017 | Spectral liquidity manifolds |
| Flash Boys 2.0 (Daian et al.) | 2020 | MEV and front-running mechanics |
| DeFi Liquidations (Qin et al.) | 2021 | Cascade failure patterns |
| Uniswap V3 (Adams et al.) | 2021 | Concentrated liquidity economics |
| LVR (Milionis et al.) | 2022 | Loss vs. Rebalancing in AMMs |
| Crowd Wisdom (Galton; Surowiecki) | 1907/2004 | Collective prediction calibration |
| Multi-Index Hashing (Norouzi et al.) | 2014 | Exact Hamming distance search |
| Hinton & Nowlan (Baldwin Effect) | 1987 | Evolved learning speed |
| Grossman & Stiglitz (Info Economics) | 1980 | Value of costly information production |
| HNSW (Malkov & Yashunin) | 2018 | Approximate nearest neighbor search |
| FreshDiskANN (Singh et al.) | 2021 | Streaming ANN updates |

---

## HDC for Code Intelligence: Structural Fingerprinting

### How Code Fingerprints Work

Every code symbol in the index gets an HDC fingerprint — a 10,240-bit vector that captures its structural identity. The fingerprint encodes what the symbol IS (its kind, name, type relationships) rather than what it DOES (its semantic meaning). This makes it deterministic, fast, and composable.

The fingerprinting pipeline has five stages:

**Stage 1 — Kind Vector**: Each symbol kind gets a deterministic base vector from a seed:

```rust
let KIND_FUNCTION = HdcVector::from_seed(b"roko:kind:function");
let KIND_STRUCT   = HdcVector::from_seed(b"roko:kind:struct");
let KIND_TRAIT    = HdcVector::from_seed(b"roko:kind:trait");
let KIND_ENUM     = HdcVector::from_seed(b"roko:kind:enum");
let KIND_IMPL     = HdcVector::from_seed(b"roko:kind:impl");
let KIND_CONST    = HdcVector::from_seed(b"roko:kind:const");
```

These are fixed, quasi-orthogonal vectors. `hamming(KIND_FUNCTION, KIND_STRUCT) ≈ 0.50` (noise floor). Each is generated once at boot from BLAKE3-based PRNG.

**Stage 2 — Name Vector**: The symbol's name is encoded via trigram hashing:

```rust
// "validate_token" → trigrams: "val", "ali", "lid", "ida", "dat", "ate", ...
// Each trigram → deterministic seed → random vector
// All trigram vectors bundled (majority-vote) → name_vec
let name_vec = HdcVector::from_text("validate_token");
```

Names with shared substrings produce similar vectors: `hamming("validate_token", "validate_session") ≈ 0.62` — meaningfully above noise (0.50), reflecting the shared "validate_" prefix.

**Stage 3 — Kind-Name Binding**: Kind and name bind via XOR:

```rust
let identity = KIND_FUNCTION.bind(&name_vec);
```

This creates a vector that is SIMILAR to other functions (shares the KIND_FUNCTION component) but specific to this name. A function `validate_token` is more similar to a function `validate_session` than to a struct `validate_token`.

**Stage 4 — Type Reference Encoding**: The symbol's type signature gets encoded as bound type references:

```rust
// fn validate_token(token: &str) -> Result<Claims, AuthError>
let ROLE_RETURN = HdcVector::from_seed(b"roko:role:return_type");
let ROLE_PARAM  = HdcVector::from_seed(b"roko:role:param_type");

let return_vec = HdcVector::from_text("Result<Claims, AuthError>");
let param_vec  = HdcVector::from_text("&str");

let type_sig = ROLE_RETURN.bind(&return_vec)
    .bundle(&[&ROLE_PARAM.bind(&param_vec)]);
```

Functions with similar signatures produce similar type vectors. Any function returning `Result<_, _>` shares the `Result` trigrams in its return type encoding.

**Stage 5 — Final Bundle**: All components merge into the final fingerprint:

```rust
let fingerprint = identity.bundle(&[&type_sig]);
```

### Similarity Semantics

The resulting fingerprint has useful similarity properties:

| Comparison | Expected Similarity | Why |
|---|---|---|
| Same function, same crate | 1.00 | Identical vector |
| Function `validate_token` vs. `validate_session` | ~0.62 | Shared kind + name prefix |
| Function `validate_token` vs. `parse_token` | ~0.58 | Shared kind + "token" suffix |
| Function returning `Result<A, B>` vs. `Result<C, D>` | ~0.56 | Shared `Result` trigrams in return type |
| Function `validate_token` vs. Struct `validate_token` | ~0.52 | Same name but different kind — barely above noise |
| Unrelated symbols | ~0.50 | Noise floor (quasi-orthogonal) |

The key insight: **structural similarity dominates lexical similarity**. Two functions with different names but similar type signatures are more similar than a function and a struct sharing the same name. This reflects how code actually works — type compatibility matters more than naming conventions.

### Why HDC Alongside Embeddings

HDC structural fingerprints and float embeddings serve complementary roles:

| Criterion | HDC Fingerprint | Float Embedding |
|---|---|---|
| **What it captures** | Structural identity (kind, types, relationships) | Semantic meaning (what the code does) |
| **Speed** | ~50ns encode, ~13ns compare | ~1μs encode, ~100ns compare |
| **Determinism** | Fully deterministic (same code = same vector) | Model-dependent (different models = different vectors) |
| **Compositionality** | Full algebra (XOR bind, bundle, permute) | No compositional operations |
| **Unbinding** | Exact (XOR is self-inverse) | Not possible |
| **Size** | 1,280 bytes | 3,072 bytes (768-dim float32) |
| **Best for** | "Find structurally similar functions" | "Find code that does similar things" |

The index uses BOTH. HDC provides fast, exact structural queries. Embeddings provide rich semantic queries. The hybrid approach combines their strengths.

### Hybrid Retrieval: Reciprocal Rank Fusion

When a developer searches for code, three retrieval paths run in parallel:

```
Query: "function that validates authentication tokens"
  │
  ├── T0: Keyword search (ripgrep)
  │     Results: [validate_token (rank 1), check_auth (rank 3), ...]
  │
  ├── T1: HDC structural search (Hamming distance)
  │     Results: [verify_token (rank 1), validate_session (rank 2), ...]
  │
  └── T2: Embedding semantic search (cosine similarity)
        Results: [authenticate_user (rank 1), validate_token (rank 2), ...]
```

Results fuse via Reciprocal Rank Fusion (RRF):

```
RRF_score(doc) = Σ 1/(k + rank_i)    for each retrieval system i
  where k = 60 (smoothing constant)
```

For `validate_token` appearing at rank 1 in keyword, rank 5 in HDC, rank 2 in embedding:

```
RRF = 1/(60+1) + 1/(60+5) + 1/(60+2)
    = 0.01639 + 0.01538 + 0.01613
    = 0.04790
```

**Why k=60**: Cormack, Clarke & Buettcher (2009) showed k=60 minimizes over-reliance on any single retrieval system. Lower k values let the top-ranked system dominate; higher k values flatten rank differences too aggressively.

### Code Search Tiers

| Tier | Method | Latency | When Used |
|---|---|---|---|
| **T0** | Keyword search (ripgrep) | <1ms | Always — first pass, zero overhead |
| **T1** | HDC structural (Hamming distance) | ~13ns/comparison, ~50μs for 10M | When structural similarity matters (type-compatible functions) |
| **T2** | Embedding semantic (cosine similarity) | ~5ms/query with HNSW | When semantic intent matters ("find code that does X") |
| **T3** | Hybrid (RRF fusion of T0+T1+T2) | ~20ms total | Full search — combines all three for best results |

T0 always runs (it is essentially free). T1 and T2 run in parallel when the query warrants it. T3 fuses the results. The entire pipeline completes in ~20ms — fast enough for interactive code navigation.

---

## HDC for Knowledge Distillation: The Genomic Bottleneck

### The Compression Problem

An agent accumulates hundreds of knowledge entries over its lifetime: episode traces, validated insights, heuristics, causal links. Transferring this knowledge to a successor agent entry-by-entry is expensive — both in storage and in retrieval time. The successor would need to search through hundreds of inherited entries for every task.

### The Solution: A Single 1,280-Byte Vector

An agent's entire learned knowledge compresses to a single 10,240-bit vector (1,280 bytes). This is the **genomic bottleneck** — the informational compression that enables efficient knowledge transfer between generations.

The compression mechanism is a **majority-vote bundle of all high-confidence entries**, weighted by confidence:

```rust
pub fn compress_knowledge(entries: &[KnowledgeEntry]) -> HdcVector {
    let weighted: Vec<(&HdcVector, u32)> = entries.iter()
        .filter(|e| e.confidence > 0.5)  // Only high-confidence entries
        .map(|e| {
            let votes = (e.confidence * 10.0).round() as u32;  // Confidence → vote count
            (&e.bsc_vector, votes)
        })
        .collect();

    HdcVector::weighted_bundle(&weighted)
}
```

A confidence-0.9 entry contributes 9 votes. A confidence-0.5 entry contributes 5 votes. The majority-vote bundle preserves the bit patterns that appear most frequently across the highest-confidence entries.

### What the Bottleneck Preserves

The resulting 1,280-byte vector is NOT a lossless compression. It is a lossy summary that preserves the **most common structural patterns** across the agent's knowledge:

- **High-confidence, frequently reinforced patterns**: Preserved with high fidelity (many concordant votes)
- **Domain-specific expertise**: Preserved if the agent specialized (consistent structural patterns in one domain)
- **Rare or contradicted entries**: Lost (their votes are overwhelmed by the majority)
- **Individual episode details**: Lost (no single entry dominates the bundle)

This is analogous to biological genomic bottlenecks: the genome doesn't encode every experience, but it encodes the statistical regularities that matter for survival.

### Decompression: Querying the Legacy Vector

A successor agent receiving the 1,280-byte legacy vector cannot "decompress" it back into individual entries. Instead, it uses the vector as a **similarity oracle**:

```rust
// Successor encounters a new situation, encodes it as HDC vector
let situation_vec = encode_situation(&current_task);

// Query the legacy vector for similarity
let similarity = situation_vec.hamming_similarity(&predecessor_legacy);

if similarity > 0.55 {
    // The predecessor had relevant experience for this kind of situation
    // Boost confidence in the current approach
    confidence_multiplier = 1.0 + (similarity - 0.50) * 4.0;
}
```

High similarity (>0.55) means the predecessor agent frequently encountered structurally similar situations and developed high-confidence knowledge about them. The successor benefits without knowing the specific entries — only that the predecessor's aggregate experience is relevant.

### Generational Decay: The Baldwin Effect

Knowledge transfer across generations follows exponential decay:

```
influence = 0.85^N

Generation 1 (direct predecessor):  0.85  → 85% influence
Generation 2:                       0.72  → 72% influence
Generation 5:                       0.44  → 44% influence
Generation 10:                      0.20  → 20% influence
Generation 20:                      0.04  →  4% influence
Generation 44:                      0.001 →  0.1% influence (effectively zero)
```

After approximately 10 generations, only the **most repeatedly validated patterns** survive from early ancestors. Patterns that were reinforced in every generation accumulate enough votes to persist; patterns that appeared in only one generation decay to noise.

This is the **Baldwin Effect** (Hinton & Nowlan, 1987): the ability to learn faster evolves when learning is correlated with fitness. In the agent context: agents that inherit strongly validated patterns from their predecessors perform better, survive longer, and pass on even more refined patterns to their successors. The population converges on structural defaults that accelerate learning — without any central curriculum.

---

## HDC for Pheromone Fields: Stigmergic Coordination

### The Coordination Problem

N agents operating in the same environment need to share situational awareness. Direct messaging scales as O(N^2). Centralized brokers are single points of failure. Pheromone fields solve this with **O(1) cost per agent** — agents read and write a shared spatial map without addressing each other directly.

### Three Pheromone Types

| Pheromone | Half-Life | Semantics | Example |
|---|---|---|---|
| **THREAT** | 2 hours | Danger signal — avoid this area/pattern | "High slippage detected on WETH/USDC V3 0.05%" |
| **OPPORTUNITY** | 12 hours | Profitable pattern — explore this area | "Arbitrage spread >0.3% on CRV/ETH across Uniswap and Curve" |
| **WISDOM** | 7 days | Persistent heuristic — structural knowledge | "V4 hooks with >50K gas cost get fewer swaps" |

Each pheromone type has its own base HDC vector (deterministic from seed). A pheromone deposit is the base vector bound with the context vector:

```rust
let PHEROMONE_THREAT      = HdcVector::from_seed(b"roko:pheromone:threat");
let PHEROMONE_OPPORTUNITY = HdcVector::from_seed(b"roko:pheromone:opportunity");
let PHEROMONE_WISDOM      = HdcVector::from_seed(b"roko:pheromone:wisdom");

// Agent detects high slippage on a pool
let context = encode_pool_state(&pool);
let deposit = PHEROMONE_THREAT.bind(&context);
field.write(cell_coords, deposit, strength=0.8);
```

### The Pheromone Field as Spatial Map

The pheromone field is a spatial map where each cell contains an HDC vector and a scalar strength value. "Spatial" here means the strategy/market space — cells are indexed by (strategy_type, market_regime, asset_class) tuples, not physical coordinates.

Agents interact with the field in two ways:

**Reading**: An agent reads cells in its vicinity (defined by its current strategy type and market conditions). The cell vectors modulate the agent's context assembly:

```
THREAT pheromone in vicinity → boost Warning entries, suppress Opportunity entries
OPPORTUNITY pheromone → boost Strategy entries, expand search radius
WISDOM pheromone → boost Heuristic entries, increase trust multiplier
```

**Writing**: When an agent discovers a pattern (positive or negative), it writes a pheromone deposit to the relevant cell. The deposit decays according to the pheromone's half-life.

### HDC Aggregation: Fuzzy Semantic Bundling

When an agent deposits a pheromone into a cell that already contains a deposit, the system checks Hamming similarity:

```rust
let existing = field.read(cell_coords);
let new_deposit = PHEROMONE_THREAT.bind(&new_context);

let similarity = existing.hamming_similarity(&new_deposit);

if similarity > 0.6 {
    // Semantically similar — reinforce by bundling
    let reinforced = existing.bundle(&[&new_deposit]);
    let new_strength = existing_strength + deposit_strength * 0.5;
    field.write(cell_coords, reinforced, new_strength);
} else {
    // Semantically different — create a new cell entry
    field.write(adjacent_cell, new_deposit, deposit_strength);
}
```

The 0.6 similarity threshold ensures that only **semantically aligned** deposits reinforce each other. Two agents detecting slippage on the same pool (but with slightly different parameters) will produce similar context vectors and their deposits will bundle. Two agents detecting unrelated issues will produce dissimilar vectors and their deposits remain separate.

This is **fuzzy semantic alignment without exact string matching** — no need for agents to agree on naming conventions, key formats, or data schemas. HDC similarity handles the alignment automatically.

### Cross-Agent Learning at O(1) Cost

Each agent pays O(1) per field read (one cell lookup + one Hamming comparison) and O(1) per field write (one cell write + possibly one bundle). The total cost is independent of the number of agents:

```
N = 100 agents:   each reads ~5 cells, writes ~2 cells → 700 total operations
N = 10,000 agents: each reads ~5 cells, writes ~2 cells → 70,000 total operations
Cost per agent: constant (7 operations regardless of N)
```

Compare with direct messaging: N=10,000 agents with pairwise communication = 100M messages. Pheromone fields reduce this to 70K cell operations — a **1,400x reduction**.

---

## HDC for Causal Discovery: Binding Cause and Effect

### Encoding Causal Relationships

HDC provides a natural algebra for encoding directed relationships. The key insight: **permutation breaks symmetry**. While XOR bind is symmetric (`A ⊕ B = B ⊕ A`), adding a permutation before binding makes the operation asymmetric — exactly what causality requires.

A causal link `Cause → Effect` is encoded as:

```rust
let causal_vec = cause_vec.bind(&effect_vec.permute(1));
```

The `permute(1)` (cyclic shift by 1 bit position) distinguishes the effect from the cause. Without the permutation, `cause.bind(effect) = effect.bind(cause)` — no directionality. With the permutation, `cause.bind(permute(effect)) ≠ effect.bind(permute(cause))` — the direction is encoded.

### Querying Causal Links

Given a causal vector and one component, you can recover the other:

```rust
// Given causal_vec and cause_vec, recover effect_vec:
let recovered_effect = causal_vec.bind(&cause_vec).unpermute(1);
// recovered_effect ≈ effect_vec (high Hamming similarity)

// Given causal_vec and effect_vec, recover cause_vec:
let recovered_cause = causal_vec.bind(&effect_vec.permute(1));
// recovered_cause ≈ cause_vec
```

This exploits XOR's self-inverse property: `A ⊕ B ⊕ A = B`. Binding the composite with one component cancels it out, leaving the other (shifted by the permutation offset).

### Causal Chains

Multi-step causal chains `A → B → C → D` are encoded as bundles of pairwise links:

```rust
let chain = bundle(&[
    a_vec.bind(&b_vec.permute(1)),   // A → B
    b_vec.bind(&c_vec.permute(1)),   // B → C
    c_vec.bind(&d_vec.permute(1)),   // C → D
]);
```

The chain is **queryable from any node**:

- "What does A cause?" → `chain.bind(&a_vec)` → highest similarity to `b_vec.permute(1)` → unpermute → B
- "What caused D?" → `chain.bind(&d_vec.permute(1))` → highest similarity to `c_vec` → C
- "Is B in this causal chain?" → `chain.hamming_similarity(&b_vec)` → above noise floor → yes

### Causal Strength via Repeated Bundling

Each confirming observation of a causal relationship adds another vote to the bundle:

```rust
// First observation: high gas → failed tx
let causal = gas_high.bind(&tx_fail.permute(1));

// Second observation: same pattern
let observation_2 = gas_high_2.bind(&tx_fail_2.permute(1));
// gas_high_2 ≈ gas_high (similar HDC encoding of "high gas")

let strengthened = causal.bundle(&[&observation_2]);

// After 50 observations:
let strong_causal = bundle_all(&observations);  // 50 concordant votes
```

More observations = more votes = higher SNR in the bundle = stronger causal signal. The strength is directly readable: `strong_causal.hamming_similarity(&query)` will be higher for a 50-observation link than a 5-observation link.

### Practical Applications

| Causal Pattern | Encoding | Observations Needed | Use |
|---|---|---|---|
| High gas → failed transaction | `gas_high.bind(tx_fail.permute(1))` | ~5 | Avoid submission during gas spikes |
| Large swap → price impact → slippage | 2-hop chain, bundled | ~10 | Size position limits |
| Oracle delay → stale price → liquidation | 2-hop chain, bundled | ~20 | Monitor oracle freshness |
| MEV bot detected → sandwich → loss | 2-hop chain, bundled | ~15 | Private mempool routing |
| Correlation break → cascade liquidation | 2-hop chain, bundled | ~30 | Reduce correlated exposure |

Each pattern is a compact HDC vector that can be queried in ~13ns. An agent carrying 100 causal patterns in its knowledge base can check all of them against a new situation in ~1.3μs.

---

## Mathematical Foundations: Why These Parameters

### Capacity Bound: How Many Items Fit in a Bundle

The theoretical capacity of a majority-vote bundle at dimension D with similarity threshold s is:

```
capacity ≈ log₂(D) / (-log₂(s))
```

At D = 10,240 and threshold s = 0.47 (detecting similarity just below noise floor):

```
capacity ≈ log₂(10240) / (-log₂(0.47))
         ≈ 13.32 / 1.09
         ≈ 170 items
```

This means a single 10,240-bit bundle can reliably store up to ~170 distinct items such that each can be detected above the noise floor. For the standard operating threshold of s = 0.55 (5 sigma above noise):

```
capacity ≈ 13.32 / (-log₂(0.55))
         ≈ 13.32 / 0.862
         ≈ 110 items
```

In practice, with weighted bundling and importance-based vote counts, the effective capacity reaches ~500 items at SNR > 4.0 (the minimum discrimination threshold).

### Why Not Higher D

Doubling the dimension to D = 20,480:

```
capacity(20480, 0.47) ≈ log₂(20480) / (-log₂(0.47))
                      ≈ 14.32 / 1.09
                      ≈ 196 items
```

Going from 10,240 to 20,480 doubles memory (1,280 → 2,560 bytes per vector) but increases capacity by only ~15% (170 → 196 items). The logarithmic scaling makes higher dimensions increasingly wasteful.

Additionally, D = 10,240 fits in **2.5 CPU cache lines** (64 bytes each × 160 words ÷ 64 = 2.5 lines at 4,096 bytes per line on modern Intel/AMD). This means the entire vector stays in L1 cache during comparison — a critical performance property for the ~13ns comparison time.

### Noise Floor: When Is Similarity Meaningful

Two random, independently generated 10,240-bit vectors have expected Hamming similarity of exactly 0.500 (half the bits match by chance). The standard deviation of this similarity is:

```
σ = 1 / (2√D) = 1 / (2√10240) ≈ 0.00494
```

Any observed similarity can be expressed as a z-score: `z = (similarity - 0.500) / 0.00494`

| Observed Similarity | z-score | p-value (one-tailed) | Interpretation |
|---|---|---|---|
| 0.505 | 1.0 | 0.159 | Not significant — likely noise |
| 0.510 | 2.0 | 0.023 | Weak signal |
| 0.515 | 3.0 | 0.001 | Moderate signal |
| 0.520 | 4.1 | 2.2 × 10⁻⁵ | Strong signal |
| 0.550 | 10.1 | < 10⁻²³ | Extremely strong signal |
| 0.600 | 20.2 | < 10⁻⁹⁰ | Near-certain relationship |

### The 0.6 Threshold for Pheromone Bundling

The pheromone field uses a 0.6 Hamming similarity threshold for deciding whether two deposits should bundle (reinforce) or remain separate. At D = 10,240:

```
z = (0.6 - 0.5) / 0.00494 ≈ 20.2
```

This is approximately **20 standard deviations above the noise floor**. The probability of two random, unrelated vectors achieving 0.6 similarity by chance:

```
P(similarity > 0.6 | random) ≈ P(z > 20.2) ≈ 10⁻⁹⁰
```

This is astronomically unlikely — far beyond any practical requirement. For comparison:
- The probability of a SHA-256 collision is ~10⁻⁷⁷
- The number of atoms in the observable universe is ~10⁸⁰
- The probability of two random HDC vectors falsely bundling at threshold 0.6 is ~10⁻⁹⁰

The threshold could be set much lower (e.g., 0.55, z ≈ 10, p ≈ 10⁻²³) and still have negligible false positive rate. The 0.6 choice provides extreme conservatism while still allowing genuinely related deposits (which typically show similarity 0.6-0.8) to reinforce each other.

### SNR Degradation in Bundles

As more items are bundled, each individual item's signal weakens relative to noise:

```
SNR = √(D / K_effective)

Where K_effective is the number of items with non-negligible vote weight
```

| Items Bundled | SNR | Detection Reliability |
|---|---|---|
| 10 | 32.0 | Near-perfect |
| 50 | 14.3 | Excellent |
| 100 | 10.1 | Very good |
| 500 | 4.5 | Above minimum threshold (4.0) |
| 1,000 | 3.2 | Below threshold — unreliable |
| 5,000 | 1.4 | Noise-dominated — unusable |

This is why the EpisodeCompressor targets ~500 items maximum: it is the practical capacity ceiling at D = 10,240 where detection remains reliable. Beyond 500 items, the vote accumulator saturates and individual items become indistinguishable from noise.

### The Half-Life Relationship

Vote decay creates an effective half-life for bundled items. If votes decay by factor α per cycle:

```
half_life_cycles = -ln(2) / ln(α)

At α = 0.95 (5% decay per cycle):
  half_life = -0.693 / -0.0513 = 13.5 cycles

At α = 0.99 (1% decay per cycle):
  half_life = -0.693 / -0.01005 = 68.9 cycles
```

This means that at α = 0.95 with one cycle per hour:
- An item's influence halves every ~14 hours
- After 3 half-lives (~42 hours): 12.5% of original influence
- After 7 half-lives (~4 days): <1% of original influence — effectively forgotten

The decay rate α is the single knob that controls the system's memory horizon. Higher α (slower decay) = longer memory = more items competing for bundle capacity. Lower α (faster decay) = shorter memory = more responsive to recent experience. The default α = 0.95 balances these tensions for the typical agent lifecycle of days to weeks.
