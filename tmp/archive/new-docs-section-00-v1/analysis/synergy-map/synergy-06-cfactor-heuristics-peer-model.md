# S6 — c-factor × Heuristics → Peer-model learning

> The system can model other agents the same way it models the world: as a set of predictions to
> calibrate. Peer-model accuracy becomes part of the collective intelligence signal, making
> social perception measurable and coordination less wasteful.

**Status**: Analysis — target-state synergy  
**Primitives involved**: P8 c-factor × P7 Heuristics  
**Reality check**: P7 Heuristics are Scaffold; P8 c-factor is partially built. Peer-model
tracking as a distinct capability is target-state.  
**Last reviewed**: 2026-04-19

---

## Primitives Involved

| Primitive | Role in this synergy |
|---|---|
| [P8 c-factor](../../reference/) | Collective intelligence signal; extended here to include peer-model accuracy as one of the diversity dimensions it tracks |
| [P7 Heuristics + falsifiers](../../subsystems/) | The mechanism for recording predictions and calibrating them against outcomes — applied here to predictions *about other agents* rather than predictions about the world |

---

## What the Synergy Unlocks

### Why peer modeling matters

In a multi-agent system, an agent's effectiveness depends not just on how well it models the
world but also on how well it models its peers. An agent that cannot predict how a collaborator
will respond will over-explain, under-coordinate, and waste synchronization cycles.

Standard multi-agent systems handle this through fixed protocols (agent A always does X, agent B
always does Y) or through explicit communication channels that require agents to announce
their state. Both are fragile: fixed protocols break when roles evolve; explicit announcements
add communication overhead.

### How it works

1. Each agent maintains a set of *peer-model heuristics*: predictions about other agents'
   likely responses to given situations ("agent B tends to escalate when confidence < 0.4").
2. These peer-model heuristics are stored in the same Heuristics store as world-model heuristics,
   but tagged with a `peer_model` kind and a target agent identifier.
3. When an interaction with a peer produces an outcome, the same calibration loop (S2) applies:
   an outcome Pulse fires, the falsifier on the relevant peer-model heuristic is evaluated, and
   confidence is updated.
4. Peer-model accuracy — the average confidence-weighted hit rate across all peer-model
   heuristics — becomes a component of the c-factor calculation. Agents that understand their
   peers better contribute more reliably to collective intelligence.
5. When peer-model accuracy for a given agent drops below threshold, c-factor signals that the
   relevant pair should be split or that a re-calibration session is needed.

The result: social perception is measurable. Agents that understand each other's likely responses
can coordinate with less friction and less wasted synchronization.

### The key insight

The system does not need to add a new primitive for peer modeling. It reuses the Heuristics
+ falsifiers mechanism with a different subject. The subject of a world-model heuristic is the
environment; the subject of a peer-model heuristic is another agent. The calibration machinery
is identical. The only addition is the `peer_model` kind tag and the routing to the c-factor
computation.

---

## What Flows

```
Peer-model heuristic creation:
  Observed pattern in agent B's behavior
  → Heuristics.store(predicate="B escalates when confidence<0.4",
                     kind=PeerModel, target=AgentB.id,
                     confidence=initial, falsifier=...)

Calibration (via S2):
  Interaction with AgentB produces outcome
  → Calibration subscriber evaluates falsifier
  → update peer-model heuristic confidence

c-factor update:
  c-factor.compute_peer_model_accuracy(agent_id) → score
  c-factor.update(peer_model_component=score)
  if score < threshold: emit PolicyPulse(kind=PeerDivergence, target_pair=...)
```

---

## Invariants

1. Peer-model heuristics are tagged with the target agent's identity. A heuristic about agent B
   is never applied to agent C.
2. Peer-model accuracy contributes to c-factor as a **component**, not as the whole signal.
   The c-factor score always integrates multiple diversity dimensions.
3. A peer-model heuristic is never used as a gate condition — it is advisory, not gatekeeping.
   Decisions based on peer predictions are suggestions to the coordination layer, not hard
   constraints.
4. Peer-model heuristics age under the same demurrage rules as world-model heuristics (S8).
   Stale peer models do not persist indefinitely.

---

## Failure Modes

| Failure | Mechanism | Mitigation |
|---|---|---|
| Peer-model lock-in | A peer-model heuristic becomes so confident that the agent stops treating the peer as a live responder | Cap peer-model confidence below 1.0; add small uncertainty floor |
| Adversarial calibration | A peer deliberately behaves unusually to corrupt the modeling agent's heuristics | Treat peer-model heuristics as lower-stakes than world-model heuristics; limit their influence on gatekeeping |
| Asymmetric models | Agent A models agent B well but B has poor models of A; coordination degrades in one direction | Surface peer-model accuracy per direction in c-factor; flag one-sided relationships |
| Heuristic proliferation | Many agents modeling many peers creates an O(n²) heuristic store | Limit peer-model heuristics to frequent interaction pairs; prune by interaction recency |

---

## Relationship to Other Synergies

- **S2** (Heuristics × Pulse × Bus): S6 uses the same calibration loop as S2. The only
  difference is the subject of the prediction.
- **S3** (c-factor × Bus × HDC): S3 uses c-factor to detect diversity in system outputs; S6
  feeds peer-model accuracy as one component of the c-factor computation.
- **S8** (Demurrage × Heuristics × calibration): S8's confidence-softening applies equally to
  peer-model heuristics — stale peer models decay as expected.

---

## Today vs. Planned

**Today**: Heuristics can be stored in Substrate. c-factor is partially built but not wired to
a heuristic-accuracy computation. No peer-model kind tag exists.

**Planned**: Heuristics gain a `PeerModel` kind variant. The calibration loop routes peer-model
outcome Pulses correctly. c-factor gains a peer-model-accuracy component in its score computation.

---

## Cross-References

- [`analysis/synergy-map/synergy-02-heuristics-pulse-bus.md`](synergy-02-heuristics-pulse-bus.md) — S2: the calibration loop S6 reuses
- [`analysis/synergy-map/synergy-03-cfactor-bus-hdc.md`](synergy-03-cfactor-bus-hdc.md) — S3: c-factor regulatory use-case
- [`analysis/synergy-map/synergy-08-demurrage-heuristic-relearning.md`](synergy-08-demurrage-heuristic-relearning.md) — S8: confidence aging for peer models
- [`analysis/readiness-audit/subsystem-learning.md`](../readiness-audit/subsystem-learning.md) — learning subsystem gaps
- [`analysis/synergy-map/99-master-synergy-table.md`](99-master-synergy-table.md) — synergy index

---

## Open Questions

- Should peer-model heuristics be shared between agents (a "public" model of agent B that all
  agents can read) or private to each agent (each agent maintains its own model of B)?
- How are peer-model heuristics scoped across sessions? If agent B's behavior changes across
  sessions, does the model reset or decay more aggressively?
- What is the right confidence initialization for a new peer-model heuristic — 0.5 (neutral)
  or a lower value (uncertain) that must be earned up?
