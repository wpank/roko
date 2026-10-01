# 39-12 Signal Processing -- Annotated Reference Map

> Research foundations for spectral decomposition, predictive filtering, TDA, and
> information theory in Roko's cognitive clock and monitoring systems.
>
> **v3 depth file** -- updated 2026-09-15.

---

## Predictive Processing

**[Clark, 2013]** *Whatever Next? Predictive Brains, Situated Agents, and the Future of Cognitive Science.* Behavioral and Brain Sciences, 36(3), 181--204.
Brains as prediction machines minimizing prediction error. Foundational for prediction-error-driven T0/T1/T2 tier routing.

---

## Topological Data Analysis

**[Carlsson, 2009]** *Topology and Data.* Bulletin of the AMS, 46(2), 255--308.
TDA foundations including persistent homology. Informs anomaly detection.

**[Gidea & Katz, 2018]** *Topological Data Analysis of Financial Time Series.* Physica A, 491, 820--834.
Persistent homology detects structural changes preceding crashes.

**[Bauer, 2021]** *Ripser: Efficient Computation of Vietoris-Rips Persistence Barcodes.* J. Applied and Computational Topology, 5, 391--423.
Efficient persistence barcodes. Enables practical TDA.

**[Perea & Harer, 2015]** *Sliding Windows and Persistence.* Foundations of Computational Mathematics, 15(3), 799--838.
Topological methods for time series via sliding window embeddings.

---

## Information Theory

**[Shannon, 1948]** *A Mathematical Theory of Communication.* Bell System Technical Journal, 27, 379--423 & 623--656.
Foundational information theory. Used throughout for measuring knowledge value.

**[Cover & Thomas, 2006]** *Elements of Information Theory.* 2nd ed. Wiley.
Comprehensive reference including rate-distortion theory.

**[Simon, 1971]** *Designing Organizations for an Information-Rich World.* Johns Hopkins Press.
"A wealth of information creates a poverty of attention." Foundational for the VCG auction.

---

## Thermodynamics of Computation

**[Still et al., 2012]** *Thermodynamics of Prediction.* Physical Review Letters, 109(12), 120604.
Formal connection between prediction efficiency and thermodynamic cost.

**[Landauer, 1961]** *Irreversibility and Heat Generation in the Computing Process.* IBM Journal R&D, 5(3), 183--191.
Erasing information has minimum thermodynamic cost. Forgetting is not computationally free.

**[Bennett, 1982]** *The Thermodynamics of Computation -- A Review.* International Journal of Theoretical Physics, 21(12), 905--940.
Selective forgetting is computationally preferable to accumulation.

---

## TDA Advances (2024--2025)

**[Su et al., 2025]** *Topological data analysis and topological deep learning beyond persistent homology: a review.* Artificial Intelligence Review, Springer. doi:10.1007/s10462-025-11462-w.
Persistent topological Laplacians capture shape evolution. Advances anomaly detection.

**[Anonymous, 2025]** *Persistent Homology-Based Unsupervised Anomaly Detection in Time Series.* OpenReview.
Delay embeddings + distance-to-measure Rips filtration for anomaly detection.

**[Liu et al., 2024]** *Multivariate Time-Series Anomaly Detection based on Enhancing Graph Attention Networks with Topological Analysis.* arXiv:2408.13082.
Enhanced GAT with persistent homology for inter-feature dependencies.

**[Yao et al., 2025]** *Change Point Detection in Financial Market Using Topological Data Analysis.* Systems, 13(10), 875.
Takens embedding + sliding window for topological change detection.

**[Ichinomiya, 2025]** *Machine Learning of Time Series Using Persistent Homology.* Scientific Reports, Nature.
ML directly on persistent homology representations.

---

## Spectral Analysis

**[Buzsaki, 2006]** *Rhythms of the Brain.* Oxford University Press.
Neural oscillations at multiple frequencies (delta, theta, alpha, gamma) serve distinct cognitive functions. Foundational for Roko's cognitive frequency architecture: Gamma (reactive, ~80% of ticks), Theta (reflective, ~15%), Delta (consolidation, ~5%).

**[Friston et al., 2015]** *Knowing One's Place: A Free-Energy Approach to Pattern Recognition.* PLoS Computational Biology.
Prediction errors at different frequency bands encode different levels of abstraction. Maps to tier routing where T0 handles low-level prediction matches and T2 handles high-level conceptual mismatches.

---

## Rate-Distortion Theory

**[Berger, 1971]** *Rate Distortion Theory: A Mathematical Basis for Data Compression.* Prentice-Hall.
Minimum bits to represent a source within a given distortion level. Provides the theoretical basis for knowledge compression during tier promotion -- how much information can be lost while preserving utility.

**[Tishby, Pereira & Bialek, 1999]** *The Information Bottleneck Method.* Proceedings of the 37th Allerton Conference. arXiv:physics/0004057.
Optimal compression preserving relevant information. The information bottleneck principle grounds tier promotion: compress Episode to Insight to Heuristic while preserving task-relevant information.

---

## Cross-References

- Streaming algorithms: [11-streaming-algorithms](./11-streaming-algorithms.md)
- Active inference: [16-active-inference](./16-active-inference.md)
- Cognitive frequencies: depth/04-execution/
