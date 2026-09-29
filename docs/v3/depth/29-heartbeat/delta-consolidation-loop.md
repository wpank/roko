# Delta: The Consolidation Loop (~Hours)

> v3 depth file for chapter 29 (Heartbeat and Cognitive Loop).
> Source: v1/16-heartbeat/06-delta-consolidation-loop.md.
> Parent: `docs/v3/29-HEARTBEAT.md` SS7.

---

## 1. Abstract

Delta is sleep. When the agent has nothing to do -- no active tasks, no pending
observations, no urgent gamma work -- it enters delta consolidation: the offline
learning phase that turns individual experiences into durable knowledge.

Delta corresponds to biological slow-wave sleep (0.5-4 Hz delta oscillations),
where the brain consolidates episodic memories into semantic knowledge, prunes
irrelevant connections, and generates novel hypotheses through dream replay
(McClelland et al. 1995, "Why there are complementary learning systems in the
hippocampus and neocortex", Psychological Review 102(3)).

Delta is where the agent gets smarter without doing any task work. It replays
prioritized episodes using the Mattar-Daw utility formula, generates counterfactual
scenarios, promotes knowledge across tiers, compiles playbook rules, optimizes model
routing, and performs meta-cognition.

Critically, delta is **non-blocking**. If a new task arrives during delta processing,
the dream pauses via `CognitiveSignal::Pause`, the dream state serializes to disk,
and gamma takes over immediately.

---

## 2. Trigger Conditions

```rust
fn should_enter_delta(
    idle_duration: Duration,
    episodes_since_last_delta: usize,
    scheduled_delta_time: Option<SystemTime>,
    explicit_trigger: bool,
) -> bool {
    explicit_trigger
        || idle_duration > Duration::from_secs(300)  // 5 minutes idle
        || episodes_since_last_delta >= 50
        || scheduled_delta_time.map_or(false, |t| SystemTime::now() >= t)
}
```

1. **Idle detection**: No active tasks for >5 minutes.
2. **Scheduled time**: Nightly consolidation cron.
3. **Episode count threshold**: ~50 episodes since last delta.
4. **Explicit command**: `roko dream run`.

---

## 3. Three-Phase Dream Cycle

### 3.1 Phase 1: NREM Replay (8-15 min)

Prioritized episode review using the Mattar-Daw utility formula (Mattar & Daw 2018):

```
Utility(episode) = Gain * Need * (1 / spacing_penalty)

where:
  Gain     = magnitude of prediction error
  Need     = how frequently similar situations arise
  spacing  = time since last replay (Ebbinghaus spacing effect)
```

**Perturbed replay (30%)**: 30% of replays inject perturbations -- simulated
adversarial conditions for robustness testing.

**PAD modulation**: When affect shows anxiety (low pleasure, high arousal), warning
episodes receive a 2x weight multiplier.

Model: Haiku-class (T1). Cost: $0.02-0.12.

### 3.2 Phase 2: REM Imagination (5-15 min)

Counterfactual generation via Boden's three creativity modes (Boden 2004):

1. **Combinational creativity**: Recombine existing knowledge fragments into
   novel combinations.
2. **Exploratory creativity**: Traverse the boundaries of the known strategy space.
3. **Transformational creativity**: Break constraints to discover entirely new regions.

Implemented via Pearl's structural causal models (Pearl 2009). The agent builds a
causal graph and generates counterfactuals by intervening on causal variables.

**Emotional depotentiation**: During REM, emotional charge on highly charged memories
is reduced by 0.3-0.5 per cycle (Walker & van der Helm 2009). The factual content is
preserved; the emotional intensity is gradually reduced.

**HDC counterfactual synthesis**: Using HDC permutation operations (Kanerva 2009),
the agent generates novel knowledge combinations in nanoseconds.

Model: Sonnet-class (T1-T2). Cost: $0.05-0.20.

### 3.3 Phase 3: Integration & Staging (5-10 min)

Dream outputs enter a staging buffer at 0.20-0.30 confidence. Nothing generated
during dreams is immediately trusted.

```
Dream output (confidence 0.20-0.30)
    | staging buffer
Live validation during subsequent gamma/theta ticks
    | if validated, confidence rises
Working memory at 0.50 confidence
    | continued validation
Consolidated memory at 0.70 confidence
    | extensive validation
Persistent memory at 0.90 confidence
```

Cost: $0.00 (pure computation).

---

## 4. Knowledge Tier Promotion

| Tier | Decay Multiplier | Promotion Threshold |
|---|---|---|
| Transient | 0.1x base | (initial state) |
| Working | 0.5x base | confidence >= 0.50, used >= 2 times |
| Consolidated | 1.0x base | confidence >= 0.70, used >= 5 times |
| Persistent | 5.0x base | confidence >= 0.90, used >= 10 times |

Base half-life by knowledge type:
- Insight: 48 hours
- Heuristic: 96 hours
- Warning: 24 hours
- CausalLink: 168 hours
- StrategyFragment: 72 hours
- AntiKnowledge: 12 hours (decays fastest)

This implements the Complementary Learning Systems framework (McClelland et al. 1995):
fast episodic memory (gamma/theta) consolidates into slow semantic memory (delta).

---

## 5. Playbook Compilation

Delta mines the episode history for patterns compilable into reusable playbook rules.
The `PatternMiner` discovers trigram patterns across episodes -- sequences of
(situation, action, outcome) that recur with positive outcomes. Top patterns become
playbook rules.

Playbook rules enable T0 processing: when a gamma tick detects a matching situation,
the agent acts without invoking an LLM, keeping the tick at $0.00.

---

## 6. Routing Optimization

Delta updates `CascadeRouter` weights based on model performance. Models with high
success + low cost get increased weight. Models with insufficient observations
maintain exploration weight. Cost: T0 computation, no LLM.

---

## 7. Non-Blocking Architecture

```rust
async fn delta_loop(
    state: Arc<RwLock<AgentState>>,
    cancel: CancellationToken,
) {
    loop {
        tokio::select! {
            _ = wait_for_delta_trigger(&state) => {},
            _ = cancel.cancelled() => break,
        }
        let dream_state = DreamState::new();
        let result = tokio::select! {
            r = run_dream_cycle(&state, &dream_state) => r,
            _ = gamma_work_arrived(&state) => {
                dream_state.serialize_to_disk().await?;
                state.write().await.emit_signal(CognitiveSignal::Pause);
                continue;
            }
        };
        if let Ok(output) = result {
            apply_dream_output(&state, &output).await;
        }
    }
}
```

---

## 8. Cost Summary

| Phase | Model Class | Duration | Cost |
|---|---|---|---|
| NREM Replay | Haiku-class (T1) | 8-15 min | $0.02-0.12 |
| REM Imagination | Sonnet-class (T1-T2) | 5-15 min | $0.05-0.20 |
| Integration/Staging | Pure computation (T0) | 5-10 min | $0.00 |
| Knowledge Promotion | Pure computation (T0) | 1-2 min | $0.00 |
| Playbook Compilation | Pure computation (T0) | 1-2 min | $0.00 |
| Meta-Cognition | Haiku-class (T1) | 1-2 min | $0.001-0.005 |
| **Total per Delta** | | **~25-45 min** | **~$0.07-0.33** |

Impact on throughput: ~0% when tasks are available.

---

## 9. References

- **McClelland et al. 1995** -- Complementary Learning Systems (Psychological
  Review 102(3)).
- **Mattar & Daw 2018** -- "Prioritized memory access" (Nature Neuroscience 21).
- **Boden 2004** -- "The Creative Mind" (2nd ed., Routledge).
- **Pearl 2009** -- "Causality" (2nd ed., Cambridge University Press).
- **Walker & van der Helm 2009** -- "Overnight therapy?" (Psychological Bulletin
  135(5)).
- **Kanerva 2009** -- "Hyperdimensional computing" (Cognitive Computation 1(2)).
- **Ebbinghaus 1885** -- "Uber das Gedachtnis". Forgetting curves.
- **Buzsaki 2006** -- "Rhythms of the Brain" (Oxford University Press).

---

## Cross-References

- `docs/v3/depth/29-heartbeat/theta-reflective-loop.md` -- Loop that feeds delta
- `docs/v3/depth/29-heartbeat/adaptive-clock.md` -- Delta scheduling
- `docs/v3/29-HEARTBEAT.md` -- Parent chapter
