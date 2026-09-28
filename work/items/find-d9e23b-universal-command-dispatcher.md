+++
id = "find-d9e23b"
kind = "finding"
title = "Universal Command Dispatcher"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/commands"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/backlog/archive/429-universal-command-dispatcher.md#429 — Universal Command Dispatcher"
discovered_from = "audit:tmp/backlog/archive/429-universal-command-dispatcher.md#429 — Universal Command Dispatcher"
anchors = ["crates/roko-cli/src/commands/", "crates/roko-runtime/src/event_bus.rs", "crates/roko-core/src/command.rs", "roko channels commands list/sync", "crates/roko-core/src/lib.rs", "crates/roko-cli/src/commands/channels.rs", "crates/roko-cli/src/commands/mod.rs", "crates/roko-serve/src/routes/commands.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
eliminates per-platform command duplication; define once, expose everywhere. Each messaging platform has its own command registration mechanism: Telegram uses `setMyCommands`, Discord uses slash commands, Slack uses slash commands in the app manifest. Without a shared abstraction, every adapter…

Imported without verification from:
- `tmp/backlog/archive/429-universal-command-dispatcher.md#429 — Universal Command Dispatcher`

Some cited files are gone: `crates/roko-cli/src/commands/channels.rs`, `crates/roko-core/src/command.rs`, `crates/roko-serve/src/routes/commands.rs`.

How to verify: Check: `CommandSpec` and `CommandRegistry` exist in `roko-core`; `CommandRegistry::with_builtins()` returns 10 built-in commands; `parse("/status")` returns correct `ParsedCommand` [evidence: 00-INDEX (2026-09-21) listed active: 2026-09-21 Channel/Webhook/Reactive Audit (#409-#4]
