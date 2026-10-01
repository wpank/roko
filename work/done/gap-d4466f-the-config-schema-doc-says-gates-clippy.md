+++
id = "gap-d4466f"
kind = "gap"
title = "The config schema doc says gates.clippy_enabled defaults to false; the code defaults it to true"
status = "done"
triage = "verified"
severity = "p3"
goal = "truth"
size = "S"
subsystem = ["docs/v3"]
created = 2026-09-30
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "d24875405"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-gates's report, checked on work/gap-3506f1b at 653818f62)"
anchors = ["docs/v3/depth/21-config/01-schema-sections.md", "crates/roko-core/src/config/gates.rs"]
lane = "rust-hot"
parent = "spec-e9d7ec"
links = { depends_on = [], blocks = [], related = ["gap-3506f1"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -qE 'clippy_enabled. \\| bool \\| false' docs/v3/depth/21-config/01-schema-sections.md"

[closed]
at = 2026-09-30
commit = "d24875405"
evidence = "docs/v3/depth/21-config/01-schema-sections.md: clippy_enabled row now says true, matching GatesConfig::default() (default_true); other [gates] rows checked against the code; verify grep passes"
+++

## Problem

`docs/v3/depth/21-config/01-schema-sections.md:166` lists `clippy_enabled | bool | false`. The code defaults it to true (`crates/roko-core/src/config/gates.rs:279`, `clippy_enabled: default_true()`). docs/v2's `19-CONFIG.md:670` has it right.

## Why it matters

Honest verdicts (epic spec-e9d7ec): users reading the schema doc expect no clippy gate, and are surprised by clippy failures. p3.

## Plan

1. Correct the row. While editing, compare the rest of the `[gates]` table with `GatesConfig::default()`.

## Done when

- [ ] The doc's defaults match the code.
- [ ] The `[[verify]]` command passes.
