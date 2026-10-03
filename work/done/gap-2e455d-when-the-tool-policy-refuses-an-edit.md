+++
id = "gap-2e455d"
kind = "gap"
title = "When the tool policy refuses an edit, the next attempt's no-changes feedback doesn't say so"
status = "done"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-cli/graph-task-dispatch"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
last_verified_rev = "21edf3817"
source = "bug-ef82eb investigation (2026-10-03)"
discovered_from = "bug-ef82eb"
anchors = ["crates/roko-cli/src/graph_task_dispatch/red_flags.rs", "crates/roko-agent/src/safety/contract.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["bug-ef82eb", "gap-31c0e8"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn no_changes_feedback_names_a_refused_edit' crates/roko-cli/src/ && cargo test -p roko-cli no_changes_feedback_names_a_refused_edit"

[closed]
at = 2026-10-03
at_ts = "2026-10-03T16:20:02Z"
commit = "21edf3817"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-03T07:50:04Z"
forced = false
evidence = "Gate 8b (work/backlog-batch-8b, merged into main as 21edf3817): cargo check --workspace --tests, nightly fmt, cargo clippy --workspace -D warnings, nextest --lib 10,702 tests over roko-agent, -cli, -compose, -core, -learn and -serve (and 4,847 roko-cli/-learn after PK39's fix), roko-cli bin 432 passed, the golden-path canaries pass incl. prompt_relevance_canary (plan_validate: only bug-2a31bc's two known alias tests fail), roko-learn, roko-agent and roko-compose integration tests pass, ViabilityBench suite 678 passed, PK36's shakedown 8/8 against the batch binary; every [[verify]] passes. A no-changes rejection names each tool call the policy refused (tool, path, rule, remedy) in its message and in the retry's diagnosis; RequireToolBeforeEdit unchanged; D6's stub reads first."
+++

## Problem

When the tool policy refuses an attempt's edit, the next attempt isn't told why nothing changed. The implementer
contract's `RequireToolBeforeEdit` rule refuses a `write_file` to an existing file the agent hasn't read with
`read_file` first. The attempt then ends `pre_verify:no_changes`, and the retry's feedback is the generic red flag
from `no_changes_red_flag` (`crates/roko-cli/src/graph_task_dispatch/red_flags.rs`): "its attempts have left the
working tree as they found it. Make the change the task asks for." It doesn't name the refused call or the rule, so
a model that writes before reading can repeat the same refused write on every retry and the task ends
`gate_failed`. Found by bug-ef82eb's investigation (2026-10-03): PK36's shakedown scenario D1 failed this way, and D6
hits the same refusal.

## Why it matters

Cheap models often write without reading first. The refusal is right (it's a safety contract), but feedback that
hides it wastes the retry budget and makes the ladder climb for a reason the task could have fixed in one turn.

## Where

- `crates/roko-cli/src/graph_task_dispatch/red_flags.rs`: `no_changes_red_flag` and the gate feedback it feeds.
- Where the attempt's denied tool operations are recorded (the safety layer's denials, see failover.rs's
  `a_denied_codex_operation_is_recorded_with_the_tool_policy`), and `crates/roko-agent/src/safety/contract.rs`
  (`RequireToolBeforeEdit`).

## Current state

The refusal is logged (roko.log) and recorded, but not passed into the no-changes feedback.

## Plan

1. When an attempt that changed nothing had tool calls refused by the policy, put them in the no-changes feedback:
   the tool, the path, the rule, and what satisfies it (for RequireToolBeforeEdit: read the file first).
2. A test: an attempt whose only write is refused for a missing read gets feedback naming the file and the rule,
   and a retry that reads first succeeds.
3. Don't relax `RequireToolBeforeEdit`; whether to allow a write when the file was already in the prompt is a
   separate policy question for Will.

## Done when

- [ ] The `[[verify]]` command passes.

## Notes

- Related: bug-ef82eb (the shakedown stub now reads before writing).
