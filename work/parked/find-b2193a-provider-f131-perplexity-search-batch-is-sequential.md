+++
id = "find-b2193a"
kind = "finding"
title = "[provider F131] Perplexity search_batch() is sequential, not parallel"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-agent/provider"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F131"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F131"
anchors = ["crates/roko-agent/src/provider/perplexity.rs", "search_batch()"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Batch search queries in `search_batch()` are executed sequentially with `for await`. Parallel execution via `tokio::join!` or `futures::join_all` would reduce latency for multi-query batches.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F131`
- `tmp/archive/provider-audit/06-perplexity-cerebras.md`

Warning: every file this item cites is gone (`crates/roko-agent/src/provider/perplexity.rs`) — likely obsolete or moved.

How to verify: Confirm in crates/roko-agent/src/provider/perplexity.rs whether still true: Perplexity `search_batch()` is sequential, not parallel
