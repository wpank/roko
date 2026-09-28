+++
id = "spec-700a16"
kind = "spec"
title = "Platform Message Formatters"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-core"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/backlog/archive/430-platform-message-formatters.md#430 — Platform Message Formatters"
discovered_from = "audit:tmp/backlog/archive/430-platform-message-formatters.md#430 — Platform Message Formatters"
anchors = ["crates/roko-core/src/connector.rs", "crates/roko-core/src/wire_protocol.rs", "format.rs", "roko-mcp-slack/src/", "roko-cli/src/tui/", "crates/roko-channels/src/rich_message.rs", "crates/roko-channels/src/formatter.rs", "crates/roko-channels/src/formatters/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
platform adapters (Telegram, Discord, Slack, Matrix) each require different markup; without a shared abstraction every adapter reimplements formatting and truncation logic. Each messaging platform enforces its own markup language: Telegram uses MarkdownV2 (mandatory escaping of `.`, `-`, `(`, `)`…

Imported without verification from:
- `tmp/backlog/archive/430-platform-message-formatters.md#430 — Platform Message Formatters`

Some cited files are gone: `crates/roko-channels/src/formatter.rs`, `crates/roko-channels/src/formatters/`, `crates/roko-channels/src/rich_message.rs`, `roko-cli/src/tui/`, `roko-mcp-slack/src/`.

How to verify: Check: `RichMessage` struct represents headers, body blocks, code, tables, status indicators, and action buttons; `PlatformFormatter` trait defined with `format()`, `max_length()`, `truncation_suffix()`; All five formatters produce valid… [evidence: 00-INDEX (2026-09-21) listed active: 2026-09-21 Channel/Webhook/Reactive Audit (#409-#4]
