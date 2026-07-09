# Safety, Observability, Glossary, Synergy, And Roadmap

## Safety and provenance

### What is strong

The safety refinements are directionally strong because they treat safety as a
system contract instead of a grab-bag:
- authorization;
- sandboxes;
- taint;
- provenance;
- attestation;
- custody;
- tenant and identity concerns.

This should remain.

### What needs narrowing

The main risk is policy overdesign:
- auth and tenancy can sprawl into a full platform program too early;
- plugin sandbox tiers can get more elaborate than the extension model needs;
- custody and typed-context can become heavy universal requirements instead of
  targeted control points.

### Best rewrite

Describe the safety spine as:
- a small set of non-negotiable contracts;
- explicit trust boundaries;
- progressive hardening layers added as extension and deployment complexity
  grows.

## Observability and telemetry

### What is strong

Treating observability as a first-class part of the product is right. The
strongest part of the observability work is the insistence that Roko-specific
signals matter, not just generic CPU/memory/process metrics.

### What is overstated

The telemetry story is strongest when it stays attached to operator action:
- what happened;
- why it happened;
- what it cost;
- what changed;
- what needs intervention.

### What to keep

- structured logs;
- explicit metrics surface;
- traces around operator boundaries;
- replay as part of observability;
- cost visibility as a first-class operator concern.

### What to narrow

- Roko-specific metrics that depend on speculative primitives;
- attempts to unify every signal surface into one meta-model immediately;
- telemetry language that outruns clear actionability.

## Glossary and naming

### What is useful

One canonical glossary is valuable. Retiring stale names is also valuable.

### Main issue

The glossary currently acts like an authority amplifier for target-state ideas.
That makes it more dangerous than helpful in places.

### Better structure

Split entries into three categories:
- current canonical term;
- proposed target-state term;
- historical or retired term.

Only current canonical terms should be written as repo-wide settled truth.

## Synergy framing

### What is worth keeping

The synergy docs are useful as internal integration maps. They help show where
proposals reinforce each other and where a feature is isolated or premature.

### What is not worth keeping as-is

The synergy and moat rhetoric often drifts into architecture theater:
- interaction density becomes implied strategic proof;
- integration webs are treated as evidence of defensibility;
- the matrix format creates a false sense of inevitability.

### Better framing

Keep synergy as:
- internal coherence tooling,
- dependency visualization,
- gap-finding aid.

Do not let it become:
- proof of moat,
- proof of maturity,
- substitute for implementation evidence.

## Roadmap and sequencing

### Strong part

The roadmap at least tries to impose dependency order and risk budgeting. That
is an improvement over a flat wishlist.

### Weak part

The roadmap is still too optimistic and too architecture-led. It tries to
advance too many deep programs at once:
- kernel transport refactor;
- learning/memory refactor;
- plugin SPI;
- StateHub projections;
- unified UX surfaces;
- domain packaging;
- safety spine;
- multi-tenant hardening.

That is too much for one redesign stream. The target-state needs fewer
concurrent bets and sharper dependency discipline.

### Better sequencing

1. Transport unification and event-surface cleanup.
2. Projection/read-model cleanup for current dashboard and operator surfaces.
3. Typed heuristics and contradiction tracking.
4. Local-first extension bundles and minimal plugin lifecycle.
5. Better session UX and a thinner first web surface.
6. Only then: stronger memory economics, domain packaging maturity, and broader
   distributed/runtime claims.

### What should move out of near-term quarter language

- full demurrage rollout as a governing memory model;
- c-factor actuation;
- five-tier plugin maturity;
- full browser UX parity;
- broad multi-tenant and OIDC assumptions;
- chain/mesh consequences downstream of core runtime seams that still need to
  be earned.

## Recommended rewrite principles for this area

1. Keep safety as a spine, but keep the contract set small and enforceable.
2. Keep observability explicit, but tie metrics to clear operator actions.
3. Reduce glossary authority over proposed terms.
4. Recast synergy as internal architecture tooling, not strategic proof.
5. Rewrite the roadmap around fewer concurrent bets and sharper dependency
   checkpoints.
