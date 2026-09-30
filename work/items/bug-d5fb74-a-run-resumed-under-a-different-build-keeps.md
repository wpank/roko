+++
id = "bug-d5fb74"
kind = "bug"
title = "A run resumed under a different build keeps the first invocation's harness and config in its manifest"
status = "open"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-learn/telemetry"]
created = 2026-09-30
updated = 2026-09-30
last_verified = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-telemetry2's report, branch work/gap-8cb382 at c5090e9a5)"
anchors = ["crates/roko-learn/src/telemetry/manifest.rs"]
lane = "rust-hot"
parent = "spec-b7303f"
links = { depends_on = ["gap-8cb382"], blocks = [], related = ["gap-8cb382", "bug-0ba3d9"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_resume_under_another_build_records_its_own_harness_and_config' crates/roko-learn/src/ && cargo test -p roko-learn --lib a_resume_under_another_build_records_its_own_harness_and_config"
+++

## Problem

gap-8cb382's run manifest (`roko-learn/src/telemetry/manifest.rs` on its branch) is rewritten whenever a process starts or resumes a run (:4). `begin_invocation` numbers each invocation and marks resumes (:71-75). wk-telemetry2 reports that when a run resumes under a different build or config, the manifest keeps the first invocation's harness and config, so the later attempts appear to have run under the first build.

## Why it matters

One settled record per attempt (epic spec-b7303f): the manifest is the provenance record. A run that spans two builds must say so, or results get attributed to the wrong code.

## Where

`RunProvenanceManifest::begin_invocation` and the fields that hold the harness sha and the config fingerprint.

## Plan

1. Record the harness sha, dirty flag and config fingerprint per invocation, and keep the run-level values as "first seen" only, or flag the run as mixed-build.
2. Add `a_resume_under_another_build_records_its_own_harness_and_config`.

## Done when

- [ ] Each invocation's harness and config are in the manifest, and a mixed-build run is visible.
- [ ] The `[[verify]]` command passes.

## Notes

- Build on gap-8cb382's branch.
- Implemented on `work/bug-0ba3d9` at `b914d2316`; cargo verification deferred to the batch check. On the branch, `a_resume_under_another_build_records_its_own_harness_and_config` (roko-learn) and `graph_plan_run_writes_run_manifest` (roko-cli, now also checking each invocation's provenance) pass. Clippy with `-D warnings` and nightly fmt are clean.
- Each `invocations[]` entry records `harness` and `config`. The run-level ones are the first invocation's, and `mixed_provenance: true` marks a run that another build or config resumed. The fields are additive to `roko.run_manifest/1`, so S01 §5.1 should list them.
