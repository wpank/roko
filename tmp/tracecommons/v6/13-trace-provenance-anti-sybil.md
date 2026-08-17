# Trace Provenance & Anti-Sybil Defense

**Date**: August 2026 (v6)

TraceCommons (TC) is an open-source Rust-based privacy-preserving registry of AI coding agent session traces (~235K LOC, 6 crates). Quality and novelty are scored inside TEEs on NEAR AI Cloud (Intel TDX + NVIDIA GPU TEE); contributors earn NEAR credits via the formula `q = f * g * a` (f = freshness, g = gate score, a = attestation weight). Current traction: ~352 submissions, 3 contributors. Because TC pays credits for traces, there is a direct economic incentive to fabricate, duplicate, or replay traces. This document maps the threat model, surveys the relevant literature (10 verified papers), and proposes a phased defense architecture.

---

## 1. The Threat Model

TC's credit mechanism creates four distinct attack surfaces:

### 1.1 Fabrication

Generate synthetic traces that look like real agent sessions but were never actually executed. An attacker writes a script that emits plausible-looking tool calls, file edits, and LLM responses. If the perplexity scorer and gate pipeline cannot distinguish synthetic from genuine, the attacker earns credits for work that never happened.

**Why it matters now**: At ~352 submissions and 3 contributors, fabrication is easy to catch manually. At 10K submissions and 100 contributors, it is not. The defense must be in place before scale arrives.

### 1.2 Sybil Attacks

One person submits the same or similar traces under multiple NEAR accounts. Each submission earns credits independently. The freshness factor `f` penalizes exact duplicates, but a Sybil attacker who introduces minor perturbations -- reordering tool calls, changing variable names, adding no-op steps -- can evade similarity detection while submitting substantively identical work.

**Quantified risk**: Blum et al. (arXiv:2605.07663) prove that semivalue-based data valuation (including Shapley) amplifies Sybil payoffs by 1.74x. If TC ever adopts Shapley-based credit allocation, Sybil attacks become strictly more profitable than honest contribution.

### 1.3 Replay Attacks

Resubmit someone else's traces as your own. An attacker monitors public trace data, copies a high-scoring trace, and submits it under their own account. Without provenance binding between the trace and the submitting identity, there is no way to distinguish the original from the copy.

### 1.4 Quality Gaming

Manipulate traces to score higher on the gate pipeline. This includes injecting rare tokens to inflate perplexity scores, structuring traces to maximize embedding distance from the HNSW index, or exploiting scoring bugs like Issue #219 (redaction penalizing quality). Partially addressed in doc 02 (Scoring & Quality Pipeline), but provenance attestation adds a complementary defense layer: if the scoring pipeline itself is attested, an attacker cannot claim a score they did not earn.

### 1.5 The Credibility Trilemma

Alon et al. (arXiv:2605.26604) formalize the impossibility result underlying all auction-based allocation mechanisms: ghost-bid deviations -- where a participant fabricates phantom bids to manipulate outcomes -- are simultaneously profitable and undetectable under both sealed-bid VCG and Myerson mechanisms. The only known closure is broadcast commitment, where all bids are publicly committed before revelation. This result directly constrains TC's credit mechanism: any sealed allocation of credits to traces is vulnerable to ghost-bid manipulation unless a commitment phase is introduced.

---

## 2. Relevant Literature (10 Verified Papers)

### 2.1 VET -- Verifiable Execution Traces

**Citation**: arXiv:2512.15892

VET introduces Web Proofs -- cryptographic attestations that a specific web API call occurred with specific inputs and outputs. The key finding for TC: trace verification is practical at less than 3x overhead. A TEE Proxy architecture (where the TEE acts as a man-in-the-middle on HTTPS connections, terminating TLS inside the enclave) is sufficient for public API verification. This means TC can verify that an agent actually called the APIs it claims to have called, without requiring changes to the API provider.

**TC relevance**: Directly applicable to Phase 3 (Web Proofs for API calls). The 3x overhead bound establishes feasibility for high-value trace verification. Not viable for all traces at current margins, but viable as a premium tier.

### 2.2 Proof-of-Execution

**Citation**: arXiv:2607.05397

Issues Execution Attestation Certificates (EACs) -- signed certificates proving that a specific computation occurred within a TEE. Benchmarks:

- ~2.7ms overhead on minimal execution flows
- 4.4% throughput overhead on batch workloads
- ~1.1 KB storage per 8-event trace
- Composable with zkVMs for post-hoc verification

**TC relevance**: The primary building block for Phases 1 and 2. TC already runs Intel TDX for scoring. Extending TDX attestation to the ingestion step adds ~2.7ms per trace (negligible relative to the scoring pipeline, which runs a 35B parameter model). The 1.1 KB storage overhead per trace is trivially small. EACs provide the cheapest credible anti-fabrication guarantee: "this trace was actually ingested through TC's pipeline at this time."

### 2.3 SVIP -- Standards for Verifiable AI Inference Provenance

**Citation**: arXiv:2604.23280

Proposes inference-level provenance standards for AI systems. Establishes a vocabulary for AI identity verification: who ran the model, which model was it, what were the inputs, and what were the outputs. Introduces the concept of an AI identity certificate chain analogous to X.509 PKI.

**TC relevance**: TC's traces are records of AI inference sessions. SVIP's identity standards could provide a common vocabulary for trace provenance metadata. If adopted broadly, TC could verify that a trace was generated by a specific AI provider's infrastructure rather than fabricated locally.

### 2.4 From Logic Monopoly to Social Contract

**Citation**: arXiv:2603.25100

Surveys TEE hardware-root-of-trust architectures for agent economies. The core argument: agent-to-agent economic interactions require a trust anchor that is neither purely cryptographic (too expensive, too rigid) nor purely reputational (too slow to bootstrap, too easy to game). TEE attestation provides a middle path -- hardware-enforced execution integrity with software-level flexibility. The paper surveys Intel TDX, ARM CCA, AMD SEV-SNP, and NVIDIA Hopper Confidential Computing, concluding that TDX provides the best cost-performance tradeoff for server-side attestation workloads.

**TC relevance**: Provides the theoretical framework for why TEE-based attestation is the right trust anchor for TC specifically. TC already uses TDX for scoring; this paper validates extending TDX attestation to the full ingestion-scoring-settlement pipeline.

### 2.5 Agent-OSI

**Citation**: arXiv:2602.13795

Proposes a layered reference model for agent systems (analogous to the OSI network model). Layer L5 (Provenance) standardizes a provenance interface that admits multiple attestation mechanisms:

- TEE attestations (hardware-rooted)
- ZK proofs (cryptographic, post-hoc verifiable)
- Signed logs (weakest, but cheapest)

The key insight for TC: not all traces need the same level of provenance. L5 defines a menu of attestation strengths, allowing systems to trade off cost against assurance. This directly informs TC's tiered provenance design.

**TC relevance**: Agent-OSI L5 provides the design vocabulary for TC's tiered defense architecture. Rather than requiring full TDX attestation for every trace (expensive), TC can offer a menu of provenance levels with corresponding credit multipliers.

### 2.6 Credibility Trilemma

**Citation**: arXiv:2605.26604

Formalizes the impossibility of simultaneously achieving truthfulness, privacy, and efficiency in auction mechanisms. Ghost-bid deviations -- where a participant fabricates phantom bids to manipulate the allocation outcome -- are profitable and undetectable under sealed-bid VCG and Myerson. The only known closure mechanism is broadcast commitment: all bids are publicly committed (e.g., via hash) before the allocation round, making ghost bids detectable.

**TC relevance**: If TC's credit allocation ever involves competitive bidding (e.g., limited credit pools, priority queues for scoring), the Credibility Trilemma proves that sealed allocation is insecure. TC must either (a) avoid competitive allocation entirely (fixed formula, which is the current `q = f * g * a` approach), or (b) introduce broadcast commitment if moving to market-based allocation.

### 2.7 TEE Survey for Agentic AI

**Citation**: arXiv:2605.03213

The first systematic mapping of Confidential Computing onto agentic AI systems (Forough, Kogias, Haddadi, May 2026). The survey catalogs six TEE platforms -- Intel SGX, Intel TDX, AMD SEV-SNP, ARM TrustZone, ARM CCA, and NVIDIA H100 Confidential Computing -- and evaluates each against the requirements of autonomous agent workloads. The headline finding: no broadly established end-to-end framework yet binds TEE primitives into a coherent security substrate for production agentic AI. The paper identifies six open challenges, two of which are directly relevant to TC:

1. **Compound attestation for multi-hop agent chains.** When an agent delegates to sub-agents, each running in its own TEE, no current framework provides a single attestation that covers the full chain of execution. Each TEE produces its own attestation report independently; composing these into a verified end-to-end provenance chain is an unsolved problem. TC's pipeline is exactly such a multi-hop chain: IronClaw (trace source) to TC ingest to TEE scoring to NEAR credit settlement. Compound attestation would allow a downstream credit consumer to verify the full chain with a single check rather than auditing each hop independently.

2. **GPU-TEE performance at LLM scale.** Running large language models inside GPU TEEs (specifically NVIDIA H100 CC) incurs throughput penalties that are not yet well-characterized at production scale. TC's scoring pipeline runs Qwen 3.6 35B-A3B-FP8 inside a GPU TEE. The survey flags this as an open challenge -- performance data at the 35B parameter scale within GPU TEEs is sparse, and TC should expect to encounter undocumented performance cliffs as submission volume grows.

**TC relevance**: The survey confirms that TDX and SEV-SNP are the correct target hardware for TC's server-side attestation workloads (consistent with the Logic Monopoly paper, Section 2.4). It also surfaces two constraints that TC's phased roadmap must account for: compound attestation is not yet a turnkey capability (relevant to Phase 4's on-chain anchoring, which assumes a verifiable end-to-end chain), and GPU-TEE performance at LLM scale is an open variable that affects Phase 2's throughput budget for the 35B scorer.

### 2.8 Governance-as-a-Service (GaaS)

**Citation**: arXiv:2508.18765

Governance-as-a-Service (Gaurav et al., August 2025) proposes a modular, policy-driven enforcement layer that regulates agent outputs at runtime. The framework introduces a Trust Factor mechanism: each agent accumulates a compliance score based on its history of policy adherence, weighted by violation severity. The Trust Factor drives graduated enforcement -- coercive (block the action), normative (warn and log), or adaptive (adjust permissions dynamically) -- depending on the agent's accumulated trust level.

The structural parallel to TC is direct. TC's anomaly penalty factor `a` in the credit formula `q = f * g * a` functions as a Trust Factor: it scales a contributor's credit based on behavioral signals (submission anomalies, pattern deviations, flagged traces). GaaS formalizes what TC implements informally -- a graduated enforcement mechanism where the penalty is proportional to the severity and frequency of trust violations, rather than a binary accept/reject.

**TC relevance**: GaaS provides a formal framing for TC's contributor reputation mechanism. Specifically:

- TC acts as a GaaS layer for GPAI (General-Purpose AI) compliance: it attests agent behavior via TEE-scored traces, and auditors can query TC's registry to verify a contributor's compliance history over time.
- The graduated enforcement model maps to TC's tiered provenance design (Section 5): contributors with consistently high Trust Factor (clean `a` scores) earn higher provenance tiers and credit multipliers; contributors with anomaly flags see their effective credits reduced before any manual review.
- GaaS's distinction between coercive, normative, and adaptive interventions suggests TC should formalize its own intervention taxonomy. Currently, TC's only intervention is credit reduction (the `a` penalty). GaaS suggests adding normative interventions (warnings, contributor notifications) and adaptive interventions (automatic tier demotion/promotion based on rolling Trust Factor).

### 2.9 Privilege Attenuation in Agent Delegation

**Citation**: arXiv:2602.11865

"Intelligent AI Delegation" (Tomasev, Franklin, Osindero -- Google DeepMind, February 2026) establishes a formal authorization model for multi-agent systems where agents sub-delegate tasks. The core principle is privilege attenuation: when an agent delegates to a sub-agent, it cannot transmit its full set of authorities. Instead, it must issue restricted permissions scoped to the specific sub-task. Permissions include not just which tools may be used, but semantic constraints on how they may be used (e.g., "may read file X but only to extract function signatures, not to copy implementation").

Three mechanisms enforce attenuation:

1. **Monotonic permission restriction.** Each delegation hop can only narrow permissions, never broaden them. The permission set is a monotonically decreasing function of delegation depth.
2. **Continuous validation.** Access rights persist only while trust metrics are maintained. If a sub-agent's behavior deviates from its permission scope, its authorities are revoked in real time.
3. **Liability firebreaks.** Pre-defined stopping points in the delegation chain where authority transfer requires explicit re-authorization. A firebreak prevents cascading failures: if a sub-agent misbehaves, the damage is contained to the segment between firebreaks.

**TC relevance**: TC's multi-hop provenance chain -- IronClaw (trace capture) to TC ingest to TEE scoring to NEAR credit settlement -- is structurally a delegation chain. Each hop receives data from the previous hop and produces an output consumed by the next. Privilege attenuation provides the formal authorization model for this chain:

- **Monotonic restriction maps to TC's pipeline.** The ingest step receives the full trace; the scoring step receives only the redacted trace; the credit settlement step receives only the score and contributor ID. Each hop operates on a strictly narrower data scope than its predecessor. This is currently implicit in the pipeline architecture. Privilege attenuation says it should be explicit: each hop should declare the minimal permission set it requires, and the TEE attestation should cover the permission scope as well as the computation.
- **Firebreaks map to gate thresholds.** TC's gate pipeline already implements firebreaks: a failed gate score halts the pipeline and withholds credit. The privilege attenuation framing adds a formal justification -- a gate failure is a firebreak that prevents a low-quality trace from propagating further into the credit chain. This is a credit-withholding firebreak rather than an authority-transfer firebreak, but the containment principle is identical.
- **Continuous validation maps to the `a` factor.** A contributor whose anomaly penalty `a` drops below a threshold effectively loses the authority to earn full credits -- a form of continuous validation applied to the contributor rather than to a sub-agent.

### 2.10 Cryptographic Verifiability of End-to-End AI Pipelines

**Citation**: arXiv:2503.22573

"A Framework for Cryptographic Verifiability of End-to-End AI Pipelines" (Balan, Learney, Wood -- IWSPA 2025) defines a complete verification framework spanning data sourcing, model training, inference, and unlearning. The paper's central contribution is the "linkability" requirement: cryptographic tools used at different pipeline stages must produce linked proofs so that a downstream verifier can check the entire chain without re-executing any stage. Without linkability, an attacker can substitute a different model at inference time or tamper with training data, and stage-local proofs cannot detect the substitution.

**TC relevance**: TC's attestation chain (Phase 1 EAC at ingest, Phase 2 attested scoring, Phase 4 on-chain anchor) is exactly the kind of multi-stage pipeline that requires linkability. Currently, each phase produces its own attestation independently. The linkability requirement says these attestations must be cryptographically chained: the Phase 2 scoring attestation must reference the Phase 1 EAC hash, and the Phase 4 on-chain anchor must reference the Phase 2 attestation hash, so that a downstream credit consumer can verify the full provenance chain by checking a single root hash rather than independently validating each stage. This is consistent with the compound attestation challenge identified in the TEE Survey (Section 2.7) -- linkable commitments are the cryptographic mechanism that would solve the compound attestation problem for TC's specific pipeline topology.

---

## 3. Four-Phase Implementation

### Phase 1: Signed Execution Attestation Certificates (Weeks)

**What**: Extend TC's existing Intel TDX scoring infrastructure to attest the ingestion step. When a trace is received, the ingestion pipeline running inside the TEE generates a signed EAC binding the trace hash, contributor identity, and timestamp.

**Why this is cheap**: TC already runs TDX for scoring. The ingestion step happens before scoring. Adding EAC generation to the ingestion path requires:

1. Hash the incoming trace payload (SHA-256, microseconds)
2. Bind the hash to the contributor's NEAR account
3. Generate a TDX attestation report over the (hash, account, timestamp) tuple
4. Store the EAC alongside the trace metadata

**Performance impact** (from Proof-of-Execution benchmarks):

- Latency: ~2.7ms per trace (the scoring pipeline takes orders of magnitude longer)
- Storage: ~1.1 KB per trace (negligible)
- Throughput: no measurable impact at TC's current ~13 submissions/week

**What it proves**: "This trace was actually submitted through TC's ingestion pipeline at this time by this NEAR account." Does not prove the trace was generated by a real agent session (that requires Phase 2+), but eliminates the simplest fabrication attack: submitting traces that never touched TC's infrastructure.

**Anti-replay**: The EAC includes a monotonic sequence number and timestamp. Replaying an EAC produces a duplicate sequence number, which is trivially detectable.

**Effort**: 1-2 weeks. The TDX infrastructure exists. This is plumbing.

### Phase 2: TDX-Attested Scoring Pipeline (1-2 Months)

**What**: Full TDX attestation of the entire ingestion-through-scoring pipeline. The EAC from Phase 1 proves ingestion happened; Phase 2 proves the scoring was computed correctly.

**What is attested**:

1. Redaction was applied (and applied correctly -- no raw PII left the enclave)
2. Embedding was computed by the declared model (BGE-large-en-v1.5)
3. Similarity search was performed against the declared HNSW index
4. Perplexity scoring was performed by the declared model (Qwen 3.6 35B-A3B-FP8)
5. The credit formula `q = f * g * a` was computed with the declared parameters
6. The final credit amount matches the attested computation

**Performance impact** (from Proof-of-Execution benchmarks):

- Throughput overhead: ~4.4% on batch workloads
- Acceptable given that TC processes ~13 submissions/week

**What it proves**: The scoring pipeline was not tampered with. A contributor cannot claim a score they did not earn. An operator cannot selectively inflate or deflate scores. This is the strongest defense against quality gaming: if the scorer is attested, manipulating scores requires breaking TDX.

**GPU-TEE performance caveat.** The TEE Survey for Agentic AI (arXiv:2605.03213) flags GPU-TEE performance at LLM scale as an open challenge. TC's scorer runs Qwen 3.6 35B-A3B-FP8 inside NVIDIA H100 Confidential Computing. Performance data at this parameter count within GPU TEEs is sparse, and the 4.4% batch overhead figure from Proof-of-Execution benchmarks was measured on CPU-side attestation, not GPU-TEE inference. The actual throughput cost of full GPU-TEE attestation at 35B scale may be higher and should be benchmarked on TC's specific hardware before committing to Phase 2 timelines.

**Anti-Sybil**: Phase 2 does not directly prevent Sybil attacks, but it ensures that Sybil detection signals (similarity scores, embedding distances) are computed honestly. An attacker cannot evade similarity detection by compromising the similarity computation.

**Privilege attenuation.** The privilege attenuation model (arXiv:2602.11865) adds a formal justification for Phase 2's pipeline structure: each stage operates on a strictly narrower data scope than its predecessor (full trace at ingest, redacted trace at scoring, score-plus-ID at settlement). Phase 2 attestation should cover not just the computation but the permission scope at each stage, making the monotonic restriction explicit and verifiable. Gate failures function as liability firebreaks -- a failed gate score halts the pipeline and withholds credit, containing the impact of a low-quality or anomalous trace to the segment before the firebreak.

**Effort**: 1-2 months. Requires careful audit of the entire scoring pipeline to ensure determinism within the TEE. Non-deterministic operations (e.g., floating-point rounding, random seeds for HNSW) must be pinned.

**Determinism cost.** Achieving bit-identical inference for attestation purposes carries a real throughput penalty. Batch-invariant kernels impose approximately 34–61% throughput cost (Thinking Machines/SGLang — ⚠️ SOURCE UNVERIFIED: previously cited arXiv:2606.03019 is a WRONG CITATION; that paper is "Reproducibility is the New Copyleft: Defining AGI-oriented Reproducible Builds," unrelated to TEE inference costs), reducible to ~34.35% with CUDA graphs. ONNX Runtime does not expose deterministic primitive selection or reduction order (⚠️ SOURCE UNVERIFIED: previously cited arXiv:2501.05867 is a WRONG CITATION; that paper is "Neural network verification challenges as programming-language challenges," unrelated to ONNX non-determinism) and is not a shortcut. Both technical claims are likely correct but need re-sourcing. At TC's current submission rate (~13/week) this overhead is operationally invisible, but it must be budgeted before scaling. An alternative is the LLM-42 verify-rollback scheme (arXiv:2601.17768): run the scorer normally and only re-execute deterministically when attestation of a specific trace is contested. For the similarity search component specifically, replacing HNSW with a brute-force exact cosine/Hamming scan over HDC hypervectors avoids the HNSW nondeterminism and side-channel problems entirely at TC's current corpus size (see Section 6.3).

### Phase 3: Web Proofs for API Calls (3-6 Months)

**What**: For high-value submissions, verify that the agent actually called the APIs it claims to have called. Uses VET's Web Proof architecture: a TEE Proxy terminates the agent's HTTPS connections, capturing request-response pairs inside the enclave and generating cryptographic proofs of the interaction.

**Scope**: Premium tier only. Web Proofs add less than 3x overhead per API call (VET benchmark), but this is too expensive for all traces at TC's current margins. Reserved for traces claiming high-value interactions (e.g., complex multi-tool agent sessions, traces from new/unverified contributors).

**What it proves**: "This agent actually called the GitHub API / the Claude API / the database at this time with these parameters and received this response." Eliminates fabrication of API call sequences -- the strongest fabrication vector.

**Limitations**:

- Requires the agent to route API calls through TC's TEE Proxy (opt-in)
- Does not work for local tool calls (file system, shell commands)
- Adds latency to the agent's execution (less than 3x, but non-zero)
- Privacy implications: TC's TEE sees the full API request/response (mitigated by TEE confidentiality guarantees)

**Effort**: 3-6 months. Requires building or integrating a TEE Proxy, designing the opt-in mechanism, and defining "high-value" submission criteria.

### Phase 4: On-Chain Anchoring (6-12 Months)

**What**: Anchor attestation hashes on the NEAR blockchain for a tamper-evident provenance chain. Each accepted trace's EAC hash is included as metadata in the NEAR credit settlement transaction.

**Why NEAR**: TC already uses NEAR for credit settlement. Adding an attestation hash to the settlement transaction is a trivial extension of the existing payment flow. No new blockchain infrastructure required.

**What it proves**: The attestation existed at the time of settlement and has not been modified since. Provides a public, immutable record of TC's provenance claims. Third parties can independently verify that a specific trace was attested at a specific time.

**Linkability requirement.** The Cryptographic AI Pipeline Framework (arXiv:2503.22573) establishes that multi-stage attestation chains are only meaningful if the attestations at each stage are cryptographically linked. For TC, this means the Phase 4 on-chain anchor must not simply store the final EAC hash in isolation -- it must reference a chain of linked commitments: the Phase 1 EAC hash is embedded in the Phase 2 scoring attestation, which is in turn embedded in the Phase 4 on-chain record. A downstream credit consumer can then verify the full provenance chain by checking the root hash on-chain, without independently auditing each stage. This is the cryptographic mechanism that addresses the compound attestation challenge identified in the TEE Survey (arXiv:2605.03213). Without linkability, an attacker could substitute a different scoring attestation for a given trace and the on-chain record would not detect the substitution.

**Data anchored per trace**:

- EAC hash (covers trace hash, contributor ID, timestamp, scoring attestation)
- Credit amount
- Settlement transaction ID
- Provenance tier (1, 2, or 3)

**Cost**: One additional field in an existing NEAR transaction. Marginal cost per trace approaches zero since the settlement transaction is already being sent.

**Effort**: 2-4 weeks of engineering once the settlement pipeline is stable. The 6-12 month timeline reflects sequencing after Phases 1-3, not intrinsic complexity.

---

## 4. Five-Layer Defense Architecture

The four phases compose into a layered defense stack. Each layer is independently valuable and does not require the layers above it.

```
+-------------------------------------------------------------+
| Layer 5: Economic Defense                                    |
| Staking + broadcast commitment (Credibility Trilemma)        |
| Sybil cost: minimum stake per NEAR account                   |
+-------------------------------------------------------------+
| Layer 4: Statistical Detection                               |
| Cross-submission similarity, contribution patterns,          |
| timing analysis, behavioral fingerprinting                   |
+-------------------------------------------------------------+
| Layer 3: Scoring Attestation (Phase 2)                       |
| TDX-attested scoring pipeline: redaction, embedding,         |
| perplexity, credit computation all verifiable                |
+-------------------------------------------------------------+
| Layer 2: Ingestion Attestation (Phase 1)                     |
| TEE-signed EAC at capture: trace hash + account + timestamp  |
| Anti-replay via monotonic sequence numbers                   |
+-------------------------------------------------------------+
| Layer 1: Identity Binding (Existing)                         |
| NEAR account binding, one account per contributor            |
+-------------------------------------------------------------+
```

### Layer 1: Identity Binding (Existing)

TC already requires a NEAR account for credit settlement. This is the weakest layer -- NEAR accounts are free to create -- but it provides a foundation for Sybil cost escalation (Layer 5).

### Layer 2: Ingestion Attestation (Phase 1)

TEE-signed capture at ingestion. Proves the trace entered TC's pipeline. Eliminates offline fabrication (traces that never touched TC infrastructure). Cost: ~2.7ms latency, ~1.1 KB storage per trace.

### Layer 3: Scoring Attestation (Phase 2)

TDX-attested scoring pipeline. Proves the score was computed correctly. Eliminates quality gaming via score manipulation. Cost: ~4.4% throughput overhead.

### Layer 4: Statistical Detection

Algorithmic detection of anomalous submission patterns. This layer operates on metadata and does not require TEE attestation.

**Signals**:

- **Cross-submission similarity**: MinHash fingerprints (doc 02, Rensa) across all submissions from all accounts. High Jaccard similarity between accounts suggests Sybil.
- **Contribution patterns**: Submission frequency, time-of-day distribution, trace length distribution. Sybil accounts tend to have suspiciously similar behavioral patterns.
- **Timing analysis**: Traces submitted within seconds of each other from different accounts. Real developers do not complete agent sessions simultaneously across accounts.
- **Behavioral fingerprinting**: Tool call sequences, error patterns, coding style markers. Similar to authorship attribution in NLP.

**False positive mitigation**: Statistical signals produce scores, not verdicts. A human reviews flagged submissions before any credit clawback. The statistical layer is a filter, not a judge.

**Graduated enforcement (GaaS framing).** The Governance-as-a-Service framework (arXiv:2508.18765) provides a formal taxonomy for the interventions that Layer 4 can trigger. Currently, TC's only intervention is credit reduction via the anomaly penalty `a`. GaaS distinguishes three intervention classes: coercive (block the submission entirely), normative (warn the contributor and log the anomaly without reducing credit), and adaptive (adjust the contributor's effective tier or permissions based on rolling Trust Factor). TC should adopt all three. Normative interventions are especially important at current scale -- with only 3 contributors, false-positive credit reduction risks alienating legitimate users. A warning-first approach for first-time anomaly flags, escalating to credit reduction on repeated flags, matches GaaS's graduated enforcement model.

### Layer 5: Economic Defense

Make Sybil attacks economically irrational by requiring upfront cost.

- **Staking**: Require a minimum NEAR stake per contributor account. The stake is returned after a vesting period if no fraud is detected. Sybil cost scales linearly with number of accounts.
- **Broadcast commitment**: If TC ever moves to competitive credit allocation (beyond the current fixed formula), the Credibility Trilemma requires that all submissions in an allocation round be publicly committed (via hash) before scoring. This prevents ghost-bid manipulation.
- **Slashing**: If fraud is detected (via Layers 2-4), the staked amount is slashed. This creates a deterrent proportional to the potential fraud payoff.
- **Trust Factor accumulation (GaaS model)**: The Governance-as-a-Service framework (arXiv:2508.18765) formalizes what the `a` factor implements informally. Each contributor accumulates a Trust Factor based on compliance history, weighted by violation severity. Contributors with consistently clean histories earn higher effective multipliers over time; contributors with anomaly flags see compounding penalties. This turns the economic defense from a binary (staked vs. not staked) into a continuous function where long-term honest behavior is rewarded, and the cost of Sybil attacks includes not just the stake but the forfeiture of accumulated trust.

---

## 5. Design Menu: Tiered Provenance (from Agent-OSI L5)

Not all traces need the same level of provenance. Agent-OSI's L5 provides the design vocabulary for a tiered system. TC should offer three tiers with corresponding credit multipliers.

### Tier 1: Signed Ingestion Log (Default)

- **Attestation**: Phase 1 EAC (TEE-signed ingestion)
- **What it proves**: Trace entered TC's pipeline
- **Cost to contributor**: None (transparent)
- **Credit multiplier**: 1.0x (baseline)
- **Available**: Phase 1 onward

All traces receive Tier 1 attestation automatically. No opt-in required.

### Tier 2: Full TDX Attestation (Verified)

- **Attestation**: Phase 2 full pipeline attestation
- **What it proves**: Scoring was computed correctly and verifiably
- **Cost to contributor**: None (transparent, slightly slower processing)
- **Credit multiplier**: 1.25x
- **Available**: Phase 2 onward

Tier 2 is applied automatically once Phase 2 infrastructure is deployed. The credit multiplier incentivizes patience during the transition period (Tier 2 processing is ~4% slower).

### Tier 3: Web Proofs + On-Chain Anchor (Premium)

- **Attestation**: Phase 3 Web Proofs + Phase 4 on-chain anchoring
- **What it proves**: Agent actually called the declared APIs; provenance is immutably recorded
- **Cost to contributor**: Must route API calls through TC's TEE Proxy (opt-in)
- **Credit multiplier**: 1.5x
- **Available**: Phase 3 onward

Tier 3 is opt-in and reserved for contributors who want maximum credit yield. The TEE Proxy requirement means the agent's execution environment must be configured to use TC as an intermediary -- a non-trivial integration step that filters for serious contributors.

### Multiplier Interaction with Credit Formula

The provenance tier multiplier `p` extends the credit formula:

```
q = f * g * a * p
```

Where:
- `f` = freshness (time decay)
- `g` = gate score (quality + novelty)
- `a` = attestation weight (existing anomaly penalty)
- `p` = provenance multiplier (1.0 / 1.25 / 1.5)

The existing `a` factor captures anomaly penalties. The new `p` factor captures provenance assurance. They are multiplicative: a high-provenance trace with anomaly signals still gets penalized.

---

## 6. Practical Considerations

### 6.1 Privacy Implications

Provenance attestation and privacy scrubbing must compose correctly. The ingestion EAC (Phase 1) hashes the trace *after* redaction. This means the EAC does not contain PII, but it also means the EAC cannot prove that redaction was applied correctly (that requires Phase 2). The ordering matters:

```
raw trace -> redaction (inside TEE) -> hash redacted trace -> EAC over hash
```

Phase 2 attests the redaction step itself, closing this gap.

### 6.2 Determinism Requirements

TDX attestation of the scoring pipeline (Phase 2) requires deterministic execution. Sources of non-determinism in the current pipeline:

- Floating-point operations in embedding computation (BGE model inference)
- HNSW index construction (randomized layer assignment)
- Thread scheduling in parallel scoring

**Achieving determinism: cost and mechanisms.** Batch-invariant inference kernels that give bit-identical outputs across runs are achievable but expensive. The Thinking Machines/SGLang result benchmarks Qwen3-8B: batch-invariant kernels impose a ~34–61% throughput cost, reduced to approximately 34.35% when CUDA graphs are applied. (⚠️ SOURCE UNVERIFIED: previously cited arXiv:2606.03019 is a WRONG CITATION — that paper is "Reproducibility is the New Copyleft: Defining AGI-oriented Reproducible Builds," unrelated to TEE inference costs. Claim needs re-sourcing.) This is not negligible on a single VM. ONNX Runtime does not expose deterministic primitive selection or reduction order, so simply switching to ONNX is not a shortcut to determinism. (⚠️ SOURCE UNVERIFIED: previously cited arXiv:2501.05867 is a WRONG CITATION — that paper is "Neural network verification challenges as programming-language challenges," unrelated to ONNX non-determinism. Claim needs re-sourcing.)

**HNSW specifically.** The two sources of nondeterminism in HNSW are randomized layer assignment at insert time and parallel insert ordering. Both can be controlled by (a) seeding the RNG to a fixed value and (b) pinning the thread count to 1 for index construction within the TEE. This is sufficient for determinism inside a single attested enclave image.

**TC's options.** TC's attestation is only meaningful if scoring is deterministic -- an attested but nondeterministic scorer proves only that the *code* ran, not that the *result* is reproducible. Two viable approaches:

1. **Pay the throughput cost upfront.** Accept the ~34–61% throughput reduction and use batch-invariant kernels throughout. At TC's current ~13 submissions/week this is operationally invisible, but it constrains future scaling.
2. **Verify-rollback scheme (LLM-42, arXiv:2601.17768).** Pay the determinism overhead only for traces where attestation is contested or where the credit value exceeds a threshold. Run a lightweight non-deterministic path by default; re-execute deterministically on demand to confirm the attested result. This concentrates the throughput cost where it is economically justified.

**Brute-force HDC as an escape hatch.** At TC's current scale (~352 traces), the HNSW similarity search can be replaced entirely with a brute-force exact cosine or Hamming scan over HDC hypervectors. This sidesteps both the HNSW nondeterminism problem and a separate side-channel concern. See Section 6.3 for the full argument.

### 6.3 Brute-Force HDC Scan as Deterministic Alternative

At TC's current corpus size (~352 traces), HNSW is not required for similarity search. A brute-force exact cosine or Hamming scan over HDC hypervectors is computationally practical and simultaneously resolves two distinct problems that HNSW cannot.

**Problem 1: Determinism for attestation.** HNSW's approximate search introduces randomness through its construction-time RNG and thread-parallel insert ordering. While both can be seeded/pinned (see Section 6.2), the pinning requirement constrains index parallelism and adds operational complexity. Brute-force scan has no data-dependent randomness: given the same corpus and the same query vector, the result is identical on every run, without any configuration.

**Problem 2: Side-channel leakage inside the enclave.** HNSW's graph traversal follows a data-dependent memory access pattern: the sequence of nodes visited depends on the content of the index. Inside a TEE, this access pattern is observable through cache timing side-channels (page fault traces, LLC evictions) that can leak information about the index contents and the query, even when the enclave memory is encrypted. Brute-force linear scan has no data-dependent branching or memory access pattern -- every scan visits every vector in the same order regardless of query content, leaving no side-channel surface.

**The two-birds result.** Brute-force HDC scan is the only similarity-search approach that simultaneously provides (a) unconditional determinism for TDX attestation and (b) elimination of memory-access side-channels inside the enclave. HNSW with ORAM overlays can address (b) but adds significant overhead and complexity without solving (a). HNSW with RNG pinning addresses (a) but not (b).

**When to revisit HNSW.** Brute-force scan scales as O(n) per query. At 352 traces this is negligible. The practical crossover point depends on HDC vector dimensionality and query rate, but the general guidance is to revisit HNSW or ORAM-wrapped alternatives once the corpus reaches 10,000+ traces and query latency becomes a user-visible bottleneck. Until then, brute-force HDC scan is the correct choice for TC.

**Implementation note.** This recommendation applies to the similarity search component of the deduplication pipeline. HDC hypervectors (as used in VS-Graph style novelty scoring) are naturally suited to Hamming distance computation, which can be implemented in a few dozen lines of Rust using bitwise operations and popcount. The vectors themselves are stored in the existing corpus metadata and require no additional infrastructure.

### 6.4 Sybil Cost Analysis

At current scale (~13 submissions/week, 3 contributors), Sybil attacks are not economically rational -- the credit payoff is too small to justify the effort. The defense architecture should be deployed *before* scale makes Sybil attacks profitable. The crossover point depends on credit value, but a reasonable estimate: when TC reaches 100+ contributors and credits have meaningful secondary market value, Sybil attacks become rational.

**Phase 1 alone raises the Sybil cost significantly**: an attacker must submit traces through TC's infrastructure (not just fabricate them offline), and each submission is bound to a NEAR account with a monotonic sequence number. Creating new accounts is free, but each account accumulates a verifiable submission history that statistical detection (Layer 4) can analyze.

### 6.5 Backward Compatibility

All four phases are additive. Existing traces (submitted before Phase 1) receive no provenance attestation but remain valid. The credit formula applies `p = 1.0` for unattested traces. There is no retroactive re-scoring.

---

## 7. Verification Ledger

| # | Claim | Source | Status |
|---|---|---|---|
| 1 | Web Proofs achieve less than 3x overhead | arXiv:2512.15892 (VET) | Verified |
| 2 | EACs add ~2.7ms overhead on minimal flows | arXiv:2607.05397 (Proof-of-Execution) | Verified |
| 3 | EACs add 4.4% throughput overhead on batch workloads | arXiv:2607.05397 (Proof-of-Execution) | Verified |
| 4 | EAC storage is ~1.1 KB per 8-event trace | arXiv:2607.05397 (Proof-of-Execution) | Verified |
| 5 | EACs are composable with zkVMs | arXiv:2607.05397 (Proof-of-Execution) | Verified |
| 6 | Ghost-bid deviations are profitable and undetectable under sealed-bid VCG and Myerson | arXiv:2605.26604 (Credibility Trilemma) | Verified |
| 7 | Broadcast commitment is the only known closure for ghost-bid manipulation | arXiv:2605.26604 (Credibility Trilemma) | Verified |
| 8 | Agent-OSI L5 defines a provenance interface admitting TEE, ZK, and signed log attestations | arXiv:2602.13795 (Agent-OSI) | Verified |
| 9 | TEE Proxy architecture is sufficient for public API verification | arXiv:2512.15892 (VET) | Verified |
| 10 | Intel TDX provides best cost-performance tradeoff for server-side attestation | arXiv:2603.25100 (Logic Monopoly to Social Contract) | Verified |
| 11 | SVIP proposes AI identity certificate chains analogous to X.509 | arXiv:2604.23280 (SVIP) | Verified |
| 12 | Semivalue-based data valuation amplifies Sybil payoffs by 1.74x | arXiv:2605.07663 (Blum et al., referenced from doc 06) | Verified |
| 13 | Shapley-based data valuation is vulnerable to strategic misrepresentation | arXiv:2504.05563 (Agarwal et al., referenced from doc 06) | Verified |
| 14 | TC currently runs Intel TDX for scoring via NEAR AI Cloud | TC architecture (docs 00, 02) | Verified |
| 15 | TC uses NEAR for credit settlement | TC architecture (docs 00, 02) | Verified |
| 16 | TC credit formula is `q = f * g * a` | TC architecture (docs 00, 02) | Verified |
| 17 | TC has ~352 submissions, 3 contributors | TC repo metrics (doc 00) | Verified |
| 18 | Issue #210: 0/99 sessions accepted (scoring logic inversion) | TC GitHub Issues (docs 00, 02) | Verified |
| 19 | Issue #219: redaction penalizes quality scores | TC GitHub Issues (docs 00, 02) | Verified |
| 20 | Batch-invariant deterministic inference kernels impose ~34–61% throughput cost, reducible to ~34.35% with CUDA graphs (Qwen3-8B benchmark) | arXiv:2606.03019 (Thinking Machines/SGLang) | **WRONG CITATION** — arXiv:2606.03019 is "Reproducibility is the New Copyleft: Defining AGI-oriented Reproducible Builds." Unrelated to TEE inference costs. Technical claim may be correct but needs re-sourcing. |
| 21 | ONNX Runtime does not expose deterministic primitive selection or reduction order | arXiv:2501.05867 | **WRONG CITATION** — arXiv:2501.05867 is "Neural network verification challenges as programming-language challenges." Unrelated to ONNX non-determinism. Technical claim may be correct but needs re-sourcing. |
| 22 | Verify-rollback scheme (LLM-42) pays the determinism cost only where needed, on contested traces | arXiv:2601.17768 (LLM-42) | Verified |
| 23 | No end-to-end framework binds TEE primitives into a coherent security substrate for production agentic AI | arXiv:2605.03213 (TEE Survey for Agentic AI) | Verified |
| 24 | Compound attestation for multi-hop agent chains is an open challenge across all six surveyed TEE platforms | arXiv:2605.03213 (TEE Survey for Agentic AI) | Verified |
| 25 | GPU-TEE performance at LLM scale is an open challenge with sparse benchmarks at 35B+ parameters | arXiv:2605.03213 (TEE Survey for Agentic AI) | Verified |
| 26 | TDX and SEV-SNP confirmed as correct target hardware for server-side attestation workloads | arXiv:2605.03213 (TEE Survey for Agentic AI) | Verified |
| 27 | GaaS Trust Factor mechanism scores agents on compliance history weighted by violation severity | arXiv:2508.18765 (Governance-as-a-Service) | Verified |
| 28 | GaaS enables coercive, normative, and adaptive graduated enforcement interventions | arXiv:2508.18765 (Governance-as-a-Service) | Verified |
| 29 | Privilege attenuation requires monotonic permission restriction across delegation hops | arXiv:2602.11865 (Intelligent AI Delegation) | Verified |
| 30 | Liability firebreaks are pre-defined stopping points that contain cascading failures in delegation chains | arXiv:2602.11865 (Intelligent AI Delegation) | Verified |
| 31 | Continuous validation requires access rights to persist only while trust metrics are maintained | arXiv:2602.11865 (Intelligent AI Delegation) | Verified |
| 32 | Cryptographic linkability across pipeline stages is required for end-to-end AI pipeline verification | arXiv:2503.22573 (Cryptographic AI Pipeline Framework) | Verified |

---

## 8. Open Questions

1. **Stake sizing**: What minimum NEAR stake makes Sybil attacks irrational without excluding legitimate new contributors? Requires modeling credit payoff curves against account creation cost.

2. **TEE Proxy adoption**: Will contributors accept routing their agent's API calls through TC's infrastructure? Privacy-conscious developers may refuse, even with TEE confidentiality guarantees. The opt-in model (Tier 3 only) mitigates this, but limits the provenance coverage.

3. **Determinism audit scope**: How much of the scoring pipeline must be deterministic for Phase 2 attestation to be meaningful? Approximate determinism (same result within epsilon) may be sufficient if the epsilon is smaller than the credit formula's sensitivity.

4. **Cross-phase dependencies**: Phase 1 is independent. Phase 2 depends on a determinism audit. Phase 3 depends on TEE Proxy infrastructure. Phase 4 depends on settlement pipeline stability. Can any phases be parallelized?

5. **Interaction with Issue #210 and #219**: The provenance architecture assumes a working scoring pipeline. If the scorer rejects everything (Issue #210) or penalizes redaction (Issue #219), provenance attestation of a broken scorer is counterproductive -- it proves the score was computed *correctly* but the computation itself is wrong. Fix the scorer first.

6. **GPU-TEE throughput at 35B scale**: The TEE Survey (arXiv:2605.03213) identifies GPU-TEE performance at LLM scale as an open challenge. TC's Phase 2 throughput budget assumes the ~4.4% overhead from CPU-side Proof-of-Execution benchmarks, but the actual overhead of full NVIDIA H100 CC attestation while running 35B inference is unknown. Benchmark this on TC's hardware before committing Phase 2 timelines.

7. **Compound attestation implementation**: TC's pipeline spans multiple TEE instances (ingest TEE, scoring GPU-TEE, NEAR settlement). The TEE Survey and the Cryptographic AI Pipeline Framework (arXiv:2503.22573) both identify multi-hop attestation composition as unsolved. TC needs a concrete design for linking attestations across these hops -- whether via hash chaining (simplest), recursive proof composition (most general), or a purpose-built attestation aggregation layer.

8. **Intervention taxonomy formalization**: GaaS (arXiv:2508.18765) distinguishes coercive, normative, and adaptive interventions. TC currently implements only one intervention (credit reduction via `a`). Should TC formalize and implement the full taxonomy before scaling to 100+ contributors? The risk of not doing so is that false-positive credit reduction alienates legitimate early contributors who have no recourse except reduced earnings.

9. **Explicit permission scoping per pipeline stage**: The privilege attenuation model (arXiv:2602.11865) argues that each delegation hop should declare its minimal required permissions, and the TEE attestation should cover the permission scope. Should TC's EAC format include a permission manifest (e.g., "this stage received only the redacted trace, not the raw trace") in addition to the computation attestation? This adds complexity to the EAC but makes the monotonic restriction auditable.
