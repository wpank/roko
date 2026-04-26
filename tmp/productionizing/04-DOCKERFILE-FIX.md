# Dockerfile Fix: Multi-Stage Build

> **NOTE**: This is the basic fix. For the fast-build version with cargo-chef + sccache
> (15-30s rebuilds instead of 10-15min), see `08-FAST-BUILD-DEPLOY.md` → Dockerfile.optimized.
> For the fastest option (5s builds), see Dockerfile.runtime (cross-compile locally).

## In-tree reality (read before merging)

| What | Today | After this doc |
|------|--------|----------------|
| Root `Dockerfile` | Rust stage only; no Node; no `COPY demo/.../dist` | Use the multi-stage recipe below (or add an equivalent Node stage). |
| `railway.toml` | Builds **`docker/mirage-demo.Dockerfile`**, not this image | For `roko serve` + SPA, change `dockerfilePath` per **03-RAILWAY-DEPLOY.md**. |
| `crates/roko-serve/build.rs` | If `SKIP_FRONTEND_BUILD` unset and `package.json` exists, runs **`npm install` / `npm run build`** during **cargo** build | In slim `rust` images **`npm` is usually missing**, so the script skips or fails silently — **do not rely on this** in Docker without Node. |
| Frontend lockfile | `demo/demo-app/package-lock.json` | CI and the Dockerfile examples below use **`npm ci`**. If you standardize on Yarn, commit **`yarn.lock`** and swap install/build commands consistently. |
| Rust toolchain | Declared **`rust-version = "1.85"`** in workspace `Cargo.toml`; Dockerfiles often use **`rust:1.91-*`** | Align image tag with your policy; no `rust-toolchain.toml` in repo root at time of writing. |

## Problem

The current `Dockerfile` only builds the Rust binary. It doesn't build the demo-app frontend, so the embedded SPA assets are empty unless you manually run `npm run build` before `cargo build`.

## Solution

Three-stage Dockerfile:
1. **Node stage** — build demo-app
2. **Rust stage** — build roko binary (with demo-app dist/ available for rust-embed)
3. **Runtime stage** — minimal Debian with the binary

## Updated Dockerfile

```dockerfile
# ============================================================
# Stage 1: Build demo-app frontend
# ============================================================
FROM node:22-bookworm-slim AS frontend

WORKDIR /app/demo/demo-app
COPY demo/demo-app/package.json demo/demo-app/package-lock.json* ./
RUN npm ci --prefer-offline
COPY demo/demo-app/ ./
RUN npm run build
# Output: /app/demo/demo-app/dist/

# ============================================================
# Stage 2: Build roko Rust binary
# ============================================================
FROM rust:1.91-bookworm AS builder

# Install build deps
RUN apt-get update && apt-get install -y \
    pkg-config libssl-dev git \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

# Copy built frontend so rust-embed can bake it in
COPY --from=frontend /app/demo/demo-app/dist/ /app/demo/demo-app/dist/

# Build release binary
RUN cargo build --release --bin roko \
    && strip target/release/roko

# ============================================================
# Stage 3: Runtime
# ============================================================
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y \
    ca-certificates libssl3 git curl \
    && rm -rf /var/lib/apt/lists/*

# For full self-hosting: install Rust toolchain
# Uncomment if agents need to run cargo test/clippy/build:
# RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
#     sh -s -- -y --default-toolchain stable --component clippy rustfmt
# ENV PATH="/root/.cargo/bin:${PATH}"

COPY --from=builder /app/target/release/roko /usr/local/bin/roko
COPY --from=builder /app/roko.toml /workspace/roko.toml

WORKDIR /workspace

# Default config
ENV RUST_LOG=info,roko=debug
ENV PORT=6677

EXPOSE ${PORT}

CMD ["sh", "-c", "roko serve --bind 0.0.0.0 --port ${PORT}"]
```

## Notes

- **rust-embed path**: `crates/roko-serve/src/embedded.rs` looks for `../../demo/demo-app/dist/` relative to the crate manifest. The `COPY --from=frontend` ensures this path exists at build time.
- **SKIP_FRONTEND_BUILD**: The roko-serve build.rs checks for this env var. We don't set it since we want the embed.
- **Full self-hosting**: Uncomment the Rust toolchain install in stage 3 if agents need to run gates (cargo test, clippy). Adds ~500MB to image size.
- **ROKO_SPA_DIR alternative**: Instead of embedding, you can set `ROKO_SPA_DIR=/app/dist` and copy the frontend output to that path. The embedded.rs will serve from disk first.

## Build & Test Locally

```bash
# Build the image
docker build -t roko:latest .

# Run it
docker run -p 6677:6677 \
  -e ANTHROPIC_API_KEY=sk-ant-... \
  -e OPENAI_API_KEY=sk-... \
  -v roko-state:/workspace/.roko \
  roko:latest

# Verify
curl http://localhost:6677/api/health
open http://localhost:6677  # demo-app
```

## Image Size

| Stage | Adds |
|-------|------|
| Frontend assets | ~5MB |
| Rust binary (stripped) | ~50MB |
| Debian slim + deps | ~80MB |
| **Total** | ~135MB |
| + Rust toolchain (if uncommented) | ~500MB |
