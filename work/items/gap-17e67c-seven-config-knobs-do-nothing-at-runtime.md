+++
id = "gap-17e67c"
kind = "gap"
title = "Seven config knobs do nothing at runtime: [conductor] parallel_enabled and six [learning] knowledge knobs, and no run warns that they are inert"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-core/config", "roko-cli/inert_settings"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
source = "status-quo-2026-10-15#finding-42"
anchors = ["crates/roko-cli/src/graph_task_dispatch/inert_settings.rs::graph_engine_inert_settings", "crates/roko-core/src/config/presets.rs", "roko.toml"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["spec-9a3131", "spec-1ced1d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn inert_settings_lists_unread_conductor_and_learning_knobs' crates/roko-cli/ && cargo test -p roko-cli inert_settings_lists_unread_conductor_and_learning_knobs"
+++

## Problem

Status-quo assessment 2026-10-15 at 7b6a5af44, finding 42 (`tmp/status-quo/2026-10-15/backlog/A03-31-32-defaults-and-specs.md`
line 101, `A05-41-42-loops-and-prompts.md` point 5):

- `roko.toml:399` sets `[conductor] parallel_enabled = false`, which reads like a switch that turns parallelism off.
  Only `roko config` display (`commands/config_cmd.rs`) and the TUI's config view (`tui/config_meta.rs`) read it, plus
  its env-registry entry and preset default (`config/presets.rs:93`); nothing at runtime does.
- Six `[learning]` knobs have no reader outside `crates/roko-core/src/config/` and the TUI: `knowledge_file_intel`,
  `knowledge_warnings`, `knowledge_wave_context`, `learning_min_occurrences`, `file_intel_max_entries` and
  `warning_max_entries`. (A seventh, `knowledge_error_patterns`, was wired by 4210.)

## Why it matters

Goal `tooling`: a setting that silently does nothing misleads whoever tunes roko, and the papers' claims about
configurable loops rest on settings doing what they say.

## Where

- `crates/roko-cli/src/graph_task_dispatch/inert_settings.rs::graph_engine_inert_settings`: the existing list of
  settings a Graph run ignores, which `roko plan run` reports.
- The knobs' definitions in `crates/roko-core/src/config/` and `roko.toml:399`.

## Current state

`graph_engine_inert_settings` already reports other inert settings (spec-1ced1d adds and removes the
`gates.impact_*` entries the same way).

## Plan

1. For each of the seven, check once more that nothing reads it at runtime; if one should act (for example
   `parallel_enabled` gating plan-set parallelism), wire it instead and say so.
2. Add the inert ones to `graph_engine_inert_settings`, so a run that sets one to a non-default value warns.
3. Remove `parallel_enabled = false` from this repo's `roko.toml` (it implies a behaviour that isn't there).
4. Test `inert_settings_lists_unread_conductor_and_learning_knobs`: a config setting each of the seven to a
   non-default value gets one inert-setting entry per knob.

## Done when

The `[[verify]]` command passes.

## Notes

Don't delete the fields from the schema in this item: old roko.toml files must still load. Parent: the hygiene epic
spec-9a3131.
