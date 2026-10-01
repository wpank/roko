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
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "ebdc0f5d5"
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

## Notes

- 2026-10-01 (wk-tuiv): implemented on work/bug-6c11d1; cargo verification deferred to the batch check.
  `roko screenshot --format ansi` writes `<tab>.ansi` (the tab's buffer through `screenshot_diff::buffer_to_ansi`)
  beside each `<tab>.txt` and names it in the manifest (`ansi_file`). `--compare <REF_DIR>` compares each captured
  `.txt` with the same-named file in REF_DIR through `screenshot_diff::compare` and writes `diff-report.json`
  (per tab: whether the reference has it, differing cells, changed line numbers, regions); it exits 1 when any
  tab differs or is new. The reference path is resolved before capturing, so `--compare .roko/screenshots/latest`
  compares with the previous run rather than with the new one. To build this, `tui/screenshot_diff.rs` is no
  longer behind the `tui-png` feature (only `png_renderer` is), its one range loop clippy would flag became an
  iterator, and `App::render_tabs_to_buffers` keeps each tab's buffer (`render_tabs_to_text` now maps them
  through `screenshot_diff::buffer_to_text`, with unchanged output). Tests: `capture_writes_ansi_beside_text_when_asked`
  (lib), `screenshot_compare_reports_each_tab_against_the_reference` (bin), plus the module's existing tests,
  which now run without the feature.
- The gate needs `cargo test -p roko-cli --lib screenshot_diff` and clippy on the ungated module: it was compiled
  only with `--features tui-png` until now. The original spec's unified per-file text diff is not produced; the
  report carries cell counts, line numbers and regions instead.
