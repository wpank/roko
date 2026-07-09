# Open Questions — Cognitive Immune System

**Kind**: Perspective
**Source**: `docs/00-architecture/26-cognitive-immune-system.md`

---

## Unresolved Design Questions

1. **What constitutes "self" in a multi-agent deployment?** If multiple Roko agents share
   a common Neuro knowledge substrate, do they share a single "self"? Can one agent's output
   be "foreign" to another agent in the same fleet?

2. **How is the tolerance threshold calibrated?** The system must accept new information
   (learning) while rejecting corrupted information (defense). The tolerance threshold
   determines this balance. Is it a fixed parameter, or does it adapt based on the system's
   operational history?

3. **Can the system detect coordinated attacks?** A single low-quality Engram from a
   suspicious source might not trigger immune responses. A coordinated campaign of many
   such Engrams might be detectable only at the aggregate level. Does Roko have aggregate-level
   threat detection?

4. **What is the "cognitive fever" — the acceptable cost of immune activation?** Biological
   fever is metabolically expensive but kills pathogens effectively. The cognitive analog
   is elevated scrutiny (T2 invocation) that consumes resources. At what threat level does
   the cost of vigilance become acceptable? How is this calibrated?

5. **Autoimmunity detection**: How does the system detect when it is exhibiting cognitive
   autoimmune behavior — systematically rejecting valid knowledge? What is the observable
   signature of autoimmunity vs. healthy immune response?

6. **Evolutionary pressure**: Biological pathogens evolve to evade immune systems. Adversarial
   inputs can be crafted to evade Gate rules. How does the cognitive immune system evolve
   in response to adversarial adaptation? Is the rule-learning pipeline (Dreams) fast enough
   to keep pace?

---

## Open Research Questions

1. Is the innate/adaptive distinction the right decomposition, or would a different
   decomposition (e.g., statistical vs. symbolic detection) be more natural for AI systems?

2. Can formal methods (model checking, type theory) provide a foundation for the
   "cognitive MHC" — a formal definition of "what belongs to self" that can be checked
   automatically?

3. What is the relationship between the immune system lens and the
   [active inference foundation](../../foundations/active-inference.md)? In active inference,
   all inputs are processed through the generative model — there is no explicit rejection.
   How does the immune metaphor coexist with the prediction-correction loop?
