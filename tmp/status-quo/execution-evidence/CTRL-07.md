# CTRL-07 — Zero corrected backlog prerequisite diagnostics

## Scope and baseline

- Worker base: `a4278ced081c9f42ef186b8c4a93528ef78c05c3`.
- Validation binary: integrated `roko 0.1.0 ... git d4749f9c7`, containing the merged CTRL-05/CTRL-06 validator behavior.
- The validator was run from a disposable repository-shaped root so its generated `plans/INDEX.md` side effect could not modify the source checkout.
- Before correction: strict backlog validation exited 1 with exactly 12 `PLAN_031` prerequisite diagnostics in 55 plans.
- Source `plans/INDEX.md` SHA-256 before and after every isolated run: `7ac5679f9ff7a32571ad0ed70e9914b579f12a5f22e4285f4804c20a19077b44`.

## Corrections

The ten missing paths were traced to their live modules and symbols rather than removed from task context:

| Task | Stale prerequisite | Canonical current prerequisite and proof |
| --- | --- | --- |
| E01-T11 | `crates/roko-core/src/config.rs` | `crates/roko-core/src/config/budget.rs`; `BudgetConfig` owns live `[budget]` plan/turn spend ceilings. |
| E02-T03 | `crates/roko-serve/src/dashboard_snapshot.rs` | `crates/roko-core/src/dashboard_snapshot.rs`; `DashboardSnapshot::load_from_workdir` contains the executor snapshot bootstrap read. |
| E09-T10 | `crates/roko-fs/src/substrate.rs` | `crates/roko-fs/src/file_substrate.rs`; current `FileSubstrate` owns the serialized JSONL writer path. |
| E14-T08 | `crates/roko-core/src/config.rs` | `crates/roko-core/src/config/provider.rs`; `ProviderConfig` owns live per-provider timeout/concurrency configuration. |
| E15-T1 | `crates/roko-agent/src/provider/claude_cli_agent.rs` | `crates/roko-agent/src/claude_cli_agent.rs`; `ClaudeCliAgent` appends `--mcp-config` and `--strict-mcp-config`. |
| E17-T02 | `crates/roko-learn/src/experiments.rs` | `crates/roko-learn/src/prompt_experiment.rs`; lines 395–455 cover `ExperimentStore` loading/active lookup/`assign_variant`, while lines 590–605 cover `record_metric`. |
| E20-T08 | `crates/roko-core/src/tool.rs` | `crates/roko-core/src/tool/registry.rs`; current `ToolRegistry` and `VecToolRegistry` runtime pattern. |
| E31-T03 | `crates/roko-core/src/secrets.rs` | `crates/roko-core/src/secrets/mod.rs`; canonical exports plus `SecretStore` trait and explicit submodule inventory. |
| E33-T03 | `crates/roko-core/src/error.rs` | `crates/roko-core/src/error/mod.rs`; canonical `RokoError`/`ErrorKind` taxonomy. |
| E35-T01 | `crates/roko-core/src/config.rs` | `crates/roko-core/src/config/serve.rs`; live `ServeAuthConfig` and `ApiKeyEntry`, including `scope` and `expires_at`. |

Two inputs are intentionally produced inside their containing plans, so their exact producer edges were declared:

- E06-T07 now depends on E06-T01, whose sole output is `tmp/status-quo/backlog/decisions/E06-canonical-surface.md`; its existing E06-T06 dependency remains.
- E18-T14 now depends on E18-T03, whose sole output is `.github/workflows/deny.yml`; its existing E18-T02 dependency remains.

No task status, manifest metadata, task count, task ID, external dependency, or unrelated task changed.

Independent review `18d16f225` rejected the first candidate only because its single E17 range ended at line 455 while claiming to cover `record_metric` at lines 599–604. The corrected context uses two surgical entries so both the selection and outcome APIs are directly inspectable; every other reviewed mapping and producer edge is unchanged.

## Proof

1. Parsed all 12 touched manifests with Python 3 `tomllib`, parsed their base versions from `git show a4278ced081c9f42ef186b8c4a93528ef78c05c3:<path>`, and compared structure. All metadata was byte-semantically equal, all task IDs and statuses were unchanged, each manifest task count still equalled `meta.total`, and only the intended task changed in each file.
2. Checked the two producer closures structurally: E06-T01 declares the ADR in `files` and is in E06-T07 `depends_on`; E18-T03 declares `deny.yml` in `files` and is in E18-T14 `depends_on`.
3. Ran the current integrated validator against `tmp/status-quo/backlog/plans` from a comprehensive disposable root: exit 0, `0 diagnostics in 55 plans` (down from 12).
4. Ran the same integrated validator with `--strict` against `tmp/status-quo/self-heal/plans` from a disposable root: exit 0, `0 diagnostics in 6 plans`.
5. `git diff --check`: exit 0. The source `plans/INDEX.md` hash remained byte-identical throughout isolated validation.

## Integration and terminal status

- Corrected candidate: `9458a6920d72e457553e31cd51b9ac89d70d2483`.
- Final prerequisite review: `81d1af92b142ce512964b078ccb5bc1a417b8e2d` (`ACCEPTED`).
- Prerequisite integration merge: `206e9079812b27f738d95f91d1135d0f663c836f`.
- Canonical-ledger reconciliation: `950fa8bc95a2b92f90dc970d6038547a28feb9e4`.
- Ledger review: `91da0fea5604e7639928824c3ab8ab07c21832af` (`ACCEPTED`).
- Ledger integration merge: `f0cf7e769306b3217b30de797e2698b1a673326e`.
- Post-merge proof at `f0cf7e769`: strict backlog validation exits zero with
  `0 diagnostics in 55 plans`; strict self-heal validation exits zero with
  `0 diagnostics in 6 plans`; `git diff --check` passes; the source
  `plans/INDEX.md` SHA-256 remains
  `7ac5679f9ff7a32571ad0ed70e9914b579f12a5f22e4285f4804c20a19077b44`.
- Final status: `DONE`. The historical six-warning claim is explicitly superseded in
  the canonical coverage ledger; no placeholder, weakened prerequisite, status
  change, or generated-index delta was used to make the gate green.
