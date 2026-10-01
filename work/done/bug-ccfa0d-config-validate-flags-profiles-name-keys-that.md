+++
id = "bug-ccfa0d"
kind = "bug"
title = "config validate flags [profiles.<name>] keys that DomainProfile collects into extra, and tools.profiles has no schema template"
status = "done"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-core/config"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-onboard's report)"
anchors = ["crates/roko-core/src/config/schema.rs::DomainProfile", "crates/roko-core/src/config/loader.rs::build_schema_tree", "crates/roko-core/src/config/loader.rs::validate_known_config_paths"]
lane = "rust-cold"
parent = "spec-9a3131"
links = { depends_on = [], blocks = [], related = ["bug-647249"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn profile_extra_keys_and_tools_profiles_are_known_config_paths' crates/roko-core/src/ && cargo test -p roko-core --lib profile_extra_keys_and_tools_profiles_are_known_config_paths"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:26Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T16:13:53Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

## Problem

- `DomainProfile` (`crates/roko-core/src/config/schema.rs:298`) collects any extra key of a `[profiles.<name>]` table into `extra` (`#[serde(default, flatten)]`, :315-316), so those keys are valid. But `config validate` reports them as unknown, because the schema tree has no entry for them.
- `tools.profiles` has no template in `build_schema_tree`, so its keys are reported (and stripped) as unknown too. The tree has templates for other dynamic maps (`extra_headers`, `tier_models`, `domain_gates`, `role_token_budgets`), but not for this one.

## Why it matters

Hygiene (epic spec-9a3131): `config validate` warns about valid settings, which teaches users to ignore it.

## Where

`DomainProfile`, and `build_schema_tree` and `validate_known_config_paths` in `loader.rs`.

## Current state

At 7fa54b873 neither map has a template or a wildcard in the tree.

## Plan

1. Mark `[profiles.<name>]` as open (any key allowed) in the tree, since `extra` accepts any key. Add a template for `tools.profiles` entries.
2. Add `profile_extra_keys_and_tools_profiles_are_known_config_paths`.

## Done when

- [ ] `roko config validate` accepts extra profile keys and `tools.profiles` entries, and loading keeps them.
- [ ] The `[[verify]]` command passes.

## Notes

- 2026-10-01 (wk-cfg): implemented on work/bug-ccfa0d; cargo verification deferred to the batch check.
  Premise checked at BASE `ebdc0f5d5`: `walk_config_paths` reported every extra `[profiles.<name>]` key as unknown,
  which makes `roko config validate` fail (it treats unknown paths as errors). `tools.profiles` serialized as an
  empty table, so validation skipped its entries unchecked and `config set tools.profiles.<name>.extra_tools …`
  failed with `unknown key` (`schema_value_for_path` had no template). Loading already kept both.
- Change (`crates/roko-core/src/config/loader.rs`): a new `OPEN_TABLES` list (`profiles.*`) whose unknown keys
  validation accepts, matched by a shared `matches_section_pattern`; a `ToolProfileConfig` sentinel for
  `tools.profiles` in `build_schema_tree`. The schema guard test now skips open tables and compares the new
  template with `ToolProfileConfig`'s serde fields. New test
  `profile_extra_keys_and_tools_profiles_are_known_config_paths`.
