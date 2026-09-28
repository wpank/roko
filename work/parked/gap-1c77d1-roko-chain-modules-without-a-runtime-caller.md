+++
id = "gap-1c77d1"
kind = "gap"
title = "roko-chain modules without a runtime caller: witness, x402 live token, KORAI, collusion, chain gates, heartbeat"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-chain"]
created = 2026-09-28
updated = 2026-09-28
source = "gaps-md#roko-chain-local-only-and-shelved-runtime-modules"
anchors = ["crates/roko-chain/src/witness.rs", "crates/roko-chain/src/x402.rs", "crates/roko-chain/src/gate/mev_gate.rs", "crates/roko-chain/src/heartbeat_ext.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++

These modules have tested logic but no production caller:
- `witness.rs` needs a deployed witness registry contract.
- `x402.rs` needs ERC-3009 and state channels on a live token.
- `korai_token.rs` has no deployed token contract.
- `collusion.rs` has no multi-agent marketplace to run against yet.
- `gate/mev_gate.rs`, `gate/tx_sim_gate.rs` and `gate/wallet_gate.rs` are not in the 7-rung gate pipeline.
- `heartbeat_ext.rs` has no chain-aware lifecycle.

The contract and consensus pieces depend on the external chain devnet. The others need roko-side service, persistence or authorisation adapters.

Fix: first wire the three chain gates as optional rungs for chain-domain plans. The rest waits for deployments.
