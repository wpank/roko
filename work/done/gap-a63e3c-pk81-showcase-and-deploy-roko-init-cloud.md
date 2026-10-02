+++
id = "gap-a63e3c"
kind = "gap"
title = "PK81 Showcase and deploy: `roko init --cloud` and the example config register deploy webhooks on another… (+7 more)"
status = "done"
triage = "verified"
severity = "p2"
goal = "proof"
rank = 81
size = "L"
subsystem = ["demo-app/showcase"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "e74148090"
source = "tmp/backlog/2026-10-02-complete-and-wire PK81"
anchors = [".github/workflows/deploy-fly.yml", "crates/roko-cli/src/commands/init.rs", "crates/roko-cli/src/commands/server.rs", "crates/roko-core/src/config/schema.rs", "demo/demo-app/e2e/navigation.spec.ts", "demo/demo-app/package.json", "demo/demo-app/playwright.config.ts", "demo/demo-app/src/components/TopNav.tsx", "demo/demo-app/src/main.tsx", "demo/demo-app/vite.config.ts", "fly.toml"]
lane = "rust-cold"
parent = "spec-0b3a32"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -qF 'owner = \\\"nunchi\\\"' crates/roko-cli/src/commands/init.rs && ! grep -qF 'owner = \\\"nunchi\\\"' crates/roko-core/src/config/schema.rs && grep -rqw 'fn cloud_init_template_names_no_third_party_repos' crates/roko-cli/src/ && cargo test -p roko-cli cloud_init_template_names_no_third_party_repos"

[[verify]]
command = "grep -q 'ROKO_STATE_ROOT' fly.toml && grep -q 'ROKO_STATE_ROOT' .github/workflows/deploy-fly.yml && grep -rqw 'fn fly_toml_points_state_root_at_the_volume' crates/roko-cli/src/ && cargo test -p roko-cli fly_toml_points_state_root_at_the_volume"

[[verify]]
command = "test -f demo/demo-app/src/showcase/contracts.ts && test -f demo/demo-app/e2e/showcase/contracts.spec.ts && cd demo/demo-app && npx tsc -p tsconfig.json && npx playwright test --project=showcase-fixture e2e/showcase/contracts.spec.ts"

[[verify]]
command = "test -f demo/demo-app/src/showcase/guard.ts && test -f demo/demo-app/e2e/showcase/guard.spec.ts && grep -q 'VITE_ALLOW_FIXTURES' demo/demo-app/vite.config.ts && cd demo/demo-app && npx playwright test --project=showcase-fixture e2e/showcase/guard.spec.ts"

[[verify]]
command = "test -f demo/demo-app/src/components/Charts/ChartTable.tsx && test -f demo/demo-app/e2e/showcase/charts.spec.ts && cd demo/demo-app && npx playwright test --project=showcase-fixture e2e/showcase/charts.spec.ts"

[[verify]]
command = "test -f demo/demo-app/src/showcase/api.ts && test -f demo/demo-app/e2e/showcase/static-source.spec.ts && cd demo/demo-app && npx playwright test --project=showcase-fixture e2e/showcase/static-source.spec.ts"

[[verify]]
command = "! grep -q 'path=\"demo\" element={null}' demo/demo-app/src/main.tsx && grep -q '\"lab' demo/demo-app/src/main.tsx && cd demo/demo-app && npx playwright test --project=chromium e2e/navigation.spec.ts"

[[verify]]
command = "test -f demo/demo-app/src/pages/showcase/Overview.tsx && test -f demo/demo-app/e2e/showcase/golden-views.spec.ts && cd demo/demo-app && npx playwright test --project=showcase-fixture e2e/showcase/golden-views.spec.ts e2e/showcase/refuse-simulated.spec.ts"

[closed]
at = 2026-10-02
at_ts = "2026-10-02T16:52:23Z"
commit = "e74148090"
executor = "claude-agent"
via = "work-batch"
size = "L"
claimed_at = "2026-10-02T13:40:01Z"
forced = false
evidence = "Gate 2 at 2199ba9ea (merged as e74148090): check/clippy/nextest green; cloud_init_template_names_no_third_party_repos and fly_toml_points_state_root_at_the_volume pass; all 6 Playwright verifies pass (showcase-fixture and chromium projects) after the gate fix 3f5875b5a-era commit renaming whisker data-axis to data-ci-axis."
+++

## Problem

This package delivers 8 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK81, slice 93xx, phase 9), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 9306 | S | p2 | `roko init --cloud` and the example config register deploy webhooks on another account's repositories | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9306-cloud-init-registers-webhooks-on-another-account.md` |
| 2 | 9307 | S | p2 | Fly deploys keep `.roko` on the ephemeral disk: the volume is mounted where nothing reads it (S11 G3) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9307-fly-deploys-keep-state-on-the-ephemeral-disk.md` |
| 3 | 9308 | M | p3 | Showcase contracts: TypeScript types, JSON Schemas, fixture bundles and the `showcase-fixture` Playwright project | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9308-showcase-contracts-schemas-and-fixture-project.md` |
| 4 | 9309 | S | p3 | Render guard, `RefusedPanel` and `ProvenanceDrawer`: refuse simulated, fixture, unattributed or tampered data | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9309-render-guard-refused-panel-and-provenance-drawer.md` |
| 5 | 9310 | M | p3 | CI-aware charts with table fallbacks: `ParetoFrontierChart`, `PassKChart`, `EnvelopeTable`, `BandLineChart`, `ChartTable` | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9310-ci-aware-charts-with-table-fallbacks.md` |
| 6 | 9311 | M | p3 | Showcase data client: static and api sources with SHA-256 checks, and a store | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9311-showcase-data-client-static-and-api-sources.md` |
| 7 | 9312 | S | p3 | Make the showcase the `/demo/` home: drop the `path="demo"` stub and move the legacy pages under `/lab/*` | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9312-make-the-showcase-the-demo-home-and-legacy-lab.md` |
| 8 | 9313 | M | p3 | R1 pages: Overview (P1 and M4 tiles), HeadToHead, AuditLottery and the Replays list | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9313-r1-pages-overview-head-to-head-audits-replays.md` |

## Why it matters

Phase 9: domains, assistant, held and parked work, cleanup, showcase and deploy. The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9300-showcase-deploy-and-release.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `.github/workflows/deploy-fly.yml`, `crates/roko-cli/src/commands/init.rs`, `crates/roko-cli/src/commands/server.rs`, `crates/roko-core/src/config/schema.rs`, `demo/demo-app/e2e/navigation.spec.ts`, `demo/demo-app/e2e/showcase/bundles/`, `demo/demo-app/e2e/showcase/charts.spec.ts`, `demo/demo-app/e2e/showcase/contracts.spec.ts`, `demo/demo-app/e2e/showcase/golden-views.spec.ts`, `demo/demo-app/e2e/showcase/guard.spec.ts`, `demo/demo-app/e2e/showcase/refuse-simulated.spec.ts`, `demo/demo-app/e2e/showcase/static-source.spec.ts`, `demo/demo-app/package.json`, `demo/demo-app/playwright.config.ts`, `demo/demo-app/src/components/Charts/BandLineChart.tsx`, `demo/demo-app/src/components/Charts/ChartTable.tsx`, `demo/demo-app/src/components/Charts/EnvelopeTable.tsx`, `demo/demo-app/src/components/Charts/ParetoFrontierChart.tsx`, `demo/demo-app/src/components/Charts/PassKChart.tsx`, `demo/demo-app/src/components/NotMeasured.tsx`, `demo/demo-app/src/components/ProvenanceDrawer.tsx`, `demo/demo-app/src/components/RefusedPanel.tsx`, `demo/demo-app/src/components/TopNav.tsx`, `demo/demo-app/src/main.tsx`, `demo/demo-app/src/pages/showcase/AuditLottery.tsx`, `demo/demo-app/src/pages/showcase/HeadToHead.tsx`, `demo/demo-app/src/pages/showcase/Overview.tsx`, `demo/demo-app/src/pages/showcase/Replays.tsx`, `demo/demo-app/src/showcase/api.ts`, `demo/demo-app/src/showcase/contracts.ts`, `demo/demo-app/src/showcase/fixtureRoutes.tsx`, `demo/demo-app/src/showcase/guard.ts`, `demo/demo-app/src/showcase/schemas/`, `demo/demo-app/src/showcase/store.ts`, `demo/demo-app/vite.config.ts`, `fly.toml`.

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

- Waits on: nothing.
- Suggested model: opus.

## Progress

Worker run of 2026-10-02 on `work/gap-a63e3c` (base `976220c3e`). Workers run no cargo, tsc or Playwright: every task is
implemented, not done, and its cargo, `npx tsc` and Playwright verification is deferred to the batch gate. Static verify
parts pass for all eight. Python checks and Node type-stripping smoke runs (scratch only, nothing added to the repo) are
noted per task.

- 9306: implemented at 38d8e51c0. Both writers emit the webhook table commented out, with placeholder owner and repo. The
  test also lets through the default `worker_image` (`ghcr.io/nunchi-trade/...`), which is 9341's to change.
- 9307: implemented at 41df07bed. `ROKO_STATE_ROOT` = the mount in the root `fly.toml`, `write_fly_toml` and the workflow
  (both TOMLs parse, checked with tomllib). The test also checks the checked-in `fly.toml`.
- 9308: implemented at 235ca532d. The schemas use only `validate.py`'s keyword subset, so 9314 can check bundles with it.
  The specs use a TypeScript port of that validator instead of ajv, which needs a registry install. Seven fixture bundles:
  golden, negatives, P1-only (no audits), and four poisoned. Checked with `validate.py`; contracts.spec logic 15/15.
- 9309: implemented at 9e4ef50ec. guard.spec logic 12/12 under Node, including the Vite config refusing a production
  build with fixtures.
- 9310: implemented at 7cee701e8. Chart palette from the dataviz validator (the rosedust tokens fail it). The charts were
  rendered with react-dom/server; whisker, band and axis checks hold.
- 9311: implemented at e415bd665. static-source.spec Node tests 7/7; the in-browser test needs Playwright.
- 9312: implemented at bf205ad4a. Also touched beyond the task's files: `AppShell.tsx` (scenario player at /lab/demo,
  shortcuts), `pages/dashboard/Layout.tsx` (tab links) and `e2e/landing.spec.ts` (opens /lab).
- 9313: implemented at 7bfb4c58e. Pages rendered server-side against the static source: golden values, counts,
  negatives, not-yet-measured and every poisoned refusal hold.
