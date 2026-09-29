+++
id = "bug-c1950e"
kind = "bug"
title = "roko config validate warns that agent.default_model references a missing model when the model is a builtin"
status = "done"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/config"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "b11ca807d"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (17:09, wk-onboard's report on bug-e1327f, branch work/bug-e1327f)"
anchors = ["crates/roko-cli/src/config_cmd.rs"]
lane = "rust-cold"
parent = "spec-9a3131"
links = { depends_on = [], blocks = [], related = ["bug-e1327f"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn validate_accepts_a_builtin_default_model' crates/roko-cli/src/ && cargo test -p roko-cli --lib validate_accepts_a_builtin_default_model"

[closed]
at = 2026-09-29
by = "coordinator (session 7622b882)"
evidence = "config validate counts a model reference as present when it is a [models] key or a builtin (aliases included) (7b5f32d2b, merged b351d2be5). Batch 6 gate (work/rust-batch-5 tree plus fmt-only and unused-import fixes fdb2a9b72, 579ffd0e6, a8dd7f09f, 2aa55ab1f): cargo check --workspace --tests clean; clippy -p roko-cli -p roko-core -p roko-agent -p roko-serve -p roko-learn -p roko-gateway --no-deps -D warnings clean; lib tests roko-cli 3087, roko-agent 2249, roko-core 1922, roko-learn 1177, roko-serve 955, roko-gate 685, roko-gateway 41, 0 failed; merged MAIN tree re-checked (cargo check --workspace --tests clean)."
+++

## Problem

`roko config validate` warns "agent.default_model references missing model '<name>'" whenever the name is not a key of the workspace's `[models]` table (`config_cmd.rs:1466` on `work/bug-e1327f`). A builtin model such as `claude-sonnet-4-6` is valid without a `[models]` entry, but it still gets the warning. The `agent.fallback_model` and `agent.tier_models` checks just below (:1480, :1499) use the same test. The existing test at `config_cmd.rs:2445` asserts the warning for `claude-sonnet-4-6`, so it encodes the false positive.

## Why it matters

Hygiene (epic spec-9a3131): a fresh workspace that uses a builtin model gets a warning on its first `config validate`, which teaches users to ignore warnings.

## Where

`crates/roko-cli/src/config_cmd.rs`: the model-reference checks in the validate report (about :1464-1500 on the branch).

## Current state

Checked on `work/bug-e1327f` (`fab9168a9`): the check is `!models.contains_key(default_model)`, with no lookup in the builtin model catalog.

## Plan

1. Resolve each reference as the runtime does: the workspace's `[models]` first, then the builtin catalog. Warn only when neither has it.
2. Apply the same rule to `fallback_model` and `tier_models`.
3. Update the test at :2445, and add `validate_accepts_a_builtin_default_model`.

## Done when

- [ ] A builtin model in `agent.default_model` gives no warning, and an unknown name still does.
- [ ] The `[[verify]]` command passes.

## Notes

- Premise confirmed at `407ce30d5`: the three checks tested `[models]` keys only; a fresh workspace on the builtin `claude-sonnet-4-6` got the warning (seen with `target/debug/roko` on bug-e1327f's no-provider template).
- A reference now resolves to a `[models]` key or a builtin model (aliases included), the rule core's `routing.unresolved_model` check uses.
- Implemented on `work/bug-12153c` at `7b5f32d2b`; cargo verification deferred to the batch check.
