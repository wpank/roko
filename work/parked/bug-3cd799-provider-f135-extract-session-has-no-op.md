+++
id = "bug-3cd799"
kind = "bug"
title = "[provider F135] extract_session has no-op default causing silent session data loss"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-agent/tool_loop"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F135"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F135"
anchors = ["crates/roko-agent/src/tool_loop/mod.rs", "extract_session"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
The default `extract_session` implementation in the `LlmBackend` trait returns `None`. Providers that don't override this silently discard session IDs from responses.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F135`
- `tmp/archive/provider-audit/14-tool-loop.md`

How to verify: Confirm in crates/roko-agent/src/tool_loop/mod.rs whether still true: `extract_session` has no-op default causing silent session data loss
