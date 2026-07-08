# Roko Application — Cognitive Energy Model

**Kind**: Perspective
**Source**: `docs/00-architecture/29-cognitive-energy-model.md`

---

## Mapping Energy Concepts to Roko's Architecture

### T0/T1/T2 as Energy Tiers

The [three cognitive speeds](../../../reference/07-speeds/README.md) map directly to the
metabolic state model:

| Speed | Biological Analog | Energy Cost | Sustainability |
|-------|-------------------|-------------|----------------|
| T0 | Resting metabolic rate | ~5 CEU per Engram | Indefinitely |
| T1 | Active metabolic rate | ~50 CEU per Engram | Hours |
| T2 | Maximal metabolic rate | ~500 CEU per Engram | Minutes |

(CEU values are illustrative, not calibrated to specific hardware.)

The Router's tier selection decision is an **energy allocation decision**: routing to T2
"spends" energy that is unavailable for other tasks. The Router should be understood as
an energy budget manager, not just a priority queue.

### The Router as Energy Budget Controller

The [Router](../../../reference/05-operators/router.md) is the energy budget controller.
Its tier assignment decisions are constrained not just by score (what deserves T2) but by
budget (what can afford T2).

**Current gap**: The Router in most implementations uses score thresholds for tier routing
without explicit energy budget tracking. Two Engrams with identical scores but different
current energy states (one arriving when T2 is fully available; one arriving when T2 is
saturated) are treated identically. An energy-budget-aware Router would route the second
Engram to T1 when T2 is saturated, even if its score would normally merit T2.

**Proposed mechanism**: A Router state variable that tracks current T2 "energy available"
(decremented when T2 is invoked, replenished over time at the T2-recovery rate). When
T2 energy is low, the Router raises the threshold for T2 invocation. This is the cognitive
analog of "fasting" — reducing consumption when the energy budget is constrained.

### The Policy as Metabolic Regulation

The [Policy operator](../../../reference/05-operators/policy.md) sets top-level constraints
on the agent's behavior, including its resource consumption. In the energy model, Policy
is the **metabolic regulation system** — the analogue of hormonal regulation that
determines overall metabolic state.

Policy-level energy decisions:
- **Metabolic setpoint**: what is the sustainable operational level? (Not maximum — sustainable.)
- **Emergency budget**: how much energy can be spent in response to a critical event?
- **Recovery requirements**: what minimum rest period is required after peak operation?
- **Budget allocation across subsystems**: what fraction of total CEU goes to T2 reasoning vs.
  Dreams consolidation vs. Neuro probing?

The Policy operator's throttling rules (rate limits on specific operations) are
metabolic regulation in practice: they prevent any single process from monopolizing the
energy budget.

### Dreams as the Mitochondrial Repair Cycle

The [Dreams consolidation](../../../reference/09-cross-cuts/README.md) process runs at
delta speed (slow, background, periodic). In the energy model, Dreams is:
1. **ATP resynthesis**: converting processed Engrams into durable Neuro knowledge
   (replenishing the "cognitive energy" stored in the knowledge base).
2. **Mitochondrial maintenance**: cleaning up processed-but-not-retained Engrams,
   repairing topological holes, recalibrating confidence scores.
3. **Recovery sleep**: the period of low-activity consolidation that enables the next
   period of active performance.

Suppressing Dreams is suppressing recovery. A system that runs continuously at T1/T2 speed
without Dreams cycles is metabolically unsustainable — it accumulates cognitive entropy
that eventually degrades performance.

### The Scorer as Metabolic Sensor

The [Scorer](../../../reference/05-operators/scorer.md) evaluates each Engram's value.
In the energy model, the Scorer is the **metabolic sensor** — it assesses whether an
Engram's information value justifies its processing cost.

An efficient Scorer would output not just a value score but a **value-to-cost ratio**:
how much value does this Engram deliver per CEU of processing? Engrams with high
value-to-cost ratios deserve priority; Engrams with low ratios should be processed at
the lowest tier or dropped.

The current Scorer outputs a 7-axis value score but does not model processing cost per
tier. Adding a cost dimension would enable explicit efficiency-based allocation.

### The Gate as Metabolic Filter

The [Gate](../../../reference/05-operators/gate.md) prevents low-value Engrams from
consuming downstream processing energy. In the energy model, the Gate is a **metabolic
filter**: it ensures that the energy budget is spent only on inputs that meet a minimum
value threshold.

The Gate's threshold is the minimum value-per-CEU that an Engram must demonstrate to be
worth processing. Below this threshold, the energy cost of processing (even minimal T0
processing) exceeds the expected value.

**Energy-adaptive Gate**: a Gate that adjusts its threshold based on the current energy
budget state. When energy is abundant (T2 fully available), the Gate can be permissive —
even low-value Engrams are cheap to process. When energy is scarce (T2 saturated), the
Gate should be restrictive — only high-value Engrams are worth the remaining budget.

### Neuro as the Long-Term Energy Store

The [Neuro knowledge layer](../../../reference/09-cross-cuts/README.md) stores consolidated
knowledge that can be accessed cheaply (T0 Neuro probe) to answer questions without T2
processing. In the energy model, Neuro is the **energy store** — the long-term reservoir
that enables cheap access to accumulated cognitive work.

A rich Neuro store reduces energy requirements: questions that would require T2 reasoning
without context can be answered with T0 Neuro probing with context. The return on investment
for Neuro consolidation (Dreams cycles) is reduced future energy requirements.

**Energy return on investment**: Each CEU invested in Dreams consolidation (converting
specific Engrams into durable Neuro knowledge) yields a return through reduced future T2
invocations. The break-even point: if the consolidation enables more than one avoided T2
invocation, it pays for itself.
