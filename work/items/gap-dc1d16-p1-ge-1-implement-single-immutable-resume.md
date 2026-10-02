+++
id = "gap-dc1d16"
kind = "gap"
title = "Implement single immutable resume generation"
status = "open"
triage = "verified"
severity = "p1"
size = "L"
goal = "core"
subsystem = ["roko-graph"]
created = 2026-09-21
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "f8906b3c0"
source = "tmp/archive/CONSOLIDATED-BACKLOG-2026-09-23.md#P1-GE-1 (Subsystem: Graph Engine)"
discovered_from = "audit:tmp/archive/CONSOLIDATED-BACKLOG-2026-09-23.md#P1-GE-1 (Subsystem: Graph Engine)"
anchors = ["crates/roko-cli/src/graph_checkpoint.rs::prepare_graph_checkpoint", "crates/roko-cli/src/graph_checkpoint.rs::resume_checkpoint", "crates/roko-cli/src/graph_checkpoint.rs::GraphCheckpointManifest", "crates/roko-cli/src/graph_checkpoint.rs::invalidate_unverified_activities", "crates/roko-cli/src/graph_checkpoint.rs::write_manifest_atomic", "crates/roko-cli/src/graph_checkpoint.rs::write_cost_ledger_atomic", "crates/roko-graph/src/replay.rs::ActivityReplayer::load_scoped", "crates/roko-graph/src/replay.rs::activity_records", "crates/roko-cli/src/graph_execution/plan_runner.rs::run_one_plan"]
links = { depends_on = [], blocks = [], related = ["gap-9c82d3"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rq 'fn resume_selects_single_immutable_generation' crates/roko-cli/src && cargo test -p roko-cli --lib resume_selects_single_immutable_generation"
+++

## Problem

When `roko plan run <dir> --resume-plan` resumes, it rebuilds the run from three files. Each file is written
independently and has no shared commit point. All three live in `.roko/state/graph/<plan>/`:

- `checkpoint.json`: the manifest. It holds identity, status, extensions and receipts.
- `activities.jsonl`: an append-only record of each completed node's output.
- `costs.json`: spent and reserved provider cost.

No single "generation" says "these exact bytes of the log, this cost state and this manifest belong together".
Several things follow from that:

- A crash between writes leaves the files describing different moments. The cost ledger is written on every
  provider reservation, settle and release. Activities are appended once per completed node. The manifest is
  written on resume, on extension or receipt changes, and at finish.
- Resume mutates its own input. `resume_checkpoint` first calls `invalidate_unverified_activities`, which rewrites
  `activities.jsonl` in place and drops the records of unverified tasks (`retain_recorded_activities`). If the process
  dies again before doing any work, a second resume sees a different log. It reports no invalidated tasks, and the
  evidence from the first run is gone. Two resumes of the same checkpoint therefore do not yield the same derived
  state.
- A crash in the middle of an append leaves a torn last line. `ActivityReplayer::load_scoped` fails closed on any
  unparseable line, so resume errors out. The only way forward is `--fresh`, which archives the whole checkpoint and
  loses every completed task.
- The readers of `activities.jsonl` disagree. `recorded_gate_verdicts` (which builds the gate-verdict extension)
  silently skips bad lines, while `load_scoped` rejects them.

Expected: resume selects exactly one committed, immutable generation. It rebuilds all derived state from that
generation deterministically: replayed node outputs, invalidated tasks, gate-verdict summary and cost spent. It never
rewrites committed data in place. Anything written after the last commit, such as a torn line or an unrecorded
spend, is set aside visibly and does not block resume.

## Why it matters

- Goal `core` ("Plan runs work reliably"). Resume is part of the core loop (plan, dispatch, gate, merge, resume). A
  resume that cannot be repeated, or that dies on a torn line, makes long plan runs fragile. This matters most for
  the self-hosting proof runs.
- Related items:
  - `gap-9c82d3` (parked): the StateHub/SSE half of the original "single immutable generation" gap.
  - `gap-6ca8fb` (parked): the process-boundary crash harness.
  - `gap-a29711`: kill-point gate.
  - `gap-36f3fb`: interrupted in-flight side effects are not recorded.
  - `gap-34b2ed`: per-task spend and retry state reset on resume.
  - `bug-4641e3`: forced exit and SIGHUP do not finalize the checkpoint.

## Where

- `crates/roko-cli/src/graph_execution/plan_runner.rs::run_one_plan` (around line 1993): the entry point. It calls
  `prepare_graph_checkpoint` and hands the cost ledger to the dispatcher (`attach_plan_budget_checkpoint`), the
  recorder to `GraphEngine::with_recorder`, and the replayer to `GraphEngine::with_replayer`.
- `crates/roko-cli/src/graph_checkpoint.rs`:
  - `prepare_graph_checkpoint` (line 867): matches the fingerprint, then loads or rebinds the cost ledger.
  - `resume_checkpoint` (line 956): invalidates unverified activities, loads the scoped replayer, opens the recorder in
    append mode, and writes the manifest.
  - `GraphCheckpointManifest` (line 445): schema v3.
  - `GraphCostLedgerCheckpoint::persist` (around line 505).
  - `write_manifest_atomic` (line 1248) and `write_cost_ledger_atomic` (line 1234): each does a temp-file write
    followed by a rename.
  - `archive_checkpoint_files` (line 1213): renames the files to `*.bak.<ms>`.
  - `invalidate_unverified_activities` (line 255).
  - `recorded_gate_verdicts` (line 287).
- `crates/roko-graph/src/replay.rs`:
  - `ActivityRecorder::create` (line 64): opens in append mode.
  - `ActivityReplayer::load_scoped` (line 166): fails closed on foreign, duplicate or unparseable records.
  - `retain_recorded_activities` (line 314): rewrites the log through a `.compact` temp file and a rename.
- `crates/roko-cli/src/graph_task_dispatch.rs` (lines 485-560): the budget ledger's reserve, settle and release. Each
  one calls `GraphCostLedgerCheckpoint::persist`.
- Not on this path: `crates/roko-graph/src/snapshot.rs` (`GraphSnapshotV2`, `GraphEngine::resume_from`). Plan runs do
  not use it, and `roko-cli` has no reference to it.

## Current state

- Re-checked 2026-09-29 at HEAD `a17d9d766`. No generation concept exists anywhere in `graph_checkpoint.rs`,
  `graph_execution/` or `roko-graph/src/replay.rs`.
- `3d0637232` moved the checkpoint to schema v3: an authored-plan fingerprint, migration from the legacy fingerprint,
  and a ledger-first rebind so that a crash between the ledger write and the manifest write stays resumable. That
  is one ad hoc ordering rule. There is no general commit protocol.
- The blocker, P0-GE-1 (crash/resume equivalence), was done on 2026-09-20. `crates/roko-graph/tests/crash_resume_equivalence.rs`
  is an in-process harness: a 3-node linear DAG where the middle node fails with an injected error. It proves that
  replay skips completed nodes. It does not cover torn writes or disagreement between files.
- Origin of the term: the old GAPS.md "tranche 4" said "StateHub baseline/overlay rebasing, cursor-atomic
  SSE/typed capture, and a single immutable resume generation remain open". The refactoring audit (P1-05)
  restated it as "resume after crash should reconstruct exact pre-crash derived state; requires deterministic
  replay from checkpoint". This item is the Graph-checkpoint half. The StateHub half is `gap-9c82d3`.

## Plan

1. Define the invariant in a doc comment on `GraphCheckpointManifest`. The manifest is the only commit point. A
   generation `N` names a byte length (or record count) plus a BLAKE3 hash of the committed prefix of
   `activities.jsonl`, and the cost values at `N`. Resume uses exactly generation `N`.
2. Choose the storage design. Recommended: option A.
   - **A. Generation-stamped manifest.** Bump to schema v4. Add `generation: u64` and
     `committed: { activity_bytes, activity_blake3, spent_micro_usd, reserved_micro_usd }`. Commit order for every
     durable change: append and fsync the activity record, write the cost ledger, then write the manifest with
     `N+1` last. This is the smallest change and keeps the file paths that TUI, serve, diagnose and
     `canonical_checkpoint_status` read. It needs a hook so that a manifest commit follows each `ActivityRecorder`
     write. Either pass a callback into the recorder, or commit from the dispatcher after each node.
   - **B. Generation directories.** Use `gen-<N>/{checkpoint.json,activities.jsonl,costs.json}` plus a `CURRENT`
     pointer that is swapped atomically, and copy forward on resume. This is easy to reason about, but it moves
     paths that many readers use.
   - **C. One log.** Fold the cost and extension changes into `activities.jsonl` as records, and make the manifest a
     cache. This is the most principled option, but it is the biggest change to the `roko-graph` replay format.
3. On resume with option A: read the manifest. Treat bytes past `committed.activity_bytes` as uncommitted. Move them
   to `activities.jsonl.uncommitted.<ms>` with a `tracing::warn!`; do not fail. Take the cost values from the
   generation, or check that the `costs.json` stamp matches it.
4. Make invalidation non-destructive. Do not rewrite committed bytes. Record the invalidated `(node_id, tick)` keys in
   the manifest at generation `N+1`, for example `invalidated: BTreeSet`. Have the replayer skip them;
   `load_scoped` already skips non-replayable verdicts. The evidence then survives repeated resumes.
5. Have `recorded_gate_verdicts` and `load_scoped` read the same committed prefix with the same parsing rules.
6. Migration: a v3 manifest without `generation` becomes generation 0, with the committed prefix set to the current
   log length after dropping a torn trailing line. Keep reading `MIN_SUPPORTED_SCHEMA_VERSION = 2`.
7. Tests in `crates/roko-cli/src/graph_checkpoint.rs` (module `tests`):
   - `resume_selects_single_immutable_generation`: create a checkpoint, record node A, commit, then append node B
     plus a torn line without committing. Resume and assert: the replayer has only A, spent equals the committed
     value, the committed bytes of the log are unchanged, and the extra bytes are in an `.uncommitted.*` file.
   - A second test: resume twice with no work in between and assert that both produce identical derived state
     (`replayed_entries`, invalidated set, gate-verdict extension, spent).

## Done when

- A torn trailing line in `activities.jsonl` no longer blocks `plan run --resume-plan`. The uncommitted bytes are set
  aside with a warning.
- Resuming the same checkpoint twice produces identical derived state, and resume never rewrites committed bytes.
- Cost spent after resume equals the value committed with the selected generation.
- A v3 checkpoint from before this change still resumes.
- `grep -rq 'fn resume_selects_single_immutable_generation' crates/roko-cli/src && cargo test -p roko-cli --lib resume_selects_single_immutable_generation`
  passes.

## Notes

- Risk: this is persistence and migration code. Do not break reading of existing checkpoints. The v2-to-v3 migration
  (`migrate_v2_to_v3`) and legacy-fingerprint rebind must keep working, and a failed resume must never delete data.
  Archive (rename), never remove.
- Several readers parse these files. They include `canonical_checkpoint_status` (used by `plan_set.rs`), the resume
  preview (`preview_plan_resume`), `commands/plan.rs`, `commands/util.rs`, `plan.rs`, `worker/cloud.rs` and
  `roko-serve/src/projection_contract.rs`. Some of those matches may be other files with the same name. Any path or
  format change must keep them working. Grep for `checkpoint.json`, `activities.jsonl` and `costs.json` before
  changing either.
- `graph_checkpoint.rs` changes often (it had concurrent edits on 2026-09-28). Do not run this in parallel with
  other items that touch `graph_checkpoint.rs` or `replay.rs`, such as `gap-36f3fb`, `gap-34b2ed` and `bug-4641e3`.
- The StateHub/SSE generation (`gap-9c82d3`) is out of scope. It can later reuse the `generation` number.
- Size is L: design, a schema bump, a commit hook across `roko-graph` and `roko-cli`, migration and tests.
- 2026-10-01 (wk-tamper): PARTIAL on work/gap-7147bb; cargo verification deferred to the batch check. Still open at
  ebdc0f5d5. First step (Plan steps 3 and 6, without the generation stamp): a record is committed once its line ends.
  `resume_checkpoint` now calls `roko_graph::replay::set_aside_uncommitted_activities` before anything reads the log.
  The bytes after the last newline, a record whose write did not finish, move to `activities.jsonl.uncommitted.<ms>`
  with a warning, and the log is cut back to its last complete record. A torn line therefore no longer blocks
  `--resume-plan`, and the resumed run appends on a line of its own. The resume preview replays the same committed
  prefix without changing a file (`committed_activity_len`, `ActivityReplayer::load_scoped_committed`). Tests:
  `resume_sets_aside_a_torn_activity_record` (roko-cli) and `a_torn_last_record_is_set_aside_and_never_replayed`
  (roko-graph).
  Still open:
  - The generation stamp itself (steps 1-2): schema v4 with `generation`, the log's committed length and BLAKE3, and
    the cost values, written last after each durable change, so the log, the cost ledger and the manifest name one
    moment.
  - Non-destructive invalidation (step 4): `invalidate_unverified_activities` still rewrites the log.
  - One parser for `recorded_gate_verdicts`, `recorded_nodes`, `recorded_outputs` and `load_scoped` (step 5).
  - `resume_selects_single_immutable_generation` and the repeated-resume test (step 7).
  - Hot Graph resume (`hot.rs`) still fails closed on a torn line.
- 2026-10-02 (wk-tamper): implemented on work/gap-7147bb; cargo verification deferred to the batch check. Step 1
  of the remainder, the roko-graph half (Plan steps 4 and 5). `replay::activity_records` is the one parser of a
  durable Activity log: each record with its line number, blank lines skipped, and an unparseable line or a record
  of another graph or run an `InvalidData` error. `ActivityReplayer::load_scoped` reads through it, and
  `ActivityReplayer::from_records` leaves out records refused by line number, so a resume can refuse a record
  without rewriting the log. `ActivityRecorder::with_commit_hook` hands the host each record's bytes once they are
  synced, and `set_aside_activities_after` sets aside a log's bytes past a committed length. Tests in `replay.rs`:
  `activity_records_number_every_line_and_fail_closed`, `records_refused_by_line_stay_in_the_log_and_never_replay`,
  `the_commit_hook_sees_each_durable_record`, `set_aside_after_keeps_exactly_the_committed_prefix`. The roko-cli
  half (the generation stamp, the readers and the item's own tests) is the next step.
- 2026-10-02 (wk-tamper): implemented on work/gap-7147bb; cargo verification deferred to the batch check. Step 2,
  the rest of the item (Plan steps 1-7):
  - The manifest is the commit point (schema v4). Each write names the next `generation`, and under `committed` the
    Activity log's committed length and BLAKE3, the spent and reserved cost, and the lines a resume refused.
  - `CheckpointCommits` is shared by the recorder, the cost ledger and the run's own manifest writes, and writes the
    manifest last: after each synced record (the recorder's commit hook), after each `costs.json` write, and on each
    manifest change.
  - Resume uses exactly that generation, checked by `select_generation` before any file changes. It fails closed
    when committed bytes changed. It sets aside the log's bytes past them, and a cost the ledger holds beyond the
    committed one, as `*.uncommitted.<ms>`. It refuses unverified records by line number instead of rewriting the
    log, so a second resume derives the same state. The spend after resume is the generation's.
  - A v2 or v3 manifest resumes as generation 0: the log's complete records and the ledger's cost.
  - One parser, `replay::activity_records`, serves every reader: the replayer, the resume preview, the gate-verdict
    summary, the interrupted-attempt check, and `inspect_canonical_checkpoint` (`roko diagnose`, `backlog audit`).
    The inspection also reports the committed cost.
  - A forced exit marks an open checkpoint interrupted through its commit point, so a later commit keeps the status.
  - `retain_recorded_activities` is gone. The backlog audit test fixture now names its plan as the records' graph,
    as real logs do.
  - Tests in `graph_checkpoint.rs`: `resume_selects_single_immutable_generation`, `resuming_twice_derives_the_same_state`,
    `a_v3_checkpoint_resumes_as_generation_zero`, `resume_fails_closed_when_committed_records_change`,
    `a_commit_after_a_forced_exit_mark_keeps_the_interrupted_status`.
  Every Done-when is implemented; the cargo gate decides. Unchanged, outside the Done-when:
  - A generation that committed an unresolved reservation (a crash during a provider call under a plan budget)
    still refuses resume, as the ledger did.
  - Manifest writes are still renames without an fsync.
  - roko-serve's `graph_task_commit` (`routes/plans/merge.rs`) still reads the whole log leniently.

## Original notes

Implement single immutable resume generation. Resume after crash should reconstruct exact pre-crash derived state. Requires deterministic replay from checkpoint. Depends on P0-GE-1.

Imported without verification from:
- `tmp/archive/CONSOLIDATED-BACKLOG-2026-09-23.md#P1-GE-1 (Subsystem: Graph Engine)`
- `tmp/archive/refactoring-audit-2026-09-21/IMPLEMENTATION-CHECKLIST.md#p1-high-blocks-self-hosting-significant-tech-debt`

How to verify: Confirm against code: Resume after crash should reconstruct exact pre-crash derived state. Requires deterministic replay from checkpoint. Depends on P0-GE-1. Its blocker P0-GE-1 is marked done 2026-09-20, so it may now be unblocked. / Check whether resume selects exactly one immutable generation and rebuilds derived state deterministically.

Merged 2 mined candidates: m1-171, m2-172.

Verified 2026-09-28: there is no generation concept in crates/roko-graph/src/snapshot.rs, crates/roko-cli/src/graph_checkpoint.rs (which has uncommitted concurrent edits) or graph_execution/. Resume combines the graph checkpoint, Activity log, cost state and extensions without selecting one immutable generation, and CLAUDE.md still lists single-generation resume as partial. P0-GE-1's in-process harness exists (crates/roko-graph/tests/crash_resume_equivalence.rs). This overlaps the second half of gap-9c82d3.

Rechecked 2026-09-29 at d9e79e9d8: still no generation. 3d0637232 moved the Graph checkpoint to schema v3 with an authored-plan fingerprint and legacy-fingerprint migration (graph_checkpoint.rs:23, :44-47). The manifest and cost ledger are each written atomically (write_manifest_atomic at :613, :857, :993; write_cost_ledger_atomic at :511, :534), but independently of activities.jsonl, so resume still combines separately written files rather than selecting one immutable generation.
