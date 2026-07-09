# Test And Proof Inventory

> Re-verified 2026-07-08 against git HEAD `5852c93c05` (deeper second pass). This ledger separates test
> *volume* from migration *proof*, and adds a **false-green / mocked-path census** (§Mock & False-Green Census).
> Counts re-confirmed: 8,285 `#[test]` + 1,777 `#[tokio::test]` = **10,062** attribute hits across
> crates+apps+tests. Companion: [71](71-CI-RELEASE-PROOF-GAPS.md) for workflow gaps; [64](64-PARITY-TEST-MATRIX.md)
> for cross-surface parity.

**Headline:** the default `roko plan run` (Graph engine) is a **stub** — its `TaskExecutorCell`
(`crates/roko-graph/src/cells/task_executor.rs:18-93`) returns synthetic `task-output:dry-run:`/`:stub:` engrams and never
calls an LLM (`dry_run: true` by default; live branch "not yet implemented" at line 80-92). Test *volume* does not touch
this path; real dispatch is only under `--engine runner-v2`. **Any proof count that includes default-plan-run tests
overstates.** Worse: green tests exist for surfaces (Perplexity search, HTTP plan-execute, self-host e2e) whose *real*
path is broken, ignored, or flattened — see the census below.

## Counts

| Count | Value | Meaning |
|---|---:|---|
| Cargo workspace members | 35 | Only 3 default members: `roko-cli`, `roko-mcp-code`, `roko-mcp-github`; bare `cargo test` is narrower than CI (`ci.yml` uses `--workspace`). |
| Cargo targets | 161 | 108 test, 31 lib, 10 bin, 8 example, 2 bench, 2 build-script targets. |
| Tracked Rust files | 1,285 | Source and test files across crates/apps/tests. |
| Rust integration test files | 111 | Files under Rust `tests/` directories. |
| Rust test attribute hits | 10,062 | 8,285 `#[test]` + 1,777 `#[tokio::test]`; **not** a passing-test count. |
| Actual ignored Rust tests | 13 | Real `#[ignore = ...]` attributes — includes the *only* self-host e2e (`e2e_self_host.rs:15`). |
| Mock/stub/fake test-support hits | 362 across 48 files | `Mock*`/`stub`/`fake`/`wiremock` occurrences (`rg`); most concentrated in `roko-agent`. |
| Frontend Playwright spec files | 15 | 112 `test(...)` calls under `demo/demo-app/e2e`; not wired into current CI. |
| Foundry contract tests | 10 files / 52 cases | Present under `contracts/test`; not wired into current CI. |
| Top-level proof/smoke scripts | 3 | `tests/security-smoke.sh`, `tests/endpoint_smoke.py`, `tests/proof/mori-diffs/prove-runtime-end-to-end.sh`. |

## Per-crate test-attribute counts (`rg '#\[test\]|#\[tokio::test\]'`)

Volume is heavily front-loaded into the agent/CLI/core crates; the crates that carry the *runtime-critical* paths
(`roko-graph`, `roko-agent-server`, the MCP crates) are thin. High count ≠ high proof.

| Crate | Attr hits | Note (proof quality) |
|---|---:|---|
| roko-agent | 1,715 | Large share **mock/wiremock-backed** (see census): `perplexity/chat.rs` 22, `perplexity/search.rs` 21, `perplexity/embed.rs` 11, `claude_agent.rs` 25, `ollama/agent.rs` 26, `codex_agent.rs` 20, `openai_agent.rs` 19, `pool.rs` 15. Real network dispatch rarely exercised. |
| roko-cli | 1,648 | Mostly unit; the one true self-host e2e (`e2e_self_host.rs`) is `#[ignore]` + mock dispatcher. |
| roko-core | 1,173 | Kernel types/config — genuinely unit-testable, highest-value real coverage. |
| roko-learn | 918 | Bandits/router/episodes; deterministic, mostly real. |
| roko-gate | 550 | Gate logic; some rungs stubbed (oracles 4-6). |
| roko-orchestrator | 490 | DAG/merge — real, but not reachable from default `plan run` (Graph stub). |
| roko-serve | 451 | Many route tests use **mock `runtime.run_once`** (e.g. `plans.rs:1504`), proving wiring not behavior. |
| roko-compose | 430 | Prompt assembly; deterministic, real. |
| roko-conductor | 300 | Watchers/circuit breaker. |
| roko-chain | 282 | Phase 2+; no runtime backend. |
| roko-runtime | 228 | Supervisor/event bus. |
| roko-std | 224 | Includes `mock_dispatcher.rs` — the fixture behind ROKO_DISPATCHER "self-host" runs. |
| roko-neuro | 166 | Knowledge store. |
| roko-primitives | 133 | HDC. |
| roko-fs | 129 | Substrate. |
| roko-acp | 128 | ACP surface — **no cross-surface parity test** vs CLI/HTTP (see [64](64-PARITY-TEST-MATRIX.md)). |
| roko-graph | 116 | **Runtime-critical stub cell lives here; none of these 116 assert against the synthetic `dry-run:` marker.** |
| roko-dreams | 92 | No runtime trigger. |
| roko-daimon | 89 | Affect. |
| roko-index | 60 | Parser/graph. |
| roko-lang-rust/ts/go | 46 / 33 / 25 | Language support. |
| roko-agent-server | 25 | **Sidecar with "real LLM dispatch" claim has only 25 attrs;** integration tests use mock messaging (`features/messaging.rs`). |
| roko-plugin | 22 | Plugin host. |
| roko-mcp-github / code / scripts / stdio / slack | 18 / 13 / 7 / 2 / 2 | MCP crates barely covered. |
| roko-demo | 6 | Demo. |

## Mock & False-Green Census

A **false-green** is a passing test whose green result does **not** prove the feature works — because it exercises a
mock, a stub, a flattened path, or is `#[ignore]`d out of CI. Hotspots found this pass: **10**.

| # | Area | Green test(s) | Real gap the green hides | Severity |
|---|---|---|---|---|
| FG-1 | **Default `roko plan run` (Graph)** | All of `roko-graph` (116 attrs); none assert on output markers | `TaskExecutorCell` (`crates/roko-graph/src/cells/task_executor.rs:70-92`) returns `task-output:dry-run:`/`:stub:` and **never dispatches**. Live branch logs "not yet implemented" and falls back to dry-run. Core product loop silently "succeeds". | **P0 — most dangerous** |
| FG-2 | **Perplexity `research search`** | ~21 `#[tokio::test]` in `crates/roko-agent/src/perplexity/search.rs:317-626` (`perplexity_search_*`), all via `MockPoster` | Mock fabricates both the request contract (`{"queries":[...]}`, `search_batch` at `search.rs:141-188`) and a **nested** response shape (`canned_results`/`canned_batch` at `search.rs:269-313`). The live `POST /search` endpoint rejects this body (422) and returns a *flat* `results` list that the parser (`search.rs:176-187`) would fail to deserialize. CLI entrypoint: `crates/roko-cli/src/commands/research.rs:718-790`. | **P0** |
| FG-3 | **HTTP `POST /api/plans/:id/execute`** | `execute_plan_runs_runtime_with_plan_context` (`crates/roko-serve/src/routes/plans.rs:1664`) with mock runtime `run_once` (`plans.rs:1504`) | Route flattens the whole plan into a single prompt and calls `runtime.run_once` (`plans.rs:206,223`) — a *single universal-loop turn*, **not** the DAG/runner-v2 path the CLI uses. Test only proves `run_once` was called. HTTP ≠ CLI observable result. | **P0 (parity)** |
| FG-4 | **Self-hosting end-to-end** | `self_hosting_workflow_with_mock_dispatcher` (`crates/roko-cli/tests/e2e_self_host.rs:16`) | Test is `#[ignore = "...run manually"]` (line 15) **and** runs under `ROKO_DISPATCHER=mock-self-host-fixture` (line 224). The only e2e proof of "roko self-hosts" never runs in CI and never touches a real LLM. | **P0** |
| FG-5 | **coverage.yml gate** | Whole coverage job "passes" | `cargo llvm-cov ... --ignore-run-fail` (`.github/workflows/coverage.yml:20`) → job is **green even when tests fail**. Reporting only; must not be read as proof. | **P1** |
| FG-6 | **release.yml ships untested** | Release job succeeds on tag | `.github/workflows/release.yml` has **no `cargo test`/`clippy` step** before building + publishing binaries. A green release proves it *compiles*, not that it *passes*. | **P1** |
| FG-7 | **MSRV drift** | `msrv.yml` green | `Cargo.toml:93` declares `rust-version = "1.85"`; `.github/workflows/msrv.yml:21` checks **1.91**. CI proves 1.91 builds; **nothing proves the declared 1.85 does.** Comment "Must match workspace rust-version" is stale. | **P1** |
| FG-8 | **Provider integration (roko-agent)** | `perplexity/chat.rs` (22), `gemini/native.rs` (8), `openai_compat_backend.rs` (8), `tests/provider_integration.rs`, `glm_tool_loop.rs`, `kimi_*` | All driven by `MockPoster`/wiremock/`mock.rs` (`roko-agent/src/mock.rs`, 17 hits). Green proves JSON wire-shape assumptions, **not** that live providers accept them (cf. FG-2, same failure class). | **P1** |
| FG-9 | **Runner-v2 real path undertested** | — | Real dispatch only exists under `--engine runner-v2` (`crates/roko-cli/src/runner/`), but its entrypoint has thin direct coverage; the bulk of "plan" tests validate the Graph stub or orchestrator internals not reachable by default. | **P2** |
| FG-10 | **Gate oracle rungs 4-6** | roko-gate tests green | Higher rungs enriched in `orchestrate.rs enrich_rung_config` but several oracle checks are placeholder/permissive; a green gate can be a stub verdict counted as positive learning (see [64](64-PARITY-TEST-MATRIX.md) "Stub gate filtering"). | **P2** |

### Why FG-1 is the most dangerous
FG-2/FG-8 fail **loudly** at runtime (422 / transport error) — a user sees the break. FG-1 fails **silently**: the
default engine returns `success` with a synthetic engram, so `roko plan run plans/` exits 0, writes a "completed"
executor snapshot, and emits episodes — while **zero LLM work happened**. It is the flagship self-hosting loop, it is
the default (no flag needed), and not one of the 116 `roko-graph` tests asserts the output is real. Volume-based proof
claims ("10k tests pass") implicitly launder this into "self-hosting works".

## Proof Classes

| Class | Examples | Current status |
|---|---|---|
| Unit/integration Rust | Workspace tests, crate `tests/` dirs, inline tests | CI runs `cargo test --workspace`, but many pass against mocks (§census) and don't touch real dispatch. |
| Runtime smoke | Security smoke, endpoint smoke, Mori diff proof | Scripts exist; not release-blocking; some tolerate missing live providers. |
| Frontend E2E | Playwright specs (dashboard, terminal, bench, builder, config, navigation) | Exists locally; no CI gate. |
| Contract tests | Foundry (registries, bounty market, ISFR, validation, workers) | Exists locally; no CI gate. |
| Demo scripts | `demo/demo-resources/*/*.sh` | Useful demos; not deterministic proof until asserted. |
| Graph examples | `examples/graphs/*.toml` | Validation candidates; not execution proof until Graph cells are live (FG-1). |

## CI Reality — which workflows actually block a merge/release

| Workflow | Trigger | Blocks? | What it really proves | Caution |
|---|---|---|---|---|
| `ci.yml` → `test` | push main / PR | **Yes (merge)** | `cargo clippy --workspace -D warnings` + `cargo test --workspace` on **stable**. | Green includes all mock false-greens (§census); stable ≠ declared MSRV 1.85. |
| `ci.yml` → `fmt` | PR | **Yes** | `cargo fmt --all --check` (nightly). | Cosmetic only. |
| `ci.yml` → `layer-check` | PR | **Yes** | `cargo run -p roko-cli -- layer-check`. | Architecture layering, not behavior. |
| `msrv.yml` | push main / PR | **Yes** | `cargo check --workspace` @ **1.91**. | `check` not `test`; wrong pin (FG-7). |
| `coverage.yml` | push main / PR | **No** | Uploads HTML/JSON coverage. | `--ignore-run-fail` → green on failing tests (FG-5). |
| `tui-parity-dry-run.yml` | ? | **No (orphan)** | Calls `tmp/tui-parity/run-tui-parity.sh` + `tmp/ux-followup-runner/run-ux-followup.sh` — **neither script is tracked**. | Dead reference; cannot gate. |
| `docker-publish.yml` | tag/dispatch | **No test gate** | Builds/pushes image. | Does not boot image + curl `/health`. |
| `deploy-fly.yml` | dispatch | **No test gate** | Deploys. | — |
| `release.yml` | tag `v*` | **No test gate** | `cargo build --release -p roko-cli -p roko-mcp-code` then GH release. | Ships binaries with **no test/clippy** run (FG-6). |

**Net:** exactly one workflow (`ci.yml`) gates behavior on merge, and it is fully satisfied by the mock false-greens.
No release-path workflow runs tests at all.

## High-Risk Gaps

- **[P0] FG-1 Default `roko plan run` stub-success** — needs a smoke test asserting on real output and **failing on the `dry-run:`/`stub:` markers** (`crates/roko-graph/src/cells/task_executor.rs`).
- **[P0] FG-2 `research search` broken endpoint** — needs a live/contract test (deterministic recorded fixture of the *real* `/search` shape) that fails when request body or response parsing diverges.
- **[P0] FG-3 HTTP plan-execute parity** — needs a test that the HTTP surface produces the same episode/event/gate records as CLI `plan run`, not just that `run_once` was invoked.
- **[P0] FG-4 self-host e2e is ignored + mocked** — un-ignore behind a deterministic-provider tier, or add a separate gated real-provider e2e.
- `roko resume` needs a snapshot-capable end-to-end test proving skipped completed tasks.
- Frontend route parity needs a generated manifest test.
- Contract + frontend tests are present but outside the release gate.
- `cargo test --workspace` can hit live external deps: Mirage devnet defaults to Railway, and a root integration test expects `127.0.0.1:7878`.
- Required-feature targets (Mirage bins/examples, `roko-cli` snapshot tests) are skipped by plain workspace tests.

## Checklist — highest-value real (non-mock) tests to add

- [ ] **FG-1:** graph plan-run smoke that fails on `task-output:dry-run:`/`:stub:` markers (proves real dispatch or explicit unsupported error).
- [ ] **FG-2:** Perplexity `/search` contract test using a recorded *real* response shape; assert request body matches live API (kill the fabricated `{"queries":[...]}` mock contract).
- [ ] **FG-3:** CLI↔HTTP↔ACP parity test — same plan → identical episode/event/gate records across surfaces.
- [ ] **FG-4:** un-ignore self-host e2e under a deterministic provider fixture wired into CI (separate tier).
- [ ] **FG-5:** drop `--ignore-run-fail` from coverage OR add a separate coverage job that *does* fail on test failure.
- [ ] **FG-6:** add `cargo test --workspace` + `clippy` as a required job before `release.yml` build.
- [ ] **FG-7:** reconcile MSRV — pick one value; add a check that `Cargo.toml` rust-version == `msrv.yml` toolchain.
- [ ] **FG-8:** add at least one live-provider smoke per provider behind an API-key gate; label all `MockPoster` tests as wire-shape-only.
- [ ] Repair/remove orphan `tui-parity-dry-run.yml` script references.
- [ ] Docker publish: boot image + assert `/health`, `/ready` before push.
- [ ] Gate live Railway Mirage, `127.0.0.1:7878`, Ollama, provider keys, alloy, Foundry, Playwright as explicit proof tiers.
- [ ] Add a proof command next to every roadmap item that claims "done".

## Roadmap

1. **Now:** land FG-1 and FG-3 tests (they turn the two most-cited "done" claims — self-hosting loop + HTTP parity — from green-on-mock to actually verified).
2. **Next:** FG-2/FG-8 provider contract tier; FG-4 deterministic self-host e2e in CI.
3. **Then:** FG-5/FG-6/FG-7 CI-gate repairs so a merge/release cannot pass on mocked green.
4. **Later:** wire frontend + contract tests as explicit tiers; add per-provider live smokes.
