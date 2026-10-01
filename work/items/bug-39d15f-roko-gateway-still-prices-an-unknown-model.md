+++
id = "bug-39d15f"
kind = "bug"
title = "roko-gateway still prices an unknown model at Sonnet rates"
status = "open"
triage = "unverified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-gateway"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-ad0d39"
anchors = ["crates/roko-gateway/src/"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-ad0d39"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -rn 'sonnet_fallback' crates/roko-gateway/src --include=*.rs"
+++

## Problem

gap-ad0d39 made roko-learn leave unknown models unpriced. The gateway still prices them with `sonnet_fallback`, so gateway cost events disagree with the cost log.

## Plan

Leave unknown models unpriced in the gateway too (cost unknown, warn once), as roko-learn does.

## Done when

- The verify passes, and the gateway's cost tests are updated.

## Notes

- Reported on 2026-10-01 by wk-model-truth, working on gap-ad0d39, during the evening close-out round.
