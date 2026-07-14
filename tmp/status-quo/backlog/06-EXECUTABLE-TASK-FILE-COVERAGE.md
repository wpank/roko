# 06 — Executable Task-File Coverage

> Coverage ledger for the full executable backlog after the per-epic task expansion.
> Source inputs: `05-MASTER-CHECKLIST.md`, `epics/E01…E18-*.md`,
> `07-SUBAGENT-TASK-AUTHORING-NOTES.md`, v2 spec docs (`docs/v2/`, `docs/v2-depth/`),
> and the generated files under `plans/`.

## Summary

- Status-quo audit tasks (E01–E18, expanded): **169**
- v2 spec implementation tasks (E19–E45): **243**
- Operational capabilities tasks (E46–E48): **35**
- **Total implementation tasks materialized**: **447**
- Remaining implementation task-definition gaps: **0**
- Executable epic plan directories: **48**
- DOC reconciliation tasks: **71** (across 6 DOC-* plans)
- Superseded authoring-gap tasks retained for provenance: **96**, all marked `skipped`

The prior `plans/status-quo-authoring-gaps/tasks.toml` layer has been consumed. It remains on disk
only as provenance, with `[meta].status = "superseded"` and every `GAP-*` task marked
`status = "skipped"` so a root `plan run` cannot re-author task blocks that now exist in the
per-epic plans.

## Per-Epic Coverage

| Epic | Plan directory | Checklist total | Materialized tasks | Remaining gaps |
|---|---|---:|---:|---:|
| E01 | `tmp/status-quo/backlog/plans/E01-execution-engine` | 16 | 16 | 0 |
| E02 | `tmp/status-quo/backlog/plans/E02-STORAGE-CONVERGENCE` | 12 | 12 | 0 |
| E03 | `tmp/status-quo/backlog/plans/E03-type-consolidation` | 7 | 7 | 0 |
| E04 | `tmp/status-quo/backlog/plans/E04-security-perimeter` | 19 | 19 | 0 |
| E05 | `tmp/status-quo/backlog/plans/E05-gate-adaptivity-live` | 8 | 8 | 0 |
| E06 | `tmp/status-quo/backlog/plans/E06-COMPOSE-UNIFY` | 9 | 9 | 0 |
| E07 | `tmp/status-quo/backlog/plans/E07-learning-knowledge` | 10 | 10 | 0 |
| E08 | `tmp/status-quo/backlog/plans/E08-conductor-supervision` | 9 | 9 | 0 |
| E09 | `tmp/status-quo/backlog/plans/E09-OBSERVABILITY` | 11 | 11 | 0 |
| E10 | `tmp/status-quo/backlog/plans/E10-FRONTEND-CONTRACT` | 7 | 7 | 0 |
| E11 | `tmp/status-quo/backlog/plans/E11-chain-isfr` | 5 | 5 | 0 |
| E12 | `tmp/status-quo/backlog/plans/E12-DEAD-CODE-CLEANUP` | 9 | 9 | 0 |
| E13 | `tmp/status-quo/backlog/plans/E13-SPEC-DEBT-V2` | 3 | 3 | 0 |
| E14 | `tmp/status-quo/backlog/plans/E14-providers-tools` | 12 | 12 | 0 |
| E15 | `tmp/status-quo/backlog/plans/E15-mcp-config` | 7 | 7 | 0 |
| E16 | `tmp/status-quo/backlog/plans/E16-prd-self-hosting-gaps` | 2 | 2 | 0 |
| E17 | `tmp/status-quo/backlog/plans/E17-acp-completion` | 8 | 8 | 0 |
| E18 | `tmp/status-quo/backlog/plans/E18-DOCS-CONFIG-OPS` | 15 | 15 | 0 |
| **E01–E18 subtotal** |  | **169** | **169** | **0** |

### v2 Spec Implementation (E19–E45)

| Epic | Plan directory | Tasks | Remaining gaps |
|---|---|---:|---:|
| E19 | `tmp/status-quo/backlog/plans/E19-signal-protocol` | 10 | 0 |
| E20 | `tmp/status-quo/backlog/plans/E20-cell-unification` | 10 | 0 |
| E21 | `tmp/status-quo/backlog/plans/E21-graph-engine` | 10 | 0 |
| E22 | `tmp/status-quo/backlog/plans/E22-execution-runtime` | 10 | 0 |
| E23 | `tmp/status-quo/backlog/plans/E23-agent-cognitive-autonomy` | 10 | 0 |
| E24 | `tmp/status-quo/backlog/plans/E24-memory-advanced` | 10 | 0 |
| E25 | `tmp/status-quo/backlog/plans/E25-learning-loops-advanced` | 10 | 0 |
| E26 | `tmp/status-quo/backlog/plans/E26-inference-gateway` | 12 | 0 |
| E27 | `tmp/status-quo/backlog/plans/E27-feeds-system` | 10 | 0 |
| E28 | `tmp/status-quo/backlog/plans/E28-groups-coordination` | 8 | 0 |
| E29 | `tmp/status-quo/backlog/plans/E29-connectivity-relay` | 9 | 0 |
| E30 | `tmp/status-quo/backlog/plans/E30-extension-system` | 9 | 0 |
| E31 | `tmp/status-quo/backlog/plans/E31-trigger-system` | 8 | 0 |
| E32 | `tmp/status-quo/backlog/plans/E32-tool-plugin-ecosystem` | 8 | 0 |
| E33 | `tmp/status-quo/backlog/plans/E33-telemetry-lens` | 9 | 0 |
| E34 | `tmp/status-quo/backlog/plans/E34-security-ifc` | 8 | 0 |
| E35 | `tmp/status-quo/backlog/plans/E35-auth-protocol` | 8 | 0 |
| E36 | `tmp/status-quo/backlog/plans/E36-payments` | 8 | 0 |
| E37 | `tmp/status-quo/backlog/plans/E37-surfaces` | 9 | 0 |
| E38 | `tmp/status-quo/backlog/plans/E38-marketplace` | 9 | 0 |
| E39 | `tmp/status-quo/backlog/plans/E39-registries-identity` | 8 | 0 |
| E40 | `tmp/status-quo/backlog/plans/E40-arenas-evals` | 8 | 0 |
| E41 | `tmp/status-quo/backlog/plans/E41-defi-products` | 8 | 0 |
| E42 | `tmp/status-quo/backlog/plans/E42-config-evolution` | 8 | 0 |
| E43 | `tmp/status-quo/backlog/plans/E43-deployment-portability` | 8 | 0 |
| E44 | `tmp/status-quo/backlog/plans/E44-cross-cut-functors` | 8 | 0 |
| E45 | `tmp/status-quo/backlog/plans/E45-orchestrator-mori-parity` | 10 | 0 |
| **E19–E45 subtotal** |  | **243** | **0** |

### Operational Capabilities (E46–E48)

| Epic | Plan directory | Tasks | Remaining gaps |
|---|---|---:|---:|
| E46 | `tmp/status-quo/backlog/plans/E46-github-workflow-integration` | 12 | 0 |
| E47 | `tmp/status-quo/backlog/plans/E47-resource-disk-management` | 11 | 0 |
| E48 | `tmp/status-quo/backlog/plans/E48-rate-limit-budgeting` | 12 | 0 |
| **E46–E48 subtotal** |  | **35** | **0** |

### Grand Total

| Layer | Tasks |
|---|---:|
| E01–E18 (status-quo audit, expanded) | 169 |
| E19–E45 (v2 spec implementation) | 243 |
| E46–E48 (operational capabilities) | 35 |
| **Implementation total** | **447** |
| DOC-* (source-corpus reconciliation) | 71 |
| **Grand total** | **518** |

## Validation Status

Current strict validation commands:

```sh
cargo run -q -p roko-cli --bin roko -- plan validate --strict tmp/status-quo/backlog/plans
cargo run -q -p roko-cli --bin roko -- plan validate --strict tmp/status-quo/self-heal/plans
```

Reviewed results on integrated commit `206e9079812b27f738d95f91d1135d0f663c836f` with the
prerequisite-aware validator:

- backlog: exit code `0`; `0 diagnostics in 55 plans`
- self-heal: exit code `0`; `0 diagnostics in 6 plans`

The historical statement that non-strict validation returned six expected `PLAN_031` warnings is
superseded. CTRL-06 made the validator distinguish dependency-created outputs from true
prerequisites. Reviewed CTRL-07 then corrected the remaining stale prerequisite paths and producer
edges; candidate `9458a6920d72e457553e31cd51b9ac89d70d2483` was accepted by review commit
`81d1af92b` and integrated by `206e90798`. The current strict results were reproduced from a
disposable repository-shaped root so validation could not alter the sealed source index; see
`tmp/status-quo/execution-evidence/CTRL-07-LEDGER-RECONCILIATION.md`.

## Compatibility Rules Applied

- Every executable implementation task uses `[meta]` plus `[[task]]`, not `[[tasks]]`.
- Human tier aliases from checklist prose were normalized to runtime-valid tiers:
  `mechanical`, `focused`, `integrative`, `architectural`.
- Runtime roles were normalized to supported roles, primarily `implementer`, `architect`,
  `reviewer`, and `scribe`.
- Same-plan dependencies use `depends_on`; cross-epic and legacy plan dependencies use
  `depends_on_plan`.
- Verify commands are executable shell commands rather than placeholders.
- `cargo run -p roko-cli` verify commands include `--bin roko` where needed because this workspace
  has multiple binaries.

## Execution Note

Until `E01-T01` lands, the safe engine flag is mandatory:

```sh
cargo run -p roko-cli --bin roko -- plan run tmp/status-quo/backlog/plans/E01-execution-engine --engine runner-v2 --fresh --max-tasks 1
```

After E01 flips the default engine and validates resume behavior, run broader slices from
`tmp/status-quo/backlog/plans/` with `--engine runner-v2` until the repo docs and CLI default are
updated.
