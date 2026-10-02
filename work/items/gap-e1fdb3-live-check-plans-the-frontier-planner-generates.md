+++
id = "gap-e1fdb3"
kind = "gap"
title = "Live check: plans the frontier planner generates from five sample inputs score at least 70"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
hold = "needs Will's approval for live planner calls, and waits for session roko-7d's plan-first generation (workflow-audit migration)"
subsystem = ["roko-cli/plan_generate"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire 3223 (blocked in wave 3, PK18)"
anchors = ["crates/roko-cli/src/plan_generate.rs", "benchmarks/viabilitybench/speclint/speclint.py"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -d crates/roko-cli/tests/fixtures/spec-gen-sample && python3 benchmarks/viabilitybench/speclint/speclint.py crates/roko-cli/tests/fixtures/spec-gen-sample --out - 2>/dev/null | python3 -c 'import json,sys,collections; s=collections.defaultdict(list); [s[r[\"plan_id\"]].append(r[\"score\"]) for r in (json.loads(l) for l in sys.stdin if l.startswith(\"{\"))]; sys.exit(0 if len(s) >= 5 and all(sum(v)/len(v) >= 70 for v in s.values()) else 1)'"
+++

## Problem

S07.10's acceptance is behavioural: plans the frontier planner generates from five sample inputs should average at least
70 per plan on the spec-quality rubric. PK18 (gap-e4bfbf, merged in 509e3e807) landed every piece the check needs
(3217's planner config, 3218's scoring and retries, 3219's TSS prompt, 3221/3222's planner-written accept tests), but
nothing has measured what the generator produces. Backlog task 3223 was blocked in wave 3: it needs a roko built from
main and live planner calls.

## Why it matters

It is the end-to-end check that the frontier planner writes specs a cheap model can execute (tldr design rule 3).

## Where

Plan generation (moving out of `crates/roko-cli/src/prd.rs` under session roko-7d's workflow-audit migration),
scored by `benchmarks/viabilitybench/speclint/speclint.py` at the linter in force.

## Current state

At 509e3e807 the generator scores and retries, but no generated sample set exists:
`crates/roko-cli/tests/fixtures/spec-gen-sample/` is absent.

## Plan

1. After roko-7d's change lands, pick five inputs of different kinds (a CLI flag, a serve route, a data change, a
   refactor, a docs task). Its plan-first generation replaces the PRD inputs of the original task.
2. Generate each plan with a roko built from main in a scratch workspace, copy the five `tasks.toml` (and `accept/`
   files) into `crates/roko-cli/tests/fixtures/spec-gen-sample/<slug>/`, and score them.
3. Record the per-plan means, bands, planner model and cost or subscription use in the commit message.
4. If a plan falls short, file what the planner missed. Never hand-edit the fixtures.

## Done when

- [ ] Five generated plans are committed, and each plan's mean score is at least 70.
- [ ] The `[[verify]]` command passes.

## Notes

- The live planner calls are spend: get Will's approval first (subscription or API).
- Full spec: `tmp/backlog/2026-10-02-complete-and-wire/3223-live-check-five-prd-plans-score-70.md`.
