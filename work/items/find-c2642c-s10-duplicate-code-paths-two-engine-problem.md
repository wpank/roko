+++
id = "find-c2642c"
kind = "finding"
title = "S10: Duplicate Code Paths & Two-Engine Problem"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli"]
created = 2026-04-28
updated = 2026-09-28
source = "tmp/archive/08-15-26/binary-issues/MASTER-INDEX.md#S10. Duplicate Code Paths & Two-Engine Problem"
discovered_from = "audit:tmp/archive/08-15-26/binary-issues/MASTER-INDEX.md#S10. Duplicate Code Paths & Two-Engine Problem"
anchors = ["orchestrate.rs", "chat_inline.rs:1044-1203", "dispatch_direct.rs", "chat_inline.rs:1170-1196", "chat.rs", "chat_inline.rs", "commands/util.rs:98", "config_cmd.rs:49"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Systemic audit finding (2026-04-28) with 6 open checklist fixes: S10.1 Deprecate one engine (migrate features, re; S10.2 Merge the two chat event loops into one; S10.3 Route dispatch_direct through roko-agent's; S10.4 Extract session summary into one function; S10.5 Remove legacy chat.rs; S10.6…

Imported without verification from:
- `tmp/archive/08-15-26/binary-issues/MASTER-INDEX.md#S10. Duplicate Code Paths & Two-Engine Problem`

How to verify: Check each open sub-item (S10.1, S10.2, S10.3, S10.4, S10.5, S10.6). S10.1 likely closed (Graph sole engine #260/#276); S10.3 overlaps CONSOLIDATED P2-RUN-1 (agent dispatch consolidation, open); check legacy chat.rs and init paths.
