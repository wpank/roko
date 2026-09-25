# Score: 7-Axis Appraisal

> **v3 depth file** -- `/docs/v3/depth/00-architecture/score-7-axis-appraisal.md`
> Canonical source: v1 `docs/v1/00-architecture/03-score-7-axis-appraisal.md`
> Implementation: `crates/roko-core/src/score.rs`
> Status: **Shipping** -- 7-axis Score struct with 6-factor effective formula, all arithmetic, constants, and extended axes are implemented and tested

---

## 1. Design Rationale

Agent systems produce and consume enormous quantities of information: task descriptions,
LLM outputs, gate verdicts, knowledge entries, tool traces. Not all information is equally
valuable. A scoring system must answer the question: "How much should I trust and attend to
this Signal?"

Simple scalar scoring (a single 0-1 confidence) loses information. A highly confident but
stale piece of knowledge should score differently from a novel but uncertain observation.
A response from a trusted model should score differently from one originating in untrusted
external data.

Roko's Score uses **orthogonal axes** that capture different quality dimensions. Every
scoring mechanism in the design corpus -- confidence scores, novelty detectors, utility
accumulators, reputation trackers, fitness functions, prediction weights, catalytic scores --
collapses into one of these axes. The multi-dimensional representation preserves information
while the effective score formula collapses them into a single scalar when a total ordering
is needed.

---

## 2. The Four Primary Axes

These four axes are the core of the Score struct (`roko-core/src/score.rs`):

```rust
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Score {
    /// [0..1] -- how confident are we this Signal is correct/valid?
    pub confidence: f32,
    /// [0..1] -- how novel is this Signal compared to prior Signals?
    pub novelty: f32,
    /// [0..inf) -- how useful has this Signal proven historically?
    pub utility: f32,
    /// [0..inf) -- reputation of the Signal's author at emission time.
    pub reputation: f32,
    /// [0..1] -- how exact or narrowly applicable? (extended)
    #[serde(default)]
    pub precision: f32,
    /// [0..1] -- how much extra ranking weight? (extended)
    #[serde(default)]
    pub salience: f32,
    /// [0..1] -- how internally consistent is the evidence? (extended)
    #[serde(default)]
    pub coherence: f32,
}
```

### 2.1 Confidence -- [0, 1]

**What it measures**: How sure are we that this Signal is correct, valid, or truthful?

**Range**: [0, 1]. Clamped at construction via `finite_unit_interval()`.

**Examples**:
- A Gate verdict with `passed = true` --> confidence near 1.0
- An LLM output that has not been verified --> confidence 0.5 (neutral)
- A prediction that has been partially falsified --> confidence drops toward 0.0

**Critical property**: Zero confidence produces zero effective score regardless of other
axes. This ensures that information known to be incorrect is never prioritized. The formula
`effective = confidence x ...` enforces this structurally.

**Where it comes from**: Gate verdicts, prediction tracking (CalibrationTracker), human
ratings, source verification.

### 2.2 Novelty -- [0, 1]

**What it measures**: How new or surprising is this Signal compared to what the system
already knows?

**Range**: [0, 1]. Clamped at construction.

**Examples**:
- A completely new insight not present in any existing knowledge --> novelty near 1.0
- A routine heartbeat tick --> novelty near 0.0
- A piece of information that updates an existing knowledge entry --> novelty ~0.5

**Role in scoring**: Novelty acts as a multiplicative bonus via `(1 + novelty)`. A Signal
with novelty 0.0 has an effective score multiplier of 1.0 from this axis; a Signal with
novelty 1.0 has a multiplier of 2.0. This ensures that novel information is prioritized
without penalizing routine information.

**Connection to Active Inference**: Novelty maps to the epistemic value component of Expected
Free Energy (Friston 2010). High-novelty Signals carry high expected information gain, making
them priority targets for attention allocation.

### 2.3 Utility -- [0, inf)

**What it measures**: How pragmatically useful has this Signal proven to be? Utility
accumulates over time as the Signal is referenced, used in compositions, or leads to
successful outcomes.

**Range**: [0, inf). Unbounded above; clamped to non-negative at construction via
`finite_non_negative()`.

**Examples**:
- A playbook rule applied 50 times with positive outcomes --> high utility
- A fresh Signal that has never been used --> utility 0.0
- A knowledge entry referenced in 10 successful task completions --> utility growing

**Role in scoring**: Like novelty, utility acts as a multiplicative bonus via `(1 + utility)`.
A Signal with utility 0.0 has a multiplier of 1.0; with utility 5.0, it has a multiplier
of 6.0. Utility accumulates, giving frequently-useful Signals exponentially increasing
priority.

**Connection to Active Inference**: Utility maps to the pragmatic value component of Expected
Free Energy. High-utility Signals have demonstrated pragmatic value through outcomes.

### 2.4 Reputation -- [0, inf)

**What it measures**: How trustworthy is the Signal's producer at the time the Signal was
created?

**Range**: [0, inf). Unbounded above; clamped to non-negative at construction.

**Examples**:
- A Signal from a Gate (ground truth) --> reputation 1.0
- A Signal from an internal agent --> reputation 0.75
- A Signal from an untrusted external source --> reputation 0.1
- A Signal from a model with accumulated positive track record --> reputation above 1.0

**Role in scoring**: Reputation directly scales the effective score. A Signal with
reputation 0.0 has zero effective score regardless of other axes -- untrusted sources are
structurally excluded.

**Connection to Provenance**: Reputation is initialized from the Signal's Provenance record
but can be updated as the author's track record evolves.

---

## 3. The Three Extended Axes

Three additional axes complete the full 7-axis appraisal. These are present in the Score
struct with `#[serde(default)]` for backward compatibility.

### 3.1 Precision -- [0, 1]

**What it measures**: How specific and well-defined is this Signal's content? Precision
captures the difference between a vague statement ("something is probably wrong") and a
specific one ("compilation fails at line 42 with error E0599").

**Role**: Used for weighting predictions and knowledge entries. High-precision Signals are
more actionable and receive higher weight in composition decisions. Precision is deliberately
**excluded** from the effective score formula -- it describes applicability narrowness, not
quality, and is consumed separately by routers that need specificity ranking.

**Connection to Active Inference**: Precision weighting is central to active inference
(Friston 2010) -- prediction errors are weighted by their precision to determine how much
they should update the model.

### 3.2 Salience -- [0, 1]

**What it measures**: How relevant is this Signal to the current context? Salience is
context-dependent -- the same Signal may be highly salient in one context and irrelevant in
another.

**Role**: Used by the VCG Attention Auction for truthful context budget allocation.
High-salience Signals bid higher for inclusion in the context window. When non-zero, salience
enters the effective formula as a soft-damping factor: `0.5 + 0.5 * salience`.

### 3.3 Coherence -- [0, 1]

**What it measures**: How consistent is this Signal with the system's existing knowledge base?
A Signal that contradicts well-established knowledge has low coherence; one that fits
seamlessly has high coherence.

**Role**: Used for knowledge integration decisions. Low-coherence Signals may signal either
an error (contradiction with ground truth) or a genuine surprise (new information that updates
the model). When non-zero, coherence enters the effective formula as a soft-damping factor:
`0.5 + 0.5 * coherence`.

---

## 4. The Effective Score Formula

All seven axes collapse into a single scalar via the 6-factor formula:

```
effective = confidence
          x (1 + novelty)
          x (1 + utility)
          x reputation
          x salience_factor
          x coherence_factor
```

where:
```
salience_factor  = 1.0        when salience == 0 (opt-in)
                 = 0.5 + 0.5 * salience   otherwise

coherence_factor = 1.0        when coherence == 0 (opt-in)
                 = 0.5 + 0.5 * coherence   otherwise
```

The shipped implementation (`roko-core/src/score.rs`):

```rust
pub fn effective(&self) -> f32 {
    if !self.is_finite() {
        return 0.0;
    }
    let salience_factor = if self.salience == 0.0 {
        1.0
    } else {
        0.5 + 0.5 * self.salience
    };
    let coherence_factor = if self.coherence == 0.0 {
        1.0
    } else {
        0.5 + 0.5 * self.coherence
    };
    finite_non_negative(
        self.confidence
            * (1.0 + self.novelty)
            * (1.0 + self.utility)
            * self.reputation
            * salience_factor
            * coherence_factor,
    )
}
```

### 4.1 Formula Properties

| Property | Guarantee | Why It Matters |
|---|---|---|
| `confidence = 0 --> effective = 0` | Zero confidence kills the score | Invalid information is never prioritized |
| `reputation = 0 --> effective = 0` | Zero reputation kills the score | Untrusted sources are structurally excluded |
| `novelty = 0 --> multiplier = 1.0` | No penalty for routine information | Routine is normal, not bad |
| `novelty = 1 --> multiplier = 2.0` | Novel information gets 2x priority | Surprise drives attention |
| `utility = 0 --> multiplier = 1.0` | New Signals start at baseline | No penalty for lack of history |
| `utility = n --> multiplier = (1+n)` | Utility accumulates multiplicatively | Frequently-useful Signals dominate |
| `salience = 0 --> factor = 1.0` | Opt-in: no effect when unused | Backward-compatible |
| `salience = 1 --> factor = 1.0` | Full salience is neutral | Only low salience damps |
| `coherence = 0 --> factor = 1.0` | Opt-in: no effect when unused | Backward-compatible |
| Non-finite --> effective = 0 | NaN/Inf protection | Defensive numeric hygiene |
| Precision excluded | Does not affect scalar | Precision is specificity, not quality |

### 4.2 Example Calculations

```
// A fresh, neutral Signal (builder defaults)
Score::NEUTRAL  // confidence=0.5, novelty=0, utility=0, reputation=1
--> 0.5 x 1.0 x 1.0 x 1.0 x 1.0 x 1.0 = 0.5

// A verified, novel insight from a trusted source
Score { confidence: 0.95, novelty: 0.8, utility: 0, reputation: 1.2,
        precision: 0, salience: 0, coherence: 0 }
--> 0.95 x 1.8 x 1.0 x 1.2 x 1.0 x 1.0 = 2.052

// A highly-utilized playbook rule
Score { confidence: 0.9, novelty: 0, utility: 5.0, reputation: 1.0, ... }
--> 0.9 x 1.0 x 6.0 x 1.0 x 1.0 x 1.0 = 5.4

// An untrusted external observation
Score { confidence: 0.8, novelty: 1.0, utility: 0, reputation: 0.1, ... }
--> 0.8 x 2.0 x 1.0 x 0.1 x 1.0 x 1.0 = 0.16

// With salience and coherence active
Score { confidence: 0.5, novelty: 1.0, utility: 1.0, reputation: 2.0,
        precision: 0, salience: 1.0, coherence: 1.0 }
--> 0.5 x 2.0 x 2.0 x 2.0 x 1.0 x 1.0 = 4.0
```

---

## 5. Score Constants

```rust
impl Score {
    /// A zero score (all axes = 0). Equivalent to "no evidence".
    pub const ZERO: Self = Self {
        confidence: 0.0, novelty: 0.0, utility: 0.0, reputation: 0.0,
        precision: 0.0, salience: 0.0, coherence: 0.0,
    };

    /// A neutral score. Default for unscored Signals.
    pub const NEUTRAL: Self = Self {
        confidence: 0.5, novelty: 0.0, utility: 0.0, reputation: 1.0,
        precision: 0.0, salience: 0.0, coherence: 0.0,
    };
}
```

`Score::NEUTRAL` is the default. It represents "we have no information about this Signal's
quality" -- moderate confidence, no novelty signal, no utility history, trusted author. The
effective value is 0.5.

`Score::ZERO` represents "no evidence" -- zero across all axes. Effective value is 0.0.

---

## 6. Score Arithmetic

Scores support element-wise arithmetic for composition across all 7 axes.

### 6.1 Element-Wise Multiplication (Scaling)

```rust
impl Mul for Score {
    type Output = Self;
    fn mul(self, other: Self) -> Self {
        Self {
            confidence: finite_unit_interval(self.confidence * other.confidence),
            novelty: finite_unit_interval(self.novelty * other.novelty),
            utility: finite_non_negative(self.utility * other.utility),
            reputation: finite_non_negative(self.reputation * other.reputation),
            precision: finite_unit_interval(self.precision * other.precision),
            salience: finite_unit_interval(self.salience * other.salience),
            coherence: finite_unit_interval(self.coherence * other.coherence),
        }
    }
}
```

Used when applying a per-axis modifier to a base score.

### 6.2 Element-Wise Addition (Aggregation)

```rust
impl Add for Score {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self {
            confidence: finite_unit_interval(self.confidence + other.confidence),
            novelty: finite_unit_interval(self.novelty + other.novelty),
            utility: finite_non_negative(self.utility + other.utility),
            reputation: finite_non_negative(self.reputation + other.reputation),
            precision: finite_unit_interval(self.precision + other.precision),
            salience: finite_unit_interval(self.salience + other.salience),
            coherence: finite_unit_interval(self.coherence + other.coherence),
        }
    }
}
```

Used when aggregating evidence from multiple Scorers. Bounded axes (confidence, novelty,
precision, salience, coherence) are clamped to 1.0. Unbounded axes (utility, reputation)
accumulate without limit.

---

## 7. Score x Decay = Weight

A Signal's effective weight at a given time combines Score and Decay (see
`decay-variants-and-tier-matrix.md`):

```
weight(t) = score.effective() x decay.apply(age_ms)
```

This is the primary ordering criterion for Store queries. The `weight_at()` method computes:

```rust
pub fn weight_at(&self, now_ms: i64) -> f32 {
    let age = now_ms - self.created_at_ms;
    self.score.effective() * self.decay.apply(age)
}
```

A highly-scored Signal with aggressive decay will eventually fall below the weight threshold
and be excluded from queries or pruned from the Store. This is how the system implements
"forgetting" -- not by deleting information, but by letting its weight decay below the
threshold of relevance.

---

## 8. How Scorers Produce Scores

Scorers implement the `Score` trait (`roko-core/src/traits.rs`):

```rust
pub trait Score: Cell + Send + Sync {
    fn score(&self, signal: &Signal, ctx: &Context) -> ScoreValue;
    fn score_pulse(&self, p: &Pulse, ctx: &Context) -> ScoreValue;
    fn score_datum(&self, datum: Datum<'_>, ctx: &Context) -> ScoreValue;
    fn name(&self) -> &'static str;
}
```

Multiple Scorers compose via `CompositeScorer`. A typical scoring pipeline:

1. **RelevanceScorer**: Scores how well the Signal matches the current goal. Sets confidence.
2. **RecencyScorer**: Scores how recent the Signal is. Reduces confidence for stale data.
3. **ReputationScorer**: Scores based on the author's track record. Sets reputation.
4. **CatalyticScorer**: Scores based on downstream Signals this one has catalyzed. Sets utility.

---

## 9. Multi-Criteria Decision Analysis

The `effective()` formula produces a single scalar for total ordering. Three classical MCDA
methods offer alternatives for specialized contexts.

### 9.1 TOPSIS (Hwang & Yoon 1981)

Technique for Order of Preference by Similarity to Ideal Solution:

```
C_i* = S_i^- / (S_i^+ + S_i^-)     in [0, 1]

where:
  S_i^+ = sqrt( sum_j (v_ij - v_j^+)^2 )  // distance to ideal
  S_i^- = sqrt( sum_j (v_ij - v_j^-)^2 )  // distance to anti-ideal
  v_ij  = w_j x r_ij                       // weighted normalized score
```

Fully compensatory. Suitable for Signal ranking when all axes are commensurable.

### 9.2 ELECTRE III (Roy 1968; Figueira et al. 2005)

Outranking approach with three thresholds per axis:

| Threshold | Symbol | Meaning |
|---|---|---|
| Indifference | q_j | Differences below q_j are ignored |
| Preference | p_j | Differences above p_j establish strict preference |
| **Veto** | v_j | Differences above v_j block outranking entirely |

The veto threshold makes ELECTRE **non-compensatory**: zero confidence vetoes the Signal
regardless of other axes. Structurally equivalent to Roko's `confidence = 0 --> effective = 0`.

### 9.3 PROMETHEE (Brans & Vincke 1985)

Pairwise preference functions:

```
pi(a, b) = sum_j w_j x P_j(a, b)           // multicriteria preference index
phi^+(a) = (1/(n-1)) sum_{x!=a} pi(a, x)   // positive flow (dominance)
phi^-(a) = (1/(n-1)) sum_{x!=a} pi(x, a)   // negative flow (dominated-ness)
phi(a)   = phi^+(a) - phi^-(a)              // net flow --> ranking
```

PROMETHEE I preserves **incomparability** -- two Signals strong on different axes remain
unranked rather than forced into a total order.

### 9.4 When to Use Each

| Situation | Method | Rationale |
|---|---|---|
| Standard Signal ranking | `effective()` formula | Simple, fast, well-understood |
| Context budget allocation | TOPSIS | Normalized [0,1] scores for bidding |
| Safety-critical verification | ELECTRE III | Veto thresholds prevent unsafe Signals |
| Exploratory routing | PROMETHEE I | Preserves incomparability for diverse selection |

---

## 10. Optimal Dimensionality: Is 7 the Right Number?

### 10.1 Miller (1956): The Magical Number Seven

George Miller demonstrated that humans can reliably distinguish approximately 7 +/- 2 levels
on a single stimulus dimension. When information is distributed across multiple dimensions,
total capacity increases but per-axis resolution decreases.

### 10.2 Factor Analysis (Thurstone 1947)

Thurstone's Multiple Factor Analysis identified approximately 7 primary mental ability factors
through empirical factor extraction. Across psychological datasets, factor analysis typically
finds 3-8 meaningful factors before dimensionality becomes redundant.

### 10.3 Appraisal Theory Consensus

Across major appraisal theorists (Scherer 2001, Lazarus 1991, Smith & Ellsworth 1985), the
consensus is **5 core dimensions** all theories agree on:

1. Goal/need relevance --> maps to **utility**
2. Goal congruence --> maps to **confidence** (was the outcome favorable?)
3. Causal agency --> maps to **reputation** (who caused this?)
4. Coping potential --> maps to **salience** (can we act on this?)
5. Normative significance --> maps to **coherence** (does it fit standards?)

Plus 2 additional dimensions that are theory-specific:

6. Novelty/unexpectedness --> maps to **novelty**
7. Certainty/predictability --> maps to **precision**

This gives 7 appraisal dimensions -- converging independently with Miller's cognitive limit,
Thurstone's factor analyses, and Roko's 7-axis design.

**Validation criterion**: The 7 axes should be verified as genuinely independent via
inter-correlation analysis on production score data. If any pair of axes correlates above
r > 0.8, they should be merged.

---

## 11. Score Calibration

### 11.1 The Calibration Problem

Scores from different Scorers, domains, and time periods may not be directly comparable.
A confidence of 0.8 from a `CompileGate` (deterministic, well-calibrated) is not the same
as 0.8 from an `LlmJudgeGate` (probabilistic, potentially overconfident).

### 11.2 Temperature Scaling (Guo et al. 2017)

```
calibrated_score = score / T
```

T > 1 reduces overconfidence; T < 1 sharpens. T is tuned on a held-out set by minimizing
Expected Calibration Error (ECE).

### 11.3 Conformal Prediction (Vovk et al. 2005)

Provides finite-sample coverage guarantees: `P(Y in C(X)) >= 1 - alpha` regardless of the
score distribution. Bounded confidence intervals on any score axis without distributional
assumptions.

### 11.4 Domain-Specific Bias Correction

```
observed_score_ij = true_quality_i + domain_bias_j + noise_ij
domain_bias_j ~ Normal(0, sigma^2_domain)
```

This hierarchical model (Gelman et al. 2013) shrinks domain biases toward zero, enabling
meaningful cross-domain comparison.

---

## 12. Bayesian Score Updating

### 12.1 Beta-Binomial for Confidence

Gate verdicts provide binary pass/fail evidence. The Beta-Binomial conjugate model updates
confidence optimally:

```rust
pub struct BayesianConfidenceUpdater {
    alpha: f64,  // prior pseudo-passes (default 2.0)
    beta: f64,   // prior pseudo-fails (default 2.0)
}

impl BayesianConfidenceUpdater {
    pub fn update(&mut self, passed: bool) {
        if passed { self.alpha += 1.0; } else { self.beta += 1.0; }
    }

    /// Posterior mean = calibrated confidence estimate.
    pub fn confidence(&self) -> f32 {
        (self.alpha / (self.alpha + self.beta)) as f32
    }

    /// Posterior variance = remaining uncertainty about quality.
    pub fn uncertainty(&self) -> f32 {
        let n = self.alpha + self.beta;
        ((self.alpha * self.beta) / (n * n * (n + 1.0))) as f32
    }
}
```

### 12.2 Multi-Axis Evidence Sources

| Axis | Evidence Source | Update Trigger |
|---|---|---|
| confidence | Gate verdicts | Each gate pass/fail |
| novelty | HDC similarity to corpus | On Signal creation |
| utility | Downstream usage count | Each time the Signal is referenced |
| reputation | Author's historical pass rate | Periodic reputation recalculation |
| precision | Error specificity | LLM evaluation |
| salience | Context match score | Per-query relevance check |
| coherence | MDL model fit | Against same-Kind corpus |

---

## Academic Foundations

| Citation | Contribution |
|---|---|
| Friston 2010, Nature Reviews Neuroscience 11(2) | Precision weighting in active inference. Foundation for the precision axis. |
| Scherer 2001, Applied AI 15 | Component Process Model: 14 SECs in 4 stages. Maps to 7-axis scoring. |
| Lazarus 1991, *Emotion and Adaptation*, OUP | Cognitive-mediational theory: 5-7 appraisal dimensions. |
| Smith & Ellsworth 1985, JPSP 48(4) | Empirical patterns of cognitive appraisal: 6 dimensions. |
| Damasio 1994, *Descartes' Error* | Somatic markers: emotional signals bias decision-making. |
| Kahneman & Tversky 1979, Econometrica 47(2) | Prospect theory: non-linear weighting. Informs multiplicative formula. |
| Miller 1956, Psychological Review 63(2) | The magical number seven: channel capacity limits. |
| Thurstone 1947, Univ. Chicago Press | Multiple Factor Analysis: empirical 7-factor structure. |
| Hwang & Yoon 1981, Springer | TOPSIS: distance to ideal/anti-ideal solution. |
| Roy 1968, RIRO | ELECTRE: outranking with veto thresholds. |
| Brans & Vincke 1985, Management Science 31(6) | PROMETHEE: pairwise preference with net flow ranking. |
| Guo et al. 2017, ICML | Temperature scaling for calibration. |
| Vovk et al. 2005, Springer | Conformal prediction: distribution-free coverage guarantees. |
| Gelman et al. 2013, CRC Press | Bayesian Data Analysis: hierarchical models for cross-domain calibration. |
