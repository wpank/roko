# Agent Runbook — 09 Daimon

Batch `09` is a **mature-subsystem doc pass**.

The aim is to make the daimon docs easy for later agents to trust
without forcing them to rediscover the runtime shape.

## Default posture

- Prefer doc honesty over new daimon code.
- Trust `roko-core/src/affect.rs` and `roko-daimon/src/lib.rs`.
- Treat `docs/09-daimon/13-current-status-and-gaps.md` as mostly right.
- Treat `docs/09-daimon/11-coding-agent-integration.md` and
  `12-collective-emotional-contagion.md` as the main frontier docs.

## What good work looks like

- Remove or downgrade stale `roko-golem` runtime language.
- Mark design-only surfaces as `Design — Phase 2+`.
- Keep live partial integrations visible and correctly scoped.
- Cross-link Doc 13 next steps to the concrete parity entries.

## What to avoid

- Do not add new daimon primitives.
- Do not implement ALMA layers, contagion, fatigue, or per-crate confidence.
- Do not regenerate Doc 13 wholesale.
- Do not widen the batch into broader compose/neuro refactors.

## Deliverable standard

Every batch should leave:

- changed docs with clear status/banners,
- commands run and verification output,
- explicit deferrals,
- a `PASS`, `FAIL`, or `BLOCKED` outcome.
