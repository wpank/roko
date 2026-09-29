+++
id = "gap-a6abd5"
kind = "gap"
title = "S08 §5.2's heading still gives the task manifest as DIR/.vb/task.json"
status = "open"
triage = "unverified"
severity = "p3"
goal = "proof"
size = "S"
subsystem = ["cybernetic-harness/specs"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-bench-fix3's report on bug-2930a8)"
anchors = ["tmp/cybernetic-harness/specs/S08-benchmark-suite.md"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["bug-2930a8", "gap-4667a4", "gap-9e7079"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'DIR/.vb/task.json' tmp/cybernetic-harness/specs/S08-benchmark-suite.md"
+++

## Problem

bug-2930a8 moved `materialize.py` to the families' layout. Each family's `gen.py --out DIR [--workdir WORKDIR]` writes the private `task.json`, the spec and the bundle into DIR, and the task repo into WORKDIR. S08 §5.2's heading still says "Task manifest `vb.task/1` (`DIR/.vb/task.json`, written by `gen.py`)" (`tmp/cybernetic-harness/specs/S08-benchmark-suite.md:295`), and §4.3 (:115) points to it as the generator contract.

## Why it matters

Pilot benchmark (epic spec-567e52): a new family written to the spec would use the old layout, which the driver no longer accepts. p3.

## Where

S08 §5.2 (and any other `.vb/` references in the section). S08 is untracked (`tmp/`), so edit it in place.

## Plan

1. Amend §5.2 to the DIR-plus-WORKDIR layout, with F4's gen.py docstring as the reference, and note the change in S08's changelog.

## Done when

- [ ] S08 describes the layout the families and the driver use.
- [ ] The `[[verify]]` command passes.
