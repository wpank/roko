# Batch AR05: Mirage runtime, proxy, Docker, Railway shape

You are a fresh coding agent. Zero prior chat context. Read these first:

- `tmp/agent-registry/implementation-pack/context-pack/00-READ-FIRST.md`
- `tmp/agent-registry/implementation-pack/context-pack/01-TARGET-STATE.md`
- `tmp/agent-registry/04-deployment-and-dev.md`
- `tmp/agent-registry/implementation-pack/context-pack/02-CODE-MAP.md`

Also inspect:

- `docker/mirage.Dockerfile`
- `docker/roko.Dockerfile`
- `docker/docker-compose.yml`
- `railway.toml`
- `apps/mirage-rs/src/rpc.rs`
- `apps/mirage-rs/src/http_api/mod.rs`
- `apps/agent-relay/src/main.rs`
- `tmp/agent-registry/implementation-pack/context-pack/03-VERIFICATION-MATRIX.md`

## Task

Make `mirage-rs + agent-relay` the default runtime shape for local and Railway
verification.

This includes:

- building and running relay next to mirage in the default Docker path
- exposing relay under `/relay/*` on the same origin by default
- updating local/remote runtime assumptions accordingly

Expected implementation shape:

- one default process shape for demos and Railway: `mirage-rs + agent-relay`
- same-origin proxying from mirage to relay for both read and message traffic
- minimal extra moving parts; avoid creating a second parallel runtime path that
  the demo or docs then need to special-case

Concrete outputs expected from this batch:

- a default container/runtime path that launches both services
- mirage-side `/relay/*` forwarding support
- updated Railway/runtime config that matches the default local shape
- verification coverage for the proxy path, not only compilation

## Suggested subagent split

- explorer: inspect current Docker, `rpc.rs`, and `http_api` structure for the
  smallest viable same-origin approach
- worker A: Dockerfile and entrypoint/runtime wiring
- worker B: same-origin `/relay/*` forwarding path
- worker C: local smoke verification or tests where practical

## Write scope

- `docker/mirage.Dockerfile`
- `docker/roko.Dockerfile` only if needed
- `docker/docker-compose.yml` if needed for local parity
- `railway.toml`
- `apps/mirage-rs/src/rpc.rs`
- `apps/mirage-rs/src/http_api/mod.rs`
- any small helper scripts needed for runtime startup

## Constraints

1. Keep `agent-relay` conceptually separate even though it is co-deployed.
2. Prefer one-origin default behavior.
3. Do not reintroduce `roko-serve` into the discovery/messaging path.
4. If you touch public `mirage-rs` items, fix `missing_docs` issues in the same
   batch before stopping.
5. Keep the proxy shape simple enough that AR06 can consume it without adding
   another layer of routing logic.

## Acceptance criteria

- default Docker path can run mirage and relay together
- relay is reachable under `{CHAIN_URL}/relay/*` in the default setup
- Railway config can support the same shape
- the same-origin forwarding path is explicit in code and not only described in
  docs

## Verification

At minimum:

```bash
cargo check -p mirage-rs --features "binary,chain"
cargo test -p mirage-rs --test http_api relay_proxy
docker build -f docker/mirage.Dockerfile .
```
