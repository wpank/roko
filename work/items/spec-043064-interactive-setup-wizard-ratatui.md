+++
id = "spec-043064"
kind = "spec"
title = "Interactive Setup Wizard (ratatui)"
status = "open"
triage = "verified"
severity = "p2"
goal = "features"
subsystem = ["roko-cli/tui"]
created = 2026-09-21
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/backlog/archive/223-setup-wizard-tui.md#223 — Interactive Setup Wizard (ratatui)"
discovered_from = "audit:tmp/backlog/archive/223-setup-wizard-tui.md#223 — Interactive Setup Wizard (ratatui)"
anchors = ["crates/roko-cli/src/commands/setup.rs::cmd_setup_interactive"]
links = { depends_on = [], blocks = [], related = ["gap-f57fe5", "gap-db648b"], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'grep -q ratatui crates/roko-cli/src/commands/setup.rs'
+++
first-run experience and ongoing config management; the single biggest onboarding gap vs Hermes. Hermes has a polished multi-step setup wizard that walks operators through:

Imported without verification from:
- `tmp/backlog/archive/223-setup-wizard-tui.md#223 — Interactive Setup Wizard (ratatui)`
- `tmp/archive/CONSOLIDATED-BACKLOG-2026-09-23.md#UXP-12 (UX/TUI Parity: Partial Items (13 items f)`
- `tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#6.13 Setup wizard TUI conversion`
- `tmp/backlog/archive/425-channel-setup-wizard.md#425 — Interactive CLI Channel Setup Wizard`

Some cited files are gone: `crates/roko-cli/src/commands/channels.rs`, `crates/roko-cli/src/commands/config.rs`, `crates/roko-cli/src/setup/`, `roko-cli/src/tui/`.

How to verify: Check: `roko setup` launches a full-screen ratatui wizard; Phase 1 (Providers): shows catalog, auto-detects API keys, checkbox toggle, "Custom..." entry; Phase 2 (Models): tabular cost comparison, sortable columns, capability badges, search/filter [evidence: CONSOLIDATED UXP-12: stdin wizard, not ratatui-based per spec; 00-STATUS-SUMMARY 3. Open / P1 -- High (Open): XL | 3 |] / Check: `roko setup channels` launches interactive platform selection; `roko setup channels --platform telegram` goes directly to Telegram flow; Telegram: token validate, allowed users, notification chats, events, mode, test, save [evidence: 00-INDEX (2026-09-21) listed active: 2026-09-21 Channel/Webhook/Reactive Audit (#409-#4]

Merged 2 mined candidates: m1-060, m1-146.

Verified 2026-09-28: still unimplemented - `roko setup` (commands/setup.rs:38, cmd_setup_interactive at :140) is a stdin line-prompt wizard (setup.rs:221-222) with no ratatui usage. Severity lowered p1 -> p2: a working stdin wizard exists; gap-f57fe5 / gap-db648b track the same gap at p3.
