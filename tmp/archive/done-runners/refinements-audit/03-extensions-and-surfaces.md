# Extensions, Domain Profiles, And User Surfaces

## Extensions and modularity

### What is right

The strongest extensibility idea is simple:
- prefer low-power extension tiers first;
- make discovery local and boring;
- treat manifests and profile bundles as real user-facing leverage;
- clean up the crate graph where obvious seams are mixed today.

This part is good.

### What is overstated

The five-tier story becomes weaker as it moves up the power curve. The redesign
benefit is obvious for:
- prompt packs;
- profile bundles;
- manifest tools;
- MCP adapters.

It becomes much less obviously necessary for:
- stable native ABI commitments;
- WASM host abstractions;
- registry/network-effect strategy.

### Best near-term modularity sequence

1. Build local Tier 1/2/3 loading first.
2. Add a minimal real plugin command surface.
3. Ship one or two concrete external examples.
4. Clean internal crate seams after those extension points are real.
5. Only revisit native ABI and WASM host ideas if real usage demands them.

### What to demote

- registry/network-effect claims;
- stable ABI rhetoric before actual extension pressure exists;
- moat language built on extension surfaces rather than user value.

## Developer UX

### Strong core

The "time to first working agent" goal is good. The layered SDK story is also
good in principle:
- one-liner;
- builder;
- trait-level customization;
- runtime implementation boundary.

### Main problem

The four-layer Rust SDK story is good as a design target, but it risks becoming
too baroque if every layer gets too much bespoke machinery too early.

Better framing:
- desired SDK shape;
- minimum stable path first;
- sharper rules about what must be ergonomic vs what can stay advanced.

### What to keep

- typed errors at the public API surface;
- docs/examples discipline;
- example-driven onboarding;
- cargo-native ergonomics as an aspirational design constraint.

### What to narrow

- runtime-impl rhetoric before the underlying runtime interfaces are stable;
- macros and cargo subcommands unless they remove real friction;
- type and API layering that adds ceremony without reducing cognitive load.

## Domain profiles

### Strong part

Domain profiles are a sensible packaging abstraction:
- they bundle tools, roles, gates, starter heuristics, and defaults;
- they give a practical unit of adoption;
- they work well with low-power extension tiers.

### Risk

The risk is over-formalizing domain abstraction too early:
- `TypedContext` is promising, but could become a premature universal
  substrate;
- `Custody` is valuable, but should stay tightly tied to safety and audit
  semantics instead of becoming a generic catch-all object.

### Better framing

Treat domain profiles as:
- curated bundles first;
- shared typed context and custody expectations second;
- full typed domain kernels later.

## User UX and CLI parity

### What is genuinely useful

- one verb set across surfaces is a good target;
- better first-run onboarding is high leverage;
- diff-first review and visible approvals are good ideas;
- resumption, transcripts, and undo/replay should be first-class.

### What is misleading today

The risk is promising symmetry before the shared interaction model is clear.
The redesign should not aim for parity as aesthetic tidiness. It should aim for
parity where the user mental model is genuinely the same across surfaces.

### Better near-term UX sequence

1. Make one good interactive session surface real.
2. Expose stable session/resume/transcript mechanics.
3. Unify action names only after actual flows exist behind them.
4. Add slash commands and per-hunk review where the data model supports them.
5. Treat the browser as a read-first ops console before a full five-page rich
   app.

## StateHub, realtime, and web UI

### Strong part

StateHub is one of the most promising refinement directions because it gives
the redesign a clean place to put:
- projections;
- subscriptions;
- replayable state transitions;
- interface-friendly read models.

### Main risk

The redesign can overreach here by trying to standardize all transport,
projection, query, auth, replay, and UI semantics in one move. Better to keep
StateHub small and composable:
- projection contract;
- subscription contract;
- snapshot/replay contract;
- auth and tenancy layered around those contracts.

### Rich UX primitives

These are strongest when they are treated as rendering consequences of upstream
contracts.

Prioritize first:
- tool banners;
- gate badges;
- replay milestones;
- heuristic footnotes.

Be careful with:
- raw reasoning streams as a stable product primitive;
- confidence bars without calibrated semantics;
- "everything everywhere" UX promises before projection semantics are settled.

## Recommended rewrite principles for this area

1. Keep plugin and profile docs target-state, but narrower and more local-first.
2. Pull platform rhetoric back toward local-first, low-power extensibility.
3. Treat domain profiles as bundles before treating them as typed domain
   runtimes.
4. Rewrite user UX docs around the ideal user journey and only then decide
   which surface parity is genuinely needed.
5. Recast StateHub as a small projection kernel, not an all-at-once interface
   platform.
6. Make the first web story read-only and ops-facing before promising a richer
   full product surface.
