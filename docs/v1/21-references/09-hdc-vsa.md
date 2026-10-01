# Hyperdimensional Computing and Vector Symbolic Architectures

> Annotations are kept only for works whose descriptions were checked against the papers (listed in
> `tools/docs_integrity/content_audit.json` and in the banner of docs/v3/REFERENCES.md). The other entries are
> bare citations: their annotations were removed because they had not been checked against the papers.

> **DEPRECATED (v1):** This document is part of the v1 specification and may be outdated. See [../../v2/](../../v2/) for the current reference.


> Academic foundations for HDC/VSA: the 10,240-bit Binary Spatter Code algebra, learned hashing, similarity search, and HDC-based knowledge representation in Roko.

**Topic**: [References](./INDEX.md)
**Prerequisites**: [Architecture](../00-architecture/INDEX.md), [Engram Data Type](../00-architecture/02-engram-data-type.md)
**Key sources**: `bardo-backup/prd/shared/hdc-vsa.md`, `bardo-backup/tmp/agent-chain/08-references.md`, `bardo-backup/tmp/agent-chain/14-academic-foundations.md` §2

> **Implementation**: Reference

---

## Abstract

Roko uses 10,240-bit Binary Spatter Codes (BSC) as the universal representation substrate for knowledge similarity, cross-domain transfer, and structural analogy detection. HDC provides three algebraic operations — XOR binding, majority-vote bundling, and cyclic-shift permutation — that compose knowledge representations in nanoseconds on commodity hardware. The mathematical foundation rests on the Johnson-Lindenstrauss lemma (preserving distances under projection) and Kanerva's insight that in very high dimensions, random vectors are nearly orthogonal with high probability.

---

## Foundational HDC Theory

- Kanerva, P. (1988). _Sparse Distributed Memory_. MIT Press.

- Kanerva, P. (2009). Hyperdimensional Computing: An Introduction to Computing in Distributed Representation with High-Dimensional Random Vectors. _Cognitive Computation_, 1(2), 139-159.

---

## VSA Surveys

- Kleyko, D., Rachkovskij, D.A., Osipov, E., & Rahimi, A. (2022). A Survey on Hyperdimensional Computing aka Vector Symbolic Architectures. _ACM Computing Surveys_, 55(6), Article 130.

- Kleyko et al. (2022). Vector Symbolic Architectures as a Computing Framework for Emerging Hardware. _Proceedings of the IEEE_. arXiv:2106.05268.

---

## Resonator Networks and Advanced Retrieval

- Frady, E.P., Kleyko, D., & Sommer, F.T. (2021). Computing on Functions Using Randomized Vector Representations. arXiv:2109.03429.

---

## Random Projection Theory

- Johnson, W.B. & Lindenstrauss, J. (1984). Extensions of Lipschitz Mappings into a Hilbert Space. _Contemporary Mathematics_, 26, 189-206.

---

## Locality-Sensitive Hashing

- Charikar, M.S. (2002). Similarity Estimation Techniques from Rounding Algorithms. _STOC_, 2002.

- Indyk, P. & Motwani, R. (1998). Approximate Nearest Neighbors: Towards Removing the Curse of Dimensionality. _STOC_, 1998.

---

## Learned Hashing

- Kulis, B. & Darrell, T. (2009). Learning to Hash with Binary Reconstructive Embeddings. _NeurIPS_, 2009.

- Cao, Z., Long, M., Wang, J., & Yu, P.S. (2017). HashNet: Deep Learning to Hash by Continuation. _ICCV_, 2017.

- Yuan, L. et al. (2020). Central Similarity Quantization for Efficient Image and Video Retrieval. _CVPR_, 2020.

---

## Differentiable HDC Operations

- Ganesan, A. et al. (2021). Learning with Holographic Reduced Representations. _NeurIPS_, 2021 (Spotlight).

- Alam, M. et al. (2023). Recasting Self-Attention with Holographic Reduced Representations (HRRFormer). _ICML_, 2023.

---

## FPGA and Hardware Acceleration


---

## Online and Streaming Hashing

- Çakir, F., He, K., Bargal, S.A., & Sclaroff, S. (2017). ICCV 2017

---

## Approximate Nearest Neighbor Search

- Malkov, Y.A. & Yashunin, D.A. (2020). Efficient and Robust Approximate Nearest Neighbor using Hierarchical Navigable Small World Graphs. _IEEE TPAMI_, 2020.

- Xu et al. (2023). SPFresh: Incremental In-Place Update for Billion-Scale Vector Search. _SIGMOD_, 2023.

---

## Holographic Reduced Representations

- Plate, T.A. (1994). Distributed Representations and Nested Compositional Structure. PhD Dissertation, University of Toronto.

---

## HDC Frameworks and Applications (2024-2025)

- Heddes et al. (2024). Hyperdimensional Computing: A Framework for Stochastic Computation and Symbolic AI. _Journal of Big Data_, 2024.

- Hernández-Cano et al. (2024). Hyperdimensional Computing with Holographic and Adaptive Encoder. _Frontiers in AI_, 2024.

- Arbore et al. (2024). HPVM-HDC: A Heterogeneous Programming System for Accelerating Hyperdimensional Computing. arXiv:2410.15179.

- Hyperdimensional Computing in Biomedical Sciences (2025). _PMC_, 2025.

---

## Cross-References

- See [01-memory-consolidation.md](./01-memory-consolidation.md) for HDC-encoded knowledge retrieval
- See [03-dreams-and-offline-learning.md](./03-dreams-and-offline-learning.md) for HDC counterfactual synthesis
- See topic [00-architecture](../00-architecture/INDEX.md) for HDC integration in the Synapse Architecture
