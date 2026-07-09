# Batch REF05 — Retell universal cognitive loop as 7 steps with broadcast

**Refinement source**: `tmp/refinements/05-loop-retold.md` (injected above under
"Canonical refinement source"). That source is the authoritative proposal;
your job is to propagate its substance into the canonical `docs/` tree.

## Target docs (candidates)

You MAY touch any file under `docs/**` if the refinement applies.
Primary candidates:

- `docs/00-architecture/09-universal-cognitive-loop.md` — rewrite from 9 steps to 7; PERSIST and BROADCAST co-equal at step 6; cross-cuts not loop steps.
- `docs/16-heartbeat/00-coala-9-step-pipeline.md` — update to reference the revised loop.
- `docs/16-heartbeat/01-universal-loop-mapping.md` — align mappings to the 7-step.
- `docs/00-architecture/13-cognitive-cross-cuts.md` — tighten wording that cross-cuts inject into specific operators (Neuro at SENSE/COMPOSE; Daimon at ASSESS/ACT; Dreams on its own Delta loop).

## Required outputs

- Every updated doc retains its filename and stable anchor IDs.
- New content is substantive — section-level or full-file rewrites are
  authorised when a doc is deeply misaligned.
- Cross-references to `tmp/refinements/05-loop-retold.md` appear in each touched file.
- Index docs (`docs/**/INDEX.md`, `docs/INDEX.md`) are updated when a
  new chapter is added or the chapter index needs to reflect the change.
- Glossary updates (if applicable) land in
  `docs/00-architecture/01-naming-and-glossary.md`.
- The loop rename 'PERCEIVE' → 'SENSE' and 'META-COGNIZE' removal should be consistently applied.

## Required vocabulary (verify)

The verify step greps for: `seven.?step|SENSE|BROADCAST`

At least one changed file must mention it (case-insensitive) before the
batch is considered complete.

## Cross-references

Dependents — downstream refinements assuming this one has propagated
into docs: REF08, REF09, REF10, REF26

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
- Commit ready with message `refinements(REF05): Retell universal cognitive loop as 7 steps with broadcast`.
- Final message lists: files changed, any new files added, any retired
  disclaimers removed, cross-references added, follow-ups identified.
