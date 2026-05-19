# 03 — Adapter Catalog

Each adapter implements `ConsensusVerifier` for one consensus mechanism. State proof
verification (MPT via `eth_getProof`) is shared across all EVM chains.

---

## Adapter 1: Threshold BLS (Tempo, daeji)

Detailed in doc 02. Key properties:

- **Consensus proof**: Single BLS12-381 threshold signature (~240 bytes)
- **Verification cost**: ~2ms (one pairing check)
- **Key management**: Static group public key, no rotation tracking
- **Dependencies**: `commonware-cryptography` (BLS12-381)
- **State proofs**: `eth_getProof` (standard EVM MPT)

Used by: Tempo mainnet (4217), Tempo testnet (42431), daeji devnet (31337),
and any future Commonware Threshold Simplex chain.

Daeji is special because it's our chain — we also write to it (witness anchoring,
InsightBoard posts). The `ChainWitnessEngine` and `ChainWallet` handle writes.
The `ConsensusVerifier` handles verified reads.

---

## Adapter 2: Sync Committee (Ethereum, OP Stack, Base, Linea)

Ethereum's beacon chain sync committee protocol. 512 validators sign block headers
every slot. Aggregate BLS signature + participation bitmap.

```rust
pub struct SyncCommitteeVerifier {
    /// Beacon API endpoint for sync committee updates.
    beacon_api: String,
    /// Current sync committee (rotates every ~27 hours).
    current_committee: RwLock<SyncCommittee>,
    /// Next sync committee (pre-fetched for seamless rotation).
    next_committee: RwLock<Option<SyncCommittee>>,
    /// Latest finalized beacon header.
    finalized_header: RwLock<Option<BeaconHeader>>,
    /// Checkpoint for initial sync.
    checkpoint: Option<[u8; 32]>,
}

#[async_trait]
impl ConsensusVerifier for SyncCommitteeVerifier {
    async fn verify_finality(&self, block: u64) -> Result<TrustedHeader, ConsensusError> {
        // 1. Get finalized beacon block via beacon API
        let beacon_block = self.fetch_beacon_block(block).await?;

        // 2. Verify sync committee aggregate signature
        let committee = self.current_committee.read().await;
        let sync_agg = &beacon_block.sync_aggregate;

        // Must have >2/3 participation
        let participating = sync_agg.participation_bits.count_ones();
        if participating * 3 <= committee.pubkeys.len() * 2 {
            return Err(ConsensusError::InsufficientParticipation);
        }

        // Aggregate participating public keys and verify
        let agg_pk = aggregate_participating_keys(&committee.pubkeys, &sync_agg.participation_bits);
        bls::verify(&agg_pk, &beacon_block.header_hash, &sync_agg.signature)?;

        // 3. Extract execution payload state root
        let execution_state_root = beacon_block.execution_payload.state_root;

        Ok(TrustedHeader {
            number: beacon_block.execution_payload.block_number,
            hash: beacon_block.execution_payload.block_hash,
            state_root: execution_state_root,
            timestamp: beacon_block.execution_payload.timestamp,
            consensus_proof: ConsensusProof::SyncCommittee {
                aggregate_signature: sync_agg.signature.clone(),
                participation_bits: sync_agg.participation_bits.clone(),
                committee_period: beacon_block.slot / 8192,
            },
        })
    }

    fn mechanism(&self) -> &str { "sync_committee" }
    fn trust_level(&self) -> TrustLevel { TrustLevel::Cryptographic }
}
```

**Complexity vs Tempo**: Ethereum requires tracking committee rotations (~27h),
handling committee period transitions, and verifying larger proofs. Tempo's static
key makes it significantly simpler.

**Helios reference**: The a16z Helios client (Rust, WASM, v0.11.1) implements exactly
this. We can use Helios as a library or reference its verification logic. Helios
syncs in ~2 seconds, uses 20 bytes/sec bandwidth.

**Extends to OP Stack chains**: Base, OP Mainnet, Linea derive their finality from
Ethereum L1. Once we verify Ethereum consensus, L2 state proofs are derivable.

---

## Adapter 3: RPC-Only (fallback for any chain)

No consensus verification. Trusts the RPC provider. This is the baseline — what
`AlloyChainClient` already does today, just wrapped in the `ConsensusVerifier`
interface for uniformity.

```rust
pub struct RpcOnlyVerifier {
    rpc: Arc<dyn ChainClient>,
}

#[async_trait]
impl ConsensusVerifier for RpcOnlyVerifier {
    async fn verify_finality(&self, block: u64) -> Result<TrustedHeader, ConsensusError> {
        let header = self.rpc.get_block_header(block).await?;
        Ok(TrustedHeader {
            number: header.number,
            hash: parse_hash(&header.hash),
            state_root: [0u8; 32], // not available from basic RPC
            timestamp: header.timestamp,
            consensus_proof: ConsensusProof::RpcTrusted,
        })
    }
    fn mechanism(&self) -> &str { "rpc" }
    fn trust_level(&self) -> TrustLevel { TrustLevel::RpcTrusted }
}
```

Used for: chains with no LC protocol (Solana), local devnets, any chain where
trust-minimization isn't needed.

---

## Adapter 4: Playback (deterministic replay)

Reads captured state from a JSONL file. Verification runs against captured proofs.

```rust
pub struct PlaybackVerifier {
    entries: Vec<PlaybackEntry>,
    cursor: AtomicUsize,
    speed: f64,
}

#[async_trait]
impl ConsensusVerifier for PlaybackVerifier {
    async fn verify_finality(&self, block: u64) -> Result<TrustedHeader, ConsensusError> {
        // Find the entry for this block in the captured data
        let entry = self.entries.iter()
            .find(|e| e.block_number == block)
            .ok_or(ConsensusError::BlockUnavailable(block))?;

        // Return the captured header — consensus proof is Playback
        Ok(TrustedHeader {
            consensus_proof: ConsensusProof::Playback {
                source_file: self.source.clone()
            },
            ..entry.header.clone()
        })
    }
    fn mechanism(&self) -> &str { "playback" }
    fn trust_level(&self) -> TrustLevel { TrustLevel::Playback }
}
```

---

## Future adapter: Cosmos / IBC (Tendermint)

Non-EVM, so needs its own state proof verifier (IAVL+, not MPT). Lower priority.
IBC Eureka (2025) already bridges Cosmos↔Ethereum via ZK proofs — we could consume
those proofs rather than implementing Tendermint verification directly.

---

## Future adapter: ZK-verified (SP1 / RISC Zero)

The frontier. Instead of verifying consensus signatures directly, verify a ZK proof
that consensus was followed. SP1 Hypercube can prove Ethereum blocks in <12 seconds.

This would be a separate `ConsensusVerifier` impl:

```rust
pub struct ZkVerifier {
    verifier_contract: Address,  // on-chain SP1 verifier
    proof_service: String,       // Succinct Prover Network URL
}
```

Not needed for v1 — direct BLS/sync-committee verification is fast enough. But the
architecture supports it as a future adapter without changes to the trait.

---

## Comparison matrix

| Chain | Adapter | Trust | Cert size | Verify time | State proofs | Deps |
|-------|---------|-------|-----------|-------------|-------------|------|
| Tempo | ThresholdBls | Cryptographic | ~240 B | ~2ms | eth_getProof (MPT) | commonware-crypto |
| daeji | ThresholdBls | Cryptographic | ~240 B | ~2ms | eth_getProof (MPT) | commonware-crypto |
| Ethereum | SyncCommittee | Cryptographic | ~100 KB | ~50ms | eth_getProof (MPT) | ethereum-consensus |
| Base/OP | SyncCommittee | Cryptographic | ~100 KB | ~50ms | eth_getProof (MPT) | ethereum-consensus |
| Cosmos | (future) | Cryptographic | ~10-50 KB | ~20ms | IAVL+ | tendermint-lc |
| Any | RpcOnly | RPC trusted | N/A | 0ms | N/A | none |
| Any | Playback | Playback | N/A | 0ms | captured | none |

The agent sees the same `VerifiedState<T>` from all of them.
