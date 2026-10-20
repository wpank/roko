+++
id = "gap-a2787f"
kind = "gap"
title = "An isolated tool stays refused in its workspace for good: nothing lists or releases a tool control (.roko/immune/tool-controls.json), since roko safety controls/release cover agent controls only, and the refusal does not say how to lift it"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
size = "M"
subsystem = ["roko-agent/safety", "roko-cli/commands"]
created = 2026-10-20
updated = 2026-10-20
last_verified = 2026-10-20
source = "bug-1f8db7 fix (2026-10-20)"
discovered_from = "bug-1f8db7"
anchors = ["crates/roko-agent/src/tool_immune.rs::check_tool_control", "crates/roko-cli/src/commands/safety.rs::SafetyCmd"]
links = { depends_on = [], blocks = [], related = ["bug-1f8db7"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn safety_lists_and_releases_a_tool_control' crates/roko-cli/src && cargo test -p roko-cli --lib safety_lists_and_releases_a_tool_control"
+++

## Problem

When the tool-result immune boundary isolates a tool, it writes the control to the workspace's
`.roko/immune/tool-controls.json` (`tool_name`, `source_scope`, `control.state = "isolated"`), and
`check_tool_control` refuses every later call of that tool in that workspace with "permission denied: tool source is
isolated by the immune boundary". The ledger lives at the canonical workspace root, so the refusal outlives the run and
the attempt worktree.

Nothing lists or lifts such a control. `roko safety controls` and `roko safety release` read and write
`.roko/immune/agent-controls.json`, the provider boundary's per-agent controls, not tool controls. The
`ToolControlState::Isolated` docs say calls "remain blocked pending an explicit review workflow", and there is no such
workflow: the only way out is to hand-edit the JSON ledger. The refusal message does not say how to release it either.

## Why it matters

One false positive is enough to take a builtin tool away from every later run in a workspace. In the 2026-10-19
golden-path runs (bug-1f8db7) one passing `run_tests` result isolated `run_tests` (`builtin:local`) for the rest of each
run; in a person's own workspace it would have stayed isolated for good. A person has no supported way to see that this
happened or to undo it after review.

## Where

- `crates/roko-agent/src/tool_immune.rs::check_tool_control` (the refusal), `persist_tool_control` (the ledger write),
  `ToolControlState::Isolated`.
- `crates/roko-cli/src/commands/safety.rs::SafetyCmd` (`controls` and `release`, agent controls only).

## Current state

bug-1f8db7 removed the false positive that isolated `run_tests` (a host-side note or cut of a structured result no
longer reads as a malformed result). Real anomalies still isolate a tool, as they should, with no release path.

## Plan

1. `roko safety controls` also lists tool controls: tool, source scope, state, issue time, the triggering output hash.
2. `roko safety release` takes a tool and source scope too, removes that control under the ledger's lock, and appends
   an audit line (who, why, when, the triggering output) as agent releases do.
3. The refusal message names the command that lists and releases the control.

## Done when

- [ ] The `[[verify]]` test passes: a workspace with an isolated tool lists it, releases it with an audit line, and the
  tool's next call runs.

## Notes

- Keep releases explicit and audited; do not let a control expire silently.
- Found while fixing bug-1f8db7.
