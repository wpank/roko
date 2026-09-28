+++
id = "spec-34ba5c"
kind = "spec"
title = "Mattermost Adapter"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-channel-mattermost"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/backlog/archive/413-mattermost-adapter.md#413 — Mattermost Adapter"
discovered_from = "audit:tmp/backlog/archive/413-mattermost-adapter.md#413 — Mattermost Adapter"
anchors = ["roko-core/src/connector.rs", "roko-serve/src/routes/webhooks.rs", "api.rs", "websocket.rs", "access.rs", "commands.rs", "format.rs", "transport.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Mattermost is the leading self-hosted Slack alternative; common in enterprises and government organizations with data-sovereignty requirements. Mattermost is an open-source, self-hosted team collaboration platform with an API surface closely modeled on Slack. It provides a WebSocket real-time API…

Imported without verification from:
- `tmp/backlog/archive/413-mattermost-adapter.md#413 — Mattermost Adapter`

How to verify: Check: `roko-channel-mattermost` crate builds and implements `PlatformTransport` from #224; WebSocket connects and receives real-time `posted` events; Bot commands (`!roko status`, `!roko plan`, `!roko doctor`) return markdown-formatted responses [evidence: 00-INDEX (2026-09-21) listed active: 2026-09-21 Channel/Webhook/Reactive Audit (#409-#4]
