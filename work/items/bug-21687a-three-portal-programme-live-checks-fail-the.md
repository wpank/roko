+++
id = "bug-21687a"
kind = "bug"
title = "Three portal-programme live checks fail: the plan scaffold no longer writes model_hint, and serve looks for /demo at a compile-time path"
status = "open"
triage = "unverified"
severity = "p3"
goal = "golden-path"
size = "S"
subsystem = ["plans/portal-programme/_harness", "roko-serve/embedded"]
created = 2026-09-30
updated = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-runstate's report, checked on work/bug-5f0604 at 059df5e1a)"
anchors = ["plans/portal-programme/_harness/authoring-check.sh", "crates/roko-serve/src/embedded.rs"]
lane = "rust-cold"
parent = "spec-9230a9"
links = { depends_on = ["bug-5f0604"], blocks = [], related = ["bug-5f0604", "gap-0f3980"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f plans/portal-programme/_harness/authoring-check.sh && ! grep -q 'model_hint' plans/portal-programme/_harness/authoring-check.sh"
+++

## Problem

On `work/bug-5f0604`, three of the portal programme's live checks fail:

- **The authoring check.** `authoring-check.sh`'s "tasks carry verify commands and model_hint" (:23, and 04's `REVIEW.md:17`) fails, because the POST /api/plans scaffold no longer writes `model_hint`: the TaskDef work (gap-0f3980) removed it.
- **The two access checks for /demo.** They fail because serve embeds the demo app from the compile-time folder `../../demo/demo-app/dist/` (`crates/roko-serve/src/embedded.rs:46`). Build trees without a built demo have nothing there.

## Why it matters

Check each attempt's diff (epic spec-9230a9): a harness that fails on correct behaviour trains people to ignore it. p3.

## Where

The harness scripts, and `embedded.rs`.

## Plan

1. Drop `model_hint` from the authoring check, since the scaffold no longer writes it by design.
2. Make the /demo checks skip, with a reason, when the build has no demo assets. Or build the demo before those checks.

## Done when

- [ ] The live checks pass on a build tree without the demo, and don't require `model_hint`.
- [ ] The `[[verify]]` command passes (it checks the authoring script; check /demo by hand).
