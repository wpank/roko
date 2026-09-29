+++
id = "gap-047224"
kind = "gap"
title = "Connectivity has one Connect transport; discovery, MCP/A2A/x402 execution and finality handling are missing"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-core/connect", "roko-serve/relay"]
created = 2026-09-28
updated = 2026-09-28
source = "gaps-md#recently-closed-epic-status/e29"
anchors = ["crates/roko-core/src/connector.rs:319", "crates/roko-core/src/traits.rs:408"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++

E29 plus R01/R02 delivered the five-method async `Connect` contract with one supervised HTTP JSON adapter, the canonical relay envelope, bounded replay, and durable exact-room subscriptions. Still product work: additional transports, startup discovery, MCP/A2A/x402 execution over connectors, finality/reorg processing, and dashboard integration.

Fix: split this into one item per transport or protocol when it is scheduled.
