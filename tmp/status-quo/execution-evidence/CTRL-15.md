# CTRL-15 executable ownership reconciliation evidence

## Assignment

- Base SHA: `ebcc3add020af2a3ff2f3041f721839c16463be2`.
- Branch/worktree: `agent/CTRL-15` / `workers/CTRL-15`.
- Integration branch: `status-quo/integration-status-quo-20260714T073140Z`.
- Reserved scope: the sealed-index manifests that require ownership changes,
  `plans/_meta/EXECUTION-OWNERSHIP.md`, intentionally regenerated
  `plans/INDEX.md`, exact historical/current notices in backlog 02/03, and this
  evidence file.
- Explicit non-goals: production code, tests, Cargo manifests/lockfiles, canonical
  E/SH manifests, task completion status, the master checklist,
  `plans/_meta/IMPLEMENTATION_ORDER.md`, remote state, or external systems.

## Requirement and source reconstruction

CTRL-15 must account for every one of the 120 tasks in the sealed imported index
without executing duplicate writers. `SUPERSEDED` is not a cancellation: a duplicate
row remains `ready` as a zero-write acceptance reviewer, waits through real scheduler
edges for named canonical owners, and carries equivalent-or-stronger task-scoped
acceptance. A distinct product outcome retains its original implementation scope.

I read the complete master, all 29 sealed-baseline manifests, the recovered 24-task
architecture queue, all current E/SH manifests, the CTRL-08/09/13/14 evidence, the
sealed and generated index forms, backlog 02/03, current parser/runtime code, and
relevant Git history. The earlier `99 retained + 21 roll-ups` figure was used only as
a checksum hypothesis. The independent row reconstruction reached that total from
these exact duplicate groups:

| Duplicate group | Roll-up rows | Named implementation owners |
|---|---:|---|
| P11 feature/default facade | 4 | E01-T01, E12-T03 |
| P12 parallel scheduler/container prescriptions | 5 | E01-T04/T05, SH02-T01, SH04-T01/T02 |
| P14 legacy gate-rung path | 3 | E05-T05/T07 |
| P18 delimiter/ad-hoc TUI bridge | 5 | SH04-T01-T05 |
| P29 duplicate develop registration/dispatch | 2 | P10-slash-command-flags#T3 and P10-slash-command-flags#T4 |
| P30 unconditional provider checks | 2 | P27-provider-error-ux#T1 |
| **Total** | **21** | |

The exact 120 rows and every retained write scope are durable in
[`plans/_meta/EXECUTION-OWNERSHIP.md`](../../../plans/_meta/EXECUTION-OWNERSHIP.md).

## Implementation and invariants

Six manifests required edits. Every roll-up now has:

- unchanged stable ID, `ready` status, tier, and task-local dependency order;
- `ownership = "acceptance-roll-up"` and a named `superseded_by` audit mapping;
- `files = []` and `role = "quick-reviewer"`;
- one or more scheduler-recognized `depends_on_plan` edges;
- nonempty task-scoped `acceptance` before `[task.context]`;
- owner-focused context, an owner-existence check, and future owner acceptance tests.

The 99 retained baseline tasks are byte-semantically unchanged when parsed as TOML:
their complete task dictionaries equal base `ebcc3add0`, including ID, order, status,
files, role, context, verify, acceptance, and dependencies. No task was marked done,
skipped, active, or blocked.

The regenerated current index intentionally reports 30 executable plans/144 tasks:
the original sealed 29/120 population plus the separately recovered
`architecture-core-queue` (24). The ledger preserves the sealed baseline boundary;
backlog 02/03 now display current notices instead of silently presenting the recovered
queue as absent. The 24 architecture-core tasks retain their original implementation
ownership and are not hidden inside the 120-row mapping.

## Ownership and scheduling proof

The combined top-level/backlog/self-heal corpus has 93 plan manifests and 881 task
records. `TasksFile` exposes task-level `depends_on_plan` as the runtime edge set;
the separately declared `meta.depends_on_plan` edges are included only in the
all-declared set. Exact traversals of base `ebcc3add0` and candidate `763f47308`
produce:

```text
                                      raw refs   unique source->target edges
base task runtime edges                    320                           160
base meta edges                              2                             2
base all-declared edges                    322                           162
candidate task runtime edges               345                           169
candidate meta edges                         2                             2
candidate all-declared edges               347                           171
task/meta overlap (base and candidate)                                     0
same-plan task references (both)           849
unresolved same-plan references              0
unresolved task-runtime/all-declared refs     0 / 0
runtime/all-declared cyclic SCCs              0 / 0
```

The two metadata edges are disjoint from the task-runtime set:
`E46-github-workflow-integration -> E01-execution-engine` and
`E48-rate-limit-budgeting -> E01-execution-engine`. The 21 roll-ups add 25 raw
task references but only nine new unique source-plan pairs. Thus 169 is the
candidate runtime edge count and 171 is the candidate all-declared edge count.

The write census reports 1,658 retained write claims over 586 unique paths and 161
paths claimed by tasks from more than one plan. Those are scheduling conflict surfaces,
not automatically equivalent outcomes; they remain visible for the coordinator's
file-exclusivity protocol. All 21 proven duplicate rows contribute zero write claims,
so CTRL-15 removes their duplicate implementation/public-API authority without
pretending that every future cross-plan edit is file-disjoint.

## Verification

### Actual Rust parser

A temporary `roko-cli` integration test imported the public
`roko_cli::task_parser::TasksFile` and parsed all 29 baseline manifests plus the
architecture queue. It asserted a 120-row qualified-ID bijection, exact 21 roll-ups,
zero files, `quick-reviewer`, nonempty plan dependencies, nonempty task-scoped
acceptance, and the separate 24-task queue:

```text
CARGO_TARGET_DIR=integration/target cargo test -p roko-cli \
  --test ctrl15_ownership_semantics -- --nocapture
test ctrl15_ownership_semantics ... ok
test result: ok. 1 passed; 0 failed
```

The temporary test was removed after proof and is not candidate scope.

### Immutable-base and TOML census

`git archive ebcc3add0 plans` was extracted to a disposable directory. A `tomllib`
comparison asserted unchanged meta, task ID order, and statuses for all 29 manifests;
all 99 retained task dictionaries were exactly equal to base. The same traversal
asserted all 21 roll-ups' zero-write role/dependency/acceptance contract:

```text
baseline_tasks=120 retained=99 rollups=21
tracked TOMLs parsed=193; parse errors=0
```

The disposable base archive was removed.

### Strict disposable-root validation and index hygiene

The exact candidate manifests were overlaid on a fresh full archive. The integrated
repository binary was invoked from inside that archive so generated state/index output
could not mutate the worker source:

```text
roko plan validate --strict tmp/status-quo/backlog/plans
exit 0; 0 diagnostics in 55 plans

roko plan validate --strict tmp/status-quo/self-heal/plans
exit 0; 0 diagnostics in 6 plans

roko plan validate --strict plans
exit 1; 94 diagnostics in 32 plans
diagnostic codes: PLAN_031 only (94)
```

The top-level result is the pre-existing, explicitly bounded missing-prerequisite
census for intended future source outputs; it contains no parse, dependency, cycle,
ownership, or index diagnostic. The archive regenerated exactly the candidate index:

```text
plans/INDEX.md SHA-256
27c6a5e0c486c2485ffc3f973a77ff73bffdf68a85cfcd78a409195c48ca95a8
Executable Total: 30 plans, 144 tasks, 0 done, 144 remaining
Excluded: 2 superseded plans, 66 tasks
```

The disposable validation archive and logs were removed. No runtime state or generated
artifact remains in the worker.

### Scope and hygiene

- `git diff --check`: exit 0.
- All 193 tracked TOMLs parse with Python 3.12 `tomllib`.
- No production source, test, Cargo manifest, lockfile, canonical E/SH manifest,
  master, implementation-order file, or task status changed.
- The original sealed checkout was restored immediately after a temporary generator
  path was mistakenly created there; its pre-existing status is unchanged and no
  content from that probe remains.

## Review readiness

- Candidate implementation commit: this coherent CTRL-15 candidate commit (exact SHA
  reported to the coordinator after commit).
- Diff scope: six reconciled manifests, canonical 120-row ledger, generated index,
  backlog 02/03 current-count notices, and this evidence.
- Required reviewer focus: independently reconstruct all 120 rows; challenge the 21
  equivalence mappings; verify retained semantic identity to `ebcc3add0`; parse with
  `TasksFile`; prove scheduler edge resolution/no cycle; regenerate the 30/144 index;
  and confirm no task was prematurely completed.
- Final status: `IMPLEMENTED_UNREVIEWED`. CTRL-15 is not done until independent
  acceptance, integration, post-merge rerun, and coordinator status reconciliation.

## Integration

- Independent review evidence: pending.
- Integration commit: pending.
- Post-merge proof: pending.
- Canonical status: remains open.
