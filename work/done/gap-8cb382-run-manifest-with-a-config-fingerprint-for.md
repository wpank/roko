+++
id = "gap-8cb382"
kind = "gap"
title = "Run manifest with a config fingerprint for every plan run (S01.P0-2)"
status = "done"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-core/config", "roko-cli/build", "roko-cli/graph_execution"]
created = 2026-09-29
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "a8159e1ec"
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e4"
discovered_from = "tmp/cybernetic-harness/specs/S01-instrumentation.md (P0-2, §4.7 config hash, §5.1 RunManifest)"
anchors = ["crates/roko-core/src/config/fingerprint.rs", "crates/roko-core/src/metric.rs::ConfigHash", "crates/roko-cli/build.rs", "crates/roko-cli/src/graph_execution/plan_runner.rs::run_graph_plan_body"]
lane = "rust-cold"
parent = "spec-b7303f"
links = { depends_on = ["gap-528762"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn config_fingerprint_is_order_independent_and_redacts_secrets' crates/roko-core/src/ && cargo test -p roko-core --lib config_fingerprint_is_order_independent_and_redacts_secrets"

[[verify]]
command = "grep -rqw 'fn graph_plan_run_writes_run_manifest' crates/roko-cli/src/ && cargo test -p roko-cli --lib graph_plan_run_writes_run_manifest"

[closed]
at = 2026-09-30
by = "coordinator (session 7622b882)"
evidence = "Merged in 9313e49f0. Batch 12a gate on the merged tree (MAIN a8159e1ec has the same tree as gated 265acb18b): cargo check --workspace --tests, nightly fmt --check (after the coordinator's rustfmt commits a3509fc46 and 6144df24b) and clippy -p roko-cli -p roko-learn -p roko-core -p roko-agent -p roko-gate -p roko-serve --no-deps -D warnings clean; lib tests pass: roko-cli 3133, roko-core 1938, roko-learn 1196, roko-serve 958, roko-gate 689, roko-agent 2257 (its one failure, a_timed_out_attempt_reports_the_usage_it_streamed, is a load flake at load 77 that passes alone). Verify: config_fingerprint_is_order_independent_and_redacts_secrets (roko-core lib) and graph_plan_run_writes_run_manifest (roko-cli lib) pass."
+++

## Problem

A plan run records no manifest: nothing says which harness commit (clean or dirty), which config and which
invocations produced its records. Two runs cannot be compared or reproduced.

What exists today:
- `crates/roko-cli/build.rs` embeds `ROKO_GIT_HASH` and reruns when `.git/HEAD` or `refs/` change (lines 44–45). It
  has no dirty flag.
- `roko_core::metric::ConfigHash::of` hashes canonical JSON with BLAKE3. But it redacts nothing, keeps only 16 hex
  characters, and only `demo_seed.rs` calls it.

## Why it matters

The S08 `RunRecord` and the S09 experiments join runs on `config.hash` (S01 §5.9). Whitepaper numbers must name the
commit and config they came from. It is part of epic spec-b7303f.

## Where

- **New:** `crates/roko-core/src/config/fingerprint.rs`, with `fingerprint(&RokoConfig)`. It reuses `metric.rs`'s
  canonical JSON and BLAKE3.
- `crates/roko-cli/build.rs`: add `ROKO_GIT_DIRTY`.
- `plan_runner.rs::run_graph_plan_body`: open or resume the manifest at the start of a run, and close it at the end.
- The `RunManifest` type comes from gap-528762. The file is `.roko/runs/<run_id>/manifest.json` (`roko_fs::layout`
  `run_dir`).

## Current state

At `41c7ffbd6` there is no config fingerprint and no manifest. The rerun half of S01.P0-2 is already done (W3a).

## Plan

1. **Fingerprint:** `"b3:" + hex(blake3(canon(redact(config))))`, the full digest. `redact` replaces `secrets`,
   `*_key` and `*token*` values and counts them in `redacted_keys`. Make `ConfigHash::of` the 16-character prefix of
   the same digest, so the two can never disagree.
2. **`build.rs`:** emit a dirty boolean, and add `rerun-if-changed=../../.git/index`. Compute the diff digest when the
   run starts, not at build time; a build-time digest would rebuild `roko-cli` on every edit.
3. **Manifest:** write the invocations (a resume appends one), the harness, the config hash and, when the run closes,
   the counts.

## Done when

- [ ] The fingerprint ignores key order, redacts secrets and matches a golden vector.
- [ ] A plan run writes `manifest.json`, and a resumed run adds a second invocation.
- [ ] Tests `config_fingerprint_is_order_independent_and_redacts_secrets` and `graph_plan_run_writes_run_manifest`
      pass: both `[[verify]]` commands.

## Notes

- Steps 1 and 2 are cold and can land now.
- Step 3 adds two calls in the hot `run_graph_plan_body`. Land it after the portal branches merge, together with
  gap-96f7ed if convenient.
- Implemented on `work/gap-8cb382` at `aa1d5eb7f`; cargo verification deferred to the batch check. On the branch, both verify tests pass.
- The redaction rule departs from S01 §4.7's letter in two ways. Key matching is ASCII case-insensitive (`GITHUB_TOKEN` in an env map is a secret), and a number or boolean under a secret key is kept, so `max_tokens` stays in the hash. `crates/roko-core/src/config/config_fingerprint_golden.json` pins the rule; its canonical forms match the `rfc8785` Python package and its hashes the `blake3` one. S01 and S08's `driver/fingerprint.py` should follow it.
