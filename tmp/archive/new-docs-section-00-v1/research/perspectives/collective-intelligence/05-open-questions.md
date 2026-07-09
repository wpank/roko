# Open Questions — Collective Intelligence Lens

**Kind**: Perspective
**Source**: `docs/00-architecture/14-c-factor-collective-intelligence.md`

---

This page collects the unresolved questions that sit at the frontier of the collective-intelligence
lens applied to Roko. Unlike the Open Questions sections at the end of individual pages, this page
gathers *cross-cutting* and *hard* questions — those that require research, empirical
investigation, or architectural experiments to resolve. Questions here are candidates for future
research workstreams, not immediate implementation tasks.

---

## Measurement Questions

### Can Roko's c-factor be directly measured?

The *c*-factor is defined operationally as the first principal component of a group's performance
across a diverse task battery. For a human group, the battery typically includes matrix reasoning,
analogy, reading comprehension, mathematics, and social tasks (Woolley et al. 2010).

For Roko, a parallel battery would include:
- **Retrieval tasks**: find a specific piece of information from Substrate
- **Inference tasks**: derive a novel conclusion from stored engrams
- **Creative tasks**: generate novel recombinations
- **Social tasks**: parse user intent, model emotional valence
- **Planning tasks**: decompose a goal into operator sub-tasks

Factor analysis of Roko's performance across this battery would yield an empirical *c* estimate.
The challenge is that the battery must be genuinely diverse — using tasks that load on
different underlying competencies — and it must be run against a *fixed system snapshot* to
produce a meaningful scalar.

**Open**: What is the minimum battery size for a reliable *c* estimate? How stable is the
estimate across operator configurations?

### Is routing accuracy observable in production?

Routing accuracy requires knowing, after the fact, whether a problem was assigned to the most
capable available operator. In a controlled setting this is measurable. In production, the ground
truth (what the optimal routing would have been) is rarely available.

Proxy measures exist — downstream output quality, user satisfaction, retry rates — but they are
noisy. A more principled approach might use *held-out validation problems* with known optimal
routings, injected periodically into the production stream.

**Open**: What is a viable production-observable proxy for routing accuracy that can be computed
without ground-truth labels?

### How should integration loss be measured?

Composer discards some operator contributions in producing its synthesis. Integration loss is the
information value of the discarded contributions. Measuring it requires knowing what the output
*would have been* if the discarded contributions had been retained — a counterfactual that is
not directly observable.

One approach: *ablation studies*. Hold out one operator's contribution at synthesis time, measure
output quality change. Aggregate over many held-out operators to estimate how much each
contributes. This gives an empirical estimate of integration loss per operator.

**Open**: Is ablation-based integration loss measurement computationally feasible in production?
What is the right sampling strategy to keep it tractable?

---

## Theoretical Questions

### What is the human-AI group c-factor?

Engel et al. (2014) extended the *c*-factor measurement to online groups and found that *c*
held up under remote collaboration. The obvious next step is hybrid human-AI groups: does a
human + Roko collective have a measurable *c*? And does the human-AI hybrid outperform
human-only or AI-only groups on the same task battery?

This is not merely an academic question. If hybrid groups have higher *c* than either component
alone, it grounds a specific architectural choice: Roko should be designed to *augment* human
cognition as part of a larger collective, not to replace it.

**Open**: Is there empirical data on c-factor measurement in human-AI hybrid groups? What task
types show the largest hybrid advantage?

### Does the Hong-Page theorem hold for AI ensembles?

Hong and Page (2004) proved their diversity theorem under specific conditions: agents use
*diverse heuristics*, agents are individually *competent* (exceed chance), and agents operate on
a *fixed problem landscape*. AI scoring ensembles satisfy some of these conditions and may
violate others.

The most uncertain condition is *problem landscape stability*. The inputs Roko processes are
drawn from a distribution that shifts over time — user queries evolve, new domains emerge, old
patterns fade. The Hong-Page proof assumes a fixed landscape; it is unclear whether error
cancellation persists across a shifting distribution.

**Open**: Under what conditions does the diversity theorem extend to non-stationary problem
distributions? Is there an empirical upper bound on diversity benefit for ensemble scorers?

### Is Daimon an emergent group mind or a designed coordinator?

The [Daimon](../../../reference/09-cross-cuts/README.md) represents Roko's persistent
self-model — the system's representation of its own values, goals, and identity. From the
collective-intelligence lens, this raises a deep question: is Daimon the *group mind* that
emerges from the interactions of all operators, or is it a *designed coordinator* that shapes
those interactions?

In human organisations, the analogue is organisational culture: partly designed (through
explicit norms, hiring, training) and partly emergent (through the accumulated interactions of
members). The appropriate relationship between designed and emergent aspects of Daimon may be
the single most important long-horizon design question.

**Open**: What properties of Daimon should be fixed by design, and which should be allowed to
emerge from system experience? Where is the boundary?

---

## Scaling Questions

### At what agent count does Roko's effective c-factor peak?

The *c*-factor literature suggests a peak around 4–7 members for simple tasks and somewhat
larger for complex tasks. Beyond that range, coordination overhead begins to dominate. For Roko,
the analogous question is: what is the optimal number of active operators per loop pass?

Theoretical prediction: the optimal count should scale with *task complexity*, measured by the
number of distinct cognitive sub-tasks the problem requires. A simple retrieval query needs one
or two operators; a complex multi-step reasoning problem might benefit from five or six. The
Router should dynamically select team size, not use a fixed pool.

**Open**: Is there empirical evidence for a complexity-vs-optimal-operator-count relationship in
multi-agent AI systems? Can this be derived from the *c*-factor literature by analogy?

### Does c-factor persist across heterogeneous operator generations?

Human groups maintain *c* as members join and leave, as long as the transactive memory system
is maintained. For Roko, operator versions change — a Scorer updated to a new model version is
effectively a new group member with a partially overlapping but partially distinct set of
capabilities.

The risk is *capability-directory desync*: the Router continues to route based on the old
capability model even after the operator's capability profile has changed. The learning loop
(Neuro / Dreams) should catch this, but the latency between operator update and directory update
creates a window of mis-routing.

**Open**: What is the maximum acceptable latency between operator capability change and Router
directory update? How should this be detected and measured?

---

## Philosophical Questions

### Is Roko's collective intelligence morally significant?

There is an emerging philosophical literature on *collective consciousness* and whether groups
can be moral patients — whether the collective experience of a group has moral weight independent
of the experiences of its members (see Shulman & Bostrom 2009; List & Pettit 2011 on group
agency).

For Roko: if the collective is genuinely intelligent in the sense the *c*-factor research
describes — if there is something it is like to be Roko as a collective — then the design
choices that shape that collective experience may carry moral weight. This is not a near-term
operational question, but it should inform the long-horizon design philosophy.

**Open**: Is there a principled distinction between a collective that is merely functionally
intelligent and one that might be morally significant? What would that distinction look like for
an AI collective?

### When does augmentation become replacement?

The strongest case for the collective-intelligence lens is that Roko should be designed to
augment human cognition in hybrid groups, not to replace it. The empirical data supports this:
hybrid groups show higher *c* than either component alone on many tasks.

But the category boundary is not obvious. A Roko that handles all of a user's cognitive tasks
faster and more accurately than the user can handle them has not augmented the user — it has
replaced the user as the cognitive agent. The user's *c*-factor contribution to the hybrid
approaches zero.

**Open**: Is there a design criterion for distinguishing augmentation from replacement in
human-AI collaborative systems? Should such a criterion be operationalised in Roko's Policy?

---

## See Also

- [00-overview.md](00-overview.md) — The lens framing
- [01-c-factor.md](01-c-factor.md) — Measurement methodology
- [02-from-individuals-to-collectives.md](02-from-individuals-to-collectives.md) — Bridging mechanisms
- [03-roko-application.md](03-roko-application.md) — Full Roko component mapping
- [04-implications.md](04-implications.md) — Design constraints with measurement criteria
- [../emergent-goals/05-open-questions.md](../emergent-goals/05-open-questions.md) — Overlapping questions on goal emergence from collective dynamics
- [../../../research/foundations/c-factor.md](../../foundations/c-factor.md) — Foundational theory
