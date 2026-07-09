# Batch Execution Contract

8 batches ordered for unattended execution. The goal is not just to “cover the chain docs”, but to let an agent turn the chain-layer parity findings into bounded work that can run overnight without re-reading 8,000+ lines of PRD first.

Because the chain layer is Tier-6 deferred, this batch is primarily **docs / audit / cross-link / status-contract work**. The default outcome is better truth-in-advertising, not new chain code.

---

## Batch Posture

- Default strategy: **honest status docs over new Solidity, libp2p, payment, or privacy code**.
- Treat `docs/08-chain/24-current-status-and-6-contracts.md` as the primary doc-contract hotspot.
- Treat `apps/mirage-rs/src/chain/` as the primary scaffold hotspot.
- Treat `contracts/src/*.sol` as a real shipping surface that the docs must acknowledge.
- Treat `apps/mirage-rs/src/http_api/isfr.rs` as the primary “partial but misleading” hotspot for the payments/settlement docs.
- If a task starts requiring actual chain-runtime, Solidity, libp2p, QP/KKT, x402, TEE, or ZK implementation, record the seam and stop.
- Every completed batch should leave behind:
  - doc changes with explicit status/banner updates,
  - verification command output,
  - explicit deferrals,
  - and any newly clarified boundary between shipping Rust, shipping Solidity demo, shipping scaffold, and frontier.

Required reads for every batch:

- [00-INDEX.md](00-INDEX.md)
- the owning section file(s) named below
- [SOURCE-INDEX.md](SOURCE-INDEX.md)
- [context-pack/agent-runbook.md](context-pack/agent-runbook.md)
- [context-pack/carry-forward-map.md](context-pack/carry-forward-map.md)

---

## Recommended Serial Order

For a single long-running agent run, prefer:

`K1 -> K2 -> K3 -> K5 -> K6 -> K4 -> K7 -> K8`

This order first locks in the real shipped surfaces, then resolves the biggest partial/adjacent surfaces, then does the large frontier pass, then regenerates the canonical status doc, and only after that does the global banner housekeeping.

---

## Batch Overview

| Batch | Tasks | Purpose | Primary Write Scope | Verify Focus | Est. LOC |
|-------|-------|---------|---------------------|--------------|----------|
| K1 | F.01-F.20 DONE anchors | Reconfirm the shipping chain foundation and fix any anchor drift | `tmp/docs-parity/08`, maybe `docs/08-chain/17-18,24` for path drift | `rg` on all F anchors | 80 |
| K2 | A.10, D.01, D.05, D.11 | Make the 7 demo Solidity contracts visible and correctly scoped | `docs/08-chain/06,10,14,24`, parity notes | `rg -n "contracts/src|AgentRegistry.sol|WorkerRegistry.sol|BountyMarket.sol|ConsortiumValidator.sol" docs/08-chain` | 120 |
| K3 | F.12, F.15, F.17 | Build a module-by-module mirage chain scaffold inventory and fix feature/RPC drift | `docs/08-chain/01,18,24`, parity notes | `rg -n "korai_|chain-extensions|default = \\[\"binary\", \"chain\", \"legacy-api\"\\]" docs/08-chain apps/mirage-rs` | 140 |
| K4 | C.*, D.02-D.04, D.07-D.09 | Mark gossip / market / peer-scoring / gaming-resistance chapters as frontier with shipping precursors | `docs/08-chain/07-14`, parity notes | `rg -n "Design — Phase 2\\+|Frontier|InsightBus|PheromoneBus|BountyMarket.sol" docs/08-chain` | 100 |
| K5 | E.01-E.13 | Reconcile `WitnessEngine` / `ChainWitnessEngine` naming and link the shipping observer/attestation surfaces | `docs/08-chain/15,16,19`, `crates/roko-chain/src/witness.rs` doc comments if needed | `rg -n "ChainWitnessEngine|WitnessEngine|roko-chain-watcher" docs/08-chain crates/roko-chain` | 100 |
| K6 | G.01-G.11 | Correct the payments / ISFR / privacy / futures status story, especially Doc 21 | `docs/08-chain/20-23`, parity notes | `rg -n "Implementation: Built|Proxy-only|ISFR_SERVICE_URL|localhost:8546" docs/08-chain/20-23*.md apps/mirage-rs/src/http_api/isfr.rs` | 100 |
| K7 | A.09, A.14 plus K1-K6 fallout | Regenerate Doc 24 as the canonical status summary | `docs/08-chain/24-*`, parity notes | `rg -n "roko-chain|roko-primitives|mirage-rs|roko-chain-watcher|roko_bridge|contracts/src" docs/08-chain/24-*.md` | 140 |
| K8 | global banner/status housekeeping | Final top-level banner pass across overstated or under-scoped PRDs | `docs/08-chain/*.md`, parity notes | `rg -n "^> \\*\\*Implementation\\*\\*:" docs/08-chain/*.md` | 80 |

---

## Dependency Graph

| Batch | Depends on |
|-------|------------|
| K1 | — |
| K2 | — |
| K3 | — |
| K4 | K2 |
| K5 | — |
| K6 | — |
| K7 | K1, K2, K3, K5, K6 |
| K8 | K4, K7 |

Why `K4 -> K2`:

- the frontier pass for market/reputation docs is cleaner once the demo-contract precursor story is already explicit.

Why `K7` depends on `K1/K2/K3/K5/K6`:

- Doc 24 should be regenerated from the settled foundation, demo-contract, scaffold, witness, and payments findings rather than from stale assumptions.

Why `K8` is last:

- the final banner pass should reflect both the frontier pass and the regenerated Doc 24.

Parallel-safe groups:

- `{K1, K2, K3, K5, K6}` can start immediately.
- `K4` should wait for `K2`.
- `K7` waits for `K1, K2, K3, K5, K6`.
- `K8` should be last.

Conflict groups:

| Group | Files | Batches |
|-------|-------|---------|
| status-doc | `docs/08-chain/24-current-status-and-6-contracts.md` | K1, K2, K3, K7, K8 |
| mirage-doc | `docs/08-chain/18-mirage-rs-evm-simulator.md`, `docs/08-chain/01-korai-chain-spec.md` | K1, K3, K8 |
| witness-doc | `docs/08-chain/15-*.md`, `16-*.md`, `19-*.md` | K5, K8 |
| payments-doc | `docs/08-chain/20-23*.md` | K6, K8 |
| parity-08 | `tmp/docs-parity/08/*` | all batches |

---

## Batch Details

### K1 — Shipping Foundation Reconfirmation

**Owns**: F DONE entries

**Read first**:

- [F-built-foundation.md](F-built-foundation.md)
- [SOURCE-INDEX.md](SOURCE-INDEX.md)

**Problem**: the rest of the batch depends on a trustworthy picture of what actually ships today in Rust.

**Scope**:

1. Walk every DONE citation in section F.
2. Fix any anchor or path drift.
3. Keep the section focused on shipped surfaces; do not widen into the partial or frontier items yet.

**Out of scope**:

- re-auditing G or A frontier surfaces,
- new code,
- revisiting `MirageChainClient`.

**Files**:

- `tmp/docs-parity/08/F-built-foundation.md`
- `tmp/docs-parity/08/SOURCE-INDEX.md`

**Verify**:

```bash
grep -oE '[A-Za-z0-9_./-]+\\.(rs|sol|md):[0-9]+(-[0-9]+)?' tmp/docs-parity/08/F-built-foundation.md | sort -u
```

**Acceptance criteria**:

- all DONE anchors in F resolve,
- later batches can treat F as settled shipping ground truth.

---

### K2 — Demo Solidity Visibility Pass

**Owns**: `A.10`, `D.01`, `D.05`, `D.11`

**Read first**:

- [A-vision-and-chain-spec.md](A-vision-and-chain-spec.md)
- [D-job-market-and-reputation.md](D-job-market-and-reputation.md)
- [SOURCE-INDEX.md](SOURCE-INDEX.md) section on `contracts/src/`

**Problem**: the docs still read too close to “zero Solidity exists”, when 7 demo contracts and their tests are already in-tree.

**Scope**:

1. Add a clear “shipping demo contracts” subsection to Doc 24.
2. Cross-link the relevant demo contracts from Docs 06, 10, and 14.
3. Make the “demo vs Korai v1 roadmap” distinction explicit.

**Out of scope**:

- migrating demo contracts,
- adding new Solidity,
- claiming the demos are the full Korai v1 system.

**Files**:

- `docs/08-chain/24-current-status-and-6-contracts.md`
- `docs/08-chain/06-erc-8004-registries.md`
- `docs/08-chain/10-spore-job-market.md`
- `docs/08-chain/14-reputation-system-7-domain.md`
- `tmp/docs-parity/08/*`

**Verify**:

```bash
rg -n "contracts/src|AgentRegistry.sol|WorkerRegistry.sol|BountyMarket.sol|ConsortiumValidator.sol" docs/08-chain
```

**Acceptance criteria**:

- Doc 24 visibly acknowledges the 7 shipping Solidity demos,
- docs 06 / 10 / 14 all point at the right precursors,
- later agents do not need to rediscover that Solidity already exists.

---

### K3 — Mirage Scaffold Inventory

**Owns**: `F.12`, `F.15`, `F.17`

**Read first**:

- [F-built-foundation.md](F-built-foundation.md)
- [SOURCE-INDEX.md](SOURCE-INDEX.md) sections on `apps/mirage-rs/`

**Problem**: mirage’s `chain/` surface is real and substantial, but the docs currently describe it too narrowly and with some naming drift.

**Scope**:

1. Inventory each `apps/mirage-rs/src/chain/*` module and its role.
2. Clarify that the shipping scaffold is an agent-coordination substrate, not the literal Korai registry set.
3. Fix the `korai_*` RPC expectation drift and the `chain-extensions` vs `chain` feature-name drift.

**Out of scope**:

- implementing registry emulation,
- implementing `MirageChainClient`,
- adding new mirage features.

**Files**:

- `docs/08-chain/18-mirage-rs-evm-simulator.md`
- `docs/08-chain/01-korai-chain-spec.md`
- `docs/08-chain/24-current-status-and-6-contracts.md`
- `tmp/docs-parity/08/*`

**Verify**:

```bash
grep -c "korai_" apps/mirage-rs/src/chain_rpc.rs
rg -n "chain-extensions|korai_" docs/08-chain/01-*.md docs/08-chain/18-*.md
```

**Acceptance criteria**:

- Doc 18 carries a concrete scaffold inventory,
- RPC and feature-name drift are explicitly corrected,
- later agents can tell what the mirage scaffold really is.

---

### K4 — Gossip, Market, And Reputation Frontier Pass

**Owns**: section C plus frontier items in D

**Read first**:

- [C-gossip-and-p2p-network.md](C-gossip-and-p2p-network.md)
- [D-job-market-and-reputation.md](D-job-market-and-reputation.md)

**Problem**: the market/gossip/reputation docs still risk being read as nearer to runtime than they really are.

**Scope**:

1. Add or strengthen `Design — Phase 2+` banners on Docs 07-14 where appropriate.
2. Cross-link the nearest shipping precursors: local pub/sub and the demo contracts.
3. Preserve partials where partials really exist.

**Out of scope**:

- libp2p,
- auction code,
- VRF work,
- C-factor implementation.

**Files**:

- `docs/08-chain/07-*.md` through `14-*.md`
- `tmp/docs-parity/08/*`

**Verify**:

```bash
rg -n "Design — Phase 2\\+|InsightBus|PheromoneBus|BountyMarket.sol|WorkerRegistry.sol" docs/08-chain
```

**Acceptance criteria**:

- later agents can tell which gossip/market docs are frontier,
- the nearest shipping precursors are discoverable from those docs,
- partial demo-contract behavior is not mistaken for the full design.

---

### K5 — Witness Surface Reconciliation

**Owns**: `E.01-E.13`

**Read first**:

- [E-witness-triage-heartbeat.md](E-witness-triage-heartbeat.md)
- [F-built-foundation.md](F-built-foundation.md)

**Problem**: `WitnessEngine` in the PRDs and `ChainWitnessEngine` in Rust describe different things, but the naming overlap is not obvious unless you already know the codebase.

**Scope**:

1. Add explicit disambiguation to Docs 15/16/19.
2. Cross-link the shipping observer app and attestation witness engine.
3. Add a small doc comment note in `crates/roko-chain/src/witness.rs` only if needed.

**Out of scope**:

- renaming public Rust types unless that is separately requested,
- implementing Binary Fuse witness logic,
- implementing MIDAS-R.

**Files**:

- `docs/08-chain/15-*.md`
- `docs/08-chain/16-*.md`
- `docs/08-chain/19-*.md`
- `crates/roko-chain/src/witness.rs` only for comments if needed
- `tmp/docs-parity/08/*`

**Verify**:

```bash
rg -n "ChainWitnessEngine|WitnessEngine|roko-chain-watcher" docs/08-chain crates/roko-chain
```

**Acceptance criteria**:

- later readers can distinguish the two witness surfaces immediately,
- the shipping observer is discoverable from Doc 15,
- the attestation anchor is discoverable from the same docs without being mistaken for the watcher architecture.

---

### K6 — Payments, ISFR, Privacy, Futures Status Pass

**Owns**: section G

**Read first**:

- [G-payments-settlement-privacy.md](G-payments-settlement-privacy.md)
- [SOURCE-INDEX.md](SOURCE-INDEX.md) section on `http_api/isfr.rs`

**Problem**: Doc 21 still has the most misleading “Built” story in topic `08`, and the rest of section G needs consistent frontier framing.

**Scope**:

1. Make Doc 21 explicitly proxy-only.
2. Make upstream-service dependency obvious.
3. Add or strengthen frontier banners for Docs 20, 22, and 23.

**Out of scope**:

- implementing the solver,
- adding x402 code,
- adding privacy/futures mechanisms.

**Files**:

- `docs/08-chain/20-*.md`
- `docs/08-chain/21-*.md`
- `docs/08-chain/22-*.md`
- `docs/08-chain/23-*.md`
- `tmp/docs-parity/08/*`

**Verify**:

```bash
rg -n "Implementation: Built|Proxy-only|ISFR_SERVICE_URL|localhost:8546" docs/08-chain/20-23*.md apps/mirage-rs/src/http_api/isfr.rs
```

**Acceptance criteria**:

- Doc 21 no longer reads like the solver is in this repo,
- the settlement/privacy/futures docs are consistently frontier-tagged,
- the shipping proxy surface is easy to find from the docs.

---

### K7 — Doc 24 Regeneration Pass

**Owns**: `A.09`, `A.14`, and the summary fallout from K1-K6

**Read first**:

- [A-vision-and-chain-spec.md](A-vision-and-chain-spec.md)
- [F-built-foundation.md](F-built-foundation.md)
- [SOURCE-INDEX.md](SOURCE-INDEX.md)
- results of K1-K6

**Problem**: Doc 24 is the canonical status summary and currently mixes under-claims, missing surfaces, and stale paths.

**Scope**:

1. Rebuild “What Is Built” from the verified shipped surfaces.
2. Rebuild “What Is Scaffolded” from the mirage inventory.
3. Rebuild “What Is Not Yet Built” to remove already-shipping items.
4. Fix stale paths and add the demo-contract subsection.

**Out of scope**:

- changing the six-contract roadmap itself,
- inventing new status categories beyond what helps clarity.

**Files**:

- `docs/08-chain/24-current-status-and-6-contracts.md`
- `tmp/docs-parity/08/*`

**Verify**:

```bash
rg -n "roko-chain|roko-primitives|mirage-rs|roko-chain-watcher|roko_bridge|contracts/src" docs/08-chain/24-*.md
rg -n "bardo-primitives" docs/08-chain/24-*.md
```

**Acceptance criteria**:

- Doc 24 works as the canonical status summary for topic `08`,
- shipping, scaffolded, partial, and deferred surfaces are no longer mixed up,
- path drift is gone.

---

### K8 — Global Banner And Housekeeping Pass

**Owns**: top-level banner/status cleanup across topic `08`

**Read first**:

- results of K1-K7

**Problem**: even after the section-specific fixes, a final banner sweep is needed so skim-readers are not misled by front-matter.

**Scope**:

1. Audit every `Implementation:` banner in `docs/08-chain/*.md`.
2. Downgrade or qualify any banner that does not match the settled parity findings.
3. Keep Doc 17 / 18 / 24 as the honest “built/partial” centers of gravity.

**Out of scope**:

- editing non-chain topics,
- changing runtime code,
- rewriting every chapter in full.

**Files**:

- `docs/08-chain/*.md`
- `tmp/docs-parity/08/*`

**Verify**:

```bash
rg -n "^> \\*\\*Implementation\\*\\*:" docs/08-chain/*.md
```

**Acceptance criteria**:

- later agents can infer shipping vs partial vs frontier from the first screen of each PRD,
- banner language no longer contradicts the body or the parity files,
- topic `08` is safe to hand off without additional chain context.
