+++
id = "q-0419ab"
kind = "question"
title = "DF-0822 fix-12: Permission bypass allowed at SandboxLevel::Restrict for CLI agents"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-agent/safety"]
created = 2026-08-22
updated = 2026-09-28
source = "tmp/archive/dogfood-2026-08-22/DOGFOOD-DEBRIEF.md#Fix 12: Sandbox permission bypass for Restrict level"
discovered_from = "audit:tmp/archive/dogfood-2026-08-22/DOGFOOD-DEBRIEF.md#Fix 12: Sandbox permission bypass for Restrict level"
anchors = ["crates/roko-agent/src/safety/sandbox.rs allows_permission_bypass", "SandboxLevel::Restrict"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
To unblock unattended runs, allows_permission_bypass() returns true for Restrict, passing --dangerously-skip-permissions to Claude CLI; Restrict path/network policy relies on SandboxPolicy::check_call, which may not see provider-internal tools. Confirm intended.

Imported without verification from:
- `tmp/archive/dogfood-2026-08-22/DOGFOOD-DEBRIEF.md#Fix 12: Sandbox permission bypass for Restrict level`

How to verify: Read allows_permission_bypass and which checks apply to CLI-internal tool calls.
