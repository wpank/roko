# Agent Runbook — Batch 06

Use this when executing any batch from `tmp/docs-parity/06`.

## Mission

Make the neuro runtime use more of the neuro infrastructure it already ships, and make the contract between runtime behavior and docs substantially more honest.

## Workflow

1. Read [00-INDEX.md](../00-INDEX.md), [BATCHES.md](../BATCHES.md), and the owning section file.
2. Read [SOURCE-INDEX.md](../SOURCE-INDEX.md) and search the actual code before planning changes.
3. Prefer wiring already-shipped neuro surfaces over building new research subsystems.
4. Keep the patch inside the batch scope unless the code makes that impossible.
5. Run the verify commands.
6. If blocked, leave a concrete blocker note with exact file paths and missing dependencies.

## Default Decision Rules

- If `roko-neuro` already has the richer surface and runtime is bypassing it, activate that before adding a second system.
- If docs describe a large transfer, backup, or HDC layer that is not real, make the boundary explicit before trying to build the whole thing.
- If a batch touches `orchestrate.rs`, choose the smallest production path that proves the contract.
- If a task starts requiring network protocols, token economics, or research-heavy HDC systems, record the handoff and stop.

## Required Completion Evidence

Every batch completion note should include:

- files changed,
- commands run,
- whether tests passed,
- what was intentionally deferred,
- and which later batch now has a cleaner seam because of this work.

## Failure Modes To Avoid

- leaving `ContextAssembler` library-only while implying the full neuro retrieval pipeline is live,
- adding speculative thresholds or APIs without making the live query contract clearer,
- treating Dreams-side cross-domain hypotheses as proof that doc-08 resonance transfer exists,
- expanding source / backup work into network or token-governed systems,
- leaving stale `roko-golem`, frontier, or “not implemented” claims uncorrected after the runtime facts are known.
