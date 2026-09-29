+++
id = "gap-057e79"
kind = "gap"
title = "Screenshot Command Completion (Flags, Symlink, Sub-Views)"
status = "open"
triage = "verified"
severity = "p3"
goal = "features"
subsystem = ["roko-cli/commands"]
created = 2026-09-07
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "tmp/backlog/archive/111-screenshot-command-completion.md#111 — Screenshot Command Completion (Flags, Symlink, Sub-Views)"
discovered_from = "audit:tmp/backlog/archive/111-screenshot-command-completion.md#111 — Screenshot Command Completion (Flags, Symlink, Sub-Views)"
anchors = ["crates/roko-cli/src/commands/screenshot.rs::ScreenshotArgs", "crates/roko-cli/src/tui/screenshot_diff.rs::compare", "crates/roko-cli/src/commands/dashboard.rs::cmd_dashboard_snapshot"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'pub compare' crates/roko-cli/src/commands/screenshot.rs && grep -q 'pub format' crates/roko-cli/src/commands/screenshot.rs && grep -q 'screenshot_diff::compare' crates/roko-cli/src/commands/screenshot.rs"
+++
Self-assessment of TUI output currently requires a real terminal; the screenshot command skeleton exists but critical flags and output modes are unimplemented.. Claude and other automated agents need a way to inspect the TUI's rendered output without an attached terminal. The `roko screenshot`…

Imported without verification from:
- `tmp/backlog/archive/111-screenshot-command-completion.md#111 — Screenshot Command Completion (Flags, Symlink, Sub-Views)`
- `tmp/backlog/_archive/_checklist-gaps.md#§0.2`
- `tmp/backlog/_archive/_mori-old-gaps.md#MO-01`

Some cited files are gone: `.roko/screenshots/2026-08-19T14-23-00/`.

How to verify: Check: `roko screenshot` produces a timestamped directory with one `.txt` file per tab (ten tabs minimum).; `.roko/screenshots/latest/` symlink is created and points to the most recent capture.; `roko screenshot --pages dashboard,plans` produces… [evidence: 00-STATUS-SUMMARY 3. Open / P1 -- High (Open): S | 3 |]

Verified 2026-09-28: Mostly done: commands/screenshot.rs has --dir/--tabs/--width/--height/--label, writes manifest.json, maintains the .roko/screenshots/latest symlink (update_latest_link), and `roko dashboard --snapshot` exists (cmd_dashboard_snapshot). Still missing: `--compare <ref-dir>` producing diff-report.json (tui/screenshot_diff.rs::compare is not exposed by the command) and `--format ansi`. Severity lowered p1 -> p3: only polish flags on a dev tool remain.

Re-verified 2026-09-29: unchanged. Two things remain: `--compare <ref-dir>` writing diff-report.json through tui/screenshot_diff.rs::compare, and `--format ansi`.
