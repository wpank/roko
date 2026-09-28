+++
id = "spec-704c28"
kind = "spec"
title = "ACP v2 Spec Compliance, Refactor, and Best-in-Class Editor UX"
status = "open"
triage = "verified"
severity = "p1"
subsystem = ["roko-acp"]
created = 2026-09-21
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/backlog/archive/18-acp-spec-upgrade-and-refactor.md#18 — ACP v2 Spec Compliance, Refactor, and Best-in-Class Editor UX"
discovered_from = "audit:tmp/backlog/archive/18-acp-spec-upgrade-and-refactor.md#18 — ACP v2 Spec Compliance, Refactor, and Best-in-Class Editor UX"
anchors = ["crates/roko-acp/src/types.rs:10", "crates/roko-acp/src/handler.rs"]
links = { depends_on = [], blocks = [], related = ["gap-83d081", "find-b2ae71"], supersedes = [], duplicate_of = "" }
+++
spec compliance, editor compatibility, UX expressiveness. Roko includes `roko-acp`, a 19,915-line crate implementing the Agent Client Protocol (ACP) — the JSON-RPC protocol used by Zed, JetBrains IDEs, Cursor, Neovim, and Devin Desktop to interact with AI agents. When an editor wants roko to…

Imported without verification from:
- `tmp/backlog/archive/18-acp-spec-upgrade-and-refactor.md#18 — ACP v2 Spec Compliance, Refactor, and Best-in-Class Editor UX`
- `tmp/archive/CONSOLIDATED-BACKLOG-2026-09-23.md#P1-ACP-1 (Subsystem: ACP)`

Some cited files are gone: `session/list`, `session/new`.

How to verify: Check: `ACP_SPEC_VERSION` is `"2.0.0-draft"` in `types.rs`; `session/delete` handler wired; removes from active map and disk; `logout` handler wired [evidence: CONSOLIDATED P1-ACP-1: open; 00-STATUS-SUMMARY 3. Open / P1 -- High (Open): XL | 6 |]

Verified 2026-09-28: still open. Changed p0 -> p1 to match its CONSOLIDATED P1-ACP-1 source; a spec upgrade is not a broken core loop. `ACP_SPEC_VERSION` is still 0.12.2 (crates/roko-acp/src/types.rs:10). The handler serves initialize and session/new, load, list, resume, close, prompt, cancel, set_mode, set_config_option and config/update, but not `session/delete`, `logout` or `authenticate`. The narrower tracker is gap-83d081.
