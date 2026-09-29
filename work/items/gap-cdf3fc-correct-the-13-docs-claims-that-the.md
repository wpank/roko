+++
id = "gap-cdf3fc"
kind = "gap"
title = "Correct the 16 docs claims that the code or the literature contradicts"
status = "done"
triage = "verified"
severity = "p1"
goal = "tooling"
size = "M"
subsystem = ["docs/v3"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "1f0f73e1f"
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e15"
discovered_from = "tmp/cybernetic-harness/tldr/05-GAPS-AND-PROPOSALS.md (P0 #7; §5 doc corrections)"
anchors = ["docs/v3/00-INDEX.md", "docs/v3/04-EXECUTION.md", "docs/v3/07-GATES.md", "docs/v3/08-LEARNING.md", "docs/v3/11-AFFECT.md", "docs/v3/12-SAFETY.md", "docs/v3/16-COORDINATION.md", "docs/v3/19-TOOLS-PLUGINS.md", "docs/v3/20-GATEWAY.md", "docs/v3/30-CONDUCTOR.md", "docs/v3/31-SELF-HOSTING.md", "CLAUDE.md"]
lane = "docs"
parent = "spec-9a3131"
links = { depends_on = [], blocks = [], related = ["bug-09690f", "gap-b23ebd", "gap-38a529"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'This is law' docs/v3/30-CONDUCTOR.md && ! grep -q 'All eight loops are wired' docs/v3/08-LEARNING.md && ! grep -q 'at least 45%' docs/v3/08-LEARNING.md && ! grep -q 'is 10/10 complete' docs/v3/11-AFFECT.md && ! grep -q 'Graph of Graphs is just a Graph' docs/v3/00-INDEX.md && ! grep -q 'via plonky2' docs/v3/12-SAFETY.md && ! grep -q '2607.25891' docs/v3/07-GATES.md && ! grep -q 'inside isolated git worktrees' docs/v3/04-EXECUTION.md && ! grep -q '124/124' docs/v3/00-INDEX.md && ! grep -q 'uses graph templates via' CLAUDE.md"

[closed]
at = 2026-09-29
commit = "1f0f73e1f"
by = "wk-docs"
evidence = "1f0f73e1f corrects all 16 rows at their listed locations, each checked against the code at 7c556bc0a: 07-GATES header and section 5 (plan tasks run only authored verify commands via ShellGate; run_gate_once is test-only), 04-EXECUTION header and sections 10-11 (shared working tree; --worktree-per-task opt-in, never merged back; merge queue ORPHANED), 31-SELF-HOSTING status, section 3 and 6.2 (ReplanController BUILT-UNWIRED; portal build as the evidence), 30-CONDUCTOR status and section 3 (BUILT-UNWIRED; 'This is law' gone), 08-LEARNING section 11 (2 of 8 loops close; per-loop status lines), section 6 (no recovery rate) and section 10 (hypothesis, with arXiv:2607.14004, 2608.18066, 2607.12227 from docs/whitepaper/references.bib), 11-AFFECT status (PARTIAL; modulate_dispatch has no caller), 00-INDEX (no impl Cell for Graph; 48/48 and the task count withdrawn; code-first, PARTIAL), 16-COORDINATION section 11, 12-SAFETY 13.12 (DOCS-ONLY), 20-GATEWAY scope note (roko serve only), 19-TOOLS-PLUGINS status and section 7 (WASM hooks never run), 07-GATES 9.5 (citation withdrawn), CLAUDE.md:75-76 (roko run writes a one-task plan and calls run_graph_plan). The 00-INDEX crate, section 13 and chapter tables and 17 depth pages carry the same corrections. Left for other items: the README copy (bug-09690f); the wrong Messier title in REFERENCES.md:1282,:1862, 39-ROADMAP.md:93 and depth/39-references/17-process-reward-models.md:51, 24-additions-2025-2026.md:347 (gap-b23ebd); '19-gate pipeline' as the verifier in 26-HTTP-API.md:12, 35-ARCHITECTURE.md:205,:675, 31-SELF-HOSTING.md:286, depth/07-gates/evaluation-lifecycle.md:260-271 and depth/35-architecture/*; 'E23 10/10 complete' in 05-AGENT.md:10,:977, 29-HEARTBEAT.md:16, 39-ROADMAP.md:379 and depth/05-agent/*cognitive-autonomy-e23.md:3; WASM hooks 'live' in 39-ROADMAP.md:388 and 35-ARCHITECTURE.md:722; '48 epics accepted' in 35-ARCHITECTURE.md:6,:1048; CLAUDE.md:138 ('Single prompt through graph templates'), not changed because the item lists only CLAUDE.md:75. Added at the coordinator's request: 00-INDEX.md:127-129 'Every Cell is a learner' (A1: PARTIAL), qualified in 1031c1137 with sections 3.5 and P2: TaskExecutorCell has no predict(), AssessCell is the only production Cell that predicts, and CalibrationPolicy runs only in run_learning_subscriber, which has no production caller. Check: the item's [[verify]] passes."
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

- [x] All 16 rows are corrected, or a row is explicitly left for a named item.
- [x] The `[[verify]]` command passes.

## Notes

- **Docs lane.** gap-452185 also edits `CLAUDE.md`, so merge one after the other.
- **Related:** gap-b23ebd (the citation errors across `docs/v3`) and gap-38a529 (the controller that is never
  driven).
- **Added 2026-09-29 (coordinator, from the filer):** a 17th claim, `docs/v3/00-INDEX.md:127-129` "Every Cell is a
  learner" (A1 rates it PARTIAL: `TaskExecutorCell` has no `predict()`). Qualified in `1031c1137`, together with its
  repeats in 00-INDEX section 3.5 and principle P2.
