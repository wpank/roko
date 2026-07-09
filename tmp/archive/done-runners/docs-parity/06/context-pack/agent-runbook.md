# Agent Runbook — Batch 06

## Mission

Refresh the PU06 parity docs so they describe current neuro/HDC reality without
carrying forward the audit's overscope.

## Workflow

1. Read the current `docs/06-neuro/` pages and the audit notes.
2. Rewrite only files under `tmp/docs-parity/06/`.
3. Separate `shipping`, `partial`, and `deferred` claims.
4. Keep HDC-on-Engram as the top follow-up item.
5. Finish by checking `bash -n tmp/docs-parity/06/run-docs-parity.sh`.

Docs-only PASS is valid for PU06 when the batch leaves verified status
corrections and explicit deferrals.

## Default Decision Rules

- If code exists, describe it in present tense.
- If code does not exist, use `deferred` or `target-state`.
- Do not turn docs refresh work into a new implementation roadmap.
- Do not describe `query_similar()` or cross-domain transfer as live.

## Required Completion Evidence

- files changed under `tmp/docs-parity/06/`
- the shell syntax check result
- explicit note that HDC-on-Engram is the next concrete follow-up
