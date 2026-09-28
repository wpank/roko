+++
id = "gap-25d390"
kind = "gap"
title = "Codex CLI Provider Kind"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-core"]
created = 2026-09-07
updated = 2026-09-28
source = "tmp/backlog/archive/158-codex-cli-provider-kind.md#158 — Codex CLI Provider Kind"
discovered_from = "audit:tmp/backlog/archive/158-codex-cli-provider-kind.md#158 — Codex CLI Provider Kind"
anchors = ["crates/roko-core/src/agent.rs", "crates/roko-agent/src/provider/claude_cli.rs", "crates/roko-cli/src/dispatch_v2.rs", "crates/roko-agent/src/provider/codex_cli/stream.rs", "crates/roko-agent/src/provider/", "codex_cli/mod.rs", "claude_cli.rs", "crates/roko-agent/src/provider/codex_cli/mod.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
enables Codex as a first-class provider without piggybacking on `claude_cli`. The 2026-08-22 dogfood session proved that Codex CLI (`codex exec --json`) works as an agent provider for `plan run`. However, it currently piggybacks on `kind = "claude_cli"` in roko.toml, with auto-detection logic that…

Imported without verification from:
- `tmp/backlog/archive/158-codex-cli-provider-kind.md#158 — Codex CLI Provider Kind`

How to verify: Check: `kind = "codex_cli"` in roko.toml is recognized and creates a Codex provider.; `kind = "claude_cli"` with `command = "codex"` continues to work (backward compat).; `roko config providers list` shows codex as its own provider kind, not… [evidence: 00-STATUS-SUMMARY 3. Open / P2 -- Medium (Open): M | 7 |]
