# Agent Runbook — Batch 02

Use this when executing any batch from `tmp/docs-parity/02`.

## Mission

Keep the agent docs honest about the current runtime. The deliverable is a tighter parity record with explicit evidence and deferrals, not a broad implementation push.

## Workflow

1. Read [00-INDEX.md](../00-INDEX.md), [BATCHES.md](../BATCHES.md), and the owning section file.
2. Read [SOURCE-INDEX.md](../SOURCE-INDEX.md) and confirm anchors with `rg` before changing docs.
3. Classify each claim as `wired`, `partial`, `not wired`, or `deferred`.
4. Keep each batch small enough for one agent to finish in about 90 minutes.
5. Prefer documentation updates and concrete handoff notes over speculative code work.
6. Run the narrowest verification needed for what you actually changed.
7. If blocked, leave a concrete blocker note with exact file paths and missing evidence.

## Default Decision Rules

- Do not promote “exists in a crate” to “runtime wired” without a visible call path.
- If `run.rs` and `orchestrate.rs` differ, document the difference precisely instead of flattening it.
- If a batch starts needing verification semantics, learning semantics, executor-state redesign, or a broad cross-crate migration, record the handoff and stop.
- If a claim depends on tool-loop coverage, distinguish shared helper coverage from provider-specific paths.
- If a claim depends on temperament behavior, confirm whether the code uses a typed config or only a free-form string.

## Required Completion Evidence

Every batch completion note should include:

- files changed
- commands run
- the current-state findings that were confirmed
- whether any tests were run
- what was intentionally deferred
- and the next owning batch or roadmap bucket for those deferrals

## Failure Modes To Avoid

- rewriting docs as roadmap promises
- turning a doc/audit batch into a multi-crate implementation batch
- calling speculative features “partial parity” without code evidence
- using whole-workspace verification when narrow evidence is enough
