# C-Factor — Theoretical Basis

> The theoretical basis for collective intelligence measurement: Woolley et al. (2010)
> and Engel et al. (2014). Architectural application links to the perspective folder:
> [`research/perspectives/collective-intelligence/`](../perspectives/collective-intelligence/README.md).

**Kind**: Foundation
**Source**: `docs/00-architecture/14-c-factor-collective-intelligence.md`
**Application**: [`research/perspectives/collective-intelligence/`](../perspectives/collective-intelligence/README.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

The **c-factor** (collective intelligence factor) is a group-level analogue of the
individual-level *g*-factor (general intelligence). Woolley et al. (2010) demonstrated
empirically that group performance across diverse cognitive tasks is predicted by a single
latent factor — *c* — that is not well-predicted by the average or maximum intelligence of
individual group members. Engel et al. (2014) extended this to online groups and found that
the social sensitivity of group members — specifically their average score on the Reading
the Mind in the Eyes Test (RME) — is the strongest predictor of *c*. These findings establish
that collective intelligence is a measurable, stable property of groups as units, distinct
from the intelligence of their parts.

---

## The Woolley et al. (2010) Study

### Design

Woolley, Chabris, Pentland, Hashmi, and Malone (*Science*, 2010) assembled 192 groups of
2–5 participants and administered a battery of cognitive tasks designed to mirror those used
to assess individual intelligence: visual puzzles, brainstorming, group moral dilemmas, a
negotiation task, and a typing/decoding task (analogous to a digit-symbol coding test).

### Findings

A **single factor, *c***, emerged from factor analysis of group performance across tasks.
This factor:
- Explained substantial variance in group performance across diverse tasks
- Was distinct from (and not well predicted by) the average or maximum individual intelligence
  of group members
- Was stable across time (groups re-tested weeks later showed similar *c*)
- Predicted group performance on a novel criterion task not used in the factor analysis

### Predictors of C

The factors that significantly predicted *c* were:
1. **Average social sensitivity** of members, measured by the Reading the Mind in the Eyes
   Test (RME; Baron-Cohen et al., 2001): ability to infer mental states from eye region photographs.
2. **Equal distribution of conversational turn-taking**: groups where one person dominated
   conversation had lower *c*. High *c* groups distributed speaking turns more evenly.
3. **Proportion of women**: this was partially mediated by social sensitivity (women scored
   higher on RME on average).

What did *not* significantly predict *c*:
- Average individual IQ of members
- Maximum individual IQ of members
- Group cohesion
- Group satisfaction

The implication is stark: to build a more collectively intelligent group, optimize for social
sensitivity and conversational equality — not for raw individual intelligence.

---

## Engel et al. (2014): Online Groups

### Extension to Distributed Teams

Engel, Woolley, Jing, Chabris, and Malone (*PLOS ONE*, 2014) extended the original study to
groups collaborating entirely online via text-only communication — no video, no audio, no
physical proximity.

### Key Findings

*C* was replicated in online groups and showed the same structure. Critically:

- **Social sensitivity** (RME scores) remained the strongest predictor of group *c*,
  suggesting that the ability to infer others' mental states is not just about reading
  physical cues — it generalizes to text-based interaction.
- **Conversational proportionality** again predicted *c*: groups where one agent dominated
  the textual exchange performed worse collectively.
- The **absence of non-verbal cues** did not destroy collective intelligence — it suppressed
  it somewhat but the factor remained stable and measurable.

The online replication is particularly important for multi-agent AI systems, which interact
entirely through structured data without physical co-presence.

---

## What C Measures

### C Is Not the Sum of Parts

The central theoretical contribution of the c-factor literature is that a group is **not
reducible to a bag of individual intelligences**. The same individuals, organized differently,
produce different *c*. Group intelligence is a property of the *organization* — the
interaction structure — not the components.

This is consistent with complex systems theory more broadly: emergent properties of systems
are often not predictable from the properties of components in isolation. *C* is an emergent
property in exactly this sense.

### C Is About Information Flow

The predictors of *c* (social sensitivity, equal turn-taking) both relate to **information
flow**. Social sensitivity means individuals accurately model each other's mental states —
they "receive" information about others' beliefs and knowledge accurately. Equal turn-taking
means information is distributed: no one monopolizes the channel, and diverse knowledge has
the opportunity to influence group decisions.

A group with low social sensitivity and unequal turn-taking is a group where information
is poorly routed: accurate signals are missed or dominated by a loud few. The group's
effective information bandwidth is lower than the sum of its parts would suggest.

### Collective Working Memory

Some researchers have conceptualized *c* as a group-level analogue of working memory
capacity — the group's ability to hold multiple pieces of information in "group working
memory" and manipulate them collectively. Just as individual working memory is a bottleneck
for individual reasoning, the quality of collective information exchange is a bottleneck
for collective reasoning.

---

## C in Multi-Agent AI Systems

### The Agent-to-Agent Translation

The c-factor framework was developed for human groups. Translating it to multi-agent AI
systems requires some reinterpretation:

| Human group concept | Multi-agent analog |
|---------------------|-------------------|
| Social sensitivity (RME) | Model of peer agent state (knowledge, uncertainty, current task) |
| Equal turn-taking | Balanced contribution of agents to shared knowledge substrate |
| Conversational dominance | One agent monopolizing the Engram substrate or Router allocation |
| Group cohesion (not predictive) | Shared embedding space (not sufficient for *c*) |

The prediction is that multi-agent systems with **accurate peer modeling** and **balanced
knowledge contribution** will outperform systems optimized for individual agent quality
alone.

### Measuring C in Agent Collectives

If *c* is to be a useful metric for Roko, it needs an operational definition. Candidate
approaches:
1. **Task generalization**: measure collective performance on a diverse battery of tasks;
   factor-analyze the results; see if a dominant factor emerges.
2. **Information diversity index**: measure the entropy of the Engram contributions across
   agents — high entropy (balanced contribution) correlates with high *c* in humans.
3. **Peer model accuracy**: measure how accurately each agent models other agents' knowledge
   states (e.g., predicts whether another agent knows a given fact).

---

## Key Papers

- **Woolley, A. W., Chabris, C. F., Pentland, A., Hashmi, N., & Malone, T. W. (2010).**
  "Evidence for a Collective Intelligence Factor in the Performance of Human Groups."
  *Science*, 330(6004), 686–688. The foundational study.

- **Engel, D., Woolley, A. W., Jing, L. X., Chabris, C. F., & Malone, T. W. (2014).**
  "Reading the Mind in the Eyes or Reading between the Lines? Theory of Mind Predicts
  Collective Intelligence Equally Well Online and Face-to-Face." *PLOS ONE*, 9(12), e115212.
  Online extension.

- **Baron-Cohen, S., Wheelwright, S., Hill, J., Raste, Y., & Plumb, I. (2001).** "The
  'Reading the Mind in the Eyes' Test Revised Version: A Study with Normal Adults, and
  Adults with Asperger Syndrome or High-functioning Autism." *Journal of Child Psychology
  and Psychiatry*, 42(2), 241–251. The RME instrument.

- **Malone, T. W., Laubacher, R., & Dellarocas, C. (2010).** "The Collective Intelligence
  Genome." *MIT Sloan Management Review*, 51(3), 21–31. Framework for classifying
  collective intelligence systems.

- **Spearman, C. (1904).** "'General Intelligence,' Objectively Determined and Measured."
  *American Journal of Psychology*, 15(2), 201–292. The individual *g*-factor, of which
  *c* is the collective analog.

---

## Open Questions

- The c-factor was measured in human groups of 2–5. Does it generalize to larger groups
  or to heterogeneous groups (e.g., mixtures of AI and human participants)?
- What is the analog of "social sensitivity" for an AI agent — the ability to model
  another agent's knowledge and uncertainty state? How is this measured operationally?
- Is *c* a single factor in multi-agent AI settings, or does it decompose into multiple
  factors when agents have more diverse capability profiles than human subjects?

---

## See Also

- [`research/perspectives/collective-intelligence/README.md`](../perspectives/collective-intelligence/README.md) — full perspective essay
- [`reference/09-cross-cuts/README.md`](../../reference/09-cross-cuts/README.md) — Neuro as collective knowledge substrate
- [`research/foundations/cybernetics.md`](cybernetics.md) — Law of Requisite Variety (variety as a group property)
