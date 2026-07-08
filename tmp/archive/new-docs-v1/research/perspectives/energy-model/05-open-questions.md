# Open Questions — Cognitive Energy Model

**Kind**: Perspective
**Source**: `docs/00-architecture/29-cognitive-energy-model.md`

---

## Unresolved Design Questions

1. **What is the right normalization for the CEU?** The CEU definition proposed weighs
   T0/T1/T2/read/write operations. What weights are correct, and are they deployment-independent
   or deployment-specific? Should cost be denominated in latency, hardware cost, or something
   else?

2. **Is cognitive debt repaid linearly?** Does one Dreams cycle fully repay the debt of
   N T2 invocations, or does debt compound (accumulated debt requires more recovery than
   the individual invocations would suggest)?

3. **Does T2 quality actually degrade with session length?** The energy model predicts this.
   Is it empirically true for Roko's current LLM-based T2? If not, why does the biological
   analog hold while the AI analog does not?

4. **What is the Carnot equivalent for Roko's processing tasks?** The Carnot limit bounds
   engine efficiency. Is there an analogous bound on the quality achievable per CEU for
   specific Roko processing tasks? Can it be computed or measured?

5. **Competing frameworks**: The attention-as-currency perspective ([`../attention-as-currency/`](../attention-as-currency/README.md))
   and the energy model are related but distinct. Both deal with resource allocation.
   How do they differ, and when should each framing be preferred?

---

## Open Research Questions

1. **Landauer's principle for cognitive processing**: Landauer (1961) showed that erasing
   one bit of information requires a minimum energy of \( k_B T \ln 2 \) (about 3×10^{-21}
   J at room temperature). Does an analogous principle bound the energy cost of "forgetting"
   (expiring) an Engram in a computational system?

2. **Information geometry and cognitive efficiency**: Can the Fisher information metric
   on the space of cognitive states provide a measure of "geodesic" processing — the
   minimum-energy path between two cognitive states?

3. **Is there a cognitive equivalent of maximum power output?** For biological systems,
   there is a tradeoff between maximum sustained power and maximum instantaneous power
   (you can sprint or run a marathon, not both). Is there an analogous tradeoff in AI
   cognitive systems between maximum sustained throughput and maximum instantaneous
   quality?

---

## References

- **Landauer, R. (1961).** "Irreversibility and Heat Generation in the Computing Process."
  *IBM Journal of Research and Development*, 5(3), 183–191.
