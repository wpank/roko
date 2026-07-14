# Executable Backlog Plans

This directory is the runnable task-file layer for `tmp/status-quo/backlog`.

- `E*/tasks.toml` files contain **447** implementation tasks across E01-E48, including the canonically expanded 169-task E01-E18 status-quo layer.
- `DOC-*/tasks.toml` files contain **71** source-corpus reconciliation tasks covering all **745**
  source documents from `tmp/status-quo/*.md`, `docs/v1/**`, `docs/v2/**`, and `docs/v2-depth/**`.
- `status-quo-authoring-gaps/tasks.toml` is retained only as provenance; it is superseded and all of its `GAP-*` authoring tasks are marked `skipped`.
- Coverage ledger: `../06-EXECUTABLE-TASK-FILE-COVERAGE.md`.
- Source-corpus coverage ledger: `../08-SOURCE-CORPUS-PLAN-COVERAGE.md`.

Validate a plan with:

```sh
cargo run -p roko-cli --bin roko -- plan validate tmp/status-quo/backlog/plans/<plan-dir>
```

| Epic | Plan directory | Implementation tasks | Remaining gaps |
|---|---|---:|---:|
| E01 | `plans/E01-execution-engine` | 16 | 0 |
| E02 | `plans/E02-STORAGE-CONVERGENCE` | 12 | 0 |
| E03 | `plans/E03-type-consolidation` | 7 | 0 |
| E04 | `plans/E04-security-perimeter` | 19 | 0 |
| E05 | `plans/E05-gate-adaptivity-live` | 8 | 0 |
| E06 | `plans/E06-COMPOSE-UNIFY` | 9 | 0 |
| E07 | `plans/E07-learning-knowledge` | 10 | 0 |
| E08 | `plans/E08-conductor-supervision` | 9 | 0 |
| E09 | `plans/E09-OBSERVABILITY` | 11 | 0 |
| E10 | `plans/E10-FRONTEND-CONTRACT` | 7 | 0 |
| E11 | `plans/E11-chain-isfr` | 5 | 0 |
| E12 | `plans/E12-DEAD-CODE-CLEANUP` | 9 | 0 |
| E13 | `plans/E13-SPEC-DEBT-V2` | 3 | 0 |
| E14 | `plans/E14-providers-tools` | 12 | 0 |
| E15 | `plans/E15-mcp-config` | 7 | 0 |
| E16 | `plans/E16-prd-self-hosting-gaps` | 2 | 0 |
| E17 | `plans/E17-acp-completion` | 8 | 0 |
| E18 | `plans/E18-DOCS-CONFIG-OPS` | 15 | 0 |
| E19 | `plans/E19-signal-protocol` | 10 | 0 |
| E20 | `plans/E20-cell-unification` | 10 | 0 |
| E21 | `plans/E21-graph-engine` | 10 | 0 |
| E22 | `plans/E22-execution-runtime` | 10 | 0 |
| E23 | `plans/E23-agent-cognitive-autonomy` | 10 | 0 |
| E24 | `plans/E24-memory-advanced` | 10 | 0 |
| E25 | `plans/E25-learning-loops-advanced` | 10 | 0 |
| E26 | `plans/E26-inference-gateway` | 12 | 0 |
| E27 | `plans/E27-feeds-system` | 10 | 0 |
| E28 | `plans/E28-groups-coordination` | 8 | 0 |
| E29 | `plans/E29-connectivity-relay` | 9 | 0 |
| E30 | `plans/E30-extension-system` | 9 | 0 |
| E31 | `plans/E31-trigger-system` | 8 | 0 |
| E32 | `plans/E32-tool-plugin-ecosystem` | 8 | 0 |
| E33 | `plans/E33-telemetry-lens` | 9 | 0 |
| E34 | `plans/E34-security-ifc` | 8 | 0 |
| E35 | `plans/E35-auth-protocol` | 8 | 0 |
| E36 | `plans/E36-payments` | 8 | 0 |
| E37 | `plans/E37-surfaces` | 9 | 0 |
| E38 | `plans/E38-marketplace` | 9 | 0 |
| E39 | `plans/E39-registries-identity` | 8 | 0 |
| E40 | `plans/E40-arenas-evals` | 8 | 0 |
| E41 | `plans/E41-defi-products` | 8 | 0 |
| E42 | `plans/E42-config-evolution` | 8 | 0 |
| E43 | `plans/E43-deployment-portability` | 8 | 0 |
| E44 | `plans/E44-cross-cut-functors` | 8 | 0 |
| E45 | `plans/E45-orchestrator-mori-parity` | 10 | 0 |
| E46 | `plans/E46-github-workflow-integration` | 12 | 0 |
| E47 | `plans/E47-resource-disk-management` | 11 | 0 |
| E48 | `plans/E48-rate-limit-budgeting` | 12 | 0 |
| **Total** |  | **447** | **0** |

Superseded authoring-gap plan total: **96** skipped tasks.

## Source-Corpus Plans

| Corpus | Plan directory | Source docs | Tasks |
|---|---|---:|---:|
| status-quo audit/control-plane pack | `plans/DOC-status-quo-corpus` | 109 | 12 |
| docs/v1 kernel | `plans/DOC-v1-kernel` | 113 | 8 |
| docs/v1 cognition | `plans/DOC-v1-cognition` | 119 | 7 |
| docs/v1 ecosystem | `plans/DOC-v1-ecosystem` | 185 | 10 |
| docs/v2 core | `plans/DOC-v2-core` | 34 | 10 |
| docs/v2-depth | `plans/DOC-v2-depth` | 185 | 24 |
| **Total** |  | **745** | **71** |

Start execution with the bootstrap epic:

```sh
cargo run -p roko-cli --bin roko -- plan run tmp/status-quo/backlog/plans/E01-execution-engine --engine runner-v2 --fresh --max-tasks 1
```
