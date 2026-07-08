# 03 — Verification Matrix

Every batch prompt includes local acceptance criteria. This file captures the
global integration target.

## Integration checkpoints

### Checkpoint A — chain identity

- target ERC-8004 contracts exist on mirage
- mirage can fork mainnet and still expose the target identity surface
- no new product logic depends on mirage's Rust `AgentRegistry`

### Checkpoint B — relay reachability

- relay accepts agent hello
- relay lists connected agents
- relay forwards messages and returns responses

### Checkpoint C — agent runtime

- `roko agent serve` starts an agent
- agent can run wallet-free through relay
- agent can update card URI on-chain when wallet/passport config is present

### Checkpoint D — default runtime shape

- mirage and relay run together by default
- same-origin `/relay/*` path works
- Docker and Railway shape reflect the default runtime

### Checkpoint E — in-repo static demo

- static demo can point at local mirage
- static demo can point at remote mirage
- static demo merges on-chain and relay-discovered agents
- static demo can message both direct and relay-backed agents

### Checkpoint F — remote mixed-topology demo

- remote Railway mirage + relay service is live
- remote deployed agent is live
- local laptop agent is live against the same relay
- static demo pointed at remote mirage can interact with both

## Common failure modes

- Rust compile hygiene: `apps/mirage-rs` and `crates/roko-cli` can fail a batch
  on `missing_docs` after otherwise-correct feature work. If you touch public
  API there, add docs before stopping.
- Over-reliance on mirage-local registry paths: the target proof must not depend
  on the old `/api/agents` discovery model.
- Wallet-only assumptions: wallet/passport-backed registration is optional.
  Wallet-free relay-backed agents must still succeed end to end.
- Report-only outputs: later batches should leave runnable or checkable assets,
  not only prose summaries.

## Final acceptance

The whole batch set is done when this commandable scenario works:

1. Deploy `mirage-rs + agent-relay` remotely.
2. Deploy one remote agent container.
3. Start one local laptop agent against the remote relay.
4. Open the in-repo static demo against the remote mirage URL.
5. Observe both agents in the UI.
6. Send test messages to both and receive responses.

## Batch-specific proof artifacts

- AR04 should leave tests that exercise relay-backed and wallet-backed
  registration paths.
- AR05 should leave a default runtime path that actually starts mirage and relay
  together and exposes `/relay/*`.
- AR06 should leave a smoke script/module for the static demo, not only updated
  browser code.
- AR07 should leave a runbook plus a helper script that validates local
  prerequisites or prints the exact remote-demo commands.
