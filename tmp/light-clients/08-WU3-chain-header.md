# WU-3: Extend ChainHeader with state_root

**Layer**: 0 (no dependencies — start immediately)
**Blocks**: WU-4, WU-5
**Estimated effort**: 1 hour
**Crate**: `crates/roko-chain`

---

## Overview

Add `state_root: String` field to `ChainHeader`. This is required for MPT proof verification — every EVM chain's state proofs are verified against the state root in a block header.

This is a **breaking change** to a public type. Every place that constructs a `ChainHeader` must be updated.

---

## Pre-read

- `crates/roko-chain/src/types.rs` — current ChainHeader definition (line 42)
- `crates/roko-chain/src/mock.rs` — MockChainClient creates headers
- `crates/roko-chain/src/alloy_impl.rs` — AlloyChainClient creates headers from RPC

---

## Tasks

### 3.1 Add `state_root` field to `ChainHeader`

**File**: `crates/roko-chain/src/types.rs`

Change `ChainHeader` at line 42:

```rust
/// Simplified block header for light-client needs.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChainHeader {
    /// Block number.
    pub number: BlockNumber,
    /// Block hash (hex, `0x`-prefixed).
    pub hash: String,
    /// Parent block hash.
    pub parent: String,
    /// Unix timestamp (seconds).
    pub timestamp: u64,
    /// State trie root hash (hex, `0x`-prefixed).
    /// Required for MPT proof verification against this block.
    pub state_root: String,
}
```

### 3.2 Update existing test in `types.rs`

The `receipt_serde_roundtrip` test doesn't construct `ChainHeader` so it's fine. But check if any other test in the file does. If so, add `state_root: "0x0...0".into()`.

### 3.3 Update `MockChainClient` — genesis block

**File**: `crates/roko-chain/src/mock.rs`

Find `MockChainClient::local()` — it creates a genesis `ChainHeader`. Add `state_root`:

```rust
// In local() constructor — genesis block
ChainHeader {
    number: 0,
    hash: "0x0000000000000000000000000000000000000000000000000000000000000000".into(),
    parent: "0x0000000000000000000000000000000000000000000000000000000000000000".into(),
    timestamp: 0,
    state_root: "0x0000000000000000000000000000000000000000000000000000000000000000".into(),
}
```

### 3.4 Update `MockChainClient::mine_empty_block()`

**File**: `crates/roko-chain/src/mock.rs`

Find `mine_empty_block()` — it constructs a `ChainHeader` for the new block. Add `state_root`:

```rust
// In mine_empty_block()
let header = ChainHeader {
    number: next_num,
    hash: format!("0x{:064x}", next_num),
    parent: prev_hash,
    timestamp: /* existing */,
    state_root: "0x0000000000000000000000000000000000000000000000000000000000000000".into(),
};
```

### 3.5 Update `AlloyChainClient::get_block_header()`

**File**: `crates/roko-chain/src/alloy_impl.rs`

Find `get_block_header()` implementation. The alloy `Block` response includes `header.state_root`. Extract it:

```rust
async fn get_block_header(&self, number: BlockNumber) -> ChainResult<ChainHeader> {
    let block = self.provider
        .get_block_by_number(BlockNumberOrTag::Number(number), false)
        .await
        .map_err(to_rpc_err)?
        .ok_or_else(|| ChainError::Rpc(format!("block {number} not found")))?;

    Ok(ChainHeader {
        number: block.header.number,
        hash: format!("{:?}", block.header.hash),
        parent: format!("{:?}", block.header.parent_hash),
        timestamp: block.header.timestamp,
        state_root: format!("{:?}", block.header.state_root),  // NEW
    })
}
```

### 3.6 Find and fix ALL other `ChainHeader` construction sites

**Command to find them**:
```bash
grep -rn 'ChainHeader {' crates/roko-chain/src/ --include='*.rs'
grep -rn 'ChainHeader {' crates/roko-chain/tests/ --include='*.rs'
```

For each site found, add `state_root: "0x0000000000000000000000000000000000000000000000000000000000000000".into()`.

**Known files to check** (based on exploration):
- `src/observer.rs` — if it constructs test headers
- `src/heartbeat_ext.rs` — if it constructs test headers
- `src/triage.rs` — if it constructs test headers
- `tests/alloy_live.rs` — if it constructs headers in assertions
- Any `#[cfg(test)]` blocks in other modules

### 3.7 Update any tests that assert on `ChainHeader` fields

Some tests may do pattern matching or field access on `ChainHeader`. Ensure `state_root` is handled.

---

## Verification Checklist

- [ ] `ChainHeader` has `state_root: String` field
- [ ] `MockChainClient::local()` genesis block has `state_root`
- [ ] `MockChainClient::mine_empty_block()` produces headers with `state_root`
- [ ] `AlloyChainClient::get_block_header()` extracts `state_root` from alloy block
- [ ] ALL `ChainHeader {` construction sites updated (grep shows zero missing)
- [ ] `cargo test -p roko-chain` — all 263+ existing tests still pass
- [ ] `cargo test -p roko-chain --features alloy-backend` — alloy tests pass
- [ ] `cargo clippy -p roko-chain --no-deps -- -D warnings` — no new warnings
- [ ] `cargo test --workspace` — no breakage from public type change
