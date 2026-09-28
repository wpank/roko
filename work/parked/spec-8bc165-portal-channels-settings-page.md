+++
id = "spec-8bc165"
kind = "spec"
title = "Portal Channels Settings Page"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-serve/routes"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/backlog/422-portal-channels-settings-page.md#422 — Portal Channels Settings Page"
discovered_from = "audit:tmp/backlog/422-portal-channels-settings-page.md#422 — Portal Channels Settings Page"
anchors = ["apps/portal/", "roko-core/src/connector.rs", "roko-core/src/wire_protocol.rs", "roko-runtime/src/connector_runtime.rs", "roko-serve/src/routes/connectors.rs", "10-SETTINGS.md", "crates/roko-serve/src/routes/channels.rs", "stores/settings.ts", "roko-core/src/trigger.rs", "roko-serve/src/routes/triggers.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Backlog #224 defines `PlatformTransport`, `PlatformConfig`, and `PlatformKind` in `roko-core`. The portal needs a dedicated sub-page under Settings to manage channel lifecycle: create, configure, test, and delete platform connections. Channels are distinct from feeds/triggers — they represent…

Imported without verification from:
- `tmp/backlog/422-portal-channels-settings-page.md#422 — Portal Channels Settings Page`
- `tmp/backlog/423-portal-webhooks-settings-page.md#423 — Portal Webhooks Settings Page`

Some cited files are gone: `apps/portal/src/app/settings/webhooks/page.tsx`, `apps/portal/src/components/settings/WebhookWizard.tsx`, `crates/roko-serve/src/routes/channels.rs`.

How to verify: Check: `/settings/channels` renders with correct sub-nav placement; Roster shows all platforms with live status LEDs; Add/configure/delete channel flows work end-to-end [evidence: 00-INDEX (2026-09-21) listed active: 2026-09-21 Channel/Webhook/Reactive Audit (#409-#4] / Check: `/settings/webhooks` renders with correct sub-nav placement; Roster lists webhook-kind triggers with inbound URL and secret status; Creation wizard generates trigger binding with auth config [evidence: 00-INDEX (2026-09-21) listed active: 2026-09-21 Channel/Webhook/Reactive Audit (#409-#4]

Merged 2 mined candidates: m1-008, m1-009.
