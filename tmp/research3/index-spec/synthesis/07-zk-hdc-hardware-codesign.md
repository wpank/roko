# Zero-Knowledge Proofs over HDC Vectors and Hardware Co-Design

This document describes a three-layer technology stack -- Hyperdimensional Computing (HDC), zero-knowledge proofs (ZK) native to binary vectors, and purpose-built hardware acceleration -- and explains why their combination creates capabilities that no existing agent platform, blockchain, or machine-learning system provides independently. It is written for someone with no prior context on any of these technologies or the systems that use them.

---

## 1. Hyperdimensional Computing Fundamentals

### What HDC Is

Hyperdimensional Computing is a computational paradigm that represents information as very long binary vectors -- in this system, 10,240 bits (1,280 bytes), called "hypervectors." The core insight is mathematical: in spaces of sufficiently high dimension, randomly generated vectors are nearly orthogonal to each other with overwhelming probability. Two random 10,240-bit vectors have an expected Hamming similarity of 0.500, with a standard deviation of approximately 0.005. Any similarity above 0.526 reflects genuine structural relationship rather than coincidence (p < 0.001).

Information is distributed holographically across all 10,240 bits -- no single bit encodes a specific feature. Destroying 10% of the bits degrades similarity scores slightly but does not destroy the representation. This noise robustness is a structural property of high-dimensional geometry.

The system stores these vectors as `[u64; 160]` arrays, enabling efficient bitwise operations via native CPU instructions. The implementation lives in `roko-primitives`, providing the `HdcVector` type with deterministic seeding (FNV-1a + SplitMix64) and serialization to 1,280 little-endian bytes.

### Three Core Operations

HDC has exactly three algebraic operations, each with a clear analogy to conventional arithmetic:

**Binding (XOR)** is the multiplicative operation. `A XOR B` is computed as 160 parallel 64-bit XOR instructions. Binding is an involution: `bind(bind(A, B), B) = A` -- it is its own inverse. This enables role-filler composition: bind a "role" vector (e.g., "author") with a "filler" vector (e.g., "Alice"), and recover "Alice" later by binding the compound with "author" again. Structured facts like `(author: Alice, topic: cryptography)` are encoded as the bundle of `bind(role_author, filler_alice)` and `bind(role_topic, filler_crypto)`.

**Bundling (majority vote)** is the additive operation. Each output bit is 1 if more than half of the N input vectors have a 1 in that position, 0 otherwise (ties break to 0). The bundled vector is similar to all constituents -- a "superposition" in hypervector space. The system provides incremental accumulators (`BundleAccumulator`, `DecayingBundleAccumulator`) supporting weighted addition, temporal decay, and streaming updates.

**Permutation (bit rotation)** is the sequence-encoding operation. Rotating left by N positions produces a nearly orthogonal but deterministically recoverable vector. The i-th sequence element is permuted i times before bundling, encoding positional information in a single fixed-size vector.

### Why Binary Vectors Matter for Zero-Knowledge Proofs

Binary vectors live in the Galois field GF(2) (F_2), where addition IS XOR -- requiring zero multiplication gates in an arithmetic circuit. Zero-knowledge proof systems measure cost in field operations, so a system built on F_2 arithmetic starts with a structural advantage: binding (the most common HDC operation) has zero cost in the proof system's native accounting.

Hamming distance is computed as `popcount(A XOR B)`. The XOR is free in F_2; the popcount decomposes into a sum of bit values (linear over F_2). The fundamental HDC similarity metric is expressible as a small arithmetic circuit in binary-field proof systems, without the field-embedding overhead that plagues prime-field approaches.

### Applications in the Stack

HDC vectors serve seven simultaneous functions in the system:

1. **Semantic similarity search.** Knowledge entries, code artifacts, and agent outputs are fingerprinted as HDC vectors. Retrieval is nearest-neighbor search in Hamming space, running at approximately 1 microsecond per query on CPU via hardware `POPCNT` instructions.
2. **Agent fingerprinting.** Each agent's behavioral profile -- its execution patterns, tool usage, model preferences, and output characteristics -- is encoded as an HDC vector that evolves over time via decaying bundle accumulation.
3. **Memory retrieval.** The knowledge store uses HDC similarity for context retrieval, with a `Codebook` system that maps symbolic names to deterministic vectors and supports cross-domain resonance detection (identifying when patterns from different domains share structural similarity above the 0.526 threshold).
4. **Reputation primitives.** Agent capability profiles are HDC vectors registered on-chain. Proving competence in a domain means proving proximity (low Hamming distance) to a reference vector for that domain -- without revealing the agent's full capability profile.
5. **Active-inference routing.** The cascade router uses HDC fingerprints to match incoming tasks to the most appropriate model tier.
6. **Quality-diversity measurement.** HDC distance serves as a novelty metric in archive-based self-improvement (the Darwin Godel Machine pattern).
7. **Immune system pattern matching.** Known attack patterns are stored as HDC vectors; incoming signals are checked against this recognition library via similarity search.

---

## 2. Zero-Knowledge Proofs Native to Binary Vectors

The goal is to prove statements about HDC vectors -- "these two vectors are within Hamming distance threshold T" or "this vector was computed correctly from these inputs" -- without revealing the vectors themselves. Five proof system approaches are relevant, each with different maturity levels and tradeoffs.

### Binius (Irreducible, September 2025)

Binius is a SNARK built natively over F_2, developed by Irreducible (formerly Ulvetanna). The cryptographic construction was presented at EUROCRYPT 2024 (Diamond, Posen, et al., ePrint 2023/1784). Because Binius operates natively in F_2, XOR costs zero in constraint accounting. A 10,240-bit Hamming-distance threshold check requires only a few thousand witness words, compared to tens of thousands in prime-field SNARKs.

Performance (September 2025): SHA-512 preimage proof in 128ms on single CPU core. Batching 10,000 HDC similarity proofs estimated at 1-3 seconds total. Commit time approximately 30x faster than Plonky3-FRI for binary witnesses. Post-quantum secure (hash-based commitments, no pairings, no trusted setup).

**Critical caveat**: The zero-knowledge property (witness hiding) is on Irreducible's roadmap for late 2025 but has not shipped. Binius currently provides integrity proofs but does not hide the witness. Recursion (needed for on-chain batch verification) is also unavailable. Plan accordingly: use Binius for integrity proofs now, add privacy when the ZK property ships.

### Lasso + Jolt (a16z, EUROCRYPT 2024)

Lasso is a lookup-argument system; Jolt is a zkVM built on Lasso (Setty, Thaler, et al., ePrint 2023/1216 and 2023/1217, EUROCRYPT 2024). Jolt compiles RISC-V programs into zkSNARK proofs -- 5x faster than RISC-Zero, 2x faster than SP1 on RV32 traces.

For HDC: build a `popcnt_xor_threshold_10240` precompile (custom instruction for `popcount(A XOR B)` over 10,240-bit inputs with threshold check). Jolt's lookup-table instruction semantics make custom precompiles natural -- expected 50-100x speedup over emulated POPCNT in a general-purpose zkVM.

Lasso + Jolt is the practical bridge while Binius matures: larger and slower proofs, but production-ready today with full ZK and recursion support.

### Worldcoin / TACEO MPC Architecture (ePrint 2024/705)

Worldcoin's iris recognition system is the only large-scale production deployment performing exactly this computation: comparing binary vectors of comparable length (12,800-bit iris codes vs. 10,240-bit HDC vectors) against a fractional Hamming distance threshold (0.375 for iris uniqueness). Described in "Scalable Private Biometric Matching" (ePrint 2024/705, Corrigan-Gibbs, Lubin, Rindal, et al.), it uses a three-party honest-majority MPC protocol on 1,152 CPU cores for million-scale iris deduplication. Each iris code is secret-shared across three non-colluding servers performing XOR and popcount over their shares.

The relevance is architectural: Worldcoin's federated Verify-oracle topology maps directly to HDC reputation. Instead of iris codes, secret-share agent capability vectors. Instead of "is this a new person?", query "does this agent match domain X at threshold T?" The MPC protocol is identical; only application semantics change.

### Compact Hamming-Weight ZK Proofs (Bouaziz-Ermann et al.)

Bouaziz-Ermann, Canard, Cosseron, Eberhart, and Music propose ZK proofs for Hamming weight with proof sizes independent of vector length. The key technique is Property-Preserving Hashing (PPH): a hash function H(x) that preserves enough structure for threshold queries ("is Hamming distance between x and y below T?") while being non-invertible. Published HDC fingerprints become PPH commitments supporting similarity queries without enabling reconstruction.

Complementary to Binius: PPH provides compact on-chain commitments; Binius provides efficient threshold-query verification against those commitments.

### TFHE Acceleration for HDC

TFHE (Torus Fully Homomorphic Encryption) enables computation on encrypted data, optimized for binary circuits. After 2024-2025 sorted-bootstrapping improvements, gate-level operations cost approximately 3-8ms per gate.

**EP-HDC** demonstrates that HDC's binary operations are naturally expressible as TFHE circuits -- both are native Boolean operations, so there is no representation mismatch. **"HDC as Rescue for PPML"** shows HDC classifiers in TFHE achieve 10-100x lower FHE cost than neural network classifiers, because HDC avoids multiplication-heavy matrix operations.

The practical capability: encrypted similarity search. An agent queries a knowledge store with an encrypted capability vector, receives ranked results, and decrypts only the results -- the store never sees the query vector. This supports private agent-to-agent capability discovery.

---

## 3. Hardware Co-Design for HDC

### The Performance Target

The target is: 1 microsecond per operation, all 7 HDC functions running in parallel, 10 million similarity searches per second. This would enable real-time agent routing at scales of 10,000+ concurrent agents, where each routing decision requires comparing an incoming task's HDC fingerprint against the full agent registry.

On current commodity CPUs (Intel/AMD with AVX-512 and POPCNT), a single 10,240-bit Hamming distance computation takes approximately 1 microsecond. This is sufficient for small deployments but becomes a bottleneck at 10,000+ agents: brute-force search through 10,000 agents takes 10 milliseconds, which is too slow for the 50-millisecond block-time target of the Korai blockchain.

### AMD Versal Premium FPGA

The AMD (Xilinx) Versal Premium adaptive SoC combines FPGA fabric with AI Engine tiles -- fixed-function VLIW/SIMD processors. The VP1902 provides 400 AI Engine tiles at 1.25 GHz. Repurposed for HDC: parallel XOR-and-popcount over 10,240-bit vectors, with 7 HDC functions executing simultaneously on separate tile subsets. Back-of-envelope: 400 tiles yield approximately 500 million comparisons per second (50x above target) at approximately 75W. Commercially available with production tooling (Vitis HLS, Vivado). Reachable in months 4-9 of a prototype path.

### IBM NorthPole (ISSCC 2024)

NorthPole (IBM Research, ISSCC 2024) achieves 611 frames/J on ResNet-50 -- 25x energy efficiency and 22x latency reduction vs. V100 -- by co-locating compute and memory at core level. Its architecture suits HDC (core-level SRAM for codebook vectors, configurable datapath for XOR-popcount), but NorthPole is a closed ecosystem with no public driver or commercial availability. Listed as an efficiency reference, not a near-term target.

### Intel Loihi 2 and Hala Point

Loihi 2 (Intel) implements 128 neuromorphic cores with programmable neuron models and asynchronous spike-based communication. Hala Point scales to 1.15 billion neurons across 1,152 chips at 15 TOPS/W INT8.

The HDC connection is direct: Frady, Kleyko, and Sommer's resonator network work (Neural Computation, 2018) maps HDC factorization -- decomposing bundled vectors into constituent bound pairs -- to neuromorphic spike dynamics, converging in O(log N) spikes for N codebook entries. Demonstrated on silicon. Loihi 2 serves as a dedicated co-processor for codebook operations (binding/unbinding, pattern lookup, cross-domain resonance), while FPGAs handle high-throughput similarity search.

### SpiNNaker 2 and SpiNNcloud

SpiNNaker 2 (University of Manchester / TU Dresden) packs 152 ARM Cortex-M4F cores per chip, connected by a multicast spike-packet network. The architectural insight: the multicast fabric IS a dispatch layer -- a task fingerprint multicasts to all cores simultaneously, each computing similarity against its local agent-registry subset. Sub-microsecond on-chip latency. SpiNNcloud Systems GmbH offers commercial boards and cloud access.

### In-Memory Computing with Phase-Change Memory (Langenegger et al., Nature Nanotechnology 2024)

Langenegger, Karunaratne, Hersche, Benini, Sebastian, and Rahimi (Nature Nanotechnology, 2024) demonstrate HDC factorization using phase-change memory (PCM), where analog conductance states encode hypervector elements and crossbar arrays compute similarity in a single read cycle. Results: problems 5 orders of magnitude larger than software baselines, 6.6x energy reduction. Limitation: PCM endurance (write degradation) restricts this to read-heavy workloads (codebook lookup, similarity search), not write-intensive bundle accumulation.

### HyDra SOT-CAM (Georgia Tech, ICCAD 2025)

HyDra (Georgia Tech, ICCAD 2025) is a Spin-Orbit Torque Content-Addressable Memory (SOT-CAM) for HDC. CAM performs parallel search across all stored entries in a single clock cycle. HyDra implements binding, permutation, and similarity directly in-array, eliminating data-movement bottleneck. Each tile achieves 300,000 queries/second; tile-parallel scaling is linear, so 34 tiles reach 10 million queries/second -- the system's target.

### FPGA Prototype Path

The hardware co-design follows a four-phase prototype path:

**Phase 0 (weeks 0-4): AMD Kria KV260.** The KV260 is a $250 development board with a Zynq UltraScale+ MPSoC. It has enough FPGA fabric for a single-function HDC accelerator (similarity search only, ~1,000 vectors). The goal is to validate the XOR-popcount pipeline in hardware and measure actual throughput against the software baseline. Deliverable: a working HDC similarity search accelerator with a PYNQ (Python) host interface.

**Phase 1 (months 1-4): Alveo U280.** The Alveo U280 is a data-center FPGA accelerator card with HBM2 memory (8 GB, 460 GB/s bandwidth). HBM bandwidth enables storing and searching large codebooks (millions of vectors) without memory bottleneck. The goal is to implement all 7 HDC functions and demonstrate 10x throughput over CPU. Deliverable: PCIe-attached HDC accelerator card usable from the Roko agent runtime.

**Phase 2 (months 4-9): Versal Premium VP1902.** Full AI Engine utilization for 10-million-searches-per-second target. Integration with ZK proof generation: the FPGA generates HDC similarity results and feeds them to a CPU-side Binius/Jolt prover. Deliverable: integrated HDC-search + ZK-prove pipeline.

**Phase 3 (months 9-18): SpiNNaker 2 + ASIC pre-RTL.** Explore neuromorphic co-processing for resonator-network workloads. Begin ASIC design studies using the FPGA implementation as the behavioral reference. Deliverable: architecture trade study for a custom HDC-ZK ASIC.

---

## 4. ZK-Attested HDC as On-Chain Reputation Primitives

### How It Works

Agent reputation is on-chain attestations backed by ZK proofs over HDC vectors:

1. **Fingerprint computation.** An agent accumulates a capability vector via `DecayingBundleAccumulator` encoding breadth and recency of experience (e.g., 50 coding tasks + 20 security audits). The codebook maps domain names to deterministic seed vectors identical across all agents.

2. **ZK proof generation.** The agent proves (via Binius or Jolt) that its capability vector is within Hamming distance T of a domain reference vector -- demonstrating competence without revealing the full capability profile.

3. **On-chain registration.** The proof is submitted to the Validation Registry (ERC-8004), storing: soulbound NFT identity, domain claim, proof, and a Property-Preserving Hash (PPH) of the capability vector for future threshold queries without re-proving.

4. **Composable verification.** Agents, job markets, and clearing systems verify proofs on-chain in O(1) time. "Coding reputation > 0.7" translates to "Hamming similarity to coding reference > 0.7," verifiable against the stored PPH commitment.

### Closest Analog: Worldcoin

Worldcoin is the closest analog: one binary vector type (12,800-bit iris codes), one purpose (biometric deduplication), federated MPC for privacy. The system generalizes in three dimensions:

- **Vector type.** Worldcoin: one encoding (iris wavelet transform). This system: open-ended codebook -- capabilities, behavioral profiles, knowledge, code structure, task characteristics. Any serializable value fingerprints into 10,240 bits.
- **Purpose.** Worldcoin: "is this a unique person?" This system: "is this agent competent in X?", "is this knowledge novel?", "does this task match this agent?", "is this trace anomalous?"
- **Scale topology.** N Verify-oracles (not just 3), each holding a reputation-database share, collectively computing match/no-match via MPC with identical cryptographic guarantees.

### Compound Defense: Four Layers

Agent reputation built on HDC vectors requires defense against multiple attack vectors. The system implements four complementary layers:

**HDLock privileged encoding** (Hernandez-Cano, Matsumoto, Karunaratne, Rahimi, et al.) uses a secret permutation key to encode codebooks. Without the key, an adversary cannot reconstruct vectors even with unlimited query access -- preventing capability-profile extraction.

**Variance Inequality on density** detects anomalous bit-density (fraction of 1-bits). Legitimate vectors have density near 0.5; Sybil vectors crafted to match multiple domains produce anomalous distributions.

**CaMeL provenance** (Fang et al., 2024) tracks origin and transformation history of every data element. Reputation attestations carry provenance tags; forged attestations lack the correct chain.

**ZK attestation on critical fingerprints** ensures the capability-vector-to-domain relationship is verifiable without trusting the agent or any single third party.

---

## 5. Adversarial Vulnerabilities in HDC

HDC's noise robustness -- the property that random bit flips degrade similarity scores gradually rather than catastrophically -- does NOT imply adversarial robustness. This distinction is critical and often misunderstood.

### Known Attacks

**Yang and Ren (2020)** demonstrated grey-box attacks achieving 78%+ misclassification by crafting inputs near the Hamming-space decision boundary. Only black-box query access is needed to construct effective adversarial examples.

**Li et al. (ACM TACO, 2025)** extend these attacks to hardware-accelerated HDC, showing comparable misclassification across CPU, GPU, and FPGA implementations -- hardware acceleration does not inherently improve adversarial robustness.

### Why Noise Robustness Is Not Adversarial Robustness

Random noise distributes uniformly, and high dimensionality ensures gradual degradation. Adversarial perturbations are concentrated on bits that maximally shift similarity relative to the decision boundary. In 10,240 dimensions, flipping as few as 2-3% of bits (Yang and Ren) suffices to cross the boundary -- the adversary finds the shortest path to the hyperplane.

### Required Layered Defense

The system cannot rely on HDC's noise robustness for security. Instead, it deploys the four-layer compound defense described in Section 4 (HDLock, Variance Inequality, CaMeL provenance, ZK attestation). Additionally:

- **Margin enforcement.** Reputation thresholds should include a safety margin beyond the theoretical decision boundary. Instead of requiring similarity > 0.526 (the 3-sigma threshold for genuine similarity), requiring similarity > 0.60 provides a buffer against adversarial perturbation.

- **Temporal consistency.** Agent capability vectors are accumulated over time via `DecayingBundleAccumulator`. An adversary attempting to shift an agent's profile must sustain adversarial inputs over many interactions, each of which is independently logged and subject to gate verification. This makes one-shot adversarial attacks ineffective against the accumulated profile.

- **Multi-domain cross-validation.** An agent claiming high reputation in domain X should have a plausible profile across related domains. Cross-domain resonance detection (checking whether the agent's coding profile is structurally consistent with its security profile, for example) catches synthetic profiles that are optimized for a single domain but incoherent across domains.

---

## 6. Why This Matters: Benchmarks, Agents, and the Missing Combination

### Private Benchmark Data Verification

A regulated benchmark requires input data to be verifiable without being publicly visible. ZK-attested HDC solves this: a data-submitting agent fingerprints its transaction dataset, generates a ZK proof of consistency with claimed characteristics (volume, price distribution, counterparty diversity), and submits the proof alongside the rate observation. The administrator verifies without seeing underlying transactions, satisfying IOSCO Principle 7 (data sufficiency) and Principle 9 (transparency) while preserving commercial confidentiality.

No existing benchmark -- SOFR (centralized NY Fed collection), CF Benchmarks (exchange API under NDA), CESR (validator self-reporting) -- has this capability.

### Real-Time Agent Routing at Scale

Agent routing is a similarity-search problem: compare a task's HDC fingerprint against all registered agents' capability vectors. At 100 agents, CPU search takes 100us (fine). At 10,000, 10ms (marginal). At 100,000, 100ms (too slow).

Hardware acceleration reduces search to sub-microsecond regardless of count, enabling: full-registry search on every cognitive-loop tick (1-5s gamma timescale), real-time load balancing across 100,000+ agents, and instant re-routing on capability-profile changes.

### The Unique Combination

No other agent stack combines these three layers:

1. **HDC as universal representation.** Other agent systems use neural-network embeddings: GPU-dependent, model-specific (OpenAI vs. Cohere incompatible), expensive in ZK circuits. HDC vectors are computed without neural networks, model-independent, and native to binary-field ZK proofs.

2. **ZK proofs native to the representation.** General-purpose zkVMs (RISC-Zero, SP1) emulate binary operations in prime fields. Binius and Hamming-weight proofs operate in the same F_2 field as the data -- 30x faster proof generation, orders of magnitude cheaper verification.

3. **Hardware acceleration for the specific workload.** GPU/TPU acceleration targets matrix multiplication. HDC's workload (parallel XOR-popcount) maps to FPGA, neuromorphic, and in-memory architectures. No other agent system has a hardware co-design path because no other system has a fixed, hardware-amenable core computation.

4. **On-chain reputation from proofs.** Other systems (Gitcoin Passport, Worldcoin, Polygon ID) use different primitives for AI computation and on-chain proof. Here, the HDC vector IS the computation substrate AND the proof witness AND the reputation primitive -- no translation layer between "what the AI computes" and "what the blockchain verifies."

### The Quantitative Case

At 10,000 agents:
- **CPU-only routing:** 10ms per query, 100 queries/second -- adequate for batch processing, insufficient for real-time.
- **FPGA (Alveo U280):** 1 microsecond per query, 1 million queries/second -- sufficient for real-time routing with margin.
- **Versal Premium:** 0.1 microsecond per query, 10 million queries/second -- sufficient for real-time routing at 100,000+ agents.

At 10,000 proofs per batch:
- **Jolt (CPU):** ~5 minutes per batch -- sufficient for hourly attestation cycles.
- **Binius (CPU, estimated):** 1-3 seconds per batch -- sufficient for per-block attestation (50ms blocks on Korai require batching across ~20-60 blocks).
- **Binius (FPGA-accelerated, estimated):** sub-second per batch -- sufficient for per-block attestation on a single block's timeline.

These numbers define the engineering tradeoffs: Jolt is usable today for periodic attestation; Binius will enable real-time attestation when its ZK property ships; FPGA acceleration will make real-time attestation at full scale practical.

---

## Paper Citations

| Reference | Citation |
|---|---|
| Binius: Succinct Arguments over Towers of Binary Fields | Diamond, Posen, et al. ePrint 2023/1784, EUROCRYPT 2024. Irreducible (formerly Ulvetanna) implementation released September 2025. |
| Lasso: Sumcheck-based SNARKs without Lookups | Setty, Thaler. ePrint 2023/1216, EUROCRYPT 2024. |
| Jolt: SNARKs for Virtual Machines via Lookups | Arun, Setty, Thaler. ePrint 2023/1217, EUROCRYPT 2024. |
| Worldcoin/TACEO Scalable Private Biometric Matching | Corrigan-Gibbs, Lubin, Rindal, et al. ePrint 2024/705. |
| Compact Hamming-Weight ZK Proofs | Bouaziz-Ermann, Canard, Cosseron, Eberhart, Music. |
| EP-HDC: Encrypted-Privacy Hyperdimensional Computing | (Privacy-preserving HDC over TFHE circuits) |
| HDC as Rescue for PPML | (HDC classifiers as efficient TFHE circuits for privacy-preserving ML) |
| IBM NorthPole | Modha et al. ISSCC 2024. 611 frames/J ResNet-50, 25x efficiency vs V100. |
| Intel Loihi 2 / Hala Point | Davies et al. 1.15B neurons, 15 TOPS/W INT8. |
| SpiNNaker 2 | Mayr et al. 152 ARM Cortex-M4F cores per chip, multicast spike packets. |
| In-memory HDC factorization (PCM) | Langenegger, Karunaratne, Hersche, Benini, Sebastian, Rahimi. Nature Nanotechnology, 2024. |
| HyDra SOT-CAM | Georgia Tech. ICCAD 2025. 300K queries/s/tile. |
| HDC adversarial attacks (grey-box) | Yang and Ren, 2020. 78%+ misclassification. |
| HDC adversarial attacks (hardware) | Li et al. ACM Transactions on Architecture and Code Optimization (TACO), 2025. |
| HDLock privileged encoding | Hernandez-Cano, Matsumoto, Karunaratne, Rahimi, et al. |
| Resonator networks for HDC factorization | Frady, Kleyko, Sommer. Neural Computation, 2018. |
| CaMeL IFC | Fang et al. 2024. Capability-tagged information flow control. |
| MacNet collaborative scaling law | Qian et al. arXiv 2406.07155, June 2024. |
| Darwin Godel Machine (DGM) | Zhang, Hu, Lu, Lange, Clune. arXiv 2505.22954, May 2025. |
| GEPA (DSPy 3.0 optimizer) | Agrawal et al. arXiv 2507.19457, July 2025. ICLR 2026 Oral. |
| Sorted bootstrapping for TFHE | ~3-8ms/gate after 2024-2025 improvements. |
| Binius roadmap (ZK property) | Irreducible, planned late 2025. Recursion not yet shipped. |

---

## Summary of Key Tradeoffs

| System | Maturity | Proof Size | Prove Time (10K HDC) | ZK Property | Post-Quantum |
|---|---|---|---|---|---|
| **Binius** | Beta (integrity only) | Small | 1-3 sec (est.) | Roadmap | Yes |
| **Jolt** | Production | Medium | ~5 min | Yes | No |
| **Worldcoin MPC** | Production (iris only) | N/A (interactive) | Real-time at 1M scale | Yes (MPC) | Partially |
| **PPH commitments** | Research | Constant (independent of vector length) | Sub-second | Yes | Depends on hash |
| **TFHE (encrypted HDC)** | Research/early production | N/A (FHE) | 3-8ms/gate | Yes (encryption) | Yes |

The recommended path: deploy Jolt-based proofs immediately for periodic attestation, integrate Binius for real-time integrity proofs when available, add the ZK property when Binius ships it, and pursue FPGA acceleration for hardware-pipelined prove-and-search in Phase 2.
