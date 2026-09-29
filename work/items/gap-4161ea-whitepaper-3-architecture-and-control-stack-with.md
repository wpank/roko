+++
id = "gap-4161ea"
kind = "gap"
title = "Whitepaper §3 Architecture and control stack, with Figure 1"
status = "open"
triage = "verified"
severity = "p1"
goal = "whitepaper"
size = "M"
subsystem = ["docs/whitepaper"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e1"
discovered_from = "tmp/cybernetic-harness/tldr/02-HOW-IT-WORKS.md (the flow; the control stack)"
anchors = ["docs/whitepaper/03-architecture.md"]
lane = "paper"
parent = "spec-ce1484"
links = { depends_on = ["gap-0191eb", "gap-af0b57"], blocks = [], related = ["gap-d1d92c", "gap-35a614", "gap-cdf3fc"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f docs/whitepaper/03-architecture.md && grep -q 'Figure 1' docs/whitepaper/03-architecture.md && test -f tools/paperlint.py && python3 tools/paperlint.py --strict docs/whitepaper/03-architecture.md"
+++

## Problem

The whitepaper needs a section that shows how Roko is built. It covers two things:
- the path from a plan to verified, recorded results;
- the control stack around that path: feedforward, sensors, comparators, fast and slow regulators, and second-order
  audits. Each layer says what runs today.

## Why it matters

It gives §4 (the golden path) and §5 (the mechanisms) their vocabulary. It is also where docs/v3 claimed too much, in
two headline claims (tldr/02; tldr/05 §5):
- "a Graph of Graphs is just a Graph", although nothing implements `Cell` for `Graph`;
- "every Cell is a learner".

## Where

- **The file:** `docs/whitepaper/03-architecture.md` (new; gap-0191eb creates it as a stub).
- **The code it describes:**
  - `crates/roko-graph/src/convert.rs::plan_to_graph` and `crates/roko-graph/src/engine.rs`;
  - `crates/roko-cli/src/graph_task_dispatch.rs`;
  - `crates/roko-agent/`, `crates/roko-gate/` and `crates/roko-learn/`.

## Current state

Checked at `41c7ffbd6`. tldr/02, written at `d9e79e9d8`, has the flow, the building blocks, the six-layer control
stack and a table of what `.roko/` records. Two points in it are now stale:
- `learn/gate-thresholds.json` is no longer "read by nothing": it sets retry budgets (`99adacd6d`);
- nodes now start as soon as their own dependencies settle (`445a60d0d`).

The other sources are the research draft's §4.1, written as designed, and W9 PW06, which rewrites §4.1 as the control
stack plus the loop.

## Plan

1. **The architecture:** plan (`tasks.toml`) → Graph (a DAG of Cells) → per-task dispatch (context, model, agent,
   verify, retry) → records under `.roko/`. Then the surfaces: CLI, TUI, HTTP/SSE, portal and ACP.
2. **The control stack:** a table of six layers, showing target and today. Tag each layer at a commit, re-derived from
   the code.
3. **What Roko records,** with only the caveats that still hold.
4. **A Figure 1 draft** (architecture and control stack): a caption plus a text diagram in a fenced block.
   gap-d1d92c draws the SVG.
5. **The two docs/v3 claims:** leave them out, or tag them DOCS-ONLY.

## Done when

- [x] Every crate path and identifier exists at HEAD, or is tagged as designed.
- [ ] The `[[verify]]` command passes.

## Notes

- **Needs `tools/paperlint.py`** (gap-af0b57) to close.
- **Tags must agree with the status matrix** (gap-35a614). The review (gap-8d2c79) checks this.
- Lane `paper`; no hot files.
- **Drafted** on `work/gap-4161ea` at `9b739b611`; the reworded claims AR2, AR5, AR7, AR8 and AR9 are recorded in the
  README at `b3f1f6d6e`. Tags are at `a17d4dadd`, the status matrix's pin, re-checked with read-only greps at
  `18946b94c`; the code these rows describe is the same at both. `tools/paperlint.py` is not at BASE, so the
  `[[verify]]` can't run yet. gap-af0b57's uncommitted draft passes `--strict` on the file: 837 words, 1.20× the
  budget, counting the fenced figure (about 757 without it, the README's rule). Close once paperlint merges.
