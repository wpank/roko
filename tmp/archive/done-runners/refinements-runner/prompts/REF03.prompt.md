# Batch REF03 — Promote Bus to kernel fabric across architecture + subsystem docs

**Refinement source**: `tmp/refinements/03-bus-as-first-class.md` (injected above under
"Canonical refinement source"). That source is the authoritative proposal;
your job is to propagate its substance into the canonical `docs/` tree.

## Target docs (candidates)

You MAY touch any file under `docs/**` if the refinement applies.
Primary candidates:

- `docs/00-architecture/07-substrate-trait.md` — rewrite or extend to introduce Bus as the transport fabric sibling to Substrate.
- Optionally add `docs/00-architecture/07b-bus-transport-fabric.md` as a full companion chapter.
- `docs/00-architecture/12-five-layer-taxonomy.md` — add Bus at L0 alongside Substrate.
- `docs/00-architecture/24-cross-section-integration-map.md` — reframe the EngineEventBus proposal as the Bus trait (now landed / planned).
- `docs/00-architecture/INDEX.md` — add Bus to the two-fabric summary.

## Required outputs

- Every updated doc retains its filename and stable anchor IDs.
- New content is substantive — section-level or full-file rewrites are
  authorised when a doc is deeply misaligned.
- Cross-references to `tmp/refinements/03-bus-as-first-class.md` appear in each touched file.
- Index docs (`docs/**/INDEX.md`, `docs/INDEX.md`) are updated when a
  new chapter is added or the chapter index needs to reflect the change.
- Glossary updates (if applicable) land in
  `docs/00-architecture/01-naming-and-glossary.md`.
- Bus trait signature documented (publish, subscribe, replay_since, current_seq, ring semantics).
- Topic + TopicFilter documented.
- doc-23 layer violation (roko-conductor → roko-learn) explicitly noted as dissolved by Bus topics.

## Required vocabulary (verify)

The verify step greps for: `Bus trait|Bus fabric|Bus primitive|Bus kernel|kernel Bus`

At least one changed file must mention it (case-insensitive) before the
batch is considered complete.

## Cross-references

Dependents — downstream refinements assuming this one has propagated
into docs: REF04, REF05, REF09, REF10, REF17, REF20, REF22, REF24, REF26, REF27

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
- Commit ready with message `refinements(REF03): Promote Bus to kernel fabric across architecture + subsystem docs`.
- Final message lists: files changed, any new files added, any retired
  disclaimers removed, cross-references added, follow-ups identified.
