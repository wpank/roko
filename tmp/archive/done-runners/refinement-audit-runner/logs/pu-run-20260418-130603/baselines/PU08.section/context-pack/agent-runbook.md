# Agent Runbook — Batch 08

Use this when executing any batch from `tmp/docs-parity/08`.

## Mission

Make the chain docs honest about what ships today, what ships only as demos or scaffolds, and what is still Tier-6 frontier work.

## Workflow

1. Read [00-INDEX.md](../00-INDEX.md), [BATCHES.md](../BATCHES.md), and the owning section file.
2. Read [SOURCE-INDEX.md](../SOURCE-INDEX.md) and search the actual code before changing docs.
3. Prefer explicit status, cross-links, and banner fixes over new implementation work.
4. Keep the patch inside the batch scope unless the docs make that impossible.
5. Run the verify commands.
6. If blocked, leave a concrete blocker note with exact file paths and missing dependencies.

## Default Decision Rules

- If a Rust, Solidity, or mirage surface already ships, make it visible before adding more frontier theory.
- If a doc describes a full Korai subsystem but only a proxy, demo, or scaffold exists, make that boundary explicit instead of stretching the partial surface to fit the spec.
- If two similarly named surfaces exist, add disambiguation before considering renames.
- If a task starts requiring actual chain-runtime, Solidity, libp2p, solver, privacy, or payment implementation, record the handoff and stop.

## Required Completion Evidence

Every batch completion note should include:

- files changed,
- commands run,
- whether checks passed,
- what was intentionally deferred,
- and which later batch now has a cleaner seam because of this work.

## Failure Modes To Avoid

- hiding the demo Solidity contracts while claiming nothing Solidity exists,
- treating the mirage scaffold as if it already is the Korai registry/gossip design,
- letting Doc 21 keep a “Built” banner over a proxy-only shipping surface,
- collapsing shipping Rust, shipping Solidity demo, and Phase 2+ frontier into the same status language,
- drifting into Tier-6 implementation work from a docs/status batch.
