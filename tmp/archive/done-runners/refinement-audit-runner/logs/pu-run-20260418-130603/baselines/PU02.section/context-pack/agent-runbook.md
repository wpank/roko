# Agent Runbook — Batch 02

Use this when executing any batch from `tmp/docs-parity/02`.

## Mission

Make the agent layer more self-consistent and more runtime-real without relying on hidden project context.

## Workflow

1. Read [00-INDEX.md](../00-INDEX.md), [BATCHES.md](../BATCHES.md), and the owning section file.
2. Read [SOURCE-INDEX.md](../SOURCE-INDEX.md) and search the actual code before planning changes.
3. Prefer activating existing agent infrastructure over building new agent frameworks.
4. Keep the patch inside the batch scope unless the code makes that impossible.
5. Run the verify commands.
6. If blocked, leave a concrete blocker note with exact file paths and missing dependencies.

## Default Decision Rules

- If a stronger runtime path already exists in `run.rs` but not in `orchestrate.rs`, prefer reusing it.
- If one type exists in two places, collapse to one owner before adding features on top.
- If a batch starts needing verification semantics, learning semantics, or executor-state redesign, record the handoff and stop.
- If a batch touches `orchestrate.rs`, choose the smallest production path that proves the behavior.

## Required Completion Evidence

Every batch completion note should include:

- files changed,
- commands run,
- whether tests passed,
- what was intentionally deferred,
- and what follow-on batch now has a cleaner seam because of this work.

## Failure Modes To Avoid

- leaving duplicate type ownership in place while layering more logic on top,
- building a second tool-execution path instead of using `ToolDispatcher` / `ToolLoop`,
- inventing a temperament system with no typed config contract,
- widening one agent batch into orchestration or learning architecture work.
