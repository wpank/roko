+++
id = "bug-a1fc98"
kind = "bug"
title = "WitnessVertex ids depend on serde_json key order, so the same vertex can hash differently in another build"
status = "open"
triage = "unverified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-agent/safety"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-ff95f5"
anchors = ["crates/roko-agent/src/safety/"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-ff95f5"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-agent --lib witness_vertex_id_is_canonical"
+++

## Problem

A WitnessVertex id hashes `serde_json::to_vec(content)`. Key order flips with serde_json's `preserve_order` feature, which workspace builds unify (see the memory on serde_json preserve_order), so the same vertex can get a different id in another build.

## Plan

Hash a canonical encoding (sorted keys). Add a test named `witness_vertex_id_is_canonical` that checks two key orders give one id.

## Done when

- The test passes.

## Notes

- Reported on 2026-10-01 by wk-tamper, working on gap-ff95f5, during the evening close-out round.
