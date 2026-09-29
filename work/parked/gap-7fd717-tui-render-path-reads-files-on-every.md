+++
id = "gap-7fd717"
kind = "gap"
title = "TUI render path reads files on every frame"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/tui"]
created = 2026-09-01
updated = 2026-09-28
source = "gaps-md#from-tuiux-parity-audit-tmptui-parity00-indexmd/rc-4"
anchors = ["crates/roko-cli/src/tui/config_meta.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++

Audit RC-4: three render functions (MCP sub-tab, F7 Inspect, F6 Config) read files on every frame, at 20-60 fps. Backlogs #235 and #387 (removing disk I/O from the render path) are archived without a status. `std::fs::read_to_string` still appears in `crates/roko-cli/src/tui/config_meta.rs`, `effects_config.rs`, `dashboard_gen.rs` and `screenshot_diff.rs`. Whether any of these calls runs per frame has not been re-checked.

Fix: cache file-backed view data and refresh it from the file watcher (`tui/fs_watch.rs`) instead of at render time.
