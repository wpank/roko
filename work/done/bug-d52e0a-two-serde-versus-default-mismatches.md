+++
id = "bug-d52e0a"
kind = "bug"
title = "Two serde-versus-Default mismatches: DeployConfig.worker_image and roko-cli's Config.gates load differently from their defaults"
status = "done"
triage = "verified"
severity = "p3"
goal = "release"
size = "S"
subsystem = ["roko-core/config/serve", "roko-cli/config"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "6da161af6"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-serve-sec's report)"
anchors = ["crates/roko-core/src/config/serve.rs", "crates/roko-cli/src/config.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-647249"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn deploy_table_without_worker_image_keeps_the_default_image' crates/roko-core/src/ && cargo test -p roko-core --lib deploy_table_without_worker_image_keeps_the_default_image && grep -rqw 'fn config_without_gate_entries_has_the_default_gates' crates/roko-cli/src/ && cargo test -p roko-cli --lib config_without_gate_entries_has_the_default_gates"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T09:00:56Z"
by = "coordinator (session 7622b882)"
forced = false
evidence = "Batch 20c gate (ca5645373 = MAIN 1bf49188d crates): lib tests pass, including roko-core deploy_table_without_worker_image_keeps_the_default_image and roko-cli config_without_gate_entries_has_the_default_gates; check, fmt, clippy clean. Merged 414fb7e40 (work/ce-l13 9c8e30a92)."
+++

## Problem

Two config fields have a `Default` that disagrees with what serde gives a partial table:

- `DeployConfig.worker_image` is `#[serde(default)] Option<String>` (`crates/roko-core/src/config/serve.rs:733-734`), so a `[deploy]` table without it loads `None`, while `Default` gives `Some("ghcr.io/nunchi-trade/roko-worker:latest")` (:752);
- roko-cli's `Config.gates` loads empty through serde, while its `Default` has one `shell: true` gate (wk-serve-sec).

So the same config means different things depending on whether a table is present.

## Why it matters

Not security-relevant, but surprising: adding an unrelated key to `[deploy]` drops the worker image. p3.

## Where

The two fields' serde attributes and `Default` impls.

## Plan

1. Use `#[serde(default = "…")]` functions that return the `Default` values (or make `Default` match serde), for both.
2. Add the two tests.

## Done when

- [ ] Each field loads the same value from a partial table as `Default` gives.
- [ ] The `[[verify]]` command passes.
- 2026-10-01 (coordinator): the verify now names the tests wk-childenv wrote before the item was filed (same coverage: a partial table equals Default for each field).
