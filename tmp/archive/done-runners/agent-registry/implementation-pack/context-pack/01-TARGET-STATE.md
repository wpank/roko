# 01 — Target State

## Architecture

The target architecture is:

- `mirage-rs` behaves as a chain host
- `IdentityRegistry` is the durable agent identity source
- `agent-relay` is the live presence and transport bridge
- `roko-agent-server` is the per-agent runtime surface
- `roko agent serve` is the production way to start an agent

## Mainnet fork stance

For the demo path, `mirage-rs` should fork Ethereum mainnet by default and use
that forked chain state as the substrate. If the target ERC-8004 contracts are
not present on the upstream fork, the implementation should deploy the target
contracts into the fork/local chain state at boot.

The point is:

- no durable identity in mirage's Rust registry
- no dashboard dependency on mirage-specific endpoint metadata

## Transport stance

Message routing should work like this:

- prefer direct HTTP when an agent publishes a usable public rest endpoint
- use relay transport when an agent is relay-backed, wallet-free, or private

## Demo stance

The in-repo mirage demo UI is not optional. It is the primary end-to-end proof
surface in this repo.

Final proof:

- remote mirage + relay deployed
- remote agent connected
- local laptop agent connected
- demo UI pointed at remote mirage URL
- demo UI can list and message both

## Target ABI stance

Treat the target contract definition in
`tmp/agent-registry/05-contracts-and-identity.md` as authoritative.

In particular:

- `updateAgentCardUri(uint256,string)` is the target signature

If current helper code differs, update the helper code. Do not change the plan
to match outdated helper code.
