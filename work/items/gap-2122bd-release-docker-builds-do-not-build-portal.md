+++
id = "gap-2122bd"
kind = "gap"
title = "Release and Docker builds do not build the portal export; packaged binary serves the fallback page"
status = "open"
triage = "verified"
severity = "p2"
goal = "release"
subsystem = ["roko-serve/embedded", "ci/release"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "plan:portal-programme/03c-backend-local-access#T10"
discovered_from = "plan:portal-programme/03c-backend-local-access#T10"
anchors = [".github/workflows/release.yml:70", "Dockerfile:16", "crates/roko-serve/build.rs:30", "crates/roko-serve/src/embedded.rs:36"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'apps/portal' .github/workflows/release.yml && grep -q 'apps/portal' Dockerfile && grep -q 'build:export' .github/workflows/release.yml"
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

Re-checked 2026-09-29: unchanged. Correction to the fix text: roko_portal_fallback selects the fallback page; build.rs sets it when apps/portal/out/index.html is missing at compile time (crates/roko-serve/build.rs:28-32), and the portal export is embedded automatically when it is present (crates/roko-serve/src/embedded.rs:36-37). The fix is therefore to run npm ci && npm run build:export in apps/portal before the cargo build in .github/workflows/release.yml and Dockerfile.
