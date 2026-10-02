+++
id = "gap-fcb44c"
kind = "gap"
title = "PK85 Showcase and deploy: Any write-scoped caller can mint a permanent public share link, also on a public bind (+8 more)"
status = "open"
triage = "verified"
severity = "p2"
goal = "release"
rank = 85
size = "L"
subsystem = ["roko-serve/showcase"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK85"
anchors = [".dockerignore", "Dockerfile", "crates/roko-cli/src/commands/server.rs", "crates/roko-serve/src/embedded.rs", "crates/roko-serve/src/lib.rs", "crates/roko-serve/src/routes/middleware.rs", "crates/roko-serve/src/routes/shared_runs.rs", "demo/demo-app/src/main.tsx", "demo/demo-app/src/transport/api.ts", "demo/demo-app/src/transport/sse.ts"]
lane = "rust-cold"
parent = "spec-0b3a32"
links = { depends_on = ["gap-a63e3c", "gap-9ecd37", "gap-4119fb"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn permanent_share_is_refused_on_a_public_bind' crates/roko-serve/src/ && cargo test -p roko-serve --lib permanent_share_is_refused_on_a_public_bind"

[[verify]]
command = "grep -rqw 'fn showcase_mode_redirects_root_to_demo' crates/roko-serve/src/ && cargo test -p roko-serve --lib showcase_mode_redirects_root_to_demo"

[[verify]]
command = "test -f crates/roko-serve/src/showcase/idle.rs && grep -rqw 'fn showcase_idle_exit_waits_for_active_run' crates/roko-serve/src/ && cargo test -p roko-serve --lib showcase_idle_"

[[verify]]
command = "grep -q 'X-Roko-CSRF' demo/demo-app/src/transport/api.ts && test -f demo/demo-app/e2e/showcase/transport-auth.spec.ts && cd demo/demo-app && npx playwright test --project=showcase-fixture e2e/showcase/transport-auth.spec.ts"

[[verify]]
command = "test -f demo/demo-app/src/pages/showcase/Login.tsx && test -f demo/demo-app/e2e/showcase/auth-gate.spec.ts && cd demo/demo-app && npx playwright test -c playwright.showcase-auth.config.ts"

[[verify]]
command = "test -f demo/demo-app/playwright.showcase-serve.config.ts && cd demo/demo-app && npx playwright test -c playwright.showcase-serve.config.ts"

[[verify]]
command = "python3 -c \"import tomllib;c=tomllib.load(open('fly.showcase.toml','rb'));assert c['app']=='roko-showcase';assert c['http_service']['auto_stop_machines']=='off';assert c['mounts'][0]['destination']=='/data';assert c['env']['ROKO_STATE_ROOT']=='/data/.roko';assert c['restart'][0]['policy']=='on-failure'\" && grep -q 'argon2id' docker/showcase-entrypoint.sh && grep -q 'docs/v3/.vitepress' .dockerignore && grep -rqw 'fn showcase_example_config_loads_without_unknown_keys' crates/roko-core/src/ && cargo test -p roko-core showcase_example_config_loads_without_unknown_keys"

[[verify]]
command = "grep -q 'AS builder-core' Dockerfile && grep -q 'AS showcase-replay' Dockerfile && docker build --target showcase-replay -t roko-showcase-replay:verify . && test \"$(docker image inspect roko-showcase-replay:verify --format '{{.Size}}')\" -lt 262144000"

[[verify]]
command = "grep -q 'build_target' crates/roko-cli/src/commands/server.rs && grep -rqw 'fn deploy_fly_dry_run_reads_app_and_region_from_config' crates/roko-cli/src/ && cargo test -p roko-cli deploy_fly_dry_run_reads_app_and_region_from_config"
+++

## Problem

This package delivers 9 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK85, slice 93xx, phase 9), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 9328 | S | p2 | Any write-scoped caller can mint a permanent public share link, also on a public bind | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9328-write-scope-can-mint-permanent-public-share-links.md` |
| 2 | 9329 | S | p3 | Showcase mode: `/` redirects to `/demo/`, the portal is not served, and `/demo/lab/*` is refused | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9329-showcase-mode-root-redirect-and-no-portal.md` |
| 3 | 9330 | M | p2 | Idle exit after 20 minutes, with an activity middleware, run blockers and a hold file (G10) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9330-showcase-idle-exit-with-activity-and-hold-file.md` |
| 4 | 9331 | M | p3 | Demo-app transport for sessions: credentials, the CSRF header, 401 → login, and an SSE probe with backoff | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9331-demo-app-transport-for-sessions.md` |
| 5 | 9332 | S | p3 | Login page and `WakeUpBanner`, with `auth-gate.spec` against a local showcase-mode serve | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9332-login-page-wakeup-banner-and-auth-gate-spec.md` |
| 6 | 9333 | S | p3 | `showcase-replay` Playwright project: the R1 specs against the real bundle served by `roko serve` at `/demo` (R1-serve) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9333-showcase-replay-project-against-roko-serve.md` |
| 7 | 9334 | S | p3 | Showcase deploy files: `fly.showcase.toml`, `docker/showcase.roko.toml`, the entrypoint, and the rest of the `.dockerignore` fix | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9334-showcase-deploy-files-and-dockerignore.md` |
| 8 | 9335 | M | p3 | Dockerfile stages `builder-core` and `showcase-replay`: a small image with only `roko`, under 250 MB | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9335-dockerfile-builder-core-and-showcase-replay.md` |
| 9 | 9336 | M | p3 | `roko deploy fly` reads an existing config, takes a build target, and checks the G0–G2 posture (revives find-3d0d97) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9336-roko-deploy-fly-reads-config-and-checks-posture.md` |

## Why it matters

Phase 9: domains, assistant, held and parked work, cleanup, showcase and deploy. The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9300-showcase-deploy-and-release.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `.dockerignore`, `Dockerfile`, `crates/roko-cli/src/commands/server.rs`, `crates/roko-core/src/config/showcase.rs`, `crates/roko-serve/src/embedded.rs`, `crates/roko-serve/src/lib.rs`, `crates/roko-serve/src/routes/middleware.rs`, `crates/roko-serve/src/routes/shared_runs.rs`, `crates/roko-serve/src/showcase/idle.rs`, `crates/roko-serve/src/showcase/mod.rs`, `demo/demo-app/e2e/showcase/auth-gate.spec.ts`, `demo/demo-app/e2e/showcase/serve-manifest.spec.ts`, `demo/demo-app/e2e/showcase/transport-auth.spec.ts`, `demo/demo-app/playwright.showcase-auth.config.ts`, `demo/demo-app/playwright.showcase-serve.config.ts`, `demo/demo-app/src/components/WakeUpBanner.tsx`, `demo/demo-app/src/main.tsx`, `demo/demo-app/src/pages/showcase/Login.tsx`, `demo/demo-app/src/transport/api.ts`, `demo/demo-app/src/transport/sse.ts`, `docker/showcase-entrypoint.sh`, `docker/showcase.roko.toml`, `fly.showcase.toml`.

## Current state

The tasks were checked against `2c3ea9f73` on 2026-10-02. Re-check each task's anchors and premise at your base commit before implementing it, and report a task that is already done instead of redoing it.

## Plan

1. Work through the tasks in the order above. For each: read its file, implement its Plan, write the test it names, and make one commit per task whose message ends with `Backlog-Task: <task id>`, `Work-Item: <this item's id>` and `Executor: claude-agent`.
2. Follow `BUILD-RULES.md` in the backlog folder. Workers run no cargo: Rust is checked by the coordinator's batched gate. Python and doc checks you may run.
3. If a task cannot be done (a premise is false, a decision is missing, or its verify cannot pass), stop at that task, keep the earlier commits, and report it; do not skip ahead to tasks that depend on it.

## Done when

- Every task's verify command passes (this item's `[[verify]]` list, one entry per task), after the coordinator's batched gate.
- Each task's own "Done when" holds (see its file).

## Notes

- Waits on: PK81 (gap-a63e3c), PK82 (gap-9ecd37), PK83 (gap-4119fb).
- Suggested model: opus.
