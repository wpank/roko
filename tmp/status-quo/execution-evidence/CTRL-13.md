# CTRL-13 combined execution-universe dependency evidence

## Assignment and scope

- Control item: `CTRL-13`, confirm zero unresolved plan IDs for the chosen
  execution universe.
- Immutable base: `bb5048f4c1d0e3f34155e89d39ccef46109c3b59`.
- Branch/worktree: `agent/CTRL-13-execution-universe` / `workers/CTRL-13`.
- Chosen resolution universe: the exact union of all tracked manifests at:
  - `plans/*/tasks.toml` — 32 retained top-level manifests;
  - `tmp/status-quo/backlog/plans/*/tasks.toml` — 55 manifests; and
  - `tmp/status-quo/self-heal/plans/*/tasks.toml` — six manifests.
- Reserved write scope: this evidence file only.
- Explicit non-goals: no master, manifest, index, production, test, status,
  ownership, completion, integration, or `CTRL-15` reconciliation edit.

I read the complete master, the CTRL-03, CTRL-04, CTRL-08, and CTRL-12
implementation/review chains, and all 93 manifests in the chosen union. The
manifest proof was rebuilt from an immutable `git archive` of the assigned base;
no prior worker census or generated index was reused.

## Exact immutable population

Python's standard `tomllib` parsed the complete bytes of each manifest and all
other tracked TOML files in the archive. A second check used the repository's
public Rust `roko_cli::task_parser::TasksFile::parse` API against every one of the
93 manifests.

```text
manifests: 93 = 32 top-level + 55 backlog + 6 self-heal
unique [meta].plan IDs: 93
tasks: 881 = 210 top-level + 614 backlog + 57 self-heal
tracked TOMLs: 193 parsed, 0 errors
duplicate plan IDs: 0
duplicate task IDs within a plan: 0
meta.total mismatches: 0
```

To bind the census to the exact bytes, SHA-256 was computed over each sorted
`relative_path + NUL + raw_bytes + NUL` stream:

```text
complete 93-manifest union: ffcdbc95017beb0a340947d754d4328db713b8abd67f75b1bfdbd797762a64e4
top-level 32 manifests:     6bf80483d6103260f17d8d7e3a4bc923bd6ad24e7507fe2875722aa57005591a
backlog 55 manifests:       9dfea56c135a6bf12a01269a987b9d54d60ac8be11169cfbdc1a92db3d75dd54
self-heal 6 manifests:      26af4b091a5558920ec403d31774345e50b1cb2efaaebaf22aa8706087257811
```

Task states in the literal union are:

| Root | Plans | Tasks | Done | Ready | Skipped |
|---|---:|---:|---:|---:|---:|
| retained top-level | 32 | 210 | 0 | 210 | 0 |
| backlog | 55 | 614 | 7 | 511 | 96 |
| self-heal | 6 | 57 | 26 | 31 | 0 |
| **union** | **93** | **881** | **33** | **752** | **96** |

Plan metadata has 90 `ready` and three `superseded` plans. For 92 plans,
`meta.done` equals the number of `done` tasks. The one deliberate alternate
terminal encoding is `status-quo-authoring-gaps`: `meta.done = 96` and all 96
tasks are `skipped`, as proved by CTRL-14.

The retained index's 120-task executable ledger is also arithmetically intact:
P08-P34 contains 27 plans/115 tasks and the two named side queues contain five
tasks (`architecture-defi-critical-path` three, `e2e-smoke` two), giving 29
plans/120 tasks. The separate recovered `architecture-core-queue` contributes 24
additional top-level tasks to this CTRL-13 resolution universe. This census does
not reconcile, complete, or supersede any of those 120 tasks and makes no
`CTRL-15` completion claim.

## Independent graph census

The graph census treated every exact `[meta].plan` as a node. It checked every
task `depends_on` against that task's containing manifest, and every declared
`depends_on_plan` against the 93-node union. It included both task-level runtime
edges and the two meta-level control-plane declarations. Tarjan's algorithm ran
over all nodes and unique plan edges, including isolated plans.

```text
same-plan task references:       848
unresolved same-plan references:   0

task depends_on_plan references: 296
meta depends_on_plan references:   2
all declared plan references:     298
unresolved plan references:         0
unique directed plan edges:       135
cyclic strongly connected components: 0
```

The 298 raw plan references route as follows:

| Source root -> target root | Raw references | Unique edges |
|---|---:|---:|
| backlog -> backlog | 268 | 111 |
| backlog -> self-heal | 5 | 3 |
| backlog -> retained top-level | 11 | 10 |
| self-heal -> self-heal | 14 | 11 |

No top-level manifest declares an outgoing plan edge. The two meta declarations
are `E46-github-workflow-integration -> E01-execution-engine` and
`E48-rate-limit-budgeting -> E01-execution-engine`. Current `TaskMeta` does not
deserialize a meta-level dependency field, so these two are control-plane graph
declarations, not runtime task gates. The other 296 references deserialize into
`TaskDef.depends_on_plan` and are enforced by
`TaskDef::is_ready_with_plan_deps`/`TaskDag` against `completed_plans`.

The CTRL-08 roll-ups are present in this same proof. Eleven have explicit
cross-plan runtime edges; `E48-T11` has the intentional same-plan dependency on
`E48-T10`. All 12 therefore have a scheduler-recognized dependency, and all their
target IDs are included in the zero-unresolved result.

## Real-parser reproduction

A temporary integration test compiled the assigned base and called the public
`TasksFile::parse` API for the exact 32+55+6 roots. It independently asserted
unique parsed plan/task IDs, `meta.total`, every same-plan reference, and the
task-level plan-reference count:

```text
cargo test -p roko-cli --test ctrl13_real_parser -- --nocapture
TASKSFILE_PARSE_OK roots=32+55+6 plans=93 tasks=881 local_refs=848 task_plan_refs=296
test parses_complete_ctrl13_execution_universe ... ok
test result: ok. 1 passed; 0 failed
```

The temporary test and its isolated 10.0 GiB target were removed immediately
after the run and are not part of candidate scope.

## Resolution is not completion or file readiness

Zero unresolved IDs proves only that every dependency string names an existing
node in the chosen union. It does not put that plan in `completed_plans`, satisfy
the 848 task prerequisites, mark any of the 752 ready tasks done, prove required
files exist, or authorize dispatch. Current scheduler source requires task-local
dependencies to be completed and every task-level plan dependency to occur in
`completed_plans` before readiness.

Strict validation makes the file-prerequisite boundary visible. The
integration-owned CLI reported version git `915d3c246` and SHA-256
`389d7f5e0f7e6ff67851a6339a4228081bc74f4e56d885389f58d294a4d5a677`;
its parser, validator, and task-DAG source is byte-unchanged through the assigned
base. Against the disposable immutable archive:

```text
tmp/status-quo/backlog/plans:
  exit 0; 0 diagnostics in 55 plans

tmp/status-quo/self-heal/plans:
  exit 0; 0 diagnostics in 6 plans

plans:
  exit 1; 32 plans checked; 0 errors; 94 warnings
  diagnostic classes: PLAN_031 = 94
  architecture-core-queue: 93 missing file prerequisites
  self-dev-ux:               1 missing file prerequisite
```

Thus all 32 retained manifests parse, but the retained root is not represented as
strict-green. The 94 `PLAN_031` results are missing-file prerequisites, not plan-ID
failures. In particular, the active separate architecture queue remains gated on
its historical inputs despite having a resolvable graph identity.

## Superseded execution semantics

All three superseded manifests remain in the 93-plan resolution universe so their
historical IDs and internal references can be audited:

| Plan | Task state | Execution disposition |
|---|---:|---|
| `self-dev-ux` | 55 ready | superseded; never execute raw |
| `self-dev-extras` | 11 ready | superseded; never execute raw |
| `status-quo-authoring-gaps` | 96 skipped | superseded by canonical per-epic plans |

Their 162 tasks are excluded from executable accounting, leaving 90 active plans
and 719 non-superseded tasks (33 done, 686 ready). No declared plan reference enters
or leaves any of the three superseded nodes, so including their IDs for complete
resolution does not satisfy or create an active dependency.

This exclusion is an execution-selection rule, not an inferred parser guarantee.
`plans/INDEX.md` explicitly excludes superseded plans and the master says they must
never execute; current broad task discovery does not itself filter on
`meta.status`. Therefore the raw `plans/` directory must not be dispatched as one
undifferentiated queue. CTRL-13 proves the union graph and records the exclusion; it
does not authorize raw execution of superseded manifests.

## Sealed index, source hygiene, and lineage

All validation ran in the disposable archive because `plan validate` regenerates
the top-level index. The archive index changed to its expected generated hash
`27c6a5e0c486c2485ffc3f973a77ff73bffdf68a85cfcd78a409195c48ca95a8`.
The source and immutable-base `plans/INDEX.md` remained byte-identical at the
reviewed sealed SHA-256:

```text
7ac5679f9ff7a32571ad0ed70e9914b579f12a5f22e4285f4804c20a19077b44
```

No source manifest or index differs from the assigned base. The archive,
temporary test, generated runtime files, and isolated target were removed.

The accepted integration merges for CTRL-03 (`4ae834b797fa`), CTRL-04
(`06e1d4404785`), CTRL-08 (`515cbff5f715`), and CTRL-12 (`1e478eaf1a6e`)
are all ancestors of the assigned base. Their historical counts are reconciled by
the fresh current census above rather than assumed.

## Review readiness

- Candidate: the evidence-only commit containing this file; exact SHA reported at
  handoff.
- Candidate diff: exactly `tmp/status-quo/execution-evidence/CTRL-13.md`.
- `git diff --check`: required before commit.
- Required independent review: export the immutable candidate, independently parse
  all three roots, reproduce 93/881/848/298/135/0, trace runtime versus meta edges,
  rerun all three strict roots, verify the three superseded exclusions and 120-task
  ledger arithmetic, and confirm the sealed index/worktree scope.
- This is an implementation/evidence candidate only. It does not mark CTRL-13 or
  CTRL-15 complete; review, merge, post-merge proof, and coordinator status remain.

## Integrated disposition

- Independent review: accepted by
  `7971c328103405255c844df4e4dba5b0794e2d43`; integrated review commit
  `5d30a1986` follows integrated candidate commit `3b40cd389`.
- Post-merge census at `5d30a1986` includes the already integrated CTRL-09 DOC
  roll-up. It reports 93 plans, 881 tasks, 849 same-plan references, 320
  task-level plan references, two meta-level plan references, 160 unique runtime
  task edges, 162 unique all-declared edges, zero unresolved references, and zero
  cyclic SCCs. The count increase from this candidate's immutable-base census is
  entirely attributable to CTRL-09; resolution remains closed.
- All 193 tracked TOMLs parsed. Disposable strict validation reported zero
  diagnostics in 55 backlog plans and zero diagnostics in six self-heal plans.
  The source `plans/INDEX.md` remained sealed at SHA-256
  `7ac5679f9ff7a32571ad0ed70e9914b579f12a5f22e4285f4804c20a19077b44`.
- The shared test target currently identifies the review candidate because the
  review's public-parser build rewrote the binary. Production parser/validator
  source is byte-identical, so the post-merge semantic result is valid; CTRL-11
  binary provenance must be rebuilt at the current integrated code SHA before a
  release claim.
- Final status: `DONE`. This proves identifier resolution only; CTRL-15 retains
  ownership of the 120-task disposition ledger.
