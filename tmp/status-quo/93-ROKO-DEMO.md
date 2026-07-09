# roko-demo — compiled chain-scenario orchestrator (`crates/roko-demo/`)

> Status-quo audit · created 2026-07-08 @ HEAD `5852c93c05` on `main` · sources: all 21 `.rs` files of `crates/roko-demo/` (~5,860 LOC), `crates/roko-demo/Cargo.toml`, root `Cargo.toml` workspace membership. Companion to `43-SURFACES-DEMO-UX.md` (which names this crate as "a fourth live TUI-class surface" but does not enumerate it) and `42-CHAIN-REGISTRIES-ISFR.md` (chain/contract context).
>
> Status vocab: ✅ wired · 🔌 built-not-wired · 🟡 partial · ❌ missing

## Why this doc exists

`roko-demo` is a **compiled workspace crate** (root `Cargo.toml` member; `crates/roko-demo/Cargo.toml:1-2`) that supersedes the old `bin/roko-demo` shell script + `demo/demo-old`. It is a self-contained chain-scenario orchestrator with **its own `clap` CLI, its own ratatui TUI, and its own WebSocket broadcast server on :9090** — a **fourth TUI-class surface** (alongside the F1–F7 CLI dashboard, the React demo-app, and Runner v2's TuiBridge). No dedicated doc enumerated it; `43` mentions it in passing. This is that ledger.

## TL;DR

- **Purpose**: manifest-driven `deploy → seed/fixtures → agent-spawn → verify` for demo chain scenarios, plus benchmark / tournament / autonomous / TUI modes. Description string: *"Manifest-driven deploy + fixtures + agent-spawn orchestrator for the roko demo environment"* (`Cargo.toml:8`).
- **It deploys REAL EVM contracts** via `alloy` (`sol!` bindings) to a JSON-RPC chain (mirage-rs / anvil), not a mock. 7 contract bindings (`bindings.rs`).
- **Own WS server on :9090** (`ws_server.rs:14 start_ws_server`; default port `main.rs:55`), broadcasting JSON-serialized `DemoEvent`s to connected dashboard clients.
- **Own ratatui TUI** (`tui.rs:183 run_tui`), 5 panels (title/agents/log/knowledge/economics), driven by an mpsc `DemoEvent` channel.
- **4 LLM backends**: `stub` (deterministic, default), `claude` (real Anthropic API), `ollama`, `multi` (round-robin) (`scenarios/llm.rs:158 create_provider`).
- **5 registered scenarios**: job_board, consortium, defi_routing, flywheel, yield_routing (`scenarios/mod.rs:60-67`).
- **Isolation**: nothing else in the repo invokes `roko-demo`/`roko_demo` (only a workspace-init string literal in `roko-serve`). It is a standalone binary, not called by the main `roko` CLI.
- **Status**: ✅ fully wired and self-contained. Not part of the self-hosting loop — it is a demo/benchmark surface.

## CLI surface (`main.rs`)

Binary `roko-demo` (`Cargo.toml:13-15`), global args: `--demo-dir` (default `demo`), `--runtime-dir` (default `demo/.runtime`), `--rpc-url`, `--llm-backend` (default `stub`), `--events` (default `none`), `--ws-port` (default `9090`), `--persist-reputation` (`main.rs:33-63`).

| Subcommand | Site | What it does |
|---|---|---|
| `up`/`run <scenario>` | `main.rs:70` | deploy + seed + agent-spawn (end-to-end) |
| `deploy <scenario>` | `main.rs:78` | deploy contracts, write `deployments.json` |
| `seed <scenario>` | `main.rs:83` | run fixtures (requires prior deploy) |
| `verify <scenario>` | `main.rs:88` | assert post-run invariants (bytecode + expected events) |
| `benchmark c-factor` | `main.rs:93,142` | measure warm-over-cold C-factor improvement |
| `tournament --rounds N` | `main.rs:98` | multi-round learning curve |
| `autonomous --agents/--jobs/--interval/--timeout` | `main.rs:105` | long-running poster/agent loop |
| `tui <scenario>` | `main.rs:118,237` | launch the ratatui TUI on a live run |
| `register-agent` | `main.rs:123` | register one agent (stake/mint/approve/register on-chain) |
| `list` | `main.rs:136` | list scenarios in the manifest |

## Subsystem census (file:line evidence)

| Subsystem | Files | Status | Evidence |
|---|---|---|---|
| **CLI + orchestration** | `main.rs` (502) | ✅ | `Cli`/`Cmd` `:31-137`; async `main` `:152` |
| **Ratatui TUI** | `tui.rs` | ✅ | `run_tui` `:183`; mpsc `DemoEvent(512)` `:188`; 5 panels `render()` `:244` (title `:262`, agents `:294`, log `:307`, knowledge `:312`, economics `:328`); `TuiState` `:48` |
| **WebSocket server (:9090)** | `ws_server.rs` | ✅ | `start_ws_server(port)` `:14`; binds `127.0.0.1:<port>` `:15`; `broadcast::channel(256)` `:17`; per-client subscribe+forward `:20-50` |
| **Event emitter abstraction** | `events.rs` | ✅ | `EventEmitter` trait `:153`; `DemoEvent` enum (23 variants) `:41-149`; `create_emitter(mode, ws_port)` `:199` (none/ndjson/ws/both); impls Null/Ndjson/Ws/Composite/Channel `:218-254` |
| **Scenario trait + registry** | `scenarios/mod.rs` + 6 scenario files | ✅ | `Scenario` trait `:45` (`name`/`register_fixtures`/`spine`); `all()` `:60`, `find()` `:71`; 5 impls registered `:62-67`; `ScenarioRuntime` `:32` |
| **Chain context** | `chain_ctx.rs` | ✅ real | `ChainCtx` `:15`; `wallet_provider` `:35`, `read_provider` `:58`, `address_of` `:70` |
| **Alloy contract bindings** | `bindings.rs` | ✅ real | `sol!` `:9`; MockERC20 `:11`, WorkerRegistry `:27`, AgentRegistry `:37`, BountyMarket `:67`, ConsortiumValidator `:88`, InsightBoard `:99`, FeeDistributor `:122` |
| **Contract deployment** | `deploy.rs` | ✅ real | reads forge artifacts `:59`; `ensure_artifacts_built` (`forge build`) `:107`; `deploy_suite` `:193`; `warmup_chain` (mirage `evm_mine`) `:154`; ctor arg coercion `:329` |
| **Manifest loader** | `manifest.rs` | ✅ | `LoadedManifest::load` / `load_scenario` / `build_deploy_ctx` `:228,443`; `write_deployments` `:345` |
| **Fixtures / seed** | `fixtures.rs` | ✅ | `RustFixture` trait `:26`; `run_fixtures` `:88`; 4 kinds (forge-script/jsonrpc/rust/contract-call) |
| **Invariant verify** | `verify.rs` | ✅ | `verify()` `:57`; bytecode-at-address check `:72`; `eth_getLogs` expected-event check `:111`; `VerifyReport` |
| **Benchmark (C-factor)** | `benchmark.rs` | ✅ | `prepare_benchmark` `:41`, `run_benchmark` `:56`; improvement% `:63` |
| **Tournament** | `tournament.rs` | ✅ | `prepare_tournament` `:46`, `run_tournament` `:61`; learning_curve `:72`, rankings `:89` |
| **Autonomous loop** | `autonomous.rs` | ✅ | `prepare_autonomous` `:34`, `run_autonomous` `:49`; per-round timeout `:63` |
| **LLM providers** | `scenarios/llm.rs` | ✅ | `LlmProvider` trait `:26`; `create_provider` `:158`; StubLlm `:37`, ClaudeApiProvider `:79`, OllamaProvider `:110`, MultiProvider `:128` |

## Scenarios (`scenarios/*.rs`)

All 6 files implement the `Scenario` trait's async `spine()`. Registered: `job_board`, `consortium`, `defi_routing`, `flywheel`, `yield_routing` (`scenarios/mod.rs:62-67`). Note `llm.rs` is the provider module, not a scenario. `yield_routing` is the default target for benchmark/tournament/autonomous/tui. Each scenario deploys its contract suite, seeds fixtures, and runs an agent bidding/execution/reputation loop emitting `DemoEvent`s.

## Drift / notes

- **Fourth TUI-class surface**: this crate's ratatui TUI is entirely separate from `roko-cli`'s F1–F7 dashboard, the React demo-app, and Runner v2's `TuiBridge`. Any "how many TUIs are there" navigation claim must count four.
- **:9090 WS server** is a second WebSocket surface distinct from `roko-serve`'s :6677 control-plane WS. Feed events do not cross between them.
- **Not in the self-hosting loop**: `roko-demo` is a demo/benchmark/marketing surface, not part of read-PRD → plan → execute → gate. CLAUDE.md's crate table does not list it.
- **Real chain dependency**: requires a running JSON-RPC endpoint (mirage-rs/anvil) and forge artifacts. `deploy.rs:107` shells to `forge build` if `out/` is empty.
- **`persist-reputation`** writes reputation snapshots to `runtime_dir` across runs (feeds tournament/benchmark curves).

## Verification checklist

- [ ] `grep -n '"crates/roko-demo"' Cargo.toml` → confirms workspace membership.
- [ ] `grep -rn "roko-demo\|roko_demo" crates/ --include='*.rs' | grep -v roko-demo/` → confirms no external invocation (only a serve workspace-init string).
- [ ] `grep -n "9090" crates/roko-demo/src/main.rs` → `:55` default ws-port.
- [ ] `grep -n "start_ws_server\|run_tui\|create_provider\|Scenario " crates/roko-demo/src/**/*.rs` → confirm the 3 surfaces + trait.
- [ ] `cargo run -p roko-demo -- list` → prints the 5 scenarios (needs `demo/manifest.toml`).

## Roadmap (ordered)

1. **[P3]** Document the :9090 WS message schema (`DemoEvent` JSON) alongside the :6677 control-plane schema so the two event surfaces are not confused (`59-API-ROUTE-LEDGER.md`, `70-RELAY-PROTOCOL-FREEZE.md`).
2. **[P3]** Add `roko-demo` to CLAUDE.md's crate table + the "how many TUIs" navigation note (currently invisible in the crate inventory).
3. **[P3]** Decide whether the demo scenarios should share the real `roko-agent` provider stack (`create_provider` here is a parallel, smaller LLM abstraction — duplicate of `roko-agent`'s backends).
4. **[P3]** Confirm forge-artifact + mirage-rs prerequisites are captured in an ops runbook (`77-OPERATIONS-DEPLOY-RUNBOOK.md`).

## Cross-references

- `43-SURFACES-DEMO-UX.md` — surface census (names this as the fourth TUI-class surface).
- `42-CHAIN-REGISTRIES-ISFR.md` — chain/contract/ISFR context.
- `73-EXAMPLES-PLANS-GRAPHS.md` — demo-resource proof status.
- `94-FEED-AGENTS-FLEET.md` — the *other* live event firehose (serve-side :6677 feed agents).
