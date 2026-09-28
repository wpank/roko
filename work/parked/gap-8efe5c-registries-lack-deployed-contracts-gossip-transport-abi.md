+++
id = "gap-8efe5c"
kind = "gap"
title = "Registries lack deployed contracts, gossip transport, ABI decoding and a background indexer"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-chain/registries"]
created = 2026-09-28
updated = 2026-09-28
source = "gaps-md#recently-closed-epic-status/e39"
anchors = ["crates/roko-chain/src/agent_registry.rs", "crates/roko-chain/src/indexer.rs", "crates/roko-chain/src/gossip.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++

The local identity/delegation, knowledge and reputation registries are durable behind authenticated serve routes, and an optional, manually run, finality-aware read-only index exists. Missing:
- deployed compatible contracts, and runtime adapters for `reputation_registry.rs` and `validation_registry.rs`;
- a governance adapter for `knowledge_registry.rs`;
- a network transport for `gossip.rs`;
- for `indexer.rs`: ABI decoding, transaction-hash enrichment, a background/WebSocket worker, and a PostgreSQL/SSE service;
- a production consumer and persistence for `trace_rank.rs`.

The best-effort legacy `register(string,bytes32)` dual-write on agent registration is not integrated with the passport lifecycle and may not match the target deployed ABI.

Fix: split this per adapter once the target chain deployment exists.
