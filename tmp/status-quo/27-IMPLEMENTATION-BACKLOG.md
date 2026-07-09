# Implementation Backlog

**Date**: 2026-07-08 · HEAD `5852c93c05` on `main`. Evidence: [95-ENGINE-DRIFT.md](95-ENGINE-DRIFT.md), [92-RUNNER-V2-MODULE-FAMILY.md](92-RUNNER-V2-MODULE-FAMILY.md), [75-SECURITY-AUTH-SCOPE-MATRIX.md](75-SECURITY-AUTH-SCOPE-MATRIX.md), [60-STATE-PERSISTENCE-LEDGER.md](60-STATE-PERSISTENCE-LEDGER.md).

This backlog turns the status-quo audit into work items. It is deliberately ordered by what can make the system lie to users (or expose it) first, then by migration leverage.

## Rules For Using This Backlog

- Do P0 items before architecture cleanup.
- Every item must close with a proof gate from [25-PROOF-GATES.md](25-PROOF-GATES.md) or [64-PARITY-TEST-MATRIX.md](64-PARITY-TEST-MATRIX.md).
- Do not mark a migration done because a type or route exists. Mark it done only when the default user path exercises it.
- When an item changes docs and code, update docs in the same PR.

## P0: Execution Must Be Honest And The Perimeter Must Be Closed

| Item | Current state | Work | Proof |
|---|---|---|---|
| Default plan execution | Clap `default_value="graph"` (`main.rs:1362`) overrides enum `#[default] RunnerV2`; Graph `TaskExecutorCell` dry-runs (`dry_run:true` + live branch unimplemented → `task-output:stub:`) | Make Runner v2 the default, or make Graph refuse non-dry-run tasks, or register the built `AgentCell` in `default_registry()` | Default `roko plan run plans/smoke` produces real agent/gate events or exits unsupported |
| Resume | `roko resume` hardcodes `PlanEngine::Graph` (`main.rs:2699`); Graph warns snapshots are ignored | Route resume to Runner v2 (auto-resume) or implement Graph snapshot hydration | Resume skips completed tasks and appends events after prior snapshot |
| Help/docs | CLI examples say Graph default; docs name dead-by-default `orchestrate.rs` as the wired hub | Make help text explicit about live vs graph paths | `roko plan run --help`, README, CLAUDE, v2 orchestrator docs agree |
| Stub success | Graph and gate stubs can still look like success in user-facing summaries | Add "stub output cannot count as pass" guard to default execution and dashboards | Smoke test fails if output contains the `task-output:stub:` marker |
| Relay proxy auth | `/relay/*` + 2 WS merged outside `/api` (`routes/mod.rs:248`), fully unauthed | Nest under `/api` or wrap in `require_api_key`+`require_scope` | 401-without-key test passes for relay GET/POST/DELETE/WS |
| Scope fallback | Unlisted mutating `/api/*` falls to `read` (`middleware.rs:385`) → read key can POST run/jobs/dream/deploy | Deny-by-default or require `write`; add CI classifier test | A read-scoped key is rejected on a mutating route |
| `research search` | Perplexity batch body → HTTP 422; mock tests false-green | Fix request body (non-batch) | A live (non-mock) search returns results; mock cannot mask the regression |
| Storage divergence | Gate verdicts → `signals.jsonl` but dashboards read `engrams.jsonl`; serve reads missing `state/executor.json` (real: `state-snapshot.json`); 44 MB `events.jsonl` firehose | Converge signals↔engrams; point serve at `state-snapshot.json`; trim `feed_tick` | Dashboard panels populate; serve loads snapshot without error |

## P1: One Runtime Contract

| Item | Current state | Work | Proof |
|---|---|---|---|
| Dispatch plan types | `DispatchPlan`, `RunnerDispatchPlan`, `RoutingContext`, `GateStatus`, `CommitOutcome`, and run ledgers are spread across crates | Pick canonical crate/type layer and write compatibility adapters | No duplicate public result type is used on the live path without adapter |
| Event contract | `StateHub`, runtime `EventBus`, `PulseBus`, server event bus, SSE, WS, and JSONL logs all coexist | Declare canonical event envelope and projection boundary | Same run visible in JSONL, StateHub snapshot, SSE, WS, and TUI with same run/task IDs |
| State layout | `.roko/episodes.jsonl`, `.roko/learn/episodes.jsonl`, `.roko/memory/episodes.jsonl`; `events.jsonl`, `runtime-events.jsonl`, `state/events.json`; `engrams`/`signals` | Write migration and read-order policy in code, not just docs | Migration command reports all legacy files and creates canonical files |
| Safety hooks | Pre-checks fail closed, but **post-checks are `Warn`-only** (`safety/mod.rs:767`); advanced warrant/taint/budget/witness hooks not proven | Promote SecretLeak/PathEscape to `Block`; add integration proof across dispatcher, standard tools, ACP/tool loops, and Runner v2 | A disallowed bash/network/git op and a secret-leaking turn are denied in every agent surface |
| ACP permission gate | `request_permission` built + tested, **zero prod callers**; write/edit/bash run unconditionally (`builtin_tools.rs:291`) | Call the gate before mutating tools in both tool loops | E2E denies an unauthorized write/bash/fetch |
| Tool-alias casing | PascalCase vs snake_case (`openai_compat.rs:252,348`) strips ALL tools on non-Claude providers | Normalize alias casing | A research analyze/enhance/prd agent completes a tool call on OpenAI/Gemini/Ollama |
| Custody verify | Prints "OK" but only checks JSON-parse + monotonic timestamps (`custody.rs:206`); real hash chain is dead code | Wire the real hash-chained audit or relabel the command | `custody verify` fails on a tampered chain |
| Cold-substrate archival | Runtime-wired (`roko-serve/lib.rs:344`) but copy-not-move (`cold_substrate.rs:218`) → unbounded hourly re-append | Move (delete source) or dedup | Cold store size is bounded across repeated cycles |
| Config secret leak | `config show --effective` prints interpolated secrets (`config_cmd.rs:222`) | Redact secret-typed fields | Seeded key is redacted in output |
| Worker callback auth | Deployed worker calls back with no auth header | Add scoped token | Unauthenticated callback rejected |

## P2: Runtime Parity

| Item | Current state | Work | Proof |
|---|---|---|---|
| Runner v2 cross-cut hooks | Runner v2 is ~80% ported (daimon/dreams/learning/efficiency/neuro/gate-dispatch fire in `event_loop.rs`); holdouts are the conductor supervision loop (`conductor_load` hardcoded 0.0, `:4258`), agent-driven replan (prompt-only, no `tasks.toml` rewrite), and worktree isolation (unwired) | Port or explicitly delete the three holdouts | Failing run updates conductor, replans via tasks.toml, and isolates via worktree, without legacy orchestrate |
| Graph parity | Loader, registry, topo engine, and graph CLI exist; plan task cells are not live | Wire `TaskExecutorCell` to real dispatch, gate cells, budget, events, resume, workspace locks | `--engine graph` produces the same observable run ledger as Runner v2 for a small plan |
| Serve API | `roko-serve` exposes 272 route declarations, but some surfaces call missing aliases | Add route contract tests against demo/TUI callers | Frontend route audit returns no missing endpoints |
| Frontend data | DataHub migration is partial; some pages bypass it; old endpoints remain | Make DataHub the route manifest and remove deprecated providers | One generated endpoint manifest drives both tests and frontend client |

## P3: New Paradigm Migration

| Item | Current state | Work | Proof |
|---|---|---|---|
| Signal/Engram | **`Engram` is canonical** — there is no `struct Signal` (compat re-export only); a second dead `Engram` lives in `roko-chain`; gate verdicts still write `signals.jsonl` | Migrate docs/code to `Engram`; delete the dead chain copy; converge the append logs | One canonical noun in public docs and one canonical append log |
| Cell/Graph | v2 Cell exists in `roko-core`; graph cells use a separate NodeOutput model | Reconcile trait contracts or document the split | A real agent/compose/gate graph can be expressed without adapters leaking to users |
| Pulse/Bus | PulseBus exists, runtime/server event buses exist, StateHub exists | Layer the buses: domain pulses, runtime events, projections | One event emitted from Runner v2 can be consumed as pulse, projection, and API event |
| Chain/ISFR | Contracts, registries, watcher, relay, local jobs, and ISFR are real but not one authority | Decide authority for identity, jobs, rates, and settlement | One job lifecycle can be observed through local jobs, chain registry, relay, and serve API |

## P4: Deletion And Archive

| Item | Current state | Work | Proof |
|---|---|---|---|
| Static demos | `demo/demo-web`, `demo/demo-old`, generated dist, and resources overlap with current app | Mark supported demos and archive the rest | Only supported demo paths appear in README and CI |
| Phase 2 stubs | Dreams/Daimon/chain Phase 2 modules compile as stubs | Feature-gate, archive, or implement | Stub modules are behind explicit feature or removed from default build |
| Tmp scratch | `tmp/` contains thousands of files across authoritative, stale, and scratch sources | Apply archive rules from [17-TMP-SOURCE-RANKING.md](17-TMP-SOURCE-RANKING.md) and [63-DELETE-ARCHIVE-PLAN.md](63-DELETE-ARCHIVE-PLAN.md) | New docs reference only authoritative tmp sources |

## Near-Term Work Queue

1. Flip or guard default `roko plan run` (detects `task-output:stub:`).
2. Fix `roko resume` (route to Runner v2).
3. Authenticate the relay proxy; deny-by-default the scope fallback.
4. Fix `research search` Perplexity body; fix the non-Claude tool-alias casing bug.
5. Add a default-plan smoke test that detects synthetic Graph output.
6. Add frontend route-contract test for the 4 known 404s.
7. Wire the ACP `request_permission` gate; promote SecretLeak/PathEscape to `Block`.
8. Converge signals↔engrams; point serve at `state-snapshot.json`.
9. Fix cold-substrate copy→move; replace false-assurance custody verify.
10. Consolidate run result/status/routing types behind one module.
11. Port the 3 Runner v2 holdouts (conductor loop, tasks.toml replan, worktree) or record non-goals.
12. Generate docs drift report in CI.
