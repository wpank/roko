# CTRL-07 independent review

## Verdict

`REJECTED`

Candidate `18973e221a5ce6f8f72366ca2d8815db21f85b7c` correctly reduces
the reproducible prerequisite diagnostics from 12 to zero and passes every
structural and scope check. It is not acceptance-ready because one newly authored
surgical line range does not include one of the two production APIs it explicitly
claims to cover.

## Review identity and scope

- Candidate: `18973e221a5ce6f8f72366ca2d8815db21f85b7c`
- Candidate base: `a4278ced081c9f42ef186b8c4a93528ef78c05c3`
- Review branch: `review/CTRL-07-18973e221a5c`
- Validator: integrated `roko 0.1.0`, Git build identifier `d4749f9c7`
- Source `plans/INDEX.md` sealed SHA-256:
  `7ac5679f9ff7a32571ad0ed70e9914b579f12a5f22e4285f4804c20a19077b44`

The candidate contains exactly 13 paths: 12 backlog manifests and
`tmp/status-quo/execution-evidence/CTRL-07.md`. No production code, validator code,
master/status record, shared index, lockfile, or unrelated manifest is in the diff.

## Finding requiring correction

### R1 — E17-T02's cited range omits `record_metric`

Severity: required metadata correctness; no product-runtime regression.

File:
`tmp/status-quo/backlog/plans/E17-acp-completion/tasks.toml`, task `E17-T02`.

The candidate changes the task's context to:

```toml
{ path = "crates/roko-learn/src/prompt_experiment.rs", lines = "395-455", why = "Current ExperimentStore load, active lookup, assign_variant, and record_metric APIs to mirror for ACP selection and outcome recording." }
```

Independent source inspection shows:

- `ExperimentStore` and its loading/lookup methods are at lines 395–441;
- `ExperimentStore::assign_variant` is at lines 447–454;
- `ExperimentStore::record_metric` is at lines 599–604, outside the declared
  395–455 range.

The task's changed `symbols` record also correctly names the three-argument
`record_metric(experiment_id, variant_id, metric)` API, so the omission is not an
optional unrelated reference. A context-constrained implementer following the new
range cannot inspect the outcome half of the explicitly required selection/outcome
pair.

Smallest correction: split this into two `read_files` entries (395–455 for
load/active/assignment and 590–605 for `record_metric`) or use another unambiguous
range representation that includes both regions. Update the worker evidence, rerun
the gates below, and submit the new immutable candidate for independent review.

## Changed-context production trace

The other nine unique stale paths are mapped to semantically live owners, and the
E17 file itself is the correct live module once R1's range is fixed:

| Task | Reviewed live module/symbol | Independent result |
|---|---|---|
| `E01-T11` | `config/budget.rs`: `BudgetConfig::{max_plan_usd,max_turn_usd}` | Correct owner and cited 1–48 range. |
| `E02-T03` | `dashboard_snapshot.rs`: `DashboardSnapshot::load_from_workdir` reads `state/executor.json` | Correct owner and cited 1260–1305 range. |
| `E09-T10` | `file_substrate.rs`: `FileSubstrate`, append-mode `log_writer`, serialized/locked JSONL writes | Correct live replacement for the removed substrate module; 1–80 contains the append writer/locking design used as the stated pattern. |
| `E14-T08` | `config/provider.rs`: `ProviderConfig` timeout and `max_concurrent` fields | Correct schema owner and cited 255–325 range. |
| `E15-T1` | `claude_cli_agent.rs`: `ClaudeCliAgent::build_command` adds `--mcp-config` and `--strict-mcp-config` | Correct consumer and cited 325–350 range. |
| `E17-T02` | `prompt_experiment.rs`: `ExperimentStore::assign_variant` and `record_metric` | Correct module and signatures; rejected line range omits `record_metric`. |
| `E20-T08` | `tool/registry.rs`: `ToolRegistry` and `VecToolRegistry` | Correct runtime-management pattern and cited 1–110 range. |
| `E31-T03` | `secrets/mod.rs`: exports, submodule inventory, `SecretStore` | Correct canonical entry point and cited 1–55 range. |
| `E33-T03` | `error/mod.rs`: canonical `RokoError` taxonomy (`ErrorKind` also lives later in this module) | Correct canonical entry point; the changed rationale only claims the `RokoError` taxonomy covered by 1–100. |
| `E35-T01` | `config/serve.rs`: `ServeAuthConfig`, `ApiKeyEntry::{scope,expires_at}` | Correct owner and cited 79–125 range. |

Every removed path is absent on disk. Every replacement path exists, and no
prerequisite was made to disappear by deleting a task context or weakening the
validator.

## Producer-edge and graph review

- `E06-T01` has the sole declared file
  `tmp/status-quo/backlog/decisions/E06-canonical-surface.md`.
  `E06-T07` now directly depends on `E06-T01` while retaining `E06-T06`.
- `E18-T03` has the sole declared file `.github/workflows/deny.yml`.
  `E18-T14` now directly depends on `E18-T03` while retaining `E18-T02`.
- All local dependency IDs resolve, and independent DFS of both complete manifest
  DAGs found no cycle.
- Neither added producer edge is fabricated; each points to the exact task whose
  `files` declaration owns the validator-reported missing prerequisite.

## Semantic diff checks

Python `tomllib` comparison of each candidate manifest against
`git show a4278ced:<path>` established:

- all 12 `[meta]` tables are semantically unchanged;
- task counts remain equal to `meta.total`;
- every ordered task ID and every task status is unchanged;
- exactly one intended task record changes per manifest;
- no external dependency changes;
- all 193 tracked TOML files in a disposable candidate archive parse successfully.

The per-manifest changed tasks and counts were:

```text
E01-execution-engine: tasks=16 changed=E01-T11
E02-STORAGE-CONVERGENCE: tasks=12 changed=E02-T03
E06-COMPOSE-UNIFY: tasks=9 changed=E06-T07
E09-OBSERVABILITY: tasks=11 changed=E09-T10
E14-providers-tools: tasks=12 changed=E14-T08
E15-mcp-config: tasks=7 changed=E15-T1
E17-acp-completion: tasks=8 changed=E17-T02
E18-DOCS-CONFIG-OPS: tasks=15 changed=E18-T14
E20-cell-unification: tasks=10 changed=E20-T08
E31-trigger-system: tasks=8 changed=E31-T03
E33-telemetry-lens: tasks=9 changed=E33-T03
E35-auth-protocol: tasks=8 changed=E35-T01
```

## Independent diagnostic reproduction

Both commits were validated from disposable full repository archives so the
validator's generated `plans/INDEX.md` side effect could not touch the review
worktree. The baseline run used the same candidate archive with only the 12
manifests restored byte-for-byte from the base commit.

Commands, abbreviated to show the reproducible method:

```sh
git archive 18973e221a5ce6f8f72366ca2d8815db21f85b7c | tar -x -C "$fixture"
(cd "$fixture" && "$roko" plan validate --strict tmp/status-quo/backlog/plans --color never)
(cd "$fixture" && "$roko" plan validate --strict tmp/status-quo/self-heal/plans --color never)
git archive a4278ced081c9f42ef186b8c4a93528ef78c05c3 -- <12-manifest-paths> | tar -x -C "$fixture"
(cd "$fixture" && "$roko" plan validate --strict tmp/status-quo/backlog/plans --color never)
```

Observed results:

```text
candidate backlog:  exit 0; 0 diagnostics in 55 plans
candidate self-heal: exit 0; 0 diagnostics in 6 plans
base backlog:       exit 1; 12 PLAN_031 diagnostics in 55 plans
candidate TOML:     193 files; 0 parse errors
```

The 12 baseline diagnostics were exactly `E01-T11`, `E02-T03`, `E06-T07`,
`E09-T10`, `E14-T08`, `E15-T1`, `E17-T02`, `E18-T14`, `E20-T08`, `E31-T03`,
`E33-T03`, and `E35-T01`. No diagnostic was hidden by excluding a plan.

## Hygiene

- `git diff 18973e221^ 18973e221 --check`: exit 0.
- Source `plans/INDEX.md` SHA-256 after all isolated runs:
  `7ac5679f9ff7a32571ad0ed70e9914b579f12a5f22e4285f4804c20a19077b44`
  (unchanged).
- Review worktree remained clean until this review file was created.
- No `.roko` state, generated index change, temporary file, lockfile, or other
  artifact remains in the worktree.

## Required next action

Correct R1 on the worker branch, update the implementation evidence to match the
new immutable commit, rerun candidate backlog/self-heal strict validation and the
semantic manifest comparison, then request a fresh independent review. All other
reviewed CTRL-07 acceptance checks pass.

