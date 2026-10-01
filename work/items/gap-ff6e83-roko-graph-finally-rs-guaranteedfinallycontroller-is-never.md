+++
id = "gap-ff6e83"
kind = "gap"
title = "roko-graph finally.rs (GuaranteedFinallyController) is never compiled: lib.rs has no mod finally"
status = "open"
triage = "unverified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-graph"]
created = 2026-10-01
updated = 2026-10-01
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
