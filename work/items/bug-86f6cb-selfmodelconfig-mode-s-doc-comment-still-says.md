+++
id = "bug-86f6cb"
kind = "bug"
title = "SelfModelConfig::mode's doc comment still says off is the default, but the self-model defaults to shadow"
status = "open"
triage = "verified"
severity = "p3"
goal = "cybernetic"
size = "S"
subsystem = ["roko-core/config"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
source = "whitepaper-review-2026-10-16 (gap-6a83de)"
discovered_from = "gap-6a83de"
anchors = ["crates/roko-core/src/config/self_model.rs::SelfModelConfig"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-9aca5d", "spec-7a71d6", "gap-6a83de"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q '(the default) or' crates/roko-core/src/config/self_model.rs && ! grep -qF '`off` (the default)' crates/roko-core/src/config/self_model.rs"
+++

## Problem

The doc comment on `SelfModelConfig::mode` in `crates/roko-core/src/config/self_model.rs` says "`off` (the default),
`shadow` or `active`". The default has been `shadow` since `5350b2c14` (gap-9aca5d, decision 2 of 2026-10-15):
`SelfModelMode`'s `#[default]`, `SelfModelConfig::default()`, the module doc and the test
`self_model_config_defaults_shadow` all say shadow. Only this field comment still says off, so a reader of the struct
or of docs generated from it is told the wrong default.

## Why it matters

The whitepaper and the research paper describe the outcome forecaster as running in shadow at first, which matches
the code; this comment contradicts both. Related: gap-9aca5d (the default change), spec-7a71d6 (the staged turn-on
epic), gap-6a83de (the whitepaper review that found it).

## Where

- `crates/roko-core/src/config/self_model.rs::SelfModelConfig`: the `mode` field's doc comment (line 46 at
  `5e32264ce`).

## Current state

Behaviour is right: the default is shadow. `git grep -F '`off` (the default)' -- crates docs` finds only this
comment at `5e32264ce`.

## Plan

1. Change the comment to "`off`, `shadow` (the default) or `active`."
2. Re-run the grep above to make sure no other doc repeats the old default.

## Done when

The `[[verify]]` command passes: the stale phrase is gone and the field comment names shadow as the default.

## Notes

Doc-only; no behaviour change and no test change needed. Found by the whitepaper review (gap-6a83de) on 2026-10-16
at `5e32264ce`.
