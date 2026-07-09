# Batch REF10 — Self-learning cybernetic loops across learning + heartbeat

**Refinement source**: `tmp/refinements/10-self-learning-cybernetic-loops.md` (injected above under
"Canonical refinement source"). That source is the authoritative proposal;
your job is to propagate its substance into the canonical `docs/` tree.

## Target docs (candidates)

You MAY touch any file under `docs/**` if the refinement applies.
Primary candidates:

- `docs/05-learning/` — describe the predict-publish-correct loop; per-operator calibration; CalibrationPolicy. New file(s) as needed.
- `docs/00-architecture/11-dual-process-and-active-inference.md` — FEP-as-literal wording; prediction/outcome Pulse framing.
- `docs/00-architecture/16-autocatalytic-and-cybernetics.md` — Bus as feedback nervous system.
- `docs/16-heartbeat/11-active-inference-state-space.md` — prediction.*/outcome.* topic family reference.

## Required outputs

- Every updated doc retains its filename and stable anchor IDs.
- New content is substantive — section-level or full-file rewrites are
  authorised when a doc is deeply misaligned.
- Cross-references to `tmp/refinements/10-self-learning-cybernetic-loops.md` appear in each touched file.
- Index docs (`docs/**/INDEX.md`, `docs/INDEX.md`) are updated when a
  new chapter is added or the chapter index needs to reflect the change.
- Glossary updates (if applicable) land in
  `docs/00-architecture/01-naming-and-glossary.md`.


## Required vocabulary (verify)

The verify step greps for: `prediction.?error|active inference`

At least one changed file must mention it (case-insensitive) before the
batch is considered complete.

## Cross-references

Dependents — downstream refinements assuming this one has propagated
into docs: REF13, REF14

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
- Commit ready with message `refinements(REF10): Self-learning cybernetic loops across learning + heartbeat`.
- Final message lists: files changed, any new files added, any retired
  disclaimers removed, cross-references added, follow-ups identified.
