# 02 — Code Map

These are the main code paths involved in this batch set.

## Chain and mirage

- `apps/mirage-rs/src/main.rs`
- `apps/mirage-rs/src/rpc.rs`
- `apps/mirage-rs/src/http_api/mod.rs`
- `apps/mirage-rs/src/http_api/topology.rs`
- `apps/mirage-rs/src/chain/agent.rs`
- `apps/mirage-rs/src/persist.rs`
- `apps/mirage-rs/static/quickstart.sh`
- `apps/mirage-rs/static/index.html`
- `apps/mirage-rs/static/js/api.js`
- `apps/mirage-rs/static/js/polling.js`
- `apps/mirage-rs/static/js/state.js`
- `apps/mirage-rs/static/js/main.js`
- `apps/mirage-rs/tests/http_api.rs`

## Relay

- `apps/agent-relay/src/main.rs`
- `apps/agent-relay/src/lib.rs`
- `apps/agent-relay/tests/integration.rs`

## Agent server

- `crates/roko-agent-server/src/lib.rs`
- `crates/roko-agent-server/src/state.rs`
- `crates/roko-agent-server/src/registration.rs`
- `crates/roko-agent-server/src/features/messaging.rs`
- `crates/roko-agent-server/src/features/mod.rs`
- `crates/roko-agent-server/src/features/relay_client.rs`

## CLI

- `crates/roko-cli/src/main.rs`
- `crates/roko-cli/src/config.rs`
- `crates/roko-cli/src/agent_config.rs`
- `crates/roko-cli/src/serve_runtime.rs`
- `crates/roko-cli/tests/`

## Contracts

- `contracts/src/AgentRegistry.sol` (legacy minimal contract)
- `contracts/src/IdentityRegistry.sol`
- `contracts/src/ReputationRegistry.sol`
- `contracts/src/ValidationRegistry.sol`
- `contracts/test/`
- `docs/14-identity-economy/01-erc-8004-three-registries.md`
- `tmp/agent-registry/05-contracts-and-identity.md`

## Deployment

- `docker/mirage.Dockerfile`
- `docker/roko.Dockerfile`
- `docker/demo.Dockerfile`
- `docker/docker-compose.yml`
- `railway.toml`

## Demo proof assets

- `tmp/agent-registry/remote-demo-runbook.md`
- `tmp/agent-registry/scripts/remote-demo-check.sh`

## Sharp edges

- `apps/mirage-rs` and `crates/roko-cli` compile with `--warn=missing_docs`.
  If you touch public structs, fields, enums, or functions in those crates,
  add doc comments as part of the same change.
- Do not treat `mirage`'s Rust `AgentRegistry` as the durable source of agent
  identity. For the new path it is runtime-local only.
- Wallet-free agents are a normal production path. Do not force dummy wallet or
  passport config just to satisfy one branch of the implementation.

## External dashboard repo, optional batch

- `/Users/will/dev/nunchi/nunchi-dashboard/src/components/ai-studio/AskPanel.tsx`
- `/Users/will/dev/nunchi/nunchi-dashboard/src/services/mirage-api.ts`
- `/Users/will/dev/nunchi/nunchi-dashboard/src/services/constants.ts`
- `/Users/will/dev/nunchi/nunchi-dashboard/.env.example`
