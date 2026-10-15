+++
id = "gap-6a83de"
kind = "gap"
title = "The rewritten whitepaper has an independent review, the fixes it asks for, and a strict paperlint pass"
status = "open"
triage = "verified"
severity = "p1"
goal = "whitepaper"
size = "M"
subsystem = ["docs/whitepaper"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
source = "status-quo-2026-10-15#decision-4"
anchors = ["docs/whitepaper/REVIEW.md", "tools/paperlint.py"]
lane = "paper"
parent = "spec-ce1484"
links = { depends_on = [], blocks = [], related = ["gap-8117a8", "gap-ac1aed"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q '^## Review of the 2026-10-02 rewrite' docs/whitepaper/REVIEW.md && sed -n '/^## Review of the 2026-10-02 rewrite/,$p' docs/whitepaper/REVIEW.md | grep -q '^Verdict: accept' && python3 tools/paperlint.py --strict --require-status reviewed docs/whitepaper"
+++

## Problem

The whitepaper was rewritten as a design paper on 2026-10-02 and committed in 7898493f2 (2026-10-08), so it is public
on origin/main. Nobody independent has reviewed it since. `docs/whitepaper/REVIEW.md` still ends with the old text's
reviews ("Verdict: accept" at line 91, "Verdict: revise" at line 159), and every section's status header says draft.

## Why it matters

Goal `whitepaper`. It is the public description of Roko. Will decided on 2026-10-15 (`tmp/status-quo/2026-10-15/DECISIONS-2026-10-15.md`,
decision 4) that both papers get an independent review before anything else is published. gap-8117a8 (publish) waits
on this.

## Where

- `docs/whitepaper/*.md`: 11 sections plus README.md.
- `docs/whitepaper/REVIEW.md`: the review record.
- `tools/paperlint.py`: the strict lint, with `--require-status reviewed`.

## Current state

The 2026-10-15 status-quo assessment checked 210 of the whitepaper's claims against the code at 7b6a5af44
(`tmp/status-quo/2026-10-15/papers/D1-whitepaper-00-05.md`, `D2-whitepaper-06-10.md` and their `-claims.jsonl`):

| Tag | Claims |
|---|---|
| True | 116 |
| True only when switched on | 20 |
| Partly true | 59 |
| Contradicted | 5 |
| Not built | 2 |

`tmp/status-quo/2026-10-15/papers/F-paper-vs-paper.md` adds:
- The text is at 1.034x its word budget.
- 8 of the 11 `evidence/` files are orphaned, and one opens with "The golden path ran…".
- "The operator's checkout is never touched" contradicts the whitepaper's own evidence.
- Agent D1 flagged a framing tension: honest wording for opt-in mechanisms against the 10-02 "no not-yet" rule.

## Plan

1. A reviewer who did not write the text (a fresh agent, or a person) reads every section against the code at HEAD,
   starting from the D1/D2 claims files.
2. Fix every CONTRADICTED and NOT-BUILT claim, and every PARTLY claim whose detail is wrong. Will decided the papers
   stay as written for the mechanisms that the staged turn-on epic (status-quo-2026-10-15, decision 2) switches on. So
   don't rewrite those as opt-in; link the review to that epic instead.
3. Trim to budget, drop or re-cite the orphaned evidence files, and fix the "never touched" sentence.
4. Append a "## Review of the 2026-10-02 rewrite" section to REVIEW.md with findings, fixes and a verdict line. Set
   the sections' status headers to `reviewed`.

## Done when

The new REVIEW.md section ends in `Verdict: accept`, and `python3 tools/paperlint.py --strict --require-status
reviewed docs/whitepaper` passes (the `[[verify]]` command).

## Notes

Publishing (a release tag and PDF, gap-8117a8) is a separate decision for Will after this item closes. Don't push or
tag anything here.

2026-10-15 (status-quo assessment, finding 5): for the independent review (decision 4). The papers' "every tool call passes a seven-step guard" (research paper C0.4, C1.21, C3.12, C3.29; appendix A.1, Sensor and Bounds) holds only for API models and, by its PreToolUse hook, the Claude CLI. Codex is checked only after a command starts (its stream broker). Cursor runs `agent --force` with no check (`crates/roko-agent/src/cursor_cli_agent.rs:259`). Gemini CLI has no check. All three are contained only by per-task worktrees (decision 1214, `crates/roko-cli/src/graph_task_dispatch/failover.rs:33-42`). Evidence: `tmp/status-quo/2026-10-15/papers/D3-paper-00-03.md:76-81,222-224` and `papers/D7-paper-appendices.md:56,91-93`. Code gaps were filed from the same finding (Cursor; Codex and Gemini).
