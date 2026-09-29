+++
id = "find-b0b48c"
kind = "finding"
title = "S3: Confirmation Theater — Commands That Lie"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/chat"]
created = 2026-04-28
updated = 2026-09-28
source = "tmp/archive/08-15-26/binary-issues/MASTER-INDEX.md#S3. Confirmation Theater — Commands That Lie"
discovered_from = "audit:tmp/archive/08-15-26/binary-issues/MASTER-INDEX.md#S3. Confirmation Theater — Commands That Lie"
anchors = ["chat_inline.rs:2134", "chat_inline.rs:2245-2273", "chat_inline.rs:2304-2311", "chat_inline.rs:2449-2468", "chat_inline.rs:2715-2821", "learn.rs:84", "Demo.tsx:21,252", "useServerHealth.ts:22-29"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Systemic audit finding (2026-04-28) with 8 open checklist fixes: S3.1 Wire `/system` into dispatch; S3.2 Wire `/effort` into dispatch (map to API ef; S3.3 Wire `/gate` to modify runtime config; S3.4 Wire `/config set` to modify runtime config; S3.5 Execute `/run`, `/plan run`, `/prd idea`, `; S3.6…

Imported without verification from:
- `tmp/archive/08-15-26/binary-issues/MASTER-INDEX.md#S3. Confirmation Theater — Commands That Lie`

How to verify: Check each open sub-item (S3.1, S3.2, S3.3, S3.4, S3.5, S3.6, S3.7, S3.8). Check chat slash-command handlers (/system,/effort,/gate,/config set,/run); `learn tune` is now a deprecated alias of `learn inspect`.
