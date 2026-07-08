# Cognitive Immune System — Overview

**Kind**: Perspective
**Source**: `docs/00-architecture/26-cognitive-immune-system.md`

---

## The Central Claim

A cognitive system that processes external information is vulnerable to corruption in the
same way that a biological organism is vulnerable to pathogens. Both face the fundamental
problem of **self/non-self discrimination**: how to distinguish "things that belong here
and are safe to process" from "things that do not belong and should be neutralized."

The biological immune system, refined over hundreds of millions of years of evolutionary
pressure, is the most sophisticated self/non-self discrimination system we know of. It
provides a rich and well-theorized model for thinking about cognitive system defenses.

Taking the analogy seriously means asking:
- What are the "pathogens" in a cognitive system? (Corrupted data, adversarial inputs,
  goal-corrupting feedback, cascading errors, outdated knowledge that has become false)
- What is "self"? (The system's current valid knowledge, active goals, trusted sources)
- What is the cognitive analog of the innate immune system? Of the adaptive immune system?
  Of immunological memory?
- What are the failure modes? (Cognitive autoimmunity — attacking valid knowledge;
  cognitive immunodeficiency — accepting harmful inputs)

---

## Why This Metaphor Now

The naive response to threats is to add security checks as bolt-ons — input validation,
rate limiting, access control. These are necessary but insufficient. The immune system
metaphor captures something these approaches miss: **defenses must be integrated throughout
the system**, not just at the boundary.

Biological organisms do not defend themselves only at the skin. Immune cells patrol the
bloodstream, the gut, the brain. Defense is a systemic property, not a perimeter property.

For cognitive architectures, this means:
- The Gate is not the only line of defense. It is the skin — the first line. Inside the
  Gate, anomaly detection (the innate system) continues. After initial processing, learned
  classifiers (the adaptive system) continue.
- The system needs an **immune memory**: a record of what attacks looked like, how they
  were defeated, and how to recognize them faster in the future.
- The system needs mechanisms to avoid **autoimmunity**: rejecting its own valid knowledge
  because it resembles the signature of known threats.

---

## Biological Background Summary

The vertebrate immune system has two major arms:

**Innate immunity** (phylogenetically ancient, present in all animals):
- Non-specific: recognizes broad classes of "danger signals" (pattern-associated molecular
  patterns, PAMPs) rather than specific pathogens.
- Fast: responds within minutes to hours.
- No memory: responds identically to second exposure.
- Key cells: macrophages, neutrophils, natural killer cells, dendritic cells.

**Adaptive immunity** (vertebrate-specific):
- Specific: recognizes precise molecular signatures (antigens) via B and T cell receptors.
- Slow: primary response takes 1–2 weeks (clonal expansion).
- Memory: secondary response is faster and more powerful.
- Key cells: B cells (antibodies), T cells (cytotoxic, helper), memory B and T cells.

The two systems are not independent: the innate system activates and directs the adaptive
system, presenting antigens and providing co-stimulatory signals. The adaptive system, once
activated, feeds back onto the innate system.

This integration is key: the biological immune system is not two separate systems that
happen to coexist — it is one integrated system with two temporal modes.

---

## Scope

This perspective focuses on the **analogy as a design tool**, not on building a literal
biological simulation. The goal is to use the vocabulary and architecture of the immune
system to think more clearly about cognitive defenses — to surface design requirements that
the engineering frame alone might miss.
