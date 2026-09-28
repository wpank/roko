+++
id = "bug-8229e9"
kind = "bug"
title = "[provider F167] Claude CLI MCP tools silently dropped from --tools allowlist"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-agent/translate"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F167"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F167"
anchors = ["crates/roko-agent/src/translate/claude.rs", "--tools"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
MCP tool names like `roko-mcp-github.list_prs` have no PascalCase Claude alias and are silently absent from the `--tools` CSV passed to `claude`. The model's allowlist will not include these tools even if the MCP server is running.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F167`
- `tmp/archive/provider-audit/21-mcp-tools.md`

How to verify: Confirm in crates/roko-agent/src/translate/claude.rs whether still true: Claude CLI MCP tools silently dropped from `--tools` allowlist
