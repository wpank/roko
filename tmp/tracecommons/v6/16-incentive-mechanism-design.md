# Incentive Mechanism Design

**Date**: August 2026 (v6)

TraceCommons (TC) is an open-source Rust AI trace registry (~235K LOC, 6 crates) that scores
AI coding agent session traces for quality and novelty inside TEEs (Trusted Execution
Environments), compensating contributors with NEAR blockchain credits. Credit formula:
`q = f * g * a` where f=quality, g=novelty, a=anomaly penalty. ~352 submissions, ~13/week,
3 contributors, 6 GitHub stars. The credit formula uses Shapley-inspired valuation -- a choice
now proven gameable across three independent papers. `vcg_allocate` is already built in the
codebase but the greedy path dominates at runtime. This document synthesizes the game-theoretic
literature on why the current formula fails, what replaces it, and the phased implementation
path from broken incentives to a provably incentive-compatible credit system.

---

## 1. The Problem

TC pays NEAR credits for traces. The credit formula `q = f * g * a` uses Shapley-inspired
valuation to price each trace's contribution: quality times novelty times anomaly penalty.
This is a direct application of cooperative game theory -- each trace's payment is its marginal
contribution to the corpus.

Shapley values are a natural starting point. They are the unique valuation satisfying
efficiency, symmetry, linearity, and the null-player property. Every data marketplace that
has tried to price contributions has started with Shapley or a close relative.

They are also fundamentally gameable. Three independent papers published in 2025-2026 prove
this definitively, each from a different angle. The result is not a minor calibration issue --
it is a structural impossibility. No amount of parameter tuning on the current formula can
fix it. The mechanism itself must change.

---

## 2. Shapley Is Broken: Three Independent Proofs

### 2.1 Shapley Fragility (arXiv:2504.05563)

**Paper**: "Do Data Valuations Make Good Data Prices?"

Claim 3 of this paper proves that strategic misrepresentation inflates Shapley values.
A contributor who knows how other traces are scored can misrepresent their own trace --
adding noise, restructuring, or splitting -- to increase their payment.

The paper evaluates the two most common data pricing mechanisms:

- **Leave-One-Out (LOO)**: Remove one trace, measure the change in corpus utility.
  Fails to incentivize truthful reporting because a contributor can craft traces that
  appear maximally impactful when removed.

- **Data Shapley**: Average marginal contribution across all possible orderings.
  Inherits the same vulnerability -- strategic misrepresentation is profitable.

The paper proposes Myerson pricing as the buyer-optimal, incentive-compatible alternative.
Myerson pricing is individually rational (contributors are never worse off participating)
and incentive-compatible (truthful reporting is optimal). However, when both sides have
private information -- the contributor knows their trace quality, TC knows the corpus
composition -- the price of anarchy is unbounded. Myerson works when only one side has
private info.

**TC implication**: The current `q = f * g * a` formula is a variant of marginal contribution
pricing. It inherits the vulnerability proven in Claim 3. A contributor who understands the
scoring pipeline can craft traces that game the quality and novelty factors.

### 2.2 Sybil Attacks on Data Shapley (arXiv:2605.07663)

This paper demonstrates a concrete attack: splitting a single contribution into two achieves
1.74x inflation of the combined payment.

The mechanism is straightforward. Shapley values are computed as average marginal
contributions. When a single trace is split into two halves, each half's marginal contribution
is computed independently. Because the marginal contribution function is typically concave
(the first unit of information is more valuable than the hundredth), two small contributions
each earn a disproportionately large share compared to one large contribution.

The 1.74x figure is not an upper bound -- it is the observed inflation for a simple
equal-weight split. More sophisticated splitting strategies (e.g., splitting into unequal
parts, or into three or more fragments) can achieve higher inflation depending on the
specific value function.

**TC implication**: A contributor with one high-quality trace can split it into two
medium-quality traces and earn 74% more credits. At TC's current scale (3 contributors,
~352 submissions), this is not yet a crisis. At scale, it breaks the credit economy.

### 2.3 Entire Semivalue Class Is Gameable (arXiv:2506.12619)

The most damaging result. This paper proves that the gameability of Shapley values is not
specific to Shapley -- it extends to the entire semivalue class:

- **Shapley values**: Gameable (per 2.1 and 2.2).
- **Banzhaf values**: Gameable. Same structural vulnerability.
- **Beta-Shapley**: Gameable. Parameterizing the weighting function does not help.
- **Any linear combination**: Gameable. There is no convex combination of semivalues
  that is incentive-compatible.

The proof is constructive: for any semivalue, there exists a misrepresentation strategy
that strictly increases the contributor's payment. The class of all semivalues is
fundamentally incompatible with incentive compatibility.

**TC implication**: Replacing Shapley with Banzhaf or beta-Shapley (which some data
marketplace designs have proposed as "more robust" alternatives) does not fix the problem.
The fix must come from outside the semivalue family entirely.

---

## 3. VCG: The DSIC Alternative

### 3.1 What VCG Provides

VCG (Vickrey-Clarke-Groves) is the canonical mechanism for dominant-strategy
incentive-compatible (DSIC) pricing. In a DSIC mechanism, truthful reporting is a dominant
strategy -- each contributor maximizes their payment by reporting accurately, regardless of
what other contributors do.

The core principle: each trace's payment equals its **externality** -- the difference in
total corpus utility with and without that trace. This is superficially similar to Shapley
(which also uses marginal contributions), but the payment rule is different. VCG pays the
externality directly, while Shapley averages over all orderings. The averaging is precisely
what creates the gameability.

For homogeneous multi-unit allocation (which approximates TC's setting when traces are
substitutable), VCG runs in O(n log n) -- sort by value, pay each winner the highest
losing bid. At TC's current throughput of ~13 traces/week, this is trivially fast.

### 3.2 TC Implementation

`vcg_allocate` already exists in TC's codebase. The function is built and exported but the
greedy allocation path dominates at runtime. Wiring VCG into credit settlement requires:

1. **Scoring phase** (unchanged): Score each trace with `q = f * g * a` to establish
   its value.
2. **Allocation phase** (new): Run `vcg_allocate` on the scored traces to determine
   which traces are accepted and at what price.
3. **Payment phase** (changed): Each accepted trace's credit is its VCG externality,
   not its raw `q` score.

The VCG externality for trace i is:

```
payment(i) = sum(q_j for j in accepted_without_i) - sum(q_j for j in accepted_with_i, j != i)
```

In words: trace i's payment is the total value of the corpus without trace i minus the
total value of the corpus with trace i (excluding i's own value). This is the damage
trace i does to everyone else by being present -- which, in a well-designed allocation,
equals the benefit trace i provides.

### 3.3 The Sybil Test

The defining test for any incentive mechanism in TC: splitting one trace into two should NOT
earn more credits under VCG.

```
Test: Let trace T have value q.
      Split T into T_a and T_b where q_a + q_b <= q.
      Compute VCG payment for T alone: p(T).
      Compute VCG payment for T_a + T_b: p(T_a) + p(T_b).
      Assert: p(T_a) + p(T_b) <= p(T).
```

Under VCG, this holds because each fragment's externality is smaller than the original
trace's externality. The concavity that makes Shapley gameable works in VCG's favor --
smaller contributions have proportionally smaller externalities.

### 3.4 Scale Considerations

At 13 traces/week, VCG is trivial. At scale:

| Throughput | VCG Approach | Notes |
|---|---|---|
| < 100/week | Exact VCG | O(n log n), milliseconds |
| 100-1000/week | Exact VCG with batching | Batch weekly, compute overnight |
| 1000-10000/week | Sampled VCG | Sample subset, extrapolate externalities |
| > 10000/week | Myerson pricing | Single-dimensional, O(n log n) |

Sampled VCG (Balkanski et al. 2017) computes externalities on random subsets and
averages. Unbiased estimator of the true VCG payment. At 1000+ traces/week, exact VCG
requires evaluating the corpus utility function O(n) times, which may be expensive if
utility evaluation involves embedding comparisons. Sampling reduces this to O(k) for
k << n samples.

---

## 4. Q-MIA / Marginal Utility Token (arXiv:2506.05379)

**Paper**: "Designing DSIC Mechanisms for Data Sharing"

This paper introduces the Marginal Utility Token (MUT), a mechanism specifically designed
for data-sharing marketplaces. MUT addresses the exact problem TC faces: pricing individual
contributions to a shared corpus in a way that is provably incentive-compatible.

### 4.1 MUT Structure

Each contributor's payment share is proportional to the product of two terms:

- **Verifiable quality** (q_i): A quality score that can be independently verified.
  In TC's case, this is the TEE-computed quality factor f.

- **Marginal utility**: The incremental value that contributor i's data adds to the
  corpus. This is the novelty dimension -- how much new information the trace provides
  beyond what the corpus already contains.

The payment formula:

```
payment(i) = budget * (q_i * mu_i) / sum(q_j * mu_j for all j)
```

Where mu_i is the marginal utility of trace i, computed as the change in corpus utility
when trace i is added.

### 4.2 Game-Theoretic Properties

MUT is provably DSIC under three conditions:

1. **Quality is verifiable**: The buyer (TC) can independently compute q_i. TC's TEE
   scoring satisfies this -- quality is computed inside the enclave, not self-reported
   by the contributor.

2. **Marginal utility is monotone**: Adding a trace to the corpus never decreases its
   utility. This holds for TC's embedding-based novelty scoring -- a new trace either
   adds information or is redundant.

3. **Budget is fixed**: The total credit pool for a period is predetermined. TC can set
   this as a weekly or monthly credit budget.

Under these conditions, MUT makes both withholding (submitting fewer traces than available)
and misreporting (manipulating trace content to game scores) strictly worse than truthful
submission.

### 4.3 Relationship to TC's Current Formula

The existing `q = f * g * a` already has the right structure:

| MUT Term | TC Term | Mapping |
|---|---|---|
| Verifiable quality q_i | f (quality factor) | Direct: TEE-computed perplexity score |
| Marginal utility mu_i | g (novelty factor) | Direct: embedding cosine distance to corpus |
| Anomaly penalty | a (anomaly penalty) | Additional: MUT does not have this, but it is compatible |

The key insight: TC's formula is already a quality-times-marginal-novelty structure. MUT
provides the game-theoretic foundation that proves this structure can be made
incentive-compatible -- provided the payment rule follows the MUT formula rather than
raw multiplication.

The difference is in the payment rule. Under the current formula, `q = f * g * a` is used
directly as the credit amount. Under MUT, `q_i * mu_i` determines the proportional share
of a fixed budget. The proportional-share mechanism is what makes it DSIC.

### 4.4 Budget Feasibility

MUT payments stay within budget by construction -- the sum of all payments equals the
budget. This is a practical advantage over VCG, where total payments can exceed or fall
short of a target budget. TC can set a weekly credit budget (e.g., 1000 NEAR) and
distribute it proportionally via MUT.

VCG and MUT are complementary: VCG determines whether a trace should be accepted (the
allocation decision), and MUT determines how much it should be paid (the payment decision).
The combined mechanism:

1. Score traces: compute f, g, a.
2. Allocate via VCG: determine the accepted set.
3. Pay via MUT: distribute the credit budget proportionally among accepted traces.

---

## 5. The Vana Emissions Trap

### 5.1 What Happened

Vana's Data Liquidity Pools (DLPs) provide a direct cautionary tale for TC's credit design.
Vana launched with a straightforward model: submit data, earn token emissions. The more data
you submit, the more tokens you earn.

The result: insufficient incentives and low data quality. Contributors optimized for volume
over quality. The token emissions were disconnected from downstream value -- a contributor
earned the same whether their data was used by ten downstream models or zero.

### 5.2 VRC-14 Correction

Vana recognized the failure and proposed VRC-14, which replaced direct emissions with
usage-linked rewards. The new allocation:

| Revenue Source | Weight | Rationale |
|---|---|---|
| Data Access Fees | 50% | Direct downstream usage -- someone paid to use the data |
| Token Trading Volume | 30% | Market signal for pool value |
| Unique Contributors | 20% | Growth and diversity metric |

The shift from "pay for submission" to "pay for usage" fundamentally changed contributor
incentives. Under direct emissions, the optimal strategy was to submit as much data as
possible, regardless of quality. Under VRC-14, the optimal strategy is to submit data that
downstream consumers will actually use and pay for.

### 5.3 TC Lesson

TC's current credit model (`q = f * g * a`) is closer to Vana's original model than to
VRC-14. Credits flow at submission time based on quality and novelty scores. There is no
mechanism linking credits to downstream usage -- whether a trace is subsequently used for
RAG queries, skill extraction, model training, or anything else.

The risk: TC develops the same pathology Vana experienced. Contributors optimize for what
the scorer rewards (high perplexity, low cosine similarity to existing traces) rather than
for what downstream consumers need (actionable traces that improve agent performance).

The fix is not to abandon submission-time scoring -- it provides immediate feedback that
contributors need. The fix is to add a usage-linked component that adjusts credits after
the fact based on how much downstream value a trace creates.

---

## 6. Ocean Protocol Cautionary Tale

Ocean Protocol pursued a pure-marketplace approach: wrap data in ERC-20 datatokens, trade
them on automated market makers (AMMs), let price discovery determine data value. The model
is theoretically elegant -- AMMs provide continuous liquidity and price signals.

As of 2026, the reality is less favorable. Liquidity is "still building," regulatory clarity
on data-as-a-token is lacking, and the AMM model has not achieved the self-sustaining
liquidity that DeFi AMMs enjoy for fungible tokens. The fundamental problem: data is
heterogeneous and non-fungible. An AMM designed for fungible token pairs does not naturally
accommodate the fact that one dataset may be vastly more valuable than another for a specific
downstream task.

**TC lesson**: Do not attempt to build a general-purpose data marketplace or AMM. TC's
advantage is that it has a specific, well-defined corpus (AI coding agent traces) with
specific downstream uses (RAG, skill extraction, model training). Price traces based on
measured utility, not on speculative trading. The marketplace abstraction adds complexity
without solving TC's core problem of incentive alignment.

---

## 7. Credibility Trilemma (arXiv:2605.26604)

### 7.1 The Trilemma

This paper proves that ghost-bid deviations are profitable and undetectable under both
sealed-bid VCG and Myerson mechanisms. A ghost-bid deviation is when a participant submits
a fake bid (or withholds a real one) to manipulate the outcome.

The result is a trilemma: no mechanism can simultaneously achieve:

1. **Incentive compatibility**: Truthful reporting is optimal.
2. **Privacy**: Bids are not revealed to other participants.
3. **Credibility**: The mechanism operator cannot profitably deviate.

Under sealed-bid VCG, the operator can insert ghost bids to inflate prices. Under Myerson,
the operator can suppress bids to manipulate the reserve price. Both deviations are
undetectable to participants because bids are sealed.

### 7.2 The Only Closure: Broadcast Commitment

The paper proves that the ONLY way to close the trilemma is **broadcast commitment** --
a mechanism where all bids (or a cryptographic commitment to all bids) are publicly visible
before the allocation is computed. This eliminates ghost-bid deviations because any inserted
or suppressed bid would be detectable.

### 7.3 TC's TEE as Credibility Mechanism

TC's TEE-based scoring is not just a privacy feature -- it is the mechanism design feature
that closes the credibility trilemma.

The TEE provides:

- **Verifiable computation**: The scoring code runs inside the enclave. The operator
  cannot modify scores after the fact.
- **Attestation**: The enclave produces a cryptographic attestation that the published
  scoring code was the code that actually ran.
- **Input integrity**: Traces enter the enclave encrypted and are scored without the
  operator seeing raw content.

This is functionally equivalent to broadcast commitment: the TEE acts as a trusted third
party that commits to the scoring function before seeing the inputs. Ghost-bid deviations
(the operator inserting fake traces to manipulate scores) are detectable because the enclave
logs all inputs.

**Implication**: TC's TEE architecture is load-bearing for incentive compatibility, not just
for privacy. Removing the TEE (e.g., for cost reasons or to simplify deployment) would
reopen the credibility trilemma. Any cost-optimization of the TEE infrastructure must
preserve the attestation and input-integrity properties.

---

## 8. Staking as Sybil Resistance

### 8.1 The Sybil Problem

Section 2.2 showed that Shapley values are vulnerable to Sybil attacks (1.74x inflation from
splitting). VCG and MUT are structurally resistant to Sybil attacks (section 3.3), but
resistance is not immunity. A sufficiently sophisticated Sybil attack might find edge cases
in the utility function where splitting is marginally profitable.

Staking provides an economic backstop: even if a Sybil attack is mechanistically possible,
the cost of staking multiple identities makes it economically unprofitable.

### 8.2 Staking Mechanism

| Phase | Action | Stake Status |
|---|---|---|
| Submission | Contributor stakes S NEAR tokens per trace | Locked |
| Scoring | TEE scores the trace (f, g, a) | Locked |
| Acceptance | Trace accepted by gate | Locked |
| Settlement | Credits distributed via VCG/MUT | Stake returned + credits |
| Rejection | Trace rejected by gate (low quality) | Stake returned, no credits |
| Fraud detection | Trace found to be fabricated or duplicated | **Stake slashed** |

The slashing condition is the critical element. Rejection for low quality returns the stake
-- the contributor made an honest attempt. Slashing occurs only for demonstrable fraud:
fabricated traces (generated content, not from a real agent session) or duplicated traces
(same content submitted multiple times, possibly with minor perturbations).

### 8.3 Stake Sizing

The stake must be large enough to make Sybil attacks unprofitable but small enough to not
deter legitimate contributors.

```
Sybil breakeven: S > (inflation_factor - 1) * expected_credit
At 1.74x inflation (section 2.2): S > 0.74 * expected_credit
```

If the expected credit per trace is 10 NEAR, the stake must exceed 7.4 NEAR to make the
1.74x Sybil attack unprofitable. Setting S = expected_credit (1:1 stake-to-credit ratio)
provides margin while remaining accessible.

### 8.4 Cold-Start Exception

At TC's current scale (3 contributors, ~352 submissions), requiring staking would deter
new contributors. Staking should be introduced after the contributor base is established --
Phase 4 in the recommended design (section 10).

---

## 9. Usage-Linked Credits

### 9.1 Why Submission-Time Scoring Is Insufficient

The Vana emissions trap (section 5) demonstrates that submission-time scoring alone creates
perverse incentives. TC's current model pays at submission time based on quality and novelty.
There is no feedback loop from downstream usage.

A trace that scores well on quality and novelty but is never used for RAG queries, skill
extraction, or model training has created no downstream value. A trace that scores modestly
but is retrieved hundreds of times for downstream tasks has created substantial value. The
credit system should eventually reflect this.

### 9.2 Usage Signals

TC can track several downstream usage signals:

| Signal | Measurability | Latency | Weight |
|---|---|---|---|
| RAG query retrievals | Direct (query logs) | Days | High |
| Skill extraction citations | Direct (extraction pipeline) | Weeks | High |
| Model training inclusion | Indirect (dataset manifests) | Months | Medium |
| API access fees | Direct (billing) | Days | Highest |
| Research citations | Indirect (publication search) | Months | Low |

### 9.3 Proposed Weighting

Drawing from Vana's VRC-14 correction, adapted for TC's context:

| Component | Weight | Source |
|---|---|---|
| Downstream access fees | 50% | Direct revenue from trace usage |
| Usage frequency | 30% | RAG retrievals + skill extraction citations |
| Submission quality score | 20% | TEE-computed `q = f * g * a` (or VCG equivalent) |

This inverts the current model. Today, 100% of credits are determined at submission time.
Under the proposed model, 80% of credits are determined by downstream usage. The submission-
time score provides immediate feedback (contributors know within minutes whether their trace
was accepted) while the usage-linked component adjusts credits over time.

### 9.4 Settlement Cadence

Usage-linked credits require a settlement delay. Proposed cadence:

- **Immediate**: 20% of estimated credits at submission (based on quality score).
- **Weekly**: Adjust based on first week of usage data.
- **Monthly**: Final settlement including access fees and usage frequency.

Contributors see immediate value (20% at submission) while the system accumulates usage
data for accurate pricing. The delay is similar to payment processing in traditional
marketplaces -- contributors understand that final payment depends on actual usage.

---

## 10. Recommended Design: Four Phases

### Phase 1: Wire VCG (Weeks)

`vcg_allocate` is already built. The work is wiring, not building.

**Tasks**:

1. Replace the greedy allocation path with `vcg_allocate` in credit settlement.
2. Compute VCG externality-based payments instead of raw `q` scores.
3. Run the Sybil test (section 3.3): split traces, verify no inflation.
4. A/B test VCG payments against current `q`-based payments on historical data.
5. Deploy to production credit settlement.

**Validation**: The Sybil test is the acceptance criterion. If splitting a trace into two
earns more under VCG than submitting the original, the implementation is wrong.

**Risk**: VCG may produce payments that differ significantly from current `q`-based
payments. Some contributors may see credits increase, others decrease. Communicate the
change and the rationale (incentive compatibility) before deployment.

### Phase 2: MUT Structure (1-2 Months)

Adopt the explicit Marginal Utility Token formulation from arXiv:2506.05379.

**Tasks**:

1. Define a weekly credit budget (total NEAR allocated to trace credits per week).
2. Implement proportional payment: `payment(i) = budget * (q_i * mu_i) / sum(q_j * mu_j)`.
3. Replace per-trace VCG externality with MUT proportional share.
4. Verify budget feasibility: total payments must equal the budget.
5. Verify DSIC property: simulate misrepresentation strategies, confirm they are
   strictly worse than truthful submission.

**Validation**: Budget feasibility (total payments = budget) and DSIC simulation (no
profitable deviation found in 1000 random misrepresentation strategies).

**Risk**: The fixed-budget model changes the economics. Under the current model, each
trace earns based on its absolute quality. Under MUT, each trace earns based on its
relative quality within the weekly cohort. A mediocre trace submitted in a weak week earns
more than a good trace submitted in a strong week. This is correct behavior (the mediocre
trace has higher marginal utility in a weak cohort) but may confuse contributors.

### Phase 3: Usage-Linked Credits (3-6 Months)

Track downstream usage and adjust credits accordingly (section 9).

**Tasks**:

1. Instrument the RAG query pipeline to log which traces are retrieved.
2. Instrument the skill extraction pipeline to log source traces.
3. Implement the 50/30/20 weighting (access fees / usage / quality).
4. Build the settlement pipeline: immediate 20%, weekly adjustment, monthly final.
5. Build the contributor dashboard showing usage analytics per trace.

**Validation**: Credits for high-usage traces should exceed credits for low-usage traces
of similar quality. The Vana test: contributors should not be able to increase credits
by submitting more traces of marginal quality.

**Risk**: Usage data is sparse in early months. The 50% access-fees component may be
near-zero until downstream consumers adopt TC's API. Mitigation: use the 20% submission
quality as a floor -- no contributor earns less than their quality score warrants,
regardless of usage.

### Phase 4: Staking + Broadcast Commitment (6-12 Months)

Full incentive-compatible design with Sybil resistance and credibility closure.

**Tasks**:

1. Implement contributor staking (section 8): stake S NEAR per trace.
2. Implement slashing for demonstrable fraud (fabrication, duplication).
3. Implement broadcast commitment: publish cryptographic commitment to all traces in
   a scoring batch before computing scores.
4. Verify credibility trilemma closure: ghost-bid deviations are detectable.
5. Publish the mechanism design as a specification for other trace registries.

**Validation**: End-to-end incentive compatibility test:

- Sybil test passes (section 3.3).
- Misrepresentation test passes (MUT simulation).
- Ghost-bid test passes (broadcast commitment detection).
- Staking economics test passes (Sybil attack is unprofitable at S = expected_credit).

**Risk**: Staking creates a barrier to entry. New contributors must acquire NEAR tokens
before they can submit traces. Mitigation: provide a staking subsidy for first-time
contributors (e.g., TC stakes on their behalf for the first 10 submissions).

---

## 11. Relationship to Other v6 Documents

| v6 Doc | Relationship |
|---|---|
| 02 (Scoring Pipeline) | This document replaces the credit settlement logic downstream of scoring. Section 02's `q = f * g * a` remains as the scoring function; this document changes how scores translate to credits. |
| 04 (Production Hardening) | VCG wiring and staking require production infrastructure changes. Settlement pipeline (Phase 3) requires new background jobs. |
| 05 (Strategy & Grants) | Incentive-compatible design is a differentiator for grant applications. NLnet and NEAR DevHub both value mechanism design rigor. |
| 09 (Conformal Gate Calibration) | The gate (accept/reject) is upstream of credit settlement. Conformal calibration determines which traces enter the VCG/MUT allocation. At N=3, the gate is also the primary anti-collusion mechanism (section 13). |
| 10 (Ground-Truth-Free Quality) | Quality estimation feeds the f factor in VCG/MUT. Better quality estimates produce better incentive alignment. |
| 13 (IronClaw Provenance Attestation) | Session-level attestations close the re-attribution collusion vector: a session's output cannot be claimed by multiple contributors. Works alongside the quality gate as the collusion defense at single-digit N (section 13). |

---

## 12. What TC Must NOT Do

1. **Do not keep the current formula unchanged.** Three independent proofs (section 2) show
   Shapley-inspired pricing is gameable. The question is not whether it will be gamed, but
   when. At 3 contributors, the trust model holds. At 30, it will not.

2. **Do not replace Shapley with Banzhaf or beta-Shapley.** The entire semivalue class is
   gameable (section 2.3). Switching to a different semivalue is rearranging deck chairs.

3. **Do not build an AMM or data marketplace.** Ocean Protocol's experience (section 6)
   shows that pure-marketplace liquidity for heterogeneous data is extremely difficult to
   bootstrap. TC has a specific corpus with specific downstream uses -- price based on
   measured utility, not speculative trading.

4. **Do not tie credits purely to submission volume.** Vana's pre-VRC-14 experience
   (section 5) shows this creates a race to the bottom on quality.

5. **Do not remove the TEE.** The credibility trilemma (section 7) proves that the TEE is
   load-bearing for incentive compatibility, not just for privacy. Any cost optimization
   must preserve attestation and input integrity.

---

## 13. Collusion Resistance at Single-Digit N

At TC's current scale of 3 contributors, collusion is a structurally different problem
from the one that incentive mechanism design normally addresses. This section treats it
as a first-class concern rather than a footnote.

### 13.1 What DSIC Mechanisms Actually Guarantee

VCG and MUT are DSIC: under unilateral deviation, each contributor's dominant strategy
is truthful submission. "Unilateral" is the key qualifier. The guarantee holds when one
contributor deviates while the others do not.

This is not collusion resistance. DSIC says nothing about coordinated deviations where
all N contributors agree on a joint strategy. At N=3, a colluding coalition can be the
entire contributor base.

### 13.2 Sybil vs. Genuine Collusion

These are distinct threats and require distinct countermeasures.

**Sybil attacks** (one actor, many identities):

- One contributor registers multiple accounts and submits the same trace split across
  identities to inflate Shapley/semivalue payments.
- False-name-resistant semivalues (arXiv:2605.07663) address this by designing the
  payment function so that splitting a contribution across identities is never
  profitable.
- Staking raises the Sybil breakeven: S NEAR must be staked per identity, so the
  inflated payment must exceed the total stake cost across all fake identities.
- VCG is structurally resistant to Sybil splitting (section 3.3).

**Genuine multi-party collusion** (multiple distinct contributors coordinating):

- Three separate contributors agree to submit traces in a coordinated pattern -- e.g.,
  timing submissions to maximize each contributor's marginal novelty score, or
  artificially inflating each other's quality attestations.
- False-name-resistant semivalues do NOT help: the contributors are genuinely distinct
  actors, not one actor with many identities.
- Staking does NOT help: each contributor can stake independently and still coordinate.
  The staking cost does not increase with the degree of coordination.
- No known payment mechanism is robustly collusion-proof at N=3 when all participants
  can collude. This is a fundamental result, not a gap in the literature.

### 13.3 What Actually Resists Collusion at N=3

Given that the payment mechanism cannot close this gap, the defense must come from the
quality layer.

**Quality gates (doc 09 -- Conformal Gate Calibration)**:

The conformal gate accepts or rejects traces based on quality scores computed inside the
TEE. Coordinating contributors cannot change what the TEE computes. If collusion produces
low-quality traces (e.g., synthetic or fabricated content), the gate rejects them --
collusion earns nothing. If collusion produces genuinely high-quality traces, the corpus
benefits -- collusion is not a problem.

The gate is the primary anti-manipulation mechanism at N=3.

**Provenance attestation (doc 13 -- IronClaw-signed sessions)**:

IronClaw-signed session attestations bind each trace to a specific agent session, tool
call sequence, and TEE execution record. Coordination that involves submitting traces
from the same underlying session under multiple identities is detectable: the session
attestation is unique and cannot be reused.

This closes the specific collusion vector of re-attributing a single session's output
to multiple contributors.

**Staking raises Sybil cost, not collusion cost**:

Staking is still worth implementing (Phase 4) for Sybil resistance. But it should not
be oversold as collusion resistance. The two problems require different solutions.

### 13.4 When to Revisit Payment-Level Collusion Resistance

Payment-mechanism-level collusion resistance becomes meaningful when N grows past ~10.
At that scale:

- Coordinating all contributors requires significant off-chain communication, which is
  observable.
- The colluding coalition is no longer the entire contributor base -- non-colluding
  contributors provide a constraint.
- Mechanism design tools such as coalition-proof Nash equilibria and correlated
  equilibria become applicable.

Until N exceeds ~10, the correct investment is in quality gates and provenance
attestation, not in increasingly complex payment schemes.

---

## 14. Open Questions

1. **Budget sizing**: How large should the weekly credit budget be? Too small and
   contributors are not motivated. Too large and TC burns through its NEAR allocation.
   The budget should scale with downstream revenue (access fees), but in early stages
   there may be no downstream revenue. A fixed subsidy with planned taper is likely
   needed.

2. **Cross-period utility**: MUT computes marginal utility within a settlement period.
   A trace submitted in week 1 that becomes highly valuable in week 10 (because a new
   downstream use case emerges) does not retroactively earn more under MUT. Usage-linked
   credits (Phase 3) partially address this, but the interaction between MUT proportional
   payments and usage-linked adjustments needs careful design.

3. **Multi-agent traces**: When a trace involves multiple agents (e.g., an orchestrator
   delegating to sub-agents), how should credit be apportioned? VCG and MUT both assume
   single-contributor traces. Multi-agent trace stitching (doc 03, A2A protocol) may
   require an extension to the mechanism.

   Shapley is the principled split for orchestrator+sub-agent (A2A) credit attribution:
   it is the unique allocation satisfying efficiency, symmetry, and the null-player
   property across all agents in a coalition. However, exact Shapley computation costs
   O(2^N) in the number of agents, which is prohibitive for large orchestrations.

   Two recent algorithms make Shapley estimation tractable:

   - **Owen Sampling** (arXiv:2508.21261): A sampling-based algorithm that estimates
     Shapley values via random coalition orderings, reducing cost from O(2^N) to O(k)
     for k samples. Directly applicable to A2A credit splits where N is the number of
     sub-agents in a session.
   - **Data-Banzhaf** (arXiv:2506.12619, adapted): An alternative semivalue estimator
     with lower variance per sample, useful when Owen Sampling variance is too high at
     small sample counts.

   However, a practical warning applies: contribution metrics are volatile round-to-round
   (arXiv:2405.08044). A sub-agent's measured Shapley value can swing substantially
   across similar task runs due to ordering effects and marginal-contribution variance.
   At N=3 total contributors, this volatility is a real risk -- single-run Shapley
   estimates may not be reliable enough to drive payments.

   **Recommendation**: Use Shapley (via Owen Sampling) for attribution reporting
   (display purposes, leaderboards, provenance), but cap payment variance with smoothing
   -- e.g., exponential moving average of Shapley estimates across recent sessions before
   converting to credits.

4. **Collusion**: VCG and MUT are DSIC under unilateral deviation -- one contributor
   cannot profitably deviate alone. They do NOT guarantee resistance to collusion
   (multiple contributors coordinating their strategies).

   Research finding B5 (arXiv:2605.07663 extended analysis) establishes this precisely:
   **no payment mechanism is robustly collusion-proof when N=3 and all three participants
   can collude.** When the entire contributor base is the colluding coalition, the
   coalition captures the full surplus regardless of the payment rule, because there is no
   non-colluding majority to constrain them.

   It is important to distinguish two separate threats:

   - **Sybil attacks** (one actor, many identities): false-name-resistant semivalues
     (arXiv:2605.07663) address this directly. Staking raises the Sybil breakeven cost
     further. VCG is structurally resistant to Sybil splitting (section 3.3).
   - **Genuine multi-party collusion** (multiple distinct actors coordinating): Sybil
     countermeasures do NOT help here. Staking raises the Sybil breakeven but NOT the
     collusion breakeven -- coordinating contributors can stake independently and still
     coordinate their submission strategy.

   The honest conclusion at TC's current scale: **the primary defense against collusion
   is NOT the payment mechanism but the quality gates and provenance system** (docs 09
   and 13). If coordinating contributors submit low-quality traces together, the conformal
   gate rejects them regardless of coordination. If they submit high-quality traces
   together, the coordination has not harmed the corpus -- it has improved it. Clever
   payment mechanisms add marginal collusion resistance at N=3 and are not worth the
   complexity until N grows past ~10, where coordination becomes harder to sustain.

---

## 15. Verification Ledger

| Item | arXiv / Source | Status |
|---|---|---|
| Shapley Fragility ("Do Data Valuations Make Good Data Prices?") | arXiv:2504.05563 | **Verified** |
| Sybil Attacks on Data Shapley; false-name-resistant semivalues | arXiv:2605.07663 | **Verified** |
| Entire Semivalue Class Gameable | arXiv:2506.12619 | **Verified** |
| Q-MIA / Marginal Utility Token | arXiv:2506.05379 | **Verified** |
| Credibility Trilemma | arXiv:2605.26604 | **Verified** |
| Owen Sampling for Shapley estimation (A2A credit attribution) | arXiv:2508.21261 | **Verified** |
| Contribution metric volatility round-to-round | arXiv:2405.08044 | **Verified** |
| Vana VRC-14 | VRC-14 governance proposal | **Verified** |
| Ocean Protocol AMM/Datatoken | Ocean Protocol documentation | **Verified** |
| VCG Mechanism | Vickrey 1961, Clarke 1971, Groves 1973 | **Established method** |
| Myerson Optimal Auction | Myerson 1981 | **Established method** |

*7 verified arXiv papers, 2 verified project references, 2 established methods. Last updated August 2026 (v6).*
