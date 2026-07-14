# CTRL-16 implementation-order reconciliation evidence

## Assignment

- Plan: `tmp/status-quo/MASTER-EXECUTION-CHECKLIST.md`, Wave 0 `CTRL-16`.
- Base SHA: `3a4e57f02efa47f3106f54969799c34486b3ed7b`.
- Branch/worktree: `agent/CTRL-16` / `workers/CTRL-16`.
- Integration branch: `status-quo/integration-status-quo-20260714T073140Z`.
- Reserved scope: `plans/_meta/IMPLEMENTATION_ORDER.md`,
  `tmp/status-quo/73-EXAMPLES-PLANS-GRAPHS.md`,
  `tmp/status-quo/backlog/02-PLANS-RECONCILIATION.md`, and this evidence.
- Candidate commit: `CANDIDATE_SHA_REPORTED_AFTER_COMMIT`.

## Requirement

Original defect: the imported implementation-order file still grouped
`dry-run-flag`, `live-demo-phase1`, and `live-demo-phase2` with the runnable
standalone queues even though Git history shows that `7899494d` deleted all
three manifests. At the same time, its terse architecture statement did not
record that CTRL-01/CTRL-05 had recovered and verified the real 24-task
`architecture-core-queue`. Two current-looking status-quo documents retained
baseline plan counts and side-queue language without an exact current
disposition for the removed roots.

Expected behavior:

- every root presented as runnable has a tracked, non-empty, parseable
  `plans/<root>/tasks.toml`;
- the recovered architecture queue remains a separate 24-task executable plan,
  and the three architecture-DeFi parity rows continue to resolve to its Q14;
- the deleted dry-run and live-demo roots are explicitly historical,
  non-runnable, absent from the index, and not recreated;
- history and semantic boundaries remain clear: related execution-honesty work
  is not mislabeled as a task-for-task dry-run replacement, and `e2e-smoke` is
  not mislabeled as equivalent to greeting/farewell demo tasks;
- no task/status/count, manifest, generated index, ownership ledger, source, or
  lockfile changes.

Dependencies: CTRL-01's canonical import (`699df4e0e`), accepted review
(`c19bd3016`), and merge (`01c00546b`); CTRL-05's architecture reconciliation
and accepted proof; and CTRL-15's corrected ownership/index integration through
base `3a4e57f02`.

Explicit non-goals: implementing a dry-run feature; executing or changing any
task; manufacturing absent directories; changing manifests, the master,
`plans/INDEX.md`, `EXECUTION-OWNERSHIP.md`, production code, Cargo metadata, or
remote/external state; or rewriting historical baseline bodies as though they
were authored against the current tree.

## Reproduction and history proof

At the base, `plans/_meta/IMPLEMENTATION_ORDER.md` said all four names were
standalone side/demo queues and directed phase 1 before phase 2. Filesystem and
Git evidence disagreed:

```text
tracked non-empty current roots:
  architecture-core-queue         ready  24 tasks
  architecture-defi-critical-path ready   3 tasks
  e2e-smoke                       ready   2 tasks

absent roots:
  dry-run-flag
  live-demo-phase1
  live-demo-phase2
```

`git show --name-status 7899494d --` records deletion of all three absent
manifests. Their parent versions contain ten proposed workflow dry-run tasks,
two synthetic greeting tasks, and two synthetic farewell tasks respectively.
The same commit also deleted the architecture queue, but CTRL-01 recovered its
manifest byte-identically from five sealed sources and the historical Git blob;
CTRL-05 then verified Q14 and its three DeFi consumers. The current architecture
manifest is tracked, non-empty, and has SHA-256
`3f90263abd24f1b937a882244e3c67290a1580bb11ebe724c6d399d0741d3fe5`.

## Implementation

- Made current count and authority boundaries explicit in the implementation
  order: generated index for counts, master for dependency order, and ownership
  ledger for the sealed 120-task mapping.
- Kept all current primary-queue names and order unchanged.
- Documented the exact current architecture-core, architecture-DeFi, and
  `e2e-smoke` roots with task counts and dependency semantics.
- Moved the three absent names into a historical-removal table keyed to
  `7899494d`, with explicit non-equivalence/supersession boundaries and a rule
  against execution or placeholder recreation.
- Clarified that the already-complete W01/P06/P07 names are historical labels,
  not current runnable roots.
- Added narrowly scoped current-control notices to the two baseline inventory
  documents; their dated bodies remain preserved for provenance.

No plan or task semantics changed. Failure/recovery/security behavior is
unaffected because this candidate changes documentation only. The safety
property is operational: absent plan names can no longer be mistaken for valid
execution inputs.

## Verification

The candidate verification contract is:

```text
1. Python tomllib parses every tracked TOML: 193/193, zero errors.
2. Current top-level manifest census: 32 manifests; 30 ready executable plans
   with 144 tasks and two superseded plans with 66 excluded tasks.
3. Runnable-root assertions: architecture-core-queue,
   architecture-defi-critical-path, and e2e-smoke are tracked/non-empty and
   parseable; their task counts are 24/3/2.
4. Historical-root assertions: dry-run-flag, live-demo-phase1, and
   live-demo-phase2 have no current tasks.toml and are not index rows.
5. Q14 resolution: exactly one Q14 task and exactly three DeFi source_ref
   consumers, all resolving to it.
6. Disposable strict validation: backlog 0 diagnostics/55 plans, self-heal
   0 diagnostics/6 plans.
7. A disposable `plans` generator run reproduces tracked plans/INDEX.md exactly:
   SHA-256 27c6a5e0c486c2485ffc3f973a77ff73bffdf68a85cfcd78a409195c48ca95a8.
8. Source plans/INDEX.md is byte-unchanged and `git diff --check` passes.
```

Exact command output is recorded below after running those checks on the final
candidate tree.

```text
TOML_OK tracked=193 parse_errors=0
PLAN_CENSUS_OK manifests=32 executable=30/144 superseded=2/66
RUNNABLE_ROOTS_OK architecture-core-queue=24 architecture-defi-critical-path=3 e2e-smoke=2
HISTORICAL_ROOTS_OK dry-run-flag/live-demo-phase1/live-demo-phase2 absent_and_unindexed
Q14_RESOLUTION_OK anchor=1 consumers=3
HISTORY_BLOBS_OK dry-run-flag=10 live-demo-phase1=2 live-demo-phase2=2 architecture-core-queue=24
IMPLEMENTATION_ORDER_ROOT_SET_OK current_ready_roots=30
CTRL16_LINK_TARGETS_OK targets=8

backlog strict:  exit 0; 0 diagnostics in 55 plans
self-heal strict: exit 0; 0 diagnostics in 6 plans
top-level strict generator run: exit 1; 94 diagnostics in 32 plans
top-level diagnostic census: 94 PLAN_031 and no other PLAN code
generated index SHA-256: 27c6a5e0c486c2485ffc3f973a77ff73bffdf68a85cfcd78a409195c48ca95a8
source index before/after: 27c6a5e0c486c2485ffc3f973a77ff73bffdf68a85cfcd78a409195c48ca95a8
git diff --check: exit 0
```

The top-level `PLAN_031` result is the existing bounded census of intended
future architecture-source outputs, not a parse, root-resolution, dependency,
or index error. All validation/generator invocations ran from fresh `git
archive` trees under `/tmp`; their generated indexes and `.roko` records were
removed with those temporary trees. The source index stayed unchanged. The
validator binary was the integrated binary reporting Git `7303d2f87`; CTRL-15
changed only control-plane documents, so its parser/generator behavior is the
reviewed behavior used for the current 30/144 index.

## Review readiness

- Candidate implementation commit: `CANDIDATE_SHA_REPORTED_AFTER_COMMIT`.
- Diff scope: exactly the four reserved documentation/evidence paths.
- Known limitations: the historical dry-run proposal has no equivalent current
  task-level owner. This change records that gap truthfully; it does not invent
  a supersession or authorize feature work.
- Required reviewer focus: reconstruct the three deleted blobs from Git,
  challenge every current/historical mapping, verify all named runnable roots,
  reproduce Q14 resolution, TOML/strict/index gates, and confirm no status,
  count, manifest, ownership, or generated-index change.

## Integration

- Review evidence: pending independent review.
- Integration commit: pending.
- Post-merge commands/results: pending integration-owner verification.
- Final status: `IMPLEMENTED_UNREVIEWED`.
