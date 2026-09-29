+++
id = "gap-7ee870"
kind = "gap"
title = "Research paper: re-pin status tags to the whitepaper matrix at its new pin"
status = "open"
triage = "unverified"
severity = "p2"
goal = "whitepaper"
size = "M"
subsystem = ["paper"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (22:55, coordinator after gap-fcea53)"
anchors = ["tmp/cybernetic-harness/paper/sections/"]
lane = "paper"
parent = "spec-f8d196"
links = { depends_on = [], blocks = [], related = ["gap-fcea53"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test $(grep -rhoE '(WIRED|PARTIAL|BROKEN|ORPHANED|BUILT-UNWIRED|MISSING|REMOVED)@a17d4dadd' tmp/cybernetic-harness/paper/sections/*.md | wc -l) -eq 0 && python3 tools/paperlint.py --budget 1.2 --check-identifiers tmp/cybernetic-harness/paper/sections/*.md"
+++

## Problem

The whitepaper's status matrix is re-pinned at `ed0c33bd5` (gap-fcea53, merged 366a7ccbe). It moved AU3 to PARTIAL and AU7 and IS4 to WIRED, and corrected overstatements about the router, verify-less tasks, write-set scheduling, the planner model and the guard. The research paper still carries 81 status tags pinned `@a17d4dadd`, and some statements the whitepaper has since corrected.

## Why it matters

The two papers must agree, and the research paper's tags must describe the code at a stated commit. Epic spec-f8d196.

## Where

Every section under `tmp/cybernetic-harness/paper/sections/`. The source of truth is `docs/whitepaper/data/mechanisms.toml` at its pin, plus the corrections gap-fcea53 lists.

## Current state

81 tags are at `a17d4dadd`. The rest sit at later commits that name specific merges.

## Plan

1. Re-pin every `@a17d4dadd` tag to the matrix pin, taking each value from its matrix row.
2. Apply the same text corrections gap-fcea53 made in the whitepaper wherever the paper repeats them.
3. Keep every `[[RESULT …]]` slot, and keep each file within 1.2× budget.
4. Regenerate CLAIMS-EVIDENCE.

## Done when

- [ ] No tag is pinned at `a17d4dadd`, and every tag matches the matrix.
- [ ] The `[[verify]]` command passes.
