# Speeds — Open Questions

**Last reviewed**: 2026-04-19

---

## OQ-1: Dynamic threshold adaptation

The Gamma/Theta confidence thresholds (0.85 / 0.60) are currently static configuration
values. A more principled approach would adapt them per agent based on observed
Theta-vs-Gamma quality difference: if Theta rarely improves over Gamma for a specific
agent, the threshold should increase; if Theta consistently improves, it should decrease.

---

## OQ-2: Partial Delta

A full Delta pass processes all Engrams from the last 24 h. For agents with large
Substrates, this can take minutes. A "partial Delta" approach — consolidating only
the highest-utility or most-recently-active Engrams — could reduce Delta latency
without meaningfully degrading consolidation quality.

---

## OQ-3: Speed tier for sub-agent dispatch

When an agent dispatches a sub-agent, the sub-agent runs its own loop at its own
speed. The parent agent's speed tier does not propagate to the sub-agent. This is
correct for independent agents but may be wrong for tightly-coupled parent/child pairs
where the child should always run at the parent's tier.

---

## OQ-4: Four-speed model

Neuroscience actually supports more than three oscillatory bands: beta (12–30 Hz),
associated with motor planning and attention, sits between gamma and theta and has a
plausible cognitive interpretation for Roko (task execution attention / focused
processing). Should Roko add a Beta tier?

---

## OQ-5: Theta caching

Theta ticks are expensive. If two agents receive the same stimulus within a short
window, could they share the Theta result rather than each running an independent tick?
This would require a Theta result cache keyed by stimulus fingerprint, with a TTL.

---

## See also

- [Open Questions (Loop)](../06-loop/16-open-questions.md)
- [Overview](00-overview.md)
