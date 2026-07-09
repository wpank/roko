# Implications — Cognitive Immune System

**Kind**: Perspective
**Source**: `docs/00-architecture/26-cognitive-immune-system.md`

---

## Design Decisions From the Immune Lens

### 1. Layered Defense, Not Perimeter Defense

The immune metaphor's most important implication: **defense must be layered throughout the
system, not concentrated at the boundary**.

Current Roko concentrates most defense at the Gate. This is perimeter defense — effective
when all threats arrive through the Gate. But threats can arise internally:
- An agent produces hallucinatory Engrams that pass all validation.
- A trusted source produces correct-looking but subtly false information.
- Cascading errors from valid Engrams produce invalid synthesis outputs.

**Implication**: Add post-Gate immune mechanisms:
- Consequence monitoring (DAMP detection): flag synthesis outputs that exhibit error signatures.
- Cross-Engram consistency checking: detect Engram clusters that contradict each other
  (the analog of lymph node surveillance).
- Output immune review: before committing a synthesis result to the Substrate, run an
  "output innate immune check" analogous to the input Gate.

### 2. Maintain Tolerance Lists

Implement explicit tolerance mechanisms to prevent cognitive autoimmunity:
- **Trusted source whitelist**: sources whose outputs are classified as "self" and not
  subjected to full Gate scrutiny. These should be carefully managed (trusted sources can
  be compromised) but high-volume trusted sources need fast paths.
- **Established knowledge protection**: Neuro entries above a confidence threshold should
  require multiple high-quality contradicting Engrams (not just one) before they are
  invalidated. Single contradictions trigger investigation, not immediate replacement.
- **Core belief anchors**: the agent's fundamental goals, identity, and safety rules should
  be protected against single-Engram overwrite — the cognitive equivalent of thymic selection.

### 3. Build Immunological Memory into the Witness DAG

The Witness DAG records interaction history. It should be extended to record:
- Gate rejection patterns (which structural patterns have been rejected, and why)
- Attack signatures (patterns that, in retrospect, constituted adversarial inputs)
- Resolution patterns (how past threats were neutralized)

This memory enables faster recognition and response on re-encounter: the second time a
particular attack pattern appears, it should be caught at the innate (Gate) level rather
than requiring T2 investigation.

### 4. Add a Formal Threat Taxonomy

The biological immune system has a rich vocabulary for threat types: viral, bacterial,
fungal, parasitic, neoplastic. This vocabulary enables precise communication about threats.

A cognitive immune system needs an analogous taxonomy:
- **Adversarial injection**: externally crafted Engrams designed to corrupt behavior
- **Cascading error**: internally generated Engrams that are individually valid but
  collectively misleading
- **Goal corruption**: Engrams that subtly redirect goal structures
- **Knowledge poisoning**: Engrams that introduce false beliefs into Neuro
- **Context flooding**: high-volume low-value Engrams that crowd out legitimate signals
  (attention monopoly)

Having names for these categories enables better monitoring, better escalation, and better
post-incident analysis.

### 5. Calibrate Gate Sensitivity to Threat Context

The biological innate system adjusts its sensitivity to context: the mucosal immune system
(gut, lungs) is more tolerant than the systemic immune system because it routinely encounters
a high diversity of non-threatening foreign material (food, inhaled particles, commensal
bacteria).

Roko's Gate sensitivity should similarly be context-dependent:
- In a high-trust operational environment (private deployment, known user base), Gate
  thresholds can be more permissive.
- In a public-facing deployment with unknown inputs, Gate thresholds should be stricter.
- During a known attack (active threat detection in the Witness DAG), all thresholds should
  increase temporarily.

This implies the Gate should accept a context signal from the Policy layer, not operate
with fixed thresholds.
