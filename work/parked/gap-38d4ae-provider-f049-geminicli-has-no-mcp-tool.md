+++
id = "gap-38d4ae"
kind = "gap"
title = "[provider F049] GeminiCli has no MCP tool passthrough"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-agent/provider"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F049"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F049"
anchors = ["crates/roko-agent/src/provider/gemini_cli.rs", "GeminiCli"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
`GeminiCli` does not accept `--mcp-config` or equivalent. MCP tool servers configured for a session are not forwarded to the Gemini CLI subprocess. The Gemini CLI cannot use dynamically-discovered MCP tools.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F049`
- `tmp/archive/provider-audit/03-cli-subprocess.md`

How to verify: CLAUDE.md claims native authenticated Gemini CLI MCP; check GeminiCli MCP passthrough. Confirm in crates/roko-agent/src/provider/gemini_cli.rs whether still true: `GeminiCli` has no MCP tool passthrough
