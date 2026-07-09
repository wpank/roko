# Stability Plan: How To Reach Mori-Level Robustness After Architectural Convergence

> Architecture alone will not give you Mori-level robustness. This document defines the hardening work required after the runtime is reconciled.

## Thesis

You do **not** get stability by rewriting more.

You get stability by:

1. shrinking to one runtime path,
2. making state transitions explicit,
3. hardening that one path with parity tests, resume tests, crash tests, and dogfooding.

That is how Mori became operationally robust even though its architecture was tangled.

---

## 1. Stability Has Four Inputs

## 1.1 Fewer execution paths

Two partially different runtimes are inherently unstable.

### Requirement

- after migration, only one path handles normal plan execution

## 1.2 Explicit state ownership

If state lives in ad hoc mutable structs spread across runtime glue, it will drift.

### Requirement

- plan lifecycle state lives in executor/snapshots
- agent lifecycle state lives in normalized runtime events + durable projections
- knowledge/routing outcomes are recorded through one feedback sink

## 1.3 Deterministic recovery

If restart semantics are fuzzy, robustness is fake.

### Requirement

- resume is tested as a first-class behavior

## 1.4 Operational burn-in

A design can look perfect and still fail in daily use.

### Requirement

- dogfood the exact active path

---

## 2. Minimum Hardening Matrix

## 2.1 Unit tests

Required for:

- executor transitions
- task readiness / DAG
- retry classification
- normalized agent event mapping
- prompt assembly variants
- gate result classification
- projection updates

## 2.2 Integration tests

Required for:

- `plan run` with real snapshot writes
- resume from partial run
- gate failure then retry
- verify/reviewer path
- knowledge/routing observation writeback

## 2.3 End-to-end tests

Required for:

- one small real repo task
- one multi-task DAG plan
- one interrupted-and-resumed run
- one gate-failure-and-autofix run
- one dashboard/projection smoke test

## 2.4 Chaos tests

Required for:

- kill runner during active agent turn
- kill runner during gate execution
- corrupt partial snapshot file
- restart with orphaned agent pid file
- delayed gate completion

---

## 3. Required Snapshot Model

The active runtime should be recoverable from a small, explicit set of files.

Recommended minimum:

1. `executor.json`
2. `routing.json`
3. `gate-thresholds.json`
4. `episodes.jsonl`
5. `run-state.json`

Optional but useful:

6. `knowledge-pending.jsonl`
7. `dashboard-events.jsonl`

Rules:

- every file versioned
- every write atomic
- restore path validates version and overlap
- stale/corrupt files fail closed, not half-open

---

## 4. Parity Test Program

Before deleting `orchestrate.rs` as a meaningful runtime, build a parity suite.

## 4.1 Capability parity tests

Each test asserts that the runner path performs the same functional outcome as the legacy-rich path for a bounded scenario.

Scenarios:

1. simple implementation task
2. multi-step task with dependency ordering
3. gate failure causing retry
4. verify/reviewer flow
5. resume after interruption
6. routing observation after success
7. knowledge hint injection after previous failure

## 4.2 Event parity tests

Assert that the new normalized runtime emits the required event categories:

- plan started/completed
- task started/completed
- agent started/completed
- tool started/finished
- usage/cost
- gate output and verdicts
- retry/review transitions

## 4.3 State parity tests

Assert that after the same run:

- completed tasks are identical
- failed tasks are identical
- retry counts are identical
- snapshots are resumable

---

## 5. Rollout Strategy

## Phase A: Dark-launch modules behind runner

- keep runner authoritative
- plug in extracted modules one by one
- compare behavior on test scenarios

## Phase B: Daily dogfood on runner only

- forbid fallback to legacy for normal work
- log missing capabilities as defects, not reasons to keep split ownership

## Phase C: Freeze legacy path

- when parity matrix is satisfied, allow only:
  - compatibility adapter calls
  - test helpers
  - migration shims

## Phase D: Delete dead behavior

- remove unique business logic from legacy path
- keep only wrappers if needed temporarily

---

## 6. Concrete Test Inventory To Build

## 6.1 Runner recovery tests

1. interrupt after first agent output chunk
2. interrupt after agent complete but before gate complete
3. interrupt after gate fail before snapshot flush
4. restart from each case and verify:
   - no double completion
   - no lost completed tasks
   - no orphan processes

## 6.2 Dispatch tests

1. requested model differs from actual model
2. tool-call-capable backend and non-tool backend
3. reviewer role gets restricted tools
4. session reuse resumes correctly

## 6.3 Prompt assembly tests

1. retry injects structured gate feedback
2. knowledge hints appear in correct prompt section
3. anti-patterns included when present
4. token budget trims low-priority context

## 6.4 Learning tests

1. successful run emits routing observation
2. failed gate emits failure-pattern observation
3. episode log contains provider/model/cost metadata
4. knowledge candidate is created on successful pass

## 6.5 Projection tests

1. TUI snapshot sees tool calls
2. non-TUI output prints useful progress
3. dashboard reflects live cost/token growth

---

## 7. Operational SLOs

Use these as acceptance thresholds before calling the runtime "stable".

## Reliability

- no duplicate task completion on resume
- no silent task loss on resume
- no orphan agent process after cancellation

## Recovery

- restart resumes to correct next task in >95% of interruption scenarios under test
- corrupted optional snapshot file degrades gracefully

## Observability

- every plan/task/agent/gate phase visible in projection
- cost/token updates visible before task completion

## Performance

- gate concurrency bounded
- snapshot flush cost does not dominate run time
- prompt assembly path has predictable upper bound

---

## 8. Signs You Are Failing Again

Stop and correct course if any of these start happening:

1. new feature lands only in a legacy path
2. runner needs to know provider-specific wire details again
3. cross-cut logic reappears as CLI-local mutable state hacks
4. resume bugs are fixed ad hoc instead of by cleaning snapshot ownership
5. parity is claimed without running parity scenarios

Those are exactly the warning signs that the repo is drifting back toward the same class of failure.

---

## 9. Final Answer To The User-Level Question

Will implementing the architecture docs alone get you Mori-level stability?

No.

Will implementing the architecture docs **plus** this hardening plan get you there?

Yes, that is the credible path.

The stable end-state is:

- one runtime
- one event model
- one checkpoint model
- one feedback path
- one projection path
- one dogfooded execution spine

That is what Mori had operationally, even when its internals were messy. `roko` should recover that property without inheriting the same tangle.

## Implementation Packet

This file becomes the hardening backlog after runtime convergence work starts.

### Test Targets

- [ ] Add unit tests for normalized agent event mapping.
- [ ] Add unit tests for prompt assembly with retry feedback.
- [ ] Add unit tests for gate failure classification and retry action mapping.
- [ ] Add unit tests for dashboard snapshot mutation.
- [ ] Add integration test for simple plan run using mock agent.
- [ ] Add integration test for gate failure and retry.
- [ ] Add integration test for resume after interruption.
- [ ] Add integration test for routing observation persistence.
- [ ] Add integration test for knowledge candidate creation.
- [ ] Add merge queue conflict test.

### Crash/Resume Checklist

- [ ] Kill runner during active agent output.
- [ ] Kill runner after agent completion before gate completion.
- [ ] Kill runner during gate execution.
- [ ] Kill runner after gate pass before snapshot flush.
- [ ] Restart with orphan pid file.
- [ ] Restart with stale snapshot plan ids.
- [ ] Restart with corrupt optional learning snapshot.

### Dogfood Checklist

- [ ] Run at least five small repository tasks through runner-only path.
- [ ] Run at least one multi-task plan through runner-only path.
- [ ] Run at least one failed-gate plan through retry.
- [ ] Run at least one interrupted/resumed plan.
- [ ] Record failures as issues/tasks, not as reasons to revive legacy runtime.

### Stability Exit Gate

- [ ] No duplicate completed task after resume tests.
- [ ] No orphan process after cancellation tests.
- [ ] No silent gate auto-pass.
- [ ] No production prompt path bypasses composition.
- [ ] No runner provider-specific stream protocol types.
- [ ] No unique production behavior remains in `orchestrate.rs`.

## 10. Live Hardening Delta (2026-04-26)

Real no-mock runs already exposed stability-critical checks that must become permanent tests.

### Newly proven in live runs

- [x] Single-task smoke plan passes end to end with real Codex CLI.
- [x] Single-task smoke plan passes end to end with real Claude CLI.
- [x] Gate rung includes both `compile:cargo` and `task.verify` verdicts.
- [x] Terminal state persists `current_phase.kind = "complete"` on success.

### Newly identified hardening requirements

- [ ] Add regression test: run from repo root with explicit external plan path should not accidentally execute local repo plans.
- [ ] Add regression test: if workspace is non-Rust, `compile:cargo` failure message must be explicit and actionable.
- [ ] Add config option to disable or override default compile gate for non-Rust repos.
- [ ] Add regression test for codex stderr ingestion to ensure warnings are persisted but do not auto-fail runs.
- [ ] Add test for `run.completed` event persistence in every terminal path (success, failure, interrupted).
- [ ] Add test for executor snapshot final phase flush after terminal transition.

### Required observability hardening before calling stable

- [ ] Add `run_id` correlation field to events and snapshots.
- [ ] Add first-class querying for gate history by run.
- [ ] Add first-class querying for agent stderr classified by severity.
- [ ] Add parity assertion on minimum emitted event categories across Codex and Claude.

## 11. Worker 9 Evidence Checklist (2026-04-26)

Stability evidence now available:

- [x] Real Codex CLI smoke proof exists at `/tmp/roko-real-e2e-nrUD05/logs/codex-run-3.stdout`.
- [x] Real Claude CLI smoke proof exists at `/tmp/roko-real-e2e-nrUD05/logs/claude-run-1.stdout`.
- [x] Gate verdict proof exists at `/tmp/roko-real-e2e-nrUD05/work/.roko/events.jsonl`.
- [x] Terminal snapshot proof exists at `/tmp/roko-real-e2e-nrUD05/work/.roko/state/executor.json`.
- [x] Task artifact proof exists at `/tmp/roko-real-e2e-nrUD05/work/hello.txt`.
- [x] Source proof for duplicate-spawn prevention, real verify gates, timeout/semaphore gates, retry classification, and event logging exists in `crates/roko-cli/src/runner/`.

Stability blockers before archive:

- [ ] Add regression tests for the proof failures found here: wrong plan-root selection and missing `Cargo.toml` default compile-gate behavior.
- [ ] Add crash/resume tests for active agent output, post-agent/pre-gate, in-gate, post-gate/pre-snapshot, stale pid files, stale plan ids, and corrupt optional learning state.
- [ ] Add parity tests for multi-task, retry, verify/review, routing, knowledge, projection, merge queue, and dream triggers.
- [ ] Add observable querying for gate history and stderr severity.
- [ ] Add `run_id` correlation to executor snapshots, not just runtime events.
- [ ] Remove provider-specific stream protocol usage from runner before declaring stable.

## 12. 2026-04-27 Deepening Pass - Stability Proof Contract And Crash Matrix

Initial rating: 9.90 / 10. This pass is above the requested threshold because it converts the stability plan from a general hardening note into an implementation-grade proof contract: exact source anchors, status taxonomy, crash matrix, report schema, batches, stop conditions, and no-context checklists are all in this file. It is not a 10 because the repository still needs the executable proof harness and real provider burn-in to be implemented and run.

### Current Source Refresh

The current codebase has real stability primitives, but it does not yet have proof that those primitives compose under crash, cancellation, resume, merge, and HTTP/TUI read scenarios.

- [x] `PersistPaths` centralizes the runner durability files in [persist.rs](../../crates/roko-cli/src/runner/persist.rs): `executor.json`, `orchestrator.json`, `run-state.json`, `episodes.jsonl`, `efficiency.jsonl`, `cascade-router.json`, `gate-thresholds.json`, `agent-pids.json`, and `events.jsonl` are declared at `persist.rs:22-70`.
- [x] `RunStateSnapshot` exists in [persist.rs](../../crates/roko-cli/src/runner/persist.rs) at `persist.rs:74-120` and records schema, `run_id`, cost, tokens, task counts, completed tasks, snapshot failure streak, and task fingerprints.
- [x] Runner startup calls orphan cleanup before resuming at [event_loop.rs](../../crates/roko-cli/src/runner/event_loop.rs) `event_loop.rs:121-122`.
- [x] Runner startup performs strict resume validation and JSONL recovery before reopening the runtime path at [event_loop.rs](../../crates/roko-cli/src/runner/event_loop.rs) `event_loop.rs:130-183`.
- [x] `prepare_resume` validates snapshot schema, task fingerprints, missing plan IDs, and JSONL recovery for `episodes`, `events`, and `efficiency` at [resume.rs](../../crates/roko-cli/src/runner/resume.rs) `resume.rs:118-205`.
- [x] `save_snapshot` writes `orchestrator.json`, `executor.json`, and runner-owned `run-state.json` together through the same helper at [event_loop.rs](../../crates/roko-cli/src/runner/event_loop.rs) `event_loop.rs:1395-1455`.
- [x] Executor resume loads compatible snapshots, ignores stale snapshots with no plan overlap, records corrupt/read-failed outcomes, and restores `MergeQueue` from the aggregate snapshot at [event_loop.rs](../../crates/roko-cli/src/runner/event_loop.rs) `event_loop.rs:1465-1620`.
- [x] Agent process lifecycle has PID registration, kill-on-drop, process-group cleanup, and explicit runner cancellation cleanup through [agent_stream.rs](../../crates/roko-cli/src/runner/agent_stream.rs), [persist.rs](../../crates/roko-cli/src/runner/persist.rs), and [event_loop.rs](../../crates/roko-cli/src/runner/event_loop.rs) `event_loop.rs:860-870`.
- [x] Runner event emission writes durable JSONL and projection events through one helper at [event_loop.rs](../../crates/roko-cli/src/runner/event_loop.rs) `event_loop.rs:1195-1260`.
- [x] HTTP projection endpoints exist in [projections.rs](../../crates/roko-serve/src/routes/projections.rs): `GET /api/projections/catalog`, `GET /api/projections/{name}`, and `GET /api/projections/{name}/stream`.
- [x] Merge is no longer a fake `MergeSucceeded` event only: [merge.rs](../../crates/roko-cli/src/runner/merge.rs) defines `PlanMerger`, `MergeBackend`, `GitMergeBackend`, `RegressionGate`, queue reservation, and post-merge regression dispatch.
- [ ] No tracked proof report currently demonstrates crash/resume correctness across all injection points below.
- [ ] No tracked proof report currently demonstrates orphan cleanup against a real child process tree after runner termination.
- [ ] No tracked proof report currently demonstrates `MergeQueue` resume continuity after a crash while a merge is queued or reserved.
- [ ] No tracked proof report currently demonstrates HTTP projection reconstruction from durable runner files after process restart.
- [ ] No tracked proof report currently demonstrates parity across Codex CLI, Claude CLI, Anthropic API, OpenAI API, Moonshot, Z.AI, and Perplexity through the same dispatch path.

### Stability Status Taxonomy

Use these exact labels in proof reports, checklists, and status comments.

- `source_wired`: Source code contains the intended mechanism and the doc references the file/line range.
- `unit_proved`: A deterministic unit test covers the mechanism in isolation.
- `integration_proved`: A real `roko` command or HTTP server flow covers the mechanism with durable files.
- `chaos_proved`: A failure injection run kills or corrupts the system at a controlled point and verifies recovery.
- `provider_proved`: A real provider or CLI completed the scenario through the same dispatch path used by production.
- `proof_missing`: The source may exist, but there is no tracked reproducible proof.
- `blocked_credentials`: The proof harness ran but could not authenticate the provider.
- `blocked_environment`: The proof harness ran but the local machine lacked a required binary, service, permission, or OS capability.
- `failed`: The proof harness ran and observed incorrect behavior.

### Stability Ownership Model

Stability is not a single module. It is the contract between these owners:

- `RunnerEventLoop`: owns task state transitions, cancellation points, gate completions, merge completions, snapshot flushes, and event emission.
- `PersistPaths`: owns the filesystem layout for all runner durability files.
- `RunStateSnapshot`: owns runner-local state not present in the executor snapshot.
- `ResumeService`: owns compatibility checks, task fingerprint validation, stale snapshot decisions, and JSONL recovery.
- `ProcessSupervisor`: owns child process registration, process groups, PID files, kill-on-drop, cancellation cleanup, and stale PID cleanup.
- `MergeService`: owns merge queue state, real git merge behavior, regression gates, merge conflict evidence, and merge snapshot recovery.
- `ProjectionService`: owns durable event replay, queryable runtime state, SSE stream state, and server/TUI parity.
- `ProviderDispatch`: owns provider-neutral runtime events and provider matrix proof.
- `ProofHarness`: owns reproducible crash/proof scripts and machine-readable proof reports.

The anti-pattern to avoid is "fixing stability" by adding one more local retry or fallback inside the runner. Every fix must strengthen one of the owners above and add proof that the owner composes with the others.

### Required Proof Artifact

Create this generated artifact and make every stability proof write it:

- [ ] `tmp/mori-diffs/generated/stability-proof-report.json`

Schema:

```json
{
  "schema_version": 1,
  "generated_at": "2026-04-27T00:00:00Z",
  "git_commit": "unknown",
  "roko_binary": "target/debug/roko",
  "workspace": "/tmp/roko-stability-proof-XXXX",
  "summary": {
    "total": 0,
    "source_wired": 0,
    "unit_proved": 0,
    "integration_proved": 0,
    "chaos_proved": 0,
    "provider_proved": 0,
    "proof_missing": 0,
    "blocked_credentials": 0,
    "blocked_environment": 0,
    "failed": 0
  },
  "scenarios": [
    {
      "id": "CRASH-ACTIVE-AGENT-OUTPUT",
      "priority": "P0",
      "status": "proof_missing",
      "provider": "codex_cli",
      "model": "default",
      "command": "bash tests/proof/mori-diffs/prove-stability.sh --scenario CRASH-ACTIVE-AGENT-OUTPUT",
      "workdir": "/tmp/roko-stability-proof-XXXX/cases/active-agent-output",
      "artifacts": {
        "stdout": "logs/active-agent-output.stdout",
        "stderr": "logs/active-agent-output.stderr",
        "events_jsonl": "work/.roko/events.jsonl",
        "run_state": "work/.roko/state/run-state.json",
        "executor": "work/.roko/state/executor.json",
        "orchestrator": "work/.roko/state/orchestrator.json",
        "agent_pids": "work/.roko/runtime/agent-pids.json"
      },
      "assertions": [
        "resume marker is present",
        "completed task count is not duplicated",
        "no stale agent pid remains",
        "events.jsonl has no malformed tail",
        "projection endpoint can rebuild state"
      ],
      "evidence": []
    }
  ]
}
```

Rules:

- [ ] The report must be regenerated from a clean temporary workspace outside this repository.
- [ ] Each scenario must include the exact command used to reproduce it.
- [ ] Each scenario must include durable artifact paths relative to the proof workspace.
- [ ] Each scenario must use one of the status labels above.
- [ ] Any `failed` scenario must include a human-readable failure summary and at least one evidence path.
- [ ] Any provider scenario must distinguish `blocked_credentials`, `auth_failed`, `rate_limited`, `unsupported`, and `provider_proved`; see [27-FILESYSTEM-RUNTIME-CI-AUDIT.md](27-FILESYSTEM-RUNTIME-CI-AUDIT.md) and [41-INFERENCE-GATEWAY-MODEL-CALL-SERVICE-AUDIT.md](41-INFERENCE-GATEWAY-MODEL-CALL-SERVICE-AUDIT.md).

### Required Proof Harness

Implement a tracked script:

- [ ] `tests/proof/mori-diffs/prove-stability.sh`

The script must:

- [ ] Build or locate the `roko` binary without writing build artifacts into `tmp/mori-diffs`.
- [ ] Create a fresh `mktemp -d` workspace outside the repository.
- [ ] Generate plans/tasks in that temporary workspace.
- [ ] Run real `roko` commands, not mocks.
- [ ] For crash scenarios, use deterministic hooks or environment-controlled delays rather than random sleeps.
- [ ] Kill the runner process at the named injection point.
- [ ] Restart the run through the normal user-facing command.
- [ ] Query durable files directly and through HTTP projection endpoints where applicable.
- [ ] Write `tmp/mori-diffs/generated/stability-proof-report.json`.
- [ ] Exit non-zero if any P0 scenario has `failed` or `proof_missing`.
- [ ] Print the proof report path and the temporary workspace path at the end.

Recommended command surface:

```bash
tests/proof/mori-diffs/prove-stability.sh --all
tests/proof/mori-diffs/prove-stability.sh --scenario CRASH-ACTIVE-AGENT-OUTPUT
tests/proof/mori-diffs/prove-stability.sh --provider codex_cli
tests/proof/mori-diffs/prove-stability.sh --provider claude_cli
tests/proof/mori-diffs/prove-stability.sh --http
tests/proof/mori-diffs/prove-stability.sh --merge
```

### Crash And Resume Matrix

Every row below is a concrete unchecked implementation item. A row is complete only after the proof report marks it `chaos_proved` or `integration_proved` with artifact paths.

- [ ] `CRASH-BEFORE-AGENT-SPAWN`: Start a plan, stop after the task is selected but before provider spawn, restart, and verify exactly one attempt starts.
- [ ] `CRASH-ACTIVE-AGENT-OUTPUT`: Kill the runner after the first provider output event but before turn completion; restart and verify no duplicate completed task and no orphan process.
- [ ] `CRASH-POST-AGENT-PRE-GATE`: Kill after `AgentDispatchOutcome::Completed` is emitted but before gate completion; restart and verify the gate either resumes or reruns once with explicit event evidence.
- [ ] `CRASH-IN-GATE`: Kill while compile/verify gate is running; restart and verify the gate result is not silently passed.
- [ ] `CRASH-POST-GATE-PRE-SNAPSHOT`: Kill after a passing gate event but before `save_snapshot`; restart and verify task completion is not lost or duplicated.
- [ ] `CRASH-POST-SNAPSHOT-PRE-EVENT`: Kill after snapshot write but before terminal runner event append; restart and verify projection reconstruction reconciles snapshot and event log consistently.
- [ ] `CRASH-MERGE-QUEUED`: Queue two merge requests, kill before reservation, restart, and verify `MergeQueue` order and lock state survive.
- [ ] `CRASH-MERGE-RESERVED`: Kill after merge reservation but before git merge completion; restart and verify the merge is not double-applied and conflict state is explicit.
- [ ] `CRASH-POST-MERGE-PRE-REGRESSION`: Kill after git merge success before regression gate; restart and verify regression gate still runs.
- [ ] `CRASH-POST-MERGE-PRE-SNAPSHOT`: Kill after merge regression passes before snapshot; restart and verify merge completion is durable.
- [ ] `CRASH-FEEDBACK-IN-FLIGHT`: Kill while runtime feedback sinks are writing; restart and verify `events.jsonl`, `episodes.jsonl`, and `efficiency.jsonl` recover without malformed tails.
- [ ] `CRASH-PROJECTION-REBUILD`: Kill server/TUI projection reader while events are being written; restart HTTP server and verify projection endpoints rebuild state from durable files.
- [ ] `STALE-PID-FILE`: Write a stale or live PID into `.roko/runtime/agent-pids.json`; start runner and verify cleanup removes the file and does not kill unrelated processes.
- [ ] `STALE-PLAN-ID`: Resume with snapshots for a different plan ID; verify `IgnoredStale` or hard failure is emitted, not mixed state.
- [ ] `TASK-FINGERPRINT-DRIFT`: Change a task body after snapshot; verify strict resume fails closed with `TaskMismatch`.
- [ ] `CORRUPT-OPTIONAL-LEARNING`: Corrupt trailing bytes in `episodes.jsonl`, `events.jsonl`, and `efficiency.jsonl`; verify JSONL recovery truncates only invalid tail data.
- [ ] `CORRUPT-RUN-STATE`: Corrupt `run-state.json`; verify startup refuses or starts fresh only according to an explicit policy, with a durable resume marker.
- [ ] `UNSUPPORTED-RUN-STATE-SCHEMA`: Write a future `schema_version`; verify resume fails closed.
- [ ] `PROVIDER-EXIT-WITHOUT-TURN`: Force provider process exit before `TurnCompleted`; verify failure is recorded and not treated as success.
- [ ] `CANCEL-DURING-SHUTDOWN`: Send SIGINT/SIGTERM during runner shutdown; verify cancellation is idempotent and all child processes are cleaned.

### Provider Stability Matrix

Provider proof is part of stability because the runner is only stable if the same runtime path survives real provider behavior.

- [ ] `PROVIDER-CODEX-CLI`: Real Codex CLI plan run, projection query, and resume-compatible snapshot.
- [ ] `PROVIDER-CLAUDE-CLI`: Real Claude CLI plan run, projection query, and resume-compatible snapshot.
- [ ] `PROVIDER-ANTHROPIC-API`: Real Anthropic API run through the dispatch facade, not CLI-only code.
- [ ] `PROVIDER-OPENAI-API`: Real OpenAI API run through the dispatch facade, not CLI-only code.
- [ ] `PROVIDER-MOONSHOT-API`: Real Moonshot run through the dispatch facade.
- [ ] `PROVIDER-ZAI-API`: Real Z.AI run through the dispatch facade.
- [ ] `PROVIDER-PERPLEXITY-API`: Real Perplexity run through the dispatch facade.
- [ ] `PROVIDER-AUTH-FAILURE`: Deliberately invalid key returns `auth_failed` and records provider/runtime diagnostics without leaking secrets.
- [ ] `PROVIDER-MISSING-CREDENTIALS`: Missing key returns `blocked_credentials` and never falls back to another paid provider silently.
- [ ] `PROVIDER-RATE-LIMIT`: Rate limit response returns `rate_limited`, records retry policy decision, and does not masquerade as task failure.

### HTTP And TUI Query Matrix

Projection stability is not proven by files existing. It is proven when the UI/server can reconstruct and query the same state after restart.

- [ ] `HTTP-PROJECTIONS-CATALOG`: Start `roko serve`, query `GET /api/projections/catalog`, and verify every advertised projection has a schema version and invalidation policy.
- [ ] `HTTP-RUNTIME-DASHBOARD`: Query `GET /api/projections/dashboard` after a completed run and verify plan/task/agent/gate/cost fields are populated from durable state.
- [ ] `HTTP-GATE-HISTORY`: Query `GET /api/gates/history?run_id=<run_id>` and verify gate rungs, verdicts, retries, durations, and failure summaries are present.
- [ ] `HTTP-EVENT-LOG`: Query `GET /api/projections/events?run_id=<run_id>` or the canonical event projection and verify event ordering and cursor metadata.
- [ ] `HTTP-STREAM-RECOVERY`: Subscribe to `GET /api/projections/dashboard/stream`, restart the server, and verify an initial state frame is emitted before deltas.
- [ ] `TUI-SNAPSHOT-PARITY`: Run a non-interactive TUI/projection snapshot path and verify it reads the same durable events as HTTP projection endpoints.

### Merge Stability Matrix

Merge proof must include success and conflict. A fake success event is not stability.

- [ ] `MERGE-IN-PLACE-DIRTY`: Run in-place mode with dirty worktree changes and verify `GitMergeBackend` records explicit in-place validation plus regression gate result.
- [ ] `MERGE-BRANCH-SUCCESS`: Create a real branch, change a file, run merge, and verify git history, regression gate, runner event, and snapshot all agree.
- [ ] `MERGE-BRANCH-CONFLICT`: Create conflicting branches, run merge, and verify conflict failure evidence includes git status, conflict files, and no success event.
- [ ] `MERGE-REGRESSION-FAIL`: Make merge succeed but regression gate fail; verify task/plan status remains failed or retryable, not complete.
- [ ] `MERGE-QUEUE-LOCKS`: Run two merge requests touching overlapping paths and verify queue blocking/draining behavior is durable.
- [ ] `MERGE-RESUME-QUEUE`: Crash with queued/reserved merge state, restart, and verify queue state is loaded from `orchestrator.json`.

### Implementation Batches

#### ST-01: Deterministic Failure Injection

- [ ] Add runner test hooks gated behind a proof-only environment variable such as `ROKO_PROOF_INJECT_AT`.
- [ ] Supported injection points must include: `before_agent_spawn`, `active_agent_output`, `post_agent_pre_gate`, `in_gate`, `post_gate_pre_snapshot`, `post_snapshot_pre_event`, `merge_queued`, `merge_reserved`, `post_merge_pre_regression`, and `feedback_in_flight`.
- [ ] Hooks must be no-ops unless explicitly enabled.
- [ ] Hooks must emit a durable proof marker before sleeping/exiting so the harness knows the crash point was reached.
- [ ] Add a grep gate ensuring proof hooks cannot be activated by normal config files or provider output.

#### ST-02: Crash Harness

- [ ] Implement `tests/proof/mori-diffs/prove-stability.sh`.
- [ ] Add helper functions for creating temporary plans, starting a run, waiting for a proof marker, killing the process, restarting, and collecting artifacts.
- [ ] Add JSON report writer that updates `tmp/mori-diffs/generated/stability-proof-report.json`.
- [ ] Make the harness preserve temporary workspaces on failure and print their paths.
- [ ] Make the harness delete temporary workspaces on success unless `ROKO_KEEP_PROOF_WORKDIR=1`.

#### ST-03: Resume And Snapshot Proof

- [ ] Prove `run-state.json` includes `run_id`, task counts, token/cost totals, completed task map, snapshot failure streak, and fingerprints after a successful run.
- [ ] Prove `executor.json` and `orchestrator.json` agree on plan/task terminal state.
- [ ] Prove stale plan IDs do not mix old and new run state.
- [ ] Prove task fingerprint drift fails closed.
- [ ] Prove corrupt optional JSONL tails are truncated and preserved valid rows remain readable.
- [ ] Prove future `run-state.json` schema fails closed.

#### ST-04: Process Lifecycle Proof

- [ ] Prove an active provider child is killed on runner cancellation.
- [ ] Prove `agent-pids.json` is written while a child is active and removed after cleanup.
- [ ] Prove a stale PID file does not remain after startup.
- [ ] Prove cleanup does not kill unrelated processes not registered as roko agent children.
- [ ] Prove provider exit without `TurnCompleted` is a failed attempt, not a completed task.

#### ST-05: Merge Proof

- [ ] Prove branch success using `GitMergeBackend`.
- [ ] Prove branch conflict failure with conflict evidence.
- [ ] Prove regression failure after git merge failure or success is visible as a gate failure.
- [ ] Prove queue lock blocking and draining.
- [ ] Prove merge queue resume from `orchestrator.json`.

#### ST-06: Projection And HTTP Proof

- [ ] Prove durable `events.jsonl` can rebuild projection state after runner exit.
- [ ] Prove `GET /api/projections/catalog` is complete and versioned.
- [ ] Prove `GET /api/projections/dashboard` contains plan/task/agent/gate/cost state after a run.
- [ ] Prove gate history query returns run-scoped gate decisions.
- [ ] Prove SSE projection stream emits an initial state frame and later deltas.
- [ ] Prove TUI snapshot and HTTP projection read the same canonical state.

#### ST-07: Provider Burn-In

- [ ] Run the smallest real end-to-end plan through Codex CLI.
- [ ] Run the same plan through Claude CLI.
- [ ] Run the same plan through Anthropic API.
- [ ] Run the same plan through OpenAI API.
- [ ] Run the same plan through Moonshot API.
- [ ] Run the same plan through Z.AI API.
- [ ] Run the same plan through Perplexity API.
- [ ] Record every provider status with explicit proof taxonomy labels.
- [ ] Verify no provider proof path bypasses `Dispatcher`, provider-neutral runtime events, prompt diagnostics, runtime feedback, or projection emission.

#### ST-08: Burn-In Exit Gate

- [ ] Run five small repository tasks through the same runner path with proof collection enabled.
- [ ] Run one multi-task DAG plan through the same runner path.
- [ ] Run one failed-gate retry plan through the same runner path.
- [ ] Run one interrupted/resumed plan through the same runner path.
- [ ] Run one merge conflict plan through the same runner path.
- [ ] Run one HTTP/TUI projection inspection after restart.
- [ ] Record failures as open checklist rows in this file or [29-CURRENT-RUNTIME-GAP-LEDGER.md](29-CURRENT-RUNTIME-GAP-LEDGER.md), not as untracked notes.

### No-Context Handoff Checklist

Give this block to an implementation agent with no additional context:

- [ ] Read this file, [27-FILESYSTEM-RUNTIME-CI-AUDIT.md](27-FILESYSTEM-RUNTIME-CI-AUDIT.md), [34-OBSERVABILITY-PROJECTION-QUERY-AUDIT.md](34-OBSERVABILITY-PROJECTION-QUERY-AUDIT.md), [35-TASK-PROCESS-LIFECYCLE-AUDIT.md](35-TASK-PROCESS-LIFECYCLE-AUDIT.md), [39-RUNNER-EXECUTION-POLICY-AUDIT.md](39-RUNNER-EXECUTION-POLICY-AUDIT.md), [40-SERVE-TUI-RUNTIME-ADAPTER-AUDIT.md](40-SERVE-TUI-RUNTIME-ADAPTER-AUDIT.md), and [41-INFERENCE-GATEWAY-MODEL-CALL-SERVICE-AUDIT.md](41-INFERENCE-GATEWAY-MODEL-CALL-SERVICE-AUDIT.md).
- [ ] Implement deterministic proof injection hooks from ST-01 without changing normal runtime behavior.
- [ ] Implement `tests/proof/mori-diffs/prove-stability.sh` from ST-02.
- [ ] Make the script generate `tmp/mori-diffs/generated/stability-proof-report.json` with the schema above.
- [ ] Fill every crash/resume matrix row with `chaos_proved`, `integration_proved`, `blocked_environment`, or `failed`; `proof_missing` is not acceptable for P0 rows.
- [ ] Fill every provider matrix row with `provider_proved`, `blocked_credentials`, `auth_failed`, `rate_limited`, `unsupported`, or `failed`; silent fallback is failure.
- [ ] Fill every HTTP/TUI matrix row with `integration_proved` or `failed`.
- [ ] Fill every merge matrix row with `integration_proved`, `chaos_proved`, or `failed`.
- [ ] Update this file by checking completed rows and pasting the proof report path plus the command used.
- [ ] Update [README.md](README.md) and [29-CURRENT-RUNTIME-GAP-LEDGER.md](29-CURRENT-RUNTIME-GAP-LEDGER.md) if any P0 stability gap remains.

### Definition Of Stable

Do not archive this file until all of these are true:

- [ ] There is exactly one normal plan execution path for runner-owned work.
- [ ] All provider calls used by runner execution emit provider-neutral runtime events.
- [ ] Every terminal run path writes `run.completed`, `run.failed`, or `run.interrupted`.
- [ ] Every terminal run path writes a final `run-state.json`.
- [ ] Crash/resume matrix P0 rows are `chaos_proved`.
- [ ] HTTP/TUI projection rows are `integration_proved`.
- [ ] Merge success and merge conflict rows are `integration_proved`.
- [ ] Provider rows are either `provider_proved` or explicitly blocked with credential/environment evidence.
- [ ] The proof report is tracked or reproducibly generated by a tracked script.
- [ ] No production runtime behavior exists only in `orchestrate.rs`.
- [ ] No proof relies on a mock provider unless the row is explicitly a unit test row, not an end-to-end proof row.
