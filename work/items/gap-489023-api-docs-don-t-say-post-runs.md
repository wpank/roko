+++
id = "gap-489023"
kind = "gap"
title = "API docs don't say POST /runs/{id}/share needs admin or that no_expire needs a loopback bind"
status = "open"
triage = "verified"
severity = "p3"
goal = "release"
size = "S"
subsystem = ["roko-serve"]
created = 2026-10-04
updated = 2026-10-04
last_verified = 2026-10-04
source = "gate-13c follow-up reports 2026-10-04 (PK85 gap-fcb44c, task 9328)"
discovered_from = "gap-fcb44c (closed; 9328 changed the route's behavior, docs never followed)"
anchors = ["docs/v3/26-HTTP-API.md"]
lane = "docs"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'runs/{id}/share' docs/v3/26-HTTP-API.md && grep -B2 'runs/{id}/share' docs/v3/26-HTTP-API.md | grep -qi admin && grep -qi 'no_expire' docs/v3/26-HTTP-API.md && grep -qi loopback docs/v3/26-HTTP-API.md"

[[verify]]
command = "grep -B6 'pub async fn create_share' crates/roko-serve/src/routes/shared_runs.rs | grep -q 'utoipa::path'"
+++

## Problem

9328 (implemented at `83cc285fe`) changed `POST /api/runs/{id}/share` to require `admin` scope
(write-scoped keys now get 403) and to refuse `no_expire: true` unless the server is bound to
loopback — but neither API doc describes either change, and one of them actively contradicts
the new behavior.

`docs/v3/26-HTTP-API.md` §3.2 ("Scope enforcement") gives a five-row table of route-prefix
exceptions (`/api/secrets`, `/api/config`, `/api/api-keys` → `admin`; `/api/agents/*` →
`agent:write`; `/api/plans/*` → `plan:write`) with a catch-all: "All other POST/PUT/PATCH/DELETE
| `read`". `/api/runs/{id}/share` matches none of the five prefixes, so by this table a
write-scoped key should be *allowed* to call it — the opposite of what
`crates/roko-serve/src/routes/middleware.rs:3186-3188` actually enforces (`(Method::POST,
"/runs/abc/share", "admin")`, confirmed by that file's own route-scope test table). The doc's
catch-all row is simply wrong for this one route, not just silent on it. `no_expire` and
"loopback" appear nowhere in the file.

The OpenAPI side has no route-specific description at all: `create_share`
(`crates/roko-serve/src/routes/shared_runs.rs:137`) carries no `#[utoipa::path]` attribute of
its own; its only registration is the generic `doc_post_value!(create_share, "/runs/{id}/share",
"shared_runs")` stub (`crates/roko-serve/src/openapi.rs:1873`), which emits the same boilerplate
response list for every route that uses it (200/201/202/400/401/404/409/500 with generic
descriptions) — it doesn't even list a 403 response, let alone describe the admin-scope
requirement or the loopback-only `no_expire` behavior.

## Why it matters

Goal: release, API accuracy for S11/PK85 (9328). A caller reading either doc would reasonably
conclude a write-scoped key can mint a share link (the HTTP-API doc's table says so outright)
and would have no way to learn that `no_expire` needs a loopback bind (neither doc mentions it
at all) until they hit a 403 or 400 in practice.

## Where

- `docs/v3/26-HTTP-API.md` §3.2 (the scope-enforcement table; needs a row or exception noting
  `/api/runs/{id}/share` is `admin`-scoped despite not matching the five listed prefixes).
- `crates/roko-serve/src/routes/shared_runs.rs::create_share` (needs its own `#[utoipa::path]`
  with a description covering the admin-scope requirement, the 403 response, and the
  loopback-only `no_expire` behavior, replacing the generic `doc_post_value!` stub at
  `crates/roko-serve/src/openapi.rs:1873`).
- Leave `docs/whitepaper/*` untouched (held for the paper-rewrite session).

## Current state

9328's code behavior (admin scope, loopback-gated `no_expire`) is shipped and tested
(`gap-fcb44c`'s done-note, `83cc285fe`). Neither doc source reflects it; the HTTP-API doc's
general scope table is actively wrong for this one route.

## Plan

1. Add an exception row (or an explicit note) to `docs/v3/26-HTTP-API.md` §3.2:
   `/api/runs/{id}/share` requires `admin`, not the catch-all `read`.
2. Document the loopback-only `no_expire` behavior near wherever share-link creation is
   described in `docs/v3` (§8 run-scoped observability, or wherever `/runs/{id}` is introduced).
3. Give `create_share` its own `#[utoipa::path]` description (admin scope, 403 on insufficient
   scope, 400 when `no_expire` is requested on a non-loopback bind) instead of the generic
   `doc_post_value!` stub.

## Done when

- `docs/v3/26-HTTP-API.md`'s scope table no longer implies `read` suffices for this route.
- The OpenAPI description for `create_share` states the admin-scope and loopback-`no_expire`
  requirements.

## Notes

- 2026-10-04 (gate-13c follow-up, PK85 gap-fcb44c, task 9328): confirmed at main HEAD
  `908f7ec40`. Both `[[verify]]` commands are grep-based checks over doc/comment text (no code
  behavior changes), since the fix is documentation-only.
