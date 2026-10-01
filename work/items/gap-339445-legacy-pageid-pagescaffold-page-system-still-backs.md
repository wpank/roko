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
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "ebdc0f5d5"
source = "gaps-md#partial-10/122"
anchors = ["crates/roko-cli/src/tui/pages/mod.rs::PageId", "crates/roko-cli/src/tui/pages/mod.rs::PageScaffold", "crates/roko-cli/src/commands/dashboard.rs::render_page_text", "crates/roko-cli/src/tui/app/mod.rs::tab_to_page"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -rqwE 'PageId|PageScaffold' crates/roko-cli/src --include='*.rs'"
+++

UX parity item #122 (legacy page removal) is blocked. `PageScaffold`/`PageId` (`crates/roko-cli/src/tui/pages/mod.rs`) are still used in 7 files under `crates/roko-cli/src`, because the page system backs text-mode output and serve paths.

Fix: move the text-mode and serve consumers onto the ratatui view models, then delete the page system.

Re-verified 2026-09-29: still open. PageId/PageScaffold are referenced from 9 roko-cli files outside tui/pages/ (8 excluding tests): the text-mode dashboard (commands/dashboard.rs::render_page_text), the App's legacy page state (tui/app/mod.rs current_page, scroll_offset, PageRegistry, tab_to_page), dashboard.rs, dashboard_model.rs, tui/mod.rs, main.rs and the lib.rs re-export. No roko-serve file uses them, so the 'serve paths' part of the title is not borne out.

## Notes

- 2026-10-01 (wk-tuiv): blocked; no code changed. Re-checked at BASE: 198 `PageId`/`PageScaffold` references in
  12 roko-cli files (tui/dashboard.rs 48, main.rs 38, tui/pages/mod.rs 28, pages/operations.rs 18,
  pages/efficiency.rs 18, tui/dashboard_model.rs 18, tui/app/mod.rs 17, commands/dashboard.rs 6, plus
  dashboard_view.rs, app/tests.rs, tui/mod.rs and the lib.rs re-export). Still none in roko-serve. What the page
  system backs: `roko dashboard --page <slug>`, `--list-pages` and `--text` (main.rs::parse_dashboard_page,
  14 slugs; commands/dashboard.rs::render_page_text), the App's legacy page state (current_page,
  PageRegistry, tab_to_page), `DashboardScaffold` and the `render_*_page(&PageScaffold)` text renderers.
  Blocked on: (1) a product decision about what `--page`/`--list-pages`/`--text` become, since the 14 page slugs
  do not map onto the ratatui tabs one to one; (2) the main.rs split in flight (gap-0d0e81), which moves the
  `Dashboard` command and `parse_dashboard_page`; (3) size: about 1,000 lines of tui/pages plus ~200 call
  sites, which needs a compiler in the loop. Next step, once (1) and (2) are settled: render text mode through
  the ratatui views the way `roko screenshot` / `roko dashboard --snapshot` already capture tabs headlessly,
  map or drop the old slugs, then delete tui/pages and the App's page state.
