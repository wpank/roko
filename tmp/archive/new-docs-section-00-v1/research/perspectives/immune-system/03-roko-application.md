# Roko Application — Cognitive Immune System

**Kind**: Perspective
**Source**: `docs/00-architecture/26-cognitive-immune-system.md`

---

## Mapping the Immune System to Roko

This page traces the full mapping from biological immune components to Roko's architecture.

---

## The Epidermis — External Boundary

**Biological**: The skin is the first line of defense, preventing pathogens from entering.

**Roko analog**: The interface layer — how Engrams enter the system. Any input validation,
schema enforcement, or trust-level assignment that happens at the point of ingestion before
Engrams enter the Substrate. This includes:
- **Attestation requirements**: Engrams must carry valid provenance (author, timestamp,
  hash). Engrams without attestation are rejected at the boundary.
- **Schema validation**: Engrams that violate the data model are rejected before processing.
- **Source registration**: only registered, trusted sources can inject Engrams directly into
  the Substrate. Unregistered sources route through a higher-scrutiny ingestion path.

---

## The Gate as Innate Immune Response

The [Gate](../../../reference/05-operators/gate.md) is the primary innate immune organ.

**Biological parallel**: Macrophages in the bloodstream, constantly patrolling and engulfing
anything that doesn't look like self. Fast, non-specific, no memory.

**Roko's Gate**:
- Applies hard-coded rule sets (PAMP equivalents): reject low-score Engrams, reject
  untrusted-source Engrams, reject Engrams that trigger safety rules.
- Operates at T0 speed — fast enough to process every Engram without creating a bottleneck.
- No per-instance memory: the Gate applies the same rules to the same Engram pattern every
  time. Memory is a property of the rule set (which can be updated), not the Gate instance.

**Innate immune failure modes → Gate failure modes**:
- **Immunodeficiency**: Gate thresholds set too low → threats pass through unchallenged.
- **Autoimmunity**: Gate rules too aggressive → valid Engrams are rejected, creating knowledge
  blindspots.
- **Immunosenescence** (aging immune decline): Gate rules that were valid when written become
  stale as the threat environment changes.

---

## The Scorer as Antigen Presentation

The [Scorer](../../../reference/05-operators/scorer.md) evaluates Engrams and flags
anomalous ones for elevated scrutiny. In immune terms, the Scorer acts as a **dendritic
cell**: it doesn't kill pathogens, but it presents antigens (signals anomalies) to the
adaptive system (T2 processing) and provides the co-stimulation signals needed to trigger
a response.

The Scorer's trust and confidence axes are the primary antigen presentation dimensions:
- Low trust + high claimed confidence = suspicious combination (the pattern of misinformation)
- High novelty + low coherence = potential adversarial injection
- Contradiction with high-confidence Neuro knowledge = requires investigation

The Scorer does not decide what happens to the Engram — it enriches it with the information
the downstream immune response needs.

---

## Neuro as Immunological Self

The [Neuro knowledge layer](../../../reference/09-cross-cuts/README.md) defines "self"
in the cognitive immune system: the accumulated, validated knowledge of the agent.

An Engram that confirms and extends existing Neuro knowledge is "self-compatible" — it is
recognized as belonging. An Engram that contradicts high-confidence Neuro knowledge is
"foreign" — it requires elevated scrutiny.

The Neuro probing mechanism (querying Neuro with Engram content to check consistency)
is the cognitive equivalent of **MHC molecule presentation**: comparing the new input against
the known-self profile to determine compatibility.

**Autoimmune risk**: If the Neuro knowledge base becomes corrupted (false beliefs become
high-confidence), the system will subsequently reject true information as "foreign." The
integrity of the Neuro knowledge base is the integrity of the self-definition.

---

## Dreams as Thymic Selection

The [Dreams subsystem](../../../reference/09-cross-cuts/README.md) consolidates Engrams
from short-term storage into long-term Neuro knowledge during delta-speed processing.

In immune terms, Dreams is the **thymus**: the organ that selects which immune cells become
part of the permanent immune repertoire. The thymus eliminates T cells that either:
1. Cannot recognize self-MHC (useless — won't work)
2. React too strongly to self-antigens (dangerous — will cause autoimmunity)

Dreams performs an analogous selection: which Engrams are promoted to durable Neuro knowledge
(useful and safe), and which are allowed to decay (useless, expired, or potentially
corrupting)?

The selection criteria in Dreams:
- **Positive selection**: high-confidence, high-coherence Engrams that extend Neuro
  knowledge → promoted
- **Negative selection**: Engrams that contradict established high-confidence Neuro
  knowledge → not promoted (or flagged for resolution)
- **Passive decay**: low-relevance Engrams → expire without promotion

---

## Daimon as Neuroimmune Modulation

The [Daimon affect cross-cut](../../../reference/09-cross-cuts/README.md) modulates the
scoring weights based on affective state. In immune terms, Daimon implements
**neuroimmune modulation**: the nervous system's influence on immune activity.

Biological neuroimmune modulation:
- Stress hormones (cortisol) suppress immune activity (evolutionary trade-off: during threat,
  resources shift to fight/flight rather than slow immune responses).
- Social bonding hormones (oxytocin) can enhance certain immune functions.

Cognitive analog:
- High arousal/threat state (Daimon negative valence) → Gate thresholds tighten (heightened
  immune vigilance).
- Normal operational state → Gate thresholds relax to reduce false positives.
- Post-task consolidation state → Daimon signals Dreams to run more thorough immunological
  review.

---

## Provenance as MHC Typing

In biology, the **major histocompatibility complex** (MHC) molecules present peptide fragments
from proteins inside the cell. The specific MHC type identifies "which body this cell belongs to."
Immune cells from the same organism recognize the same MHC; foreign MHC triggers a response.

The [Provenance type](../../../reference/10-types/provenance.md) on Engrams is the cognitive
MHC: it identifies which agent, process, or source produced the Engram and what trust level
should be assigned. An Engram from a trusted internal source has "self-MHC." An Engram from
an unregistered external source has "foreign MHC" and receives elevated scrutiny.

---

## Key References

- **Matzinger, P. (1994).** "Tolerance, Danger, and the Extended Family." *Annual Review
  of Immunology*, 12, 991–1045. Danger theory.

- **Janeway, C. A., et al. (2001).** *Immunobiology: The Immune System in Health and
  Disease*, 5th ed. Garland. Standard reference.

- **Jerne, N. K. (1955).** "The Natural-Selection Theory of Antibody Formation."
  *PNAS*, 41(11), 849–857. Clonal selection precursor.

- **Burnet, F. M. (1959).** *The Clonal Selection Theory of Acquired Immunity*. Vanderbilt.
