+++
id = "spec-2b7fa4"
kind = "spec"
title = "WhatsApp Bridge Adapter"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-channel-whatsapp"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/backlog/archive/412-whatsapp-bridge-adapter.md#412 — WhatsApp Bridge Adapter"
discovered_from = "audit:tmp/backlog/archive/412-whatsapp-bridge-adapter.md#412 — WhatsApp Bridge Adapter"
anchors = ["roko-serve/src/routes/webhooks.rs", "roko-core/src/connector.rs", "cloud_api.rs", "bridge.rs", ".roko/state/whatsapp-session/", "access.rs", "media.rs", "image/jpeg"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
WhatsApp has 2B+ users and is the dominant messaging platform globally; important for mobile-first operators. WhatsApp integration supports two modes:

Imported without verification from:
- `tmp/backlog/archive/412-whatsapp-bridge-adapter.md#412 — WhatsApp Bridge Adapter`

Some cited files are gone: `.roko/state/whatsapp-session/`, `image/jpeg`.

How to verify: Check: `roko-channel-whatsapp` crate builds and implements `PlatformTransport` from #224; Cloud API mode: outbound text and image messages send via Graph API; Cloud API mode: webhook receives inbound messages and dispatches to agent [evidence: 00-INDEX (2026-09-21) listed active: 2026-09-21 Channel/Webhook/Reactive Audit (#409-#4]
