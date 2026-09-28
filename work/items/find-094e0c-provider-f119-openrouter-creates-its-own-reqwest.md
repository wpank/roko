+++
id = "find-094e0c"
kind = "finding"
title = "[provider F119] OpenRouter creates its own reqwest::Client instead of using shared pool"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-agent/provider"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F119"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F119"
anchors = ["crates/roko-agent/src/provider/openrouter.rs", "reqwest::Client"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
`OpenRouterAdapter::create_agent()` constructs a new `reqwest::Client` per provider. This bypasses the shared connection pool, creating separate connection pools per adapter instance.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F119`
- `tmp/archive/provider-audit/01-provider-adapters.md`

Warning: every file this item cites is gone (`crates/roko-agent/src/provider/openrouter.rs`) — likely obsolete or moved.

How to verify: Confirm in crates/roko-agent/src/provider/openrouter.rs whether still true: OpenRouter creates its own `reqwest::Client` instead of using shared pool
