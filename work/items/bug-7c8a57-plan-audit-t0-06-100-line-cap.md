+++
id = "bug-7c8a57"
kind = "bug"
title = "100-line cap on read_files context injection"
status = "open"
triage = "verified"
severity = "p1"
size = "S"
goal = "core"
subsystem = ["roko-cli/task_parser"]
created = 2026-09-21
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "faa378453"
source = "tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T0-06: Remove 100-line file injection cap"
discovered_from = "audit:tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T0-06: Remove 100-line file injection cap"
anchors = ["crates/roko-cli/src/plan_policy.rs::render_declared_context", "crates/roko-cli/src/dispatch/prompt_builder.rs::build_runner_context", "crates/roko-cli/src/task_parser.rs::build_prompt", "crates/roko-neuro/src/context.rs::gather_read_files", "crates/roko-compose/src/context_provider.rs::add_inline_files"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'lines().take(100)' crates/roko-cli/src/task_parser.rs && grep -qw 'fn renderer_injects_unranged_file_past_line_80' crates/roko-cli/src/plan_policy.rs && cargo test -p roko-cli renderer_injects_unranged_file_past_line_80"
+++

## Problem

A plan task that lists a file under `[tasks.context] read_files` without a `lines` range gets only the first
80 lines of that file in its prompt. Nothing tells the agent the file is longer: the block is rendered as
`<declared-file path="..." lines="1-80" ...>` with no total line count and no truncation marker. The same prompt
section then says "Do not search ... If required context is absent, stop and request plan repair instead of
exploring." For a typical Rust source file the first 80 lines are `use` statements and struct fields, so the agent
either edits blind, ignores the instruction and explores, or stops.

Expected: an un-ranged entry is injected in full up to a clear per-file bound; when it is cut, the output says so
(for example `[... truncated: showed lines 1-N of M; add a lines range to read_files]`).

The item was first filed against `content.lines().take(100)` in `TaskDef::build_prompt`
(`crates/roko-cli/src/task_parser.rs`). That function has no callers at HEAD; the live cap is the 80-line default
in `plan_policy::render_declared_context`.

## Why it matters

- Goal `core` (plan runs work reliably): every `roko plan run` task prompt goes through this renderer. Generated
  and hand-written plans commonly list `read_files` without ranges, so agents routinely get a truncated view and
  are told not to look further.
- Wasted turns and wrong edits, and it undermines the "declared context is authoritative" contract that
  `plan_policy.rs` is built on.
- Backlog #407 (original spec, rated P0 there).

## Where

- `crates/roko-cli/src/plan_policy.rs::render_declared_context` (line ~590): renders the "Declared source context"
  block. For each `read_file`: explicit `lines` range (fails if longer than `policy.max_range_lines`), plus ±8
  lines (`ANCHOR_CONTEXT_RADIUS`) around each explicit symbol anchor found in the file; if neither applies,
  `ranges.push((1, lines.len().min(80)))` (line ~663). After each file it fails the task with "declared snippets
  ... exceed the N byte prompt budget" when `output.len() > policy.max_declared_context_bytes`.
- `crates/roko-cli/src/plan_policy.rs` constants: `NORMAL_MAX_RANGE_LINES = 1_000`,
  `NORMAL_MAX_DECLARED_CONTEXT_BYTES = 64 * 1024`, `FAST_MAX_RANGE_LINES = 240`,
  `FAST_MAX_DECLARED_CONTEXT_BYTES = 24 * 1024`; `PlanExecutionPolicy::for_environment()` picks FAST when
  `ROKO_FAST_MODE` is set.
- `crates/roko-cli/src/plan_policy.rs` preflight (line ~486): validates explicit ranges only; says nothing about
  un-ranged entries.
- `crates/roko-cli/src/dispatch/prompt_builder.rs::build_runner_context` (line ~1250): the Graph-path caller; maps
  a renderer error to `RunnerDispatchError::PreValidationFailed`.
- Entry point: `roko plan run` -> Graph task dispatch (`crates/roko-cli/src/graph_task_dispatch.rs`) -> prompt
  builder -> `build_runner_context`.
- `crates/roko-cli/src/task_parser.rs::build_prompt` (line ~503, `take(100)` at ~565): dead code, no callers.
  Its helper `extract_line_range` (line ~1402) is used only by `build_prompt` and a unit test.
- Other `take(100)` copies of the same logic, reachability from `roko plan run` not confirmed:
  `crates/roko-neuro/src/context.rs::gather_read_files` (line ~802, via `ContextAssembler::gather`) and
  `crates/roko-compose/src/context_provider.rs::add_inline_files` (line ~1654, via the "task-requirements"
  context source). `crates/roko-neuro/src/context.rs:1729` (`summarize_content`, a 100-char log preview) is not
  injection and must stay.

## Current state

- The 80-line default came in with the fail-fast plan policy in `72e0a76b8` (2026-09-04); unchanged since.
- No `[context]` section exists in `roko.toml` or in `roko-core` config (`grep -rn max_file_bytes crates/roko-core`
  finds nothing), so both limits are hard-coded.
- Existing test `renderer_inlines_numbered_range_and_anchor_context` (plan_policy.rs, ~line 1040) covers ranges
  and anchors, not un-ranged long files.
- The 2026-09-28 note below says `roko-neuro/src/context.rs` no longer has the cap; that is wrong — `take(100)` is
  still at line ~802.

## Plan

Design choice for un-ranged entries:

- Option A (recommended): render the whole file up to `policy.max_range_lines` lines (1,000 normal, 240 FAST) and
  never let an un-ranged entry fail the task. If the file is longer, or if adding it would push `output` past
  `max_declared_context_bytes`, stop at the last line that fits and append a marker line
  `[... truncated: showed lines 1-N of M; add a lines range to read_files]`. Explicit ranges keep today's
  fail-fast behaviour. Add the total line count to the tag (for example `total="M"`).
- Option B: keep a small default but make un-ranged entries a preflight warning (`PLAN_CONTEXT_RANGE_MISSING`) so
  plan authors must add ranges. Honest, but pushes work onto every plan and does not help existing plans.
- Option C (original spec): a configurable `[context] max_file_bytes` byte budget per file. Needs a new config
  section in `roko-core`; can be layered on A later.

Steps (Option A):

1. In `render_declared_context`, replace `ranges.push((1, lines.len().min(80)))` with
   `(1, lines.len().min(policy.max_range_lines))` and record whether the file was cut.
2. For un-ranged entries only, check the byte budget line by line while rendering; on overflow stop and write the
   marker instead of returning `Err`. Keep the `Err` for explicit ranges and anchors.
3. Emit the marker when cut, and `total="M"` on the `<declared-file>` tag.
4. Delete the dead `TaskDef::build_prompt` and `extract_line_range` in `task_parser.rs` (and the
   `extract_line_range_works` test there), or route it through `render_declared_context`. Deleting is simpler;
   run clippy to confirm nothing else becomes dead.
5. Optionally apply the same bound + marker to `roko-neuro` `gather_read_files` and `roko-compose`
   `add_inline_files` with a shared helper, so every injection path behaves alike.
6. Tests in `plan_policy.rs`: `renderer_injects_unranged_file_past_line_80` (a 300-line file with no range is
   rendered past line 80 under `PlanExecutionPolicy::normal()`), and a test that a file longer than
   `max_range_lines` ends with the truncation marker and returns `Ok`.

## Done when

- A task whose `read_files` entry names a 300-line file without `lines` gets all 300 lines in its prompt.
- A longer file is cut with a visible marker naming shown and total lines; the task is not failed for it.
- `lines().take(100)` no longer appears in `crates/roko-cli/src/task_parser.rs`.
- Verify (proposed; the current `[[verify]]` only checks that two problem patterns are gone, which changing `80`
  to another constant would satisfy):

  ```
  ! grep -q 'lines().take(100)' crates/roko-cli/src/task_parser.rs && grep -qw 'fn renderer_injects_unranged_file_past_line_80' crates/roko-cli/src/plan_policy.rs && cargo test -p roko-cli renderer_injects_unranged_file_past_line_80
  ```

## Notes

- `plan_policy.rs` is deliberately fail-fast and bounded; keep the per-task byte budget and the explicit-range
  errors. Only the silent default for un-ranged entries changes.
- FAST mode has a 24 KiB total budget; with Option A a FAST task with several large un-ranged files will see
  truncation markers, which is intended.
- Do not touch `summarize_content` in `roko-neuro/src/context.rs` (a log preview).
- Safe to do in parallel with most work; conflicts only with other edits to `plan_policy.rs` or the
  prompt-assembly section of `dispatch/prompt_builder.rs`.
- Implemented on `work/bug-7c8a57` at `feb21ddb1`; cargo verification deferred to the batch check. Option A: an un-ranged entry renders whole within `max_range_lines` and a fair share of the byte budget left after explicit ranges and anchors, else it is cut with the truncation marker and never fails the task; every tag carries `total`. The dead `TaskDef::build_prompt` and `extract_line_range` are deleted. Step 5 (the `roko-neuro` and `roko-compose` copies) is not done.

## Original notes

read_files entries without a line range are truncated with .take(100), so agents see only imports/declarations. Replace with a configurable byte budget (e.g. 64KB). Backlog #407.

Imported without verification from:
- `tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T0-06: Remove 100-line file injection cap`
- `tmp/archive/plan-audit-2026-09-23/15-file-context-injection.md`
- `tmp/backlog/archive/407-file-context-injection-cap.md#407 — Remove 100-line cap on `read_files` context injection`

How to verify: grep -n 'take(100)' in task_parser.rs and roko-neuro context.rs. / Check: Files listed in `read_files` without a `lines` range are injected in full up to a; When the budget is exceeded, the injected content ends with a marker of the form; The budget is configurable under `[context]` in `roko.toml` as… [evidence: no status line; no index/roll-up evidence]

Merged 2 mined candidates: m3-114, m1-130.

Verified 2026-09-28: crates/roko-cli/src/task_parser.rs:565 still truncates read_files entries without a line range via `content.lines().take(100)`; the crates/roko-neuro/src/context.rs anchor no longer contains a take(100) cap.

Rechecked 2026-09-29 at d9e79e9d8: the anchor points at dead code. TaskDef::build_prompt (task_parser.rs:503, take(100) at :565) has no callers. Graph-path prompts get read_files content from plan_policy::render_declared_context (plan_policy.rs:590), via dispatch/prompt_builder.rs::build_runner_context. That function shows only lines 1-80 of an un-ranged entry that has no symbol anchor (plan_policy.rs:663). It fails the task, rather than truncating, when declared context exceeds max_declared_context_bytes: 64 KiB normal, 24 KiB FAST (plan_policy.rs:21, :30). Nothing under [context] in roko.toml configures either limit. The fix belongs in render_declared_context. The dead build_prompt should be deleted or routed through it.
