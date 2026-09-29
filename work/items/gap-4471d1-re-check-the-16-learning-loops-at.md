+++
id = "gap-4471d1"
kind = "gap"
title = "Re-check the 16 learning loops at a post-merge commit and refreeze B5 for whitepaper §5.2"
status = "open"
triage = "unverified"
severity = "p2"
goal = "whitepaper"
size = "M"
subsystem = ["paper"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (21:50, wk-repin's report on gap-fcea53)"
anchors = ["docs/whitepaper/evidence/learning-loops-B5.md", "docs/whitepaper/05-cybernetic-mechanisms.md", "tmp/cybernetic-harness/paper/sections/07-results-regulation.md", "tmp/cybernetic-harness/tldr/research/B5-learning-loops-v2-vs-graph.md"]
lane = "paper"
parent = "spec-ce1484"
links = { depends_on = ["gap-8f6206"], blocks = [], related = ["gap-fcea53", "gap-29a64e", "gap-b64fba"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q '98ee1418f' docs/whitepaper/05-cybernetic-mechanisms.md && python3 tools/paperlint.py --strict --require-status reviewed docs/whitepaper/*.md && (cd docs/whitepaper/evidence && shasum -a 256 -c SHA256SUMS)"
+++

## Problem

Whitepaper §5.2 cites the frozen B5 re-check at `98ee1418f` for the 16 Runner-v2 learning loops: 2 wired, 5 partial, 7 orphaned, 1 broken, 1 built but unwired (`docs/whitepaper/evidence/learning-loops-B5.md`). Later merges likely move several of those loops (wk-repin, 2026-09-29):

- the router now learns from failures (91cfe0467);
- consolidation is off by default (d5b17759b, 584abd414);
- the reflex credit is fixed (328123077);
- the WAL and router persistence changed (b11ca807d, 277cc0d56);
- learners are about to read the settled verdict (gap-8f6206).

## Why it matters

The loop census is the whitepaper's and the research paper's main evidence for the cybernetics claim. It must describe the code at the tag. Epic spec-ce1484.

## Where

The frozen evidence file and its `SHA256SUMS` entry, whitepaper §5.2, research paper §7 (C7.18), and `tmp/cybernetic-harness/tldr/research/B5`.

## Current state

B5 is frozen at `98ee1418f`, and the matrix is re-pinned at `ed0c33bd5`.

## Plan

1. After gap-8f6206 merges, re-check each of the 16 loops against the code at that commit, using B5's method.
2. Write a new frozen copy with its sha256, and keep the old copy as history.
3. Update §5.2, research paper C7.18 and the tldr note to the new counts, each with its pin.
4. Pass `paperlint --strict` and `status_matrix --check`.

## Done when

- [ ] The loop counts are re-derived at a post-merge commit and cited from a frozen copy.
- [ ] The `[[verify]]` command passes.
