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
| E17-T02 | `crates/roko-learn/src/experiments.rs` | `crates/roko-learn/src/prompt_experiment.rs`; current `ExperimentStore` exposes `assign_variant` and `record_metric`. |
| E20-T08 | `crates/roko-core/src/tool.rs` | `crates/roko-core/src/tool/registry.rs`; current `ToolRegistry` and `VecToolRegistry` runtime pattern. |
| E31-T03 | `crates/roko-core/src/secrets.rs` | `crates/roko-core/src/secrets/mod.rs`; canonical exports plus `SecretStore` trait and explicit submodule inventory. |
| E33-T03 | `crates/roko-core/src/error.rs` | `crates/roko-core/src/error/mod.rs`; canonical `RokoError`/`ErrorKind` taxonomy. |
| E35-T01 | `crates/roko-core/src/config.rs` | `crates/roko-core/src/config/serve.rs`; live `ServeAuthConfig` and `ApiKeyEntry`, including `scope` and `expires_at`. |

Two inputs are intentionally produced inside their containing plans, so their exact producer edges were declared:

- E06-T07 now depends on E06-T01, whose sole output is `tmp/status-quo/backlog/decisions/E06-canonical-surface.md`; its existing E06-T06 dependency remains.
- E18-T14 now depends on E18-T03, whose sole output is `.github/workflows/deny.yml`; its existing E18-T02 dependency remains.

No task status, manifest metadata, task count, task ID, external dependency, or unrelated task changed.

## Proof

1. Parsed all 12 touched manifests with Python 3 `tomllib`, parsed their base versions from `git show HEAD:<path>`, and compared structure. All metadata was byte-semantically equal, all task IDs and statuses were unchanged, each manifest task count still equalled `meta.total`, and only the intended task changed in each file.
2. Checked the two producer closures structurally: E06-T01 declares the ADR in `files` and is in E06-T07 `depends_on`; E18-T03 declares `deny.yml` in `files` and is in E18-T14 `depends_on`.
3. Ran the current integrated validator against `tmp/status-quo/backlog/plans` from a comprehensive disposable root: exit 0, `0 diagnostics in 55 plans` (down from 12).
4. Ran the same integrated validator with `--strict` against `tmp/status-quo/self-heal/plans` from a disposable root: exit 0, `0 diagnostics in 6 plans`.
5. `git diff --check`: exit 0. The source `plans/INDEX.md` hash remained byte-identical throughout isolated validation.
