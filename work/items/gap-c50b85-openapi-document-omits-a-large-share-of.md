+++
id = "gap-c50b85"
kind = "gap"
title = "OpenAPI document omits a large share of live routes"
status = "open"
triage = "verified"
severity = "p2"
goal = "visibility"
subsystem = ["roko-serve/openapi"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-serve/src/openapi.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn openapi_documents_every_registered_route' crates/roko-serve/ && cargo test -p roko-serve openapi_documents_every_registered_route"
+++

A local measurement (2026-09-26) found `GET /api/openapi.json` with 192 paths / 221 operations while 24 of 45 sampled method+path routes were missing, so generated clients and API docs cannot reach them.
Fix: add the missing entries plus a coverage test comparing registered routes with OpenAPI paths (ratchet), and publish the spec with releases.

Verified 2026-09-28 (static check against 3d0ee4d02): crates/roko-serve/src/openapi.rs is unchanged since 244f564e1 and has no route-coverage/ratchet test; its only test is openapi_endpoint_is_served_under_api (:1370). A static comparison on 2026-09-28 (tools/http_route_inventory.py --json against the doc_* macro paths in openapi.rs, /api stripped, path params normalized, nest prefixes not applied so approximate) found 434 live method+path registrations vs 202 documented, with 237 live registrations undocumented, e.g. GET /affect/state, GET/DELETE /agent-tokens, DELETE /signals/{id}, DELETE /workspaces/{id}.

Re-checked 2026-09-29: unchanged, and the gap has grown. The portal-programme backend routes (POST /api/plans/execute, /api/plans/{id}/cancel, /api/plans/{id}/revise, /api/plans/{id}/source, /api/auth/session) are not in crates/roko-serve/src/openapi.rs. The existing verify (cargo test -p roko-serve openapi) runs only openapi_endpoint_is_served_under_api and passes while routes are missing.
