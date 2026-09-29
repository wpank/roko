+++
id = "gap-b64fba"
kind = "gap"
title = "Refresh the TL;DR against the 2026-09-29 merges and fix three known errors"
status = "open"
triage = "verified"
severity = "p1"
goal = "whitepaper"
size = "M"
subsystem = ["tldr"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e18"
discovered_from = "tmp/cybernetic-harness/workstreams/assessment/W9-paper-workstream.md"
anchors = ["tmp/cybernetic-harness/tldr/"]
lane = "paper"
parent = "spec-f8d196"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'Re-checked at' tmp/cybernetic-harness/tldr/05-GAPS-AND-PROPOSALS.md && ! grep -q 'bug-28becc' tmp/cybernetic-harness/tldr/04-FRONTIER-PLANS-CHEAP-EXECUTES.md"
+++

## Problem

The tldr was written against `d9e79e9d8`. Since then the portal session has merged or closed several of the items it
names:
- the ready-queue scheduler (`bbf6517fc`);
- hermetic child environments (bug-7d7200);
- dispatch cost (bug-690dc6);
- learning-loop re-wires (gap-fdd27f, reg-3f5969, gap-5fb9a7, reg-06ae9f);
- SkipFailed (gap-96d348).

Three known errors also remain:
- tldr/04 names bug-28becc as a benchmark prerequisite, but it is not on the S08 path (W10);
- research note B5 says 13 ids "do not resolve", but they are parked (W3a);
- the docs-corrections count is 16, not 13.

## Why it matters

The whitepaper's status tags and the plan both read the tldr. Epic spec-f8d196.

## Where

`tmp/cybernetic-harness/tldr/` (00–06 and `research/B5`).

## Current state

Status tags as of `d9e79e9d8`.

## Plan

1. **Re-check** each scorecard row and each changed status tag against the current HEAD, and add
   "Re-checked at `<sha>`" to 05's scorecard.
2. **Fix** the three errors.
3. **Changelog:** add dated lines to 00-README.

## Done when

- [ ] The scorecard and status tags are current, each with its commit.
- [ ] The three errors are fixed, and the `[[verify]]` command passes.
