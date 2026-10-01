# 39-16 Active Inference -- Annotated Reference Map

> Research foundations for the Free Energy Principle, expected free energy,
> predictive coding, and Bayesian cognition in Roko's tier routing and context
> selection systems.
>
> **v3 depth file** -- updated 2026-09-15.

---

## Free Energy Principle

**[Friston, 2006]** *A Free Energy Principle for the Brain.* Journal of Physiology-Paris, 100(1--3), 70--87.
FEP foundation: self-organizing systems minimize variational free energy. Foundational for prediction-error-driven cognition throughout Roko. Perception, action, and learning are all aspects of free energy minimization.

**[Friston, 2010]** *The Free-Energy Principle: A Unified Brain Theory?* Nature Reviews Neuroscience, 11(2), 127--138.
FEP as unifying principle. Free energy minimization unifies perception, action, and learning under one objective. Roko's cognitive loop implements this: sense (query), predict (context assembly), act (dispatch), verify (gate), learn (knowledge update).

---

## Expected Free Energy

**[Friston et al., 2015]** *Active Inference and Epistemic Value.* Cognitive Neuroscience, 6(4), 187--214.
EFE = pragmatic_value + epistemic_value. Resolves exploration-exploitation without hyperparameters. Grounds EFE-based tier routing in `roko-primitives`: tasks with high uncertainty get routed to exploratory (T2) models; familiar tasks go to T0.

**[Millidge, Tschantz & Buckley, 2021]** *Whence the Expected Free Energy?* Neural Computation, 33(2), 447--482.
Critical analysis: naive future free energy discourages exploration. Essential corrective for EFE implementation -- without the epistemic term, agents exploit prematurely. Applied in the CascadeRouter's exploration bonus.

---

## Comprehensive Reference

**[Parr, Pezzulo & Friston, 2022]** *Active Inference: The Free Energy Principle in Mind, Brain, and Behavior.* MIT Press.
First comprehensive active inference textbook. Complete mathematical framework. Primary reference for the tier routing implementation.

---

## Bayesian Surprise and Attention

**[Itti & Baldi, 2005]** *Bayesian Surprise Attracts Human Attention.* NeurIPS 2005.
Bayesian surprise as KL divergence between prior and posterior. Formally identical to the epistemic value component of EFE. Grounds surprise-driven attention allocation in context assembly.

---

## Active Inference for LLM Agents

**[Prakki, 2024]** *Active Inference for Self-Organizing Multi-LLM Systems: A Bayesian Thermodynamic Approach to Adaptation.* arXiv:2412.10425.
Active inference as cognitive layer above LLM agents, dynamically adjusting prompts through information-seeking behavior. Validates EFE-driven context assembly as practical, not just theoretical.

**[Shafiei et al., 2025]** *Distributionally Robust Free Energy Principle for Decision-Making.* Nature Communications, 17, 707.
DR-FREE model: robust active inference under model uncertainty. Agents complete tasks when SOTA fails under distributional shift. Target for hardening Roko's tier routing under uncertainty.

---

## Bayesian Experimental Design

**[Choudhury et al., 2025]** *BED-LLM: Intelligent Information Gathering with LLMs and Bayesian Experimental Design.* Oxford. arXiv.
Sequential information gathering framed as Bayesian experimental design. Informs the research agent's query strategy and provides principled exploration sequencing.

---

## Reference Implementation

**[Heins et al., 2022]** *pymdp: A Python Library for Active Inference on Discrete State Spaces.* JOSS.
JAX-accelerated active inference reference implementation. The API design informs the tier routing implementation, though Roko uses Rust with HDC rather than POMDP state spaces.

---

## Predictive Coding

**[Rao & Ballard, 1999]** *Predictive Coding in the Visual Cortex.* Nature Neuroscience, 2(1), 79--87.
Hierarchical predictive coding: only prediction errors propagate upward. Grounds tier routing -- T0 handles predicted signals, T1/T2 handle prediction errors requiring deeper processing.

---

## Neuromodulation and Learning

**[Doya, 2002]** *Metalearning and Neuromodulation.* Neural Networks, 15(4--6), 495--506.
Different neuromodulators control different learning aspects: dopamine (reward), serotonin (time horizon), noradrenaline (exploration), acetylcholine (learning rate). Maps to the 7-axis Score on Signals.

**[Rescorla & Wagner, 1972]** *A Theory of Pavlovian Conditioning.* In Classical Conditioning II.
Prediction error learning: learning occurs proportionally to surprise. The simplest form driving T0 probes and the CalibrationTracker.

---

## Scalable Implementations

**[van de Laar et al., 2024]** *Synthetic Active Inference Agents, Part II: Variational Message Updates.* arXiv:2306.02733.
Message passing on Forney-style Factor Graphs for generalized free energy minimization. Scalable path for Roko's EFE tier routing beyond discrete state spaces.

**[Raffa & Acciai, 2024]** *Free Energy Principle and Active Inference in Neural Language Models.* CEUR-WS Vol-3923.
Direct application of FEP to neural language model behavior. Shows language generation can be understood as free energy minimization. Validates the conceptual bridge.

---

## Cross-References

- Predictive processing: [12-signal-processing](./12-signal-processing.md)
- Cybernetic regulation: [15-cybernetics-and-vsm](./15-cybernetics-and-vsm.md)
- Cognitive architectures: [20-cognitive-architectures](./20-cognitive-architectures.md)
