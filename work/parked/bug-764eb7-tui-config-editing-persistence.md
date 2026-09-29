+++
id = "bug-764eb7"
kind = "bug"
title = "TUI Config Editing Persistence"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/tui"]
created = 2026-09-07
updated = 2026-09-28
source = "tmp/backlog/archive/188-tui-config-editing.md#188 — TUI Config Editing Persistence"
discovered_from = "audit:tmp/backlog/archive/188-tui-config-editing.md#188 — TUI Config Editing Persistence"
anchors = ["crates/roko-cli/src/tui/app.rs:1876", "roko.toml", "crates/roko-core/src/config/loader.rs", "app.rs:4226", "app.rs:4253", "config_meta.rs", "crates/roko-cli/src/tui/app.rs", "crates/roko-cli/src/tui/state.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
config changes made in the TUI are silently discarded; operators expect save to persist. The TUI F6:config tab has a `ConfigSave` action and a `save_config_changes()` method in `app.rs` that writes pending edits to `roko.toml`. However the save path only covers the happy case — there is no…

Imported without verification from:
- `tmp/backlog/archive/188-tui-config-editing.md#188 — TUI Config Editing Persistence`

Some cited files are gone: `crates/roko-cli/src/tui/app.rs`, `crates/roko-cli/src/tui/state.rs`.

How to verify: Check: `ConfigSave` shows a green toast on success and red toast on failure; Ctrl-Z reverts the last save and writes the previous value back to disk; Unknown config keys are rejected with an error message, not silently written [evidence: 00-STATUS-SUMMARY 3. Open / P2 -- Medium (Open): S | 5 |]
