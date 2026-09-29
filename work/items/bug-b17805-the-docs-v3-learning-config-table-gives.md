+++
id = "bug-b17805"
kind = "bug"
title = "The docs/v3 [learning] config table gives wrong defaults for eight fields"
status = "open"
triage = "unverified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["docs/v3"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (14:32, wk-dream-default's report on bug-470de8)"
anchors = ["docs/v3/depth/21-config/01-schema-sections.md", "crates/roko-core/src/config/learning.rs::LearningConfig"]
lane = "docs"
parent = "spec-9a3131"
links = { depends_on = [], blocks = [], related = ["bug-470de8", "gap-7a3527", "reg-c7ecf6", "gap-cdf3fc"], supersedes = [], duplicate_of = "" }

[[verify]]
command = '''grep -q '^| `auto_playbook_refresh` | bool | true |' docs/v3/depth/21-config/01-schema-sections.md && grep -q '^| `knowledge_file_intel` | bool | true |' docs/v3/depth/21-config/01-schema-sections.md && grep -q '^| `replan_on_gate_failure` | bool | true |' docs/v3/depth/21-config/01-schema-sections.md && grep -q '^| `gate_threshold_flush_interval` | u64 | 10 |' docs/v3/depth/21-config/01-schema-sections.md'''
+++

## Problem

The `[learning] -- LearningConfig` table in `docs/v3/depth/21-config/01-schema-sections.md` (about lines 185-198)
disagrees with the code's defaults (the serde defaults and `impl Default for LearningConfig`,
`crates/roko-core/src/config/learning.rs:47-222`) in eight rows:

| Field | The table says | The code |
|---|---|---|
| `auto_playbook_refresh` | false | true |
| `knowledge_file_intel`, `knowledge_warnings`, `knowledge_wave_context`, `knowledge_error_patterns` | false | true |
| `replan_on_gate_failure` | false | true |
| `replan_max_per_plan` | 1 | 2 |
| `gate_threshold_flush_interval` | 300 (seconds) | 10 gate observations, not seconds (`learning.rs:10`, `:116-123`) |

The table also leaves out `learning_min_occurrences` (2), `file_intel_max_entries` (15), `warning_max_entries` (5),
`replan_gate_attempts` (3), `lookahead_threshold` (0.7), `override_learning_dampening` (unset, read as 0.5), and the
`dreams` and `knowledge` sub-tables.

## Why it matters

Operators and agents read docs/v3 as the configuration reference, and it states the opposite of the real behaviour for
the knowledge-injection switches and the playbook refresh. Several of these keys also do nothing on Graph runs
(gap-7a3527: `replan_max_per_plan`, `replan_gate_attempts`; reg-c7ecf6: `gate_threshold_flush_interval`), and the page
does not say so. Part of epic spec-9a3131.

## Where

- `docs/v3/depth/21-config/01-schema-sections.md`: the `[learning]` table.
- `crates/roko-core/src/config/learning.rs::LearningConfig`: the field docs, serde defaults and `Default` impl.
- `crates/roko-cli/src/graph_task_dispatch.rs::graph_engine_inert_settings`: the keys the Graph engine ignores.

## Current state

Checked at `4c0326dfc`: all eight rows are wrong. `docs/v2/19-CONFIG.md:733-743` already has the right values for
`auto_playbook_refresh` and `gate_threshold_flush_interval`. bug-470de8's branch edits the `dream_on_completion` row of
the same table, together with its code change.

## Plan

1. Correct the eight rows from `LearningConfig`, including the unit of `gate_threshold_flush_interval`.
2. Add the missing fields, with the `dreams` and `knowledge` sub-tables as their own small tables.
3. Mark the keys `graph_engine_inert_settings` lists as having no effect on Graph runs, with the item that tracks each
   (gap-7a3527, reg-c7ecf6).
4. Leave the `dream_on_completion` row to bug-470de8.

## Done when

- [ ] Every row in the `[learning]` table matches `LearningConfig`'s default, and every field has a row.
- [ ] Keys that do nothing on Graph runs say so.
- [ ] The `[[verify]]` command passes.

## Notes

- Docs lane. bug-470de8 edits one row of this table. Merge after it, or leave its row alone.
- If gap-7a3527 removes the replan keys from the schema first, drop their rows instead of correcting them.
