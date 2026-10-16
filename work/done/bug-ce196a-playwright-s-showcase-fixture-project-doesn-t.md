+++
id = "bug-ce196a"
kind = "bug"
title = "Playwright's showcase-fixture project doesn't ignore fly-smoke.spec.ts, so a whole-project fixture run fails without SHOWCASE_PASSPHRASE"
status = "done"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["demo-app/e2e"]
created = 2026-10-15
updated = 2026-10-16
last_verified = 2026-10-16
last_verified_rev = "7b695c7fe"
source = "status-quo-2026-10-15#finding-43"
anchors = ["demo/demo-app/playwright.config.ts", "demo/demo-app/e2e/showcase/fly-smoke.spec.ts"]
lane = "frontend"
links = { depends_on = [], blocks = [], related = ["bug-92808a"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -Eq \"testIgnore: \\[[^]]*fly-smoke\\.spec\\.ts\" demo/demo-app/playwright.config.ts"

[closed]
at = 2026-10-16
at_ts = "2026-10-16T14:09:38Z"
commit = "7b695c7fe"
by = "commit trailer"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-16T08:07:09Z"
forced = false
evidence = "demo/demo-app/playwright.config.ts:29 showcase-fixture's testIgnore now includes '**/fly-smoke.spec.ts' alongside the existing auth-gate.spec.ts and serve-manifest.spec.ts entries. Verify command (grep -Eq on the testIgnore line) passes. Did not run the fixture project itself (no SHOWCASE_PASSPHRASE needed for the verify). The S11 §6.2 runbook's --project=fly-smoke step (untracked tmp/cybernetic-harness/specs/S11-fly-deploy-security.md) is left uncorrected, out of this change's scope. Commit 7b695c7fe."
+++

## Problem

`demo/demo-app/playwright.config.ts` defines a `showcase-fixture` project (line 25) whose `testIgnore` (line 28) lists
only `**/auth-gate.spec.ts` and `**/serve-manifest.spec.ts`. `e2e/showcase/fly-smoke.spec.ts` belongs to the separate
`playwright.fly-smoke.config.ts` (task 9338), but the fixture project picks it up too, and its A1-A4 test throws
without `SHOWCASE_PASSPHRASE` (`fly-smoke.spec.ts:26`). So `npx playwright test --project=showcase-fixture` run as a
whole fails, while the per-file task verifies never notice. Status-quo assessment 2026-10-15 at 7b6a5af44, finding 43
(`tmp/status-quo/2026-10-15/backlog/A11-93-showcase-deploy.md` lines 130-133).

## Why it matters

Goal `visibility`: the showcase's fixture suite is the check that the replay-only showcase works; a suite that fails
on a clean machine gets skipped.

## Where

`demo/demo-app/playwright.config.ts` (the `showcase-fixture` project).

## Current state

`deploy/showcase/README.md` gives the right command for the fly smoke (`-c playwright.fly-smoke.config.ts`); the
S11 §6.2 runbook's step 5 says `--project=fly-smoke`, which doesn't exist in the main config.

## Plan

1. Add `'**/fly-smoke.spec.ts'` to the `showcase-fixture` project's `testIgnore`.
2. Correct the S11 §6.2 runbook's step 5 to `-c playwright.fly-smoke.config.ts`.
3. Run `npx playwright test --project=showcase-fixture` once without `SHOWCASE_PASSPHRASE` and check it passes.

## Done when

The `[[verify]]` command passes and the whole fixture project runs green without the passphrase.

## Notes

bug-92808a touches the same config (the `chromium` project's ignores); coordinate if both are in flight.
