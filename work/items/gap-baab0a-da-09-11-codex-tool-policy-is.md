+++
id = "gap-baab0a"
kind = "gap"
title = "DA-09/11: Codex tool policy is advisory; Roko-owned operation-level broker missing"
status = "open"
triage = "verified"
severity = "p1"
subsystem = ["roko-agent/codex-cli"]
created = 2026-09-14
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/dev-audit/09-additional-live-run-findings.md#Codex tool policy was advisory, not binding"
discovered_from = "audit:tmp/dev-audit/09-additional-live-run-findings.md#Codex tool policy was advisory, not binding"
anchors = ["crates/roko-cli/src/dispatch_v2.rs::build_codex_invocation", "crates/roko-cli/src/dispatch_v2.rs:607"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Codex CLI has no binding allowlist for built-ins; runs logged 'codex CLI cannot enforce tool policy; proceeding without enforcement' and one attempt made 70 tool calls incl. web search and cross-worktree reads. Strict requests now fail closed, but nothing enforces read/write roots, network or com...

Imported without verification from:
- `tmp/dev-audit/09-additional-live-run-findings.md#Codex tool policy was advisory, not binding`
- `tmp/dev-audit/11-implementation-status.md#Explicit residuals`
- `tmp/dev-audit/08-decisions-needed.md#6. P0 implementation order`
- `tmp/dogfood/2026-09-20-final-session.md#Items Completed This Session`
- `tmp/archive/dogfood-2026-08-22/DOGFOOD-DEBRIEF.md#Fix 4: Tool policy enforcement softened`

A source claims this was fixed; confirm against current code before closing.

How to verify: grep 'cannot enforce tool policy'; confirm build_codex_invocation fails closed for restrictive contracts and whether any broker exists.

Verified 2026-09-28: still true. `build_codex_invocation` (crates/roko-cli/src/dispatch_v2.rs:586) fails closed on allowed/disallowed tools only when `ROKO_REQUIRE_BINDING_TOOL_POLICY` is set (:596-602). Otherwise it logs 'codex CLI cannot enforce tool policy; proceeding without enforcement' (:607) and relies on codex `--sandbox` (:648). No Roko-owned operation-level broker exists for read/write roots, network or commands.
