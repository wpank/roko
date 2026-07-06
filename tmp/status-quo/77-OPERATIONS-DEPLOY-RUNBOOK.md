# Operations, Deploy, And Runbook Ledger

> Re-verified 2026-07-08 against HEAD `5852c93c0` (`main`). Blockers below confirmed still present.

This ledger covers operational assets outside the Rust runtime.

## Confirmed blockers (file:line, current HEAD)

| Blocker | Evidence | Effect |
|---|---|---|
| Root `roko.toml` untracked but COPY'd | `git ls-files roko.toml` → empty; `Dockerfile:77 COPY roko.toml`; `railway.toml:9` lists it | Clean-checkout `docker build .` from root fails; Railway build inherits same assumption |
| Main compose uses removed `--listen` flag | `docker/docker-compose.yml:82` → `command: ["serve", "--listen", "0.0.0.0:9092"]`; current CLI exposes `--bind`/`--port` | `docker compose up` fails to start `roko-serve` |
| Fly config divergence | root `fly.toml`: image `ghcr.io/nunchi-trade/roko:latest`, `internal_port=3000`, `/api/health` vs `FLY_TOML_TEMPLATE` (`server.rs:755-778`): Dockerfile build, `internal_port=6677`, `/api/health` | `roko deploy fly` silently overwrites the root file with different port/image |
| `deploy docker` never pushes | `server.rs:441-442` build+tag only; no `--push` flag (`main.rs:1699-1709`) | Cannot publish an image via the CLI |
| Worker callback unauthenticated | `worker/handler.rs:179-182` (no auth header) | Forged deployment callbacks possible |
| `docker/gateway.Dockerfile` placeholder | `roko-gateway` crate does not exist | Listed image is unbuildable |

## Current Assets

| Area | Files | Status |
|---|---|---|
| GitHub workflows | `ci`, `coverage`, `deploy-fly`, `docker-publish`, `msrv`, `release`, `tui-parity-dry-run` | CI exists but release/deploy proof is under-scoped. |
| Docker | Root `Dockerfile`, `roko`, `worker`, `mirage`, `gateway`, `demo`, `mirage-demo` Dockerfiles plus compose/prometheus/start scripts | Build assets exist, but root image assumes a tracked `roko.toml`, compose has stale flags, and container health proof is not release-blocking. |
| Railway | `railway.toml`, `docker/RAILWAY.md`, `docker/start-railway.sh` | Substantial operational guidance; root config assumes `roko.toml` and must align with config/env matrix. |
| Fly | Root `fly.toml`, `deploy-fly` workflow, and CLI deploy command | Configs disagree on image/build, port, and health path; needs one source of truth and smoke proof after deploy. |
| Contracts | 13 Solidity sources, 10 Foundry tests, deploy script | Generated `out/` artifacts exist; tests not wired into CI. |
| Scripts/tools | layer check, coverage, parity scripts, ABI update, knowledge feedback demo | Useful operator/dev scripts; not all are part of proof gates. |

## Operational Reality

- Clean-checkout root Railway/Docker build is currently blocked until root `roko.toml` is tracked, generated, or removed from Docker/Railway/dev-compose assumptions.
- Main compose is not runnable as written because it invokes `roko serve --listen 0.0.0.0:9092`; current CLI exposes `--bind` and `--port`.
- Health/readiness exist at top-level `/health`, `/ready`, `/metrics`, and `/api/health`; worker has `/health`, `/task`, `/status`; Mirage has `/health` and `/relay/health`.
- Mirage `/relay/health` returns HTTP 200 even when JSON status is `degraded`; health checks must inspect body when using that endpoint.
- Docker publish does not run containers and curl health endpoints before pushing.
- Release workflow builds artifacts without depending on frontend, Foundry, Docker health, or deterministic runtime smoke.
- MSRV references drift across workspace (`1.85`), CI (`1.91`), Docker, and docs.
- Railway scripts handle provider env passthrough and volume roots, but path canonicalization is still spread across binaries.
- Root `fly.toml` uses a GHCR image, port `3000`, and `/api/health`; `roko deploy fly` writes a Dockerfile-based config on port `6677`.
- `docker/gateway.Dockerfile` is a placeholder because `roko-gateway` does not exist.
- Contract `out/` artifacts are generated and should be excluded from source counts and roadmap evidence.
- Demo app `dist` and `node_modules` are checked-in/generated local state in the workspace and should not count as source proof.

## Missing Runbooks

| Runbook | Needed content |
|---|---|
| Local all-up | Start `roko serve`, demo app, relay, Mirage, worker, and verify health routes. |
| Clean-checkout Docker | Prove root Dockerfile either has or does not require root `roko.toml`; build and boot image. |
| Local compose smoke | Fix compose flags, boot services, and curl `/health`, `/ready`, `/api/health`, `/metrics`. |
| Release preflight | Exact commands for Rust, frontend, contracts, Docker, and runtime smoke. |
| Railway deploy | Required env vars, `roko.toml` policy, volume roots, readiness expectations, rollback. |
| Railway multi-service | CLI worker/Mirage deploy flow, templates, callback auth, and required Dockerfiles. |
| Fly deploy | App naming, secrets, port/health source of truth, rollback. |
| Chain/Mirage | Local fork/devnet setup, ISFR proxy behavior, contract deployment proof. |
| GHCR/release | Docker publish health checks, signing/SBOM/attestation decision, artifact verification. |
| Incident/debug | Logs, StateHub events, `.roko` state doctor, config doctor, route health. |

## Checklist

- [ ] Decide whether root `roko.toml` is tracked, generated during build, or removed from Docker/Railway/compose assumptions.
- [ ] Fix compose `--listen` to current `--bind`/`--port` syntax and boot smoke it.
- [ ] Add Docker boot test for `roko`, `worker`, and `mirage` images.
- [ ] Curl `/health` and `/ready` for every deploy image before publishing.
- [ ] Curl `/api/health`, `/metrics`, worker `/status`, and Mirage `/relay/health` body where applicable.
- [ ] Reconcile root `fly.toml` with CLI-generated Fly config.
- [ ] Mark `docker/gateway.Dockerfile` as placeholder or remove it from supported image lists.
- [ ] Make release workflow depend on CI, deny, frontend, Foundry, Docker, and runtime smoke gates.
- [ ] Unify MSRV across workspace, CI, Docker, and docs.
- [ ] Repair or delete CI references to missing tmp parity scripts.
- [ ] Add an operator env reference generated from `61-CONFIG-ENV-MATRIX.md`.
- [ ] Add runbook commands that use one canonical state root resolver.
- [ ] Decide whether cargo-dist, signing, SBOM, and attestation are release requirements or target-state docs.
- [ ] Mark generated directories (`contracts/out`, `demo/demo-app/dist`, `node_modules`) as generated in archive/delete docs.

## Runbook drafts (command-level, verify before publishing)

These are the concrete steps that fill the "Missing Runbooks" table. All commands assume repo root `/Users/will/dev/nunchi/roko/roko`. Health routes confirmed in code: control plane `/health` `/ready` `/metrics` `/api/health` (`roko-serve/src/routes/`); worker `/health` `/task` `/status` (`worker/handler.rs:51-53`); Mirage `/health` `/relay/health`.

### Local all-up
```bash
cargo run -p roko-cli -- serve            # control plane on :6677
cargo run -p roko-cli -- daemon start     # background loops (NOTE: does not run job_runner)
(cd demo/demo-app && npm run dev)         # frontend :5173
curl -sf localhost:6677/health && curl -sf localhost:6677/ready && curl -sf localhost:6677/api/health
curl -sf localhost:6677/metrics | head
```

### Clean-checkout Docker (currently BLOCKED)
```bash
git clean -xdn                            # confirm roko.toml is untracked
docker build -t roko:test .               # FAILS today at `COPY roko.toml` (Dockerfile:77)
# Fix path A: generate roko.toml before COPY, or make COPY optional; Fix path B: `roko config init` in build stage
docker run --rm -p 6677:6677 roko:test    # after fix: boot and curl /health /ready
```

### Local compose smoke (currently BLOCKED)
```bash
# Fix docker/docker-compose.yml:82 first: replace `--listen 0.0.0.0:9092` with `--bind 0.0.0.0 --port 9092`
docker compose -f docker/docker-compose.yml up -d
for p in health ready api/health metrics; do curl -sf localhost:9092/$p || echo "FAIL $p"; done
```

### Railway deploy
```bash
# Requires deploy.railway_api_token in roko.toml (server.rs:279-282). Security gate blocks unless serve.auth set (server.rs:181-232).
cargo run -p roko-cli -- deploy railway [--with-mirage] [--workers coding_agent]
# Uses docker/roko.Dockerfile, healthcheck /api/health, volume /workspace/.roko (server.rs:314-317)
# Context persisted to .roko/state/railway.json; reused on subsequent deploys
```

### Fly deploy (reconcile config first)
```bash
# roko deploy fly OVERWRITES root fly.toml with FLY_TOML_TEMPLATE (port 6677, Dockerfile build) — back up first
cargo run -p roko-cli -- deploy fly
# then verify: flyctl status; curl https://<app>.fly.dev/api/health
```

### Docker publish (incomplete)
```bash
cargo run -p roko-cli -- deploy docker --registry ghcr.io/nunchi-trade
# Builds + tags only. Push manually until CLI gains a step:
docker push ghcr.io/nunchi-trade/roko:latest
docker run --rm -p 6677:6677 ghcr.io/nunchi-trade/roko:latest &   # smoke before relying on it
curl -sf localhost:6677/health
```

### Incident / debug
```bash
cargo run -p roko-cli -- daemon status          # IPC over .roko/daemon.sock
cargo run -p roko-cli -- daemon logs
cargo run -p roko-cli -- doctor                 # workspace bootstrap state
cargo run -p roko-cli -- config validate
# Daemon IPC verbs: status/stop/restart/reload/list_subscriptions/pause/resume (daemon.rs:47-69)
```
