# Cognitive Energy Model — Overview

**Kind**: Perspective
**Source**: `docs/00-architecture/29-cognitive-energy-model.md`

---

## The Central Claim

Cognitive processing is work. Work has a cost. Cost is bounded. A system that ignores its
energy budget will either fail catastrophically (run out of resources) or degrade gracefully
(do less as resources deplete). A system that manages its energy budget can sustain
performance over time and make principled tradeoffs between cost and quality.

The biological brain is the most relevant existence proof: it consumes approximately 20% of
the body's total energy budget despite being only 2% of body weight. It has evolved
sophisticated mechanisms for managing this budget — selective activation of neural circuits,
resting states for consolidation, metabolic regulation of arousal levels. The brain does not
run all processes at full intensity all the time; it manages its energy budget with
extraordinary sophistication.

The cognitive energy model asks: what would it mean to apply similar budget management
to an AI cognitive architecture?

---

## Why "Energy" Rather Than "Compute"?

The term "compute" refers to a specific technical resource (CPU cycles, GPU operations).
"Energy" is a more general concept that:
1. Encompasses multiple resources simultaneously (CPU, memory, I/O, context window).
2. Has well-developed mathematics (thermodynamics, statistical mechanics).
3. Connects to biological performance limits in ways that "compute" does not.
4. Enables reasoning about **efficiency** (work per unit energy) and **recovery** (replenishing
   depleted budgets).

The energy framing does not replace compute analysis. It provides a higher-level conceptual
framework within which compute analysis sits.

---

## Biological Background

### ATP: The Universal Currency of Cellular Energy

Adenosine triphosphate (ATP) is the molecule that cells use to store and transfer chemical
energy. Almost all cellular work — mechanical (muscle contraction), chemical (biosynthesis),
electrical (ion pumping across membranes) — is powered by ATP hydrolysis.

ATP is not stored in large quantities. The body's total ATP supply, if not replenished,
would be exhausted in about 1 minute of sustained activity. Continuous performance requires
continuous ATP synthesis, primarily through oxidative phosphorylation in mitochondria.

**Key insight**: performance is gated by the rate of energy *replenishment*, not just the
rate of energy *use*. A system that can use energy faster than it can replenish it will
sustain peak performance briefly and then crash. A system that matches use rate to
replenishment rate can sustain performance indefinitely.

### Mitochondria as Cognitive Power Plants

Mitochondria are the cellular organelles responsible for ATP synthesis. The more mitochondria
a cell has (and the more efficiently they function), the higher the cell's energy output
capacity. Neurons have very high mitochondrial density because they are metabolically
expensive: a typical neuron consumes ~4.7 billion ATP molecules per second.

The mitochondrial density metaphor for AI systems: the "cognitive power plant" is the
hardware and software infrastructure that delivers compute capacity. Peak cognitive
performance is gated by the power plant's throughput.

### Metabolic States: Rest, Active, and Maximal

Biological metabolism operates across three regimes:
- **Resting metabolic rate (RMR)**: baseline energy consumption, maintaining essential
  functions without active work. Even at rest, the brain consumes 10W (human).
- **Active metabolic rate**: moderate work, sustainable for extended periods. The rate
  at which replenishment keeps pace with consumption.
- **Maximal metabolic rate**: maximum possible work rate, sustainable for seconds to minutes.
  The replenishment system cannot keep pace; energy reserves are depleted.

**Cognitive analog**:
- **Resting (T0)**: background monitoring, heartbeat, basic maintenance.
- **Active (T1)**: standard processing of incoming Engrams, sustainable over extended sessions.
- **Maximal (T2)**: full deliberate reasoning, high compute cost, not sustainable indefinitely.

---

## What the Model Predicts

If the energy model is taken seriously, it predicts:

1. **Performance degradation under sustained maximum load**: T2 processing cannot be
   sustained indefinitely. Systems that consistently route to T2 will eventually fail
   (context exhaustion, OOM, latency violations).

2. **Recovery requirements**: After sustained high-load processing, the system needs
   a recovery period at lower intensity. This maps to the need for Dreams consolidation
   cycles and Substrate cleanup.

3. **Efficiency as a primary metric**: Energy-efficient processing (high quality per
   compute unit) is more sustainable than energy-intensive processing. Investments in
   better T0/T1 heuristics that reduce T2 invocations have compounding energy benefits.

4. **Graceful degradation under budget pressure**: when compute is constrained, a budget-
   aware system degrades gracefully (reduces T2 frequency, increases T0 reliance) rather
   than crashing.
