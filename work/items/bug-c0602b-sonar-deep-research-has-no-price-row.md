+++
id = "bug-c0602b"
kind = "bug"
title = "sonar-deep-research has no price row and is priced as sonar"
status = "open"
triage = "unverified"
severity = "p3"
goal = "truth"
size = "S"
subsystem = ["roko-core/pricing"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-2dfd23"
anchors = ["crates/roko-core/src/config/model_registry.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-2dfd23"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-core --lib sonar_deep_research_price"
+++

## Problem

The pricing registry has no sonar-deep-research row, so the prefix rule prices it at sonar's $1/$1, far below its real price.

## Plan

Add the row from Perplexity's dated price page, or leave it unpriced. Add a test named `sonar_deep_research_price`.

## Done when

- The test passes.

## Notes

- Reported on 2026-10-01 by wk-learn2, working on bug-2dfd23, during the evening close-out round.
