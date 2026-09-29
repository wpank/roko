+++
id = "gap-00cc7f"
kind = "gap"
title = "TL;DR: re-pin tags and refresh statements to the whitepaper matrix after the 2026-09-29 merges"
status = "open"
triage = "unverified"
severity = "p2"
goal = "whitepaper"
size = "M"
subsystem = ["tldr"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (22:55, coordinator after gap-fcea53)"
anchors = ["tmp/cybernetic-harness/tldr/00-README.md", "tmp/cybernetic-harness/tldr/03-MECHANISMS.md", "tmp/cybernetic-harness/tldr/04-FRONTIER-PLANS-CHEAP-EXECUTES.md", "tmp/cybernetic-harness/tldr/05-GAPS-AND-PROPOSALS.md", "tmp/cybernetic-harness/tldr/research/"]
lane = "paper"
parent = "spec-f8d196"
links = { depends_on = [], blocks = [], related = ["gap-fcea53", "gap-f3be74"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test $(grep -rhoE '(WIRED|PARTIAL|BROKEN|ORPHANED|BUILT-UNWIRED|MISSING|REMOVED)@(98ee1418f|a17d4dadd)' tmp/cybernetic-harness/tldr/*.md | wc -l) -eq 0"
+++

## Problem

The TL;DR's 138 status tags are pinned at `98ee1418f` or `a17d4dadd`. Since then the day's merges changed many facts:

- the guard and isolation work;
- attempt records, settled verdicts and learners reading them;
- router persistence;
- the scheduler's write-set exclusion;
- the frontier planner;
- pinned acceptance tests and spec-quality checks;
- turn caps;
- the benchmark's families, driver, arms, proxy, ledger, report and verifier CI;
- the matrix re-pin itself (gap-fcea53, `ed0c33bd5`).

## Why it matters

The TL;DR is the entry point for both papers. Epic spec-f8d196.

## Where

The top-level TL;DR files (research notes may keep dated pins), with tags from `docs/whitepaper/data/mechanisms.toml`.

## Current state

Tags are at `98ee1418f` and `a17d4dadd`.

## Plan

1. Re-pin the top-level files' tags to the matrix pin.
2. Refresh each changed statement, and add a changelog entry.
3. Research notes keep their dated pins, but get a one-line "changed since" pointer wherever the matrix moved.

## Done when

- [ ] The top-level TL;DR matches the matrix at its pin.
- [ ] The `[[verify]]` command passes.
