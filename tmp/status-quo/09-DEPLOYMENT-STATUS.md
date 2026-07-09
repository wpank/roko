# 09 — Deployment Status

**Docker, Railway, Fly.io, CI/CD.**

---

> Current correction, re-verified 2026-07-08 against HEAD `5852c93c0`: this inherited deployment status overstates readiness. Confirmed blockers: (1) root `roko.toml` is UNTRACKED (`git ls-files roko.toml` empty) yet `Dockerfile:77` does `COPY roko.toml` — clean-checkout `docker build .` fails; (2) `docker/docker-compose.yml:82` uses the removed `--listen` flag (current CLI: `--bind`/`--port`); (3) root `fly.toml` (image `ghcr.io/nunchi-trade/roko`, port 3000) diverges from `roko deploy fly`'s template (Dockerfile build, port 6677). Use `77-OPERATIONS-DEPLOY-RUNBOOK.md` for command-level runbooks and `58-JOBS-DEPLOY.md` for the full deploy/jobs/daemon matrix. Note: unlike what the "Infrastructure Gaps" table below says, the **worker now has `/health`** (`worker/handler.rs:51`).

## Railway (Recommended Target, Blocked Until Proven)

- **Config**: `railway.toml` + `Dockerfile`
- **Status**: ⚠️ Configured but clean-checkout build proof is blocked by the missing root `roko.toml` assumption
- **Architecture**: Single service with 3 co-located processes
  - `roko serve` (HTTP control plane on :6677)
  - `roko daemon` (background workers)
  - Agent processes (spawned per-task)
- **Health check**: `/health` endpoint
- **Secrets**: Via Railway environment variables
- **Deployment**: `railway up` from repo root
- **Notes**: The recommended deployment target. Handles scaling automatically.

## Docker Compose (Development)

- **Config**: `docker-compose.yml` + `docker-compose.dev.yml`
- **Status**: ⚠️ Assets exist, but current compose command flags must be fixed and smoke-tested
- **Services**:
  - `roko-serve`: HTTP control plane
  - `roko-worker`: Background task execution
  - Networking between services configured
- **Dev mode**: `docker compose -f docker-compose.dev.yml up`
  - Hot-reload with cargo watch
  - Volume mounts for source code

## Fly.io

- **Config**: `fly.toml` (if present)
- **Status**: ⚠️ Stale — config may not reflect current architecture
- **Notes**: Was an early deployment target. Railway is now preferred. Fly config should be audited or removed if no longer used.

## CI/CD

### GitHub Actions
- **Workflows**: `.github/workflows/`
- **Checks**:
  - `cargo +nightly fmt --all` — formatting
  - `cargo clippy --workspace --no-deps -- -D warnings` — linting
  - `cargo test --workspace` — test suite
- **Status**: ✅ Active, runs on PR and push
- **Requirement**: Rust 1.91+ (for alloy deps)

### Pre-Commit Requirements
```bash
cargo +nightly fmt --all                              # Nightly formatter
cargo clippy --workspace --no-deps -- -D warnings     # Zero warnings
cargo test --workspace                                 # All tests pass
```

## Dev Environment

| Tool | Command | Port |
|------|---------|------|
| HTTP server | `roko serve` | :6677 |
| Demo frontend | `cd demo/demo-app && npm run dev` | :5173 |
| Docker dev | `docker compose -f docker-compose.dev.yml up` | :6677 |
| TUI dashboard | `roko dashboard` | N/A (terminal) |
| Combined | `roko-dev-full` (local alias, not in repo) | :6677 + :5173 |

## Infrastructure Gaps

| Gap | Description | Priority |
|-----|-------------|----------|
| Fly.io config stale | May not match current architecture | Low — remove if unused |
| No staging environment | Production deployments go direct | Medium |
| ~~No health check for worker~~ | RESOLVED — worker exposes `/health` `/task` `/status` (`worker/handler.rs:51-53`) | — |
| Worker callback unauthenticated | `worker/handler.rs:179-182` POSTs results with no auth header | Medium |
| Docker CLI publish incomplete | `deploy docker` builds+tags, no push; no `--push` flag exists | Medium |
| No automated deployment | Manual `railway up` / `git push` | Medium |
| Demo app deployment | No deployment config for frontend | Low |

## Container Image

- **Dockerfile**: Multi-stage Rust build
- **Base image**: Debian slim (runtime)
- **Build time**: ~5-10 min cold, faster with cache layers
- **Image size**: Moderate (Rust binary + runtime deps)
- **Rust version**: 1.91+ required (alloy dependency)
