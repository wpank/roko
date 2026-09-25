# Dream Scheduling and Triggers

> **v3 depth file** -- `/docs/v3/depth/10-dreams/scheduling-and-triggers.md`
> Canonical source: v1 `docs/v1/10-dreams/13-scheduling-and-triggers.md`
> Implementation: `crates/roko-dreams/src/runner.rs`
> Status: **Wired** -- adaptive idle, seven-field cron, episode-count triggers,
> manual `roko knowledge dream run`, quality-adaptive delay multipliers, and
> checkpoint restore are live. Bus-triggered wakeup and intensive backlog
> draining remain future work.

---

## 1. Trigger Conditions

Dreams in daemon mode are triggered by adaptive idle, optional cron, and
optional episode-count policies; manual execution is also available. All
automatic paths respect the idle boundary. The two-fabric Bus wakeup
described later in this document remains a target, not current runtime
behavior.

### 1.1 Idle-Time Trigger (Primary)

The primary trigger fires when three conditions are simultaneously met:

| Condition | Default | Configuration |
|-----------|---------|---------------|
| Agent has no active tasks | -- | Detected by the plan executor / orchestration layer |
| Agent has been idle for >= threshold | 15 minutes | `dreams.idle_threshold_mins` in `roko.toml` |
| Agent has >= minimum unprocessed episodes | 5 episodes | `dreams.min_episodes_for_dream` in `roko.toml` |

The idle detection logic from `DreamRunner::schedule()`:

```rust
pub fn schedule(&self) -> Option<Duration> {
    if !self.config.auto_dream {
        return None;
    }

    let episodes = load_episodes_since_last_dream();
    if episodes.len() < self.config.min_episodes_for_dream {
        return None;
    }

    let latest_episode = episodes.iter().map(|e| e.timestamp).max()?;
    let idle_threshold = Duration::from_secs(
        self.config.idle_threshold_mins * 60
    );
    let target_fire_at = latest_episode + idle_threshold;
    let now = Utc::now();

    if target_fire_at <= now {
        Some(Duration::ZERO)  // Dream now
    } else {
        (target_fire_at - now).to_std().ok()
    }
}
```

### 1.2 Scheduled Trigger (Secondary)

The scheduled trigger is the fallback cadence. In the two-fabric model,
Delta-speed consolidation is usually Pulse-triggered: the dream runner
subscribes to `substrate.engram.stored` and wakes when enough durable
Signals have landed to justify a consolidation batch. Fixed intervals remain
as a safety net for deployments where notifications are delayed or the Bus
is temporarily unavailable:

```toml
[dreams]
scheduled_cron = "0 0 */4 * * * *"  # seven-field expression, every four hours
```

When the scheduled trigger fires during active task execution, the dream is
queued and executed at the next available idle gap. Dreams never interrupt
active tasks, and Pulse-triggered Delta wakeups still respect the same idle
boundary before they run.

### 1.3 Episode-Count Trigger

When the unconsolidated episode count exceeds a configurable threshold, a
dream cycle is scheduled regardless of idle time:

```toml
[dreams]
episode_count_trigger = 50  # fire once this many unconsolidated episodes exist (0 = disabled)
```

This prevents unbounded consolidation debt when the agent is continuously
busy. The trigger fires once at the threshold; back-to-back intensive
draining remains future work.

### 1.4 Manual Trigger

The CLI provides a manual dream trigger for testing and development:

```bash
roko knowledge dream run       # Fire a dream cycle now
roko knowledge dream report    # Show the latest dream report
roko knowledge dream schedule  # Inspect the next automatic deadline
```

---

## 2. What Does NOT Trigger Dreams

These are mechanisms from the legacy Bardo architecture that are **removed**
in Roko:

| Legacy Trigger | Why Removed |
|----------------|-------------|
| Death-clock proximity | Death-clock logic is retired. Dreams fire based on idle time and backlog, not end-of-life framing. |
| Vitality score thresholds | Vitality phases are retired. Budget exhaustion and knowledge plateau are continuous metrics, not dream triggers. |
| Terminal-phase frantic dreaming | Terminal-phase dreaming is retired. Large backlogs trigger the episode-count mechanism instead. |

---

## 3. Dream Frequency Adaptation

Dream frequency adapts to the agent's operational state:

| State | Dream Frequency | Mechanism |
|-------|----------------|-----------|
| Low activity, few episodes | Infrequent (1/day or less) | `min_episodes_for_dream` threshold not met |
| Normal activity | 2-4 per day | Standard idle gaps between tasks |
| High activity, many episodes | 4-8 per day | More episodes accumulate faster, reaching threshold sooner |
| Very high activity, no idle time | Scheduled only | Idle trigger never fires; scheduled trigger ensures periodic consolidation |
| Large backlog | Episode-count trigger | One cycle is scheduled at the configured threshold |

### Quality-Adaptive Delay Multipliers

After each dream cycle, the scheduler adjusts the next idle-threshold delay
based on the quality of the cycle's output:

```toml
[dreams]
quality_gain = 0.75    # after a productive dream, shorten next delay
quality_penalty = 1.25 # after a low-quality dream, lengthen next delay
```

A productive dream (many insights staged, high hypothesis diversity) shortens
the next delay by 25%, encouraging the agent to dream again sooner when
dreaming is proving valuable. A low-quality dream (few insights, low
diversity) lengthens the delay by 25%, preventing the agent from wasting
compute on unproductive dreams.

### Intensive Consolidation Mode

> **Status:** Proposed; not implemented by the current resident scheduler.

When the unprocessed episode count exceeds a high-water mark (default: 50
episodes), the proposed dream scheduler enters intensive mode:

1. Dream cycles fire back-to-back until the backlog is reduced to the
   low-water mark (default: 10 episodes)
2. Each cycle processes a batch of episodes (default: 10 per cycle)
3. Intensive mode is logged as a separate category for monitoring

This replaces the legacy "frantic dreaming" concept: instead of dreaming
intensely because death is approaching, the agent dreams intensely because
it has a lot of material to process. The motivation is cognitive, not mortal.

---

## 4. Dream Outputs: The Two-Fabric Model

The trigger side is only half of the story. When a dream cycle runs, it
writes durable consolidation results and also emits live promotion Pulses:

| Output | Fabric | Purpose |
|--------|--------|---------|
| Consolidated `Kind::Insight` / `Kind::Heuristic` Signals | Substrate | Persist durable dream results with lineage |
| `engram.promoted` Pulse | Bus | Notify subscribers that a durable Signal graduated (target-state) |
| `neuro.insight.promoted` Pulse | Bus | Wake Neuro and Compose refresh paths (target-state) |

This means Dreams stay complete on the durable side and reactive on the live
side. Delta-speed does not poll for its own downstream effects any more than
it polls for its wakeup conditions.

---

## 5. Interaction with the Plan Executor

The dream scheduler coordinates with the plan execution layer to find
appropriate idle windows:

```
Plan Executor                    Dream Scheduler
    |                                  |
    |-- Task A starts ----------------->|  (blocked: active task)
    |                                  |
    |-- Task A completes -------------->|
    |                                  |
    |-- Check for idle gap ------------>|
    |                                  |-- idle_threshold not met yet
    |                                  |
    |-- Task B starts ----------------->|  (blocked: active task)
    |                                  |
    |-- Task B completes -------------->|
    |                                  |
    |-- Check for idle gap ------------>|
    |                                  |-- idle_threshold met, episodes >= min
    |                                  |-- DREAM CYCLE FIRES
    |                                  |
    |<- Dream complete, resume ---------|
    |                                  |
    |-- Task C starts ----------------->|
```

The daemon starts a resident scheduler that observes active work, episode
growth, elapsed idle time, and cron deadlines. It queues one pending cron
fire while work is active and runs it at the next idle boundary.

---

## 6. Circadian-Inspired Scheduling

Biological sleep follows circadian rhythms -- not purely reactive. Roko
agents operating on long-running tasks benefit from a circadian-like
scheduling pattern that ensures dream cycles happen at regular intervals
even when idle gaps are plentiful.

The circadian scheduler layers on top of the idle-time and scheduled triggers.
It does not replace them -- it biases the timing of dreams toward preferred
hours while still respecting the `min_episodes_for_dream` and
`idle_threshold_mins` constraints.

```rust
/// Circadian-inspired dream scheduling.
pub struct CircadianScheduler {
    /// Preferred dream times (hours of day, 0-23).
    pub preferred_hours: Vec<u8>,         // default: [2, 6, 14, 22]
    /// Circadian strength: how strongly preferred hours bias scheduling.
    /// 0.0 = no bias, 1.0 = only dream during preferred hours.
    pub circadian_strength: f64,          // default: 0.3, range: 0.0-1.0
    /// Minimum interval between dream cycles (minutes).
    pub min_interval_mins: u64,           // default: 60, range: 30-480
    /// Maximum interval between dream cycles (minutes).
    pub max_interval_mins: u64,           // default: 360, range: 120-720
    /// Whether to align dream cycles with task completion boundaries.
    pub align_to_task_boundaries: bool,   // default: true
}
```

Key behaviors:
- When `circadian_strength = 0.0`, preferred hours have no effect
- When `circadian_strength = 1.0`, dreams only fire during preferred hours
- `max_interval_mins` acts as a safety net: even a continuously busy agent
  will dream within this interval to prevent unbounded consolidation debt

### Test Criteria

```
1. Circadian preference: with circadian_strength=1.0, dreams only fire during preferred_hours.
2. Min interval: no two dream cycles fire within min_interval_mins of each other.
3. Max interval: a dream cycle always fires within max_interval_mins, even without idle time.
4. Task alignment: with align_to_task_boundaries=true, dreams never interrupt a running task.
5. Zero strength: with circadian_strength=0.0, preferred_hours has no effect on scheduling.
```

---

## 7. Configuration Reference

```toml
[dreams]
# Enable automatic idle-triggered dreaming
auto_dream = true

# Minutes of inactivity before a dream can fire
idle_threshold_mins = 15

# Minimum unprocessed episodes required before a dream can fire
min_episodes_for_dream = 5

# Optional seven-field cron cadence (omit to disable)
scheduled_cron = "0 0 */4 * * * *"

# Fire once this many unconsolidated episodes exist (0 = disabled)
episode_count_trigger = 50

# Adaptive idle-delay multipliers after high/low-quality reports
quality_gain = 0.75
quality_penalty = 1.25

# Fraction of inference budget allocated to dreams
budget_fraction = 0.15

# Episodes processed per dream cycle
batch_size = 10

[dreams.agent]
# Agent backend for dream consolidation
command = "claude"
model = "claude-haiku-4-5-20251001"
bare_mode = true
effort = "low"
timeout_ms = 120000
```

---

## 8. Cross-References

| Document | Relevance |
|----------|-----------|
| [three-phase-cycle.md](three-phase-cycle.md) | Dream cycle structure that scheduling triggers |
| [sleep-time-compute.md](sleep-time-compute.md) | Compute budget that constrains dream frequency |
| [cross-system-integration.md](cross-system-integration.md) | Plan executor coordination with dream scheduler |
| [dream-journals.md](dream-journals.md) | Journal entries record trigger source per cycle |
