# W11-B: Chain Client Unwrap Panic + Lock Poisoning

**Priority**: P0 -- unwrap panics crash the process; poisoned Mutex makes all subsequent operations fail permanently
**Effort**: ~20 min
**Files to modify**: 1
**Dependencies**: None

## Problem

Two classes of panic risk in `orchestrate.rs`:

1. **Chain client unwrap**: `Arc::clone(self.chain_client.as_ref().unwrap())` at line 16348 is guarded by an `if self.chain_client.is_some()` check at line 16346, but the guard is not type-level binding. If the code is ever refactored or the check desyncs, it panics at runtime.

2. **Lock poisoning**: Three `.lock().expect()` calls on `std::sync::Mutex` at lines 1552, 1556, and 17875. If any panic occurs while a lock is held (e.g., from an agent event handler), every subsequent `.lock().expect()` panics permanently. The process becomes unrecoverable.

## Exact Code to Change

### File 1: `crates/roko-cli/src/orchestrate.rs`

#### Change 1: Replace chain client `is_some()` + `unwrap()` with `if let Some`

**Find this code** (line 16346):
```rust
            let resolver: Arc<dyn HandlerResolver> = if self.chain_client.is_some() {
                let chain_map = chain_handler_map(
                    Arc::clone(self.chain_client.as_ref().unwrap()),
                    self.chain_wallet.clone(),
                );
                Arc::new(chain_aware_resolver(chain_map))
            } else {
```

**Replace with:**
```rust
            let resolver: Arc<dyn HandlerResolver> = if let Some(client) = self.chain_client.as_ref() {
                tracing::debug!("chain client available -- using chain-aware handler resolver");
                let chain_map = chain_handler_map(
                    Arc::clone(client),
                    self.chain_wallet.clone(),
                );
                Arc::new(chain_aware_resolver(chain_map))
            } else {
```

#### Change 2: Change `EnrichmentRuntimeClient.stats` lock calls to remove `.expect()`

The struct field at line 1547 uses `std::sync::Mutex`:
```rust
    stats: Arc<Mutex<EnrichmentRunStats>>,
```

This stays the same type (it uses the top-level `use std::sync::{Arc, Mutex};` import at line 15). But change it to `parking_lot::Mutex` for poison-safety.

**Find this code** (line 1547):
```rust
    stats: Arc<Mutex<EnrichmentRunStats>>,
```

**Replace with:**
```rust
    stats: Arc<parking_lot::Mutex<EnrichmentRunStats>>,
```

#### Change 3: Remove `.expect()` from `snapshot()`

**Find this code** (line 1551):
```rust
    fn snapshot(&self) -> EnrichmentRunStats {
        self.stats.lock().expect("enrichment stats lock").clone()
    }
```

**Replace with:**
```rust
    fn snapshot(&self) -> EnrichmentRunStats {
        self.stats.lock().clone()
    }
```

#### Change 4: Remove `.expect()` from `record_usage()`

**Find this code** (line 1555):
```rust
    fn record_usage(&self, usage: &roko_agent::Usage) {
        let mut stats = self.stats.lock().expect("enrichment stats lock");
```

**Replace with:**
```rust
    fn record_usage(&self, usage: &roko_agent::Usage) {
        let mut stats = self.stats.lock();
```

#### Change 5: Update the construction site for `EnrichmentRuntimeClient`

Search for where `EnrichmentRuntimeClient` is constructed (the `stats: Arc::new(Mutex::new(...))` call). It should be around line 9206.

**Find this code:**
```rust
            stats: Arc::new(Mutex::new(EnrichmentRunStats::default())),
```

**Replace with:**
```rust
            stats: Arc::new(parking_lot::Mutex::new(EnrichmentRunStats::default())),
```

#### Change 6: Replace gate sink `std::sync::Mutex` with `parking_lot::Mutex`

**Find this code** (line 17865):
```rust
        let sink = Arc::new(Mutex::new(Vec::new()));
```

**Replace with:**
```rust
        let sink = Arc::new(parking_lot::Mutex::new(Vec::new()));
```

**Find this code** (line 17875):
```rust
        let verdicts = sink.lock().expect("recorded gate sink poisoned").clone();
```

**Replace with:**
```rust
        let verdicts = sink.lock().clone();
```

#### Change 7: Remove unused `Mutex` from `std::sync` import

After these changes, the bare `Mutex` name from the `std::sync` import is no longer used
anywhere in the file (the only other `Mutex` usages are fully-qualified as `tokio::sync::Mutex`
or `parking_lot::Mutex`). Leaving the import as-is will cause a dead-code warning / compile
error with `#[deny(unused_imports)]`.

**Find this code** (line 15):
```rust
use std::sync::{Arc, Mutex};
```

**Replace with:**
```rust
use std::sync::Arc;
```

## Verification

```bash
cd /Users/will/dev/nunchi/roko/roko

# 1. Verify parking_lot is in Cargo.toml
grep 'parking_lot' crates/roko-cli/Cargo.toml

# 2. Confirm no unwrap() remains on chain_client
grep -n 'chain_client.*unwrap' crates/roko-cli/src/orchestrate.rs
# Should return 0 results

# 3. Confirm no .expect() on enrichment stats lock
grep -n 'enrichment stats lock' crates/roko-cli/src/orchestrate.rs
# Should return 0 results

# 4. Confirm no .expect() on gate sink
grep -n 'gate sink poisoned' crates/roko-cli/src/orchestrate.rs
# Should return 0 results

# 5. Build
cargo check -p roko-cli

# 6. Test
cargo test -p roko-cli
```

## Agent Prompt

```
Fix two panic risks in `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs`.

## Context

The file has `use std::sync::{Arc, Mutex};` at line 15 and uses `parking_lot::Mutex` already
at line 867-868. `parking_lot` is already a dependency of `roko-cli`.

## Fix 1: Chain client unwrap (line ~16346)

The current code uses a check-then-unwrap pattern that is not type-safe:
```rust
let resolver: Arc<dyn HandlerResolver> = if self.chain_client.is_some() {
    let chain_map = chain_handler_map(
        Arc::clone(self.chain_client.as_ref().unwrap()),  // <-- PANICS if refactored
```

Replace with `if let Some(client) = self.chain_client.as_ref()` and use `Arc::clone(client)`.
Add `tracing::debug!("chain client available -- using chain-aware handler resolver");` inside
the `Some` branch for visibility.

## Fix 2: Lock poisoning (lines ~1547, 1552, 1556, 9206, 17865, 17875)

Replace `std::sync::Mutex` with `parking_lot::Mutex` (no poison) at these specific sites:

1. Line ~1547: Change `stats: Arc<Mutex<EnrichmentRunStats>>` to
   `stats: Arc<parking_lot::Mutex<EnrichmentRunStats>>`

2. Line ~1552: Change `.lock().expect("enrichment stats lock")` to `.lock()`

3. Line ~1556: Change `.lock().expect("enrichment stats lock")` to `.lock()`

4. Line ~9206: Change `Arc::new(Mutex::new(EnrichmentRunStats::default()))` to
   `Arc::new(parking_lot::Mutex::new(EnrichmentRunStats::default()))`

5. Line ~17865: Change `Arc::new(Mutex::new(Vec::new()))` to
   `Arc::new(parking_lot::Mutex::new(Vec::new()))`

6. Line ~17875: Change `.lock().expect("recorded gate sink poisoned")` to `.lock()`

7. Line 15: Change `use std::sync::{Arc, Mutex};` to `use std::sync::Arc;` -- after the
   changes above, the bare `Mutex` import is unused (the remaining Mutex usages in the file
   are all fully-qualified as `tokio::sync::Mutex` or `parking_lot::Mutex`). Leaving it
   will cause a dead-code lint error.

Run `cargo check -p roko-cli` and `cargo test -p roko-cli` to verify.
```

## Commit

This batch is committed with Wave 11. Do not commit individually.

## Checklist

- [ ] `if self.chain_client.is_some()` + `unwrap()` replaced with `if let Some(client)`
- [ ] `tracing::debug!` added for chain client branch visibility
- [ ] `EnrichmentRuntimeClient.stats` uses `parking_lot::Mutex`
- [ ] `snapshot()` lock call has no `.expect()`
- [ ] `record_usage()` lock call has no `.expect()`
- [ ] Construction site at ~line 9206 uses `parking_lot::Mutex::new`
- [ ] Gate sink creation at ~line 17865 uses `parking_lot::Mutex::new`
- [ ] Gate sink lock at ~line 17875 has no `.expect()`
- [ ] `use std::sync::{Arc, Mutex}` changed to `use std::sync::Arc` (bare `Mutex` no longer used)
- [ ] `cargo check -p roko-cli` passes
- [ ] `cargo test -p roko-cli` passes

## Audit Status

Audited: 2026-05-05. 3 issues fixed: (1) Change 7 incorrectly said to leave `use std::sync::{Arc, Mutex}` as-is because `ConnectorRegistry`/`FeedRegistry` still use bare `Mutex` -- those types do not exist in this file; after all changes bare `Mutex` is unused and the import must become `use std::sync::Arc;` to avoid dead-import lint error. (2) Updated Agent Prompt step 7 accordingly. (3) Updated checklist item.
