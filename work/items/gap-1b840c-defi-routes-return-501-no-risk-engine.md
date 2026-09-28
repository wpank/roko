+++
id = "gap-1b840c"
kind = "gap"
title = "DeFi routes return 501; no risk engine, venue execution or durable/on-chain adapters"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-chain/defi"]
created = 2026-09-28
updated = 2026-09-28
source = "gaps-md#recently-closed-epic-status/e41"
anchors = ["crates/roko-chain/src/defi.rs", "crates/roko-chain/src/futures_market.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

Local DeFi primitives exist:
- in `defi.rs`: instruments, checked bond and insurance lifecycles, reputation-option pricing, synthetic indices, affect sizing and rate effects;
- futures types in `futures_market.rs`;
- Nelson-Siegel term structures in `nelson_siegel.rs`.

The authenticated routes intentionally return structured 501 responses. Nothing else is in place: no risk engine, no market data, no venue or derivative execution, no durable or on-chain service, and no consumer of the term-structure model.

Fix: scope a risk-admission engine before building any venue adapter.
