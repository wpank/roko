# Naming And Term Cuts

This file proposes cleaner names and clearer scope boundaries for the terms most
likely to confuse users or overcomplicate the redesign.

## Naming principles

Prefer names that:
- reveal which lane a concept belongs to;
- are legible to a new engineer without a glossary deep-dive;
- do not force a theoretical worldview into every API;
- can survive contact with CLI, UI, logs, and code.

## Keep

| Term | Decision | Why |
|---|---|---|
| `Engram` | keep | Distinct, stable, and well-matched to durable records. |
| `Pulse` | keep | Best ephemeral transport noun in the set. |
| `Bus` | keep | Standard transport term with low ambiguity. |
| `Substrate` | keep | Still works as the storage fabric term. |
| `Topic` | keep | Familiar routing handle; just validate it. |
| `Projection` | elevate | This should become the main public read-model term. |
| `Session` | elevate | More intuitive user-facing object than many lower-level terms. |

## Keep, but narrow or reposition

| Term | Decision | Recommended role |
|---|---|---|
| `StateHub` | keep, but demote | Runtime host for projections, not the main public abstraction. |
| `Custody` | keep, but narrow | Safety/audit term only, not a general runtime noun. |
| `Domain profile` | keep | Good packaging term for bundles of tools, roles, and defaults. |
| `Runtime shape` | add | Use this for laptop/server/container/cluster to avoid overloading `profile`. |

## Split

| Current term | Proposed split | Why |
|---|---|---|
| `Policy` | `Policy` + `Calibrator` | Control and learning should not be one concept. |
| `Heuristic` | `HeuristicSpec` + `Calibration` | Rule and evidence history evolve differently. |
| `Profile` | `domain profile` + `runtime shape` | One word is doing too much work today. |
| `StateHub` | `Projection` + `StateHub` host | Public concept and implementation host should not be fused. |

## Replace in public-facing docs

| Current term | Better term | Why |
|---|---|---|
| `TypedContext` | `Situation` | More intuitive for users and operators. |
| `c-factor` | `coordination health` | Says what it means without prior literature knowledge. |
| `worldview` | `belief bundle` | Less doctrinal and easier to operationalize. |
| `falsifier` | `counterexample check` | Much clearer as a working mechanism. |
| `demurrage` | `retention pressure` | Avoids leading with an economic metaphor. |
| `BusReceiver` | `Subscription` | Better matches broadcast semantics. |
| `dashboard` | `ops console` or `workspace console` | Better product language for the browser surface. |
| `Daimon` | `AffectBias` | Keeps the meaning, drops the philosophical overhead. |

## Replace or avoid unless there is a hard need

| Term | Recommendation | Why |
|---|---|---|
| `Datum` | avoid canonizing | Too abstract and weakens the two-medium story. |
| `Signal` | avoid reclaiming | Too much historical baggage in this repo. |
| `Event` | avoid as the core noun | Too generic to carry architectural weight. |
| `Message` | avoid as the core noun | Collides with chat semantics and general transport vocabulary. |
| `Envelope` | keep private only | Good as transport scaffolding, bad as the main concept. |
| `marketplace` | avoid near-term | Pushes the docs into ecosystem theater too early. |
| `full parity` | avoid near-term | Encourages fake symmetry instead of shared semantics. |

## Recommended canonical vocabulary

If the redesign is tightened, the public concept stack should read roughly like
this:

- durable record: `Engram`
- live message: `Pulse`
- storage fabric: `Substrate`
- transport fabric: `Bus`
- routing handle: `Topic`
- read model: `Projection`
- projection host: `StateHub`
- user work object: `Session`
- control logic: `Policy`
- learning logic: `Calibrator`
- reusable judgment rule: `HeuristicSpec`
- evidence and performance record: `Calibration`
- user/domain bundle: `domain profile`
- deployment form: `runtime shape`

## Practical consequence

The redesign should reduce, not increase, the number of names a new engineer
must internalize before they can navigate the system. Terms that do not buy a
clear seam should stay private, secondary, or experimental.
