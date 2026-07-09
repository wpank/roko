# Speed Examples

> Worked scenarios across all three speed tiers.

**Status**: Shipping
**Last reviewed**: 2026-04-19

---

## Example 1: Monitoring Agent (All-Gamma)

**Setup**: An agent monitors a DeFi protocol. Every 10 s it checks if the ISFR rate
has moved > 0.5%.

**Tick 1 (Gamma)**:
- Stimulus: `{ kind: ScheduledCheck, content: "ISFR rate check" }`
- ROUTE: static route to `tool=isfr_oracle`, confidence 1.0 (route_hint)
- ACT: oracle call returns current rate
- VERIFY: pass (rate within expected bounds)
- REACT: schedule next check in 10 s

**Observation**: 100% of ticks run at Gamma. Cost is ~$0.0001/tick.
At 6 ticks/minute, daily cost ≈ $0.86.

---

## Example 2: Research Agent (Gamma + Theta + Delta)

**Setup**: An agent researches DeFi yield strategies. It has a mix of known and novel
questions.

**Tick 1 (Gamma)**: "What is the current KORAI/USDC yield?" → route hint, fast answer.

**Tick 2 (Theta)**: "How does the ISFR feed interact with Roko's active inference
prediction mechanism?" — novel intersection question. Routing confidence 0.67 → Theta.
CoT scaffold used. Answer passes VERIFY. Theta tick cost: $0.04.

**Tick 3 (Gamma)**: "What was the KORAI price yesterday?" → prior similar question,
confidence 0.88 → Gamma.

**After 4 h (Delta)**: Consolidation pass runs. 47 Engrams promoted (high utility
from the research session). 12 Engrams pruned (stale price data). Routing priors
updated. `free_energy_avg` drops from 0.28 to 0.19.

**Observation**: This agent runs 90% Gamma, 10% Theta. The Delta pass each night
improves Gamma efficiency over time.

---

## Example 3: Free Energy Trigger (Gamma → Delta Emergency)

**Setup**: A deployed agent encounters a novel deployment environment where many
stimuli are unfamiliar.

**Ticks 1–20**: Most stimuli route to Theta (confidence < 0.85). Multiple SoftFails.
`free_energy_avg` rises to 0.41 (> emergency threshold 0.35).

**Emergency Delta triggered**:
- Background Delta pass starts immediately
- 200 Engrams from previous deployments are reviewed
- Routing priors for the new environment are bootstrapped from semantic similarity
  to prior environments
- `free_energy_avg` drops to 0.22 after the pass

**Ticks 21–30**: More ticks now route to Gamma (confidence improved). Agent is
adapting to the new environment.

**Observation**: The free energy trigger allows the agent to self-diagnose
disorientation and automatically consolidate.

---

## Example 4: Multi-Agent Speed Coordination

**Setup**: Three agents (A, B, C) in the same deployment share the Substrate.
All have their Delta interval set to 4 h.

**4 h mark**: Agent A triggers its scheduled Delta.
- A publishes `delta.start` Pulse
- B and C see the Pulse, delay their own Delta by 30 s
- A completes its consolidation in 45 s, publishes `delta.complete`
- B starts its Delta immediately
- B completes in 38 s
- C starts its Delta
- C completes in 41 s

**Total coordination window**: 124 s (vs. 134 s if run simultaneously with contention)

**Observation**: Staggered Delta passes reduce Substrate write contention by ~15%.

---

## See also

- [Gamma](01-gamma-reactive.md), [Theta](02-theta-reflective.md), [Delta](03-delta-consolidation.md)
- [Speed Coordination](04-speed-coordination.md) — multi-agent coordination
- [Loop Examples](../06-loop/15-examples.md) — per-tick examples
