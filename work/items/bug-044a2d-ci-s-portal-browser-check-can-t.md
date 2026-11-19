+++
id = "bug-044a2d"
kind = "bug"
title = "CI's Portal browser check can't run on GitHub's runners: ubuntu-latest is now 26.04, where demo/demo-app's Playwright 1.59.1 has no Chromium (PR #88 and #89), and behind that, on Linux a parallel run's second delivery fails 'Author identity unknown' because a runner has no git identity"
status = "open"
triage = "verified"
severity = "p2"
goal = "release"
size = "S"
subsystem = ["ci"]
created = 2026-11-19
updated = 2026-11-19
last_verified = 2026-11-19
source = "round v8 portal-v8, the lead's ask for release PR #89 (2026-11-19)"
discovered_from = "agent:portal-v8 (round v8)"
anchors = [".github/workflows/ci.yml"]
links = { depends_on = [], blocks = [], related = ["find-8cc7ac", "bug-37a893"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "J=\"$(awk '/^  portal-browser:/{f=1;next} /^  [a-z][a-z-]*:$/{f=0} f' .github/workflows/ci.yml)\" && printf '%s\\n' \"$J\" | grep -q 'runs-on: ubuntu-24.04' && printf '%s\\n' \"$J\" | grep -q 'git config --global user.email' && ! printf '%s\\n' \"$J\" | grep -q 'shared-key: layer-check'"
+++

## Problem

CI's "Portal browser check (fake agent)" job can't run on GitHub's runners now. It failed on PR #88 (run
43183197388) and on release PR #89 (run 44516751180, job 135534623400). Two causes, the second hidden behind the first:

1. **Playwright has no Chromium for the runner's new image.** `ubuntu-latest` is now `ubuntu-26.04` (image version
   20261111.196.1). The step "Build the demo app and install Playwright's Chromium" runs
   `npx playwright install --with-deps chromium` with demo/demo-app's Playwright 1.59.1, which knows Ubuntu up to
   24.04. The step fails with "Playwright does not support chromium on ubuntu26.04-x64". The demo app itself builds
   fine. Playwright 1.65.1 supports ubuntu26.04-x64.
2. **On Linux, a parallel run's second delivery needs a git identity, and a runner has none.** The last time the
   check itself ran on a runner (run 39408440267, 2026-10-20), par-b's delivery failed with "git commit-tree
   failed: Author identity unknown". roko merges a second plan into its run's branch with `git commit-tree`, which
   takes the person's identity (bug-37a893). That run's other failures came from the fake agent's knobs, fixed
   since in gap-60f6b6. On Will's Mac the check's harness gives git an identity, so it passed there (39/39 at
   456afc779).

## Why it matters

Goal release: PR #89 must go green, and the job has never had a green run on Linux.

## Where

`.github/workflows/ci.yml`, job `portal-browser`.

## Plan

- Run the job on `ubuntu-24.04`, which Playwright 1.59.1 supports. Leave a comment saying why, and when to unpin
  it (demo/demo-app's Playwright at 1.65 or later). This leaves the demo app's dependencies, and every local
  Playwright harness that uses them, as they are.
- Drop the job's reuse of layer-check's Rust cache. That cache is built on `ubuntu-latest`, and rust-cache keys
  only by OS name (`Linux`), so it would restore a 26.04 `target/` onto 24.04. The job now builds roko-cli cold,
  in about 15 minutes, well inside its 45.
- Give git an identity before the check (global config, so roko's own `-c` identities still win).

## Done when

- The job passes on PR #89's next run.
- The `[[verify]]` passes: it checks the job's runner and identity step.

## Notes

- Round v8 portal-v8, the lead's ask (2026-11-19). It's part of find-8cc7ac (workflows red on main), which it
  doesn't close.
