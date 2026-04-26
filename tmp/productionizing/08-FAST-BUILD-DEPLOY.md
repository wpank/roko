# Fast Build & Deploy Guide

Goal: minimize the time from code change to running in production.

**Files not in repo yet:** `Dockerfile.runtime` and `Dockerfile.optimized` are described below but **must be added** at the repo root (or paths updated) before `docker build -f Dockerfile.runtime` works. The stock root `Dockerfile` is the slow, Rust-only path unless you adopt **04-DOCKERFILE-FIX.md**.

## Current State (slow)

```
Code change → docker build (full Rust compile, 10-15 min) → push image → Railway deploys
Total: ~15-20 minutes
```

## What You Already Have (but aren't using optimally)

| Asset | Location | Status |
|-------|----------|--------|
| GHCR image publishing | `.github/workflows/docker-publish.yml` | Publishes `ghcr.io/nunchi-trade/roko:latest` on push to main |
| Cross-compiled release binaries | `.github/workflows/release.yml` | Builds for macOS ARM64, macOS Intel, Linux glibc, Linux musl |
| Swatinem/rust-cache in CI | `.github/workflows/ci.yml` | Caches cargo registry + target dir |
| fly.toml pre-built image | `fly.toml` | Already set to `ghcr.io/nunchi-trade/roko:latest` |
| docker-compose dev stack | `docker/docker-compose.yml` | Full local stack with mirage, prometheus, grafana |
| Worker Dockerfile | `docker/worker.Dockerfile` | Includes Node, Python, Claude CLI |
| buildx with GHA cache | `docker-publish.yml` | `type=gha,mode=max` cache |

## The Three Speed Tiers

### Tier 1: Fastest (30-60 seconds total)

**Cross-compile locally → push binary-only image → deploy from registry**

```bash
# One-time setup
brew install zig
cargo install cargo-zigbuild
rustup target add x86_64-unknown-linux-gnu

# Build for Linux on your Mac (~30s incremental, ~3min clean)
cargo zigbuild --release --target x86_64-unknown-linux-gnu --bin roko

# Build + push minimal Docker image (~5s build, ~10s push)
docker buildx build -f Dockerfile.runtime -t ghcr.io/nunchi-trade/roko:latest --push .

# Deploy to Railway from image (~6s)
railway up --image ghcr.io/nunchi-trade/roko:latest
# Or Fly.io (fly.toml already configured for this):
fly deploy
```

**Total: ~45 seconds** for an incremental source change.

This works because:
- `cargo zigbuild` does incremental compilation on your local machine (warm target dir)
- The Docker image is just `COPY binary` into debian-slim (~5s to build)
- Railway/Fly pull a pre-built image instead of building (~6s deploy)

**Requires `Dockerfile.runtime`** (see below).

---

### Tier 2: Fast (2-5 minutes total)

**cargo-chef + sccache in Docker → push → deploy**

For when you can't cross-compile (C deps, CI builds, reproducibility needs).

```bash
# Build with cargo-chef (deps cached, only your code recompiles)
docker buildx build \
  -f Dockerfile.optimized \
  -t ghcr.io/nunchi-trade/roko:latest \
  --cache-from type=gha \
  --cache-to type=gha,mode=max \
  --push .

# Deploy from image
railway up --image ghcr.io/nunchi-trade/roko:latest
```

**Total: ~2-5 min** (15-30s for source-only changes with warm cache).

---

### Tier 3: Development (0 seconds deploy)

**Cloudflare Tunnel to your local machine. No deploy at all.**

```bash
# One-time setup
brew install cloudflared
cloudflared tunnel create roko-dev

# Run roko locally
cargo run -p roko-cli -- serve --bind 0.0.0.0 --port 6677

# Expose to the internet
cloudflared tunnel run --url http://localhost:6677 roko-dev
```

**Total: 0 seconds deploy.** Code changes are live immediately after `cargo build`.
Good for demos and testing. Not for production persistence.

---

## Dockerfile.runtime (for Tier 1)

Create this at the repo root:

```dockerfile
# Minimal image — no Rust toolchain, just the pre-built binary
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates libssl3 git curl \
    && rm -rf /var/lib/apt/lists/*

# Copy cross-compiled binary
COPY target/x86_64-unknown-linux-gnu/release/roko /usr/local/bin/roko

# Copy frontend assets (if pre-built)
COPY demo/demo-app/dist/ /usr/share/roko/spa/

# Copy default config
COPY roko.toml /workspace/roko.toml

WORKDIR /workspace
ENV RUST_LOG=info,roko=debug
ENV ROKO_SPA_DIR=/usr/share/roko/spa

EXPOSE 6677
CMD ["roko", "serve", "--bind", "0.0.0.0", "--port", "6677"]
```

Build time: **under 5 seconds** (no compilation, just copy).

---

## Dockerfile.optimized (for Tier 2)

Full cargo-chef + sccache + BuildKit cache mounts:

```dockerfile
# syntax=docker/dockerfile:1

# ============================================================
# Stage 0: Frontend (runs in PARALLEL with Rust stages)
# ============================================================
FROM node:22-bookworm-slim AS frontend
WORKDIR /app/demo/demo-app
COPY demo/demo-app/package.json demo/demo-app/package-lock.json* ./
RUN npm ci --prefer-offline
COPY demo/demo-app/ ./
RUN npm run build

# ============================================================
# Stage 1: Chef planner (analyze dependency graph)
# ============================================================
FROM lukemathwalker/cargo-chef:latest-rust-1.91-bookworm AS chef
WORKDIR /app

FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
RUN cargo chef prepare --recipe-path recipe.json

# ============================================================
# Stage 2: Cook dependencies (cached until Cargo.toml changes)
# ============================================================
FROM chef AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# Install sccache
RUN cargo install sccache --locked
ENV RUSTC_WRAPPER=sccache
ENV SCCACHE_DIR=/sccache

COPY --from=planner /app/recipe.json recipe.json

# Cook deps with cache mounts (this is the expensive step, cached across builds)
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,target=/sccache,sharing=locked \
    cargo chef cook --release --recipe-path recipe.json --bin roko

# Copy source and build (only your code recompiles)
COPY . .
COPY --from=frontend /app/demo/demo-app/dist/ demo/demo-app/dist/

RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,target=/sccache,sharing=locked \
    cargo build --release --bin roko && \
    strip target/release/roko

# ============================================================
# Stage 3: Runtime (minimal)
# ============================================================
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates libssl3 git curl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/roko /usr/local/bin/roko
COPY --from=builder /app/roko.toml /workspace/roko.toml

WORKDIR /workspace
ENV RUST_LOG=info,roko=debug
ENV PORT=6677
EXPOSE ${PORT}
CMD ["sh", "-c", "roko serve --bind 0.0.0.0 --port ${PORT}"]
```

**Performance comparison**:

| Scenario | Naive Dockerfile | Optimized | Speedup |
|----------|-----------------|-----------|---------|
| Clean build (cold) | 10-15 min | 3-5 min | 3x |
| Source-only change | 10-15 min | 15-30 sec | 30-60x |
| Cargo.toml change | 10-15 min | 2-3 min | 5x |
| Frontend-only change | 10-15 min | 30-60 sec | 15x |

**Why it's fast**:
- `cargo chef cook` compiles ALL dependencies once. Cached until Cargo.toml/Cargo.lock changes.
- sccache caches individual compilation units. Even when deps change, unchanged units are cached.
- BuildKit cache mounts persist across builds (locally and on Railway with service-specific IDs).
- Frontend stage runs in PARALLEL with Rust planner/cook stages (BuildKit parallelism).

---

## Optimal .dockerignore

Prevent non-essential files from busting Docker layer cache:

```
target/
.roko/
.git/
test-results/
contracts/broadcast/
tmp/
*.md
!CLAUDE.md
.github/
.vscode/
```

Every file in the Docker context is hashed for layer cache. Files that change frequently
but don't affect the build (docs, test results, tmp) should be excluded.

---

## Quick-Deploy Script

Save as `deploy.sh` at the repo root:

```bash
#!/bin/bash
set -euo pipefail

TARGET="${1:-railway}"  # railway | fly | local
REGISTRY="ghcr.io/nunchi-trade/roko"
TAG="${2:-latest}"

echo "=== Building frontend ==="
cd demo/demo-app && npm run build && cd ../..

echo "=== Cross-compiling for Linux ==="
cargo zigbuild --release --target x86_64-unknown-linux-gnu --bin roko

echo "=== Building Docker image ==="
docker buildx build \
  -f Dockerfile.runtime \
  -t "$REGISTRY:$TAG" \
  --platform linux/amd64 \
  --push .

echo "=== Deploying to $TARGET ==="
case "$TARGET" in
  railway)
    railway up --image "$REGISTRY:$TAG"
    ;;
  fly)
    fly deploy --image "$REGISTRY:$TAG"
    ;;
  local)
    docker run -p 6677:6677 \
      -e ANTHROPIC_API_KEY="$ANTHROPIC_API_KEY" \
      -e OPENAI_API_KEY="$OPENAI_API_KEY" \
      -v roko-state:/workspace/.roko \
      "$REGISTRY:$TAG"
    ;;
  *)
    echo "Unknown target: $TARGET"
    exit 1
    ;;
esac

echo "=== Done ==="
```

Usage:
```bash
chmod +x deploy.sh
./deploy.sh railway        # deploy to Railway
./deploy.sh fly            # deploy to Fly.io
./deploy.sh local          # run locally in Docker
```

---

## CI: GitHub Actions → GHCR → Railway auto-deploy

The existing `docker-publish.yml` already pushes to GHCR on main. To auto-deploy to
Railway after the image is published, add a deploy step:

```yaml
# Add to .github/workflows/docker-publish.yml after the build step:
deploy-railway:
  needs: build
  if: github.ref == 'refs/heads/main'
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - name: Install Railway CLI
      run: npm i -g @railway/cli
    - name: Deploy
      env:
        RAILWAY_TOKEN: ${{ secrets.RAILWAY_TOKEN }}
      run: railway up --image ghcr.io/nunchi-trade/roko:latest
```

This gives you: push to main → CI builds image (~3min with cache) → auto-deploy to Railway (~6s).

---

## cargo-zigbuild Setup (one-time)

```bash
# Install zig (the cross-linker)
brew install zig

# Install cargo-zigbuild
cargo install cargo-zigbuild

# Add Linux target
rustup target add x86_64-unknown-linux-gnu

# Test it works
cargo zigbuild --release --target x86_64-unknown-linux-gnu --bin roko

# If you hit OpenSSL linking issues, try musl (fully static):
rustup target add x86_64-unknown-linux-musl
cargo zigbuild --release --target x86_64-unknown-linux-musl --bin roko
```

**If OpenSSL fails**: roko uses `openssl-sys`. Options:
1. Add `features = ["vendored"]` to openssl in Cargo.toml (statically links OpenSSL)
2. Use `rustls` instead of native OpenSSL (if roko supports it)
3. Fall back to `cross` tool: `cargo install cross && cross build --release --target x86_64-unknown-linux-gnu --bin roko`

---

## Railway-Specific Cache Mount Syntax

If building ON Railway (not pre-built images), use their cache mount IDs:

```dockerfile
RUN --mount=type=cache,id=s/<SERVICE_ID>-cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=s/<SERVICE_ID>-sccache,target=/sccache \
    cargo build --release --bin roko
```

Get your service ID from the Railway dashboard or `railway status`.

---

## Decision Matrix

| Situation | Use | Time |
|-----------|-----|------|
| Quick iteration during development | Tier 3: cloudflare tunnel | 0s deploy |
| Deploying a code change | Tier 1: zigbuild + runtime image | ~45s |
| CI/CD automated deploy | Tier 2: cargo-chef in Docker | ~2-5min |
| First-time setup / clean build | Tier 2: cargo-chef in Docker | ~5min |
| Demo for someone right now | Tier 3: tunnel | 0s |
| Production release | Tier 1 or 2 via CI → GHCR → Railway | ~3-6min |

---

## Timing Benchmarks to Validate

After implementing, measure these to confirm speedups:

```bash
# Baseline: current naive Docker build
time docker build -t roko:naive .

# Tier 1: cross-compile + runtime image
time cargo zigbuild --release --target x86_64-unknown-linux-gnu --bin roko
time docker build -f Dockerfile.runtime -t roko:fast .

# Tier 2: cargo-chef optimized (source-only change)
# Touch a .rs file, then:
time docker build -f Dockerfile.optimized -t roko:chef .

# Tier 2: cargo-chef optimized (clean, no cache)
docker builder prune -af
time docker build -f Dockerfile.optimized -t roko:chef-cold .
```
