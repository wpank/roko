# Roko live-run issues: 2026-07-11

Observed command:

```text
target/debug/roko plan run tmp/status-quo/backlog/plans/E01-execution-engine --approval --fresh
```

No product code was changed during this audit. Findings are split by failure mode:

1. [01-preflight-success-reenters-same-task.md](01-preflight-success-reenters-same-task.md) - successful preflight takes an invalid phase transition and executes twice.
2. [02-fresh-run-dispatches-seeded-completed-task.md](02-fresh-run-dispatches-seeded-completed-task.md) - six seeded completions do not prevent T01 from being selected.
3. [03-approval-tui-discards-run-output.md](03-approval-tui-discards-run-output.md) - approval mode selects `NoopSink`, matching blank output/diagnosis panels.
4. [04-run-ledger-duplicates-terminal-events.md](04-run-ledger-duplicates-terminal-events.md) - the ledger records duplicate successful terminal events.
5. [05-resume-cannot-reacquire-existing-worktree.md](05-resume-cannot-reacquire-existing-worktree.md) - two resume attempts fail on contradictory worktree handling.
6. [06-timeout-reports-stale-in-flight-attempt.md](06-timeout-reports-stale-in-flight-attempt.md) - timeout diagnostics retain a completed retry attempt.
7. [07-task-verification-depends-on-runner-state.md](07-task-verification-depends-on-runner-state.md) - T01 verification changes result based on worktree-local `.roko` state.
8. [08-agent-id-contract-drops-task-and-output.md](08-agent-id-contract-drops-task-and-output.md) - slash/colon identity mismatch prevents task and output association.
9. [09-routes-panel-renders-zero-data-rows.md](09-routes-panel-renders-zero-data-rows.md) - panel height arithmetic hides the only route row.
10. [10-live-token-progress-is-not-live.md](10-live-token-progress-is-not-live.md) - context stays at 0k/200k until end-of-turn usage arrives.
11. [11-active-agent-can-show-all-phases-complete.md](11-active-agent-can-show-all-phases-complete.md) - phase state is independent of active agent state.
12. [12-runner-v2-diagnosis-feed-is-unwired.md](12-runner-v2-diagnosis-feed-is-unwired.md) - diagnosis panel has no runner-v2 producer.
13. [13-plan-max-parallel-is-ignored.md](13-plan-max-parallel-is-ignored.md) - four tasks execute concurrently despite `max_parallel = 1`.
14. [14-preflight-work-is-invisible-and-counted-as-dispatch.md](14-preflight-work-is-invisible-and-counted-as-dispatch.md) - multi-minute gates look frozen and inflate dispatch time.
15. [15-dashboard-refresh-runs-synchronous-full-git-diffs.md](15-dashboard-refresh-runs-synchronous-full-git-diffs.md) - repeated Git diff subprocesses consume CPU and can stall refresh.
16. [16-no-agent-liveness-or-last-event-age.md](16-no-agent-liveness-or-last-event-age.md) - the UI cannot distinguish alive, silent, disconnected, and stalled agents.
17. [40-CRASH-TIMELINE.md](40-CRASH-TIMELINE.md) - exact terminal sequence and causal chain.
18. [41-PLAN-SCOPED-PHASE-ORPHANS-CONCURRENT-TASKS.md](41-PLAN-SCOPED-PHASE-ORPHANS-CONCURRENT-TASKS.md) - sibling completions are discarded.
19. [42-LOST-GATE-COMPLETION-LEAVES-DAG-RUNNING.md](42-LOST-GATE-COMPLETION-LEAVES-DAG-RUNNING.md) - T08 never becomes terminal.
20. [43-NO-DAG-DEADLOCK-DETECTION.md](43-NO-DAG-DEADLOCK-DETECTION.md) - unschedulable work waits for the global timeout.
21. [44-FIXED-RUN-TIMEOUT-IS-NOT-PROGRESS-AWARE.md](44-FIXED-RUN-TIMEOUT-IS-NOT-PROGRESS-AWARE.md) - useful and deadlocked time are treated alike.
22. [45-TIMEOUT-SNAPSHOT-PRECEDES-TERMINAL-STATE.md](45-TIMEOUT-SNAPSHOT-PRECEDES-TERMINAL-STATE.md) - resume state still says implementing.
23. [46-RUN-SUMMARY-CONTRADICTS-PLAN-SUMMARY.md](46-RUN-SUMMARY-CONTRADICTS-PLAN-SUMMARY.md) - global 9/5 versus plan 0/0.
24. [47-EVENT-AND-ATTEMPT-LIFECYCLES-ARE-INCOMPLETE.md](47-EVENT-AND-ATTEMPT-LIFECYCLES-ARE-INCOMPLETE.md) - starts, exits, attempts, and terminals do not balance.
25. [48-SPAWN-BUSY-LOOP-AND-LOG-STORM.md](48-SPAWN-BUSY-LOOP-AND-LOG-STORM.md) - 1,396 false spawn messages.
26. [49-SHARED-WORKTREE-DESTROYS-TASK-OWNERSHIP.md](49-SHARED-WORKTREE-DESTROYS-TASK-OWNERSHIP.md) - concurrent tasks test and commit each other's changes.
27. [50-PASSED-TASK-HAS-NO-COMMIT.md](50-PASSED-TASK-HAS-NO-COMMIT.md) - T15 passed without a durable task commit.
28. [51-GATES-FAIL-ON-UNRELATED-CONCURRENT-BREAKAGE.md](51-GATES-FAIL-ON-UNRELATED-CONCURRENT-BREAKAGE.md) - focused tasks are poisoned by shared-tree failures.
29. [52-TIMEOUT-ABSENT-FROM-RUN-LEDGER.md](52-TIMEOUT-ABSENT-FROM-RUN-LEDGER.md) - the ledger ends 22 minutes before run failure.
30. [53-STALE-LOCK-AND-DIRTY-WORKTREE-BLOCK-RECOVERY.md](53-STALE-LOCK-AND-DIRTY-WORKTREE-BLOCK-RECOVERY.md) - cleanup leaves dead PID lock and registered dirty branch.
31. [54-RETRY-CLASSIFICATION-AND-ATTEMPTS-CONTRADICT.md](54-RETRY-CLASSIFICATION-AND-ATTEMPTS-CONTRADICT.md) - permanent is retryable and attempt numbering skips/reuses.
32. [55-INVALID-MODEL-CONFIG-CONTINUES-WITH-AMBIGUOUS-FALLBACK.md](55-INVALID-MODEL-CONFIG-CONTINUES-WITH-AMBIGUOUS-FALLBACK.md) - missing and duplicate slugs do not stop dispatch.
33. [56-SNAPSHOT-BACKUP-CADENCE-LOSES-RUN-HISTORY.md](56-SNAPSHOT-BACKUP-CADENCE-LOSES-RUN-HISTORY.md) - no recoverable checkpoints during the hour.
34. [57-TUI-LOG-OMITS-RUNTIME-AND-EXIT-STATE.md](57-TUI-LOG-OMITS-RUNTIME-AND-EXIT-STATE.md) - TUI log only records startup.
35. [58-ATOMIC-WRITE-DEBRIS-IS-NOT-RECOVERED.md](58-ATOMIC-WRITE-DEBRIS-IS-NOT-RECOVERED.md) - abandoned temp and corrupted router files accumulate.
36. [59-AGENT-OUTPUT-IS-MISCLASSIFIED-AS-ERROR.md](59-AGENT-OUTPUT-IS-MISCLASSIFIED-AS-ERROR.md) - normal source lines become `agent.error` events.
37. [60-BLOCKED-PLANS-ARE-MARKED-IMPLEMENTING.md](60-BLOCKED-PLANS-ARE-MARKED-IMPLEMENTING.md) - all six plans appear dispatched before dependencies clear.
38. [61-PREFLIGHT-HAS-NO-DURABLE-HEARTBEAT.md](61-PREFLIGHT-HAS-NO-DURABLE-HEARTBEAT.md) - logs, events, snapshots, and ledger freeze during active gates.
39. [62-PLAN-STATE-START-TIMESTAMPS-ARE-ZERO.md](62-PLAN-STATE-START-TIMESTAMPS-ARE-ZERO.md) - persisted plan timing is unusable.
40. [63-SELF-HEAL-VERIFY-COMMANDS-MISUSE-CARGO-FILTERS.md](63-SELF-HEAL-VERIFY-COMMANDS-MISUSE-CARGO-FILTERS.md) - several generated task gates are guaranteed to fail.
41. [64-TASK-TIMING-AND-EXIT-RECONCILIATION-ARE-WRONG.md](64-TASK-TIMING-AND-EXIT-RECONCILIATION-ARE-WRONG.md) - terminal metrics omit agent time and disagree on exit status.
42. [65-AGENT-AND-GATE-CARGO-ENVIRONMENTS-DIVERGE.md](65-AGENT-AND-GATE-CARGO-ENVIRONMENTS-DIVERGE.md) - identical verification rebuilds under incompatible Cargo fingerprints.
43. [66-SECRET-SCRUBBER-CORRUPTS-ORDINARY-IDENTIFIERS.md](66-SECRET-SCRUBBER-CORRUPTS-ORDINARY-IDENTIFIERS.md) - `task-verify` is presented as a redacted API key.
44. [67-TASK-LOC-BUDGET-IS-NOT-ENFORCED.md](67-TASK-LOC-BUDGET-IS-NOT-ENFORCED.md) - agents exceed declared change budgets without warning or termination.

---

# Comprehensive code audit: 2026-07-11

Static analysis across all 18 crates (~728K LOC) using 28 parallel agents. ~160 issues cataloged by subsystem:

| # | File | Subsystem | Critical issues |
|---|---|---|---|
| 10 | [10-EVENT-LOOP.md](10-EVENT-LOOP.md) | Runner event loop | Blocking I/O in async, fabricated metrics, state machine errors |
| 11 | [11-AGENT-DISPATCH.md](11-AGENT-DISPATCH.md) | Agent dispatch | Hook params dropped, no 429 retry, stderr as output |
| 12 | [12-GATE-PIPELINE.md](12-GATE-PIPELINE.md) | Gate pipeline | Rungs 3-6 stub-pass, parallel gates sequential, no persistence |
| 13 | [13-STATE-PERSISTENCE.md](13-STATE-PERSISTENCE.md) | State persistence | Checkpoint never written, failed_tasks not restored |
| 14 | [14-TUI-DASHBOARD.md](14-TUI-DASHBOARD.md) | TUI dashboard | Unbounded output_lines, background panics, blocking git |
| 15 | [15-HTTP-SERVE.md](15-HTTP-SERVE.md) | HTTP control plane | Unauthenticated relay, read-scope allows mutation |
| 16 | [16-LEARNING-FEEDBACK.md](16-LEARNING-FEEDBACK.md) | Learning & feedback | LinUCB never persisted, CostsDb in-memory, firehose |
| 17 | [17-SAFETY-LAYER.md](17-SAFETY-LAYER.md) | Safety layer | Claude CLI bypasses all safety, post-checks warn-only |
| 18 | [18-GRAPH-ENGINE.md](18-GRAPH-ENGINE.md) | Graph engine | TaskExecutorCell dry-run, AgentCell unregistered, no resume |
| 19 | [19-CLI-COMMANDS.md](19-CLI-COMMANDS.md) | CLI commands | Help text wrong, --no-replan unwired, max_gate_rung=0 bug |
| 20 | [20-MERGE-QUEUE.md](20-MERGE-QUEUE.md) | Merge queue | No rollback after failed regression, crash deadlocks |
| 21 | [21-PRD-PLAN-GEN.md](21-PRD-PLAN-GEN.md) | PRD & plan gen | Perplexity 100% broken, no cycle detection in runner |
| 22 | [22-KNOWLEDGE-NEURO.md](22-KNOWLEDGE-NEURO.md) | Knowledge (neuro) | Cross-process corruption, O(n) scans, no runtime GC |
| 23 | [23-CONFIG.md](23-CONFIG.md) | Configuration | Dead fields, wallet key plaintext, cold_storage default on |
| 24 | [24-DREAMS-DAIMON.md](24-DREAMS-DAIMON.md) | Dreams & daimon | Staging path mismatch, no periodic trigger, income dead |
| 25 | [25-DUPLICATE-TYPES.md](25-DUPLICATE-TYPES.md) | Duplicate types | GateVerdict 4x, Usage 4x, AgentConfig 3x |
| 26 | [26-DEAD-CODE.md](26-DEAD-CODE.md) | Dead code | orchestrate.rs 23K, conductor 10K, feature flags no-op |
| 27 | [27-AGENT-SERVER.md](27-AGENT-SERVER.md) | Agent sidecar | Non-constant-time auth, unbounded WS, no keepalive |
| 28 | [28-PROCESS-SUPERVISION.md](28-PROCESS-SUPERVISION.md) | Process supervision | Zombies on drop, no per-task timeout, cancel depth bug |
| 29 | [29-PROMPT-COMPOSITION.md](29-PROMPT-COMPOSITION.md) | Prompt composition | Two parallel paths, token budget hardcoded 64K |
| 30 | [30-FILESYSTEM-JSONL.md](30-FILESYSTEM-JSONL.md) | Filesystem / JSONL | 43MB firehose, run-ledger write-only, episodes triplicated |
| 31 | [31-E01-RECENT-CHANGES.md](31-E01-RECENT-CHANGES.md) | E01 recent changes | Resume regression (infinite wait), split_into dead |
| 32 | [32-COLD-SUBSTRATE.md](32-COLD-SUBSTRATE.md) | Cold substrate | Copy-not-move, no dedup, unbounded re-append |
| 33 | [33-ERROR-HANDLING.md](33-ERROR-HANDLING.md) | Error handling | Reachable unreachable!(), silent channel drops |
| 34 | [34-MCP-INTEGRATION.md](34-MCP-INTEGRATION.md) | MCP integration | Config format mismatch, env vars dropped |
| 35 | [35-COST-BUDGET.md](35-COST-BUDGET.md) | Cost & budget | Budget enforcement log-only, Sonnet fallback, no attribution |
| 36 | [36-DEPENDENCIES.md](36-DEPENDENCIES.md) | Dependencies | roko-runtime→roko-gate violation, 82 duplicate versions |
| 37 | [37-STATEHUB-EVENTS.md](37-STATEHUB-EVENTS.md) | StateHub events | Broadcast overflow, TUI races runner, cost double-counted |
| 38 | [38-TEST-COVERAGE.md](38-TEST-COVERAGE.md) | Test coverage | run() untested, agent_events zero tests, E2E ignored |
| 39 | [39-DAEMON-DEPLOY.md](39-DAEMON-DEPLOY.md) | Daemon & deploy | Port mismatch, SIGTERM unhandled, docker push missing |

## Top 10 most impactful (cross-cutting)

1. **Graph engine is a dry-run stub** (18) — default `roko plan run` does nothing real
2. **Gate rungs 3-6 always pass** (12) — validation is incomplete
3. **Claude CLI bypasses safety** (17) — the primary backend has zero guardrails
4. **State checkpoint never written** (13) — crash = full restart
5. **Blocking I/O in async event loop** (10) — single slow agent stalls all
6. **LinUCB bandit never persists** (16) — model routing resets every run
7. **Unauthenticated HTTP relay** (15) — any caller can execute agents
8. **orchestrate.rs 23K LOC dead code** (26) — confusion for contributors
9. **Unbounded TUI output_lines** (14) — long runs OOM the dashboard
10. **No per-task timeout** (28) — stuck agent blocks the entire plan
