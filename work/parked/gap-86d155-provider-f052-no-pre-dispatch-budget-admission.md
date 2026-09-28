+++
id = "gap-86d155"
kind = "gap"
title = "[provider F052] No pre-dispatch budget admission for ACP"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-acp/bridge_events"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F052"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F052"
anchors = ["crates/roko-acp/src/bridge_events.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
ACP does not check remaining budget before dispatching a session. Budget checks fire only after the previous turn's cost is tallied.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F052`
- `tmp/archive/provider-audit/04-acp-integration.md`

Warning: every file this item cites is gone (`crates/roko-acp/src/bridge_events.rs`) — likely obsolete or moved.

How to verify: CLAUDE.md claims ACP persisted/enforced USD budgets; check pre-dispatch admission. Confirm in crates/roko-acp/src/bridge_events.rs whether still true: No pre-dispatch budget admission for ACP
