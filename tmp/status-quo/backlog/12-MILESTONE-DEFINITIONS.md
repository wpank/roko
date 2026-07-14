# 12 — Milestone Definitions

> **Detailed milestone specifications for the roko executable backlog.**
> - Repo HEAD: `5852c93c05` on `main` -- authored 2026-07-10
> - Sources: `03-WORK-BREAKDOWN-EPICS.md`, `04-EXECUTION-READINESS.md`,
>   `05-MASTER-CHECKLIST.md`, `12-ROADMAP.md` (status-quo pack), `25-PROOF-GATES.md`,
>   `28-DEFINITION-OF-DONE.md`, `29-RISK-REGISTER.md`
> - Epic scope: E01-E48 (389 implementation tasks + 71 DOC reconciliation tasks)
> - Milestone assignment: follows the dependency DAG in `03` and the risk-ordered
>   roadmap in `12-ROADMAP.md`

---

## Overview

| Milestone | Theme | Epics | Tasks | Effort class |
|---|---|---|---|---|
| **M0** | Bootstrap -- honest self-execution becomes possible | E01, E04 subset | ~13 | Small; 1-2 focused sprints |
| **M1** | Correctness -- types, storage, gates, compose, providers, MCP converge | E02-E06, E14-E16 | ~72 | Medium; 3-4 parallel tracks |
| **M2** | Completeness -- learning, conductor, observability, surfaces, ACP, docs/ops | E07-E10, E17-E18, E33-E35, E42, E44-E45 | ~117 | Large; longest wall-clock phase |
| **Phase 1** | Kernel Upgrade -- Signal/Cell/Graph/Runtime primitives | E19-E22 | 40 | Large; deep architectural |
| **Phase 2** | Agent & Infrastructure -- cognition, memory, inference, connectivity, plugins | E23-E32 | 91 | Large; highest parallelism |
| **Phase 3** | Economy -- payments, marketplace, registries, arenas, DeFi | E36-E41 | 50 | Large; depends on chain |
| **Phase 3+** | Long Horizon -- cleanup, spec-debt, chain/ISFR, deployment, parity | E11-E13, E43, E46-E48 | ~44 | Ongoing; gated by earlier milestones |

---

## M0 -- Bootstrap

> **Theme:** Make bare `roko plan run` do real work, report honest pass/fail, and
> enforce minimum safety for unattended execution.

### Scope

| Epic | Tasks in M0 | What |
|---|---|---|
| **E01** | T01, T02, T03, T09, T10 (5) | Flip engine default, fix resume, warn on Graph stub, regression test, doc reconciliation |
| **E04** subset | T05, T06, T07 (3) | Block SecretLeak/PathEscape, safety funnel on Claude-CLI path, custody hash-chain |
| **E05** minimum | T02, T03 (2) | Stub verdicts become Skipped (not pass), skipped excluded from EMA/passed |
| Verify runner | -- (0, already confirmed working) | Per-task `[[task.verify]]` steps via `ShellGate` -- no work needed, ship regression guard |
| **Subtotal** | **~13 tasks** (10 epic + 3 from existing plans P11/P16) | |

### Entry criteria

- [ ] `rustup show` reports stable >= 1.91 (alloy deps require it)
- [ ] `cargo build --workspace` succeeds (no new compile errors introduced)
- [ ] `cargo test -p roko-cli --lib` passes (baseline sanity before touching dispatch)
- [ ] No other in-progress branches touch `main.rs:1361`, `rung_dispatch.rs:290`, or `safety/mod.rs:767` (merge conflict risk)

### Exit criteria

All of the following must be true simultaneously:

1. **Engine default is Runner v2.** `rg 'default_value.*=.*"runner-v2"' crates/roko-cli/src/main.rs` returns a match at the clap `--engine` arg.
2. **Resume routes to Runner v2.** `rg 'PlanEngine::Graph' crates/roko-cli/src/main.rs` returns 0 matches in the resume path (line ~2699).
3. **Graph path warns instead of fabricating SUCCESS.** `rg 'task-output:stub:' crates/roko-graph/src/cells/task_executor.rs` returns 0 matches, or the cell emits a warning/error instead.
4. **Real edits after a run.** After `cargo run -p roko-cli -- plan run plans/e2e-smoke`:
   - `git status --porcelain` is non-empty (files were actually changed)
   - `.roko/episodes.jsonl` grew (episodes recorded)
   - `.roko/state/state-snapshot.json` was written (snapshot persisted)
5. **Honest gate verdicts.** A task with tier >= `integrative` that has a failing verify command reports **fail or skip**, not pass. Specifically:
   - `rg 'fn stub_verdict' crates/roko-gate/src/rung_dispatch.rs` shows the return is `Verdict::skip` (or `Skipped`), not `Verdict::pass`
   - `rg 'Skipped' crates/roko-cli/src/runner/gate_dispatch.rs` confirms skipped verdicts are excluded from the `passed` count and EMA
6. **Safety post-checks block.** `rg 'Block' crates/roko-agent/src/safety/mod.rs` shows SecretLeak and PathEscape actions are `Block`, not `Warn`.
7. **Custody hash-chain is real.** `cargo test -p roko-cli -- custody` passes and includes a tamper-detection test.
8. **Regression test passes.** `cargo test -p roko-cli -- e2e_default_engine` (or equivalent) asserts the bare-default plan run produces real artifacts.

### Estimated effort

- **Task count:** ~13 (10 mechanical/focused + 3 integrative)
- **LOC delta:** ~200-400 (mostly one-liners and test additions)
- **Model cost:** ~$5-15 per task at Haiku tier; ~$50-100 total if agent-executed
- **Calendar estimate:** 1-2 focused sprints (the tasks are serial on the critical path E01-T01 -> T02 -> T09)

### Risks and mitigations

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Flipping engine default breaks existing users/scripts that expect Graph | Medium | Medium | Keep `--engine graph` selectable; warn on Graph, don't remove it |
| Safety funnel on Claude-CLI may interfere with agent tool use | Low | High | Test with a real plan before merging; keep `dangerously_skip_permissions` as escape hatch |
| Custody hash-chain changes could break existing `.roko/custody/` data | Low | Low | Migration/re-seal on first verify; old chains get a compat pass |

### Verification commands

```bash
# --- Build ---
cd /Users/will/dev/nunchi/roko/roko
cargo build -p roko-cli

# --- M0.1: Engine default is runner-v2 ---
rg 'default_value.*=.*"runner-v2"' crates/roko-cli/src/main.rs
# Expected: 1 match

# --- M0.1: Resume not hardcoded to Graph ---
rg 'PlanEngine::Graph' crates/roko-cli/src/main.rs | grep -c 'resume'
# Expected: 0

# --- M0.3: Stub verdicts are Skipped, not pass ---
rg 'fn stub_verdict' -A3 crates/roko-gate/src/rung_dispatch.rs
# Expected: returns Verdict::skip or similar, NOT Verdict::pass

# --- M0.4: Post-checks block ---
rg 'SecretLeak|PathEscape' -A2 crates/roko-agent/src/safety/mod.rs | grep -i 'block'
# Expected: matches showing Block action

# --- Smoke test (requires Claude API key) ---
cargo run -p roko-cli -- plan validate plans/e2e-smoke
cargo run -p roko-cli -- plan run plans/e2e-smoke --fresh
git status --porcelain                    # non-empty = real edits
tail -3 .roko/episodes.jsonl             # episodes recorded
ls -la .roko/state/state-snapshot.json   # snapshot exists

# --- Regression lock ---
cargo test -p roko-cli -- e2e_default_engine
cargo test -p roko-cli -- custody
```

### Behavioral tests

- Bare `roko plan run plans/<dir>` (no `--engine` flag) dispatches a real agent, runs gates, writes episodes.
- `roko plan run plans/<dir> --engine graph` emits a clear warning that Graph execution is a dry-run stub.
- `roko resume .roko/state/state-snapshot.json` resumes via Runner v2, skipping completed tasks.
- A task with `verify.command = "false"` is marked failed in the run ledger.
- A task dispatched to Claude-CLI with a forbidden tool in the deny-list has that tool refused.

### Files that should exist after M0

- `.roko/episodes.jsonl` (non-empty, grew during run)
- `.roko/state/state-snapshot.json` (valid JSON, task statuses present)
- `.roko/state/run-ledger.jsonl` (task-level pass/fail entries)

### Files that should NOT exist / should be unchanged

- No new `task-output:stub:` markers in any output log after a default run

### Dashboard metrics that should change

- `roko status` reports non-zero completed tasks after a run
- `.roko/learn/gate-thresholds.json` may not yet exist (full persistence is M1/E05-T01), but stub verdicts no longer inflate the EMA

---

## M1 -- Correctness

> **Theme:** Close the type/storage/gate/compose/provider/MCP loops so the live
> path is correct, converged, and honest end-to-end.

### Scope

| Epic | Tasks | What |
|---|---|---|
| **E02** | 12 | Storage convergence: one canonical writer per `.roko/` concern, fix empty dashboards |
| **E03** | 7 | Type consolidation: collapse 5 runtime-critical duplicate type families |
| **E04** remainder | ~16 | Security perimeter: relay auth, scope deny-by-default, ACP permission gate, rate limiting |
| **E05** full | 6 (remaining after M0) | Gate adaptivity: real rung inputs, per-rung EMA, threshold persistence, VerdictPublisher |
| **E06** | 9 | Compose/prompt unification: route Runner v2 through canonical 12-slot builder |
| **E14** | 7 | Providers & tools: retries, tool survival on non-Claude, builtin parity |
| **E15** | 6 | MCP config & passthrough: normalizer, env, tool delivery |
| **E16** | 2 | PRD self-hosting: Perplexity fix, front-half smoke test |
| **E46** | ~8 | GitHub workflow integration: issue/PR automation, CI triggers |
| **E47** | ~8 | Resource & disk management: `.roko/` growth control, GC policies |
| **E48** | ~8 | Rate limit budgeting: per-provider rate tracking, backoff, cost caps |
| **Subtotal** | **~89 tasks** | |

### Entry criteria

- [ ] **M0 exit green.** All M0 verification commands pass.
- [ ] E03 must lead Track C (it gates E02 and E10). Start E03 before E02.
- [ ] E14/E15 can start in parallel with E03 (Track B, file-disjoint).
- [ ] E04 remainder can start in parallel (Track A, touches `roko-serve` middleware/relay).
- [ ] E46/E47/E48 epic files and plan directories exist under `backlog/plans/`.

### Exit criteria

1. **Types converged.** `rg 'struct GateVerdict' crates/ --include='*.rs' | wc -l` returns exactly 1 (the canonical one). Same for `DashboardSnapshot`.
2. **Storage converged.** Gate verdicts are written to the path dashboards read (`engrams.jsonl`); `roko serve` reads `state-snapshot.json` (not `state/executor.json`). `.roko/events.jsonl` is bounded (no `feed_tick` firehose).
3. **Gate adaptivity live.** After a multi-task run:
   - `.roko/learn/gate-thresholds.json` exists and `jq '.rungs | length'` >= 1
   - `GateThresholds::save` has a non-zero call count (the file was written, not just read)
   - Advanced rungs (Symbol/GenTest/Integration) execute with real inputs when `enable_advanced_rungs = true`
4. **Compose unified.** `rg 'build_role_system_prompt' crates/roko-cli/src/dispatch/ | wc -l` >= 1 (Runner v2 routes through the canonical builder). `rg 'PromptAssembler' crates/roko-compose/src/templates/assembly.rs` returns 0 (dead surface deleted).
5. **Providers honest.** `cargo test -p roko-std -- tool_handler_parity` passes (advertised builtins == executable handlers). A non-Claude dispatch (OpenAI/Gemini) preserves file-editing tools (no PascalCase stripping).
6. **MCP tools reach agents.** A `.mcp.json` server configured in `roko.toml` appears as `mcpServers` in the Claude-CLI invocation (inspectable via `--verbose` flag or dispatch log).
7. **PRD front-half works.** `cargo run -p roko-cli -- prd idea "test" && cargo run -p roko-cli -- prd draft new "test-slug" && cargo run -p roko-cli -- prd plan test-slug` produces a parseable `tasks.toml`. `prd status` shows columns for the created PRD.
8. **Security perimeter closed.** Relay proxy returns 401 without API key. No mutating `/api/*` route falls through to `read` scope. `cargo test -p roko-serve -- auth_scope` passes.
9. **GitHub integration wired.** Issue creation and PR automation are reachable via CLI or `roko serve` routes (per E46 scope).
10. **Resource management active.** `.roko/` directory has bounded growth policies (per E47 scope).
11. **Rate limits enforced.** Per-provider rate tracking prevents 429 cascades (per E48 scope).

### Estimated effort

- **Task count:** ~89 tasks across 11 epics
- **LOC delta:** ~3,000-6,000 (mix of focused one-file changes and cross-crate integrative work)
- **Model cost:** ~$300-800 total if agent-executed (mix of Haiku/Sonnet tiers)
- **Parallelism:** 3 tracks (A: security/GitHub, B: providers/tools/MCP, C: types->storage->gates->compose) + E47/E48 as independent tracks

### Risks and mitigations

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| E03 type renames cause cascading compile failures across crates | Medium | High | Use `From` adapters first, then rename; run `cargo check --workspace` after each step |
| E02 storage migration breaks existing `.roko/` state | Medium | Medium | `LayoutVersion::V2` with idempotent migration + `roko doctor` audit |
| E06 compose unification drops prompt sections agents depend on | Low | High | Diff the old vs new prompt output before switching; keep old path behind feature flag until verified |
| E04 relay auth breaks existing unauthenticated clients | Medium | Low | Grace period with deprecation warnings; `ROKO_SKIP_AUTH=1` escape hatch for local dev |
| E46/E47/E48 epics not yet materialized as plan files | High | Medium | Author plan files as first step of M1 |

### Verification commands

```bash
# --- Types ---
rg 'struct GateVerdict' crates/ --include='*.rs' -c
# Expected: 1

rg 'struct DashboardSnapshot' crates/ --include='*.rs' -c
# Expected: 1 (canonical) + adapters

# --- Storage ---
cargo run -p roko-cli -- plan run plans/e2e-smoke --fresh
rg '"kind":"GateVerdict"' .roko/engrams.jsonl | wc -l
# Expected: > 0 (verdicts in the right log)

rg '"kind":"GateVerdict"' .roko/signals.jsonl | wc -l
# Expected: 0 (old path retired)

cat .roko/learn/gate-thresholds.json | jq '.rungs | length'
# Expected: >= 1

# --- Compose ---
rg 'build_role_system_prompt' crates/roko-cli/src/dispatch/
# Expected: >= 1 match

# --- Providers ---
cargo test -p roko-std -- tool_handler_parity
cargo test -p roko-agent -- tool_alias_normalize

# --- MCP ---
cargo test -p roko-cli -- mcp_passthrough

# --- Security ---
cargo test -p roko-serve -- auth_scope
cargo test -p roko-serve -- relay_requires_auth

# --- PRD front-half ---
cargo run -p roko-cli -- prd idea "milestone test"
cargo run -p roko-cli -- prd status | grep -q "milestone"

# --- Full workspace ---
cargo clippy --workspace --no-deps -- -D warnings
cargo test --workspace
```

### Behavioral tests

- `roko plan run` exercises the 12-slot `SystemPromptBuilder` (visible in dispatch logs or `--verbose`).
- A task routed to OpenAI-compat provider keeps `Read`, `Write`, `Edit` tools (no PascalCase stripping).
- A single HTTP 429 from a provider triggers retry with backoff (not abort).
- `roko serve` returns 401 on unauthenticated `/relay/*` requests.
- A read-scoped API key is rejected on `POST /api/jobs` (or any mutating route).
- `roko doctor` reports canonical vs legacy `.roko/` paths.
- Dashboard gate panels show verdicts after a run (not empty).

### Dashboard metrics that should change

- Gate verdict panels in TUI/serve populated (storage convergence)
- `.roko/learn/gate-thresholds.json` updates across runs (adaptivity live)
- `roko status` episode/signal counts match across all readers

---

## M2 -- Completeness

> **Theme:** Make the system observable, learnable, surface-correct, consent-gated,
> and shippable. Close all live-path loops and hygiene gaps.

### Scope

| Epic | Tasks | What |
|---|---|---|
| **E07** | 10 | Learning & knowledge: LinUCB persistence, knowledge reinforcement, HDC |
| **E08** | 7 | Conductor supervision: anomaly detection, ghost-turn abort, real `conductor_load` |
| **E09** | 9 | Observability: MetricRegistry threading, log rotation, firehose trimming |
| **E10** | 7 | Frontend/API contract: fix 4 frontend 404s, casing drift, double SSE, replay |
| **E17** | 6 | ACP completion: consent-gated, learning-informed, MCP-equipped ACP turns |
| **E18** | 13 | Docs/config/CI/ops: MSRV bump, cargo deny, Docker fix, doc rewrites, docs-lint CI |
| **E33** | 9 | Telemetry & Lens: StateHub projections, Observe protocol, c-factor computation |
| **E34** | 8 | Security IFC: taint lattice, immune system pipeline, corrigibility, sandbox |
| **E35** | 8 | Auth protocol: API key rotation, agent tokens, JWKS caching, team RBAC |
| **E42** | 8 | Config evolution: config-as-Signal, schema versioning, hot-reload |
| **E44** | 8 | Cross-cut functors: endofunctor algebra, natural transformations, VCG arbitration |
| **E45** | 10 | Orchestrator Mori parity: structured review, auto-fix, reflection loop |
| **Subtotal** | **~103 tasks** | |

### Entry criteria

- [ ] **M1 exit green.** All M1 verification commands pass.
- [ ] E03 landed (E10 needs canonical `DashboardEvent`).
- [ ] E04 full landed (E17 needs permission infrastructure).
- [ ] E07 can start immediately after M1 (needs E01 only).
- [ ] E15 landed (E17 needs MCP session threading).
- [ ] E18 doc rewrites (T10-T13) need E01 + E18's own T05-T08 fixes first.

### Exit criteria

1. **Learning durable.** LinUCB state survives restart: `cargo run -p roko-cli -- learn all` shows non-zero arm weights after a second run. Knowledge store `balance > 0` after episode reinforcement.
2. **Conductor live.** A ghost-turn loop (agent emitting identical output repeatedly) is detected and aborted before wall-clock exhaustion. `conductor_load` in dispatch routing is a real computed value, not `0.0`.
3. **Observability operational.** `.roko/metrics/prometheus.txt` contains `roko_gate_verdicts_total` after a run. `roko.log` rotates daily. `events.jsonl` is bounded.
4. **Frontend contract fixed.** The 4 known 404s resolve:
   - `/api/shared/{token}` (not `/api/share/{token}`)
   - `/ws/agents` WebSocket endpoint exists
   - `POST /api/bench/matrix` routes to MatrixRun engine
   - `GET /api/isfr/stream` SSE route exists
   - One SSE manager remains (deprecated `EventStreamProvider` removed).
5. **ACP consent-gated.** An ACP turn's `write_file`/`edit_file`/`bash` calls `request_permission` before executing. An unauthorized call is denied end-to-end. The ACP turn consults `ExperimentStore` for A/B and receives session MCP tools.
6. **CI pipeline honest.**
   - `cargo deny check` runs in CI (`.github/workflows/deny.yml` exists)
   - Release workflow gates on clippy+test before building binaries
   - `coverage.yml` does not use `--ignore-run-fail`
   - Workspace `rust-version` is 1.91+
   - Docs-lint CI job catches forbidden drift patterns
7. **Docs trustworthy.** `README.md` and `CLAUDE.md` reference `runner-v2` as default engine. No doc claims "wired" for an unwired feature. Canonical counts (35 workspace members, 37 tools, `Engram` noun) are correct.
8. **Telemetry pipeline wired.** 7 StateHub projections feed the Lens stack (E33). Observe protocol is defined.
9. **Security IFC enforced.** Taint lattice and immune system pipeline are wired (E34). Sandbox levels are defined.
10. **Auth protocol complete.** API key rotation, agent tokens, and JWKS caching are operational (E35).

### Estimated effort

- **Task count:** ~103 tasks across 12 epics
- **LOC delta:** ~8,000-15,000 (substantial cross-crate work + doc rewrites)
- **Model cost:** ~$500-1,500 total if agent-executed
- **Parallelism:** E07/E08/E09 are dep-free after M1; E10 needs E03; E17 needs E04+E07+E15; E18 partially serial

### Risks and mitigations

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| E17 ACP changes break editor integration | Medium | High | Feature-flag the consent gate; test with Claude Code integration |
| E18 doc rewrites lag behind code changes | High | Low | Docs-lint CI prevents drift; rewrites are the last E18 tasks |
| E08 conductor false positives abort healthy agent turns | Medium | Medium | Conservative thresholds + cool-off period before abort; configurable sensitivity |
| E10 frontend changes require `npm run build` before `cargo build -p roko-serve` | High | Low | Document in CLAUDE.md; add a CI step that rebuilds embedded SPA |
| E33-E35/E42/E44-E45 are deep architectural epics that may need design docs first | Medium | Medium | E33-T01 and E34-T01 are design tasks; land designs before implementation |

### Verification commands

```bash
# --- Learning ---
cargo run -p roko-cli -- learn all | grep -q 'linucb'
# Expected: non-zero arm weights

cargo run -p roko-cli -- knowledge stats | grep -q 'balance'
# Expected: balance > 0 after episode reinforcement

# --- Conductor ---
cargo test -p roko-cli -- conductor_ghost_turn
# Expected: ghost-turn detected and aborted

rg 'conductor_load' crates/roko-cli/src/runner/event_loop.rs | grep -v '0.0'
# Expected: real computation, not hardcoded

# --- Observability ---
ls .roko/metrics/prometheus.txt
rg 'roko_gate_verdicts_total' .roko/metrics/prometheus.txt
# Expected: file exists with metric

# --- Frontend ---
cargo test -p roko-serve -- route_shared_token
cargo test -p roko-serve -- ws_agents_endpoint
cargo test -p roko-serve -- bench_matrix_route
cargo test -p roko-serve -- isfr_stream_route

# --- ACP ---
cargo test -p roko-acp -- permission_gate_e2e
cargo test -p roko-acp -- experiment_consultation

# --- CI ---
test -f .github/workflows/deny.yml
rg 'cargo deny' .github/workflows/deny.yml
rg 'ignore-run-fail' .github/workflows/coverage.yml
# Expected: no matches (flag removed)

# --- Docs ---
rg 'runner-v2' README.md CLAUDE.md | head -5
# Expected: references to runner-v2 as default

# --- Full workspace ---
cargo +nightly fmt --all --check
cargo clippy --workspace --no-deps -- -D warnings
cargo test --workspace
```

### Behavioral tests

- LinUCB routing preferences persist across process restarts (warm state reused).
- Knowledge store balance increases after successful task episodes.
- A runaway agent (ghost-turn loop) is killed by conductor before consuming > 5 turns.
- `roko serve` SSE stream delivers gate verdicts in real-time to connected frontends.
- An ACP `bash` call prompts for permission; denial prevents execution.
- `cargo deny check` fails on a known-bad advisory (proving it actually runs).
- A release tag build runs clippy+test+deny before producing binaries.

### Dashboard metrics that should change

- TUI Observability tab shows live metrics
- Learning tab shows non-zero LinUCB arm weights
- Conductor load is a real value in routing decisions
- Frontend panels are populated (no more empty gate/event views)

---

## Phase 1 -- Kernel Upgrade

> **Theme:** Upgrade the core primitives (Signal, Cell, Graph, Execution Runtime)
> to the v2 specification. These are the foundation for Phase 2+ features.

### Scope

| Epic | Tasks | What |
|---|---|---|
| **E19** | 10 | Signal protocol: graduation, Pulse bridges, demurrage economics, HDC fingerprints, IFC taint, Kind registry |
| **E20** | 10 | Cell unification: Cell supertrait with 9 protocols, TypeSchema, predict-publish-correct, CellContext, CellRegistry |
| **E21** | 10 | Graph engine: typed edge validation, Hot Graphs, Workflow/Activity split, parallel waves, snapshot/resume, merge queue |
| **E22** | 10 | Execution runtime: 7 cognitive loop Cells, nested gamma/theta/delta loops, T0 short-circuit, error taxonomy, budget, replay |
| **Subtotal** | **40 tasks** | |

### Entry criteria

- [ ] **M1 exit green.** Types and storage are converged (E03/E02).
- [ ] E01 engine decision is documented (E22 depends on knowing which engine is strategic).
- [ ] E20 depends on E01 (Cell must know the execution context).
- [ ] E21 depends on E20 (Graph nodes are Cells).
- [ ] E22 depends on E20 and E21 (runtime executes Cells in Graphs).

### Exit criteria

1. **Signal protocol complete.** `rg 'trait Signal' crates/roko-core/src/ | wc -l` >= 1 (canonical trait). Graduation lifecycle (draft->confirmed->archived) is implemented. Pulse bridges connect runtime events to Signal emission. Demurrage economics decay stale signals. HDC fingerprints are computed for all new signals. IFC taint labels propagate through signal chains. Kind registry is extensible.
2. **Cell unification complete.** `rg 'trait Cell' crates/roko-core/src/ | wc -l` returns exactly 1 canonical supertrait. All 9 protocols (execute, predict, publish, correct, observe, snapshot, resume, validate, describe) are defined. `TypeSchema` validates cell I/O. `CellContext` provides execution environment. `CellRegistry` discovers and instantiates cells.
3. **Graph engine upgraded.** Typed edge validation prevents invalid cell connections. Hot Graphs support live topology changes. Workflow/Activity split separates orchestration from execution. Parallel waves execute independent subgraphs concurrently. Graph snapshots and resume work with the same semantics as Runner v2. Merge queue handles concurrent graph modifications.
4. **Execution runtime upgraded.** 7 cognitive loop Cells (sense, score, route, compose, act, verify, learn) are implemented. Nested gamma/theta/delta loops run at appropriate frequencies. T0 short-circuit skips unnecessary computation. Error taxonomy classifies failures for appropriate recovery. Budget enforcement caps resource consumption. Replay reproduces execution from recorded signals.

### Estimated effort

- **Task count:** 40 tasks (all architectural/complex tier)
- **LOC delta:** ~10,000-20,000 (deep architectural changes to core primitives)
- **Model cost:** ~$800-2,000 total if agent-executed (requires Sonnet/Opus tier for design work)
- **Parallelism:** E19 is independent; E20 gates E21 and E22; E21 and E22 can partially overlap

### Risks and mitigations

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Signal/Cell trait changes break all downstream crates | High | High | Design traits with backward-compatible defaults; use blanket impls; migrate crate-by-crate |
| Graph engine upgrade invalidates existing plan execution | Medium | High | Keep Runner v2 as fallback during transition; graph parity tests before switching |
| Execution runtime nested loops add latency | Medium | Medium | Benchmark before/after; make loop nesting configurable |
| E19 demurrage economics may conflict with existing signal persistence | Low | Medium | Demurrage runs as a background sweep, not inline with writes |

### Verification commands

```bash
# --- Signal ---
rg 'trait Signal' crates/roko-core/src/
cargo test -p roko-core -- signal_graduation
cargo test -p roko-core -- signal_demurrage
cargo test -p roko-core -- signal_ifc_taint

# --- Cell ---
rg 'trait Cell' crates/roko-core/src/
cargo test -p roko-core -- cell_supertrait
cargo test -p roko-core -- cell_type_schema

# --- Graph ---
cargo test -p roko-graph -- typed_edge_validation
cargo test -p roko-graph -- hot_graph_topology
cargo test -p roko-graph -- parallel_waves
cargo test -p roko-graph -- graph_snapshot_resume

# --- Runtime ---
cargo test -p roko-runtime -- cognitive_loop_cells
cargo test -p roko-runtime -- nested_loops
cargo test -p roko-runtime -- budget_enforcement
cargo test -p roko-runtime -- replay
```

### Behavioral tests

- A new Signal kind can be registered at runtime via the Kind registry.
- A Cell with TypeSchema rejects invalid input and reports the mismatch.
- A Graph with conditional edges takes the correct branch based on cell output.
- Two independent subgraphs in a workflow execute concurrently (timestamp proof).
- A budget-capped execution halts cleanly when the budget is exhausted.
- A replayed execution produces identical Signal output as the original.

---

## Phase 2 -- Agent & Infrastructure

> **Theme:** Build out advanced agent cognition, memory, inference, connectivity,
> plugins, and operational infrastructure on top of the Phase 1 kernel.

### Scope

| Sub-phase | Epics | Tasks | What |
|---|---|---|---|
| **Agent Cognition** | E23, E24, E25, E26 | 42 | Cognitive autonomy, advanced memory, learning loops, inference gateway |
| **Infrastructure** | E27, E28, E29, E30, E31, E32 | 49 | Feeds, groups, connectivity, extensions, triggers, tool/plugin ecosystem |
| **Subtotal** | **10 epics** | **91 tasks** | |

### Entry criteria

- [ ] **Phase 1 exit green.** Signal/Cell/Graph/Runtime primitives are stable.
- [ ] E23 depends on E19 (Signal protocol) and E20 (Cell unification).
- [ ] E24 depends on E07 (M2 learning & knowledge loops).
- [ ] E25 depends on E07 (M2 learning loops).
- [ ] E26 depends on E14 (M1 providers & tools).
- [ ] E27 depends on E19 and E20.
- [ ] E28 depends on E20.
- [ ] E29 depends on E04 (M1 security perimeter).
- [ ] E30 depends on E20.
- [ ] E31 depends on E08 (M2 conductor supervision).
- [ ] E32 depends on E14 and E15 (M1 providers/tools + MCP).

### Exit criteria

1. **Agent cognition autonomous.** Agent type-state machine (idle->planning->executing->reflecting) is operational. Behavioral phases are configurable. EFE (Expected Free Energy) routing selects actions that minimize surprise. Emergent goals arise from sustained attention patterns. Energy budget limits agent-initiated actions.
2. **Memory advanced.** Heuristics with falsifiers support hypothesis testing. Allen temporal intervals enable temporal reasoning. Resonator networks provide associative recall. Income policy governs knowledge acquisition rate. Dream triggers fire consolidation cycles. ODE tuning adjusts memory dynamics.
3. **Learning advanced.** L3 HDC defragmentation compacts the vector space. L4 c-factor governance aggregates collective intelligence. Experiment lifecycle manages A/B tests from creation to conclusion. Playbooks capture and replay successful strategies. Variance inequality bounds learning rate.
4. **Inference gateway operational.** 9-stage pipeline (loop detect, cache, prune, budget, think, converge, call, store, track) processes every LLM request. Batch API support enables cost-efficient bulk inference.
5. **Infrastructure wired.** Feed trait + registry enable data ingestion. Groups support 4 coordination modes. Relay protocol handles reconnection with backpressure. Extension system loads plugins with 22 hooks. Trigger system responds to events with debounce/filter. Tool/plugin ecosystem validates and sandboxes third-party tools.

### Estimated effort

- **Task count:** 91 tasks across 10 epics
- **LOC delta:** ~20,000-40,000 (substantial new systems + integration)
- **Model cost:** ~$2,000-5,000 total if agent-executed
- **Parallelism:** High -- Agent Cognition (E23-E26) and Infrastructure (E27-E32) are largely independent tracks once Phase 1 is done

### Risks and mitigations

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Inference gateway adds latency to every LLM call | High | Medium | Make pipeline stages optional/configurable; benchmark each stage |
| Extension system plugin loading is a security surface | Medium | High | Sandbox by default; CaMeL IFC for capability binding; audit trail |
| Groups/coordination complexity may not be needed initially | Medium | Low | Feature-gate groups; start with single-agent coordination only |
| Relay protocol backward compatibility with existing agents | Medium | Medium | Version the protocol; support v1 for a deprecation period |

### Verification commands

```bash
# --- Agent cognition ---
cargo test -p roko-agent -- type_state_machine
cargo test -p roko-agent -- behavioral_phases
cargo test -p roko-agent -- efe_routing

# --- Memory ---
cargo test -p roko-neuro -- allen_intervals
cargo test -p roko-neuro -- resonator_networks
cargo test -p roko-neuro -- dream_triggers

# --- Learning ---
cargo test -p roko-learn -- hdc_defragmentation
cargo test -p roko-learn -- cfactor_governance
cargo test -p roko-learn -- experiment_lifecycle

# --- Inference ---
cargo test -p roko-agent -- inference_pipeline
cargo test -p roko-agent -- batch_api

# --- Infrastructure ---
cargo test -p roko-core -- feed_trait
cargo test -p roko-core -- group_coordination
cargo test -p roko-runtime -- relay_protocol
cargo test -p roko-core -- extension_hooks
cargo test -p roko-runtime -- trigger_system
cargo test -p roko-core -- plugin_sandbox
```

### Behavioral tests

- An agent transitions through idle->planning->executing->reflecting states during a task.
- A dream consolidation cycle fires after sufficient episode accumulation.
- The inference gateway caches a repeated prompt and returns the cached result on the second call.
- A plugin loaded via the extension system can hook into the pre-dispatch pipeline.
- A trigger fires a task when a watched file changes.
- A group of agents coordinates via pheromone fields on a shared task.

---

## Phase 3 -- Economy

> **Theme:** Build the economic layer: payments, marketplace, registries, identity,
> arenas, and DeFi products on top of the chain infrastructure.

### Scope

| Epic | Tasks | What |
|---|---|---|
| **E36** | 8 | Payments: x402 per-request, MPP session-based, reputation pricing, settlement batching |
| **E37** | 9 | Surfaces: 5 named surfaces (Workbench, Inbox, Canvas, Minimap, Autonomy Slider), 12 object types |
| **E38** | 9 | Marketplace: agent passport, TraceRank reputation, publish/discover/fork, Package SPI, DAW composability |
| **E39** | 8 | Registries & identity: ERC-8004 transferable identity, ZK-HDC, on-chain InsightStore, gossip, job market |
| **E40** | 8 | Arenas & evals: 7-step flywheel, scoring functions, leaderboards, bounty escrow, arena-to-learning pipeline |
| **E41** | 8 | DeFi products: VCG clearing Cell, yield perpetuals, VenueAdapter, DeFiRiskEngine, affect-modulated sizing |
| **Subtotal** | **50 tasks** | |

### Entry criteria

- [ ] **Phase 2 exit green** (at minimum: E29 connectivity for relay, E23 agent autonomy for marketplace agents).
- [ ] E11 chain/ISFR (M3+ prerequisite) has landed: `architecture-core-queue` recovered, `get_logs` implemented, deploy parity reached.
- [ ] E36 depends on E11 (chain) and E29 (connectivity/relay).
- [ ] E38 depends on E36 (payments) and E39 (registries).
- [ ] E39 depends on E11 (chain).
- [ ] E40 depends on E25 (advanced learning) and E39 (registries).
- [ ] E41 depends on E11 (chain) and E39 (registries).
- [ ] Contract deployment infrastructure is operational (`Deploy.s.sol` handles all 13 contracts).

### Exit criteria

1. **Payments operational.** x402 per-request payments settle on-chain. MPP session-based payments track cumulative spend. Reputation pricing adjusts cost by agent reputation score. Settlement batching amortizes gas costs.
2. **Surfaces rendered.** All 5 named surfaces are accessible via the TUI and web frontend. 12 object types are renderable in each surface context.
3. **Marketplace live.** Agent passports are minted as ERC-8004 tokens. TraceRank reputation scores are computed and displayed. Agents can be published, discovered, and forked. Package SPI enables DAW-style composition.
4. **Registries operational.** ERC-8004 transferable identity is deployed and functional. ZK-HDC proofs verify agent capabilities without revealing internals. On-chain InsightStore persists durable knowledge claims. Gossip protocol propagates registry updates. Job market matches tasks to agents.
5. **Arenas functional.** The 7-step flywheel (submit, evaluate, rank, reward, learn, improve, resubmit) is operational. Scoring functions are pluggable. Leaderboards update in real-time. Bounty escrow holds and releases funds correctly. Arena results feed back into the learning pipeline.
6. **DeFi products deployed.** VCG clearing Cell allocates resources via auction. Yield perpetuals are deployable. VenueAdapter connects to external DeFi protocols. DeFiRiskEngine enforces position limits. Affect-modulated sizing adjusts exposure based on agent confidence.

### Estimated effort

- **Task count:** 50 tasks across 6 epics
- **LOC delta:** ~15,000-30,000 (mix of Rust + Solidity)
- **Model cost:** ~$1,500-4,000 total if agent-executed
- **Parallelism:** E36/E39 can start in parallel; E38/E40/E41 depend on E39; E37 is independent (surfaces)

### Risks and mitigations

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Smart contract bugs in payment/escrow logic | High | Critical | Foundry test suite; formal verification for critical paths; staged deployment |
| Gas costs make per-request payments impractical | Medium | High | Settlement batching; L2/rollup deployment; off-chain micropayments with on-chain settlement |
| Marketplace adoption requires critical mass of agents | High | Medium | Start with internal agents; reputation bootstrapping from historical performance |
| DeFi products expose the system to financial risk | Medium | Critical | Position limits; circuit breakers; separate risk budget from operational budget |
| ERC-8004 standard may evolve | Low | Medium | Version the identity contract; migration path for token upgrades |

### Verification commands

```bash
# --- Payments ---
cargo test -p roko-chain -- x402_payment
cargo test -p roko-chain -- mpp_session
cargo test -p roko-chain -- settlement_batch

# --- Surfaces ---
cargo test -p roko-cli -- surface_render
# TUI smoke: roko dashboard shows all 5 surfaces

# --- Marketplace ---
cargo test -p roko-chain -- agent_passport
cargo test -p roko-chain -- tracerank_reputation
cargo test -p roko-chain -- package_spi

# --- Registries ---
cargo test -p roko-chain -- erc8004_identity
cargo test -p roko-chain -- zk_hdc_proof
cargo test -p roko-chain -- job_market

# --- Arenas ---
cargo test -p roko-chain -- arena_flywheel
cargo test -p roko-chain -- bounty_escrow

# --- DeFi ---
cargo test -p roko-chain -- vcg_clearing
cargo test -p roko-chain -- defi_risk_engine

# --- Contracts (Foundry) ---
cd contracts && forge test
```

### Behavioral tests

- An agent registers on-chain, receives an ERC-8004 identity token, and appears in the marketplace.
- A task dispatched to a marketplace agent triggers x402 payment on completion.
- An arena submission is evaluated, ranked, and the winner receives the bounty from escrow.
- A VCG auction allocates compute resources to the highest-value bidders.
- DeFi risk engine prevents an agent from exceeding its position limit.

---

## Phase 3+ -- Long Horizon

> **Theme:** Cleanup, spec-debt resolution, chain/ISFR prerequisites, deployment
> portability, and orchestrator parity. These items are gated by earlier milestones
> and represent ongoing improvement rather than a single deliverable.

### Scope

| Epic | Tasks | What |
|---|---|---|
| **E11** | 5 | Chain/ISFR: recover core queue, implement `get_logs`, deploy parity |
| **E12** | 9 | Dead-code cleanup: delete ~52K-LOC legacy island (orchestrate.rs, roko-orchestrator) |
| **E13** | 3 | v2 spec-debt: trait Lens, MetricRegistry adapter, Cell/Block naming |
| **E43** | 8 | Deployment & portability: brain export/import, daemon lifecycle, secrets rotation |
| **E46** | ~8 | GitHub workflow integration (if not completed in M1) |
| **E47** | ~8 | Resource & disk management (if not completed in M1) |
| **E48** | ~8 | Rate limit budgeting (if not completed in M1) |
| **Subtotal** | **~44 tasks** (25 core + ~24 overflow from M1 if E46-E48 slip) | |

### Entry criteria

- [ ] **M2 exit green** for E12 deletions (E12-T07 requires E05+E06+E08 merged with "live value extracted" acceptance green).
- [ ] E12-T05 requires E03 (HDC de-dup onto roko-primitives).
- [ ] E12-T06 requires E01+E04 (drop roko-orchestrator after safety is ported).
- [ ] E12-T07 requires E05+E06+E08 (delete orchestrate.rs only after gate-adaptivity, compose-enrichment, and conductor value ported out).
- [ ] E13-T01 requires E09-T09 (design doc for telemetry-as-Lens).
- [ ] E11-T01 (recover core queue) is the prerequisite for all chain work and the DeFi critical path.
- [ ] E43 depends on E18 (docs/config/CI/ops hygiene).

### Exit criteria

1. **Chain prerequisites met.** `architecture-core-queue` recovered into committed `plans/` directory. `AlloyChainClient::get_logs` returns real results via `eth_getLogs`. `Deploy.s.sol` deploys all 13 contracts (ERC-8004 trio + FeeDistributor). The BLOCKED `architecture-defi-critical-path` is unblocked.
2. **Legacy island deleted.** `orchestrate.rs` (23.7K LOC) is deleted. `roko-orchestrator` crate is deleted. `legacy-orchestrate` feature is removed. `cargo build --workspace` still succeeds after deletions. `cargo test --workspace` passes.
3. **Spec-debt resolved.** `rg 'trait Lens' crates/roko-core/src/` returns >= 1 match. `MetricRegistry` feeds `StateHub` via a `CollectorLens` adapter. Cell/Block naming decision doc exists.
4. **Deployment portable.** Brain export/import (Merkle-CRDT) works across instances. Daemon lifecycle (start/stop/status/logs/install) is operational. Secrets rotation updates all consumers. Tier advisor recommends deployment sizing.
5. **E46/E47/E48 complete** (if not done in M1): GitHub integration, resource management, and rate limiting are fully operational.

### Estimated effort

- **Task count:** ~44 tasks (varies based on M1 overflow)
- **LOC delta:** -52,000 (net negative due to legacy deletion) + ~5,000-10,000 (new features)
- **Model cost:** ~$200-500 for deletions; ~$300-800 for new features
- **Parallelism:** E11 and E12 are largely independent; E13 is small; E43 is independent

### Risks and mitigations

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Deleting orchestrate.rs breaks undiscovered callers | Medium | High | `rg` sweep for all symbols before deletion; feature-gate first, delete after CI green |
| Recovering architecture-core-queue introduces stale tasks | Medium | Low | Re-validate each Q-task against current HEAD before committing |
| Brain export/import Merkle-CRDT is complex to implement correctly | High | Medium | Start with a simpler JSON export/import; upgrade to CRDT when correctness is proven |
| E12-T07 deletion has the deepest dependency chain (E05+E06+E08) | High | Low (schedule only) | Parallelize E05/E06/E08 porting; E12-T07 is the last task in the cleanup epic |

### Verification commands

```bash
# --- Chain ---
ls plans/architecture-core-queue/tasks.toml
# Expected: file exists (recovered)

cargo test -p roko-chain -- alloy_get_logs
# Expected: passes

# --- Legacy deletion ---
test ! -f crates/roko-cli/src/orchestrate.rs
# Expected: file does not exist

test ! -d crates/roko-orchestrator
# Expected: directory does not exist

rg 'legacy-orchestrate' crates/roko-cli/Cargo.toml
# Expected: 0 matches

cargo build --workspace
cargo test --workspace
# Expected: both pass after deletions

# --- Spec-debt ---
rg 'trait Lens' crates/roko-core/src/
# Expected: >= 1 match

# --- Deployment ---
cargo run -p roko-cli -- daemon status
cargo test -p roko-cli -- brain_export_import
cargo test -p roko-cli -- secrets_rotation

# --- Full workspace health ---
cargo +nightly fmt --all --check
cargo clippy --workspace --no-deps -- -D warnings
cargo test --workspace
```

### Behavioral tests

- `roko plan run plans/architecture-core-queue` executes (proves recovery).
- After deleting `orchestrate.rs`, `roko plan run` still works (Runner v2 is self-sufficient).
- `roko deploy docker` pushes an image that boots and passes `/health`.
- Brain export from instance A, import to instance B, and B can resume A's work.
- Secrets rotation updates all provider API keys without service interruption.

### Files that should NOT exist after Phase 3+

- `crates/roko-cli/src/orchestrate.rs` (deleted by E12-T07)
- `crates/roko-orchestrator/` (deleted by E12-T06)
- `crates/roko-core/src/pulse_bus.rs` (deleted by E12-T01)
- `crates/roko-core/src/state_hub.rs` (deleted by E12-T01/E03-T01)

---

## Cross-Milestone Summary

### Critical path (longest dependency chain)

```
M0: E01 (flip engine) ──> M1: E03 (types) ──> E02 (storage) ──> E05 (gates full) ──> M2: E08 (conductor) ──> Phase 3+: E12-T07 (delete orchestrate.rs)
                              ├──> E06 (compose) ──────────────────────────────────────────────────┘
                              └──> E14/E15 (providers/MCP) ──> E16 (PRD) ──> E17 (ACP, needs E04+E07+E15)
```

Depth: **~6 epic-layers** from E01 to E12-T07 (the legacy island retirement).

### Grand totals

| Metric | Value |
|---|---|
| **Total epics** | 48 (E01-E48) |
| **Total implementation tasks** | ~389 (149 status-quo + 240 v2 spec) |
| **Total DOC reconciliation tasks** | 71 |
| **Existing executable plans (P08-P34)** | 120 tasks |
| **Recovered queue (architecture-core-queue)** | 24 tasks |
| **Grand executable total** | ~604 |

### Milestone dependency diagram

```
  ┌───────────┐
  │    M0     │  Bootstrap (E01 + E04/E05 subsets)
  │  ~13 tasks│
  └─────┬─────┘
        │
  ┌─────▼─────┐
  │    M1     │  Correctness (E02-E06, E14-E16, E46-E48)
  │  ~89 tasks│
  └─────┬─────┘
        │
  ┌─────▼─────┐     ┌────────────┐
  │    M2     │ ──> │  Phase 1   │  Kernel Upgrade (E19-E22)
  │ ~103 tasks│     │  40 tasks  │
  └─────┬─────┘     └──────┬─────┘
        │                  │
        │           ┌──────▼─────┐
        │           │  Phase 2   │  Agent & Infrastructure (E23-E32)
        │           │  91 tasks  │
        │           └──────┬─────┘
        │                  │
  ┌─────▼─────┐     ┌──────▼─────┐
  │ Phase 3+  │     │  Phase 3   │  Economy (E36-E41)
  │  ~44 tasks│     │  50 tasks  │
  └───────────┘     └────────────┘
```

Note: Phase 1 can start after M1 (needs converged types); M2 and Phase 1 can overlap.
Phase 2 requires Phase 1. Phase 3 requires Phase 2 + E11 (chain). Phase 3+ items
(E11/E12/E13/E43) can proceed in parallel with Phases 1-3 where their specific
dependencies are met.

---

_Back to the backlog index: [`00-INDEX.md`](00-INDEX.md)._
