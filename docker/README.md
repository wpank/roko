# roko/docker

Container images and local-stack compose file for Roko (§42.a of the Mori parity checklist).

## Contents

| File | Purpose | Checklist |
| --- | --- | --- |
| `roko.Dockerfile` | Multi-stage build of the `roko` CLI (control plane) | §42.1 |
| `worker.Dockerfile` | Agent worker image (Claude CLI, Python/OpenAI, curl/Ollama) | §42.1b |
| `mirage.Dockerfile` | `mirage-rs` (EVM fork simulator) with `binary,chain` features | §42.2 |
| `gateway.Dockerfile` | Placeholder `roko-gateway` image (TODO until the crate exists) | §42.3 |
| `docker-compose.yml` | roko + gateway + mirage + prometheus + grafana topology | §42.4 |
| `prometheus.yml` | Scrape config consumed by the Prometheus service | §42.4 |
| `entrypoint.sh` | Root-to-user entrypoint: fixes volume perms then drops privileges via gosu | — |
| `RAILWAY.md` | Railway multi-service deployment guide | — |

Multi-arch build and GHCR publishing live in `.github/workflows/docker-publish.yml` (§42.5, §42.6).

## Quick start

```bash
cd roko/docker
docker compose up --build
```

First build pulls the Rust toolchain and warms the cargo registry/target caches
(takes a few minutes). Subsequent builds reuse BuildKit cache mounts.

The root `Dockerfile` bakes the committed `docker/roko.toml` into
`/workspace/roko.toml`, so a clean checkout has a valid runtime configuration. A
mounted workspace can supply its own root `roko.toml` instead.

## Services and ports

| Service | Port(s) | Notes |
| --- | --- | --- |
| `mirage` | `8545` JSON-RPC, `9091` metrics | Default CMD enables `--enable-hdc --enable-knowledge --enable-stigmergy` and binds `0.0.0.0`. |
| `roko` | `9092` | Runs `roko serve --bind 0.0.0.0 --port 9092`; operators can `docker compose exec roko roko <cmd>`. |
| `gateway` | `8080` | Placeholder — see the TODO in `gateway.Dockerfile`. |
| `prometheus` | `9090` | Scrapes mirage/roko/gateway + itself. |
| `grafana` | `3000` | Login `admin / admin` unless overridden. |

## Environment variables

| Variable | Service | Default | Description |
| --- | --- | --- | --- |
| `RUST_LOG` | roko, mirage, gateway | `info` | `tracing-subscriber` env-filter. |
| `ROKO_MIRAGE_URL` | roko, gateway | `http://mirage:8545` | Upstream RPC endpoint. |
| `GF_SECURITY_ADMIN_USER` | grafana | `admin` | Grafana admin user. |
| `GF_SECURITY_ADMIN_PASSWORD` | grafana | `admin` | Grafana admin password — override in prod. |

## State persistence

mirage-rs supports periodic atomic snapshots of all in-memory state (dirty accounts, deployed contracts, chain extension data) to a single JSON file. This lets seeded data survive container restarts.

To enable persistence in Docker Compose, add a named volume and pass `--state-dir`:

```yaml
services:
  mirage:
    volumes:
      - mirage-state:/workspace/.roko
    command:
      - "--host"
      - "0.0.0.0"
      - "--port"
      - "8545"
      - "--enable-hdc"
      - "--enable-knowledge"
      - "--enable-stigmergy"
      - "--state-dir"
      - "/workspace/.roko/state"
      - "--snapshot-interval-secs"
      - "15"

volumes:
  mirage-state:
```

See `apps/mirage-rs/README.md` > "State persistence" for full details on what is persisted, Railway deployment, and configuration options.

## Healthcheck

`mirage` has a healthcheck that invokes `mirage-rs healthcheck --url http://127.0.0.1:8545`,
which issues an `eth_blockNumber` RPC. Other services `depends_on` mirage so
they wait for it to come up.

The Roko server exposes stable unauthenticated probes at `GET /health` (liveness)
and `GET /ready` (readiness). With the compose port mapping, check them at
`http://127.0.0.1:9092/health` and `http://127.0.0.1:9092/ready`. The current
compose topology starts Roko after Mirage but does not gate other services on a
Roko healthcheck; Fly and the root runtime image use `/health` directly.

## Publishing

Three images are published to GHCR by `.github/workflows/docker-publish.yml` on pushes to `main` and on `v*` tags:

| Image | Dockerfile |
| --- | --- |
| `ghcr.io/nunchi-trade/roko:latest` | `roko.Dockerfile` |
| `ghcr.io/nunchi-trade/roko-worker:latest` | `worker.Dockerfile` |
| `ghcr.io/nunchi-trade/mirage:latest` | `mirage.Dockerfile` |

Tags: `latest` (main only), `sha-<short>`, `v<version>` (semver from git tag).

## Railway deployment

See [RAILWAY.md](RAILWAY.md) for the full multi-service deployment guide covering:
- `roko deploy railway` CLI (control plane + mirage + workers)
- Per-user Railway tokens via REST API
- Multi-backend workers (Claude, OpenAI, Ollama)
- Volume persistence and troubleshooting

## TODO

- Replace the `roko-gateway` placeholder once the crate is scaffolded.
- Add a Grafana provisioning volume for default dashboards.
- Add alertmanager once §40 alerting lands.
