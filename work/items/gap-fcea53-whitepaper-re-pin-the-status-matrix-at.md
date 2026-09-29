+++
id = "gap-fcea53"
kind = "gap"
title = "Whitepaper: re-pin the status matrix at a post-merge commit before the whitepaper-v1 tag"
status = "open"
triage = "unverified"
severity = "p2"
goal = "whitepaper"
size = "M"
subsystem = ["paper"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (16:12, wk-wp-review's report on gap-8d2c79)"
anchors = ["docs/whitepaper/data/mechanisms.toml", "docs/whitepaper/appendix-status-matrix.md", "docs/whitepaper/figures/fig3-status-matrix.svg", "tools/status_matrix.py"]
lane = "paper"
parent = "spec-ce1484"
links = { depends_on = ["gap-8d2c79"], blocks = [], related = ["gap-8d2c79", "gap-8117a8", "gap-35a614"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "python3 tools/status_matrix.py --check && ! grep -q 'pin = \"a17d4dadd\"' docs/whitepaper/data/mechanisms.toml && python3 tools/paperlint.py --strict --require-status reviewed docs/whitepaper/*.md"
+++

## Problem

The status matrix and every whitepaper tag are pinned at `a17d4dadd`. Several fixes merged after that pin, so the rows may now move (wk-wp-review, 2026-09-29):

- IS4 and IS5: bug-7de5df and bug-a66941, merged at `0728a2817`.
- LM1 and RC6: gap-528762, merged at `1c5371f9a`, and bug-470de8.
- Anything else the current wave lands.

The tool also checks items at the pin. So rows SS6 (gap-0d64d5) and IS6 (gap-8f8544) can't name their new items until the pin moves.

## Why it matters

The `whitepaper-v1` tag should fix text whose tags describe the code at that commit (gap-8117a8). Epic spec-ce1484.

## Where

- `docs/whitepaper/data/mechanisms.toml`: the rows and `pin`.
- The generated appendix and Figure 3.
- Figures 1 and 2: `--check` compares their marks with the matrix.
- Every section that quotes a tag.

## Current state

Everything is pinned at `a17d4dadd`, and the matrix is marked reviewed (gap-8d2c79).

## Plan

1. **When:** right before tagging, after the current wave's fixes merge and pass the batch check.
2. **Re-check rows:** re-check every row against the new commit. Update the tags, the next ids (SS6 → gap-0d64d5, IS6 → gap-8f8544) and the pin. Record each tag change and its evidence.
3. **Regenerate:** regenerate the appendix and Figure 3, and update Figures 1 and 2 where marks changed.
4. **Sections:** update every section tag to the new pin. Keep the text within budget.
5. **Checks:** run `status_matrix.py --check` and `paperlint --strict --require-status reviewed`.

## Done when

- [ ] The matrix and all tags are pinned at the new commit.
- [ ] The `[[verify]]` command passes.
