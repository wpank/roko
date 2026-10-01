+++
id = "spec-f09094"
kind = "spec"
title = "Epic: golden-path acceptance tests"
status = "open"
triage = "unverified"
severity = "p1"
goal = "golden-path"
size = "L"
subsystem = ["roko-cli/tests", ".github/workflows"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e11"
discovered_from = "tmp/cybernetic-harness/workstreams/assessment/W8-roko-as-executor.md (canaries C1-C8 and GP)"
anchors = ["crates/roko-cli/tests/common/mod.rs", ".github/workflows/ci.yml"]
doc = "tmp/cybernetic-harness/workstreams/PLAN.md"
lane = "rust-cold"
links = { depends_on = ["gap-3aa9cb", "gap-f30b8e"], blocks = [], related = ["gap-cd3529", "gap-0e2c40", "gap-af00b1", "gap-b954ad", "gap-987064", "gap-9eebcb", "gap-e21595"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn golden_path_suite_covers_c1_to_c8' crates/roko-cli/tests/ && cargo test -p roko-cli --test golden_path_suite"

[[verify]]
command = "grep -rqw 'fn golden_path_fixture_plan_merges_green' crates/roko-cli/tests/ && cargo test -p roko-cli --test golden_path_acceptance golden_path_fixture_plan_merges_green"
+++

## Problem

Each golden-path epic ends in its own canary, a deterministic integration test with a scripted fake agent:

| Canary | What it proves | Item | Epic |
|---|---|---|---|
| C1 | Honest verdicts | gap-cd3529 | E2 |
| C2 | Secrets and the git guard | gap-0e2c40 | E3 |
| C3, C4 | Per-task commits on a plan branch; the whole-plan gate | gap-af00b1 | E6 |
| C5 | The attempt diff check | gap-b954ad | E9 |
| C6 | The scheduler | gap-987064 | E7 |
| C7 | The watchdog and the disk check | gap-9eebcb | E10 |
| C8 | The tier ladder | gap-e21595 | E5 |

Nothing yet runs them together as the regulator's regression suite. And nothing checks the whole loop on real
models: a frontier-written plan executed on cheap models, escalated when a check fails, integrated, verified and
merged with no human step.

## Why it matters

This is the golden path's acceptance. Assessment W8 sets the bar: gates G1–G8, each proven by a canary, then GP, a
real-model plan run that passes 3 runs out of 3. Until it passes, the whitepaper's cheap-model claim is untested: all
210 portal attempts pinned Sonnet 4.6 (research note B7). `PLAN.md` §4 estimates week 6 (range: weeks 5–8).

## Where

- `crates/roko-cli/tests/`: the canaries.
  - `tests/common/mod.rs` has the shared helpers and a fixed mock Claude script (`setup_sample_plan_workspace`).
  - `graph_budget_resume.rs` has the scripted-provider pattern.
- `.github/workflows/ci.yml`: today a single `cargo test --workspace` job runs everything, so a canary failure is not
  reported on its own.

## Current state

Checked at `41c7ffbd6`: none of C1–C8 exists yet; each is an open item under its epic. There is no golden-path
fixture plan and no live acceptance run.

## Plan

This is the implementation plan. These are tests of Roko, not the way the backlog gets done.

1. **The canaries land with their epics** (the C items above), each in its own test file.
2. **One suite** (gap-3aa9cb): once two canaries exist, extract their fake provider into a shared scripted provider,
   and move the others onto it as they land. Add a `golden-path` CI job, and a guard test that fails when a canary is
   missing.
3. **End-to-end acceptance** (gap-f30b8e): a fixture plan of 8–10 tasks, replayed with the scripted provider in CI,
   then run three times on real models through the D11 ladder.

## Done when

- [x] gap-3aa9cb: One CI suite for the golden-path integration tests C1–C8, with a shared scripted fake provider
- [ ] gap-f30b8e: End-to-end acceptance test: a fixture plan runs through the ladder on real cheap models and merges
      green
- [ ] The epic's two `[[verify]]` commands pass on the merged branch, and gap-f30b8e's closing evidence records 3 of
      3 live passes.

## Notes

- The C items stay children of their own epics. This epic tracks them through gap-3aa9cb's `depends_on`.
- Real-model runs cost money and draw on the same subscription limits as the Claude workers (W8 risk 7). Budget them
  separately. tldr/05 decision 9 defines "cheaper".
- W8 proposed a canary *plan* (`plans/golden-path-canary/`); `PLAN.md` E11 uses Rust integration tests instead.
