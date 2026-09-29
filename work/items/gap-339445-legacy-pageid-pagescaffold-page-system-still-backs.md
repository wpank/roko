+++
id = "gap-339445"
kind = "gap"
title = "Legacy PageId/PageScaffold page system still backs text-mode and serve paths"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
subsystem = ["roko-cli/tui"]
created = 2026-08-31
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "gaps-md#partial-10/122"
anchors = ["crates/roko-cli/src/tui/pages/mod.rs::PageId", "crates/roko-cli/src/tui/pages/mod.rs::PageScaffold", "crates/roko-cli/src/commands/dashboard.rs::render_page_text", "crates/roko-cli/src/tui/app/mod.rs::tab_to_page"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -rqwE 'PageId|PageScaffold' crates/roko-cli/src --include='*.rs'"
+++

UX parity item #122 (legacy page removal) is blocked. `PageScaffold`/`PageId` (`crates/roko-cli/src/tui/pages/mod.rs`) are still used in 7 files under `crates/roko-cli/src`, because the page system backs text-mode output and serve paths.

Fix: move the text-mode and serve consumers onto the ratatui view models, then delete the page system.

Re-verified 2026-09-29: still open. PageId/PageScaffold are referenced from 9 roko-cli files outside tui/pages/ (8 excluding tests): the text-mode dashboard (commands/dashboard.rs::render_page_text), the App's legacy page state (tui/app/mod.rs current_page, scroll_offset, PageRegistry, tab_to_page), dashboard.rs, dashboard_model.rs, tui/mod.rs, main.rs and the lib.rs re-export. No roko-serve file uses them, so the 'serve paths' part of the title is not borne out.
