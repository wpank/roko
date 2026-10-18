+++
id = "gap-d7b6a4"
kind = "gap"
title = "The research paper exists only in untracked tmp/cybernetic-harness/paper/, with no history and no backup"
status = "open"
triage = "verified"
severity = "p1"
goal = "whitepaper"
size = "S"
subsystem = ["papers/research-paper"]
created = 2026-10-15
updated = 2026-10-18
last_verified = 2026-10-15
source = "status-quo-2026-10-15#finding-31"
anchors = ["tmp/cybernetic-harness/paper/00-README.md", "tmp/cybernetic-harness/paper/sections/", "tmp/cybernetic-harness/paper/tools/build.sh"]
lane = "paper"
links = { depends_on = [], blocks = [], related = ["spec-f8d196", "gap-ac1aed", "gap-394f28"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test \"$(git -C tmp/cybernetic-harness/paper rev-parse --show-toplevel 2>/dev/null)\" = \"$PWD/tmp/cybernetic-harness/paper\" || git ls-files --error-unmatch docs/research-paper/sections/00-abstract.md >/dev/null 2>&1"
+++

## Problem

The research paper, *Loops Within Loops* (20 files: sections 00-14 and appendices A-E, 5 figures, the bibliography,
`CLAIMS-EVIDENCE.md`, about 17,700 words of main text plus 5,100 of appendices, 6.4 MB with its archive), exists only
in `tmp/cybernetic-harness/paper/`. `tmp/` is gitignored (`.gitignore:69`), and `git ls-files` lists no file under
`tmp/cybernetic-harness/`. There is no version history and no copy anywhere else: a `git clean -x`, a lost disk or a
bad edit loses it. The empirical draft it replaced survives only in the same folder
(`paper/archive/2026-10-02-empirical-draft/`). Status-quo assessment 2026-10-15 at 7b6a5af44, finding 31
(`tmp/status-quo/2026-10-15/backlog/A12-95-papers.md` lines 23 and 128).

## Why it matters

Goal `whitepaper`. The paper is one of the programme's two main outputs. Will's 2026-10-15 decision 4 starts an
independent review of both papers with fixes (gap-ac1aed); edits from that review need history so they can be
reviewed and undone. The whitepaper, by contrast, is tracked in `docs/whitepaper/`.

## Where

- `tmp/cybernetic-harness/paper/` (sections, bibliography, figures, tools, archive).
- The companion report (`tmp/cybernetic-harness/companion-audit/`) is in the same state, but Will's decision 4 drops
  the companion, so only the paper is in scope.

## Current state

The paper is drafted in full and edited (status `edited`, not final); publication is decided after the independent
review (decision 9539).

## Plan

The choice is Will's (see Notes). Options:

1. **A private git repository for the paper folder** (recommended): `git init` in `tmp/cybernetic-harness/paper/`,
   commit everything, and push to a private remote Will creates (or at least a second local bare repo on another
   disk). It gives history and a backup without publishing anything. `build.sh` reads the working files, so nothing
   else changes.
2. **Track it in this repo as `docs/research-paper/`**: history comes with the repo, but the repo is public on
   `origin`, so the draft becomes public with the next push. Publishing is Will's call (decision 9539 is asked
   after the review), and paths in `build.sh`, paperlint and the paper lane would move.
3. **A dated archive** (a tarball outside the repo, for example in cloud storage, refreshed after each editing
   session): a backup, but no history.

## Done when

The `[[verify]]` command passes: the paper folder is its own git repository (option 1) or tracked under
`docs/research-paper/` (option 2). For option 3, replace the verify with a check on the archive's manifest.

## Notes

**Will's decision:** which option, and for option 1, where the private remote lives. Creating a remote or pushing the
draft anywhere public is an outward-facing action that needs his approval. Don't move the folder while gap-ac1aed's
review is editing it. The paper lane's paths (`work/lanes.toml`) list `tmp/cybernetic-harness/paper/**`; update them
if option 2 moves the paper.

## Decision (2026-10-18, Will)

Option 1: a private git repository for the paper folder. The coordinator runs git init and commits locally in tmp/cybernetic-harness/paper/; Will creates the private GitHub repository and gives its URL, and nothing is pushed until he says so.
