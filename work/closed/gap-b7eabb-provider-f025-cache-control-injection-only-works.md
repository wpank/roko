+++
id = "gap-b7eabb"
kind = "gap"
title = "[provider F025] cache_control injection only works with anthropic_api kind (not the default claude_cli)"
status = "superseded"
triage = "verified"
severity = "p1"
subsystem = ["roko-agent/translate"]
created = 2026-09-01
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F025"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F025"
anchors = ["crates/roko-agent/src/translate/claude.rs::inject_cache_markers_into_content", "crates/roko-agent/src/claude_agent.rs:375"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "find-6ee709" }

[closed]
at = 2026-09-28
evidence = "duplicate of find-6ee709 (verified 2026-09-28). Cache markers become cache_control only on the Anthropic API paths (inject_cache_markers_into_content: crates/roko-agent/src/claude_agent.rs:375, translate/claude.rs:147, :182). On every other provider, including the default claude_cli subprocess, they stay inert HTML comments, which is exactly find-6ee709's remaining scope."
+++
The cache marker injection (`inject_cache_markers_into_content`) only runs in `ClaudeAgent` which is the direct HTTP Anthropic API adapter. The default production configuration routes Claude through `claude_cli` (subprocess). The subprocess receives a flat string system prompt; HTML comment marke...

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F025`
- `tmp/archive/provider-audit/19-caching.md`

How to verify: Confirm in crates/roko-agent/src/translate/claude.rs, crates/roko-agent/src/claude_agent.rs whether still true: `cache_control` injection only works with `anthropic_api` kind (not the default `claude_cli`)

Verified 2026-09-28: superseded as a duplicate; see `[closed].evidence`.
