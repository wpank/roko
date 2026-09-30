+++
id = "bug-2379dc"
kind = "bug"
title = "When a CLI names no model, the CLI adapters still record the configured slug as the served model"
status = "done"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-agent/cli_adapters"]
created = 2026-09-29
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "9c0b9aed0"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-model-truth's report on branch work/bug-31438d at ee6a541ef)"
anchors = ["crates/roko-agent/src/claude_cli_agent.rs", "crates/roko-agent/src/codex_agent.rs"]
lane = "rust-hot"
parent = "spec-b7303f"
links = { depends_on = ["bug-31438d"], blocks = [], related = ["bug-31438d", "bug-35379d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_cli_that_names_no_model_leaves_the_served_model_unknown' crates/roko-agent/src/ && cargo test -p roko-agent --lib a_cli_that_names_no_model_leaves_the_served_model_unknown"

[closed]
at = 2026-09-30
by = "coordinator (session 7622b882)"
evidence = "Merged in 38521c19f. When a CLI names no model, the served model is recorded as unknown instead of the configured slug. Batch 15a gate on 9005da604, re-assembled as 8a2ee8bca with only settle's rustfmt commit changing two files' formatting (MAIN 9c0b9aed0 has the same code): cargo check --workspace --tests clean; nightly fmt clean; clippy -p roko-agent -p roko-cli -p roko-compose -p roko-core -p roko-learn -p roko-serve --keep-going -D warnings clean; lib tests pass: roko-cli 3190 (two flakes, the turn_policy escalated-timeout test and graph_run_routing_observations_survive_a_crash, pass alone and in their module), roko-agent 2268, roko-core 1952, roko-learn 1204, roko-serve 986, roko-compose 560; cargo test -p roko-cli --test learning_wiring_census: 2 passed. Verify: its test passes in that run and its static checks pass on MAIN."
+++

## Problem

bug-31438d (its branch at ee6a541ef, not merged at 7fa54b873) makes Roko record the model the provider reports. The CLI adapters still fill the reported model with the configured slug when the CLI's output names none:

- the Claude CLI adapter sets `usage.model = Some(model.to_string())` (`claude_cli_agent.rs:604` on the branch), and the stream path falls back with `unwrap_or(&self.model)` (:1256);
- the Codex adapter builds its usage with `model: Some(self.model.clone())` (`codex_agent.rs:309`).

A record then shows a "reported" model that nobody reported, and a substitution would look like a match.

## Why it matters

One settled record per attempt (epic spec-b7303f): `model_reported` must mean what the provider said. When the provider said nothing, the record should say unknown.

## Where

The two adapters' usage construction.

## Plan

1. Leave `usage_obs.model` as None when the CLI output names no model. Keep the configured slug in the dispatched-model field only.
2. Add `a_cli_that_names_no_model_leaves_the_served_model_unknown`.

## Done when

- [ ] A CLI run whose output names no model records the served model as unknown.
- [ ] The `[[verify]]` command passes.

## Notes

- Build on bug-31438d's branch.
- Implemented on `work/bug-b8af02` at `366d63a91`; cargo verification deferred to the batch check. `a_cli_that_names_no_model_leaves_the_served_model_unknown` (targeted `cargo test` passed at the branch head). When the output names no model, the adapters now leave `usage_obs.model` as None: the Claude CLI (its stream usage, on success and failure), Codex, and the same pattern in Cursor and the Anthropic API adapter. Failure paths with no response record None. The configured slug stays on the output's `model` tag, which dispatch records as `model_dispatched`. Codex's `usage_obs_falls_back_to_configured_model` became `usage_obs_leaves_an_unnamed_model_unknown`. Not done: the native Gemini adapter still fills it with the configured slug, because its response type does not parse `modelVersion` (follow-up).
