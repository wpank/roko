# Batch REF09 — Phase-2 implications: chain / dreams / mesh / coordination

**Refinement source**: `tmp/refinements/09-phase-2-implications.md` (injected above under
"Canonical refinement source"). That source is the authoritative proposal;
your job is to propagate its substance into the canonical `docs/` tree.

## Target docs (candidates)

You MAY touch any file under `docs/**` if the refinement applies.
Primary candidates:

- `docs/08-chain/` — introduce ChainBus vs ChainSubstrate split.
- `docs/10-dreams/` — document Substrate scan + Bus-subscription input.
- `docs/13-coordination/` — stigmergy as pheromone Engram + mesh.pheromone Pulse.
- `docs/16-heartbeat/` — HeartbeatPolicy publishes heartbeat.{gamma,theta,delta}.tick Pulses.
- `docs/00-architecture/24-cross-section-integration-map.md` — Bus-based integration supersedes prior proposals.

## Required outputs

- Every updated doc retains its filename and stable anchor IDs.
- New content is substantive — section-level or full-file rewrites are
  authorised when a doc is deeply misaligned.
- Cross-references to `tmp/refinements/09-phase-2-implications.md` appear in each touched file.
- Index docs (`docs/**/INDEX.md`, `docs/INDEX.md`) are updated when a
  new chapter is added or the chapter index needs to reflect the change.
- Glossary updates (if applicable) land in
  `docs/00-architecture/01-naming-and-glossary.md`.
- Each touched subdir's INDEX.md should summarize the new framing briefly.

## Required vocabulary (verify)

The verify step greps for: `ChainBus|two.?fabric`

At least one changed file must mention it (case-insensitive) before the
batch is considered complete.

## Cross-references

Dependents — downstream refinements assuming this one has propagated
into docs: REF32, REF33

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
- Commit ready with message `refinements(REF09): Phase-2 implications: chain / dreams / mesh / coordination`.
- Final message lists: files changed, any new files added, any retired
  disclaimers removed, cross-references added, follow-ups identified.
