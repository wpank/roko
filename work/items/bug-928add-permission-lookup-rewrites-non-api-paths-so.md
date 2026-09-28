+++
id = "bug-928add"
kind = "bug"
title = "Permission lookup rewrites non-/api paths, so /relay permission rows never match"
status = "open"
triage = "verified"
severity = "p2"
goal = "release"
subsystem = ["roko-serve/rbac"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-serve/src/routes/route_permissions.rs::required_permission_for", "crates/roko-serve/src/routes/route_permissions.rs:183"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'cargo test -p roko-serve route_permissions'
+++

`required_permission_for` rewrites any path that does not start with `/api/` to `/api{path}` before matching (`route_permissions.rs:198-204`), but the manifest declares the relay row as `/relay` (`:183`, `AgentSpawn`).
`/relay/...` requests are looked up as `/api/relay/...`, miss their row and fall through to the generic rules (per a local audit: reads unrestricted, `ConfigEdit` otherwise), so the declared policy never applies.
Fix: match the raw path first (or declare rows in the rewritten form) and add a test that `/relay` resolves to its declared permission.
