# Agent Runbook - Batch 07 Docs Refresh

Use this when editing the owned docs package in `tmp/docs-parity/07/`.

## Mission

Refresh the conductor parity context so it accurately describes the current
runtime and clearly separates:

- live conductor behavior,
- adjacent library surfaces,
- and deferred theory or implementation work.

This is a docs-only pass. Do not treat it as a license to edit Rust code.

## Workflow

1. Read the current files in `tmp/docs-parity/07/` before editing.
2. Use source reads to confirm live status in `crates/roko-conductor/`,
   `crates/roko-cli/`, and `crates/roko-learn/`.
3. Use `00-INDEX.md` and `BATCHES.md` as the current docs-refresh contract.
4. Edit only the owned files in this package.
5. Verify counts, terminology, and deferrals with `rg`, `find`, and
   `wc`.
6. If a finding needs Rust changes, record the handoff instead of
   expanding the scope.

## Default Decision Rules

- If the source shows a surface is live, say so plainly.
- If the source shows a library surface exists but is not the target of
  this refresh, describe it carefully and stop there.
- If a topic is theory-heavy or federated, mark it deferred.
- If a task requires edits outside the owned docs files, hand it off.

## Required Completion Evidence

Every completion note should include:

- files changed,
- commands run,
- whether the scope stayed docs-only,
- and what was intentionally deferred.

## Failure Modes To Avoid

- repeating the old "go wire the runtime" posture in this package,
- understating the live conductor core (`10` watchers, breaker,
  diagnosis),
- leaving `RoutingBias` or retry-path `ConductorBandit` out of the status
  picture,
- turning theory/federation sections into active batch work,
- implying tests or cargo verification were required for a docs-only edit
  when no code changed.
