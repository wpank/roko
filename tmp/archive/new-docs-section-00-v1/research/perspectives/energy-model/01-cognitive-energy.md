# Cognitive Energy — Budget, ATP Metaphor, Mitochondria Analogy

**Kind**: Perspective
**Source**: `docs/00-architecture/29-cognitive-energy-model.md`

---

## The Cognitive Energy Budget

### Defining the Budget

The **cognitive energy budget** is the total resource envelope available to the system per
unit time. It has multiple dimensions:
- **Compute budget**: CPU/GPU cycles available
- **Memory budget**: RAM available for active processing
- **Context budget**: tokens available in the LLM context window for T2 operations
- **Latency budget**: time available before responses must be delivered
- **I/O budget**: Substrate reads/writes, network calls

These dimensions are interdependent but not interchangeable. Spending the context budget
does not free up memory. Exceeding the latency budget does not earn additional context.

The energy model abstracts over these dimensions by defining a single **cognitive energy
unit** (CEU) that represents a normalized combination. The specific normalization is a
calibration parameter — different deployments may weight dimensions differently.

### Budget Allocation vs. Budget Consumption

The energy model distinguishes between:
- **Allocation**: the planned assignment of energy to tasks (routing decisions, tier
  selection, Composer context selection).
- **Consumption**: the actual energy used by executing the task.

Efficient systems allocate close to consumption: they predict task costs accurately and
allocate appropriately. Inefficient systems over-allocate (wasting resources that could
serve other tasks) or under-allocate (failing to complete tasks that needed more resources).

---

## The ATP Metaphor: Energy as a Fungible Currency

In biochemistry, ATP is **fungible**: any source of energy (fat, glucose, protein) can
be converted to ATP, and ATP can power any type of cellular work. This fungibility is
what makes ATP a "universal currency."

For cognitive systems, the fungibility analog is: different types of cognitive work
(scoring, routing, composing, writing, reasoning) can all be described in terms of the
same energy unit (CEU), even though their underlying compute profiles differ. This
fungibility enables principled tradeoffs: spend less on routing to have more for composing.

### The ATP Cycle: Use and Replenishment

ATP (adenosine triphosphate) is hydrolyzed to ADP (adenosine diphosphate) during work.
ADP is then phosphorylated back to ATP during energy metabolism. The cycle:

```
Energy input (glucose/O2) + ADP + Pi → ATP + H2O + heat
Work + ATP → ADP + Pi + useful work
```

The cognitive analog:

```
Compute allocation + task → processing result + "cognitive heat" (side-channel costs)
Processing result + consolidation → durable knowledge (replenishing Neuro)
```

The "cognitive heat" (side-channel costs) includes: context pollution from processed but
not-retained Engrams, increased routing overhead from filled queues, latency from
garbage-collection-equivalent processes.

---

## The Mitochondria Analogy: Infrastructure as Power Plant

### Different Cells, Different Mitochondrial Density

Not all cells have the same mitochondrial density. Heart muscle cells and neurons (high
sustained energy demand) have dense mitochondria. Fat cells (low energy demand, high
energy storage) have sparse mitochondria.

For cognitive systems, the analog is: **different processing functions have different
compute intensities**. T2 reasoning is the "heart muscle" — metabolically expensive and
performance-critical. T0 response is the "fat cell" — low compute intensity, high
throughout. The system's architecture should allocate "mitochondria" (compute capacity)
to functions in proportion to their metabolic requirements.

### Mitochondrial Dysfunction

Mitochondrial dysfunction (failure of energy production) produces fatigue, cognitive
impairment, and ultimately cell death. The cognitive analog:
- **Context window saturation**: T2 operations can no longer complete because the context
  window is full. The "power plant" is overwhelmed.
- **Substrate backpressure**: Engrams cannot be written to the Substrate because of queue
  saturation. Processing results are lost.
- **Scheduler starvation**: high-priority tasks consume all scheduling capacity, starving
  background processes (Dreams, Neuro consolidation).

These failure modes are the cognitive analogs of mitochondrial dysfunction: the energy
production system fails, and higher-level function degrades.

---

## Kahneman's Dual Process as Energy Zones

Kahneman (2011) notes that System 2 thinking is "effortful" — it requires sustained
attention and produces measurable physiological signatures of effort (pupil dilation,
glucose consumption). System 1 thinking is "effortless" — it occurs automatically without
detectable effort cost.

In energy model terms:
- **System 1 (T0/T1 in Roko)**: low CEU cost, parallelizable, does not deplete the budget
  significantly per operation.
- **System 2 (T2 in Roko)**: high CEU cost, serial (the context window is a serial
  resource), depletes the budget significantly per operation.

**Decision Fatigue** (Baumeister et al., 2008): the finding that decision quality degrades
after many high-cognitive-load decisions has been interpreted as evidence for a depletable
cognitive energy resource. While the glucose-depletion interpretation has been contested,
the performance degradation phenomenon is robust.

The cognitive analog for AI systems: T2 quality may degrade if T2 is invoked too
frequently within a session, because:
- The context window fills with prior T2 outputs, leaving less room for new T2 operations.
- Prior T2 reasoning may bias subsequent T2 reasoning (context anchoring).
- Accumulated cognitive overhead (managing many in-flight T2 operations) reduces the
  quality of each.

---

## References

- **Attwell, D., & Laughlin, S. B. (2001).** "An Energy Budget for Signaling in the Grey
  Matter of the Brain." *Journal of Cerebral Blood Flow and Metabolism*, 21(10), 1133–1145.
  Neural energy costs.

- **Kahneman, D. (2011).** *Thinking, Fast and Slow*. Farrar, Straus and Giroux.

- **Baumeister, R. F., Vohs, K. D., & Tice, D. M. (2007).** "The Strength Model of
  Self-Control." *Current Directions in Psychological Science*, 16(6), 351–355.
  Decision fatigue and ego depletion.
