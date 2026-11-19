+++
id = "bug-623001"
kind = "bug"
title = "CI's Test + Clippy times out on a cold cache: clippy plus a cold cargo test --workspace takes over its 60 minutes after a Cargo.lock change, and a cancelled job saves no cache, so every later run stays cold"
status = "open"
triage = "verified"
severity = "p2"
goal = "release"
size = "S"
subsystem = ["ci"]
created = 2026-11-19
updated = 2026-11-19
last_verified = 2026-11-19
source = "round v8 lead: PR #89's CI run 44558280110 at ffd178a22, job 135680212779 (cancelled at 60.3 min)"
discovered_from = "agent:roko-recovery (round v8, 2026-11-19)"
anchors = [".github/workflows/ci.yml"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-996dc9"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "awk '/^  test:/{t=1;next} /^  [a-z-]+:/{t=0} t' .github/workflows/ci.yml | grep -q 'timeout-minutes: 120' && awk '/^  test:/{t=1;next} /^  [a-z-]+:/{t=0} t' .github/workflows/ci.yml | grep -q 'cache-on-failure: true'"
+++

## Problem

PR #89's Test + Clippy at ffd178a22 (CI run 44558280110, job 135680212779) was cancelled by its 60-minute limit after
60.3 minutes. No test failed: clippy passed, then `cargo test --workspace` ran out of time. bug-996dc9 changed
Cargo.lock, so Swatinem/rust-cache's key missed and the build was cold. A cancelled job skips rust-cache's post step,
which is what saves the cache, so the next run starts cold too, and so does every run after it.

On the same cold cache, every other job of that run finished within its limit:

| Job | Took (minutes) | Limit |
|---|---|---|
| Golden-path canaries | 16.8 | 45 |
| Layer Check | 14.8 | 30 |
| Lean CLI + Alloy opt-in | 16.0 | 30 |
| Portal browser check (no Rust cache) | 19.0 | 45 |
| coverage.yml's job | finished | 45 |

## Why it matters

Goal release. The release PR's required test job can't finish after a dependency update, and since a timed-out run
saves nothing, it never recovers on its own.

## Where

- `.github/workflows/ci.yml`, job `test` (Test + Clippy).

## Plan

1. Raise the job's `timeout-minutes` from 60 to 120, so a cold run can finish and save the cache.
2. Give its rust-cache step `cache-on-failure: true`, so a run with a failing test still saves the build.
3. Leave the other jobs' limits alone: they finished cold with room to spare.

## Done when

A Test + Clippy run on a cold cache finishes and the run after it starts warm. The `[[verify]]` checks the workflow.
