# 08-learning/15 -- Stability Mechanisms

> Without stability mechanisms, eight concurrent feedback loops oscillate
> rather than converge. Three mechanisms -- hysteresis, frequency
> separation, and EMA damping -- prevent this oscillation and are
> prerequisites for the compound improvement described in the autocatalytic
> thesis.

**Parent:** [08-LEARNING](../../08-LEARNING.md) section 11

**Theoretical basis:** Control theory (hysteresis, frequency separation,
damping), Ashby's Law of Requisite Variety (Ashby 1956), Beer's Viable
System Model (Beer 1972)

**Source:** `crates/roko-learn/src/cascade_router.rs` (hysteresis, alpha
decay), `crates/roko-learn/src/cfactor.rs` (C-Factor EMA, regression
detection), `crates/roko-gate/src/adaptive.rs` (gate threshold EMA)

---

## 1. Purpose

A system with eight feedback loops operating simultaneously can oscillate:
loop 1 routes away from a provider, loop 6 routes back because the
alternative is more expensive, loop 7 routes away again because the
alternative is slower, and the system thrashes between options without
settling. Stability mechanisms prevent this oscillation by introducing
damping, hysteresis, and frequency separation.

These mechanisms are not an optimization -- they are a **prerequisite**.
Without them, the compound improvement described in the autocatalytic thesis
cannot occur because the system spends its energy oscillating rather than
converging.

---

## 2. Hysteresis

### 2.1 Definition

Hysteresis introduces a switching threshold: the system only changes its
decision when the new option is **sufficiently better** than the current
option. Small improvements are ignored, preventing rapid oscillation
between near-equal alternatives.

### 2.2 Cascade Router Hysteresis

The cascade router uses a 10% score delta threshold for model switching:

```
Current model: claude-sonnet-4 (score: 0.82)
Challenger: claude-opus-4 (score: 0.85)

Delta: 0.85 - 0.82 = 0.03
Hysteresis threshold: 0.10

0.03 < 0.10 -> Keep current model (no switch)
```

The system only switches when the challenger's score exceeds the current
model's score by at least 10%.

### 2.3 Threshold Selection

The 10% threshold balances responsiveness and stability:

| Threshold | Behavior |
|-----------|----------|
| 1% | Near-zero hysteresis -- switches on noise |
| 5% | Low hysteresis -- switches on moderate improvements |
| **10%** | Moderate hysteresis -- switches on meaningful improvements |
| 20% | High hysteresis -- misses genuine improvements |
| 50% | Extreme -- never switches except for dramatic changes |

Why 10%:
- Typical model performance differences are 5-15% in pass rate
- Cost differences between tiers are 5-10x (much larger than 10%)
- A 10% improvement in the composite score represents a genuine, actionable
  improvement

### 2.4 Hysteresis in Other Subsystems

| Subsystem | Hysteresis Mechanism |
|-----------|---------------------|
| Playbook rules | Confidence must cross `min_confidence` to prune (not oscillate near threshold) |
| Circuit breaker | Half-open requires a successful probe before closing (not just cooldown expiry) |
| Adaptive thresholds | EMA smoothing prevents threshold oscillation from batch-to-batch noise |
| Pattern discovery | `min_support` threshold prevents low-confidence patterns from promotion |

---

## 3. Frequency Separation

### 3.1 Definition

Frequency separation assigns different update rates to subsystems based on
their characteristic timescales. Fast subsystems (model routing) update
every episode. Slow subsystems (pattern discovery) update every 20 episodes.
This prevents fast loops from reacting to signals that have not been
confirmed by slow loops.

### 3.2 Update Frequency Hierarchy

```
+--- Every episode -------------------------------------------+
|  Cascade router:     update bandit arms                      |
|  Episode logger:     append episode                          |
|  Cost log:           append cost record                      |
|  Provider health:    update circuit breaker                   |
|  Anomaly detector:   check prompt loop, cost spike           |
+--------------------------------------------------------------+
                        |
+--- Every 5 episodes ----------------------------------------+
|  Gate thresholds:    EMA update of adaptive thresholds        |
|  Regression check:   compare current vs baseline             |
|  Efficiency grading: update section effectiveness             |
+--------------------------------------------------------------+
                        |
+--- Every 20 episodes ----------------------------------------+
|  Pattern discovery:  trigram mining, pattern extraction       |
|  Skill extraction:   Voyager-style skill mining              |
|  Cross-episode:      HDC clustering consolidation            |
+--------------------------------------------------------------+
                        |
+--- Every 50 episodes ----------------------------------------+
|  Pareto frontier:    recompute Pareto-optimal models         |
|  C-Factor:           recompute collective capability         |
+--------------------------------------------------------------+
```

### 3.3 Frequency Rationale

| Subsystem | Frequency | Rationale |
|-----------|-----------|-----------|
| Cascade router | Every 1 | Routing decisions benefit from immediate feedback |
| Gate thresholds | Every 5 | Thresholds need multiple data points to avoid noise |
| Pattern discovery | Every 20 | Patterns need a statistically meaningful sample |
| Pareto frontier | Every 50 | Model statistics need many observations for stable estimates |

The frequencies are chosen so that each subsystem has enough observations
to make a reliable update at its cadence. A subsystem that updates too
frequently produces noisy decisions; one that updates too infrequently
misses genuine changes.

### 3.4 Information Cascade

The frequency hierarchy creates a natural information cascade:

1. **Per-episode** data flows into fast subsystems (routing, health).
2. **Aggregated** data (5-episode windows) flows into medium subsystems
   (thresholds, regression).
3. **Consolidated** data (20-episode batches) flows into slow subsystems
   (patterns, skills).
4. **Summary** data (50-episode summaries) flows into the slowest
   subsystems (Pareto, C-Factor).

Each level receives data that has already been filtered and stabilized by
the level above it. Fast oscillations in routing decisions are invisible
to pattern discovery, which only sees the 20-episode trend.

---

## 4. Damping (EMA Smoothing)

### 4.1 Definition

Exponential Moving Average (EMA) smoothing damps oscillation in
continuously-valued quantities:

```
ema_new = alpha * observation + (1 - alpha) * ema_old
```

where alpha in (0, 1) controls the smoothing rate. Small alpha means heavy
smoothing (slow response). Large alpha means light smoothing (fast
response).

### 4.2 EMA Parameters Across Subsystems

| Subsystem | alpha | Behavior |
|-----------|-------|----------|
| Gate thresholds | 0.1 | Heavy smoothing -- thresholds change slowly |
| Cost EWMA | 0.2 | Moderate smoothing -- cost baseline adapts over ~5 observations |
| Latency EMA | 0.1 | Heavy smoothing -- latency baseline is conservative |
| LinUCB alpha decay | exp(-obs/60) | Exponential decay -- exploration decreases gradually |

### 4.3 Why Not Moving Average?

Simple moving averages (mean of last N values) have a discontinuity problem:
when an old value exits the window, the average can jump even without new
data. EMA avoids this by weighting all past observations with exponentially
decaying weights. The result is a smooth, continuous signal that responds
proportionally to the magnitude of new observations.

### 4.4 EMA for Gate Thresholds

Gate thresholds are the most stability-critical use of EMA. A gate that
oscillates between pass and fail on the same input destabilizes the entire
learning pipeline. The alpha = 0.1 EMA means the threshold reflects a
weighted average of the last ~10 observations with exponentially decaying
influence:

```
After 1 observation:  threshold = 0.1 * obs + 0.9 * old
After 5 observations: oldest weight = 0.9^5 = 0.59
After 10 observations: oldest weight = 0.9^10 = 0.35
After 20 observations: oldest weight = 0.9^20 = 0.12
```

---

## 5. Compound Stability

The interaction of hysteresis, frequency separation, and EMA smoothing
creates compound stability:

1. **Hysteresis** prevents switching on noise.
2. **Frequency separation** prevents fast loops from disrupting slow loops.
3. **EMA smoothing** prevents continuous quantities from oscillating.

Together, these mechanisms ensure that the eight feedback loops converge
to a stable operating point rather than oscillating. The system "locks in"
to good configurations and only moves when there is strong evidence for
improvement.

### 5.1 Stability Budget

Each feedback loop has a "stability budget": the amount of perturbation
it can absorb without oscillating. The hysteresis threshold, update
frequency, and EMA alpha collectively determine this budget.

| Loop | Stability Budget | Characteristic |
|------|-----------------|----------------|
| Routing decisions | Small | Low cost to change, responsive |
| Gate thresholds | Medium | Moderate cost to change |
| Pattern discovery | Large | High cost to change (wrong rules degrade all future agents) |
| Pareto frontier | Large | High cost to change (wrong model set degrades all routing) |

The design ensures that stability budgets increase with the severity of
the action: routing decisions (low cost to change) have small stability
budgets, while pattern promotion (high cost to change) has large stability
budgets.

---

## 6. Anti-Patterns: Positive Feedback Loops

Stability mechanisms are designed to prevent positive feedback loops --
self-reinforcing cycles that drive the system to extremes:

| Anti-pattern | What happens | Prevention |
|-------------|-------------|------------|
| Model lock-in | Bandit exploits one model so heavily that alternatives never get enough data to compete | UCB exploration term, alpha decay |
| Playbook explosion | Rules accumulate without pruning, consuming entire prompt budget | Confidence decay, min_confidence threshold |
| Cost death spiral | Budget pressure forces cheap models -> failures -> more iterations -> higher cost | Per-task budget limit, hard stop |
| Threshold collapse | Adaptive thresholds relax so far that gates are meaningless | Floor on threshold values |

Each anti-pattern has a specific stability mechanism that prevents it. The
compound effect is that the system remains in its "viable region" (Beer's
VSM) -- operating within the bounds where all feedback loops function
correctly.

### 6.1 Model Lock-In

Without exploration, the bandit can lock into a single model because it
never samples alternatives. The UCB1 exploration term and LinUCB alpha
decay provide forced exploration:

```
ucb(a) = mean_a + C * sqrt(ln(N) / pulls_a)
```

Arms with few pulls have large UCB values, ensuring they are eventually
sampled even if their estimated mean is low.

### 6.2 Cost Death Spiral

Budget pressure (loop 6) can force the router to use cheap models, which
fail more often, consuming more iterations, which increases cost, which
increases budget pressure. The per-task budget limit breaks this spiral:
when a single task has consumed its budget, it is skipped rather than
allowing the iteration count to grow without bound.

---

## 7. Theoretical Foundation

### 7.1 Ashby's Law of Requisite Variety

A control system must have at least as much variety as the system it
controls. Roko's stability mechanisms provide a different damping mechanism
for each type of oscillation:

| Oscillation Type | Required Variety | Mechanism |
|-----------------|-----------------|-----------|
| Binary switching (model A vs B) | Two states + threshold | Hysteresis |
| Continuous drift (parameter values) | Continuous damping | EMA smoothing |
| Multi-rate interference | Frequency isolation | Frequency separation |
| Degenerate convergence (all traffic to one arm) | Forced exploration | UCB exploration term |

### 7.2 Beer's Viable System Model

Beer's VSM defines five systems required for organizational viability.
Stability mechanisms primarily implement Systems 2 (coordination between
subsystems) and 3 (control over aggregate behavior):

| VSM System | Function | Roko Implementation |
|-----------|----------|---------------------|
| System 1 | Operations | Individual learning subsystems |
| System 2 | Coordination | Frequency separation, LearningRuntime ordering |
| System 3 | Control | Regression detection, C-Factor monitoring |
| System 4 | Intelligence | Pattern discovery, predictive foraging |
| System 5 | Policy | Hysteresis thresholds, EMA parameters |

### 7.3 Good Regulator Theorem

A system that is a good regulator of another system must be a model of
that system. The C-Factor is Roko's model of its own health -- it captures
key performance indicators in a single composite score. The regression
detector uses this model to identify deviations from expected behavior and
trigger corrective actions.

---

## References

- Ashby, W.R. (1956). *An Introduction to Cybernetics*. Chapman & Hall.
- Beer, S. (1972). *Brain of the Firm*. Allen Lane.
- Auer, P., Cesa-Bianchi, N. & Fischer, P. (2002). Finite-time analysis
  of the multiarmed bandit problem. *Machine Learning* 47(2-3), 235-256.
- Li, L. et al. (2010). A Contextual-Bandit Approach to Personalized
  News Article Recommendation. *WWW 2010*.
