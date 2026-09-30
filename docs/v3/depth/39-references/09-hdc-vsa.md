# 39-09 HDC and Vector Symbolic Architectures -- Annotated Reference Map

> Research foundations for the 10,240-bit Binary Spatter Code algebra, learned
> hashing, similarity search, and HDC-based knowledge representation.
>
> **v3 depth file** -- updated 2026-09-15. Added PathHD.

---

## Foundational HDC Theory

**[Kanerva, 1988]** *Sparse Distributed Memory.* MIT Press.
Content-addressable memory via orthogonal random vectors. Foundational for all of Roko's HDC operations.

**[Kanerva, 2009]** *Hyperdimensional Computing.* Cognitive Computation, 1(2), 139--159.
Binding, bundling, permutation. 10,000-dimensional binary vectors provide sufficient capacity. Primary reference for the 10,240-bit BSC dimensionality.

---

## VSA Surveys

**[Kleyko et al., 2022]** *A Survey on Hyperdimensional Computing aka Vector Symbolic Architectures.* ACM Computing Surveys, 55(6), Article 130.
Covers all VSA families. Validates BSC selection and capacity bounds.

**[Kleyko et al., 2022]** *Vector Symbolic Architectures as a Computing Framework.* Proceedings of the IEEE. arXiv:2106.05268.
BSC hardware: FPGA/ASIC implementations achieving sub-microsecond operations.

---

## Resonator Networks

**[Frady, Kleyko & Sommer, 2021]** *Computing on Functions Using Randomized Vector Representations.* arXiv:2109.03429.
Iterative convergence for large-scale HDC retrieval. Future optimization path for very large dictionaries.

---

## Random Projection Theory

**[Johnson & Lindenstrauss, 1984]** *Extensions of Lipschitz Mappings into a Hilbert Space.* Contemporary Mathematics, 26, 189--206.
JL lemma: distances preserved under projection. For eps=0.1, N=100,000: D >= 4,604. Roko's 10,240 provides generous headroom.

---

## Locality-Sensitive Hashing

**[Charikar, 2002]** *Similarity Estimation Techniques from Rounding Algorithms.* STOC 2002.
SimHash: binary codes where collision probability = 1 - theta/pi. Phase 1 encoding in the HDC pipeline.

**[Indyk & Motwani, 1998]** *Approximate Nearest Neighbors.* STOC 1998.
Random hash functions preserve similarity in sub-linear query time. Foundation for efficient HDC search.

---

## Learned Hashing

**[Kulis & Darrell, 2009]** *Learning to Hash with Binary Reconstructive Embeddings.* NeurIPS 2009.
Data-dependent hashing outperforms random projections. Phase 2 encoding path.

**[Cao et al., 2017]** *HashNet: Deep Learning to Hash by Continuation.* ICCV 2017.
Differentiable hashing; +14.6% MAP on ImageNet. Phase 2 encoding.

**[Yuan et al., 2020]** *Central Similarity Quantization.* CVPR 2020.
Hash centers via Hadamard matrices. Maps to knowledge domain prototypes. Phase 4 encoding.

---

## Differentiable HDC

**[Ganesan et al., 2021]** *Learning with Holographic Reduced Representations.* NeurIPS 2021 (Spotlight).
Made HRR viable as differentiable deep learning components; +100x retrieval improvement.

**[Alam et al., 2023]** *HRRFormer.* ICML 2023.
HDC attention scaling to sequence length 131,072.

---

## Hardware Acceleration

**[Imani et al., 2019]** *FloatHD.* IEEE/ACM ICCAD 2019.
FPGA: ~3--5ns per comparison at 200 MHz.

---

## Streaming Hashing

**[Cakir et al., 2017]** *MIHash: Online Hashing with Mutual Information.* NeurIPS 2017.
Synchronous binary code updates for continuous data arrival.

---

## Approximate Nearest Neighbor

**[Malkov & Yashunin, 2020]** *Efficient and Robust Approximate Nearest Neighbor Search Using Hierarchical Navigable Small World Graphs.* IEEE TPAMI 2020.
O(log N) search at 95--99% recall for billion-scale binary vectors. Production search infrastructure.

**[Zhang et al., 2023]** *SPFresh: Incremental In-Place Update.* SIGMOD 2023.
LIRE rebalancing for continuous insert/delete.

---

## Holographic Reduced Representations

**[Plate, 1994]** *Distributed Representations and Nested Compositional Structure.* PhD Dissertation, Toronto.
HRR theory using circular convolution. Theoretical ancestor of BSC.

---

## HDC Frameworks (2024--2025)

**[Heddes et al., 2024]** *HDC: A Framework for Stochastic Computation and Symbolic AI.* Journal of Big Data.
Unified HDC as general-purpose computation substrate. Validates BSC for knowledge representation.

**[Hernández-Cano et al., 2024]** *Hyperdimensional computing with holographic and adaptive encoder.* Frontiers in AI.
Gradient-descent encoder learning. Bridges fixed and learned encoding phases.

**[Arbore et al., 2024]** *HPVM-HDC: A Heterogeneous Programming System for Accelerating Hyperdimensional Computing.* arXiv:2410.15179.
Unified CPU/GPU/FPGA execution model.

**[Anonymous, 2025]** *Hyperdimensional Computing in Biomedical Sciences.* PMC review.
Production deployments validating practical viability.

**[Anonymous, 2025]** *Optimal Hyperdimensional Representation.* OpenReview.
Theoretical optimal representations. Informs the 10,240-bit dimensionality choice.

**[Anonymous, 2025]** *The Hyperdimensional Transform for Distributional Modeling.* Neural Computing and Applications.
HDC for distributional modeling, regression, and classification.

---

## 2026 Addition: PathHD

**[Liu et al., 2026]** *Encoder-Free Knowledge-Graph Reasoning with LLMs via Hyperdimensional Path Retrieval.* arXiv (2026).
Path-based encoding extends standard HDC with sequential structure preservation. Applicable to encoding temporal patterns in agent episode sequences where order matters for knowledge retrieval.

---

## Cross-References

- HDC in knowledge retrieval: [01-memory-consolidation](./01-memory-consolidation.md)
- HDC counterfactual synthesis: [03-dreams-and-offline-learning](./03-dreams-and-offline-learning.md)
- HDC in architecture: depth/01-signal/
