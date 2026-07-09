# WU-17: MPP Client & Provider

**Layer**: 4
**Depends on**: WU-7 (VerifiedChainClient), WU-10 (Config)
**Blocks**: WU-18, WU-19, WU-20
**Estimated effort**: 3-4 hours
**Crate**: `crates/roko-chain`
**Feature gate**: `mpp`

---

## Overview

`MppClient` wraps the official `mpp-rs` Rust SDK to integrate MPP (Machine Payments Protocol) payments with light-client settlement verification. When an agent pays for a service, MppClient:

1. Executes the MPP HTTP payment flow (via mpp-rs `TempoProvider`)
2. Waits for on-chain settlement
3. Verifies the settlement transaction via the light client (`VerifiedChainClient`)
4. Returns a `VerifiedPayment` with full cryptographic provenance

The result is an end-to-end verifiable payment: the agent can prove it paid, what it paid for, and that the settlement finalized on Tempo -- all backed by the same trust level as the rest of the verified chain layer.

---

## Pre-read

- `crates/roko-chain/src/verified_client.rs` -- `VerifiedChainClient`, `verify_transfer()` method
- `crates/roko-chain/src/verified_state.rs` -- `VerifiedState<T>` wrapper with trust metadata
- `crates/roko-chain/src/consensus.rs` -- `TrustLevel` enum
- `crates/roko-chain/src/types.rs` -- `ChainError`, `Receipt`
- `crates/roko-core/src/config/chain.rs` -- `ChainConfig` struct (where `MppConfig` will live)
- `12-WU7-verified-client.md` -- VerifiedChainClient design and verify_transfer signature

---

## Tasks

### 17.1 Add mpp dependency to Cargo.toml

**File**: `crates/roko-chain/Cargo.toml`

Add to `[dependencies]`:

```toml
[dependencies]
mpp = { version = "0.10", optional = true, features = ["client", "tempo"] }
```

Add to `[features]`:

```toml
[features]
mpp = ["dep:mpp", "alloy-backend"]
```

The `mpp` feature implies `alloy-backend` because settlement verification requires a live `VerifiedChainClient` which depends on alloy for RPC access.

> **Note (alloy version mismatch)**: mpp 0.10.0 uses alloy 2.0 internally. The roko workspace uses alloy 1.x. This may cause duplicate type errors at the boundary. Mitigation options:
> - Pin mpp to a version that uses alloy 1.x (if one exists)
> - Feature-gate the boundary so mpp types never leak into roko-chain's public API
> - Use string/bytes conversions at the mpp<->roko boundary instead of sharing alloy types
>
> See Open Questions below. This task may require adjusting the version constraint once the actual dependency tree is resolved.

### 17.2 Create `crates/roko-chain/src/mpp_client.rs`

**File**: `crates/roko-chain/src/mpp_client.rs`

This is the core file. The entire module is feature-gated behind `#[cfg(feature = "mpp")]`.

```rust
//! MPP (Machine Payments Protocol) client with light-client settlement verification.
//!
//! Wraps the official `mpp-rs` SDK to add cryptographic verification of payment
//! settlement via the verified chain layer.
//!
//! # Feature gate
//! Requires `mpp` feature: `cargo build -p roko-chain --features mpp`

use std::sync::Arc;
use serde::{Deserialize, Serialize};
use crate::consensus::TrustLevel;
use crate::verified_state::VerifiedState;
use crate::types::ChainError;

// ── Types ────────────────────────────────────────────────────────────

/// Verified payment receipt -- proof that a payment settled on-chain
/// with cryptographic verification of the settlement transaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifiedPayment {
    /// The service URL that was paid.
    pub service_url: String,
    /// Amount paid in base units (e.g., 1000000 = 1 USDC).
    pub amount: String,
    /// Token contract address (TIP-20 on Tempo).
    pub token: String,
    /// The on-chain settlement, wrapped in verification metadata.
    pub settlement: VerifiedState<PaymentSettlement>,
    /// The HTTP response body from the paid service.
    pub service_response: serde_json::Value,
}

/// On-chain settlement details extracted from the transaction receipt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentSettlement {
    /// Transaction hash of the settlement.
    pub tx_hash: String,
    /// Sender address.
    pub from: String,
    /// Recipient address.
    pub to: String,
    /// Amount transferred (in base units).
    pub amount: String,
    /// TIP-20 memo field (e.g., "service:api.example.com,request:abc123").
    pub memo: Option<String>,
}

/// MPP payment mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentMode {
    /// Single payment per request. No persistent state.
    OneTime,
    /// Session-based payment channel. Opens once, sends vouchers per request.
    Session,
}

/// Configuration for the MPP client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MppConfig {
    /// Tempo RPC URL for payment transactions.
    pub rpc_url: String,
    /// Private key environment variable name (NOT the key itself).
    pub wallet_key_env: String,
    /// Default payment mode.
    pub default_mode: PaymentMode,
    /// Maximum amount per single payment (in base units). Safety limit.
    pub max_per_payment: Option<u64>,
    /// Maximum total spend per session (in base units). Safety limit.
    pub max_per_session: Option<u64>,
}

// ── Client ───────────────────────────────────────────────────────────

/// The MPP client. Wraps mpp-rs with VerifiedChainClient for settlement verification.
pub struct MppClient {
    /// mpp-rs TempoProvider (handles the HTTP payment protocol).
    provider: mpp::client::TempoProvider,
    /// Optional session provider for channel-based payments.
    session_provider: Option<mpp::client::TempoSessionProvider>,
    /// HTTP client for making requests.
    http_client: reqwest::Client,
    /// Verified chain client for settlement verification.
    verifier: Arc<crate::verified_client::VerifiedChainClient>,
    /// Configuration.
    config: MppConfig,
}

impl MppClient {
    /// Create a new MPP client.
    ///
    /// Reads the wallet private key from the environment variable named in
    /// `config.wallet_key_env`. The key itself is never stored in config files.
    ///
    /// # Arguments
    /// - `config`: MPP configuration (RPC URL, payment limits, mode)
    /// - `verifier`: VerifiedChainClient for settlement verification
    ///
    /// # Errors
    /// Returns error if the wallet key env var is not set or contains invalid hex.
    pub fn new(
        config: MppConfig,
        verifier: Arc<crate::verified_client::VerifiedChainClient>,
    ) -> Result<Self, ChainError> {
        let wallet_key = std::env::var(&config.wallet_key_env)
            .map_err(|_| ChainError::Other(format!(
                "MPP wallet key env var '{}' not set", config.wallet_key_env
            )))?;

        let key_bytes = hex::decode(wallet_key.trim_start_matches("0x"))
            .map_err(|e| ChainError::Other(format!("Invalid wallet key hex: {e}")))?;

        let key_array: [u8; 32] = key_bytes
            .try_into()
            .map_err(|_| ChainError::Other("Wallet key must be 32 bytes".into()))?;

        let signer = mpp::PrivateKeySigner::from_bytes(&key_array)
            .map_err(|e| ChainError::Other(format!("Invalid signer: {e}")))?;

        let provider = mpp::client::TempoProvider::new(signer.clone(), &config.rpc_url)
            .map_err(|e| ChainError::Other(format!("MPP provider error: {e}")))?;

        // Session provider is optional — construction failure is non-fatal.
        let session_provider = mpp::client::TempoSessionProvider::new(signer, &config.rpc_url)
            .ok();

        Ok(Self {
            provider,
            session_provider,
            http_client: reqwest::Client::new(),
            verifier,
            config,
        })
    }

    /// Access the underlying configuration.
    pub fn config(&self) -> &MppConfig {
        &self.config
    }

    /// Whether session-based payments are available.
    pub fn supports_sessions(&self) -> bool {
        self.session_provider.is_some()
    }

    /// Execute a one-time MPP payment and verify settlement.
    ///
    /// Flow:
    /// 1. Send HTTP request to `service_url`
    /// 2. If the service returns 402 Payment Required, parse the payment
    ///    challenge and pay via `TempoProvider`
    /// 3. Wait for on-chain settlement confirmation
    /// 4. Verify the settlement tx via VerifiedChainClient
    /// 5. Return `VerifiedPayment` with service response + settlement proof
    ///
    /// # Arguments
    /// - `service_url`: The MPP-enabled service endpoint
    /// - `method`: HTTP method (GET, POST, etc.)
    /// - `body`: Optional JSON request body
    ///
    /// # Errors
    /// Returns error if payment fails, settlement is not found, or
    /// verification fails.
    pub async fn pay_one_time(
        &self,
        service_url: &str,
        method: reqwest::Method,
        body: Option<serde_json::Value>,
    ) -> Result<VerifiedPayment, ChainError> {
        // Build the request
        let mut request = self.http_client.request(method, service_url);
        if let Some(body) = &body {
            request = request.json(body);
        }

        // Send with MPP payment (handles 402 challenge-response automatically).
        // The Fetch trait from mpp-rs intercepts 402 responses, parses the
        // payment challenge, executes the on-chain payment, and retries the
        // request with a payment receipt header.
        use mpp::client::Fetch; // PaymentExt trait
        let response = request
            .send_with_payment(&self.provider)
            .await
            .map_err(|e| ChainError::Other(format!("MPP payment failed: {e}")))?;

        let status = response.status();
        let service_response: serde_json::Value = response.json().await
            .map_err(|e| ChainError::Other(format!("Failed to read service response: {e}")))?;

        if !status.is_success() {
            return Err(ChainError::Other(format!(
                "Service returned {status} after payment"
            )));
        }

        // TODO(17.2a): Extract tx_hash from the Payment-Receipt header.
        //
        // The mpp-rs `Fetch` trait returns a raw `reqwest::Response` after
        // payment. The settlement tx_hash should be available in the
        // `Payment-Receipt` header (or equivalent). Steps:
        //
        // 1. Read the `Payment-Receipt` header from the response
        // 2. Parse it to extract the tx_hash (format TBD from mpp-rs source)
        // 3. Call `self.verifier.verify_transfer(&tx_hash)` to get
        //    `VerifiedState<Receipt>` with consensus proof
        // 4. Extract `PaymentSettlement` fields from the verified receipt
        // 5. Construct and return `VerifiedPayment`
        //
        // Blocked on: confirming the receipt header name and format from
        // the mpp-rs source code. See Open Questions #2.

        // TODO(17.2b): Enforce safety limits.
        //
        // Before executing payment, check:
        // - `config.max_per_payment` against the challenge amount
        // - `config.max_per_session` against cumulative session spend
        // Return `ChainError::Other("payment exceeds limit")` if violated.

        // TODO(17.2c): Handle alloy version mismatch.
        //
        // If mpp-rs returns alloy 2.0 types (e.g., `alloy::primitives::TxHash`),
        // convert to strings at the boundary before passing to
        // VerifiedChainClient which uses alloy 1.x types.

        todo!("Extract tx_hash from Payment-Receipt header and verify via light client")
    }
}
```

### 17.3 Add MppConfig to chain config

**File**: `crates/roko-core/src/config/chain.rs`

Add the MPP configuration struct below the existing config types:

```rust
/// MPP (Machine Payments Protocol) configuration.
///
/// ```toml
/// [chain.mpp]
/// wallet_key_env = "ROKO_MPP_WALLET_KEY"
/// default_mode = "one_time"
/// max_per_payment = 10000000
/// max_per_session = 50000000
/// ```
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct MppConfig {
    /// Environment variable name containing the wallet private key.
    /// The key itself is never stored in config -- only the env var name.
    #[serde(default)]
    pub wallet_key_env: Option<String>,

    /// Default payment mode: "one_time" or "session".
    #[serde(default)]
    pub default_mode: Option<String>,

    /// Maximum amount per single payment (in token base units).
    /// Acts as a safety limit to prevent accidental overspend.
    #[serde(default)]
    pub max_per_payment: Option<u64>,

    /// Maximum total spend per session (in token base units).
    /// Resets when the agent session ends.
    #[serde(default)]
    pub max_per_session: Option<u64>,
}
```

Add the field to the existing `ChainConfig` struct:

```rust
    /// MPP (Machine Payments Protocol) configuration for agent payments.
    #[serde(default)]
    pub mpp: Option<MppConfig>,
```

Config surface in `roko.toml`:

```toml
[chain.mpp]
wallet_key_env = "ROKO_MPP_WALLET_KEY"
default_mode = "one_time"
max_per_payment = 10000000   # 10 USDC max per payment
max_per_session = 50000000   # 50 USDC max per session
```

### 17.4 Register module in lib.rs

**File**: `crates/roko-chain/src/lib.rs`

Add:

```rust
#[cfg(feature = "mpp")]
pub mod mpp_client;

#[cfg(feature = "mpp")]
pub use mpp_client::{MppClient, MppConfig as MppClientConfig, VerifiedPayment, PaymentSettlement, PaymentMode};
```

The config-level `MppConfig` (in roko-core) and the runtime `MppConfig` (in mpp_client.rs) are separate types. The re-export aliases the runtime version as `MppClientConfig` to avoid collision when both are in scope.

### 17.5 Tests

**File**: `crates/roko-chain/src/mpp_client.rs` (append at bottom)

```rust
#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    // ── Serialization roundtrips ─────────────────────────────────────

    #[test]
    fn mpp_config_serde_roundtrip() {
        let config = MppConfig {
            rpc_url: "https://rpc.tempo.xyz".to_string(),
            wallet_key_env: "TEST_KEY".to_string(),
            default_mode: PaymentMode::OneTime,
            max_per_payment: Some(10_000_000),
            max_per_session: Some(50_000_000),
        };
        let json = serde_json::to_string(&config).unwrap();
        let roundtripped: MppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtripped.rpc_url, config.rpc_url);
        assert_eq!(roundtripped.wallet_key_env, config.wallet_key_env);
        assert_eq!(roundtripped.max_per_payment, Some(10_000_000));
        assert_eq!(roundtripped.max_per_session, Some(50_000_000));
    }

    #[test]
    fn verified_payment_serde_roundtrip() {
        let payment = VerifiedPayment {
            service_url: "https://api.example.com/v1/query".to_string(),
            amount: "1000000".to_string(),
            token: "0xUSDC".to_string(),
            settlement: VerifiedState {
                data: PaymentSettlement {
                    tx_hash: "0xabc123".to_string(),
                    from: "0xSender".to_string(),
                    to: "0xRecipient".to_string(),
                    amount: "1000000".to_string(),
                    memo: Some("service:api.example.com".to_string()),
                },
                chain_id: 4217,
                network: "tempo".to_string(),
                block_number: 142857,
                block_hash: "0xblockhash".to_string(),
                block_timestamp: 1700000000,
                trust_level: TrustLevel::RpcTrusted,
                consensus_mechanism: "threshold_bls".to_string(),
                consensus_proof_bytes: vec![],
                state_proof_bytes: vec![],
                verified_at: 1700000001,
            },
            service_response: serde_json::json!({"result": "ok"}),
        };
        let json = serde_json::to_string(&payment).unwrap();
        let roundtripped: VerifiedPayment = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtripped.service_url, payment.service_url);
        assert_eq!(roundtripped.settlement.data.tx_hash, "0xabc123");
        assert_eq!(roundtripped.settlement.trust_level, TrustLevel::RpcTrusted);
    }

    #[test]
    fn payment_settlement_serde_roundtrip() {
        let settlement = PaymentSettlement {
            tx_hash: "0xdef456".to_string(),
            from: "0xAlice".to_string(),
            to: "0xBob".to_string(),
            amount: "5000000".to_string(),
            memo: None,
        };
        let json = serde_json::to_string(&settlement).unwrap();
        let roundtripped: PaymentSettlement = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtripped.tx_hash, "0xdef456");
        assert!(roundtripped.memo.is_none());
    }

    #[test]
    fn payment_mode_serde() {
        let one_time = serde_json::to_string(&PaymentMode::OneTime).unwrap();
        assert_eq!(one_time, "\"one_time\"");
        let session = serde_json::to_string(&PaymentMode::Session).unwrap();
        assert_eq!(session, "\"session\"");

        let parsed: PaymentMode = serde_json::from_str("\"session\"").unwrap();
        assert!(matches!(parsed, PaymentMode::Session));
    }

    // ── Constructor error handling ───────────────────────────────────

    // NOTE: MppClient::new() requires the mpp crate at runtime, so we
    // test the failure path (env var not set) without needing a real
    // mpp-rs provider. The success path requires a live TempoProvider
    // and is covered by integration tests.

    // This test verifies that MppClient::new fails gracefully when the
    // wallet key environment variable is not set, rather than panicking.
    //
    // Uncomment when mpp crate is available:
    //
    // #[test]
    // fn new_fails_when_env_var_not_set() {
    //     use std::sync::Arc;
    //     use crate::{MockChainClient, adapter::create_rpc_verifier};
    //     use crate::verified_client::VerifiedChainClient;
    //
    //     let mock = MockChainClient::local();
    //     mock.mine_empty_block();
    //     let client = Arc::new(mock);
    //     let verifier = create_rpc_verifier(client.clone());
    //     let vc = Arc::new(VerifiedChainClient::new(client, verifier, "test", 31337));
    //
    //     // Use a unique env var name to avoid collisions with real env
    //     let config = MppConfig {
    //         rpc_url: "https://rpc.tempo.xyz".to_string(),
    //         wallet_key_env: "ROKO_TEST_MPP_KEY_DOES_NOT_EXIST_12345".to_string(),
    //         default_mode: PaymentMode::OneTime,
    //         max_per_payment: None,
    //         max_per_session: None,
    //     };
    //
    //     let result = MppClient::new(config, vc);
    //     assert!(result.is_err());
    //     let err = result.unwrap_err().to_string();
    //     assert!(err.contains("not set"), "expected 'not set' in error: {err}");
    // }
}
```

Tests for the config struct in `crates/roko-core/src/config/chain.rs` (add to existing test module):

```rust
    #[test]
    fn mpp_config_defaults() {
        let config: MppConfig = serde_json::from_str("{}").unwrap();
        assert!(config.wallet_key_env.is_none());
        assert!(config.default_mode.is_none());
        assert!(config.max_per_payment.is_none());
        assert!(config.max_per_session.is_none());
    }

    #[test]
    fn mpp_config_from_toml() {
        let toml_str = r#"
            wallet_key_env = "ROKO_MPP_WALLET_KEY"
            default_mode = "one_time"
            max_per_payment = 10000000
            max_per_session = 50000000
        "#;
        let config: MppConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.wallet_key_env.as_deref(), Some("ROKO_MPP_WALLET_KEY"));
        assert_eq!(config.default_mode.as_deref(), Some("one_time"));
        assert_eq!(config.max_per_payment, Some(10_000_000));
        assert_eq!(config.max_per_session, Some(50_000_000));
    }

    #[test]
    fn chain_config_with_mpp_field() {
        let toml_str = r#"
            [mpp]
            wallet_key_env = "MY_KEY"
            max_per_payment = 5000000
        "#;
        let config: ChainConfig = toml::from_str(toml_str).unwrap();
        assert!(config.mpp.is_some());
        let mpp = config.mpp.unwrap();
        assert_eq!(mpp.wallet_key_env.as_deref(), Some("MY_KEY"));
        assert_eq!(mpp.max_per_payment, Some(5_000_000));
    }

    #[test]
    fn chain_config_without_mpp_parses() {
        let toml_str = r#"
            rpc_url = "http://localhost:8545"
            chain_id = 31337
        "#;
        let config: ChainConfig = toml::from_str(toml_str).unwrap();
        assert!(config.mpp.is_none());
    }
```

---

## Verification

```bash
# Build with mpp feature (may fail on first attempt due to alloy version mismatch --
# see Open Question #1)
cargo build -p roko-chain --features mpp

# Run tests
cargo test -p roko-chain --features mpp

# Lint
cargo clippy -p roko-chain --features mpp --no-deps -- -D warnings

# Verify the config-side types compile without the mpp feature
cargo test -p roko-core

# Verify no workspace breakage
cargo test --workspace
```

---

## Open Questions

1. **alloy version mismatch**: mpp 0.10.0 uses alloy 2.0, but the roko workspace uses alloy 1.x. This will likely cause duplicate type errors if alloy types cross the mpp<->roko boundary. Mitigation options:
   - Pin mpp to a compatible version (check if a 0.9.x or earlier uses alloy 1.x)
   - Keep mpp types fully internal to `mpp_client.rs` and convert to strings/bytes at the boundary
   - Wait for the workspace to upgrade to alloy 2.0 before wiring this
   - Use `mpp` as a completely isolated dependency with no shared alloy types

2. **Payment-Receipt header parsing**: mpp-rs's `Fetch` trait returns a raw `reqwest::Response`. The receipt header name and format need to be confirmed from the mpp-rs source code. The header likely contains the settlement tx_hash needed for `verify_transfer()`, but the exact field name (e.g., `Payment-Receipt`, `X-Payment-Receipt`, `Mpp-Receipt`) and format (JSON, base64, plain hex) are TBD.

3. **Settlement confirmation timing**: How long after the mpp-rs payment completes before the settlement tx is on-chain and verifiable by the light client? Tempo finalizes in ~500ms, so minimal delay is expected. However, `verify_transfer()` may need a retry loop with backoff if called immediately after payment.

4. **Session lifecycle**: Should `MppClient` own the session lifecycle (open/close channels automatically), or should the agent control it explicitly via `open_session()` / `close_session()` methods? Automatic management is simpler but gives the agent less control over channel funding and lifetime.
