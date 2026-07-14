# Issues and self-heal state audit — 2026-07-14

## Scope and method

This audit covers every file under `issues/` and `self-heal/`: 75 issue
documents, 7 self-heal Markdown documents, and 6 executable task manifests.
It compares the issue catalogue with manifest status, the dated self-heal
audit, focused test evidence, and current source where a claim could be checked
directly. A plan that validates or a task that has precursor code is not counted
as accepted work.

Classifications mean:

- **Implemented** — the reported defect has a completed manifest task or direct
  implementation and focused regression evidence.
- **Partial** — meaningful implementation exists, but an acceptance condition
  or another defect grouped in the same document remains open.
- **Open** — no complete implementation evidence was found.
- **Superseded/stale** — historical evidence or an inventory that is useful as
  provenance, but is not a current status source.
- **Unclear** — evidence is contradictory or insufficient. No document needed
  this classification; uncertainty is represented by confidence instead.

## Overall result

| Population | Implemented | Partial | Open | Superseded/stale | Unclear |
|---|---:|---:|---:|---:|---:|
| Issue documents (75) | 6 | 21 | 46 | 2 | 0 |
| Self-heal files (13) | 1 | 7 | 5 | 0 | 0 |
| Manifest tasks (57) | 26 | 9 | 22 | 0 | 0 |

The authoritative task acceptance result is **26 of 57 done (45.6%)**. Nine
more tasks have credible partial implementation, so work has started on 35 of
57 tasks (61.4%), but partially implemented tasks must not be counted as done.
SH01 is 26/28 accepted; SH02 through SH06 remain 0/N accepted.

Confidence is high for manifest counts and the recently tested SH01/current
working-tree paths, and medium for the broad 2026-07-11 subsystem documents:
those documents group many independent findings and do not carry resolution
markers.

## Issue-document ledger

The evidence column names the decisive current signal. “Future batch” means the
mapped task remains `ready`, not that the issue lacks a plan.

| Document | State | Confidence | Evidence / remaining gap |
|---|---|---|---|
| [00-INDEX](../issues/00-INDEX.md) | Superseded/stale | High | Complete discovery inventory, but it contains no resolution ledger and still describes the 2026-07-11 snapshot as current. |
| [01 preflight re-entry](../issues/01-preflight-success-reenters-same-task.md) | Implemented | High | SH01-T02 is done; exactly-once transition regressions passed. |
| [02 seeded completed task](../issues/02-fresh-run-dispatches-seeded-completed-task.md) | Open | High | SH03-T04 remains ready. |
| [03 approval output](../issues/03-approval-tui-discards-run-output.md) | Open | High | Approval still selects `NoopSink`; SH04-T03 remains ready. |
| [04 duplicate terminals](../issues/04-run-ledger-duplicates-terminal-events.md) | Implemented | High | SH01-T03 is done with idempotent terminalization tests. |
| [05 worktree reacquisition](../issues/05-resume-cannot-reacquire-existing-worktree.md) | Partial | High | Safe same-repository/branch/tip reattachment is implemented and tested; full SH02-T04 acceptance remains ready. |
| [06 stale timeout attempt](../issues/06-timeout-reports-stale-in-flight-attempt.md) | Partial | High | Exact ownership/deadlines are implemented; SH01-T06C4 race and resume proof remains ready. |
| [07 verification mutable state](../issues/07-task-verification-depends-on-runner-state.md) | Open | Medium | Mapped persistence/seed semantics tasks remain ready. |
| [08 agent identity](../issues/08-agent-id-contract-drops-task-and-output.md) | Open | High | Structured identity SH04-T01 remains ready. |
| [09 routes zero rows](../issues/09-routes-panel-renders-zero-data-rows.md) | Partial | High | One-row layout has a real buffer regression; the enclosing SH04-T06 task remains ready. |
| [10 live tokens](../issues/10-live-token-progress-is-not-live.md) | Open | High | Estimated/final token reconciliation in SH04-T05 remains ready. |
| [11 phase display](../issues/11-active-agent-can-show-all-phases-complete.md) | Open | High | Phase invariants are not accepted; SH04-T06 remains ready. |
| [12 diagnosis feed](../issues/12-runner-v2-diagnosis-feed-is-unwired.md) | Open | High | SH04-T04 remains ready. |
| [13 max_parallel](../issues/13-plan-max-parallel-is-ignored.md) | Open | High | Audit confirms the runner still uses a global semaphore; SH02-T01 remains ready. |
| [14 invisible preflight](../issues/14-preflight-work-is-invisible-and-counted-as-dispatch.md) | Partial | High | Preflight is under exact asynchronous ownership, but durable progress/heartbeat telemetry remains open. |
| [15 blocking Git refresh](../issues/15-dashboard-refresh-runs-synchronous-full-git-diffs.md) | Open | High | SH04-T07 remains ready. |
| [16 liveness](../issues/16-no-agent-liveness-or-last-event-age.md) | Open | High | SH04-T05 remains ready. |
| [10 event loop audit](../issues/10-EVENT-LOOP.md) | Partial | Medium | SH01 fixed lifecycle/state-machine portions; blocking I/O, metrics, and other grouped findings lack complete acceptance. |
| [11 dispatch audit](../issues/11-AGENT-DISPATCH.md) | Open | Medium | Retry/output-channel findings remain in SH04/SH05 or E14 without completion proof. |
| [12 gate audit](../issues/12-GATE-PIPELINE.md) | Open | Medium | Exactly-once ownership improved, but the document’s stub rungs, parallelism, persistence, and threshold findings remain unaccepted. |
| [13 persistence audit](../issues/13-STATE-PERSISTENCE.md) | Partial | High | Wholly invalid JSONL cleanup is fixed; checkpoint, complete resume, backups, and snapshot gaps remain. |
| [14 TUI audit](../issues/14-TUI-DASHBOARD.md) | Partial | High | Output is bounded and route layout fixed; dual paths, async Git refresh, telemetry, and cleanup proof remain. |
| [15 HTTP audit](../issues/15-HTTP-SERVE.md) | Open | Medium | SSE reconnect improved outside this document’s listed defects; auth/relay/scrub/CORS findings lack acceptance. |
| [16 learning audit](../issues/16-LEARNING-FEEDBACK.md) | Open | Medium | Mapped SH05/E-series work has no completion evidence here. |
| [17 safety audit](../issues/17-SAFETY-LAYER.md) | Open | Medium | Existing-plan mapping is not proof of implementation. |
| [18 graph audit](../issues/18-GRAPH-ENGINE.md) | Open | Medium | Existing E21/E22 plans are not resolution evidence. |
| [19 CLI audit](../issues/19-CLI-COMMANDS.md) | Open | Medium | Existing E18/E37/E42 mappings have no acceptance ledger here. |
| [20 merge queue audit](../issues/20-MERGE-QUEUE.md) | Partial | High | Exact merge ownership/producer cleanup is implemented; rollback, resume redispatch, conflict and timeout findings remain. |
| [21 PRD/plan generation](../issues/21-PRD-PLAN-GEN.md) | Open | Medium | E16 mapping only; no accepted resolution evidence. |
| [22 knowledge/neuro](../issues/22-KNOWLEDGE-NEURO.md) | Open | Medium | E07/E24 mapping only; no accepted resolution evidence. |
| [23 configuration](../issues/23-CONFIG.md) | Partial | High | Dispatch-boundary model-reference/duplicate-slug validation is implemented; other listed defaults, secrets, dead fields, and env support remain. |
| [24 dreams/daimon](../issues/24-DREAMS-DAIMON.md) | Open | Medium | Existing-plan mapping only. |
| [25 duplicate types](../issues/25-DUPLICATE-TYPES.md) | Open | Medium | E03 mapping only; no consolidation acceptance evidence. |
| [26 dead code](../issues/26-DEAD-CODE.md) | Open | Medium | E12 mapping only; no removal ledger. |
| [27 agent server](../issues/27-AGENT-SERVER.md) | Open | Medium | Existing-plan mapping only. |
| [28 process supervision](../issues/28-PROCESS-SUPERVISION.md) | Partial | High | Exact process cancellation/ownership is a substantial precursor; bounded channels and deterministic end-to-end shutdown remain. |
| [29 prompt composition](../issues/29-PROMPT-COMPOSITION.md) | Open | Medium | E06 mapping only. |
| [30 filesystem/JSONL](../issues/30-FILESYSTEM-JSONL.md) | Partial | High | Atomic staging/sync and wholly invalid JSONL recovery are fixed; retention, compaction, triplication, and durable consumer gaps remain. |
| [31 E01 changes](../issues/31-E01-RECENT-CHANGES.md) | Partial | High | SH01 repairs lifecycle/resume-deadlock portions; split handling, effective concurrency, and other grouped findings remain. |
| [32 cold substrate](../issues/32-COLD-SUBSTRATE.md) | Open | Medium | E02/E24 mapping only. |
| [33 error handling](../issues/33-ERROR-HANDLING.md) | Open | Medium | SH01 ownership improvements do not establish removal of the document’s listed panics and swallowed errors. |
| [34 MCP](../issues/34-MCP-INTEGRATION.md) | Open | Medium | E15/E32 mapping only. |
| [35 cost/budget](../issues/35-COST-BUDGET.md) | Open | High | SH05-T04 remains ready. |
| [36 dependencies](../issues/36-DEPENDENCIES.md) | Open | Medium | Existing-plan mapping only; no verified dependency convergence. |
| [37 StateHub/events](../issues/37-STATEHUB-EVENTS.md) | Partial | High | Commit-before-broadcast, atomic mutation, reconnect cursor, and gap snapshot are implemented; all-consumer durability and other grouped metrics gaps remain. |
| [38 test coverage](../issues/38-TEST-COVERAGE.md) | Partial | High | Focused regressions were added and pass, but SH06 crash, connected-TUI, interruption, and self-host suites remain open. |
| [39 daemon/deploy](../issues/39-DAEMON-DEPLOY.md) | Open | Medium | Existing-plan mapping only. |
| [40 crash timeline](../issues/40-CRASH-TIMELINE.md) | Superseded/stale | High | Valuable immutable acceptance provenance, but it is a historical observation rather than an unresolved implementation item. |
| [41 plan-scoped orphaning](../issues/41-PLAN-SCOPED-PHASE-ORPHANS-CONCURRENT-TASKS.md) | Implemented | High | Canonical per-attempt state and terminalization in SH01-T01/T03 are done. |
| [42 lost gate completion](../issues/42-LOST-GATE-COMPLETION-LEAVES-DAG-RUNNING.md) | Partial | High | Lost producers now expire through exact ownership; SH01-T06C4 race/resume acceptance remains ready. |
| [43 DAG deadlock](../issues/43-NO-DAG-DEADLOCK-DETECTION.md) | Implemented | High | SH01-T04 is done. |
| [44 progress-aware timeout](../issues/44-FIXED-RUN-TIMEOUT-IS-NOT-PROGRESS-AWARE.md) | Implemented | High | Independent hard-run/scheduler-progress clocks in SH01-T06C3 are done. |
| [45 timeout snapshot ordering](../issues/45-TIMEOUT-SNAPSHOT-PRECEDES-TERMINAL-STATE.md) | Open | High | Complete post-reconciliation terminal snapshots are SH03-T01 and remain ready. |
| [46 contradictory summaries](../issues/46-RUN-SUMMARY-CONTRADICTS-PLAN-SUMMARY.md) | Open | High | SH01-T07 remains ready. |
| [47 incomplete lifecycles](../issues/47-EVENT-AND-ATTEMPT-LIFECYCLES-ARE-INCOMPLETE.md) | Partial | High | Attempt ownership/terminalization are implemented; complete idempotent runtime-ledger replay remains open. |
| [48 spawn busy loop](../issues/48-SPAWN-BUSY-LOOP-AND-LOG-STORM.md) | Open | High | SH02-T05 remains ready. |
| [49 shared worktree](../issues/49-SHARED-WORKTREE-DESTROYS-TASK-OWNERSHIP.md) | Open | High | Isolation remains plan-scoped; SH02-T02 remains ready. |
| [50 no durable commit](../issues/50-PASSED-TASK-HAS-NO-COMMIT.md) | Open | High | SH02-T03 remains ready. |
| [51 unrelated gate breakage](../issues/51-GATES-FAIL-ON-UNRELATED-CONCURRENT-BREAKAGE.md) | Open | High | Immutable task-owned gate input remains open in SH02-T02. |
| [52 timeout absent from ledger](../issues/52-TIMEOUT-ABSENT-FROM-RUN-LEDGER.md) | Partial | High | Timeout JSONL ordering is fixed and tested, but it is still ad hoc rather than a first-class idempotent runtime-ledger replay record. |
| [53 stale lock/dirty recovery](../issues/53-STALE-LOCK-AND-DIRTY-WORKTREE-BLOCK-RECOVERY.md) | Partial | High | Workspace lock truncation race and stale diagnostics are fixed; dirty quarantine/ownership recovery remains. |
| [54 retry contradiction](../issues/54-RETRY-CLASSIFICATION-AND-ATTEMPTS-CONTRADICT.md) | Implemented | High | Unified retry lifecycle SH01-T05 is done. |
| [55 invalid model config](../issues/55-INVALID-MODEL-CONFIG-CONTINUES-WITH-AMBIGUOUS-FALLBACK.md) | Partial | High | Dispatch now rejects duplicate/unresolved routing references; SH05-T01 remains ready for complete batch acceptance. |
| [56 backup cadence](../issues/56-SNAPSHOT-BACKUP-CADENCE-LOSES-RUN-HISTORY.md) | Open | High | Rotating transition checkpoints SH03-T02 remain ready. |
| [57 TUI log](../issues/57-TUI-LOG-OMITS-RUNTIME-AND-EXIT-STATE.md) | Open | High | Operational TUI logging SH04-T08 remains ready. |
| [58 atomic debris](../issues/58-ATOMIC-WRITE-DEBRIS-IS-NOT-RECOVERED.md) | Partial | High | Atomic collision/debris safety and invalid JSONL cleanup improved; comprehensive discovery/quarantine/pruning remains in SH03-T05. |
| [59 output misclassified](../issues/59-AGENT-OUTPUT-IS-MISCLASSIFIED-AS-ERROR.md) | Open | High | Nonempty stderr is still promoted to an error; SH04-T02 remains ready. |
| [60 blocked plans implementing](../issues/60-BLOCKED-PLANS-ARE-MARKED-IMPLEMENTING.md) | Open | High | No mapped accepted task or proof of queued/blocked plan-state transition. |
| [61 preflight heartbeat](../issues/61-PREFLIGHT-HAS-NO-DURABLE-HEARTBEAT.md) | Open | High | SH04-T04 remains ready. |
| [62 zero plan timestamps](../issues/62-PLAN-STATE-START-TIMESTAMPS-ARE-ZERO.md) | Open | High | No accepted plan-timing reconciliation; overlaps SH03-T01/SH01-T07. |
| [63 invalid Cargo filters](../issues/63-SELF-HEAL-VERIFY-COMMANDS-MISUSE-CARGO-FILTERS.md) | Partial | High | All observed multi-filter commands are split with `&&`; validator-level command-shape checking is still absent. |
| [64 timing/exit reconciliation](../issues/64-TASK-TIMING-AND-EXIT-RECONCILIATION-ARE-WRONG.md) | Open | High | SH01-T07 remains ready and no accepted total/phase timing invariant was found. |
| [65 Cargo environment divergence](../issues/65-AGENT-AND-GATE-CARGO-ENVIRONMENTS-DIVERGE.md) | Open | High | No canonical build-environment fingerprint/reuse proof was found. |
| [66 scrubber false positive](../issues/66-SECRET-SCRUBBER-CORRUPTS-ORDINARY-IDENTIFIERS.md) | Open | High | Source still contains `sk-[A-Za-z0-9-]+` and a test requiring `sk-short` redaction. |
| [67 LOC budget](../issues/67-TASK-LOC-BUDGET-IS-NOT-ENFORCED.md) | Open | High | `max_loc` is parsed/prompted, but no runner-owned diff enforcement/terminal disposition was found. |

## Self-heal file ledger

| Document / manifest | State | Confidence | Annotation |
|---|---|---|---|
| [COVERAGE.md](../self-heal/COVERAGE.md) | Partial | High | Good mapping for issues 01–59, but its “every issue source” claim is false: issues 60–67 are absent. |
| [README.md](../self-heal/README.md) | Partial | High | Current runbook and valid commands, but it describes an unfinished six-batch programme. |
| [changelog/AUDIT-2026-07-14.md](../self-heal/changelog/AUDIT-2026-07-14.md) | Partial | High | Accurate dated audit and focused test evidence; its explicit remaining gaps are still open. |
| [changelog/COMMIT-INVENTORY.md](../self-heal/changelog/COMMIT-INVENTORY.md) | Implemented | High | Complete provenance artifact for its stated 88-commit cutoff; it does not claim task acceptance. |
| [changelog/README.md](../self-heal/changelog/README.md) | Partial | High | Correctly separates completed, precursor, and planned work; programme remains incomplete. |
| [changelog/SH01-runner-lifecycle.md](../self-heal/changelog/SH01-runner-lifecycle.md) | Partial | High | Detailed completed history through T06C3; C4 and T07 remain non-done. |
| [changelog/SH02-SH06-planned-batches.md](../self-heal/changelog/SH02-SH06-planned-batches.md) | Partial | High | Correct 0/N acceptance counts, although later working-tree precursors now exist in several batches. |
| [SH01 tasks.toml](../self-heal/plans/SH01-runner-lifecycle/tasks.toml) | Partial | High | 26/28 tasks done; C4 and T07 remain ready. |
| [SH02 tasks.toml](../self-heal/plans/SH02-isolation-recovery/tasks.toml) | Open | High | 0/6 accepted; T04/T06 have partial implementation. |
| [SH03 tasks.toml](../self-heal/plans/SH03-persistence-integrity/tasks.toml) | Open | High | 0/6 accepted; T03/T05/T06 have partial implementation. |
| [SH04 tasks.toml](../self-heal/plans/SH04-runtime-telemetry-tui/tasks.toml) | Open | High | 0/8 accepted; T06 has partial implementation. |
| [SH05 tasks.toml](../self-heal/plans/SH05-config-dispatch/tasks.toml) | Open | High | 0/4 accepted; T01/T03 have partial implementation. |
| [SH06 tasks.toml](../self-heal/plans/SH06-regression-harness/tasks.toml) | Open | High | 0/5 accepted; crash-chain, interruption, connected-TUI, full quality, and self-host acceptance remain. |

## Per-task acceptance annotation

Manifest `status` remains authoritative. “Partial” below records reviewed
working-tree precursors without changing that status.

| Classification | Tasks |
|---|---|
| Implemented (26) | SH01-T01, SH01-T02, SH01-T03, SH01-T04, SH01-T05, SH01-T06A, SH01-T06B1, SH01-T06B2A, SH01-T06B2B1, SH01-T06B2B2, SH01-T06B2C1, SH01-T06B2C1P, SH01-T06B2C2, SH01-T06B2C3A, SH01-T06B2C3B, SH01-T06B2C3C1A, SH01-T06B2C3C1B, SH01-T06B2C3C2, SH01-T06B2C3C3, SH01-T06B2D1, SH01-T06B2D2, SH01-T06B2D3, SH01-T06B2D4, SH01-T06C1, SH01-T06C2, SH01-T06C3 |
| Partial (9) | SH01-T06C4 (lost-effect/ledger ordering), SH02-T04 (safe reattachment), SH02-T06 (lock race), SH03-T03 (timeout record), SH03-T05 (atomic/JSONL recovery), SH03-T06 (ordered StateHub/SSE resync), SH04-T06 (route/output bounds), SH05-T01 (routing validation), SH05-T03 (exact process cancellation precursor) |
| Open (22) | SH01-T07; SH02-T01, SH02-T02, SH02-T03, SH02-T05; SH03-T01, SH03-T02, SH03-T04; SH04-T01, SH04-T02, SH04-T03, SH04-T04, SH04-T05, SH04-T07, SH04-T08; SH05-T02, SH05-T04; SH06-T01, SH06-T02, SH06-T03, SH06-T04, SH06-T05 |

## Contradictions and organization gaps

1. `COVERAGE.md` says it maps every issue source, but it stops at issue 59 and
   omits 60–67 entirely.
2. `issues/00-INDEX.md` is a discovery index, not a status index. It reports the
   original findings with no fixed/partial/open annotation, so readers cannot
   infer present state from it.
3. Numbering 10–16 is overloaded between live-run documents and subsystem
   audits. Any automated mapping must use the full filename, not the numeric
   prefix.
4. Structural validation and dry-run prove that six plans and 57 tasks can be
   parsed; they do not prove verification-command semantics or implementation.
   Issue 63 demonstrates this distinction. The known malformed multi-filter
   Cargo commands are now split, but generic validator coverage remains absent.
5. The manifests correctly leave all precursor-only future-batch tasks as
   `ready`. The changelog documents those precursors, so changing them to done
   without full acceptance would make status less truthful.
6. Broad subsystem documents contain roughly 160 individual findings, while the
   execution ledger operates at 57 task granularity. A document-level “partial”
   can therefore conceal many open sub-findings; future status reporting should
   assign stable finding IDs inside those documents.

## Recommended next closure order

1. Finish SH01-T06C4 and SH01-T07, update their manifest statuses only after
   race/resume and truthful-summary acceptance pass, and rerun the full SH01
   focused suite.
2. Execute SH02 before increasing concurrency: per-plan capacity, task-owned
   worktrees, durable commits, queued wakeups, and dirty recovery are still the
   main correctness boundary.
3. Close SH03 terminal snapshots/checkpoints/ledger/seed semantics, then accept
   the already-started atomic-write and StateHub work as complete batches.
4. Finish SH04–SH05 runtime observability, dispatch, and budget enforcement.
5. Treat SH06 as the release gate. Until crash-chain, interruption/resume,
   connected-TUI, and deterministic self-host tests pass, the programme is not
   end-to-end complete even if focused unit suites are green.
6. Add issues 60–67 to `COVERAGE.md` and introduce a generated finding ledger
   with stable IDs, owning task, manifest status, last verification, and proof
   link. That will prevent plans, issue prose, and changelogs from drifting.
