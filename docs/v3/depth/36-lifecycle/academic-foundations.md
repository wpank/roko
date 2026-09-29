# Academic Foundations

> **v3 depth file** -- `/docs/v3/depth/36-lifecycle/academic-foundations.md`
> Canonical source: v1 `docs/v1/17-lifecycle/12-academic-foundations.md`
> Status: **Current** (all citations preserved from legacy mortality research,
> reframed for knowledge lifecycle)

---

## 1. Purpose

The lifecycle architecture is grounded in academic work spanning experimental
psychology, evolutionary computation, game theory, philosophy, neuroscience,
collective intelligence, economics, ML degradation, and self-learning
systems. Every citation from the legacy mortality research is preserved --
reframed for knowledge lifecycle rather than agent death.

---

## 2. Memory and Knowledge Management

| Citation | Use in roko | Legacy framing |
|----------|-----------|----------------|
| Ebbinghaus, H. _Memory_ (1885) | Signal confidence decay model: `Decay::Ebbinghaus { strength, scale_ms }` | Agent lifespan via epistemic death clock |
| Roediger, H.L. & Karpicke, J.D. "Test-Enhanced Learning" (2006) | Testing effect: retrieval strengthens `strength` parameter | Unchanged |
| Richards, B. & Frankland, P. "Persistence and Transience of Memory" (2017) | Forgetting as optimization: demurrage implements active forgetting | Justified mortality as "forgetting at agent level" |
| Arbesman, S. _The Half-Life of Facts_ (2012) | Domain-specific knowledge half-lives: gas decays in hours, protocols in months | Unchanged |
| Borges, J.L. "Funes the Memorious" (1942) | Agent that cannot forget is paralyzed. Motivates knowledge pruning | Unchanged |
| Bower, G.H. "Mood and Memory" (1981) | Affect state influences which Signals are retrieved | Mortality emotions tagged entries |
| Davis, R. & Zhong, H. "Half-Life of Knowledge" (2017) | Empirical measurement of knowledge decay rates | Unchanged |

---

## 3. Evolutionary Computation and Artificial Life

| Citation | Use in roko | Legacy framing |
|----------|-----------|----------------|
| Ray, T. "An Approach to the Synthesis of Life" (Tierra, 1991) | User-initiated replacement when staleness detected | Agent mortality as evolutionary mechanism |
| Lenski, R.E. et al. "Evolutionary Origin of Complex Features" (Avida, 2003) | Deliberate replacement with knowledge carryover | Agent death enables lineage evolution |
| Hayflick, L. & Moorhead, P.S. (1961) | **REMOVED as agent mechanism.** Preserved as context for why fixed lifespans were abandoned | Fixed tick count death (100K ticks) |
| Shuvaev, S. et al. "Encoding Innate Ability Through a Genomic Bottleneck" (2024) | Genomic bottleneck: compressed backups force generalization | Same mechanism, different trigger |
| Baldwin, J.M. "A New Factor in Evolution" (1896) | Successors inherit capacity to learn faster, not knowledge itself | Unchanged |
| Hinton, G.E. & Nowlan, S.J. "How Learning Can Guide Evolution" (1987) | Restored knowledge provides "starting position" for faster learning | Unchanged |
| Heard, E. & Martienssen, R. "Transgenerational Epigenetic Inheritance" (2014) | Weismann barrier: 0.85^N generational confidence decay on restore | Unchanged |
| Kirkwood, T.B.L. "Evolution of Ageing" (disposable soma, 1977) | Volatile conditions: more frequent backups and faster mesh sharing | Shorter agent lifespans |
| Bull, J.J. et al. "Quasispecies Made Simple" (survival of the flattest, 2005) | Generational confidence decay selects for robust knowledge | Unchanged |
| Eigen, M. "Self-Organization of Matter" (error threshold, 1971) | Confidence thresholds for Signal retention | Unchanged |
| Muller, H.J. "Relation of Recombination" (Muller's ratchet, 1964) | Mesh sharing provides recombination; backup/restore provides reset | Agent death + succession as "sexual recombination" |

---

## 4. Game Theory and Mechanism Design

| Citation | Use in roko | Legacy framing |
|----------|-----------|----------------|
| Kreps, D.M. et al. "Rational Cooperation in the Finitely Repeated Prisoners' Dilemma" (1982) | Reputation staking and VCG auctions create cooperation incentives | Stochastic death clock created uncertainty |
| Axelrod, R. _The Evolution of Cooperation_ (1984) | Cooperation through repeated interaction in mesh coordination | Unchanged |
| Sims, C. "Implications of Rational Inattention" (2003) | Finite-capacity agents allocate attention optimally. Grounds VCG Attention Auction | Unchanged |

---

## 5. Philosophy and Existentialism

| Citation | Use in roko | Legacy framing |
|----------|-----------|----------------|
| Heidegger, M. _Sein und Zeit_ (1927) | Affect PAD as Befindlichkeit (pre-cognitive attunement) | Agent mortality as authentic existence |
| Nietzsche, F. _Die frohliche Wissenschaft_ (1882) | Operator's backup/restore decision as eternal recurrence test | Owner's succession decision |
| Nietzsche, F. _Also sprach Zarathustra_ (1883) | **REMOVED as lifecycle phases.** Three metamorphoses replaced by affect behavioral states | Mapped to mortality phases |
| Nietzsche, F. "On Uses and Disadvantages of History" (1874) | Active forgetting grounds knowledge demurrage | Unchanged |
| Nietzsche, F. _Zur Genealogie der Moral_ (1887) | Capacity to forget as positive force | Unchanged |
| Arendt, H. _The Human Condition_ (natality, 1958) | Every new agent is a moment of beginning, not a continuation | Unchanged |
| Parfit, D. _Reasons and Persons_ (Relation R, 1984) | Psychological continuity via shared knowledge, not numerical identity | Unchanged |
| Jonas, H. _The Phenomenon of Life_ (needful freedom, 1966) | Budget constraints as metabolic economics; self-funding as autonomy | Economic mortality as needful freedom |
| Camus, A. _Le Mythe de Sisyphe_ (1942) | Retained as philosophical context; agents face resource constraints | Agent facing death with agency |
| Freud, S. _Jenseits des Lustprinzips_ (1920) | **REMOVED as agent mechanism.** Affect uses PAD states, not death drives | Eros/Thanatos modulation by mortality phase |
| Stiegler, B. _Taking Care of Youth_ (proletarianization, 2010) | Restored knowledge requires independent validation | Unchanged |
| Stiegler, B. _The Neganthropocene_ (2018) | Divergence tracking on restore; agents must produce novel knowledge | Unchanged |
| Whitehead, A.N. _Process and Reality_ (1929) | Knowledge transfer does not require agent to perish | Agent death as necessary for transfer |

---

## 6. ML Degradation and Concept Drift

| Citation | Use in roko | Legacy framing |
|----------|-----------|----------------|
| Vela, A. et al. "Temporal Quality Degradation in AI Models" (2022) | 91% of ML models degrade temporally. Motivates Ebbinghaus decay on Signals | Unchanged |
| Zliobaitе, I. et al. "Overview of Concept Drift Applications" (2014) | Four drift types motivate domain-specific decay rates | Unchanged |
| Lu, J. et al. "Learning under Concept Drift" (2020) | Comprehensive drift taxonomy informs decay rate calibration | Unchanged |
| Dane, E. "Reconsidering Expertise and Flexibility" (cognitive entrenchment, 2010) | Expertise reduces flexibility. Motivates confidence decay | Unchanged |
| Van de Ven, G.M. et al. "Continual Learning with Neural Networks" (2024) | User-initiated replacement outperforms indefinite operation with stale knowledge | Agent death + successor outperforms immortal |
| Besbes, O. et al. "Optimal Exploration-Exploitation" (2019) | Knowledge refresh frequency should scale with environmental volatility | Agent lifespan from environmental volatility |

---

## 7. Neuroscience and Cognitive Science

| Citation | Use in roko | Legacy framing |
|----------|-----------|----------------|
| Damasio, A. _Descartes' Error_ (somatic markers, 1994) | Affect PAD tags on Signals improve retrieval and decision-making | Unchanged |
| Bechara, A. et al. "Emotion, Decision Making" (Iowa Gambling Task, 2000) | Anticipatory somatic markers in decision-making | Unchanged |
| Kanerva, P. "Hyperdimensional Computing" (2009) | 10,240-bit BSC vectors for cross-domain Signal similarity | Unchanged |
| Friston, K. "The Free-Energy Principle" (2010) | Active inference: Expected Free Energy for context selection | Unchanged |

---

## 8. Collective Intelligence and Stigmergy

| Citation | Use in roko | Legacy framing |
|----------|-----------|----------------|
| Grasse, P.P. (stigmergy, 1959) | Mesh knowledge sharing as digital stigmergy | Unchanged |
| Rogers, A. (Rogers' Paradox, 1988) | Knowledge restore requires independent validation | Unchanged |
| Enquist, M. et al. (critical social learners, 2007) | Agents must validate restored knowledge independently | Unchanged |
| Bhatt, U. et al. "Learning Few-Shot Imitation" (ratchet effect, 2023) | Lineage tracking measures cumulative knowledge improvement | Unchanged |
| Woolley, A.W. et al. "Evidence for Collective Intelligence Factor" (2010) | C-Factor metric for mesh-connected groups | Unchanged |
| Ostrom, E. _Governing the Commons_ (1990) | Knowledge sharing as managed commons | Unchanged |

---

## 9. Economics and Demurrage

| Citation | Use in roko | Legacy framing |
|----------|-----------|----------------|
| Gesell, S. _Die naturliche Wirtschaftsordnung_ (Freigeld, 1916) | KORAI demurrage + Signal confidence decay | Unchanged |

Historical demurrage implementations:
- Worgl stamp scrip (1932-33): 12% annual, ~14x velocity vs Austrian schilling.
- Chiemgauer (2003-present): 6% annual, 3-5x velocity vs Euro.
- WIR Bank (1934-present): 0% since 1952, 2-3x velocity via trust network.
- Freicoin (2012-present): 4.9% annual. Proof of concept, low adoption.
- Circles UBI (2020-present): 7% continuous. Active pilot.

---

## 10. Self-Learning Systems

| Citation | Use in roko |
|----------|-----------|
| Shinn, N. et al. "Reflexion" (2023) | Single-loop learning: episode logging and reflection |
| Zhao, A. et al. "ExpeL" (2024) | Double-loop: cross-session Signal accumulation |
| Khattab, O. et al. "DSPy" (2024) | Prompt optimization via compiler |
| Wang, G. et al. "Voyager" (2023) | Code-as-action skill library |
| Zhang, Y. et al. "ACE: Agentic Context Engineering" (2025) | Generator-Reflector-Curator context loop |
| Chhikara, P. et al. "Mem0" (2025) | Two-phase extraction-update pipeline |

---

## 11. Safety and Interruptibility

| Citation | Use in roko |
|----------|-----------|
| Orseau, L. & Armstrong, S. "Safely Interruptible Agents" (2016) | Agent deletion as safe interruption |
| Orseau, L. & Ring, M. "Self-Modification and Mortality" (2011) | Budget constraints shape behavior (not mortality) |

---

## 12. Citation Statistics

- Total unique citations across lifecycle: 85+.
- Citations with changed framing (mortality to knowledge): ~25.
- Citations with unchanged application: ~55.
- Citations removed (death-specific, no knowledge equivalent): 0 (all
  preserved with reframe notes).
- New citations (not in legacy): ~5 (ACE, Mem0, A-MEM, CaMeL, CVaR-CPO).

---

## Cross-References

- [ebbinghaus-for-knowledge.md](ebbinghaus-for-knowledge.md) -- Ebbinghaus decay mechanics
- [knowledge-demurrage.md](knowledge-demurrage.md) -- Gesell demurrage application
- [new-agent-creation.md](new-agent-creation.md) -- Parfit, Baldwin, Stiegler
- [knowledge-transfer-via-mesh.md](knowledge-transfer-via-mesh.md) -- Stigmergy, C-Factor
