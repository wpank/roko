+++
id = "bug-ab8118"
kind = "bug"
title = "A typo inside a [providers.*] or [models.*] entry fails the whole config load, where a typo elsewhere is stripped with a warning"
status = "done"
triage = "verified"
severity = "p3"
goal = "release"
size = "S"
subsystem = ["roko-core/config"]
created = 2026-09-29
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "90307ad5e"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-onboard's report)"
anchors = ["crates/roko-core/src/config/provider.rs::ProviderConfig", "crates/roko-core/src/config/provider.rs::ModelProfile", "crates/roko-core/src/config/loader.rs::strip_unknown_fields"]
lane = "rust-cold"
parent = "spec-ae5f94"
links = { depends_on = [], blocks = [], related = ["bug-647249", "bug-12153c"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_typo_inside_a_provider_or_model_entry_is_handled_like_any_other' crates/roko-core/src/ && cargo test -p roko-core --lib a_typo_inside_a_provider_or_model_entry_is_handled_like_any_other"

[closed]
at = 2026-09-30
by = "coordinator (session 7622b882)"
evidence = "Merged in 2cb2fc282. A typo inside a [providers.*] or [models.*] entry is warned about and stripped like elsewhere. Batch 17 gate: first run on 2c4abe35b (check clean; lib tests roko-agent 2271, roko-cli 3244 (gate_rows writer flake, fixed by bug-779ae7), roko-core 1955, roko-fs 260, roko-learn 1204, roko-serve 988), then re-gated on 53feea92e (same code as MAIN 90307ad5e) after the coordinator's doc-paragraph and rustfmt fix on guard2's branch (3f3a7be84): nightly fmt clean; clippy -p roko-agent -p roko-cli -p roko-core -p roko-fs -p roko-learn -p roko-serve --keep-going -D warnings clean; roko-fs lib 260; --test secret_canary 11 passed; --test secrets_and_git_guard_canary 2 passed. Verify: its test passes in that run and its static checks pass on MAIN."
+++

## Problem

`ProviderConfig` (`crates/roko-core/src/config/provider.rs:275`) and `ModelProfile` (:562) are `#[serde(deny_unknown_fields)]`. The loader strips unknown keys elsewhere, with a warning (`strip_unknown_fields`), but a misspelled key inside a `[providers.<name>]` or `[models.<name>]` entry reaches serde, and the whole load fails. One typo in a provider entry stops every command.

## Why it matters

Release blockers (epic spec-ae5f94): the same mistake gets two behaviours, a warning in one section and a hard failure in another. The failure is also where a new user is most likely to edit (providers and models). p3, because the error at least names the key.

## Where

- The two structs' serde attributes.
- `strip_unknown_fields` and the schema tree's templates for dynamic map entries (loader.rs).

## Current state

At 7fa54b873 both structs deny unknown fields.

## Plan

Pick one rule and apply it everywhere. Either:

- strip unknown keys inside provider and model entries too (the tree needs a template child per entry), with the same warning; or
- keep failing, but for every section, with a message that names the key and suggests the nearest known one.

Stripping matches the rest of the loader. Then add `a_typo_inside_a_provider_or_model_entry_is_handled_like_any_other`.

## Done when

- [ ] A typo inside a provider or model entry is handled like a typo anywhere else.
- [ ] The `[[verify]]` command passes.

## Notes

- 2026-09-30 (wk-childenv): Implemented on `work/bug-17f0e4` at `3b6af343f`; cargo verification deferred to the batch
  check. Cause: `strip_unknown_fields` recursed into each dynamic-map entry with the section's own prefix, so no key
  inside an entry was checked. Only `providers` and `models` entries are now stripped (their structs deny unknown
  fields); profiles, roles and tool profiles are left alone because serde collects or ignores their extra keys. The
  `--config <path>` path in roko-cli (`Config::from_file`) parses with serde directly and still fails on such a typo.
