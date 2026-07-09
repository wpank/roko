# Proof Gates

These are the checks that should be used before calling a migration item done.

> Updated 2026-07-08 against HEAD 5852c93c05. Each proof gate should be a **runnable command + explicit pass criteria**, not a prose assertion. The `## Gates` section below is fully specified in that form (paired with `35-GATES-VERIFICATION.md`); other sections retain their acceptance bullets and share the command bank at the end. Run narrow (`-p <crate>`) before workspace-wide; this repo is large.

## Execution

- [ ] `roko plan run plans/` does not silently dry-run. Proof: default run emits a real agent dispatch/gate event, or exits non-zero with a message that Graph task execution is unsupported.
- [ ] `roko plan run plans/ --engine runner-v2` still executes a real task path after any default-engine changes.
- [ ] `roko resume` resumes from `.roko/state/executor.json` or equivalent and skips completed tasks.
- [ ] A default run writes expected state: run ledger, episode or event, gate result, and snapshot.

## Graph

- [ ] `TaskExecutorCell` live mode calls an injected dispatcher and produces non-synthetic output.
- [ ] Graph execution honors conditional edges with an engine-level test.
- [ ] Graph execution honors parallelism with timestamp or controlled-delay proof.
- [ ] Graph execution persists node outputs and can resume.
- [ ] Graph gate nodes or `gate-pipeline` cell fail on a known compile/test failure.

## Runner V2

- [ ] Runner v2 still passes plan validation, agent dispatch, gate dispatch, snapshot, resume, merge, and feedback smoke tests.
- [ ] Cross-plan dependencies behave consistently with Graph or are documented as unsupported.
- [ ] Worktree isolation is either restored or explicitly removed from docs.

## Gates

Each gate below pairs a **command** with an explicit **pass criterion**. Cross-ref: `35-GATES-VERIFICATION.md` drift ledger.

- [ ] **[P0] Adaptive thresholds file is written after a verdict-producing run.**
  - Command: `cargo run -p roko-cli -- plan run plans/ && cat .roko/learn/gate-thresholds.json | jq '.rungs | length'`
  - Pass: file exists and `.rungs` length ≥ 1. Currently **FAILS** — file absent despite 467 `GateVerdict` signals; save happens only at graceful teardown (orchestrate.rs:5947-5953). Fix = save incrementally after each `observe` batch.
- [ ] **[P0] roko-graph `gate-pipeline` cell runs real gates, not a passthrough stub.**
  - Command: `rg -n 'PassthroughCell|"gate-pipeline"' crates/roko-graph/src/cells/stubs.rs`
  - Pass: `gate-pipeline` no longer listed in `COGNITIVE_LOOP_STUBS`; a graph-loop test fails on a known compile/test failure (delegates to `roko_gate::GatePipeline`, which already `impl Cell`).
- [ ] **[P1] Stub/inconclusive rungs do not count as pass-rate learning.**
  - Command: `rg -n 'fn stub_verdict' crates/roko-gate/src/rung_dispatch.rs` then inspect `AdaptiveThresholds::observe` call sites.
  - Pass: stub verdicts return a neutral/inconclusive verdict (not `Verdict::pass`, rung_dispatch.rs:290-292) and are excluded from `observe`; a unit test asserts a stub does not raise the EMA.
- [ ] **[P1] `enable_advanced_rungs` actually enables rungs 3/5/6 in the pipeline.**
  - Command: set `[gates] enable_advanced_rungs = true` in `roko.toml`, run `cargo run -p roko-cli -- plan run plans/` on a Complex-tier task, inspect transcript.
  - Pass: Symbol/PropertyTest/Integration steps execute (not `skipped_count += 1`). Currently **FAILS** — both branches skip (orchestrate.rs:18259-18270). This is the real bug behind tmp-feedback/2/23 (whose `select_rung`/`task.priority` code is fabricated — ignore it).
- [ ] **[P1] GateService and canonical rungs use one rung vocabulary or separate namespaced metrics.**
  - Command: `rg -n 'fn rung_for_name' crates/roko-gate/src/gate_service.rs`
  - Pass: name→rung maps to `Rung::from_index`, OR GateService thresholds are namespaced to a distinct file. Currently diff=3/fmt=4/shell=5/judge=6 (gate_service.rs:51-59) collide with canonical Symbol/GenTest/PropTest/Integration.
- [ ] **[P1] Gate verdicts surface through StateHub/SSE/WS and match persisted files.**
  - Command: `rg -n 'engrams.jsonl|signals.jsonl|events.jsonl' crates/roko-serve/src/routes/status/gates.rs`
  - Pass: the `/gates/*` handlers read the **same** substrate that receives verdicts. Currently **MISMATCH** — handlers read `.roko/engrams.jsonl`+`events.jsonl` (routes/status/gates.rs:84-92) but 467 verdicts land in `.roko/signals.jsonl`; dashboard gate views are likely silently empty.
- [ ] **[P1] `VerdictPublisher` Pulse reentry is attached at runtime.**
  - Command: `rg -rn 'set_verdict_publisher' crates/roko-cli/src/ | grep -v 'pub fn'`
  - Pass: ≥1 real caller wires the runtime event bus; `gate.verdict.emitted` pulses observable via `roko serve` SSE. Currently 0 callers (orchestrate.rs:5353 defined, never invoked).
- [ ] **[P2] `StubJudgeGate` replaced by real `LlmJudgeGate` in the wired GateService path.**
  - Command: `rg -n 'StubJudgeGate' crates/roko-gate/src/gate_service.rs`
  - Pass: no matches; the `judge`/`llm-judge` mapping (gate_service.rs:80) uses `LlmJudgeGate`+`AgentJudgeOracle`.
- [ ] **[P2] Built-not-wired gates are wired or removed.**
  - Command: for each of DiffGate, CodeExecutionGate, BenchmarkRegressionGate, SecurityScanGate, GateGenerator, ParallelGate/VotingGate/FallbackGate, PELT, ProcessRewardModel, EvalGenerator, forensic replay — `rg -n '<Symbol>' crates/ | grep -v 'crates/roko-gate/' | grep -v test`.
  - Pass: each has ≥1 non-test caller outside roko-gate, or is removed from `lib.rs` exports.
- [ ] **[P2] SPC/Hotelling anomaly alerts drive a reaction, not just `tracing::warn`.**
  - Command: `rg -n 'observe_pipeline|Hotelling|drain.*alert' crates/roko-cli/src/orchestrate.rs`
  - Pass: an injected failure streak triggers an executor action (escalate complexity / freeze merge / PlanRevision), not only a warn at orchestrate.rs:17804-17806.

## Learning And State

- [ ] Root, learn, and memory episode readers agree on the same canonical source.
- [ ] Feedback records preserve model source, cost, latency, retries, gate result, and prompt-section data.
- [ ] Backpressure/drop behavior is observable and documented.
- [ ] Knowledge reinforcement changes retrieval/routing or is explicitly not claimed.
- [ ] State doctor lists canonical, legacy, generated, and unknown `.roko` paths.
- [ ] Config doctor reports value provenance for provider/model/env/secret-derived fields without leaking secrets.

## Server And Surfaces

- [ ] API reference can be regenerated or verified against `build_router`.
- [ ] Frontend route manifest matches server routes, or every mismatch is an intentional alias/external route.
- [ ] Auth scope tests cover representative mutating routes and fail on any new mutating `/api/*` route that falls through to `read`.
- [ ] Terminal/workspace routes require explicit write/admin treatment and have public-bind/path-prefix/secret-interpolation tests.
- [ ] SSE/WS replay cursor tests cover missed events and filter application.
- [ ] React demo DataHub path works without deprecated providers.
- [ ] ISFR and dream UI event names match backend events.
- [ ] Relay response shapes match frontend helper types and serve proxy payloads.

## ACP

- [ ] `initialize` capabilities match actual text/image/MCP/tool behavior.
- [ ] Builtin write/bash/fetch tools request permission through ACP.
- [ ] `session/new` persistence and `session/close` unknown-session behavior are tested.
- [ ] Prompt-active read loop does not drop JSON-RPC requests.

## Docs

- [ ] `README.md`, `CLAUDE.md`, docs/v2 orchestration/API/ACP pages, and this status pack agree on engine defaults.
- [ ] Any doc claiming "wired" cites a command/test/path that proves it.
- [ ] Any target-state concept is labeled target, not current state.
- [ ] Source-doc manifest covers every `docs/v1`, `docs/v2`, `docs/v2-depth`, research prompt, and v1 reference file with status and owner.
- [ ] Maintained root docs (`README.md`, `CLAUDE.md`) no longer contain unsafe `plan run`, old resume, `roko neuro`, F1-F7, 18-crate, 19-tool, or permissive-fallback claims.
- [ ] Command-example lint rejects stale examples listed in `82-COMMAND-EXAMPLE-DRIFT-LEDGER.md` outside archive/historical folders.
- [ ] Research prompts and references are fenced as strategy/provenance unless a code/proof link is present.

## Examples And Plans

- [ ] Graph examples are classified as live, topology-only, target, stale, or unsupported.
- [ ] `parallel-gates.toml`, `conditional-branch.toml`, and `task-execution.toml` are fixed or quarantined before automated proof.
- [ ] 29 executable plans / 120 ready tasks are reconciled with `.roko/GAPS.md` or a tracked issue ledger.
- [ ] PRD-derived plans use canonical `[[task]]` shape or are labeled historical.

## Data Contracts

- [ ] Schema registry lists every event/log/snapshot/API/TS contract owner, tag style, version, and compatibility policy.
- [ ] `.roko/events.jsonl` has one schema or a discriminated envelope with reader tests.
- [ ] Rust serializes representative event/DTO fixtures and frontend validates them.
- [ ] OpenAPI uses real route DTOs or generated schemas, not doc-only mirrors.
- [ ] SSE resume behavior is aligned between `Last-Event-ID` header and frontend query behavior.

## Config And Env

- [ ] Direct env-var manifest is regenerated and matches current source.
- [ ] Every `secret/auth` env var has scrub/no-front-end policy coverage.
- [ ] Every `runtime-deploy` env var has an ops-doc owner.
- [ ] Every `mcp-script` env var has a trust-boundary note.
- [ ] Test-only env vars are kept out of user-facing docs except proof-gate appendices.

## CI And Release

- [ ] `cargo deny check` is part of CI or explicitly waived.
- [ ] Frontend install/build/E2E or route-smoke runs in CI.
- [ ] Foundry contract tests run or contracts are marked non-release.
- [ ] Release workflow depends on the same proof gates as shipped artifacts.
- [ ] Docker image boots and passes `/health` and `/ready`.
- [ ] Root Docker/Railway build either provides root `roko.toml` or no longer requires it.
- [ ] Local compose boots with current `--bind`/`--port` syntax and curls `/health`, `/ready`, `/api/health`, and `/metrics`.
- [ ] Docker publish boots `roko`, `worker`, and `mirage` images and checks health before pushing.
- [ ] Fly config has one source of truth for image/build, port, and health path.
- [ ] MSRV is one value across workspace, CI, docs, and Docker.
- [ ] Orphan workflow references to missing tmp scripts are repaired or removed.

## Suggested Verification Commands

```sh
cargo metadata --no-deps --format-version 1
cargo check -q -p roko-learn -p roko-cli
cargo test -p roko-graph
cargo test -p roko-gate
rg 'default_value = "graph"|TaskExecutorCell|resume_plan' crates/roko-cli/src crates/roko-graph/src
rg '\.route\(' crates/roko-serve/src/routes crates/roko-serve/src/lib.rs
cargo deny check
```

Gate-subsystem spot-checks (fast, no full run):

```sh
# Canonical rung vocabulary (should be 7, Compile..Integration)
rg -n 'pub enum Rung|CANONICAL_ORDER|fn select_rungs' crates/roko-gate/src/rung_selector.rs
# Dead toggle: both branches must NOT both just skip
rg -n 'enable_advanced_rungs' crates/roko-cli/src/orchestrate.rs
# Threshold persistence: file should exist after a run
ls -la .roko/learn/gate-thresholds.json; grep -c GateVerdict .roko/signals.jsonl
# Dual dialect
rg -n 'fn rung_for_name' -A8 crates/roko-gate/src/gate_service.rs
# Publisher never attached
rg -rn 'set_verdict_publisher' crates/roko-cli/src | grep -v 'pub fn'
# Serve reads wrong substrate for verdicts
rg -n 'engrams.jsonl|signals.jsonl|events.jsonl' crates/roko-serve/src/routes/status/gates.rs
# Stub verdicts return pass
rg -n 'fn stub_verdict|Verdict::pass' crates/roko-gate/src/rung_dispatch.rs
```

Run expensive workspace tests only after narrowing the changed area; this repo is large and test count is high.
