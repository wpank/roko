# Parity Test Matrix

> Re-verified 2026-07-08 against git HEAD `5852c93c05` (deeper second pass). Lists proof tests that should exist before
> the project can be considered migrated, and adds a **cross-surface false-green census** (§Parity False-Green Census).
> Core claim: the same plan/tool/gate flow must produce the same observable result across **CLI vs HTTP vs ACP**, and
> **no such cross-surface parity test currently exists** — worse, the surfaces are *known to diverge* (HTTP flattens to
> `run_once`; default CLI uses a stub Graph cell). Companion: [74](74-TEST-AND-PROOF-INVENTORY.md) for the full mock census.

> **Verified default:** `roko plan run` defaults to `PlanEngine::Graph` (`crates/roko-cli/src/main.rs:1299,2699`), and
> the Graph task cell is a stub — `TaskExecutorCell` (`crates/roko-graph/src/cells/task_executor.rs:18-93`) is
> `dry_run: true` by default (line 31-34) and its live branch is "not yet implemented" (line 80-92), emitting synthetic
> `task-output:dry-run:`/`:stub:` engrams. Real agent dispatch is only under `--engine runner-v2`
> (`crates/roko-cli/src/runner/`). The "Default plan run" and "Graph plan run" rows below currently **fail their "must
> prove" clause** (synthetic success).

## Execution

| Flow | Command/test | Must prove | Status (2026-07-08) |
|---|---|---|---|
| Default plan run | `roko plan run plans/smoke` | Performs real dispatch or fails unsupported; no synthetic success. | **FAILS** — default Graph engine returns `task-output:dry-run:` stub (`task_executor.rs:76-79`). |
| Runner v2 plan run | `roko plan run plans/smoke --engine runner-v2` | Agent dispatch, gates, episodes, events, StateHub update. | Real path; entrypoint undertested (see [10]/[74]). |
| Graph plan run | `roko plan run plans/smoke --engine graph` | Same observable artifacts as Runner v2 or explicit unsupported error. | **FAILS** — stub cell, no dispatch, no explicit unsupported error. |
| HTTP plan execute | `POST /api/plans/:id/execute` | Same DAG dispatch + records as CLI `plan run`. | **FAILS** — flattens plan to one `run_once` prompt (`crates/roko-serve/src/routes/plans.rs:206,223`); no DAG. |
| Resume | `roko resume` after interrupted run | Reads `.roko/state/executor.json`, skips completed tasks, appends new events. | No snapshot-capable e2e test. |
| Fresh run | `roko plan run --fresh` | Archives old state and does not reuse stale snapshot. | Untested. |

## Parity False-Green Census (CLI vs HTTP vs ACP)

Where the three surfaces *should* produce identical observable results but a green test either doesn't exist or only
proves wiring. A green here does **not** prove surface parity.

| # | Surface pair | Green test (if any) | Real divergence it hides | Severity |
|---|---|---|---|---|
| PFG-1 | CLI `plan run` vs HTTP `plans/:id/execute` | `execute_plan_runs_runtime_with_plan_context` (`crates/roko-serve/src/routes/plans.rs:1664`) using **mock `run_once`** (`plans.rs:1504`) | HTTP builds a single natural-language prompt (`build_plan_execution_prompt`, used at `plans.rs:206`) and calls `runtime.run_once` — a **single universal-loop turn**. CLI (runner-v2) walks the task DAG with per-task gates/episodes. Different executor, different records. Test only asserts `run_once` was called. | **P0** |
| PFG-2 | CLI default vs CLI `--engine runner-v2` | 116 `roko-graph` attrs, all green | Default Graph = stub (`task_executor.rs`); runner-v2 = real. Two "plan run" behaviors under one command; no test asserts they yield the same artifacts (or that default is explicitly unsupported). | **P0** |
| PFG-3 | ACP vs CLI/HTTP | none (`roko-acp` 128 attrs, none cross-surface) | ACP session path (`crates/roko-acp/src/session.rs`) has its own dispatch/bridge (`bridge_events.rs`); no test proves an ACP-issued plan/tool/gate call yields the same episode/event/gate records as CLI or HTTP. | **P0** |
| PFG-4 | Safety denial across surfaces | per-surface unit tests only | Bash/network/git denial is asserted in the agent loop, but not proven identical when triggered via ACP and serve-triggered runs. A surface could bypass a check with a green suite. | **P1** |
| PFG-5 | Self-host workflow (all surfaces) | `e2e_self_host.rs:16` | `#[ignore]` + `ROKO_DISPATCHER` mock (see [74] FG-4). The only end-to-end that chains PRD→plan→run→gate→persist never runs in CI and never dispatches real work. | **P0** |
| PFG-6 | Gate verdict fidelity | roko-gate green | Stub/placeholder verdicts (rungs 4-6 oracles) can register as positive — a "pass" via CLI may be a real gate while the same via a stubbed rung is synthetic. No test forbids stub verdicts counting as learning. | **P2** |

**Most dangerous parity false-green: PFG-1.** Users and the HTTP dashboard treat `POST /plans/:id/execute` as "run the
plan" — but it silently degrades to a single-shot prompt, losing the DAG, per-task gates, and per-task episodes that the
CLI produces. The only test proves the mock was called, so the divergence is invisible to CI.

## State

| Flow | Must prove | Status |
|---|---|---|
| Episode path migration | Reads old root/learn/memory paths and writes one canonical path. | Two paths still written (`.roko/episodes.jsonl` + `.roko/memory/episodes.jsonl`, cf. `e2e_self_host.rs:89-105`). |
| Event hydration | StateHub can hydrate from canonical event log. | Untested. |
| Signal migration | `signals.jsonl` legacy input migrates or is read without becoming canonical. | Untested (now `engrams.jsonl`). |
| Daimon state | One affect path read/written across CLI, serve, learning runtime. | Untested cross-surface. |
| Learning snapshot | Cascade router, provider outcomes, gate thresholds, efficiency persist under `.roko/learn`. | Partial. |

## API/Frontend

| Flow | Must prove |
|---|---|
| Route manifest | Every frontend `get/post/put/delete/EventSource/WebSocket` path exists or is marked external. |
| Share route | Share page uses `/api/shared/{token}` or server intentionally aliases `/api/share/{token}`. |
| ISFR stream | ISFR UI uses existing events route or server exposes `/api/isfr/stream`. |
| Agent WS | UI uses `/ws`, `/api/workflow/ws`, or `/relay/agents/ws`; no missing `/ws/agents`. |
| Auth matrix | Public/read/write/admin/secret routes enforce scopes. |
| SSE replay | Reconnect does not silently drop required workflow events. |
| Relay shapes | Frontend helper types match standalone relay and serve proxy response shapes. |
| Surface parity (CLI/HTTP/ACP) | Same plan run / tool call / gate verdict via `roko` CLI, `roko-serve` HTTP, and ACP produces the same observable result (same episode/event/gate records). **No cross-surface parity test exists** — and PFG-1/2/3 show they actively diverge. |

## Safety And Tools

| Flow | Must prove |
|---|---|
| Bash denial | Dangerous command denied through CLI agent loop, ACP, and serve-triggered run. |
| Network denial | Private network/blocked host denial works in all tool surfaces. |
| Git denial | Destructive git operation denied unless explicitly allowed. |
| Metrics/audit | Tool denial emits audit, trace, and metrics records. |
| MCP scripts | Script tool obeys allowlist and timeout. |

## Learning/Gates

| Flow | Must prove |
|---|---|
| Stub gate filtering | Stub/pass placeholder verdict cannot count as positive learning (see PFG-6). |
| Gate threshold persistence | Adaptive thresholds survive restart and update incrementally. |
| Model choice fidelity | Episode records preserve explicit override vs router choice vs fallback. |
| Feedback backpressure | Dropped feedback is counted and visible. |
| Dream advice | If dream routing advice is generated, it is consumed or explicitly ignored with reason. |

## CI Command Matrix

| Tier | Commands | Gates merge? |
|---|---|---|
| Fast | `cargo check --workspace`, `cargo test -p roko-core -p roko-runtime -p roko-cli --lib` | — |
| Runtime smoke | `cargo test -p roko-cli --test smoke --test e2e_self_host --test resume_cycle_e2e` | **No** — `e2e_self_host` is `#[ignore]`. |
| Server/API | `cargo test -p roko-serve --test api_integration --test job_lifecycle` | Partial (mock runtime). |
| Agent/safety | `cargo test -p roko-agent --test safety_integration --test contracts` | Partial. |
| Learning | `cargo test -p roko-learn` | Yes (deterministic). |
| Frontend route contract | Generate route manifest, then run a script over `demo/demo-app/src` callers | **No CI gate.** |
| Full CI (`ci.yml`) | `cargo clippy --workspace --no-deps -- -D warnings`, `cargo test --workspace`, `cargo fmt --all --check`, `layer-check` | **Yes** — but satisfied by mock false-greens ([74] §census). |
| MSRV (`msrv.yml`) | `cargo check --workspace` @ 1.91 | Yes (wrong pin vs `Cargo.toml` 1.85). |
| Dependency policy | `cargo deny check` | Not wired. |
| Contracts | `(cd contracts && forge test)` | **No CI gate.** |
| Frontend | `(cd demo/demo-app && npm ci && npm run build && npm run e2e)` | **No CI gate.** |
| Release smoke | Deterministic provider e2e, Docker boot, `/health`, `/ready` | **No** — `release.yml` runs no tests at all. |
| Feature matrix | Serve `hdc/otlp`, index `sqlite/rkyv`, Rust lang `tree-sitter`, Mirage feature sets, CLI `legacy-orchestrate` | Not covered by plain workspace tests. |

## Missing Tests To Add First (ranked)

- [ ] **PFG-1:** CLI `plan run` vs HTTP `plans/:id/execute` produce identical episode/event/gate records (not just "run_once called").
- [ ] **PFG-2 / Default plan path cannot stub-succeed** — fail on `task-output:dry-run:`/`:stub:`.
- [ ] **PFG-3:** ACP-issued plan/tool/gate call matches CLI/HTTP records.
- [ ] **PFG-5:** un-ignore self-host e2e under a deterministic provider fixture, wired into CI.
- [ ] `roko resume` uses a snapshot-capable engine (skips completed tasks).
- [ ] Graph/Runner observable artifact parity.
- [ ] Frontend route manifest parity.
- [ ] `.roko` state migration dry-run (dual episode paths → one canonical).
- [ ] Config env documentation parity.
- [ ] MSRV consistency check (`Cargo.toml` == `msrv.yml`).
- [ ] Coverage workflow fails when tests fail (drop `--ignore-run-fail`).
- [ ] Stub gate verdicts cannot count as positive learning.

## Roadmap

1. **Now:** PFG-1 + PFG-2 — turn the two headline "wired" claims (HTTP plan execute, default plan run) into real parity/dispatch proofs.
2. **Next:** PFG-3 ACP parity harness; PFG-5 CI-wired deterministic self-host e2e.
3. **Then:** state-migration + resume snapshot tests; frontend route manifest.
4. **Later:** contract/frontend CI tiers; feature-matrix coverage.
