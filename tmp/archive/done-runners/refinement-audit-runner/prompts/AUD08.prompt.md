# Batch AUD08: Apply naming/term cuts and simpler target architecture

**Audit refs**: 07-naming-and-term-cuts.md (full file), 08-simpler-target-architecture.md
(full file), 01-executive-summary.md (recommended next moves). This is the
final Phase 1 batch -- it applies the audit's recommended vocabulary tightening
and architecture simplification across the docs.

Read these files first:

- `tmp/refinement-audit-runner/context-pack/00-AUDIT-RULES.md`
- `tmp/refinements-audit/07-naming-and-term-cuts.md` (full file -- naming recommendations)
- `tmp/refinements-audit/08-simpler-target-architecture.md` (full file -- simpler mechanisms)
- `tmp/refinements-audit/01-executive-summary.md` (recommended next moves)
- `tmp/refinements-audit/00-MASTER-SUMMARY.md` ("The 5 Things That Are Wrong" section)
- `tmp/refinements-audit/02-foundation-learning.md` (glossary hardening section)
- `docs/00-architecture/01-naming-and-glossary.md` (the canonical glossary -- this is the primary edit target)
- `docs/00-architecture/INDEX.md` (the architecture lead-in)
- `docs/INDEX.md` (top-level, "Current Framing" block)
- `docs/00-architecture/00-vision-and-thesis.md`
- `docs/00-architecture/11-dual-process-and-active-inference.md`
- `docs/00-architecture/14-c-factor-collective-intelligence.md`
- `docs/00-architecture/04-decay-variants.md`

## Task

Apply the audit's naming and terminology recommendations to make the docs
clearer for a new reader. The audit identified terms that are overloaded,
overly doctrinal, or confusing. It also identified simpler mechanisms that
should replace the current heavy abstractions in the target architecture
descriptions. This batch applies those recommendations.

## Current state (evidence)

### Naming issues found by the audit

1. **`TypedContext`** -- opaque jargon. Better: **`Situation`** ("more intuitive
   for users and operators").

2. **`c-factor`** -- requires prior literature knowledge. Better: **`coordination
   health`** in public-facing docs (keep `c-factor` as the internal metric name).

3. **`worldview`** -- doctrinal. Better: **`belief bundle`** in public-facing
   docs.

4. **`falsifier`** -- opaque. Better: **`counterexample check`** in public-facing
   docs.

5. **`demurrage`** -- leads with economic metaphor. Better: **`retention
   pressure`** in public-facing docs.

6. **`Daimon`** -- philosophical overhead. Consider adding "AffectBias" as the
   public-facing alias.

7. **`Datum`** -- too abstract, weakens the two-medium story. Avoid canonizing
   in public docs.

8. **`dashboard`** -- better: **`ops console`** or **`workspace console`** for
   the browser surface.

9. **Profile overloading** -- "profile" is used for both domain profiles and
   deployment shapes. Split: **`domain profile`** (tools/roles/defaults) vs.
   **`runtime shape`** (laptop/server/container/cluster).

10. **`Policy` overloading** -- does both control and learning. The audit
    suggests splitting into `Policy` (control) + `Calibrator` (learning) in
    docs.

### Architecture simplification recommendations

The audit recommends replacing heavy doctrine with simpler mechanisms:

| Heavy doctrine | Simpler mechanism |
|---|---|
| Universal active inference for all operators | Expectation/outcome loops per operator |
| Demurrage-first memory model | Retention tiers + optional pressure |
| Worldview algebra | Contradiction management (typed claims + challenger slots) |
| c-factor control doctrine | Coordination health as observability, then optional actuation |
| Registry-first platform | Capability-gated local plugin lifecycle |
| Raw-event UX | Projection-first UX |

### The "Current Framing" wall-of-text

`docs/INDEX.md` lines 171-214 form a single growing paragraph that was
appended to by successive refinements. Each REF added another clause. The
result is a 150-line block that no developer will read. The doc quality
audit calls this a P1 issue.

## Implementation

### 1. Add public-facing aliases to the glossary

In `docs/00-architecture/01-naming-and-glossary.md`:
- For each term in the naming-cuts table, add a "Public alias" or
  "User-facing name" note in the glossary entry. Do NOT rename the internal
  terms -- just add the clearer alias for docs/UI/CLI contexts.
  Examples:
  - `TypedContext` entry: add "Public alias: **Situation**"
  - `c-factor` entry: add "Public alias: **coordination health**"
  - `worldview` entry: add "Public alias: **belief bundle**"
  - `falsifier` entry: add "Public alias: **counterexample check**"
  - `demurrage` entry: add "Public alias: **retention pressure**"
- Add a new entry for `Calibrator` as the proposed learning-logic split from
  Policy, marked `[planned]`
- Add a new entry for `runtime shape` to distinguish from `domain profile`
- Add a note in the glossary introduction explaining the public-alias
  convention: "Some terms have a public alias used in user-facing docs, CLI
  output, and UI. The internal term remains canonical in code and architecture
  docs."

### 2. Apply simpler framing in architecture narrative docs

In `docs/00-architecture/11-dual-process-and-active-inference.md`:
- Where "every operator is a predictor" doctrine appears, soften to:
  "Operators that make discrete, measurable choices (especially Router) benefit
  most from prediction/outcome loops. Universal operator prediction is a
  target-state aspiration, not a first-pass requirement."
- Where FEP/Friston is cited as the governing theory, add: "The engineering
  mechanism is simpler: expectation/outcome records per operator, with
  calibration updates on mismatch."

In `docs/00-architecture/14-c-factor-collective-intelligence.md`:
- Where c-factor is presented as a control input, reframe:
  "Near-term: c-factor (coordination health) is an observability metric.
  Target-state: once the signal matures, it can optionally drive Policy
  interventions."

In `docs/00-architecture/04-decay-variants.md`:
- Where demurrage is the lead framing, add the simpler alternative:
  "The simpler near-term mechanism is retention tiers (hot/warm/cold) with
  promotion/demotion thresholds and optional retention pressure. Full
  demurrage economics is a target-state extension."

### 3. Collapse the INDEX.md "Current Framing" wall-of-text

In `docs/INDEX.md`:
- Rewrite the "Current Framing" block (lines ~171-214) from a single accretive
  paragraph into a structured format:

```markdown
## Current Framing

> The architecture is organized around:
>
> | Concept | What | Key docs |
> |---|---|---|
> | **Two mediums** | Engram (durable) + Pulse (ephemeral, planned) | [02-engram](00-architecture/02-engram-data-type.md), [02b-pulse](00-architecture/02b-pulse-ephemeral-event.md) |
> | **Two fabrics** | Substrate (storage) + Bus (transport, planned) | [07-substrate](00-architecture/07-substrate-trait.md), [07b-bus](00-architecture/07b-bus-transport-fabric.md) |
> | **Six operators** | Scorer, Gate, Router, Composer, Policy, Substrate | [06-traits](00-architecture/06-synapse-traits.md) |
> | **Learning** | Prediction/outcome loops, bandits, skill library | [05-learning/INDEX](05-learning/INDEX.md) |
> | **HDC** | 10,240-bit fingerprints for similarity | [02-engram](00-architecture/02-engram-data-type.md) |
> | **Safety** | 5-policy chain, contracts, warrants | [11-safety/INDEX](11-safety/INDEX.md) |
>
> For canonical vocabulary, see [Naming and Glossary](00-architecture/01-naming-and-glossary.md).
```

- Remove the per-REF citation sentences. The individual docs already have
  proper cross-references.

### 4. Clean up architecture INDEX lead-in

In `docs/00-architecture/INDEX.md`:
- The opening paragraph (lines 1-28) is also accretive with many REF citations.
  Tighten it to focus on the core story without per-REF citations.
- Keep the "two mediums, two fabrics, six operators" framing but remove the
  inline `tmp/refinements/` references. Those belong in source-tracking
  metadata, not the reader's introduction.

### 5. Apply Profile -> domain profile / runtime shape split in docs

Search the docs tree for places where "profile" is used ambiguously:
- Where it means tools/roles/defaults: clarify as "domain profile"
- Where it means laptop/server/container/cluster: clarify as "runtime shape"
- Focus on `docs/19-deployment/INDEX.md` and `docs/12-interfaces/` where the
  ambiguity is most confusing

## Write scope

Primary:
- `docs/00-architecture/01-naming-and-glossary.md` (public aliases + new entries)
- `docs/INDEX.md` (collapse Current Framing wall-of-text)
- `docs/00-architecture/INDEX.md` (tighten lead-in)

Secondary:
- `docs/00-architecture/11-dual-process-and-active-inference.md`
- `docs/00-architecture/14-c-factor-collective-intelligence.md`
- `docs/00-architecture/04-decay-variants.md`
- `docs/00-architecture/00-vision-and-thesis.md` (only if it has heavy doctrine)

Tertiary (profile split):
- `docs/19-deployment/INDEX.md`
- `docs/12-interfaces/` files that use "profile" ambiguously

## Rules

1. **Add aliases, do not rename.** Internal terms (`TypedContext`, `c-factor`,
   `demurrage`) stay in the code and architecture docs. Public aliases are
   added for user-facing contexts.
2. **Soften doctrine, do not remove it.** The active-inference, demurrage, and
   c-factor ideas are valuable research directions. Reframe them as
   target-state aspirations with simpler near-term mechanisms.
3. **The INDEX rewrite is the highest-impact change.** The wall-of-text is the
   P1 doc-quality issue. A clean table or list makes the docs navigable.
4. **Do not touch files already fully handled by AUD01-AUD07** unless applying
   a naming/framing fix that those batches did not address.
5. **Do not rename types in code snippets.** If a code snippet uses
   `TypedContext`, leave it. Add a prose note that the public alias is
   `Situation`.
6. **Keep `tmp/refinements/` references in source-tracking sections** (like
   "Generation Notes" at doc bottoms) but remove them from reader-facing
   introductions and overviews.
7. **Do not add new sections to docs.** This batch tightens existing content;
   it does not add new design material.

## Done when

- Glossary has public aliases for TypedContext, c-factor, worldview, falsifier,
  demurrage, and Daimon
- Glossary has entries for Calibrator and runtime shape
- Glossary introduction explains the public-alias convention
- `docs/INDEX.md` "Current Framing" is a structured table/list, not a
  wall-of-text
- `docs/00-architecture/INDEX.md` lead-in is clean of per-REF inline citations
- Active inference, c-factor, and demurrage docs have simpler near-term
  mechanisms presented alongside the target-state doctrine
- "Profile" ambiguity is resolved in deployment and interface docs
- No internal type names were renamed
- Final message lists: (a) public aliases added to glossary, (b) the old and
  new shape of the INDEX.md Current Framing block, (c) number of files where
  doctrine was softened, (d) number of files where profile was disambiguated
