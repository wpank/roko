+++
id = "bug-c1950e"
kind = "bug"
title = "roko config validate warns that agent.default_model references a missing model when the model is a builtin"
status = "open"
triage = "unverified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/config"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (17:09, wk-onboard's report on bug-e1327f, branch work/bug-e1327f)"
anchors = ["crates/roko-cli/src/config_cmd.rs"]
lane = "rust-cold"
parent = "spec-9a3131"
links = { depends_on = [], blocks = [], related = ["bug-e1327f"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn validate_accepts_a_builtin_default_model' crates/roko-cli/src/ && cargo test -p roko-cli --lib validate_accepts_a_builtin_default_model"
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
