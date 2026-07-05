# Testing Status

> Status-quo audit · re-verified 2026-07-08 against git HEAD `5852c93c05` on `main` · supersedes earlier drafts.

> Reconciled counts (2026-07-08): **9,968** `#[test]`/`#[tokio::test]` attributes across `crates`+`apps`+`tests`
> (8,285 `#[test]` + 1,777 `#[tokio::test]`); **9,521** in `crates/` alone. This is an *attribute* census, not a
> passing-test count. Counts are stable since the last pass (±0.4%): the "~200 tests"/TUI-parity commits already
> landed. Use `16-CODEBASE-INVENTORY.md` for the all-workspace figure.

> **P0 semantic gap — read this before trusting any green run.** The *default* `roko plan run` uses `PlanEngine::Graph`
> (`crates/roko-cli/src/main.rs:1299,2699`), whose task cell is a stub: `crates/roko-graph/src/cells/task_executor.rs`
> `TaskExecutorCell::default()` is `dry_run: true` and even its live branch is "not yet implemented" — it emits a
> synthetic `task-output:dry-run:{task}` / `:stub:{task}` engram and **never dispatches an agent**. Real dispatch is
> only behind `--engine runner-v2` (`crates/roko-cli/src/runner/`). Therefore **mocked-dispatch tests and default
> plan-run tests can pass while the shipped default path is inert** (the wave-2 `/search` precedent: tests green,
> endpoint 100% broken). Treat any "self-hosts end-to-end" claim as unproven by CI.

Method: static census via `rg '#\[(tokio::)?test\]'` per crate (attribute occurrences, ±2% — a handful sit in macros/strings), `tests/` dir enumeration, and file reads of `.github/workflows/*`, `Cargo.toml`, `roko-graph/.../task_executor.rs`, and the three design docs. `cargo test` was **not executed** for this audit.

---

## Summary

- **9,615** `#[test]`/`#[tokio::test]` attributes across the 31 `crates/*` crates (653,226 src LOC ⇒ ~14.7 tests/KLOC workspace-wide). Prior draft said 7,625; the v1 design doc (2026-04-17) counted 3,761 — the "~200 tests" commits and TUI-parity batches roughly **2.5×'d** the suite since April.
- Plus: workspace-level `tests/` crate (member `"tests"` in `/Users/will/dev/nunchi/roko/roko/Cargo.toml:27`) with 20 tests (`end_to_end.rs` 5, `tool_equivalence.rs` 8, `tool_replay.rs` 6, `integration_test.rs` 1) and `apps/` (mirage-rs alone has 145+; apps are outside this census but inside `cargo test --workspace`).
- **18 of 31 crates have `tests/` integration dirs**; 13 have none (incl. roko-fs, roko-dreams, roko-neuro, roko-index, all roko-mcp-*, all roko-lang-*, roko-demo).
- Property tests **now exist** (roko-core, roko-conductor, roko-primitives, roko-std — 61 `proptest!`/`prop_assert` sites); the design doc's "declared, unused" claim is stale. Categories B (stateful) and C (metamorphic) remain unimplemented.
- Golden/snapshot/regression suites exist: `roko-std/tests/golden_tools.rs`, `roko-compose/tests/system_prompt_snapshot.rs` (insta 1.43), `roko-gate/tests/gate_truth.rs`, `roko-daimon/tests/pe09_regression.rs`.
- **11 `#[ignore]` attributes** (all with reason strings — good hygiene), but the flagship self-hosting e2e (`crates/roko-cli/tests/e2e_self_host.rs:15`) is one of them, and the `.roko/plans/ignored-tests.md` ledger that `roko-compose/src/templates/common.rs:231` tells agents to maintain **does not exist**.
- Biggest design-vs-reality gaps: no eval harness, no red-team/adversarial suite, no capability-preservation baselines, no mutation testing, no fuzzing, no observability-contract tests, no benchmark regression gating, release workflow ships binaries **without running tests**.

## Per-crate test census

Counts = `#[test]` + `#[tokio::test]` attribute occurrences anywhere in the crate (src + tests/). LOC = `src/` only. Verified 2026-07-07.

| Crate | #[test] count | Integration tests (`tests/` files) | src LOC | t/KLOC | Notes |
|---|---|---|---|---|---|
| roko-agent | 1,725 | 32 | 80,029 | 21.6 | Best covered: per-provider parity/conformance suites (codex, cursor, kimi, glm, ollama, openai, gemini, perplexity, openrouter), `safety_integration.rs`, `subprocess_safety_parity.rs`, `contracts.rs`+`contract_tests.rs`, wiremock `mock_provider.rs`, 8 `#[ignore]`s (flaky-parallelism + env-dependent) |
| roko-cli | 1,649 | 27 | 179,348 | 9.2 | Big e2e set (`e2e.rs`, `resume_cycle_e2e.rs`, `prd_pipeline*.rs`, `phase0_wiring.rs`, `phase1_protocols.rs`, `tui_tabs.rs`…) but `orchestrate.rs` = 23,676 LOC with only **85** inline tests; `e2e_self_host.rs` is `#[ignore]`d |
| roko-core | 1,174 | 5 | 51,823 | 22.7 | Kernel well covered; `tests/property_tests.rs` (Category A), `phase1_integration.rs`, `config_loader_integration.rs` |
| roko-learn | 918 | 5 | 58,577 | 15.7 | `learning_loop.rs`, `cascade_router_integration.rs`, `model_router_integration.rs` — design doc's "Episode→Skill not tested" is stale |
| roko-gate | 550 | 5 | 21,494 | 25.6 | `gate_truth.rs` (invariant regression), `compile_real_project.rs`, `rungs.rs`, `adaptive_threshold.rs`; no stateful proptest for ratchet/thresholds |
| roko-orchestrator | 490 | 1 | 20,777 | 23.6 | Only `lifecycle.rs` integration; DAG/merge-queue tests are inline |
| roko-serve | 467 | 8 | 61,372 | 7.6 | **288 `.route(` registrations** vs 8 integration files (jobs, PRD publish, lifecycle, `security_bind.rs`, `sanitize_xml.rs`); most routes + SSE/WS untested |
| roko-compose | 430 | 2 | 26,480 | 16.2 | insta snapshot + `cache_stability.rs`; only crate using insta |
| roko-chain | 342 | 1 | 23,355 | 14.6 | `alloy_live.rs` (live-skip); phase-2 crate, decently covered |
| roko-conductor | 300 | 1 | 10,101 | 29.7 | `tests/property_tests.rs`; dense inline coverage |
| roko-runtime | 233 | 1 | 18,751 | 12.4 | `process_supervisor.rs` integration |
| roko-std | 224 | 4 | 6,853 | 32.7 | Densest: `golden_tools.rs`, `universal_loop.rs`, `property_tests.rs`, `builtin_handlers.rs`; MockToolDispatcher lives here |
| roko-neuro | 166 | 0 | 16,553 | 10.0 | Knowledge durability store, **no tests/ dir** |
| roko-primitives | 133 | 1 | 4,682 | 28.4 | `property_tests.rs` (HDC); one of 2 crates with `benches/` |
| roko-fs | 130 | 0 | 5,518 | 23.6 | Inline only; no GC-cycle integration test |
| roko-acp | 128 | 3 | 15,851 | 8.1 | `protocol_conformance.rs`, `telemetry_integration.rs`; low density for a protocol crate |
| roko-graph | 116 | 2 | 4,447 | 26.1 | Density misleads: `engine.rs` **4** tests, `cell.rs` **0**, `error.rs` **0**; only `fanout_condition.rs` + `plan_conversion.rs` |
| roko-dreams | 92 | 0 | 13,741 | 6.7 | No tests/ dir; consolidation cycle untested end-to-end |
| roko-daimon | 89 | 1 | 7,332 | 12.1 | `pe09_regression.rs` |
| roko-index | 60 | 0 | 4,575 | 13.1 | Inline only |
| roko-lang-rust | 46 | 0 | 1,390 | 33.1 | Small, fine |
| roko-lang-typescript | 33 | 0 | 938 | 35.2 | Small, fine |
| roko-agent-server | 25 | 1 | 3,783 | 6.6 | Up from 9 in prior draft, but "T19 integration tests" = one file (`relay_registration.rs`); `/message` real dispatch + `/stream` WS untested |
| roko-lang-go | 25 | 0 | 673 | 37.1 | Small, fine |
| roko-plugin | 22 | 1 | 1,663 | 13.2 | 1 `#[ignore]` (flaky file watcher) |
| roko-mcp-github | 18 | 0 | 3,195 | 5.6 | Minimal |
| roko-mcp-code | 13 | 0 | 1,935 | 6.7 | Ships in release.yml yet ~untested |
| roko-mcp-scripts | 7 | 0 | 765 | 9.2 | Minimal |
| roko-demo | 6 | 0 | 5,860 | 1.0 | Lowest density in workspace |
| roko-mcp-slack | 2 | 0 | 1,114 | 1.8 | Effectively untested |
| roko-mcp-stdio | 2 | 0 | 251 | 8.0 | Effectively untested |
| **Total** | **9,615** | **~104 files / 18 crates** | **653,226** | **14.7** | + workspace `tests/` (20) + `apps/` (145+) |

Doctests: 310 `///`/`//!` code fences across `crates/*/src` (compiled-vs-`ignore` split not audited). `cargo test --workspace` runs them implicitly.

## CI reality (`.github/workflows/` vs CLAUDE.md pre-commit mandate)

7 workflows exist (prior draft's "CI checks: 3" and design doc's "no .github/workflows visible" are both stale):

| Workflow | Triggers | What it actually runs | Gate? |
|---|---|---|---|
| `ci.yml` | push main, PR | job **test**: `cargo clippy --workspace --no-deps -- -D warnings` + `cargo test --workspace` (stable, `RUSTFLAGS=-D warnings`, 60 min); job **fmt**: `cargo fmt --all --check` on **nightly**; job **layer-check**: `cargo run -p roko-cli -- layer-check` | Yes |
| `coverage.yml` | push main, PR | `cargo llvm-cov` HTML + JSON summary, uploads artifacts; `--ignore-run-fail` means **failing tests don't fail this job**; no threshold | No (informational) |
| `msrv.yml` | push main, PR | `cargo check --workspace` on pinned 1.91 | Yes (check only, no tests) — **but `Cargo.toml:93` declares `rust-version = "1.85"`, so the pin lies about matching the workspace** |
| `tui-parity-dry-run.yml` | PR touching `tmp/tui-parity/**`, `tmp/ux-followup-runner/**` | dry-runs two runner shell scripts | Narrow |
| `release.yml` | tag `v*` | 4-target matrix **build only** (`roko-cli`, `roko-mcp-code`) → GitHub Release. **No tests before release** | No |
| `deploy-fly.yml`, `docker-publish.yml` | deploy paths | deployment, not testing | No |

- CLAUDE.md mandate (`cargo +nightly fmt --all`, `clippy -D warnings`, `cargo test --workspace`) maps 1:1 onto ci.yml's fmt+test jobs — **mandate and CI agree**, incl. the nightly-fmt quirk. CI additionally runs layer-check, MSRV, and coverage beyond the mandate.
- Absent vs the designed CI (`docs/v2-depth/21-roadmap/09-test-strategy-and-verification.md` §CI/CD): dedicated proptest stage (1000 cases), binary-size check, nightly benchmarks, eval subset, weekly red-team tier, benchmark regression detection (±5%). None exist.
- `tests/security-smoke.sh` and `tests/endpoint_smoke.py` exist but are wired to no workflow.

## Test strategy vs design docs (what's missing)

Designs: `docs/v1/00-architecture/32-comprehensive-test-strategy.md` (1,574 lines), `docs/v2-depth/00-index/test-strategy-for-self-improving-systems.md` (5 layers), `docs/v2-depth/21-roadmap/09-test-strategy-and-verification.md` (test pyramid as tier costs).

| Strategy element | Designed | Reality 2026-07-07 |
|---|---|---|
| Unit tests (Layer 1) | ~5,000 target | **Exceeded**: 9,615 attrs; 801 of 1,091 src files carry `#[cfg(test)]` |
| Integration `tests/` dirs (Layer 3) | matrix in v1 §4.1 | 18/31 crates; several matrix rows since covered (learning loop, safety→tool dispatch, resume cycle via `roko-cli/tests/resume_cycle_e2e.rs`) — the doc's status column is stale |
| Workspace cross-crate tests | 19 → 65 target | Still **20** (`tests/tests/`); no `signal_lifecycle`, `gate_pipeline_e2e`, `telemetry_contract` files |
| Property tests Cat A (Layer 2) | ~35 tests | **Partially done**: 4 crates, 61 sites (`crates/{roko-core,roko-conductor,roko-primitives,roko-std}/tests/property_tests.rs`) |
| Property tests Cat B/C (stateful, metamorphic) | ratchet/threshold/bandit machines | **Missing**; no `proptest-state-machine` dep |
| Golden/snapshot tests | behavioral snapshots (v1 §7.5) | **Partial**: insta only in roko-compose; roko-std `golden_tools.rs` |
| Invariant regression | "gate never passes compile error" | **Partial**: `roko-gate/tests/gate_truth.rs`, `roko-daimon/tests/pe09_regression.rs` |
| Mock dispatcher usage | MockAgent/MockToolDispatcher/wiremock | **Wired**: 117 uses across 22 files (`roko-agent/src/mock.rs`, `roko-std/src/tool/mock_dispatcher.rs`, wiremock in roko-agent only) |
| Eval harness (T2, LLM-as-judge) | `crates/roko-cli/src/eval.rs` + rubrics | **Missing** (file doesn't exist); only `roko-gate/src/llm_judge_gate.rs` as a gate, no eval set/rubric/baseline |
| Red-team / adversarial (Delta) | prompt-injection, gate-bypass, poisoning suites (v1 §6) | **Missing entirely** |
| Capability-preservation / self-improvement regression | `.roko/baselines/capabilities.json` + ratcheted eval | **Missing** (no `.roko/baselines/`) |
| Benchmarks | criterion + iai-callgrind, 5 hot paths, CI tiers | **Partial**: `benches/` in roko-core + roko-primitives (criterion 0.5); no iai-callgrind, no CI job, no baselines |
| Mutation testing / fuzzing (v1 §9–10) | cargo-mutants, cargo-fuzz | **Missing entirely** |
| Observability contract tests (v1 §4.4, Layer 5) | log/metric/trace/pulse/replay schema asserts | **Mostly missing**; only `roko-acp/tests/telemetry_integration.rs` touches this |
| Ignored-test ledger | `.roko/plans/ignored-tests.md` (prompt template mandates it) | **Missing** — template at `roko-compose/src/templates/common.rs:231` points at a nonexistent file |

## Critical coverage gaps (ranked)

1. **`crates/roko-cli/src/orchestrate.rs`** — 23,676 LOC (the entire runtime loop: dispatch enrichment, gate-failure replan, CFactor, daimon modulation, rung oracles) with 85 inline tests ≈ 3.6/KLOC, and the one true e2e (`crates/roko-cli/tests/e2e_self_host.rs`) is `#[ignore]`d pending a `ROKO_DISPATCHER` fixture. The self-hosting claim rests on a path CI never exercises end-to-end.
2. **roko-serve routes** — 288 `.route(` registrations (CLAUDE.md still says ~85) vs 8 integration files; 7.6 t/KLOC on 61K LOC. No per-route smoke sweep, no SSE/WebSocket contract tests; `tests/security-smoke.sh` unwired.
3. **roko-agent-server** — real LLM dispatch + WS streaming sidecar at 25 tests / 1 integration file. "T19 integration tests" (CLAUDE.md) overstates: `relay_registration.rs` only; `/message`, `/stream`, `/predictions`, `/research` uncovered.
4. **roko-graph engine** — the DAG execution engine that plan-runs stand on: `engine.rs` 4 tests, `cell.rs` 0, `error.rs` 0; integration only covers fanout conditions + plan conversion. Budget/hot-reload paths thin.
5. **Safety enforcement adversarial coverage** — happy-path tests exist (`roko-agent/tests/{safety_integration,subprocess_safety_parity,contracts,contract_tests}.rs`) and bundled/restricted fallback is now fail-closed, but the role-auth matrix and operator carve-outs need adversarial tests; none of v1 §6.2's prompt-injection / gate-bypass / threshold-manipulation suites exist.

Honorable mentions: roko-dreams (6.7/KLOC, no tests/), roko-neuro (10/KLOC, no tests/ — knowledge durability), roko-mcp-slack/-stdio (2 tests each; roko-mcp-code ships in releases at 13 tests), roko-acp (8.1/KLOC protocol crate).

### Convention drift

- Inline `#[cfg(test)]` mods dominate (801/1,091 src files); `tests/` dirs are a secondary convention in 18 crates — both are fine, but 13 crates (incl. 16K-LOC roko-neuro, 13K-LOC roko-dreams) have **neither integration dir nor e2e**.
- Duplicate-name drift inside `tests/`: `roko-agent/tests/contracts.rs` vs `contract_tests.rs`; `roko-cli/tests/plan_validate.rs` vs `plan_validation.rs`.
- Property tests standardize on `tests/property_tests.rs` (4 crates) — not the paths the roadmap doc prescribed; fine, but future ones should follow the de-facto convention.
- `#[ignore]` reasons are consistently written (11/11) but never ledgered.

## Checklist

- [ ] **[P0]** **Default-plan-run stub-detection test**: assert `roko plan run` on a real plan produces real agent output and **fails on `task-output:dry-run:`/`:stub:` markers** — the single highest-leverage missing gate (see P0 banner + `crates/roko-graph/src/cells/task_executor.rs`) — verify: `cargo test -p roko-cli default_plan_run_not_stub`
- [ ] **[P0]** Un-ignore the self-hosting e2e: build the `ROKO_DISPATCHER` mock fixture `crates/roko-cli/tests/e2e_self_host.rs:15` needs — verify: `cargo test -p roko-cli --test e2e_self_host -- --ignored`
- [ ] **[P0]** Reconcile MSRV: `Cargo.toml:93` says `1.85`, `msrv.yml` pins `1.91`, CLAUDE.md says update — verify: `rg 'rust-version|toolchain' Cargo.toml .github/workflows/msrv.yml`
- [ ] **[P0]** Run `cargo test --workspace` (or at least `-p roko-cli -p roko-serve`) in `release.yml` before building binaries — verify: `grep 'cargo test' .github/workflows/release.yml`
- [ ] **[P1]** Route smoke sweep for roko-serve (assert 2xx/4xx per registered route, incl. SSE/WS handshake) — verify: `cargo test -p roko-serve route_smoke`
- [ ] **[P1]** roko-agent-server `/message` + `/stream` integration tests against MockAgent — verify: `cargo test -p roko-agent-server`
- [ ] **[P1]** roko-graph engine unit tests (cell exec, error propagation, budget exhaustion, cycle detection) — verify: `cargo test -p roko-graph engine`
- [ ] **[P1]** Category B stateful proptests for GateRatchet/AdaptiveThresholds + learn bandits (add `proptest-state-machine`) — verify: `cargo test -p roko-gate -p roko-learn property`
- [ ] **[P1]** Wire `tests/security-smoke.sh` + `tests/endpoint_smoke.py` into a workflow — verify: `grep -r smoke .github/workflows/`
- [ ] **[P2]** Coverage gate: drop `--ignore-run-fail` or add a minimum-percent check to `coverage.yml` — verify: PR with failing test turns coverage job red
- [ ] **[P2]** Eval harness + `.roko/baselines/capabilities.json` capability ratchet (design v1 §7) — verify: `ls .roko/baselines/capabilities.json`
- [ ] **[P2]** Adversarial suite: prompt-injection via tool output, contract permissive-fallback abuse, threshold manipulation (v1 §6.2) — verify: `cargo test -p roko-agent adversarial`
- [ ] **[P2]** Create `.roko/plans/ignored-tests.md` ledger and backfill the 11 current `#[ignore]`s — verify: `ls .roko/plans/ignored-tests.md`
- [ ] **[P3]** Tests for roko-neuro/roko-dreams persistence + roko-mcp-slack/stdio, or mark those crates experimental in CLAUDE.md — verify: `cargo test -p roko-neuro -p roko-dreams`
- [ ] **[P3]** cargo-mutants pilot on roko-gate + cargo-fuzz targets for JSONL/TOML parsers (v1 §9–10) — verify: `ls fuzz/ mutants.out/`
- [ ] **[P3]** Add iai-callgrind + nightly benchmark job with ±5% regression flag — verify: `.github/workflows` has a bench job

## Open questions

- Does `cargo test --workspace` currently pass locally on 1.91+? This audit is static; CI on main is the only evidence.
- Of the 310 doc-comment code fences, how many compile as doctests vs `ignore`/`text`? (Affects the real doctest count.)
- 288 `.route(` in roko-serve vs "~85 routes" in CLAUDE.md — nested routers, or is CLAUDE.md 3× stale?
- What does `roko layer-check` (ci.yml third job) actually enforce, and should it be in the CLAUDE.md pre-commit mandate?
- `apps/` (mirage-rs, agent-relay, roko-chain-watcher) are workspace members with their own tests — should they enter the crate census and per-crate targets?
- Prior draft cited crates `roko-benches`/`roko-test-utils` — neither exists in `crates/` today. Removed, or never existed?
