+++
id = "gap-9f37e6"
kind = "gap"
title = "Deterministic Graph Template Routing"
status = "done"
triage = "verified"
severity = "p1"
subsystem = ["roko-graph"]
created = 2026-09-05
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/backlog/archive/278-deterministic-template-routing.md#278 — Deterministic Graph Template Routing"
discovered_from = "audit:tmp/backlog/archive/278-deterministic-template-routing.md#278 — Deterministic Graph Template Routing"
anchors = ["crates/roko-cli/src/commands/do_cmd.rs::resolve_do_route"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-cli do_cmd"

[closed]
at = 2026-09-28
commit = "91b4745f8"
evidence = "Implemented and committed (91b4745f8): crates/roko-cli/src/commands/do_cmd.rs implements the frozen #278 routing table as the pure resolve_do_route (:140). Unqualified TTY prompts get preview+confirm, non-TTY prompts are rejected, a single word matching a plan dir gets a roko plan run hint, and dry-run/ghost/compare only print. Table-driven tests start at do_cmd.rs:1393 (37 #[test] in the file). The import's 'gone' warning for commands/do_cmd.rs was wrong."
+++
[blocked] Blocked —

Imported without verification from:
- `tmp/backlog/archive/278-deterministic-template-routing.md#278 — Deterministic Graph Template Routing`

Some cited files are gone: `tmp/engine-audit/04-silent-delegation.md`, `tmp/engine-audit/SPEC-HARDENING-ADDENDUM.md`.

How to verify: Check: Implement the pure resolver and every fixed routing-table row above.; Remove filesystem-sensitive single-word plan execution unless intent is explicit.; Make automatic complexity classification opt-in or preview-confirmed. [evidence: own status: Blocked; 00-STATUS-SUMMARY 3. Open / Engine Convergence Program (: Blocked]

Verified 2026-09-28: closed as done; routing table, preview/confirm and tests are in commands/do_cmd.rs.
