+++
id = "gap-319a2c"
kind = "gap"
title = "roko-chain Phase 2+ modules lack production runtime callers (chain/economic runtime integration)"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-chain"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/archive/CONSOLIDATED-BACKLOG-2026-09-23.md#Phase 2+ Deferred Work (roko-chain modules)"
discovered_from = "audit:tmp/archive/CONSOLIDATED-BACKLOG-2026-09-23.md#Phase 2+ Deferred Work (roko-chain modules)"
anchors = ["crates/roko-chain/src/witness.rs", "crates/roko-chain/src/x402.rs", "crates/roko-chain/src/korai_token.rs", "crates/roko-chain/src/marketplace.rs", "crates/roko-chain/src/agent_registry.rs", "crates/roko-chain/src/reputation_registry.rs", "crates/roko-chain/src/validation_registry.rs", "crates/roko-chain/src/knowledge_registry.rs", "crates/roko-chain/src/indexer.rs", "crates/roko-chain/src/gossip.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
[deferred] 20 roko-chain modules (witness, x402, korai_token, marketplace, registries, indexer, gossip, trace_rank, arena, defi, futures, chain gates mev/tx_sim/wallet, heartbeat_ext) have tested logic but no production caller; each waits on a deployed-contract/transport/durable adapter.

Imported without verification from:
- `tmp/archive/CONSOLIDATED-BACKLOG-2026-09-23.md#Phase 2+ Deferred Work (roko-chain modules)`
- `tmp/archive/CONSOLIDATED-BACKLOG-2026-09-23.md#P3-CHN-1 (Subsystem: Chain / DeFi)`
- `tmp/archive/08-15-26/MASTER-TASKS.md#7. Deferred / Blocked (chain runtime integration)`
- `tmp/archive/status-quo-audit-2026-09-21/06-remaining-work.md#18. Architecture production residuals (chain/economic runtime)`
- `tmp/archive/status-quo-audit-2026-09-21/09-completion-tracking.md#Deferred Items (require multi-sprint effort or operator action)`

How to verify: For each module listed, grep for non-test callers outside crates/roko-chain; confirm chain gates are absent from the 7-rung gate pipeline. / Compare GAPS.md chain residual list with roko-chain adapters.

Merged 2 mined candidates: m1-183, m3-100.
