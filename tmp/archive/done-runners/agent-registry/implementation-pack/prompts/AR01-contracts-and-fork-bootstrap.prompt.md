# Batch AR01: Contracts and mirage fork bootstrap

You are a fresh coding agent. Zero prior chat context. Read these first:

- `tmp/agent-registry/implementation-pack/context-pack/00-READ-FIRST.md`
- `tmp/agent-registry/implementation-pack/context-pack/01-TARGET-STATE.md`
- `tmp/agent-registry/implementation-pack/context-pack/02-CODE-MAP.md`
- `tmp/agent-registry/implementation-pack/context-pack/03-VERIFICATION-MATRIX.md`
- `tmp/agent-registry/05-contracts-and-identity.md`
- `apps/mirage-rs/src/main.rs`
- `apps/mirage-rs/src/rpc.rs`
- `apps/mirage-rs/src/chain/agent.rs`
- `contracts/src/AgentRegistry.sol`
- `docs/14-identity-economy/01-erc-8004-three-registries.md`

## Task

Implement the target ERC-8004 identity surface on mirage and make mirage treat
the chain as the identity source instead of expanding the Rust `AgentRegistry`.

The design target is:

- mirage forks Ethereum mainnet for the demo path
- if the upstream fork does not already have the target ERC-8004 contracts,
  deploy the target contracts into the fork/local chain state at boot
- do not solve discovery by adding endpoint metadata to mirage's Rust
  `AgentRegistry`

## Deliverables

1. New target contracts:
   - `contracts/src/IdentityRegistry.sol`
   - `contracts/src/ReputationRegistry.sol`
   - `contracts/src/ValidationRegistry.sol`
2. Boot/deploy wiring in mirage.
3. Legacy `apps/mirage-rs/src/chain/agent.rs` marked as deprecated or clearly
   kept out of the new discovery path.

## Suggested subagent split

- explorer: inspect current contract/deploy/tooling path and how mirage can
  initialize fork state
- worker A: target contract files and tests
- worker B: mirage boot/fork integration
- worker C: deprecation/compatibility cleanup for legacy registry references

## Write scope

- `contracts/src/*`
- `contracts/script/*` if needed
- `apps/mirage-rs/src/main.rs`
- `apps/mirage-rs/src/rpc.rs`
- `apps/mirage-rs/src/chain/agent.rs`
- related tests only as needed

## Constraints

1. Treat `tmp/agent-registry/05-contracts-and-identity.md` as the target ABI.
2. Do not add a new endpoint field to mirage's `AgentEntry` as the solution.
3. Preserve existing mirage functionality unless it directly conflicts with the
   new identity path.

## Acceptance criteria

- target identity contract surface exists in `contracts/src/`
- mirage startup path can expose the target identity registry on a forked or
  fresh chain
- no new code path relies on mirage's Rust registry for durable identity
- `updateAgentCardUri(uint256,string)` is the contract target, not the older
  helper signature

## Verification

Run whatever is necessary for the actual touched code, but at minimum:

```bash
cd contracts && forge build
cargo check -p mirage-rs --features "binary,chain"
```
