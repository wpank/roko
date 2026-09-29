+++
id = "gap-0f3980"
kind = "gap"
title = "TaskDef Routing Metadata Wiring (24-field gap)"
status = "open"
triage = "verified"
severity = "p1"
size = "L"
goal = "core"
subsystem = ["roko-cli/dispatch"]
created = 2026-09-21
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "a17d9d766"
source = "tmp/backlog/archive/403-taskdef-routing-metadata-wiring.md#403 — TaskDef Routing Metadata Wiring (24-field gap)"
discovered_from = "audit:tmp/backlog/archive/403-taskdef-routing-metadata-wiring.md#403 — TaskDef Routing Metadata Wiring (24-field gap)"
anchors = ["crates/roko-cli/src/task_parser.rs::TaskDefSerde", "crates/roko-cli/src/task_parser.rs::TaskDef", "crates/roko-cli/src/task_parser.rs::TaskDef::build_prompt", "crates/roko-core/src/task.rs::Task", "crates/roko-cli/src/graph_task_dispatch.rs::build_routing_context", "crates/roko-cli/src/dispatch/model_routing.rs::RoutingInputs::from_task", "crates/roko-compose/src/templates/mod.rs::format_enhancements"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn parses_every_task_routing_field' crates/roko-cli/src/ && grep -rqw 'fn routing_context_uses_authored_task_metadata' crates/roko-cli/src/ && cargo test -p roko-cli --lib parses_every_task_routing_field && cargo test -p roko-cli --lib routing_context_uses_authored_task_metadata"
+++

## Problem

Plan authors cannot give the dispatcher per-task routing, gate, prompt or scheduling hints. `roko_core::Task`
(`crates/roko-core/src/task.rs`) defines 25 such fields, but the struct that `tasks.toml` is actually parsed into,
`TaskDefSerde` in `crates/roko-cli/src/task_parser.rs`, has none of them. It has no `deny_unknown_fields`, so a
task like this one parses without an error or warning, and every one of the extra values is thrown away:

```toml
[[task]]
id = "T01"
title = "Check the parser"
role = "implementer"
category = "verification"
complexity_band = "complex"
reasoning_level = "high"
preferred_model = "claude-opus-4-1"
```

The Graph dispatcher routes this task as `Implementation` (category comes from the role) and `Standard`
(complexity comes from the defaulted `tier`), with `thinking_level: None`. `roko plan validate` is silent.

The 25 missing fields, grouped by purpose:
- Routing: `category`, `complexity_band`, `reasoning_level`, `speed_priority`, `preferred_model`,
  `preferred_provider`, `escalate_on_retry`.
- Gates: `quality_profile`, `test_invariants`.
- Prompt and context: `context_weight`, `skills`, `example_pattern`, `context_files`, `plan_section`,
  `types_to_define`, `formulas`, `imports`, `research_before_edit`.
- Scheduling and infrastructure: `parallel_group`, `exclusive_files`, `tags`, `dependency_tags`, `fixture_keys`,
  `sidecar_requirements`, `integration_surfaces`.

## Why it matters

- Goal `core`. Routing and gates cannot tell a verification task from an implementation task, or a hard one
  from an easy one, except through `role`/`tier` spellings. Cost and quality depend on that.
- Code written against `roko_core::Task` never receives real data, because the Graph path only ever sees
  `TaskDef`:
  - `roko-agent/src/composition.rs::select` reads `complexity_band`, `reasoning_level`, `speed_priority` and
    `quality_profile`.
  - `roko-daimon/src/lib.rs` has `from_task(&Task, …)` helpers.
  - `roko-compose/src/templates/mod.rs::{TaskEnhancements, format_enhancements}` renders `types_to_define`,
    `formulas`, `imports`, `example_pattern` and `test_invariants`. No CLI code fills `TaskEnhancements`.
- No `tasks.toml` under `plans/` uses any of the 25 keys today (checked 2026-09-29), so nothing breaks now.
  The cost is that authors and plan generators cannot use them at all.
- Related: `gap-439794` (file-conflict detection, fed by `exclusive_files`) and `gap-7c9e48` (plan parallelism).

## Where

- `crates/roko-cli/src/task_parser.rs`: `TaskDefSerde` (line ~131, what serde fills), `TaskDef` (line ~59, what
  the runtime uses), `impl From<TaskDefSerde> for TaskDef` (line ~185), `TaskDef::build_prompt` (line ~503, the
  task section of the prompt), `mod tests` (line ~1759).
- `crates/roko-core/src/task.rs::Task` (line ~359): the full field list and the typed enums
  (`TaskCategory`, `TaskComplexityBand` with `escalate()`, `TaskReasoningLevel`, `TaskSpeedPriority`,
  `TaskQualityProfile`, `TaskContextWeight`). Also `default_exclusive_files` (line ~516, returns `true`) and
  `deserialize_optional_boolish` (line ~631).
- `crates/roko-cli/src/graph_task_dispatch.rs::build_routing_context` (line ~3105, called at ~3440 and ~4215):
  derives `task_category` from the role and `complexity` from `task.tier`, and hard-codes
  `thinking_level: None`, `iteration: 0` and `has_prior_failure: false`.
- `crates/roko-cli/src/dispatch/model_routing.rs::RoutingInputs::from_task` (line ~99): reads `task.model_hint`.
  Budget pressure is the only thing that sets `prefer_cheaper` (lines ~495-510).
- Entry point: `roko plan run <dir>` → `run_graph_plan` → `GraphTaskDispatcher`. Plans load through
  `runner::plan_loader` → `TasksFile::parse_str`.

## Current state

- Unchanged as of `d9e79e9d8`: `Task` has 34 fields, `TaskDef` 27, `TaskDefSerde` 26; all 25 fields above are
  missing from both CLI structs.
- `TaskDef` has no `Default` impl. There are 44 `TaskDef { … }` struct literals in `crates/` (14 in
  `task_parser.rs`, the rest in dispatch, runner and test files). Adding 25 separate fields to `TaskDef` means
  editing every one of them.
- Checkpoint fingerprints are computed from the authored `tasks.toml`, not from `TaskDef`
  (`graph_checkpoint.rs` lines 8-14). The module doc says adding task fields keeps in-flight checkpoints
  resumable.

## Plan

1. Decide how the CLI carries the fields.
   - Option A: copy all 25 fields into `TaskDefSerde` and `TaskDef`. This is the backlog's plan. It touches all
     44 literals and creates a third copy of the field list.
   - Option B (recommended): a `TaskHints` struct in `roko-core/src/task.rs` with the 25 fields (same types and
     serde attributes as `roko_core::Task`), deriving `Default`. Add `#[serde(flatten)] pub hints: TaskHints` to
     `TaskDefSerde` (flatten works with TOML and the boolish deserializers) and `pub hints: TaskHints` to
     `TaskDef`. Each struct literal gains one line: `hints: TaskHints::default()`.
   - Leave `roko_core::Task` alone in this item. Migrating it to embed `TaskHints` touches `roko-daimon`,
     `orchestrator/dag.rs` and `demo_seed.rs`, about 100 uses.
2. Add a drift-guard test, `parses_every_task_routing_field`, in `task_parser.rs` `mod tests`.
   - Build a `roko_core::Task` with every optional field set and serialize it to a TOML `[[task]]` table (`TasksFile` renames `tasks` to `task`).
   - Parse it with `TasksFile::parse_str` and assert that every value comes through in `TaskDef.hints`.
   - It should fail when someone adds a field to `Task` but not to `TaskHints`. Comparing serialized key sets is
     the simplest way to catch that.
3. In `build_routing_context`: `task_category = hints.category.unwrap_or(<role inference>)`,
   `complexity = hints.complexity_band.unwrap_or(<tier inference>)`, and
   `thinking_level = hints.reasoning_level.map(|r| r.label().to_string())` (`label()` exists, `task.rs` ~185).
4. Wire the model and provider fields:
   - In `RoutingInputs::from_task`, use `task_model_hint = task.model_hint.or(hints.preferred_model)`, with
     `model_hint` winning. Make it a separate field, not a serde alias: an alias rejects files that set both keys.
   - Pass `preferred_provider` to the provider selection. Put it where `force_backend` is resolved.
   - Map `speed_priority = Latency` to `prefer_cheaper: true`, merged the way budget pressure is merged.
5. Retries: with `escalate_on_retry = true`, apply `complexity.escalate()` when `task_attempts`
   (graph_task_dispatch.rs ~line 1178) shows a previous attempt. Set `iteration` and `has_prior_failure` from the
   same count; both are hard-coded today, so the router never knows a dispatch is a retry.
6. Prompt fields: reuse the existing renderer. In `TaskDef::build_prompt`, build a
   `roko_compose::templates::TaskEnhancements` from `types_to_define`, `formulas`, `imports`, `example_pattern`
   and `test_invariants`, and append `format_enhancements(&enhancements)`. Expand `context_files` into
   `context.read_files` (`why = "context"`) inside `From<TaskDefSerde>`.
7. Everything else (`quality_profile`, `context_weight`, `skills`, `plan_section`, `research_before_edit`,
   `parallel_group`, `exclusive_files`, `tags`, `dependency_tags`, `fixture_keys`, `sidecar_requirements`,
   `integration_surfaces`) gets parsed and kept, but not acted on.
   - Make `roko plan validate` add a `TaskQualityWarning` naming each set field that the Graph engine ignores.
   - File follow-up items for the gate profile (`quality_profile`/`test_invariants` → rung selection in
     `runner/gate_dispatch.rs`) and for context depth. `exclusive_files` belongs to `gap-439794`.

## Done when

- A task that sets all 25 keys parses with every value present on `TaskDef`; a task without them parses as before.
- A test builds a `RoutingContext` for a task with `role = "implementer"`, `category = "verification"`,
  `complexity_band = "complex"` and `reasoning_level = "high"`, and gets `Verification`, `Complex` and
  `Some("high")`.
- `preferred_model` reaches `RoutingInputs.task_model_hint` when `model_hint` is absent.
- `roko plan validate` warns about set fields that are parsed but not yet used.
- Verify:
  `grep -rqw 'fn parses_every_task_routing_field' crates/roko-cli/src/ && grep -rqw 'fn routing_context_uses_authored_task_metadata' crates/roko-cli/src/ && cargo test -p roko-cli --lib parses_every_task_routing_field && cargo test -p roko-cli --lib routing_context_uses_authored_task_metadata`

## Notes

- Do not add `deny_unknown_fields` to `TaskDefSerde`. Existing plans carry extra keys, and rejecting them would
  break runs. Warn in `roko plan validate` instead.
- `exclusive_files` defaults to `true` in `roko_core`. Parsing it must not change scheduling in this item.
  Nothing reads it yet, and `gap-439794` owns the conflict check.
- Keep the new fields out of the Graph node identity and the fingerprint (`roko-graph/src/fingerprint.rs`), so
  checkpoints stay resumable.
- Routing changes affect learning: `routing_ctx_for_feedback` is recorded with the category and complexity.
  After this change the recorded category is the authored one, which is the point, but it shifts cascade-router
  statistics for tasks that use the new keys.
- Teaching the plan generator (`roko prd plan`) to emit these keys is a separate follow-up.
- Parallel safety: this edits `task_parser.rs`, `graph_task_dispatch.rs` and `dispatch/model_routing.rs`. Avoid
  running it alongside `bug-c34782`, which also edits routing feedback in `graph_task_dispatch.rs`.

## Original notes

`roko_core::Task` (`crates/roko-core/src/task.rs`) defines 34 fields covering model routing, gate selection, prompt assembly, scheduling, and infrastructure declarations. Every field has a complete type definition, serde derives, unit tests, and doc comments.

Imported without verification from:
- `tmp/backlog/archive/403-taskdef-routing-metadata-wiring.md#403 — TaskDef Routing Metadata Wiring (24-field gap)`

Some cited files are gone: `crates/roko-cli/src/runner/event_loop.rs`, `crates/roko-gate/src/dispatch.rs`.

How to verify: Check: `cargo build --workspace` compiles with all 24 new fields in `TaskDefSerde` and `TaskDef`.; `cargo test -p roko-cli` passes, including task parser roundtrip tests with all new fields.; A `tasks.toml` that uses every new field parses without… [evidence: no status line; no index/roll-up evidence]

Verified 2026-09-28: roko_core::Task (crates/roko-core/src/task.rs) has 34 fields, crates/roko-cli/src/task_parser.rs TaskDef 27 and TaskDefSerde 26; 25 Task field names are absent from TaskDef, among them preferred_model, preferred_provider, reasoning_level, speed_priority, quality_profile, escalate_on_retry, complexity_band, category, tags, skills, parallel_group, exclusive_files, context_files, test_invariants.

2026-09-29: re-verified at d9e79e9d8. Unchanged: Task 34 fields, TaskDef 27, TaskDefSerde 26, and 25 Task fields are still missing from TaskDef.
