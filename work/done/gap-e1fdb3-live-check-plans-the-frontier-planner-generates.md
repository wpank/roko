+++
id = "gap-e1fdb3"
kind = "gap"
title = "Live check: plans the frontier planner generates from five sample inputs score at least 70"
status = "done"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/plan_generate"]
created = 2026-10-02
updated = 2026-10-16
last_verified = 2026-10-16
last_verified_rev = "1a6a2567a"
source = "tmp/backlog/2026-10-02-complete-and-wire 3223 (blocked in wave 3, PK18)"
anchors = ["crates/roko-cli/src/plan_generate/pipeline.rs::generate_plan", "benchmarks/viabilitybench/speclint/speclint.py"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -d crates/roko-cli/tests/fixtures/spec-gen-sample && python3 benchmarks/viabilitybench/speclint/speclint.py crates/roko-cli/tests/fixtures/spec-gen-sample --out - 2>/dev/null | python3 -c 'import json,sys,collections; s=collections.defaultdict(list); [s[r[\"plan_id\"]].append(r[\"score\"]) for r in (json.loads(l) for l in sys.stdin if l.startswith(\"{\"))]; sys.exit(0 if len(s) >= 5 and all(sum(v)/len(v) >= 70 for v in s.values()) else 1)'"

[closed]
at = 2026-10-16
at_ts = "2026-10-16T14:48:35Z"
commit = "1a6a2567a"
by = "commit trailer"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-16T08:07:16Z"
forced = false
model = "claude-opus-5-5"
evidence = "1a6a2567a adds crates/roko-cli/tests/fixtures/spec-gen-sample/{cli-flag,serve-route,data-change,refactor,docs}/ (tasks.toml + accept/, byte for byte): five plans written live by roko plan generate on 2026-10-16 with L1's roko built from main 5e32264ce, in a scratch copy of main (scratch HOME, so no ~/.roko/.env or global config; claude_cli the only reachable provider), planner claude-opus-5-5 via the Claude CLI on the Max login. speclint sq-3 per-plan means: cli-flag 100.0 A, serve-route 94.3 A, data-change 95.0 A, refactor 100.0 A, docs 100.0 A; no hard fails; the [[verify]] passes on 1a6a2567a. Two kinds needed a second generation: the serve-route prompt failed 3/3 on the Claude CLI's default 10-turn cap and was given as a short written spec (filed bug-09cc5f); the docs prompt's first generation failed after three drafts that did not validate, and the same prompt passed the second time (filed gap-f8a6ea; 8 drafts failed validation in all). Subscription use: 18 planner calls (13 clean, 5 turn-capped) + 3 episode-distiller calls (claude-sonnet-4-6), $0.00 billed, $7.35 API-equivalent (prices-2026-09-28). ~/.roko unchanged (listing and checksums before and after). Evidence: tmp/status-quo/2026-10-16-live/small-live-evidence/plan-gen-e1fdb3/."
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

Plan generation (`crates/roko-cli/src/plan_generate/pipeline.rs::generate_plan`, reached by
`roko plan generate "<request>"` and `roko run --plan --dry-run "<request>"`),
scored by `benchmarks/viabilitybench/speclint/speclint.py` at the linter in force.

## Current state

At 509e3e807 the generator scores and retries, but no generated sample set exists:
`crates/roko-cli/tests/fixtures/spec-gen-sample/` is absent.

## Plan

1. Pick five requests of different kinds (a CLI flag, a serve route, a data change, a refactor, a docs task),
   each a prompt or a short written spec. They replace the PRD inputs of the original task.
2. Generate each plan with `roko plan generate "<request>"` (a roko built from main, in a scratch workspace), copy the five `tasks.toml` (and `accept/`
   files) into `crates/roko-cli/tests/fixtures/spec-gen-sample/<slug>/`, and score them.
3. Record the per-plan means, bands, planner model and cost or subscription use in the commit message.
4. If a plan falls short, file what the planner missed. Never hand-edit the fixtures.

## Done when

- [ ] Five generated plans are committed, and each plan's mean score is at least 70.
- [ ] The `[[verify]]` command passes.

## Notes

- The live planner calls are spend: get Will's approval first (subscription or API).
- Full spec: `tmp/backlog/2026-10-02-complete-and-wire/3223-live-check-five-prd-plans-score-70.md`.
- 2026-10-02 (roko-7d): the workflow-audit migration (merge bfd36512f) removed the PRD pipeline, `roko do` and `roko develop`; `roko run` is the one entry point and plans come from a prompt. The hold now waits only on Will's approval for the live planner calls.

2026-10-15 (Will, 2026-10-15; tmp/status-quo/2026-10-15/DECISIONS-2026-10-15.md), decision 6: live frontier-planner calls approved. Hold lifted.
