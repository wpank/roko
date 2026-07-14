# CTRL-04 external plan dependency resolution evidence

## Assignment

- Task: `CTRL-04` — resolve the 11 backlog dependencies on retained
  P08/P09/P16/P19/P22/P23/P25/P28 plans.
- Base SHA: `0e6c4cd81938df9cc0f18638242402cb4e53dfab`.
- Branch: `agent/CTRL-04-external-dependency-resolution`.
- Worktree:
  `/Users/will/dev/nunchi/roko/agent-worktrees/status-quo-20260714T073140Z/workers/CTRL-04-EXTERNAL-DEPS`.
- Integration branch: `status-quo/integration-status-quo-20260714T073140Z`.
- Reserved write scope: this evidence file only.
- Explicit non-goals: changing the master, manifests, task or plan status, generated/shared
  indexes, production code, validator behavior, execution ownership, or the integration worktree.

## Requirement and outcome

CTRL-03 deliberately preserved 11 exact `depends_on_plan` references whose targets were outside
`tmp/status-quo/backlog/plans`. The chosen canonical execution universe is the union of that
backlog root and the retained top-level `plans/` root. A dependency is resolved for CTRL-04 only
when its exact string equals an actual `[meta].plan` ID in that union and the target owns a real,
internally coherent task set.

The existing reviewed imports already satisfy that requirement, so this is an evidence-only
resolution candidate. It does **not** complete any imported P-plan task or make any dependent
consumer executable ahead of its prerequisites. All 38 target-plan tasks remain `ready`, zero are
`done`, and their consumers must still wait for reviewed plan completion or an explicit reviewed
supersession mapping.

## Integrated provenance

Two already-integrated reviewed changes jointly establish the result:

1. CTRL-03 candidate `ace630cebebc0b00aadcb60e8b5af3414ccadf88`, accepted in
   `1c0fd5cc0dd1c9857c5734c589283cbaaff0d6ad` and merged by
   `4ae834b797fac4bf3be61714418388b2012e4206`, canonicalized all internal backlog IDs while
   preserving the exact 11-reference external multiset.
2. CTRL-01 import `699df4e0ea34bddabc4516695d28d1bf41328774`, accepted in
   `c19bd30160443759f96d8fef6149cc9b146a5bde` and merged by
   `01c00546bc57a485ff53553d0fe53006afa8ed42`, restored the retained manifests byte-for-byte from
   the sealed recovery source.

Both merge commits are ancestors of the assigned base.

## Exact resolved dependencies

An immutable-snapshot TOML census independently reproduced the CTRL-03 multiset at its merge and
at the current base. No reference was aliased, rewritten, or removed:

| Consumer task | Exact `depends_on_plan` target |
|---|---|
| `E04-security-perimeter/E04-T06` | `P16-safety-contracts` |
| `E04-security-perimeter/E04-T14` | `P22-acp-tool-permission` |
| `E07-learning-knowledge/E07-T09` | `P19-cascade-router-acp` |
| `E16-prd-self-hosting-gaps/E16-T1` | `P08-search-command-fix` |
| `E16-prd-self-hosting-gaps/E16-T2` | `P09-tool-alias-fix` |
| `E16-prd-self-hosting-gaps/E16-T2` | `P23-prd-pipeline-fix` |
| `E17-acp-completion/E17-T01` | `P22-acp-tool-permission` |
| `E17-acp-completion/E17-T02` | `P19-cascade-router-acp` |
| `E17-acp-completion/E17-T03` | `P25-mcp-acp-passthrough` |
| `E17-acp-completion/E17-T04` | `P22-acp-tool-permission` |
| `E17-acp-completion/E17-T05` | `P28-image-support` |

Every target string exactly equals both its imported directory name and its manifest's
`[meta].plan`. All same-plan `depends_on` values in the imported targets resolve to task IDs in
that target's actual task set.

## Imported task-set and recovery identity

The current files reproduce every SHA-256 recorded by the independent CTRL-01 import review.
Exact byte identity also proves that no plan metadata, task ID, dependency, or status has changed
since import.

| Imported plan | Actual task IDs | State | Reviewed recovery SHA-256 |
|---|---|---|---|
| `P08-search-command-fix` | `T1`–`T4` | 0/4 done; plan/tasks ready | `e2406f0dbbf1ecc436d7c2de32faabdc0419a0e326b32d3a7efaf6ead2689991` |
| `P09-tool-alias-fix` | `T1`–`T3` | 0/3 done; plan/tasks ready | `d3ded0c373224b458920122e924463557e7b0c4f795593808d43b3559fb489a7` |
| `P16-safety-contracts` | `T1`–`T5` | 0/5 done; plan/tasks ready | `fc63e6addac2631d909d5f7c371b1f614c93c79f61e12bc48a5390b1974c7ce2` |
| `P19-cascade-router-acp` | `T1`–`T6` | 0/6 done; plan/tasks ready | `29f202968a6566fcf55824d5ed7aaf275ac71a18043f01aa8c5183d165fa8892` |
| `P22-acp-tool-permission` | `T1`–`T5` | 0/5 done; plan/tasks ready | `9ea7c6b3a1cc77b094ad2e1ca05ae1c18b488619920aa14a49ed4035ebaaea1a` |
| `P23-prd-pipeline-fix` | `T1`–`T6` | 0/6 done; plan/tasks ready | `5b465603f8115b1b10a7a28508ae7a227147558b735162cf26dedff16b9886cc` |
| `P25-mcp-acp-passthrough` | `T1`–`T4` | 0/4 done; plan/tasks ready | `821a0a3dc72405aef894e1c12617a885aea8a3e7d73dced58b4b1af0d946f0a5` |
| `P28-image-support` | `T1`–`T5` | 0/5 done; plan/tasks ready | `15baf1d1a198cabe00d681ddef950523d5ff718377cc2d0e21bda57fc25b8dc6` |

Aggregate target state: eight plans, 38 tasks, zero done, 38 ready.

## Immutable parser and ID census

Both the current base and the CTRL-03 merge were exported with `git archive` to disposable
directories. Python's standard `tomllib` parsed the snapshots and asserted unique plan/task IDs,
`meta.total == len([[task]])`, same-plan dependency existence, exact external multiset equality,
target hashes, and target statuses.

```text
ALL_TOML_PARSE parsed=193 errors=0
UNION_OK backlog=55 retained=32 unique_plan_ids=87 unresolved_cross_plan=0
CTRL03_EXTERNAL_MULTISET_OK refs=11 unchanged_at_current=true
TARGET_TOTAL plans=8 tasks=38 done=0 ready=38
```

The zero-unresolved result is an ID-resolution statement for the chosen union. It is not a claim
that all plan tasks are complete, that every file prerequisite is available, or that the broader
CTRL-13/CTRL-15 ownership and completion gates are closed.

## Validator verification and known separate diagnostics

The integration-built prerequisite-aware validator (`roko 0.1.0`, Git build identifier
`d4749f9c7`) was run only against disposable exports of the assigned base:

```text
backlog strict:                 0 diagnostics in 55 plans
P08-search-command-fix strict: 0 diagnostics in 1 plan
P09-tool-alias-fix strict:     0 diagnostics in 1 plan
P16-safety-contracts strict:   0 diagnostics in 1 plan
P19-cascade-router-acp strict: 0 diagnostics in 1 plan
P22-acp-tool-permission strict: 0 diagnostics in 1 plan
P23-prd-pipeline-fix strict:   0 diagnostics in 1 plan
P25-mcp-acp-passthrough strict: 0 diagnostics in 1 plan
P28-image-support strict:      0 diagnostics in 1 plan
```

For completeness, strict validation of the entire retained root currently exits 1 with 94
`PLAN_031` file-prerequisite diagnostics: 93 belong to `architecture-core-queue` historical source
paths and one belongs to superseded `self-dev-ux`. There are no other plan-diagnostic classes in
that run, and none belongs to the eight CTRL-04 targets. Those file-prerequisite findings are not
cross-plan ID failures and are outside this evidence-only assignment; they are not hidden or
represented as green.

All validator commands ran in disposable archives because validation regenerates
`plans/INDEX.md`. The disposable copy changed as expected; the source index remained byte-identical
before and after at SHA-256
`7ac5679f9ff7a32571ad0ed70e9914b579f12a5f22e4285f4804c20a19077b44`, and the source worktree
remained clean.

## Navigation decision

No navigation edit is required for CTRL-04:

- `plans/INDEX.md` already lists each of the eight exact imported IDs once, with the correct totals
  and ready state.
- `plans/_meta/IMPLEMENTATION_ORDER.md` already names each exact ID once in the primary queue.
- `tmp/status-quo/backlog/plans/00-INDEX.md` intentionally indexes only its own backlog root; the
  master cross-wave ledger is the navigation bridge to retained plans.

Changing the generated/historical plan index would overlap CTRL-15, and repairing unrelated stale
side-queue names in the implementation-order document belongs to CTRL-16. Neither is necessary to
resolve these 11 exact dependencies, so this candidate leaves all navigation files untouched.

## Review readiness and integration

- Candidate implementation: the evidence-only commit containing this file; its immutable SHA is
  reported at handoff to avoid a self-referential commit.
- Diff scope: exactly this evidence file.
- Required reviewer focus: independently rebuild the union ID set from immutable commits, compare
  the 11-reference multiset with CTRL-03, verify all eight recovery hashes/task sets/statuses, and
  confirm that dependency resolution has not been described as task completion.
- Candidate: `c0be145d077b3989e6644bd9f0ca49823ce4da85`.
- Independent review: `ACCEPTED` in `CTRL-04-REVIEW.md`; review commit
  `b4661477763fbf1721bfb47ca1f6580a29ab6e63`.
- Integration merge: `06e1d4404785b7d5c1fadcdd40740b89a8fe04b4`.
- Post-merge verification: the immutable union census reports 87 plan IDs, 264
  cross-plan references, zero unresolved references, and the exact eight targets with
  38 ready/zero done tasks. Strict backlog validation reports `0 diagnostics in 55
  plans`; each of the eight imported target plans reports `0 diagnostics in 1 plan`.
  The sealed source index hash remains unchanged and `git diff --check` passes.
- Final status: `DONE` for external ID resolution only. The 38 imported tasks remain
  active work under their own canonical owners; this status does not supersede or
  complete them.
