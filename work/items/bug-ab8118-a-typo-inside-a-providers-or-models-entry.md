+++
id = "bug-ab8118"
kind = "bug"
title = "A typo inside a [providers.*] or [models.*] entry fails the whole config load, where a typo elsewhere is stripped with a warning"
status = "open"
triage = "unverified"
severity = "p3"
goal = "release"
size = "S"
subsystem = ["roko-core/config"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-onboard's report)"
anchors = ["crates/roko-core/src/config/provider.rs::ProviderConfig", "crates/roko-core/src/config/provider.rs::ModelProfile", "crates/roko-core/src/config/loader.rs::strip_unknown_fields"]
lane = "rust-cold"
parent = "spec-ae5f94"
links = { depends_on = [], blocks = [], related = ["bug-647249", "bug-12153c"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_typo_inside_a_provider_or_model_entry_is_handled_like_any_other' crates/roko-core/src/ && cargo test -p roko-core --lib a_typo_inside_a_provider_or_model_entry_is_handled_like_any_other"
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
