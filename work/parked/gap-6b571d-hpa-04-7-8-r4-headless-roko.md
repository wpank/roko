+++
id = "gap-6b571d"
kind = "gap"
title = "HPA-04 §7.8/R4: Headless `roko plan run` is invisible to a running `roko serve`"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/state_hub"]
created = 2026-09-04
updated = 2026-09-28
source = "tmp/hermes-product-audit/04-roko-client-server-gaps.md#7.8. Plan execution initiated by CLI (`roko plan run`) is invisible to the serve SSE stream when no serve is running"
discovered_from = "audit:tmp/hermes-product-audit/04-roko-client-server-gaps.md#7.8. Plan execution initiated by CLI (`roko plan run`) is invisible to the serve SSE stream when no serve is running"
anchors = ["crates/roko-cli/src/state_hub_ipc.rs", "crates/roko-serve/src/routes/event_ingest.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
CLI plan runs publish to an in-process StateHub only; with serve running there is no bridge (proposal: --serve-url posting to /api/events/ingest or local IPC). state_hub_ipc.rs was first compiled 2026-09-26 for portal plan 03, so this is in progress.

Imported without verification from:
- `tmp/hermes-product-audit/04-roko-client-server-gaps.md#7.8. Plan execution initiated by CLI (`roko plan run`) is invisible to the serve SSE stream when no serve is running`
- `tmp/hermes-product-audit/04-roko-client-server-gaps.md#R4 (Medium Priority): Bridge headless `roko plan run` events to serve`
- `tmp/dogfood/2026-09-25-portal-programme-run.md#R-3. `state_hub_ipc.rs` was never compiled by anything`

How to verify: Run plan run with serve up; check events appear on /api/events.
