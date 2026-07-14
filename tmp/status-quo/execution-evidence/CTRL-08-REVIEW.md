# CTRL-08 independent review

- **Verdict:** `REJECTED`
- **Candidate:** `b9387fe6c3f42209a317a301302b027a6b882042`
- **Base:** `310ec1b2754aa87c55a3a75f0188f20e8d0feaa0`
- **Review branch:** `review/CTRL-08-b9387fe6-independent`
- **Review worktree:** `/Users/will/dev/nunchi/roko/agent-worktrees/status-quo-20260714T073140Z/reviews/CTRL-08-b9387fe6-independent`
- **Current integration checked:** `1cbca115f34420a3058c5a1d1aca62b863187b0e`
- **Review date:** 2026-07-14

## Independent method

I read the complete master checklist, the complete backlog-roadmap audit, the new
ownership matrix and worker evidence, all touched manifests and epic prose, the
E43-E48 audit, the self-heal coverage/audit, relevant current runtime/parser code,
tests, and manifest history. I inspected every changed line. I independently
exported the candidate and base with `git archive` and parsed their manifests with a
new throwaway `tomllib` census; I did not import or execute worker scripts or use
worker-generated results as review evidence.

The candidate is the direct child of the stated base. It changes exactly 24 paths:
one dated-audit notice, one new canonical ownership document, eight epic documents,
eleven backlog manifests, two self-heal manifests, and `CTRL-08.md`. It changes no
production source, tests, master, shared index, lockfile, or top-level plan index.
`git diff --check` passes.

## Independently reproduced positive controls

The immutable-archive census produced:

```text
backlog plans: 55
self-heal plans: 6
combined tasks: 671
status population: 33 done, 542 ready, 96 skipped
base/candidate ID order and status: exactly equal
acceptance roll-ups: 11; all files=[], role=quick-reviewer
task-local dependency references: 631; unresolved: 0
task-level plan references: 291; unresolved: 0
meta-level plan references: 2; unresolved: 0
tracked TOML files: 193; parse errors: 0
plans/INDEX.md SHA-256: 7ac5679f9ff7a32571ad0ed70e9914b579f12a5f22e4285f4804c20a19077b44
```

Using the integration-owned `roko 0.1.0` binary from git `d4749f9c7` against a
separate disposable candidate archive reproduced:

```text
roko plan validate --strict tmp/status-quo/backlog/plans
0 diagnostics in 55 plans

roko plan validate --strict tmp/status-quo/self-heal/plans
0 diagnostics in 6 plans
```

Those controls establish syntax, name resolution, stable counts/statuses, and sealed
index scope. They do not establish schedulability or acceptance equivalence. The
following findings prevent acceptance.

## Findings

### 1. Critical — the candidate creates an executable E14/E48 plan deadlock

`E14-T08` waits for the entire `E48-rate-limit-budgeting` plan
(`E14-providers-tools/tasks.toml:411-422`), while `E48-T05` waits for the entire
`E14-providers-tools` plan (`E48-rate-limit-budgeting/tasks.toml:331-363`). In
addition, canonical owner `E14-T10` depends on blocked roll-up `E14-T08`
(`E14-providers-tools/tasks.toml:537-549`).

The independent plan graph has one strongly connected component:

```text
[E14-providers-tools, E48-rate-limit-budgeting]
```

This is a real runtime deadlock, not a documentation concern. `TaskDef` readiness
requires every `depends_on_plan` value in `completed_plans`
(`task_parser.rs:450-466`), and the runner adds a plan to that set only after its
phase is `Complete` (`runner/event_loop.rs:8457-8469`). Neither plan can become
complete first. Downstream E14/E48 tasks consequently remain unreachable. Strict
validation does not detect this cross-plan cycle, which explains the false-green
validator result.

**Required correction:** restructure the ownership so these mechanisms induce only
one plan-level direction, or add a supported task-output dependency mechanism that
does not require completion of the mutually consuming whole plans. Add an explicit
cross-plan cycle/schedulability census and prove zero strongly connected components.

### 2. High — three SH roll-ups do not depend on their canonical producers

`E01-T07 -> SH02-T02`, `E01-T11 -> SH05-T04`, and `E01-T12 -> SH05-T02` have no
runtime dependency on their named owners (`E01-execution-engine/tasks.toml:370-380,
616-626,677-687`). They can be dispatched after only their E01-local prerequisites.

The new task-level `ownership`, `superseded_by`, and `uses_owner` keys are not fields
of `TaskDef` or `TaskDefSerde` (`task_parser.rs:49-100,115-163`) and therefore are
ignored during execution. Only `depends_on` and `depends_on_plan` gate readiness.
The worker claim that every roll-up “cannot pass before the canonical output exists”
is therefore false for these three records. A zero-write reviewer can still run
early and can pass against latent or partial code without the producer plan being
accepted.

**Required correction:** express an execution-supported, validator-recognized
dependency on the SH producers across the two plan roots and add a scheduling test,
or stop describing these records as executable producer-dependent roll-ups until
such a contract exists. Informational unknown keys alone are insufficient.

### 3. High — E02-T08's verifier requires the duplicate mechanism the matrix forbids

The matrix assigns heartbeat persistence solely to `E09-T04` in
`crates/roko-runtime/src/state_hub.rs` and expressly forbids a second serve-side
persistence filter. But roll-up `E02-T08` still requires filtering syntax in
`crates/roko-serve/src/state.rs`, `lib.rs`, or `feed_agents/mod.rs`
(`E02-STORAGE-CONVERGENCE/tasks.toml:507-542`). The canonical producer's own
verification correctly targets `roko-runtime/src/state_hub.rs`
(`E09-OBSERVABILITY/tasks.toml:201-215`).

Thus a correct canonical E09 implementation can leave E02's structural check red;
making it green as written encourages the forbidden duplicate serve-side filter.

**Required correction:** make E02-T08 review the canonical StateHub persistence
filter and its live-broadcast/durable-event tests. Remove stale serve-side
implementation requirements while preserving the original dashboard and critical
event acceptance.

### 4. High — E08-T08 preserves an incompatible, stale acceptance contract

The roll-up's acceptance still requires a `DiskSpaceWatcher` implementing a
`Watcher` trait, a `[conductor.watchers.disk_space]` config path, and
`evaluate() -> non-Continue` behavior
(`E08-conductor-supervision/tasks.toml:460-495`). The selected owner `E47-T08`
instead specifies the current `React` model, a `DiskPressureWatcher`, Engram
intervention signals, and `ResourcesConfig` thresholds
(`E47-resource-disk-management/tasks.toml:532-587`). Current conductor watchers
implement `roko_core::React`; there is no watcher implementation trait matching the
roll-up's stated API.

Only E08's verify commands were renamed to the E47 artifact; its acceptance and
context were not reconciled. The producer is therefore not equivalent-or-stronger,
and the roll-up asks a reviewer to judge two different contracts.

**Required correction:** reconcile E08-T08 context, symbols, anti-patterns, and
acceptance to the canonical React/intervention/config contract, or retain any truly
distinct configurable conductor adapter as a separately owned consumer with its own
write scope and dependency.

### 5. High — E01-T07's plan-merge outcome is not wholly superseded by SH02-T02

E01-T07 still requires per-plan `ensure_for_plan` behavior, removal of the in-place
merge fallback, and plan-branch `MergeBranch` regression gating
(`E01-execution-engine/tasks.toml:363-420`). SH02-T02 owns task-attempt worktrees and
immutable gate inputs; its acceptance does not own plan aggregation branch creation
or merge-back. The matrix separately assigns plan branch naming to `E01-T14`, but
E01-T07 neither names T14 as a producer nor depends on it.

This is not a harmless stronger review: the old per-plan isolation mechanism and
the new task-owned isolation model have different lifecycle boundaries. Mapping the
whole task only to SH02 loses a distinct merge/aggregation outcome.

**Required correction:** split/reframe E01-T07 as acceptance over both SH02 task
isolation and the E01-T14 aggregation/merge owner, or retain the merge adapter as a
distinct consumer. Its dependency and verify scope must match that decomposition.

### 6. High — additional producer contracts are weaker than their roll-ups

Two further mappings fail the required equivalent-or-stronger producer test:

- `E48-T05` requires use of the existing `ProviderHealthRegistry`, all-stage health
  filtering, half-open behavior, fallback selection, and success/failure recording.
  Owner `E14-T10` permits a generic health score and does not require that registry
  or those circuit-breaker semantics. This can create a second provider-health
  mechanism even if the deadlock is removed.
- `E09-T10` requires a configurable default 100 MB threshold and discovery of
  timestamped rotations by readers/GC. Owner `E47-T07` adds safe serialized rotation
  but its acceptance does not require reader/GC discovery, while its prerequisite
  `E47-T01` explicitly specifies a conflicting 50 MB default. The two records cannot
  both be the truth for one canonical configuration.

**Required correction:** strengthen each canonical producer's files, context,
acceptance, and verification to cover the retained roll-up contract, or preserve the
uncovered behavior as a named distinct consumer. Re-run a field-by-field comparison
for all eleven mappings rather than relying on similar titles.

## Integration compatibility

The current integration worktree was clean at
`1cbca115f34420a3058c5a1d1aca62b863187b0e`. Its base-relative changes are limited
to the master and CTRL-14 evidence, with no candidate-path overlap. The candidate
and current integration share merge base
`310ec1b2754aa87c55a3a75f0188f20e8d0feaa0`, and:

```text
git merge-tree --write-tree 1cbca115f34420a3058c5a1d1aca62b863187b0e b9387fe6c3f42209a317a301302b027a6b882042
141c0e11beb2bf43b3fbb65efebfc4e1721bd382
```

completed without a textual conflict. This proves mechanical mergeability only;
the semantic findings above prohibit integration.

## Verdict and next action

`REJECTED`. Do not merge candidate
`b9387fe6c3f42209a317a301302b027a6b882042`. Correct all six findings on a new
immutable candidate, preserve this rejection, rerun strict validation plus a real
cross-plan schedulability/cycle census, and obtain a fresh independent review.
