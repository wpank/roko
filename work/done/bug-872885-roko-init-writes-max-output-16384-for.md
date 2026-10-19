+++
id = "bug-872885"
kind = "bug"
title = "roko init writes max_output = 16384 for glm-4-7, so on a large task the mid rung spends its whole output on reasoning (finish_reason=length, no tool call) and the try changes nothing"
status = "done"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/init"]
created = 2026-10-19
updated = 2026-10-19
last_verified = 2026-10-19
last_verified_rev = "83b013d42"
source = "round-v7-hermes-pilot#C-p0-run1"
discovered_from = "agent:hermes-pilot (round v7 phase 0, run 6988f0b8, T3 try 1)"
anchors = ["crates/roko-cli/src/commands/init.rs"]
links = { depends_on = [], blocks = [], related = ["bug-f6b99f", "bug-c22bd9"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_reasoning_only_length_cut_is_retried_within_the_try' crates/ && cargo test -p roko-agent -- a_reasoning_only_length_cut_is_retried_within_the_try"

[closed]
at = 2026-10-19
at_ts = "2026-10-19T14:38:05Z"
commit = "83b013d42"
size = "S"
claimed_at = "2026-10-19T13:24:39Z"
forced = false
evidence = "83b013d42: (1) roko init gives the reasoning models max_output = 32768: glm-4-7 and glm-4-7-openrouter (were 16384), gpt-5-4-mini and its OpenRouter twin (16384), gpt-oss-120b and its OpenRouter twin (8192; context_window 128000 -> 98304 so window plus output fits the provider 131072). Z.ai docs and a live check (max_tokens 131073 refused with code 1210, 131072 and 32768 accepted; about 0.00002 USD) confirm GLM-4.7 allows up to 131072; 32768 keeps a call that reasons to the limit (~37 tok/s) inside the 30-minute stream cap. Test reasoning_models_leave_room_to_reason_and_act. (2) Tool loop: a reply that reaches the output limit with no tool call and no text is a traced, billed turn under the turn cap, and the model is asked again within the run with a note to act now; a second such reply in a row ends the run as before (bug-f6b99f); a turn that acts resets the count. Tests a_reasoning_only_length_cut_is_retried_within_the_try (T3 shape: read, cut, write, answer in one run; failed before the fix with final text \"\") and two_reasoning_only_length_cuts_in_a_row_end_the_try. (3) The blame: T3 try 1 made 20 read-only calls before the cut, and bug-f6b99f ends a cut after tool work as Stop, so the unchanged tree was rejected as pre_verify:no_changes (agent, counts to the climb). Turn traces now carry finish_reason, and such a no_changes rejection names the output-limit stop in its message and the next try diagnosis. Tests through the graph dispatcher: a_try_whose_reply_ran_out_of_output_reasoning_goes_on_and_passes (passes in one try) and no_changes_feedback_names_a_closing_reply_cut_at_the_output_limit. Checks at 83b013d42: roko-agent lib 2373 passed; roko-cli lib 3853 passed (share_run_scrubs_output_text_and_prompt skipped); clippy -D warnings on roko-agent and roko-cli clean; nightly rustfmt clean."
+++

## Problem

Round v7, Undertow phase 0, run `6988f0b8`, T3 try 1 (mid rung, glm-4.7 on Z.ai, roko 3b34aed92). The agent read
the task's files over 5 iterations. Then one model call streamed 16,491 reasoning deltas and ended:

```
tool_loop: stop — no tool calls, returning final text iterations=5 final_text_len=0 final_text_empty=true finish_reason=Some("length") input_tokens=1166 output_tokens=16384
tool_loop: model hit output token limit (finish_reason=length) and produced no final text — increase max_output for this model
```

The model spent its whole output limit on reasoning, so the call produced no tool call and no text. The try changed
nothing (`pre_verify:no_changes`, blamed on the agent). It took 472 s and $0.063, and counted toward the climb.

`roko init` writes this model's profile with `max_output = 16384` (and `context_window = 128000`). GLM-4.7 is a
reasoning model, and on a large task (a JUCE plugin skeleton) its thinking alone outgrew the limit.

## Why it matters

glm-4.7 is the mid rung of `roko init`'s default ladder, and integrative tasks start there. A try lost to a
too-small output limit is wasted time and money, and it pushes climbs that the task's difficulty didn't earn.
bug-f6b99f made a length cut count against the agent, which is right when the model rambles, but here the limit is
roko's own setting.

## Where

- `crates/roko-cli/src/commands/init.rs`: the `glm-4-7` model entry (`key: "glm-4-7"`, `max_output = 16384`).
- The tool loop's length handling: a cut with no tool work fails the try (bug-f6b99f).

## Plan

1. Raise `max_output` for reasoning models in `roko init`'s profiles to what the provider allows (Z.ai documents a
   far larger output limit for GLM-4.7). Or set a reasoning budget below `max_output` where the provider takes one.
2. When a call ends `finish_reason=length` with reasoning only (no tool call, no text), retry that call once in the
   same try, with a larger output limit or a "stop thinking and act" nudge. Don't end the try.

## Done when

`roko init`'s glm-4-7 entry leaves room for reasoning plus a tool call, and a reasoning-only length cut is retried
within the try. Checked by the test in `[[verify]]`.

## Notes

- Evidence: `tmp/hermes-roko-v7/pilot/evidence/p0-run1/` (attempts.jsonl and the log excerpt).
- Related:
  - bug-f6b99f: length cuts blamed on the agent;
  - bug-c22bd9 (the 60 s tool cut).
