# Agent Runbook — Batch 03

Use this when executing any batch from `tmp/docs-parity/03`.

## Mission

Make composition policy real in runtime execution without widening into mechanism-design research or evaluation-harness work.

## Workflow

1. Read [00-INDEX.md](../00-INDEX.md), [BATCHES.md](../BATCHES.md), and the owning section file.
2. Read [SOURCE-INDEX.md](../SOURCE-INDEX.md) and search the actual code before planning changes.
3. Prefer wiring already-shipped composition surfaces over building new design systems.
4. Keep the patch inside the batch scope unless the code makes that impossible.
5. Run the verify commands.
6. If blocked, leave a concrete blocker note with exact file paths and missing dependencies.

## Default Decision Rules

- If a budget helper exists but runtime does not call it, wire it before inventing a new helper.
- If docs describe a canonical composition path that production no longer uses, prefer hardening the live path instead of reviving the old path blindly.
- If a batch starts needing learning-policy math or evaluation harnesses, record the handoff and stop.
- If a batch touches `orchestrate.rs`, choose the smallest production path that proves the behavior.

## Required Completion Evidence

Every batch completion note should include:

- files changed,
- commands run,
- whether tests passed,
- what was intentionally deferred,
- and what follow-on batch now has a cleaner seam because of this work.

## Failure Modes To Avoid

- leaving hardcoded budget literals in place while claiming budget helpers are wired,
- activating dormant composition code on every path when one production path would prove the contract,
- widening prompt hardening into a full eval framework build,
- treating misleading names like harmless docs drift when they steer later agents toward the wrong system.
