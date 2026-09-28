+++
id = "gap-8cc2a2"
kind = "gap"
title = "Chain tool handlers are 'Phase 2+ not wired' placeholders (roko-std ChainToolHandler, chain.wallet_create)"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-std/tools"]
created = 2026-08-17
updated = 2026-09-28
source = "crates/roko-std/src/tool/handlers.rs:61"
discovered_from = "audit:crates/roko-std/src/tool/handlers.rs:61"
anchors = ["roko_std::tool::handlers::ChainToolHandler", "crates/roko-cli/src/chain_handler.rs::handle_wallet_create"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
roko-std registers chain tools whose handler returns a structured 'Phase 2+ not wired' error; chain.wallet_create in roko-cli needs a KMS not wired into ChainWallet. Wire or remove per the chain deprecation plan (v3 roadmap §4).

Imported without verification from:
- `crates/roko-std/src/tool/handlers.rs:61`
- `crates/roko-std/src/tool/handlers.rs:95`
- `crates/roko-cli/src/chain_handler.rs:833`
- `docs/v2/14-TOOLS.md:4`
- `docs/v3/39-ROADMAP.md#4. Chain Deprecation`

How to verify: List tools registered with --features chain and which return the not-wired error; align with the deprecation decision.
