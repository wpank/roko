+++
id = "bug-2379dc"
kind = "bug"
title = "When a CLI names no model, the CLI adapters still record the configured slug as the served model"
status = "open"
triage = "unverified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-agent/cli_adapters"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-model-truth's report on branch work/bug-31438d at ee6a541ef)"
anchors = ["crates/roko-agent/src/claude_cli_agent.rs", "crates/roko-agent/src/codex_agent.rs"]
lane = "rust-hot"
parent = "spec-b7303f"
links = { depends_on = ["bug-31438d"], blocks = [], related = ["bug-31438d", "bug-35379d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_cli_that_names_no_model_leaves_the_served_model_unknown' crates/roko-agent/src/ && cargo test -p roko-agent --lib a_cli_that_names_no_model_leaves_the_served_model_unknown"
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
