+++
id = "find-1cbe12"
kind = "finding"
title = "[provider F065] Perplexity pplx_meta tag is opaque string; citations not surfaced structurally"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-agent/provider"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F065"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F065"
anchors = ["crates/roko-agent/src/provider/perplexity.rs", "pplx_meta"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Perplexity API responses include citation metadata in `pplx_meta`. This is treated as an opaque string rather than parsed into structured citation objects. Search result citations are not surfaced to the agent or stored in the episode.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F065`
- `tmp/archive/provider-audit/06-perplexity-cerebras.md`

Warning: every file this item cites is gone (`crates/roko-agent/src/provider/perplexity.rs`) — likely obsolete or moved.

How to verify: Confirm in crates/roko-agent/src/provider/perplexity.rs whether still true: Perplexity `pplx_meta` tag is opaque string; citations not surfaced structurally
