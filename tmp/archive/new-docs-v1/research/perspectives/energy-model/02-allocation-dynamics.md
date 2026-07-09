# Allocation Dynamics — Energy Flow in Cognitive Systems

**Kind**: Perspective
**Source**: `docs/00-architecture/29-cognitive-energy-model.md`

---

## Thermodynamic Principles for Cognitive Systems

### The First Law: Conservation

The first law of thermodynamics: energy is conserved. Energy spent on task A is not
available for task B.

**Cognitive implication**: the total cognitive energy budget is conserved. Spending more on
T2 reasoning means spending less on T0/T1 monitoring, Substrate maintenance, or
background consolidation. There is no free lunch.

This seems obvious but is violated regularly in system design: adding features (new processing
paths, new Engram types, new operators) increases total energy demand without explicitly
decreasing allocations elsewhere. Eventually the system is over-committed — more energy
demand than supply.

### The Second Law: Entropy Production

The second law of thermodynamics: in any closed system, entropy increases over time. All
real processes produce waste heat.

**Cognitive implication**: all cognitive processing produces entropy — waste products that
require energy to clean up. These include:
- Processed-but-not-retained Engrams cluttering the Substrate
- Accumulated prediction errors in the confidence graph
- Context window pollution from low-value content
- Queue buildup in the Router

Ignoring second-law effects means ignoring the cumulative cost of cognitive processing's
waste products. Garbage collection, Substrate compaction, confidence recalibration, and
Dreams consolidation are all entropy-reduction operations — they require energy and must
be budgeted.

### The Carnot Efficiency Limit

The Carnot cycle defines the maximum efficiency of any heat engine operating between
temperatures \( T_H \) (hot reservoir) and \( T_C \) (cold reservoir):

\[
\eta_{Carnot} = 1 - \frac{T_C}{T_H}
\]

No heat engine can exceed this efficiency. The implication is that efficiency is bounded —
there is a theoretical maximum beyond which no implementation can improve.

For cognitive systems, the Carnot analog is the **maximum achievable quality per compute
unit** for a given processing task. This maximum is set by the inherent information-theoretic
complexity of the task (how much information must be processed) and the implementation's
overhead. No routing or scheduling optimization can exceed this bound.

Understanding the Carnot-equivalent bound helps set realistic expectations: if a task's
inherent complexity requires T2 processing, no T1 optimization will match T2 quality.
The bound is real.

---

## Energy States and Transitions

### Activation Energy

In chemistry, a reaction requires an **activation energy** — a minimum energy input to
overcome the barrier and initiate the reaction. Even exothermic reactions (that release
energy overall) require activation energy to start.

**Cognitive analog**: starting a T2 reasoning process has an activation energy — the cost
of loading context, initializing the reasoning chain, making the LLM call. Even if the
T2 reasoning quickly produces a high-value output, the activation energy is spent first.

This activation energy effect explains why **T2 invocation should be batched when possible**:
if multiple tasks require T2, batching them (sharing context setup, running in one LLM
call) amortizes the activation energy across tasks.

### Energy Wells and Stability

In quantum mechanics and chemistry, particles/molecules tend to occupy **energy wells** —
local minima in the potential energy landscape. Moving out of an energy well requires
investment of activation energy.

For cognitive systems, energy wells are **stable processing patterns**: the system has
settled into a low-energy routine (established context, cached lookups, well-worn reasoning
paths) and moving to a new pattern requires activation energy investment.

This explains the cognitive cost of **context switching**: leaving a current processing
domain (an energy well) and entering a new one requires activating against the energy
gradient. The old context's state must be discarded (energy cost) and the new context's
state must be established (activation energy cost).

### Resonance and Coherence

In physics, **resonance** occurs when a driving force matches a system's natural frequency,
producing large amplitude oscillations with low energy input. Acoustic resonance amplifies
sound; electrical resonance (LC circuits) amplifies signals.

**Cognitive analog**: when a new Engram resonates with the current context (its content,
timing, and relevance align with the current processing state), it can be integrated with
low energy cost. When an Engram is out of resonance (irrelevant, poorly timed, or
contradictory to current context), integration requires more energy.

The Scorer's coherence axis measures, in part, this resonance: a high-coherence Engram is
in resonance with the current context and can be integrated cheaply.

---

## Energy Recovery and Sustainable Throughput

### Recovery Cycles

Biological systems have explicit recovery mechanisms:
- **Sleep**: the brain's primary recovery cycle, consolidating memories, clearing metabolic
  waste, restoring energy stores.
- **Rest**: lower-intensity activity that allows partial recovery between peak efforts.

**Cognitive analog**:
- **Dreams consolidation**: the delta-speed processing cycle that consolidates Engrams,
  updates Neuro, and clears processed-but-not-retained content. This is the "sleep" cycle.
- **T0/T1 periods between T2 bursts**: lower-intensity processing between peaks of deliberate
  reasoning. This is "rest."

Sustainable throughput requires respecting these recovery cycles. A system that suppresses
Dreams consolidation (because there is always urgent work to do) will accumulate cognitive
debt: entropy increases, knowledge topology fragments, and T2 quality degrades.

### Sustainable vs. Peak Performance

The distinction between **sustainable throughput** (energy use ≤ replenishment rate) and
**peak throughput** (energy use > replenishment rate, sustainable briefly) is fundamental
to capacity planning.

For Roko:
- **Sustainable** configuration: T2 invocations paced to allow context cleanup between
  calls; Dreams consolidation runs at a frequency that keeps the Substrate clean; Scorer
  calibration is maintained.
- **Peak** configuration: maximum T2 invocations, context window filled, Dreams
  consolidation suspended. Sustainable for one session; not sustainable over days.

Production deployments should be designed to operate sustainably. Peak configurations
should be available for exceptional circumstances with explicit budgeting for the recovery
period afterward.
