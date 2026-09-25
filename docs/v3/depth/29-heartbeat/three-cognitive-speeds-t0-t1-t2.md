# Three Cognitive Speeds: Gamma, Theta, Delta

> v3 depth file for chapter 29 (Heartbeat and Cognitive Loop).
> Source: v1/16-heartbeat/03-three-cognitive-speeds.md.
> Parent: `docs/v3/29-HEARTBEAT.md` SS4--7.

---

## 1. Abstract

Every Roko agent operates at three timescales simultaneously. In the two-fabric
model, the adaptive clock does not call the loops directly; `HeartbeatPolicy`
publishes `heartbeat.gamma.tick`, `heartbeat.theta.tick`, and
`heartbeat.delta.tick` Pulses on the Bus, and the speed-specific consumers
subscribe by topic. These are not sequential phases -- they are three concurrent
async consumers running in parallel, each processing information at a different
temporal grain. The naming follows EEG frequency bands: Gamma (fast reactive),
Theta (medium reflective), and Delta (slow consolidation).

The three-speed model draws from Friston's free energy principle (2010), which
frames perception as hierarchical prediction at different temporal grains. Clark
(2013) extends this into the "predictive brain" framework: biological cognition
is nested prediction loops at multiple timescales. Buzsaki's "Rhythms of the
Brain" (2006) establishes that oscillatory hierarchies in the brain enable
simultaneous processing at different temporal resolutions -- fast gamma
oscillations (30-100 Hz) ride on top of slower theta oscillations (4-8 Hz),
which ride on top of delta oscillations (0.5-4 Hz).

Kahneman's dual-process theory (2011, "Thinking, Fast and Slow") provides the
cognitive science grounding: System 1 (fast, automatic) handles the majority of
processing (Gamma T0), while System 2 (slow, deliberate) engages only when
System 1 detects something requiring attention (Gamma T1/T2). Roko extends this
two-tier model to three tiers with three timescales.

---

## 2. The Three Speeds

| Speed | Period | Name | What Happens | Trigger | Cost |
|---|---|---|---|---|---|
| **Gamma** | ~5-15s | Reactive | One complete loop tick. Tool calls, LLM inference, verification. | `heartbeat.gamma.tick` Pulse | T0: $0.00, T1: $0.001-0.003, T2: $0.01-0.25 |
| **Theta** | ~75s (30-120s) | Reflective | Summarize recent work. Update Daimon. Check predictions. Re-evaluate plan. | `heartbeat.theta.tick` Pulse, every N=5 gamma ticks or on episode completion | T1-T2: $0.01-0.10 |
| **Delta** | Hours (~50 theta cycles) | Consolidation | Dreams: replay, synthesis, pruning. Knowledge tier promotion. Playbook compilation. | `heartbeat.delta.tick` Pulse, on idle detection or scheduled | T0-T1: $0.00-0.01 |

### 2.1 Why Three Speeds, Not One

A single-speed architecture forces a painful tradeoff: either the agent ticks fast
enough to catch urgent events (expensive, ~2,000 ticks/day at ~$0.10/tick = $200/day)
or it ticks slowly enough to be cheap (missing time-sensitive signals).

The three-speed model resolves this:
- **Gamma** handles urgent reactive tasks -- what is happening RIGHT NOW? Most ticks
  suppress at T0 ($0.00), making high-frequency perception affordable.
- **Theta** handles strategic reflection -- am I on the right track? This fires less
  frequently but with more cognitive depth.
- **Delta** handles consolidation -- what have I learned? This fires during idle time
  at minimal cost.

Total daily cost with three speeds: ~$2-50 (depending on domain volatility), versus
$100-500+ with a single-speed always-on approach.

### 2.2 Concurrency Model

All three speeds consume their own Bus topics and share state through
`Arc<RwLock<AgentState>>`:

```rust
// Conceptual structure (target architecture)
let gamma_ticks = bus.subscribe("heartbeat.gamma.tick");
let theta_ticks = bus.subscribe("heartbeat.theta.tick");
let delta_ticks = bus.subscribe("heartbeat.delta.tick");
```

The control relationship is inverted from a special orchestration loop: the
`HeartbeatPolicy` emits the tick Pulses, and each speed reacts to its topic.
Gamma still has priority in the shared state layer, but that priority comes
from cadence and queueing, not from one loop calling another.

---

## 3. Gamma: The Heartbeat (~5-15s)

Gamma is the high-frequency consumer -- what most agent frameworks call "the agent."
Every `heartbeat.gamma.tick` Pulse runs the canonical seven-step loop:

1. **SENSE**: Run T0 probes, read Substrate, drain relevant Bus topics.
2. **ASSESS**: Score candidates, route toward T0/T1/T2.
3. **COMPOSE**: Assemble context when the tick escalates past T0.
4. **ACT**: Call the model, tool, or domain action.
5. **VERIFY**: Run gates and compare against ground truth.
6. **PERSIST + BROADCAST**: Store durable outputs and publish live Pulses.
7. **REACT**: Let policies, Daimon bias, and other cross-cuts respond.

**Adaptive interval**: Gamma accelerates when the environment is volatile (more
anomalies -> faster ticks, down to 5s) and slows when calm (fewer anomalies ->
slower ticks, up to 15s).

```rust
fn compute_gamma_interval(violations: &[Violation]) -> Duration {
    Duration::from_secs(15)
        .mul_f64(1.0 / (1.0 + violations.len() as f64 * 0.3))
        .max(Duration::from_secs(5))
}
```

**Cost structure**: ~80% of gamma ticks suppress at T0 ($0.00). ~15% escalate to
T1 ($0.001-0.003). ~5% reach T2 ($0.01-0.25).

---

## 4. Theta: The Breath (~75s)

Theta consumes `heartbeat.theta.tick` Pulses -- every 5 gamma cycles or upon
episode completion. Named after EEG theta oscillations (4-8 Hz) associated with
navigation, memory encoding, and hippocampal indexing (Buzsaki 2006).

**Five phases:**

1. **Summarize recent gamma work**: Outcome distribution, anomaly patterns, action
   patterns, cost accumulation.
2. **Update Daimon state**: Aggregate affect at the ALMA mood layer (Gebhard 2005).
   Three-layer model: Emotion (seconds), Mood (hours), Personality (permanent).
3. **Check predictions**: CalibrationTracker aggregates prediction residuals per
   (model, task_category) pair.
4. **Re-evaluate plan**: LLM reasons about plan validity, progress, failure patterns.
5. **Trigger interventions**: Stuck detection, cost anomaly, calibration collapse,
   complacency detection.

**Adaptive interval**: Regime-based multipliers:

| Regime | Multiplier | Theta Interval | Ticks/Hour |
|---|---|---|---|
| Calm | 1.6x | 120s | 30 |
| Normal | 1.0x | 75s | 48 |
| Volatile | 0.4x | 30s | 120 |
| Crisis | 0.2x | 15s | 240 |

Theta always invokes at least T1 -- it needs LLM reasoning to reflect.

---

## 5. Delta: Sleep (~Hours)

Delta consumes `heartbeat.delta.tick` Pulses during agent idle time. It is the
agent's offline learning phase, corresponding to biological slow-wave sleep
(McClelland et al. 1995).

**Trigger conditions:**
1. Idle detection: No active tasks for >5 minutes.
2. Scheduled time: Nightly consolidation cron.
3. Episode count threshold: ~50 episodes since last delta.
4. Explicit command: `roko dream run`.

**Three-phase dream cycle:**
- Phase 1: NREM Replay (8-15 min) -- Mattar-Daw utility formula.
- Phase 2: REM Imagination (5-15 min) -- Boden's creativity modes.
- Phase 3: Integration & Staging (5-10 min) -- staging buffer at 0.20-0.30 confidence.

**Non-blocking**: If a new task arrives during delta, the dream pauses via
`CognitiveSignal::Pause`, state serializes to disk, gamma takes over immediately.

---

## 6. The Hierarchy: Gamma Rides on Theta Rides on Delta

```
Delta (~hours):    +--------------------------------------------------+
                   |  Consolidation: dreams, tier promotion, playbook  |
                   |  Fires: ~50 theta cycles or on idle              |
                   +--------------------------------------------------+
                        | contains ~50 theta cycles
                        v
Theta (~75s):      +------+------+------+------+------+
                   | Refl | Refl | Refl | Refl | Refl | ...
                   +------+------+------+------+------+
                        | each contains ~5 gamma cycles
                        v
Gamma (~10s):      +--+--+--+--+--+--+--+--+--+--+
                   |T0|T0|T1|T0|T0|T0|T0|T2|T0|T0| ...
                   +--+--+--+--+--+--+--+--+--+--+
                   80%     15%           5%
                   free    cheap         full
```

Information flows bidirectionally:
- **Upward**: gamma produces tick observations; theta summarizes them into patterns;
  delta consolidates patterns into durable knowledge.
- **Downward**: delta knowledge improves gamma perception; theta adjustments change
  gamma behavior.

---

## 7. Mapping to Existing Code

| Speed | Primitives | Learning | Compose |
|---|---|---|---|
| Gamma | `InferenceTier` T0/T1/T2 | `CascadeRouter` tier selection | `ContextTier::Surgical/Focused/Full` |
| Theta | -- | `episode_logger` episode boundaries | -- |
| Delta | -- | `PatternMiner`, `KMedoids`, `baseline` | -- |

The `InferenceTier` enum in `crates/roko-primitives/src/tier.rs` directly
corresponds to the T0/T1/T2 gating within gamma ticks. The `CascadeRouter` in
`crates/roko-learn/src/cascade_router.rs` implements the three-stage routing
that drives tier selection.

---

## 8. References

- **Buzsaki 2006** -- "Rhythms of the Brain" (Oxford University Press). Oscillatory
  hierarchies in biological cognition.
- **Kahneman 2011** -- "Thinking, Fast and Slow" (Farrar, Straus and Giroux).
  System 1 / System 2 dual-process theory.
- **Friston 2010** -- "The Free-Energy Principle" (Nature Reviews Neuroscience
  11(2)). Hierarchical prediction at different temporal grains.
- **Clark 2013** -- "Whatever Next?" (Behavioral and Brain Sciences 36(3)).
  Predictive processing at multiple timescales.
- **Sumers et al. 2023** -- CoALA framework (arXiv:2309.02427).
- **Chen et al. 2023** -- FrugalGPT (arXiv:2305.05176).
- **McClelland et al. 1995** -- Complementary Learning Systems (Psychological
  Review 102(3)).
- **Gebhard 2005** -- "ALMA: A Layered Model of Affect" (AAMAS 2005).

---

## Cross-References

- `docs/v3/depth/29-heartbeat/gamma-reactive-loop.md` -- Gamma details
- `docs/v3/depth/29-heartbeat/theta-reflective-loop.md` -- Theta details
- `docs/v3/depth/29-heartbeat/delta-consolidation-loop.md` -- Delta details
- `docs/v3/depth/29-heartbeat/adaptive-clock.md` -- Clock managing all three
- `docs/v3/29-HEARTBEAT.md` -- Parent chapter
