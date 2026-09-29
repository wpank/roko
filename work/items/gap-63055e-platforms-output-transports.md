+++
id = "gap-63055e"
kind = "gap"
title = "Platforms & Output Transports"
status = "open"
triage = "verified"
severity = "p2"
goal = "features"
subsystem = ["roko-runtime/platforms"]
created = 2026-09-07
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "tmp/backlog/archive/224-platforms-and-transports.md#224 — Platforms & Output Transports"
discovered_from = "audit:tmp/backlog/archive/224-platforms-and-transports.md#224 — Platforms & Output Transports"
anchors = ["crates/roko-runtime/src/platforms.rs::ChatBridge", "crates/roko-runtime/src/adapters/telegram.rs", "crates/roko-runtime/src/adapters/slack.rs", "crates/roko-runtime/src/adapters/discord.rs", "crates/roko-core/src/config/platforms.rs"]
links = { depends_on = [], blocks = [], related = ["spec-b3ab28"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qE '^pub mod platforms' crates/roko-runtime/src/lib.rs && grep -qE '^pub mod platforms' crates/roko-core/src/config/mod.rs && ! grep -rqE '(telegram|slack|discord)-stub-' crates/roko-runtime/src/adapters && grep -rqE 'PlatformRegistry' crates/roko-serve/src crates/roko-cli/src"
+++
operators have no way to converse with roko through messaging platforms; all interaction requires the local terminal. Hermes supports ~30 messaging platforms (Telegram, Discord, Slack, WhatsApp, LINE, Signal, Matrix, IRC, Nostr, X/Twitter, Reddit, email, SMS, Twitch, etc.) as first-class output…

Imported without verification from:
- `tmp/backlog/archive/224-platforms-and-transports.md#224 — Platforms & Output Transports`

Some cited files are gone: `crates/roko-mcp-slack/`.

How to verify: Check: `[platforms.*]` config section exists in `roko.toml` schema; `PlatformTransport` trait defined with send/listen/health/disconnect; At least 3 platform adapters work end-to-end: Slack (existing MCP), Telegram (new), Discord (new) [evidence: 00-STATUS-SUMMARY 3. Open / P1 -- High (Open): XL | 3 |]

Verified 2026-09-28: the scaffolding exists, but no platform works end to end. Commit 9c6ec420c added [platforms]/[channels] config (crates/roko-core/src/config/platforms.rs, channels.rs), a ChatBridge trait with a platform registry (crates/roko-runtime/src/platforms.rs:289, :442), and adapters for Slack, Telegram, Discord, WhatsApp, Matrix and Mattermost. The adapters are stubs: telegram.rs:1 says so, and send_message returns a fake `telegram-stub-...` ID (:94-97). No roko-serve or roko-cli code constructs or starts an adapter. Subsystem corrected to roko-runtime/platforms, since the roko-mcp-slack crate is gone. Severity lowered p1 to p2 because this is a new product surface, not the core loop.

Re-verified 2026-09-29: still open, and the scaffolding recorded on 2026-09-28 is not compiled. crates/roko-runtime/src/lib.rs has no `mod platforms`, `mod adapters` or `mod channel_binding`, and crates/roko-core/src/config/mod.rs has no `mod platforms` or `mod channels`, so the ChatBridge trait, the six adapters and the [[platforms]] config are dead files that may not build. A fix must first wire these modules into their crates.
