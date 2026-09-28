+++
id = "bug-8dbffd"
kind = "bug"
title = "roko acp rejects spec-shaped permission responses"
status = "open"
triage = "verified"
severity = "p1"
subsystem = ["roko-acp/permissions"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-acp/src/types.rs::PermissionOutcome"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'cargo test -p roko-acp'
+++

`PermissionOutcome` deserializes with `#[serde(rename_all = "camelCase", tag = "type")]` (`roko-acp/src/types.rs:1174`), i.e. `{"type":"selected","optionId":..}`.
Clients built on the ACP spec SDK answer `session/request_permission` with `{"outcome":{"outcome":"selected","optionId":"allow_once"}}`, which fails to parse, so every shell/write/edit approval from a spec client is rejected.
Fix: accept the spec shape (keep the legacy shape as an alias) and add golden-file conformance tests for permission responses.
