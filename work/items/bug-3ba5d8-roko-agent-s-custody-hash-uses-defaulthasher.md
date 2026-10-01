+++
id = "bug-3ba5d8"
kind = "bug"
title = "roko-agent's custody hash uses DefaultHasher, unstable across Rust releases and unlike roko-cli's SHA-256 chain"
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
command = "cargo test -p roko-agent --lib custody_hash_is_sha256"
+++

## Problem

`Custody::compute_hash` in roko-agent says SHA-256 but uses `DefaultHasher`, which is not stable across Rust releases. So `CustodyLogger::log_chained` and `verify_chain` disagree with roko-cli's SHA-256 chain (gap-ff95f5), and a toolchain upgrade can break the verification of old chains.

## Plan

Hash with SHA-256 as documented. Add a test named `custody_hash_is_sha256` that pins a known digest. Old chains need a migration note: verify them with the old hasher once, or mark them legacy.

## Done when

- The test passes, and the migration is noted.

## Notes

- Reported on 2026-10-01 by wk-tamper, working on gap-ff95f5, during the evening close-out round.
