+++
id = "gap-08d9b2"
kind = "gap"
title = "Whitepaper: final matrix re-pin right before the whitepaper-v1 tag"
status = "open"
triage = "unverified"
severity = "p2"
goal = "whitepaper"
size = "S"
subsystem = ["paper"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (23:15, wk-tldr2 on gap-00cc7f; wk-wp-s7 on gap-4471d1)"
anchors = ["docs/whitepaper/data/mechanisms.toml", "docs/whitepaper/appendix-status-matrix.md"]
lane = "paper"
parent = "spec-ce1484"
links = { depends_on = ["gap-fcea53"], blocks = [], related = ["gap-fcea53", "gap-8117a8", "gap-4471d1"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "python3 tools/status_matrix.py --check 2>&1 | grep -v 'at ed0c33bd5' | grep -q 'rows ok at' && python3 tools/paperlint.py --strict --require-status reviewed docs/whitepaper/*.md"
+++

## Problem

The matrix was re-pinned at `ed0c33bd5` (gap-fcea53). Several merges after that pin make some row notes stale (wk-tldr2 and wk-wp-s7, 2026-09-29):

- RC2 and QA2 still say learning counts Unverified as a pass. gap-8f6206 and bug-c34782 fixed that (74eaf5c8f).
- IS4 and IS5 predate bug-62e7e6 and bug-0bc728 (9b1d46be4).
- Routing may move once bug-35379d's router-credit fix merges.
- The rest of the golden-path wave (tiers, tampering, telemetry, model truth, integration) may move more rows.

## Why it matters

The `whitepaper-v1` tag must describe the code at its pin. Epic spec-ce1484.

## Where

`docs/whitepaper/data/mechanisms.toml`, the generated appendix and Figure 3, Figures 1 and 2, and the section tags.

## Current state

Pinned at `ed0c33bd5`.

## Plan

1. Right before tagging, after the golden-path wave merges, re-pin at the head, following gap-fcea53's procedure.
2. Update the notes and tags that moved, and the section text that quotes them.
3. Pass `status_matrix --check` and `paperlint --strict --require-status reviewed`, and rebuild the PDF.

## Done when

- [ ] The matrix and every tag are pinned at the tag-candidate commit.
- [ ] The `[[verify]]` command passes.
