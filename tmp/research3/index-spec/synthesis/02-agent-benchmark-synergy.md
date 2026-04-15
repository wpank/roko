# 02 -- Agent-Benchmark Synergy

> How an AI agent-orchestration runtime and a regulated financial benchmark
> are not two products but one self-reinforcing system. This document is
> self-contained. No prior knowledge of Nunchi, Korai, Roko, or ISFR is
> assumed.

---

## 0. Definitions for the Uninitiated

**Roko** is a Rust toolkit (~177K LOC, 18 crates) for building AI agents that
build themselves. It reads product requirements, generates implementation
plans, dispatches LLM-backed agents to execute tasks, validates results through
automated gate pipelines, and learns from each cycle. The defining property is
self-hosting: Roko is the tool that develops Roko.

**Korai** is a purpose-built Layer-1 blockchain designed for agent-native
computation, on-chain knowledge storage, and decentralized context engineering.

**ISFR** (Index of Structured Finance Rates) is a proposed regulated financial
benchmark -- specifically, a Yield-Bearing Stablecoin Reference Rate
(ISFR-YBS) intended to serve as the money-market-fund benchmark for crypto
dollars. It would be administered under UK BMR (Benchmark Regulation)
authorization, following the path CF Benchmarks took in 2019-2020 under FCA
Cat-6 registration.

**The compound thesis** is that these three things are inseparable. The agent
runtime is not an R&D project adjacent to the benchmark business. It is the
cheapest, most auditable, and most defensible way to *operate* a regulated
benchmark. The benchmark is not a financial product adjacent to the agent
platform. It is the highest-stakes, highest-margin *customer* of the agent
platform. Every dollar spent improving one side pays dividends on the other.

---

## 1. The Compound Thesis

The traditional index business is a toll booth: methodology + data +
calculation + licensing. MSCI's Index segment runs at approximately 76%
adjusted EBITDA margin on roughly $1.6B in annual revenue. S&P Dow Jones
Indices runs at comparable scale. FTSE Russell generated GBP 918M. The
economics are extraordinary because once an ETF prospectus hardwires an index,
switching benchmarks is functionally impossible.

But the *operations* of an index business are fragile. LIBOR's collapse --
$9B+ in fines, Tom Hayes's 14-year sentence -- was caused by five governance
failures: submission-based rather than transaction-based inputs, an industry
trade group (BBA) rather than an independent regulated entity, traders sitting
next to submitters, no audit trail, and panel banks with direct economic
incentive to lie. Even SOFR, which replaced LIBOR by anchoring in over $1T
daily repo volume, depends on centralized collection by the NY Fed with opaque
operational infrastructure.

The compound thesis is that an agent-orchestration runtime solves every one of
these operational fragilities, and a regulated benchmark gives that runtime its
highest-value production workload:

- **Transaction-anchored, not opinion-anchored**: Roko agents read on-chain
  borrow/lend events and perp funding settlements directly from smart
  contracts. No panelist submissions. Every input is an observable transaction.
- **Independent, auditable administration**: Agent execution traces are
  cryptographically logged. Every methodology computation has a verifiable
  replay path. IOSCO Principle 17 (audit trail) is structurally satisfied
  rather than procedurally approximated.
- **No human-in-the-loop manipulation surface**: The methodology is a
  deterministic pipeline of agents operating under role-specific permission
  constraints (the 28-role taxonomy, from Conductor to PerformanceSentinel,
  each with explicit read/write/exec/git/network permissions). A Reviewer
  role with `write=false` cannot alter data. An Implementer role cannot access
  the network.
- **Continuous self-improvement under governance bounds**: The system learns
  from each index calculation cycle via cascade routing, prompt experiments,
  adaptive gate thresholds, and section-effectiveness tracking -- but only
  within bounds approved by the Independent Oversight Committee.

The benchmark needs the agent stack to be credible. The agent stack needs the
benchmark to have a production workload worth hundreds of millions in
referenced notional. They compound.

---

## 2. Agent-Driven Index Calculation

### MacNet-Topology Fleets

Qian et al.'s **MacNet** (arXiv 2406.07155, June 2024, revised March 2025)
demonstrated that multi-agent systems follow a collaborative scaling law:
performance grows logistically with agent count, and **small-world network
topologies dominate** over fully-connected, star, or chain topologies. The
critical finding is a concrete *k*-agent budgeting heuristic: there exists an
optimal fleet size beyond which adding agents produces diminishing returns,
and this optimum depends on topology.

For ISFR index calculation, this translates directly. A fleet of Roko agents
-- each a DSPy/GEPA-optimized program (see Section 8) -- computes ISFR-YBS
sub-indices in parallel:

- **ISFR-YBS-T** (T-bill / RWA-backed: USDY, OUSG, USD0, USDtb)
- **ISFR-YBS-L** (lending-based: aUSDC, aUSDT, syrupUSDC, sFRAX)
- **ISFR-YBS-D** (delta-neutral / basis: sUSDe, Falcon, Resolv)
- **ISFR-YBS-S** (savings-rate composites: sUSDS, sDAI)

Each sub-index is computed by an independent agent cluster. MacNet's scaling
law tells the IOC exactly how many agents to fund per sub-index and which
topology to use. Roko's existing `ParallelExecutor` -- a pure state machine
that emits `DispatchAgent` and `RunGate` actions without performing I/O --
already supports this pattern. It reads a DAG of tasks, dispatches agents in
parallel via Tokio `JoinSet`, and applies gate pipelines to each result.

### CRDT-Shared State

**CodeCRDT** (arXiv 2510.18893, October 2025) provides conflict-free
replicated data types for multi-agent collaboration with **provable
at-most-one-winner safety**. When multiple agents compute overlapping portions
of an index, CRDT merge semantics guarantee deterministic convergence without
a central coordinator. Korai finalizes the merge: each agent posts its partial
result as an on-chain transaction, and the CRDT merge function produces the
canonical index value.

This matters for regulatory credibility. The IOSCO Principles require that
methodology computation be reproducible and deterministic. A CRDT-based
multi-agent calculation is both -- any third party can replay the individual
agent computations and verify that the merge produces the same result.

### DGM Self-Improvement

Zhang, Hu, Lu, Lange, and Clune's **Darwin Godel Machine** (DGM; arXiv
2505.22954, May 2025, revised March 2026) achieved dramatic gains on
SWE-bench (20.0% to 50.0%) and Polyglot (14.2% to 30.7%) through
archive-based self-improvement. The mechanism: agents propose modifications to
their own code, evaluate the modifications against a benchmark, and retain
successful variants in an archive that seeds future evolution.

For ISFR, DGM-style self-improvement evolves the methodology computation
itself within IOC-approved bounds. The archive is on-chain Korai storage. Each
agent variant is an NFT-bound lineage with a verifiable benchmark score.
**OMNI-EPIC** (the DGM's open-ended quality-diversity component) generates
synthetic stress-test scenarios -- regulatory shocks, market dislocations,
Kelp-style constituent failures (the April 18, 2026 rsETH bridge drain caused
$292M in direct losses and $236M in cascaded bad debt across Aave, Compound,
and Euler) -- that the index must remain robust to. Methodology evolution is
not unconstrained: it operates within a sandbox of IOC-specified invariants
(inclusion thresholds, constituent caps, volume filters), and every proposed
change goes through the full gate pipeline (compilation, linting, tests, LLM
judge review, integration tests) before entering the archive.

---

## 3. Autonomous Methodology Validation

### GEPA Reflective Evolution

Agrawal et al.'s **GEPA** (Generalized Evolutionary Prompt Optimization;
arXiv 2507.19457, July 2025, ICLR 2026 Oral) achieved +10% over MIPROv2 on
AIME-2025 and +6-20% over GRPO with 35x fewer rollouts. GEPA is the
optimizer inside DSPy 3.0 (28K+ GitHub stars as of mid-2026), which compiles
natural-language specifications into optimized LLM programs with
JSON-serializable, hash-stable representations.

For ISFR, GEPA reflective evolution rewrites methodology prompts against a
regulator-graded evaluation suite. The methodology pipeline -- from raw data
ingestion through volume-weighted median computation to outlier filtering --
is expressed as a DSPy program. GEPA optimizes this program against historical
data and adversarial scenarios. Every optimization step produces a new
hash-stable artifact stored on Korai as an immutable methodology version with
cryptographic provenance and a falsifiable backtested justification.

This directly addresses **IOSCO Principle 10** (periodic review of
methodology) and **IOSCO Principle 12** (changes to methodology): the review
is continuous, the changes are versioned, and the justification is machine-
verifiable.

### Inspect AI Eval Suites

**Inspect AI** (UK AI Safety Institute, open-sourced May 2024; 200+ pre-built
evaluations; adopted by METR, Apollo Research, and the US Center for AI Safety
and Innovation) provides a regulator-grade evaluation substrate. ISFR uses
Inspect AI to build evaluation suites for agent-attested data sources:

- **Accuracy evals**: Do the agents correctly read on-chain data?
- **Manipulation resistance evals**: Can adversarial inputs cause agents to
  produce incorrect index values?
- **Consistency evals**: Do independent agent runs converge to the same result?
- **Latency evals**: Do agents meet the 16:00 UTC daily fixing deadline?

The UK AISI's adoption of Inspect AI makes it the de facto standard for
government-facing AI evaluation. Using the same framework for ISFR methodology
validation creates a natural alignment with the FCA's expectations for
benchmark administration.

### SWE-ABS Adversarial Strengthening

**SWE-ABS** (arXiv 2603.00520, 2026) is an adversarial benchmark-strengthening
methodology that prevents gaming of evaluation frameworks. Berkeley RDI's 2026
research showed that 8 major agent benchmarks (SWE-bench Verified,
Terminal-Bench, GAIA, OSWorld, WebArena) can be exploited to near-perfect
scores via leaked references and prompt-injectable judges -- evaluation
frameworks themselves are attack surfaces.

ISFR applies SWE-ABS-style adversarial strengthening to the methodology
itself. Red-team agents attempt to game the index calculation (e.g., by
constructing inputs that exploit the volume-weighted median's edge cases).
Successful attacks are converted into regression tests. The methodology
evolves specifically to resist attacks that have been demonstrated to work.
This is methodology contestation resistance built into the operational
pipeline rather than addressed post-hoc.

---

## 4. Agent-Attested Data Sources

### ERC-8004 Registered Agents

**ERC-8004** (De Rossi/MetaMask, Crapis/Ethereum Foundation, Ellis/Google,
Reppel/Coinbase; EIP draft August 2025) establishes trustless on-chain agent
registries. By late 2025, approximately **106,996 agents** were indexed across
Base, BSC, and Ethereum. ERC-8004 gives every Roko agent a verifiable on-chain
identity: a registered agent that fetches Aave's `getReserveData()` has an
immutable record of its code hash, its operator, and its historical
attestation accuracy.

For ISFR, each input data feed -- price, on-chain TVL, governance vote,
off-chain proof-of-reserves -- is fetched by an ERC-8004-registered Roko
agent. The fetch-and-process trajectory is logged in Inspect AI format and
posted to Korai. This creates an audit trail that exceeds what any TradFi
benchmark administrator can achieve: not just "what data entered the
calculation" but "which specific agent fetched it, what code that agent was
running, and what the agent's historical accuracy is."

### x402 Micropayments

**x402** (Coinbase, May 2025; +10,000% month-over-month growth in October
2025, >900K weekly settlements; x402 Foundation co-founded with Cloudflare;
integrations with Visa, Google, Stellar, and Solana; v2 released December
2025) is the emerging standard for agent-to-agent and agent-to-service
micropayments. Each data fetch by a Roko agent is an x402-paid transaction:
the agent pays the data source (an on-chain protocol's API, a proof-of-
reserves endpoint, a price oracle) a micropayment for the data.

This creates two properties that regulators value: (1) an economic audit trail
-- every data point has a verifiable cost, which makes the methodology's
operational expenses transparent under IOSCO Principle 4 (control framework);
and (2) a Sybil-resistance mechanism -- fabricating data requires paying for
it, making large-scale data poisoning economically irrational.

### Eigen-AVS Intersubjective Validation

Yuan et al.'s work on **intersubjective validation** (arXiv 2504.13443, April
2025) provides a non-naive reputation primitive that improves on
Sybil-vulnerable Eigentrust. In the Eigen-AVS (Actively Validated Service)
model, multiple independent agents attest to the same data point, and the
attestations are cross-checked before the data enters the index calculation.

For ISFR-YBS, *N* independent Roko agents each fetch the same on-chain borrow
rate from Aave V3. Their attestations are compared. If fewer than a
configurable quorum agree (e.g., 2/3 supermajority), the data point is flagged
for manual review. This is **IOSCO Principle 7** (data sufficiency) and
**IOSCO Principle 17** (audit trail) with cryptographic guarantees: the
attestation disagreement itself is a verifiable on-chain event.

---

## 5. HDC as Universal Substrate

### Hyperdimensional Computing Primer

Hyperdimensional computing (HDC) represents information as high-dimensional
vectors (typically 10,000 dimensions) drawn from a carefully chosen algebraic
structure. Three operations -- binding (element-wise multiplication),
bundling (element-wise addition), and permutation (coordinate shift) --
compose to encode arbitrarily complex structured data into fixed-size vectors
that support approximate nearest-neighbor search via cosine similarity.

### The Patentable Moat Anchor

The on-chain HDC similarity-search precompile is the single most defensible
technical bet in the entire system. **No public prior art exists** for a
blockchain precompile that performs HDC similarity search natively. This means:

1. **Patent-eligible**: A provisional patent application covering the
   precompile's instruction set, the on-chain vector storage format, and the
   approximate nearest-neighbor query protocol has no blocking references in
   the prior art landscape.
2. **Structurally defensible**: Any competitor chain that wants to offer
   equivalent functionality must either license the patent or implement a
   workaround that is necessarily less efficient (since the precompile
   operates at the EVM execution layer, not in smart-contract bytecode).
3. **Uniquely enabling for ISFR**: The precompile lets any ISFR consumer
   query "which historical market conditions are most similar to the current
   state?" -- a query no other L1 can answer natively.

### Supporting Research

**Torchhd** (Heddes et al., arXiv 2205.09208, published in JMLR 2023)
provides GPU-accelerated HDC operations with **100x speedup** over
CPU baselines. Roko uses Torchhd for episodic-to-semantic memory distillation
today.

**VSA-Lisp** (Hanley, Tomkins-Flanagan, and Kelly; arXiv 2511.08767, November
2025) demonstrates that a Turing-complete Lisp can be encoded entirely in FHRR
(Fourier Holographic Reduced Representation) and Residue HDC algebras. This
means Roko agent skills -- executable programs -- can be represented as bound
hypervectors, stored on-chain, and composed via HDC operations. An agent
doesn't retrieve a skill from storage and then execute it; the skill *is* a
hypervector, and executing it *is* a sequence of HDC operations. This collapses
the distinction between data and computation in a way that is native to the
chain's precompile.

**SRMU** (Streaming Resonator Memory Update; arXiv 2604.15121, 2026) extends
HDC memory to streaming contexts, enabling Roko's knowledge store to
incorporate new information without full recomputation. This is critical for
the daily ISFR fixing: the agent fleet must update its memory with today's
market data without replaying the entire history.

---

## 6. Active Inference as Planning and Economic Accounting

### Free Energy Minimization

Active inference, grounded in Karl Friston's free energy principle, frames
agent behavior as minimizing the divergence between predicted and observed
states. An agent that expects a certain borrow rate and observes a different
one experiences "surprise" (technically, variational free energy). The agent
then acts to reduce this surprise: either by updating its model (perceptual
inference) or by changing the world to match its expectations (active
inference).

### Agent Compute Budgeting

The critical insight for ISFR is that free energy and economic cost share
units. Both measure work (in the thermodynamic sense). This means Korai can
account for agent computation, x402 payments, and learning in a single
mathematical framework:

- **Information gain** from fetching a data source is measured in nats
  (natural units of information).
- **Economic cost** of the fetch is measured in x402 micropayments.
- **Expected free energy** (the planning quantity in active inference) combines
  both: it is the expected information gain minus the expected cost, plus a
  term for the agent's prior preferences about outcomes.

The cascade router in Roko already performs a simpler version of this
calculation: it uses a LinUCB contextual bandit to select which model to use
for each task, balancing gate-pass rate (information gain) against token cost
(economic cost). Upgrading to a full active-inference formulation means the
router also accounts for epistemic value (how much the agent will *learn* from
this fetch, improving future performance) and pragmatic value (how much the
fetch reduces uncertainty about the index value).

This is directly relevant to IOSCO Principle 4 (control framework): the
methodology's operational cost is not an opaque line item but a
mathematically justified allocation of compute budget across data sources.

### Caveats on VERSES AXIOM

**VERSES AXIOM** (Heins et al., arXiv 2505.24784, May 2025) claims 99% less
compute and 39x faster learning than DreamerV3 using active inference
principles. However, AXIOM is object-centric reinforcement learning, not
LLM-native, and VERSES is a publicly traded company (NEO: VERS) with
marketing incentives. The architecture should not be bet on FEP-for-LLMs as
proven technology. The mathematical framework is sound; the engineering
implementation remains speculative. Use the planning formalism; do not depend
on the VERSES codebase.

---

## 7. Sleep-Cycle Consolidation and Machine Unlearning

### The Biological Metaphor Made Operational

Roko implements a sleep-cycle consolidation pipeline inspired by biological
memory systems. During "waking" operation, agents accumulate episodic memories
-- raw records of what they did, what worked, and what failed. During "sleep"
cycles (triggered nightly or on a configurable schedule), these episodes are
distilled into semantic knowledge entries in the neuro store.

The neuro store uses a four-tier retention model:

| Tier | Half-Life Multiplier | Effective Half-Life (Insight) |
|:-----|:--------------------:|:----------------------------:|
| Transient | 0.1x | 3 days |
| Working | 0.5x | 15 days |
| Consolidated | 1.0x | 30 days |
| Persistent | 5.0x | 150 days |

Entries start at Transient and are promoted through independent confirmation:
2+ confirmations for Working, 3+ across distinct contexts for Consolidated,
explicit validation for Persistent. A demurrage model (balance decays at 0.005
per hour) ensures unused knowledge is garbage-collected. Five reinforcement
signals (Retrieved, Cited, Gated, Surprised, AgentQuoted) keep useful
knowledge alive.

### Research Foundations

Three papers ground the sleep-cycle implementation:

- **SleepGate** (arXiv 2603.14517, March 2026): Gating mechanisms for
  selective memory consolidation during offline cycles. Controls which
  episodic memories are promoted to semantic storage and which are allowed to
  decay.
- **SCM** (Sleep Consolidation Model; arXiv 2604.20943, April 2026): Achieves
  **90.9% memory-noise reduction** through structured consolidation cycles.
  This is the empirical justification for Roko's sleep pipeline: noise
  reduction at this level means the knowledge store converges toward high-
  quality entries rather than accumulating junk.
- **Wake-Sleep Consolidated Learning** (arXiv 2401.08623, January 2024):
  Theoretical framework for alternating online learning (wake) and offline
  consolidation (sleep) in neural systems. Provides convergence guarantees
  under mild assumptions.

### GDPR Compliance via Subtractive HDC Binding

Machine unlearning is not optional for a regulated benchmark administrator.
GDPR's right to be forgotten and IOSCO Principle 13 (transition, covering
cessation and delisting) both require the ability to remove specific data from
the system's memory.

HDC's algebraic structure makes this tractable. Because knowledge entries are
encoded as bound hypervectors, removing an entry is a **subtractive binding
operation**: unbind the entry's vector from the aggregate semantic memory. The
result is a new aggregate that provably does not contain the removed entry's
information (up to the HDC algebra's approximation guarantees). The index can
then be recomputed deterministically from the cleaned memory.

This gives Nunchi a clean compliance story: agent attestations from a delisted
source (e.g., a stablecoin issuer that fails the inclusion criteria) can be
HDC-subtracted from semantic memory and the index recomputed. The audit trail
shows exactly what was removed and why. IOSCO Principle 13 and GDPR Article 17
are satisfied by the same mechanism.

### IOSCO Audit Trail

The 5-year retention requirement under IOSCO Principle 17 is structurally
trivial on-chain. Every agent attestation, every methodology version, every
gate verdict, and every index calculation is an immutable Korai transaction.
The question is not whether the audit trail exists (it does, by construction)
but whether it is *interpretable* by auditors. Inspect AI's evaluation format
provides the interpretation layer: each agent run is a structured evaluation
record with inputs, outputs, scores, and explanations that a KPMG or PwC
auditor can review without understanding the underlying HDC algebra.

---

## 8. Key Research Papers and Integrations

### DSPy 3.0 + GEPA

- **Paper**: Agrawal et al., arXiv 2507.19457, July 2025. ICLR 2026 Oral.
- **Performance**: +10% over MIPROv2 on AIME-2025; +6-20% over GRPO with
  35x fewer rollouts.
- **GitHub**: 28,000+ stars (DSPy).
- **Integration**: Every ISFR methodology pipeline is a DSPy program.
  GEPA optimizes prompts against historical data and adversarial scenarios.
  Artifacts are JSON-serializable, hash-stable, stored on Korai with
  versioned lineage. This is the compiler for methodology-as-code.

### Darwin Godel Machine (DGM)

- **Paper**: Zhang, Hu, Lu, Lange, Clune; arXiv 2505.22954, May 2025,
  revised March 2026.
- **Performance**: SWE-bench 20.0% to 50.0%; Polyglot 14.2% to 30.7%.
- **Integration**: Archive-based self-improvement for methodology evolution.
  The archive is on-chain Korai storage; each variant is an NFT-bound
  lineage. OMNI-EPIC generates synthetic stress-test scenarios. Evolution
  operates within IOC-approved invariant bounds.

### MacNet

- **Paper**: Qian et al., arXiv 2406.07155, June 2024, revised March 2025.
- **Finding**: Collaborative scaling law; small-world topology dominance;
  logistic growth to 1000+ agents.
- **Integration**: Gives the IOC a concrete *k*-agent budgeting heuristic
  for ISFR sub-index computation. Roko's `ParallelExecutor` DAG maps to
  MacNet topologies directly.

### VSA-Lisp

- **Paper**: Hanley, Tomkins-Flanagan, Kelly; arXiv 2511.08767, November 2025.
- **Finding**: Turing-complete Lisp encoded in FHRR + Residue HDC.
- **Integration**: Agent skills become bound hypervectors stored and composed
  on-chain. Pairs with SRMU for streaming updates.

### SRMU (Streaming Resonator Memory Update)

- **Paper**: arXiv 2604.15121, 2026.
- **Integration**: Streaming HDC memory for daily index updates without full
  history replay.

### SleepGate + SCM

- **Papers**: SleepGate arXiv 2603.14517, March 2026; SCM arXiv 2604.20943,
  April 2026. SCM achieves 90.9% memory-noise reduction.
- **Integration**: Nightly episodic-to-semantic distillation. GDPR/unlearning
  via subtractive HDC binding.

### Wake-Sleep Consolidated Learning

- **Paper**: arXiv 2401.08623, January 2024.
- **Integration**: Theoretical grounding for the wake-sleep alternation
  pattern in Roko's knowledge store.

### VERSES AXIOM

- **Paper**: Heins et al., arXiv 2505.24784, May 2025.
- **Claims**: 99% less compute, 39x faster learning than DreamerV3.
- **Caveats**: Object-centric RL, not LLM-native. VERSES (NEO: VERS) is a
  publicly traded company with marketing incentives. Use the free-energy
  planning formalism; do not depend on the codebase. Watch, do not integrate.

### Inspect AI

- **Source**: UK AI Safety Institute, open-sourced May 2024.
- **Adoption**: METR, Apollo Research, US CAISI. 200+ pre-built evals.
- **Integration**: Regulator-grade eval substrate for agent-attested data
  quality. Aligns with FCA expectations for benchmark administration.

### ERC-8004 + x402

- **ERC-8004**: De Rossi/MetaMask, Crapis/EF, Ellis/Google, Reppel/Coinbase;
  EIP draft August 2025. 106,996 indexed agents across Base/BSC/Ethereum by
  late 2025.
- **x402**: Coinbase, May 2025. +10,000% MoM October 2025; >900K weekly
  settlements. x402 Foundation co-founded with Cloudflare. Integrations: Visa,
  Google, Stellar, Solana. v2 December 2025.
- **Integration**: De facto standards for agent identity and agent payments.
  Korai must implement both natively or be a second-class agent chain.

### Intersubjective Validation (Eigen-AVS)

- **Paper**: Yuan et al., arXiv 2504.13443, April 2025.
- **Integration**: Non-naive reputation primitive for cross-checking agent
  attestations. Better than Sybil-vulnerable Eigentrust.

### SWE-ABS

- **Paper**: arXiv 2603.00520, 2026.
- **Integration**: Adversarial strengthening of the methodology itself.
  Prevents gaming of index calculations via the same techniques that exploit
  LLM benchmarks.

### CodeCRDT

- **Paper**: arXiv 2510.18893, October 2025.
- **Integration**: CRDT-shared state for multi-agent index computation with
  provable at-most-one-winner safety and deterministic merge semantics.

### Torchhd

- **Paper**: Heddes et al., arXiv 2205.09208, published JMLR 2023.
- **Performance**: 100x speedup over CPU baselines.
- **Integration**: Ships HDC episodic-to-semantic distillation today.

---

## 9. Implementation Priorities

### Integrate Now (Weeks)

These components have mature implementations, clear APIs, and direct
applicability to ISFR operations:

| Component | Effort | Why Now |
|:----------|:-------|:--------|
| **DSPy 3.0 + GEPA** | 2-4 weeks | Methodology pipelines must be hash-stable, versioned, and optimizable from day one. 28K+ stars; production-grade. |
| **ERC-8004 agent registration** | 2-3 weeks | Every Roko agent needs an on-chain identity before Phase-1 attestor signing. 107K agents already indexed. |
| **x402 micropayments** | 2-3 weeks | Data fetch provenance requires economic audit trail. >900K weekly settlements; Coinbase-backed standard. |
| **Inspect AI eval suite** | 3-4 weeks | FCA pre-application must demonstrate methodology validation. UK AISI standard. |
| **Torchhd distillation** | 1-2 weeks | Already shipping in Roko. Formalize the episodic-to-semantic pipeline. |
| **CRDT shared state** | 2-3 weeks | Multi-agent index calculation must be deterministic and reproducible. |

### Spec and Plan (Months)

These require architectural decisions, protocol design, and potentially
governance approval:

| Component | Effort | Why Wait |
|:----------|:-------|:---------|
| **HDC similarity-search precompile** | 3-6 months | Novel instruction set design; patent application; formal verification of approximation guarantees. No public prior art means no reference implementation to build on. |
| **DGM self-improvement archive** | 2-4 months | Requires IOC-approved invariant bounds before deployment. The evolution sandbox must be designed before the evolution runs. |
| **MacNet topology optimization** | 2-3 months | Depends on empirical data from initial fleet deployment. Need real workload profiles before optimizing topology. |
| **VSA-Lisp skill encoding** | 3-4 months | Depends on HDC precompile. Skills-as-hypervectors requires the on-chain execution layer. |
| **Active inference compute budgeting** | 2-3 months | Upgrade from LinUCB bandit to full FEP formulation. Mathematical framework is sound but engineering integration touches cascade router, attention bidding, and x402 payment logic. |
| **SleepGate + SCM consolidation** | 2-3 months | Sleep pipeline exists; needs formalization with SleepGate gating and SCM noise reduction. |
| **SRMU streaming memory** | 1-2 months | Depends on Torchhd integration maturity. |
| **GDPR unlearning via HDC subtraction** | 2-3 months | Requires formal analysis of approximation bounds for regulatory defensibility. |

### Watch (Quarters to Years)

These are promising but carry significant uncertainty:

| Component | Status | Caveat |
|:----------|:-------|:-------|
| **VERSES AXIOM** | Interesting formalism | Object-centric RL, not LLM-native. Publicly traded company. Don't bet the architecture. |
| **Eigen-AVS intersubjective validation** | Strong theory | Depends on EigenLayer ecosystem maturity and restaking economics stabilizing. |
| **Berkeley RDI benchmark exploit research** | Critical awareness | Means any ISFR claim built on agent-benchmark numbers is fragile. Eval frameworks are attack surfaces. |
| **Full FEP-for-LLMs** | Theoretical appeal | No production implementation exists. Use the planning math; don't wait for the general framework. |

---

## 10. Why This Makes Korai the Canonical L1 for Regulated Benchmark Operation

The argument is not that Korai is the best general-purpose blockchain. It is
that Korai is the only chain that can natively support all seven requirements
of a regulated benchmark administered by an AI agent fleet:

1. **Agent identity** (ERC-8004 native implementation): Every data-fetching
   agent has a verifiable on-chain identity with auditable code hashes and
   historical accuracy records.

2. **Agent payments** (x402 native implementation): Every data fetch has an
   economic audit trail. Fabricating data requires paying for it, making
   large-scale poisoning economically irrational.

3. **Deterministic multi-agent computation** (CRDT merge at the consensus
   layer): Index calculations are reproducible by any third party. The merge
   function is part of the chain's state transition, not an application-layer
   afterthought.

4. **On-chain similarity search** (HDC precompile with no public prior art):
   Historical condition queries ("what past market states resemble today?")
   execute at the EVM layer, not in expensive smart-contract loops. This is
   the patentable moat.

5. **Immutable methodology versioning** (DSPy/GEPA artifacts as chain
   state): Every methodology revision is a hash-stable, JSON-serializable
   artifact stored on-chain with cryptographic provenance. IOSCO Principle 12
   (changes to methodology) is satisfied by construction.

6. **5-year audit trail** (inherent to chain storage): IOSCO Principle 17
   requires retaining inputs, calculations, and decisions for 5 years. On
   an append-only blockchain, this is not a feature to build but a property
   that already exists.

7. **Compliant forgetting** (HDC subtractive binding): GDPR Article 17 and
   IOSCO Principle 13 are satisfied by the same algebraic operation. Data
   from delisted sources is provably removed from semantic memory and the
   index is recomputed deterministically.

No other L1 has items 3, 4, and 7. Ethereum has items 1 and 2 (via ERC-8004
and x402 as application-layer standards) but not at the precompile level.
Solana has throughput but no HDC precompile and no native agent registry
standard. Cosmos app-chains could theoretically implement any of these but
none has. The first-mover advantage is real because regulated benchmark
administrators are structurally conservative -- the first chain that
demonstrates all seven properties to the FCA's satisfaction becomes the
default, and switching chains is as difficult as switching index providers.

The compound thesis closes the loop: Roko agents operate the benchmark. The
benchmark creates revenue and regulatory credibility. The regulatory
credibility attracts institutional capital. The institutional capital funds
more agents. The agents improve the benchmark. The benchmark justifies the
chain. The chain enables the agents. No component stands alone, and no
competitor can replicate the full stack by building any single piece.
