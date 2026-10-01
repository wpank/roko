+++
id = "gap-9b0e6d"
kind = "gap"
title = "The portal browser check records console errors and page errors but never fails on them"
status = "open"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["apps/portal", "plans/portal-programme"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "d5c1dc6be"
source = "session:roko-b6 2026-09-29 portal close-out"
discovered_from = "gap-e5bbd6 plan step 3 and bug-48494b notes ('consider making browser-flow.cjs fail on console errors'), neither done"
anchors = ["plans/portal-programme/09-acceptance/browser-flow.cjs::consoleErrors", "plans/portal-programme/09-acceptance/portal-check.sh", "plans/portal-programme/09-acceptance/hello-world-real.sh"]
links = { depends_on = [], blocks = [], related = ["gap-e5bbd6", "bug-48494b"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'no-console-errors' plans/portal-programme/09-acceptance/browser-flow.cjs && cargo build -p roko-cli && (cd apps/portal && npm run build:export) && bash plans/portal-programme/09-acceptance/portal-check.sh"
+++

## Problem

`browser-flow.cjs` collects every browser console message of type `error` and every uncaught page exception
(`pageerror`), writes them to `browser-<mode>.json`, and moves on. None of them turns into a
`BROWSER <name>: FAIL` line. So a portal that throws in the browser, or whose API requests fail, still passes
PORTAL-CHECK, and CI's portal-browser job (added by gap-e5bbd6) inherits the blind spot. bug-48494b is the example:
every validate call the portal made returned 400 while the 09 checks passed.

## Why it matters

Goal `visibility`: the portal check is the one end-to-end guard on the two-action flow, in plans and in CI. A check
that ignores browser errors lets regressions through that a person sees at once in devtools.

## Where

- `plans/portal-programme/09-acceptance/browser-flow.cjs`: `consoleErrors` and `pageErrors` are collected
  (:322-328) and written in the `finally` block (:340-345). `passStep` and `failStep` (:77-82) print the lines the
  shell scripts parse.
- `plans/portal-programme/09-acceptance/portal-check.sh` parses `BROWSER …` lines from `browser-flow.out` and
  `browser-parallel.out` (:172-205, :268-292), and only copies `browser-*.json` into the evidence (:321).
- `plans/portal-programme/09-acceptance/hello-world-real.sh` (the real-model run, `real` mode) does the same
  (:118-137).

## Current state

Checked at `d5c1dc6be`. The recorded runs show what a strict check would meet today:

- `tmp/portal-audit/evidence/portal-check/browser-flow.json` (2026-09-29 10:59): 2 × 404 and 6 × 400. The 400s were
  bug-48494b's validate calls, since fixed.
- `browser-parallel.json`: 1 × 404 and 1 × 400.
- The real-model runs: `hello-world-real-run1/browser-real.json` has 30 × 404 and 3 × 400, and
  `hello-world-real/browser-real.json` has 22 × 404 and 3 × 400.
- No `pageErrors` were recorded.
- The console text ("Failed to load resource: the server responded with a status of 404 (Not Found)") does not
  name the URL, so the evidence cannot say which requests failed.

## Plan

1. Record failed requests with their URL and status (`page.on('response')` for status >= 400, and
   `page.on('requestfailed')`) in the JSON beside the console messages.
2. Run the check once and triage the failures. Real defects become work items. Expected ones (for example a probe
   of an optional endpoint) go on a short allow-list matched by URL pattern, with a comment saying why.
3. After the flow, print `BROWSER no-console-errors: PASS` or `FAIL <n> <first few>`. Fail on any page error, and on
   any console error or failed request that the allow-list does not match. Do the same in `real` and `parallel`
   modes.

## Done when

- A console error or page error outside the allow-list fails PORTAL-CHECK.
- The `[[verify]]` command passes: the check is present and the full portal check passes with it.

## Notes

- The verify needs `apps/portal/node_modules` (symlink it from the main checkout in a worktree) and Playwright's
  Chromium in `demo/demo-app` (see gap-e5bbd6). The run takes about 100 s.
- Do not weaken the existing 09 checks.
- 2026-10-01 (wk-filer4): implemented on work/gap-2bc1b9 (plan steps 1 and 3); the portal check is deferred to the coordinator's gate. browser-flow.cjs now records failed requests (status >= 400, and requestfailed) with method, URL and status in browser-<mode>.json, and after a completed flow prints `BROWSER no-console-errors: PASS`, or `FAIL <n> <first three>` with exit code 1. All three scripts (portal-check.sh flow and parallel, hello-world-real.sh real) already turn every BROWSER line into a check, so they needed no change. "Failed to load resource" console lines are judged through their requests, which have URLs. ALLOWED_BROWSER_ERRORS starts empty: the recorded evidence names no URLs, so plan step 2's triage happens on the first gate run, whose FAIL line lists the failing requests. Expected ones go on the allow-list with a why; real defects become items.
