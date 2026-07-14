# CTRL-08 corrected operational ownership evidence

## Candidate lineage and scope

- Corrected base: `dd611500e7f9051fbdd3843cd20c5472efcfcbb7`
- Branch/worktree: `agent/CTRL-08-ownership-dedup-r4` / `workers/CTRL-08-r4`
- Integration branch: `status-quo/integration-status-quo-20260714T073140Z`
- First rejected candidate/review: `b9387fe6c3f42209a317a301302b027a6b882042` /
  `b0e21f69f427e738a7198f43ad5d827cf0b7c486`
- Second rejected candidate/review: `ff6dc54afeccf4d06ebd95e476756d2383422205` /
  `87461143496d405a0c3a0adffa9bfa2c278f1bc6`
- Third rejected candidate/review: `ec3ecf2f89f0dd74a6c5e973c9ea4c7185bec30e` /
  `ac3cfb8439bd4223663759360ad73a7b18461419`
- Scope: relevant E01/SH/E02/E08/E09/E14/E15/E17/E18/E46/E47/E48
  manifests and epic/audit ownership prose, plus this evidence.
- Non-goals: production/tests, master, shared indexes, `plans/INDEX.md`, lockfiles,
  task completion, implementation, or integration edits.

This is a fresh candidate reconstructed on the current integration base. It does not
rewrite or discard any of the three rejected candidates or independent reviews.

## Disposition of the six rejection findings

1. **E14/E48 dependency cycle.** `E48-T02/T03` now jointly own configured live
   RPM/TPM enforcement; `E48-T05` owns provider-health outcomes and selection using
   one runtime-scoped `ProviderHealthRegistry` across real model-call and
   `CascadeRouter` paths. `E14-T08/T10` are zero-write acceptance roll-ups with a
   one-way runtime dependency on `E48-rate-limit-budgeting`. There is no E48
   dependency back to E14.
2. **Non-operative ownership metadata.** The canonical ownership document now states
   that `ownership` and `superseded_by` are audit metadata, not scheduler edges. Every
   zero-write roll-up has a supported `depends_on_plan` runtime dependency, including
   E01-T07/T11/T12. The combined top-level/backlog/self-heal graph is audited directly.
3. **Duplicate E02 serve-side persistence filter.** `E02-T08` verifies the canonical
   `roko-runtime::StateHub` classification and its live dashboard plus durable critical
   event behavior. Its contract explicitly forbids a second serve-side filter.
4. **Stale E08 disk-watcher API.** `E08-T08` now consumes E47's actual
   `React`/`DiskPressureWatcher`/`Metric::DiskFreeMb`/`ResourcesConfig` contract and
   checks conductor registration, intervention, and recovery behavior.
5. **Lost E01 aggregation outcome.** `E01-T07` explicitly rolls up both SH02 task
   worktrees/immutable gate inputs and the distinct E01-T14 accepted-task plan
   aggregation branch through `GitMergeBackend`. E01-T14 remains the sole owner of
   that plan-level aggregation outcome.
6. **Weak producer contracts.** SH05-T02 now defines the exact retry classes,
   `Retry-After`, bounded exponential jitter, configurable default of five, terminal
   events, and non-retriable status set. SH05-T04 now defines absent/unlimited budget,
   pre-dispatch exhaustion, per-dispatch checks, exact attribution, durable resume,
   and no fabricated zero-cost state. E47-T07 and E09-T10 now agree on one
   `ResourcesConfig.log_rotation_max_mb` threshold (default 100 MB), rotation under the
   serialized append boundary, complete timestamped `.jsonl` generations, a live
   unsuffixed file, and reader/GC discovery. This avoids the rejected 50 MB default and
   avoids relying on an undeclared transitive compression crate.

## Disposition of the second rejection finding

The r2 review accepted all six corrections above and found one further bounded defect:
E08-T09 still named a nonexistent `Watcher`/`WatcherOutput`/`evaluate` interface and
claimed E47-T09 produced a count even though it required only `disk_budget_remaining`.

R3 makes the producer/consumer boundary executable:

- `E47-T09` is the sole producer of `Kind::Metric` Engrams tagged
  `name=worktree_count` and `value=<usize>`. The value must come directly from
  `WorktreeManager::active_count()` at create/attach/remove/reclaim and parallel
  admission transitions. It is emitted to the conductor Engram stream on legacy
  PlanRunner and runner-v2 and cannot be inferred from `disk_budget_remaining`.
- `E47-T09` waits on `SH05-config-dispatch` before touching the runner-v2 hot spot.
- `E08-T09` is the sole pure consumer. It implements `React::decide`, reads only the
  exact tagged count Metric, and returns a `conductor.intervention` Engram tagged
  `severity=warning` when the count exceeds its configured maximum.
- `WatcherThresholds.worktree_count` / `WorktreeCountConfig.max_live` owns that
  consumer threshold, with a documented missing-config default of 8. There is no
  second worktree scan, disk accounting mechanism, or retired watcher framework.

## Disposition of the third rejection findings

The r3 review accepted the graph, preservation, E08/E47 producer-consumer, and prior
ownership corrections, then found two provider/rate defects plus a semantic TOML
placement defect.

R4 corrects all three:

- `E48-T05` no longer names `crates/roko-agent/src/dispatcher/mod.rs`. Its exact files,
  context, symbols, anti-patterns, verification, and acceptance now cover
  `ModelCallService`, legacy `dispatch_agent_with`/`run_prepared_agent`, runner-v2's
  typed event subscriber, and the same runtime-scoped `Arc<ProviderHealthRegistry>`
  used by `CascadeRouter`. ModelCallService uses a trait-typed callback so this does
  not create a roko-agent→roko-learn production dependency. Real success and typed
  429/rate-limit, 529/server-error,
  and timeout outcomes are recorded before later code-gate verdicts. Tool handler
  success/failure is explicitly excluded.
- `E14-T08` is mapped to both `E48-T02` configuration/limiter ownership and `E48-T03`
  live pooled acquisition. `E48-T02` names the live `ProviderConfig` definition;
  `E48-T03` covers legacy and runner-v2 construction plus `ModelCallService` and
  provider request acquisition. The roll-up cannot accept a dormant limiter or a
  process-global/per-agent substitute.
- Every one of the twelve acceptance roll-ups and every producer whose acceptance was
  strengthened by CTRL-08 now has `acceptance` at task scope before any
  `[task.context]`/`[[task.verify]]` table. A semantic parser check asserts
  `TaskDef.acceptance` is nonempty and no strengthened acceptance remains nested in
  a verify table.

## Canonical roll-ups and preserved distinct work

The twelve zero-write acceptance records are:

```text
E01-T07  -> SH02-T02 + E01-T14  task isolation plus plan aggregation
E01-T11  -> SH05-T04             resumable cost halt and attribution
E01-T12  -> SH05-T02             symmetric transient dispatch lifecycle
E01-T13  -> E15-T7               GitHub MCP discovery/config precedence
E01-T15  -> E47-T04              configurable pre-run disk refusal
E01-T16  -> E47-T03              policy-driven lifecycle GC
E02-T08  -> E09-T04              StateHub ephemeral/durable event policy
E08-T08  -> E47-T08              runtime disk-pressure watcher
E09-T10  -> E47-T07              episodes/signals/efficiency rotation
E14-T08  -> E48-T02 + E48-T03    configured limiter plus live pooled acquisition
E14-T10  -> E48-T05              shared-registry provider outcomes and routing
E48-T11  -> E48-T05              durable recovery and provider/gate separation
```

Each remains `ready`, writes no files, uses `quick-reviewer`, names the implementation
owner, has a runtime plan dependency, and carries equivalent-or-stronger executable
verification. Each also deserializes nonempty task-level acceptance. Distinct adapters
and consumers remain implementer work: E01-T14 plan
aggregation; E08-T09 conductor worktree pressure; E09-T11 target-size metric;
E14-T09/T12 header parsing and GitHub quota observation; E17 ACP adapters; E18 CI/docs;
and E48 retry, pooling, overrides, queueing, budget policy, outcomes, and UI.

## Scheduling proof

Cross-plan ownership is enforced by `depends_on_plan`; same-plan hot-file overlap is
serialized by `depends_on`. E17 is conservatively single-threaded because its ACP tasks
share one module. E15, E18, E46, E47, and E48 add exact local edges for shared writers.

The combined dependency audit covers top-level `plans/`, backlog plans, and self-heal
plans as one graph:

```text
plans: 93
unique meta/task cross-plan edges: 135
unresolved local references: 0
unresolved plan references: 0
strongly connected components with a cycle: 0
unordered ready-task same-file writer pairs in every changed plan with max_parallel > 1: 0
```

## Preservation and validation

An independent `tomllib` census compares every changed manifest to the corrected base:

```text
repository TOMLs: 193 parsed, 0 errors
backlog manifests: 55
self-heal manifests: 6
combined tasks: 671
status population: 33 done, 542 ready, 96 skipped (unchanged)
changed-manifest task IDs/order/status: unchanged
changed-manifest meta total/done: unchanged
roll-ups: 12; all quick-reviewer, files=[], required local/plan dependencies present,
          TaskDef.acceptance nonempty
strengthened producer tasks: TaskDef.acceptance nonempty; nested acceptance: 0
```

The semantic acceptance check used a temporary integration test importing the public
`roko_cli::task_parser::TasksFile`, parsed the eight affected manifests through the
actual Rust deserializer, and asserted all twelve roll-ups plus all fifteen
strengthened contracts had nonempty `TaskDef.acceptance`. It also inspected raw TOML
to reject acceptance nested under a verify table and asserted every roll-up remained
`quick-reviewer`, `files = []`, and runtime-dependent:

```text
cargo test -p roko-cli --test ctrl08_acceptance_semantics -- --nocapture
1 passed; 0 failed
```

The temporary test file was removed after the proof and is not part of candidate
scope.

Strict validator results are recorded after running the integration-built repository
CLI at `dd611500e7f9051fbdd3843cd20c5472efcfcbb7` against this worktree:

```text
roko plan validate --strict tmp/status-quo/backlog/plans
0 diagnostics in 55 plans; exit 0

roko plan validate --strict tmp/status-quo/self-heal/plans
0 diagnostics in 6 plans; exit 0
```

Additional release checks:

- `git diff --check`: exit 0.
- Source `plans/INDEX.md` is restored unchanged after validator regeneration; SHA-256
  `7ac5679f9ff7a32571ad0ed70e9914b579f12a5f22e4285f4804c20a19077b44`.
- No production, test, master, shared-index, lockfile, or top-level plan-index path is
  changed.

## Review handoff

- Final candidate: `1e07967a348944a4a4a0a88395e1485ad076d94a`.
- Independent review: `ACCEPTED` in `CTRL-08-REVIEW-R4.md`; review commit
  `47c9df0f2666e5316a800b5af0c0095d69b138db`.
- Integration merge: `515cbff5f71558948d53d9b67d75f1b3f892209e`.
- Immutable rejected-review evidence is retained as `CTRL-08-REVIEW.md`,
  `CTRL-08-REVIEW-R2.md`, and `CTRL-08-REVIEW-R3.md`; their original review
  commits are named above and their evidence-only transfers are integrated at
  `e853ce0a3`, `41aed0b4b`, and `77829976b`.
- Post-merge proof: the combined 93-plan universe has zero unresolved references
  and zero dependency cycles; all 12 roll-ups have real scheduler dependencies,
  zero files, `quick-reviewer`, and nonempty task-level acceptance with none nested
  under verify. Strict backlog reports `0 diagnostics in 55 plans`; strict
  self-heal reports `0 diagnostics in 6 plans`; the sealed source index is unchanged.
- Final status: `DONE` for ownership deduplication. No implementation task was
  marked complete; canonical producers and acceptance roll-ups remain `ready`.
