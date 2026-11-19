+++
id = "gap-a2787f"
kind = "gap"
title = "An isolated tool stays refused in its workspace for good: nothing lists or releases a tool control (.roko/immune/tool-controls.json), since roko safety controls/release cover agent controls only, and the refusal does not say how to lift it"
status = "done"
triage = "verified"
severity = "p2"
goal = "core"
size = "M"
subsystem = ["roko-agent/safety", "roko-cli/commands"]
created = 2026-10-20
updated = 2026-10-21
last_verified = 2026-10-21
last_verified_rev = "c758c9048"
source = "bug-1f8db7 fix (2026-10-20)"
discovered_from = "bug-1f8db7"
anchors = ["crates/roko-agent/src/tool_immune.rs::check_tool_control", "crates/roko-cli/src/commands/safety.rs::SafetyCmd"]
links = { depends_on = [], blocks = [], related = ["bug-1f8db7"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn safety_lists_and_releases_a_tool_control' crates/roko-cli/src && cargo test -p roko-cli --lib safety_lists_and_releases_a_tool_control"

[closed]
at = 2026-10-21
at_ts = "2026-10-21T03:49:50Z"
commit = "c758c9048"
by = "commit trailer"
executor = "claude-agent"
via = "work-batch"
size = "M"
claimed_at = "2026-10-21T01:31:08Z"
forced = false
evidence = "c758c9048. Cause: the tool-result boundary writes a tool's isolation to the canonical workspace's .roko/immune/tool-controls.json and check_tool_control refuses the tool there until the entry goes, but roko safety controls and release read agent-controls.json only, nothing else listed or removed a tool control, and the refusal said nothing about lifting it. Fix: roko-agent's tool_immune.rs adds list_tool_controls (the controls in force, read under the ledger's lock and failing closed on an invalid ledger) and release_tool_control (removes one tool and scope's control under the lock, writing first one ReleasedToolControl line to .roko/immune/tool-control-releases.jsonl: released_at_ms, tool, scope, state, issued_at_ms, triggering_output, by, reason; nothing to release writes no line), reusing the agent releases' audit appender; the isolation refusal now says a person releases it after review and names roko safety controls and roko safety release --tool <tool> --scope <scope> --reason <why>. roko-cli: roko safety controls lists tool controls after agent ones (tool, scope, state, issue time, triggering output hash) and says how to release one, --json is an object with agent_controls and tool_controls (no reader of the old array in the repo or the Hermes plugin); roko safety release takes --tool and --scope (both required together, exclusive with an agent id); the command module moved into the library, as diagnose did, so its tests run under --lib. Checked through heavy.sh on c758c9048's tree: the [[verify]] test safety_lists_and_releases_a_tool_control passes (a tool isolated through the real ToolDispatcher by a handler's non-JSON structured result is refused with the release command, listed, released through the parsed command with one audit line naming its triggering output, its next call runs, and a second release fails with no new line), with safety_still_lists_and_releases_an_agent_control; roko-agent tool_immune, immune_evidence and dispatcher tests 71 passed, among them the new a_released_tool_control_is_audited_and_the_next_call_runs and bug-1f8db7's isolation checks; roko-cli safety_cli 3 passed through the roko binary, among them the new safety_release_clears_a_tool_control; the built binary's clap refuses --tool without --scope and an agent id beside --tool; cargo clippy -p roko-agent -p roko-cli --no-deps -- -D warnings and cargo +nightly fmt -p roko-agent -p roko-cli -- --check clean. Filed find-d05b5e (nothing stops an agent's own bash from running roko safety release, and its audit line would name the person's USER)."
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
