+++
id = "bug-cbbaa8"
kind = "bug"
title = "A try ends when the agent hands back even with its own verify red and most of its turns left, so each red hand-off spends a whole try and the next starts from a worse tree (phase 3: all six strong tries handed back at 13-25 of 120 turns with tests failing; T1 went 131/132 -> 0/2 -> exhausted)"
status = "open"
triage = "verified"
severity = "p1"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-10-21
updated = 2026-10-21
last_verified = 2026-10-21
source = "round-v7-hermes-pilot#M-p3-resume-dfba45e8"
discovered_from = "agent:hermes-pilot (round v7 phase 3 runs bdc4461e and dfba45e8)"
anchors = ["crates/roko-agent/src/tool_loop/mod.rs", "crates/roko-cli/src/graph_task_dispatch/verification.rs::settle_task_verification", "crates/roko-cli/src/graph_task_dispatch/streaming.rs"]
links = { depends_on = [], blocks = [], related = ["bug-62ae1e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_red_hand_off_with_turns_left_continues_the_same_try' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_red_hand_off_with_turns_left_continues_the_same_try"
+++

## Problem

A try ends as soon as the agent hands back, even when its own verify steps are red and most of its turn budget is
left. Every red hand-off spends a whole try, and the next try starts from whatever tree this one left, often a worse
one.

Round v7, Undertow phase 3 (roko e4643dafd, plan `p3-undertow-hypnotic-sequencer`, T1 the 270-line sequencer,
architectural: 120 turns, no time bound on the highest rung a chat run reaches). Run `bdc4461e`, then its resume
(serve run `dfba45e8`). All six tries on strong (gpt-5.4-mini) ran the spec tests themselves, saw them fail, and ended
their turn:

| Try | Turns used (of 120) | Agent | The agent's own last test result | Ended |
|---|---|---|---|---|
| T1#3 | 19 | 55 s | 131 of 132 assertions (P3.02: `e8.size()` 16 == 8) | gate_failed |
| T1#4 | 21 | 56 s | the same | gate_failed |
| T1#5 | 21 | 72 s | 77 of 149 | gate_failed |
| T1#6 | 13 | 57 s | 0 of 2 | gate_failed |
| T1#7 | 25 | 64 s | 0 of 2 | gate_failed |
| T1#8 | 18 | 56 s | 86 of 149 | gate_failed, so T1 was exhausted |

The tree went from 131/132 to 77/149 to 0/2 to 0/2 to 86/149, and the phase failed twice ($1.10). Each strong try
had about 100 turns left when it stopped. With the failing output sent back into the same session, the agent could
have kept working from 131 of 132.

## Where

- `crates/roko-agent/src/tool_loop/mod.rs` (~line 1580): "No tool calls -> final answer". A turn without tool calls
  ends the run ("tool_loop: stop — no tool calls, returning final text", ~1631), whatever the task's checks say.
  - The loop already continues in one case. A blank reply at the output limit is "asked again within the run"
    (bug-872885, ~1603–1618): a user message is pushed and the loop continues.
- `crates/roko-cli/src/graph_task_dispatch/verification.rs::settle_task_verification` (~line 63) runs the verify
  steps after the agent's result. It's called from `crates/roko-cli/src/graph_task_dispatch/streaming.rs` (~549)
  and `graph_task_dispatch.rs` (~2076).
  - A failing step settles the try as `gate_failed`. The retry feedback goes to the next try, a new session.

## Plan

When the implementer hands back and its task's verify steps fail with turn budget left, roko sends the failing output
back into the same try and continues the agent's session. Up to a bound, 2–3 continues, it says: "your check still
fails: <the step's output>; keep going". Only after that does the try settle as `gate_failed`.

- The continue counts as turns of the same try, not a new try.
- Each continue runs the verify steps again.
- A pass after a continue settles the try as passed.
- Record the continues on the verdict (`continues: n`) and in the run summary, so their effect can be measured.

## Done when

An implementer that hands back with a failing verify step and turns left is sent back with that step's output, up to
the bound, before its try settles. A pass after a continue settles as passed. Checked by the test in `[[verify]]`.
