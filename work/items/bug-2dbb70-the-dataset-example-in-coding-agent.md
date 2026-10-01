+++
id = "bug-2dbb70"
kind = "bug"
title = "The dataset example in coding-agent-benchmarks/README.md has no test_files, so it fails the bench's grading-tests check"
status = "open"
triage = "unverified"
severity = "p3"
goal = "release"
size = "S"
subsystem = ["demo/demo-resources"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-honestbench's report, checked on work/bug-0320da at dd7cb7320)"
anchors = ["demo/demo-resources/coding-agent-benchmarks/README.md"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-28becc", "bug-0320da"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'test_files' demo/demo-resources/coding-agent-benchmarks/README.md"
+++

## Problem

The dataset example in `demo/demo-resources/coding-agent-benchmarks/README.md` (instance `local__case-1`, :158) has no `test_files`. Since bug-28becc, `roko bench swe` checks that each instance names the grading tests it restores before scoring, so the documented example fails that check.

## Why it matters

Release: the first example a user copies doesn't run. p3.

## Plan

1. Add `test_files` to the example, and run it through `roko bench swe` once.

## Done when

- [ ] The README's example passes the bench's checks.
- [ ] The `[[verify]]` command passes.
