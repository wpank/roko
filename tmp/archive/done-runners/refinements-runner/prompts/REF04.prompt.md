# Batch REF04 — Generalize six operators over Datum (Engram|Pulse)

**Refinement source**: `tmp/refinements/04-operators-generalized.md` (injected above under
"Canonical refinement source"). That source is the authoritative proposal;
your job is to propagate its substance into the canonical `docs/` tree.

## Target docs (candidates)

You MAY touch any file under `docs/**` if the refinement applies.
Primary candidates:

- `docs/00-architecture/06-synapse-traits.md` — rewrite around "two mediums, two fabrics, six operators." Soften the "number six is not arbitrary" claim into "six operations + two fabric traits is the complete kernel grammar."
- `docs/00-architecture/08-scorer-gate-router-composer-policy.md` — update each trait's signature (Scorer.score_pulse, Gate.verify_stream, Router.select_pulse, Composer over Datum, Policy over Pulse stream with PolicyOutputs).
- `docs/00-architecture/23-architectural-analysis-improvements.md` — update §2.2 and §3.2 to note the generalization resolves the trait-fit concerns.

## Required outputs

- Every updated doc retains its filename and stable anchor IDs.
- New content is substantive — section-level or full-file rewrites are
  authorised when a doc is deeply misaligned.
- Cross-references to `tmp/refinements/04-operators-generalized.md` appear in each touched file.
- Index docs (`docs/**/INDEX.md`, `docs/INDEX.md`) are updated when a
  new chapter is added or the chapter index needs to reflect the change.
- Glossary updates (if applicable) land in
  `docs/00-architecture/01-naming-and-glossary.md`.


## Required vocabulary (verify)

The verify step greps for: `Datum|two mediums|two fabrics`

At least one changed file must mention it (case-insensitive) before the
batch is considered complete.

## Cross-references

Dependents — downstream refinements assuming this one has propagated
into docs: REF05, REF08, REF10, REF22, REF23

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
- Commit ready with message `refinements(REF04): Generalize six operators over Datum (Engram|Pulse)`.
- Final message lists: files changed, any new files added, any retired
  disclaimers removed, cross-references added, follow-ups identified.
