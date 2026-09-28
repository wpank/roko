+++
id = "bug-bfb8ce"
kind = "bug"
title = "roko acp default code mode may deny every builtin tool"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-acp/tools"]
created = 2026-09-28
updated = 2026-09-28
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-acp/src/tools.rs:458"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

A local static trace found that the builtin-tool contract gate looks up the session mode string `"code"` as a role; no bundled contract exists for it, so it falls back to a restricted contract with an empty allow-list and every builtin tool is denied in the default mode.
No test exercises role `"code"`.
Confirm with a conformance test that a builtin tool executes in `code` mode; fix by mapping modes to roles explicitly (e.g. `code` -> implementer).
