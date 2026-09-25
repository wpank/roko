# Dream Journals

> **v3 depth file** -- `/docs/v3/depth/10-dreams/dream-journals.md`
> Implementation: `crates/roko-dreams/src/runner.rs`, `crates/roko-dreams/src/cycle.rs`
> Status: **Wired** -- `DreamCycleReport` persisted to `.roko/dreams/dream-{timestamp}.json`;
> `roko knowledge dream report` displays the latest report; journal-level trend
> analysis and lucid dream monitoring remain target design

---

## 1. What Dream Journals Are

The `DreamCycleReport` (see [three-phase-cycle.md](three-phase-cycle.md))
captures the output of a single dream cycle. A dream journal is the
longitudinal record: every cycle's metadata, outcomes, and quality metrics,
accumulated over the agent's lifetime. The journal enables trend analysis,
detects degradation in dream effectiveness, and supports metacognitive
monitoring -- the agent reasoning about the quality of its own dreaming.

---

## 2. The DreamCycleReport

Every completed dream cycle produces a `DreamCycleReport`:

```rust
pub struct DreamCycleReport {
    /// When the dream cycle started.
    pub started_at: DateTime<Utc>,
    /// When the dream cycle completed.
    pub completed_at: DateTime<Utc>,
    /// Timestamp of the most recent episode processed.
    pub processed_through: Option<DateTime<Utc>>,
    /// Number of episodes replayed during NREM.
    pub episodes_replayed: usize,
    /// Number of counterfactuals generated during REM.
    pub counterfactuals_generated: usize,
    /// Insights extracted during the cycle.
    pub insights: Vec<InsightRecord>,
    /// Patterns discovered via cross-episode consolidation.
    pub patterns: Vec<PatternRecord>,
    /// Hypotheses staged for future validation.
    pub staged_hypotheses: usize,
    /// Hypotheses promoted to permanent knowledge.
    pub promoted_hypotheses: usize,
    /// Confidence updates applied to existing knowledge.
    pub confidence_updates: usize,
    /// Emotional depotentiation summary.
    pub depotentiation: DepotentiationSummary,
}
```

Reports are stored as `dream-{unix_timestamp_ms}.json` in `.roko/dreams/`.
The `DreamRunner::latest_report()` method retrieves the most recent one.

---

## 3. CLI Access

Dream reports are accessible through the CLI:

```bash
roko knowledge dream run       # Fire a dream cycle and produce a report
roko knowledge dream report    # Display the latest dream report
roko knowledge dream schedule  # Show the next automatic dream deadline
roko knowledge dream journal   # Browse dream journal entries
roko knowledge dream archive   # Archive old dream journal entries
```

The `dream report` command reads the most recent report file from
`.roko/dreams/` and formats it for terminal output, showing episodes
replayed, insights discovered, hypotheses staged, and routing advice
generated.

---

## 4. Journal Entry Structure (Target Design)

Beyond the single-cycle report, the journal system captures richer per-cycle
metadata for longitudinal analysis:

```rust
pub struct DreamJournalEntry {
    pub cycle_id: String,
    pub agent_id: String,
    pub cycle_start: chrono::DateTime<chrono::Utc>,
    pub cycle_end: chrono::DateTime<chrono::Utc>,
    pub trigger: DreamTrigger,
    /// Duration of each phase in seconds.
    pub nrem_duration_secs: u64,
    pub rem_duration_secs: u64,
    pub consolidation_duration_secs: u64,
    /// Hypothesis counts.
    pub hypotheses_generated: usize,
    pub hypotheses_staged: usize,
    pub hypotheses_promoted: usize,
    pub hypotheses_refuted: usize,
    pub nightmares_detected: usize,
    pub human_review_required: bool,
    /// Diversity score: mean pairwise HDC cosine distance.
    pub hypothesis_diversity: f64,
    /// Compute cost in token-equivalents.
    pub total_tokens: u64,
    /// Whether the cycle was terminated early by the lucid dream monitor.
    pub early_termination: bool,
    pub early_termination_reason: Option<String>,
}

pub enum DreamTrigger {
    IdleTimeout,
    FailureEvent { gate_id: String },
    NoveltyDetection { novelty_score: f64 },
    Scheduled { cycle_number: u64 },
    Solicited { requester: String },
}
```

---

## 5. Trend Analysis

The `DreamTrendAnalysis` struct aggregates journal data across N cycles to
surface actionable patterns:

```rust
pub struct DreamTrendAnalysis {
    pub analyzed_at: chrono::DateTime<chrono::Utc>,
    pub cycle_count: usize,
    /// Promotion rate per creativity mode.
    pub promotion_rate_by_mode: std::collections::HashMap<String, f64>,
    /// Optimal cycle duration in seconds (at peak promotion rate).
    pub optimal_duration_secs: u64,
    /// Mean hypothesis diversity across analyzed cycles.
    pub mean_diversity: f64,
    /// Nightmare rate: nightmares per cycle.
    pub nightmare_rate: f64,
    /// Whether nightmare rate is trending upward.
    pub nightmare_rate_increasing: bool,
    /// Failure-triggered vs. scheduled cycle promotion rate comparison.
    pub failure_trigger_promotion_rate: f64,
    pub scheduled_trigger_promotion_rate: f64,
}
```

Across a sequence of entries, the journal supports queries such as:

- **Which creativity modes produce the most eventually-promoted hypotheses?**
  Ground truth: hypothesis_id appears in a promoted episode within 10 waking
  cycles.
- **Is dream effectiveness declining as the agent matures?** Compare promotion
  rates across time windows.
- **What is the optimal dream cycle duration for this agent's task domain?**
  Plot hypothesis count, diversity, and promotion rate against cycle duration.
- **Are failure-triggered dreams more productive than scheduled ones?** Compare
  promotion rates by trigger type.
- **Is the nightmare rate rising?** A rising rate may indicate the agent's
  threat simulation is producing increasingly unconstrained outputs.

---

## 6. Lucid Dreaming: Metacognitive Monitoring

Standard dream cycles run to completion and report results afterward. **Lucid
dreaming** is the analog of the biological phenomenon (Filevich et al. 2015,
Journal of Neuroscience): the dreaming system maintains metacognitive awareness
of its own state and can modify or terminate the dream based on that awareness.

### Three Monitoring Signals

```rust
pub struct LucidDreamMonitor {
    /// Minimum hypothesis diversity before triggering a warning.
    pub diversity_threshold: f64,          // default: 0.30, range: 0.10-0.60
    /// Minimum novelty score for the rolling window of recent hypotheses.
    pub novelty_decay_threshold: f64,      // default: 0.25
    /// Number of recent hypotheses to include in novelty decay calculation.
    pub novelty_window_size: usize,        // default: 5
    /// Whether to enable coherence collapse detection.
    pub enable_coherence_check: bool,      // default: true
    /// Number of signals below threshold required to trigger early termination.
    pub early_termination_signal_count: usize, // default: 2
    /// Check interval: run monitor every N hypotheses generated.
    pub check_interval: usize,             // default: 3
}
```

The three signals:

1. **Hypothesis diversity**: Are generated hypotheses all variations on the
   same theme (low diversity) or genuinely distinct (high diversity)? Measured
   by pairwise HDC cosine distance across the current cycle's hypotheses.

2. **Novelty decay**: Is the novelty score of successive hypotheses declining?
   If hypotheses are becoming less novel over time within a cycle, the creative
   recombination engine has likely exhausted its productive combinations.

3. **Coherence collapse**: Are hypotheses beginning to fail basic logical
   consistency checks? This can happen late in long cycles when the LLM's
   context is saturated.

### Early Termination

When two or more signals fall below threshold simultaneously, the monitor
triggers early termination: the current cycle concludes, consolidation runs
on whatever has been generated, and the next cycle is rescheduled at a
slightly shorter duration.

```rust
impl LucidDreamMonitor {
    pub fn evaluate(
        &self,
        hypotheses: &[Hypothesis],
    ) -> Option<String> {
        let mut failing_signals = 0;
        let mut reasons = Vec::new();

        // Signal 1: diversity check
        let diversity = compute_mean_pairwise_hdc_distance(hypotheses);
        if diversity < self.diversity_threshold {
            failing_signals += 1;
            reasons.push(format!(
                "diversity={:.2} below threshold={:.2}",
                diversity, self.diversity_threshold
            ));
        }

        // Signal 2: novelty decay check
        if hypotheses.len() >= self.novelty_window_size {
            let recent = &hypotheses[hypotheses.len() - self.novelty_window_size..];
            let mean_novelty: f64 =
                recent.iter().map(|h| h.novelty_score).sum::<f64>()
                    / recent.len() as f64;
            if mean_novelty < self.novelty_decay_threshold {
                failing_signals += 1;
                reasons.push(format!(
                    "novelty_decay={:.2} below threshold={:.2}",
                    mean_novelty, self.novelty_decay_threshold
                ));
            }
        }

        // Signal 3: coherence collapse
        if self.enable_coherence_check {
            let incoherent_count = hypotheses
                .iter()
                .filter(|h| !h.is_coherent)
                .count();
            if incoherent_count as f64 / hypotheses.len() as f64 > 0.4 {
                failing_signals += 1;
                reasons.push(format!(
                    "coherence_collapse: {}/{} hypotheses incoherent",
                    incoherent_count, hypotheses.len()
                ));
            }
        }

        if failing_signals >= self.early_termination_signal_count {
            Some(reasons.join("; "))
        } else {
            None
        }
    }
}
```

### Query Predictability and Early Termination

Lin et al. (2025, sleep-time compute) showed that query predictability is the
key determinant of sleep-time effectiveness. The lucid dream monitor applies
this principle: if the query driving this dream cycle is low-predictability
(high entropy), early termination is less costly because the cycle was
unlikely to produce high-value insights anyway.

---

## 7. Neuroscience-Informed Lucid Monitoring (Target Design)

### EEG Microstate Analogs

Reference: "EEG Microstates reveal distinct network dynamics in lucid and
non-lucid REM sleep," bioRxiv (2025).

The LucidDreamMonitor can be extended with computational analogs of EEG
microstates:

| Computational Microstate | EEG Analog | Description |
|-------------------------|-----------|-------------|
| SelfReferential | Microstate A | Hypothesis about agent's own behavior patterns |
| Executive | Microstate G | Structured counterfactual with explicit causal chain |
| Emotional | Microstate B | Hypothesis driven by affect rather than logic |
| DefaultMode | Microstate C | Unstructured associative drift |
| SensoryReplay | Microstate D | Replay-dominated, closely tracking episode content |

Lucidity increases when SelfReferential + Executive microstates dominate;
the monitor can intervene to shift the balance when Emotional/DefaultMode/
SensoryReplay dominance produces low-quality output.

```rust
/// Neuroscience-informed lucid dream monitoring.
pub struct NeuroinformedLucidMonitor {
    /// Minimum metacognitive microstate ratio (A+G / total) for lucidity.
    pub min_metacognitive_ratio: f64,      // default: 0.55, range: 0.40-0.80
    /// Window size for microstate analysis (hypotheses).
    pub microstate_window: usize,          // default: 5, range: 3-10
    /// Gamma analog: minimum information density per hypothesis.
    pub min_information_density: f64,      // default: 0.60, range: 0.30-0.90
    /// Whether to intervene when metacognitive ratio drops.
    pub auto_intervene: bool,              // default: true
    /// Intervention strategy: inject metacognitive prompt.
    pub intervention_prompt: String,
    // default: "Reflect: what patterns are you noticing about your own reasoning?"
}
```

---

## 8. Journal Storage

### File Layout

```
.roko/dreams/
  dream-1726367400000.json     # Individual cycle reports
  dream-1726371000000.json
  dream-1726378200000.json
  journal.jsonl                # Longitudinal journal entries (target)
```

### Rotation

Dream reports accumulate over the agent's lifetime. The existing resource
lifecycle system (bounded JSONL generations, disk-aware admission) applies
to dream reports. Old reports beyond a configurable retention window
(default: 100 reports or 30 days) can be archived through the cold storage
system (`roko knowledge archive`).

---

## 9. Academic Citations

| Paper | Concept Informed |
|-------|-----------------|
| Filevich et al. (2015), Journal of Neuroscience | Lucid dreaming: frontal metacognition applied to dream monitoring |
| Lin et al. (2025), sleep-time compute, arXiv:2504.13171 | Query predictability determines sleep-time effectiveness |
| "EEG Microstates" (bioRxiv 2025) | Microstates A/G dominate lucid REM; decreased C = heightened metacognition |
| "Electrophysiological Correlates of Lucid Dreaming" (J. Neuroscience 2025) | Gamma power increase at lucidity onset; alpha-gamma coupling |

---

## 10. Cross-References

| Document | Relevance |
|----------|-----------|
| [three-phase-cycle.md](three-phase-cycle.md) | Dream cycle producing the reports that journals aggregate |
| [scheduling-and-triggers.md](scheduling-and-triggers.md) | Trigger conditions recorded in journal entries |
| [consolidation-and-staging.md](consolidation-and-staging.md) | Hypothesis promotion metrics tracked in journals |
| [sleep-time-compute.md](sleep-time-compute.md) | Compute budget context for dream effectiveness analysis |
| [advanced-dream-concepts.md](advanced-dream-concepts.md) | Nightmare detection events tracked in journal entries |
