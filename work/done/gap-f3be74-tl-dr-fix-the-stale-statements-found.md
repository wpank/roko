+++
id = "gap-f3be74"
kind = "gap"
title = "TL;DR: fix the stale statements found by the second refresh pass"
status = "done"
triage = "verified"
severity = "p3"
goal = "whitepaper"
size = "S"
subsystem = ["tldr"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "751bc8f13"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (15:57, wk-tldr2's report on gap-4d516a)"
anchors = ["tmp/cybernetic-harness/tldr/00-README.md", "tmp/cybernetic-harness/tldr/04-FRONTIER-PLANS-CHEAP-EXECUTES.md", "tmp/cybernetic-harness/tldr/05-GAPS-AND-PROPOSALS.md", "tmp/cybernetic-harness/tldr/research/B7-real-run-evidence.md"]
lane = "paper"
parent = "spec-f8d196"
links = { depends_on = [], blocks = [], related = ["gap-4d516a", "gap-b64fba"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'agents can read your key files' tmp/cybernetic-harness/tldr/00-README.md && ! grep -q '| B | Roko with a frontier executor |' tmp/cybernetic-harness/tldr/04-FRONTIER-PLANS-CHEAP-EXECUTES.md && ! grep -q 'a `max_parallel` of 3–4' tmp/cybernetic-harness/tldr/research/B7-real-run-evidence.md"

[closed]
at = 2026-09-29
by = "coordinator (session 7622b882)"
evidence = "TL;DR (tmp, wk-tldr2): ten fixes. 00-README item 8 rebuilt from matrix rows IS4-IS6 and the closed bug-a66941/bug-7de5df (0728a2817), best effort without an OS sandbox (gap-8f8544 held), no guard for Codex/Cursor/Gemini; 04 'How to prove it' now three ViabilityBench arms plus the 48-task Roko+frontier probe (D1/S09); 05 §6 marks decisions 3, 6, 9, 12, 13 settled with sources; B7 TL;DR caps 2-4; plus 6 more stale key-file and arm lines in 01, 04 and 05. Verify passes; the frozen B7 copy still matches SHA256SUMS."
+++

## Problem

The pass for gap-4d516a found four more statements that went stale on 2026-09-29:

- `00-README.md` item 8 still says agents can read your key files, and that the git guard misses `reset --hard`. bug-a66941 and bug-7de5df closed at `0728a2817`.
- `04-FRONTIER-PLANS-CHEAP-EXECUTES.md`, "How to prove it", lists Roko with a frontier executor as full arm B. Decision D1 makes it a 48-task probe, which assessment W10 also flagged.
- `05-GAPS-AND-PROPOSALS.md` §6 doesn't mark the five decisions Will settled on 09-29: 3, 6, 9, 12 and 13. The 00-README changelog names them with sources.
- The TL;DR of `research/B7` says within-plan `max_parallel` was 3–4, but the note's own corrections say 2–4.

## Why it matters

The TL;DR is the entry point for readers of both papers. Epic spec-f8d196.

## Where

The four files above. Status tags follow `docs/whitepaper/data/mechanisms.toml`.

## Current state

All four statements are still in the files.

## Plan

1. **Item 8:** rewrite it from the matrix's isolation rows and the two closed bugs. Keep the missing OS sandbox (gap-8f8544, on hold).
2. **04:** make B a probe, as D1 and S09 do.
3. **05 §6:** mark decisions 3, 6, 9, 12 and 13 as settled, each with its source.
4. **B7:** change 3–4 to 2–4 in the TL;DR. Edit the tldr copy only; the frozen copy in `docs/whitepaper/evidence/` stays byte-for-byte.

## Done when

- [ ] The four statements are fixed.
- [ ] The `[[verify]]` command passes.
