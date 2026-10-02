+++
id = "bug-9affca"
kind = "bug"
title = "Under trusted live output, unscreened tool starts and results stay beside their screened copies"
status = "done"
triage = "verified"
severity = "p2"
goal = "visibility"
size = "S"
subsystem = ["roko-cli/tui"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "efb9acf44"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-cc61a3"
anchors = ["crates/roko-cli/src/tui/state/mod.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["bug-cc61a3"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn screened_tool_steps_replace_unscreened' crates/roko-cli/src/ && cargo test -p roko-cli --lib screened_tool_steps_replace_unscreened"

[closed]
at = 2026-10-02
at_ts = "2026-10-02T01:38:30Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-02T00:41:52Z"
forced = false
evidence = "Gate 6g on 698ca793d, merged as efb9acf44 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on roko-cli, roko-core, roko-learn and roko-neuro; lib tests pass (roko-cli 3431, roko-core 1986, roko-learn 1234, roko-neuro 239); all eight canaries, golden_path_suite, secret_canary and C2 pass; graph_timeout_matrix 7/7 including the worktree-mode case; bin 445; portal tsc clean and vitest 807/807; Cargo.lock unchanged. Implemented in this round; the item's notes name the change and its test."
+++

## Problem

bug-cc61a3 settles unscreened assistant text when the screened transcript arrives. Unscreened tool starts and results (raw arguments and output) still stay in the TUI beside their screened copies.

## Plan

Settle tool steps the same way. Add a test named `screened_tool_steps_replace_unscreened`.

## Done when

- The test passes.

## Notes

- Reported on 2026-10-02 by wk-streams, working on bug-cc61a3, during the overnight close-out round.
- 2026-10-02 (wk-streams): implemented on work/gap-b35a57; cargo verification deferred to the batch check.
  `StreamRecord::ToolResult` now carries `live` and `screened` like the other variants, and `classify_output_line`
  tracks a tool start or result that is live and unscreened (raw arguments or output), so bug-cc61a3's settle drops
  it when the screened transcript begins, and drops a late one until the next attempt. The live tool step
  (`TuiBridge::tool_step`: name and scrubbed target) is screened and stays. A tool pair evicted together also leaves
  the unscreened set. Test: `screened_tool_steps_replace_unscreened` (`tui/app/tests.rs`); gap-836ae9's parity test
  now expects its unscreened tool result settled away too.
