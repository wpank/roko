# Batch REF08 — Code sketches appendix + inline snippets in trait chapters

**Refinement source**: `tmp/refinements/08-code-sketches.md` (injected above under
"Canonical refinement source"). That source is the authoritative proposal;
your job is to propagate its substance into the canonical `docs/` tree.

## Target docs (candidates)

You MAY touch any file under `docs/**` if the refinement applies.
Primary candidates:

- `docs/00-architecture/06-synapse-traits.md` — short Rust snippets illustrating the Bus trait, Pulse struct, Datum enum.
- `docs/00-architecture/07-substrate-trait.md` — minimal query_similar signature snippet.
- `docs/00-architecture/08-scorer-gate-router-composer-policy.md` — updated signatures for each operator (matches REF04's rewrite; confirm consistency).

## Required outputs

- Every updated doc retains its filename and stable anchor IDs.
- New content is substantive — section-level or full-file rewrites are
  authorised when a doc is deeply misaligned.
- Cross-references to `tmp/refinements/08-code-sketches.md` appear in each touched file.
- Index docs (`docs/**/INDEX.md`, `docs/INDEX.md`) are updated when a
  new chapter is added or the chapter index needs to reflect the change.
- Glossary updates (if applicable) land in
  `docs/00-architecture/01-naming-and-glossary.md`.
- Snippets are illustrative, not normative. Cross-link to `tmp/refinements/08-code-sketches.md` for the full sketch.

## Required vocabulary (verify)

The verify step greps for: ````rust|pub trait|pub struct`

At least one changed file must mention it (case-insensitive) before the
batch is considered complete.

## Cross-references

Dependents — downstream refinements assuming this one has propagated
into docs: (none)

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
- Commit ready with message `refinements(REF08): Code sketches appendix + inline snippets in trait chapters`.
- Final message lists: files changed, any new files added, any retired
  disclaimers removed, cross-references added, follow-ups identified.
