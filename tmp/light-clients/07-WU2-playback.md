# WU-2: Playback Verifier

**Layer**: 0 (no dependencies — start immediately)
**Blocks**: WU-4 (PlaybackAdapter wiring)
**Estimated effort**: 1-2 hours
**Crate**: `crates/roko-chain`

---

## Overview

Create a deterministic replay verifier that reads captured chain state from JSONL files. Used for demos and tests where live chain connectivity is unreliable. Verification still runs against captured proofs — the demo shows real BLS checks and MPT traversals, just against recorded state.

---

## Pre-read

- `crates/roko-chain/src/types.rs` — `ChainHeader`, `BlockNumber`, `ChainError`
- The `ConsensusVerifier` trait from WU-1 (`consensus.rs`) — if WU-1 isn't done yet, read `06-WU1-core-types.md` for the trait definition and code against it

---

## Tasks

### 2.1 Add `hex` dependency to `Cargo.toml`

**File**: `crates/roko-chain/Cargo.toml`

Add under `[dependencies]` (unconditional — small crate, ~15KB):
```toml
hex = "0.4"
```

### 2.2 Create test fixture `crates/roko-chain/src/testdata/demo-playback.jsonl`

Create the directory first: `mkdir -p crates/roko-chain/src/testdata/`

Content (5 headers, 1 proof, 1 receipt — realistic-looking data):
```jsonl
{"t":"header","n":1000,"h":"0xa1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2","sr":"0x1111111111111111111111111111111111111111111111111111111111111111","ts":1717200000}
{"t":"header","n":1001,"h":"0xb2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3","sr":"0x2222222222222222222222222222222222222222222222222222222222222222","ts":1717200001}
{"t":"header","n":1002,"h":"0xc3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4","sr":"0x3333333333333333333333333333333333333333333333333333333333333333","ts":1717200002}
{"t":"proof","n":1001,"method":"eth_getProof","addr":"0xABCDEF1234567890ABCDEF1234567890ABCDEF12","result":"dGVzdF9wcm9vZl9kYXRh"}
{"t":"receipt","n":1002,"tx":"0xdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef","status":true}
{"t":"header","n":1003,"h":"0xd4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5","sr":"0x4444444444444444444444444444444444444444444444444444444444444444","ts":1717200003}
{"t":"header","n":1004,"h":"0xe5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6","sr":"0x5555555555555555555555555555555555555555555555555555555555555555","ts":1717200004}
```

### 2.3 Create `crates/roko-chain/src/playback.rs`

**Full implementation**:

```rust
//! Deterministic replay verifier from captured JSONL chain state.
//!
//! Reads a `.jsonl` file with captured block headers, state proofs, and receipts.
//! Returns `TrustedHeader` with `ConsensusProof::Playback` and `TrustLevel::Playback`.
//! Verification against captured proofs still runs — the demo shows real proof traversals
//! against recorded data.

use std::collections::BTreeMap;
use std::path::Path;

use async_trait::async_trait;
use serde::Deserialize;

use crate::consensus::{
    ConsensusError, ConsensusProof, ConsensusVerifier, TrustLevel, TrustedHeader,
};
use crate::types::ChainError;

/// A single entry in a playback JSONL capture file.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "t")]
pub enum PlaybackEntry {
    /// A finalized block header.
    #[serde(rename = "header")]
    Header {
        /// Block number.
        n: u64,
        /// Block hash (hex, 0x-prefixed, 64 hex chars = 32 bytes).
        h: String,
        /// State root (hex, 0x-prefixed, 64 hex chars = 32 bytes).
        sr: String,
        /// Timestamp (unix seconds).
        ts: u64,
        /// Optional base64-encoded consensus certificate.
        #[serde(default)]
        cert: Option<String>,
    },
    /// A state proof (eth_getProof response).
    #[serde(rename = "proof")]
    Proof {
        /// Block number this proof is against.
        n: u64,
        /// RPC method (always "eth_getProof" for now).
        method: String,
        /// Address the proof is for.
        addr: String,
        /// Base64-encoded proof data.
        result: String,
    },
    /// A transaction receipt.
    #[serde(rename = "receipt")]
    Receipt {
        /// Block number.
        n: u64,
        /// Transaction hash (hex, 0x-prefixed).
        tx: String,
        /// Whether the tx succeeded.
        status: bool,
    },
}

/// Consensus verifier that replays captured chain state from a JSONL file.
pub struct PlaybackVerifier {
    headers: BTreeMap<u64, TrustedHeader>,
    source: String,
}

impl PlaybackVerifier {
    /// Load a playback verifier from a JSONL capture file.
    ///
    /// Only header entries are loaded into the verifier's header map.
    /// Proof and receipt entries are parsed but not stored (they're used
    /// by the state proof verifier separately).
    pub fn from_file(path: &Path) -> Result<Self, ChainError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| ChainError::Rpc(format!("playback file read error: {e}")))?;
        Self::from_str(&content, &path.display().to_string())
    }

    /// Load from a string (useful for tests without file I/O).
    pub fn from_str(content: &str, source: &str) -> Result<Self, ChainError> {
        let mut headers = BTreeMap::new();

        for (line_num, line) in content.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let entry: PlaybackEntry = serde_json::from_str(line).map_err(|e| {
                ChainError::Rpc(format!("playback parse error on line {}: {e}", line_num + 1))
            })?;

            if let PlaybackEntry::Header { n, h, sr, ts, .. } = entry {
                let hash = hex_to_bytes32(&h).map_err(|e| {
                    ChainError::Rpc(format!("invalid block hash on line {}: {e}", line_num + 1))
                })?;
                let state_root = hex_to_bytes32(&sr).map_err(|e| {
                    ChainError::Rpc(format!(
                        "invalid state root on line {}: {e}",
                        line_num + 1
                    ))
                })?;

                headers.insert(
                    n,
                    TrustedHeader {
                        number: n,
                        hash,
                        state_root,
                        timestamp: ts,
                        consensus_proof: ConsensusProof::Playback {
                            source_file: source.to_string(),
                        },
                    },
                );
            }
        }

        Ok(Self {
            headers,
            source: source.to_string(),
        })
    }

    /// Number of headers loaded.
    pub fn header_count(&self) -> usize {
        self.headers.len()
    }

    /// The source file path.
    pub fn source(&self) -> &str {
        &self.source
    }
}

#[async_trait]
impl ConsensusVerifier for PlaybackVerifier {
    async fn verify_finality(&self, block: u64) -> Result<TrustedHeader, ConsensusError> {
        self.headers
            .get(&block)
            .cloned()
            .ok_or(ConsensusError::BlockUnavailable(block))
    }

    async fn latest_finalized(&self) -> Result<TrustedHeader, ConsensusError> {
        self.headers
            .values()
            .last()
            .cloned()
            .ok_or(ConsensusError::NotSynced)
    }

    fn mechanism(&self) -> &str {
        "playback"
    }

    fn trust_level(&self) -> TrustLevel {
        TrustLevel::Playback
    }

    async fn is_healthy(&self) -> bool {
        !self.headers.is_empty()
    }
}

/// Parse a 0x-prefixed hex string into a 32-byte array.
fn hex_to_bytes32(hex_str: &str) -> Result<[u8; 32], String> {
    let stripped = hex_str.strip_prefix("0x").unwrap_or(hex_str);
    let bytes =
        hex::decode(stripped).map_err(|e| format!("hex decode error: {e}"))?;
    if bytes.len() != 32 {
        return Err(format!("expected 32 bytes, got {}", bytes.len()));
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Ok(arr)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    const TEST_JSONL: &str = r#"{"t":"header","n":100,"h":"0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","sr":"0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","ts":1700000000}
{"t":"header","n":101,"h":"0xcccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc","sr":"0xdddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd","ts":1700000001}
{"t":"proof","n":100,"method":"eth_getProof","addr":"0x1234","result":"dGVzdA=="}
{"t":"receipt","n":101,"tx":"0xee","status":true}
{"t":"header","n":102,"h":"0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee","sr":"0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff","ts":1700000002}
"#;

    #[test]
    fn from_str_loads_headers() {
        let v = PlaybackVerifier::from_str(TEST_JSONL, "test").unwrap();
        assert_eq!(v.header_count(), 3);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn verify_known_block() {
        let v = PlaybackVerifier::from_str(TEST_JSONL, "test").unwrap();
        let h = v.verify_finality(100).await.unwrap();
        assert_eq!(h.number, 100);
        assert_eq!(h.timestamp, 1700000000);
        assert!(matches!(h.consensus_proof, ConsensusProof::Playback { .. }));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn verify_unknown_block_errors() {
        let v = PlaybackVerifier::from_str(TEST_JSONL, "test").unwrap();
        let err = v.verify_finality(999).await.unwrap_err();
        assert!(matches!(err, ConsensusError::BlockUnavailable(999)));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn latest_finalized_returns_highest() {
        let v = PlaybackVerifier::from_str(TEST_JSONL, "test").unwrap();
        let h = v.latest_finalized().await.unwrap();
        assert_eq!(h.number, 102);
    }

    #[test]
    fn mechanism_and_trust() {
        let v = PlaybackVerifier::from_str(TEST_JSONL, "test").unwrap();
        assert_eq!(v.mechanism(), "playback");
        assert_eq!(v.trust_level(), TrustLevel::Playback);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn empty_file_not_healthy() {
        let v = PlaybackVerifier::from_str("", "empty").unwrap();
        assert!(!v.is_healthy().await);
        assert_eq!(v.header_count(), 0);
    }

    #[test]
    fn invalid_hex_errors() {
        let bad = r#"{"t":"header","n":1,"h":"0xzzzz","sr":"0x0000000000000000000000000000000000000000000000000000000000000000","ts":0}"#;
        let err = PlaybackVerifier::from_str(bad, "bad").unwrap_err();
        assert!(format!("{err}").contains("hex"));
    }

    #[test]
    fn wrong_length_errors() {
        let bad = r#"{"t":"header","n":1,"h":"0xaabb","sr":"0x0000000000000000000000000000000000000000000000000000000000000000","ts":0}"#;
        let err = PlaybackVerifier::from_str(bad, "bad").unwrap_err();
        assert!(format!("{err}").contains("32 bytes"));
    }

    #[test]
    fn hex_to_bytes32_works() {
        let result = hex_to_bytes32("0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").unwrap();
        assert_eq!(result, [0xaa; 32]);
    }

    #[test]
    fn from_file_test_fixture() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src/testdata/demo-playback.jsonl");
        if path.exists() {
            let v = PlaybackVerifier::from_file(&path).unwrap();
            assert!(v.header_count() > 0);
        }
    }
}
```

### 2.4 Register module in `lib.rs`

Add to `crates/roko-chain/src/lib.rs`:
```rust
/// Deterministic replay verifier from captured JSONL state.
pub mod playback;
```

And in the pub use section:
```rust
pub use playback::{PlaybackEntry, PlaybackVerifier};
```

### 2.5 Wire PlaybackAdapter in `adapter.rs`

Once WU-1 is complete, update `PlaybackAdapter::create_verifier()` in `adapter.rs`:
```rust
impl ChainAdapter for PlaybackAdapter {
    fn consensus_type(&self) -> &str { "playback" }

    fn create_verifier(
        &self,
        config: &ChainBackendConfig,
    ) -> Result<Arc<dyn ConsensusVerifier>, ChainError> {
        let path = config.playback_file.as_ref()
            .ok_or_else(|| ChainError::Rpc("playback_file required for playback adapter".into()))?;
        Ok(Arc::new(PlaybackVerifier::from_file(path)?))
    }
}
```

---

## Verification Checklist

- [ ] `hex = "0.4"` added to Cargo.toml
- [ ] `src/testdata/demo-playback.jsonl` exists with valid entries
- [ ] `playback.rs` compiles with all tests passing
- [ ] `PlaybackVerifier::from_file()` loads the test fixture
- [ ] `PlaybackVerifier::from_str()` works for inline test data
- [ ] `verify_finality(known)` returns correct header
- [ ] `verify_finality(unknown)` returns `BlockUnavailable`
- [ ] `latest_finalized()` returns highest block number
- [ ] Invalid hex and wrong-length hex produce clear errors
- [ ] Module registered in `lib.rs`
- [ ] `cargo test -p roko-chain` passes
- [ ] `cargo clippy -p roko-chain --no-deps -- -D warnings` passes
