+++
id = "gap-2d26e8"
kind = "gap"
title = "Per-Platform Agent Identity"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-core/config"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/backlog/archive/427-per-platform-agent-identity.md#427 — Per-Platform Agent Identity"
discovered_from = "audit:tmp/backlog/archive/427-per-platform-agent-identity.md#427 — Per-Platform Agent Identity"
anchors = ["crates/roko-core/src/config/schema.rs", "crates/roko-core/src/config/agent.rs", "crates/roko-agent/src/lifecycle.rs", "roko config secrets set/get/list/rotate", "crates/roko-core/src/config/channel.rs", "crates/roko-core/src/config/validation.rs", "crates/roko-agent/src/dispatcher/mod.rs", "crates/roko-cli/src/commands/config.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
agents need platform-specific bot credentials and identity resolution before channels can route inbound messages. When roko exposes agents on messaging platforms, each agent needs a platform-specific identity. A "support-bot" agent might be `@roko_support_bot` on Telegram and `Roko Support#1234`…

Imported without verification from:
- `tmp/backlog/archive/427-per-platform-agent-identity.md#427 — Per-Platform Agent Identity`

Some cited files are gone: `crates/roko-cli/src/commands/config.rs`, `crates/roko-core/src/config/channel.rs`.

How to verify: Check: `AgentDefinition.channels` field round-trips through TOML; `ChannelIdentityResolver::resolve()` returns correct agent for (channel, token, chat_id); Two agents claiming the same (channel, chat_id) produces a validation error [evidence: 00-INDEX (2026-09-21) listed active: 2026-09-21 Channel/Webhook/Reactive Audit (#409-#4]
