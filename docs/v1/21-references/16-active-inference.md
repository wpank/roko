# Active Inference and Free Energy Principle

> Annotations are kept only for works whose descriptions were checked against the papers (listed in
> `tools/docs_integrity/content_audit.json` and in the banner of docs/v3/REFERENCES.md). The other entries are
> bare citations: their annotations were removed because they had not been checked against the papers.

> **DEPRECATED (v1):** This document is part of the v1 specification and may be outdated. See [../../v2/](../../v2/) for the current reference.


> Academic foundations for active inference, expected free energy, Bayesian surprise, and predictive processing in Roko's context selection and action selection.

**Topic**: [References](./INDEX.md)
**Prerequisites**: [Architecture](../00-architecture/INDEX.md)
**Key sources**: `bardo-backup/tmp/agent-chain/14-academic-foundations.md` §4, `bardo-backup/prd/shared/citations.md` §4

> **Implementation**: Reference

---

## Abstract

Active inference provides the principled answer to "how should an agent decide what to attend to?" Expected Free Energy (EFE) decomposes into pragmatic value (goal achievement) and epistemic value (information gain), resolving the exploration-exploitation dilemma without ad-hoc hyperparameters. Roko uses EFE for both context selection (which knowledge to retrieve) and action selection (which cognitive tier to invoke). This is not a metaphor — the mathematics directly determine retrieval ranking and tier routing.

---

## Free Energy Principle

- Friston, K. (2006). A Free Energy Principle for the Brain. _Journal of Physiology–Paris_, 100(1-3), 70-87.

- Friston, K. (2010). The Free-Energy Principle: A Unified Brain Theory? _Nature Reviews Neuroscience_, 11(2), 127-138.

---

## Expected Free Energy and Exploration

- Friston, K., Rigoli, F., Ognibene, D., Mathys, C., Fitzgerald, T., & Pezzulo, G. (2015). Active Inference and Epistemic Value. _Cognitive Neuroscience_, 6(4), 187-214.

- Millidge, B., Tschantz, A., & Buckley, C.L. (2021). Whence the Expected Free Energy? _Neural Computation_, 33(2), 447-482.

---

## Active Inference Textbook

- Parr, T., Pezzulo, G., & Friston, K. (2022). _Active Inference: The Free Energy Principle in Mind, Brain, and Behavior_. MIT Press.

---

## Bayesian Surprise

- Itti, L. & Baldi, P. (2005). Bayesian Surprise Attracts Human Attention. _NeurIPS_, 2005.

---

## Active Inference for LLMs

- Prakki (2024). Active Inference for Self-Organizing Multi-LLM Systems. arXiv, 2024.

- Choudhury et al. (2025). BED-LLM: Intelligent Information Gathering with LLMs and Bayesian Experimental Design. Oxford. arXiv, 2025.

---

## Implementation Libraries

- Heins, C., Millidge, B., Demekas, D. et al. (2022). pymdp: A Python Library for Active Inference on Discrete State Spaces. _Journal of Open Source Software_.

---

## Predictive Processing

- Rao, R.P. & Ballard, D.H. (1999). Predictive Coding in the Visual Cortex: A Functional Interpretation of Some Extra-Classical Receptive-Field Effects. _Nature Neuroscience_, 2(1), 79-87.

- Clark, A. (2013). Whatever Next? Predictive Brains, Situated Agents, and the Future of Cognitive Science. _Behavioral and Brain Sciences_, 36(3), 181-204.

---

## Dopaminergic Prediction Errors

- Doya, K. (2002). Metalearning and Neuromodulation. _Neural Networks_, 15(4-6), 495-506.

- Rescorla, R.A. & Wagner, A.R. (1972). A Theory of Pavlovian Conditioning. In _Classical Conditioning II_. Appleton-Century-Crofts.

---

## Distributionally Robust Active Inference (2025)

- Shafiei, A., Jesawada, H., Friston, K., & Russo, G. (2025). Distributionally Robust Free Energy Principle for Decision-Making. _Nature Communications_, 17, 707.

---

## Active Inference for LLM Systems (2024-2025)

- Prakki (2024). Active Inference for Self-Organizing Multi-LLM Systems: A Bayesian Thermodynamic Approach to Adaptation. arXiv:2412.10425.

- Synthetic Active Inference Agents (2024). Realising Synthetic Active Inference Agents, Part II: Variational Message Updates. arXiv:2306.02733.

---

## Cross-References

- See [12-signal-processing.md](./12-signal-processing.md) for predictive processing
- See [15-cybernetics-and-vsm.md](./15-cybernetics-and-vsm.md) for cybernetic regulation
- See [20-cognitive-architectures.md](./20-cognitive-architectures.md) for CoALA and dual-process cognition
