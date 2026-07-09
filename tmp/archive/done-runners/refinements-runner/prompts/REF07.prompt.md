# Batch REF07 — Naming decisions: Pulse/Bus/Topic/Datum across glossary + docs

**Refinement source**: `tmp/refinements/07-naming.md` (injected above under
"Canonical refinement source"). That source is the authoritative proposal;
your job is to propagate its substance into the canonical `docs/` tree.

## Target docs (candidates)

You MAY touch any file under `docs/**` if the refinement applies.
Primary candidates:

- `docs/00-architecture/01-naming-and-glossary.md` — rewrite/extend to canonicalize Pulse, Bus, Topic, TopicFilter, Datum, PulseSource, plus the retired-terms table (Bardo, Golem, Mori, Grimoire, Styx, Clade, Event-as-type-name, EventBus-as-trait, Envelope-as-user-type, Signal-as-ephemeral).
- `docs/00-architecture/INDEX.md` — lead one-liner adopts the canonical vocabulary.

## Required outputs

- Every updated doc retains its filename and stable anchor IDs.
- New content is substantive — section-level or full-file rewrites are
  authorised when a doc is deeply misaligned.
- Cross-references to `tmp/refinements/07-naming.md` appear in each touched file.
- Index docs (`docs/**/INDEX.md`, `docs/INDEX.md`) are updated when a
  new chapter is added or the chapter index needs to reflect the change.
- Glossary updates (if applicable) land in
  `docs/00-architecture/01-naming-and-glossary.md`.
- Retired-terms table must be prominent. Every retired term in `context-pack/02-TERMINOLOGY-TABLE.md` appears here with a Current replacement.

## Required vocabulary (verify)

The verify step greps for: `Pulse|Topic|Datum|Bus`

At least one changed file must mention it (case-insensitive) before the
batch is considered complete.

## Cross-references

Dependents — downstream refinements assuming this one has propagated
into docs: REF34

## Rules

Follow all rules in `context-pack/00-REFINEMENTS-RULES.md`:

- Only touch files under `docs/`. The verify scope gate fails the
  batch otherwise.
- Aggressive edit posture authorised: full-file rewrites allowed when the
  refinement's framing contradicts the existing doc.
- Retired terms (see `context-pack/02-TERMINOLOGY-TABLE.md`) only
  appear in lines explicitly marked retired/deprecated/historical/
  formerly/legacy.
- Substantive edits — no "TODO: rewrite later" placeholders.

## Done when

- Diff gate + scope gate + terminology gate + required-term gate all pass.
- Commit ready with message `refinements(REF07): Naming decisions: Pulse/Bus/Topic/Datum across glossary + docs`.
- Final message lists: files changed, any new files added, any retired
  disclaimers removed, cross-references added, follow-ups identified.
