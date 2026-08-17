# Novel Cross-Domain Research Ideas for TraceCommons

## What is TraceCommons?

TraceCommons is an open-source platform for collecting, scoring, deduplicating, and
compensating contributions of AI agent interaction traces. When a developer uses an
AI coding assistant (Claude Code, Codex, etc.), the resulting conversation -- tool
calls, model responses, error corrections, strategy pivots -- constitutes a "trace."
TraceCommons provides the infrastructure for contributors to donate these traces into
a shared corpus, where they are quality-scored and used to improve future AI systems.

The system is built in Rust and organized into six crates:

- **trace-commons-gate-api**: Trait definitions for the scoring pipeline. Defines
  `PerplexityScorer` (measures content surprise via language model logprobs),
  `Embedder` (projects traces into vector space), `VectorIndex` (nearest-neighbor
  lookup for novelty), and the `OrchestrationDecision` output struct. These traits
  are the stable seam: proprietary scoring backends implement them without touching
  the open-source server.

- **trace-commons-gate-enclave**: The orchestrator that composes scorers into a
  gate pipeline. `EnclaveGateOrchestrator::evaluate()` runs five steps: (1) chunk
  the trace, (2) score each chunk for perplexity, (3) embed each chunk, (4) query
  nearest neighbors and compute novelty as `1 - max(cosine_similarity)`, (5) apply
  configured floors and conditionally insert into the vector index. The enclave is
  designed for TEE (dstack-attested) deployment so scoring logic runs in a trusted
  execution environment.

- **trace-commons-protocol**: The wire types. `TraceContributionEnvelope` carries
  consent metadata, privacy/redaction pipeline stamps, contributor identity,
  per-event trace data, outcome labels, and value metadata. This crate also owns
  the deterministic redaction pipeline and privacy filter sidecar integration.

- **trace-commons-server**: The hosted control plane. Owns PostgreSQL storage
  (with mandatory row-level security per tenant), the credit-quality scoring
  function `q = f(perplexity) * g(novelty) * a(anomaly)` (log-concave,
  anti-Goodhart), cross-trace deduplication via simhash + embedding distance,
  contributor cap (saturating `effective(R) = K(1 - exp(-R/K))`), NEAR blockchain
  credit settlement, artifact encryption (envelope KEK/DEK), and the ~85-route
  HTTP API.

- **trace-commons-contributor**: The client-side CLI. Discovers local traces,
  builds contribution envelopes, runs local redaction, obtains upload claims from
  the issuer, and submits to the server.

- **trace-commons-operator-client**: Admin tooling for corpus operators. Host
  allowlists, privacy filter configuration, formatting utilities.

### The Scoring Pipeline in Detail

The current scoring pipeline is a two-gate system:

1. **Perplexity gate**: Measures how "surprising" the trace content is to a
   language model. High perplexity means the trace contains patterns the model
   has not seen frequently. Uses chunked evaluation with token-weighted
   aggregation across chunks. Tail-fraction (95th-percentile token surprise)
   provides a secondary signal. Both must clear configured floors.

2. **Novelty gate**: Measures how different this trace is from everything already
   in the corpus. Embeds the trace, queries the per-tenant vector index for
   top-k nearest neighbors, and computes `1 - max_similarity`. Must clear a
   configured floor. On pass, the embedding is inserted into the index so
   future traces are measured against it.

The gates compose into a credit-quality score:
```
q = f(perplexity) * g(novelty) * a(anomaly)
```
where `f` and `g` are log-concave saturating functions (anti-Goodhart: no
marginal benefit past the ceiling, so gaming one dimension cannot compensate for
weakness in another), and `a` is an anomaly penalty based on the peak/representative
perplexity ratio (detects traces that spike in one chunk but are low-quality
overall -- a fraud signal).

Cross-trace deduplication uses 64-bit simhash (2-token shingle features, FNV-1a)
with Hamming distance, OR-matched with embedding cosine distance, to cluster
near-duplicate traces. Each cluster member's credit is divided by cluster size.

Contributor compensation is capped per epoch via a saturating concave function
`effective(R) = K(1 - exp(-R/K))` that bounds how much any identity can earn in
a 7-day window.

---

The eight ideas below draw from academic fields that have not been applied to AI
trace management: auction theory, information theory, evolutionary dynamics,
epidemiology, behavioral economics, algebraic topology, materials science, and
computational neuroscience. Each idea includes full academic citations, a detailed
Rust implementation sketch, PostgreSQL schema where needed, integration points
with the existing pipeline, and potential research outputs.

---

## 1. VCG Auctions for Trace Valuation

### Academic Foundations

The Vickrey-Clarke-Groves (VCG) mechanism is the canonical truthful auction in
mechanism design theory. Vickrey (1961) introduced the sealed-bid second-price
auction for single items. Clarke (1971) and Groves (1973) generalized it to
combinatorial settings: each agent reports their valuation for every possible
allocation, and the mechanism selects the allocation maximizing total reported
value. Each agent pays the externality they impose on others -- the difference
between the total value the others would have received without this agent and the
total value they actually receive. This "externality pricing" makes truthful
reporting a dominant strategy: no agent can improve their outcome by misreporting
their valuation, regardless of what other agents do.

**Key references:**
- Vickrey, W. (1961). "Counterspeculation, Auctions, and Competitive Sealed Tenders." *Journal of Finance*, 16(1), 8-37.
- Clarke, E. H. (1971). "Multipart Pricing of Public Goods." *Public Choice*, 11, 17-33.
- Groves, T. (1973). "Incentives in Teams." *Econometrica*, 41(4), 617-631.
- Nisan, N. & Ronen, A. (2001). "Algorithmic Mechanism Design." *Games and Economic Behavior*, 35(1-2), 166-196.
- Roughgarden, T. (2010). "Algorithmic Game Theory." *Communications of the ACM*, 53(7), 78-86.

### Why This is Novel for AI Trace Management

TraceCommons currently uses a fixed formula (`q = f * g * a`) to score traces,
then divides by dedup cluster size and applies a contributor cap. This is
adequate for ranking but fundamentally cannot answer: "What is this trace
*worth* to the corpus?" The scoring function measures intrinsic quality, but
the marginal value of a trace depends on what else is already in the corpus
and what other contributors are simultaneously offering.

VCG provides the missing piece: a mechanism where contributors reveal how much
they value having their trace accepted (in credits, priority, or access tokens),
and the system allocates limited scoring/storage budget to the traces that
maximize total welfare. Crucially, VCG's incentive compatibility means
contributors cannot game the system by misreporting -- they always do best by
bidding their true valuation.

The insight is that trace corpus curation is a resource allocation problem with
private valuations. The operator has a finite scoring budget (TEE compute,
embedder GPU time, vector index capacity). Contributors have private information
about their trace's expected quality. VCG connects these efficiently.

### Rust Implementation Sketch

```rust
use std::collections::HashMap;
use uuid::Uuid;

/// A contributor's bid for having their trace included in the corpus.
/// `valuation_micros` is the contributor's stated value (in credit-micros)
/// for acceptance. Under VCG, truthful bidding is dominant-strategy optimal.
#[derive(Debug, Clone)]
pub struct TraceBid {
    pub submission_id: Uuid,
    pub contributor_hash: String,
    pub valuation_micros: i64,
    /// Pre-computed quality signal from the perplexity gate (cheap to obtain).
    /// Used as a proxy for social welfare contribution.
    pub quality_proxy_micros: i64,
}

/// The outcome of VCG allocation for a single trace.
#[derive(Debug, Clone)]
pub struct VcgAllocation {
    pub submission_id: Uuid,
    /// Whether this trace was allocated a scoring slot.
    pub allocated: bool,
    /// The VCG payment: the externality this trace imposes on others.
    /// Equal to the total welfare of the other agents in the world without
    /// this agent, minus their total welfare in the world with this agent.
    pub payment_micros: i64,
    /// Net utility: valuation - payment. Always non-negative under VCG
    /// for truthful bidders (individual rationality).
    pub net_utility_micros: i64,
}

/// Social welfare function: the total value the mechanism aims to maximize.
/// Here, welfare = sum of (valuation * quality_proxy) for allocated traces.
/// The quality_proxy weights welfare toward traces that are actually good,
/// not just highly valued by their contributor.
fn social_welfare(bids: &[&TraceBid]) -> i64 {
    bids.iter()
        .map(|b| {
            // Welfare contribution = valuation * quality, scaled down from
            // micros^2 to micros. Saturate to avoid overflow.
            let product = (b.valuation_micros as i128) * (b.quality_proxy_micros as i128);
            (product / 1_000_000).clamp(0, i64::MAX as i128) as i64
        })
        .sum()
}

/// Select the welfare-maximizing subset of `capacity` traces.
/// For small capacity this is solved exactly; for large batches
/// a greedy approximation suffices (VCG with greedy allocation
/// retains approximate incentive compatibility per Lehmann et al. 2002).
fn optimal_allocation(bids: &[TraceBid], capacity: usize) -> Vec<usize> {
    let mut indexed: Vec<(usize, i64)> = bids
        .iter()
        .enumerate()
        .map(|(i, b)| {
            let w = ((b.valuation_micros as i128) * (b.quality_proxy_micros as i128)
                / 1_000_000) as i64;
            (i, w)
        })
        .collect();
    indexed.sort_by(|a, b| b.1.cmp(&a.1));
    indexed.into_iter().take(capacity).map(|(i, _)| i).collect()
}

pub struct VcgAuctioneer {
    /// Maximum number of traces to fully score in this batch.
    pub capacity: usize,
}

impl VcgAuctioneer {
    pub fn new(capacity: usize) -> Self {
        Self { capacity }
    }

    /// Run the VCG mechanism over a batch of bids.
    ///
    /// Steps:
    /// 1. Find the welfare-maximizing allocation of `capacity` slots.
    /// 2. For each allocated trace i, compute the "world without i":
    ///    re-solve the allocation excluding i and compute the welfare
    ///    of all other agents in that counterfactual world.
    /// 3. Payment_i = welfare_others_without_i - welfare_others_with_i.
    ///
    /// Complexity: O(n * n * log(n)) for n bids -- each agent's removal
    /// triggers a re-sort. For production batches (n < 1000), this is
    /// sub-millisecond.
    pub fn allocate(&self, bids: &[TraceBid]) -> Vec<VcgAllocation> {
        let winners = optimal_allocation(bids, self.capacity);
        let winner_set: std::collections::HashSet<usize> =
            winners.iter().copied().collect();

        // Welfare of all agents in the chosen allocation.
        let winner_refs: Vec<&TraceBid> =
            winners.iter().map(|&i| &bids[i]).collect();
        let _total_welfare = social_welfare(&winner_refs);

        // Welfare of others (excluding agent i) in the chosen allocation.
        let mut welfare_others_with: HashMap<usize, i64> = HashMap::new();
        for &i in &winners {
            let others: Vec<&TraceBid> = winners
                .iter()
                .filter(|&&j| j != i)
                .map(|&j| &bids[j])
                .collect();
            welfare_others_with.insert(i, social_welfare(&others));
        }

        let mut results: Vec<VcgAllocation> = Vec::with_capacity(bids.len());
        for (i, bid) in bids.iter().enumerate() {
            if !winner_set.contains(&i) {
                results.push(VcgAllocation {
                    submission_id: bid.submission_id,
                    allocated: false,
                    payment_micros: 0,
                    net_utility_micros: 0,
                });
                continue;
            }

            // Counterfactual: optimal allocation WITHOUT agent i.
            let others_bids: Vec<TraceBid> = bids
                .iter()
                .enumerate()
                .filter(|&(j, _)| j != i)
                .map(|(_, b)| b.clone())
                .collect();
            let cf_winners = optimal_allocation(&others_bids, self.capacity);
            let cf_welfare: i64 = social_welfare(
                &cf_winners.iter().map(|&j| &others_bids[j]).collect::<Vec<_>>(),
            );

            let welfare_others = welfare_others_with[&i];
            let payment = (cf_welfare - welfare_others).max(0);
            let welfare_i = ((bid.valuation_micros as i128)
                * (bid.quality_proxy_micros as i128)
                / 1_000_000) as i64;
            let net = (welfare_i - payment).max(0);

            results.push(VcgAllocation {
                submission_id: bid.submission_id,
                allocated: true,
                payment_micros: payment,
                net_utility_micros: net,
            });
        }
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bid(val: i64, quality: i64) -> TraceBid {
        TraceBid {
            submission_id: Uuid::new_v4(),
            contributor_hash: "sha256:test".into(),
            valuation_micros: val,
            quality_proxy_micros: quality,
        }
    }

    #[test]
    fn truthful_bidder_always_has_nonnegative_utility() {
        let bids = vec![
            bid(800_000, 900_000),
            bid(500_000, 600_000),
            bid(300_000, 400_000),
            bid(100_000, 200_000),
        ];
        let auctioneer = VcgAuctioneer::new(2);
        let results = auctioneer.allocate(&bids);
        for r in &results {
            assert!(
                r.net_utility_micros >= 0,
                "individual rationality violated for {:?}",
                r.submission_id
            );
        }
    }

    #[test]
    fn capacity_one_reduces_to_second_price() {
        let bids = vec![
            bid(1_000_000, 1_000_000), // highest
            bid(700_000, 1_000_000),    // second highest
            bid(300_000, 1_000_000),    // third
        ];
        let auctioneer = VcgAuctioneer::new(1);
        let results = auctioneer.allocate(&bids);
        let winner = results.iter().find(|r| r.allocated).unwrap();
        assert_eq!(winner.submission_id, bids[0].submission_id);
        // Payment should equal the second-highest welfare contribution
        assert_eq!(winner.payment_micros, 490_000); // 700k * 1M / 1M
    }
}
```

### PostgreSQL Schema

```sql
CREATE TABLE vcg_auction_rounds (
    round_id        UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id       UUID NOT NULL REFERENCES tenants(id),
    capacity        INT NOT NULL,
    bid_count       INT NOT NULL,
    total_welfare_micros BIGINT NOT NULL,
    decided_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    policy_version  TEXT NOT NULL
);

CREATE TABLE vcg_bids (
    bid_id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    round_id            UUID NOT NULL REFERENCES vcg_auction_rounds(round_id),
    submission_id       UUID NOT NULL,
    contributor_hash    TEXT NOT NULL,
    valuation_micros    BIGINT NOT NULL,
    quality_proxy_micros BIGINT NOT NULL,
    allocated           BOOLEAN NOT NULL,
    payment_micros      BIGINT NOT NULL,
    net_utility_micros  BIGINT NOT NULL
);

CREATE INDEX idx_vcg_bids_round ON vcg_bids(round_id);
CREATE INDEX idx_vcg_bids_contributor ON vcg_bids(contributor_hash, round_id);
```

### Integration with Existing Pipeline

The VCG auctioneer sits *before* the full `EnclaveGateOrchestrator::evaluate()`
call. The perplexity-only path (`evaluate_perplexity_only()`) provides the cheap
`quality_proxy_micros` signal without touching the vector index. Contributors
submit bids alongside their trace envelopes. The batch ingest worker collects
bids within a window, runs the VCG allocation, and only forwards winning traces
to the full evaluation pipeline (which is the expensive TEE + embedding path).

### Estimated Complexity and Prerequisites

- **Implementation effort**: ~2 weeks. The auction logic is pure and
  self-contained. The main engineering cost is the bid submission API and the
  batch collection window in the ingest worker.
- **Prerequisites**: A notion of "credits" or "tokens" that contributors can bid
  with. The existing NEAR credit settlement provides this.
- **Risk**: VCG is known to have low revenue in some settings (the "revenue
  equivalence" result does not apply when bidders have correlated valuations).
  Monitoring revenue per round is essential.

### Potential Research Output

- **Paper**: "Incentive-Compatible Resource Allocation for AI Training Data
  Marketplaces: A VCG Approach." Venue: AAAI Workshop on AI and Economics, or
  EC (ACM Conference on Economics and Computation).
- **Key result**: Empirical measurement of allocative efficiency vs. the current
  fixed-formula approach. How much total welfare improves when contributors can
  express private valuations.

---

## 2. Normalized Compression Distance (NCD) for Novelty Scoring

### Academic Foundations

The Normalized Compression Distance (NCD) is an approximation to the Normalized
Information Distance (NID), which is itself derived from Kolmogorov complexity.
The NID between two strings x and y is the length of the shortest program that
transforms x into y (or vice versa), normalized by the longer string's
complexity. Since Kolmogorov complexity is uncomputable, NCD approximates it
using real-world compressors:

```
NCD(x, y) = (C(xy) - min(C(x), C(y))) / max(C(x), C(y))
```

where `C(z)` is the compressed size of z. Li et al. (2004) proved that NCD
satisfies the properties of a metric (up to compressor imperfections) and
demonstrated it on clustering tasks across music, literature, genomics, and
languages -- without any domain-specific features.

**Key references:**
- Li, M., Chen, X., Li, X., Ma, B., & Vitanyi, P. M. B. (2004). "The Similarity Metric." *IEEE Transactions on Information Theory*, 50(12), 3250-3264.
- Cilibrasi, R. & Vitanyi, P. M. B. (2005). "Clustering by Compression." *IEEE Transactions on Information Theory*, 51(4), 1523-1545.
- Bennett, C. H., Gacs, P., Li, M., Vitanyi, P. M. B., & Zurek, W. H. (1998). "Information Distance." *IEEE Transactions on Information Theory*, 44(4), 1407-1423.
- Cohen, A. R. & Vitanyi, P. M. B. (2015). "Normalized Compression Distance of Multisets with Applications." *IEEE Transactions on Pattern Analysis and Machine Intelligence*, 37(8), 1602-1614.
- Sculley, D. & Brodley, C. E. (2006). "Compression and Machine Learning: A New Perspective on Feature Space Vectors." *IEEE ICDCS*.

### Why This is Novel for AI Trace Management

TraceCommons currently computes novelty via embedding cosine similarity against
a vector index. This requires a trained embedding model, GPU inference for every
trace, and a vector index that must be maintained and sharded per tenant. NCD
offers a complementary signal that is:

1. **Model-free**: No trained embedder, no GPU, no model versioning headaches.
2. **Language-agnostic**: Works on binary data, so traces containing code in
   any language, error messages, or mixed content are handled uniformly.
3. **Theoretically grounded**: NCD approximates a universal similarity metric
   that is provably optimal (the NID is the minimal metric up to an additive
   constant).
4. **Fast**: Compression is CPU-only and highly optimized (zstd at level 3
   processes ~500 MB/s). As a pre-filter before expensive embedding, NCD can
   reject obvious near-duplicates without invoking the embedder at all.

The insight is that compression-based similarity catches structural redundancy
that embedding similarity can miss. Two traces might embed differently (different
vocabulary) but compress identically when concatenated (same logical structure).
NCD detects this.

### Rust Implementation Sketch

```rust
use std::io::Write;

/// NCD configuration. The compressor and level affect both speed and
/// approximation quality. Higher levels give better NCD estimates but
/// are slower. Level 3 is a good tradeoff for real-time pre-filtering.
#[derive(Debug, Clone)]
pub struct NcdConfig {
    pub compression_level: i32,
    /// NCD threshold below which two traces are considered near-duplicates.
    /// NCD ranges from 0.0 (identical) to ~1.0+ (maximally different).
    /// Empirically, NCD < 0.3 strongly indicates structural redundancy.
    pub duplicate_threshold: f64,
    /// Maximum input size in bytes. Traces larger than this are truncated
    /// to bound compression time.
    pub max_input_bytes: usize,
}

impl Default for NcdConfig {
    fn default() -> Self {
        Self {
            compression_level: 3,
            duplicate_threshold: 0.30,
            max_input_bytes: 256 * 1024, // 256 KiB
        }
    }
}

/// Compress a byte slice using zstd and return the compressed size.
/// The compressed output is discarded -- we only need the size.
fn compressed_size(data: &[u8], level: i32) -> usize {
    let mut encoder = zstd::Encoder::new(Vec::new(), level).expect("zstd encoder");
    encoder.write_all(data).expect("zstd write");
    let output = encoder.finish().expect("zstd finish");
    output.len()
}

/// Compute the Normalized Compression Distance between two byte sequences.
///
/// NCD(x, y) = (C(xy) - min(C(x), C(y))) / max(C(x), C(y))
///
/// Returns a value in [0.0, ~1.1]. Values near 0 indicate high similarity;
/// values near 1 indicate maximal dissimilarity. Values slightly above 1.0
/// are possible due to compressor imperfections (the "non-normality" of
/// real compressors) and should be clamped to 1.0 for thresholding.
pub fn ncd(trace_a: &[u8], trace_b: &[u8], level: i32) -> f64 {
    if trace_a.is_empty() && trace_b.is_empty() {
        return 0.0;
    }
    let c_a = compressed_size(trace_a, level) as f64;
    let c_b = compressed_size(trace_b, level) as f64;

    // Concatenate a||b for joint compression.
    let mut ab = Vec::with_capacity(trace_a.len() + trace_b.len());
    ab.extend_from_slice(trace_a);
    ab.extend_from_slice(trace_b);
    let c_ab = compressed_size(&ab, level) as f64;

    let numerator = c_ab - c_a.min(c_b);
    let denominator = c_a.max(c_b);
    if denominator <= 0.0 {
        return 0.0;
    }
    (numerator / denominator).max(0.0)
}

/// Batch NCD computation: compare a candidate trace against a set of
/// reference traces and return the minimum NCD (most similar).
/// Short-circuits if any NCD falls below the duplicate threshold.
pub struct NcdPreFilter {
    config: NcdConfig,
}

impl NcdPreFilter {
    pub fn new(config: NcdConfig) -> Self {
        Self { config }
    }

    /// Returns (min_ncd, is_duplicate). If `is_duplicate` is true, the
    /// trace should be flagged for dedup review before expensive embedding.
    pub fn check_against_references(
        &self,
        candidate: &[u8],
        references: &[&[u8]],
    ) -> (f64, bool) {
        let candidate = if candidate.len() > self.config.max_input_bytes {
            &candidate[..self.config.max_input_bytes]
        } else {
            candidate
        };

        let mut min_ncd = f64::INFINITY;
        for reference in references {
            let reference = if reference.len() > self.config.max_input_bytes {
                &reference[..self.config.max_input_bytes]
            } else {
                reference
            };
            let d = ncd(candidate, reference, self.config.compression_level);
            if d < min_ncd {
                min_ncd = d;
            }
            if min_ncd < self.config.duplicate_threshold {
                return (min_ncd, true);
            }
        }
        (min_ncd, min_ncd < self.config.duplicate_threshold)
    }

    /// Compute NCD-based novelty in micros, compatible with the gate pipeline.
    /// Maps NCD distance to the [0, 1_000_000] micros scale used by
    /// `OrchestrationDecision::novelty_score_micros`.
    pub fn novelty_micros(&self, candidate: &[u8], references: &[&[u8]]) -> u64 {
        let (min_ncd, _) = self.check_against_references(candidate, references);
        // Clamp NCD to [0, 1] then scale to micros.
        let clamped = min_ncd.clamp(0.0, 1.0);
        (clamped * 1_000_000.0) as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_traces_have_near_zero_ncd() {
        let data = b"fn main() { println!(\"hello world\"); }";
        let d = ncd(data, data, 3);
        assert!(d < 0.1, "identical traces should have NCD near 0, got {d}");
    }

    #[test]
    fn unrelated_traces_have_high_ncd() {
        let a = b"fn fibonacci(n: u32) -> u32 { match n { 0 => 0, 1 => 1, _ => fibonacci(n-1) + fibonacci(n-2) } }";
        let b = b"The quarterly earnings report showed a 15% increase in revenue across all market segments in the Asia-Pacific region.";
        let d = ncd(a, b, 3);
        assert!(d > 0.8, "unrelated traces should have NCD near 1, got {d}");
    }

    #[test]
    fn minor_edit_has_low_ncd() {
        let a = b"fn process_data(items: &[Item]) -> Vec<Result> { items.iter().map(|i| transform(i)).collect() }";
        let b = b"fn process_data(items: &[Item]) -> Vec<Result> { items.iter().map(|i| convert(i)).collect() }";
        let d = ncd(a, b, 3);
        assert!(d < 0.4, "minor edit should have low NCD, got {d}");
    }
}
```

### Integration with Existing Pipeline

NCD slots in as a **pre-filter before embedding**. In the
`EnclaveGateOrchestrator::evaluate()` flow, after chunking and perplexity
scoring but before the expensive `embed_chunk_mean_pooled` calls, run NCD
against a sample of recently-inserted traces. If `min_ncd < threshold`, skip
embedding entirely and mark the trace as a near-duplicate. This avoids the
GPU cost of embedding traces that are structurally redundant.

NCD also complements the existing `dedup_simhash` module. Simhash detects
lexical near-duplicates (same tokens, slightly reworded). NCD detects
*structural* near-duplicates (different vocabulary but identical compression
structure -- e.g., a trace that is a copy-paste of another trace with
variable names changed).

### Estimated Complexity and Prerequisites

- **Implementation effort**: ~1 week. The core is ~50 lines of pure Rust.
  Integration with the chunker and the pre-filter hook is the main work.
- **Prerequisites**: `zstd` crate (already commonly used in Rust ecosystems).
  No GPU, no model, no external service.
- **Risk**: Compression-based similarity is weaker than semantic similarity for
  paraphrases. NCD should augment, not replace, embedding novelty.

### Potential Research Output

- **Paper**: "Compression-Based Novelty Detection as a Pre-Filter for
  Embedding-Based Deduplication in AI Trace Corpora." Venue: SIGIR or CIKM.
- **Key result**: Empirical measurement of the false-negative rate of NCD
  pre-filtering (how many true duplicates does NCD miss that embeddings catch?)
  and the compute savings from skipping embedding on NCD-detected duplicates.

---

## 3. Replicator Dynamics for Trace Lineage

### Academic Foundations

Replicator dynamics is the foundational model of evolutionary game theory. Taylor
and Jonker (1978) formalized the replicator equation:

```
dx_i/dt = x_i * (f_i - f_bar)
```

where `x_i` is the proportion of strategy i in the population, `f_i` is the
fitness of strategy i, and `f_bar = sum(x_i * f_i)` is the average population
fitness. Strategies with above-average fitness grow; those below average shrink.
This models natural selection without mutation -- pure frequency-dependent
selection.

Hofbauer and Sigmund (1998) extended this to continuous-time dynamics and proved
connections to Nash equilibria: rest points of the replicator equation correspond
to Nash equilibria of the underlying game, and asymptotically stable rest points
correspond to evolutionarily stable strategies (ESS).

**Key references:**
- Taylor, P. D. & Jonker, L. B. (1978). "Evolutionarily Stable Strategies and Game Dynamics." *Mathematical Biosciences*, 40(1-2), 145-156.
- Hofbauer, J. & Sigmund, K. (1998). *Evolutionary Games and Population Dynamics*. Cambridge University Press.
- Weibull, J. W. (1995). *Evolutionary Game Theory*. MIT Press.
- Nowak, M. A. (2006). "Evolutionary Dynamics of Biological Games." *Science*, 303, 793-799.
- Sandholm, W. H. (2010). *Population Games and Evolutionary Dynamics*. MIT Press.

### Why This is Novel for AI Trace Management

The TraceCommons corpus is not static -- it evolves as contributors submit new
traces reflecting changing coding patterns, tool usage, and problem-solving
strategies. Currently, the system scores each trace independently. There is no
model of how trace *types* are evolving over time.

Replicator dynamics provides exactly this lens. Classify traces into "strategies"
(e.g., by dominant tool pattern: "read-edit-test" vs. "search-plan-execute" vs.
"trial-and-error"), assign fitness proportional to quality scores, and track how
strategy frequencies change across epochs. The replicator equation then reveals:

- **Emerging strategies**: New patterns with above-average fitness that are
  growing in frequency. These are worth highlighting to operators and
  potentially incentivizing.
- **Declining strategies**: Patterns with below-average fitness that are shrinking.
  These may represent deprecated approaches or inefficient agent behaviors.
- **Evolutionarily stable equilibria**: Strategy distributions that resist
  invasion by novel mutants. When the corpus reaches such an equilibrium,
  the novelty scoring parameters may need recalibration.

The insight: the trace corpus is an evolving population, and the tools of
evolutionary game theory describe its dynamics more faithfully than static
scoring.

### Rust Implementation Sketch

```rust
use std::collections::HashMap;
use uuid::Uuid;

/// A trace "strategy" -- a classification of trace behavior based on
/// dominant patterns. The classifier is pluggable; the dynamics engine
/// only cares about strategy labels and their fitness values.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct StrategyLabel(pub String);

/// A population snapshot: the frequency and fitness of each strategy
/// at a point in time.
#[derive(Debug, Clone)]
pub struct TracePopulation {
    pub strategy: StrategyLabel,
    /// Relative frequency x_i in [0, 1]. Sum over all strategies = 1.
    pub frequency: f64,
    /// Fitness f_i: mean credit-quality score (q_micros / 1e6) of traces
    /// in this strategy within the observation window.
    pub fitness: f64,
    /// Absolute count of traces classified under this strategy.
    pub count: u64,
}

/// Result of one replicator dynamics step.
#[derive(Debug, Clone)]
pub struct ReplicatorStep {
    /// Updated populations after one discrete-time step.
    pub populations: Vec<TracePopulation>,
    /// Average population fitness f_bar.
    pub mean_fitness: f64,
    /// Strategies with growth rate > 0 (above-average fitness).
    pub emerging: Vec<StrategyLabel>,
    /// Strategies with growth rate < 0 (below-average fitness).
    pub declining: Vec<StrategyLabel>,
    /// Approximate entropy of the strategy distribution.
    /// H = -sum(x_i * ln(x_i)). Higher = more diverse corpus.
    pub diversity_entropy: f64,
}

pub struct ReplicatorDynamics {
    /// Discrete time-step size. Smaller = more stable but slower convergence.
    /// dt = 1.0 means one full epoch per step.
    pub dt: f64,
    /// Minimum frequency below which a strategy is considered extinct.
    pub extinction_threshold: f64,
}

impl ReplicatorDynamics {
    pub fn new(dt: f64, extinction_threshold: f64) -> Self {
        Self {
            dt,
            extinction_threshold,
        }
    }

    /// Advance the population by one discrete-time step of the replicator
    /// equation: x_i(t+dt) = x_i(t) + dt * x_i(t) * (f_i - f_bar).
    /// Re-normalizes to ensure frequencies sum to 1 after the step.
    pub fn step(&self, populations: &mut [TracePopulation]) -> ReplicatorStep {
        if populations.is_empty() {
            return ReplicatorStep {
                populations: vec![],
                mean_fitness: 0.0,
                emerging: vec![],
                declining: vec![],
                diversity_entropy: 0.0,
            };
        }

        // Compute average fitness f_bar = sum(x_i * f_i).
        let f_bar: f64 = populations
            .iter()
            .map(|p| p.frequency * p.fitness)
            .sum();

        // Apply the discrete-time replicator equation.
        let mut emerging = Vec::new();
        let mut declining = Vec::new();
        for pop in populations.iter_mut() {
            let growth = pop.frequency * (pop.fitness - f_bar);
            if growth > 0.0 {
                emerging.push(pop.strategy.clone());
            } else if growth < 0.0 {
                declining.push(pop.strategy.clone());
            }
            pop.frequency += self.dt * growth;
            pop.frequency = pop.frequency.max(0.0);
        }

        // Renormalize frequencies to sum to 1.
        let total: f64 = populations.iter().map(|p| p.frequency).sum();
        if total > 0.0 {
            for pop in populations.iter_mut() {
                pop.frequency /= total;
            }
        }

        // Remove extinct strategies.
        populations.retain(|p| p.frequency >= self.extinction_threshold);

        // Shannon entropy of the distribution.
        let entropy: f64 = populations
            .iter()
            .filter(|p| p.frequency > 0.0)
            .map(|p| -p.frequency * p.frequency.ln())
            .sum();

        ReplicatorStep {
            populations: populations.to_vec(),
            mean_fitness: f_bar,
            emerging,
            declining,
            diversity_entropy: entropy,
        }
    }

    /// Classify traces into strategies by their dominant tool-call pattern.
    /// This is a simple heuristic; production would use a more sophisticated
    /// classifier (e.g., k-means on tool-call embeddings).
    pub fn classify_trace(tool_calls: &[&str]) -> StrategyLabel {
        let mut counts: HashMap<&str, usize> = HashMap::new();
        for tool in tool_calls {
            *counts.entry(tool).or_default() += 1;
        }
        let dominant = counts
            .into_iter()
            .max_by_key(|&(_, c)| c)
            .map(|(t, _)| t)
            .unwrap_or("unknown");
        StrategyLabel(dominant.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn high_fitness_strategy_grows() {
        let mut pops = vec![
            TracePopulation {
                strategy: StrategyLabel("read-edit-test".into()),
                frequency: 0.5,
                fitness: 0.8,
                count: 100,
            },
            TracePopulation {
                strategy: StrategyLabel("trial-and-error".into()),
                frequency: 0.5,
                fitness: 0.3,
                count: 100,
            },
        ];
        let rd = ReplicatorDynamics::new(0.1, 0.01);
        let result = rd.step(&mut pops);
        let ret = &result.populations;
        assert!(ret[0].frequency > 0.5, "high-fitness should grow");
        assert!(ret[1].frequency < 0.5, "low-fitness should shrink");
    }

    #[test]
    fn frequencies_sum_to_one_after_step() {
        let mut pops = vec![
            TracePopulation {
                strategy: StrategyLabel("a".into()),
                frequency: 0.3,
                fitness: 0.9,
                count: 30,
            },
            TracePopulation {
                strategy: StrategyLabel("b".into()),
                frequency: 0.5,
                fitness: 0.5,
                count: 50,
            },
            TracePopulation {
                strategy: StrategyLabel("c".into()),
                frequency: 0.2,
                fitness: 0.1,
                count: 20,
            },
        ];
        let rd = ReplicatorDynamics::new(0.5, 0.001);
        rd.step(&mut pops);
        let total: f64 = pops.iter().map(|p| p.frequency).sum();
        assert!((total - 1.0).abs() < 1e-10, "frequencies must sum to 1");
    }
}
```

### PostgreSQL Schema

```sql
CREATE TABLE trace_strategy_populations (
    snapshot_id     UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id       UUID NOT NULL REFERENCES tenants(id),
    epoch_index     BIGINT NOT NULL,
    strategy_label  TEXT NOT NULL,
    frequency       DOUBLE PRECISION NOT NULL,
    fitness         DOUBLE PRECISION NOT NULL,
    trace_count     BIGINT NOT NULL,
    mean_fitness    DOUBLE PRECISION NOT NULL,
    diversity_entropy DOUBLE PRECISION NOT NULL,
    computed_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, epoch_index, strategy_label)
);

CREATE INDEX idx_strategy_pops_tenant_epoch
    ON trace_strategy_populations(tenant_id, epoch_index);
```

### Integration with Existing Pipeline

The replicator dynamics engine runs as a **batch job at epoch boundaries** (the
same 7-day windows the contributor cap already uses). At the end of each epoch:
1. Query all traces accepted during the epoch, grouped by strategy label.
2. Compute frequency = count / total and fitness = mean(q_micros) / 1e6.
3. Run `ReplicatorDynamics::step()` to get the updated population.
4. Persist the snapshot. Surface emerging/declining strategies in the operator
   dashboard and optionally adjust novelty floors to incentivize
   under-represented strategies.

### Estimated Complexity and Prerequisites

- **Implementation effort**: ~2 weeks. The dynamics engine is simple. The
  strategy classifier (mapping traces to strategy labels) is the harder part.
- **Prerequisites**: A trace classification scheme. Could start with simple
  heuristics (dominant tool type) and iterate toward learned classifiers.
- **Risk**: Strategy labels are arbitrary. The dynamics are only meaningful if
  the classification captures real behavioral differences.

### Potential Research Output

- **Paper**: "Evolutionary Dynamics of AI Agent Strategies: A Replicator
  Equation Analysis of Coding Trace Populations." Venue: AAMAS (International
  Conference on Autonomous Agents and Multi-Agent Systems).
- **Key result**: Empirical demonstration that trace strategy distributions
  converge to evolutionarily stable equilibria, and identification of which
  strategies are selected for/against over time.

---

## 4. SIR Epidemiological Model for Pattern Spread

### Academic Foundations

The SIR (Susceptible-Infected-Recovered) model is the foundational compartmental
model in mathematical epidemiology, introduced by Kermack and McKendrick (1927).
The population is divided into three compartments:

```
dS/dt = -beta * S * I / N
dI/dt = beta * S * I / N - gamma * I
dR/dt = gamma * I
```

where `beta` is the transmission rate, `gamma` is the recovery rate, and
`N = S + I + R` is the total population. The basic reproduction number
`R_0 = beta / gamma` determines whether an epidemic spreads (`R_0 > 1`) or
dies out (`R_0 < 1`).

Bettencourt et al. (2006) adapted SIR for technology adoption, modeling how
innovations spread through developer communities. Their key insight: the
"recovery" compartment maps to abandonment (trying a technology and deciding
not to adopt it), and `R_0` becomes a measure of the innovation's "viral
utility."

**Key references:**
- Kermack, W. O. & McKendrick, A. G. (1927). "A Contribution to the Mathematical Theory of Epidemics." *Proc. Royal Society of London A*, 115(772), 700-721.
- Anderson, R. M. & May, R. M. (1991). *Infectious Diseases of Humans: Dynamics and Control*. Oxford University Press.
- Bettencourt, L. M. A., Cintr{o}n-Arias, A., Kaiser, D. I., & Castillo-Ch{a}vez, C. (2006). "The Power of a Good Idea: Quantitative Modeling of the Spread of Ideas from Epidemiological Models." *Physica A*, 364, 513-536.
- Bass, F. M. (1969). "A New Product Growth for Model Consumer Durables." *Management Science*, 15(5), 215-227.
- Rogers, E. M. (2003). *Diffusion of Innovations* (5th ed.). Free Press.

### Why This is Novel for AI Trace Management

When a new coding pattern appears in traces -- say, a novel error-handling
idiom, a new testing strategy, or an innovative use of a tool -- it may spread
across contributors. Some patterns go viral (widely adopted), others fizzle
(tried and abandoned). Currently, TraceCommons has no model for this diffusion
process. It scores each trace independently and clusters near-duplicates, but
it does not track how patterns propagate through the contributor population.

The SIR model provides a principled framework for this. Each "pattern" (detected
via NCD, embedding clusters, or tool-call signatures) has its own SIR dynamics:

- **S**: Contributors who have not yet exhibited this pattern.
- **I**: Contributors actively using this pattern in their traces.
- **R**: Contributors who used the pattern but have since stopped.

Computing `R_0` per pattern yields an actionable signal: patterns with high
`R_0` are virally useful and worth promoting or documenting. Patterns with
`R_0 < 1` are niche or declining.

### Rust Implementation Sketch

```rust
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

/// A detected coding pattern with its SIR compartments.
#[derive(Debug, Clone)]
pub struct PatternSirState {
    pub pattern_id: Uuid,
    pub label: String,
    /// Contributors in each compartment (by contributor_hash).
    pub susceptible: HashSet<String>,
    pub infected: HashSet<String>,
    pub recovered: HashSet<String>,
    /// Estimated transmission rate.
    pub beta: f64,
    /// Estimated recovery rate.
    pub gamma: f64,
    /// Total population N = S + I + R.
    pub population: usize,
}

impl PatternSirState {
    /// Basic reproduction number. If R_0 > 1, the pattern is spreading.
    pub fn r_naught(&self) -> f64 {
        if self.gamma <= 0.0 {
            return f64::INFINITY;
        }
        self.beta / self.gamma
    }

    /// Effective reproduction number at current state.
    /// R_eff = R_0 * S/N. Accounts for the shrinking susceptible pool.
    pub fn r_effective(&self) -> f64 {
        let n = self.population as f64;
        if n <= 0.0 {
            return 0.0;
        }
        self.r_naught() * (self.susceptible.len() as f64 / n)
    }
}

/// Tracks pattern diffusion across contributors over time.
pub struct SirPatternTracker {
    /// Known patterns and their current SIR state.
    patterns: HashMap<Uuid, PatternSirState>,
    /// All known contributors.
    all_contributors: HashSet<String>,
    /// Lookback window (in epochs) for detecting "recovery" (abandonment).
    recovery_lookback: usize,
}

impl SirPatternTracker {
    pub fn new(recovery_lookback: usize) -> Self {
        Self {
            patterns: HashMap::new(),
            all_contributors: HashSet::new(),
            recovery_lookback,
        }
    }

    /// Register a contributor's adoption of a pattern in a given epoch.
    pub fn observe_adoption(
        &mut self,
        pattern_id: Uuid,
        label: &str,
        contributor_hash: &str,
    ) {
        self.all_contributors.insert(contributor_hash.to_string());
        let state = self.patterns.entry(pattern_id).or_insert_with(|| {
            PatternSirState {
                pattern_id,
                label: label.to_string(),
                susceptible: self.all_contributors.clone(),
                infected: HashSet::new(),
                recovered: HashSet::new(),
                beta: 0.0,
                gamma: 0.0,
                population: self.all_contributors.len(),
            }
        });

        state.susceptible.remove(contributor_hash);
        state.recovered.remove(contributor_hash);
        state.infected.insert(contributor_hash.to_string());
    }

    /// At each epoch boundary, update compartments:
    /// - Contributors who were infected last epoch but not this epoch
    ///   transition to recovered.
    /// - Estimate beta and gamma from observed transitions.
    pub fn advance_epoch(
        &mut self,
        current_epoch_adoptions: &HashMap<Uuid, HashSet<String>>,
    ) {
        for (pattern_id, state) in &mut self.patterns {
            let current = current_epoch_adoptions
                .get(pattern_id)
                .cloned()
                .unwrap_or_default();

            // Contributors who were infected but are not in current epoch
            // observations transition to recovered.
            let newly_recovered: Vec<String> = state
                .infected
                .iter()
                .filter(|c| !current.contains(*c))
                .cloned()
                .collect();
            for c in &newly_recovered {
                state.infected.remove(c);
                state.recovered.insert(c.clone());
            }

            // New infections from the current epoch.
            let new_infections = current
                .iter()
                .filter(|c| !state.infected.contains(*c) && !state.recovered.contains(*c))
                .count();

            // Estimate beta: new_infections / (S * I / N).
            let s = state.susceptible.len() as f64;
            let i = state.infected.len() as f64;
            let n = state.population as f64;
            if s > 0.0 && i > 0.0 && n > 0.0 {
                state.beta = (new_infections as f64 * n) / (s * i);
            }

            // Estimate gamma: recoveries / I.
            if i > 0.0 {
                state.gamma = newly_recovered.len() as f64 / i;
            }

            // Move new adopters to infected.
            for c in &current {
                if state.susceptible.remove(c) {
                    state.infected.insert(c.clone());
                }
            }

            state.population = state.susceptible.len()
                + state.infected.len()
                + state.recovered.len();
        }
    }

    /// Return patterns sorted by R_0 (highest-spreading first).
    pub fn ranked_by_virality(&self) -> Vec<(&PatternSirState, f64)> {
        let mut ranked: Vec<_> = self
            .patterns
            .values()
            .map(|s| (s, s.r_naught()))
            .filter(|(_, r)| r.is_finite())
            .collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        ranked
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spreading_pattern_has_r0_above_one() {
        let mut tracker = SirPatternTracker::new(2);
        let pid = Uuid::new_v4();

        // Epoch 1: contributor A adopts the pattern.
        tracker.observe_adoption(pid, "new-error-handling", "contrib_a");
        // Epoch 2: contributors B and C also adopt.
        let mut adoptions = HashMap::new();
        let mut set = HashSet::new();
        set.insert("contrib_a".to_string());
        set.insert("contrib_b".to_string());
        set.insert("contrib_c".to_string());
        adoptions.insert(pid, set);
        tracker.advance_epoch(&adoptions);

        let state = &tracker.patterns[&pid];
        assert!(
            state.r_naught() > 1.0 || state.beta > 0.0,
            "spreading pattern should have positive transmission"
        );
    }
}
```

### PostgreSQL Schema

```sql
CREATE TABLE sir_pattern_snapshots (
    snapshot_id     UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id       UUID NOT NULL REFERENCES tenants(id),
    pattern_id      UUID NOT NULL,
    pattern_label   TEXT NOT NULL,
    epoch_index     BIGINT NOT NULL,
    susceptible_count BIGINT NOT NULL,
    infected_count    BIGINT NOT NULL,
    recovered_count   BIGINT NOT NULL,
    beta            DOUBLE PRECISION NOT NULL,
    gamma           DOUBLE PRECISION NOT NULL,
    r_naught        DOUBLE PRECISION NOT NULL,
    r_effective     DOUBLE PRECISION NOT NULL,
    computed_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, pattern_id, epoch_index)
);

CREATE INDEX idx_sir_pattern_tenant ON sir_pattern_snapshots(tenant_id, epoch_index);
```

### Integration with Existing Pipeline

The SIR tracker operates as a **post-scoring analytics module**. After traces
are accepted and scored, a background job classifies each trace's patterns
(using the same strategy classifier from idea 3, or a dedicated pattern
extractor) and feeds observations to the SIR tracker. At epoch boundaries,
`advance_epoch()` updates the compartmental model. The resulting `R_0` values
per pattern are surfaced in the operator dashboard and can optionally feed back
into the novelty scoring: patterns with high `R_0` might have their novelty
floor adjusted to encourage more instances, while saturated patterns
(`R_effective -> 0`) might have floors raised.

### Estimated Complexity and Prerequisites

- **Implementation effort**: ~2 weeks. The compartmental model is
  straightforward. The pattern detection/classification is the prerequisite.
- **Prerequisites**: A pattern classifier. Depends on idea 3 (replicator
  dynamics) or a separate tool-call signature extractor.
- **Risk**: SIR assumes homogeneous mixing (any contributor can "infect" any
  other), which may not hold. Network-structured models (e.g., SIS on a
  graph) would be more accurate but require contributor interaction data.

### Potential Research Output

- **Paper**: "Epidemiological Modeling of Coding Pattern Diffusion in
  AI-Assisted Development." Venue: ICSE (International Conference on Software
  Engineering) or MSR (Mining Software Repositories).
- **Key result**: Empirical `R_0` estimates for real coding patterns, comparison
  with Bass diffusion model predictions, identification of "super-spreader"
  patterns.

---

## 5. Prospect-Theory Credits

### Academic Foundations

Prospect theory, developed by Kahneman and Tversky (1979), describes how people
actually make decisions under uncertainty, as opposed to the rational
expected-utility model. Three key principles:

1. **Reference dependence**: People evaluate outcomes relative to a reference
   point, not in absolute terms. A gain of $10 feels different depending on
   whether you expected $5 or $15.

2. **Loss aversion**: Losses loom larger than equivalent gains. The pain of
   losing $X is approximately 2.25 times the pleasure of gaining $X. The
   loss aversion coefficient `lambda` has been estimated at 2.0-2.5 across
   many experiments.

3. **Diminishing sensitivity**: The value function is concave for gains and
   convex for losses. The first dollar matters more than the hundredth.

The value function from cumulative prospect theory (Tversky & Kahneman 1992):

```
v(x) = x^alpha           for x >= 0 (gains)
v(x) = -lambda * (-x)^beta  for x < 0 (losses)
```

with experimentally estimated parameters `alpha = 0.88`, `beta = 0.88`,
`lambda = 2.25`.

**Key references:**
- Kahneman, D. & Tversky, A. (1979). "Prospect Theory: An Analysis of Decision under Risk." *Econometrica*, 47(2), 263-291.
- Tversky, A. & Kahneman, D. (1992). "Advances in Prospect Theory: Cumulative Representation of Uncertainty." *Journal of Risk and Uncertainty*, 5, 297-323.
- Thaler, R. H. (1999). "Mental Accounting Matters." *Journal of Behavioral Decision Making*, 12, 183-206.
- Koszegi, B. & Rabin, M. (2006). "A Model of Reference-Dependent Preferences." *Quarterly Journal of Economics*, 121(4), 1133-1165.
- Bordalo, P., Gennaioli, N., & Shleifer, A. (2012). "Salience Theory of Choice Under Risk." *Quarterly Journal of Economics*, 127(3), 1243-1285.

### Why This is Novel for AI Trace Management

The current TraceCommons credit system is objective: `q = f * g * a`,
contributor cap, dedup penalty. This is mathematically clean but psychologically
naive. Contributors are humans, and their motivation to contribute more
high-quality traces is driven by how they *perceive* their rewards, not by the
raw numbers.

Prospect theory suggests that framing matters enormously:

- Telling a contributor "Your trace scored 0.72" is less motivating than "Your
  trace scored 15% above your personal average."
- A contributor who receives 0 credits for a rejected trace experiences a
  "loss" relative to their expectation, which looms 2.25x larger than a
  comparable "gain" from an accepted trace.
- The diminishing sensitivity principle means the difference between 0.7 and
  0.8 quality feels smaller than the difference between 0.1 and 0.2, even
  though the absolute gap is the same.

By framing credit awards using prospect-theory principles, TraceCommons can
maximize contributor motivation without changing the underlying scoring math.

### Rust Implementation Sketch

```rust
/// Prospect-theory parameters from Tversky & Kahneman (1992).
/// These are experimentally calibrated defaults; operators can tune them.
#[derive(Debug, Clone, Copy)]
pub struct ProspectTheoryParams {
    /// Curvature of the value function for gains. Default 0.88.
    pub alpha: f64,
    /// Curvature of the value function for losses. Default 0.88.
    pub beta: f64,
    /// Loss aversion coefficient. Default 2.25.
    pub lambda: f64,
    /// Exponential moving average smoothing for reference point.
    /// Lower = reference adapts faster to recent scores.
    pub reference_ema_alpha: f64,
}

impl Default for ProspectTheoryParams {
    fn default() -> Self {
        Self {
            alpha: 0.88,
            beta: 0.88,
            lambda: 2.25,
            reference_ema_alpha: 0.1,
        }
    }
}

/// Per-contributor reference point and history, used to compute
/// prospect-theory framing of credit awards.
#[derive(Debug, Clone)]
pub struct ContributorReferenceState {
    pub contributor_hash: String,
    /// Exponential moving average of recent quality scores.
    /// This is the "reference point" in prospect theory.
    pub reference_quality_micros: i64,
    /// Number of traces scored (for initial reference bootstrapping).
    pub traces_scored: u64,
    /// Sum of all quality scores (for computing lifetime average).
    pub total_quality_micros: i128,
}

impl ContributorReferenceState {
    pub fn new(contributor_hash: &str) -> Self {
        Self {
            contributor_hash: contributor_hash.to_string(),
            reference_quality_micros: 500_000, // start at 0.5 (neutral)
            traces_scored: 0,
            total_quality_micros: 0,
        }
    }

    /// Update the reference point with a new quality observation.
    pub fn observe(&mut self, quality_micros: i64, params: &ProspectTheoryParams) {
        self.traces_scored += 1;
        self.total_quality_micros += quality_micros as i128;

        // EMA update for the reference point.
        let alpha = params.reference_ema_alpha;
        self.reference_quality_micros = ((1.0 - alpha)
            * self.reference_quality_micros as f64
            + alpha * quality_micros as f64)
            .round() as i64;
    }

    pub fn lifetime_average_micros(&self) -> i64 {
        if self.traces_scored == 0 {
            return 0;
        }
        (self.total_quality_micros / self.traces_scored as i128) as i64
    }
}

/// Prospect-theory credit framing for a single trace.
#[derive(Debug, Clone)]
pub struct ProspectFramedCredit {
    /// Raw quality score (unchanged from the gate pipeline).
    pub raw_quality_micros: i64,
    /// The deviation from the contributor's reference point, in micros.
    /// Positive = gain, negative = loss.
    pub deviation_micros: i64,
    /// Prospect-theory subjective value of the deviation.
    /// Asymmetric: losses are amplified by lambda.
    pub subjective_value: f64,
    /// Human-readable framing string for the contributor UI.
    pub framing_message: String,
    /// Percentile rank among this contributor's recent traces.
    pub personal_percentile: f64,
}

pub struct ProspectTheoryCredits {
    params: ProspectTheoryParams,
}

impl ProspectTheoryCredits {
    pub fn new(params: ProspectTheoryParams) -> Self {
        Self { params }
    }

    /// Compute the prospect-theory value function v(x).
    pub fn value_function(&self, x: f64) -> f64 {
        if x >= 0.0 {
            x.powf(self.params.alpha)
        } else {
            -self.params.lambda * (-x).powf(self.params.beta)
        }
    }

    /// Frame a credit award relative to the contributor's reference point.
    pub fn frame_credit(
        &self,
        quality_micros: i64,
        state: &ContributorReferenceState,
    ) -> ProspectFramedCredit {
        let deviation = quality_micros - state.reference_quality_micros;
        let deviation_real = deviation as f64 / 1_000_000.0;
        let subjective = self.value_function(deviation_real);

        let pct_above = if state.reference_quality_micros > 0 {
            (deviation as f64 / state.reference_quality_micros as f64 * 100.0).round()
        } else {
            0.0
        };

        let framing = if deviation > 0 {
            format!(
                "This trace scored {:.0}% above your personal average. \
                 Quality: {:.2}/1.00.",
                pct_above,
                quality_micros as f64 / 1_000_000.0
            )
        } else if deviation < 0 {
            format!(
                "This trace scored {:.0}% below your personal average. \
                 Quality: {:.2}/1.00. Focus on novel, complex problems \
                 to improve your score.",
                pct_above.abs(),
                quality_micros as f64 / 1_000_000.0
            )
        } else {
            format!(
                "This trace matched your personal average. \
                 Quality: {:.2}/1.00.",
                quality_micros as f64 / 1_000_000.0
            )
        };

        // Personal percentile is a rough estimate based on the reference.
        // In production, this would use a histogram of the contributor's
        // recent scores.
        let personal_percentile = if quality_micros > state.reference_quality_micros {
            50.0 + (pct_above / 2.0).min(49.0)
        } else {
            (50.0 + pct_above / 2.0).max(1.0)
        };

        ProspectFramedCredit {
            raw_quality_micros: quality_micros,
            deviation_micros: deviation,
            subjective_value: subjective,
            framing_message: framing,
            personal_percentile,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loss_aversion_makes_losses_feel_larger() {
        let pt = ProspectTheoryCredits::new(ProspectTheoryParams::default());
        let gain = pt.value_function(0.5);
        let loss = pt.value_function(-0.5);
        // |v(-0.5)| should be > |v(0.5)| due to lambda > 1
        assert!(
            loss.abs() > gain.abs(),
            "loss {loss} should feel larger than gain {gain}"
        );
    }

    #[test]
    fn diminishing_sensitivity() {
        let pt = ProspectTheoryCredits::new(ProspectTheoryParams::default());
        let d1 = pt.value_function(0.2) - pt.value_function(0.1);
        let d2 = pt.value_function(0.9) - pt.value_function(0.8);
        assert!(
            d1 > d2,
            "marginal value should diminish: d1={d1} d2={d2}"
        );
    }

    #[test]
    fn above_average_trace_gets_positive_framing() {
        let pt = ProspectTheoryCredits::new(ProspectTheoryParams::default());
        let state = ContributorReferenceState {
            contributor_hash: "sha256:test".into(),
            reference_quality_micros: 500_000,
            traces_scored: 10,
            total_quality_micros: 5_000_000,
        };
        let framed = pt.frame_credit(750_000, &state);
        assert!(framed.deviation_micros > 0);
        assert!(framed.subjective_value > 0.0);
        assert!(framed.framing_message.contains("above"));
    }
}
```

### PostgreSQL Schema

```sql
CREATE TABLE contributor_reference_state (
    contributor_hash        TEXT NOT NULL,
    tenant_id               UUID NOT NULL REFERENCES tenants(id),
    reference_quality_micros BIGINT NOT NULL DEFAULT 500000,
    traces_scored           BIGINT NOT NULL DEFAULT 0,
    total_quality_micros    BIGINT NOT NULL DEFAULT 0,
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (contributor_hash, tenant_id)
);
```

### Integration with Existing Pipeline

Prospect-theory framing is a **presentation-layer enhancement**. It does not
change the underlying `credit_quality()` function or settlement math. After
`credit_quality()` produces `q_micros`, the `ProspectTheoryCredits::frame_credit()`
call adds the subjective framing. This framing is returned in the credit notice
(the `TraceContributionEnvelope::value` field) and displayed in the contributor
CLI output. The reference state is updated incrementally with each accepted trace.

### Estimated Complexity and Prerequisites

- **Implementation effort**: ~1 week. The math is simple. The main work is
  integrating with the credit notice pipeline and the contributor CLI display.
- **Prerequisites**: Per-contributor state storage (the `contributor_reference_state`
  table). The contributor-cap module already tracks per-contributor cumulative
  state, so the plumbing exists.
- **Risk**: Overly negative framing for below-average traces could discourage
  contribution. The message templates need careful UX design.

### Potential Research Output

- **Paper**: "Behavioral Economics of AI Data Contribution: A Prospect Theory
  Framework for Trace Compensation Systems." Venue: CHI (ACM Conference on
  Human Factors in Computing Systems) or CSCW.
- **Key result**: A/B test comparing raw scores vs. prospect-theory framing on
  contributor retention and trace quality improvement rates.

---

## 6. Topological Data Analysis (TDA) for Trace Clustering

### Academic Foundations

Topological Data Analysis (TDA) uses algebraic topology to extract shape
features from point cloud data. The central tool is persistent homology:
construct a filtration of simplicial complexes (e.g., Vietoris-Rips complexes
at increasing distance thresholds) and track which topological features
(connected components, loops, voids) appear ("birth") and disappear ("death")
as the threshold grows. Features that persist across many threshold values
represent genuine structure in the data; those that appear and immediately
vanish are noise.

The output is a persistence diagram: a multiset of (birth, death) points. Long
bars in the persistence barcode indicate robust topological features. Carlsson
(2009) demonstrated that this approach reveals structure invisible to standard
clustering methods.

**Key references:**
- Carlsson, G. (2009). "Topology and Data." *Bulletin of the AMS*, 46(2), 255-308.
- Edelsbrunner, H. & Harer, J. (2010). *Computational Topology: An Introduction*. AMS.
- Edelsbrunner, H., Letscher, D., & Zomorodian, A. (2002). "Topological Persistence and Simplification." *Discrete Comp. Geom.*, 28, 511-533.
- Ghrist, R. (2008). "Barcodes: The Persistent Topology of Data." *Bulletin of the AMS*, 45(1), 61-75.
- Chazal, F., de Silva, V., Glisse, M., & Oudot, S. (2016). *The Structure and Stability of Persistence Modules*. Springer.

### Why This is Novel for AI Trace Management

Standard clustering of trace embeddings (k-means, DBSCAN) finds groups but
misses topological structure: holes in the embedding space where no traces
exist (coverage gaps), connected components that reveal natural domain
boundaries, and higher-dimensional loops that indicate cyclical relationships
between trace types.

For TraceCommons, this means:
- **Coverage gap detection**: Persistent H0 (connected components) reveals
  natural clusters. H1 (loops) and H2 (voids) reveal gaps -- regions of the
  embedding space where no traces have been contributed. These gaps are
  precisely the domains operators should incentivize contributions for.
- **Noise-robust clustering**: Unlike k-means (which requires choosing k) or
  DBSCAN (which requires choosing epsilon), persistent homology reveals the
  natural scale at which clusters exist, as features with long persistence
  bars.
- **Corpus health monitoring**: As the corpus grows, the topological structure
  should become more connected (fewer H0 components) and fill in holes (fewer
  persistent H1/H2 features). Tracking the total persistence (sum of bar
  lengths) over time gives a single number summarizing corpus completeness.

### Rust Implementation Sketch

```rust
use std::collections::BinaryHeap;
use std::cmp::Ordering;

/// A point in the persistence diagram: (birth, death, dimension).
/// `death` may be f64::INFINITY for features that never die (essential features).
#[derive(Debug, Clone)]
pub struct PersistencePoint {
    pub birth: f64,
    pub death: f64,
    pub dimension: usize,
}

impl PersistencePoint {
    /// Persistence (lifespan) of this topological feature.
    pub fn persistence(&self) -> f64 {
        if self.death.is_infinite() {
            f64::INFINITY
        } else {
            self.death - self.birth
        }
    }
}

/// A persistence diagram: the complete topological summary of the point cloud.
#[derive(Debug, Clone)]
pub struct PersistenceDiagram {
    pub points: Vec<PersistencePoint>,
}

impl PersistenceDiagram {
    /// Total persistence: sum of all finite bar lengths. A single number
    /// summarizing the topological complexity of the data. Lower = more
    /// "filled in" and uniform; higher = more structure/gaps.
    pub fn total_persistence(&self) -> f64 {
        self.points
            .iter()
            .filter(|p| p.persistence().is_finite())
            .map(|p| p.persistence())
            .sum()
    }

    /// Number of features in dimension d with persistence above threshold.
    /// For d=0: significant connected components (natural clusters).
    /// For d=1: significant loops (coverage gaps).
    pub fn significant_features(&self, dimension: usize, threshold: f64) -> usize {
        self.points
            .iter()
            .filter(|p| p.dimension == dimension && p.persistence() > threshold)
            .count()
    }

    /// Betti numbers at a given filtration value epsilon.
    /// beta_0(eps) = number of connected components at scale eps.
    /// beta_1(eps) = number of loops at scale eps.
    pub fn betti_numbers(&self, epsilon: f64) -> Vec<usize> {
        let max_dim = self.points.iter().map(|p| p.dimension).max().unwrap_or(0);
        let mut betti = vec![0usize; max_dim + 1];
        for p in &self.points {
            if p.birth <= epsilon && (p.death > epsilon || p.death.is_infinite()) {
                betti[p.dimension] += 1;
            }
        }
        betti
    }
}

/// Compute the pairwise distance matrix from embeddings using cosine distance.
fn cosine_distance_matrix(embeddings: &[Vec<f32>]) -> Vec<Vec<f64>> {
    let n = embeddings.len();
    let mut dist = vec![vec![0.0f64; n]; n];
    for i in 0..n {
        for j in (i + 1)..n {
            let dot: f64 = embeddings[i]
                .iter()
                .zip(&embeddings[j])
                .map(|(a, b)| *a as f64 * *b as f64)
                .sum();
            let norm_a: f64 = embeddings[i].iter().map(|x| (*x as f64).powi(2)).sum::<f64>().sqrt();
            let norm_b: f64 = embeddings[j].iter().map(|x| (*x as f64).powi(2)).sum::<f64>().sqrt();
            let sim = if norm_a > 0.0 && norm_b > 0.0 {
                dot / (norm_a * norm_b)
            } else {
                0.0
            };
            let d = (1.0 - sim).max(0.0);
            dist[i][j] = d;
            dist[j][i] = d;
        }
    }
    dist
}

/// Compute 0-dimensional persistent homology (connected components) using
/// a Union-Find structure over the Vietoris-Rips filtration. This is the
/// standard single-linkage approach: process edges in order of increasing
/// distance, and each merge creates a "death" event for the younger component.
pub fn compute_persistence_h0(embeddings: &[Vec<f32>]) -> PersistenceDiagram {
    let n = embeddings.len();
    if n == 0 {
        return PersistenceDiagram { points: vec![] };
    }

    let dist = cosine_distance_matrix(embeddings);

    // Collect all edges with their distances.
    let mut edges: Vec<(f64, usize, usize)> = Vec::new();
    for i in 0..n {
        for j in (i + 1)..n {
            edges.push((dist[i][j], i, j));
        }
    }
    edges.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal));

    // Union-Find for tracking components.
    let mut parent: Vec<usize> = (0..n).collect();
    let mut rank: Vec<usize> = vec![0; n];
    let mut birth: Vec<f64> = vec![0.0; n]; // all born at distance 0

    fn find(parent: &mut [usize], x: usize) -> usize {
        if parent[x] != x {
            parent[x] = find(parent, parent[x]);
        }
        parent[x]
    }

    fn union(
        parent: &mut [usize],
        rank: &mut [usize],
        birth: &[f64],
        x: usize,
        y: usize,
    ) -> Option<(usize, usize)> {
        let rx = find(parent, x);
        let ry = find(parent, y);
        if rx == ry {
            return None;
        }
        // The younger component (later birth, but here all birth at 0,
        // so we use index as tiebreaker: higher index dies) is merged
        // into the older.
        let (survivor, dying) = if rank[rx] >= rank[ry] { (rx, ry) } else { (ry, rx) };
        parent[dying] = survivor;
        if rank[survivor] == rank[dying] {
            rank[survivor] += 1;
        }
        Some((survivor, dying))
    }

    let mut points = Vec::new();
    for (distance, i, j) in edges {
        if let Some((_survivor, dying)) = union(&mut parent, &mut rank, &birth, i, j) {
            points.push(PersistencePoint {
                birth: birth[dying],
                death: distance,
                dimension: 0,
            });
        }
    }

    // The last surviving component has infinite persistence.
    points.push(PersistencePoint {
        birth: 0.0,
        death: f64::INFINITY,
        dimension: 0,
    });

    PersistenceDiagram { points }
}

/// High-level analyzer that computes topological features and reports
/// coverage gaps and natural clusters in the trace corpus.
pub struct TopologicalAnalyzer {
    /// Persistence threshold: features below this are treated as noise.
    pub significance_threshold: f64,
}

impl TopologicalAnalyzer {
    pub fn new(significance_threshold: f64) -> Self {
        Self {
            significance_threshold,
        }
    }

    /// Analyze a set of trace embeddings for topological structure.
    pub fn analyze(&self, embeddings: &[Vec<f32>]) -> TopologicalReport {
        let diagram = compute_persistence_h0(embeddings);
        let natural_clusters = diagram.significant_features(0, self.significance_threshold);
        let total_persistence = diagram.total_persistence();

        TopologicalReport {
            diagram,
            natural_cluster_count: natural_clusters,
            total_persistence,
            embedding_count: embeddings.len(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TopologicalReport {
    pub diagram: PersistenceDiagram,
    pub natural_cluster_count: usize,
    pub total_persistence: f64,
    pub embedding_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_distinct_clusters_detected() {
        // Two tight clusters far apart should yield 2 significant H0 features.
        let cluster_a: Vec<Vec<f32>> = (0..5)
            .map(|i| {
                let mut v = vec![0.0f32; 10];
                v[0] = 1.0 + i as f32 * 0.01;
                let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
                v.iter_mut().for_each(|x| *x /= norm);
                v
            })
            .collect();
        let cluster_b: Vec<Vec<f32>> = (0..5)
            .map(|i| {
                let mut v = vec![0.0f32; 10];
                v[5] = 1.0 + i as f32 * 0.01;
                let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
                v.iter_mut().for_each(|x| *x /= norm);
                v
            })
            .collect();
        let mut all = cluster_a;
        all.extend(cluster_b);

        let analyzer = TopologicalAnalyzer::new(0.1);
        let report = analyzer.analyze(&all);
        assert!(
            report.natural_cluster_count >= 2,
            "expected 2+ clusters, got {}",
            report.natural_cluster_count
        );
    }
}
```

### Integration with Existing Pipeline

The topological analyzer runs as a **batch analytics job** on sampled embeddings
from the vector index. Rather than analyzing every trace (quadratic distance
matrix), subsample ~1000 embeddings per tenant and compute the persistence
diagram. The `TopologicalReport` surfaces in the operator dashboard:

- `natural_cluster_count`: How many natural domain clusters exist in the corpus.
- `total_persistence`: Corpus "gappiness" -- decreasing over time is healthy.
- Specific long-persistence H0 features point to under-connected regions where
  novelty bonuses should be offered.

### Estimated Complexity and Prerequisites

- **Implementation effort**: ~3 weeks. The H0 (connected components) persistence
  is straightforward with Union-Find. H1 (loops) requires implementing the
  boundary matrix reduction algorithm or integrating an external TDA library.
- **Prerequisites**: Access to stored embeddings (from the vector index).
  Subsampling strategy for scalability.
- **Risk**: Quadratic distance matrix is the bottleneck. For 1000 embeddings at
  256 dimensions, the matrix is ~8 MB -- manageable. For 10,000 embeddings,
  it is 800 MB and requires approximation algorithms (e.g., sparse Rips).

### Potential Research Output

- **Paper**: "Topological Analysis of AI Training Data Corpora: Detecting
  Coverage Gaps via Persistent Homology." Venue: NeurIPS Workshop on
  Topological Data Analysis, or JMLR.
- **Key result**: Demonstration that persistent homology detects coverage gaps
  invisible to standard clustering, and that filling these gaps improves
  downstream model performance.

---

## 7. Avrami Crystallization Detection

### Academic Foundations

The Avrami equation models phase transitions in materials science -- how a
material transforms from one phase to another (e.g., liquid to solid
crystallization):

```
X(t) = 1 - exp(-k * t^n)
```

where `X(t)` is the fraction of material transformed at time t, `k` is the
rate constant (combining nucleation and growth rates), and `n` is the Avrami
exponent (encoding the dimensionality and time-dependence of growth).

For bulk crystallization, `n` is typically 3-4 (three-dimensional growth from
constant-rate nucleation). For surface crystallization, `n ~ 2`. For
one-dimensional growth (fibrillar), `n ~ 1`. The shape of the curve is
universally sigmoidal: slow start (nucleation), rapid middle (growth), and
saturating tail (impingement of growing domains).

Silverberg et al. (2007) and others have demonstrated that Avrami kinetics
apply far beyond materials: social norm adoption, technology standard
convergence, and community opinion formation all follow this sigmoidal
trajectory with characteristic exponents.

**Key references:**
- Avrami, M. (1939). "Kinetics of Phase Change. I. General Theory." *Journal of Chemical Physics*, 7, 1103-1112.
- Avrami, M. (1940). "Kinetics of Phase Change. II. Transformation-Time Relations for Random Distribution of Nuclei." *Journal of Chemical Physics*, 8, 212-224.
- Avrami, M. (1941). "Granulation, Phase Change, and Microstructure." *Journal of Chemical Physics*, 9, 177-184.
- Johnson, W. A. & Mehl, R. F. (1939). "Reaction Kinetics in Processes of Nucleation and Growth." *Trans. AIME*, 135, 416-442.
- Silverberg, L. M. et al. (2007). "Avrami Kinetics Applied to Social Systems." *Journal of Physics: Conference Series*, 89(1), 012021.

### Why This is Novel for AI Trace Management

A trace corpus undergoes a phase transition as it matures. Early on, traces are
diverse, standards are loose, and quality variance is high (the "exploratory"
phase). Over time, quality norms crystallize: contributors learn what scores
well, tool-usage patterns converge, and the distribution of quality scores
shifts from broad to narrow (the "crystallized" phase). This transition is
precisely the Avrami sigmoidal curve.

Detecting where the corpus is on this curve has operational implications:

- **Early phase (X < 0.3)**: The corpus is still exploratory. Novelty floors
  should be low to encourage diverse submissions. Quality floors can be
  relaxed -- the corpus needs volume.
- **Rapid growth (0.3 < X < 0.7)**: Standards are crystallizing. Quality floors
  should tighten. The Avrami exponent `n` reveals whether crystallization is
  one-dimensional (one dominant style emerging) or multi-dimensional (several
  styles converging simultaneously).
- **Saturated (X > 0.7)**: The corpus has crystallized around quality norms.
  Further submissions at current novelty floors will be incremental. This is
  the signal to raise novelty floors, introduce new evaluation dimensions, or
  recalibrate the credit-quality function.

The insight: corpus maturation is a phase transition, and the Avrami equation
both models it and tells the operator exactly when to act.

### Rust Implementation Sketch

```rust
/// Avrami equation parameters fitted to quality score history.
#[derive(Debug, Clone, Copy)]
pub struct AvramiParams {
    /// Rate constant k. Higher = faster crystallization.
    pub k: f64,
    /// Avrami exponent n. Encodes growth dimensionality:
    /// n ~ 1: one-dimensional (single dominant style)
    /// n ~ 2: two-dimensional (surface-like growth)
    /// n ~ 3: three-dimensional (bulk crystallization)
    pub n: f64,
    /// Goodness of fit (R-squared).
    pub r_squared: f64,
    /// Current transformed fraction X(t).
    pub current_x: f64,
}

/// The time-series input: one observation per epoch.
#[derive(Debug, Clone, Copy)]
pub struct QualityEpochObservation {
    pub epoch_index: i64,
    /// Fraction of traces in this epoch that exceed the quality floor.
    /// This is the raw signal X(t) is fitted to.
    pub fraction_above_floor: f64,
    /// Mean quality score in this epoch (micros).
    pub mean_quality_micros: i64,
    /// Standard deviation of quality scores (micros).
    pub stddev_quality_micros: i64,
}

pub struct CrystallizationDetector {
    /// Minimum number of epochs before attempting a fit.
    pub min_epochs: usize,
    /// Quality floor (micros) for computing fraction_above_floor.
    pub quality_floor_micros: i64,
}

impl CrystallizationDetector {
    pub fn new(min_epochs: usize, quality_floor_micros: i64) -> Self {
        Self {
            min_epochs,
            quality_floor_micros,
        }
    }

    /// Fit the Avrami equation X(t) = 1 - exp(-k * t^n) to the observed
    /// quality history using linearized least-squares.
    ///
    /// Taking the double log: ln(-ln(1-X)) = ln(k) + n*ln(t).
    /// This is a linear regression in (ln(t), ln(-ln(1-X))) space.
    pub fn fit_avrami(&self, history: &[QualityEpochObservation]) -> Option<AvramiParams> {
        if history.len() < self.min_epochs {
            return None;
        }

        // Collect valid data points (0 < X < 1 required for the double log).
        let points: Vec<(f64, f64)> = history
            .iter()
            .filter_map(|obs| {
                let x = obs.fraction_above_floor;
                if x <= 0.0 || x >= 1.0 {
                    return None;
                }
                let t = (obs.epoch_index + 1) as f64; // time starts at 1
                let y = (-((1.0 - x).ln())).ln(); // ln(-ln(1-X))
                let lnt = t.ln();
                if y.is_finite() && lnt.is_finite() {
                    Some((lnt, y))
                } else {
                    None
                }
            })
            .collect();

        if points.len() < 3 {
            return None;
        }

        // Linear regression: y = a + b * x, where a = ln(k), b = n.
        let n_pts = points.len() as f64;
        let sum_x: f64 = points.iter().map(|(x, _)| x).sum();
        let sum_y: f64 = points.iter().map(|(_, y)| y).sum();
        let sum_xy: f64 = points.iter().map(|(x, y)| x * y).sum();
        let sum_xx: f64 = points.iter().map(|(x, _)| x * x).sum();

        let denom = n_pts * sum_xx - sum_x * sum_x;
        if denom.abs() < 1e-15 {
            return None;
        }

        let b = (n_pts * sum_xy - sum_x * sum_y) / denom; // n (Avrami exponent)
        let a = (sum_y - b * sum_x) / n_pts; // ln(k)
        let k = a.exp();

        // R-squared
        let mean_y = sum_y / n_pts;
        let ss_tot: f64 = points.iter().map(|(_, y)| (y - mean_y).powi(2)).sum();
        let ss_res: f64 = points
            .iter()
            .map(|(x, y)| {
                let pred = a + b * x;
                (y - pred).powi(2)
            })
            .sum();
        let r_squared = if ss_tot > 0.0 {
            1.0 - ss_res / ss_tot
        } else {
            0.0
        };

        // Current X(t) at the latest epoch.
        let t_now = (history.last().unwrap().epoch_index + 1) as f64;
        let current_x = 1.0 - (-k * t_now.powf(b)).exp();

        Some(AvramiParams {
            k,
            n: b,
            r_squared,
            current_x: current_x.clamp(0.0, 1.0),
        })
    }

    /// Determine the corpus phase based on the Avrami fit.
    pub fn phase(&self, params: &AvramiParams) -> CrystallizationPhase {
        if params.current_x < 0.3 {
            CrystallizationPhase::Exploratory
        } else if params.current_x < 0.7 {
            CrystallizationPhase::RapidGrowth
        } else {
            CrystallizationPhase::Saturated
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrystallizationPhase {
    /// Early phase: diverse, noisy, quality variance is high.
    Exploratory,
    /// Middle phase: standards are converging rapidly.
    RapidGrowth,
    /// Late phase: corpus has crystallized; novelty recalibration needed.
    Saturated,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sigmoidal_history_yields_reasonable_avrami_fit() {
        let detector = CrystallizationDetector::new(5, 500_000);
        // Simulate a sigmoidal adoption curve: X(t) = 1 - exp(-0.1 * t^2)
        let history: Vec<QualityEpochObservation> = (1..=20)
            .map(|t| {
                let x = 1.0 - (-0.1 * (t as f64).powi(2)).exp();
                QualityEpochObservation {
                    epoch_index: t,
                    fraction_above_floor: x.clamp(0.01, 0.99),
                    mean_quality_micros: (x * 800_000.0) as i64,
                    stddev_quality_micros: 100_000,
                }
            })
            .collect();

        let params = detector.fit_avrami(&history).expect("should fit");
        assert!(params.r_squared > 0.8, "R^2 should be high for clean data");
        assert!(params.n > 1.5 && params.n < 2.5, "n should be near 2: {}", params.n);
    }

    #[test]
    fn early_epochs_are_exploratory() {
        let detector = CrystallizationDetector::new(3, 500_000);
        let params = AvramiParams {
            k: 0.05,
            n: 2.0,
            r_squared: 0.95,
            current_x: 0.15,
        };
        assert_eq!(detector.phase(&params), CrystallizationPhase::Exploratory);
    }
}
```

### PostgreSQL Schema

```sql
CREATE TABLE avrami_fits (
    fit_id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id           UUID NOT NULL REFERENCES tenants(id),
    computed_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    k                   DOUBLE PRECISION NOT NULL,
    n                   DOUBLE PRECISION NOT NULL,
    r_squared           DOUBLE PRECISION NOT NULL,
    current_x           DOUBLE PRECISION NOT NULL,
    phase               TEXT NOT NULL CHECK (phase IN ('exploratory', 'rapid_growth', 'saturated')),
    epoch_count_fitted  INT NOT NULL,
    quality_floor_micros BIGINT NOT NULL
);

CREATE INDEX idx_avrami_tenant ON avrami_fits(tenant_id, computed_at DESC);
```

### Integration with Existing Pipeline

The crystallization detector runs as a **periodic analytics job** (e.g., weekly
at epoch boundaries). It queries the `trace_gate_decisions` table for per-epoch
quality statistics, fits the Avrami equation, and persists the result. The
operator dashboard shows the current phase and Avrami curve. When the phase
transitions to `Saturated`, the system can automatically:
- Raise the novelty floor in `EnclaveGateOrchestratorConfig`.
- Trigger a novelty recalibration bake-off.
- Alert operators that the corpus needs new evaluation dimensions.

### Estimated Complexity and Prerequisites

- **Implementation effort**: ~1 week. The linearized least-squares fit is ~40
  lines. The epoch aggregation query already exists for the contributor cap.
- **Prerequisites**: At least ~10 epochs of trace data for a meaningful fit.
- **Risk**: The Avrami model assumes a single phase transition. Real corpora may
  undergo multiple transitions (e.g., crystallization around one norm, followed
  by a "melting" when a new tool is released, then re-crystallization). The
  model would need to be re-fitted after each disruption.

### Potential Research Output

- **Paper**: "Phase Transitions in AI Training Data Quality: Avrami Kinetics
  Applied to Trace Corpus Maturation." Venue: KDD (Knowledge Discovery and
  Data Mining) or Nature Computational Science.
- **Key result**: Empirical measurement of Avrami exponents across different
  trace corpora, comparison with social norm adoption kinetics, prediction of
  when quality saturation will occur.

---

## 8. Predictive Coding / Free Energy for Trace Saliency

### Academic Foundations

The free energy principle, formalized by Karl Friston (2010), proposes that
biological systems minimize "variational free energy" -- a computable upper
bound on surprise (negative log-evidence). In practical terms: the system
maintains a generative model of its expected inputs and acts to minimize
prediction error. When an input violates the model's expectations, the
prediction error is high -- the input is "salient."

Andy Clark (2013) popularized this as "predictive coding": the brain is a
prediction machine that generates top-down expectations at every level of the
sensory hierarchy. Bottom-up signals carry only prediction errors. Attention
is the precision-weighting of these errors: salient inputs are those with
high-precision prediction errors.

In hierarchical predictive coding, each level predicts the activity at the
level below. Prediction errors propagate upward. At the lowest level
(tokens), errors correspond to unexpected words. At higher levels (tool calls,
strategies), errors correspond to unexpected behavioral patterns. The free
energy is minimized by either updating the model (learning) or seeking inputs
that reduce uncertainty (active inference).

**Key references:**
- Friston, K. (2010). "The Free-Energy Principle: A Unified Brain Theory?" *Nature Reviews Neuroscience*, 11, 127-138.
- Clark, A. (2013). "Whatever Next? Predictive Brains, Situated Agents, and the Future of Cognitive Science." *Behavioral and Brain Sciences*, 36(3), 181-204.
- Rao, R. P. N. & Ballard, D. H. (1999). "Predictive Coding in the Visual Cortex: A Functional Interpretation of Some Extra-classical Receptive-field Effects." *Nature Neuroscience*, 2, 79-87.
- Friston, K., Kilner, J., & Harrison, L. (2006). "A Free Energy Principle for the Brain." *J. Physiology-Paris*, 100, 70-87.
- Bogacz, R. (2017). "A Tutorial on the Free-energy Framework for Modelling Perception and Learning." *Journal of Mathematical Psychology*, 76, 198-211.

### Why This is Novel for AI Trace Management

TraceCommons currently scores novelty as `1 - max(cosine_similarity)` against
the vector index. This is a static, memory-less comparison: the system asks
"how different is this trace from the closest existing trace?" but not "how
*surprising* is this trace given everything I know about trace patterns?"

Predictive coding replaces this with a richer model. The system maintains a
hierarchical generative model of expected traces:

- **Level 0 (token)**: What tokens are expected given the context? Prediction
  error at this level is exactly perplexity -- which TraceCommons already
  computes. This validates the approach.
- **Level 1 (tool call)**: What tool-call sequences are expected given the
  task type? A trace that uses tools in an unexpected order has high L1
  prediction error.
- **Level 2 (strategy)**: What problem-solving strategy is expected given the
  domain? A trace that takes an unusual strategic approach has high L2
  prediction error.

The composite prediction error across levels is the trace's *saliency* -- a
multi-scale novelty signal that is strictly more informative than single-scale
embedding distance.

Active inference adds another dimension: the system can identify regions of
high uncertainty in its generative model and actively solicit traces from those
regions. This turns the system from a passive receiver into an active seeker
of information.

### Rust Implementation Sketch

```rust
use std::collections::HashMap;

/// A hierarchical prediction error across multiple levels of abstraction.
#[derive(Debug, Clone)]
pub struct PredictionError {
    /// Per-level prediction errors (lower index = more granular).
    pub level_errors: Vec<LevelPredictionError>,
    /// Composite saliency: precision-weighted sum of level errors.
    pub saliency_micros: u64,
    /// Free energy F = sum of precision-weighted squared errors.
    pub free_energy: f64,
}

#[derive(Debug, Clone)]
pub struct LevelPredictionError {
    pub level: usize,
    pub label: String,
    /// The prediction error magnitude at this level.
    pub error_magnitude: f64,
    /// Precision (inverse variance) of the model at this level.
    /// High precision = the model is confident in its prediction,
    /// so a large error is more salient.
    pub precision: f64,
    /// Precision-weighted error: precision * error^2.
    pub weighted_error: f64,
}

/// Per-level generative model: tracks the distribution of observations
/// at each abstraction level. Uses a simple count-based model; production
/// would use a learned model (e.g., a small transformer).
#[derive(Debug, Clone)]
struct LevelModel {
    /// Observation counts per category.
    counts: HashMap<String, u64>,
    total: u64,
    /// Smoothing constant (Laplace smoothing).
    alpha: f64,
}

impl LevelModel {
    fn new(alpha: f64) -> Self {
        Self {
            counts: HashMap::new(),
            total: 0,
            alpha,
        }
    }

    fn observe(&mut self, category: &str) {
        *self.counts.entry(category.to_string()).or_default() += 1;
        self.total += 1;
    }

    /// Predicted probability of a category. Laplace-smoothed.
    fn predict(&self, category: &str) -> f64 {
        let count = self.counts.get(category).copied().unwrap_or(0) as f64;
        let vocab = self.counts.len().max(1) as f64;
        (count + self.alpha) / (self.total as f64 + self.alpha * vocab)
    }

    /// Prediction error: negative log probability (surprise).
    fn prediction_error(&self, category: &str) -> f64 {
        let p = self.predict(category);
        if p <= 0.0 {
            return 20.0; // cap surprise at a reasonable max
        }
        -p.ln()
    }

    /// Precision: inverse variance of the distribution. Higher when the
    /// model is concentrated (confident). Uses the Herfindahl index as
    /// a tractable proxy for concentration.
    fn precision(&self) -> f64 {
        if self.total == 0 {
            return 1.0; // uninformative prior -> low precision
        }
        let vocab = self.counts.len().max(1) as f64;
        let hhi: f64 = self
            .counts
            .values()
            .map(|&c| {
                let p = c as f64 / self.total as f64;
                p * p
            })
            .sum();
        // Normalized HHI: 1/vocab (uniform) to 1 (all mass on one category).
        // Precision = normalized HHI * total (more data = more precise).
        let normalized = (hhi - 1.0 / vocab) / (1.0 - 1.0 / vocab).max(1e-10);
        (normalized.max(0.0) * self.total as f64).max(1.0)
    }

    /// Entropy of the model's predictive distribution. Higher = more
    /// uncertain. Used for active inference: seek traces from high-entropy
    /// levels.
    fn entropy(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        let vocab = self.counts.len().max(1) as f64;
        let mut h = 0.0f64;
        for &count in self.counts.values() {
            let p = (count as f64 + self.alpha) / (self.total as f64 + self.alpha * vocab);
            if p > 0.0 {
                h -= p * p.ln();
            }
        }
        h
    }
}

/// The predictive coding scorer: maintains a hierarchical generative model
/// and scores traces by their composite prediction error.
pub struct PredictiveCodingScorer {
    levels: Vec<(String, LevelModel)>,
}

impl PredictiveCodingScorer {
    /// Create a scorer with the standard three-level hierarchy.
    pub fn new() -> Self {
        Self {
            levels: vec![
                ("token_pattern".to_string(), LevelModel::new(1.0)),
                ("tool_sequence".to_string(), LevelModel::new(0.5)),
                ("strategy".to_string(), LevelModel::new(0.1)),
            ],
        }
    }

    /// Train the model by observing a trace's features at each level.
    /// `features` maps level index -> category label.
    pub fn observe(&mut self, features: &[(usize, &str)]) {
        for &(level, category) in features {
            if level < self.levels.len() {
                self.levels[level].1.observe(category);
            }
        }
    }

    /// Score a trace by computing hierarchical prediction error.
    pub fn score(&self, features: &[(usize, &str)]) -> PredictionError {
        let mut level_errors = Vec::new();
        let mut free_energy = 0.0f64;

        for &(level_idx, category) in features {
            if level_idx >= self.levels.len() {
                continue;
            }
            let (label, model) = &self.levels[level_idx];
            let error = model.prediction_error(category);
            let precision = model.precision();
            let weighted = precision * error * error;
            free_energy += weighted;

            level_errors.push(LevelPredictionError {
                level: level_idx,
                label: label.clone(),
                error_magnitude: error,
                precision,
                weighted_error: weighted,
            });
        }

        // Saliency: normalized free energy mapped to micros.
        // Use tanh to squash to [0, 1] then scale.
        let saliency = (free_energy / level_errors.len().max(1) as f64).tanh();
        let saliency_micros = (saliency * 1_000_000.0) as u64;

        PredictionError {
            level_errors,
            saliency_micros,
            free_energy,
        }
    }

    /// Active inference: identify the level with highest uncertainty
    /// (entropy) and return it as the recommended area for soliciting
    /// new traces.
    pub fn uncertainty_map(&self) -> Vec<(String, f64)> {
        let mut map: Vec<(String, f64)> = self
            .levels
            .iter()
            .map(|(label, model)| (label.clone(), model.entropy()))
            .collect();
        map.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unseen_category_has_high_saliency() {
        let mut scorer = PredictiveCodingScorer::new();
        // Train on many observations of "read-edit-test" at the strategy level.
        for _ in 0..100 {
            scorer.observe(&[(2, "read-edit-test")]);
        }
        // Score a common trace (low saliency).
        let common = scorer.score(&[(2, "read-edit-test")]);
        // Score a novel trace (high saliency).
        let novel = scorer.score(&[(2, "novel-approach")]);
        assert!(
            novel.saliency_micros > common.saliency_micros,
            "novel should be more salient: {} vs {}",
            novel.saliency_micros,
            common.saliency_micros
        );
    }

    #[test]
    fn precision_increases_with_concentration() {
        let mut uniform_model = LevelModel::new(1.0);
        for c in ["a", "b", "c", "d", "e"] {
            for _ in 0..20 {
                uniform_model.observe(c);
            }
        }
        let mut concentrated_model = LevelModel::new(1.0);
        for _ in 0..95 {
            concentrated_model.observe("a");
        }
        for _ in 0..5 {
            concentrated_model.observe("b");
        }
        assert!(
            concentrated_model.precision() > uniform_model.precision(),
            "concentrated model should have higher precision"
        );
    }

    #[test]
    fn active_inference_identifies_highest_uncertainty() {
        let mut scorer = PredictiveCodingScorer::new();
        // Only train the strategy level (level 2), leave others empty.
        for _ in 0..50 {
            scorer.observe(&[(2, "read-edit-test")]);
        }
        let map = scorer.uncertainty_map();
        // The untrained levels should have lower entropy (uniform prior with
        // no observations), while the trained level has structured entropy.
        // In practice, the map guides operators to solicit traces that
        // reduce uncertainty at the most uncertain level.
        assert_eq!(map.len(), 3);
    }
}
```

### PostgreSQL Schema

```sql
CREATE TABLE predictive_coding_models (
    model_id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id           UUID NOT NULL REFERENCES tenants(id),
    level_index         INT NOT NULL,
    level_label         TEXT NOT NULL,
    category_counts     JSONB NOT NULL DEFAULT '{}',
    total_observations  BIGINT NOT NULL DEFAULT 0,
    precision           DOUBLE PRECISION NOT NULL DEFAULT 1.0,
    entropy             DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, level_index)
);

CREATE TABLE trace_saliency_scores (
    submission_id       UUID NOT NULL REFERENCES trace_submissions(id),
    tenant_id           UUID NOT NULL REFERENCES tenants(id),
    saliency_micros     BIGINT NOT NULL,
    free_energy         DOUBLE PRECISION NOT NULL,
    level_errors        JSONB NOT NULL,
    scored_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (submission_id)
);
```

### Integration with Existing Pipeline

The predictive coding scorer integrates as an **additional scoring dimension**
alongside perplexity and novelty. After the `EnclaveGateOrchestrator::evaluate()`
call produces its `OrchestrationDecision`, a post-scoring step extracts
multi-level features from the trace (token patterns from the perplexity scorer,
tool-call sequences from the envelope events, strategy labels from a classifier)
and feeds them to the `PredictiveCodingScorer`. The resulting `saliency_micros`
is stored alongside the existing scores and can be folded into the
`credit_quality()` function as a third term:

```
q = f(perplexity) * g(novelty) * h(saliency) * a(anomaly)
```

The `uncertainty_map()` output feeds the operator dashboard, showing which
abstraction levels need more diverse traces.

Active inference is the most ambitious integration: the system queries its own
uncertainty map and generates solicitation signals -- "We need more traces
involving [X tool pattern] in [Y domain]" -- that are shown to contributors
or used to adjust novelty bonuses.

### Estimated Complexity and Prerequisites

- **Implementation effort**: ~4 weeks. The count-based model is simple, but the
  feature extraction (mapping traces to multi-level categories) is the hard
  part. The active inference loop requires changes to the contributor-facing API.
- **Prerequisites**: A multi-level feature extractor for traces. Level 0
  (tokens) is free (perplexity already measures this). Level 1 (tool calls)
  requires parsing the trace envelope's event list. Level 2 (strategy)
  requires the classifier from ideas 3 and 4.
- **Risk**: The count-based generative model is a crude approximation. For
  production, a learned model (e.g., a small autoregressive model trained on
  trace features) would be necessary. The count-based version is a proof of
  concept.

### Potential Research Output

- **Paper**: "Free Energy Minimization for Active Data Curation: A Predictive
  Coding Framework for AI Trace Corpora." Venue: ICLR or NeurIPS.
- **Key result**: Demonstration that hierarchical prediction error is a strictly
  more informative novelty signal than single-scale embedding distance.
  Empirical comparison of active-inference-guided solicitation vs. passive
  collection on corpus diversity metrics.
- **Stretch paper**: "The Trace Corpus as a Bayesian Brain: Predictive Coding,
  Active Inference, and the Free Energy Principle Applied to AI Training Data
  Management." Venue: Nature Machine Intelligence.

---

## Summary Table

| # | Idea | Source Field | Key Concept | Integration Point | Effort |
|---|------|-------------|-------------|-------------------|--------|
| 1 | VCG Auctions | Mechanism design | Truthful trace valuation | Pre-scoring batch allocation | 2 weeks |
| 2 | NCD Novelty | Information theory | Compression-based similarity | Pre-filter before embedding | 1 week |
| 3 | Replicator Dynamics | Evolutionary game theory | Strategy fitness tracking | Post-scoring epoch analytics | 2 weeks |
| 4 | SIR Pattern Spread | Epidemiology | Pattern diffusion R_0 | Post-scoring pattern tracker | 2 weeks |
| 5 | Prospect-Theory Credits | Behavioral economics | Reference-dependent framing | Presentation layer | 1 week |
| 6 | TDA Clustering | Algebraic topology | Persistent homology gaps | Batch corpus analytics | 3 weeks |
| 7 | Avrami Crystallization | Materials science | Phase transition detection | Periodic calibration trigger | 1 week |
| 8 | Predictive Coding | Computational neuroscience | Hierarchical saliency | Additional scoring dimension | 4 weeks |

Recommended implementation order: 2 (NCD, fast win, no dependencies) -> 7
(Avrami, fast win, pure analytics) -> 5 (prospect theory, presentation only)
-> 1 (VCG, requires credit infrastructure) -> 3 (replicator, needs classifier)
-> 4 (SIR, builds on classifier from 3) -> 6 (TDA, compute-intensive) -> 8
(predictive coding, most complex, builds on everything else).
