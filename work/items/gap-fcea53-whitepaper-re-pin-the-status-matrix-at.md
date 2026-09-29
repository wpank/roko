+++
id = "gap-fcea53"
kind = "gap"
title = "Whitepaper: re-pin the status matrix at a post-merge commit before the whitepaper-v1 tag"
status = "done"
triage = "verified"
severity = "p2"
goal = "whitepaper"
size = "M"
subsystem = ["paper"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "12a8d7793"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (16:12, wk-wp-review's report on gap-8d2c79)"
anchors = ["docs/whitepaper/data/mechanisms.toml", "docs/whitepaper/appendix-status-matrix.md", "docs/whitepaper/figures/fig3-status-matrix.svg", "tools/status_matrix.py"]
lane = "paper"
parent = "spec-ce1484"
links = { depends_on = ["gap-8d2c79"], blocks = [], related = ["gap-8d2c79", "gap-8117a8", "gap-35a614"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "python3 tools/status_matrix.py --check && ! grep -q 'pin = \"a17d4dadd\"' docs/whitepaper/data/mechanisms.toml && python3 tools/paperlint.py --strict --require-status reviewed docs/whitepaper/*.md"

[closed]
at = 2026-09-29
commit = "12a8d7793"
by = "commit trailer"
evidence = "12a8d7793 re-checks all 71 rows at ed0c33bd5 and re-pins the matrix, appendix, Figures 1-3 and every section tag there: AU3 MISSING to PARTIAL, AU7 and IS4 PARTIAL to WIRED; SS6 names gap-0d64d5, IS6 gap-8f8544. Verify passes: status_matrix.py --check (71 rows ok at ed0c33bd5) and paperlint --strict --require-status reviewed (14 files clean)."
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

- [x] The matrix and all tags are pinned at the new commit.
- [x] The `[[verify]]` command passes.

## Notes

- 2026-09-29 (wk-repin): pinned at `ed0c33bd5`. Totals there: 23 wired, 22 partial, 2 broken, 7 orphaned, 7 built but
  unwired, 9 missing, 1 removed (was 21/23/2/7/7/10/1). `REVIEW.md` keeps its record of the review at `a17d4dadd`.
- The verify's middle check, `! grep -q 'pin = "a17d4dadd"'`, can never fail: the key is `pinned`, so the pattern
  never matches. `status_matrix.py --check` and the section tags carry the real check.
