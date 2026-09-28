+++
id = "find-fc8be2"
kind = "finding"
title = "[tool T026] ToolContext::testing() constructor not #[cfg(test)]-gated (defense-in-depth hardening)"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-core/tool"]
created = 2026-09-14
updated = 2026-09-28
source = "tmp/archive/tool-audit-2026-09-21/10-FINDINGS-REGISTER.md#register"
discovered_from = "audit:tmp/archive/tool-audit-2026-09-21/10-FINDINGS-REGISTER.md#register"
anchors = ["crates/roko-core/src/tool/handler.rs", "crates/roko-agent/src/tool_loop/context_factory.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
T026 resolved: production uses ToolContext::production()/ToolExecutionContextFactory; all testing() call sites are test-only. Audit lists optional hardening: gate the public testing() constructor behind cfg(test) so production code cannot regress.

Imported without verification from:
- `tmp/archive/tool-audit-2026-09-21/10-FINDINGS-REGISTER.md#register`
- `tmp/archive/tool-audit-2026-09-21/11-IMPLEMENTATION-CHECKLIST.md#release-gate`
- `tmp/archive/tool-audit-2026-09-21/00-INDEX.md#status-note-2026-09-14`

How to verify: grep for ToolContext::testing( outside #[cfg(test)]/tests/benches and check whether the constructor is cfg(test)-gated.
