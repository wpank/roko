# CTRL-07 canonical ledger reconciliation

## Scope and identity

- Assignment: reconcile only the stale validation statement in
  `tmp/status-quo/backlog/06-EXECUTABLE-TASK-FILE-COVERAGE.md` after reviewed CTRL-07 integration.
- Base and integrated prerequisite snapshot:
  `206e9079812b27f738d95f91d1135d0f663c836f`.
- Branch: `agent/CTRL-07-ledger-reconciliation`.
- Worktree:
  `/Users/will/dev/nunchi/roko/agent-worktrees/status-quo-20260714T073140Z/workers/CTRL-07-LEDGER`.
- Changed paths are limited to the canonical coverage ledger and this evidence record. The master,
  indexes, manifests, production code, lockfiles, and integration worktree are out of scope and
  unchanged.

## Reconciled defect

The ledger still said whole-root non-strict validation returned six expected `PLAN_031` warnings.
That was a historical pre-CTRL-06/CTRL-07 observation, not the integrated strict-validation state.

CTRL-06 made prerequisite validation aware of dependency-created outputs. CTRL-07 candidate
`9458a6920d72e457553e31cd51b9ac89d70d2483` corrected the remaining ten stale prerequisite paths
and two producer edges. Independent final review accepted it in
`81d1af92b142ce512964b078ccb5bc1a417b8e2d`; integration merge
`206e9079812b27f738d95f91d1135d0f663c836f` contains that reviewed result. The ledger now
explicitly supersedes the six-warning statement and reports the integrated strict outcomes.

## Independent strict validation

The integration-built validator was run against the current source snapshot from a disposable
repository-shaped root. This keeps generated-index behavior outside the source worktree while
preserving the repository paths needed by prerequisite validation.

```sh
(cd "$fixture" && "$roko" plan validate --strict "$repo/tmp/status-quo/backlog/plans" --color never)
(cd "$fixture" && "$roko" plan validate --strict "$repo/tmp/status-quo/self-heal/plans" --color never)
```

Observed results:

```text
backlog:  exit 0; 0 diagnostics in 55 plans
self-heal: exit 0; 0 diagnostics in 6 plans
```

The sealed source `plans/INDEX.md` SHA-256 before and after validation was identical:

```text
7ac5679f9ff7a32571ad0ed70e9914b579f12a5f22e4285f4804c20a19077b44
```

## Ledger census

Fresh manifest-field and task-status counts at the base snapshot reproduced the ledger totals:

```text
E01-E18:                         169 tasks
E19-E45:                         243 tasks
E46-E48:                          35 tasks
implementation:                  447 tasks (7 done, 440 ready)
DOC plans:                        71 tasks (0 done, 71 ready)
implementation plus DOC:         518 tasks
authoring-gap compatibility plan: 96 tasks (96 skipped; meta status superseded)
backlog manifests:                55
self-heal manifests:               6
```

All 48 implementation epic directories and all six DOC directories contributed to the census.
The superseded authoring-gap compatibility plan is intentionally excluded from the 518 executable
implementation-plus-DOC total.

## Hygiene and handoff

- Strict validation created no source-worktree change.
- The source index hash stayed sealed.
- Final reruns reproduced both zero-diagnostic strict results and the exact task-record census above.
- `git diff --check`: exit `0`.
- Pre-commit status contains exactly the two scoped documentation paths named above.
- Independent review and integration status: pending coordinator assignment.
