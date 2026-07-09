# 02 — Tempo Integration

Tempo is the first chain to integrate. Two layers: consensus verification (Threshold
Simplex BLS certs) and machine payments (MPP). Together they give agents trustless
payment for services.

---

## Tempo facts (as of 2026-05-04)

| | |
|---|---|
| **Mainnet** | Live since 2026-03-18 |
| **Chain ID** | 4217 (mainnet), 42431 (Moderato testnet) |
| **RPC** | `https://rpc.tempo.xyz` (mainnet), `https://rpc.moderato.tempo.xyz` (testnet) |
| **Consensus** | Threshold Simplex (Commonware) — BLS12-381 threshold signatures |
| **Execution** | Reth SDK (EVM, Osaka hardfork target) |
| **State model** | Standard EVM — Merkle Patricia Trie, `eth_getProof` works |
| **Block time** | ~0.5s deterministic finality |
| **Finality cert** | ~240 bytes: 96-byte BLS G2 signature + 48-byte G1 group pubkey + metadata |
| **Native gas** | None. Fees paid in TIP-20 stablecoins via Fee AMM |
| **Payment standard** | MPP (Machine Payments Protocol), co-authored with Stripe |
| **Token standard** | TIP-20 (ERC-20 + memo, RBAC, compliance hooks, reward distribution) |
| **Key feature** | Payment lanes: dedicated block space for TIP-20 transfers, cannot be crowded by DeFi |
| **Validators** | Permissioned at launch; DKG ceremony produces static group key |

---

## Consensus verification: Threshold BLS

Tempo's consensus produces a single BLS12-381 threshold signature per finalized block.
The group public key is **static across validator set changes** (resharing preserves it).
A verifier only needs to store one 48-byte public key.

### Verification

```rust
pub struct ThresholdBlsVerifier {
    /// The static BLS12-381 group public key (G1 point, 48 bytes).
    /// Obtained once from chain genesis or a trusted source.
    /// Does NOT change when validators rotate (resharing preserves it).
    group_pubkey: bls::PublicKey,
    /// Chain ID for this network.
    chain_id: u64,
    /// RPC client for fetching block headers.
    rpc: Arc<AlloyChainClient>,
    /// Cache of verified headers.
    verified_headers: RwLock<BTreeMap<u64, TrustedHeader>>,
}

impl ThresholdBlsVerifier {
    /// Verify a Tempo consensus certificate.
    /// Single BLS pairing check: ~1-2ms regardless of validator count.
    fn verify_cert(&self, header_hash: &[u8; 32], sig: &[u8]) -> Result<(), ConsensusError> {
        // e(signature, G2_generator) == e(H(header_hash), group_pubkey)
        bls::verify(&self.group_pubkey, header_hash, sig)
            .map_err(|_| ConsensusError::InvalidSignature)
    }
}

#[async_trait]
impl ConsensusVerifier for ThresholdBlsVerifier {
    async fn verify_finality(&self, block: u64) -> Result<TrustedHeader, ConsensusError> {
        // 1. Fetch block header + finality cert from Tempo RPC
        //    (Tempo may expose this via a custom RPC method or standard eth_ methods)
        let header = self.rpc.get_block_header(block).await?;

        // 2. Fetch the consensus certificate for this block
        //    (Custom Tempo RPC: tempo_getConsensusProof or similar)
        let cert = self.fetch_consensus_cert(block).await?;

        // 3. Verify BLS threshold signature
        self.verify_cert(&header.hash, &cert.signature)?;

        // 4. Cache and return
        let trusted = TrustedHeader {
            number: header.number,
            hash: parse_hash(&header.hash),
            state_root: parse_hash(&header.state_root),
            timestamp: header.timestamp,
            consensus_proof: ConsensusProof::ThresholdBls {
                signature: cert.signature,
                group_pubkey: self.group_pubkey.to_bytes().to_vec(),
            },
        };
        self.verified_headers.write().await.insert(block, trusted.clone());
        Ok(trusted)
    }

    fn mechanism(&self) -> &str { "threshold_bls" }
    fn trust_level(&self) -> TrustLevel { TrustLevel::Cryptographic }
}
```

### Comparison to Ethereum

| | Tempo | Ethereum |
|---|---|---|
| Cert size | ~240 bytes | ~100 KB (sync committee update) |
| Verification | 1 BLS pairing (~2ms) | 512-key aggregation + pairing (~50ms) |
| Key tracking | Static group key, never changes | Must track sync committee rotations every ~27h |
| State proofs | `eth_getProof` MPT (same) | `eth_getProof` MPT (same) |

Because both use EVM state, the state proof verifier is identical. Only the consensus
layer differs.

---

## Machine Payments Protocol (MPP)

MPP is how agents actually pay on Tempo. It's an HTTP-based standard (Stripe + Tempo,
targeting IETF spec at paymentauth.org). SDKs: `mpp-rs` (Rust), `mppx` (TypeScript),
`pympp` (Python).

### Three payment modes

| Mode | Flow | Use case |
|------|------|----------|
| **One-time** | Agent sends HTTP request with MPP payment header → Tempo TIP-20 transfer settles in ~500ms → service responds | Single API call, data purchase |
| **Session (pay-as-you-go)** | Agent deposits funds, signs off-chain vouchers, pays per-request via payment channel | Multi-turn agent tasks, ongoing API access |
| **Streaming** | Per-token billing over SSE, word-by-word content delivery with continuous payment | LLM inference, real-time data feeds |

### MPP + Light-Client verification

This is the novel workflow. An agent pays for a service via MPP, then the light-client
layer verifies the payment settled correctly:

```
Agent                          Service                    Tempo chain
  │                               │                          │
  ├── MPP payment request ──────►│                          │
  │   (HTTP + payment auth)       │                          │
  │                               ├── TIP-20 transfer ────►│
  │                               │                     block N finalized
  │   ◄── service response ──────┤                          │
  │   (data / inference / etc)    │                          │
  │                                                          │
  ├── verify_settlement(tx_hash, block=N) ─────────────────►│
  │     ConsensusVerifier.verify_finality(N) ──► BLS check  │
  │     eth_getProof(token_contract, slot) ──► MPT verify   │
  │                                                          │
  ◄── VerifiedState<TransferReceipt> ──────────────────────┘
       trust_level: Cryptographic
       amount: 1.50 USDC
       from: agent_address
       to: service_address
```

Why this matters: the agent can **prove** it paid, and **prove** what it got. The
payment receipt is cryptographically verified, not self-reported. This feeds into:
- BountyMarket settlement (cross-chain payment proof)
- InsightStore entries (verified data provenance: "I paid $X for this data")
- Reputation updates (reliable payer/provider)
- Episode commitments (deterministic proof of spend)

### `MppClient` integration

```rust
/// Tempo MPP client for agent payments.
/// Wraps mpp-rs with light-client settlement verification.
pub struct MppClient {
    /// The mpp-rs client (handles HTTP payment protocol).
    inner: mpp::Client,
    /// Light-client verifier for settlement confirmation.
    verifier: Arc<VerifiedChainClient>,
    /// Agent's wallet for signing payments.
    wallet: Arc<dyn ChainWallet>,
}

impl MppClient {
    /// Pay for a one-time service call and verify settlement.
    pub async fn pay_and_verify(
        &self,
        service_url: &str,
        amount: u128,
        token: &str,  // TIP-20 contract address
    ) -> Result<VerifiedPayment, MppError> {
        // 1. Execute MPP payment (HTTP + on-chain settlement)
        let receipt = self.inner.pay(service_url, amount, token).await?;

        // 2. Wait for finalization (~500ms on Tempo)
        tokio::time::sleep(Duration::from_millis(600)).await;

        // 3. Verify settlement via light client
        let verified = self.verifier.verify_transfer(
            &receipt.tx_hash,
            &receipt.to,
            amount,
        ).await?;

        Ok(VerifiedPayment {
            service_response: receipt.response,
            settlement: verified,
        })
    }

    /// Open a pay-as-you-go session.
    pub async fn open_session(
        &self,
        service_url: &str,
        budget: u128,
        token: &str,
    ) -> Result<MppSession, MppError> {
        // Session deposit verified via LC
        // Subsequent per-request vouchers are off-chain (payment channel)
        // Final settlement verified via LC on session close
        todo!()
    }
}
```

### `MppTool` — agent tool for MPP payments

```rust
// Tool definition (extends existing chain.transfer tooldef)
ToolDef {
    name: "chain.mpp_pay",
    description: "Pay for a service via Tempo MPP. Returns the service response + verified payment receipt.",
    params: json!({
        "service_url": "string — the MPP-enabled service endpoint",
        "amount": "string — payment amount in token units",
        "token": "string — TIP-20 token address (default: USDC)",
        "mode": "string — one_time | session | streaming (default: one_time)"
    }),
}
```

This is a new tool def added alongside the existing 17. The handler uses `MppClient`.

---

## TIP-20 specifics

TIP-20 extends ERC-20 with `transferWithMemo`. This means Tempo transfers can carry
structured metadata (invoice IDs, job references, agent IDs). The light-client layer
can verify both the transfer AND the memo:

```rust
/// Verify a TIP-20 transfer with memo.
async fn verify_transfer_with_memo(
    verifier: &VerifiedChainClient,
    tx_hash: &str,
) -> Result<VerifiedState<Tip20Transfer>, ChainError> {
    // 1. Get verified receipt
    let receipt = verifier.get_receipt(tx_hash).await?;
    // 2. Decode Transfer event + memo from logs
    let transfer = decode_tip20_transfer(&receipt.logs)?;
    // 3. Return verified transfer with memo
    Ok(VerifiedState {
        data: transfer,  // { from, to, amount, memo }
        // ... verification metadata
    })
}
```

The memo field enables structured payment attribution without a separate indexing layer.
An agent paying for a bounty can include `memo: "bounty:42,agent:0xABC"` and the
entire payment is verifiable end-to-end.

---

## Offline playback

For demos where live Tempo connectivity is unreliable. Capture format:

```jsonl
{"t":"header","n":100,"h":"0xabc","sr":"0xdef","ts":1717200000,"cert":"base64(bls_sig)"}
{"t":"proof","n":100,"method":"eth_getProof","addr":"0xABC","result":"base64(mpt_proof)"}
{"t":"receipt","n":100,"tx":"0x123","logs":[...],"status":true}
```

The playback adapter reads these sequentially. Verification still runs against the
captured data — the demo shows real BLS checks and MPT traversals, just against
recorded state.

```bash
# Capture
roko chain capture --network tempo-mainnet --address 0xABC --duration 60s \
  --output .roko/chain-captures/tempo-demo.jsonl

# Replay
roko chain tail --playback .roko/chain-captures/tempo-demo.jsonl --speed 1.0
```

---

## Open questions for Tempo team

1. **Consensus cert RPC**: Does Tempo expose finality certificates via a custom RPC
   method (e.g., `tempo_getConsensusCertificate(blockNumber)`)? Or must we extract
   them from the P2P layer?

2. **Group public key distribution**: How do new verifiers bootstrap the BLS group
   public key? Genesis block? On-chain registry? Hardcoded in docs?

3. **Payment lanes**: Can we subscribe to payment-lane-only events for agent payment
   monitoring without processing general DeFi activity?

4. **MPP Rust SDK maturity**: Is `mpp-rs` production-ready, or should we wrap the
   HTTP protocol directly?
