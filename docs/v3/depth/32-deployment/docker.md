# 32-deployment/04 -- Docker Deployment

> Container shape packaging: two image variants, volume layout, health
> probes, Compose stack, CI build workflow, and cargo-chef caching.

**Parent:** [32-DEPLOYMENT](../../32-DEPLOYMENT.md)

**Source:** `docker/` (packaging artifacts)

---

## 1. Deployment Shape Model

Docker is the packaging layer for the container shape. The same Rust
binary powers all five shapes (laptop-local, single-server, container,
clustered, edge). The shape is selected by configuration, not code:

```toml
profile = "container"

[profile.container]
listen = "0.0.0.0:6677"
```

The binary stays the same; the profile controls defaults for storage,
transport, and listening behavior.

---

## 2. Image Registry and Naming

Images are published to GitHub Container Registry:

```
ghcr.io/nunchi/roko-cli:latest
ghcr.io/nunchi/roko-cli:0.3.0
ghcr.io/nunchi/roko-cli:0.3.0-full
ghcr.io/nunchi/roko-serve:latest
ghcr.io/nunchi/roko-serve:0.3.0
```

The shape is chosen by config, not encoded in the image name. The `-full`
suffix denotes images with web terminal support.

---

## 3. Two Image Variants

### Slim Images

Contain the statically linked binary and minimum runtime dependencies.
Default for production containers and clustered nodes.

| Base | Size | Shell | Use case |
|------|------|-------|----------|
| `gcr.io/distroless/cc-debian12` | small | No | Dynamic linking |
| `cgr.dev/chainguard/static` | smaller | No | Static binaries |
| `scratch` | smallest | No | Absolute minimum (musl) |

Use slim images for laptop-local service emulation, single-server
production, and clustered nodes where operator interaction happens
through logs, metrics, and the API.

### Full Images (with Web Terminal)

Add `tmux`, `ttyd`, and an entrypoint that keeps a terminal session
attached to the service. Operator-friendly for demos, debugging, and
controlled shared servers. Same binary and profile system underneath.

---

## 4. Slim Dockerfile: roko-cli

```dockerfile
FROM debian:trixie-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
       ca-certificates git curl \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -m -s /bin/bash -d /data roko \
    && mkdir -p /data/.roko/state /data/.roko/prd \
    && chown -R roko:roko /data

COPY target/x86_64-unknown-linux-musl/release/roko-cli \
     /usr/local/bin/roko

USER roko
WORKDIR /data
EXPOSE 6677
ENV RUST_LOG=info
ENTRYPOINT ["roko"]
CMD ["serve", "--bind", "0.0.0.0"]
```

### Slim Dockerfile: roko-serve

```dockerfile
FROM debian:trixie-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -m -s /bin/bash roko

COPY target/x86_64-unknown-linux-musl/release/roko-serve \
     /usr/local/bin/roko-serve

USER roko
EXPOSE 6677
ENV RUST_LOG=info
ENTRYPOINT ["roko-serve"]
CMD ["--bind", "0.0.0.0"]
```

---

## 5. State and Observability

Container deployments rely on a mounted state directory at `/data/.roko/`
(or `/var/lib/roko/`):

- `roko state export <archive>` captures substrate state, bus queues,
  and config
- `roko state import <archive>` restores the same bundle elsewhere

Observability is part of the container contract:

- Structured logs go to stderr by default (via `tracing`)
- `/metrics` exposes Prometheus-compatible metrics
- `/healthz` and `/readyz` match orchestrator probes
- OpenTelemetry can be enabled through standard environment variables

For shared container hosts, multi-tenancy is enforced by the service
layer, not the image: run one tenant per volume when isolation matters.

---

## 6. Docker Build: cargo-chef

cargo-chef splits dependency compilation from source compilation so cached
Docker layers stay hot across source-only changes:

```dockerfile
FROM rust:1.96 AS chef
RUN cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release -p roko-cli

FROM debian:trixie-slim
COPY --from=builder /app/target/release/roko-cli /usr/local/bin/roko
```

### Pre-Compiled Binary Strategy

The preferred CI path cross-compiles the Rust binary first via
cargo-zigbuild, then copies it into a minimal image. This keeps Docker
builds small, reproducible, and independent of the Rust toolchain inside
the final image.

---

## 7. docker-compose.yml

Local packaging bundle for laptop-local and single-server testing:

```yaml
version: "3.9"
services:
  roko-serve:
    image: ghcr.io/nunchi/roko-serve:latest
    ports: ["6677:6677"]
    volumes:
      - roko_data:/data/.roko
    environment:
      ANTHROPIC_API_KEY: ${ANTHROPIC_API_KEY:?Required}
      RUST_LOG: info
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:6677/readyz"]
      interval: 30s
      timeout: 5s
      retries: 3

volumes:
  roko_data:
```

Health checks use `/readyz` so Compose, Helm, Fly.io, and Kubernetes all
agree on readiness. A shared volume gives the container a durable state
directory.

### docker-compose.dev.yml

Development variant that mounts source directories and runs with debug
logging. Used alongside `roko-dev-full` for local development with
hot-reload.

---

## 8. .dockerignore

```
target/
.git/
.roko/
tmp/
*.md
.env
.env.*
generated-tests/
```

Excludes build outputs, workspace caches, and local state so the context
stays small and deterministic.

---

## 9. CI Workflow for Docker Builds

```yaml
name: Docker
on:
  push:
    tags: ["roko-cli-v*", "roko-serve-v*"]

jobs:
  build:
    runs-on: ubuntu-latest
    permissions:
      packages: write
    steps:
      - uses: actions/checkout@v4
      - uses: docker/setup-buildx-action@v3
      - uses: docker/login-action@v3
        with:
          registry: ghcr.io
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}
      - uses: docker/build-push-action@v6
        with:
          push: true
          tags: ghcr.io/nunchi/roko-cli:${{ github.ref_name }}
          file: docker/roko-cli.Dockerfile
          platforms: linux/amd64,linux/arm64
```

Multi-arch builds produce images for both AMD64 and ARM64 in a single
manifest, so `docker pull` fetches the correct architecture automatically.

---

## 10. Entrypoint Scripts

Entry points launch the service, keep the terminal session alive, and
fail the container if the service dies so the orchestrator can restart it:

```bash
#!/bin/bash
# docker/entrypoints/roko-serve.sh
set -e
exec roko-serve --bind 0.0.0.0 --port ${PORT:-6677} "$@"
```

For full images, the entrypoint starts both the service and ttyd:

```bash
#!/bin/bash
set -e
tmux new-session -d -s roko "roko-serve --bind 0.0.0.0"
ttyd -p 7681 tmux attach -t roko
```

---

## 11. Directory Structure

```
docker/
  roko-cli.Dockerfile
  roko-cli-full.Dockerfile
  roko-serve.Dockerfile
  roko-serve-full.Dockerfile
  entrypoints/
    roko-cli.sh
    roko-serve.sh
  docker-compose.yml
  docker-compose.dev.yml
  .dockerignore
```

These are packaging artifacts, not alternate code paths.

---

## 12. Implementation Status

> **Implementation status:** Docker images are designed but not built.
> The Dockerfiles, Compose files, CI workflow, and entrypoint scripts are
> specified. The same binary and profile system underlie all shapes. The
> `roko deploy docker` CLI subcommand generates a Docker deploy bundle
> (Dockerfile + .env + README). Port 6677 is the default for roko-serve.
> The preferred CI path uses pre-compiled binaries via cargo-zigbuild
> rather than in-Docker Rust compilation.
