# 39-02 Affective Computing -- Annotated Reference Map

> Research foundations for emotion modeling, somatic markers, and affect-modulated
> cognition in the Roko Daimon subsystem.
>
> **v3 depth file** -- updated 2026-09-15.

---

## PAD Emotional State Model

**[Mehrabian, 1996]** *Pleasure-Arousal-Dominance: A General Framework.* Current Psychology, 14(4), 261--292.
Foundational PAD model. Three continuous dimensions capture more variance than discrete labels. Roko uses: Pleasure = task success, Arousal = urgency/load, Dominance = confidence.

**[Russell & Mehrabian, 1977]** *Evidence for a Three-Factor Theory of Emotions.* Journal of Research in Personality, 11(3), 273--294.
Empirical validation. PAD octants map to behavioral tendencies. Grounds the three-axis choice.

---

## Somatic Marker Hypothesis

**[Damasio, 1994]** *Descartes' Error: Emotion, Reason, and the Human Brain.* Putnam.
Patients without emotion make worse decisions under uncertainty. Somatic markers bias choices before deliberation. Roko implements a k-d tree over 8-dimensional strategy space.

**[Bechara, Damasio & Damasio, 2000]** *Emotion, Decision Making and the Orbitofrontal Cortex.* Cerebral Cortex, 10(3), 295--307.
Anticipatory SCRs precede conscious awareness in the Iowa Gambling Task. SomaticLandscape provides fast heuristic feelings before analytical reasoning.

**[Bechara & Damasio, 2005]** *The Somatic Marker Hypothesis: A Neural Theory of Economic Decision.* Games and Economic Behavior, 52, 336--372.
Formal integration with economic decision theory. Validates PAD-modulated tier routing for cost-sensitive decisions.

---

## Mood-Congruent Memory

**[Bower, 1981]** *Mood and Memory.* American Psychologist, 36(2), 129--148.
Emotional states bias retrieval via associative activation. Implemented as the emotional factor (0.15 weight) in four-factor retrieval. Motivates 15% contrarian retrieval.

**[Blaney, 1986]** *Affect and Memory: A Review.* Psychological Bulletin, 99(2), 229--246.
Comprehensive review confirming mood-congruent effects. Supports emotional congruence as a retrieval factor.

**[Phelps, 2004]** *Human Emotion and Memory.* Current Opinion in Neurobiology, 14(2), 198--202.
Emotion and memory as a single system. Grounds the `emotional_tag` field on every NeuroStore entry.

**[Cahill & McGaugh, 1998]** *Mechanisms of Emotional Arousal and Lasting Declarative Memory.* Trends in Neurosciences, 21(7), 294--299.
Arousal enhances consolidation via amygdala modulation. Grounds the `arousal_encoding_factor()` function.

---

## Emotion Classification and Appraisal

**[Plutchik, 1980]** *Emotion: A Psychoevolutionary Synthesis.* Harper & Row.
Eight primary emotion pairs. PAD octants map to Plutchik categories for human-readable state labeling.

**[Scherer, 2001]** *Appraisal Considered as a Process of Multilevel Sequential Checking.* In Appraisal Processes in Emotion, Oxford University Press.
Multi-level sequential checking mirrors the 7-axis Score on Signals. Each axis is an independent appraisal dimension.

**[Ortony, Clore & Collins, 1988]** *The Cognitive Structure of Emotions.* Cambridge University Press.
OCC cognitive appraisal model. Complements PAD with a structural framework for interpreting behavioral states.

---

## Temporal Affect Models

**[Gebhard, 2005]** *ALMA -- A Layered Model of Affect.* AAMAS 2005.
Three-layer temporal affect: emotion (seconds), mood (hours), personality (lifetime). Implemented as tick-level emotion, EMA mood, and static personality configuration.

---

## Affect in Agent Systems

**[Zhang et al., 2024]** *Self-Emotion Blended Dialogue Generation in Social Simulation Agents.* SIGDIAL 2024.
Self-emotion changes ~50% of agent decisions. Demonstrates affect as a primary behavioral driver, not decorative.

**[Gadanho, 2003]** *Learning Behavior-Selection by Emotions and Cognition.* JMLR, 4, 385--412.
ALEC architecture: 40% fewer collisions vs cognition alone. Validates combined Daimon + cognitive loop.

**[Barthet et al., 2022]** *Play with Emotion: Affect-Driven Reinforcement Learning.* 2022 10th International Conference on Affective Computing and Intelligent Interaction (ACII).
Affect modulates RL exploration-exploitation. Informs affect-energy coupling in CorticalState.

**[Seligman, 1972]** *Learned Helplessness.* Annual Review of Medicine, 23, 407--412.
Learned helplessness from repeated failure. Dominance < -0.3 for 200+ ticks triggers burnout alert.

**[van Haeringen et al., 2023]** *Emotion contagion in agent-based simulations of crowds: a systematic review.* Autonomous Agents and Multi-Agent Systems.
Anger spreads competitively. Arousal contagion capped at +0.3 per sync cycle in Collectives.

---

## Emotional Depotentiation

**[Walker & van der Helm, 2009]** *Overnight Therapy? The Role of Sleep in Emotional Brain Processing.* Psychological Bulletin, 135(5), 731--748.
REM depotentiates emotional charge while preserving content. Dream cycles reduce arousal by 0.3--0.5 per cycle.

---

## Emotional RAG

**[Huang et al., 2024b]** *Emotional RAG.* arXiv:2410.23041.
Emotion-tagged retrieval outperforms non-emotional retrieval across three datasets. Validates PAD vectors on every NeuroStore entry.

---

## Surveys and Frameworks (2025)

**[Zall et al., 2025]** *Intelligent Agents with Emotional Intelligence.* arXiv:2511.20657.
Emotional intelligence identified as architecturally vital for agent systems.

**[Hegde & Jayalath, 2025]** *Emotions in the Loop.* arXiv:2505.01542.
Affect integrated into interaction loops. Validates Daimon-in-the-loop design.

**[Fabiano, 2025]** *Affective Computing and Emotional Data: Challenges in Privacy Regulations.* arXiv:2509.20153.
Privacy implications under the EU AI Act. Informs compliance requirements for the Daimon.

---

## Cross-References

- Dreams emotional depotentiation: [03-dreams-and-offline-learning](./03-dreams-and-offline-learning.md)
- Mood-congruent retrieval in memory context: [01-memory-consolidation](./01-memory-consolidation.md)
- Daimon subsystem design: depth/11-affect/
