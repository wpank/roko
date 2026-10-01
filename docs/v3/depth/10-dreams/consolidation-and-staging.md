# Consolidation and the Staging Buffer

> **v3 depth file** -- `/docs/v3/depth/10-dreams/consolidation-and-staging.md`
> Canonical source: v1 `docs/v1/10-dreams/04-consolidation-and-staging.md`
> Implementation: `crates/roko-dreams/src/staging.rs`, `crates/roko-dreams/src/cycle.rs`
> Status: **Wired** -- `StagingBuffer`, `ConfidenceStage`, tier progression,
> promotion to NeuroStore, GC, and staging statistics in `DreamCycleReport` are live

---

## 1. What Consolidation Does

Consolidation is the third and final phase of each dream cycle. It is a pure
computation phase -- no LLM call required. Its purpose is to evaluate the outputs
from NREM replay (insights, cross-episode patterns) and REM imagination
(counterfactual hypotheses, creative recombinations), stage them for future
validation, and promote validated entries to permanent knowledge in NeuroStore.

The biological basis is the synaptic homeostasis hypothesis (Tononi & Cirelli
2006, Sleep Medicine Reviews): during waking, synaptic connections accumulate.
During sleep, a global renormalization occurs -- important connections are
strengthened while unimportant ones are pruned.

---

## 2. The Staging Buffer

### 2.1 Design

Dream-generated hypotheses do not go directly into permanent knowledge. They
enter a **staging buffer** -- a holding area where hypotheses wait for waking
validation. This is the "dream to reality check" pipeline.

The staging buffer is implemented in `crates/roko-dreams/src/staging.rs`:

```rust
pub enum ConfidenceStage {
    /// Just entered from a dream. No waking evidence yet. Confidence 0.20--0.30.
    Raw,
    /// Some waking evidence supports the hypothesis. Confidence 0.30--0.50.
    Replayed,
    /// Multiple independent confirmations. Confidence 0.50--0.70.
    Validated,
    /// Promoted to permanent NeuroStore entry. Confidence >= 0.70.
    Promoted,
    /// Waking evidence directly contradicts. Special terminal state.
    Refuted,
    /// Not validated within the expiration window. Terminal state.
    Expired,
}
```

### 2.2 Why a Staging Buffer?

Dreams are creative but unreliable. The REM phase deliberately suppresses
executive control to enable novel combinations -- but this means many dream
outputs are speculative, contradictory, or simply wrong. The staging buffer
provides a "trial period" where hypotheses can be tested against reality before
being trusted.

This mirrors the biological process: not all dream content is adaptive. Much of
it is noise. The brain's consolidation mechanisms selectively strengthen useful
associations and let useless ones decay.

---

## 3. The Confidence Ladder

Hypotheses climb through a five-stage confidence ladder:

| Stage | Confidence | Status | Action |
|-------|-----------|--------|--------|
| 1 | 0.20--0.30 | `Raw` | Stored. Not used for decision-making. |
| 2 | 0.30--0.50 | `Replayed` | Agent may reference but does not rely on it. |
| 3 | 0.50--0.70 | `Validated` | Agent acts on it tentatively. |
| 4 | >= 0.70 | `Promoted` | Written to permanent NeuroStore. |
| 5 | (special) | `Refuted` | Contradicted by waking evidence. |

### Confidence Boost Mechanics

Each independent waking confirmation boosts confidence:

```
new_confidence = old_confidence + confirmation_boost x (1.0 - old_confidence)
```

Where `confirmation_boost` is configurable (default: 0.15). The
`(1.0 - old_confidence)` factor ensures diminishing returns.

An "independent confirmation" requires:
1. HDC similarity > 0.60 to the hypothesis content
2. Occurred after the hypothesis was created
3. Successful outcome (passed all gates)
4. Not itself generated during a dream cycle

### Refutation Mechanics

A hypothesis is refuted when waking evidence directly contradicts it. Refuted
hypotheses are not deleted -- they remain for reference. The refutation itself
is useful data: "I dreamed X, but reality showed not-X."

If the hypothesis originally contradicted an existing knowledge entry, that
entry receives a confidence boost of 0.10.

---

## 4. Promotion to Permanent Knowledge

When a hypothesis reaches confidence >= 0.70:

### Step 1: Knowledge Type Assignment

| Generation Mode | Typical Knowledge Type |
|----------------|----------------------|
| NREM standard/reverse replay | Insight |
| NREM cross-episode pattern | Insight |
| REM Pearl SCM (any level) | Heuristic |
| REM Boden combinational | Insight |
| REM Boden exploratory | Heuristic |
| REM Boden transformational | Strategy |
| Threat simulation | Warning |

### Step 2: NeuroStore Write

The hypothesis is written as a `KnowledgeEntry`:

```rust
let entry = KnowledgeEntry {
    id: generate_id(),
    content: hypothesis.content.clone(),
    kind: assigned_knowledge_type,
    confidence: hypothesis.confidence,
    source: "dream".to_string(),
    provenance: Provenance::Dream {
        dream_cycle_id: current_cycle_id.clone(),
        generation_mode: hypothesis.generation_mode.clone(),
    },
    // ...
};
knowledge_store.write(entry).await?;
```

### Step 3: Playbook Update

If the promoted entry is a Heuristic, it is compiled into the playbook store.

### Step 4: Event Emission

Promotion events are emitted for downstream listeners.

---

## 5. Temporal Decay and Expiration

### Unvalidated Hypothesis Expiration

Hypotheses not validated within a configurable window (default: 14 days) expire.
Expired hypotheses are not deleted -- they remain queryable but are no longer
candidates for promotion.

### Knowledge Demurrage

Promoted knowledge entries are subject to temporal decay:

| Knowledge Type | Default Half-Life |
|---------------|------------------|
| Insight | 30 days |
| Heuristic | 90 days |
| Strategy | 60 days |
| Warning | 14 days |
| Fact | 365 days |

Confidence decays exponentially:

```
current_confidence = initial_confidence x 2^(-age / half_life)
```

Each waking confirmation resets the decay clock and applies a 1.5x boost.
Dream-validated entries are exempt from decay for the first 7 days.

---

## 6. Dream-Specific Quality Calibration

| Quality Signal | Effect on Confidence |
|---------------|---------------------|
| Confirms existing knowledge | +0.05 bonus |
| Contradicts existing knowledge | -0.05 penalty |
| Transformational creativity | -0.05 penalty |
| Multiple source episodes | +0.02 per source (max +0.10) |
| High prediction error source | +0.03 bonus |

---

## 7. Safety Constraints

1. Dream hypotheses cannot trigger tools or modify files
2. Staging buffer capped at 1,000 entries (GC oldest expired first)
3. Maximum 3 contradictions per dream cycle
4. Initial confidence capped at 0.30

---

## 8. Staging Buffer Statistics

The dream cycle report includes staging buffer stats:

```rust
pub struct StagingBufferStats {
    pub total_entries: usize,
    pub raw_count: usize,
    pub replayed_count: usize,
    pub validated_count: usize,
    pub promoted_this_cycle: usize,
    pub demoted_this_cycle: usize,
    pub gc_removed: usize,
}
```

---

## 9. Synaptic Homeostasis: Causal Evidence (2024)

Sawada et al. (2024, Science) provided the **first causal demonstration** of the
Synaptic Homeostasis Hypothesis. Using SYNCit-K, they causally manipulated
dendritic spine size in PFC excitatory neurons, confirming: potentiation leads to
increased sleep drive leads to downscaling during sleep. PP2Ac-alpha (Communications
Biology 2025) further confirmed that synaptic homeostasis operates specifically
through excitatory circuits.

This validates the consolidation phase's design: during integration, the agent
performs global renormalization -- strengthen important knowledge, prune
unimportant knowledge.

---

## 10. Academic Citations

| Paper | How It Informs Consolidation |
|-------|------------------------------|
| Tononi & Cirelli (2006), Sleep Medicine Reviews | Synaptic homeostasis: strengthen important, prune unimportant |
| Stickgold & Walker (2013) | Sleep-dependent memory triage |
| McClelland et al. (1995), CLS theory | Fast episodic to slow semantic transfer |
| Grasse (1959), Insectes Sociaux 6(1) | Stigmergic knowledge: decay without reinforcement |
| Park et al. (2023), UIST | Generative Agents memory synthesis |
| WSCL (Sorrenti et al. 2024) | Wake, NREM and REM phases beat continual-learning baselines on image classification, with positive forward transfer (abstract) |
| Sawada et al. (2024), Science | First causal demonstration of SHY |
| PP2Ac-alpha (2025), Communications Biology | Phosphatase regulation of synaptic homeostasis |

---

## 11. Cross-References

| Document | Relevance |
|----------|-----------|
| [nrem-replay.md](nrem-replay.md) | NREM outputs entering the staging buffer |
| [rem-imagination.md](rem-imagination.md) | REM outputs entering the staging buffer |
| [dream-evolution.md](dream-evolution.md) | EVOLUTION phase operates on promoted knowledge |
| [dream-journals.md](dream-journals.md) | DreamCycleReport captures staging statistics |
