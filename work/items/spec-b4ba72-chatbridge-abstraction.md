+++
id = "spec-b4ba72"
kind = "spec"
title = "ChatBridge Abstraction"
status = "open"
triage = "verified"
severity = "p2"
goal = "features"
hold = "Set aside per tldr/05 §3; Will chose hold over park on 2026-09-29 (dec-e70592). Remove this line to revive."
subsystem = ["roko-cli"]
created = 2026-09-21
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "tmp/backlog/archive/414-chat-bridge-abstraction.md#414 — ChatBridge Abstraction"
discovered_from = "audit:tmp/backlog/archive/414-chat-bridge-abstraction.md#414 — ChatBridge Abstraction"
anchors = ["crates/roko-runtime/src/platforms.rs::ChatBridge", "crates/roko-cli/src/chat.rs", "crates/roko-cli/src/chat_session.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rq 'impl ChatBridge for' crates/roko-cli/src && grep -rq 'impl ChatBridge for' crates/roko-serve/src"
+++
agents have no channel-neutral conversation interface; every transport (CLI, portal WebSocket, Telegram, Discord) requires bespoke dispatch wiring. Roko has multiple conversation surfaces that each implement their own message plumbing. `roko chat` (direct CLI REPL in `crates/roko-cli/src/chat.rs`)…

Imported without verification from:
- `tmp/backlog/archive/414-chat-bridge-abstraction.md#414 — ChatBridge Abstraction`

How to verify: Check: `ChatMessage` and `ChatBridge` trait defined in `roko-core`; `TerminalChatBridge` wraps existing CLI chat with no behavior change; `PortalChatBridge` wraps WebSocket connections in `roko-serve` [evidence: 00-INDEX (2026-09-21) listed active: 2026-09-21 Channel/Webhook/Reactive Audit (#409-#4]

Verified 2026-09-28: partially implemented - a ChatBridge trait exists in roko-runtime (platforms.rs:268, not roko-core as specified) with Telegram/WhatsApp/Discord/Matrix/Mattermost/Slack adapters (roko-runtime/src/adapters/*), but no terminal (`roko chat`, chat.rs) or portal WebSocket (roko-serve) bridge implements it. Severity lowered p1 -> p2: architecture refactor with no user-facing breakage.

Re-verified 2026-09-29: unchanged. The trait is at roko-runtime/src/platforms.rs:289. The terminal (roko chat) and portal WebSocket bridges are still missing, and roko-cli and roko-serve do not reference ChatBridge.
