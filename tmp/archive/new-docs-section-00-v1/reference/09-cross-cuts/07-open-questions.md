# Cross-Cuts — Open Questions

**Last reviewed**: 2026-04-19

---

## OQ-1: Should cross-cuts be a formal layer (L2b)?

Currently cross-cuts live at L2 but have a different lifecycle (they are initialized
once, maintain state across ticks, and participate in Delta). A "L2b" tier would
make this distinction explicit and allow the layer-check CI to enforce it. The
counter-argument: L2b would complicate the layer story for little gain, since the
current DI model already captures the distinction.

---

## OQ-2: Cross-cut for external knowledge (Oracles)

The architecture doc mentions "Oracles" as a fourth knowledge source (external APIs,
databases, live feeds). Should Oracles be a cross-cut? Or is an Oracle just a tool
called by ACT? The difference matters: a cross-cut participates in QUERY and SCORE;
a tool is called in ACT. If Oracles need to influence QUERY retrieval (not just ACT
output), they need to be a cross-cut.

---

## OQ-3: Daimon and safety

Daimon's urgency signal lowers routing confidence thresholds, potentially causing
the agent to escalate to Theta more aggressively. In a high-urgency state, could
this cause runaway escalation and cost explosion? A safety cap on how much Daimon
can shift thresholds would address this.

---

## OQ-4: Dreams and trust bootstrapping

New `Kind::Imagined` Engrams start with trust = 0.20. This means they are unlikely
to be composed (SCORE will rank them low on Trust). How do imagined Engrams ever
demonstrate utility if they are never composed? A "probationary" mechanism — where
occasionally a low-trust Imagined Engram is sampled regardless of score — would
allow bootstrapping.

---

## OQ-5: Cross-cut observability

Currently, cross-cut internal state (PAD vector, HDC index size, utility EMA
distribution) is not directly queryable via the public API. Adding read-only
endpoints for cross-cut state would improve debuggability.

---

## See also

- [Overview](00-overview.md)
- [Boundaries](06-boundaries.md)
- [Open Questions (Loop)](../06-loop/16-open-questions.md)
