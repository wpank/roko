# Depth: Chain Event Pipeline

> Parent: [15-TRIGGERS](../../15-TRIGGERS.md) -- Section 3.5
>
> **Deprecation notice**: The chain event trigger subsystem is deprecated as
> part of the broader chain deprecation. The `roko-chain` crate and its
> associated runtime primitives are migrating to the separate `daeji`
> repository. This documentation describes the existing implementation for
> reference; no new chain trigger features will be added.

---

## Overview

Chain event triggers process on-chain EVM logs through a five-stage pipeline
inside the `TriggerCoordinator`. The pipeline handles both pre-decoded events
(from the legacy `ServerEvent::ChainContractEvent` path) and raw log data
(from `ServerEvent::ChainLogObserved`), with automatic finality promotion
and reorg invalidation.

## Stage 1: Raw log observation

The `observe_raw_chain_log` method processes `ServerEvent::ChainLogObserved`
events, which carry raw topics and data bytes. For each armed `ChainEvent`
binding matching the chain ID and contract address:

1. **Dedup check**: `chain_log_seen_for_binding` ensures the same log (keyed
   by chain_id + block_number + block_hash + tx_hash + log_index) has not
   already been submitted for this binding, covering both pending and
   delivered maps.

2. **Removed flag**: If the log carries `removed = true`, the coordinator
   immediately routes to reorg invalidation for that block hash.

## Stage 2: ABI decoding

When a binding includes a JSON ABI, `decode_chain_log` uses the `alloy`
crate stack (feature-gated behind `#[cfg(feature = "chain")]`):

```rust
use alloy_dyn_abi::EventExt;
use alloy_json_abi::Event as AbiEvent;
use alloy_primitives::B256;
```

The decoder:

1. Parses the ABI's event definition.
2. Computes the topic0 selector from the event signature.
3. Matches topic0 against the first raw topic.
4. Decodes indexed and non-indexed parameters from the remaining topics and
   data bytes using `EventExt::decode_log_parts`.
5. Returns a structured JSON value with named fields.

Decoding errors emit a `TriggerEventKind::Error` lifecycle event with
`phase: "chain_decode"` and the log's block/tx/index coordinates.

## Stage 3: Finality gating

Each `ChainEventTrigger` declares a `FinalityRequirement`:

| Level | Confirmations required | Constant |
|---|---|---|
| `Reversible` | 0 | Fires immediately |
| `QuasiFinalized` | 12 | `QUASI_FINALITY_CONFIRMATIONS` |
| `Final` | 64 | `FINAL_CONFIRMATIONS` |

If the current finality of the log meets or exceeds the binding's
requirement, the event is submitted immediately. Otherwise, it enters the
`pending_chain` map keyed by `ChainLogKey`.

## Stage 4: Canonical block tracking and finality promotion

The `observe_chain_block` method maintains the `canonical_blocks` BTreeMap.
On each new block:

1. If the block is already known at the same height with the same hash,
   `promote_confirmed_chain_events` is called to check pending events.

2. If the block's parent hash does not match the known canonical chain at
   height-1, a reorg is detected. The coordinator identifies orphaned
   blocks from the fork point forward, invalidates them, then inserts the
   new block.

`promote_confirmed_chain_events` iterates pending events and computes
confirmations as `head - event_block_number`. Events whose block hash no
longer matches the canonical hash are invalidated.

Explicit `ServerEvent::ChainFinalityUpdated` events also drive promotion
via `promote_chain_finality`, which upgrades pending events for a specific
block hash.

## Stage 5: Reorg invalidation

`invalidate_chain_reorg` handles orphaned block hashes:

1. Removes all pending events on orphaned blocks from `pending_chain`.
2. For delivered events on orphaned blocks, emits `TriggerEventKind::Error`
   lifecycle events with `phase: "chain_reorg"` and `reorg_invalidated: true`,
   providing a durable audit trail that a previously-delivered trigger was
   based on a block that has been reorged away.
3. Removes the invalidated entries from `delivered_chain`.

## Idempotency guarantees

The `ChainLogKey` struct provides a composite key:

```rust
struct ChainLogKey {
    chain_id: u64,
    block_number: u64,
    block_hash: String,
    tx_hash: String,
    log_index: u32,
}
```

Combined with `chain_log_seen_for_binding` (which checks both pending and
delivered maps), this ensures each log is processed at most once per binding
even across finality promotions and reorg replays.

**Source:** `crates/roko-serve/src/trigger_runtime.rs` (methods `observe_raw_chain_log`,
`observe_chain_block`, `promote_confirmed_chain_events`, `promote_chain_finality`,
`invalidate_chain_reorg`, `decode_chain_log`)
