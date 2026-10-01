+++
id = "gap-8921a3"
kind = "gap"
title = "Dead code paths (GraphExecutionEvent sink, duplicate streaming verify path, build_fix_prompt)"
status = "open"
triage = "verified"
severity = "p2"
size = "S"
goal = "tooling"
subsystem = ["roko-graph/engine"]
created = 2026-09-25
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "ebdc0f5d5"
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#P4 — dead config and dead code"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#P4 — dead config and dead code"
anchors = ["crates/roko-graph/src/engine.rs::with_event_sink", "crates/roko-cli/src/graph_execution/plan_runner.rs::run_graph_plan_body", "crates/roko-cli/src/runner/structured_log.rs::GraphEventLogger", "crates/roko-cli/src/task_parser.rs::build_fix_prompt"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "{ ! grep -q 'fn with_event_sink' crates/roko-graph/src/engine.rs || grep -n 'event_sink' crates/roko-graph/src/engine.rs | grep -vE 'event_sink: |fn with_event_sink|self\\.event_sink = Some|^[0-9]+:[[:space:]]*//' | grep -q .; } && { ! grep -q 'pub fn build_fix_prompt' crates/roko-cli/src/task_parser.rs || grep -rn 'build_fix_prompt(' crates/roko-cli/src --include='*.rs' | grep -v 'task_parser.rs' | grep -q .; } && ! grep -q 'GraphEventLogger::open' crates/roko-cli/src/graph_execution/plan_runner.rs"
+++

## Problem

Three pieces of dead code remain from the 2026-09-25 dogfood finding. The fourth piece, the duplicate streaming
verify path, is already fixed.

1. `GraphEngine` stores a `GraphEventSink` (`crates/roko-graph/src/engine.rs:327`, set by `with_event_sink` at
   `:448-450`) but never reads it, so it never emits a `GraphExecutionEvent`.
2. The host still wires a sink that can never be attached. `run_graph_plan_body`
   (`crates/roko-cli/src/graph_execution/plan_runner.rs:1259-1273`) opens a
   `runner::structured_log::GraphEventLogger` when `log_file` is `Some`, and attaches it with
   `engine.with_event_sink` at `:2015-2017`. `run_graph_plan` (`:702-706`), however, hands every `--log-file` run to
   `event_log::run_recorded`, which takes `log_file` out of the params before it calls the body. `log_file` is
   therefore always `None` in the body, and this code never runs.
3. `TaskDef::build_fix_prompt` (`crates/roko-cli/src/task_parser.rs:612`) is called only from its two unit tests
   (`build_fix_prompt_includes_error_output` and `build_fix_prompt_truncates_long_error`, around lines 2462-2535).
   It also truncates with a byte slice (`&error_output[..4000]`), which would panic on a multi-byte character if it
   were ever used.

Expected: every piece is either used in production or deleted.

## Why it matters

- Goal `tooling` (code hygiene). Dead wiring misleads readers. A comment at `plan_runner.rs:2013` says "attach the
  event sink so every GraphExecutionEvent is written to JSONL", which is not true. Unused prompt builders also invite
  someone to wire the wrong one.
- Related:
  - `reg-cbfff6` (goal `visibility`): the same dead engine sink, framed as "make the engine emit node lifecycle
    events". Whatever is decided for the sink must be consistent across both items.
  - `bug-230de6`: the Graph engine does not produce `events.jsonl` in mock runs.

## Where

- `crates/roko-graph/src/engine.rs`: the `event_sink` field (line 327), its `None` initializer (line 353) and
  `with_event_sink` (line 448). `event_seq` (line 329) sits next to it.
- `crates/roko-graph/src/events.rs`: `GraphEventSink` and `GraphExecutionEvent` (line 204), re-exported from
  `crates/roko-graph/src/lib.rs:106`. The test `crates/roko-cli/tests/side_effect_parity.rs` also uses them.
- `crates/roko-cli/src/graph_execution/plan_runner.rs`:
  - `run_graph_plan` (line 701): delegates `--log-file` runs.
  - `run_graph_plan_body` (line 772): `graph_event_logger` is at lines 1259-1273 and is passed through the context
    field `graph_event_logger` (lines 1336 and 1693) to the attach at lines 2015-2017.
- `crates/roko-cli/src/graph_execution/event_log.rs::run_recorded` (line 129): the real `--log-file` recorder. It
  subscribes to the StateHub and writes JSONL.
- `crates/roko-cli/src/runner/structured_log.rs`:
  - `GraphEventLogger` (line 102) and `FanOutGraphEventSink` (line 158). Their only production reference is the
    dead code above.
  - `StructuredLogger` (line 33) in the same file is still used by `runner/types.rs`. Keep it.
- `crates/roko-cli/src/task_parser.rs::TaskDef::build_fix_prompt` (line 612) and its tests.
- The production retry and fix feedback lives elsewhere: the prompt assembler's "# Previous attempt feedback"
  section (`crates/roko-cli/src/dispatch/prompt_builder.rs:2798`) and `crates/roko-compose/src/gate_feedback.rs`.

## Current state

- Fixed in `725f21e05`: `--log-file` writes JSONL through the StateHub-subscribing recorder
  (`graph_execution/event_log.rs`). The streaming path no longer has its own verify logic. The batch and streaming
  paths both call the shared `settle_task_verification` in `graph_task_dispatch.rs`.
- Still dead at HEAD `a17d9d766`, re-checked 2026-09-29: the engine sink (1), the unreachable `GraphEventLogger`
  wiring (2) and `build_fix_prompt` (3).
- The current `[[verify]]` only passes if the engine starts *using* the sink. It cannot pass if the sink is deleted,
  although the original notes allow either outcome.

## Plan

1. Delete `TaskDef::build_fix_prompt` and its two tests in `task_parser.rs`. Nothing in production calls it, and
   production feedback goes through `prompt_builder.rs` and `gate_feedback.rs`.
2. Delete the unreachable host wiring in `plan_runner.rs`: the `graph_event_logger` block (lines 1259-1273), the
   context field `graph_event_logger`, and the attach at lines 2015-2017 with its misleading comment. Then delete
   `GraphEventLogger` and `FanOutGraphEventSink` from `runner/structured_log.rs` if nothing else uses them. Check
   with `grep -rn 'GraphEventLogger\|FanOutGraphEventSink' crates/`. Keep `StructuredLogger`.
3. Settle the engine sink together with `reg-cbfff6`. Pick one option:
   - Emit (the `reg-cbfff6` fix): the engine emits node lifecycle events through `event_sink`. The sink is then no
     longer dead, and the `[[verify]]` below accepts that.
   - Remove: delete `event_sink` and `with_event_sink` from `GraphEngine`, and close `reg-cbfff6` as wontfix, with
     the reason that the StateHub, not engine events, is the live event path.

   Recommended: do not decide this inside this item. If `reg-cbfff6` is not being worked on, leave the engine sink
   alone, finish steps 1 and 2, and add a dated note to this item saying that only step 3 remains. The item closes
   once `reg-cbfff6` lands or the sink is removed, because the `[[verify]]` below checks for that.
4. Run `cargo clippy --workspace --no-deps -- -D warnings`. Removing a field or function can leave unused imports
   behind, for example `GraphEventSink` in `plan_runner.rs`.

## Done when

- `build_fix_prompt` is gone, or it has a production caller.
- `plan_runner.rs` no longer opens a `GraphEventLogger`.
- `GraphEngine.event_sink` is either read (events are emitted) or removed.
- `cargo test -p roko-cli task_parser` and `cargo test -p roko-graph` still pass.
- Verify (it accepts either outcome for the sink):
  `{ ! grep -q 'fn with_event_sink' crates/roko-graph/src/engine.rs || grep -n 'event_sink' crates/roko-graph/src/engine.rs | grep -vE 'event_sink: |fn with_event_sink|self\.event_sink = Some|^[0-9]+:[[:space:]]*//' | grep -q .; } && { ! grep -q 'pub fn build_fix_prompt' crates/roko-cli/src/task_parser.rs || grep -rn 'build_fix_prompt(' crates/roko-cli/src --include='*.rs' | grep -v 'task_parser.rs' | grep -q .; } && ! grep -q 'GraphEventLogger::open' crates/roko-cli/src/graph_execution/plan_runner.rs`

## Notes

- Deletions only. There is no behaviour change, because the removed paths never run.
- `plan_runner.rs` is a busy file, so keep the diff small and rebase often. Steps 1 and 2 are safe in parallel with
  most items. The sink decision in step 3 must not collide with work on `reg-cbfff6` or `bug-230de6`.
- Do not touch `event_log.rs`. It is the live `--log-file` path, and `scripts/run_evidence.py --require-events`
  (`./dev.sh fast`) depends on its format.

## Original notes

The engine stores a GraphExecutionEvent sink but never emits, so --log-file writes nothing; a second streaming verify implementation (graph_task_dispatch.rs:3463-3602) drifts with no cap/feedback/auto-fix; TaskDef::build_fix_prompt is test-only.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#P4 — dead config and dead code`
- `tmp/archive/dogfood-audit-2026-09-03/01-findings-register.md#Implemented findings with additional context from the audits`
- `tmp/archive/dogfood-2026-08-25/DOGFOOD-DEBRIEF.md#Issue 2: Log output buffering prevents monitoring (MEDIUM)`

How to verify: Run plan run --log-file and check file content.

Partly fixed (checked 2026-09-28 against 3d0ee4d02): Fixed in 725f21e05: --log-file now writes JSONL through a StateHub-subscribing recorder (crates/roko-cli/src/graph_execution/event_log.rs, run_recorded), and the streaming path no longer has its own verify logic: both the batch path (graph_task_dispatch.rs:3545-3551) and the streaming path (:3943-3965) call the shared settle_task_verification (:1708). Still dead: the engine stores event_sink (crates/roko-graph/src/engine.rs:327) and with_event_sink sets it (:448-450; attached from graph_execution/plan_runner.rs:1886), but the engine never reads it, so no GraphExecutionEvent is emitted. TaskDef::build_fix_prompt (crates/roko-cli/src/task_parser.rs:612) is still called only from tests (:2494, :2535, inside mod tests at :1759).

Rechecked 2026-09-29 at d9e79e9d8: the --log-file recorder and the single settle_task_verification path (725f21e05) are still in place. Two dead paths remain: GraphEngine stores event_sink (roko-graph engine.rs:327, set by with_event_sink at :448-449) but never reads or emits through it (the same defect is tracked in reg-cbfff6), and TaskDef::build_fix_prompt (roko-cli task_parser.rs:612) is still called only from tests (:2494, :2535). Close once the sink is used (or removed) and build_fix_prompt is used in production or deleted.
- 2026-10-01 (coordinator): the "dead GraphExecutionEvent sink" part is no longer dead: since reg-cbfff6 (work/reg-cbfff6) the Graph engine emits node lifecycle events to it (wk-runstate). Re-check this item's remaining dead paths.
- 2026-10-01 (wk-scheduler): implemented on work/bug-28b604; cargo verification deferred to the batch check.
  Steps 1 and 2 are done; step 3 resolved itself, because the engine emits node lifecycle events since
  reg-cbfff6.
  - Deleted `TaskDef::build_fix_prompt` and its two tests.
  - Deleted the unreachable `--log-file` wiring in `plan_runner.rs`: the `GraphEventLogger` block, the
    `graph_event_logger` context field and the `with_event_sink` attach. The body now destructures `log_file: _`,
    because `event_log::run_recorded` takes that path.
  - Deleted `GraphEventLogger` and `FanOutGraphEventSink` from `runner/structured_log.rs`, with their tests, and kept
    `StructuredLogger`.
  - The `[[verify]]` greps pass statically.
