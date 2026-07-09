# Agent Runbook — PU03 Audit Refresh

Use this when executing any batch from `tmp/docs-parity/03`.

## Mission

Audit and tighten the live composition path without widening into learning theory, eval harnesses, distributed context design, or mechanism-design research.

## Time Box

Assume one agent and about 90 minutes per batch.

- Prefer one wired-path fix plus verification.
- If the task is larger, leave a precise handoff instead of half-finishing a redesign.

## Workflow

1. Read [00-INDEX.md](../00-INDEX.md), [BATCHES.md](../BATCHES.md), and the owning section file.
2. Read [SOURCE-INDEX.md](../SOURCE-INDEX.md) plus this context pack.
3. Trace the live path in code before planning edits.
4. Keep the patch inside the batch scope and inside your owned files unless the batch explicitly requires code changes elsewhere.
5. Run the targeted verify command plus any supporting `rg` checks.
6. If blocked, leave a blocker note with exact files, symbols, and the missing dependency.

## Default Decision Rules

- Start from `crates/roko-cli/src/orchestrate.rs`, not from the oldest design doc.
- Prefer the shared prompt path in `crates/roko-cli/src/prompting.rs` and `crates/roko-compose/src/role_prompts.rs`.
- Treat `ContextProvider` -> `roko-neuro::ContextAssembler` as the default context path unless code proves otherwise.
- If a helper path is only partially wired, describe it honestly before trying to activate it.
- If the task starts requiring VCG, MVT, distributed-context architecture, or eval-theory work, record the seam and stop.

## Required Completion Evidence

Every batch completion note should include:

- files changed,
- commands run,
- whether tests passed,
- the exact wired path checked,
- what was intentionally deferred,
- and which later batch or pass now owns the deferred work.

## Failure Modes To Avoid

- claiming a helper library is live without tracing a runtime call site,
- reviving dormant composition helpers when the shipped path only needs documentation or a small fix,
- widening a prompt audit into full enrichment, distributed-context, or scoring-theory redesign,
- using workspace-wide verification when targeted package tests would prove the contract faster,
- hiding inline role fallback strings or shared-template reuse behind vague wording.
