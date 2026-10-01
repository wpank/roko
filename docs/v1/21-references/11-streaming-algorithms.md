# Streaming Algorithms and Online Statistics

> Annotations are kept only for works whose descriptions were checked against the papers (listed in
> `tools/docs_integrity/content_audit.json` and in the banner of docs/v3/REFERENCES.md). The other entries are
> bare citations: their annotations were removed because they had not been checked against the papers.

> **DEPRECATED (v1):** This document is part of the v1 specification and may be outdated. See [../../v2/](../../v2/) for the current reference.


> Academic foundations for probabilistic data structures, adaptive windowing, online estimation, and streaming computation used in Roko's real-time monitoring.

**Topic**: [References](./INDEX.md)
**Prerequisites**: [Architecture](../00-architecture/INDEX.md)
**Key sources**: `bardo-backup/prd/shared/citations.md` §§10-11

> **Implementation**: Reference

---

## Abstract

Roko agents process continuous data streams at Gamma frequency (~5-15s ticks). Streaming algorithms provide constant-memory, single-pass computation: approximate counting (HyperLogLog), frequency estimation (Count-Min Sketch), set membership (Bloom filters), and change detection (ADWIN). These are the computational primitives behind T0 probes and real-time monitoring.

---

## Adaptive Windowing

- Bifet, A. & Gavaldà, R. (2007). Learning from Time-Changing Data with Adaptive Windowing. _SIAM_, 2007.

---

## Calibration and Uncertainty Estimation

- Guo, C. et al. (2017). On Calibration of Modern Neural Networks. _ICML_, 2017. arXiv:1706.04599.

- Lakshminarayanan, B., Pritzel, A., & Blundell, C. (2017). Simple and Scalable Predictive Uncertainty Estimation using Deep Ensembles. _NeurIPS_, 2017. arXiv:1612.01474.

- Vovk, V., Gammerman, A., & Shafer, G. (2005). _Algorithmic Learning in a Random World_. Springer.

- Farquhar, S. et al. (2024). Detecting Hallucinations in Large Language Models Using Semantic Entropy. _Nature_, 630.

- Xiong, M. et al. (2023). Can LLMs Express Their Uncertainty? An Empirical Evaluation of Confidence Elicitation in LLMs. arXiv:2306.13063.

---

## Distributional Reinforcement Learning

- Dabney, W. et al. (2018). Distributional Reinforcement Learning with Quantile Regression. _AAAI_, 2018.

- Dabney, W. et al. (2020). A Distributional Code for Value in Dopamine-Based Reinforcement Learning. _Nature_, 577.

---

## Anytime Algorithms

- Hansen, E.A. & Zilberstein, S. (2001). Monitoring and Control of Anytime Algorithms. _Artificial Intelligence_, 126(1-2).

---

## Cross-References

- See [12-signal-processing.md](./12-signal-processing.md) for Kalman filtering and spectral methods
- See [16-active-inference.md](./16-active-inference.md) for free energy-based uncertainty
- See topic [00-architecture](../00-architecture/INDEX.md) for T0 probe architecture
