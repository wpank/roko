+++
id = "gap-96c09b"
kind = "gap"
title = "Six crates keep crate-wide clippy allows (roko-std, -fs, -eval, -index, -runtime, -primitives)"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["workspace/hygiene"]
created = 2026-10-08
updated = 2026-10-08
last_verified = 2026-10-08
source = "stash-triage-2026-10-08 (roko-worktree-archive/2026-10-08-stash-triage/TRIAGE.md)"
discovered_from = "archive/stash-2026-09-21-main-2"
anchors = ["crates/roko-std/src/lib.rs", "crates/roko-runtime/src/lib.rs"]
links = { depends_on = [], blocks = [], related = ["gap-1535e7"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'allow(clippy::unnecessary_literal_bound)' crates/roko-std/src/lib.rs && ! grep -q 'allow(clippy::derive_partial_eq_without_eq' crates/roko-runtime/src/lib.rs && cargo clippy -p roko-std -p roko-fs -p roko-eval -p roko-index -p roko-runtime -p roko-primitives --no-deps -- -D warnings"
+++

## Problem

Six crates (roko-std, roko-fs, roko-eval, roko-index, roko-runtime, roko-primitives) silenced clippy lints
crate-wide (`#![allow(clippy::…)]` in each `lib.rs`), so new code in them was never checked for those lints.

## Why it matters

Hygiene, the same concern as gap-1535e7 for roko-cli: crate-wide allows hide new violations.

## Where

The crate roots: `crates/roko-std/src/lib.rs`, `crates/roko-fs/src/lib.rs`, `crates/roko-eval/src/lib.rs`,
`crates/roko-index/src/lib.rs`, `crates/roko-runtime/src/lib.rs`, `crates/roko-primitives/src/lib.rs`.

## Current state

The removal and the code fixes sat uncommitted in `archive/stash-2026-09-21-main-2` (triage:
`salvage-08-lint-crate-allows.patch`).

## Plan

Remove the crate-wide allows, fix what clippy then reports, keep item-level allows only with a reason.

## Done when

- [ ] None of the six `lib.rs` files carries a crate-wide clippy allow, and clippy passes on them with `-D warnings`.
- [ ] `[[verify]]` passes.
