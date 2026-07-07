# Market Mechanics for Attention Allocation

**Kind**: Perspective
**Source**: `docs/00-architecture/25-attention-as-currency.md`

---

## Why Market Mechanisms?

If attention is a finite resource and multiple processes compete for it, the question "how
should attention be allocated?" is a **mechanism design** problem. Mechanism design asks:
what rules for collective decision-making produce desirable outcomes?

The desiderata for an attention allocation mechanism:
1. **Efficiency**: attention goes to the processes where its marginal value is highest.
2. **Incentive compatibility**: processes cannot gain advantage by misrepresenting their
   attention needs.
3. **Fairness** (when relevant): no important class of signals is systematically excluded.
4. **Computability**: the mechanism must be tractable to run in real time.

Classic auction theory provides mechanisms that satisfy subsets of these properties. The
key results are:

---

## Auction-Theoretic Foundations

### Second-Price (Vickrey) Auctions

In a **second-price auction** (Vickrey, 1961), each bidder submits a bid; the highest bidder
wins but pays the second-highest bid price. The key property: bidding one's true valuation
is a dominant strategy. A bidder who bids higher than their true value risks paying more
than the good is worth; a bidder who bids lower risks losing to someone who values it less.

Second-price auctions are **incentive-compatible**: truthful revelation is optimal for
each bidder regardless of what others do. This makes the mechanism robust to strategic
manipulation.

For attention allocation: if Engrams bid for attention using their true importance scores
(rather than inflated scores to game the system), a second-price-like mechanism would produce
efficient allocations. In Roko's current architecture, Engrams do not "bid" strategically
(they cannot misrepresent their scores), so incentive compatibility is trivially satisfied —
but the second-price logic still informs why the Scorer should produce *accurate* importance
assessments rather than biased ones.

### VCG Mechanisms

The **Vickrey-Clarke-Groves** (VCG) mechanism (Clarke, 1971; Groves, 1973) generalizes
second-price auctions to settings with **combinatorial allocations**: when multiple goods
are allocated simultaneously and agents have preferences over bundles.

In VCG:
- Each agent reports a valuation over all possible allocations.
- The mechanism selects the allocation that maximizes total reported value.
- Each agent pays a "Clarke tax" equal to the externality they impose on others — the
  reduction in total value that other agents experience because of the winning agent's
  presence.

VCG is efficient (maximizes total value) and incentive-compatible under broad conditions.
The practical challenge is computational: finding the welfare-maximizing allocation is
NP-hard in general.

**For Roko:** The VCG framework is relevant when the [Composer](../../../reference/05-operators/composer.md)
must select a *bundle* of Engrams to include in a synthesis context. Including Engram A
may increase the value of including Engram B (complementarity) or decrease it (substitution).
VCG would select the bundle that maximizes total synthesis value, accounting for these
interactions. Current practice uses a greedy selection heuristic — not VCG — because
the optimization problem is too large. Understanding the VCG ideal clarifies what the
heuristic approximates.

### Sponsored Search Auctions

Large-scale attention allocation already occurs in practice: **sponsored search auctions**
(Google, Bing) allocate attention slots on search result pages. Each advertiser bids for
a position; the mechanism determines who wins which position and at what price.

The generalized second-price (GSP) auction used in practice is not fully incentive-compatible
but is approximately so in large markets and easy to understand, compute, and manipulate
under partial information. It has been extensively studied (Varian, 2007; Edelman, Ostrovsky,
& Schwarz, 2007) and provides practical insights about:
- **Position effects**: earlier attention slots are disproportionately valuable (attention
  gradient).
- **Quality scores**: ad ranking is based on bid × quality score, not bid alone. The quality
  score is analogous to Roko's relevance axis in the Score.
- **Reserve prices**: a minimum bid that stimuli must exceed to acquire any attention.

The reserve price concept maps directly to Roko's Gate: the Gate sets the minimum score
threshold that an Engram must reach to be routed further. It is a reserve price for attention.

---

## Attention Markets in Practice

### Herbert Simon's Warning

Herbert Simon (1971) coined the phrase "information-rich, attention-poor" to describe the
condition where information abundance creates attention scarcity. He wrote:

> "...a wealth of information creates a poverty of attention and a need to allocate that
> attention efficiently among the overabundance of information sources that might consume it."

This is the cognitive economic condition that any sufficiently rich data environment
(including a high-volume Engram substrate) creates. The allocation mechanism — implicit
or explicit — determines what the agent effectively knows and acts on.

### Goldhaber's Attention Economy

Michael Goldhaber (1997) articulated the concept of the **attention economy**: in a world
of information abundance, attention is the binding scarce resource, and economic value
flows to whatever captures attention. While Goldhaber was writing about the internet economy,
the same logic applies within a cognitive architecture: the "currency" that enables Engrams
to be acted upon is attention, and the design of the attention allocation mechanism is
equivalent to designing the economic rules of the system.

### Davenport and Beck

Davenport and Beck (2001) operationalized the concept in management terms: attention is a
cognitive resource that can be measured (through eye-tracking, physiological markers,
behavioral correlates), and organizations can be analyzed in terms of how they allocate
collective attention. Their framework for **attention management** includes:
- **Capture**: how stimuli acquire attention (salience, novelty, affect)
- **Focus**: maintaining attention on the relevant signal
- **Attention switching**: the cost of redirecting attention (context-switching overhead)
- **Attention restoration**: recovery from attention depletion

Each of these maps to Roko components: Scorer and Gate manage capture; Policy manages
focus and switching; the T0/T1/T2 structure manages switching costs.

---

## Key Papers

- **Vickrey, W. (1961).** "Counterspeculation, Auctions, and Competitive Sealed Tenders."
  *Journal of Finance*, 16(1), 8–37.

- **Clarke, E. H. (1971).** "Multipart Pricing of Public Goods." *Public Choice*, 11(1), 17–33.

- **Groves, T. (1973).** "Incentives in Teams." *Econometrica*, 41(4), 617–631.

- **Varian, H. R. (2007).** "Position Auctions." *International Journal of Industrial
  Organization*, 25(6), 1163–1178.

- **Edelman, B., Ostrovsky, M., & Schwarz, M. (2007).** "Internet Advertising and the
  Generalized Second-Price Auction: Selling Billions of Dollars Worth of Keywords."
  *American Economic Review*, 97(1), 242–259.

- **Simon, H. A. (1971).** "Designing Organizations for an Information-Rich World." In
  M. Greenberger (ed.), *Computers, Communication, and the Public Interest*, 37–72.

- **Goldhaber, M. H. (1997).** "The Attention Economy and the Net." *First Monday*, 2(4).

- **Davenport, T. H., & Beck, J. C. (2001).** *The Attention Economy: Understanding the
  New Currency of Business*. Harvard Business School Press.
