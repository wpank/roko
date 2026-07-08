# Layer Taxonomy Rationale

> Why five layers, what alternatives were considered, and what was rejected.

**Status**: Shipping (rationale is historical; not subject to further change)
**Last reviewed**: 2026-04-19

---

## Why Layering at All?

Without explicit layer rules, Roko's dependency graph would grow organically. In
practice, "organic" means: whatever is convenient at the time. The result is a graph
with cycles, tight coupling between unrelated concerns, and implementations that cannot
be tested in isolation.

Explicit layering is a structural discipline that makes the dep graph a DAG.

---

## Why Five?

The five layers emerged from asking: "what are the genuinely distinct kinds of code in
this system?"

| Kind of code | Layer |
|---|---|
| "I abstract the platform" | L0 |
| "I define the language" | L1 |
| "I implement the language" | L2 |
| "I wire implementations into a running agent" | L3 |
| "I expose the agent to the outside world" | L4 |

Three layers were tried first (core / agent / entry). The problem was that "agent"
became a catch-all for both implementations and wiring, making it impossible to test
implementations without wiring infrastructure.

Six layers were also considered. The proposed sixth layer ("subsystem") would be
between L2 and L3, for the Neuro/Daimon/Dreams cross-cuts. This was rejected because
cross-cuts implement L1 traits and are best understood as L2 implementations that
happen to be injected by L3 — not a qualitatively different kind of code.

---

## Alternatives Considered

### Modules within a monolith

All code in one crate, with modules for each concern. Rejected because:
- Cannot enforce dependency rules (modules within a crate can freely import each other)
- Compile times increase linearly with crate size
- No clear artifact boundary for packaging and deployment

### Feature flags for platform abstraction

Instead of `roko-runtime` (L0), use feature flags to gate platform code.
Rejected because:
- Feature flags are composable in unexpected ways; the combinatorial space is large
- A separate crate makes the platform boundary explicit and auditable

### "Plugin" architecture without layer rules

A runtime plugin system where any crate can register itself as a scorer, router, etc.
Rejected because:
- Plugins bypass the static dep graph; the layer-check CI tool cannot verify them
- Runtime registration makes the system harder to reason about statically

---

## See also

- [Overview](00-overview.md) — the five-layer structure
- [Dependency Rules](06-dependency-rules.md) — the rule the layers enforce
- [Adding a Layer](09-adding-a-layer.md) — the bar for extension
