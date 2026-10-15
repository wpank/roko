+++
id = "bug-ac2a01"
kind = "bug"
title = "tools/status_matrix.py --check fails at HEAD: it still renders the whitepaper's status-matrix appendix and Figure 3, which the design-paper rewrite deleted"
status = "open"
triage = "verified"
severity = "p3"
goal = "whitepaper"
size = "S"
subsystem = ["tools/status_matrix"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
source = "status-quo-2026-10-15#finding-34"
anchors = ["tools/status_matrix.py", "tools/test_status_matrix.py", "docs/whitepaper/data/mechanisms.toml"]
lane = "paper"
links = { depends_on = [], blocks = [], related = ["gap-08d9b2", "gap-d4a1c1", "spec-ce1484"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "{ test ! -e tools/status_matrix.py && ! grep -rq 'status_matrix' tools/ docs/whitepaper/; } || python3 tools/status_matrix.py --check"
+++

## Problem

`python3 tools/status_matrix.py --check` exits 1 at HEAD: "docs/whitepaper/appendix-status-matrix.md and
docs/whitepaper/figures/fig3-status-matrix.svg are out of date". Its defaults (`tools/status_matrix.py:44-45`, `OUT`
and `SVG`) still point at the appendix and Figure 3, which the whitepaper's design-paper rewrite deleted
(`7898493f2`, 2026-10-08; the paper framing has no status tags). `docs/whitepaper/data/mechanisms.toml` and its 8
evidence files are left orphaned. Two items whose verifies called it (gap-08d9b2, gap-d4a1c1) have since been closed
as superseded; `spec-ce1484` still lists the tool, and `docs/whitepaper/REVIEW.md` mentions it. Status-quo assessment
2026-10-15 at 7b6a5af44, finding 34 (`tmp/status-quo/2026-10-15/backlog/A12-95-papers.md` row 9502 and line 139).

## Why it matters

Goal `whitepaper`: a checker that always fails teaches people to ignore it, and any verify that calls it can never
pass. The mechanism status data may still be useful as a dated record outside the paper.

## Where

`tools/status_matrix.py` (`DATA`, `OUT`, `SVG`, the `--check` path), `tools/test_status_matrix.py`,
`docs/whitepaper/data/mechanisms.toml`, `docs/whitepaper/REVIEW.md`, spec-ce1484's Tools line.

## Current state

No CI job runs the tool. Its unit tests use their own fixtures.

## Plan

Pick one (recommended: option 1, since the papers no longer carry status tags and the work graph is the live status):

1. **Retire it:** delete `tools/status_matrix.py` and `tools/test_status_matrix.py`, move `mechanisms.toml` and its
   evidence files to a dated record (for example `work/history/status-matrix-2026-10-02/`), and update REVIEW.md
   and spec-ce1484's text.
2. **Re-target it:** keep the tool, but render to a dated record outside the whitepaper and drop Figure 3's check, so
   `--check` passes.

## Done when

The `[[verify]]` command passes (it accepts either option).

## Notes

Don't touch the whitepaper's sections (decision 2 keeps the papers as written); only its data folder and REVIEW.md
mention the tool.
