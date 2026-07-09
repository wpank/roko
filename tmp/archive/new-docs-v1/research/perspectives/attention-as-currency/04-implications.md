# Implications — Attention as Currency

**Kind**: Perspective
**Source**: `docs/00-architecture/25-attention-as-currency.md`

---

## Design Decisions That Follow From the Metaphor

Taking the attention-as-currency lens seriously produces specific, testable design
recommendations. This page lists the most consequential.

---

## 1. Make the Aggregation Function Explicit

The 7-axis Score is a multi-dimensional bid. Somewhere, these 7 dimensions are combined
into a priority ordering. This aggregation currently happens implicitly, through
hardcoded weights or ad-hoc rules.

**Implication**: The aggregation function should be explicit, documented, and tunable.
A change in aggregation weights is a change in what the system attends to — it is a
policy decision, not an implementation detail. Different deployments may require different
aggregation functions (a medical assistant should weight accuracy/confidence more heavily;
a creative assistant might weight novelty more heavily).

The aggregation function is the **social welfare function** of the attention market. Making
it explicit enables principled debate about its design.

---

## 2. Implement Attention Budgets at the Policy Level

Currently, compute allocation is managed at the infrastructure level (CPU/memory quotas).
The attention economy framing suggests complementing this with **semantic attention budgets**:
caps on how many Engrams from a given topic, source, or type can acquire T2 attention in
a given time window.

Without semantic attention budgets, a high-volume signal source (an agent that produces many
high-scoring Engrams about the same topic) can monopolize T2 attention, starving other
topics.

**Implementation suggestion**: Add a rate-limiting dimension to the Policy operator's rules
that tracks "attention share by category" and throttles categories that exceed their budget,
regardless of individual bid scores.

---

## 3. Measure Attention Allocation Empirically

The attention-as-currency lens predicts measurable allocation patterns. These should be
instrumented:

- **Attention Gini coefficient**: how unequally is T2 attention distributed across Engram
  categories? High Gini → attention monopoly risk.
- **Attention poverty rate**: what fraction of Engrams that were manually retrospectively
  judged important were, in fact, gate-rejected? Non-zero rate → systematic blind spots.
- **Allocation efficiency**: for Engrams that received T2 attention, how often did the T2
  response exceed the quality of a hypothetical T1 response? Low rate → over-spending on T2.

These metrics translate the economic concepts into operational measurements that can be
tracked over time.

---

## 4. Context Window as Portfolio

The Composer selects a bundle for the synthesis context. The attention-as-currency lens
suggests treating this bundle as a **portfolio** — a collection of assets selected to
maximize expected return (synthesis quality) subject to a budget constraint (context window).

Portfolio theory (Markowitz, 1952) tells us that:
- Diversification reduces variance without necessarily reducing expected return.
- Correlated assets (Engrams about the same topic) do not diversify risk.
- The efficient frontier is the set of portfolios with the best risk-adjusted return.

Applied to Engram bundle selection: prefer bundles that cover diverse aspects of the query
(diversification) over bundles that repeat the same content (correlated assets), even if
individual Engram scores are similar.

---

## 5. Price the Externality of Attention Switching

Context switching — shifting attention from one topic to another — has a cost. Empirical
research on human multi-tasking (Rubinstein, Meyer, & Evans, 2001) shows significant
**switch costs**: cognitive overhead when redirecting attention.

In Roko, attention switching between T2 tasks has an analogous cost: context windows must
be cleared and rebuilt; Neuro probes must be re-run; coherence must be re-established.
These switch costs are currently not accounted for in the allocation decision — each Engram
is allocated attention as if the system starts fresh.

**Implication**: The Router should model switch costs when deciding whether to promote an
Engram to T2. An Engram that would marginally justify T2 attention in isolation may not
justify it if the system is currently engaged in a different T2 task and switching would
incur a large cost.

---

## 6. Reserve the Algedonic Channel

Following Beer's VSM (see [cybernetics.md](../../foundations/cybernetics.md)), the attention
allocation market should have a **bypass channel** for urgent safety signals that skip the
normal allocation queue.

This is already partially implemented through priority scoring, but the economic framing
clarifies why it is essential: in a market-based allocation, any finite bid can be outbid.
A safety signal competing in the same market as routine signals may lose to accumulated
high-value but non-urgent signals. The bypass channel takes safety signals out of the market
entirely — they are not allocated, they are mandated.

---

## References

- **Markowitz, H. (1952).** "Portfolio Selection." *Journal of Finance*, 7(1), 77–91.

- **Rubinstein, J. S., Meyer, D. E., & Evans, J. E. (2001).** "Executive Control of
  Cognitive Processes in Task Switching." *Journal of Experimental Psychology: Human
  Perception and Performance*, 27(4), 763–797.
