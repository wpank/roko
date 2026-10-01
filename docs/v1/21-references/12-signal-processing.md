# Signal Processing and Time Series

> Annotations are kept only for works whose descriptions were checked against the papers (listed in
> `tools/docs_integrity/content_audit.json` and in the banner of docs/v3/REFERENCES.md). The other entries are
> bare citations: their annotations were removed because they had not been checked against the papers.

> **DEPRECATED (v1):** This document is part of the v1 specification and may be outdated. See [../../v2/](../../v2/) for the current reference.


> Academic foundations for spectral decomposition, predictive filtering, and temporal pattern detection in Roko's cognitive clock and monitoring systems.

**Topic**: [References](./INDEX.md)
**Prerequisites**: [Architecture](../00-architecture/INDEX.md)
**Key sources**: `bardo-backup/prd/shared/citations.md` §§30-31

> **Implementation**: Reference

---

## Abstract

Roko agents operate at three cognitive frequencies — Gamma (~5-15s), Theta (~75s), Delta (~hours). The signal processing literature provides the mathematical foundations for spectral decomposition of agent performance data, predictive filtering for state estimation, and multi-scale temporal analysis. These methods underpin the adaptive clock, drift detection, and performance monitoring.

---

## Predictive Processing

- Clark, A. (2013). Whatever Next? Predictive Brains, Situated Agents, and the Future of Cognitive Science. _Behavioral and Brain Sciences_, 36(3), 181-204.

---

## Topological Data Analysis

- Carlsson, G. (2009). Topology and Data. _Bulletin of the American Mathematical Society_, 46(2), 255-308.

- Gidea, M. & Katz, Y. (2018). Topological Data Analysis of Financial Time Series: Landscapes of Crashes. _Physica A_, 491, 820-834.

- Bauer, U. (2021). Ripser: Efficient Computation of Vietoris-Rips Persistence Barcodes. _Journal of Applied and Computational Topology_, 5, 391-423.

- Perea, J.A. & Harer, J. (2015). Sliding Windows and Persistence: An Application of Topological Methods to Signal Analysis. _Foundations of Computational Mathematics_, 15(3), 799-838.

---

## Information Theory

- Shannon, C.E. (1948). A Mathematical Theory of Communication. _Bell System Technical Journal_, 27, 379-423 & 623-656.

- Cover, T.M. & Thomas, J.A. (2006). _Elements of Information Theory_, 2nd ed. Wiley.

- Simon, H.A. (1971). Designing Organizations for an Information-Rich World. In _Computers, Communications, and the Public Interest_. Johns Hopkins Press.

- Still, S. et al. (2012). Thermodynamics of Prediction. _Physical Review Letters_, 109(12), 120604.

---

## Computational Irreversibility

- Landauer, R. (1961). Irreversibility and Heat Generation in the Computing Process. _IBM Journal of Research and Development_, 5(3), 183-191.

- Bennett, C.H. (1982). The Thermodynamics of Computation — A Review. _International Journal of Theoretical Physics_, 21(12), 905-940.

---

## TDA Advances (2024-2025)

- Topological Data Analysis and Topological Deep Learning Beyond Persistent Homology (2025). _Artificial Intelligence Review_, Springer, 2025.

- Persistent Homology-Based Algorithm for Unsupervised Anomaly Detection in Time Series (2025). OpenReview.

- Multivariate Time-Series Anomaly Detection based on Enhancing Graph Attention Networks with Topological Analysis (2024). arXiv:2408.13082.

- Change Point Detection in Financial Market Using Topological Data Analysis (2025). _Systems_, 13(10), 875.

---

## Cross-References

- See [11-streaming-algorithms.md](./11-streaming-algorithms.md) for ADWIN and online statistics
- See [16-active-inference.md](./16-active-inference.md) for free energy and prediction error
- See topic [00-architecture](../00-architecture/INDEX.md) for the three cognitive frequencies
