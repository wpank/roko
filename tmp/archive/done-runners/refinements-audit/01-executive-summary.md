# Executive Summary

## Bottom line

The refinement set has a strong architectural core and is worth using as a
design reference for what should be built next.

Its best parts point toward the right redesign:
- make transport first-class instead of implicit;
- make learning loop through explicit calibration and contradiction handling;
- make StateHub/projections the bridge from runtime to interfaces;
- make extension surfaces modular, local-first, and composable;
- make safety, provenance, and observability explicit architectural contracts.

Its weakest parts are not "too futuristic." They are too totalizing. Several
proposals try to become universal law before they have earned that status.
That would make the redesign harder, more brittle, and more doctrinal than it
needs to be.

## The main verdict

The refinements should not be rolled back. They should be tightened into a
stricter redesign blueprint:

1. Keep the diagnosis.
2. Keep the target-state ambition.
3. Narrow the number of concepts that become universal architecture.
4. Prefer strong seams and contracts over system-wide metaphors.
5. Sequence around a few compounding runtime wins instead of a grand rewrite.

## What is directionally right

### 1. Storage vs transport is a real missing axis

Treating transport as a first-class architectural concern is the right move.
That gives the redesign a cleaner runtime story than a storage-only kernel.

The strongest version of this idea is:
- formalize a shared transport contract;
- let projections, observers, and learning consume the same stream;
- use `Bus` as a real runtime seam;
- avoid making every downstream type system bend around one universal transport
  doctrine.

### 2. Evented calibration is the right learning pattern

The redesign is strongest when learning means:
- prediction or expectation;
- observed outcome;
- contradiction, drift, or surprise;
- updated heuristic or routing choice.

That is concrete, extensible, and inspectable. It is much stronger than making
"active inference" the master explanation for the whole runtime.

### 3. A heuristic layer between episodes and playbooks is valuable

This is one of the strongest ideas in the entire set. It gives Roko a middle
layer between raw run history and distilled playbooks. That is likely to create
real leverage:
- reusable judgment without overfitting to one run;
- inspectable defaults and fallbacks;
- a place to store contradiction and calibration history.

### 4. StateHub as shared projection infrastructure is a strong direction

This is one of the cleanest target-state ideas. A projection layer is the right
way to connect runtime events to CLI, TUI, chat, and web without forcing each
surface to invent its own query and replay semantics.

### 5. Low-power extension tiers are the right platform story

Tier 1/2/3 style extension surfaces are promising:
- prompt packs,
- profile bundles,
- manifest tools,
- MCP adapters.

The platform story is best when it stays local-first, inspectable, and easy to
reason about. It gets worse when it jumps immediately to ecosystem theater.

## What is wrong or overbuilt

### 1. Too many proposals try to become universal law

- all operators as dual-medium polymorphs;
- demurrage as the governing memory economy;
- c-factor as an operational intelligence signal;
- worldview/falsifier/dissonance as structured first-class runtime objects;
- research claims as direct runtime config inputs;
- synergy matrices and moat language as if integration itself proves leverage.

Many of these ideas may be worth exploring, but only some are mature enough to
become redesign primitives.

### 2. New jargon expands faster than decision quality

Some new terms are useful. Too many at once creates conceptual drag.

Most defensible:
- `Pulse`
- `Bus`
- `StateHub`
- `TypedContext`
- `Custody`

Most likely too early or too broad as top-level canon:
- `Datum` as universal operator input
- universal active-inference framing
- worldview/dissonance stack
- demurrage economy as primary memory explanation
- synergy/matrix framing as central architectural lens

### 3. Some proposals are stronger as local mechanisms than as global ideology

Examples:
- `Bus` is strong as transport infrastructure, not as the answer to every
  runtime concern.
- heuristics are strong as a middle layer, not as proof of a full worldview
  algebra.
- domain profiles are strong as bundles and constraints, not yet as total typed
  domain kernels.
- observability is strong as emitted signals and replay, not as a giant unified
  meta-model from day one.

## The highest-value reframing

### Keep as near-term architecture

- Formalize transport as a first-class runtime concern.
- Build one event surface that projections and learning can both consume.
- Treat StateHub as the bridge from transport to interface.
- Keep calibration as the core learning pattern.
- Grow a typed heuristic layer carefully.
- Keep plugins/profile bundles local-first and low-power first.
- Make safety and observability explicit contracts.

### Keep as target-state, but with weaker doctrinal force

- full dual-medium operator algebra;
- seven-step loop as a reference architecture;
- generalized active inference across the runtime;
- demurrage as one future memory-economics option;
- c-factor as an experiment rather than a foundational scalar;
- paper/claim/replication-ledger runtime;
- one typed realtime surface across all UX surfaces;
- five-tier plugin ecosystem with registry and ABI guarantees.

### Move to research backlog or hypothesis framing

- superlinear scaling claims;
- moat claims driven by interaction density;
- prediction-market style or consensus-semantics claims around HDC;
- broad worldview algebra;
- externally meaningful commons/network-effect claims before a real external
  usage loop exists.

## Recommended next moves

1. Rewrite the docs so they read as an intentional redesign blueprint, not an
   everything-at-once manifesto.
2. Reduce the number of terms that become repo-wide canon.
3. Make transport unification the first real architectural follow-up.
4. Define a small projection contract and let StateHub grow from that.
5. Build typed heuristics and contradiction tracking before worldview rhetoric.
6. Keep memory economics experimental until simpler learning loops are working.
7. Build Tier 1/2/3 extension capability before any registry or ABI story.
8. Rework the roadmap around compounding runtime milestones with sharp exit
   criteria.

## The single sentence summary

The refinements are strongest when they drive Roko toward cleaner seams, better
learning loops, better projections, and safer extensibility, and weakest when
they try to lock the redesign into an overgeneralized cybernetic ideology.
