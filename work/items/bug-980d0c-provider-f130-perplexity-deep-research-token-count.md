+++
id = "bug-980d0c"
kind = "bug"
title = "[provider F130] Perplexity deep research token count cast to u32 — silent truncation"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-agent/provider"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F130"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F130"
anchors = ["crates/roko-agent/src/provider/perplexity.rs", "u32"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Deep research token counts (which can be very large) are cast to `u32` (`as u32`). Counts exceeding 4,294,967,295 silently wrap.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F130`
- `tmp/archive/provider-audit/06-perplexity-cerebras.md`

Warning: every file this item cites is gone (`crates/roko-agent/src/provider/perplexity.rs`) — likely obsolete or moved.

How to verify: Confirm in crates/roko-agent/src/provider/perplexity.rs whether still true: Perplexity deep research token count cast to `u32` — silent truncation
