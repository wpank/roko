# NREM Replay: Utility-Weighted Episode Consolidation

> **v3 depth file** -- `/docs/v3/depth/10-dreams/nrem-replay.md`
> Canonical source: v1 `docs/v1/10-dreams/02-nrem-replay.md` (1,380 lines)
> Implementation: `crates/roko-dreams/src/replay.rs`, `crates/roko-dreams/src/cycle.rs`
> Status: **Wired** -- `ReplayUtility`, `DreamReplayMode`, `MattarDawConfig`,
> affect-weighted selection, four replay modes, HDC cluster-compression are live

---

## 1. What NREM Replay Does

NREM (Non-Rapid Eye Movement) replay is the first phase of every dream cycle. It
takes accumulated episodes from the agent's episode log and replays them -- not
verbatim, but with controlled mutations -- to consolidate memory and extract
patterns. The goal is threefold:

1. **Strengthen useful memories**: Episodes that contain valuable patterns get
   reinforced in NeuroStore. Their associated knowledge entries gain confidence.
2. **Weaken irrelevant memories**: Episodes that contain no actionable patterns
   enter temporal decay. Their associated knowledge entries lose confidence.
3. **Extract cross-episode patterns**: Structural similarities between unrelated
   episodes are discovered through HDC clustering and trigram mining.

The biological analogy is hippocampal sharp-wave ripples during slow-wave sleep
(stages N2--N3). Ji & Wilson (2007, Nature Neuroscience) showed that these ripples
replay compressed versions of waking experiences, and the replay is not passive --
it actively reorganizes memory.

---

## 2. The Mattar-Daw Utility Formula

### 2.1 Theoretical Foundation

Mattar & Daw (2018, "Prioritized memory access explains planning and hippocampal
replay," Nature Neuroscience 21(11):1609--1617) provided the first formal model
of which experiences should be replayed during offline consolidation. Their key
insight: replay is not random memory refresh -- it is **prioritized planning**.
The brain selectively replays experiences that would produce the largest expected
improvement in future behavior.

The utility formula is:

```
U(episode) = Gain(episode) x Need(episode) x SpacingInverse(episode)
```

Each term captures a distinct aspect of replay value.

### 2.2 Gain: Expected Learning Value

Gain measures how much the agent's behavior would improve by replaying this
episode. It is derived from prediction error -- the gap between what the agent
expected and what actually happened.

The implementation in `crates/roko-dreams/src/replay.rs`:

```rust
/// Gain: derived from prediction error.  Verify failures have high gain
/// (more to learn), clean passes have low gain.
fn compute_gain(episode: &Episode) -> f64 {
    let gate_total = episode.gate_verdicts.len().max(1) as f64;
    let fail_count = episode
        .gate_verdicts
        .iter()
        .filter(|v| !v.passed)
        .count() as f64;
    // Prediction error: how surprising was the outcome?
    let error_rate = fail_count / gate_total;
    let surprise = if episode.success {
        // Successful but with some failures -- moderately surprising
        1.0 + error_rate * 0.5
    } else {
        // Failed -- high prediction error
        1.5 + error_rate
    };
    // Token usage as a secondary signal (complex tasks have more to learn)
    let complexity = (episode.tokens_used.max(1) as f64)
        .log10()
        .clamp(0.0, 3.0)
        * 0.1;
    surprise + complexity
}
```

The gain function has two components:

1. **Surprise** (primary): Episodes where the outcome was unexpected have high
   gain. Failed episodes are more surprising than successful ones (base 1.5 vs
   1.0). The gate failure rate adds to surprise -- more gate failures mean the
   agent's model was further from reality.

2. **Complexity** (secondary): The log of token usage serves as a proxy for task
   complexity. Complex tasks have more to learn from, contributing a small
   additive boost (0.0--0.3).

The intuition: a failed task that consumed many tokens and had multiple gate
failures is maximally surprising and maximally informative -- it should be
replayed first.

### 2.3 Need: Policy Relevance

Need measures how much the current policy needs updating for situations similar
to this episode. It combines novelty (how rare this episode pattern is) with
recency (how recently the agent encountered this type of situation).

```rust
/// Need: how much the current policy needs updating.  Combines novelty
/// (low for frequently-seen patterns) with recency (recent episodes are
/// more policy-relevant).
fn compute_need(novelty: f64, recency: f64) -> f64 {
    let novelty_term = novelty.clamp(0.0, 1.0);
    let recency_term = recency.clamp(0.0, 1.0);
    // Weighted combination: novelty matters more than raw recency
    0.6 * novelty_term + 0.4 * recency_term
}
```

Novelty and recency are weighted 60/40 in favor of novelty: novel episodes
(those unlike anything seen before) have higher need than merely recent ones.
This prevents the agent from obsessively replaying recent familiar episodes at
the expense of rare but important experiences.

### 2.4 Spacing Inverse: Spaced Repetition

The spacing term implements the spacing effect from memory research (Cepeda et
al. 2006, "Distributed practice in verbal recall tasks: A review and
quantitative synthesis," Psychological Bulletin 132(3):354--380). Episodes
replayed too recently receive lower priority; episodes not replayed for a while
receive higher priority.

```rust
/// Spacing inverse: inverse of effective "time since last useful replay".
fn compute_spacing_inv(episode: &Episode, recency: f64) -> f64 {
    let base_spacing = 1.0 - recency.clamp(0.0, 0.99);
    let never_replayed = !episode.extra.contains_key("dream:replayed");
    let boost = if never_replayed { 1.5 } else { 1.0 };
    (base_spacing * boost).max(0.01)
}
```

Key behaviors:
- Episodes that have never been replayed (no `dream:replayed` marker) receive
  a 1.5x boost -- they represent unprocessed experience.
- Recently replayed episodes have recency close to 1.0, producing a low spacing
  score -- they should not be replayed again immediately.
- The minimum spacing of 0.01 prevents any episode from being completely excluded.

### 2.5 Configurable Weights

The gain and need terms accept configurable weights via `MattarDawConfig`:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MattarDawConfig {
    /// Weight applied to the gain term (default 1.0).
    #[serde(default = "default_mattar_daw_weight")]
    pub gain_weight: f64,
    /// Weight applied to the need term (default 1.0).
    #[serde(default = "default_mattar_daw_weight")]
    pub need_weight: f64,
}
```

Setting `gain_weight = 2.0` doubles the influence of prediction error on replay
priority. This is useful for agents operating in volatile environments where
failures are highly informative. Setting `need_weight = 0.5` halves the influence
of novelty/recency, useful for agents operating in stable domains where familiar
patterns are as important as novel ones.

### 2.6 Full Utility Computation

The complete utility score is the product of all three terms:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ReplayUtility {
    /// Expected improvement from replaying this episode.
    pub gain: f64,
    /// How policy-relevant this episode is.
    pub need: f64,
    /// Inverse of time since last replay (spaced-repetition term).
    pub spacing_inv: f64,
    /// Final utility: `gain * need * spacing_inv`.
    pub utility: f64,
}

impl ReplayUtility {
    pub fn compute(
        episode: &Episode,
        novelty: f64,
        recency: f64,
        config: &MattarDawConfig,
    ) -> Self {
        let gain = Self::compute_gain(episode) * config.gain_weight;
        let need = Self::compute_need(novelty, recency) * config.need_weight;
        let spacing_inv = Self::compute_spacing_inv(episode, recency);
        let utility = gain * need * spacing_inv;
        Self { gain, need, spacing_inv, utility }
    }
}
```

The multiplicative structure means all three factors must be present for high
utility. A surprising episode (high gain) that is completely redundant (low need)
still has low utility. A novel episode (high need) that revealed nothing
unexpected (low gain) also has low utility. Only episodes that are both surprising
AND novel AND not recently replayed receive the highest replay priority.

---

## 3. Four Replay Modes

The implementation provides four distinct replay modes, each selecting episodes
according to a different strategy.

### 3.1 Random Mode

```rust
pub enum DreamReplayMode {
    /// Sample episodes uniformly using a deterministic pseudo-random ordering.
    Random,
    // ...
}
```

Random mode uses a deterministic pseudo-random ordering derived from episode
signatures. This is the baseline mode: it ensures broad coverage of the episode
space without any particular bias. Random mode is the default.

The implementation hashes episode signatures and IDs to produce a stable random
rank, then sorts by that rank and truncates to `max_episodes`:

```rust
fn select_random(
    mut candidates: Vec<ReplayCandidate>,
    max_episodes: usize,
) -> Vec<ReplayCandidate> {
    candidates.sort_by(|left, right| {
        left.random_rank
            .cmp(&right.random_rank)
            .then_with(|| left.episode.timestamp.cmp(&right.episode.timestamp))
    });
    candidates.truncate(max_episodes);
    candidates
}
```

The determinism is important: the same episode set produces the same replay
batch. This makes dream cycles reproducible and testable.

### 3.2 Consequence Mode

```rust
    /// Prioritize episodes with the largest outcome signal.
    Consequence,
```

Consequence mode sorts candidates by Mattar-Daw utility score in descending
order. It replays the episodes with the highest expected learning value first.
This is the most aggressive mode: it concentrates replay on the most informative
experiences.

```rust
fn select_consequence(
    mut candidates: Vec<ReplayCandidate>,
    max_episodes: usize,
) -> Vec<ReplayCandidate> {
    candidates.sort_by(|left, right| {
        right.utility
            .partial_cmp(&left.utility)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.episode.timestamp.cmp(&right.episode.timestamp))
    });
    candidates.truncate(max_episodes);
    candidates
}
```

### 3.3 Causal Mode

```rust
    /// Follow the earliest failure chains back toward likely root causes.
    Causal,
```

Causal mode groups episodes by task chain (using `task_id` as the chain key) and
identifies failure points within each chain. For each chain containing at least
one failure, it selects the first failure and its immediately preceding episode
(if any). This captures the causal boundary -- the point where success transitioned
to failure.

```rust
fn select_causal(
    mut candidates: Vec<ReplayCandidate>,
    max_episodes: usize,
) -> Vec<ReplayCandidate> {
    let original_candidates = candidates.clone();
    let mut groups = BTreeMap::<String, Vec<ReplayCandidate>>::new();
    for candidate in candidates.drain(..) {
        groups
            .entry(replay_chain_key(&candidate.episode))
            .or_default()
            .push(candidate);
    }

    let mut roots = Vec::new();
    for mut chain in groups.into_values() {
        chain.sort_by(|left, right| {
            left.episode.timestamp
                .cmp(&right.episode.timestamp)
                .then_with(|| left.episode.id.cmp(&right.episode.id))
        });

        let failure_index = chain
            .iter()
            .position(|candidate| !candidate.episode.success);
        if let Some(index) = failure_index {
            roots.push(chain[index].clone());
            if index > 0 {
                roots.push(chain[index - 1].clone());
            }
        }
    }

    if roots.is_empty() {
        return select_consequence(original_candidates, max_episodes);
    }

    dedupe_by_episode_id(&mut roots);
    roots.sort_by(|left, right| {
        left.episode.timestamp
            .cmp(&right.episode.timestamp)
            .then_with(|| left.episode.id.cmp(&right.episode.id))
    });
    roots.truncate(max_episodes);
    roots
}
```

The causal mode falls back to consequence mode when no failure chains exist.
This prevents empty replay batches when the agent has had a run of successes.

### 3.4 Hypothetical Mode

```rust
    /// Replay counterfactual variants of the strongest episodes.
    Hypothetical,
```

Hypothetical mode selects the highest-utility episodes (as in consequence mode)
then mutates them to create counterfactual variants. The mutations:

- Replace the episode ID with `{original_id}-hyp-{index}`
- Replace the model with the next tier (haiku becomes sonnet, sonnet becomes opus)
- Set `trigger_kind` to `"dream:hypothetical"`
- Flip failure to success (what if this had succeeded?)
- Add metadata recording the counterfactual transformation

```rust
fn hypothetical_variant(episode: &Episode, index: usize) -> Episode {
    let mut variant = episode.clone();
    variant.id = format!("{}-hyp-{index}", episode.id);
    variant.model = counterfactual_model(&episode.model);
    variant.trigger_kind = "dream:hypothetical".to_string();
    variant.extra.insert(
        "dream:hypothetical".to_string(),
        json!({
            "mode": "hypothetical",
            "source_episode_id": episode.id,
            "source_model": episode.model,
            "counterfactual_model": variant.model,
            "source_success": episode.success,
        }),
    );
    if !variant.success {
        variant.success = true;
        variant.failure_reason = None;
    }
    variant
}
```

The utility of hypothetical variants is discounted by 5% (multiplied by 0.95)
to reflect the inherent uncertainty of counterfactual reasoning.

---

## 4. Episode Signature and Novelty

### 4.1 Signature Computation

Each episode receives a deterministic signature computed from its structural
features (task ID, model, trigger kind, success, failure reason, gate verdicts):

```rust
fn episode_signature(episode: &Episode) -> u64 {
    let mut hasher = DefaultHasher::new();
    episode.task_id.hash(&mut hasher);
    episode.model.hash(&mut hasher);
    episode.trigger_kind.hash(&mut hasher);
    episode.success.hash(&mut hasher);
    episode.failure_reason.hash(&mut hasher);
    for EpisodeGateVerdict { gate, passed, signature } in &episode.gate_verdicts {
        gate.hash(&mut hasher);
        passed.hash(&mut hasher);
        signature.hash(&mut hasher);
    }
    hasher.finish()
}
```

Episodes with the same signature are structurally similar -- they represent the
same task/model/outcome pattern. The novelty score decays as more episodes with
the same signature are seen:

```
novelty = 1.0 / (1.0 + (seen_count / novelty_window))
```

Where `novelty_window` (default: 12) controls how quickly novelty decays. The
first instance of a signature has novelty 1.0; the 12th instance has novelty
0.5; by the 36th instance, novelty is 0.25.

### 4.2 Recency Decay

Recency uses exponential decay with a configurable half-life (default: 24 hours):

```rust
fn recency_decay(timestamp: DateTime<Utc>, now: DateTime<Utc>, half_life_hours: f64) -> f64 {
    let age_hours = (now - timestamp).num_seconds().max(0) as f64 / 3600.0;
    let half_life = half_life_hours.max(0.1);
    (-age_hours / half_life).exp()
}
```

An episode from 24 hours ago has recency ~0.37. An episode from 48 hours ago has
recency ~0.14. An episode from 1 hour ago has recency ~0.96.

---

## 5. Affect-Weighted Replay Selection

When the Daimon (affect engine) provides emotional context, the replay selection
is modulated by the agent's current PAD (Pleasure-Arousal-Dominance) state.

```rust
pub fn select_replay_episodes_with_affect(
    episodes: &[Episode],
    policy: &DreamReplayPolicy,
    now: DateTime<Utc>,
    emotional_context: Option<&PadVector>,
) -> DreamReplayBatch {
    let Some(pad) = emotional_context else {
        return select_replay_episodes(episodes, policy, now);
    };

    // Arousal-based intensity: high arousal increases max_episodes by up to 50%.
    let arousal_factor = 1.0 + 0.5 * pad.arousal.max(0.0);
    let effective_max = ((policy.max_episodes as f64) * arousal_factor).round() as usize;

    let adjusted_policy = DreamReplayPolicy {
        max_episodes: effective_max,
        ..policy.clone()
    };

    let mut candidates = score_candidates(episodes, &adjusted_policy, now);

    // Negative pleasure biases toward failure episodes.
    let failure_bias = 1.0 + 0.5 * (-pad.pleasure).max(0.0);
    for candidate in &mut candidates {
        if !candidate.episode.success {
            candidate.utility *= failure_bias;
        }
    }

    // ... selection proceeds with adjusted candidates
}
```

Two affect modulations:

1. **Arousal intensity**: High arousal (A > 0) increases the effective
   `max_episodes` by up to 50%. An aroused agent replays more intensely -- more
   episodes per cycle. At arousal = 1.0, effective_max = 1.5x policy.max_episodes.

2. **Valence failure bias**: Negative pleasure (P < 0) biases replay toward
   failed episodes. At pleasure = -1.0, failure episodes receive a 1.5x utility
   multiplier. This implements the biological finding that negative emotional
   states drive more intensive processing of threatening experiences (Revonsuo
   2000, Threat Simulation Theory).

When no emotional context is available, the function delegates to the base
`select_replay_episodes` without modification.

---

## 6. Replay Policy Configuration

The full replay policy is configured via `DreamReplayPolicy`:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DreamReplayPolicy {
    /// Which replay mode to use.
    pub mode: DreamReplayMode,
    /// Maximum number of episodes to replay in a single pass.
    pub max_episodes: usize,          // default: 24
    /// How many prior episodes of the same signature are needed before the
    /// novelty score drops sharply.
    pub novelty_window: usize,        // default: 12
    /// Half-life used by the recency decay term, in hours.
    pub recency_half_life_hours: f64, // default: 24.0
    /// Mattar-Daw gain/need weights.
    pub mattar_daw: MattarDawConfig,
}
```

Corresponding TOML configuration in `roko.toml`:

```toml
[dreams]
batch_size = 10  # episodes per cycle

[dreams.replay]
mode = "consequence"  # random | consequence | causal | hypothetical
max_episodes = 24
novelty_window = 12
recency_half_life_hours = 24.0

[dreams.replay.mattar_daw]
gain_weight = 1.0
need_weight = 1.0
```

---

## 7. HDC Cluster-Compression

### 7.1 Overview

After episode selection, the replay phase uses Hyperdimensional Computing (HDC)
to cluster structurally similar episodes. This is the "compression" step: raw
episodes are compressed into cluster summaries that capture shared patterns.

The `CrossEpisodeConsolidator` from `roko-learn/src/pattern_discovery.rs` uses
K-medoids clustering over 10,240-bit Binary Spatter Code (BSC) vectors:

```rust
pub fn consolidate(
    &self,
    episode_vectors: &[(usize, HdcVector)],
) -> Vec<CrossEpisodeMetaPattern> {
    let config = KMedoidsConfig {
        k: self.target_clusters,
        max_iterations: self.max_iterations,
    };
    let result = k_medoids(&vectors, &config);
    // convert clusters to meta-patterns with coherence scores
}
```

### 7.2 Vector Generation

Each episode is converted to an HDC vector using `text_fingerprint`:

```rust
let episode_vector = text_fingerprint(&episode_text);
```

The text fingerprint encodes the episode's task description, model, gate
verdicts, and outcome into a 10,240-bit BSC vector using character n-gram
hashing. Episodes with similar content produce similar vectors (high Hamming
similarity); episodes with different content produce dissimilar vectors.

### 7.3 K-Medoids Clustering

K-medoids is preferred over K-means for BSC vectors because:

1. **Binary vectors**: K-means computes centroids via arithmetic mean, which is
   undefined for binary vectors. K-medoids selects actual data points as cluster
   centers (medoids), which works naturally with Hamming distance.

2. **Robustness**: K-medoids is more robust to outliers than K-means. Unusual
   episodes do not distort cluster centers.

3. **Interpretability**: Each cluster center is an actual episode, making the
   clusters interpretable -- the center episode is the "most representative"
   member.

The target number of clusters is configurable (default: 5--8, depending on
episode count). Each cluster produces a `CrossEpisodeMetaPattern` with:

- **Episode indices**: Which episodes belong to this cluster
- **Coherence score**: How tightly clustered the members are (mean Hamming
  similarity to the medoid)
- **Description**: Auto-generated text summarizing the cluster's common features
- **Signature**: Stable hash for deduplication across dream cycles

### 7.4 Pattern Mining

Within each cluster, the `PatternMiner` uses trigram mining with FNV-1a hashing
to discover recurring action patterns:

1. Extract action sequences from each episode in the cluster
2. Compute trigrams (sequences of three consecutive actions)
3. Hash each trigram with FNV-1a for fast comparison
4. Identify trigrams that appear in >= 60% of cluster members

Recurring trigrams become candidate patterns -- structural regularities in the
agent's behavior that span multiple episodes. These patterns are the raw material
for insight generation.

### 7.5 Cluster Reports

Each cluster in the dream cycle report includes:

```rust
pub struct DreamClusterReport {
    /// Cluster key (task_id or model or domain).
    pub key: DreamClusterKey,
    /// Number of episodes in this cluster.
    pub episode_count: usize,
    /// Success rate within the cluster.
    pub success_rate: f64,
    /// Average token usage.
    pub mean_tokens: f64,
    /// Most common gate failure (if any).
    pub primary_failure_gate: Option<String>,
    /// Whether a knowledge entry was generated from this cluster.
    pub knowledge_generated: bool,
}
```

---

## 8. Replay Batch Output

The complete replay batch is returned as a `DreamReplayBatch`:

```rust
pub struct DreamReplayBatch {
    /// Replay mode that produced the batch.
    pub mode: DreamReplayMode,
    /// Selected episodes in replay order.
    pub episodes: Vec<Episode>,
    /// Total Mattar-Daw utility accumulated by the batch.
    pub utility_score: f64,
    /// Number of hypothetical variants emitted for the batch.
    pub hypothetical_count: usize,
}
```

The batch provides:
- The selected episodes in replay order (mode-dependent)
- The total accumulated utility score (sum of individual episode utilities)
- The count of hypothetical variants (nonzero only in Hypothetical mode)

The `DreamCycle` consumes this batch, runs the consolidation pipeline over the
selected episodes, and produces a `DreamCycleReport`.

---

## 9. Standalone Utility Computation

For external callers that want the decomposed Mattar-Daw score without running
the full selection pipeline:

```rust
/// Compute Mattar-Daw utility score for a single episode.
pub fn compute_replay_utility(
    episode: &Episode,
    policy: &DreamReplayPolicy,
    now: DateTime<Utc>,
) -> ReplayUtility {
    let recency = recency_decay(episode.timestamp, now, policy.recency_half_life_hours);
    ReplayUtility::compute(episode, 1.0, recency, &policy.mattar_daw)
}
```

This uses a default novelty of 1.0 (unknown context) since the full novelty
computation requires the complete episode set for signature counting.

---

## 10. Counterfactual Model Selection

The hypothetical mode replaces the original model with the next tier:

```rust
fn counterfactual_model(model: &str) -> String {
    let trimmed = model.trim();
    if trimmed.is_empty() {
        return "dream-counterfactual-model".to_string();
    }
    if trimmed.contains("haiku") {
        return trimmed.replace("haiku", "sonnet");
    }
    if trimmed.contains("sonnet") {
        return trimmed.replace("sonnet", "opus");
    }
    if trimmed.contains("fast") {
        return trimmed.replace("fast", "standard");
    }
    format!("{trimmed}-counterfactual")
}
```

This implements the counterfactual question: "What if we had used a more capable
model?" The model tier escalation (haiku to sonnet, sonnet to opus) is a
structured perturbation that asks whether the task's failure was due to model
capability rather than task complexity.

---

## 11. Integration with the Dream Cycle

The replay module is consumed by `DreamCycle::run()` in
`crates/roko-dreams/src/cycle.rs`:

1. The dream cycle loads episodes from `EpisodeLogger`
2. It filters to episodes since the last dream report's `processed_through`
   timestamp
3. It calls `select_replay_episodes_with_affect()` with the Daimon's PAD vector
4. The returned batch is processed through:
   - Tier progression analysis
   - HDC cluster-compression
   - Cross-episode consolidation
   - Knowledge entry generation
   - Playbook creation
   - Routing advice generation
5. A `DreamCycleReport` is written to `.roko/dreams/`

The replay phase does not directly invoke LLM inference. It selects and
organizes episodes for downstream processing. The LLM call happens during the
consolidation step, where the dream agent reviews cluster summaries and
generates natural-language insights.

---

## 12. Biological Grounding

### 12.1 Hippocampal Replay

The NREM replay implementation is grounded in the hippocampal memory
consolidation literature:

- **Ji & Wilson (2007)**: Showed that hippocampal place cells replay recent
  experiences during sleep in compressed form (20x faster than real-time).
  Roko's episode replay is analogously compressed -- episodes are reduced to
  their structural signatures rather than replayed verbatim.

- **Buzsaki (1989)**: Identified sharp-wave ripples as the neural mechanism
  coordinating replay. In Roko, the `DreamReplayPolicy` serves as the
  scheduling mechanism that coordinates which episodes are replayed.

- **Ambrose et al. (2016)**: Demonstrated bidirectional replay -- both forward
  (cause to effect) and backward (effect to cause). Roko's causal mode
  implements this by selecting both the failure episode and its preceding
  success episode, enabling the agent to trace causal chains in both directions.

### 12.2 Spaced Repetition

The spacing term implements the spacing effect from Cepeda et al. (2006):

- Massed practice (replaying the same episode repeatedly) produces inferior
  long-term retention compared to distributed practice (spacing replays over
  time).
- The optimal spacing interval increases with each successful retrieval -- the
  Leitner system principle.
- Roko's exponential recency decay approximates this: episodes replayed
  recently have low spacing scores, naturally distributing replay over time.

### 12.3 Prediction Error and Learning

The gain term's reliance on prediction error connects to:

- **Rescorla-Wagner model (1972)**: Learning rate is proportional to prediction
  error. Surprising outcomes produce more learning than expected outcomes.
- **Mattar & Daw (2018)**: Extended the prediction-error principle to replay
  selection -- the brain prioritizes replaying experiences where the prediction
  error was largest, because those experiences have the most to teach.

---

## 13. Test Coverage

The replay module includes comprehensive tests verifying:

1. **Consequence mode ordering**: Higher-utility episodes appear first in the
   batch, and the batch utility score equals the sum of individual scores.

2. **Causal mode root selection**: Failure chain roots (first failure + preceding
   episode) are selected; fallback to consequence mode when no failures exist.

3. **Hypothetical mode mutation**: Selected episodes are mutated with
   counterfactual IDs, upgraded models, flipped success flags, and hypothetical
   metadata markers.

4. **Random mode determinism**: The same episode set produces the same replay
   batch, and `max_episodes` is respected.

5. **Affect modulation**: `None` emotional context delegates to base selection;
   high arousal increases effective max_episodes; negative pleasure biases
   toward failure episodes.

6. **Mattar-Daw properties**: Failed episodes have higher gain than successes;
   configurable weights modify utility proportionally; standalone utility
   computation returns positive components.

---

## 14. Academic Citations

| Paper | How It Informs NREM Replay |
|-------|---------------------------|
| Mattar & Daw (2018), Nature Neuroscience 21(11):1609--1617 | Utility formula for prioritized replay selection |
| Ji & Wilson (2007), Nature Neuroscience | Hippocampal sharp-wave ripple replay during SWS |
| Buzsaki (1989) | Sharp-wave ripple coordination model |
| Ambrose et al. (2016) | Bidirectional replay in hippocampal circuits |
| Cepeda et al. (2006), Psychological Bulletin 132(3):354--380 | Spacing effects in distributed practice |
| Rescorla & Wagner (1972) | Prediction error drives learning rate |
| Revonsuo (2000), Behavioral and Brain Sciences | Threat simulation: negative affect drives intensive replay |
| Walker & van der Helm (2009), Psychological Bulletin | REM emotional depotentiation |
| Diekelmann & Born (2010), Psychological Review | NREM consolidation mechanisms |
| McClelland et al. (1995), Psychological Review, CLS theory | Fast/slow memory systems bridged by replay |
| Kanerva (2009), Cognitive Computation 1(2) | Hyperdimensional Computing fundamentals |
| WSCL (Skenderi et al. 2024) | 38% catastrophic forgetting reduction via wake-sleep interleaving |

---

## 15. Cross-References

| Document | Relevance |
|----------|-----------|
| [three-phase-cycle.md](three-phase-cycle.md) | NREM replay is Phase 1 of the cycle |
| [rem-imagination.md](rem-imagination.md) | REM imagination consumes NREM replay outputs |
| [consolidation-and-staging.md](consolidation-and-staging.md) | Integration phase that stages replay discoveries |
| [hdc-counterfactual-synthesis.md](hdc-counterfactual-synthesis.md) | HDC vector operations for cluster compression |
| [scheduling-and-triggers.md](scheduling-and-triggers.md) | When replay cycles fire |
| [dream-routing-advice.md](dream-routing-advice.md) | Routing advice generated from replay clusters |
