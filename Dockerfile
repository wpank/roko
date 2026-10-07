# Roko Railway image.
#
# Three build targets:
#   runtime   — full Railway service with sidecars (mirage-rs, agent-relay, Claude CLI, Rust toolchain)
#   distroless — minimal roko-cli-only image using gcr.io/distroless/cc-debian12
#   showcase-replay — the replay-only Fly showcase (S11 F1): roko alone on Debian slim, < 250 MB
#
# The sidecars are required build artifacts for the `runtime` target. A deploy must
# fail if they do not build, instead of silently shipping a half-functional control plane.
#
# Usage:
#   docker build --target runtime   -t roko:railway .       # Full Railway deploy
#   docker build --target distroless -t roko:slim .          # Minimal distroless image
#   docker build --target showcase-replay -t roko:showcase . # Fly showcase (fly.showcase.toml)
#
# `runtime` stays the last stage, so a build without --target still makes the Railway image.

# ---- Frontend (Vite) -------------------------------------------------------
FROM node:22-bookworm-slim AS frontend
WORKDIR /app/demo/demo-app
COPY demo/demo-app/package.json demo/demo-app/package-lock.json* ./
RUN npm ci --prefer-offline
COPY demo/demo-app/ ./
RUN npm run build

# ---- Portal (Next.js static export, served at /) ---------------------------
FROM node:22-bookworm-slim AS portal
WORKDIR /app/apps/portal
ENV NEXT_TELEMETRY_DISABLED=1
COPY apps/portal/package.json apps/portal/package-lock.json ./
RUN npm ci --prefer-offline
COPY apps/portal/ ./
RUN npm run build:export

# ---- Rust binaries --------------------------------------------------------
FROM rust:1.96.1-slim-bookworm AS builder
WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        git \
        libssl-dev \
        pkg-config \
    && rm -rf /var/lib/apt/lists/* \
    && rustup component add clippy rustfmt

COPY . .
COPY --from=frontend /app/demo/demo-app/dist ./demo/demo-app/dist
COPY --from=portal /app/apps/portal/out ./apps/portal/out

# Fail instead of embedding the fallback page (crates/roko-serve/build.rs).
ENV ROKO_REQUIRE_EMBEDDED_UI=1

RUN cargo build --release -p roko-cli --bin roko --features alloy-backend,acp,fault-injection \
    && cargo build --release -p mirage-rs --bin mirage-rs --features "binary,roko" \
    && cargo build --release -p agent-relay --bin agent-relay \
    && strip target/release/roko target/release/mirage-rs target/release/agent-relay \
    && cp target/release/roko /tmp/roko \
    && cp target/release/mirage-rs /tmp/mirage-rs \
    && cp target/release/agent-relay /tmp/agent-relay

# ---- Distroless (minimal roko-cli only) ------------------------------------
# Binary-only image: no shell, no package manager, no source.
# roko reads no ROKO_BIND or ROKO_PORT variable (only docker/start-railway.sh
# does, in the runtime image), so `roko serve` takes its bind and port from
# /workspace/roko.toml: docker/roko.toml listens on 0.0.0.0:6677 and
# acknowledges the public bind. A platform's PORT variable replaces the port.
FROM gcr.io/distroless/cc-debian12 AS distroless

LABEL org.opencontainers.image.title="roko-serve (distroless)" \
      org.opencontainers.image.description="Roko control plane — minimal distroless image" \
      org.opencontainers.image.source="https://github.com/nunchi/roko"

COPY --from=builder /tmp/roko /usr/local/bin/roko
COPY docker/roko.toml /workspace/roko.toml
WORKDIR /workspace

# Runtime environment
ENV RUST_LOG=info

EXPOSE 6677

# distroless has no shell, so HEALTHCHECK CMD cannot use shell syntax.
# Use the binary itself for the health check.
HEALTHCHECK --interval=30s --timeout=5s --start-period=30s --retries=3 \
    CMD ["/usr/local/bin/roko", "status", "--json"]

ENTRYPOINT ["/usr/local/bin/roko"]
CMD ["serve"]

# ---- Frontend for the showcase (the api source, served by roko at /demo) ----
FROM frontend AS frontend-showcase
ENV VITE_SHOWCASE_SOURCE=api
RUN npm run build

# ---- Rust: roko alone (the showcase images) --------------------------------
# Only the roko binary, with default features and both UIs embedded: no mirage-rs, agent-relay
# or chain features, so the build and the image stay small.
FROM rust:1.96.1-slim-bookworm AS builder-core
WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        git \
        libssl-dev \
        pkg-config \
    && rm -rf /var/lib/apt/lists/*

COPY . .
COPY --from=frontend-showcase /app/demo/demo-app/dist ./demo/demo-app/dist
COPY --from=portal /app/apps/portal/out ./apps/portal/out

# Fail instead of embedding the fallback page (crates/roko-serve/build.rs).
ENV ROKO_REQUIRE_EMBEDDED_UI=1

RUN cargo build --release -p roko-cli --bin roko \
    && strip target/release/roko \
    && cp target/release/roko /tmp/roko

# ---- Showcase, replay only (S11 F1) ----------------------------------------
# roko on Debian slim, serving the showcase at /demo from bundles on the /data volume. tini
# reaps and forwards signals; the entrypoint checks the passphrase hash, links state to the
# volume and drops to the roko user with gosu. Small for fast cold starts after an idle exit.
FROM debian:bookworm-slim AS showcase-replay

LABEL org.opencontainers.image.title="roko showcase (replay)" \
      org.opencontainers.image.description="Roko showcase: replay only, passphrase-gated" \
      org.opencontainers.image.source="https://github.com/nunchi/roko"

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        gosu \
        libssl3 \
        tini \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --uid 1000 --create-home --shell /usr/sbin/nologin roko \
    && mkdir -p /workspace \
    && chown roko:roko /workspace

COPY --from=builder-core /tmp/roko /usr/local/bin/roko
COPY docker/showcase.roko.toml /workspace/roko.toml
COPY docker/showcase-entrypoint.sh /usr/local/bin/showcase-entrypoint.sh
RUN chmod 0755 /usr/local/bin/showcase-entrypoint.sh

WORKDIR /workspace
ENV RUST_LOG=info
EXPOSE 6677

ENTRYPOINT ["/usr/bin/tini", "--", "/usr/local/bin/showcase-entrypoint.sh"]

# ---- Runtime (full) -------------------------------------------------------
# Compiled binaries + Rust toolchain (for gate pipeline) + Claude CLI (for agent dispatch).
FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        bash \
        ca-certificates \
        coreutils \
        curl \
        git \
        gosu \
        libssl3 \
        nodejs \
        npm \
        tini \
    && npm install -g @anthropic-ai/claude-code \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/* /root/.npm

COPY --from=builder /tmp/roko /usr/local/bin/roko
COPY --from=builder /tmp/mirage-rs /usr/local/bin/mirage-rs
COPY --from=builder /tmp/agent-relay /usr/local/bin/agent-relay

# Rust toolchain (needed for gate pipeline: cargo check/clippy/test)
COPY --from=builder /usr/local/rustup /usr/local/rustup
COPY --from=builder /usr/local/cargo /usr/local/cargo
ENV RUSTUP_HOME=/usr/local/rustup
ENV CARGO_HOME=/usr/local/cargo
ENV PATH="/usr/local/cargo/bin:${PATH}"

COPY docker/start-railway.sh /usr/local/bin/start-railway
# A committed Docker default is baked into the image for clean-checkout builds.
# The start-railway script can still generate a fresh default if the file is absent
# at runtime (e.g. when a mounted workspace omits roko.toml).
# Runtime behavior is configured via ROKO_* environment variables.
COPY docker/roko.toml /workspace/roko.toml

RUN chmod +x /usr/local/bin/start-railway \
    && useradd --create-home --shell /bin/bash --uid 1000 roko \
    && mkdir -p \
        /workspace/.roko/dreams \
        /workspace/.roko/learn \
        /workspace/.roko/neuro \
        /workspace/.roko/state \
    && chown -R roko:roko /workspace

WORKDIR /workspace

ENV RUST_LOG=info
ENV SHELL=/bin/bash
ENV ROKO_BIND=0.0.0.0
ENV ROKO_PORT=6677
ENV MIRAGE_HOST=127.0.0.1
ENV MIRAGE_PORT=8545
ENV MIRAGE_CHAIN_ID=31337
ENV MIRAGE_BLOCK_INTERVAL_MS=1000
ENV MIRAGE_SNAPSHOT_INTERVAL_SECS=15
ENV ROKO_AGENT_RELAY_BIND=127.0.0.1:9011
ENV ROKO_AGENT_RELAY_URL=http://127.0.0.1:9011
ENV ROKO_MIRAGE_URL=http://127.0.0.1:8545
ENV MIRAGE_RPC_URL=http://127.0.0.1:8545

EXPOSE 6677

HEALTHCHECK --interval=30s --timeout=5s --start-period=30s --retries=3 \
    CMD public_port="${PORT:-${ROKO_PORT:-6677}}" \
    && curl -fsS "http://127.0.0.1:${public_port}/health" >/dev/null \
    && curl -fsS "http://127.0.0.1:${MIRAGE_PORT:-8545}/health" >/dev/null \
    && curl -fsS "http://${ROKO_AGENT_RELAY_BIND:-127.0.0.1:9011}/relay/health" >/dev/null

ENTRYPOINT ["/usr/bin/tini", "--", "/usr/local/bin/start-railway"]
