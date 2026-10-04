+++
id = "gap-9ecd37"
kind = "gap"
title = "PK82 Showcase and deploy: Showcase bundle builder and verifier (Python, in ViabilityBench) (+7 more)"
status = "open"
triage = "verified"
severity = "p2"
goal = "proof"
rank = 82
size = "L"
subsystem = ["benchmarks/viabilitybench/showcase"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK82"
anchors = ["Cargo.lock", "Cargo.toml", "crates/roko-cli/Cargo.toml", "crates/roko-cli/src/commands/mod.rs", "crates/roko-cli/src/main.rs", "crates/roko-cli/src/serve_runtime.rs", "crates/roko-core/src/config/loader.rs", "crates/roko-core/src/config/mod.rs", "crates/roko-core/src/config/schema.rs", "crates/roko-serve/src/lib.rs", "crates/roko-serve/src/routes/bench.rs", "crates/roko-serve/src/runtime.rs", "demo/demo-app/src/components/CostRace.tsx", "demo/demo-app/src/components/TerminalPreview.tsx", "demo/demo-app/src/components/TopNav.tsx", "demo/demo-app/src/lib/prd-pipeline-sample.ts", "demo/demo-app/src/pages/Bench.tsx"]
lane = "rust-cold"
parent = "spec-0b3a32"
links = { depends_on = ["gap-5ebb4f", "gap-2ca903", "gap-5e9292", "gap-a63e3c"], blocks = [], related = ["gap-f30b8e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f benchmarks/viabilitybench/showcase/verify_bundle.py && benchmarks/viabilitybench/.venv/bin/python -m pytest -q benchmarks/viabilitybench/showcase/test_bundle.py"

[[verify]]
command = "test -f demo/demo-app/e2e/showcase/copy-rules.spec.ts && test -f demo/demo-app/e2e/showcase/retired.spec.ts && cd demo/demo-app && npx playwright test --project=showcase-fixture e2e/showcase/copy-rules.spec.ts e2e/showcase/retired.spec.ts"

[[verify]]
command = "! grep -q \"id: 'demo'\" demo/demo-app/src/pages/Bench.tsx && test -f demo/demo-app/e2e/no-simulated.spec.ts && cd demo/demo-app && npx playwright test --project=chromium e2e/no-simulated.spec.ts"

[[verify]]
command = "grep -rqw 'fn bench_run_without_gates_is_unverified' crates/roko-cli/src/ && cargo test -p roko-cli bench_run_without_gates_is_unverified"

[[verify]]
command = "test -f crates/roko-core/src/config/showcase.rs && grep -rqw 'fn showcase_table_survives_loading' crates/roko-core/src/ && grep -rqw 'fn showcase_mode_refuses_to_start_half_configured' crates/roko-serve/src/ && cargo test -p roko-core showcase_table_survives_loading && cargo test -p roko-serve --lib showcase_mode_refuses_to_start_half_configured"

[[verify]]
command = "grep -q 'name = \"argon2\"' Cargo.lock && grep -rqw 'fn passphrase_hash_round_trips' crates/roko-cli/src/ && cargo test -p roko-cli passphrase_hash_round_trips"
+++

## Problem

This package delivers 8 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK82, slice 93xx, phase 9), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 9314 | M | p3 | Showcase bundle builder and verifier (Python, in ViabilityBench) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9314-showcase-bundle-builder-and-verifier.md` |
| 2 | 9315 | S | p2 | Build, verify and keep the first real showcase bundle from the pilot runs (the golden path) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9315-build-the-first-real-bundle-from-the-pilot.md` |
| 3 | 9316 | M | p3 | `showcase-static` Playwright config, the R1 specs and the bundle budget, against the staged real bundle (R1-static) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9316-showcase-static-project-r1-specs-and-budget.md` |
| 4 | 9317 | S | p3 | Retire the fake terminal lines, the sample pipeline, `CostRace` and the Demo scenario player from the showcase | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9317-retire-fake-and-sample-pieces-from-the-showcase.md` |
| 5 | 9318 | S | p3 | The Bench page still offers the simulated "Demo" strategy | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9318-bench-page-still-offers-the-simulated-demo-strategy.md` |
| 6 | 9319 | M | p2 | A bench run "passes" when the provider answers: serve's bench path reports success with no gate results | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9319-bench-run-passes-when-the-provider-answers.md` |
| 7 | 9320 | M | p2 | A `[showcase]` config table that survives loading, and a serve that refuses to start half-configured in showcase mode | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9320-showcase-config-table-and-fail-closed-startup.md` |
| 8 | 9321 | S | p3 | Add `argon2` and `roko showcase passphrase {new,hash}` | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9321-argon2-and-the-showcase-passphrase-cli.md` |

## Why it matters

Phase 9: domains, assistant, held and parked work, cleanup, showcase and deploy. The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9300-showcase-deploy-and-release.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `Cargo.lock`, `Cargo.toml`, `benchmarks/viabilitybench/showcase/build_bundle.py`, `benchmarks/viabilitybench/showcase/fixtures/`, `benchmarks/viabilitybench/showcase/test_bundle.py`, `benchmarks/viabilitybench/showcase/verify_bundle.py`, `crates/roko-cli/Cargo.toml`, `crates/roko-cli/src/commands/mod.rs`, `crates/roko-cli/src/commands/showcase.rs`, `crates/roko-cli/src/main.rs`, `crates/roko-cli/src/serve_runtime.rs`, `crates/roko-core/src/config/loader.rs`, `crates/roko-core/src/config/mod.rs`, `crates/roko-core/src/config/schema.rs`, `crates/roko-core/src/config/showcase.rs`, `crates/roko-serve/src/lib.rs`, `crates/roko-serve/src/routes/bench.rs`, `crates/roko-serve/src/runtime.rs`, `demo/demo-app/e2e/no-simulated.spec.ts`, `demo/demo-app/e2e/showcase/copy-rules.spec.ts`, `demo/demo-app/e2e/showcase/head-to-head.spec.ts`, `demo/demo-app/e2e/showcase/m4-lottery.spec.ts`, `demo/demo-app/e2e/showcase/mobile.spec.ts`, `demo/demo-app/e2e/showcase/negatives.spec.ts`, `demo/demo-app/e2e/showcase/provenance.spec.ts`, `demo/demo-app/e2e/showcase/retired.spec.ts`, `demo/demo-app/playwright.showcase-static.config.ts`, `demo/demo-app/scripts/showcase-bundle-budget.mjs`, `demo/demo-app/scripts/stage-showcase-bundle.mjs`, `demo/demo-app/src/components/CostRace.tsx`, `demo/demo-app/src/components/TerminalPreview.tsx`, `demo/demo-app/src/components/TopNav.tsx`, `demo/demo-app/src/lib/prd-pipeline-sample.ts`, `demo/demo-app/src/pages/Bench.tsx`.

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

- Waits on: PK20 (gap-5ebb4f), PK30 (gap-2ca903), PK73 (gap-5e9292), PK81 (gap-a63e3c).
- Existing work items this package covers or touches: gap-f30b8e. When its tasks are done, close those whose verify then passes.
- Suggested model: opus.
- 2026-10-02 (roko-7d): the workflow-audit migration (merge bfd36512f) removed the PRD pipeline, `roko do` and `roko develop`; `roko run` is the one entry point and plans come from a prompt. In demo-app it fixed only what broke (the scenario runners and `PipelineStagesPanel` say `roko run`, the transport types lost the Atelier fields, `lib/cmd-descriptions.ts` describes the new commands). The PRD sample pieces (`lib/prd-pipeline-sample.ts`, `PrdPipelinePanel.tsx`, the fake terminal lines) are left for task 9317.

## Progress

- 9314: implemented at c66dfade0 (`benchmarks/viabilitybench/showcase/`: build_bundle.py, verify_bundle.py, a fixture P1 experiment; `test_bundle.py`, 9 tests, passes in a venv)
- 9315: blocked: held by the coordinator for a follow-up; it builds the first real bundle from the pilot runs, which have not happened
- 9316: blocked: held with 9315; the showcase-static project's specs and budget need the real pilot bundle
- 9317: implemented at be9bb659f (TSX; npm not run: the gate should run `npx tsc --noEmit` and the showcase-fixture specs copy-rules and retired)
- 9318: implemented at a082101cf (TSX; npm not run: `npx playwright test --project=chromium e2e/no-simulated.spec.ts`)
- 9319: implemented at 3fa7ee21a (cargo verification deferred to the batch gate; the bench route already graded tasks by executed check since 3343, so the change is `RunResult::verdict()` plus two tests)
- 9320: implemented at f0bf3d36b (cargo verification deferred to the batch gate; config invariant 15)
- 9321: implemented at a847660ea (cargo verification deferred to the batch gate; Cargo.lock must be regenerated there for the verify's argon2 grep)
- 2026-10-04 (coordinator, gate 12b): 9315 (the first real showcase bundle from the pilot runs) and 9316 (the showcase-static Playwright specs against it) and their verifies left this item for a held follow-up: no pilot has run. 9314's verify now runs pytest with the bench venv (the system python3 has no pytest).
