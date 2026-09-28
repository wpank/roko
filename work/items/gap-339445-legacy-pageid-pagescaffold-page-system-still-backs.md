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
updated = 2026-09-28
last_verified = 2026-09-28
source = "gaps-md#partial-10/122"
anchors = ["crates/roko-cli/src/tui/pages/mod.rs:13"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

UX parity item #122 (legacy page removal) is blocked. `PageScaffold`/`PageId` (`crates/roko-cli/src/tui/pages/mod.rs`) are still used in 7 files under `crates/roko-cli/src`, because the page system backs text-mode output and serve paths.

Fix: move the text-mode and serve consumers onto the ratatui view models, then delete the page system.
