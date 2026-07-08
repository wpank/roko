# From Individual to Collective Intelligence

**Kind**: Perspective
**Source**: `docs/00-architecture/14-c-factor-collective-intelligence.md`

---

## The Aggregation Problem

Individual cognitive ability does not straightforwardly sum. A team of ten people, each scoring
90th-percentile IQ, does not reliably out-perform a team of ten average-IQ people on complex tasks.
The difference lies in how individual contributions are *integrated*: how information flows between
agents, how conflicts are resolved, and how shared representations form.

This is the aggregation problem, and it is one of the oldest puzzles in social science. Arrow's
impossibility theorem (1951) showed that no voting rule can simultaneously satisfy all reasonable
fairness axioms when aggregating individual preference orderings into a collective preference. The
impossibility extends beyond voting: aggregating beliefs, plans, and memories faces irreducible
coordination costs.

Collective intelligence research asks: under what conditions does the aggregate exceed the sum?

---

## Bridging Mechanisms

### 1. Division of Cognitive Labour

The most basic mechanism: different agents specialize in different cognitive sub-tasks, reducing
redundancy and increasing effective bandwidth. Adam Smith's division of labour extends to cognition.
Formal models (Garicano 2000) show that hierarchical knowledge teams — where generalists route
problems to specialists — outperform flat teams when problem complexity is high. The key variable is
the *matching quality* between problem type and specialist expertise.

In distributed AI systems, division of cognitive labour maps onto routing decisions. A router that
consistently assigns summarization tasks to agents with strong compression models and inference tasks
to agents with deep knowledge stores is exploiting cognitive division of labour.

### 2. Social Epistemic Transmission

Information does not merely flow — it transforms in transmission. Rumour research (Bartlett 1932,
*Remembering*) showed that serial reproduction degrades details, preserves structure, and
systematically biases content toward cultural schemas. This is not a failure mode but a feature:
transmission filters noise and amplifies signal that is culturally salient.

For AI systems, transmission fidelity and bias are design choices. What is worth preserving across
agent handoffs? What should be compressed? What must be verbatim?

### 3. Transactive Memory Systems

Wegner (1987) introduced the concept of *transactive memory*: a group-level memory system where
individuals hold specialised slices and simultaneously maintain a shared directory — *who knows
what*. The group's effective memory capacity exceeds any individual's because the directory allows
retrieval from the entire distributed store.

Critical properties of transactive memory systems:

| Property | Description | Failure mode |
|---|---|---|
| **Specialization** | Each member stores a distinct domain | Overlap without coverage |
| **Credibility assignment** | Members trust each other's expertise appropriately | Over- or under-reliance |
| **Coordination** | Directory is accurate and accessible | Directory rot |

High-*c* groups have stronger transactive memory systems (Woolley et al., 2010). The correlation is
not coincidental: a group that knows who-knows-what can route problems efficiently, reducing wasted
effort and false-confidence errors.

### 4. Distributed Sensemaking

Klein, Moon, and Hoffman (2006) describe sensemaking as *the process of creating situational
awareness and understanding in situations of high complexity or uncertainty*. Distributed sensemaking
occurs when multiple agents jointly construct a shared mental model of a situation.

Conditions that support effective distributed sensemaking:
- **Common ground**: shared vocabulary, shared reference frames (Clark & Brennan 1991)
- **Feedback loops**: rapid correction of misunderstanding
- **External representations**: artefacts (whiteboards, diagrams, logs) that externalise partial
  models and make them jointly inspectable

Conditions that impair it:
- **Shared information bias** (Stasser & Titus 1985): groups over-discuss information that all
  members already hold, and under-discuss uniquely-held information — even when the unique
  information is decision-critical
- **Authority gradients**: junior members suppress relevant information when senior members signal
  confidence

### 5. Diversity and Error Cancellation

Lu Hong and Scott Page (2004) proved a formal theorem: under certain conditions, *cognitively
diverse* groups out-perform groups composed purely of the highest-individual-performers. Diversity
in *heuristics* (problem-solving approaches) allows errors to be uncorrelated, producing error
cancellation when predictions are averaged.

The conditions are not always met — diversity benefits require that group members can actually
communicate their diverse perspectives. But the theorem grounds an important design principle: a
team composed of agents with correlated failure modes is epistemically fragile even if each
individual agent is strong.

---

## The Role of Communication Structure

Beyond what agents know, *how they are connected* shapes collective intelligence. Research on
network topology consistently finds that:

- **All-to-all networks** (everyone talks to everyone) maximise information sharing but fail under
  high volume: members are overwhelmed, useful signals drown in noise.
- **Sparse hierarchies** reduce noise but create bottlenecks; authority figures distort information
  flow by signalling preferred answers, suppressing dissent.
- **Small-world networks** (Watts & Strogatz 1998) with high local clustering and short path
  lengths appear to balance noise reduction with rapid global propagation.

Experimental work by Bala and Goyal (1998) on learning networks confirms: network structure
determines whether a group converges on the truth, converges on a false consensus, or fails to
converge at all. The network is not neutral infrastructure — it is part of the cognitive system.

---

## Superforecasting as an Empirical Case

Tetlock and Gardner's Good Judgment Project (2011–2015) provides a large-scale empirical study of
collective human prediction. Key findings:

1. **Individual superforecasters exist**: roughly 2% of participants showed sustained above-chance
   accuracy on two-year-ahead geopolitical predictions.
2. **Superforecaster teams outperform individuals**: teams of superforecasters scored ~10–15% better
   than individual superforecasters, even after controlling for individual ability.
3. **Team protocols matter**: teams given structured debate protocols (explicitly considering
   alternatives, updating on evidence) outperformed teams with unstructured discussion.
4. **Calibration is trainable**: groups exposed to calibration training became more accurate and
   better-calibrated, with improvements persisting across years.

The Tetlock findings ground the claim that collective intelligence is not merely the sum of
individual abilities — coordination mechanism design produces measurable, replicable improvements.

---

## From Human Groups to Human-AI Systems

The mechanisms above were developed for human groups, but they transfer to hybrid human-AI
systems — and from there, to multi-agent AI architectures — with appropriate reinterpretation.

| Human mechanism | AI system analogue |
|---|---|
| Division of cognitive labour | Operator specialization and task routing |
| Transactive memory | Substrate + agent capability metadata |
| Common ground | Shared schema / ontology |
| Small-world network topology | Router topology and message propagation rules |
| Error cancellation through diversity | Ensemble scoring with diverse scoring functions |
| Structured debate protocols | Policy-enforced multi-perspective elicitation |

The key translation challenge: human transactive memory systems develop organically over years of
collaboration. AI systems must construct the equivalent explicitly — which creates both a burden
(it must be designed) and an opportunity (it can be optimised).

---

## Open Questions

- **At what group size does collective intelligence peak?** Large groups show lower *c*, but the
  mechanism is unclear. Is it coordination overhead, attention dilution, or authority-gradient
  suppression?
- **Can transactive memory be learned online?** Human groups update their shared directories slowly.
  Can an AI system maintain an accurate, low-latency model of which agents hold which information?
- **Does cognitive diversity require disagreement?** Error cancellation requires uncorrelated errors,
  which requires genuinely different approaches. But disagreement has coordination costs. What is
  the optimal diversity/coherence trade-off?
- **Is calibration transferable across domains?** Superforecaster calibration is domain-general to a
  degree. Is this transferable to AI agent ensembles, or is calibration domain-specific in ways that
  require per-domain training?
