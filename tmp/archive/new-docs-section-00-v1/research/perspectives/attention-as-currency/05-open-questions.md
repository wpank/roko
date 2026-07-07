# Open Questions — Attention as Currency

**Kind**: Perspective
**Source**: `docs/00-architecture/25-attention-as-currency.md`

---

## Unresolved Design Questions

1. **What is the right aggregation function for the 7-axis Score?** The current function is
   implicit. Should it be a weighted sum? A lexicographic ordering (safety first, then
   relevance, then urgency)? A multiplicative product? Each choice embeds different values.
   Is there an empirical way to determine which aggregation produces better outcomes?

2. **Can Engram bids be "gamed"?** In the current architecture, Engrams are produced by
   agents and other processes that do not strategically misrepresent their importance.
   But in multi-agent settings, an agent could conceivably produce many Engrams calibrated
   to score highly, crowding out other agents. What defense mechanisms are needed?

3. **What is the right unit of "attention" for measuring allocation efficiency?** Token count?
   Compute cycles? Latency? Human time? Different units lead to different efficiency criteria.

4. **Does the attention economy frame apply equally to all tiers?** T0 processing is so cheap
   that "scarcity" may not apply meaningfully. T2 processing is expensive enough that every
   allocation decision matters. Does the attention economy model need to be tier-specific?

5. **How should complementarity between Engrams be represented in the Score?** The current
   Score is computed per-Engram, not per-bundle. Bundle value may be superadditive or
   subadditive relative to individual scores. Can a per-Engram score encode bundle
   complementarity without exponential complexity?

6. **Is efficiency the right criterion?** Economic efficiency maximizes total value. But
   safety may require satisficing rather than maximizing — ensuring a minimum coverage of
   safety-relevant signals regardless of their bid value. How is fairness-to-safety-signals
   formalized?

7. **Temporal discounting**: how should the urgency axis be calibrated against the cost
   of T2 invocation? An Engram with urgency 0.9 and a T2 invocation cost of 200ms — is
   the urgency "worth it"? What is the exchange rate between urgency units and latency
   milliseconds?

---

## Open Research Questions

1. Do multi-agent AI systems exhibit measurable *c*-factor variation (see
   [c-factor foundation](../../foundations/c-factor.md))? If so, does attention allocation
   policy predict *c*-factor, analogously to how conversational equality predicts *c* in
   human groups?

2. Can attention allocation be learned end-to-end? Current allocation uses hand-designed
   rules. Could a learned allocation policy (trained on task-performance feedback) outperform
   designed rules? What training signal would be used?

3. The relationship between attention allocation and belief updating: in active inference
   terms, attending to an Engram is equivalent to increasing the precision weight on the
   prediction error it carries. Can the attention market be re-interpreted as a precision
   allocation market, connecting more formally to the
   [active inference foundation](../../foundations/active-inference.md)?
