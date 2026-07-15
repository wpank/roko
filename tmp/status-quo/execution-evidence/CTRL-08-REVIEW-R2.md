# CTRL-08 corrected-candidate independent review

- **Verdict:** `REJECTED`
- **Candidate:** `ff6dc54afeccf4d06ebd95e476756d2383422205`
- **Corrected base:** `1cbca115f34420a3058c5a1d1aca62b863187b0e`
- **Prior rejected candidate:** `b9387fe6c3f42209a317a301302b027a6b882042`
- **Prior rejection:** `b0e21f69f427e738a7198f43ad5d827cf0b7c486`
- **Current integration checked:** `128dc950c`
- **Review branch:** `review/CTRL-08-ff6dc54a-independent`
- **Review date:** 2026-07-14

## Independent method and scope

I read the complete master checklist, both prior candidate and rejection records,
the corrected worker evidence, the complete ownership matrix and dated audit, every
touched manifest and ownership-prose change, and the relevant current scheduler,
conductor, StateHub, provider-health, and routing code. I inspected the exact
base-to-candidate diff and reconstructed the graph/count checks without using a
worker script or worker-generated archive.

The candidate is the direct child of its stated base and changes exactly 24 paths:
13 manifests, eight epic documents, one dated-audit notice, the new ownership
matrix, and worker evidence. It changes no production source, tests, master,
shared index, lockfile, or top-level plan index. `git diff --check` passes.

## Disposition of the six prior findings

1. **E14/E48 plan cycle: corrected.** The combined 93-plan graph has 134 unique
   meta/task plan edges and zero cyclic strongly connected components. E14-T08 and
   E14-T10 wait one-way on `E48-rate-limit-budgeting`; no E48 task or plan waits on
   E14. There is no E14/E48 SCC or any other cycle.
2. **Runtime producer gating for roll-ups: corrected.** All eleven
   `acceptance-roll-up` records are `ready`, `files = []`, `quick-reviewer`, have a
   named owner, and have at least one scheduler-recognized `depends_on_plan`.
   E01-T07/T11/T12 now wait on SH02/SH05. Current `TaskDef` and `TaskDag` confirm
   that `depends_on_plan` gates readiness until the producer plan is complete.
3. **E02 persistence filter: corrected.** E02-T08 is a zero-write review of
   E09-T04 at `roko-runtime::StateHub`. Its context, checks, and acceptance preserve
   live broadcast and durable critical events while expressly forbidding a second
   serve-side filter.
4. **E08 disk watcher: corrected.** E08-T08 now reviews E47-T08's actual
   `React::decide`/intervention-Engram contract and canonical `ResourcesConfig`
   thresholds. It explicitly rejects the nonexistent `Watcher/evaluate` API and
   the duplicate `[conductor.watchers.disk_space]` schema.
5. **E01 plan aggregation: corrected.** E01-T07 waits locally on E01-T14 and at
   plan level on SH02. Its acceptance separately covers task-owned worktrees,
   immutable gates, the deterministic plan aggregation branch, `GitMergeBackend`,
   and the post-merge regression gate.
6. **Producer strength: corrected for the cited contracts.** SH05-T02 now specifies
   429/529/connection-timeout retry, `Retry-After`, bounded jitter, configurable
   five-attempt default, non-retriable 400/401/403/config failures, and exactly one
   terminal event per attempt. SH05-T04 covers absent/unlimited budget, pre-first
   and repeated pre-dispatch checks, exact attribution, durable resume state, and
   no fabricated zero-cost records. E48-T05 and E14-T10 consistently require the
   existing `ProviderHealthRegistry`, all-stage Open filtering, HalfOpen probing,
   healthy fallback, explicit all-unavailable failure, and outcome recording.
   E47-T07/E09-T10 consistently require the sole 100 MB
   `ResourcesConfig.log_rotation_max_mb`, serialized line-safe rotation, complete
   timestamped JSONL generations, a live unsuffixed file, and reader/GC discovery.

## Independently reproduced controls

An independent `tomllib` and graph census produced:

```text
tracked TOMLs parsed: 193; errors: 0
combined graph manifests: 93
unique meta/task plan edges: 134
unresolved local task references: 0
unresolved plan references: 0
cyclic strongly connected components: 0
backlog + self-heal status: 33 done, 542 ready, 96 skipped
changed manifests: 13
changed ID order/status drift: 0
changed meta plan/total/done/status drift: 0
acceptance roll-ups: 11; malformed: 0
unordered same-file ready pairs in changed max_parallel > 1 plans: 0
plans/INDEX.md SHA-256 in candidate: 7ac5679f9ff7a32571ad0ed70e9914b579f12a5f22e4285f4804c20a19077b44
```

Using the integration-owned `roko` binary built from git `128dc950c` against a
fresh disposable `git archive` of the immutable candidate reproduced:

```text
roko plan validate --strict tmp/status-quo/backlog/plans
0 diagnostics in 55 plans; exit 0

roko plan validate --strict tmp/status-quo/self-heal/plans
0 diagnostics in 6 plans; exit 0
```

The validator's generated-index side effect occurred only inside the disposable
archive, which was removed; the candidate worktree stayed clean and retained the
sealed index. Mechanical integration compatibility also passes:

```text
git merge-tree --write-tree 128dc950c ff6dc54afeccf4d06ebd95e476756d2383422205
7a7a1ae0e6d8251aa847c6d5b6525c7fde2f846d
```

This proves clean syntax, counts, graph schedulability, local writer ordering,
scope, and textual mergeability. It does not make the following retained consumer
contract implementable.

## Finding

### High - touched E08-T09 consumes neither a real conductor API nor its declared producer output

The candidate newly classifies E08-T09 as a distinct consumer of E47-T09 and adds
the plan dependency, while the matrix and corrected E08 prose retain it as the
worktree-pressure consumer. Its executable contract was not reconciled:

- `E08-conductor-supervision/tasks.toml:545-560` requires a `Watcher` trait,
  direct `WatcherOutput`, and non-Continue/evaluate semantics. Current
  `crates/roko-conductor/src/watchers/mod.rs:1-6` says every watcher implements
  `roko_core::React`, and `ghost_turn.rs:7,83-84` confirms the live contract is
  `React::decide(&[Engram], &Context) -> Vec<Engram>`. There is no conductor
  `Watcher` implementation trait. `WatcherOutput` is an internal policy projection,
  not the watcher interface.
- `E08-conductor-supervision/tasks.toml:523-525,565` requires a
  worktree-count Metric Engram supplied by a runner or adapter. The corrected epic
  prose claims E47-T09 produces that signal. But
  `E47-resource-disk-management/tasks.toml:644-671` requires only aggregate disk
  estimates, pressure serialization, and a `disk_budget_remaining` metric. It never
  requires or verifies a live-worktree-count signal.

Consequently, completing the declared E47 owner still leaves E08-T09 without its
input, and implementing E08-T09 literally would introduce the same retired watcher
API that the corrected E08-T08 contract explicitly prohibits. The candidate's
claims that distinct consumers use the canonical owner and that the E08 watcher
contract is reconciled are therefore not yet true.

Reproduction:

```text
rg 'trait Watcher|impl Watcher' crates/roko-conductor/src
# no matches

rg 'worktree_count|worktree-count' \
  tmp/status-quo/backlog/plans/E47-resource-disk-management/tasks.toml
# no matches
```

Expected: the retained distinct consumer names the live `React` API and has an
explicit canonical producer for the exact Metric Engram it consumes.

Actual: it names a nonexistent API and a producer that emits a different metric.

## Required correction

Do not merge `ff6dc54afeccf4d06ebd95e476756d2383422205` as accepted CTRL-08 work.
On a new immutable candidate:

1. Reconcile E08-T09's acceptance, context, symbols, anti-patterns, and verification
   to `React::decide` and intervention Engrams, just as E08-T08 now does. Preserve
   Warning semantics and the intended configurable worktree threshold without
   introducing a second watcher framework.
2. Give the worktree-count Metric Engram one executable producer. Either strengthen
   E47-T09's files/description/acceptance/tests to emit and verify that exact metric
   on the live runner path, or map E08-T09 to another named canonical task whose
   contract already does so. Do not claim that `disk_budget_remaining` is a count.
3. Update the matrix, E08 prose, and worker evidence to match that exact producer,
   then rerun the existing graph, status, strict-validation, scope, and integration
   compatibility proofs and obtain fresh independent review.
