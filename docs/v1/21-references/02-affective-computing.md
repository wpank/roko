# Affective Computing

> Annotations are kept only for works whose descriptions were checked against the papers (listed in
> `tools/docs_integrity/content_audit.json` and in the banner of docs/v3/REFERENCES.md). The other entries are
> bare citations: their annotations were removed because they had not been checked against the papers.

> **DEPRECATED (v1):** This document is part of the v1 specification and may be outdated. See [../../v2/](../../v2/) for the current reference.


> Academic foundations for emotion modeling, somatic markers, and affect-modulated cognition in the Roko Daimon subsystem.

**Topic**: [References](./INDEX.md)
**Prerequisites**: [Architecture](../00-architecture/INDEX.md), [Daimon](../09-daimon/INDEX.md)
**Key sources**: `bardo-backup/prd/02-mortality/14-research-foundations.md` §3, `bardo-backup/prd/shared/citations.md` §28

> **Implementation**: Reference

---

## Abstract

The Daimon is not cosmetic. Five independent research lines — somatic markers, mood-congruent retrieval, exploration modulation, narrative transfer, and empirical trading results — converge on the conclusion that emotion-like states serve genuine computational functions that cognition alone cannot replicate. The PAD (Pleasure-Arousal-Dominance) vector, the ALMA three-layer temporal model, the Somatic Landscape, and affect-modulated retrieval are all grounded in the citations below.

---

## PAD Emotional State Model

- Mehrabian, A. (1996). Pleasure-Arousal-Dominance: A General Framework for Describing and Measuring Individual Differences in Temperament. _Current Psychology_, 14(4), 261-292.

- Russell, J.A. & Mehrabian, A. (1977). Evidence for a Three-Factor Theory of Emotions. _Journal of Research in Personality_, 11(3), 273-294.

---

## Somatic Marker Hypothesis

- Damasio, A.R. (1994). _Descartes' Error: Emotion, Reason, and the Human Brain_. Putnam.

- Bechara, A., Damasio, H., & Damasio, A.R. (2000). Emotion, Decision Making and the Orbitofrontal Cortex. _Cerebral Cortex_, 10(3), 295-307.

- Bechara, A. & Damasio, A.R. (2005). The Somatic Marker Hypothesis: A Neural Theory of Economic Decision. _Games and Economic Behavior_, 52, 336-372.


---

## Mood-Congruent Memory and Emotional Retrieval

- Bower, G.H. (1981). Mood and Memory. _American Psychologist_, 36(2), 129-148.

- Blaney, P.H. (1986). Affect and Memory: A Review. _Psychological Bulletin_, 99(2), 229-246.

- Phelps, E.A. (2004). Human Emotion and Memory: Interactions of the Amygdala and Hippocampal Complex. _Current Opinion in Neurobiology_, 14(2), 198-202.

- Cahill, L. & McGaugh, J.L. (1998). Mechanisms of Emotional Arousal and Lasting Declarative Memory. _Trends in Neurosciences_, 21(7), 294-299.

---

## Emotion Classification and Appraisal

- Plutchik, R. (1980). _Emotion: A Psychoevolutionary Synthesis_. Harper & Row.

- Scherer, K.R. (2001). Appraisal Considered as a Process of Multilevel Sequential Checking. In _Appraisal Processes in Emotion_, Oxford University Press.

- Ortony, A., Clore, G.L., & Collins, A. (1988). _The Cognitive Structure of Emotions_. Cambridge University Press.

---

## Temporal Affect Models

- Gebhard, P. (2005). ALMA — A Layered Model of Affect. _AAMAS_, 2005.

---

## Affect in Agent Systems

- Zhang, Y. et al. (2024). Self-Emotion Blended Dialogue Generation in Social Simulation Agents. _SIGDIAL_, 2024.

- Gadanho, S.C. (2003). Learning Behavior-Selection by Emotions and Cognition in a Multi-Goal Robot Task. _Journal of Machine Learning Research_, 4, 385-412.

- Barthet, M. et al. (2022). Play with Emotion: Affect-Driven Reinforcement Learning. _2022 10th International Conference on Affective Computing and Intelligent Interaction (ACII)_.

- Seligman, M.E.P. (1972). Learned Helplessness. _Annual Review of Medicine_, 23, 407-412.

- van Haeringen et al. (2023). Emotion contagion in agent-based simulations of crowds: a systematic review. _Autonomous Agents and Multi-Agent Systems_.

---

## Emotional Depotentiation

- Walker, M.P. & van der Helm, E. (2009). Overnight Therapy? The Role of Sleep in Emotional Brain Processing. _Psychological Bulletin_, 135(5), 731-748.

---

## Emotional RAG

- Huang et al. (2024). Emotional RAG. arXiv:2410.23041.
  *Grounds: PAD vectors on knowledge entries — emotion-tagged retrieval significantly outperforms non-emotional retrieval across three datasets. Validates attaching PAD vectors to every NeuroStore entry.*

---

## Affective Computing Surveys and Frameworks (2025)


- Intelligent Agents with Emotional Intelligence (2025). Current Trends, Challenges, and Future Prospects. arXiv:2511.20657.

- Emotions in the Loop (2025). A Survey of Affective Computing for Emotional Support. arXiv:2505.01542.

---

## Cross-References

- See [03-dreams-and-offline-learning.md](./03-dreams-and-offline-learning.md) for Walker & van der Helm emotional depotentiation in the dreams context
- See [01-memory-consolidation.md](./01-memory-consolidation.md) for mood-congruent retrieval in the memory context
- See topic [09-daimon](../09-daimon/INDEX.md) for the full Daimon subsystem design
