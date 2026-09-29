+++
id = "gap-31f96f"
kind = "gap"
title = "ACP Registry Publication and Editor Compatibility Matrix"
status = "open"
triage = "verified"
severity = "p2"
goal = "hermes"
subsystem = ["roko-acp"]
created = 2026-09-21
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "tmp/backlog/archive/434-acp-registry-publication-editor-compat.md#434 — ACP Registry Publication and Editor Compatibility Matrix"
discovered_from = "audit:tmp/backlog/archive/434-acp-registry-publication-editor-compat.md#434 — ACP Registry Publication and Editor Compatibility Matrix"
anchors = ["crates/roko-acp/src/types.rs:10", "crates/roko-acp/src/handler.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "git ls-files | grep -qiE '(acp[-_/]?registry|registry/agents)/.*roko.*\\.json$|(^|/)agent\\.json$'  (in-repo manifest precondition; closure also needs roko listed in the upstream agentclientprotocol/registry, which is a manual/network check)"
+++
roko is not discoverable in any editor's agent marketplace. The ACP Registry (launched January 2026, co-maintained by Zed Industries and JetBrains) is the universal agent distribution system for code editors. It lists 42+ agents including Claude Agent, Gemini CLI, Codex CLI, GitHub Copilot…

Imported without verification from:
- `tmp/backlog/archive/434-acp-registry-publication-editor-compat.md#434 — ACP Registry Publication and Editor Compatibility Matrix`

Some cited files are gone: `agentclientprotocol/registry`, `agentclientprotocol/registry/agents/roko.json`, `crates/roko-cli/src/commands/acp.rs`, `cursor/*`.

How to verify: Check: Registry manifest file created and submitted as PR; `roko` appears in `agentclientprotocol/registry/agents/`; Zed users can install roko from Agent Settings → External Agents → ACP Registry [evidence: 00-INDEX (2026-09-21) listed active: ACP v2, Editor UX, and MCP Modernization (#18, #39]

Verified 2026-09-28: still true as far as the repo shows. This is distribution rather than core function, so p1 -> p2. `git ls-files` has no ACP registry manifest or agent.json, and `ACP_SPEC_VERSION` is still 0.12.2 (crates/roko-acp/src/types.rs:10). The upstream agentclientprotocol/registry was not checked (no network). The cited crates/roko-cli/src/commands/acp.rs does not exist.
