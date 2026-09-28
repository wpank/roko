+++
id = "bug-d586e4"
kind = "bug"
title = "[provider F113] render_claude_tool_policy silently drops unrecognized tools"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-agent/translate"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F113"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F113"
anchors = ["crates/roko-agent/src/translate/claude.rs", "render_claude_tool_policy"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Tools without a known Claude alias are dropped silently from the `--tools` flag. No warning or error indicates which tools were not forwarded to the subprocess.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F113`
- `tmp/archive/provider-audit/01-provider-adapters.md`

How to verify: Confirm in crates/roko-agent/src/translate/claude.rs whether still true: `render_claude_tool_policy` silently drops unrecognized tools
