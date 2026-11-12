+++
id = "gap-2c1b76"
kind = "gap"
title = "The serve scope guards check hand-kept route lists, so eleven mutating routes, the safety release among them, resolve to write:unclassified unnoticed"
status = "open"
triage = "verified"
severity = "p3"
goal = "release"
size = "S"
subsystem = ["roko-serve/auth"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
source = "gap-fd9dfb (2026-10-26)"
discovered_from = "gap-fd9dfb"
anchors = ["crates/roko-serve/src/routes/middleware.rs::ROUTE_SCOPE_MANIFEST", "crates/roko-serve/src/routes/middleware.rs::required_scope_for", "crates/roko-serve/src/routes/mod.rs::route_scope_manifest_matches_router"]
links = { depends_on = [], blocks = [], related = ["gap-fd9dfb"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn every_inventoried_mutating_route_is_classified' crates/roko-serve/src && cargo test -p roko-serve --lib every_inventoried_mutating_route_is_classified"
+++

## Problem

`roko serve` picks the scope a mutating request needs in `required_scope_for`
(`crates/roko-serve/src/routes/middleware.rs`): the first `ROUTE_SCOPE_MANIFEST` prefix that matches, else the
sentinel `write:unclassified`, which acts as `write` at runtime. Two tests are meant to catch a mutating route
that has no manifest row:

- `mutating_routes_are_classified` (middleware.rs), whose doc says a route added without a manifest entry "will
  resolve to SCOPE_WRITE_UNCLASSIFIED and this test will fail";
- `route_scope_manifest_matches_router` (`crates/roko-serve/src/routes/mod.rs`): "Every mutating route registered
  in `build_router` must be explicitly classified".

Both check hand-kept lists of paths (`EXPECTED_MUTATING_ROUTES`, `router_routes`), so a route added without a
manifest row is never looked at. Eleven registered mutating routes on the authenticated `/api` router resolve to
the sentinel at 754f45ebd (each registration in `tools/http_route_inventory.snapshot.json`, checked against the
manifest's prefixes):

- `POST /api/safety/controls/{agent_id}/release` (backlog 1106: releases an immune isolation control);
- `POST /api/triggers`, `DELETE /api/triggers/{name}`, `POST /api/triggers/{name}/fire`;
- `POST /api/cache/prune`;
- `POST /api/learn/homeostasis/mode`, `POST /api/learn/homeostasis/ack`;
- `POST /api/statehub/lens-runtimes/{runtime_id}/{lens}/reset`, `.../enable`, `.../disable`;
- `POST /api/surface-events`.

None of them appears in either test's list.

## Why it matters

At runtime the gap fails closed: the sentinel acts as `write`, and the RBAC layer
(`route_permissions::required_permission_for`) gives an unmatched mutation `config:edit`. But nobody chose a scope
for these routes, and the guards' promise, that a new route without a classification fails the build, does not
hold. The next route that needs `admin`, or a narrower scope than `write`, can land unreviewed. Releasing an
immune control is a security action (find-d05b5e), so its scope should be a decision rather than a fallback.

## Where

- `crates/roko-serve/src/routes/middleware.rs`: `ROUTE_SCOPE_MANIFEST`, `required_scope_for`,
  `SCOPE_WRITE_UNCLASSIFIED`, and the test `mutating_routes_are_classified` with `EXPECTED_MUTATING_ROUTES`.
- `crates/roko-serve/src/routes/mod.rs`: the test `route_scope_manifest_matches_router`; `build_router` mounts
  `auth_session::routes()` and the public routers (`webhooks`, `triggers::public_routes`, `shared_runs`) outside
  `require_api_key`, so they are out of scope here.
- `tools/http_route_inventory.snapshot.json`: every registration (`path`, `method`, `source_file`, `router_fn`);
  CI keeps it current (`tools/http_route_inventory.py --check-snapshot`, `.github/workflows/ci.yml`).

## Current state

Unchanged since the routes were added. `docs/v3/26-HTTP-API.md` §3.2 describes the runtime behaviour correctly
("else `write`: an unlisted route fails closed"); its §8.27 documents the release route's `config:edit`
requirement (gap-fd9dfb).

## Plan

1. Add a test, `every_inventoried_mutating_route_is_classified`, that reads the inventory snapshot
   (`include_str!`), skips reads and the routers mounted outside `require_api_key`, maps each path to its external
   `/api/...` form (the snapshot's paths are relative to the router they are registered on: providers' `/{id}/test`
   is mounted under `/api/providers`), and fails on any route that resolves to `SCOPE_WRITE_UNCLASSIFIED`.
2. Give each of the eleven a manifest row with a chosen scope. The release route needs `config:edit` already, so
   `admin` would match what it takes in practice; the others need a look at what they change.
3. Either keep the hand-kept lists as examples or drop them for the new test, and correct their doc comments.

## Done when

- `every_inventoried_mutating_route_is_classified` exists and passes with all eleven routes classified; the
  `[[verify]]` command runs it.

## Notes

- Found while doing gap-fd9dfb (documenting the release route). Keep the runtime fallback: it fails closed.
