+++
id = "gap-cdf3fc"
kind = "gap"
title = "Correct the 16 docs claims that the code or the literature contradicts"
status = "open"
triage = "verified"
severity = "p1"
goal = "tooling"
size = "M"
subsystem = ["docs/v3"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e15"
discovered_from = "tmp/cybernetic-harness/tldr/05-GAPS-AND-PROPOSALS.md (P0 #7; §5 doc corrections)"
anchors = ["docs/v3/00-INDEX.md", "docs/v3/04-EXECUTION.md", "docs/v3/07-GATES.md", "docs/v3/08-LEARNING.md", "docs/v3/11-AFFECT.md", "docs/v3/12-SAFETY.md", "docs/v3/16-COORDINATION.md", "docs/v3/19-TOOLS-PLUGINS.md", "docs/v3/20-GATEWAY.md", "docs/v3/30-CONDUCTOR.md", "docs/v3/31-SELF-HOSTING.md", "CLAUDE.md"]
lane = "docs"
parent = "spec-9a3131"
links = { depends_on = [], blocks = [], related = ["bug-09690f", "gap-b23ebd", "gap-38a529"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'This is law' docs/v3/30-CONDUCTOR.md && ! grep -q 'All eight loops are wired' docs/v3/08-LEARNING.md && ! grep -q 'at least 45%' docs/v3/08-LEARNING.md && ! grep -q 'is 10/10 complete' docs/v3/11-AFFECT.md && ! grep -q 'Graph of Graphs is just a Graph' docs/v3/00-INDEX.md && ! grep -q 'via plonky2' docs/v3/12-SAFETY.md && ! grep -q '2607.25891' docs/v3/07-GATES.md && ! grep -q 'inside isolated git worktrees' docs/v3/04-EXECUTION.md && ! grep -q '124/124' docs/v3/00-INDEX.md && ! grep -q 'uses graph templates via' CLAUDE.md"
+++

## Problem

`docs/v3` and `CLAUDE.md` state things that the code or the literature contradicts. tldr/05 §5 lists them; each one was
still present at `41c7ffbd6`:

| Claim (where) | Reality (research note) |
|---|---|
| 19 gates and a 7-rung pipeline, all wired (`07-GATES.md:9`) | Plan tasks run only their authored verify commands; only tests reach `run_gate_once` (B4) |
| Tasks run inside isolated git worktrees (`04-EXECUTION.md:5`) | The shared working tree is the default (B2) |
| Five replan strategies (`31-SELF-HOSTING.md:299`) | Nothing outside its own file references `ReplanController` (B2, B5) |
| The conductor is "built and wired… This is law." (`30-CONDUCTOR.md:10`, `:248`) | No production caller (A5) |
| "All eight loops are wired." (`08-LEARNING.md:1018`) | Two are closed (A4, F) |
| Affect is wired and E23 is 10/10 (`11-AFFECT.md:3`, `:9`) | Partly wired; `modulate_dispatch` is a stub (A4) |
| A Graph of Graphs is just a Graph (`00-INDEX.md:150`, `:259`) | There is no `impl Cell for Graph` (A1) |
| 124/124 tasks, 48/48 epics (`00-INDEX.md:9`, `:760`; `31-SELF-HOSTING.md:639`) | The runner is credited with 0.22–0.67% of the Rust lines added (A1) |
| An exponential flywheel, autocatalytic compounding (`16-COORDINATION.md`, `08-LEARNING.md`) | The literature contradicts this; the defensible claim is bounded, audited improvement (C4) |
| Hindsight recovers at least 45% (`08-LEARNING.md:720`) | No relabelling code; the number has no source (A4) |
| ZK proofs via plonky2 (`12-SAFETY.md:1350`) | No code (A3) |
| The gateway is COMPLETE as Roko's inference path (`20-GATEWAY.md:11`) | Only `roko serve` uses it, never plan runs (A2, B3) |
| `roko run` uses `WorkflowGraphController` (`CLAUDE.md:75`) | It writes a one-task plan and calls `run_graph_plan` (A7, B8) |
| WASM plugin hooks are live (`19-TOOLS-PLUGINS.md:12`, §7) | The runtime was removed with Runner-v2 (A6) |
| Composable and domain-agnostic (`00-INDEX.md:27`) | Code-first today (B8) |
| arXiv:2607.25891 shows partial-pass scoring is 2.3× faster (`07-GATES.md:983`) | That paper is an unrelated benchmark corpus (A3) |

## Why it matters

The whitepaper, newcomers and agents read these pages as fact, and `CLAUDE.md` briefs every agent session. This is
tldr/05 P0 #7. It is part of epic spec-9a3131.

## Where

The files in the table.

## Current state

The table has 16 rows; this item covers all 16. The README's copy of the 124/124 figure
belongs to bug-09690f.

## Plan

1. For each claim, write what the code does now, citing the file, or mark the feature as planned or parked. Don't
   delete chapters, and keep each `> **Implementation status:**` marker truthful.
2. Fix the `depth/` pages that repeat a claim where that is cheap, and list the rest in the closing evidence.

## Done when

- [ ] All 16 rows are corrected, or a row is explicitly left for a named item.
- [ ] The `[[verify]]` command passes.

## Notes

- **Docs lane.** gap-452185 also edits `CLAUDE.md`, so merge one after the other.
- **Related:** gap-b23ebd (the citation errors across `docs/v3`) and gap-38a529 (the controller that is never
  driven).
