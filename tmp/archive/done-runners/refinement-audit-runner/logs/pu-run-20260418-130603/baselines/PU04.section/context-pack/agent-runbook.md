# Agent Runbook — Batch 04

Use this when executing any batch from `tmp/docs-parity/04`.

## Mission

Make the verification runtime match the shipped verification code more closely without widening into reward-model research, autonomous evaluator-agent design, or replay-product work.

## Workflow

1. Read [00-INDEX.md](../00-INDEX.md), [BATCHES.md](../BATCHES.md), and the owning section file.
2. Read [SOURCE-INDEX.md](../SOURCE-INDEX.md) and search the actual code before planning changes.
3. Prefer wiring already-shipped verification surfaces over building new theory-heavy subsystems.
4. Keep the patch inside the batch scope unless the code makes that impossible.
5. Run the verify commands.
6. If blocked, leave a concrete blocker note with exact file paths and missing dependencies.

## Default Decision Rules

- If a gate / selector / feedback helper exists but runtime does not call it, wire it before inventing a replacement.
- If a doc describes runtime behavior that `orchestrate.rs` does not actually perform, prefer fixing the live path over strengthening the doc language alone.
- If a batch touches `orchestrate.rs`, choose the smallest production path that proves the contract.
- If a task starts requiring reward models, autonomous test-generation agents, or replay analytics, record the handoff and stop.

## Required Completion Evidence

Every batch completion note should include:

- files changed,
- commands run,
- whether tests passed,
- what was intentionally deferred,
- and which later batch now has a cleaner seam because of this work.

## Failure Modes To Avoid

- leaving the runtime with two conflicting notions of “rung”,
- claiming higher-rung verification is live when the required runtime inputs still do not exist,
- training adaptive thresholds without making them affect runtime behavior,
- routing raw gate stderr into AutoFix while claiming structured feedback is wired,
- widening signal-contract hardening into a full replay-analysis system.
