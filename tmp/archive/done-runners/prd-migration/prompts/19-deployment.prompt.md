# Prompt: 19-deployment

You are a fresh Claude Opus agent. Zero prior context. Read every file this prompt references before writing.

## Your mission

Generate `/Users/will/dev/nunchi/roko/roko/docs/19-deployment/`. Covers packaging, native/WASM/Docker/daemon/edge/cloud targets, daemon mode (launchd/systemd), Fly.io deployment, multi-repo subscription config, secret management, production hardening.

## Step 1 — Context pack (MANDATORY)

Read all 7 files in `/Users/will/dev/nunchi/roko/roko/tmp/prd-migration/context-pack/` in order.

## Step 2 — refactoring-prd canonical sources

1. `/Users/will/dev/nunchi/roko/refactoring-prd/10-developer-guide.md` §5 Deployment Targets (Native/WASM/Docker/Daemon/Cloud/Edge)
2. `/Users/will/dev/nunchi/roko/refactoring-prd/05-agent-types.md` §8 Deployment Flexibility table
3. `/Users/will/dev/nunchi/roko/refactoring-prd/06-interfaces.md` §7 Port Allocation
4. `/Users/will/dev/nunchi/roko/refactoring-prd/07-implementation-priorities.md` §Tier 3H (daemon, cloud deploy)

## Step 3 — SOURCE-INDEX entry `## 19-deployment.md`

Key legacy:
- `bardo-backup/prd/13-runtime/10-packaging-deployment.md`
- `bardo-backup/prd/15-dev/02-deployment.md`, `05-tooling.md`
- `bardo-backup/prd/25-mori/mori-deployment.md`
- `bardo-backup/tmp/death/07-project-deployment.md`, `29-fly-deploy.md` (extract mechanism)
- All of `bardo-backup/tmp/production/` (7 files: overview, dependency-refactor, packaging-distribution, config-and-state, deployment, migration-plan, playground-architecture)
- `bardo-backup/tmp/mori-agents/13-cli-and-deployment.md`

## Step 4 — implementation-plans

- `11-agent-dogfooding.md` §Phase 5-6 (daemon, multi-repo)
- `11-sections/phase-5-6.md` — daemon wiring, launchd plist, systemd unit, Fly.io, remote orchestrator, multi-repo config, secret management
- `modelrouting/15-operational-surface.md` — CLI commands, testing, validation, dashboard, config migration
- `modelrouting/16-production-hardening.md` — timeouts, retries, concurrency, shutdown, hedging, serve API

## Step 5 — Output and sub-doc plan

```bash
mkdir -p /Users/will/dev/nunchi/roko/roko/docs/19-deployment
```

Write **14 sub-docs** plus `INDEX.md`:

| # | Filename | Content |
|---|---|---|
| 00 | `00-packaging-and-distribution.md` | Cargo workspace packaging. Binary distribution. Installation via cargo install, brew, direct download. |
| 01 | `01-native-x86-arm.md` | Native build. Full features, all cores, optimal performance. Primary deployment target. |
| 02 | `02-wasm-browser-edge.md` | `cargo build --target wasm32-wasi -p roko-core -p roko-std`. MemorySubstrate (no file I/O). HTTP-based backend (fetch API). What works: core types, all Scorer/Router/Composer/Policy, MemorySubstrate, HDC. What doesn't: FileSubstrate, ExecAgent, ProcessSupervisor, git worktrees. Use cases: browser playground, edge functions, serverless, embedded. Example Rust code. |
| 03 | `03-docker.md` | Dockerfile example (multi-stage build, rust:1.91-slim → debian:bookworm-slim). Mount workspace, set env vars. CI/CD integration. |
| 04 | `04-daemon-launchd-macos.md` | `roko daemon --install` generates `~/Library/LaunchAgents/com.nunchi.roko.plist`. Lifecycle commands. Log rotation. Auto-restart. |
| 05 | `05-daemon-systemd-linux.md` | `/etc/systemd/user/roko.service` generation. systemctl --user start/stop/status. Restart policies. Resource limits. |
| 06 | `06-cloud-fly-io.md` | `roko daemon --export-fly > fly.toml`. fly deploy. Secrets management. Regional deployment. Scaling. |
| 07 | `07-edge-embedded.md` | Minimal feature set. ~500KB binary. IoT use cases. `cargo build` with `default-features = false` and `memory-only` feature. |
| 08 | `08-subscription-configuration.md` | Per-repo subscription config format in roko.toml. Cron schedules. File watchers. Webhook endpoints. GitHub/Slack event filters. |
| 09 | `09-multi-repo-coordination.md` | Multiple repos from one daemon instance. Per-repo isolation. Cross-repo coordination via shared Collective/Mesh. |
| 10 | `10-secret-management.md` | Environment variables. Secret stores (1Password, AWS Secrets Manager). Never in config files. Per-subscription scoped secrets. |
| 11 | `11-remote-orchestrator.md` | Running roko as a remote service. HTTP API. Authentication. Rate limiting. Cost tracking per tenant. |
| 12 | `12-production-hardening.md` | From modelrouting/16-production-hardening.md: adaptive timeouts (p95×2 per-role/per-model), full-jitter backoff, per-provider semaphores, graceful shutdown (3-phase drain), content-addressed dedup cache, hedged requests. Each mechanism explained. |
| 13 | `13-current-status-and-port-allocation.md` | Partial Docker support. Daemon mode not yet wired. Cloud deploy not yet wired. WASM feature flags exist but not validated. Port allocation table (3000/8080/8443/8545). Tier 3H roadmap. |

Plus `INDEX.md`.

## Step 7-9 — Rules, INDEX, self-check

Per context-pack rules. ≥200 lines per sub-doc, ≥2800 total. Low citation count OK.

Cross-reference 12-interfaces (roko-serve), 02-agents (model routing), 07-conductor (production hardening), 11-safety (secret management).

## CRITICAL REMINDERS

- DO NOT SUMMARIZE. DO NOT TRUNCATE.
- Apply naming map: mori → Roko Orchestrator; bardo → roko; golem → agent.
- No death framing.
- Use Write tool. Don't ask questions.
