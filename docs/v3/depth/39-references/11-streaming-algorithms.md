# 39-11 Streaming Algorithms -- Annotated Reference Map

> Research foundations for probabilistic data structures, adaptive windowing, online
> estimation, and streaming computation in Roko's real-time monitoring.
>
> **v3 depth file** -- updated 2026-09-15.

---

## Adaptive Windowing

**[Bifet & Gavalda, 2007]** *Learning from Time-Changing Data with Adaptive Windowing.* SIAM 2007.
ADWIN: automatic distribution change detection with adaptive window size. Grounds drift detection in CascadeRouter model routing.

---

## Calibration and Uncertainty

**[Guo et al., 2017]** *On Calibration of Modern Neural Networks.* ICML 2017. arXiv:1706.04599.
Modern networks are poorly calibrated; temperature scaling helps. Grounds CalibrationTracker bias correction.

**[Lakshminarayanan, Pritzel & Blundell, 2017]** *Simple and Scalable Predictive Uncertainty Estimation using Deep Ensembles.* NeurIPS 2017. arXiv:1612.01474.
Deep ensembles provide well-calibrated uncertainty. Informs multi-model confidence aggregation.

**[Vovk, Gammerman & Shafer, 2005]** *Algorithmic Learning in a Random World.* Springer.
Conformal prediction: distribution-free intervals with guaranteed coverage. Grounds prediction confidence bounds.

**[Farquhar et al., 2024]** *Detecting hallucinations in large language models using semantic entropy.* Nature, 630.
Semantic entropy for hallucination detection. Informs confidence estimation in Gates.

**[Xiong et al., 2023]** *Can LLMs Express Their Uncertainty?* arXiv:2306.13063.
Empirical LLM confidence evaluation. Informs 7-axis Score confidence.

---

## Distributional RL

**[Dabney et al., 2018]** *Distributional Reinforcement Learning With Quantile Regression.* AAAI 2018.
Learning full return distributions. Provides richer learning signals.

**[Dabney et al., 2020]** *A Distributional Code for Value in Dopamine-Based RL.* Nature, 577.
Dopamine neurons encode distributional value. Neurobiological validation.

---

## Anytime Algorithms

**[Hansen & Zilberstein, 2001]** *Monitoring and Control of Anytime Algorithms.* Artificial Intelligence, 126(1--2).
Progressive improvement with interruption at any point. Grounds the T0/T1/T2 cascade.

---

## Probabilistic Data Structures

**[Flajolet et al., 2007]** *HyperLogLog: The Analysis of a Near-Optimal Cardinality Estimation Algorithm.* DMTCS.
Cardinality estimation with sub-linear memory. Applicable to tracking unique knowledge entries, unique agent interactions, and signal diversity metrics without maintaining full sets.

**[Cormode & Muthukrishnan, 2005]** *An Improved Data Stream Summary: The Count-Min Sketch.* Journal of Algorithms, 55(1), 58--75.
Frequency estimation in streams with bounded error. Applicable to tracking knowledge retrieval frequencies and hot-spot detection in the NeuroStore.

**[Bloom, 1970]** *Space/Time Trade-offs in Hash Coding with Allowable Errors.* Communications of the ACM, 13(7), 422--426.
Bloom filters for set membership testing. Used for efficient "have I seen this before?" checks in knowledge deduplication and Signal processing.

---

## Online Learning Theory

**[Cesa-Bianchi & Lugosi, 2006]** *Prediction, Learning, and Games.* Cambridge University Press.
Comprehensive treatment of online learning and regret minimization. Foundational reference for CascadeRouter's online model selection.

**[Shalev-Shwartz, 2012]** *Online Learning and Online Convex Optimization.* Foundations and Trends in ML, 4(2), 107--194.
Online convex optimization framework. Informs the gradient-free optimization approaches used in prompt experiment parameter tuning.

---

## Change Detection

**[Page, 1954]** *Continuous Inspection Schemes.* Biometrika, 41(1--2), 100--115.
CUSUM algorithm for sequential change detection. Grounds the CalibrationTracker's drift detection for model performance monitoring.

**[Basseville & Nikiforov, 1993]** *Detection of Abrupt Changes: Theory and Application.* Prentice-Hall.
Comprehensive treatment of change-point detection. Informs the gate threshold adaptation mechanism that detects shifts in task difficulty or model capability.

---

## Exponential Moving Averages

**[Hunter, 1986]** *The Exponentially Weighted Moving Average.* Journal of Quality Technology, 18(4), 203--210.
EWMA for control charts and trend detection. Directly implemented in gate threshold adaptation (EMA per rung) and the CascadeRouter's model performance tracking.

---

## Cross-References

- Signal processing: [12-signal-processing](./12-signal-processing.md)
- Free energy uncertainty: [16-active-inference](./16-active-inference.md)
- T0 probe architecture: depth/04-execution/
