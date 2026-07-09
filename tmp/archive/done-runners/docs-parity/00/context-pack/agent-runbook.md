# Agent Runbook — Batch 00

Use this when editing any file under `tmp/docs-parity/00/`.

## Mission

Keep the architecture parity pack truthful after the audit.

The pack is a documentation maintenance and verification surface. It is not a hidden
implementation roadmap.

## Workflow

1. Read [../00-INDEX.md](../00-INDEX.md), [../BATCHES.md](../BATCHES.md), and the owning file.
2. Check the relevant source docs in `docs/00-architecture/` and the audit notes in
   `tmp/refinements-audit/`.
3. Before changing a claim, ask what artifact justifies it: code, audit, or source-doc intent.
4. Rewrite claims so they clearly say one of:
   - shipped
   - partial
   - planned / target-state
   - deferred
5. Keep edits inside `tmp/docs-parity/00/`.
6. Run text checks for stale wording and the runner syntax check.
7. Keep the batch realistic for a single-agent editorial pass; defer anything that starts reading
   like code work or quarter-scale planning.

## Decision Rules

- If code exists, describe it concretely.
- If code does not exist, do not speak in present tense.
- If a concept appears only in docs, move it into a future-work sentence.
- If a meta doc reads like current architecture but is really planning material, rewrite it as a
  planning reference.
- If a fix depends on new implementation work, defer it and record the handoff.
- If a fact conflicts with the audit pack for this run, use the audit pack.
- If a roadmap chapter reads like a staffed quarter plan, reduce it to dependency ordering plus a
  planning artifact note.

## Required Facts For This Batch

- Workspace baseline: 36 workspace members
- Audit breakdown: 32 crates + 3 apps + 1 test crate
- Total Rust LOC: 322,088
- `roko-serve`: wired, 200+ routes
- TUI: wired, ~58K LOC
- Event bus: exactly two live `RokoEvent` variants
- `Pulse`, `Datum`, `Demurrage`, `Worldview`, `Custody`: zero-code concepts
- Engram is canonical for parity purposes; old naming survives only as legacy residue

## Failure Modes To Avoid

- Turning a docs refresh into a code implementation backlog
- Calling speculative concepts `already wired`
- Preserving stale counts because they were copied from earlier context
- Treating source anchors as proof rather than spot checks
- Leaving parity files inconsistent with one another
- Framing the roadmap as a live 5-7 engineer plan
- Leaving literal stale serve/TUI status phrases or old repo-scale numbers in the final pack
