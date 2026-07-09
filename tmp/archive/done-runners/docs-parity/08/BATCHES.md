# Batch Execution Contract

This batch set is now **docs-only**. It exists to keep `tmp/docs-parity/08/` honest after the PU08 audit.

Do not treat these batches as a mandate to build a Korai chain, token, registry suite, gossip mesh, settlement solver, or privacy stack.

---

## Batch Posture

- Default strategy: **rewrite the parity materials around the narrow shipped chain surface**.
- Only edit files under `tmp/docs-parity/08/`.
- Keep the shipped anchor set small: `ChainClient`, `ChainWallet`, `AlloyChainClient`, `WalletGate`, `TxSimGate`, `ChainWitnessEngine`, minimal witness helpers, and the Solidity demo contracts.
- Prefer `shipped`, `demo`, `partial`, and `deferred` language over implementation backlogs.
- If a section depends on Korai runtime features that are not in the shipped anchor set, move it to explicit `DEFERRED / Phase 2+`.

Required reads for every batch:

- [00-INDEX.md](00-INDEX.md)
- [SOURCE-INDEX.md](SOURCE-INDEX.md)
- [context-pack/agent-runbook.md](context-pack/agent-runbook.md)

---

## Recommended Serial Order

For a single agent run:

`PU08.1 -> PU08.2 -> PU08.3 -> PU08.4 -> PU08.5`

This order first resets the posture, then makes the demo-contract truth visible, then narrows the witness story, then marks the rest as deferred, and finally refreshes anchors and runner metadata.

---

## Batch Overview

| Batch | Purpose | Primary Files | Verify |
|---|---|---|---|
| `PU08.1` | Reset the overall posture around the narrow shipped chain surface | `00-INDEX.md`, `A-vision-and-chain-spec.md`, `F-built-foundation.md` | `rg -n "Phase 2\\+|DEFERRED|ChainClient|WalletGate|TxSimGate" tmp/docs-parity/08` |
| `PU08.2` | Make the core Solidity demo suite visible without overstating it | `A-vision-and-chain-spec.md`, `B-identity-and-on-chain-trust.md`, `D-job-market-and-reputation.md` | `rg -n "AgentRegistry|WorkerRegistry|BountyMarket|ConsortiumValidator|FeeDistributor|InsightBoard|MockERC20" tmp/docs-parity/08` |
| `PU08.3` | Split the shipped attestation helper from the deferred witness theory | `E-witness-triage-heartbeat.md`, `F-built-foundation.md`, `SOURCE-INDEX.md` | `rg -n "ChainWitnessEngine|attestation|deferred" tmp/docs-parity/08` |
| `PU08.4` | Mark the rest of the chain docs as Phase 2+ and remove present-tense overreach | `B-identity-and-on-chain-trust.md`, `C-gossip-and-p2p-network.md`, `D-job-market-and-reputation.md`, `G-payments-settlement-privacy.md` | `rg -n "DEFERRED|Phase 2\\+|design" tmp/docs-parity/08/B-identity-and-on-chain-trust.md tmp/docs-parity/08/C-gossip-and-p2p-network.md tmp/docs-parity/08/D-job-market-and-reputation.md tmp/docs-parity/08/G-payments-settlement-privacy.md` |
| `PU08.5` | Final consistency sweep for source anchors and local runner docs | `SOURCE-INDEX.md`, `context-pack/*.md`, `run-docs-parity.sh` | `bash -n tmp/docs-parity/08/run-docs-parity.sh` |

---

## Dependency Graph

| Batch | Depends on |
|---|---|
| `PU08.1` | — |
| `PU08.2` | `PU08.1` |
| `PU08.3` | `PU08.1` |
| `PU08.4` | `PU08.1`, `PU08.2`, `PU08.3` |
| `PU08.5` | `PU08.4` |

---

## Batch Details

### PU08.1 — Reset The Shipped Story

**Owns**:

- overall posture in [00-INDEX.md](00-INDEX.md)
- top-level chain status in [A-vision-and-chain-spec.md](A-vision-and-chain-spec.md)
- shipped foundation framing in [F-built-foundation.md](F-built-foundation.md)

**Scope**:

1. Replace broad chain-backlog framing with doc-honesty framing.
2. Keep the shipped story limited to the core traits/backend/gates/witness helper.
3. Stop using wider repo surfaces as proof that Korai runtime features ship.

**Out of scope**:

- adding new chain code
- refreshing every historical design detail
- treating adjacent demo/scaffold code as full Korai parity

---

### PU08.2 — Demo Solidity Visibility Pass

**Owns**:

- [A-vision-and-chain-spec.md](A-vision-and-chain-spec.md)
- [B-identity-and-on-chain-trust.md](B-identity-and-on-chain-trust.md)
- [D-job-market-and-reputation.md](D-job-market-and-reputation.md)

**Scope**:

1. Make the core seven-contract demo suite visible.
2. Keep them clearly scoped as demo or precursor surfaces.
3. Use them to correct claims of "nothing exists", not to imply Korai v1 is built.

**Out of scope**:

- contract migration plans
- new Solidity
- roadmap decomposition for a future contract suite

---

### PU08.3 — Witness Honesty Pass

**Owns**:

- [E-witness-triage-heartbeat.md](E-witness-triage-heartbeat.md)
- [F-built-foundation.md](F-built-foundation.md)
- [SOURCE-INDEX.md](SOURCE-INDEX.md)

**Scope**:

1. Keep `ChainWitnessEngine` in the shipped story as an attestation helper.
2. Keep only the minimal witness primitives in the source index.
3. Mark broader witness-observer ambitions as deferred.

**Out of scope**:

- watcher redesign
- new block-observer logic
- runtime naming cleanup outside docs

---

### PU08.4 — Defer The Frontier Tail

**Owns**:

- [B-identity-and-on-chain-trust.md](B-identity-and-on-chain-trust.md)
- [C-gossip-and-p2p-network.md](C-gossip-and-p2p-network.md)
- [D-job-market-and-reputation.md](D-job-market-and-reputation.md)
- [G-payments-settlement-privacy.md](G-payments-settlement-privacy.md)

**Scope**:

1. Mark the non-shipped chain design as `DEFERRED / Phase 2+`.
2. Preserve the design notes, but move them out of the shipped runtime narrative.
3. Keep only the narrow demo-contract cross-links where they are genuinely useful.

**Out of scope**:

- libp2p planning
- payment / solver planning
- privacy / futures implementation plans

---

### PU08.5 — Final Consistency Sweep

**Owns**:

- [SOURCE-INDEX.md](SOURCE-INDEX.md)
- [context-pack/agent-runbook.md](context-pack/agent-runbook.md)
- [context-pack/repo-map.md](context-pack/repo-map.md)
- [run-docs-parity.sh](run-docs-parity.sh)

**Scope**:

1. Keep the runbook aligned with the docs-only mission.
2. Ensure the source index still points only at the narrow shipped anchor set.
3. Refresh verify commands and runner text so they match the deferred posture.

**Out of scope**:

- code changes outside `tmp/docs-parity/08/`
- cargo / forge execution plans

---

## Completion Standard

A successful refresh leaves:

- a narrow shipped chain story,
- the Solidity demos visible but clearly partial,
- the witness helper separated from frontier witness theory,
- most of topic `08` explicitly deferred,
- and a source index that no longer overstates chain parity by leaning on broader adjacent surfaces.
