+++
id = "bug-cc1bd8"
kind = "bug"
title = "The anthropic provider catalog still offers the retired claude-haiku-3-5"
status = "open"
triage = "unverified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-core/provider_catalog"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-0c0747"
anchors = ["crates/roko-core/src/provider_catalog.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-0c0747"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -n 'claude-haiku-3-5' crates/roko-core/src/provider_catalog.rs"
+++

## Problem

The anthropic entry in `provider_catalog.rs` still offers claude-haiku-3-5, which Anthropic has retired, so `roko config providers add` can write a model that no longer serves.

## Plan

Remove it, and check the other catalog models against the providers' current model lists.

## Done when

- The verify passes.

## Notes

- Reported on 2026-10-01 by wk-model-truth, working on bug-0c0747, during the evening close-out round.
