# syntax=docker/dockerfile:1.7
#
# mirage-rs container image (§42.2).
#
# Multi-stage build:
#   builder : rust:1.85-bookworm-slim — compiles `mirage-rs` with the `chain` feature
#             (binary + chain = HDC / knowledge / stigmergy RPC surface enabled)
#   runtime : distroless/cc-debian12:nonroot — minimal, non-root, no shell
#
# The `mirage-rs` crate's binary is named `mirage-rs` (see apps/mirage-rs/Cargo.toml).
# Build context is expected to be the `roko/` workspace root.

ARG BUILDPLATFORM
ARG TARGETPLATFORM

FROM --platform=$BUILDPLATFORM rust:1.85-bookworm-slim AS builder
WORKDIR /src

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        pkg-config \
        libssl-dev \
        ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY . .

# Build mirage-rs with the `binary` + `chain` features so the resulting image
# exposes the chain_* JSON-RPC methods and --enable-hdc/knowledge/stigmergy flags.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release -p mirage-rs --bin mirage-rs --features "binary,chain" && \
    cp target/release/mirage-rs /mirage-rs

FROM gcr.io/distroless/cc-debian12:nonroot
LABEL org.opencontainers.image.source="https://github.com/wpank/bardo"
LABEL org.opencontainers.image.description="mirage-rs: in-process EVM fork simulator with HDC/knowledge/stigmergy extensions"
LABEL org.opencontainers.image.licenses="MIT OR Apache-2.0"
LABEL org.opencontainers.image.title="mirage-rs"
LABEL org.opencontainers.image.vendor="Bardo"

COPY --from=builder --chown=nonroot:nonroot /mirage-rs /usr/local/bin/mirage-rs

USER nonroot
WORKDIR /workspace

# Default JSON-RPC port.
EXPOSE 8545

ENTRYPOINT ["/usr/local/bin/mirage-rs"]
# Default CMD: bind all interfaces, enable full chain extension surface.
CMD ["--host", "0.0.0.0", "--port", "8545", "--enable-hdc", "--enable-knowledge", "--enable-stigmergy"]
