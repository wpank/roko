+++
id = "gap-94cef2"
kind = "gap"
title = "error_pattern_store.rs's module doc names upsert_observation and similar_patterns, which do not exist"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-learn/error_pattern_store"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
source = "bug-56e072 worker 2026-10-16"
discovered_from = "bug-56e072"
anchors = ["crates/roko-learn/src/error_pattern_store.rs::observe_gate_failure"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-56e072", "gap-a11687"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -qE 'upsert_observation|similar_patterns' crates/roko-learn/src/error_pattern_store.rs"
+++

## Problem

`crates/roko-learn/src/error_pattern_store.rs`'s module doc names two methods the store does not have, and one doc
comment sits on the wrong item:

- "Entry Lifecycle" step 2: "`ErrorPatternStore::upsert_observation` normalises the observation to a stable `key`".
  The method is `observe_gate_failure` (with `observe_gate_failure_at`).
- "Similarity Matching": "[`ErrorPatternStore::similar_patterns`] finds patterns whose digest is close to a query
  string". No such method exists, and the intra-doc link is broken. The store never matches by digest similarity:
  prompts select a pattern by the plan's task it was seen on or by verify command (`bounded_summary_keyed`), and the
  planner block by crate (`resolved_for`).
- The doc comment "`text` with each run of whitespace made one space, and none at the ends." is attached to
  `impl FailurePatternQuery<'_>`; it describes `collapse_whitespace`, which has no doc.

## Why it matters

The module doc is the first thing a reader of the store sees. It describes a similarity feature nothing has and a
method name that leads nowhere, the same drift gap-a11687 fixed for `mark_resolved`.

## Where

- `crates/roko-learn/src/error_pattern_store.rs`: the module doc (`//!`), step 2 of "Entry Lifecycle" and the
  "Similarity Matching" section; the comment above `impl FailurePatternQuery<'_>`.

## Current state

Found while fixing bug-56e072 (2026-10-16). Only the docs are wrong; the code is unaffected.

## Plan

1. Step 2: name `ErrorPatternStore::observe_gate_failure`.
2. Replace "Similarity Matching" with how selection works: `bounded_summary_keyed` (the plan's task or a verify
   command), `resolved_for` (crates), `graduation_candidates` (recurrence across plans).
3. Move the stray comment onto `collapse_whitespace`.

## Done when

The `[[verify]]` command passes, and `cargo doc -p roko-learn --no-deps` reports no broken intra-doc link in this
module.
