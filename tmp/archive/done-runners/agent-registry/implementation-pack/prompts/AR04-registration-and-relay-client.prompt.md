# Batch AR04: Agent relay client and chain registration

You are a fresh coding agent. Zero prior chat context. Read these first:

- `tmp/agent-registry/implementation-pack/context-pack/00-READ-FIRST.md`
- `tmp/agent-registry/implementation-pack/context-pack/01-TARGET-STATE.md`
- `tmp/agent-registry/05-contracts-and-identity.md`
- `tmp/agent-registry/02-relay-design.md`

Also inspect:

- `crates/roko-agent-server/src/lib.rs`
- `crates/roko-agent-server/src/registration.rs`
- `crates/roko-agent-server/src/state.rs`
- `crates/roko-agent-server/src/features/mod.rs`
- `crates/roko-agent-server/src/features/messaging.rs`
- `apps/agent-relay/src/lib.rs`
- `apps/agent-relay/tests/integration.rs`
- `crates/roko-chain/src/wallet.rs`
- `crates/roko-chain/src/types.rs`
- `tmp/agent-registry/implementation-pack/context-pack/02-CODE-MAP.md`

## Task

Teach `roko-agent-server` to:

1. connect outbound to the relay
2. publish a relay-hosted card when relay-backed
3. update `agentCardUri` on-chain when wallet/passport configuration is present
4. remain fully functional when wallet config is absent

Expected implementation shape:

- add a dedicated relay client feature/module instead of scattering relay HTTP
  calls across unrelated files
- keep the card-generation path centralized so the same card shape can be
  hosted directly or pushed to relay storage
- make the registration flow branch explicitly on wallet/passport availability:
  wallet-backed updates chain state, wallet-free skips chain writes and still
  completes relay registration

Concrete outputs expected from this batch:

- relay client code in `crates/roko-agent-server/src/features/relay_client.rs`
- wiring from server startup/config into that relay client
- tests that cover both wallet-free and wallet-backed success paths
- no regression to local `/health` or existing direct message handling

## Suggested subagent split

- explorer: inspect current registration helper and identify exact ABI mismatch
- worker A: relay client implementation
- worker B: chain registration helper alignment to `updateAgentCardUri(uint256,string)`
- worker C: tests for wallet-free and wallet-backed paths

## Write scope

- `crates/roko-agent-server/src/lib.rs`
- `crates/roko-agent-server/src/registration.rs`
- `crates/roko-agent-server/src/state.rs`
- `crates/roko-agent-server/src/features/mod.rs`
- `crates/roko-agent-server/src/features/relay_client.rs` new file
- related tests

## Constraints

1. Wallet-free must be a first-class success path, not an error path.
2. Treat `updateAgentCardUri(uint256,string)` as the target signature.
3. Relay-backed card hosting is normal, not a fallback hack.
4. Do not reintroduce the older `updateAgentCardUri(string,string)` helper path.
5. Before stopping, run the batch verification commands yourself and fix any
   compile/doc-hygiene issues that appear in touched public items.

## Acceptance criteria

- agent can connect to relay and appear in `/relay/agents`
- relay-hosted card path exists for relay-backed agents
- wallet/passport-configured agent can update card URI through the target ABI
- wallet-free agent skips chain writes cleanly
- dedicated integration tests cover relay connection, relay-hosted card
  publication, wallet-free success, and the `updateAgentCardUri(uint256,string)`
  path
- implementation clearly separates relay reachability/card hosting from the
  optional on-chain URI update path

## Verification

At minimum:

```bash
cargo check -p roko-agent-server
cargo test -p roko-agent-server --tests
```
