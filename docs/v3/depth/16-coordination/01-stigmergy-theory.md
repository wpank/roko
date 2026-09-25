# Depth: Stigmergy Theory

> Parent: [16-COORDINATION](../../16-COORDINATION.md) -- Section 1

---

## What Is Stigmergy?

Stigmergy is a mechanism of indirect coordination between agents, where the
trace left in the environment by one agent's action stimulates the performance
of a subsequent action by the same or a different agent. The term was coined
by Pierre-Paul Grasse in 1959 to describe how termites coordinate the
construction of elaborate mound structures without any centralized plan,
blueprint, or direct communication between individuals [Grasse, P.-P. "La
Reconstruction du Nid et les Coordinations Inter-Individuelles chez
Bellicositermes Natalensis et Cubitermes sp." *Insectes Sociaux*, 6(1):41-80,
1959].

The core insight: **agents do not need to communicate with each other
directly. They only need to read from and write to a shared environment.**
The environment itself becomes the coordination medium.

---

## The Termite Example

Grasse observed that termites building a mound follow no central plan:

1. A termite picks up a mud pellet and deposits it at a location.
2. The deposited pellet contains a chemical pheromone that attracts other
   termites.
3. Attracted termites deposit their own pellets nearby, reinforcing the
   pheromone signal.
4. The growing pile of pellets, through its physical shape and pheromone
   concentration, guides further construction -- arches form, chambers emerge,
   ventilation shafts develop.
5. No termite has a model of the whole structure. Each termite responds to
   local stimuli.

The structure that emerges is far more complex than any individual termite
could plan. This is the hallmark of stigmergy: **simple local rules +
persistent environmental modification = complex global behavior**.

---

## Three Conditions (Theraulaz & Bonabeau 1999)

Stigmergy requires three conditions [Theraulaz, G. & Bonabeau, E. "A Brief
History of Stigmergy." *Artificial Life*, 5(2):97-116, 1999]:

| Condition | Description | Roko Equivalent |
|-----------|-------------|-----------------|
| **Shared environment** | All agents can read from and write to a common medium | NeuroStore (local Substrate), Agent Mesh (peer network), chain (global ledger) |
| **Persistent modifications** | Agent actions leave traces that outlast the agent's presence | Signals with configurable decay rates (1h--infinite) |
| **Stimulus-response coupling** | Traces in the environment trigger specific behaviors in agents that encounter them | Agents react to scored Signals in the pheromone field |

When all three conditions are met, coordination emerges without any agent
needing a global view, without any central coordinator, and without agents
needing to know about each other's existence.

---

## Two Forms of Stigmergy

Grasse and subsequent researchers identified two distinct forms [Holland, O.
& Melhuish, C. "Stigmergy, Self-Organization, and Sorting in Collective
Robotics." *Artificial Life*, 5(2):173-202, 1999]:

### Sematectonic Stigmergy (Structure-Based)

The physical structure created by agents guides subsequent work. The product
itself is the signal. Grasse observed this in termite mound construction: the
shape of a partially built arch tells the next termite where to place its mud
pellet.

**In Roko**: The codebase itself is sematectonic stigmergy. When a coding
agent writes a function, the function's signature, its location in the module
hierarchy, its documentation, and its test coverage all constitute structural
signals that guide subsequent agents.

| Structural Feature | Signal Conveyed | Agent Response |
|-------------------|----------------|----------------|
| Empty test file | "Tests needed here" | Testing agent writes tests |
| `TODO` comment | "Incomplete implementation" | Coding agent completes it |
| Unused import | "Stale code" | Refactoring agent cleans up |
| Well-documented API | "Ready for integration" | Integration agent uses it |
| Failing CI badge | "Broken build" | Debugging agent investigates |

### Marker-Based Stigmergy (Signal-Based)

Agents deposit explicit signals (markers or pheromones) in the environment.
These signals carry information beyond the physical structure -- they encode
urgency, type, confidence, and decay over time. Ant trail pheromones are the
canonical example.

**In Roko**: Digital pheromones -- typed Signals with explicit `kind`,
`intensity`, `scope`, and exponential decay profiles -- implement marker-based
stigmergy. When an agent detects a threat, it deposits a `Threat` pheromone
that other agents can sense and react to. The pheromone decays over time
(Threat signals have a 2-hour half-life), so stale threats do not permanently
distort behavior.

Key properties of marker-based stigmergy in Roko:

1. **Typed signals**: Different pheromone kinds trigger different agent
   responses (see `05-pheromone-kinds.md`).
2. **Exponential decay**: Signals lose intensity over time, preventing stale
   information from accumulating (see `04-digital-pheromones.md`).
3. **Confirmation reinforcement**: When multiple agents deposit the same
   pheromone type at the same location, the effective half-life extends.
4. **Scope control**: Pheromones can be local (one Substrate), mesh-wide (one
   Collective), or global (public chain) -- see `06-pheromone-scope.md`.

---

## Why Stigmergy, Not Direct Communication?

### Scalability

Direct communication scales as O(N^2) for N agents. Stigmergy scales as
O(N x M) where M is the number of distinct signal types (pheromone kinds),
which is bounded and small. In Roko, M = 7 universal kinds + domain
extensions, so coordination cost grows linearly with the number of agents.

| Agents (N) | Direct Comm (O(N^2)) | Stigmergy (O(N x M), M=10) |
|-----------|--------------------|-----------------------------|
| 5 | 25 channels | 50 read/writes |
| 50 | 2,500 channels | 500 read/writes |
| 500 | 250,000 channels | 5,000 read/writes |
| 5,000 | 25,000,000 channels | 50,000 read/writes |

> "Stigmergy provides a clear separation between the coordination mechanism
> and the individual agents, allowing the system to scale without modifying
> agent behavior." -- [Parunak, H.V.D. "Go to the Ant: Engineering Principles
> from Natural Multi-Agent Systems." *Annals of Operations Research*, 75:69-101,
> 1997]

### Robustness

In a direct communication system, the failure of a key node can paralyze the
entire collective. Stigmergy is inherently robust because the coordination
state is distributed across the environment:

- Previously deposited pheromones persist and continue to guide other agents.
- No other agent needs to be notified of the failure.
- The pheromone field naturally adapts as the failed agent's signals decay.
- New agents joining the collective immediately sense the current state.

This is **graceful degradation** [Bonabeau, E., Dorigo, M. & Theraulaz, G.
*Swarm Intelligence: From Natural to Artificial Systems*. Oxford University
Press, 1999].

### Asynchrony

Stigmergy is inherently asynchronous. The depositing agent and the sensing
agent do not need to be active at the same time. This is essential for Roko's
multi-speed cognitive architecture:

| Cognitive Speed | Tick Duration | Stigmergy Role |
|----------------|---------------|----------------|
| T0 (System-1, fast) | ~15 seconds | Sense ambient pheromones, react to high-intensity signals |
| T1 (System-2, deliberate) | ~60 seconds | Analyze pheromone patterns, deposit new observations |
| T2 (Reflective) | ~5 minutes | Consolidate pheromone history, emit Wisdom Signals |

### Minimal Agent Complexity

Each agent only needs to implement two operations:

1. **Deposit**: Write a Signal to the Substrate with a pheromone kind and
   intensity.
2. **Sense**: Query the Substrate for nearby Signals above a threshold
   intensity.

The agent does not need to know how many other agents exist, what strategies
they follow, or whether they are online.

---

## Stigmergy in Computer Science

### Ant Colony Optimization (ACO)

Dorigo's Ant Colony Optimization is the most prominent computational
application [Dorigo, M., Maniezzo, V. & Colorni, A. "Ant System:
Optimization by a Colony of Cooperating Agents." *IEEE Transactions on
Systems, Man, and Cybernetics B*, 26(1):29-41, 1996]. In ACO:

1. Artificial ants traverse a graph.
2. Each ant deposits pheromone on the edges it traverses, proportional to
   solution quality.
3. Subsequent ants probabilistically prefer edges with higher pheromone
   concentration.
4. Pheromone decays over time (evaporation), preventing premature convergence.
5. The collective converges on high-quality solutions without global view.

### Digital Pheromone Systems

Parunak extended stigmergy to general software agent systems with digital
pheromones [Parunak, H.V.D., Brueckner, S.A. & Sauter, J.A. "Digital
Pheromones for Coordination of Unmanned Vehicles." *E4MAS*, LNCS 3374:246-263,
Springer, 2005].

| Property | Biological | Digital (Roko) |
|----------|-----------|----------------|
| Deposition | Chemical secretion | `Substrate::store(Signal { kind: Threat, ... })` |
| Diffusion | Brownian motion | Mesh gossip propagation |
| Evaporation | Chemical degradation | Exponential decay: `intensity(t) = base x e^(-0.693 x elapsed / tau)` |
| Sensing | Chemoreceptors | `Substrate::query()` with pheromone kind filter |
| Reinforcement | Multiple depositions | Confirmation count extends effective half-life |

---

## The Grossman-Stiglitz Paradox

A fundamental challenge in any information-sharing system: if information is
freely available, no agent has an incentive to incur the cost of producing it
[Grossman, S.J. & Stiglitz, J.E. "On the Impossibility of Informationally
Efficient Markets." *AER*, 70(3):393-408, 1980].

Roko resolves this through natural pheromone properties:

1. **Decay creates scarcity**: Pheromones decay exponentially. Information
   freely sensed yesterday may be gone today.
2. **Confirmation extends value**: Confirming another agent's pheromone extends
   the effective half-life -- a cooperative act.
3. **Reputation tracks contribution**: Agents producing high-quality pheromones
   accumulate reputation, affecting routing priority.
4. **Domain scoping prevents free-riding**: Pheromones are scoped to domains.
   An agent must be active in a domain to sense its pheromones.

---

## The Stigmergic Loop in Roko

The complete loop follows this sequence:

```
Agent A acts -> deposits pheromone Signal to Substrate
    |
Signal propagates (local -> Mesh -> Global based on scope)
    |
Agent B queries Substrate -> senses pheromone
    |
Scorer rates pheromone intensity and relevance
    |
Router selects highest-priority pheromone signal
    |
Agent B acts in response -> deposits its own pheromone Signal
    |
(cycle continues -- emergent coordination without direct communication)
```

This loop is isomorphic to Roko's universal cognitive loop (query -> score ->
route -> compose -> act -> verify -> write -> react), with pheromone Signals
serving as the coordination substrate.

---

## References

- [Grasse 1959] Original observation of stigmergy in termite mound construction
- [Theraulaz & Bonabeau 1999] History and formalization of stigmergy, *Artificial Life*
- [Bonabeau, Dorigo & Theraulaz 1999] *Swarm Intelligence*, Oxford University Press
- [Dorigo, Maniezzo & Colorni 1996] Ant Colony Optimization, *IEEE SMC-B*
- [Parunak 1997] Engineering principles from natural MAS, *Ann. Oper. Res.*
- [Parunak, Brueckner & Sauter 2005] Digital pheromones, *E4MAS*
- [Grossman & Stiglitz 1980] Informationally Efficient Markets, *AER*
- [Holland & Melhuish 1999] Stigmergy in collective robotics, *Artificial Life*
