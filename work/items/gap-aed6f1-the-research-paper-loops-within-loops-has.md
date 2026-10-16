+++
id = "gap-aed6f1"
kind = "gap"
title = "The research paper Loops Within Loops has an independent review, the fixes it asks for, and a final lint pass"
status = "open"
triage = "verified"
severity = "p2"
goal = "whitepaper"
size = "M"
subsystem = ["tmp/cybernetic-harness/paper"]
created = 2026-10-15
updated = 2026-10-16
last_verified = 2026-10-16
source = "status-quo-2026-10-15#decision-4"
anchors = ["tmp/cybernetic-harness/paper/OUTLINE.md", "tmp/cybernetic-harness/paper/REVIEW-SKELETON.md", "tools/paperlint.py"]
lane = "paper"
parent = "spec-f8d196"
links = { depends_on = ["gap-8ff818"], blocks = [], related = ["gap-ac1aed", "dec-536bbd", "gap-6a83de", "gap-c22583", "gap-d2c478", "bug-f12b2d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f tmp/cybernetic-harness/paper/REVIEW.md && grep -q '^Verdict: accept' tmp/cybernetic-harness/paper/REVIEW.md && python3 tools/paperlint.py --strict tmp/cybernetic-harness/paper/sections"
+++

## Problem

The research paper *Loops Within Loops* (`tmp/cybernetic-harness/paper/`, 20 sections, last edited 2026-10-02) has
never had an independent review:
- every section is marked "edited", none "final";
- `paper/REVIEW-SKELETON.md` lists 11 open review items;
- `paperlint --strict` reports 81 problems: 55 ledger path formats, 25 unsourced numbers (dec-536bbd), and 1 false
  positive (`guardrail_max_retries` is CrewAI's setting, agent D7).

## Why it matters

Goal `whitepaper`. Will re-scoped PK101 (gap-ac1aed) on 2026-10-15 to an independent review of both papers, the
fixes it finds and a final lint pass, with publication asked separately afterwards (`tmp/status-quo/2026-10-15/DECISIONS-2026-10-15.md`,
decision 4).

## Where

- `tmp/cybernetic-harness/paper/sections/*.md` (untracked);
- `paper/OUTLINE.md`;
- `paper/REVIEW-SKELETON.md`;
- `tools/paperlint.py`.

## Current state

The 2026-10-15 assessment checked 515 claims of the paper against the code at 7b6a5af44
(`tmp/status-quo/2026-10-15/papers/D3-…`, `D4-…`, `D5-…`, `D6-…`, `D7-…`, `ALL-claims-summary.md`). The contradicted claims include:
- plan complexity choosing gate rungs (`run_gate_once` is test-only);
- "an outage never pushes work to a costlier tier" (failover climbs since bfc8eb72b);
- "one substrate that `roko replay` walks" (plan runs write no signals);
- "the portal build used isolated worktrees and integration" (done by hand per its own evidence);
- "no knowledge in the prompt by default".

§9's audit level is mostly true only when switched on (23 of 36 claims). `tmp/status-quo/2026-10-15/papers/F-paper-vs-paper.md` lists 11
small disagreements with the whitepaper and three inconsistencies inherited from FRAMING.

## Plan

1. A reviewer who didn't write the paper reviews it against the code at HEAD, using the D3–D7 claims files and
   REVIEW-SKELETON.md.
2. Fix every contradicted, not-built and wrong-detail claim. Mechanisms switched on by the staged turn-on epic
   (decision 2) keep their present-tense text.
3. Resolve the cross-paper disagreements with the whitepaper item.
4. Apply dec-536bbd's paperlint exemption (filed separately), then fix the remaining lint.
5. Write `paper/REVIEW.md` with the findings, the fixes and a verdict line.

## Done when

`paper/REVIEW.md` ends in `Verdict: accept`, and `python3 tools/paperlint.py --strict tmp/cybernetic-harness/paper/sections`
passes (the `[[verify]]` command).

## Notes

- The paper is untracked, with no history or backup (a separate item). Copy any file you rewrite into
  `tmp/cybernetic-harness/archive/<date>-review/` first.
- Publication (9539: authors, licence, arXiv) is Will's later decision.

2026-10-15 (status-quo assessment, finding 5): for the independent review (decision 4). The papers' "every tool call passes a seven-step guard" (research paper C0.4, C1.21, C3.12, C3.29; appendix A.1, Sensor and Bounds) holds only for API models and, by its PreToolUse hook, the Claude CLI. Codex is checked only after a command starts (its stream broker). Cursor runs `agent --force` with no check (`crates/roko-agent/src/cursor_cli_agent.rs:259`). Gemini CLI has no check. All three are contained only by per-task worktrees (decision 1214, `crates/roko-cli/src/graph_task_dispatch/failover.rs:33-42`). Evidence: `tmp/status-quo/2026-10-15/papers/D3-paper-00-03.md:76-81,222-224` and `papers/D7-paper-appendices.md:56,91-93`. Code gaps were filed from the same finding (Cursor; Codex and Gemini).

2026-10-16 (independent review, a Claude agent on work/gap-aed6f1): the review is written (`tmp/cybernetic-harness/paper/REVIEW.md`, 39 findings, each fixed in the paper except the staged turn-on epic's mechanisms, which decision 2 keeps as written). Originals are in `tmp/cybernetic-harness/archive/2026-10-16-review/paper/`. The verdict is `revise` only because main's `tools/paperlint.py` still flags numbers inside the nine "Claims ledger" tables; gap-8ff818's exemption (closed on its branch at `f2f1b6ef6`) removes them, and with its paperlint the sections lint clean. Once gap-8ff818 is on main and the strict lint passes, the last line of REVIEW.md becomes `Verdict: accept` with no paper change, and this item can close. Filed from the review: gap-c22583 (verdict records never fill the request and diff hashes), gap-d2c478 (figure text below 8 pt), bug-f12b2d (`--force-resume`'s help text).
