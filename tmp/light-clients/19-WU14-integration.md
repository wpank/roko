# WU-14: Integration Tests

**Layer**: 4
**Depends on**: WU-10 (config/factory), WU-11 (watcher)
**Blocks**: none (leaf unit)
**Estimated effort**: 2-3 hours
**Crates**: `crates/roko-chain` (integration tests)

---

## Overview

Write integration tests that exercise the full light-client pipeline: config → factory → VerifiedChainClient → verified queries. Two test suites:

1. **Mock tests** (`tests/verified_client.rs`) — run always, use `MockChainClient` + `RpcOnlyVerifier`
2. **Live tests** (`tests/tempo_live.rs`) — feature-gated, require `ROKO_TEST_TEMPO_RPC_URL`, gracefully skip when unavailable

Follow the existing `tests/alloy_live.rs` pattern: `live_or_skip()` function, `#[cfg(feature)]` guard, env var for RPC URL.

---

## Pre-read

- `crates/roko-chain/tests/alloy_live.rs` — existing live test pattern with `live_or_skip()`
- `crates/roko-chain/src/mock.rs` — `MockChainClient::local()`, `mine_empty_block()`, `set_balance()`, `insert_storage()`, `paired_mocks()`
- `crates/roko-chain/src/verified_client.rs` — `VerifiedChainClient` (from WU-7)
- `crates/roko-chain/src/backend_factory.rs` — `build_backend_pool()` (from WU-10)
- `crates/roko-chain/src/watcher.rs` — `spawn_watcher()`, `WatcherEvent` (from WU-11)

---

## Tasks

### 14.1 Create `crates/roko-chain/tests/verified_client.rs`

This runs with default features (no `alloy-backend` required):

```rust
//! Integration tests for VerifiedChainClient with mock infrastructure.
//!
//! These tests verify the full pipeline: ChainClient → ConsensusVerifier →
//! VerifiedChainClient → VerifiedState<T>. No live RPC needed.

#![allow(clippy::unwrap_used)]

use std::sync::Arc;
use roko_chain::{
    ChainClient, MockChainClient, TxRequest,
    consensus::{ConsensusVerifier, TrustLevel},
    verified_client::VerifiedChainClient,
    adapter::create_rpc_verifier,
};

fn make_client() -> (VerifiedChainClient, Arc<MockChainClient>) {
    let mock = MockChainClient::local();
    mock.mine_empty_block();
    mock.mine_empty_block();
    let mock = Arc::new(mock);
    let verifier = create_rpc_verifier(Arc::clone(&mock) as Arc<dyn ChainClient>);
    let vc = VerifiedChainClient::new(
        Arc::clone(&mock) as Arc<dyn ChainClient>,
        verifier,
        "test-net",
        31337,
    );
    (vc, mock)
}

// ── Drop-in ChainClient replacement ─────────────────────────────────

#[tokio::test]
async fn verified_client_implements_chain_client() {
    let (vc, _mock) = make_client();
    let client: &dyn ChainClient = &vc;
    let block = client.block_number().await.unwrap();
    assert_eq!(block, 2);
    assert_eq!(client.chain_id().await.unwrap(), 31337);
    assert_eq!(client.name(), "verified");
}

#[tokio::test]
async fn verified_client_as_arc_dyn() {
    let (vc, _mock) = make_client();
    let _arc: Arc<dyn ChainClient> = Arc::new(vc);
    // Compile test — proves VerifiedChainClient can be passed anywhere
    // that expects Arc<dyn ChainClient>
}

// ── Verified balance ─────────────────────────────────────────────────

#[tokio::test]
async fn verified_balance_returns_correct_amount() {
    let (vc, mock) = make_client();
    mock.set_balance("0xABC", 1_000_000);

    let vs = vc.verified_balance("0xABC", None).await.unwrap();
    assert_eq!(vs.data, 1_000_000);
    assert_eq!(vs.chain_id, 31337);
    assert_eq!(vs.network, "test-net");
    assert_eq!(vs.trust_level, TrustLevel::RpcTrusted);
    assert_eq!(vs.consensus_mechanism, "rpc");
    assert!(vs.verified_at > 0);
    assert!(vs.block_number <= 2);
}

#[tokio::test]
async fn verified_balance_at_specific_block() {
    let (vc, mock) = make_client();
    mock.set_balance("0xDEF", 500);

    let vs = vc.verified_balance("0xDEF", Some(1)).await.unwrap();
    assert_eq!(vs.block_number, 1);
}

#[tokio::test]
async fn verified_balance_zero_for_unknown_address() {
    let (vc, _mock) = make_client();
    let vs = vc.verified_balance("0xNOBODY", None).await.unwrap();
    assert_eq!(vs.data, 0);
}

// ── Verified storage ─────────────────────────────────────────────────

#[tokio::test]
async fn verified_storage_returns_slot_value() {
    let (vc, mock) = make_client();
    mock.insert_storage("0xContract", "0x01", None, vec![42, 43]);

    let vs = vc.verified_storage("0xContract", "0x01", None).await.unwrap();
    assert_eq!(vs.data, vec![42, 43]);
    assert_eq!(vs.trust_level, TrustLevel::RpcTrusted);
}

#[tokio::test]
async fn verified_storage_empty_for_unset_slot() {
    let (vc, _mock) = make_client();
    let vs = vc.verified_storage("0xContract", "0x99", None).await.unwrap();
    assert!(vs.data.is_empty());
}

// ── Verify transfer ──────────────────────────────────────────────────

#[tokio::test]
async fn verify_transfer_with_successful_tx() {
    let (mock_client, mock_wallet) = roko_chain::paired_mocks(1_000_000);
    let tx = TxRequest {
        to: Some("0xBEEF".into()),
        value: 100,
        ..Default::default()
    };
    let tx_hash = mock_wallet.sign_and_submit(tx).await.unwrap();

    let client = Arc::new(mock_client);
    let verifier = create_rpc_verifier(Arc::clone(&client) as Arc<dyn ChainClient>);
    let vc = VerifiedChainClient::new(client, verifier, "test", 1);

    let vs = vc.verify_transfer(&tx_hash).await.unwrap();
    assert!(vs.data.status);
    assert_eq!(vs.trust_level, TrustLevel::RpcTrusted);
    assert!(vs.block_number > 0);
}

#[tokio::test]
async fn verify_transfer_nonexistent_tx_returns_error() {
    let (vc, _mock) = make_client();
    let bad_hash = roko_chain::TxHash::new("0xdeadbeef");
    let result = vc.verify_transfer(&bad_hash).await;
    assert!(result.is_err());
}

// ── Consensus mechanism propagation ──────────────────────────────────

#[tokio::test]
async fn consensus_accessor_works() {
    let (vc, _mock) = make_client();
    assert_eq!(vc.consensus().mechanism(), "rpc");
    assert_eq!(vc.consensus().trust_level(), TrustLevel::RpcTrusted);
    assert!(vc.consensus().is_healthy().await);
}
```

### 14.2 Create `crates/roko-chain/tests/tempo_live.rs`

Feature-gated live tests against Tempo Moderato testnet:

```rust
//! Live integration tests against Tempo Moderato testnet.
//!
//! Requires:
//! - `alloy-backend` feature enabled
//! - `ROKO_TEST_TEMPO_RPC_URL` env var set (default: https://rpc.moderato.tempo.xyz)
//!
//! Run: `cargo test -p roko-chain --features alloy-backend -- --ignored tempo`
//!
//! These tests are `#[ignore]`d by default so they don't run in regular CI.
//! They gracefully skip if the RPC is unreachable.

#![cfg(feature = "alloy-backend")]
#![allow(clippy::unwrap_used)]

use std::sync::Arc;
use roko_chain::{ChainClient, alloy_impl::AlloyChainClient};
use roko_chain::consensus::TrustLevel;
use roko_chain::verified_client::VerifiedChainClient;
use roko_chain::adapter::create_rpc_verifier;

fn tempo_rpc_url() -> String {
    std::env::var("ROKO_TEST_TEMPO_RPC_URL")
        .unwrap_or_else(|_| "https://rpc.moderato.tempo.xyz".into())
}

async fn tempo_or_skip() -> Option<AlloyChainClient> {
    let url = tempo_rpc_url();
    let Ok(client) = AlloyChainClient::http(&url) else {
        eprintln!("skip: invalid tempo RPC url");
        return None;
    };
    match client.block_number().await {
        Ok(b) => {
            eprintln!("tempo moderato connected: block {b}");
            Some(client)
        }
        Err(e) => {
            eprintln!("skip tempo live tests ({url}): {e}");
            None
        }
    }
}

fn make_verified(client: AlloyChainClient) -> VerifiedChainClient {
    let rpc = Arc::new(client) as Arc<dyn ChainClient>;
    let verifier = create_rpc_verifier(Arc::clone(&rpc));
    VerifiedChainClient::new(rpc, verifier, "tempo-moderato", 4217)
}

#[tokio::test]
#[ignore] // Requires live RPC
async fn tempo_block_number_and_chain_id() {
    let Some(client) = tempo_or_skip().await else { return };
    let block = client.block_number().await.unwrap();
    assert!(block > 0, "tempo should have blocks");
    let chain_id = client.chain_id().await.unwrap();
    eprintln!("tempo chain_id = {chain_id}");
}

#[tokio::test]
#[ignore]
async fn tempo_verified_balance_zero_address() {
    let Some(client) = tempo_or_skip().await else { return };
    let vc = make_verified(client);

    // Zero address should have zero or some balance
    let vs = vc.verified_balance("0x0000000000000000000000000000000000000000", None)
        .await
        .unwrap();

    assert_eq!(vs.chain_id, 4217);
    assert_eq!(vs.network, "tempo-moderato");
    assert_eq!(vs.trust_level, TrustLevel::RpcTrusted);
    assert_eq!(vs.consensus_mechanism, "rpc");
    assert!(vs.block_number > 0);
    eprintln!("zero address balance: {} wei at block {}", vs.data, vs.block_number);
}

#[tokio::test]
#[ignore]
async fn tempo_verified_storage_read() {
    let Some(client) = tempo_or_skip().await else { return };
    let vc = make_verified(client);

    // Read storage slot 0 from zero address (will return empty/zero)
    let vs = vc.verified_storage(
        "0x0000000000000000000000000000000000000000",
        "0x0000000000000000000000000000000000000000000000000000000000000000",
        None,
    ).await.unwrap();

    assert!(vs.block_number > 0);
    eprintln!("storage value: {} bytes", vs.data.len());
}

#[tokio::test]
#[ignore]
async fn tempo_get_block_header_has_state_root() {
    let Some(client) = tempo_or_skip().await else { return };
    let block = client.block_number().await.unwrap();
    let header = client.get_block_header(block).await.unwrap();

    assert_eq!(header.number, block);
    assert!(!header.hash.is_empty());
    assert!(!header.state_root.is_empty(), "state_root should be populated (WU-3)");
    eprintln!("header #{}: hash={}, state_root={}", header.number, header.hash, header.state_root);
}

#[tokio::test]
#[ignore]
async fn tempo_verified_client_as_chain_client() {
    let Some(client) = tempo_or_skip().await else { return };
    let vc = make_verified(client);

    // Use as dyn ChainClient — should work identically
    let dyn_client: &dyn ChainClient = &vc;
    let block = dyn_client.block_number().await.unwrap();
    assert!(block > 0);
    assert_eq!(dyn_client.name(), "verified");
}
```

### 14.3 Create `crates/roko-chain/tests/watcher_integration.rs`

Test the watcher with mocks:

```rust
//! Integration tests for the block watcher.

#![allow(clippy::unwrap_used)]

use std::sync::Arc;
use std::time::Duration;

use roko_chain::{MockChainClient, WatcherConfig, WatcherEvent, spawn_watcher};
use roko_chain::observer::BlockObserverConfig;

#[tokio::test]
async fn watcher_processes_mined_blocks() {
    let mock = MockChainClient::local();

    // Mine some blocks before starting watcher
    for _ in 0..5 {
        mock.mine_empty_block();
    }

    let config = WatcherConfig {
        poll_interval: Duration::from_millis(50),
        observer: BlockObserverConfig::default(),
        max_blocks_per_poll: 10,
    };

    let (handle, mut rx) = spawn_watcher(Arc::new(mock), config, Some(0));

    // Collect events for a short window
    let mut blocks_seen = Vec::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);

    while tokio::time::Instant::now() < deadline {
        match tokio::time::timeout(Duration::from_millis(200), rx.recv()).await {
            Ok(Some(WatcherEvent::NewBlock { block_number, .. })) => {
                blocks_seen.push(block_number);
                if blocks_seen.len() >= 5 {
                    break;
                }
            }
            Ok(Some(_)) => {} // other events
            _ => break,
        }
    }

    assert!(!blocks_seen.is_empty(), "should have seen at least one block");
    // Blocks should be sequential
    for window in blocks_seen.windows(2) {
        assert!(window[1] > window[0], "blocks should be increasing");
    }

    handle.stop().await;
}

#[tokio::test]
async fn watcher_stop_is_clean() {
    let mock = MockChainClient::local();
    mock.mine_empty_block();

    let config = WatcherConfig {
        poll_interval: Duration::from_millis(50),
        ..Default::default()
    };

    let (handle, _rx) = spawn_watcher(Arc::new(mock), config, Some(0));
    assert!(handle.is_running());

    handle.stop().await;
    // No panic, no hang
}
```

### 14.4 Add backend pool integration test

Create `crates/roko-chain/tests/backend_pool.rs`:

```rust
//! Integration tests for backend pool construction.

#![allow(clippy::unwrap_used)]

use std::collections::HashMap;
use roko_chain::build_backend_pool;
use roko_core::config::chain::ChainBackendEntry;

#[test]
fn empty_config_produces_empty_pool() {
    let pool = build_backend_pool(&HashMap::new(), None);
    assert!(pool.is_empty());
    assert!(pool.default_backend().is_none());
    assert!(pool.default_verified_client().is_none());
    assert!(pool.default_rpc_client().is_none());
}

#[cfg(feature = "alloy-backend")]
#[test]
fn missing_rpc_url_is_skipped() {
    let mut entries = HashMap::new();
    entries.insert("broken".into(), ChainBackendEntry {
        rpc_url: None,
        consensus: "rpc".into(),
        ..Default::default()
    });

    let pool = build_backend_pool(&entries, Some("broken"));
    assert!(pool.is_empty());
}

// NOTE: Tests that construct live backends against a real RPC would go
// in tempo_live.rs with the #[ignore] + env-var pattern.
```

---

## Test Execution Guide

```bash
# Mock tests (always run, no external deps)
cargo test -p roko-chain --test verified_client
cargo test -p roko-chain --test watcher_integration
cargo test -p roko-chain --test backend_pool

# Live Tempo tests (requires network)
ROKO_TEST_TEMPO_RPC_URL=https://rpc.moderato.tempo.xyz \
  cargo test -p roko-chain --features alloy-backend -- --ignored tempo

# Full workspace regression
cargo test --workspace
```

---

## Verification Checklist

- [ ] `tests/verified_client.rs` — 10+ tests covering all VerifiedChainClient methods with mocks
- [ ] `tests/tempo_live.rs` — 5+ tests with `#[ignore]`, `tempo_or_skip()` pattern, `alloy-backend` feature gate
- [ ] `tests/watcher_integration.rs` — watcher start/stop/event-collection with mocks
- [ ] `tests/backend_pool.rs` — empty pool, missing RPC skip
- [ ] All mock tests pass: `cargo test -p roko-chain`
- [ ] All mock tests pass with alloy: `cargo test -p roko-chain --features alloy-backend`
- [ ] Live tests skip gracefully when no RPC: `cargo test -p roko-chain --features alloy-backend -- --ignored`
- [ ] `cargo test --workspace` — no breakage
- [ ] No `#[allow(dead_code)]` needed — all types are exercised
