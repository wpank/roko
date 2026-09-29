# 08-learning/19 -- Hindsight Adjustments

> Append-only retrospective corrections for episode outcomes. When later
> evidence reveals that an earlier episode's success was fragile (regression)
> or that a failed episode's approach was sound (successful reuse), the
> hindsight relabeler emits immutable adjustment records without modifying
> the original episodes. Cite: Andrychowicz et al. (2017).

**Parent:** [08-LEARNING](../../08-LEARNING.md) section 6

**Source:** `crates/roko-learn/src/hindsight.rs` (`HindsightRelabeler`,
`EpisodeAdjustment`, `AdjustmentKind`)

**Academic basis:** Andrychowicz, M. et al. (2017). Hindsight Experience
Replay. *NeurIPS 2017*. (Goal relabeling for failed trajectories.)

---

## 1. Purpose

Standard episode logging records a binary outcome at completion time: the
agent passed or failed the gate pipeline. This contemporaneous assessment is
often wrong in retrospect:

| Situation | Contemporaneous | Retrospective |
|-----------|----------------|---------------|
| Code passes gates but introduces a regression discovered later | Success | Fragile success |
| Code fails gates but the approach is reused successfully later | Failure | Partially correct |
| A rule extracted from the episode is subsequently contradicted | Authoritative | Discredited |

Without hindsight adjustment, the learning system treats all successes as
equally durable and all failures as equally uninformative. This produces
two errors:

1. **False positives** -- skills and playbook rules mined from fragile
   successes accumulate incorrect knowledge.
2. **Discarded signal** -- failed episodes that contained partially correct
   approaches are thrown away entirely, losing useful learning signal.

Andrychowicz et al. (2017) demonstrated that relabeling failed trajectories
with achieved sub-goals (rather than the original goal) recovers useful
learning signal from at least 45% of otherwise-discarded episodes. Roko
applies this principle to software engineering episodes: a failed episode
that was later reused successfully is relabeled as a positive episode for a
smaller (achieved) goal.

---

## 2. Adjustment Kinds

The hindsight relabeler recognizes three categories of retrospective
correction, each corresponding to a distinct evidence pattern:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdjustmentKind {
    /// A later gate regression invalidated an earlier success.
    Regression,
    /// A later successful playbook reused an approach from a failed episode.
    SuccessfulReuse,
    /// A rule sourced from the episode was subsequently contradicted.
    HeuristicFalsified,
}
```

### 2.1 Regression

A successful episode is downgraded when a later episode on the same task or
overlapping file set fails a gate that the original passed. This indicates
the original success was fragile -- the code it produced was correct at
commit time but broke under subsequent changes.

Evidence pattern:
```
Episode A: success=true, files=[src/config.rs], timestamp=T
Episode B: success=false, files=[src/config.rs], timestamp=T+delta
    where B.gate_verdicts contains a failed gate
    -> Adjustment: A's success was fragile
```

### 2.2 SuccessfulReuse

A failed episode is upgraded when a later successful episode explicitly
reused its approach (referenced in the `extra.reused_episodes` field). This
is the direct application of Andrychowicz et al.'s goal relabeling: the
original episode failed to achieve its stated goal but succeeded at a
sub-goal that was later useful.

Evidence pattern:
```
Episode A: success=false, approach=X, timestamp=T
Episode B: success=true, extra.reused_episodes contains A.id, timestamp=T+delta
    -> Adjustment: A's approach was partially correct
```

### 2.3 HeuristicFalsified

An episode's contribution to the knowledge base is degraded when a playbook
rule sourced from that episode accumulates contradictions. The original
episode may have been genuinely successful, but the generalization extracted
from it was incorrect.

Evidence pattern:
```
Episode A: success=true, id=X
Rule R: source_episodes contains X, contradictions > 0
    -> Adjustment: lower confidence by 0.10
```

---

## 3. The EpisodeAdjustment Record

```rust
pub struct EpisodeAdjustment {
    /// Episode being corrected.
    pub original_episode_id: String,
    /// Correction category.
    pub adjustment_kind: AdjustmentKind,
    /// Original outcome or confidence value.
    pub old_value: Value,
    /// Corrected outcome or confidence value.
    pub new_value: Value,
    /// Auditable explanation.
    pub reason: String,
    /// Time the correction was inferred.
    pub timestamp: DateTime<Utc>,
}
```

### 3.1 Immutability Invariant

Adjustments are **append-only**. The original episode record is never
modified. Instead, adjustments are appended to the same JSONL log as typed
extension records with `kind = "episode_adjustment"`. This design preserves
the full audit trail:

```
Episode A (original):  { "id": "ep-001", "success": true, "kind": "episode", ... }
Adjustment (append):   { "id": "adj-001", "kind": "episode_adjustment",
                         "task_id": "ep-001",
                         "extra": { "adjustment": { ... } } }
```

Downstream consumers that need the corrected view must join episodes with
their adjustments. Consumers that only need the raw contemporaneous view
(diagnostics, raw statistics) read episodes directly without joining.

### 3.2 Append Pipeline

```rust
pub async fn append(
    &self,
    logger: &EpisodeLogger,
    adjustments: &[EpisodeAdjustment],
) -> Result<(), LoggerError> {
    for adjustment in adjustments {
        let mut record = Episode::new("hindsight", &adjustment.original_episode_id);
        record.kind = "episode_adjustment".into();
        record.timestamp = adjustment.timestamp;
        record.extra.insert(
            "adjustment".into(),
            serde_json::to_value(adjustment).unwrap_or(Value::Null),
        );
        logger.append(&record).await?;
    }
    Ok(())
}
```

The adjustment is serialized into the episode's `extra` map and written
through the standard `EpisodeLogger::append` path, inheriting the same
crash-safety guarantees (process-wide mutex, O_APPEND atomicity).

---

## 4. Scan Algorithm

The `HindsightRelabeler::scan` method cross-references recent episodes
against each other and against current playbook rule evidence:

```
For each episode E in the recent window (max_age = 30 days):
    Skip if E.kind == "episode_adjustment" (avoid circular corrections)

    If E.success == true:
        Search later episodes for regression evidence:
            Find any later episode L where:
                L.success == false
                AND L.timestamp >= E.timestamp
                AND (shared files OR same task_id)
                AND L has a failed gate verdict
            If found:
                Emit Regression adjustment for E
                Continue to next episode

    If E.success == false:
        Search later episodes for successful reuse:
            Find any later episode L where:
                L.success == true
                AND L.timestamp >= E.timestamp
                AND L.extra references E.id as reused
            If found:
                Emit SuccessfulReuse adjustment for E
                Continue to next episode

    Check rule contradictions:
        Find any playbook rule R where:
            R.contradictions > 0
            AND R.source_episodes contains E.id
        If found:
            Emit HeuristicFalsified adjustment for E
            (lower confidence by 0.10, floor at 0.0)
```

### 4.1 Staleness Boundary

The 30-day `max_age` limit prevents the scan from growing unboundedly with
the episode log. Episodes older than 30 days are considered settled -- any
regressions or reuse would have been detected within that window. This
bounds the scan to O(W^2) where W is the number of episodes in the 30-day
window (typically a few hundred), not the total log size.

### 4.2 File Overlap Detection

Regression detection uses file overlap as a proxy for code dependency:

```rust
fn shared_files(a: &HashSet<String>, b: &HashSet<String>) -> HashSet<String> {
    a.intersection(b).cloned().collect()
}
```

Two episodes that touch the same files are likely to interact. If the later
episode fails a gate, the earlier episode's success is suspect -- the code
it produced may have been invalidated by the later change.

---

## 5. Relationship to Andrychowicz et al. (2017)

Hindsight Experience Replay (HER) addresses a fundamental problem in
reinforcement learning: in sparse-reward environments, most trajectories
fail to reach the goal and produce zero reward, providing no useful gradient
signal.

HER's key insight is **goal relabeling**: instead of discarding a failed
trajectory, relabel it with a goal that *was* achieved during the
trajectory. The agent then learns from the successful-under-a-different-goal
experience.

### 5.1 Adaptation to Software Engineering

| HER Concept | Roko Adaptation |
|-------------|-----------------|
| Trajectory | Episode (agent turn through tool loop) |
| Goal | Task specification (files to modify, tests to pass) |
| Achieved sub-goal | Partial progress (approach reused later) |
| Relabeled experience | SuccessfulReuse adjustment |
| Sparse reward | Binary gate pass/fail |

The SuccessfulReuse adjustment is the closest analogue to HER: a failed
episode is relabeled as a positive experience for the sub-goal "produce a
reusable approach." The original episode failed its stated task but
succeeded at generating knowledge that was later useful.

### 5.2 Differences from HER

1. **Retrospective, not contemporaneous.** HER relabels at training time
   (immediately after the trajectory). Roko relabels retrospectively, when
   later evidence confirms that the approach was reused. This is more
   conservative: only approaches that were *actually* reused are relabeled,
   not all approaches that *could have* achieved a sub-goal.

2. **Append-only, not mutating.** HER replaces the trajectory's goal in the
   replay buffer. Roko appends an adjustment record without modifying the
   original episode. Both the original and corrected views are available.

3. **Cross-episode, not within-episode.** HER relabels within a single
   trajectory. Roko relabels across episodes: episode A fails, episode B
   (later) succeeds using A's approach.

---

## 6. Downstream Effects

### 6.1 Playbook Rule Confidence

When a HeuristicFalsified adjustment is emitted, the associated playbook
rule's confidence is decremented by 0.10:

```
old_confidence: 0.75
new_confidence: max(0.0, 0.75 - 0.10) = 0.65
```

Rules whose confidence drops below the `min_confidence` threshold (default
0.30) are automatically demoted: they stop being injected into agent
prompts. This creates a self-correcting cycle: rules that produce incorrect
generalizations are progressively weakened until they are no longer applied.

### 6.2 Skill Library

Skills mined from episodes that receive Regression adjustments are flagged
for re-evaluation. A skill whose source episode was retrospectively
downgraded may be teaching an approach that produces fragile code.

### 6.3 Cascade Router

SuccessfulReuse adjustments provide additional positive signal for the model
that produced the partially-correct episode. Even though the episode failed
its gate, the model demonstrated partial competence -- relevant for routing
decisions where partial success rates influence arm selection.

---

## 7. Configuration

```rust
pub struct HindsightRelabeler {
    max_age: Duration,  // default: 30 days
}
```

The 30-day max_age is the only configuration parameter. It is deliberately
not configurable through `roko.toml` because the scan's quadratic cost in
the window size makes very large windows impractical, and very small windows
miss most regression evidence.

---

## 8. Integration with LearningRuntime

The hindsight relabeler runs as part of the `LearningRuntime` batch update
cycle, after all per-episode updates are complete:

```
CompletedRunInput
    |
    +-- 1-10. Per-episode updates (episode logger, costs, rules, etc.)
    |
    +-- 11. HindsightRelabeler::scan(recent_episodes, current_rules)
    +-- 12. HindsightRelabeler::append(logger, adjustments)
```

Because the scan examines the full 30-day window on each invocation, it is
idempotent: running the scan twice produces the same adjustments (minus any
already appended). The `kind = "episode_adjustment"` filter prevents
adjustments from generating further adjustments.

---

## 9. Diagnostics

The `roko learn episodes` CLI command reports hindsight statistics:

```
Hindsight adjustments (last 30 days):
    Regressions:        12 episodes downgraded
    Successful reuses:   7 episodes upgraded
    Rules falsified:     3 rule-episode links weakened
    Signal recovered:   ~45% of failed episodes had reusable approaches
```

The 45% signal recovery rate aligns with Andrychowicz et al.'s finding that
nearly half of failed trajectories contain useful sub-goal achievements.

---

## References

- Andrychowicz, M. et al. (2017). Hindsight Experience Replay. *NeurIPS
  2017*.
- Schaul, T. et al. (2015). Universal Value Function Approximators. *ICML
  2015*.
- Shinn, N. et al. (2023). Reflexion: Language Agents with Verbal
  Reinforcement Learning. *NeurIPS 2023*.
