+++
id = "gap-c50b85"
kind = "gap"
title = "OpenAPI document omits a large share of live routes"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-serve/openapi"]
created = 2026-09-28
updated = 2026-09-28
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-serve/src/openapi.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'cargo test -p roko-serve openapi'
+++

A local measurement (2026-09-26) found `GET /api/openapi.json` with 192 paths / 221 operations while 24 of 45 sampled method+path routes were missing, so generated clients and API docs cannot reach them.
Fix: add the missing entries plus a coverage test comparing registered routes with OpenAPI paths (ratchet), and publish the spec with releases.
