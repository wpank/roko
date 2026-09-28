+++
id = "gap-1cae8a"
kind = "gap"
title = "[provider F163] Agent isolation saturation at 512 entries — no expiry or rotation"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-agent/immune_evidence"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F163"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F163"
anchors = ["crates/roko-agent/src/immune_evidence.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
The agent control ledger holds at most 512 entries. When saturated, all new agents are denied dispatch. There is no automatic expiry, TTL, or rotation for isolation entries.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F163`
- `tmp/archive/provider-audit/20-safety-screening.md`

How to verify: Confirm in crates/roko-agent/src/immune_evidence.rs whether still true: Agent isolation saturation at 512 entries — no expiry or rotation
