+++
id = "spec-b3ab28"
kind = "spec"
title = "Telegram Bot Integration"
status = "open"
triage = "verified"
severity = "p2"
goal = "features"
subsystem = ["roko-runtime/platforms"]
created = 2026-09-07
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "tmp/backlog/archive/225-telegram-bot-integration.md#225 — Telegram Bot Integration"
discovered_from = "audit:tmp/backlog/archive/225-telegram-bot-integration.md#225 — Telegram Bot Integration"
anchors = ["crates/roko-runtime/src/adapters/telegram.rs", "crates/roko-runtime/src/platforms.rs::ChatBridge", "crates/roko-core/src/config/platforms.rs"]
links = { depends_on = [], blocks = [], related = ["gap-63055e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qE '^pub mod adapters' crates/roko-runtime/src/lib.rs && test -f crates/roko-runtime/src/adapters/telegram.rs && ! grep -q 'telegram-stub-' crates/roko-runtime/src/adapters/telegram.rs"
+++
Telegram is the most accessible way to interact with roko remotely; mobile + desktop + web clients with zero setup for end users. Telegram bots are the simplest way to make an AI agent accessible from anywhere — phone, tablet, desktop, any OS. Hermes supports Telegram as a first-class platform…

Imported without verification from:
- `tmp/backlog/archive/225-telegram-bot-integration.md#225 — Telegram Bot Integration`

Some cited files are gone: `.roko/state/telegram-sessions.json`, `crates/roko-mcp-telegram/`, `crates/roko-mcp-telegram/src/access.rs`, `crates/roko-mcp-telegram/src/client.rs`.

How to verify: Check: `roko-mcp-telegram` crate builds and exposes 10 MCP tools; Bot responds to `/start`, `/status`, `/help`, `/plan`, `/doctor`, `/cost` commands; Free-text messages dispatch to agent and responses are sent back to the chat [evidence: 00-STATUS-SUMMARY 3. Open / P1 -- High (Open): L | 3 |]

Verified 2026-09-28: still true. There is no roko-mcp-telegram crate; the codebase uses a ChatBridge adapter instead, and crates/roko-runtime/src/adapters/telegram.rs (added in 9c6ec420c) is a stub. It makes no Bot API calls, send_message returns `telegram-stub-...` IDs (:94-97), there is no /start, /status or other command handling, and nothing in serve or the CLI starts it. Subsystem corrected to roko-runtime/platforms. This is a child of the umbrella gap-63055e. Severity lowered p1 to p2 because this is a new product surface.

Re-verified 2026-09-29: still a stub. The adapters/ module and the ChatBridge trait it implements (platforms.rs) are not declared in crates/roko-runtime/src/lib.rs, so telegram.rs is not compiled at all. Wiring those modules (umbrella gap-63055e) is a prerequisite.
