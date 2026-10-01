+++
id = "gap-ff6e83"
kind = "gap"
title = "roko-graph finally.rs (GuaranteedFinallyController) is never compiled: lib.rs has no mod finally"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-graph"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "2293a5c63"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-a29711"
anchors = ["crates/roko-graph/src/finally.rs", "crates/roko-graph/src/lib.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-a29711"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -n 'mod finally' crates/roko-graph/src/lib.rs || ! test -e crates/roko-graph/src/finally.rs"
+++

## Problem

`crates/roko-graph/src/finally.rs` defines `GuaranteedFinallyController`, but `roko-graph/src/lib.rs` has no `mod finally`, so the file is never compiled. Code and docs that cite it describe dead code, and it can rot unnoticed.

## Plan

Wire it (add the module and a caller with a test) or delete it. If deleted, update anything that cites it.

## Done when

- The verify passes, and the workspace builds.

## Notes

- Reported on 2026-10-01 by wk-tamper, working on gap-a29711.
- 2026-10-01 (wk-tamper): implemented on work/gap-7147bb by deleting the file; cargo verification deferred to the
  batch check. The design has no place for it: `run_one_plan` already does a plan run's cleanup (on an interrupt it
  cancels the graph and stops in-flight agents with SIGTERM, then SIGKILL; then it writes the terminal checkpoint and
  closes the run manifest), and the controller would have duplicated that through host releasers nothing provided.
  The file was never compiled, so deleting it cannot change a build. The 20 docs in `docs/v2`, `docs/v2-depth` and
  `docs/v3` that called it wired now say it was deleted and point at `run_one_plan`; `work/history/` is unchanged.
  gap-a29711's anchor moved to `run_one_plan`. The verify passes, and `check_markdown_links.py` and
  `check_citation_errata.py --prose` are clean.
