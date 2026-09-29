+++
id = "gap-2122bd"
kind = "gap"
title = "Release and Docker builds do not build the portal export; packaged binary serves the fallback page"
status = "open"
triage = "verified"
severity = "p2"
subsystem = ["roko-serve/embedded", "ci/release"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "plan:portal-programme/03c-backend-local-access#T10"
discovered_from = "plan:portal-programme/03c-backend-local-access#T10"
anchors = [".github/workflows/release.yml", "Dockerfile"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

`roko serve` probes for the portal in this order:

1. `ROKO_SPA_DIR` (env override)
2. `apps/portal/out` beside the crate (Next.js export)
3. A copy embedded at compile time under `roko_portal_fallback` cfg
4. A fallback page explaining how to build the portal

`.github/workflows/release.yml` and `Dockerfile` build only the demo app
(`demo/demo-app`); neither runs `next build` in `apps/portal` nor sets the
`roko_portal_fallback` cfg feature. A packaged binary therefore falls through
to the fallback page at `/`.

**Impact:** end users installing `roko` from a release binary or Docker image see
"how to build the portal" at `http://localhost:6677/` instead of the portal UI.

**Fix needed:** add `cd apps/portal && npm run build` (Next.js export to `out/`)
before the cargo build step in `release.yml` and `Dockerfile`, or embed the
portal at compile time via `roko_portal_fallback` when the `out/` directory is
present. The embedded approach keeps the binary self-contained but roughly
doubles its size (estimate: ~8 MB gzipped for a Next.js export).

Plan 09 (`portal-programme/09-integration`) is the natural home for this fix,
since it verifies the zero-config `roko serve` launch path end to end.
