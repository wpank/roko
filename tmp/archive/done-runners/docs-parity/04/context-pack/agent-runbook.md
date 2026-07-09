# Agent Runbook — Batch 04

Use this when refreshing the verification parity materials.

## Mission

Keep `tmp/docs-parity/04/` aligned with the audit:

- document shipped verification core honestly
- narrow partials
- defer research-heavy material explicitly

## Workflow

1. Read [00-INDEX.md](../00-INDEX.md), [SOURCE-INDEX.md](../SOURCE-INDEX.md), and the owned section file.
2. Prefer the live runtime path over older parity assumptions.
3. Edit only files under `tmp/docs-parity/04/`.
4. Replace overscoped backlog language with `shipped`, `partial`, or `DEFERRED` wording.
5. Run `bash -n tmp/docs-parity/04/run-docs-parity.sh` before closing the batch.

## Default Decisions

- If core verification behavior is live, describe it in present tense.
- If a module exists but its wider runtime role is limited, call it a shipped foundation with narrowed scope.
- If a concept is research-heavy and not live, mark it `DEFERRED`.
- If a line reference is stale, refresh it instead of carrying it forward.

## Failure Modes To Avoid

- treating `A-D` like a missing implementation program
- describing `E-F` as active implementation scope
- collapsing the live verdict-signal path into the deferred forensic system
- centering stale `orchestrate.rs` anchors instead of current code
