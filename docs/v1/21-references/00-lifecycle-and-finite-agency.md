# Lifecycle and Finite Agency

> Annotations are kept only for works whose descriptions were checked against the papers (listed in
> `tools/docs_integrity/content_audit.json` and in the banner of docs/v3/REFERENCES.md). The other entries are
> bare citations: their annotations were removed because they had not been checked against the papers.

> **DEPRECATED (v1):** This document is part of the v1 specification and may be outdated. See [../../v2/](../../v2/) for the current reference.


> Academic foundations for knowledge lifecycle management, resource-bounded cognition, and evolutionary computing as they apply to the Roko agent framework.

**Topic**: [References](./INDEX.md)
**Prerequisites**: [Architecture](../00-architecture/INDEX.md)
**Key sources**: `bardo-backup/prd/02-mortality/14-research-foundations.md`, `bardo-backup/prd/02-mortality/15-references.md`

> **Implementation**: Reference

---

## Abstract

This domain collects the research originally framed around agent mortality and finite lifespans, reframed for Roko's architecture where the relevant concepts are **knowledge lifecycle management**, **resource-bounded cognition**, and **evolutionary skill evolution**. The core insight remains valid: systems without resource constraints, decay mechanisms, or turnover exhibit stagnation, plasticity loss, and technical debt accumulation. Roko applies these findings to knowledge decay (Ebbinghaus half-lives on Engrams), budget-driven urgency (economic pressure replaces economic "death"), and evolutionary skill libraries (EvoSkills). The mortality-specific narrative is removed; the empirical findings and mechanisms are preserved in full.

The original source (`02-mortality/14-research-foundations.md`) contained 130+ papers across mortality modeling, memory, affect, coordination, self-learning, security, generational learning, and context engineering. Citations from domains with their own sub-docs (memory, affect, dreams, etc.) appear in those dedicated sub-docs and are cross-referenced here. This sub-doc retains the citations most directly related to lifecycle, finite agency, and evolutionary computing.

---

## Evolutionary Computing and Digital Life

- Ray, T.S. (1991). An Approach to the Synthesis of Life. _Artificial Life II_, Addison-Wesley, 1992.

- Lenski, R.E. et al. (2003). The Evolutionary Origin of Complex Features. _PNAS_, 100(9).

- Vostinar, A.E. et al. (2019). Suicidal selection: Programmed cell death evolves as adaptive behavior under spatial structure. _Evolution_, 73(5).


- Werfel, J. et al. (2017). How Short-Lived Agents Can Collectively Build Long-Lived Structures. _Artificial Life_, 23(3).

---

## Plasticity, Continual Learning, and Drift

- Dohare, S. et al. (2024). Loss of Plasticity in Deep Continual Learning. _Nature_, 632.

- Vela, B. et al. (2022). Temporal quality degradation in AI models. _Journal of Data and Information Quality_, 14(1).

- Sculley, D. et al. (2015). Hidden Technical Debt in Machine Learning Systems. _NeurIPS_.

- Arbesman, S. (2012). _The Half-Life of Facts: Why Everything We Know Has an Expiration Date_. Current/Penguin.

---

## Resource-Bounded Cognition

- Ord, T. (2025). Is there a half-life for the success rates of AI agents?. Working paper.

- Sims, C.A. (2003). Implications of Rational Inattention. _Journal of Monetary Economics_, 50(3), 665-690.

- Orseau, L. & Ring, M. (2011). Self-Modification and Mortality in Artificial Agents. _AGI_, 2011.

- Orseau, L. & Armstrong, S. (2016). Safely Interruptible Agents. _UAI_, 2016.

---

## Cooperation Under Resource Constraints

- Kreps, D., Milgrom, P., Roberts, J., & Wilson, R. (1982). Rational Cooperation in the Finitely Repeated Prisoners' Dilemma. _Journal of Economic Theory_, 27(2), 245-252.

- Ohtsuki, H. et al. (2006). A simple rule for the evolution of cooperation on graphs and social networks. _Nature_, 441, 502-505.

- Nakamaru, M., Matsuda, H., & Iwasa, Y. (1997-1998). The Evolution of Cooperation in a Lattice-Structured Population. _Journal of Theoretical Biology_.

- Smith, J.M. (1992). Byte-sized evolution. _Nature_, 355, 772-773.

---

## Compression as Regularization

- Shuvaev, S. et al. (2024). Encoding Innate Ability Through a Genomic Bottleneck. _PNAS_, 121(39).

- Hinton, G.E. (2022). The Forward-Forward Algorithm: Some Preliminary Investigations. Working paper.

- Ororbia, A. & Friston, K. (2023). Mortal Computation. Working paper.

---

## Generational Learning and Cultural Evolution

- Baldwin, J.M. (1896). A New Factor in Evolution. _American Naturalist_, 30, 441-451.

- Heard, E. & Martienssen, R.A. (2014). Transgenerational Epigenetic Inheritance: Myths and Mechanisms. _Cell_, 157(1), 95-109.

- Bhoopchand et al. (2023). Few-shot imitation as cultural transmission. Working paper.



- Martin, J., Everitt, T., & Hutter, M. (2016). Death and Suicide in Universal Artificial Intelligence. _AGI_, 2016.

- Gerstgrasser, M. et al. (2023). Selectively Sharing Experiences Improves Multi-Agent Reinforcement Learning. Working paper.

---

## Biological Analogues (Historical Reference)

These citations are preserved for historical completeness. They were originally framed as direct analogues for agent mortality; in Roko they serve as inspirational references for lifecycle management patterns.

- Hayflick, L. (1961). The Serial Cultivation of Human Diploid Cell Strains. _Experimental Cell Research_, 25(3), 585-621.

- Kirkwood, T.B.L. (1977). Evolution of Ageing. _Nature_, 270, 301-304.

- Hanahan, D. & Weinberg, R.A. (2000, 2011). The Hallmarks of Cancer. _Cell_, 100(1) & 144(5).

- Skulachev, V.P. (1999). Phenoptosis: Programmed Death of an Organism. _Biochemistry (Moscow)_, 64(12).

- Ramsdell, F. & Fowlkes, B.J. (1990). Clonal Deletion versus Clonal Anergy: The Role of the Thymus in Inducing Self Tolerance. _Science_, 248(4961).

- Simard, S.W. (2018). Mycorrhizal networks facilitate tree communication, learning, and memory. In _Memory and Learning in Plants_, Springer.

---

## Cross-References

- See [01-memory-consolidation.md](./01-memory-consolidation.md) for memory-specific citations (McClelland 1995, Ebbinghaus 1885, etc.)
- See [06-self-learning-systems.md](./06-self-learning-systems.md) for Reflexion, ExpeL, Voyager, and DSPy
- See [23-generational-and-evolutionary.md](./23-generational-and-evolutionary.md) for Ray 1991 and Lenski LTEE in the evolutionary computing context
- See topic [17-lifecycle](../17-lifecycle/INDEX.md) for how these citations ground Roko's agent lifecycle design
