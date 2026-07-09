# Agent Runbook — Batch 05

Use this when executing any batch from `tmp/docs-parity/05`.

## Mission

Make the learning runtime use more of the learning infrastructure it already ships, and make the contract between runtime behavior and docs substantially more honest.

## Workflow

1. Read [00-INDEX.md](../00-INDEX.md), [BATCHES.md](../BATCHES.md), and the owning section file.
2. Read [SOURCE-INDEX.md](../SOURCE-INDEX.md) and search the actual code before planning changes.
3. Prefer wiring already-shipped learning surfaces over building new learning theory.
4. Keep the patch inside the batch scope unless the code makes that impossible.
5. Run the verify commands.
6. If blocked, leave a concrete blocker note with exact file paths and missing dependencies.

## Default Decision Rules

- If the learning library already supports richer matching or reporting than runtime uses, activate that before inventing a new subsystem.
- If two plausible data paths exist, pick one canonical source of truth and reduce ambiguity.
- If a batch touches `orchestrate.rs`, choose the smallest production path that proves the contract.
- If a task starts requiring routing research, new storage architecture, or governance / constitutional systems, record the handoff and stop.

## Required Completion Evidence

Every batch completion note should include:

- files changed,
- commands run,
- whether tests passed,
- what was intentionally deferred,
- and which later batch now has a cleaner seam because of this work.

## Failure Modes To Avoid

- leaving production on role-only learned-context matching while claiming rules are fully metadata-aware,
- keeping regression slices or calibration metrics as doc claims without runtime evidence,
- creating two competing calibration data paths,
- leaving large dead learning modules ambiguous,
- widening docs-honesty work into a speculative architecture build.
