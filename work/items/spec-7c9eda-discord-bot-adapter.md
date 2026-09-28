+++
id = "spec-7c9eda"
kind = "spec"
title = "Discord Bot Adapter"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-channel-discord"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/backlog/archive/409-discord-bot-adapter.md#409 — Discord Bot Adapter"
discovered_from = "audit:tmp/backlog/archive/409-discord-bot-adapter.md#409 — Discord Bot Adapter"
anchors = ["roko-serve/src/routes/webhooks.rs", "roko-core/src/connector.rs", "gateway.rs", "rest.rs", "access.rs", "commands.rs", "format.rs", "transport.rs", "roko-serve/src/templates.rs", "socket_mode.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Discord is the dominant community platform for developer and crypto teams; many roko operators already have active Discord servers. Discord bots integrate through a WebSocket gateway for real-time events and a REST API (v10) for sending messages, registering slash commands, and querying guild…

Imported without verification from:
- `tmp/backlog/archive/409-discord-bot-adapter.md#409 — Discord Bot Adapter`
- `tmp/backlog/archive/410-slack-bot-adapter.md#410 — Slack Bot Adapter`

How to verify: Check: `roko-channel-discord` crate builds and implements `PlatformTransport` from #224; Gateway WebSocket connects, heartbeats, identifies, and receives events; Slash commands `/status`, `/plan`, `/doctor`, `/cost`, `/providers`, `/help` return… [evidence: 00-INDEX (2026-09-21) listed active: 2026-09-21 Channel/Webhook/Reactive Audit (#409-#4] / Check: `roko-channel-slack` crate builds and implements `PlatformTransport` from #224; Socket Mode connects, receives events, and acknowledges within 3 seconds; Slash commands `/roko-status`, `/roko-plan`, `/roko-doctor`, `/roko-cost` return Block… [evidence: 00-INDEX (2026-09-21) listed active: 2026-09-21 Channel/Webhook/Reactive Audit (#409-#4]

Merged 2 mined candidates: m1-132, m1-133.
