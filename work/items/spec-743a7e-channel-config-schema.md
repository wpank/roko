+++
id = "spec-743a7e"
kind = "spec"
title = "Channel Config Schema"
status = "open"
triage = "verified"
severity = "p1"
goal = "features"
subsystem = ["roko-core/config"]
created = 2026-09-21
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/backlog/archive/426-channel-config-schema.md#426 — Channel Config Schema"
discovered_from = "audit:tmp/backlog/archive/426-channel-config-schema.md#426 — Channel Config Schema"
anchors = ["crates/roko-core/src/config/channels.rs::ChannelConfig", "crates/roko-core/src/config/mod.rs", "crates/roko-core/src/config/schema.rs::RokoConfig"]
links = { depends_on = [], blocks = [], related = ["gap-8d80d3", "spec-8bc165"], supersedes = [], duplicate_of = "" }
+++
foundation for all channel/webhook composability; every channel adapter and cross-platform feature depends on this. Roko has per-platform config scattered across backlog items (#224, #225) and the existing `PlatformConfig` / `PlatformKind` concepts in #224, but there is no committed `[channels]`…

Imported without verification from:
- `tmp/backlog/archive/426-channel-config-schema.md#426 — Channel Config Schema`

Some cited files are gone: `crates/roko-cli/src/commands/config.rs`, `crates/roko-core/src/config/channel.rs`.

How to verify: Check: `ChannelKind` enum with 7 variants exists in `roko-core`; `ChannelConfig` struct with `name`, `kind`, `enabled`, `access_control`, and; `RokoConfig.channels` field is a `HashMap<String, ChannelConfig>` [evidence: 00-INDEX (2026-09-21) listed active: 2026-09-21 Channel/Webhook/Reactive Audit (#409-#4]

Verified 2026-09-28: commit 9c6ec420c added crates/roko-core/src/config/channels.rs (a `[[channels]]` table; ChannelConfig {platform_id, channel_id, agent_name, trigger_bindings}) and config/platforms.rs. Neither is declared in config/mod.rs, so both are orphan and uncompiled. RokoConfig (config/schema.rs, which has uncommitted concurrent edits) has no channels field, and the spec's ChannelKind (7 variants) and HashMap<String, ChannelConfig> shape is not implemented. The import's 'gone' warning for config/channel.rs is only a path difference (channels.rs).
