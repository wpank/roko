# Implications — Cognitive Energy Model

**Kind**: Perspective
**Source**: `docs/00-architecture/29-cognitive-energy-model.md`

---

## Design Decisions From the Energy Lens

### 1. Define and Track the CEU

The most important implication: **define a cognitive energy unit (CEU) and track it**.
Without a defined unit, energy-based reasoning stays metaphorical. With a unit, it becomes
operational.

Proposed CEU definition for Roko: a weighted combination of:
- T0 invocations × cost_T0
- T1 invocations × cost_T1  
- T2 invocations × cost_T2
- Substrate reads × cost_read
- Substrate writes × cost_write

The weights should be calibrated to actual hardware costs (latency, memory, dollar cost)
in the target deployment environment. The CEU then becomes a single number that can be
tracked, budgeted, and optimized.

### 2. Add Energy State to the Router

The Router should track current T2 "energy available" and adjust tier routing based on
energy state, not just on Engram score:

```
tier = if score >= T2_threshold AND t2_energy > t2_energy_minimum:
    T2
elif score >= T1_threshold:
    T1
else:
    T0
```

This prevents T2 budget depletion: once T2 energy is exhausted, no further Engrams can
access T2 regardless of their score. They queue for T2 when energy recovers, or process
at T1 immediately.

### 3. Schedule Dreams as a Recovery Obligation

Dreams consolidation is currently scheduled when compute is available. The energy model
reframes this: Dreams is a **recovery obligation**, not an optional background task.
Just as an athlete must sleep or performance degrades, the cognitive system must run Dreams
or T2 quality degrades.

Operational implication: Dreams cycles should have a minimum frequency guarantee (e.g.,
every 4 hours of active T2 processing, a Dreams cycle runs). This guarantee takes priority
over other scheduling decisions — it is not preemptable by workload.

### 4. Instrument Energy Debt

**Cognitive energy debt** is the accumulated difference between energy consumed and energy
recovered. A system that has been running at peak performance for extended periods has
high energy debt: its T2 quality is degraded, its Substrate is cluttered, its Neuro
is stale.

Energy debt should be:
- **Measured**: track the time since last Dreams cycle, the current Substrate clutter level,
  the T2 invocation rate over the past hour.
- **Reported**: expose energy debt as a first-class metric in system monitoring.
- **Bounded**: trigger mandatory Dreams cycles when energy debt exceeds a threshold.

### 5. Value-to-Cost Ratio in the Scorer

Add a **processing cost model** to the Scorer's output: for each Engram, estimate the
expected CEU cost of processing it at T0, T1, and T2. Combine this with the value score
to produce a **value-to-cost ratio** for each tier.

The Router then uses the value-to-cost ratio rather than the raw value score for tier
selection. An Engram with moderate value but very low T1 cost may outrank an Engram with
high value but very high T2 cost, when the energy budget is tight.

### 6. Define a Sustainable Operation Profile

Create a formal **sustainable operation profile** for each deployment type:
- Maximum T2 invocations per hour
- Required Dreams cycle frequency
- Maximum Substrate size before compaction
- Maximum context window utilization before forced flush

Operations documentation ([operations/configuration/](../../../operations/configuration/README.md))
should include this profile as a required configuration parameter, not an optional
performance tuning.
