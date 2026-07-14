# Executable Backlog Plans

This directory is the runnable task-file layer for `tmp/status-quo/backlog`.

- `E*/tasks.toml` files now contain all **149** implementation tasks from `../05-MASTER-CHECKLIST.md`.
- `DOC-*/tasks.toml` files contain **71** source-corpus reconciliation tasks covering all **744**
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
| E01 | `plans/E01-execution-engine` | 10 | 0 |
| E02 | `plans/E02-STORAGE-CONVERGENCE` | 12 | 0 |
| E03 | `plans/E03-type-consolidation` | 7 | 0 |
| E04 | `plans/E04-security-perimeter` | 19 | 0 |
| E05 | `plans/E05-gate-adaptivity-live` | 8 | 0 |
| E06 | `plans/E06-COMPOSE-UNIFY` | 9 | 0 |
| E07 | `plans/E07-learning-knowledge` | 10 | 0 |
| E08 | `plans/E08-conductor-supervision` | 7 | 0 |
| E09 | `plans/E09-OBSERVABILITY` | 9 | 0 |
| E10 | `plans/E10-FRONTEND-CONTRACT` | 7 | 0 |
| E11 | `plans/E11-chain-isfr` | 5 | 0 |
| E12 | `plans/E12-DEAD-CODE-CLEANUP` | 9 | 0 |
| E13 | `plans/E13-SPEC-DEBT-V2` | 3 | 0 |
| E14 | `plans/E14-providers-tools` | 7 | 0 |
| E15 | `plans/E15-mcp-config` | 6 | 0 |
| E16 | `plans/E16-prd-self-hosting-gaps` | 2 | 0 |
| E17 | `plans/E17-acp-completion` | 6 | 0 |
| E18 | `plans/E18-DOCS-CONFIG-OPS` | 13 | 0 |
| **Total** |  | **149** | **0** |

Superseded authoring-gap plan total: **96** skipped tasks.

## Source-Corpus Plans

| Corpus | Plan directory | Source docs | Tasks |
|---|---|---:|---:|
| status-quo audit pack | `plans/DOC-status-quo-corpus` | 108 | 12 |
| docs/v1 kernel | `plans/DOC-v1-kernel` | 113 | 8 |
| docs/v1 cognition | `plans/DOC-v1-cognition` | 119 | 7 |
| docs/v1 ecosystem | `plans/DOC-v1-ecosystem` | 185 | 10 |
| docs/v2 core | `plans/DOC-v2-core` | 34 | 10 |
| docs/v2-depth | `plans/DOC-v2-depth` | 185 | 24 |
| **Total** |  | **744** | **71** |

Start execution with the bootstrap epic:

```sh
cargo run -p roko-cli --bin roko -- plan run tmp/status-quo/backlog/plans/E01-execution-engine --engine runner-v2 --fresh --max-tasks 1
```
