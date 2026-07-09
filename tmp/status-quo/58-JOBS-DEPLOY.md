# Jobs Marketplace + Deployment Shapes

> Status-quo audit · verified 2026-07-08 against HEAD `5852c93c0` (branch `main`) · sources: 24 code files, 10 deploy artifacts, 7 design docs, 5 test suites
> Re-verification note: every code claim below was re-checked file:line against current HEAD. No code drift since the 2026-07-07 pass — the newest commit touching jobs/daemon/deploy/worker is `bfe0f82d6` (2026-05-23). Line refs corrected where they had drifted (daemon HTTP router now at `daemon.rs:1344`).

> Current correction: the deployment subsystem is substantial, but root container/deploy proof is not green from a clean checkout. The root `Dockerfile`, `railway.toml`, and dev compose path assume a tracked root `roko.toml`; the main compose file still uses stale `roko serve --listen` syntax. Use `77-OPERATIONS-DEPLOY-RUNBOOK.md` for current blockers.

## Summary

The **jobs marketplace is a real, working local system** — file-backed store (`.roko/jobs/*.json`), full lifecycle state machine, agent matchmaking, and a background auto-execute runner that dispatches real agents — but it is **not the marketplace the v2 docs describe**. There are no payments, no escrow, no on-chain identity: `reward: "2500 KORAI"` is a display string split proportionally in a response body (`crates/roko-serve/src/routes/jobs.rs:449-485`). The chain-side marketplace (escrow, VRF/auction/direct hiring, disputes) is fully modeled in `roko-chain/src/marketplace.rs` but referenced by nothing outside its own crate. Meanwhile `.roko/jobs/` is empty — zero jobs have ever been run in this workspace.

**Deployment is the strongest subsystem audited**: `roko daemon` is a genuine background service (Unix-socket IPC + full HTTP control plane + 5 background loops) with real launchd and systemd installers; `roko deploy railway` makes real GraphQL calls to `backboard.railway.com`; `roko worker` is a real containerized task executor with control-plane callbacks; the root `Dockerfile` ships roko + mirage-rs + agent-relay + Claude CLI + Rust toolchain. Of the five v2 deployment shapes, three run today (Laptop, Server, Container); Clustered, Edge, and WASM do not exist, and the "shape as config Signal" model (`DeploymentShape` enum, `[deployment] shape =`) is pure design — shapes are selected by picking different subcommands, the old way.

Notable wiring gaps: the daemon does **not** start the job runner (auto-execute jobs only work under `roko serve`), `deploy docker` builds and tags but never pushes, and the jobs dashboard events emit **hardcoded fake token/cost metrics**.

## Current state table

| Component | Design source | Code | Status | Evidence |
|---|---|---|---|---|
| `roko job list/create/show/cancel` | v2/21-MARKETPLACE | `roko-cli/src/commands/job.rs:6-386` | ✅ | File CRUD on `.roko/jobs/*.json`; prefix ID resolution `job.rs:393-433` |
| `roko job execute` (local) | — | `commands/job.rs:282-369` | ✅ | Builds prompt by job_type, runs `run_once` universal loop (`job.rs:349`), persists status transitions |
| `roko job execute --serve-url` / `job match` | v2-depth/15 §hiring | `commands/job.rs:121-237,287-318` | ✅ | POSTs `/api/jobs/{id}/execute`, `/api/jobs/match` with auth headers |
| Job store schema | — | `roko-core/src/job.rs:85-125` (`MarketplaceJob`) | ✅ | status/state legacy alias, submission, evaluation, reward, plan_id, auto_execute |
| Serve jobs API (11 routes) | v2/21 | `roko-serve/src/routes/jobs.rs:18-33` | ✅ | CRUD + assign/start/submit/evaluate/execute/cancel + stats + match |
| Job lifecycle state machine | v2-depth/15 | `jobs.rs:35-64` | ✅ | open→assigned→in_progress→submitted→completed/failed/cancelled, transition validation `jobs.rs:616-641` |
| Agent matchmaking | v2-depth/15/01-02 (identity/reputation) | `jobs.rs:369-503` | 🟡 | Tier (Unverified…Pioneer `jobs.rs:306`), skills, reputation, load-factor scoring; agents are in-memory `AppState` registrations, no durable reputation protocol |
| Background job runner (auto_execute) | v2/25 §worker | `roko-serve/src/job_runner.rs:47-142` | ✅ | 5s poll + `JobCreated` event fast-path, stale-lock claiming `job_runner.rs:1007-1030`; started only from serve (`roko-serve/src/lib.rs:343,799`) |
| Coding-job execution path | — | `job_runner.rs:307-400,594-673` | ✅ | Materializes PRD, generates or synthesizes plan+tasks.toml, runs plan, collects artifacts + gate evidence |
| Chain monitor/analysis jobs | v2-depth/15 | `job_runner.rs:403-526` | 🟡 | Runs real `TriagePipeline` but on **synthetic MockChainClient events** (`job_runner.rs:414-434,490-502`) |
| Job evaluation | v2-depth/15/06 (economy) | `job_runner.rs:241-250` | 🟡 | Rubber-stamp: always `accepted: true, "auto-evaluated by job runner"` — no verification economy |
| On-chain marketplace (escrow, 3 hiring models, disputes) | v2-depth/15/04-06 | `roko-chain/src/marketplace.rs:1-60`, `x402.rs`, `korai_token.rs` | 🔌 | Full in-memory model (RandomVRF/BlindAuction/DirectHire, 4-level disputes) but referenced only by `roko-chain/src/lib.rs` — never called from jobs runtime |
| Demurrage timer | v2-depth/15/05 | `roko-serve/src/lib.rs:347` (`start_demurrage_timer`) | ✅ | Wired at serve startup |
| `roko daemon start/stop/status/logs/reload/restart` | v1/19/04-05, v2-depth/20/02 | `roko-cli/src/daemon.rs:300-717` | ✅ | Detached child spawn `daemon.rs:751-771`, PID/socket files in `.roko/`, graceful stop w/ SIGTERM→SIGKILL `daemon.rs:426-475` |
| Daemon runtime (what it runs) | v2-depth/20/02 | `daemon.rs:364-395` | ✅ | scheduler + fswatchers + dispatch loop + feedback loop + dream loop (5 loops, `daemon.rs:364-368`) + Unix-socket IPC + **full ~85-route HTTP router** (`daemon.rs:1344` calls `roko_serve::routes::build_router`). Confirmed: `start_job_runner` is NOT among these — grep for it in `crates/roko-cli/` returns zero hits. |
| Daemon IPC protocol | v2-depth/20/02 | `daemon.rs:47-69,1027-1135` | ✅ | status/stop/restart/reload/list_subscriptions/pause/resume over `.roko/daemon.sock` |
| `roko daemon install` (launchd) | v1/19/04 | `daemon.rs:485-554`, `daemon/launchd.rs:47-102` | ✅ | Writes `~/Library/LaunchAgents/dev.nunchi.roko.plist` (KeepAlive, RunAtLoad), runs `launchctl load -w` |
| `roko daemon install` (systemd) | v1/19/05, v1/19/12 | `daemon/systemd.rs:29-89` | ✅ | User unit w/ hardening: `NoNewPrivileges`, `ProtectSystem=strict`, `ReadWritePaths`, `WatchdogSec=60`, restart backoff; `systemctl enable+start` |
| `roko deploy railway` | v1/19/03,06 | `commands/server.rs:254-408` | ✅ | Release build, security posture gate, **real Railway GraphQL** via `RailwayApiBackend` (`roko-serve/src/deploy/railway_api.rs:16`, 923 LOC), context persisted to `.roko/state/railway.json`, `--with-mirage`, `--workers <templates>`, GitHub webhook registration via octocrab (`server.rs:545-673`) |
| `roko deploy fly` | v1/19/06 | `server.rs:410-425,487-492,755-778` | ✅ | Writes `fly.toml` from template, shells `flyctl deploy --remote-only` |
| `roko deploy docker` | v1/19/03 | `server.rs:427-445` | 🟡 | `docker build` + `docker tag` only (`server.rs:441-442`) — **no `docker push`**, no run/compose step. There is **no `--push` flag** on the CLI at all (`DeployCmd::Docker` in `main.rs:1699-1709` has only `workdir`/`registry`/`unsafe_public`). See cross-check note below re: tmp-feedback/2/35. |
| Security posture gate | v1/19/12 | `server.rs:181-232` | ✅ | Blocks deploy without `serve.auth` unless `--unsafe-public`; checklist output |
| Deploy backends (pluggable) | v2/25 | `roko-serve/src/deploy/mod.rs:131-174` | ✅ | `DeployBackend` trait; `railway-api` / `railway-cli` (shells `railway`, `railway_cli.rs:32`) / `manual` (writes bundle to `.roko/deploy-bundles/`, `manual.rs:32-81`) |
| Serve deployments API | v2/25 | `roko-serve/src/routes/deployments.rs:29-42` | ✅ | CRUD + `/task` proxy + `/callback` + logs; per-request Railway token; persisted deployments (`routes/mod.rs:125`) |
| `roko worker` | v2/25 §worker | `roko-cli/src/worker/mod.rs:38-84` | ✅ | Decodes `ROKO_TEMPLATE_JSON` (base64), `PORT` (Railway-injected), serves `/health` `/task` `/status` (`worker/handler.rs:49-55`) |
| Worker task execution | v2/25 | `handler.rs:79-197` | ✅ | Renders template prompt → `run_once` universal loop in temp workdir; result callback to `{control_plane}/api/deployments/{id}/callback` (`handler.rs:178-184`) |
| Worker cloud code-implementer | v1/19/11 (remote orchestrator) | `worker/cloud.rs:1-65` | ✅ | Ephemeral clone/branch/commit/push/PR flow with GitHub MCP |
| Root `Dockerfile` | v1/19/03 | `Dockerfile:1-113` | ✅ | Multi-stage: Vite frontend + roko + mirage-rs + agent-relay; runtime has Claude CLI + full Rust toolchain for gates; tini + healthchecks on 3 ports |
| Repo deploy artifacts | v1/19/03,06 | `railway.toml`, `railway.json`, `fly.toml`, `docker/` (6 Dockerfiles, compose, prometheus.yml, start-railway.sh), `deploy/{roko-agent,mirage}/` | ✅ | Real, current configs; `docker/RAILWAY.md` + `docker/README.md` are the actual docker docs |
| Sigstore binary verification | v1/19/00,12 | `roko-cli/src/deployment.rs:13-66` | 🔌 | `cosign verify-blob` wrapper built + exported (`lib.rs:150`) but no runtime caller (no self-update flow) |
| Observability (/metrics) | v1/19/14 | `roko-serve/src/routes/metrics.rs`, `routes/status/metrics.rs`, `docker/prometheus.yml` | 🟡 | Metrics routes + tracing sample-rate exist; no OTLP exporter wiring found |
| Subscriptions system | v1/19/08, v2-depth/20/02 | daemon IPC + `state.subscriptions` (`daemon.rs:1134`), `roko config subscriptions` | ✅ | list/pause/resume live over IPC |
| Five-shape config Signal | v2-depth/20/01 | — | ❌ | No `DeploymentShape`/`ROKO_SHAPE`/`[deployment] shape` anywhere in `crates/` (grep: 0 hits) |
| Package marketplace (publish/install/fork Cells/Graphs) | v2/21 §1-2 | — | ❌ | Nothing; `roko config plugins` is the closest primitive |
| Payments (x402/MPP/ERC-8183) | v2-depth/15/04 | `roko-chain/src/x402.rs` | 🔌 | Modeled in roko-chain, unwired to jobs or serve |
| Tests | — | `roko-serve/tests/job_lifecycle.rs`, `job_runner_integration.rs`; `roko-cli/tests/job_cli.rs`; in-file: `jobs.rs:1350-1754` (full matchmaking flow), `main.rs:4068-4135` (deploy CLI parse) | ✅ | Job lifecycle, matchmaking, runner integration covered; no daemon-install or deploy-backend integration tests |

## Deployment shapes matrix (five shapes of v2-depth/20/01 vs today)

| Shape (doc) | Doc backing (Store/Bus/Connect) | Today's equivalent | Runs today? | Evidence |
|---|---|---|---|---|
| **Laptop** | FileSubstrate JSONL / in-mem ring / localhost | `roko run`, `roko serve`, `roko dashboard`, `roko up` | ✅ | `commands/server.rs:6-136` (`cmd_up` = serve + agents); `.roko/` JSONL substrate |
| **Server** (daemon) | SQLite or JSONL / in-mem ring / unix+TCP | `roko daemon start` + `daemon install` (launchd/systemd) | ✅ | `daemon.rs:300-395`; JSONL only — no SQLite store backing |
| **Container** | SQLite on volume / in-mem ring / HTTP | `roko deploy railway\|fly\|docker`, `roko worker`, `Dockerfile`, compose | ✅ | Volume-mounts `.roko` (`fly.toml:19-21`, `server.rs:317`); still JSONL, not SQLite |
| **Clustered** | replicated Store / NATS-Redis Bus / mesh | — | ❌ | No networked Bus; `agent-relay` sidecar (`Dockerfile:37`) is a chain/agent relay, not a cluster bus; relay-backed Bus of v2/25 §1 not implemented |
| **Edge** | MemorySubstrate / tiny ring / MQTT-BLE | — | ❌ | v1/19/07 design only |
| **WASM** (6th) | kernel-only Cells | — | ❌ | No `wasm32`/`wasm-bindgen` in any Cargo.toml |
| *Shape selection itself* | config Signal, auto-detect docker/launchd/systemd | separate subcommands per shape | ❌ | `resolve_shape()` in doc is `todo!()` (`20-deployment/01:82-92`); nothing in code |

## V2-aligned

- **Job lifecycle as state machine with validated transitions + event emission** — `jobs.rs:46-64`, every transition publishes `ServerEvent::JobTransitioned` + rich `DashboardEvent`s to StateHub (`jobs.rs:834-1119`); matches the signal-flow paradigm.
- **Auto-execute loop is event-driven + poll fallback with lock-based claiming** — `job_runner.rs:52-142`; jobs auto-plan (PRD → plan → tasks.toml → gates → artifacts) exactly like the self-hosting loop (`job_runner.rs:594-763`).
- **Matchmaking uses tier/reputation/skill/load scoring with proportional reward split** — `jobs.rs:389-502`; directly prefigures v2-depth/15/02 reputation protocol.
- **Deploy backend as trait + factory** (`deploy/mod.rs:151-174`) — pluggable Connect-style abstraction; manual backend degrades gracefully to a bundle.
- **One binary, many modes** — worker/daemon/serve/CLI are all the same `roko` binary re-invoking itself (`daemon.rs:760-762`, launchd plist args); this is the v2 "same binary everywhere" premise, minus the config-Signal shape selector.
- **Hardened systemd unit** matches v1/19/12 hardening doc nearly line-for-line (`systemd.rs:35-68`).
- **Security posture gate before any cloud deploy** (`server.rs:181-232`) + serve bind-safety validation (`roko-serve/src/lib.rs:784`).
- **Demurrage timer** from tokenomics depth doc actually runs at serve startup (`lib.rs:347`).

## Cross-check: tmp-feedback/2/35-DAEMON-DEPLOYMENT-STATUS.md

That older intent doc is **partially wrong about the docker-push fix**:
- It names the file `crates/roko-cli/src/commands/deploy.rs` — **that file does not exist**. The deploy handlers live in `crates/roko-cli/src/commands/server.rs`.
- It claims "The `--push` flag is defined in the CLI args but not wired to the push command." **False** — there is no `--push` flag defined anywhere (`main.rs:1699-1709`). The fix is to *add* the flag and the push step, not merely wire an existing flag.
- Its high-level status matrix (daemon launchd/systemd/lifecycle working, Railway/Fly working, Docker partial, worker working) matches reality and this audit.
Treat #35 as stale-by-file-path; this doc supersedes it for the docker-push work item.

## Old paradigm & tech debt

- **Fake telemetry constants**: job dashboard events hardcode `input_tokens 4200`, `output_tokens 1800`, `cost_usd 0.042` / `0.003` (`jobs.rs:963-981,1053-1058`) — demo residue masquerading as efficiency data.
- **Rubber-stamp evaluation**: job runner always self-accepts (`job_runner.rs:241-250`); the CLI local path likewise marks `completed` on any non-error (`commands/job.rs:350-360`). No independent verification.
- **Duplicate job schema**: serve's `JobRecord` (`jobs.rs:66-112`, serializes status as `"state"`) vs core `MarketplaceJob` (`job.rs:85`, prefers `status`) — same file format read by two different structs with alias glue; CLI `job execute` (`commands/job.rs:319-368`) reimplements a simpler version of `job_runner::execute_job` without artifacts/gates.
- **Daemon ≠ serve for jobs**: daemon starts scheduler/fswatch/dispatch/feedback/dreams (`daemon.rs:364-368`) but **not** `start_job_runner` — `auto_execute` jobs sit idle under `roko daemon`.
- **`deploy docker` half-done**: build + tag, never pushes to the resolved registry (`server.rs:441-443`), despite `resolve_docker_registry` computing one.
- **fly.toml drift**: checked-in `fly.toml:1-25` (ghcr image, port 3000) disagrees with the template `deploy fly` overwrites it with (`server.rs:755-778`: Dockerfile build, port 6677).
- **Chain job types run on synthetic data** (`job_runner.rs:414-434,490-502`) — `chain_monitor`/`chain_analysis` are demos, not monitors.
- **SigstoreVerifier is dead code** (`deployment.rs`) — exported, tested, never called.
- **docs/docker/ is an empty directory**; real docker docs live at `docker/README.md`/`docker/RAILWAY.md`.
- **`.roko/jobs/` empty; `.roko/GAPS.md` has zero jobs/deploy/daemon entries** — subsystem untracked by the canonical gap file.
- **Root `roko.toml` not tracked but `COPY roko.toml`'d**: `git ls-files roko.toml` is empty (untracked), yet `Dockerfile:77` does `COPY roko.toml /workspace/roko.toml` and `railway.toml:9` lists `roko.toml` in its build inputs. Clean-checkout `docker build` from repo root **fails** on the missing COPY source. `Dockerfile:76` comment claims start-railway generates a default if absent, but the COPY runs at build time before any startup script — mitigation does not apply. This is the primary deploy blocker (see doc 77).
- **Worker callback has no auth**: `worker/handler.rs:179-182` POSTs the task result to `{control_plane}/api/deployments/{id}/callback` with no `Authorization` header — any party who learns the deployment id can forge callbacks. (Open question #4.)
- Worktree copies (`.roko/worktrees/`, `.claude/worktrees/`) carry stale duplicates of every deploy artifact — noise for any Glob-based tooling.

## Not implemented

- ❌ **Payments/escrow at runtime** — x402 / MPP / ERC-8183 (v2-depth/15/04); `roko-chain` models exist (🔌) but nothing charges, escrows, or settles; reward strings are cosmetic.
- ❌ **On-chain identity/attribution** — ERC-8004, passports (v2/21, v2-depth/15/01); agent registrations are in-process records.
- ❌ **Hiring models** — RandomVRF / BlindAuction / DirectHire (`roko-chain/src/marketplace.rs:6-9`) unwired; runtime "hiring" = greedy score sort, truncate to 5.
- ❌ **Package marketplace** — publish/discover/install/fork of Cells/Graphs/Racks/Knowledge Bundles with 5-tier SPI (v2/21 §2): no registry, no publish flow, no Verified Run badges.
- ❌ **Clustered shape** — no networked Bus (NATS/Redis), no replicated Store, no relay-backed Bus (v2/25 §1).
- ❌ **Edge + WASM shapes** (v1/19/02,07) — no wasm32 target, no MQTT/BLE connect.
- ❌ **Shape-as-config-Signal** — `DeploymentShape`, `[deployment]` TOML section, environment auto-detect: absent.
- ❌ **SQLite store backing** for server/container shapes — everything is JSONL FileSubstrate.
- ❌ **OTLP/telemetry export** (v1/19/14) — only in-process metrics routes + prometheus.yml scaffold.
- ❌ **Daemon watchdog integration** — systemd unit declares `WatchdogSec=60` but the daemon never calls `sd_notify`.

## Migration checklist

- [ ] **[P0]** Start `job_runner` in daemon mode so `auto_execute` works under `roko daemon` — add `job_runner::start_job_runner` beside `daemon.rs:364-368` — verify: `roko daemon start && roko job create --title t --auto-execute && sleep 10 && roko job show <id>` → completed
- [ ] **[P0]** Remove hardcoded token/cost dashboard metrics (`jobs.rs:963-981,1053-1058`); source from real episode/efficiency data or omit — verify: `grep -n '4200.0\|0.042' crates/roko-serve/src/routes/jobs.rs` → empty
- [ ] **[P1]** Unify `JobRecord` and `MarketplaceJob` into one roko-core type (keep `state` alias for compat) — verify: `cargo test -p roko-serve --test job_lifecycle`
- [ ] **[P1]** Make CLI local `job execute` delegate to `job_runner::execute_job` logic (artifacts + gate evidence) instead of its own `run_once` shortcut — verify: `roko job create --type coding_task … && roko job execute <id> && roko job show <id>` shows submission artifacts
- [ ] **[P1]** `deploy docker`: add `docker push` (and optional `--push/--no-push`) — verify: `roko deploy docker --registry ghcr.io/nunchi-trade` pushes
- [ ] **[P1]** Reconcile root `fly.toml` with `FLY_TOML_TEMPLATE` (one source of truth; don't silently overwrite user file) — verify: `roko deploy fly` on a repo with existing fly.toml prompts or merges
- [ ] **[P2]** Replace rubber-stamp evaluation with gate-verdict-driven accept/reject in `job_runner.rs:241-250` — verify: failing-gate job ends `failed`/rejected, not `completed`
- [ ] **[P2]** Wire `roko-chain` marketplace (escrow + hiring models) behind a feature flag on `/api/jobs` assign/evaluate — verify: `roko job create --reward "100 KORAI"` creates escrow entry in chain state
- [ ] **[P2]** Introduce `DeploymentShape` config (`[deployment] shape=`, `ROKO_SHAPE`, auto-detect docker/launchd/systemd) mapping to existing serve/daemon/worker entrypoints — verify: `ROKO_SHAPE=container roko serve` logs resolved shape
- [ ] **[P2]** Feed real chain endpoints into `chain_monitor`/`chain_analysis` jobs (config-driven RPC instead of `MockChainClient`) — verify: job summary reports non-synthetic block numbers
- [ ] **[P3]** Wire `SigstoreVerifier` into a `roko self-update`/install path or delete it — verify: `grep -rn 'SigstoreVerifier' crates/ | grep -v deployment.rs | grep -v lib.rs` shows a caller
- [ ] **[P3]** sd_notify watchdog pings in daemon when running under systemd — verify: `systemctl --user show roko.service -p WatchdogTimestamp` advances
- [ ] **[P3]** Populate `docs/docker/` (or delete it and link `docker/RAILWAY.md`); log jobs/deploy gaps into `.roko/GAPS.md` — verify: `ls docs/docker/` non-empty or gone

## Open questions

1. **Which marketplace is the target?** v2/21 describes a *package* marketplace (Cells/Graphs/prompts); the code implements a *labor* marketplace (jobs + agents). v2-depth/15 covers both. Are these one system or two subsystems?
2. **Is KORAI meant to go live?** `roko-chain` has `korai_token.rs` + demurrage timer already running in serve — is the intent mirage-simulated economy first, real chain later (Phase 2+ per CLAUDE.md)?
3. **Daemon vs serve identity**: daemon already serves the full control-plane router on its port. Is `roko daemon` supposed to converge with `roko serve --background` (one code path), or stay a separate supervisor with IPC?
4. **Worker trust model**: workers receive base64 templates via env and call back over plain HTTP with no auth header on the callback (`handler.rs:178-184`) — is callback auth (deployment token) planned?
5. **`deploy railway --workers`** requires `docker/worker.Dockerfile` in the repo being deployed — is worker deployment intended only for the roko repo itself, or for arbitrary user repos (which lack that Dockerfile)?
6. **Where should reputation persist?** Match scoring reads in-memory `AppState` agent records; restarts lose reputation/past-jobs unless the snapshot covers them — is `.roko/` persistence or chain the intended store?
